//! What scalac's class files offer to code that teq did not compile, in link mode: the jars'
//! bytecode, reflection and the `java` launcher. A case class's companion with `apply`,
//! `unapply` and `fromProduct`, the case class's `copy`.

use crate::lir::{Catch, Caught, ClassFlags, Comparing, ErasedType, ExceptionHandler, FieldFlags, Invoke, Kind, LirId, MethodDef, MethodFlags, MethodParam, MethodRef, ParamRole, Relation, SwitchCase, TempId, Visibility};
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
const SERIALIZABLE: &str = "java/io/Serializable";
/// The cases from which a lookup is a switch, its default not counted: dotty's pattern matcher writes one from four
/// cases on, the default among them.
const SWITCH_FROM: usize = 3; // dotty: PatternMatcher.MinSwitchCases

/// What a module class's file lists after `interfaces`, its parents': `java/io/Serializable`,
/// which dotty's `BCodeSkelBuilder` gives every module class (a written object, an object nested
/// in a class, a companion the backend makes, an object of each enclosing instance, a file's
/// `$package`) whose parents do not name it. An object a block owns is no module class there:
/// `LambdaLift` drops `Module` from the class it lifts (`owned_by_block`).
pub fn module_interfaces(interfaces: &[String], owned_by_block: bool) -> Vec<String> {
    if !owned_by_block && !interfaces.iter().any(|i| i == SERIALIZABLE) {
        vec![SERIALIZABLE.to_string()]
    } else {
        Vec::new()
    }
}

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
fn type_name_order(a: &ErasedType, b: &ErasedType, result: bool) -> std::cmp::Ordering {
    fn segments(t: &ErasedType, result: bool) -> Vec<String> {
        let scala = |name: &str| vec!["scala".to_string(), name.to_string()];
        match t {
            ErasedType::V if result => scala("Unit"),
            ErasedType::V => vec!["scala".to_string(), "runtime".to_string(), "BoxedUnit".to_string()],
            ErasedType::Z => scala("Boolean"),
            ErasedType::B => scala("Byte"),
            ErasedType::S => scala("Short"),
            ErasedType::C => scala("Char"),
            ErasedType::I => scala("Int"),
            ErasedType::J => scala("Long"),
            ErasedType::F => scala("Float"),
            ErasedType::D => scala("Double"),
            ErasedType::L(n) if n.starts_with('[') => {
                let mut of = segments(&parse_type(&n[1..]).0, false);
                if let Some(last) = of.last_mut() {
                    last.push_str("[]");
                }
                of
            }
            ErasedType::L(n) => {
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
    /// A value class the typer gave an outer instance: one nested in a parameterless given, which dotty places as an
    /// object and teq's typer as the class of an instance. The typer's row of that given's class deletes it; until
    /// then such a class is written so that a program fails where master's does (`Gen::lower_vc_box`, `Gen::fit`).
    pub fn vc_with_outer(&self, c: ClassId) -> bool {
        self.input.syms.class(c).value_class && !self.statically_placed(c)
    }

    /// A class of a package or of objects, whose companion module is reached statically.
    pub fn statically_placed(&self, c: ClassId) -> bool {
        let syms = self.input.syms;
        match syms.class(c).owner {
            Owner::Package(_) => true,
            Owner::Class(o) => syms.class(o).kind == ClassKind::Object && self.statically_placed(o),
            Owner::Local => false,
        }
    }

    /// An enum whose companion is an instance, a local object's or an object's nested in a
    /// class: its values are the instance's fields, made with it.
    pub fn instance_enum(&self, e: ClassId) -> bool {
        let syms = self.input.syms;
        syms.class(e).companion.is_some_and(|m| syms.class(m).local_module.is_some() || syms.class(m).inner_object.is_some())
    }

    /// A case class whose companion module class holds scalac's factory members.
    /// Whether `m` is the companion the typer made for a case class without a written one
    /// (`Desugar.classDef`'s `companionDefs`): written here as scalac's synthesized companion
    /// (`emit_companion`, `emit_inner_companion`), not from its own class body.
    pub fn made_case_companion(&self, m: ClassId) -> bool {
        let info = self.input.syms.class(m);
        info.def.is_none() && info.companion.is_some_and(|k| {
            let ki = self.input.syms.class(k);
            // An enum's class case's too (`enter_enum_case_companion`), but a block's, which no
            // rule writes and its own class body does, as a block's case class's companion.
            let case = match ki.kind {
                ClassKind::Class => true,
                ClassKind::EnumCase => !self.in_a_block(k),
                _ => false,
            };
            case && ki.mods & crate::ast::mods::CASE != 0
        })
    }

    pub fn has_case_companion(&self, c: ClassId) -> bool {
        let info = self.input.syms.class(c);
        let case = self.is_case_class(c) || (info.kind == ClassKind::EnumCase && info.singleton.is_none());
        case
            && info.kind != ClassKind::Anon
            && self.emits(c)
            && self.statically_placed(c)
            && self.input.prog.classes[self.tclass_of[c.idx()] as usize].captures == 0
    }

    /// Whether the program's case class `c`, nested in a class, has the companion `Desugar` gives
    /// it, an inner module `O$C$` holding its enclosing instance, which the class `O` reaches
    /// through the lazy `C()` (the companion the products' TASTy declares): one without a
    /// written companion.
    pub fn has_inner_case_companion(&self, c: ClassId) -> bool {
        let info = self.input.syms.class(c);
        let Owner::Class(o) = info.owner else { return false };
        let i = self.tclass_of[c.idx()];
        self.is_case_class(c)
            && info.kind == ClassKind::Class
            && info.companion.map_or(true, |m| self.made_case_companion(m))
            && matches!(self.input.syms.class(o).kind, ClassKind::Class | ClassKind::Trait)
            && i != u32::MAX
            && self.input.prog.classes[i as usize].captures == 1
            && self.emits(c)
            && self.emits(o)
    }

    /// Whether the program's case class `c`, nested in a class, has a companion of each enclosing
    /// instance with a definition, written or the parser's (`companionDefs`), which
    /// `inner_written_companion_members` completes with scalac's members, the getters of the
    /// constructor's defaults among them.
    pub fn has_inner_written_case_companion(&self, c: ClassId) -> bool {
        let info = self.input.syms.class(c);
        let i = self.tclass_of[c.idx()];
        // A local enum's class case's made companion too, which no rule writes (`made_case_companion`).
        let case = match info.kind {
            ClassKind::Class => self.is_case_class(c),
            ClassKind::EnumCase => info.singleton.is_none() && self.in_a_block(c),
            _ => false,
        };
        matches!(info.owner, Owner::Class(_))
            && case
            && info.companion.is_some_and(|m| self.input.syms.class(m).inner_object.is_some() && !self.made_case_companion(m))
            && i != u32::MAX
            && self.input.prog.classes[i as usize].captures == 1
    }

    /// Whether the getters of the defaults of the primary constructor of the program's class `c`,
    /// nested in a class, are members of its companion, an object of each enclosing instance
    /// (`O$C$`, which `O.C()` holds): `Desugar.classDef` places them among the companion's
    /// members, a written companion's or the one it makes for a case class or for a class whose
    /// constructor has defaults (`companionDefs`), numbered over the declared parameters, and
    /// `ExplicitOuter` gives them no outer parameter: the companion's enclosing instance is the
    /// class's.
    pub fn ctor_defaults_on_inner_companion(&self, c: ClassId) -> bool {
        let syms = self.input.syms;
        let info = syms.class(c);
        let Owner::Class(o) = info.owner else { return false };
        let i = self.tclass_of[c.idx()];
        // A class of a trait has its companion as a class's has, which the classes mixing the trait
        // in hold.
        let owner = syms.class(o).kind;
        info.kind == ClassKind::Class
            && matches!(owner, ClassKind::Class | ClassKind::Trait)
            && i != u32::MAX
            && self.input.prog.classes[i as usize].captures >= 1
            && self.has_ctor_defaults(c)
            && self.emits(c)
            && self.emits(o)
            && info.companion.map_or(true, |m| self.made_case_companion(m) || (syms.class(m).inner_object.is_some() && self.emits(m)))
            && self.companion_holds_default_captures(c)
    }

    /// Whether the companion of `c`, a class nested in a class of a block that captures more than
    /// its enclosing instance, captures what its constructors' defaults read of those, as the
    /// typer has it capture them before the capture lowering (`Worker::companion_default_reads`).
    fn companion_holds_default_captures(&self, c: ClassId) -> bool {
        let prog = &self.input.prog;
        let tc = &prog.classes[self.tclass_of[c.idx()] as usize];
        let captured = &tc.ctor_params[1.min(tc.captures)..tc.captures];
        if captured.is_empty() {
            return true;
        }
        let held: &[SymId] = match self.input.syms.class(c).companion.map(|m| self.tclass_of[m.idx()]).filter(|&j| j != u32::MAX) {
            Some(j) => {
                let mc = &prog.classes[j as usize];
                &mc.ctor_params[..mc.captures]
            }
            None => &[],
        };
        let roots = tc.ctor_defaults.iter().flatten().copied().chain(tc.ctors.iter().flat_map(|&f| prog.funs[f.idx()].defaults.iter().flatten().copied()));
        for root in roots {
            for e in prog.descendants(root) {
                if let crate::tir::TExpr::Local(s) | crate::tir::TExpr::CallStatic(s, _) = prog.expr(e) {
                    if captured.contains(&s) && !held.contains(&s) {
                        return false;
                    }
                }
            }
        }
        true
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

    /// A local class whose constructors' defaults are instance getters of its local companion,
    /// the object written beside it or the one `Desugar.classDef` makes for them (`companionDefs`),
    /// which captures what they read (`Worker::companion_default_reads`) and which a creation's
    /// getter is selected on (`Applications.findDefaultGetter`), as scalac's lifted companion
    /// holds them.
    pub fn ctor_defaults_on_local_companion(&self, c: ClassId) -> bool {
        let info = self.input.syms.class(c);
        info.owner == Owner::Local
            && info.kind == ClassKind::Class
            && info.companion.is_some_and(|m| self.input.syms.class(m).local_module.is_some())
            && self.has_ctor_defaults(c)
            && self.emits(c)
    }

    /// The local module the default getters of secondary constructor `s` are read on where its class keeps them on a
    /// local companion (`ctor_defaults_on_local_companion`): a default of a secondary constructor is given no
    /// receiver, so the route reads the companion's val itself (`Gen::lower_ctor_default`).
    pub fn local_companion_of_secondary(&self, s: SymId) -> Option<SymId> {
        let info = self.input.syms.sym(s);
        let Owner::Class(c) = info.owner else { return None };
        if info.name != crate::names::INIT || !self.ctor_defaults_on_local_companion(c) {
            return None;
        }
        self.input.syms.class(c).companion.and_then(|m| self.input.syms.class(m).local_module)
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
        case && info.kind != ClassKind::Anon && self.emits(c)
    }

    /// The companion class that takes the static forwarders of module class `o`, when both are
    /// written.
    pub fn forwarding_class(&self, o: ClassId) -> Option<ClassId> {
        let syms = self.input.syms;
        if syms.class(o).kind != ClassKind::Object || !self.emits(o) {
            return None;
        }
        let x = syms.class(o).companion?;
        // An enum's class case is a case class whose companion is the object of its name in the enum's companion
        // (`DesugarEnums`): one module class, the object's.
        let info = syms.class(x);
        let class_case = info.kind == ClassKind::EnumCase && info.singleton.is_none();
        ((matches!(info.kind, ClassKind::Class | ClassKind::Enum) || class_case) && self.emits(x) && self.statically_placed(x)).then_some(x)
    }

    /// The mirror class of static forwarders of top-level module class `o` that has no
    /// companion class taking them, named as scalac names it.
    fn mirror_class(&self, o: ClassId) -> Option<String> {
        let info = self.input.syms.class(o);
        // An opaque type's companion is nested in its file's `$package` object: no mirror class.
        if !self.program_file(info.file) || info.kind != ClassKind::Object || !matches!(info.owner, Owner::Package(_)) || self.opaque_companion(o) {
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

    /// The methods of the class `tc` this build writes, each with its own defaults' getters: those
    /// the program reaches, a body of their own or none (an inline method's getters are ordinary
    /// methods: dotty's `isMethodWithExtension` excludes the inline method alone).
    pub fn reached_methods<'t>(&'t self, tc: &'t crate::tir::TClass) -> impl Iterator<Item = crate::tir::FunId> + 't {
        tc.methods.iter().copied().filter(|f| self.input.reach.funs[f.idx()])
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

    /// A class case of an enum of a class, whose companion is an object of each enclosing
    /// instance as scalac lays it out: `O$K$` holding
    /// its `$outer` and the factories (`Desugar.classDef`, `ExplicitOuter`), reached through the
    /// enclosing instance's `K()` over the field made with it (`LazyVals
    /// .transformSyntheticModule`). Not a class of a block, nor one a trait holds.
    pub fn has_instance_case_companion(&self, c: ClassId) -> bool {
        let syms = self.input.syms;
        let info = syms.class(c);
        // A case class of a class has its companion from `has_inner_case_companion`; this is the
        // class case of an enum of a class, whose companion's instance holds its factories.
        let case = info.kind == ClassKind::EnumCase && info.singleton.is_none();
        let Owner::Class(o) = info.owner else { return false };
        case && info.kind != ClassKind::Anon
            && !info.value_class
            && info.companion.map_or(true, |m| self.made_case_companion(m))
            && self.emits(c)
            && !self.statically_placed(c)
            && matches!(syms.class(o).kind, ClassKind::Class | ClassKind::Enum)
            && !self.in_a_block(o)
            && self.tclass_of[c.idx()] != u32::MAX
            && self.input.prog.classes[self.tclass_of[c.idx()] as usize].captures == 1
    }

    /// Whether the class `c` is defined in a block, at any depth of classes.
    pub(crate) fn in_a_block(&self, c: ClassId) -> bool {
        let mut at = self.input.syms.class(c).owner;
        loop {
            match at {
                Owner::Local => return true,
                Owner::Class(k) => at = self.input.syms.class(k).owner,
                Owner::Package(_) => return false,
            }
        }
    }

    /// The case classes nested in `o` whose companions are objects of `o`'s instances.
    pub fn instance_case_companions(&self, o: ClassId) -> Vec<ClassId> {
        let syms = self.input.syms;
        let mut out: Vec<ClassId> = syms.class(o).nested.values().copied().filter(|&k| self.has_instance_case_companion(k)).collect();
        if let Some(e) = syms.class(o).companion.filter(|&e| syms.class(e).kind == ClassKind::Enum) {
            out.extend(syms.class(e).children.iter().copied().filter(|&k| syms.class(k).owner == Owner::Class(o) && self.has_instance_case_companion(k)));
        }
        out.sort();
        out.dedup();
        out
    }
}

impl<'a> Gen<'a> {
    /// A class with the module class whose static forwarders it takes, which is written first
    /// so that its method table is known; a module class with its mirror class.
    pub fn emit_class_unit(&mut self, i: usize) -> (String, Vec<u8>) {
        let cx = self.cx;
        let c = cx.input.prog.classes[i].id;
        if cx.has_inner_case_companion(c) {
            let companion = self.emit_inner_companion(i);
            self.extra_classes.push(companion);
            self.reset_class();
        }
        let written = cx.input.syms.class(c).companion.filter(|&o| !cx.made_case_companion(o) && cx.forwarding_class(o) == Some(c));
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
                let info = cx.input.syms.class(c);
                let class = self.mirror(&mirror, &out.0, &table, Some((info.file, info.span)));
                self.extra_classes.push(class);
            }
            if cx.has_instance_case_companion(c) {
                self.reset_class();
                let companion = self.emit_instance_companion(i);
                self.extra_classes.push(companion);
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
            let class = self.mirror(&name, &out.0, &table, None);
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
        let keep = std::mem::take(&mut self.keep_table);
        let class = self.finish_writer();
        if keep {
            let pair_bridged = std::mem::take(&mut self.pair_bridged);
            self.module_table = std::mem::take(&mut self.plan.methods).into_iter().filter(|m| !pair_bridged.iter().any(|(pn, pd)| **pn == **m.name() && spells_method_desc(pd, m.params().iter().map(|p| &p.ty), m.ret()))).collect();
        }
        (name.to_string(), class)
    }

    /// A class of static forwarders only, as scalac writes for a top-level object.
    fn mirror(&mut self, name: &str, module: &str, table: &[MethodDef], place: Option<(crate::source::FileId, crate::source::Span)>) -> (String, Vec<u8>) {
        self.reset_class();
        self.begin_class(name, ClassFlags::FINAL, OBJECT, &[], &[], None, place);
        self.writing_mirror = true;
        self.this_name = Rc::from(name);
        self.this_class = None;
        self.line_file = crate::source::FileId(u32::MAX);
        self.context = name.to_string();
        self.static_forwarders(module, table);
        (name.to_string(), self.finish_writer())
    }

    /// Per public instance method of `module`, a static method of the class being written that
    /// calls it on `MODULE$`, unless the class declares a method of that name; the program's
    /// entry point is the `main(String[])` the `java` launcher calls.
    fn static_forwarders(&mut self, module: &str, table: &[MethodDef]) {
        self.writing_forwarders = true;
        self.static_forwarders_of(module, table);
        self.writing_forwarders = false;
    }

    fn static_forwarders_of(&mut self, module: &str, table: &[MethodDef]) {
        let cx = self.cx;
        let mut table: Vec<(MethodFlags, String, String)> = table.iter().map(|m| (m.flags(), m.name().to_string(), method_desc(&m.param_types(), m.ret()))).collect();
        if let Some(&o) = cx.class_by_name.get(module) {
            for (n, d) in self.inherited_methods(o, true) {
                if !table.iter().any(|(_, tn, td)| *tn == n && *td == d) {
                    table.push((MethodFlags::PUBLIC, n, d));
                }
            }
        }
        // A constructor, a static, private or bridge method gets no forwarder: the rest sorted, apart from them.
        table.retain(|(flags, name, _)| !name.starts_with('<') && !flags.is_static && flags.visibility != Visibility::Private && !flags.bridge);
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
        let declared: Vec<bool> = table.iter().map(|(_, n, _)| self.plan.has_method_named(n) || inherited_names.contains(n)).collect();
        let entry = cx.input.prog.main.filter(|&m| matches!(cx.input.syms.sym(m).owner, Owner::Class(o) if cx.class_names[o.idx()] == module));
        let entry_ref = entry.map(|m| self.mref(m));
        let module_name: Rc<str> = Rc::from(module);
        // A member's name expanded by a class of the module's (`T$$x`, a trait's private member)
        // gets no forwarder, a trait setter's (`T$_setter_$x_$eq`) a synthetic one, as dotty's
        // `addForwarders` has them (`ExpandedName`, `SyntheticSetterName`); a name the source
        // writes so (`` `T$$x` ``) is no expansion.
        let bases: Vec<ClassId> = cx.class_by_name.get(module).map_or(Vec::new(), |&o| cx.input.syms.class(o).base_types.iter().map(|&(b, _)| b).collect());
        let prefixes: Vec<String> = bases.iter().map(|b| cx.class_names[b.idx()].replace('/', "$")).collect();
        let written = |name: &str| cx.input.interner.lookup(name).map_or(false, |n| bases.iter().any(|&b| cx.input.syms.class(b).members.contains_key(&n)));
        let qualified = |name: &str, separator: &str| name.find(separator).map_or(false, |i| prefixes.iter().any(|p| *p == name[..i])) && !written(name);
        // The member a method of the table is, the first declaration of its name and descriptor
        // in the module class's linearisation (the overriding one), with whether it is protected
        // or private: such a member gets none, though its class file's method is public (dotty's
        // `addForwarders` over `sortedMembersBasedOnFlags(.., excluded = ExcludedForwarder)`):
        // `App.args`, but not a public override of a protected member; a qualified private one
        // (`private[p]`) does. A library member whose signature nothing completed is told by its
        // name alone.
        let mut declarations: Vec<(String, Option<String>, bool)> = Vec::new();
        for &b in &bases {
            for m in cx.members_in_order(b) {
                let info = cx.input.syms.sym(m);
                if info.owner != Owner::Class(b) || !matches!(info.kind, SymKind::Def | SymKind::Val | SymKind::Var | SymKind::Given) {
                    continue;
                }
                let hidden = info.mods & crate::ast::mods::PROTECTED != 0 || info.mods & crate::ast::mods::PRIVATE != 0 && !info.scoped_private;
                if info.sig.is_some() {
                    let r = self.mref(m);
                    declarations.push((r.name.clone(), Some(r.desc.clone()), hidden));
                } else {
                    declarations.push((encode(cx.input.interner.get(info.name)), None, hidden));
                }
            }
        }
        for ((_, name, desc), declared) in table.iter().zip(declared) {
            if declared {
                continue;
            }
            let member = declarations.iter().find(|(n, d, _)| n == name && d.as_ref().map_or(true, |d| d == desc));
            if member.is_some_and(|&(_, _, hidden)| hidden) {
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
            let typed: Vec<(Option<SymId>, ErasedType)> = params.iter().map(|t| (None, t.clone())).collect();
            let forwarded = method_ref(Invoke::Virtual, &module_name, false, name, &params, &ret);
            let flags = MethodFlags { is_static: true, synthetic: trait_setter, ..MethodFlags::PUBLIC };
            self.hold_member(flags, name, desc, &typed, ret.clone(), &[], super::classes::MethodSite::default(), "static forwarder", |g| {
                let instance = g.lir.module(&module_name);
                let args: Vec<_> = (0..params.len()).map(|i| g.param(i)).collect();
                let result = g.lir.call(&forwarded, instance, &args);
                g.lir.ret(result)
            });
        }
    }

    /// The methods class `c` inherits, by name and descriptor: a jar class's from its class
    /// file, a program class's from its members; `concrete` keeps those with a body only.
    /// `java.lang.Object`'s are left out.
    pub(super) fn inherited_methods(&mut self, c: ClassId, concrete: bool) -> Vec<(String, String)> {
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
            for m in cx.members_in_order(b) {
                let info = syms.sym(m);
                // A trait's given with parameters is a default method, which no class implements
                // again (`is_given_def`).
                let given_def = self.is_given_def(m);
                let private = info.mods & crate::ast::mods::PRIVATE != 0 && !info.scoped_private;
                if info.owner != Owner::Class(b) || private || !(matches!(info.kind, SymKind::Def | SymKind::Val | SymKind::Var) || given_def) {
                    continue;
                }
                let f = cx.fun_of_sym[m.idx()];
                let has_body = f != u32::MAX && cx.input.prog.funs[f as usize].body.is_some();
                if info.kind == SymKind::Def && (concrete && !has_body || !cx.input.reach.funs.get(f as usize).copied().unwrap_or(false) && concrete) {
                    continue;
                }
                // A trait's constant is the trait's own default getter, no field of the class that mixes
                // it in (`class_fields`): a concrete method of the trait, as a def with a body is.
                let trait_constant = info.kind == SymKind::Val && syms.class(b).kind == ClassKind::Trait && self.is_trait_constant(b, m);
                if concrete && info.kind != SymKind::Def && !given_def && !trait_constant {
                    continue;
                }
                let r = self.mref(m);
                out.push((r.name.clone(), r.desc.clone()));
            }
        }
        out
    }

    /// Whether the val `m` of the program's trait `b` is a constant (`constant`): its getter is the trait's
    /// default method. An inline val is left to its own lowering.
    fn is_trait_constant(&self, b: ClassId, m: SymId) -> bool {
        let idx = self.cx.tclass_of[b.idx()];
        self.cx.input.syms.sym(m).mods & crate::ast::mods::INLINE == 0
            && idx != u32::MAX
            && self.cx.input.prog.classes[idx as usize].init.iter().any(|i| matches!(*i, crate::tir::TInit::Field(s, e) if s == m && self.constant(s, e).is_some()))
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
        let place = info.def.and_then(|def| self.definition_start(info.file, def));
        self.begin_class(&name, ClassFlags::FINAL, OBJECT, &[], &[], None, place);
        self.this_name = Rc::from(name.as_str());
        self.this_class = None;
        self.line_file = info.file;
        self.context = name.clone();
        self.held_context(MethodFlags::PUBLIC, "<init>", "()V", &[], ErasedType::V, &[], super::classes::MethodSite::default());
        let object_init = method_ref(Invoke::Special, &Rc::from(OBJECT), false, "<init>", &[], &ErasedType::V);
        self.hold_tree("main class's constructor", |g| {
            let made = g.lir.init(&object_init, &[]);
            let unit = g.lir.unit();
            let done = g.lir.seq(&[made], unit);
            g.lir.ret(done)
        });
        self.entry_main(main, None);
        Some((name, self.finish_writer()))
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

    /// `O$R$$` for a case object `R` nested in the class `O` (`has_inner_case_companion`): the
    /// enclosing instance in `$outer`, its constructor taking it, `apply` making `R` with it, an
    /// `unapply` that always matches, `toString` and the outer accessor.
    /// A case object has no parameters and no product mirror; a case class's companion is the parser's.
    pub fn emit_inner_companion(&mut self, idx: usize) -> (String, Vec<u8>) {
        let cx = self.cx;
        let tc = &cx.input.prog.classes[idx];
        let c = tc.id;
        self.reset_class();
        let name: Rc<str> = Rc::from(cx.companion_name(c));
        self.context = name.to_string();
        let info = cx.input.syms.class(c);
        let path = &cx.input.sources[info.file.0 as usize].path;
        self.begin_class(&name, ClassFlags::FINAL, OBJECT, &[], &[SERIALIZABLE.to_string()], Some(path.rsplit(std::path::is_separator).next().unwrap_or(path)), Some((info.file, info.span)));
        self.this_name = name.clone();
        self.this_class = None;
        self.is_interface = false;
        self.abstract_class = false;
        self.line_file = info.file;
        let this = ErasedType::L(name.clone());
        let outer = self.sym_type(tc.ctor_params[0]);
        self.field(FieldFlags { is_final: true, ..FieldFlags::PRIVATE }, "$outer", &outer, None);
        self.held_context(MethodFlags::PUBLIC, "<init>", &method_desc(&[outer.clone()], &ErasedType::V), &[MethodParam::declared(None, outer.clone())], ErasedType::V, &[], super::classes::MethodSite::default());
        let object_init = method_ref(Invoke::Special, &Rc::from(OBJECT), false, "<init>", &[], &ErasedType::V);
        self.hold_tree("inner companion's constructor", |g| {
            let made = g.lir.init(&object_init, &[]);
            let instance = g.lir.this(this.clone());
            let given = g.param(0);
            let kept = g.lir.store(instance, &name, "$outer", given);
            let unit = g.lir.unit();
            let done = g.lir.seq(&[made, kept], unit);
            g.lir.ret(done)
        });

        let case_name = self.class_name(c);
        let case_type = ErasedType::L(case_name.clone());
        let case_init = method_ref(Invoke::Special, &case_name, false, "<init>", std::slice::from_ref(&outer), &ErasedType::V);
        self.hold_member(MethodFlags::PUBLIC, "apply", &method_desc(&[], &case_type), &[], case_type.clone(), &[], super::classes::MethodSite::default(), "inner companion's member", |g| {
            let instance = g.lir.this(this.clone());
            let held = g.lir.field(instance, &name, "$outer", outer.clone());
            let made = g.lir.make(&case_init, &[held]);
            g.lir.ret(made)
        });
        if !tc.retracted_unapply {
            self.hold_member(MethodFlags::PUBLIC, "unapply", &method_desc(&[case_type.clone()], &ErasedType::Z), &[(None, case_type.clone())], ErasedType::Z, &[], super::classes::MethodSite::default(), "inner companion's member", |g| {
                let matched = g.lir.bool(true);
                g.lir.ret(matched)
            });
        }
        let string = ErasedType::L(Rc::from(STRING));
        let shown: Rc<str> = Rc::from(cx.input.interner.get(info.name));
        self.hold_member(MethodFlags::PUBLIC, "toString", "()Ljava/lang/String;", &[], string.clone(), &[], super::classes::MethodSite::default(), "inner companion's member", |g| {
            let text = g.lir.text(&shown, string.clone());
            g.lir.ret(text)
        });
        // The outer accessor under scalac's name, which a downstream of either compiler calls
        // (`Worker::outer_accessor` names it so on the JVM).
        let accessor = format!("{}$$$outer", name.replace('/', "$"));
        self.hold_member(MethodFlags { is_final: true, synthetic: true, ..MethodFlags::PUBLIC }, &accessor, &method_desc(&[], &outer), &[], outer.clone(), &[], super::classes::MethodSite::default(), "inner companion's member", |g| {
            let instance = g.lir.this(this.clone());
            let held = g.lir.field(instance, &name, "$outer", outer.clone());
            g.lir.ret(held)
        });
        self.drain_pending();
        self.finish_class(&name)
    }

    /// The getters of the defaults of the primary constructor of `k` in its companion being
    /// written, an object of each instance of the class `k` is nested in
    /// (`ctor_defaults_on_inner_companion`): scalac's `$lessinit$greater$default$N`, numbered over
    /// the declared parameters and taking those of the clauses before the default's
    /// (`Desugar.addDefaultGetters`), each default read with the companion's enclosing instance,
    /// which `outer_of` reads, declared where the class's stands; a by-name parameter's
    /// returning the value, as a companion's getter does (`companion_default_getters`).
    fn inner_companion_default_getters(&mut self, k: ClassId, outer: &ErasedType, outer_of: &dyn Fn(&mut Self) -> LirId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let tc = &cx.input.prog.classes[cx.tclass_of[k.idx()] as usize];
        // The primary constructor's, whose defaults stand by its parameters after the enclosing
        // instance, and a secondary one's (`Desugar.classDef` collects every constructor's), whose
        // parameters follow its own enclosing instance and whose defaults are its declared
        // parameters' (`type_secondary_ctor`); only one constructor may have defaults.
        // The captures after the enclosing instance are the companion's own fields.
        let clauses = syms.class(k).ctor.clone();
        let n = tc.captures;
        self.inner_getters_of(tc.ctor_params[0], &tc.ctor_params[n..], &tc.ctor_defaults[n..], &clauses, outer, outer_of);
        for &f in &tc.ctors {
            let fun = &cx.input.prog.funs[f.idx()];
            if !cx.input.reach.funs[f.idx()] || fun.defaults.iter().all(Option::is_none) || fun.params.is_empty() {
                continue;
            }
            let clauses: Vec<ClauseSig> = syms.sym(fun.sym).sig.as_ref().map_or(Vec::new(), |sig| sig.clauses.clone());
            let declared: usize = clauses.iter().map(|cl| cl.params.len()).sum();
            let lead = fun.params.len().saturating_sub(declared).max(1);
            self.inner_getters_of(fun.params[0], &fun.params[lead..], &fun.defaults, &clauses, outer, outer_of);
        }
    }

    /// `inner_companion_default_getters` of one constructor: its enclosing instance `outer_sym`, its
    /// declared parameters and their defaults, parallel.
    fn inner_getters_of(&mut self, outer_sym: SymId, params: &[SymId], defaults: &[Option<crate::tir::TExprId>], clauses: &[ClauseSig], outer: &ErasedType, outer_of: &dyn Fn(&mut Self) -> LirId) {
        let mut start = 0;
        for cl in clauses {
            let before: Vec<(Option<SymId>, ErasedType)> = params[..start.min(params.len())].iter().map(|&p| (Some(p), self.sym_type(p))).collect();
            let before_types: Vec<ErasedType> = before.iter().map(|p| p.1.clone()).collect();
            for (j, p) in cl.params.iter().enumerate() {
                let i = start + j;
                let Some(d) = defaults.get(i).copied().flatten() else { continue };
                let ret = self.default_getter_ret(p.by_name.then_some(p.ty), |g| g.sym_type(params[i]));
                let site = super::classes::MethodSite::default();
                self.hold_member(MethodFlags::PUBLIC, &format!("$lessinit$greater$default${}", i + 1), &method_desc(&before_types, &ret), &before, ret.clone(), &[d], site, "inner companion's default getter", |g| {
                    let held = outer_of(g);
                    let from = g.lir.ty(held).clone();
                    let instance = g.lower_adapt(held, &from, outer);
                    g.lower_declared.insert(outer_sym, (outer.clone(), false));
                    let kept = g.lir.val(outer_sym, instance);
                    let value = g.lower_default_value(d, p.by_name, &ret);
                    let done = g.lir.seq(&[kept], value);
                    g.lir.ret(done)
                });
            }
            start += cl.params.len();
        }
    }

    /// The getters of the constructor's defaults of `c`, a case class nested in a class whose
    /// getters stay the class's (a block's, `ctor_defaults_on_inner_companion` false), on its
    /// companion: scalac's `$lessinit$greater$default$N` (`Desugar.defaultGetters`), each taking the
    /// parameters of the clauses before its own and calling the class's static getter, which takes
    /// the enclosing instance first (`ctor_default_getters`), which `outer_of` reads.
    fn forwarding_default_getters(&mut self, c: ClassId, tc: &crate::tir::TClass, outer: &ErasedType, outer_of: impl Fn(&mut Self) -> LirId) {
        let case_name = self.class_name(c);
        let clauses = self.cx.input.syms.class(c).ctor.clone();
        let mut start = 0;
        for cl in &clauses {
            for k in 0..cl.params.len() {
                let i = start + k;
                if tc.ctor_defaults.get(1 + i).copied().flatten().is_none() {
                    continue;
                }
                let before: Vec<(Option<SymId>, ErasedType)> = tc.ctor_params[1..1 + start].iter().map(|&p| (Some(p), self.sym_type(p))).collect();
                let before_types: Vec<ErasedType> = before.iter().map(|p| p.1.clone()).collect();
                let ret = self.sym_type(tc.ctor_params[1 + i]);
                let getter = format!("$lessinit$greater$default${}", i + 1);
                if self.plan.has_method(&getter, &method_desc(&before_types, &ret)) {
                    continue;
                }
                let statics: Vec<ErasedType> = std::iter::once(outer.clone()).chain(before_types.iter().cloned()).collect();
                let forwarded = method_ref(Invoke::Static, &case_name, false, &format!("$lessinit$greater$default${}", i + 2), &statics, &ret);
                let site = super::classes::MethodSite::default();
                self.hold_member(MethodFlags::PUBLIC, &getter, &method_desc(&before_types, &ret), &before, ret.clone(), &[], site, "forwarding default getter", |g| {
                    let mut args = vec![outer_of(g)];
                    for j in 0..before_types.len() {
                        args.push(g.param(j));
                    }
                    let value = g.lir.static_call(&forwarded, &args);
                    g.lir.ret(value)
                });
            }
            start += cl.params.len();
        }
    }

    /// In the written companion object `c` of the program's case class `k` nested in a class (an
    /// inner module, `O$E$`), what scalac's `Desugar` gives it beside its own members: `apply`
    /// constructing `k` with the companion's enclosing instance (teq's outer accessor of the
    /// module), the identity `unapply` unless withdrawn, and scalac's outer accessor; and the
    /// getters of the defaults of the constructor of `k`, a plain class's too
    /// (`inner_companion_default_getters`).
    pub fn inner_written_companion_members(&mut self, k: ClassId, c: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let info = syms.class(k);
        let (Owner::Class(_), Some(_)) = (info.owner, syms.class(c).inner_object) else { return };
        let i = cx.tclass_of[k.idx()];
        // A case class's, and a local enum's class case's (`has_inner_written_case_companion`); a
        // plain class's written companion holds its constructor's getters alone.
        let case = match info.kind {
            ClassKind::Class => cx.is_case_class(k),
            ClassKind::EnumCase => info.singleton.is_none() && cx.in_a_block(k),
            _ => false,
        };
        if i == u32::MAX || !case && !(info.kind == ClassKind::Class && cx.ctor_defaults_on_inner_companion(k)) {
            return;
        }
        let tc = &cx.input.prog.classes[i as usize];
        if tc.captures == 0 {
            return;
        }
        let name = self.this_name.clone();
        let this = ErasedType::L(name.clone());
        let outer = self.sym_type(tc.ctor_params[0]);
        // The companion's own outer accessor, by scalac's name (`Worker::outer_accessor`); a
        // companion of a class of a block's object has none, its enclosing instance a capture.
        let accessor = format!("{}$$$outer", name.replace('/', "$"));
        let captured = if self.plan.has_method(&accessor, &method_desc(&[], &outer)) { None } else { self.class_captures.first().map(|c| (c.1.clone(), c.2.clone())) };
        let outer_of = |g: &mut Self| g.companion_outer(&this, &name, &accessor, &outer, &captured);
        // A case class's companion members here take its enclosing instance alone; the getters
        // read what else the defaults read from the companion's own captures.
        if !case || tc.captures != 1 {
            if cx.ctor_defaults_on_inner_companion(k) {
                self.inner_companion_default_getters(k, &outer, &outer_of);
            }
            return;
        }
        let case_name = self.class_name(k);
        let case_type = ErasedType::L(case_name.clone());
        let params: Vec<(Option<SymId>, ErasedType)> = tc.ctor_params[1..].iter().map(|&p| (Some(p), self.sym_type(p))).collect();
        let types: Vec<ErasedType> = params.iter().map(|p| p.1.clone()).collect();
        let init_types: Vec<ErasedType> = std::iter::once(outer.clone()).chain(types.iter().cloned()).collect();
        let case_init = method_ref(Invoke::Special, &case_name, false, "<init>", &init_types, &ErasedType::V);
        let apply_desc = method_desc(&types, &case_type);
        // `apply` with every parameter, as `companionDefs` gives it whatever the defaults, which a
        // call fills in through the getters below.
        if info.mods & crate::ast::mods::ABSTRACT == 0 && !self.plan.has_method("apply", &apply_desc) {
            self.hold_member(MethodFlags::PUBLIC, "apply", &apply_desc, &params, case_type.clone(), &[], super::classes::MethodSite::default(), "inner case companion's member", |g| {
                let mut args = vec![g.companion_outer(&this, &name, &accessor, &outer, &captured)];
                for i in 0..types.len() {
                    args.push(g.param(i));
                }
                let made = g.lir.make(&case_init, &args);
                g.lir.ret(made)
            });
        }
        if !tc.retracted_unapply {
            let first = info.ctor.first();
            let repeated = first.and_then(|cl| cl.params.last()).map_or(false, |p| p.repeated);
            let extractor = if repeated { "unapplySeq" } else { "unapply" };
            let no_params = first.map_or(true, |cl| cl.params.is_empty());
            let ret = if no_params { ErasedType::Z } else { case_type.clone() };
            let desc = method_desc(&[case_type.clone()], &ret);
            if !self.plan.has_method(extractor, &desc) {
                self.hold_member(MethodFlags::PUBLIC, extractor, &desc, &[(None, case_type.clone())], ret.clone(), &[], super::classes::MethodSite::default(), "inner case companion's member", |g| {
                    let matched = if no_params { g.lir.bool(true) } else { g.param(0) };
                    g.lir.ret(matched)
                });
            }
        }
        // The `toString` `Desugar.classDef` gives the companion it makes is the typer's member of it
        // (`case_companion_to_string`), written with the class's methods.
        // `fromProduct` of the companion that is the case's mirror (`Mirror.Product` among its
        // bases, `SyntheticMembers.addMirrorSupport`), the case made with the enclosing instance.
        let mirror = syms.class(c).base_types.iter().any(|&(b, _)| &*cx.class_names[b.idx()] == MIRROR_PRODUCT);
        let product = ErasedType::L(Rc::from(PRODUCT));
        let from_desc = method_desc(&[product.clone()], &case_type);
        if mirror && !self.plan.has_method("fromProduct", &from_desc) {
            let object_type = ErasedType::object();
            let element = method_ref(Invoke::Interface, &Rc::from(PRODUCT), true, "productElement", &[ErasedType::I], &object_type);
            self.hold_member(MethodFlags::PUBLIC, "fromProduct", &from_desc, &[(None, product.clone())], case_type.clone(), &[], super::classes::MethodSite::default(), "inner case companion's member", |g| {
                let mut args = vec![g.companion_outer(&this, &name, &accessor, &outer, &captured)];
                for (i, t) in types.iter().enumerate() {
                    let elements = g.param(0);
                    let index = g.lir.int(i as i32);
                    let read = g.lir.call(&element, elements, &[index]);
                    args.push(g.lower_adapt(read, &object_type, t));
                }
                let made = g.lir.make(&case_init, &args);
                g.lir.ret(made)
            });
            let from_product = method_ref(Invoke::Virtual, &name, false, "fromProduct", &[product.clone()], &case_type);
            let flags = MethodFlags { synthetic: true, bridge: true, ..MethodFlags::PUBLIC };
            self.hold_member(flags, "fromProduct", &method_desc(&[product.clone()], &object_type), &[(None, product.clone())], object_type.clone(), &[], super::classes::MethodSite::default(), "inner case companion's member", |g| {
                let instance = g.lir.this(this.clone());
                let elements = g.param(0);
                let made = g.lir.call(&from_product, instance, &[elements]);
                g.lir.ret(made)
            });
        }
        // The companion's own getters where the class's are its companion's
        // (`ctor_defaults_on_inner_companion`), else ones that forward to the class's.
        if cx.ctor_defaults_on_inner_companion(k) {
            self.inner_companion_default_getters(k, &outer, &outer_of);
        } else {
            self.forwarding_default_getters(k, tc, &outer, outer_of);
        }
        let scalac_accessor = format!("{}$$$outer", name.replace('/', "$"));
        let desc = method_desc(&[], &outer);
        if !self.plan.has_method(&scalac_accessor, &desc) {
            self.hold_member(MethodFlags { is_final: true, ..MethodFlags::PUBLIC }, &scalac_accessor, &desc, &[], outer.clone(), &[], super::classes::MethodSite::default(), "inner case companion's member", |g| {
                let held = g.companion_outer(&this, &name, &accessor, &outer, &captured);
                g.lir.ret(held)
            });
        }
    }

    /// The enclosing instance of the companion being written: through its outer accessor, or its
    /// capture where it has none (`inner_written_companion_members`).
    fn companion_outer(&mut self, this: &ErasedType, name: &Rc<str>, accessor: &str, outer: &ErasedType, captured: &Option<(String, ErasedType)>) -> LirId {
        let instance = self.lir.this(this.clone());
        match captured {
            Some((field, ty)) => self.lir.field(instance, name, field, ty.clone()),
            None => {
                let read = method_ref(Invoke::Virtual, name, false, accessor, &[], outer);
                self.lir.call(&read, instance, &[])
            }
        }
    }

    /// `O$K$`, the companion of a case class nested in a class, an object of each instance of the
    /// class (`has_instance_case_companion`): its enclosing instance in `$outer`, `apply`,
    /// `unapply`, `toString`, and as a `Mirror.Product` `fromProduct`, each case made with that
    /// enclosing instance.
    pub fn emit_instance_companion(&mut self, idx: usize) -> (String, Vec<u8>) {
        let cx = self.cx;
        let c = cx.input.prog.classes[idx].id;
        let Owner::Class(o) = cx.input.syms.class(c).owner else { unreachable!("a nested class") };
        let interfaces: Vec<String> = cx.has_product_mirror(c).then(|| MIRROR_PRODUCT.to_string()).into_iter().collect();
        let listed = module_interfaces(&interfaces, false);
        let name: Rc<str> = Rc::from(cx.companion_name(c));
        self.context = name.to_string();
        let info = cx.input.syms.class(c);
        let path = &cx.input.sources[info.file.0 as usize].path;
        self.begin_class(&name, ClassFlags::FINAL, OBJECT, &interfaces, &listed, Some(path.rsplit(std::path::is_separator).next().unwrap_or(path)), Some((info.file, info.span)));
        self.this_name = name.clone();
        self.this_class = None;
        self.is_interface = false;
        self.abstract_class = false;
        self.line_file = info.file;
        let this = ErasedType::L(name.clone());
        let outer_name = self.class_name(o);
        let outer = ErasedType::L(outer_name.clone());
        self.field(FieldFlags { is_final: true, synthetic: true, ..FieldFlags::PRIVATE }, "$outer", &outer, None);
        self.held_context(MethodFlags::PUBLIC, "<init>", &method_desc(&[outer.clone()], &ErasedType::V), &[MethodParam::declared(None, outer.clone())], ErasedType::V, &[], super::classes::MethodSite::default());
        let object_init = method_ref(Invoke::Special, &Rc::from(OBJECT), false, "<init>", &[], &ErasedType::V);
        self.hold_tree("instance companion's constructor", |g| {
            let instance = g.lir.this(this.clone());
            let given = g.param(0);
            let kept = g.lir.store(instance, &name, "$outer", given);
            let made = g.lir.init(&object_init, &[]);
            let unit = g.lir.unit();
            let done = g.lir.seq(&[kept, made], unit);
            g.lir.ret(done)
        });

        let tc = &cx.input.prog.classes[idx];
        let case_name = self.class_name(c);
        let case_type = ErasedType::L(case_name.clone());
        let all: Vec<ErasedType> = tc.ctor_params.iter().map(|&p| self.sym_type(p)).collect();
        let declared: Vec<(Option<SymId>, ErasedType)> = tc.ctor_params[1..].iter().map(|&p| (Some(p), self.sym_type(p))).collect();
        let declared_types: Vec<ErasedType> = declared.iter().map(|p| p.1.clone()).collect();
        let case_init = method_ref(Invoke::Special, &case_name, false, "<init>", &all, &ErasedType::V);
        self.hold_member(MethodFlags::PUBLIC, "apply", &method_desc(&declared_types, &case_type), &declared, case_type.clone(), &[], super::classes::MethodSite::default(), "instance companion's member", |g| {
            let instance = g.lir.this(this.clone());
            let mut args = vec![g.lir.field(instance, &name, "$outer", outer.clone())];
            for i in 0..declared_types.len() {
                args.push(g.param(i));
            }
            let made = g.lir.make(&case_init, &args);
            g.lir.ret(made)
        });

        let first = cx.input.syms.class(c).info.ctor.first();
        let repeated = first.and_then(|cl| cl.params.last()).map_or(false, |p| p.repeated);
        let extractor = if repeated { "unapplySeq" } else { "unapply" };
        let no_params = first.map_or(true, |cl| cl.params.is_empty());
        let ret = if no_params { ErasedType::Z } else { case_type.clone() };
        self.hold_member(MethodFlags::PUBLIC, extractor, &method_desc(&[case_type.clone()], &ret), &[(None, case_type.clone())], ret.clone(), &[], super::classes::MethodSite::default(), "instance companion's member", |g| {
            let matched = if no_params { g.lir.bool(true) } else { g.param(0) };
            g.lir.ret(matched)
        });

        let string = ErasedType::L(Rc::from(STRING));
        let shown: Rc<str> = Rc::from(cx.input.interner.get(cx.input.syms.class(c).name));
        self.hold_member(MethodFlags::PUBLIC, "toString", &method_desc(&[], &string), &[], string.clone(), &[], super::classes::MethodSite::default(), "instance companion's member", |g| {
            let text = g.lir.text(&shown, string.clone());
            g.lir.ret(text)
        });

        if cx.has_product_mirror(c) {
            let product = ErasedType::L(Rc::from(PRODUCT));
            let object_type = ErasedType::object();
            let desc = method_desc(&[product.clone()], &case_type);
            let element = method_ref(Invoke::Interface, &Rc::from(PRODUCT), true, "productElement", &[ErasedType::I], &object_type);
            self.hold_member(MethodFlags::PUBLIC, "fromProduct", &desc, &[(None, product.clone())], case_type.clone(), &[], super::classes::MethodSite::default(), "instance companion's member", |g| {
                let instance = g.lir.this(this.clone());
                let mut args = vec![g.lir.field(instance, &name, "$outer", outer.clone())];
                for (i, t) in declared_types.iter().enumerate() {
                    let elements = g.param(0);
                    let index = g.lir.int(i as i32);
                    let read = g.lir.call(&element, elements, &[index]);
                    args.push(g.lower_adapt(read, &object_type, t));
                }
                let made = g.lir.make(&case_init, &args);
                g.lir.ret(made)
            });
            let from_product = method_ref(Invoke::Virtual, &name, false, "fromProduct", &[product.clone()], &case_type);
            self.hold_member(MethodFlags { synthetic: true, bridge: true, ..MethodFlags::PUBLIC }, "fromProduct", &method_desc(&[product.clone()], &object_type), &[(None, product.clone())], object_type.clone(), &[], super::classes::MethodSite::default(), "instance companion's member", |g| {
                let instance = g.lir.this(this.clone());
                let elements = g.param(0);
                let made = g.lir.call(&from_product, instance, &[elements]);
                g.lir.ret(made)
            });
        }
        self.drain_pending();
        self.finish_class(&name)
    }

    /// The enclosing instance's side of `has_instance_case_companion`: the getter `K()` of the
    /// companion of each case class nested in the class being written, a module val, `Final`, held
    /// in a thread-safe lazy val (dotty makes it eagerly when `allOverriddenSymbols` finds nothing,
    /// `transformSyntheticModule`; a queued item), made with this instance as its outer one.
    pub fn instance_case_companion_fields(&mut self, o: ClassId) {
        for k in self.cx.instance_case_companions(o) {
            let cx = self.cx;
            let accessor = encode(cx.input.interner.get(cx.input.syms.class(k).name));
            let module: Rc<str> = Rc::from(cx.companion_name(k));
            let ty = ErasedType::L(module.clone());
            let outer = self.this_name.clone();
            self.thread_safe_member(&accessor, &ty, super::classes::HolderInit::Module { class: module, outer }, MethodFlags { is_final: true, ..MethodFlags::PUBLIC });
        }
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
        self.extension_bodies(c);
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
        // A local class's captures lead its parameters: its companion holds them as its own.
        let n = tc.captures;
        self.companion_default_getters(&tc.ctor_params[n..], &tc.ctor_defaults[n..], &clauses);
        for &f in &tc.ctors {
            let fun = &cx.input.prog.funs[f.idx()];
            if !cx.input.reach.funs[f.idx()] || fun.defaults.iter().all(Option::is_none) {
                continue;
            }
            let clauses: Vec<ClauseSig> = syms.sym(fun.sym).sig.as_ref().map_or(Vec::new(), |sig| sig.clauses.clone());
            let declared: usize = clauses.iter().map(|cl| cl.params.len()).sum();
            let lead = fun.params.len().saturating_sub(declared);
            self.companion_default_getters(&fun.params[lead..], &fun.defaults, &clauses);
        }
    }

    /// A by-name parameter's getter returns the value, as a method's (`emit_fun`), which the
    /// constructor's caller passes as a thunk calling the getter.
    fn companion_default_getters(&mut self, params: &[SymId], defaults: &[Option<crate::tir::TExprId>], clauses: &[ClauseSig]) {
        let mut start = 0;
        for cl in clauses {
            let size = cl.params.len();
            let before: Vec<(Option<SymId>, ErasedType)> = params[..start.min(params.len())].iter().map(|&p| (Some(p), self.sym_type(p))).collect();
            let before_types: Vec<ErasedType> = before.iter().map(|p| p.1.clone()).collect();
            for i in start..(start + size).min(params.len()) {
                let Some(d) = defaults.get(i).copied().flatten() else { continue };
                let by_name = cl.params[i - start].by_name;
                let ret = self.default_getter_ret(by_name.then_some(cl.params[i - start].ty), |g| g.sym_type(params[i]));
                self.hold_member(MethodFlags::PUBLIC, &format!("$lessinit$greater$default${}", i + 1), &method_desc(&before_types, &ret), &before, ret.clone(), &[d], super::classes::MethodSite::default(), "companion's default getter", |g| {
                    let value = g.lower_default_value(d, by_name, &ret);
                    g.lir.ret(value)
                });
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
    pub fn extension_bodies(&mut self, c: ClassId) {
        let cx = self.cx;
        let u = self.vc_underlying(c);
        // The lifted methods so far are the module's own; those of the bodies below read `this`
        // as the box, the class they were written in.
        self.drain_pending();
        let saved = self.this_class.replace(c);
        let funs: Vec<crate::tir::FunId> = cx.reached_methods(&cx.input.prog.classes[cx.tclass_of[c.idx()] as usize]).collect();
        for f in funs {
            let fun = &cx.input.prog.funs[f.idx()];
            let m = self.mref(fun.sym);
            let info = cx.input.syms.sym(fun.sym);
            self.context = format!("{}.{}$extension", self.this_name, m.name);
            let mut params: Vec<MethodParam> = vec![MethodParam { ty: u.clone(), role: ParamRole::Underlying(c) }];
            params.extend(self.class_file_params(fun.sym, &fun.params).iter().zip(&m.params).map(|(&s, t)| MethodParam::declared(Some(s), t.clone())));
            let types: Vec<ErasedType> = params.iter().map(|p| p.ty.clone()).collect();
            if let Some(body) = fun.body {
                let site = super::classes::MethodSite { at: Some((info.file, info.span)), dotty: self.dotty_extension(c, fun.sym), ..Default::default() };
                self.held_context(MethodFlags { is_final: true, ..MethodFlags::PUBLIC }, &format!("{}$extension", m.name), &method_desc(&types, &m.ret), &params, m.ret.clone(), &[body], site);
                self.hold_def("value class's extension method", cx.line_of(info.file, info.span.start), (fun.sym, &fun.params, body));
            }
            for (i, d) in fun.own_defaults() {
                let shape = self.default_getter_shape(fun.sym, i);
                let (before, ret) = (shape.before, shape.ret.clone());
                let getter = format!("{}$default${}$extension", self.source_method_name(fun.sym), shape.index);
                let site = super::classes::MethodSite { at: Some((info.file, info.span)), ..Default::default() };
                self.held_context(MethodFlags { is_final: true, ..MethodFlags::PUBLIC }, &getter, &method_desc(&types[..before + 1], &ret), &params[..before + 1], ret.clone(), &[d], site);
                self.hold_tree("value class extension's default getter", |g| {
                    let value = g.lower_default_value(d, shape.by_name, &ret);
                    g.lir.ret(value)
                });
            }
        }
        // The members scalac synthesizes, `hashCode$extension(u)`, `equals$extension(u, that)` and
        // a case class's product members (`value_class_members`).
        for (member, params, ret) in self.vc_synthesized(c) {
            let mut types = vec![u.clone()];
            types.extend(params.iter().cloned());
            let typed: Vec<(Option<SymId>, ErasedType)> = types.iter().map(|t| (None, t.clone())).collect();
            self.context = format!("{}.{}$extension", self.this_name, member);
            let flags = MethodFlags { is_final: true, ..MethodFlags::PUBLIC };
            self.hold_member(flags, &format!("{}$extension", member), &method_desc(&types, &ret), &typed, ret.clone(), &[], super::classes::MethodSite::default(), "value class's extension member", |g| {
                g.vc_member_tree(c, member, &u, super::classes::Underlying::Param(0), 1)
            });
        }
        if cx.is_case_class(c) {
            let x = cx.input.syms.class(c).ctor.first().and_then(|cl| cl.params.first()).map(|p| self.sym_type(p.sym)).unwrap_or_else(|| u.clone());
            // An extension copy the class holds already (the user's copy's, a typed `_1`'s) is the class's.
            for (name, params, answered) in [("copy$extension", vec![u.clone(), x.clone()], 1), ("copy$default$1$extension", vec![u.clone()], 0), ("_1$extension", vec![u.clone()], 0)] {
                let desc = method_desc(&params, &u);
                if self.plan.has_method(name, &desc) {
                    continue;
                }
                let typed: Vec<(Option<SymId>, ErasedType)> = params.iter().map(|t| (None, t.clone())).collect();
                let flags = MethodFlags { is_final: true, ..MethodFlags::PUBLIC };
                self.hold_member(flags, name, &desc, &typed, u.clone(), &[], super::classes::MethodSite::default(), "value class's extension copy", |g| {
                    let value = g.param(answered);
                    g.lir.ret(value)
                });
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
        // The companion of a class a block owns is an object the block owns.
        let listed = module_interfaces(interfaces, cx.input.syms.class(c).owner == Owner::Local);
        let info = cx.input.syms.class(c);
        let path = &cx.input.sources[info.file.0 as usize].path;
        self.begin_class(&name, ClassFlags::FINAL, OBJECT, interfaces, &listed, Some(path.rsplit(std::path::is_separator).next().unwrap_or(path)), Some((info.file, info.span)));
        self.this_name = name.clone();
        self.this_class = None;
        self.is_interface = false;
        self.abstract_class = false;
        self.line_file = info.file;
        let this = ErasedType::L(name.clone());
        self.field(super::plan::MODULE_FIELD, "MODULE$", &this, None);

        // Its constructor's tree: `Object`'s constructor, then the enum's values it holds, which
        // the class file runs in the `<clinit>` (`Gen::write_module_part`), written first.
        let values = if info.kind == ClassKind::Enum { self.enum_value_fields(c) } else { Vec::new() };
        let touch = (!values.is_empty() && !cx.has_enum_fields(c)).then(|| self.class_name(c));
        let object_init = method_ref(Invoke::Special, &Rc::from(OBJECT), false, "<init>", &[], &ErasedType::V);
        let tree = |g: &mut Self| {
            let made = g.lir.init(&object_init, &[]);
            let mut parts = super::classes::Parts::of(vec![made]);
            g.enum_companion_parts(&mut parts, &name, touch.clone(), &values);
            g.returning(parts)
        };
        self.held_context(MethodFlags { visibility: Visibility::Package, is_static: true, ..MethodFlags::PUBLIC }, "<clinit>", "()V", &[], ErasedType::V, &[], super::classes::MethodSite::default());
        self.hold_module_part(false, tree);

        self.held_context(MethodFlags { visibility: Visibility::Private, ..MethodFlags::PUBLIC }, "<init>", "()V", &[], ErasedType::V, &[], super::classes::MethodSite::default());
        self.hold_module_part(true, |_| unreachable!("the tree its `<clinit>` made"));
        name
    }

    /// The members scalac gives the companion of case class `c`, in the module class being
    /// written; `object` is the written companion, whose own definitions of the same name and
    /// parameters win.
    pub fn companion_members(&mut self, c: ClassId, object: Option<ClassId>) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let tc = &cx.input.prog.classes[cx.tclass_of[c.idx()] as usize];
        let own: Vec<(Rc<str>, String)> = self.plan.methods.iter().map(|m| (m.name().clone(), method_desc(&m.param_types(), m.ret()))).collect();
        let written = |name: &str, params: &[ErasedType]| {
            let prefix = method_desc(params, &ErasedType::V);
            let prefix = &prefix[..prefix.len() - 1];
            own.iter().any(|(n, d)| &**n == name && d.starts_with(prefix))
        };
        let case_name = self.class_name(c);
        let case_type = ErasedType::L(case_name.clone());
        let object_type = ErasedType::object();
        let product = ErasedType::L(Rc::from(PRODUCT));
        let params: Vec<(Option<SymId>, ErasedType)> = tc.ctor_params.iter().map(|&p| (Some(p), self.sym_type(p))).collect();
        let types: Vec<ErasedType> = params.iter().map(|p| p.1.clone()).collect();
        let case_init = method_ref(Invoke::Special, &case_name, false, "<init>", &types, &ErasedType::V);

        // A value case class's are scalac's erased ones, over the underlying value.
        if cx.input.syms.class(c).value_class {
            let u = self.vc_underlying(c);
            for name in ["apply", "unapply"] {
                if !written(name, &[u.clone()]) {
                    self.hold_member(MethodFlags::PUBLIC, name, &method_desc(&[u.clone()], &u), &[(None, u.clone())], u.clone(), &[], super::classes::MethodSite::default(), "companion's member", |g| {
                        let given = g.param(0);
                        g.lir.ret(given)
                    });
                }
            }
        }
        if !written("apply", &types) && !cx.input.syms.class(c).value_class {
            // dotty's `apply` of a case class's companion: the constructor's parameters, the class's type.
            let site = super::classes::MethodSite { at: self.class_definition_start(c), dotty: self.dotty_constructor(c, &tc.ctor_params), ..Default::default() };
            self.hold_member(MethodFlags::PUBLIC, "apply", &method_desc(&types, &case_type), &params, case_type.clone(), &[], site, "companion's member", |g| {
                let mut args = Vec::with_capacity(types.len());
                for i in 0..types.len() {
                    args.push(g.param(i));
                }
                let made = g.lir.make(&case_init, &args);
                g.lir.ret(made)
            });
        }

        // Scala 3's extractor is the argument itself; a class without parameters answers a
        // Boolean, and one whose last parameter is repeated is matched through `unapplySeq`.
        let first = syms.class(c).info.ctor.first();
        let repeated = first.and_then(|cl| cl.params.last()).map_or(false, |p| p.repeated);
        let extractor = if repeated { "unapplySeq" } else { "unapply" };
        // Present unless the typer withdrew it (`retracted_unapply`): a companion's own
        // extractor of another result (`unapply(c: C[String]): Option[Int]`) leaves it in place,
        // as scalac emits both.
        let unapply_params = [case_type.clone()];
        let no_params = first.map_or(true, |cl| cl.params.is_empty());
        let ret = if no_params { ErasedType::Z } else { case_type.clone() };
        let unapply_desc = method_desc(&unapply_params, &ret);
        let same_desc = own.iter().any(|(n, d)| &**n == extractor && *d == unapply_desc);
        if !tc.retracted_unapply && !same_desc && !cx.input.syms.class(c).value_class {
            self.hold_member(MethodFlags::PUBLIC, extractor, &unapply_desc, &[(None, case_type.clone())], ret.clone(), &[], super::classes::MethodSite::default(), "companion's member", |g| {
                let matched = if no_params { g.lir.bool(true) } else { g.param(0) };
                g.lir.ret(matched)
            });
        }

        if cx.has_product_mirror(c) && !written("fromProduct", &[product.clone()]) {
            let desc = method_desc(&[product.clone()], &case_type);
            let element = method_ref(Invoke::Interface, &Rc::from(PRODUCT), true, "productElement", &[ErasedType::I], &object_type);
            self.hold_member(MethodFlags::PUBLIC, "fromProduct", &desc, &[(None, product.clone())], case_type.clone(), &[], super::classes::MethodSite::default(), "companion's member", |g| {
                let mut args = Vec::with_capacity(types.len());
                for (i, t) in types.iter().enumerate() {
                    let elements = g.param(0);
                    let index = g.lir.int(i as i32);
                    let read = g.lir.call(&element, elements, &[index]);
                    args.push(g.lower_adapt(read, &object_type, t));
                }
                let made = g.lir.make(&case_init, &args);
                g.lir.ret(made)
            });

            let this = ErasedType::L(self.this_name.clone());
            let from_product = method_ref(Invoke::Virtual, &self.this_name, false, "fromProduct", &[product.clone()], &case_type);
            let flags = MethodFlags { synthetic: true, bridge: true, ..MethodFlags::PUBLIC };
            self.hold_member(flags, "fromProduct", &method_desc(&[product.clone()], &object_type), &[(None, product.clone())], object_type.clone(), &[], super::classes::MethodSite::default(), "companion's member", |g| {
                let instance = g.lir.this(this);
                let elements = g.param(0);
                let made = g.lir.call(&from_product, instance, &[elements]);
                g.lir.ret(made)
            });
        }

        if cx.vc_on_companion(c) {
            self.extension_bodies(c);
        }
        // The `toString` `Desugar.classDef` gives the companion it makes, where the
        // typer's member of it is not the class's already.
        if object.map_or(true, |o| syms.class(o).synthetic_companion) && !self.plan.has_method("toString", "()Ljava/lang/String;") {
            let string = ErasedType::L(Rc::from(STRING));
            let shown: Rc<str> = Rc::from(cx.input.interner.get(syms.class(c).name));
            self.hold_member(MethodFlags::PUBLIC, "toString", "()Ljava/lang/String;", &[], string.clone(), &[], super::classes::MethodSite::default(), "companion's member", |g| {
                let text = g.lir.text(&shown, string);
                g.lir.ret(text)
            });
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

    /// scalac's `Mirror.Sum` of an enum's companion: `ordinal(E)I`, the value's ordinal, and its
    /// bridge `ordinal(Object)I`, in the module class being written; `object` is the written
    /// companion, whose own `ordinal` wins.
    fn enum_mirror_members(&mut self, e: ClassId, object: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        // The companion's own `ordinal` wins (`existingDef`); the one the typer entered beside the
        // mirror parent is this one.
        let own = cx.input.interner.lookup("ordinal").and_then(|n| syms.class(object).members.get(&n).copied());
        if own.is_some_and(|s| syms.sym(s).def.is_some()) {
            return;
        }
        let enum_type = self.class_type(e);
        let enum_name = self.class_name(e);
        let object_type = ErasedType::object();
        let desc = method_desc(&[enum_type.clone()], &ErasedType::I);
        let ordinal = method_ref(Invoke::Virtual, &enum_name, false, "ordinal", &[], &ErasedType::I);
        self.hold_member(MethodFlags::PUBLIC, "ordinal", &desc, &[(None, enum_type.clone())], ErasedType::I, &[], super::classes::MethodSite::default(), "enum companion's member", |g| {
            let value = g.param(0);
            let read = g.lir.call(&ordinal, value, &[]);
            g.lir.ret(read)
        });
        let this = ErasedType::L(self.this_name.clone());
        let typed = method_ref(Invoke::Virtual, &self.this_name, false, "ordinal", &[enum_type.clone()], &ErasedType::I);
        self.hold_member(MethodFlags { synthetic: true, bridge: true, ..MethodFlags::PUBLIC }, "ordinal", &method_desc(&[object_type.clone()], &ErasedType::I), &[(None, object_type.clone())], ErasedType::I, &[], super::classes::MethodSite::default(), "enum companion's member", |g| {
            let instance = g.lir.this(this);
            let given = g.param(0);
            let value = g.lower_adapt(given, &object_type, &enum_type);
            let read = g.lir.call(&typed, instance, &[value]);
            g.lir.ret(read)
        });
    }

    /// The value `v` of an enum as its companion reads it: the instance's field for an enum of
    /// an instance, else as a read with no receiver.
    fn lower_enum_value_read(&mut self, v: SymId, instance: bool) -> LirId {
        if !instance {
            return self.lower_static(v);
        }
        let t = self.sym_type(v);
        let name = self.field_name(v);
        let receiver = self.lower_this().expect("the companion's instance");
        let owner = self.this_name.clone();
        self.lir.field(receiver, &owner, &name, t)
    }

    /// scalac's `fromOrdinal` of an enum's companion, and `values` and `valueOf` where every case
    /// is a value; `object` is the companion being written, whose own definitions win.
    pub fn enum_companion_members(&mut self, e: ClassId, object: ClassId) {
        let cx = self.cx;
        let syms = cx.input.syms;
        let written = |name: &str| cx.input.interner.lookup(name).map_or(false, |n| syms.class(object).members.contains_key(&n));
        let enum_type = self.class_type(e);
        let string = ErasedType::L(Rc::from(STRING));
        let display = match cx.input.prog.local_enum_names.iter().find(|&&(x, _)| x == e) {
            Some((_, name)) => name.clone(),
            None => syms.scala_full_name(cx.input.interner, e),
        };
        let values: Vec<(SymId, u32, String)> = syms
            .class(e)
            .children
            .iter()
            .filter(|&&k| cx.input.reach.classes[k.idx()])
            .filter_map(|&k| syms.class(k).singleton.map(|v| (v, syms.class(k).ordinal, cx.input.interner.get(syms.class(k).name).to_string())))
            .collect();
        let all_values = values.len() == syms.class(e).children.len();
        let object_type = ErasedType::object();
        // A class's enum's companion is an instance, its values its fields.
        let instance = cx.instance_enum(e);
        if cx.has_enum_fields(e) || instance {
            self.enum_mirror_members(e, object);
        }

        // dotty's `enumLookupMethods`: `$values(ordinal)` where every case is a value, whose every
        // failure is the `NoSuchElementException`, else the value case of the ordinal.
        let stored = all_values && cx.has_enum_fields(e) && values.len() == syms.class(e).children.len();
        let value_of = method_ref(Invoke::Static, &Rc::from(STRING), false, "valueOf", &[ErasedType::I], &string);
        let no_such: Rc<str> = Rc::from("java/util/NoSuchElementException");
        let no_ordinal: Rc<str> = Rc::from(format!("enum {} has no case with ordinal: ", display));
        // The lookups of an enum whose values are static fields of its companion, as dotty's: a match on the argument,
        // a switch from three value cases on (`PatternMatcher`'s `MinSwitchCases`, the default among the four). An
        // enum of values only reads `fromOrdinal` off `$values`. The others keep the chains below.
        let placed = !instance && cx.has_enum_fields(e);
        if !written("fromOrdinal") && stored {
            let array = ErasedType::L(Rc::from(format!("[{}", self.desc_of(&enum_type))));
            let module = self.this_name.clone();
            self.hold_member(MethodFlags::PUBLIC, "fromOrdinal", &method_desc(&[ErasedType::I], &enum_type), &[(None, ErasedType::I)], enum_type.clone(), &[], super::classes::MethodSite::default(), "enum companion's member", |g| {
                let all = g.lir.static_field(&module, "$values", array.clone());
                let ordinal = g.param(0);
                let read = g.lir.push(Kind::ArrayLoad { array: all, index: ordinal }, enum_type.clone());
                let failed = g.lower_no_ordinal(&no_ordinal, &enum_type);
                g.lower_try_any(read, failed, &enum_type)
            });
        } else if !written("fromOrdinal") && placed && !all_values {
            self.hold_member(MethodFlags::PUBLIC, "fromOrdinal", &method_desc(&[ErasedType::I], &enum_type), &[(None, ErasedType::I)], enum_type.clone(), &[], super::classes::MethodSite::default(), "enum companion's member", |g| {
                // The switch's default, like each of its cases, leaves no value.
                if values.len() >= SWITCH_FROM {
                    let failed = g.lower_no_ordinal(&no_ordinal, &ErasedType::V);
                    return g.lower_ordinal_switch(&values, failed, &enum_type);
                }
                let failed = g.lower_no_ordinal(&no_ordinal, &enum_type);
                g.lower_lookup_match(&values, failed, &enum_type, |g, ordinal, _, x| {
                    let at = g.lir.int(ordinal as i32);
                    let given = g.lir.kept(x, ErasedType::I);
                    g.lir.compare(Relation::Eq, Comparing::Numbers, at, given)
                })
            });
        } else if !written("fromOrdinal") {
            self.hold_member(MethodFlags::PUBLIC, "fromOrdinal", &method_desc(&[ErasedType::I], &enum_type), &[(None, ErasedType::I)], enum_type.clone(), &[], super::classes::MethodSite::default(), "enum companion's member", |g| {
                let mut answers = Vec::with_capacity(values.len());
                for (v, ordinal, _) in &values {
                    let given = g.param(0);
                    let at = g.lir.int(*ordinal as i32);
                    let matched = g.lir.compare(Relation::Eq, Comparing::Numbers, given, at);
                    let value = g.lower_enum_value_read(*v, instance);
                    let t = g.lir.ty(value).clone();
                    let value = g.lower_adapt(value, &t, &enum_type);
                    let answer = g.lir.ret(value);
                    answers.push(g.lir.when(matched, answer));
                }
                let ordinal = g.param(0);
                let shown = g.lir.static_call(&value_of, &[ordinal]);
                let failed = g.lower_throw_with(&no_such, &no_ordinal, shown);
                g.lir.seq(&answers, failed)
            });
        }
        if !all_values {
            return;
        }
        if !written("values") && cx.has_enum_fields(e) && values.len() == syms.class(e).children.len() {
            let array = ErasedType::L(Rc::from(format!("[{}", self.desc_of(&enum_type))));
            let ErasedType::L(array_name) = &array else { unreachable!() };
            let array_name = array_name.clone();
            let module = self.this_name.clone();
            let clone = method_ref(Invoke::Virtual, &array_name, false, "clone", &[], &object_type);
            self.hold_member(MethodFlags::PUBLIC, "values", &method_desc(&[], &array), &[], array.clone(), &[], super::classes::MethodSite::default(), "enum companion's member", |g| {
                let stored = g.lir.static_field(&module, "$values", array.clone());
                let copied = g.lir.call(&clone, stored, &[]);
                let copy = g.lower_adapt(copied, &object_type, &array);
                g.lir.ret(copy)
            });
        } else if !written("values") {
            let array = ErasedType::L(Rc::from(format!("[{}", self.desc_of(&enum_type))));
            self.hold_member(MethodFlags::PUBLIC, "values", &method_desc(&[], &array), &[], array.clone(), &[], super::classes::MethodSite::default(), "enum companion's member", |g| {
                let mut items = Vec::with_capacity(values.len());
                for (v, _, _) in &values {
                    let value = g.lower_enum_value_read(*v, instance);
                    let t = g.lir.ty(value).clone();
                    items.push(g.lower_adapt(value, &t, &enum_type));
                }
                let items = g.lir.list(&items);
                let made = g.lir.push(Kind::ArrayOf { items }, array.clone());
                g.lir.ret(made)
            });
        }
        if !written("valueOf") && placed {
            let no_name: Rc<str> = Rc::from(format!("enum {} has no case with name: ", display));
            let equals = method_ref(Invoke::Virtual, &Rc::from(OBJECT), false, "equals", &[ErasedType::object()], &ErasedType::Z);
            self.hold_member(MethodFlags::PUBLIC, "valueOf", &method_desc(&[string.clone()], &enum_type), &[(None, string.clone())], enum_type.clone(), &[], super::classes::MethodSite::default(), "enum companion's member", |g| {
                let name = g.param(0);
                let failed = g.lower_lookup_failure("java/lang/IllegalArgumentException", &no_name, name, &enum_type);
                if values.len() >= SWITCH_FROM {
                    return g.lower_name_switch(&values, failed, &enum_type, &equals);
                }
                g.lower_lookup_match(&values, failed, &enum_type, |g, _, case, x| {
                    let text = g.lir.text(&Rc::from(case), string.clone());
                    let given = g.lir.kept(x, string.clone());
                    g.lir.call(&equals, text, &[given])
                })
            });
        } else if !written("valueOf") {
            let no_name: Rc<str> = Rc::from(format!("enum {} has no case with name: ", display));
            let illegal: Rc<str> = Rc::from("java/lang/IllegalArgumentException");
            let equals = method_ref(Invoke::Virtual, &Rc::from(STRING), false, "equals", &[ErasedType::object()], &ErasedType::Z);
            self.hold_member(MethodFlags::PUBLIC, "valueOf", &method_desc(&[string.clone()], &enum_type), &[(None, string.clone())], enum_type.clone(), &[], super::classes::MethodSite::default(), "enum companion's member", |g| {
                let mut answers = Vec::with_capacity(values.len());
                for (v, _, name) in &values {
                    let text = g.lir.text(&Rc::from(name.as_str()), string.clone());
                    let given = g.param(0);
                    let matched = g.lir.call(&equals, text, &[given]);
                    let value = g.lower_enum_value_read(*v, instance);
                    let t = g.lir.ty(value).clone();
                    let value = g.lower_adapt(value, &t, &enum_type);
                    let answer = g.lir.ret(value);
                    answers.push(g.lir.when(matched, answer));
                }
                let given = g.param(0);
                let failed = g.lower_throw_with(&illegal, &no_name, given);
                g.lir.seq(&answers, failed)
            });
        }
    }

    /// Throws a new `class` whose message is `prefix` and the string on the stack.
    /// A new exception of the class `class` thrown, its message `prefix` followed by `rest`, a
    /// string held while the exception is made.
    fn lower_throw_with(&mut self, class: &Rc<str>, prefix: &Rc<str>, rest: LirId) -> LirId {
        let string = ErasedType::L(Rc::from(STRING));
        let concat = method_ref(Invoke::Virtual, &Rc::from(STRING), false, "concat", std::slice::from_ref(&string), &string);
        let ctor = method_ref(Invoke::Special, class, false, "<init>", std::slice::from_ref(&string), &ErasedType::V);
        let held = self.lir.temp();
        let kept = self.lir.keep(held, rest);
        let text = self.lir.text(prefix, string.clone());
        let part = self.lir.kept(held, string);
        let message = self.lir.call(&concat, text, &[part]);
        let made = self.lir.make(&ctor, &[message]);
        let thrown = self.lir.throw(made, ErasedType::V);
        self.lir.seq(&[kept], thrown)
    }

    /// `throw new NoSuchElementException(prefix + ordinal.toString)`, `ordinal.toString` as dotty's erasure leaves
    /// it (`BoxesRunTime.boxToInteger(ordinal).toString()`).
    fn lower_no_ordinal(&mut self, prefix: &Rc<str>, want: &ErasedType) -> LirId {
        let integer: Rc<str> = Rc::from("java/lang/Integer");
        let boxed = method_ref(Invoke::Static, &Rc::from("scala/runtime/BoxesRunTime"), false, "boxToInteger", &[ErasedType::I], &ErasedType::L(integer.clone()));
        let shown = method_ref(Invoke::Virtual, &integer, false, "toString", &[], &ErasedType::L(Rc::from(STRING)));
        let ordinal = self.param(0);
        let ordinal = self.lir.static_call(&boxed, &[ordinal]);
        let ordinal = self.lir.call(&shown, ordinal, &[]);
        self.lower_lookup_failure("java/util/NoSuchElementException", prefix, ordinal, want)
    }

    /// A companion's lookup failing: `throw new class(prefix + shown)`, its message one concatenation.
    fn lower_lookup_failure(&mut self, class: &str, prefix: &Rc<str>, shown: LirId, want: &ErasedType) -> LirId {
        let string = ErasedType::L(Rc::from(STRING));
        let ctor = method_ref(Invoke::Special, &Rc::from(class), false, "<init>", std::slice::from_ref(&string), &ErasedType::V);
        let text = self.lir.text(prefix, string.clone());
        let message = self.concat(&[text, shown]);
        let made = self.lir.make(&ctor, &[message]);
        self.lir.throw(made, want.clone())
    }

    /// dotty's `try body catch { case _ => failed }`, returned: one handler of `Throwable` that drops the exception.
    fn lower_try_any(&mut self, body: LirId, failed: LirId, want: &ErasedType) -> LirId {
        let done = self.lir.label();
        let handler = self.lower_handler_value(failed, Vec::new(), done, want);
        let handlers = Rc::from([ExceptionHandler { class: Rc::from(THROWABLE), caught: Caught::Dropped, body: handler }]);
        let protected = self.push_protected(Kind::Try { body, catch: Some(Catch { done, handlers }), finalizer: None, returns: false }, want.clone());
        self.lir.ret(protected)
    }

    /// A lookup of a static enum's values as dotty's match on the method's argument: the argument in a temporary,
    /// then for each value case its test (`test` of its ordinal and name over the temporary) and its value, read
    /// through the companion's `this` and returned; `failed` where no case matches.
    fn lower_lookup_match(&mut self, values: &[(SymId, u32, String)], failed: LirId, want: &ErasedType, test: impl Fn(&mut Self, u32, &str, TempId) -> LirId) -> LirId {
        if values.is_empty() {
            return failed;
        }
        let x = self.lir.temp();
        let given = self.param(0);
        let mut stmts = vec![self.lir.keep(x, given)];
        for (v, ordinal, name) in values {
            let cond = test(self, *ordinal, name, x);
            let answer = self.lower_lookup_answer(*v, want);
            stmts.push(self.lir.when(cond, answer));
        }
        self.lir.seq(&stmts, failed)
    }

    /// The value case `v`'s value returned by a lookup: its prefix, the companion, evaluated and dropped before its
    /// static field is read.
    fn lower_lookup_answer(&mut self, v: SymId, want: &ErasedType) -> LirId {
        let this = ErasedType::L(self.this_name.clone());
        let qualifier = self.lir.this(this.clone());
        let qualifier = self.lower_adapt(qualifier, &this, &ErasedType::V);
        let value = self.lower_enum_value_read(v, false);
        let t = self.lir.ty(value).clone();
        let value = self.lower_adapt(value, &t, want);
        let value = self.lir.seq(&[qualifier], value);
        self.lir.ret(value)
    }

    /// dotty's match on the ordinal as a switch: the ordinal in a temporary, then one case a value case, its ordinal
    /// the key (distinct: each case's position among the enum's cases), returning its value; `failed` the default.
    fn lower_ordinal_switch(&mut self, values: &[(SymId, u32, String)], failed: LirId, want: &ErasedType) -> LirId {
        let x = self.lir.temp();
        let given = self.param(0);
        let kept = self.lir.keep(x, given);
        let mut cases = Vec::with_capacity(values.len());
        for (v, ordinal, _) in values {
            let body = self.lower_lookup_answer(*v, want);
            cases.push(SwitchCase { keys: Rc::from([*ordinal as i32]), body });
        }
        let scrutinee = self.lir.kept(x, ErasedType::I);
        let switch = self.lir.switch(scrutinee, cases, failed, ErasedType::V);
        self.lir.seq(&[kept], switch)
    }

    /// dotty's match on a name as its back end writes a match on a `String` (BCodeBodyBuilder.scala 1018-1093 at
    /// 3.8.4): the name in a temporary; a switch on its `hashCode`, 0 for null; for each hash in increasing order (one
    /// key a case: distinct) the test of each of its names, the latest first, `equals` on the name's literal, each
    /// returning its value, then the jump to the default; the default `failed`, after the switch.
    fn lower_name_switch(&mut self, values: &[(SymId, u32, String)], failed: LirId, want: &ErasedType, equals: &Rc<MethodRef>) -> LirId {
        let string = ErasedType::L(Rc::from(STRING));
        let hash_code = method_ref(Invoke::Virtual, &Rc::from(STRING), false, "hashCode", &[], &ErasedType::I);
        let x = self.lir.temp();
        let given = self.param(0);
        let kept = self.lir.keep(x, given);
        // `String.hashCode` of the name, as the JVM computes it over its UTF-16 code units.
        let mut by_hash: std::collections::BTreeMap<i32, Vec<(SymId, &str)>> = std::collections::BTreeMap::new();
        for (v, _, name) in values {
            let hash = name.encode_utf16().fold(0i32, |h, c| h.wrapping_mul(31).wrapping_add(c as i32));
            by_hash.entry(hash).or_default().insert(0, (*v, name.as_str()));
        }
        let default = self.lir.label();
        let mut cases = Vec::with_capacity(by_hash.len());
        for (hash, names) in by_hash {
            let mut tests = Vec::with_capacity(names.len());
            for (v, name) in names {
                let text = self.lir.text(&Rc::from(name), string.clone());
                let named = self.lir.kept(x, string.clone());
                let cond = self.lir.call(equals, text, &[named]);
                let answer = self.lower_lookup_answer(v, want);
                tests.push(self.lir.when(cond, answer));
            }
            let missed = self.lir.push(Kind::Break { label: default, value: None }, ErasedType::V);
            cases.push(SwitchCase { keys: Rc::from([hash]), body: self.lir.seq(&tests, missed) });
        }
        let tested = self.lir.kept(x, string.clone());
        let null = self.lir.push(Kind::Null, string.clone());
        let is_null = self.lir.compare(Relation::Eq, Comparing::Identity, tested, null);
        let zero = self.lir.int(0);
        let named = self.lir.kept(x, string.clone());
        let hashed = self.lir.call(&hash_code, named, &[]);
        let scrutinee = self.lir.push(Kind::If { cond: is_null, then: zero, otherwise: Some(hashed) }, ErasedType::I);
        let missed = self.lir.push(Kind::Break { label: default, value: None }, ErasedType::V);
        let switch = self.lir.switch(scrutinee, cases, missed, ErasedType::V);
        let matched = self.lir.labeled(default, switch);
        self.lir.seq(&[kept, matched], failed)
    }

    pub(super) fn desc_of(&self, t: &ErasedType) -> String {
        let mut d = String::new();
        t.desc(&mut d);
        d
    }

    /// `copy` and its defaults, the fields of the instance, in case class `c`.
    pub fn copy_members(&mut self, c: ClassId) {
        let cx = self.cx;
        let tc = &cx.input.prog.classes[cx.tclass_of[c.idx()] as usize];
        let owner = self.this_name.clone();
        let this = ErasedType::L(owner.clone());
        let all: Vec<(Option<SymId>, ErasedType)> = tc.ctor_params.iter().map(|&p| (Some(p), self.sym_type(p))).collect();
        let all_types: Vec<ErasedType> = all.iter().map(|p| p.1.clone()).collect();
        // A class of a class takes its enclosing instance first, which `copy` passes on from its
        // own field: scalac's `copy` of an inner case class takes the class's parameters alone.
        // The constructor takes what the class captures as `class_captured` expands it (the
        // captures a local def or a local lazy val brings beside the declared ones), each passed on
        // from the field holding it.
        let mut captured: Vec<(String, ErasedType)> = self.class_captures.clone().into_iter().map(|(s, f, _, _)| (f, self.anon_capture_type(s))).collect();
        // Its `Extras` after them, from their fields too.
        captured.extend(self.extras_fields().into_iter().map(|(f, t)| (f.to_string(), t)));
        if captured.len() < tc.captures || tc.captures > all.len() {
            return;
        }
        let k = tc.captures;
        let params: Vec<(Option<SymId>, ErasedType)> = all[k..].to_vec();
        let types: Vec<ErasedType> = all_types[k..].to_vec();
        let init_types: Vec<ErasedType> = captured.iter().map(|(_, t)| t.clone()).chain(types.iter().cloned()).collect();
        let prefix = method_desc(&types, &ErasedType::V);
        let prefix = &prefix[..prefix.len() - 1];
        // A value case class's `copy` answers the underlying value, as scalac's erasure has it.
        if cx.input.syms.class(c).value_class && types.len() == 1 {
            let u = types[0].clone();
            if !self.plan.methods.iter().any(|m| &**m.name() == "copy" && method_desc(&m.param_types(), m.ret()).starts_with(prefix)) {
                self.hold_member(MethodFlags::PUBLIC, "copy", &method_desc(&types, &u), &params, u.clone(), &[], super::classes::MethodSite::default(), "case class's copy", |g| {
                    let given = g.param(0);
                    g.lir.ret(given)
                });
            }
        } else if !self.plan.methods.iter().any(|m| &**m.name() == "copy" && method_desc(&m.param_types(), m.ret()).starts_with(prefix)) {
            // dotty's `copy` (Desugar.scala's case class members): the constructor's parameters, the class's own type.
            let declared: Vec<SymId> = params.iter().filter_map(|p| p.0).collect();
            let site = super::classes::MethodSite { at: self.class_definition_start(c), dotty: self.dotty_constructor(c, &declared), ..Default::default() };
            let init = method_ref(Invoke::Special, &owner, false, "<init>", &init_types, &ErasedType::V);
            self.hold_member(MethodFlags::PUBLIC, "copy", &method_desc(&types, &this), &params, this.clone(), &[], site, "case class's copy", |g| {
                let mut args = Vec::with_capacity(init_types.len());
                for (field, t) in captured.iter() {
                    let instance = g.lir.this(this.clone());
                    args.push(g.lir.field(instance, &owner, field, t.clone()));
                }
                for i in 0..types.len() {
                    args.push(g.param(i));
                }
                let made = g.lir.make(&init, &args);
                g.lir.ret(made)
            });
        }
        let first = cx.input.syms.class(c).ctor.first().map_or(0, |cl| cl.params.len());
        // A default getter of copy's the class holds already (the user's copy's own) is the class's (Namer.invalidateIfClashingSynthetic).
        for (i, &p) in tc.ctor_params[k..].iter().enumerate().take(first) {
            let t = types[i].clone();
            let name = format!("copy$default${}", i + 1);
            let desc = method_desc(&[], &t);
            if self.plan.has_method(&name, &desc) {
                continue;
            }
            let field = self.field_name(p);
            self.hold_member(MethodFlags::PUBLIC, &name, &desc, &[], t.clone(), &[], super::classes::MethodSite::default(), "case class's copy", |g| {
                let instance = g.lir.this(this.clone());
                let read = g.lir.field(instance, &owner, &field, t);
                g.lir.ret(read)
            });
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
            let typed: Vec<(Option<SymId>, ErasedType)> = m.params.iter().map(|t| (None, t.clone())).collect();
            let this = ErasedType::L(self.this_name.clone());
            let init = method_ref(Invoke::Special, &class, false, "<init>", &ctor, &ErasedType::V);
            let flags = MethodFlags { is_final: !self.is_interface, ..MethodFlags::PUBLIC };
            self.hold_member(flags, &m.name, &m.desc, &typed, m.ret.clone(), &[], super::classes::MethodSite::default(), "given class's def", |g| {
                let mut args = Vec::with_capacity(ctor.len());
                if let Some(t) = outer.first() {
                    let instance = g.lir.this(this.clone());
                    args.push(g.lower_adapt(instance, &this, t));
                }
                for (i, (from, to)) in m.params.iter().zip(&ctor[outer.len()..]).enumerate() {
                    let given = g.param(i);
                    args.push(g.lower_adapt(given, from, to));
                }
                let made = g.lir.make(&init, &args);
                g.lir.ret(made)
            });
            if self.is_interface {
                self.trait_static_forwarder(&m.name, &m.params, &m.ret);
            }
        }
    }

    /// scalac's `Mirror.Singleton` of a case object, in its module class being written:
    /// `fromProduct(Product)` answering the object, and the bridge to it. Where the object
    /// writes its own `fromProduct`, it and the bridges erasure makes for it are the object's
    /// (SyntheticMembers.addMirrorSupport).
    pub fn singleton_mirror_members(&mut self) {
        let this = ErasedType::L(self.this_name.clone());
        let product = ErasedType::L(Rc::from(PRODUCT));
        let singleton = ErasedType::L(Rc::from(MIRROR_SINGLETON));
        let object = ErasedType::object();
        let desc = method_desc(&[product.clone()], &singleton);
        if !self.plan.has_method("fromProduct", &desc) {
            self.hold_member(MethodFlags::PUBLIC, "fromProduct", &desc, &[(None, product.clone())], singleton.clone(), &[], super::classes::MethodSite::default(), "case object's mirror member", |g| {
                let instance = g.lir.this(this.clone());
                let mirror = g.lower_adapt(instance, &this, &singleton);
                g.lir.ret(mirror)
            });
        }
        let bridge_desc = method_desc(&[product.clone()], &object);
        if !self.plan.has_method("fromProduct", &bridge_desc) {
            let from_product = method_ref(Invoke::Virtual, &self.this_name, false, "fromProduct", &[product.clone()], &singleton);
            self.hold_member(MethodFlags { synthetic: true, bridge: true, ..MethodFlags::PUBLIC }, "fromProduct", &bridge_desc, &[(None, product.clone())], object.clone(), &[], super::classes::MethodSite::default(), "case object's mirror member", |g| {
                let instance = g.lir.this(this);
                let elements = g.param(0);
                let made = g.lir.call(&from_product, instance, &[elements]);
                g.lir.ret(made)
            });
        }
    }

    /// The erased parameters and result of an export forwarder of member `s` whose signature the
    /// qualifier sees as `sig`: the member's own where nothing of it was substituted (a jar's as
    /// its class file has it), else the erasure of the substituted types.
    fn erase_seen_sig(&mut self, s: SymId, sig: &MethodSig) -> (Vec<ErasedType>, ErasedType) {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        let same = info.sig.as_ref().map_or(false, |o| {
            let u = |t: TypeId| cx.input.types.underlying(t);
            u(o.ret) == u(sig.ret) && o.clauses.len() == sig.clauses.len() && o.clauses.iter().zip(&sig.clauses).all(|(a, b)| a.params.len() == b.params.len() && a.params.iter().zip(&b.params).all(|(x, y)| u(x.ty) == u(y.ty)))
        });
        let m = self.mref(s);
        if same || m.params.len() != sig.clauses.iter().map(|c| c.params.len()).sum::<usize>() {
            return (m.params.clone(), m.ret.clone());
        }
        let mut params: Vec<ErasedType> = Vec::new();
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
    pub fn super_accessor_ref(&mut self, t: ClassId, member: SymId) -> MethodRef {
        let cx = self.cx;
        let mut accessor = (*self.mref(member)).clone();
        accessor.name = self.super_accessor_name(t, member);
        accessor.owner = self.class_name(t);
        accessor.owner_is_interface = true;
        accessor.kind = Invoke::Interface;
        let syms = cx.input.syms;
        let (Owner::Class(k), Some(sig)) = (syms.sym(member).owner, syms.sym(member).info.sig.clone()) else { return accessor };
        let base = syms.class(t).base_types.iter().find(|&&(b, _)| b == k).map(|&(_, ty)| ty);
        let base = base.map(|b| match cx.input.types.get(b) {
            Type::Nested(_, class) => class,
            _ => b,
        });
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
                if types.same(seen, upper) {
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
            declared: crate::tir::NO_TYPE,
        };
        let (params, ret) = self.erase_seen_sig(member, &seen);
        accessor.desc = method_desc(&params, &ret);
        accessor.params = params;
        accessor.ret = ret;
        accessor
    }

    /// The abstract accessors of trait `t`, one per member its `super` calls name.
    pub fn super_accessor_declarations(&mut self, t: ClassId) {
        let members = self.cx.super_called.get(&t).cloned().unwrap_or_default();
        for call in members {
            let m = match call.getter {
                Some(i) => self.super_getter_accessor_ref(t, call.member, i as usize),
                None => self.super_accessor_ref(t, call.member),
            };
            if !self.plan.has_method(&m.name, &m.desc) {
                self.abstract_method(MethodFlags { synthetic: true, ..MethodFlags::ABSTRACT }, &m.name, &m.desc);
            }
        }
    }

    /// The super accessors of the traits a class is the first to mix in: `invokespecial` on
    /// the definition that follows the trait in the class's linearisation.
    pub fn super_accessors(&mut self, tc: &crate::tir::TClass) {
        let cx = self.cx;
        let this = ErasedType::L(self.this_name.clone());
        for a in tc.super_accessors.clone() {
            if let Some(i) = a.getter {
                self.super_getter_accessor(tc.id, a.of_trait, a.member, a.target, i as usize);
                continue;
            }
            let m = self.super_accessor_ref(a.of_trait, a.member);
            if self.plan.has_method(&m.name, &m.desc) {
                continue;
            }
            // Bound to a local trait's member that takes proxies: its static method with the
            // ones this class holds (dotty's `ResolveSuper` accessor after `LambdaLift`).
            if let Some(t) = a.target {
                if let (Some(proxies), Owner::Class(o)) = (self.member_proxies(t), cx.input.syms.sym(t).owner) {
                    self.proxied_super_accessor(&m, t, o, &proxies);
                    continue;
                }
            }
            let params: Vec<(Option<SymId>, ErasedType)> = m.params.iter().map(|t| (None, t.clone())).collect();
            let target = a.target.and_then(|t| match cx.input.syms.sym(t).owner {
                Owner::Class(o) => Some((t, o)),
                _ => None,
            });
            // The arguments as the definition the call runs takes them (`Base.id(String)` for
            // the trait's `id(Object)` of a generic parent), as scalac's erasure casts them.
            let taken: Vec<ErasedType> = match target {
                Some((t, _)) => self.mref(t).params.clone(),
                None => self.mref(a.member).params.clone(),
            };
            let special = match target {
                Some((t, o)) => {
                    let tm = self.mref(t);
                    let through = match cx.input.syms.class(tc.id).superclass {
                        Some(sc) if self.object_member_on_java_interface(o, &tm.name, &tm.desc) => sc,
                        _ => self.super_call_owner(tc.id, o),
                    };
                    MethodRef { owner: self.class_name(through), owner_is_interface: cx.input.syms.class(through).kind == ClassKind::Trait, kind: Invoke::Special, ..(*tm).clone() }
                }
                None => MethodRef { owner: Rc::from(OBJECT), owner_is_interface: false, kind: Invoke::Special, ..(*self.mref(a.member)).clone() },
            };
            let special = Rc::new(special);
            self.hold_member(MethodFlags { synthetic: true, ..MethodFlags::PUBLIC }, &m.name, &m.desc, &params, m.ret.clone(), &[], super::classes::MethodSite::default(), "super accessor", |g| {
                let receiver = g.lir.this(this.clone());
                let mut args = Vec::with_capacity(m.params.len());
                for (i, t) in m.params.iter().enumerate() {
                    let given = g.param(i);
                    args.push(match taken.get(i) {
                        Some(to) => g.lower_adapt(given, t, to),
                        None => given,
                    });
                }
                let result = g.lir.call(&special, receiver, &args);
                let result = g.lower_adapt(result, &special.ret, &m.ret);
                g.lir.ret(result)
            });
        }
    }

    /// The accessor through which trait `t` selects the getter of `member`'s default `i` on
    /// `super` (`T$$super$m$default$N`, `SuperAccessors`): the getter's shape, a method of the
    /// trait's interface.
    pub fn super_getter_accessor_ref(&mut self, t: ClassId, member: SymId, i: usize) -> MethodRef {
        let m = self.mref(member);
        let shape = self.default_getter_shape(member, i);
        let mut accessor = (*m).clone();
        accessor.name = format!("{}$default${}", self.super_accessor_name(t, member), shape.index);
        accessor.owner = self.class_name(t);
        accessor.owner_is_interface = true;
        accessor.kind = Invoke::Interface;
        accessor.params = m.params[..shape.before].to_vec();
        accessor.ret = shape.ret.clone();
        accessor.desc = method_desc(&accessor.params, &accessor.ret);
        accessor
    }

    /// A class's implementation of the accessor of a getter selected on `super` in trait `t`:
    /// `invokespecial` of the getter of `target`, the member after the trait in the class's
    /// linearisation that declares the default.
    fn super_getter_accessor(&mut self, c: ClassId, t: ClassId, member: SymId, target: Option<SymId>, i: usize) {
        let cx = self.cx;
        let m = self.super_getter_accessor_ref(t, member, i);
        let Some(target) = target else { return };
        let Owner::Class(o) = cx.input.syms.sym(target).owner else { return };
        if self.plan.has_method(&m.name, &m.desc) {
            return;
        }
        let this = ErasedType::L(self.this_name.clone());
        let params: Vec<(Option<SymId>, ErasedType)> = m.params.iter().map(|t| (None, t.clone())).collect();
        let shape = self.default_getter_shape(target, i);
        let tm = self.mref(target);
        let taken: Vec<ErasedType> = tm.params[..shape.before].to_vec();
        let (name, desc) = self.default_getter_ref(target, i, &taken);
        let through = self.super_call_owner(c, o);
        let owner = self.class_name(through);
        let interface = cx.input.syms.class(through).kind == ClassKind::Trait;
        let (ps, ret) = parse_method_desc(&desc);
        let getter = method_ref(Invoke::Special, &owner, interface, &name, &ps, &ret);
        self.hold_member(MethodFlags { synthetic: true, ..MethodFlags::PUBLIC }, &m.name, &m.desc, &params, m.ret.clone(), &[], super::classes::MethodSite::default(), "super accessor of a default getter", |g| {
            let receiver = g.lir.this(this.clone());
            let mut args = Vec::with_capacity(m.params.len());
            for (k, t) in m.params.iter().enumerate() {
                let given = g.param(k);
                args.push(match taken.get(k) {
                    Some(to) => g.lower_adapt(given, t, to),
                    None => given,
                });
            }
            let result = g.lir.call(&getter, receiver, &args);
            let result = g.lower_adapt(result, &ret, &m.ret);
            g.lir.ret(result)
        });
    }

    /// The accessor `m` of a `super` call bound to the member `t` of the local trait `o` that takes
    /// `proxies`: the trait's static `t$` on `this`, the proxies as this class holds them.
    fn proxied_super_accessor(&mut self, m: &MethodRef, t: SymId, o: ClassId, proxies: &[SymId]) {
        let params: Vec<(Option<SymId>, ErasedType)> = m.params.iter().map(|t| (None, t.clone())).collect();
        let this = ErasedType::L(self.this_name.clone());
        let extras = self.member_extras(t);
        let tm = self.mref(t);
        let pm = self.proxied_mref(&tm, proxies, &extras);
        let owner = self.class_name(o);
        let mut all = vec![ErasedType::L(owner.clone())];
        all.extend(pm.params.iter().cloned());
        let static_t = method_ref(Invoke::Static, &owner, true, &format!("{}$", pm.name), &all, &pm.ret);
        self.hold_member(MethodFlags { synthetic: true, ..MethodFlags::PUBLIC }, &m.name, &m.desc, &params, m.ret.clone(), &[], super::classes::MethodSite::default(), "super accessor", |g| {
            let mut args = vec![g.lir.this(this)];
            for &p in proxies {
                let want = g.anon_capture_type(p);
                args.push(g.lower_capture_as(p, &want));
            }
            args.extend(g.lower_extras(&extras));
            for (i, ty) in m.params.iter().enumerate() {
                let given = g.param(i);
                args.push(match tm.params.get(i) {
                    Some(to) => g.lower_adapt(given, ty, to),
                    None => given,
                });
            }
            let result = g.lir.static_call(&static_t, &args);
            let result = g.lower_adapt(result, &pm.ret, &m.ret);
            g.lir.ret(result)
        });
    }

    /// The class an `invokespecial` of a super call from class `c` names for a method defined
    /// in `target`: `target` itself when it is the superclass or a trait the class mixes in
    /// directly, else the superclass, through which the JVM resolves a default method of a
    /// trait the superclass inherits (an interface `invokespecial` must name a direct
    /// superinterface).
    fn super_call_owner(&self, c: ClassId, target: ClassId) -> ClassId {
        let syms = self.cx.input.syms;
        // A Java interface's method is called on the interface itself, as a written super call's
        // is (`BCodeBodyBuilder.genCallMethod`): the class that holds the call lists it.
        if self.java_interface(target) {
            return target;
        }
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
        let this = ErasedType::L(self.this_name.clone());
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
                // The std declares the JDK's varargs `PrintStream.printf` and `format` with a Scala repeated parameter,
                // so the accessor's descriptor (a `Seq`) finds them where the JDK's class has an array-taking method
                // alone: the class would not link. scalac refuses it (`ResolveSuper.rebindSuper`); the typer's own
                // refusal reads a trait's super calls from its body, which this mode never reads.
                if let Some((b, _)) = &target {
                    if matches!(member, "printf" | "format") && !cx.class_files.contains_key(b) && &*self.class_name(*b) == "java/io/PrintStream" {
                        let (file, span) = (syms.class(c).file, syms.class(c).span);
                        self.refused.push((file, span, format!("not supported on the JVM yet: a trait's super call of PrintStream.{}", member)));
                    }
                }
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
            if self.plan.has_method(&name, &desc) {
                continue;
            }
            let (params, ret) = parse_method_desc(&desc);
            let typed: Vec<(Option<SymId>, ErasedType)> = params.iter().map(|t| (None, t.clone())).collect();
            let special = match target {
                Some((b, member)) => {
                    let through = match syms.class(c).superclass {
                        Some(sc) if self.object_member_on_java_interface(b, &member, &desc) => sc,
                        _ => self.super_call_owner(c, b),
                    };
                    let owner = self.class_name(through);
                    method_ref(Invoke::Special, &owner, syms.class(through).kind == ClassKind::Trait, &member, &params, &ret)
                }
                None => method_ref(Invoke::Special, &Rc::from(OBJECT), false, name.rsplit("$$super$").next().unwrap_or(&name), &params, &ret),
            };
            self.hold_member(MethodFlags::PUBLIC, &name, &desc, &typed, ret.clone(), &[], super::classes::MethodSite::default(), "jar trait's mixin member", |g| {
                let receiver = g.lir.this(this.clone());
                let args: Vec<LirId> = (0..params.len()).map(|i| g.param(i)).collect();
                let result = g.lir.call(&special, receiver, &args);
                g.lir.ret(result)
            });
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
