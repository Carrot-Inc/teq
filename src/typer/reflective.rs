//! Scala.js's reflective instantiation (`std/scalajs/reflect.scala`): once the walk reaches a
//! lookup, the classes and objects that carry `@EnableReflectiveInstantiation` or inherit it
//! are found among the program's classes and the jars' TASTy files, and each gets its
//! registration with the runtime as typed IR, which the class's module runs when it loads.
//!
//! A class registers the constructors that are neither private nor protected, the secondary
//! ones last to first and then the primary one, each with the erased classes of its
//! parameters, made when a lookup first asks since those classes may register after it, and a
//! function from an array of arguments to an instance; an object registers
//! the accessor of its instance. Abstract classes, traits, local classes and objects that are
//! not static register nothing, as under Scala.js.

use super::Worker;
use crate::ast::mods;
use crate::symbols::*;
use crate::tir::TExpr;
use crate::types::*;

const ANNOTATION: &str = "EnableReflectiveInstantiation";

impl<'a> Worker<'a> {
    /// The registrations of the program, by class, in the order of the classes.
    pub fn reflective_registrations(&mut self) -> Vec<(ClassId, crate::tir::TExprId)> {
        let mut candidates: Vec<ClassId> = (0..self.syms.classes.len() as u32)
            .map(ClassId)
            .filter(|&c| {
                let info = self.syms.class(c);
                info.def.is_some() && self.is_program_file(info.file) && !self.is_library_class(c)
            })
            .collect();
        candidates.extend(self.reflective_jar_classes());
        candidates.sort_unstable();
        candidates.dedup();
        let mut annotated: FxMapBool = FxMapBool::default();
        let mut out = Vec::new();
        for c in candidates {
            self.complete_class(c);
            let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
            if !bases.into_iter().any(|b| self.carries_reflect_annotation(b, &mut annotated)) {
                continue;
            }
            if let Some(e) = self.registration(c) {
                out.push((c, e));
            }
        }
        out
    }

    /// The classes of the Scala.js jars' TASTy files that name the annotation, and of those that
    /// name a class or trait that carries or inherits it, of theirs or of the std's (sbt's
    /// test interface, whose frameworks' jars name `Framework`), until no more are found: read
    /// once per session, since the jars do not change.
    fn reflective_jar_classes(&mut self) -> Vec<ClassId> {
        if self.loaded.is_none() {
            return Vec::new();
        }
        self.enter_std_naming(ANNOTATION);
        if let Some(found) = &self.reflective_jar_found {
            return found.clone();
        }
        let mut found: Vec<ClassId> = Vec::new();
        let mut searched: Vec<String> = vec![ANNOTATION.to_string()];
        let mut memo: FxMapBool = FxMapBool::default();
        for c in 0..self.syms.classes.len() as u32 {
            let c = ClassId(c);
            let info = self.syms.class(c);
            let std = info.def.is_some() && self.files.as_slice().get(info.file.0 as usize).map_or(false, |f| f.is_std);
            if std && !self.is_library_class(c) && self.carries_reflect_annotation(c, &mut memo) {
                let name = self.interner.get(self.syms.class(c).name).to_string();
                if !searched.contains(&name) {
                    searched.push(name);
                }
            }
        }
        let mut needles: Vec<String> = searched.clone();
        let mut files_seen: Vec<crate::classpath::CpFile> = Vec::new();
        let mut annotated: FxMapBool = FxMapBool::default();
        while !needles.is_empty() {
            let names: Vec<&str> = needles.iter().map(|s| s.as_str()).collect();
            let hits = self.loaded_mut().cp.scalajs_tasty_files_naming(&names);
            let failures = std::mem::take(&mut self.loaded_mut().cp.scan_failures);
            for (f, why) in failures {
                let file = self.loaded.as_ref().unwrap().jar_files[f.jar as usize];
                self.diags.error(file, crate::source::Span::default(), why);
            }
            let mut fresh: Vec<ClassId> = Vec::new();
            for f in hits {
                if files_seen.contains(&f) {
                    continue;
                }
                files_seen.push(f);
                fresh.extend(self.enter_found_file(f));
            }
            found.extend(fresh.iter().copied());
            needles.clear();
            for c in fresh {
                let info = self.syms.class(c);
                let open = info.kind == ClassKind::Trait || (info.kind == ClassKind::Class && info.mods & mods::FINAL == 0);
                if !open {
                    continue;
                }
                let bases: Vec<ClassId> = info.base_types.iter().map(|&(b, _)| b).collect();
                if bases.into_iter().any(|b| self.carries_reflect_annotation(b, &mut annotated)) {
                    let name = self.interner.get(self.syms.class(c).name).to_string();
                    if !searched.contains(&name) {
                        searched.push(name.clone());
                        needles.push(name);
                    }
                }
            }
        }
        self.reflective_jar_found = Some(found.clone());
        found
    }

    fn carries_reflect_annotation(&mut self, c: ClassId, memo: &mut FxMapBool) -> bool {
        if let Some(&known) = memo.get(&c) {
            return known;
        }
        let annotated = if self.is_library_class(c) {
            self.loaded_reflect_info(c).0
        } else {
            self.source_class_annotated(c)
        };
        memo.insert(c, annotated);
        annotated
    }

    /// Whether an annotation of the program's or the std's class `c` names Scala.js's
    /// annotation class, as the annotation's type resolves where the class is defined: under
    /// any alias, and not a class of the program that takes its name.
    fn source_class_annotated(&mut self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        let (Some(d), file, owner, at) = (info.def, info.file, info.owner, info.span.start) else { return false };
        if owner == Owner::Local {
            return false;
        }
        let ast = self.ast(file);
        let annots = &ast.def(d).annots;
        if annots.is_empty() {
            return false;
        }
        let Some(target) = self.reflect_annotation_class() else { return false };
        let env = self.env_at(file, owner, at);
        self.with_env(env, |t| {
            let mark = t.diags.items.len();
            let found = annots.iter().any(|a| match ast.annot_new(a) {
                Some((ty, _)) => {
                    let resolved = t.resolve_type_ctor(ty);
                    match t.types.get(resolved) {
                        Type::Class(k, _) | Type::Ctor(k) => k == target,
                        _ => false,
                    }
                }
                _ => false,
            });
            t.drop_reported_since(mark);
            found
        })
    }

    fn reflect_annotation_class(&mut self) -> Option<ClassId> {
        let mut p = ROOT_PKG;
        for seg in ["scala", "scalajs", "reflect", "annotation"] {
            let n = self.interner.intern(seg);
            p = self.demand_pkg(p, n)?;
        }
        let n = self.interner.intern(ANNOTATION);
        self.demand_class(p, n)
    }

    /// Whether `f` is a file of the program: none of the std, of a jar or of a library body.
    pub fn is_program_file(&self, f: crate::source::FileId) -> bool {
        self.files.as_slice().get(f.0 as usize).map_or(false, |f| !f.is_std)
    }

    fn registration(&mut self, c: ClassId) -> Option<crate::tir::TExprId> {
        let info = self.syms.class(c);
        if info.js != JsKind::Scala || info.local_module.is_some() {
            return None;
        }
        match info.kind {
            ClassKind::Object if self.is_static_owner(info.owner) => {
                let name = self.binary_name(c);
                let module = self.prog.add(TExpr::Module(c));
                let none = self.prog.syms(&[]);
                let load = self.prog.add(TExpr::Lambda(none, module));
                Some(self.register_call("$reflectModule($0, $1, $2)", &name, c, load))
            }
            ClassKind::Class | ClassKind::EnumCase
                if info.mods & mods::ABSTRACT == 0 && info.singleton.is_none() && self.is_member_owner(info.owner) =>
            {
                let outer = self.outer_class(c);
                let ctors = self.visible_ctors(c);
                if ctors.is_empty() {
                    return None;
                }
                let name = self.binary_name(c);
                let mut items = Vec::with_capacity(ctors.len());
                for (via, params) in ctors {
                    let mut erased: Vec<ClassId> = outer.into_iter().collect();
                    for p in &params {
                        erased.push(self.erased_param_class(p));
                    }
                    // A `Unit` parameter erases to the boxed unit, whose class teq's std does not define.
                    let void = self.prog.add_str("$classNamed(\"java.lang.Void\")");
                    let classes: Vec<crate::tir::TExprId> = erased
                        .iter()
                        .map(|&k| if k == self.b.unit { self.prog.add(TExpr::Js(void, crate::ast::ListRef::EMPTY)) } else { self.prog.add(TExpr::ClassOf(k)) })
                        .collect();
                    let classes = self.prog.list(&classes);
                    let classes = self.prog.add(TExpr::ArrayLit(classes));
                    let none = self.prog.syms(&[]);
                    let classes = self.prog.add(TExpr::Lambda(none, classes));
                    let make = self.ctor_function(c, via, outer, &params);
                    let pair = self.prog.list(&[classes, make]);
                    items.push(self.prog.add(TExpr::ArrayLit(pair)));
                }
                let items = self.prog.list(&items);
                let ctors = self.prog.add(TExpr::ArrayLit(items));
                Some(self.register_call("$reflectClass($0, $1, $2)", &name, c, ctors))
            }
            _ => None,
        }
    }

    fn register_call(&mut self, template: &str, name: &str, c: ClassId, rest: crate::tir::TExprId) -> crate::tir::TExprId {
        let template = self.prog.add_str(template);
        let name = self.prog.add_str(name);
        let name = self.prog.add(TExpr::Str(name));
        let class = self.prog.add(TExpr::ClassOf(c));
        let args = self.prog.list(&[name, class, rest]);
        self.prog.add(TExpr::Js(template, args))
    }

    /// `(a) => new C(a[0], ..)`, or through the secondary constructor `via`, each argument in
    /// the form the parameter's erasure gives it.
    fn ctor_function(&mut self, c: ClassId, via: Option<SymId>, outer: Option<ClassId>, params: &[ParamSig]) -> crate::tir::TExprId {
        let any = self.b.t_any_ref;
        let array = self.types.class(self.b.array, &[any]);
        let a = self.fresh_local("a", array, crate::source::Span::default());
        let arity = params.len() + outer.is_some() as usize;
        let mut args: Vec<crate::tir::TExprId> = Vec::with_capacity(arity);
        for i in 0..arity {
            let r = self.prog.add(TExpr::Local(a));
            let arg = self.prog.add(TExpr::Index(r, i as u32));
            let adapted = match i.checked_sub(outer.is_some() as usize).map(|j| &params[j]) {
                Some(p) if !p.by_name && !p.repeated => self.adapt_argument(p.ty, arg, 0),
                _ => arg,
            };
            args.push(adapted);
        }
        let args = self.prog.list(&args);
        let made = match via {
            Some(s) => self.prog.add(TExpr::NewVia(s, args)),
            None => self.prog.add(TExpr::New(c, args)),
        };
        let params = self.prog.syms(&[a]);
        self.prog.add(TExpr::Lambda(params, made))
    }

    /// An erased argument as the constructor takes it: a `null` for a primitive is its zero, as
    /// Scala.js unboxes it, and a value class's underlying value is wrapped in the value class,
    /// which teq keeps as an object.
    fn adapt_argument(&mut self, t: TypeId, arg: crate::tir::TExprId, depth: u32) -> crate::tir::TExprId {
        let t = self.deref(t);
        let t = self.dealias(t);
        if depth < 16 {
            if let Type::Class(v, args) = self.types.get(t) {
                if self.syms.class(v).value_class {
                    self.complete_class(v);
                    let info = self.syms.class(v);
                    if let Some(under) = info.ctor.first().and_then(|cl| cl.params.first()).map(|p| p.ty) {
                        let subst: Subst = info.tparams.iter().copied().zip(self.types.items(args).iter().copied()).collect();
                        let under = self.types.subst(under, &subst);
                        let inner = self.adapt_argument(under, arg, depth + 1);
                        let l = self.prog.list(&[inner]);
                        return self.prog.add(TExpr::New(v, l));
                    }
                }
            }
        }
        let k = self.erased_class(t, 0);
        match self.primitive_zero(k) {
            Some(zero) => {
                let unbox = self.prog.add_str(super::prims::UNBOX);
                let zero = self.prog.add(zero);
                let l = self.prog.list(&[arg, zero]);
                self.prog.add(TExpr::Js(unbox, l))
            }
            None => arg,
        }
    }

    fn primitive_zero(&self, k: ClassId) -> Option<TExpr> {
        let b = &self.b;
        Some(match k {
            _ if k == b.int || k == b.short || k == b.byte => TExpr::Int(0),
            _ if k == b.double || k == b.float => TExpr::Double(0.0),
            _ if k == b.long => TExpr::Long(0),
            _ if k == b.boolean => TExpr::Bool(false),
            _ if k == b.char => TExpr::Char(0),
            _ => return None,
        })
    }

    /// The constructors a reflective lookup offers, in Scala.js's order, each with the types
    /// of its parameters over all its clauses: `None` stands for the primary one. A protected
    /// constructor is hidden, and a private one unless it is `private[p]`.
    fn visible_ctors(&mut self, c: ClassId) -> Vec<(Option<SymId>, Vec<ParamSig>)> {
        let mut out = Vec::new();
        let library = self.is_library_class(c);
        let secondaries = self.syms.class(c).ctors.clone();
        for &s in secondaries.iter().rev() {
            let hidden = if library {
                self.loaded_ctor_hidden(s)
            } else {
                self.syms.sym(s).mods & mods::PROTECTED != 0 || self.is_private(s)
            };
            if hidden {
                continue;
            }
            let sig = self.sig_of(s);
            let params = sig.clauses.iter().flat_map(|cl| cl.params.iter().cloned()).collect();
            out.push((Some(s), params));
        }
        let info = self.syms.class(c);
        let hidden = if library {
            self.loaded_reflect_info(c).1
        } else {
            info.mods & mods::PROTECTED_CTOR != 0
                || (info.mods & mods::PRIVATE_CTOR != 0 && !self.ast(info.file).access_scopes.iter().any(|&(at, _)| at == info.span.end))
        };
        if !hidden {
            let info = self.syms.class(c);
            let params = info.ctor.iter().flat_map(|cl| cl.params.iter().cloned()).collect();
            out.push((None, params));
        }
        out
    }

    /// Where Scala.js registers a class: a package, or a class, trait or object that is itself
    /// defined there, the outer instance of a class nested in a class or trait coming first
    /// among the constructor's parameters.
    fn is_member_owner(&self, owner: Owner) -> bool {
        match owner {
            Owner::Package(_) => true,
            Owner::Class(o) => {
                let info = self.syms.class(o);
                info.local_module.is_none() && info.inner_object.is_none() && info.kind != ClassKind::Anon && self.is_member_owner(info.owner)
            }
            Owner::Local => false,
        }
    }

    /// A package, or an object that is itself static: where Scala.js registers an object.
    fn is_static_owner(&self, owner: Owner) -> bool {
        match owner {
            Owner::Package(_) => true,
            Owner::Class(o) => {
                let info = self.syms.class(o);
                info.kind == ClassKind::Object && info.local_module.is_none() && self.is_static_owner(info.owner)
            }
            Owner::Local => false,
        }
    }

    /// The name the JVM and Scala.js give the class: `p.Outer$Inner`, `p.Obj$` for an object.
    pub(crate) fn binary_name(&self, c: ClassId) -> String {
        let info = self.syms.class(c);
        let mut s = match info.owner {
            Owner::Package(p) => {
                let path = self.pkg_path(p);
                if path.is_empty() { path } else { path + "." }
            }
            Owner::Class(o) => {
                let mut outer = self.binary_name(o);
                if !outer.ends_with('$') {
                    outer.push('$');
                }
                outer
            }
            Owner::Local => String::new(),
        };
        s.push_str(self.interner.get(info.name));
        if info.kind == ClassKind::Object {
            s.push('$');
        }
        s
    }

    /// The class a parameter erases to, as scalac erases it: a by-name parameter to
    /// `Function0`, a repeated one to `Seq`, the rest by its type.
    fn erased_param_class(&mut self, p: &ParamSig) -> ClassId {
        if p.by_name {
            if let Some(f) = self.b.functions.first().copied().flatten() {
                return f;
            }
        }
        if p.repeated {
            if let Some(seq) = self.b.seq {
                return seq;
            }
        }
        self.erased_class(p.ty, 0)
    }

    /// The class a type erases to: a type parameter to its bound, an alias to what it stands
    /// for, an opaque type and a value class to the underlying type, an intersection to its
    /// class component before a trait (the first of two alike), a union to `Object`. Arrays
    /// are one class on JavaScript, whatever their elements.
    fn erased_class(&mut self, t: TypeId, depth: u32) -> ClassId {
        let t = self.deref(t);
        if depth > 16 {
            return self.b.any_ref;
        }
        match self.types.get(t) {
            Type::Class(c, args) => {
                let kind = self.syms.class(c).kind;
                if kind == ClassKind::Opaque || self.syms.class(c).value_class {
                    self.complete_class(c);
                    let info = self.syms.class(c);
                    let under = if kind == ClassKind::Opaque {
                        info.underlying
                    } else {
                        info.ctor.first().and_then(|cl| cl.params.first()).map(|p| p.ty)
                    };
                    if let Some(under) = under {
                        let subst: Subst = info.tparams.iter().copied().zip(self.types.items(args).iter().copied()).collect();
                        let under = self.types.subst(under, &subst);
                        return self.erased_class(under, depth + 1);
                    }
                    return self.b.any_ref;
                }
                c
            }
            Type::Param(p) | Type::AppParam(p, _) => {
                let upper = self.syms.tparam(p).upper;
                self.erased_class(upper, depth + 1)
            }
            Type::Alias(a, args) => match self.alias_expansion(a, args) {
                Some(expanded) if expanded != t => self.erased_class(expanded, depth + 1),
                _ => self.b.any_ref,
            },
            Type::Inter(a, b) => {
                let (x, y) = (self.erased_class(a, depth + 1), self.erased_class(b, depth + 1));
                let trait_only = |t: &Self, k: ClassId| t.syms.class(k).kind == ClassKind::Trait;
                if trait_only(self, x) && !trait_only(self, y) { y } else { x }
            }
            _ => self.b.any_ref,
        }
    }
}

type FxMapBool = crate::intern::FxMap<ClassId, bool>;
