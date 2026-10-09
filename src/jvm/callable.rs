//! What scalac's class files offer to code that teq did not compile, in link mode: the jars'
//! bytecode, reflection and the `java` launcher. A case class's companion with `apply`,
//! `unapply` and `fromProduct`, the case class's `copy`.

use super::classfile::*;
use super::gen::*;
use super::names::*;
use super::Cx;
use super::names::encode;
use crate::symbols::*;
use crate::types::*;
use std::rc::Rc;

pub const MIRROR_PRODUCT: &str = "scala/deriving/Mirror$Product";
pub const MIRROR_SUM: &str = "scala/deriving/Mirror$Sum";
pub const MIRROR_SINGLETON: &str = "scala/deriving/Mirror$Singleton";
const ACC_ENUM: u16 = 0x4000;

/// scalac's order of the forwarders of a mirror class (`sortedMembersBasedOnFlags`): the
/// members by their source names, the module's own and the inherited ones alike, and the
/// overloads of a name by the erased types of their parameters in turn, then of the result.
/// The type parameters a signature counts are not told apart here.
fn forwarder_order(a_name: &str, a_desc: &str, b_name: &str, b_desc: &str) -> std::cmp::Ordering {
    let by_name = decode(a_name).encode_utf16().cmp(decode(b_name).encode_utf16());
    by_name.then_with(|| {
        let (a_params, a_ret) = parse_method_desc(a_desc);
        let (b_params, b_ret) = parse_method_desc(b_desc);
        let params = a_params.iter().zip(&b_params).map(|(a, b)| type_name_order(a, b, false)).find(|o| o.is_ne());
        params.unwrap_or_else(|| a_params.len().cmp(&b_params.len())).then_with(|| type_name_order(&a_ret, &b_ret, true))
    })
}

/// Two erased types as scalac's signatures order their names: a class's own name first, then
/// its owner's the same way, and a name without an owner before one with.
fn type_name_order(a: &JType, b: &JType, result: bool) -> std::cmp::Ordering {
    fn segments(t: &JType, result: bool) -> Vec<String> {
        let scala = |name: &str| vec!["scala".to_string(), name.to_string()];
        match t {
            JType::V if result => scala("Unit"),
            JType::V => vec!["scala".to_string(), "runtime".to_string(), "BoxedUnit".to_string()],
            JType::Z => scala("Boolean"),
            JType::B => scala("Byte"),
            JType::S => scala("Short"),
            JType::C => scala("Char"),
            JType::I => scala("Int"),
            JType::J => scala("Long"),
            JType::F => scala("Float"),
            JType::D => scala("Double"),
            JType::L(n) if n.starts_with('[') => {
                let mut of = segments(&parse_type(&n[1..]).0, false);
                if let Some(last) = of.last_mut() {
                    last.push_str("[]");
                }
                of
            }
            JType::L(n) => {
                let module = n.ends_with('$');
                let mut of: Vec<String> = n.trim_end_matches('$').split(|c| c == '/' || c == '$').map(str::to_string).collect();
                if let (true, Some(last)) = (module, of.last_mut()) {
                    last.push('$');
                }
                of
            }
        }
    }
    fn order(a: &[String], b: &[String]) -> std::cmp::Ordering {
        match (a.split_last(), b.split_last()) {
            (Some((x, [])), Some((y, []))) => x.encode_utf16().cmp(y.encode_utf16()),
            (Some((_, [])), Some(_)) => std::cmp::Ordering::Less,
            (Some(_), Some((_, []))) => std::cmp::Ordering::Greater,
            (Some((x, before_x)), Some((y, before_y))) => x.encode_utf16().cmp(y.encode_utf16()).then_with(|| order(before_x, before_y)),
            (a, b) => a.is_some().cmp(&b.is_some()),
        }
    }
    order(&segments(a, result), &segments(b, result))
}

impl<'a> Cx<'a> {
    /// A class of a package or of objects, whose companion module is reached statically.
    pub fn statically_placed(&self, c: ClassId) -> bool {
        let syms = self.input.syms;
        match syms.class(c).owner {
            Owner::Package(_) => true,
            Owner::Class(o) => syms.class(o).kind == ClassKind::Object && self.statically_placed(o),
            Owner::Local => false,
        }
    }

    /// A case class whose companion module class holds scalac's factory members.
    pub fn has_case_companion(&self, c: ClassId) -> bool {
        let info = self.input.syms.class(c);
        let case = self.is_case_class(c) || (info.kind == ClassKind::EnumCase && info.singleton.is_none());
        case
            && info.kind != ClassKind::Anon
            && self.emits(c)
            && self.statically_placed(c)
            && self.input.prog.classes[self.tclass_of[c.idx()] as usize].captures == 0
    }

    /// Whether a constructor of program class `c`, its primary one or a secondary one the output
    /// keeps, has a default.
    pub fn has_ctor_defaults(&self, c: ClassId) -> bool {
        let i = self.tclass_of[c.idx()];
        if i == u32::MAX {
            return false;
        }
        let tc = &self.input.prog.classes[i as usize];
        let funs = &self.input.prog.funs;
        tc.ctor_defaults.iter().any(Option::is_some) || tc.ctors.iter().any(|&f| self.input.reach.funs[f.idx()] && funs[f.idx()].defaults.iter().any(Option::is_some))
    }

    /// A class or trait of a package or of objects whose constructors' default getters are
    /// instance methods of its companion module class, `C$.$lessinit$greater$default$N`, as
    /// scalac lays them out (with static forwarders in `C`), each taking the parameters of the
    /// clauses before its own.
    pub fn ctor_defaults_on_companion(&self, c: ClassId) -> bool {
        let info = self.input.syms.class(c);
        matches!(info.kind, ClassKind::Class | ClassKind::Trait | ClassKind::Enum | ClassKind::EnumCase)
            && info.singleton.is_none()
            && self.emits(c)
            && self.statically_placed(c)
            && self.input.prog.classes[self.tclass_of[c.idx()] as usize].captures == 0
            && self.has_ctor_defaults(c)
    }

    /// A plain class or a trait without a companion object whose companion module class the
    /// backend writes for the getters of its constructor's defaults, as scalac makes one
    /// (`Plain$`).
    pub fn has_default_companion(&self, c: ClassId) -> bool {
        let info = self.input.syms.class(c);
        matches!(info.kind, ClassKind::Class | ClassKind::Trait)
            && self.ctor_defaults_on_companion(c)
            && !self.has_case_companion(c)
            && !self.has_value_companion(c)
            && info.companion.map_or(true, |o| !self.emits(o))
    }

    /// A case class whose `copy` the backend writes: one whose constructor takes no captures.
    pub fn has_copy(&self, c: ClassId) -> bool {
        let info = self.input.syms.class(c);
        let case = self.is_case_class(c) || (info.kind == ClassKind::EnumCase && info.singleton.is_none());
        case
            && info.kind != ClassKind::Anon
            && self.emits(c)
            && self.input.prog.classes[self.tclass_of[c.idx()] as usize].captures == 0
    }

    /// The companion class that takes the static forwarders of module class `o`, when both are
    /// written.
    pub fn forwarding_class(&self, o: ClassId) -> Option<ClassId> {
        let syms = self.input.syms;
        if syms.class(o).kind != ClassKind::Object || !self.emits(o) {
            return None;
        }
        let x = syms.class(o).companion?;
        let kind = syms.class(x).kind;
        (matches!(kind, ClassKind::Class | ClassKind::Enum) && self.emits(x) && self.statically_placed(x)).then_some(x)
    }

    /// The mirror class of static forwarders of top-level module class `o` that has no
    /// companion class taking them, named as scalac names it.
    fn mirror_class(&self, o: ClassId) -> Option<String> {
        let info = self.input.syms.class(o);
        if !self.program_file(info.file) || info.kind != ClassKind::Object || !matches!(info.owner, Owner::Package(_)) {
            return None;
        }
        let name = self.class_names[o.idx()].strip_suffix('$')?;
        (!self.class_by_name.contains_key(name)).then(|| name.to_string())
    }

    /// A value class that is no case class, whose companion module class holds the instance
    /// copies of its extension methods.
    pub fn has_value_companion(&self, c: ClassId) -> bool {
        let info = self.input.syms.class(c);
        info.value_class
            && info.kind == ClassKind::Class
            && !self.is_case_class(c)
            && self.emits(c)
            && self.statically_placed(c)
    }

    /// A program enum whose companion has no class of its own to take `values` and the rest.
    pub fn has_enum_statics(&self, e: ClassId) -> bool {
        let info = self.input.syms.class(e);
        info.kind == ClassKind::Enum
            && self.program_file(info.file)
            && self.statically_placed(e)
            && info.companion.map_or(false, |o| !self.emits(o))
    }

    /// A program enum whose companion module class this build writes with the values as its
    /// public static fields (`E$.A`), through which the program reads them as a build over the
    /// enum's products reads them, and with scalac's `Mirror.Sum`: an enum of a package or of
    /// objects.
    pub fn has_enum_fields(&self, e: ClassId) -> bool {
        let info = self.input.syms.class(e);
        info.kind == ClassKind::Enum
            && self.emits(e)
            && self.statically_placed(e)
            && info.companion.map_or(false, |o| self.emits(o) || self.has_enum_statics(e))
    }

    /// A file of the program rather than of the std.
    fn program_file(&self, f: crate::source::FileId) -> bool {
        !self.input.sources[f.0 as usize].is_std
    }

    /// A case class of one parameter clause, whose companion is its `Mirror.Product`.
    pub fn has_product_mirror(&self, c: ClassId) -> bool {
        self.input.syms.class(c).ctor.len() == 1 && !self.input.syms.class(c).value_class
    }

    /// A value class whose companion module class this build writes with the bodies of its
    /// methods, scalac's `m$extension(u, args)` instance methods, which the box's methods,
    /// the static forwarders of the class and every call go through.
    pub fn vc_on_companion(&self, c: ClassId) -> bool {
        self.input.syms.class(c).value_class && (self.has_value_companion(c) || self.has_case_companion(c))
    }

    /// The binary name of the companion module class of case class `c`.
    pub fn companion_name(&self, c: ClassId) -> String {
        format!("{}$", self.class_names[c.idx()])
    }
}

impl<'a> Gen<'a> {
    /// A class with the module class whose static forwarders it takes, which is written first
    /// so that its method table is known; a module class with its mirror class.
    pub fn emit_class_unit(&mut self, i: usize) -> (String, Vec<u8>) {
        let cx = self.cx;
        let c = cx.input.prog.classes[i].id;
        let written = cx.input.syms.class(c).companion.filter(|&o| cx.forwarding_class(o) == Some(c));
        self.keep_table = written.is_some() || cx.has_case_companion(c) || cx.has_enum_statics(c) || cx.has_value_companion(c) || cx.has_default_companion(c);
        let module = match written {
            Some(o) => Some(self.emit_class(cx.tclass_of[o.idx()] as usize)),
            None if cx.has_case_companion(c) => Some(self.emit_companion(i)),
            None if cx.has_enum_statics(c) => Some(self.emit_enum_companion(c)),
            None if cx.has_value_companion(c) => Some(self.emit_value_companion(c)),
            None if cx.has_default_companion(c) => Some(self.emit_default_companion(c)),
            None => None,
        };
        let Some(module) = module else {
            self.keep_table = cx.mirror_class(c).is_some();
            let out = self.emit_class(i);
            if let Some(mirror) = cx.mirror_class(c) {
                let table = std::mem::take(&mut self.module_table);
                let class = self.mirror(&mirror, &out.0, &table);
                self.extra_classes.push(class);
            }
            return out;
        };
        self.forward_from = Some((Rc::from(module.0.as_str()), std::mem::take(&mut self.module_table)));
        self.reset_class();
        let out = self.emit_class(i);
        self.extra_classes.push(module);
        out
    }

    /// The module class of a file's top-level definitions, and in link mode its mirror class.
    pub fn emit_file_unit(&mut self, f: crate::source::FileId) -> (String, Vec<u8>) {
        let name = self.cx.file_modules[f.0 as usize].strip_suffix('$').unwrap_or_default().to_string();
        let mirror = self.cx.program_file(f) && !self.cx.class_by_name.contains_key(&name);
        self.keep_table = mirror;
        let out = self.emit_file_module(f);
        if mirror {
            let table = std::mem::take(&mut self.module_table);
            let class = self.mirror(&name, &out.0, &table);
            self.extra_classes.push(class);
        }
        out
    }

    /// Finishes the class being written: the static forwarders it takes, and the method table
    /// of a module class for the class that will take its forwarders.
    pub fn finish_class(&mut self, name: &str) -> (String, Vec<u8>) {
        if let Some((module, table)) = self.forward_from.take() {
            self.static_forwarders(&module, &table);
        }
        if std::mem::take(&mut self.keep_table) {
            self.module_table = self.cw.method_table();
        }
        let cw = std::mem::replace(&mut self.cw, ClassWriter::new(0, "x", OBJECT, &[]));
        (name.to_string(), cw.finish(self.cx.input.output_version))
    }

    /// A class of static forwarders only, as scalac writes for a top-level object.
    fn mirror(&mut self, name: &str, module: &str, table: &[(u16, String, String)]) -> (String, Vec<u8>) {
        self.reset_class();
        self.cw = ClassWriter::new(ACC_PUBLIC | ACC_FINAL | ACC_SUPER, name, OBJECT, &[]);
        self.this_name = Rc::from(name);
        self.this_class = None;
        self.line_file = crate::source::FileId(u32::MAX);
        self.context = name.to_string();
        self.static_forwarders(module, table);
        let cw = std::mem::replace(&mut self.cw, ClassWriter::new(0, "x", OBJECT, &[]));
        (name.to_string(), cw.finish(self.cx.input.output_version))
    }

    /// Per public instance method of `module`, a static method of the class being written that
    /// calls it on `MODULE$`, unless the class declares a method of that name; the program's
    /// entry point is the `main(String[])` the `java` launcher calls.
    fn static_forwarders(&mut self, module: &str, table: &[(u16, String, String)]) {
        let cx = self.cx;
        let mut table = table.to_vec();
        if let Some(&o) = cx.class_by_name.get(module) {
            for (n, d) in self.inherited_methods(o, true) {
                if !table.iter().any(|(_, tn, td)| *tn == n && *td == d) {
                    table.push((ACC_PUBLIC, n, d));
                }
            }
        }
        table.sort_by(|(_, an, ad), (_, bn, bd)| forwarder_order(an, ad, bn, bd));
        let mut inherited_names: Vec<String> = match self.this_class {
            Some(x) => self.inherited_methods(x, false).into_iter().map(|(n, _)| n).collect(),
            None => Vec::new(),
        };
        // An enum inherits `ordinal` from `scala.reflect.Enum`, which its class file lists as
        // an interface only.
        if self.this_class.map_or(false, |x| cx.input.syms.class(x).kind == ClassKind::Enum) {
            inherited_names.push("ordinal".to_string());
        }
        let declared: Vec<bool> = table.iter().map(|(_, n, _)| self.cw.has_method_named(n) || inherited_names.contains(n)).collect();
        let entry = cx.input.prog.main.filter(|&m| matches!(cx.input.syms.sym(m).owner, Owner::Class(o) if cx.class_names[o.idx()] == module));
        let entry_ref = entry.map(|m| self.mref(m));
        let module_type = JType::L(Rc::from(module));
        // A member's name expanded by a class of the module's (`T$$x`, a trait's private member)
        // gets no forwarder, a trait setter's (`T$_setter_$x_$eq`) a synthetic one, as dotty's
        // `addForwarders` has them (`ExpandedName`, `SyntheticSetterName`); a name the source
        // writes so (`` `T$$x` ``) is no expansion.
        let bases: Vec<ClassId> = cx.class_by_name.get(module).map_or(Vec::new(), |&o| cx.input.syms.class(o).base_types.iter().map(|&(b, _)| b).collect());
        let prefixes: Vec<String> = bases.iter().map(|b| cx.class_names[b.idx()].replace('/', "$")).collect();
        let written = |name: &str| cx.input.interner.lookup(name).map_or(false, |n| bases.iter().any(|&b| cx.input.syms.class(b).members.contains_key(&n)));
        let qualified = |name: &str, separator: &str| name.find(separator).map_or(false, |i| prefixes.iter().any(|p| *p == name[..i])) && !written(name);
        for ((access, name, desc), declared) in table.iter().zip(declared) {
            let access = *access;
            if declared || name.starts_with('<') || access & (ACC_STATIC | ACC_PRIVATE | ACC_BRIDGE) != 0 {
                continue;
            }
            let trait_setter = qualified(name, "$_setter_$");
            if !trait_setter && qualified(name, "$$") {
                continue;
            }
            if let (Some(m), Some(r)) = (entry, &entry_ref) {
                if r.name == *name && r.desc == *desc {
                    self.entry_main(m, cx.input.prog.main_object);
                    continue;
                }
            }
            let (params, ret) = parse_method_desc(desc);
            let typed: Vec<(Option<SymId>, JType)> = params.iter().map(|t| (None, t.clone())).collect();
            self.begin_method(true, false, &typed, ret.clone(), &[]);
            self.getstatic(module, "MODULE$", &module_type);
            let mut slot = 0u16;
            for t in &params {
                self.load(slot, t);
                slot += if t.wide() { 2 } else { 1 };
            }
            self.invoke_desc(Invoke::Virtual, module, false, name, desc, params.len(), &ret);
            self.return_value(&ret);
            self.end_method(if trait_setter { ACC_PUBLIC | ACC_STATIC | ACC_SYNTHETIC } else { ACC_PUBLIC | ACC_STATIC }, name, desc);
        }
    }

    /// The methods class `c` inherits, by name and descriptor: a jar class's from its class
    /// file, a program class's from its members; `concrete` keeps those with a body only.
    /// `java.lang.Object`'s are left out.
    fn inherited_methods(&mut self, c: ClassId, concrete: bool) -> Vec<(String, String)> {
        let cx = self.cx;
        let syms = cx.input.syms;
        let mut out: Vec<(String, String)> = Vec::new();
        let bases: Vec<ClassId> = syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        for b in bases {
            if cx.class_names[b.idx()] == OBJECT {
                continue;
            }
            if let Some(cf) = cx.class_files.get(&b) {
                for m in &cf.methods {
                    let skip = ACC_STATIC | ACC_PRIVATE | ACC_BRIDGE | ACC_SYNTHETIC | if concrete { ACC_ABSTRACT } else { 0 };
                    if m.access & skip == 0 && !m.name.starts_with('<') {
                        out.push((m.name.clone(), m.descriptor.clone()));
                    }
                }
                continue;
            }
            if cx.tclass_of[b.idx()] == u32::MAX {
                continue;
            }
            // Its export forwarders, which its class file holds as methods.
            if let Some(list) = cx.input.reach.exports.classes.get(&b) {
                out.extend(self.export_forwarder_descs(list));
            }
            for m in cx.members_in_order(b) {
                let info = syms.sym(m);
                // A trait's given with parameters is a default method, which no class implements
                // again (`is_given_def`).
                let given_def = self.is_given_def(m);
                if info.owner != Owner::Class(b) || info.mods & crate::ast::mods::PRIVATE != 0 || !(matches!(info.kind, SymKind::Def | SymKind::Val | SymKind::Var) || given_def) {
                    continue;
                }
                let f = cx.fun_of_sym[m.idx()];
                let has_body = f != u32::MAX && cx.input.prog.funs[f as usize].body.is_some();
                if info.kind == SymKind::Def && (concrete && !has_body || !cx.input.reach.funs.get(f as usize).copied().unwrap_or(false) && concrete) {
                    continue;
                }
                if concrete && info.kind != SymKind::Def && !given_def {
                    continue;
                }
                let r = self.mref(m);
                out.push((r.name.clone(), r.desc.clone()));
            }
        }
        out
    }

    /// Class `run` of a `@main def run`, with the `main(String[])` the `java` launcher calls.
    pub fn emit_main_class(&mut self, main: SymId, held: &dyn Fn(&str) -> bool) -> Option<(String, Vec<u8>)> {
        let cx = self.cx;
        let info = cx.input.syms.sym(main);
        // A `@main` method's, of a package or of an object (scalac's `MainProxies`), named after
        // the method in the package of its file; a `main(args: Array[String])` runs as it is.
        if !info.is_main {
            return None;
        }
        let name = format!("{}{}", cx.pkg_path(cx.input.file_pkgs[info.file.0 as usize]), encode(cx.input.interner.get(info.name)));
        if cx.class_by_name.contains_key(&name) || held(&name) {
            return None;
        }
        self.reset_class();
        self.cw = ClassWriter::new(ACC_PUBLIC | ACC_FINAL | ACC_SUPER, &name, OBJECT, &[]);
        self.this_name = Rc::from(name.as_str());
        self.this_class = None;
        self.line_file = info.file;
        self.context = name.clone();
        let this = JType::L(self.this_name.clone());
        self.begin_method(false, true, &[], JType::V, &[]);
        self.load(0, &this);
        self.invoke(Invoke::Special, OBJECT, false, "<init>", &[], &JType::V);
        self.return_value(&JType::V);
        self.end_method(ACC_PUBLIC, "<init>", "()V");
        self.entry_main(main, None);
        let cw = std::mem::replace(&mut self.cw, ClassWriter::new(0, "x", OBJECT, &[]));
        Some((name, cw.finish(self.cx.input.output_version)))
    }

    /// `Point$` for a case class without a written companion, as scalac makes it.
    pub fn emit_companion(&mut self, idx: usize) -> (String, Vec<u8>) {
        let cx = self.cx;
        let c = cx.input.prog.classes[idx].id;
        let interfaces: Vec<String> = cx.has_product_mirror(c).then(|| MIRROR_PRODUCT.to_string()).into_iter().collect();
        let name = self.begin_module(c, &interfaces);
        self.companion_members(c, None);
        self.drain_pending();
        self.finish_class(&name)
    }

    /// `Color$` for an enum whose companion has no body, with its statics.
    pub fn emit_enum_companion(&mut self, e: ClassId) -> (String, Vec<u8>) {
        let interfaces: Vec<String> = self.cx.has_enum_fields(e).then(|| MIRROR_SUM.to_string()).into_iter().collect();
        let name = self.begin_module(e, &interfaces);
        let object = self.cx.input.syms.class(e).companion.expect("an enum has a companion");
        self.enum_companion_members(e, object);
        if self.cx.ctor_defaults_on_companion(e) {
            self.ctor_default_getters_of(e);
        }
        self.drain_pending();
        self.finish_class(&name)
    }

    /// `Meters$` for a value class without a written companion, with the instance copies of its
    /// extension methods.
    pub fn emit_value_companion(&mut self, c: ClassId) -> (String, Vec<u8>) {
        let name = self.begin_module(c, &[]);
        self.extension_copies(c);
        if self.cx.ctor_defaults_on_companion(c) {
            self.ctor_default_getters_of(c);
        }
        self.drain_pending();
        self.finish_class(&name)
    }

    /// `Plain$` for a plain class without a companion object, with the getters of its
    /// constructor's defaults.
    pub fn emit_default_companion(&mut self, c: ClassId) -> (String, Vec<u8>) {
        let name = self.begin_module(c, &[]);
        self.ctor_default_getters_of(c);
        self.drain_pending();
        self.finish_class(&name)
    }

    /// The getters of the defaults of class `c`'s constructors in its companion module class
    /// being written, scalac's `$lessinit$greater$default$N` (numbered across the clauses of
    /// one constructor; a secondary constructor's are its own, as only one constructor may
    /// have defaults), each taking the parameters of the clauses before its own, which is all
    /// a default may name.
    pub fn ctor_default_getters_of(&mut self, c: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let tc = &cx.input.prog.classes[cx.tclass_of[c.idx()] as usize];
        let clauses = syms.class(c).ctor.clone();
        self.companion_default_getters(&tc.ctor_params, &tc.ctor_defaults, &clauses);
        for &f in &tc.ctors {
            let fun = &cx.input.prog.funs[f.idx()];
            if !cx.input.reach.funs[f.idx()] || fun.defaults.iter().all(Option::is_none) {
                continue;
            }
            let clauses: Vec<ClauseSig> = syms.sym(fun.sym).sig.as_ref().map_or(Vec::new(), |sig| sig.clauses.clone());
            self.companion_default_getters(&fun.params, &fun.defaults, &clauses);
        }
    }

    /// A by-name parameter's getter returns the value, as a method's (`emit_fun`), which the
    /// constructor's caller passes as a thunk calling the getter.
    fn companion_default_getters(&mut self, params: &[SymId], defaults: &[Option<crate::tir::TExprId>], clauses: &[ClauseSig]) {
        let mut start = 0;
        for cl in clauses {
            let size = cl.params.len();
            let before: Vec<(Option<SymId>, JType)> = params[..start.min(params.len())].iter().map(|&p| (Some(p), self.sym_type(p))).collect();
            let before_types: Vec<JType> = before.iter().map(|p| p.1.clone()).collect();
            for i in start..(start + size).min(params.len()) {
                let Some(d) = defaults.get(i).copied().flatten() else { continue };
                let by_name = cl.params[i - start].by_name;
                let ret = if by_name { self.erase(cl.params[i - start].ty) } else { self.sym_type(params[i]) };
                self.begin_method(false, false, &before, ret.clone(), &[d]);
                self.default_value(d, by_name, &ret);
                self.return_value(&ret);
                self.end_method(ACC_PUBLIC, &format!("$lessinit$greater$default${}", i + 1), &method_desc(&before_types, &ret));
            }
            start += size;
        }
    }

    /// Per method of value class `c`, the companion's `m$extension(u, args)`, public and final,
    /// with the method's body, which reads the underlying value `u` where the body reads `this`,
    /// and per default of its parameters `m$default$N$extension(u, args before)`: scalac's
    /// layout, whose static forwarders the class takes. For a case class, scalac's erased
    /// family beside them: `copy$extension(u, x)`, `copy$default$1$extension(u)` and
    /// `_1$extension(u)`, each the underlying value.
    pub fn extension_copies(&mut self, c: ClassId) {
        let cx = self.cx;
        if !cx.input.syms.class(c).value_class {
            return;
        }
        if cx.vc_on_companion(c) {
            self.extension_bodies(c);
            return;
        }
        let class = self.class_name(c);
        let u = self.vc_underlying(c);
        let funs: Vec<crate::tir::FunId> = cx.input.prog.classes[cx.tclass_of[c.idx()] as usize].methods.clone();
        for f in funs {
            let fun = &cx.input.prog.funs[f.idx()];
            if !cx.input.reach.funs[f.idx()] || fun.body.is_none() {
                continue;
            }
            let m = self.mref(fun.sym);
            let mut types = vec![u.clone()];
            types.extend(m.params.iter().cloned());
            let name = format!("{}$extension", m.name);
            let desc = method_desc(&types, &m.ret);
            if self.cw.has_method(&name, &desc) {
                continue;
            }
            let params: Vec<(Option<SymId>, JType)> = types.iter().map(|t| (None, t.clone())).collect();
            self.begin_method(false, false, &params, m.ret.clone(), &[]);
            let mut slot = 1u16;
            for t in &types {
                self.load(slot, t);
                slot += if t.wide() { 2 } else { 1 };
            }
            self.invoke_desc(Invoke::Static, &class, false, &name, &desc, types.len(), &m.ret);
            self.return_value(&m.ret);
            self.end_method(ACC_PUBLIC, &name, &desc);
        }
    }

    fn extension_bodies(&mut self, c: ClassId) {
        let cx = self.cx;
        let u = self.vc_underlying(c);
        // The lifted methods so far are the module's own; those of the bodies below read `this`
        // as the box, the class they were written in.
        self.drain_pending();
        let saved = self.this_class.replace(c);
        let funs: Vec<crate::tir::FunId> = cx.input.prog.classes[cx.tclass_of[c.idx()] as usize].methods.clone();
        for f in funs {
            let fun = &cx.input.prog.funs[f.idx()];
            let Some(body) = fun.body.filter(|_| cx.input.reach.funs[f.idx()]) else { continue };
            let m = self.mref(fun.sym);
            let info = cx.input.syms.sym(fun.sym);
            self.context = format!("{}.{}$extension", self.this_name, m.name);
            let mut params: Vec<(Option<SymId>, JType)> = vec![(None, u.clone())];
            params.extend(fun.params.iter().zip(&m.params).map(|(&s, t)| (Some(s), t.clone())));
            let types: Vec<JType> = params.iter().map(|p| p.1.clone()).collect();
            self.begin_method(false, false, &params, m.ret.clone(), &[body]);
            self.m.this_slot = Some(1);
            self.m.this_underlying = Some(u.clone());
            self.code.line(cx.line_of(info.file, info.span.start));
            self.method_body(fun.sym, &fun.params, body);
            self.end_method(ACC_PUBLIC | ACC_FINAL, &format!("{}$extension", m.name), &method_desc(&types, &m.ret));
            for (i, d) in fun.defaults.iter().enumerate() {
                let Some(d) = *d else { continue };
                let shape = self.default_getter_shape(fun.sym, i);
                let (before, ret) = (shape.before, shape.ret.clone());
                let getter = format!("{}$default${}$extension", self.source_method_name(fun.sym), shape.index);
                self.begin_method(false, false, &params[..before + 1], ret.clone(), &[d]);
                self.m.this_slot = Some(1);
                self.m.this_underlying = Some(u.clone());
                self.default_value(d, shape.by_name, &ret);
                self.return_value(&ret);
                self.end_method(ACC_PUBLIC | ACC_FINAL, &getter, &method_desc(&types[..before + 1], &ret));
            }
        }
        // The members scalac synthesizes, `hashCode$extension(u)`, `equals$extension(u, that)` and
        // a case class's product members (`value_class_members`).
        for (member, params, ret) in self.vc_synthesized(c) {
            let mut types = vec![u.clone()];
            types.extend(params.iter().cloned());
            let typed: Vec<(Option<SymId>, JType)> = types.iter().map(|t| (None, t.clone())).collect();
            self.context = format!("{}.{}$extension", self.this_name, member);
            self.begin_method(false, false, &typed, ret.clone(), &[]);
            let ps = if u.wide() { 3 } else { 2 };
            self.vc_synthesized_body(c, member, &ret, &u, 1, ps);
            self.end_method(ACC_PUBLIC | ACC_FINAL, &format!("{}$extension", member), &method_desc(&types, &ret));
        }
        if cx.is_case_class(c) {
            let x = cx.input.syms.class(c).ctor.first().and_then(|cl| cl.params.first()).map(|p| self.sym_type(p.sym)).unwrap_or_else(|| u.clone());
            let slot_x = if u.wide() { 3 } else { 2 };
            for (name, params, slot) in [("copy$extension", vec![u.clone(), x.clone()], slot_x), ("copy$default$1$extension", vec![u.clone()], 1), ("_1$extension", vec![u.clone()], 1)] {
                let typed: Vec<(Option<SymId>, JType)> = params.iter().map(|t| (None, t.clone())).collect();
                self.begin_method(false, false, &typed, u.clone(), &[]);
                self.load(slot, &u);
                self.return_value(&u);
                self.end_method(ACC_PUBLIC | ACC_FINAL, name, &method_desc(&params, &u));
            }
        }
        self.drain_pending();
        self.this_class = saved;
    }

    /// The module class of class `c`'s companion written by the backend, initialised as a
    /// written object is: `<clinit>` makes the instance and stores it.
    fn begin_module(&mut self, c: ClassId, interfaces: &[String]) -> Rc<str> {
        let cx = self.cx;
        let name: Rc<str> = Rc::from(cx.companion_name(c));
        self.context = name.to_string();
        self.cw = ClassWriter::new(ACC_PUBLIC | ACC_FINAL | ACC_SUPER, &name, OBJECT, interfaces);
        self.this_name = name.clone();
        self.this_class = None;
        self.is_interface = false;
        self.abstract_class = false;
        let info = cx.input.syms.class(c);
        let path = &cx.input.sources[info.file.0 as usize].path;
        self.cw.source_file(path.rsplit(std::path::is_separator).next().unwrap_or(path));
        self.line_file = info.file;
        let this = JType::L(name.clone());
        self.cw.field(MODULE_FIELD, "MODULE$", &format!("L{};", name));

        self.begin_module_clinit(&name, &[]);
        let enum_values = if info.kind == ClassKind::Enum { self.enum_value_fields(c) } else { Vec::new() };
        if !enum_values.is_empty() && !cx.has_enum_fields(c) {
            let enum_name = self.class_name(c);
            self.invoke(Invoke::Static, &enum_name, false, "$touch", &[], &JType::V);
        }
        self.store_enum_fields(&name, &enum_values);
        self.return_value(&JType::V);
        self.end_method(ACC_STATIC, "<clinit>", "()V");

        self.begin_method(false, true, &[], JType::V, &[]);
        self.load(0, &this);
        self.invoke(Invoke::Special, OBJECT, false, "<init>", &[], &JType::V);
        self.return_value(&JType::V);
        self.end_method(ACC_PRIVATE, "<init>", "()V");
        name
    }

    /// The members scalac gives the companion of case class `c`, in the module class being
    /// written; `object` is the written companion, whose own definitions of the same name and
    /// parameters win.
    pub fn companion_members(&mut self, c: ClassId, object: Option<ClassId>) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let tc = &cx.input.prog.classes[cx.tclass_of[c.idx()] as usize];
        let own = self.cw.method_table();
        let written = |name: &str, params: &[JType]| {
            let prefix = method_desc(params, &JType::V);
            let prefix = &prefix[..prefix.len() - 1];
            own.iter().any(|(_, n, d)| n == name && d.starts_with(prefix))
        };
        let case_name = self.class_name(c);
        let case_type = JType::L(case_name.clone());
        let object_type = JType::object();
        let product = JType::L(Rc::from(PRODUCT));
        let params: Vec<(Option<SymId>, JType)> = tc.ctor_params.iter().map(|&p| (Some(p), self.sym_type(p))).collect();
        let types: Vec<JType> = params.iter().map(|p| p.1.clone()).collect();

        // A value case class's are scalac's erased ones, over the underlying value.
        if cx.input.syms.class(c).value_class {
            let u = self.vc_underlying(c);
            for name in ["apply", "unapply"] {
                if !written(name, &[u.clone()]) {
                    self.begin_method(false, false, &[(None, u.clone())], u.clone(), &[]);
                    self.load(1, &u);
                    self.return_value(&u);
                    self.end_method(ACC_PUBLIC, name, &method_desc(&[u.clone()], &u));
                }
            }
        }
        if !written("apply", &types) && !cx.input.syms.class(c).value_class {
            self.begin_method(false, false, &params, case_type.clone(), &[]);
            self.new_object(&case_name);
            let mut slot = 1u16;
            for t in &types {
                self.load(slot, t);
                slot += if t.wide() { 2 } else { 1 };
            }
            self.invoke(Invoke::Special, &case_name, false, "<init>", &types, &JType::V);
            self.return_value(&case_type);
            self.end_method(ACC_PUBLIC, "apply", &method_desc(&types, &case_type));
        }

        // Scala 3's extractor is the argument itself; a class without parameters answers a
        // Boolean, and one whose last parameter is repeated is matched through `unapplySeq`.
        let first = syms.class(c).info.ctor.first();
        let repeated = first.and_then(|cl| cl.params.last()).map_or(false, |p| p.repeated);
        let extractor = if repeated { "unapplySeq" } else { "unapply" };
        let unapply_params = [case_type.clone()];
        if !written(extractor, &unapply_params) && !cx.input.syms.class(c).value_class {
            let no_params = first.map_or(true, |cl| cl.params.is_empty());
            let ret = if no_params { JType::Z } else { case_type.clone() };
            self.begin_method(false, false, &[(None, case_type.clone())], ret.clone(), &[]);
            if no_params {
                self.iconst(1);
            } else {
                self.load(1, &case_type);
            }
            self.return_value(&ret);
            self.end_method(ACC_PUBLIC, extractor, &method_desc(&unapply_params, &ret));
        }

        if cx.has_product_mirror(c) && !written("fromProduct", &[product.clone()]) {
            self.begin_method(false, false, &[(None, product.clone())], case_type.clone(), &[]);
            self.new_object(&case_name);
            for (i, t) in types.iter().enumerate() {
                self.load(1, &product);
                self.iconst(i as i32);
                self.invoke(Invoke::Interface, PRODUCT, true, "productElement", &[JType::I], &object_type);
                self.adapt(&object_type, t);
            }
            self.invoke(Invoke::Special, &case_name, false, "<init>", &types, &JType::V);
            self.return_value(&case_type);
            let desc = method_desc(&[product.clone()], &case_type);
            self.end_method(ACC_PUBLIC, "fromProduct", &desc);

            self.begin_method(false, false, &[(None, product.clone())], object_type.clone(), &[]);
            self.load(0, &JType::L(self.this_name.clone()));
            self.load(1, &product);
            let owner = self.this_name.clone();
            self.invoke_desc(Invoke::Virtual, &owner, false, "fromProduct", &desc, 1, &case_type);
            self.return_value(&object_type);
            self.end_method(ACC_PUBLIC | ACC_SYNTHETIC | ACC_BRIDGE, "fromProduct", &method_desc(&[product], &object_type));
        }

        self.extension_copies(c);
        if object.is_none() {
            let string = JType::L(Rc::from(STRING));
            self.begin_method(false, false, &[], string.clone(), &[]);
            self.sconst(cx.input.interner.get(syms.class(c).name));
            self.return_value(&string);
            self.end_method(ACC_PUBLIC, "toString", "()Ljava/lang/String;");
        }

        self.ctor_default_getters_of(c);
    }

    /// In link mode, the values of program enum `e` as scalac's companion holds them in public
    /// static fields: the field names, with the value symbols.
    pub fn enum_value_fields(&mut self, e: ClassId) -> Vec<(String, SymId)> {
        let cx = self.cx;
        let syms = cx.input.syms;
        if syms.class(e).kind != ClassKind::Enum {
            return Vec::new();
        }
        syms.class(e)
            .children
            .iter()
            .filter(|&&k| cx.input.reach.classes[k.idx()])
            .filter_map(|&k| syms.class(k).singleton.map(|v| (encode(cx.input.interner.get(syms.class(k).name)), v)))
            .collect()
    }

    /// Declares the static fields of an enum's values in the module class being written and
    /// stores them, in its `<clinit>`, before the companion's body runs, which may read them. An
    /// enum read through those fields (`has_enum_fields`) has its values made here, in their
    /// order, each stored in its field as soon as it is made, as scalac's companion makes them:
    /// a value's constructor may read an earlier value through its field (`case B extends
    /// E(E.A)`); each is its case's instance too (`$instance`). Another enum's values are read
    /// from their cases' instances, once the enum made them all (the caller's `$touch`). An
    /// enum whose every case is a value has scalac's `$values` too, the array `values` copies.
    pub fn store_enum_fields(&mut self, module: &str, values: &[(String, SymId)]) {
        let Some(&(_, first)) = values.first() else { return };
        let Owner::Class(o) = self.cx.input.syms.sym(first).owner else { return };
        let Some(e) = self.cx.input.syms.class(o).companion else { return };
        let enum_type = self.class_type(e);
        let desc = self.desc_of(&enum_type);
        let enum_name = self.class_name(e);
        let makes = self.cx.has_enum_fields(e);
        let string = JType::L(Rc::from(STRING));
        for (name, v) in values {
            self.cw.field(ACC_PUBLIC | ACC_STATIC | ACC_FINAL | ACC_ENUM, name, &desc);
            let SymKind::EnumValue(case) = self.cx.input.syms.sym(*v).kind else { continue };
            let case_name = self.class_name(case);
            let t = JType::L(case_name.clone());
            if makes {
                let cinfo = self.cx.input.syms.class(case);
                self.new_object(&case_name);
                self.sconst(self.cx.input.interner.get(cinfo.name));
                self.iconst(cinfo.ordinal as i32);
                self.invoke(Invoke::Special, &case_name, false, "<init>", &[string.clone(), JType::I], &JType::V);
                self.dup();
                self.putstatic(&case_name, "$instance", &t);
            } else {
                self.getstatic(&case_name, "$instance", &t);
            }
            self.adapt(&t, &enum_type);
            self.putstatic(module, name, &enum_type);
        }
        if values.len() == self.cx.input.syms.class(e).children.len() {
            let array = JType::L(Rc::from(format!("[{}", desc)));
            self.cw.field(ACC_PRIVATE | ACC_STATIC | ACC_FINAL, "$values", &format!("[{}", desc));
            self.iconst(values.len() as i32);
            let class = self.cw.cp.class(&enum_name);
            self.code.op_u16(op::ANEWARRAY, class);
            self.code.pop();
            let vt = self.vt(&array);
            self.code.push(vt);
            for (i, (name, _)) in values.iter().enumerate() {
                self.dup();
                self.iconst(i as i32);
                self.getstatic(module, name, &enum_type);
                self.code.op(op::AASTORE);
                self.code.popn(3);
            }
            self.putstatic(module, "$values", &array);
        }
    }

    /// scalac's `Mirror.Sum` of an enum's companion: `ordinal(E)I`, the value's ordinal, and its
    /// bridge `ordinal(Object)I`, in the module class being written; `object` is the written
    /// companion, whose own `ordinal` wins.
    fn enum_mirror_members(&mut self, e: ClassId, object: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        if cx.input.interner.lookup("ordinal").map_or(false, |n| syms.class(object).members.contains_key(&n)) {
            return;
        }
        let enum_type = self.class_type(e);
        let enum_name = self.class_name(e);
        let object_type = JType::object();
        let desc = method_desc(&[enum_type.clone()], &JType::I);
        self.begin_method(false, false, &[(None, enum_type.clone())], JType::I, &[]);
        self.load(1, &enum_type);
        if cx.is_java_enum(e) {
            self.invoke(Invoke::Virtual, &enum_name, false, "ordinal", &[], &JType::I);
        } else {
            self.invoke(Invoke::Interface, ENUM, true, "ordinal", &[], &JType::I);
        }
        self.return_value(&JType::I);
        self.end_method(ACC_PUBLIC, "ordinal", &desc);
        self.begin_method(false, false, &[(None, object_type.clone())], JType::I, &[]);
        let this = JType::L(self.this_name.clone());
        self.load(0, &this);
        self.load(1, &object_type);
        self.adapt(&object_type, &enum_type);
        let owner = self.this_name.clone();
        self.invoke_desc(Invoke::Virtual, &owner, false, "ordinal", &desc, 1, &JType::I);
        self.return_value(&JType::I);
        self.end_method(ACC_PUBLIC | ACC_BRIDGE | ACC_SYNTHETIC, "ordinal", &method_desc(&[object_type], &JType::I));
    }

    /// scalac's `fromOrdinal` of an enum's companion, and `values` and `valueOf` where every case
    /// is a value; `object` is the companion being written, whose own definitions win.
    pub fn enum_companion_members(&mut self, e: ClassId, object: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let written = |name: &str| cx.input.interner.lookup(name).map_or(false, |n| syms.class(object).members.contains_key(&n));
        let enum_type = self.class_type(e);
        let string = JType::L(Rc::from(STRING));
        let display = cx.input.interner.get(syms.class(e).name).to_string();
        let values: Vec<(SymId, u32, String)> = syms
            .class(e)
            .children
            .iter()
            .filter(|&&k| cx.input.reach.classes[k.idx()])
            .filter_map(|&k| syms.class(k).singleton.map(|v| (v, syms.class(k).ordinal, cx.input.interner.get(syms.class(k).name).to_string())))
            .collect();
        let all_values = values.len() == syms.class(e).children.len();
        let object_type = JType::object();
        if cx.has_enum_fields(e) {
            self.enum_mirror_members(e, object);
        }

        if !written("fromOrdinal") {
            self.begin_method(false, false, &[(None, JType::I)], enum_type.clone(), &[]);
            for (v, ordinal, _) in &values {
                let next = self.code.new_label();
                self.load(1, &JType::I);
                self.iconst(*ordinal as i32);
                self.jump_if(op::IF_ICMPNE, 2, next);
                let t = self.static_value(*v);
                self.adapt(&t, &enum_type);
                self.return_value(&enum_type);
                self.code.bind(next);
            }
            self.load(1, &JType::I);
            self.invoke(Invoke::Static, STRING, false, "valueOf", &[JType::I], &string);
            self.throw_with("java/util/NoSuchElementException", &format!("enum {} has no case with ordinal: ", display));
            self.end_method(ACC_PUBLIC, "fromOrdinal", &method_desc(&[JType::I], &enum_type));
        }
        if !all_values {
            return;
        }
        if !written("values") && cx.has_enum_fields(e) && values.len() == syms.class(e).children.len() {
            let array = JType::L(Rc::from(format!("[{}", self.desc_of(&enum_type))));
            let JType::L(array_name) = &array else { unreachable!() };
            let array_name = array_name.clone();
            self.begin_method(false, false, &[], array.clone(), &[]);
            let module = self.this_name.clone();
            self.getstatic(&module, "$values", &array);
            self.invoke(Invoke::Virtual, &array_name, false, "clone", &[], &object_type);
            self.adapt(&object_type, &array);
            self.return_value(&array);
            self.end_method(ACC_PUBLIC, "values", &method_desc(&[], &array));
        }
        if !written("values") {
            let array = JType::L(Rc::from(format!("[{}", self.desc_of(&enum_type))));
            self.begin_method(false, false, &[], array.clone(), &[]);
            self.iconst(values.len() as i32);
            let element = self.class_name(e);
            let class = self.cw.cp.class(&element);
            self.code.op_u16(op::ANEWARRAY, class);
            self.code.pop();
            let vt = self.vt(&array);
            self.code.push(vt);
            for (i, (v, _, _)) in values.iter().enumerate() {
                self.dup();
                self.iconst(i as i32);
                let t = self.static_value(*v);
                self.adapt(&t, &enum_type);
                self.code.op(op::AASTORE);
                self.code.popn(3);
            }
            self.return_value(&array);
            self.end_method(ACC_PUBLIC, "values", &method_desc(&[], &array));
        }
        if !written("valueOf") {
            self.begin_method(false, false, &[(None, string.clone())], enum_type.clone(), &[]);
            for (v, _, name) in &values {
                let next = self.code.new_label();
                self.sconst(name);
                self.load(1, &string);
                self.invoke(Invoke::Virtual, STRING, false, "equals", &[JType::object()], &JType::Z);
                self.jump_if(op::IFEQ, 1, next);
                let t = self.static_value(*v);
                self.adapt(&t, &enum_type);
                self.return_value(&enum_type);
                self.code.bind(next);
            }
            self.load(1, &string);
            self.throw_with("java/lang/IllegalArgumentException", &format!("enum {} has no case with name: ", display));
            self.end_method(ACC_PUBLIC, "valueOf", &method_desc(&[string], &enum_type));
        }
    }

    /// Throws a new `class` whose message is `prefix` and the string on the stack.
    fn throw_with(&mut self, class: &str, prefix: &str) {
        let string = JType::L(Rc::from(STRING));
        let rest = self.store_new(&string);
        self.new_object(class);
        self.sconst(prefix);
        self.load(rest, &string);
        self.invoke(Invoke::Virtual, STRING, false, "concat", &[string.clone()], &string);
        self.invoke(Invoke::Special, class, false, "<init>", &[string], &JType::V);
        self.code.op(op::ATHROW);
        self.code.pop();
        self.code.end_path();
    }

    fn desc_of(&self, t: &JType) -> String {
        let mut d = String::new();
        t.desc(&mut d);
        d
    }

    /// `copy` and its defaults, the fields of the instance, in case class `c`.
    pub fn copy_members(&mut self, c: ClassId) {
        let cx = self.cx;
        let tc = &cx.input.prog.classes[cx.tclass_of[c.idx()] as usize];
        let owner = self.this_name.clone();
        let this = JType::L(owner.clone());
        let params: Vec<(Option<SymId>, JType)> = tc.ctor_params.iter().map(|&p| (Some(p), self.sym_type(p))).collect();
        let types: Vec<JType> = params.iter().map(|p| p.1.clone()).collect();
        let prefix = method_desc(&types, &JType::V);
        let prefix = &prefix[..prefix.len() - 1];
        // A value case class's `copy` answers the underlying value, as scalac's erasure has it.
        if cx.input.syms.class(c).value_class && types.len() == 1 {
            let u = types[0].clone();
            if !self.cw.method_table().iter().any(|(_, n, d)| n == "copy" && d.starts_with(prefix)) {
                self.begin_method(false, false, &params, u.clone(), &[]);
                self.load(1, &u);
                self.return_value(&u);
                self.end_method(ACC_PUBLIC, "copy", &method_desc(&types, &u));
            }
        } else if !self.cw.method_table().iter().any(|(_, n, d)| n == "copy" && d.starts_with(prefix)) {
            self.begin_method(false, false, &params, this.clone(), &[]);
            self.new_object(&owner);
            let mut slot = 1u16;
            for t in &types {
                self.load(slot, t);
                slot += if t.wide() { 2 } else { 1 };
            }
            self.invoke(Invoke::Special, &owner, false, "<init>", &types, &JType::V);
            self.return_value(&this);
            self.end_method(ACC_PUBLIC, "copy", &method_desc(&types, &this));
        }
        let first = cx.input.syms.class(c).ctor.first().map_or(0, |cl| cl.params.len());
        for (i, &p) in tc.ctor_params.iter().enumerate().take(first) {
            let t = types[i].clone();
            let field = self.field_name(p);
            self.begin_method(false, false, &[], t.clone(), &[]);
            self.load(0, &this);
            self.getfield(&owner, &field, &t);
            self.return_value(&t);
            self.end_method(ACC_PUBLIC, &format!("copy$default${}", i + 1), &method_desc(&[], &t));
        }
    }

    /// The selectors `_1` .. `_N` of case class `c`, scalac's `productElemMeths`, which its
    /// pattern match on the class calls: one per parameter of the first clause, of the
    /// parameter's erased type, reading the parameter's accessor, unless the parameter is itself
    /// named so or the class defines a member of that name.
    pub fn selector_members(&mut self, c: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let params: Vec<SymId> = syms.class(c).ctor.first().map_or(Vec::new(), |cl| cl.params.iter().map(|p| p.sym).collect());
        let owner = self.this_name.clone();
        let this = JType::L(owner.clone());
        for (i, &p) in params.iter().enumerate() {
            let name = format!("_{}", i + 1);
            let named = cx.input.interner.lookup(&name);
            let defined = named.map_or(false, |n| params.iter().any(|&q| syms.sym(q).name == n) || syms.class(c).members.get(&n).map_or(false, |&m| syms.sym(m).owner == Owner::Class(c)));
            if defined {
                continue;
            }
            let ty = self.sym_type(p);
            let accessor = self.field_name(p);
            self.begin_method(false, false, &[], ty.clone(), &[]);
            self.load(0, &this);
            self.invoke(Invoke::Virtual, &owner, false, &accessor, &[], &ty);
            self.return_value(&ty);
            self.end_method(ACC_PUBLIC, &name, &method_desc(&[], &ty));
        }
    }

    /// The defs scalac writes for the given classes of the class, object or file being written
    /// (`Cx::given_classes`): `listShow(Show)Show$listShow`, public and final, making the class
    /// of its arguments, which teq's own calls make directly; one of an instance makes it of
    /// `this` first, a trait's is a default method with its static `listShow$`.
    pub fn given_class_defs(&mut self, owner: Owner, file: Option<crate::source::FileId>) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let mut defs: Vec<(SymId, ClassId)> = cx
            .given_classes
            .iter()
            .filter(|&(&g, &k)| syms.sym(g).owner == owner && file.map_or(true, |f| syms.sym(g).file == f) && cx.emits(k))
            .map(|(&g, &k)| (g, k))
            .collect();
        defs.sort_by(|a, b| crate::emit::layout::compare_syms(syms, cx.input.interner, a.0, b.0));
        for (g, k) in defs {
            let m = self.mref(g);
            let class = self.class_name(k);
            let outer = self.anon_ctor_params(k);
            let mut ctor = outer.clone();
            ctor.extend(self.ctor_param_types(k));
            let typed: Vec<(Option<SymId>, JType)> = m.params.iter().map(|t| (None, t.clone())).collect();
            self.begin_method(false, false, &typed, m.ret.clone(), &[]);
            self.new_object(&class);
            if let Some(t) = outer.first() {
                let this = JType::L(self.this_name.clone());
                self.load(0, &this);
                self.adapt(&this, t);
            }
            let mut slot = 1u16;
            for (from, to) in m.params.iter().zip(&ctor[outer.len()..]) {
                self.load(slot, from);
                self.adapt(from, to);
                slot += if from.wide() { 2 } else { 1 };
            }
            self.invoke(Invoke::Special, &class, false, "<init>", &ctor, &JType::V);
            self.return_value(&m.ret);
            if self.is_interface {
                self.end_method(ACC_PUBLIC, &m.name, &m.desc);
                self.trait_static_forwarder(&m.name, &m.params, &m.ret);
            } else {
                self.end_method(ACC_PUBLIC | ACC_FINAL, &m.name, &m.desc);
            }
        }
    }

    /// scalac's `Mirror.Singleton` of a case object, in its module class being written:
    /// `fromProduct(Product)` answering the object, and the bridge to it.
    pub fn singleton_mirror_members(&mut self) {
        let this = JType::L(self.this_name.clone());
        let product = JType::L(Rc::from(PRODUCT));
        let singleton = JType::L(Rc::from(MIRROR_SINGLETON));
        let object = JType::object();
        let desc = method_desc(&[product.clone()], &singleton);
        self.begin_method(false, false, &[(None, product.clone())], singleton.clone(), &[]);
        self.load(0, &this);
        self.adapt(&this, &singleton);
        self.return_value(&singleton);
        self.end_method(ACC_PUBLIC, "fromProduct", &desc);
        self.begin_method(false, false, &[(None, product.clone())], object.clone(), &[]);
        self.load(0, &this);
        self.load(1, &product);
        let owner = self.this_name.clone();
        self.invoke_desc(Invoke::Virtual, &owner, false, "fromProduct", &desc, 1, &singleton);
        self.return_value(&object);
        self.end_method(ACC_PUBLIC | ACC_BRIDGE | ACC_SYNTHETIC, "fromProduct", &method_desc(&[product], &object));
    }

    /// The forwarders of the export clauses of the class or file being written, scalac's
    /// `final def n(ps): R = q.m(ps)` as methods: an exported object's, `n()` answering its
    /// module; a member's, of its signature as the qualifier sees it, calling the member on the
    /// qualifier, or the qualifier's own forwarder of it where the qualifier exports it (a
    /// relay); with the forwarders of its defaults' getters, `n$default$i`, and the bridges an
    /// inherited member of the name needs. Public and final, a trait's a default method with its
    /// static `n$`. teq's own references go to the original (`typer::exports`).
    pub fn export_forwarders(&mut self, list: &[crate::typer::export_plan::JvmForwarder]) {
        use crate::typer::export_plan::JvmTarget;
        use crate::typer::exports::ExportQualifier;
        let cx = self.cx;
        let syms = cx.input.syms;
        let access = if self.is_interface { ACC_PUBLIC } else { ACC_PUBLIC | ACC_FINAL };
        for f in list {
            let name = encode(cx.input.interner.get(f.name));
            // A given object's forwarder answers its module, as an object's does.
            let target = match f.target {
                JvmTarget::Member(s) if cx.given_objects.contains_key(&s) => JvmTarget::Object(cx.given_objects[&s]),
                t => t,
            };
            let s = match target {
                JvmTarget::Object(k) => {
                    // An object's module, a case class's companion's for a case class.
                    let module: Rc<str> = match syms.class(k).kind {
                        ClassKind::Object | ClassKind::GivenImpl => self.class_name(k),
                        _ => Rc::from(cx.companion_name(k)),
                    };
                    let t = JType::L(module.clone());
                    self.begin_method(false, false, &[], t.clone(), &[]);
                    self.getstatic(&module, "MODULE$", &t);
                    self.return_value(&t);
                    self.end_method(access, &name, &method_desc(&[], &t));
                    if self.is_interface {
                        self.trait_static_forwarder(&name, &[], &t);
                    }
                    continue;
                }
                JvmTarget::Member(s) => s,
            };
            let Some(sig) = f.sig.clone() else { continue };
            let (params, ret) = self.erase_export_sig(s, &sig);
            let m = self.mref(s);
            // The member's `@targetName` is the forwarder's name too, renamed or not, as
            // scalac's; the getters' forwarders keep the forwarder's source name.
            let source = name.clone();
            let name = self.target_name(s).unwrap_or(name);
            // What the forwarder calls: the member, or the qualifier's forwarder of it.
            let (owner, kind, iface, callee, desc, callee_params, callee_ret): (Rc<str>, Invoke, bool, String, String, Vec<JType>, JType) = match (f.relayed, f.q) {
                (true, ExportQualifier::Object(o)) => {
                    let qn = self.class_name(o);
                    let selected = self.target_name(s).unwrap_or_else(|| encode(cx.input.interner.get(f.selected)));
                    (qn, Invoke::Virtual, false, selected, method_desc(&params, &ret), params.clone(), ret.clone())
                }
                _ => (m.owner.clone(), m.kind, m.owner_is_interface, m.name.clone(), m.desc.clone(), m.params.clone(), m.ret.clone()),
            };
            let typed: Vec<(Option<SymId>, JType)> = params.iter().map(|t| (None, t.clone())).collect();
            self.begin_method(false, false, &typed, ret.clone(), &[]);
            if kind != Invoke::Static {
                self.export_receiver(f, s);
            }
            let mut slot = 1u16;
            for (from, to) in params.iter().zip(&callee_params) {
                self.load(slot, from);
                self.adapt(from, to);
                slot += if from.wide() { 2 } else { 1 };
            }
            self.invoke_desc(kind, &owner, iface, &callee, &desc, callee_params.len(), &callee_ret);
            self.adapt(&callee_ret, &ret);
            self.return_value(&ret);
            let fdesc = method_desc(&params, &ret);
            self.end_method(access, &name, &fdesc);
            if self.is_interface {
                self.trait_static_forwarder(&name, &params, &ret);
            }
            let fm = MRef { owner: self.this_name.clone(), owner_is_interface: self.is_interface, name: name.clone(), params: params.clone(), ret: ret.clone(), desc: fdesc, kind: if self.is_interface { Invoke::Interface } else { Invoke::Virtual } };
            self.emit_bridges_as_pub(s, f.name, &fm);
            // The forwarders of the defaults' getters, after the method's, as scalac writes them.
            if syms.sym(s).kind != SymKind::Def || self.jvm_param_order(s).is_some() {
                continue;
            }
            let mut start = 0usize;
            for cl in &sig.clauses {
                for (k, p) in cl.params.iter().enumerate() {
                    let i = start + k;
                    if !p.has_default {
                        continue;
                    }
                    let gret = if p.by_name { self.erase(p.ty) } else { params[i].clone() };
                    let gparams = params[..start].to_vec();
                    let (gowner, gkind, giface, gname, gdesc) = match (f.relayed, f.q) {
                        (true, ExportQualifier::Object(o)) => {
                            let qn = self.class_name(o);
                            let selected = encode(cx.input.interner.get(f.selected));
                            (qn, Invoke::Virtual, false, format!("{}$default${}", selected, i + 1), method_desc(&gparams, &gret))
                        }
                        _ => {
                            let (n, d) = self.default_getter_ref(s, i, &gparams);
                            (m.owner.clone(), m.kind, m.owner_is_interface, n, d)
                        }
                    };
                    let (cparams, cret) = parse_method_desc(&gdesc);
                    let typed: Vec<(Option<SymId>, JType)> = gparams.iter().map(|t| (None, t.clone())).collect();
                    self.begin_method(false, false, &typed, gret.clone(), &[]);
                    if gkind != Invoke::Static {
                        self.export_receiver(f, s);
                    }
                    let mut slot = 1u16;
                    for (from, to) in gparams.iter().zip(&cparams) {
                        self.load(slot, from);
                        self.adapt(from, to);
                        slot += if from.wide() { 2 } else { 1 };
                    }
                    self.invoke_desc(gkind, &gowner, giface, &gname, &gdesc, cparams.len(), &cret);
                    self.adapt(&cret, &gret);
                    self.return_value(&gret);
                    let fname = format!("{}$default${}", source, i + 1);
                    self.end_method(access, &fname, &method_desc(&gparams, &gret));
                    if self.is_interface {
                        self.trait_static_forwarder(&fname, &gparams, &gret);
                    }
                }
                start += cl.params.len();
            }
        }
    }

    /// The bridges class `c` needs for the export forwarders of a trait it newly mixes in: a
    /// forwarder implementing another parent's member under another erasure (`f(String)` of
    /// `trait Parent { export Impl.f }` for `API[String].f(Object)`) is bridged in the class, as
    /// scalac's mixin bridges it; a superclass's forwarders were bridged in the superclass.
    pub fn inherited_export_bridges(&mut self, c: ClassId) {
        use crate::typer::export_plan::JvmTarget;
        let cx = self.cx;
        let syms = cx.input.syms;
        let above: Vec<ClassId> = syms.class(c).superclass.map_or(Vec::new(), |s| syms.class(s).base_types.iter().map(|&(b, _)| b).collect());
        let traits: Vec<ClassId> = syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).filter(|b| syms.class(*b).kind == ClassKind::Trait && !above.contains(b)).collect();
        for b in traits {
            let Some(list) = cx.input.reach.exports.classes.get(&b) else { continue };
            for f in list {
                let JvmTarget::Member(s) = f.target else { continue };
                if cx.given_objects.contains_key(&s) {
                    continue;
                }
                let Some(sig) = f.sig.clone() else { continue };
                let (params, ret) = self.erase_export_sig(s, &sig);
                let desc = method_desc(&params, &ret);
                let name = self.target_name(s).unwrap_or_else(|| encode(cx.input.interner.get(f.name)));
                let fm = MRef { owner: self.this_name.clone(), owner_is_interface: false, name, params, ret, desc, kind: Invoke::Virtual };
                self.emit_bridges_as_pub(s, f.name, &fm);
            }
        }
    }

    /// The names and descriptors of the methods `export_forwarders` writes for `list`.
    fn export_forwarder_descs(&mut self, list: &[crate::typer::export_plan::JvmForwarder]) -> Vec<(String, String)> {
        use crate::typer::export_plan::JvmTarget;
        let cx = self.cx;
        let mut out = Vec::new();
        for f in list {
            let name = encode(cx.input.interner.get(f.name));
            let target = match f.target {
                JvmTarget::Member(s) if cx.given_objects.contains_key(&s) => JvmTarget::Object(cx.given_objects[&s]),
                t => t,
            };
            let s = match target {
                JvmTarget::Object(k) => {
                    let module: Rc<str> = match cx.input.syms.class(k).kind {
                        ClassKind::Object | ClassKind::GivenImpl => self.class_name(k),
                        _ => Rc::from(cx.companion_name(k)),
                    };
                    out.push((name, method_desc(&[], &JType::L(module))));
                    continue;
                }
                JvmTarget::Member(s) => s,
            };
            let Some(sig) = f.sig.clone() else { continue };
            let (params, ret) = self.erase_export_sig(s, &sig);
            out.push((self.target_name(s).unwrap_or_else(|| name.clone()), method_desc(&params, &ret)));
            if cx.input.syms.sym(s).kind != SymKind::Def || self.jvm_param_order(s).is_some() {
                continue;
            }
            let mut start = 0usize;
            for cl in &sig.clauses {
                for (k, p) in cl.params.iter().enumerate() {
                    let i = start + k;
                    if p.has_default {
                        let gret = if p.by_name { self.erase(p.ty) } else { params[i].clone() };
                        out.push((format!("{}$default${}", name, i + 1), method_desc(&params[..start], &gret)));
                    }
                }
                start += cl.params.len();
            }
        }
        out
    }

    /// The receiver of what export forwarder `f` calls of member `s`: the qualifier's module, or
    /// for a package's member the object of its file or class.
    fn export_receiver(&mut self, f: &crate::typer::export_plan::JvmForwarder, s: SymId) {
        use crate::typer::exports::ExportQualifier;
        let cx = self.cx;
        match (f.q, cx.input.syms.sym(s).owner) {
            (ExportQualifier::Object(o), _) => {
                let name = self.class_name(o);
                self.getstatic(&name, "MODULE$", &JType::L(name.clone()));
            }
            (ExportQualifier::Package(_), Owner::Class(c)) => {
                let name = self.class_name(c);
                self.getstatic(&name, "MODULE$", &JType::L(name.clone()));
            }
            (ExportQualifier::Package(_), _) => {
                self.load_file_module(cx.input.syms.sym(s).file);
            }
        }
    }

    /// The erased parameters and result of an export forwarder of member `s` whose signature the
    /// qualifier sees as `sig`: the member's own where nothing of it was substituted (a jar's as
    /// its class file has it), else the erasure of the substituted types.
    fn erase_export_sig(&mut self, s: SymId, sig: &MethodSig) -> (Vec<JType>, JType) {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        let same = info.sig.as_ref().map_or(false, |o| {
            o.ret == sig.ret && o.clauses.len() == sig.clauses.len() && o.clauses.iter().zip(&sig.clauses).all(|(a, b)| a.params.len() == b.params.len() && a.params.iter().zip(&b.params).all(|(x, y)| x.ty == y.ty))
        });
        let m = self.mref(s);
        if same || m.params.len() != sig.clauses.iter().map(|c| c.params.len()).sum::<usize>() {
            return (m.params.clone(), m.ret.clone());
        }
        let mut params: Vec<JType> = Vec::new();
        for p in sig.clauses.iter().flat_map(|c| c.params.iter()) {
            params.push(if p.by_name || p.repeated { self.sym_type(p.sym) } else { self.storage(p.ty) });
        }
        if let Some(order) = self.jvm_param_order(s) {
            params = order.iter().map(|&k| params[k].clone()).collect();
        }
        let is_value = !matches!(cx.input.syms.sym(s).kind, SymKind::Def) && params.is_empty();
        let ret = if is_value { self.storage(sig.ret) } else { self.erase(sig.ret) };
        (params, ret)
    }

    /// `T$$super$m`, scalac's name of the accessor through which trait `t` calls `super.m`.
    pub fn super_accessor_name(&mut self, t: ClassId, member: SymId) -> String {
        format!("{}$$super${}", self.class_name(t).replace('/', "$"), self.method_name(member))
    }

    /// The accessor through which trait `t` calls `super.m` of `member`, a method of its
    /// interface: the member as the trait sees it (`SuperAccessors`' `superInfo`), a generic
    /// parent's type parameters bound as the trait's base type binds them, erased; what the
    /// trait's body calls, the trait declares and a class that mixes it in implements.
    pub fn super_accessor_ref(&mut self, t: ClassId, member: SymId) -> MRef {
        let cx = self.cx;
        let mut accessor = (*self.mref(member)).clone();
        accessor.name = self.super_accessor_name(t, member);
        accessor.owner = self.class_name(t);
        accessor.owner_is_interface = true;
        accessor.kind = Invoke::Interface;
        let syms = cx.input.syms;
        let (Owner::Class(k), Some(sig)) = (syms.sym(member).owner, syms.sym(member).info.sig.clone()) else { return accessor };
        let base = syms.class(t).base_types.iter().find(|&&(b, _)| b == k).map(|&(_, ty)| ty);
        let Some(Type::Class(_, args)) = base.map(|b| cx.input.types.get(b)) else { return accessor };
        let mut subst: Subst = syms.class(k).tparams.iter().copied().zip(cx.input.types.items(args).iter().copied()).collect();
        if subst.is_empty() {
            return accessor;
        }
        let types = cx.input.types;
        // The method's own type parameters erase as their bounds seen from the trait do
        // (`id[B <: A]` of `Parent[String]` takes a `String`), the pickle's fresh copies'
        // (`member_sig_seen_from`): a parameter whose bound the binding changes stands for it,
        // through a chain of bounds in any order (`f[B <: C, C <: A]`), each round settling
        // one more link of it.
        for _ in 0..=sig.tparams.len() {
            let mut changed = false;
            for &tp in &sig.tparams {
                let upper = syms.tparam(tp).upper;
                let seen = types.subst(upper, &subst);
                if seen == upper {
                    continue;
                }
                match subst.iter_mut().find(|(p, _)| *p == tp) {
                    Some(entry) if entry.1 == seen => {}
                    Some(entry) => {
                        entry.1 = seen;
                        changed = true;
                    }
                    None => {
                        subst.push((tp, seen));
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        let seen = MethodSig {
            tparams: sig.tparams.clone(),
            clauses: sig.clauses.iter().map(|cl| ClauseSig { params: cl.params.iter().map(|p| ParamSig { ty: types.subst(p.ty, &subst), ..p.clone() }).collect(), ..cl.clone() }).collect(),
            ret: types.subst(sig.ret, &subst),
        };
        let (params, ret) = self.erase_export_sig(member, &seen);
        accessor.desc = method_desc(&params, &ret);
        accessor.params = params;
        accessor.ret = ret;
        accessor
    }

    /// The abstract accessors of trait `t`, one per member its `super` calls name.
    pub fn super_accessor_declarations(&mut self, t: ClassId) {
        let members = self.cx.super_called.get(&t).cloned().unwrap_or_default();
        for member in members {
            let m = self.super_accessor_ref(t, member);
            if !self.cw.has_method(&m.name, &m.desc) {
                self.cw.method(ACC_PUBLIC | ACC_ABSTRACT | ACC_SYNTHETIC, &m.name, &m.desc, None);
            }
        }
    }

    /// The super accessors of the traits a class is the first to mix in: `invokespecial` on
    /// the definition that follows the trait in the class's linearisation.
    pub fn super_accessors(&mut self, tc: &crate::tir::TClass) {
        let cx = self.cx;
        let this = JType::L(self.this_name.clone());
        for a in tc.super_accessors.clone() {
            let m = self.super_accessor_ref(a.of_trait, a.member);
            if self.cw.has_method(&m.name, &m.desc) {
                continue;
            }
            let params: Vec<(Option<SymId>, JType)> = m.params.iter().map(|t| (None, t.clone())).collect();
            self.begin_method(false, false, &params, m.ret.clone(), &[]);
            self.load(0, &this);
            let target = a.target.and_then(|t| match cx.input.syms.sym(t).owner {
                Owner::Class(o) => Some((t, o)),
                _ => None,
            });
            // The arguments as the definition the call runs takes them (`Base.id(String)` for
            // the trait's `id(Object)` of a generic parent), as scalac's erasure casts them.
            let taken: Vec<JType> = match target {
                Some((t, _)) => self.mref(t).params.clone(),
                None => self.mref(a.member).params.clone(),
            };
            let mut slot = 1u16;
            for (i, t) in m.params.iter().enumerate() {
                self.load(slot, t);
                if let Some(to) = taken.get(i) {
                    self.adapt(t, to);
                }
                slot += if t.wide() { 2 } else { 1 };
            }
            match target {
                Some((t, o)) => {
                    let mut special = (*self.mref(t)).clone();
                    let through = self.super_call_owner(tc.id, o);
                    special.owner = self.class_name(through);
                    special.owner_is_interface = cx.input.syms.class(through).kind == ClassKind::Trait;
                    special.kind = Invoke::Special;
                    self.invoke_mref(&special);
                    self.adapt(&special.ret, &m.ret);
                }
                None => {
                    let any = self.mref(a.member);
                    self.invoke_desc(Invoke::Special, OBJECT, false, &any.name, &any.desc, any.params.len(), &any.ret);
                    self.adapt(&any.ret, &m.ret);
                }
            }
            self.return_value(&m.ret);
            self.end_method(ACC_PUBLIC | ACC_SYNTHETIC, &m.name, &m.desc);
        }
    }

    /// The class an `invokespecial` of a super call from class `c` names for a method defined
    /// in `target`: `target` itself when it is the superclass or a trait the class mixes in
    /// directly, else the superclass, through which the JVM resolves a default method of a
    /// trait the superclass inherits (an interface `invokespecial` must name a direct
    /// superinterface).
    fn super_call_owner(&self, c: ClassId, target: ClassId) -> ClassId {
        let syms = self.cx.input.syms;
        let Some(sup) = syms.class(c).superclass else { return target };
        if target == sup || syms.class(target).kind != ClassKind::Trait {
            return target;
        }
        if syms.class(sup).base_types.iter().any(|&(b, _)| b == target) {
            sup
        } else {
            target
        }
    }

    /// In link mode, what scalac's mixin phase gives a class for the jar traits it extends: the
    /// super accessors (`abi$Doubling$$super$m`) of the traits it is the first to mix in, calling
    /// the definition after the trait in its linearisation, and for a jar trait's method a
    /// forwarder to the trait default that wins where the JVM would not pick it (two unrelated
    /// defaults, or a superclass's method that the trait overrides in the linearisation).
    pub fn jar_trait_mixins(&mut self, c: ClassId) {
        let cx = self.cx;
        if self.is_interface {
            return;
        }
        let syms = cx.input.syms;
        let bases: Vec<ClassId> = syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        let jar_traits: Vec<ClassId> = bases.iter().copied().filter(|&b| cx.class_files.contains_key(&b) && syms.class(b).kind == ClassKind::Trait).collect();
        if jar_traits.is_empty() {
            return;
        }
        let inherited = |t: ClassId| syms.class(c).superclass.map_or(false, |s| syms.class(s).base_types.iter().any(|&(b, _)| b == t));
        let this = JType::L(self.this_name.clone());
        let mut wanted: Vec<(String, String, Option<(ClassId, String)>)> = Vec::new();
        for &t in &jar_traits {
            let cf = &cx.class_files[&t];
            let prefix = format!("{}$$super$", cf.name.replace('/', "$"));
            for m in cf.methods.iter().filter(|m| m.access & ACC_ABSTRACT != 0) {
                let Some(member) = m.name.strip_prefix(&prefix) else { continue };
                if inherited(t) {
                    continue;
                }
                let after = bases.iter().position(|&b| b == t).map_or(0, |i| i + 1);
                let target = self.concrete_after(&bases[after..], member, &m.descriptor);
                wanted.push((m.name.clone(), m.descriptor.clone(), target));
            }
        }
        for &t in &jar_traits {
            let cf = &cx.class_files[&t];
            for m in cf.methods.iter().filter(|m| m.access & (ACC_ABSTRACT | ACC_STATIC | ACC_PRIVATE | ACC_BRIDGE) == 0 && !m.name.starts_with('<')) {
                if wanted.iter().any(|(n, d, _)| *n == m.name && *d == m.descriptor) {
                    continue;
                }
                let definers: Vec<ClassId> = bases.iter().copied().filter(|&b| self.defines(b, &m.name, &m.descriptor)).collect();
                let Some(&winner) = definers.first() else { continue };
                if syms.class(winner).kind != ClassKind::Trait {
                    continue;
                }
                let dominated = definers.iter().skip(1).all(|&d| syms.class(d).kind == ClassKind::Trait && syms.class(winner).base_types.iter().any(|&(b, _)| b == d));
                // `Object`'s own method wins over an interface default on the JVM, which a value
                // class's parents never name: its box forwards to the trait's.
                let object_member = syms.class(c).value_class
                    && matches!((m.name.as_str(), m.descriptor.as_str()), ("hashCode", "()I") | ("toString", "()Ljava/lang/String;") | ("equals", "(Ljava/lang/Object;)Z"));
                if !dominated || object_member {
                    wanted.push((m.name.clone(), m.descriptor.clone(), Some((winner, m.name.clone()))));
                }
            }
        }
        for (name, desc, target) in wanted {
            if self.cw.has_method(&name, &desc) {
                continue;
            }
            let (params, ret) = parse_method_desc(&desc);
            let typed: Vec<(Option<SymId>, JType)> = params.iter().map(|t| (None, t.clone())).collect();
            self.begin_method(false, false, &typed, ret.clone(), &[]);
            self.load(0, &this);
            let mut slot = 1u16;
            for t in &params {
                self.load(slot, t);
                slot += if t.wide() { 2 } else { 1 };
            }
            match target {
                Some((b, member)) => {
                    let through = self.super_call_owner(c, b);
                    let owner = self.class_name(through);
                    let interface = syms.class(through).kind == ClassKind::Trait;
                    self.invoke_desc(Invoke::Special, &owner, interface, &member, &desc, params.len(), &ret);
                }
                None => {
                    let member = name.rsplit("$$super$").next().unwrap_or(&name).to_string();
                    self.invoke_desc(Invoke::Special, OBJECT, false, &member, &desc, params.len(), &ret);
                }
            }
            self.return_value(&ret);
            self.end_method(ACC_PUBLIC, &name, &desc);
        }
    }

    /// The first of `bases` that defines `name` with a body under descriptor `desc`.
    fn concrete_after(&mut self, bases: &[ClassId], name: &str, desc: &str) -> Option<(ClassId, String)> {
        bases.iter().copied().find(|&b| self.defines(b, name, desc)).map(|b| (b, name.to_string()))
    }

    /// Whether class `b` declares `name` with a body under descriptor `desc`: from its class
    /// file for a jar class, from its members for a class of the program.
    fn defines(&mut self, b: ClassId, name: &str, desc: &str) -> bool {
        let cx = self.cx;
        if let Some(cf) = cx.class_files.get(&b) {
            return cf.methods.iter().any(|m| m.name == name && m.descriptor == desc && m.access & (ACC_ABSTRACT | ACC_STATIC) == 0);
        }
        let syms = cx.input.syms;
        if cx.tclass_of[b.idx()] == u32::MAX {
            return false;
        }
        let Some(n) = cx.input.interner.lookup(name) else { return false };
        let Some(&m) = syms.class(b).members.get(&n) else { return false };
        let alts: Vec<SymId> = syms.alternatives(m).map_or(vec![m], |a| a.to_vec());
        alts.into_iter().any(|m| {
            let f = cx.fun_of_sym[m.idx()];
            syms.sym(m).owner == Owner::Class(b) && f != u32::MAX && cx.input.prog.funs[f as usize].body.is_some() && self.mref(m).desc == desc
        })
    }
}
