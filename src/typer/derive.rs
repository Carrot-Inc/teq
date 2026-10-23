//! Scala's derivation protocol. `derives TC` reaches the typer as a given whose right-hand side
//! is `Expr::Derived`, and the givens of `scala.deriving.Mirror` and its refinements are
//! synthesized here, as scalac synthesizes them.

use super::{resolve, Worker};
use crate::ast::{self, mods, TyExpr};
use crate::intern::Name;
use crate::names;
use crate::source::Span;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;

/// The classes of Scala's derivation protocol: `scala.deriving.Mirror` with `Product` and
/// `Sum`, and scala.runtime's classes behind the mirrors the compiler synthesizes.
#[derive(Clone, Copy)]
struct ScalaMirrors {
    mirror: ClassId,
    product: ClassId,
    sum: ClassId,
    singleton: ClassId,
    /// `Mirror.SingletonProxy`, the mirror of a Scala 2 singleton.
    singleton_proxy: Option<ClassId>,
    /// `scala.runtime.TupleMirror`, every tuple's mirror.
    tuple_mirror: Option<ClassId>,
    product_impl: ClassId,
    sum_impl: ClassId,
    singleton_impl: ClassId,
}

#[derive(Clone, Copy)]
struct MirrorNames {
    mono: Name,
    mirrored: Name,
    label: Name,
    elem_types: Name,
    elem_labels: Name,
}

#[derive(Clone, Copy, PartialEq)]
enum Wants {
    Any,
    Product,
    Sum,
}

#[derive(Default)]
pub struct Derivation {
    scala: Option<ScalaMirrors>,
    names: Option<MirrorNames>,
}

enum Shape {
    Product,
    Singleton,
    Sum(Vec<ClassId>),
}

impl Derivation {
    pub(super) fn fork(&mut self) {}

    pub(super) fn to_shared(&mut self, _on: bool) {}

    pub(super) fn merge_own(&mut self) {}

    pub(super) fn apply_pending(&mut self) {}

    pub(super) fn absorb(&mut self, other: &mut Derivation) {
        if self.scala.is_none() {
            self.scala = other.scala;
        }
        if self.names.is_none() {
            self.names = other.names;
        }
    }

    /// Another worker's derivation state over the same mirrors.
    pub(super) fn attach(&self) -> Derivation {
        Derivation { scala: self.scala, names: self.names }
    }

    /// The mirrors under the merge's ids (`merge.rs`).
    pub(super) fn remap(&mut self, _sym: impl Fn(SymId) -> SymId, class: impl Fn(ClassId) -> ClassId) {
        if let Some(m) = &mut self.scala {
            for c in [&mut m.mirror, &mut m.product, &mut m.sum, &mut m.singleton, &mut m.product_impl, &mut m.sum_impl, &mut m.singleton_impl] {
                *c = class(*c);
            }
            for c in [&mut m.singleton_proxy, &mut m.tuple_mirror].into_iter().flatten() {
                *c = class(*c);
            }
        }
    }
}

impl<'a> Worker<'a> {
    /// `scala.deriving.Mirror` and its cases among the std files entered so far, or
    /// scala-library's under `--std=scala-library`, where the jar's package and class are entered
    /// on demand; the values behind the mirrors (`std/mirrors.scala`) are the std's in both modes.
    pub fn find_mirror_classes(&mut self) {
        let library = self.scala_library_std();
        // Entered with the std's classes, the lean std's file too: the parents of a case object
        // name them (`mirror_parent`) whether or not the program names a mirror.
        let pkg = |t: &mut Self, path: [&str; 2]| {
            let mut names = path.map(|n| t.interner.intern(n)).into_iter();
            names.try_fold(ROOT_PKG, |p, n| match t.syms.entry_at(p, n).and_then(|e| e.pkg) {
                Some(sub) => Some(sub),
                None if library => match t.pkg_term(p, n) {
                    Some(resolve::TermRef::Package(p)) => Some(p),
                    _ => None,
                },
                None => t.demand_pkg(p, n),
            })
        };
        let (Some(deriving), Some(runtime)) = (pkg(self, ["scala", "deriving"]), pkg(self, ["scala", "runtime"])) else { return };
        let class = |t: &mut Self, p: PkgId, name: &str| {
            let name = t.interner.intern(name);
            if let Some(c) = t.syms.entry_at(p, name).and_then(|e| e.class) {
                return Some(c);
            }
            if !library {
                return t.demand_class(p, name);
            }
            match t.pkg_type(p, name) {
                Some(resolve::TypeRef::Class(c)) => Some(c),
                _ => None,
            }
        };
        let Some(mirror) = class(self, deriving, "Mirror") else { return };
        // The runtime's mirror classes come with `Mirror` whether or not the program names them.
        for name in ["ProductMirror", "SumMirror", "SingletonMirror"] {
            let n = self.interner.intern(name);
            self.demand_class(runtime, n);
        }
        self.complete_class(mirror);
        let Some(module) = self.syms.class(mirror).companion else { return };
        self.complete_class(module);
        let nested = |t: &mut Self, name: &str| {
            let name = t.interner.intern(name);
            t.syms.class(module).nested.get(&name).copied()
        };
        let (Some(product), Some(sum), Some(singleton)) = (nested(self, "Product"), nested(self, "Sum"), nested(self, "Singleton")) else { return };
        let (Some(product_impl), Some(sum_impl), Some(singleton_impl)) =
            (class(self, runtime, "ProductMirror"), class(self, runtime, "SumMirror"), class(self, runtime, "SingletonMirror"))
        else {
            return;
        };
        let singleton_proxy = nested(self, "SingletonProxy");
        let tuple_mirror = class(self, runtime, "TupleMirror");
        self.derive.scala = Some(ScalaMirrors { mirror, product, sum, singleton, singleton_proxy, tuple_mirror, product_impl, sum_impl, singleton_impl });
        // The members a companion's `fromProduct` and `ordinal` implement (`addMirrorSupport`),
        // whose signatures the backends' bridges read (`Erasure`): complete before any body, which
        // would publish what it holds in the middle of a class.
        for (k, name) in [(product, "fromProduct"), (sum, "ordinal")] {
            self.complete_class(k);
            let name = self.interner.intern(name);
            if let Some(decl) = self.syms.class(k).members.get(&name).copied() {
                let _ = self.sig_of(decl);
            }
        }
    }

    /// The mirror classes, entering `scala.deriving.Mirror` where no file entered so far defines
    /// it: a desugared parent asks for them whether or not the program names a mirror.
    fn demanded_mirror_classes(&mut self) -> Option<ScalaMirrors> {
        self.derive.scala
    }

    /// Whether `b` is `scala.deriving.Mirror` or its `Product`, `Sum` or `Singleton`.
    pub(crate) fn is_mirror_class(&self, b: ClassId) -> bool {
        self.derive.scala.is_some_and(|m| b == m.mirror || b == m.product || b == m.singleton || b == m.sum)
    }

    /// dotty's `is(Scala2x)`: a class read from a Scala 2 library, or the lean std's class standing
    /// for one of scala-library's compiled from Scala 2's sources.
    pub(super) fn is_scala2x(&self, c: ClassId) -> bool {
        self.is_scala2_class(c) || self.is_std_scala2_class(c)
    }

    /// The parent `SyntheticMembers.addMirrorSupport` gives `c`: `Mirror.Singleton` to a case
    /// object, and to an enum's singleton case (`DesugarEnums`' `ExtendsSingletonMirror`), but of
    /// a Java enum; none to a class of Scala 2, whose objects scalac mirrors through a
    /// `Mirror.SingletonProxy`. `Mirror.Product` to the companion of a generic product, and
    /// `Mirror.Sum` to an enum's, where the companion is the mirror (`mirror_companion`).
    pub(super) fn mirror_parent(&mut self, c: ClassId) -> Option<ClassId> {
        let (kind, case, singleton, companion) = {
            let info = self.syms.class(c);
            (info.kind, info.mods & mods::CASE != 0, info.singleton.is_some(), info.companion)
        };
        let sm = self.demanded_mirror_classes()?;
        match kind {
            ClassKind::EnumCase => (singleton && !self.is_java_enum_value(c) && !self.is_scala2x(c)).then_some(sm.singleton),
            _ if case && self.mirror_module(c) => (!self.is_scala2x(c)).then_some(sm.singleton),
            _ if self.mirror_module(c) => {
                let k = companion?;
                if self.mirror_companion(k) != Some(c) {
                    return None;
                }
                if self.syms.class(k).kind == ClassKind::Enum {
                    return Some(sm.sum);
                }
                // The companion of a sealed class of the program's sources that is a generic sum
                // (`useCompanionAsSumMirror`'s `isDefinedInSource`, `isGenericSum`), its children
                // known once its file's classes are complete.
                match self.sealed_companion_class(c, k) {
                    Some(true) => {
                        self.complete_sealed_file(k);
                        matches!(self.mirror_shape(k), Some(Shape::Sum(_))).then_some(sm.sum)
                    }
                    Some(false) => None,
                    None => self.is_generic_product_by_definition(k).then_some(sm.product),
                }
            }
            _ => None,
        }
    }

    /// The members `SyntheticMembers.addMirrorSupport` gives the companion `o` that is its
    /// class's mirror (`mirror_parent`), where it has no definition of the name (`existingDef`):
    /// of a generic product's, `fromProduct(x$0: Product)`, the class made of the product's
    /// elements, each cast to its field's type (`fromProductBody`); of an enum's, `ordinal(x$0)`,
    /// the value's ordinal (`ordinalBody`). JavaScript and the interpreter run them
    /// (`TClassLists::mirror_members`); the JVM backend and the pickle write them by rule; a
    /// companion read from scalac's pickle has them as written.
    /// The mirror members of the typed class `ti` of `c`, made where a check that only reports
    /// left them (`emits_bodies`): the interpreter running a macro calls them. A shared class's
    /// were made with it (`check_class_now`).
    pub(crate) fn ensure_mirror_members(&mut self, c: ClassId, ti: usize) {
        if self.emits_bodies || self.shared_class(c) || !self.prog.classes[ti].mirror_members().is_empty() {
            return;
        }
        let mirror = self.mirror_members(c);
        if !mirror.is_empty() {
            self.prog.classes[ti].lists_mut().mirror_members = mirror;
        }
    }

    pub(super) fn mirror_members(&mut self, o: ClassId) -> Vec<FunId> {
        let (companion, file, span) = {
            let info = self.syms.class(o);
            (info.companion, info.file, info.span)
        };
        let (Some(sm), Some(k)) = (self.derive.scala, companion) else { return Vec::new() };
        if !self.mirror_module(o) || self.mirror_companion(k) != Some(o) {
            return Vec::new();
        }
        let (product, sum) = {
            let bases = &self.syms.class(o).base_types;
            (bases.iter().any(|&(b, _)| b == sm.product), bases.iter().any(|&(b, _)| b == sm.sum))
        };
        let (from_product, ordinal) = (self.interner.intern("fromProduct"), self.interner.intern("ordinal"));
        let declares = |t: &Self, n: Name| t.syms.class(o).members.contains_key(&n);
        let env = self.env_at(file, Owner::Class(o), span.start);
        if product && !self.is_tuple_class(k) && self.is_generic_product_class(k) && !declares(self, from_product) {
            let Some(decl) = self.syms.class(sm.product).members.get(&from_product).copied() else { return Vec::new() };
            return self.with_env(env, |t| vec![t.from_product_member(o, k, decl, span)]);
        }
        if sum && !declares(self, ordinal) {
            let Some(decl) = self.syms.class(sm.sum).members.get(&ordinal).copied() else { return Vec::new() };
            let Some(Shape::Sum(children)) = self.mirror_shape(k) else { return Vec::new() };
            return self.with_env(env, |t| vec![t.ordinal_member(o, k, &children, decl, span)]);
        }
        Vec::new()
    }

    /// `fromProduct` of the companion `o` of `k` (`mirror_members`).
    fn from_product_member(&mut self, o: ClassId, k: ClassId, decl: SymId, span: Span) -> FunId {
        let product_ty = self.b.product.map_or(ANY, |p| self.types.class(p, &[]));
        let x0 = self.interner.intern("x$0");
        let param = self.new_local(x0, SymKind::Val, product_ty, span);
        let element = self.b.product.and_then(|p| {
            let name = self.interner.intern("productElement");
            self.syms.class(p).members.get(&name).copied()
        });
        let template = element.and_then(|sym| self.syms.sym(sym).intrinsic.clone());
        let anys: Vec<TypeId> = self.syms.class(k).tparams.iter().map(|_| ANY).collect();
        let subst: Subst = self.syms.class(k).tparams.iter().map(|&tp| (tp, ANY)).collect();
        let fields: Vec<TypeId> = self.syms.class(k).ctor.first().map_or(Vec::new(), |cl| cl.params.iter().map(|p| p.ty).collect());
        let values: Vec<TExprId> = fields
            .into_iter()
            .enumerate()
            .map(|(i, t)| {
                let p_ref = self.prog.add(TExpr::Local(param));
                let index = self.prog.add(TExpr::Int(i as i32));
                let value = match (&template, element) {
                    (Some(text), Some(sym)) => {
                        let s = self.prog.add_str(text);
                        // Named by its member, as a call's template is (`Product.productElement`).
                        self.prog.template_syms.insert(s, sym);
                        if self.forked && !self.prog.template_syms.to_shared {
                            self.pending_shared.push(super::PendingShared::Template(s, sym));
                        }
                        let l = self.prog.list(&[p_ref, index]);
                        self.prog.add(TExpr::Js(s, l))
                    }
                    (None, Some(sym)) => {
                        let args = self.prog.list(&[index]);
                        self.prog.add(TExpr::CallMethod(p_ref, sym, args))
                    }
                    _ => self.prog.add(TExpr::Unit),
                };
                let t = self.types.subst(t, &subst);
                self.lower_cast(value, ANY, t, span).0
            })
            .collect();
        let values = self.prog.list(&values);
        let body = self.new_instance(k, values, span);
        let ret = self.types.class(k, &anys);
        self.mirror_member(o, decl, &[(param, x0, product_ty)], ret, body, span)
    }

    /// `ordinal` of the companion `o` of the sum `k` of the cases `children` (`mirror_members`).
    fn ordinal_member(&mut self, o: ClassId, k: ClassId, children: &[ClassId], decl: SymId, span: Span) -> FunId {
        let anys: Vec<TypeId> = self.syms.class(k).tparams.iter().map(|_| ANY).collect();
        let sum_ty = self.types.class(k, &anys);
        let x0 = self.interner.intern("x$0");
        let param = self.new_local(x0, SymKind::Val, sum_ty, span);
        let is_enum = self.syms.class(k).kind == ClassKind::Enum;
        let body = self.ordinal_of(param, children, is_enum);
        let int = self.b.t_int;
        self.mirror_member(o, decl, &[(param, x0, sum_ty)], int, body, span)
    }

    /// A member of `o` named and dispatched as `decl`, of the parameters and the body given.
    fn mirror_member(&mut self, o: ClassId, decl: SymId, params: &[(SymId, Name, TypeId)], ret: TypeId, body: TExprId, span: Span) -> FunId {
        let file = self.syms.class(o).file;
        let name = self.syms.sym(decl).name;
        let msym = self.syms.new_sym(name, SymKind::Def, mods::OVERRIDE, Owner::Class(o), file, None, span);
        let sig = MethodSig {
            tparams: Vec::new(),
            clauses: vec![ClauseSig {
                params: params.iter().map(|&(sym, name, ty)| ParamSig { name, ty, by_name: false, repeated: false, has_default: false, sym }).collect(),
                is_using: false,
                is_implicit: false,
            }],
            ret,
            declared: crate::tir::NO_TYPE,
        };
        {
            let mut s = self.syms.sym_mut(msym);
            s.sig = Some(std::sync::Arc::new(sig));
            s.state().set(Completion::Done);
        }
        self.name_like(msym, decl);
        let f = self.prog.add_fun(TFun {
            sym: msym,
            defaults: vec![None; params.len()],
            params: params.iter().map(|&(s, _, _)| s).collect(),
            body: Some(body),
            body_unconsuming: false,
            captures: 0,
            lifted: false,
        });
        self.fun_of_sym.insert(msym, f);
        f
    }

    /// The classes of the file of the sealed class `k` but local ones, completed: the children of
    /// a sealed class are in its file and known once complete (`ClassInfo::children`).
    fn complete_sealed_file(&mut self, k: ClassId) {
        let file = self.syms.class(k).file;
        let mut classes: Vec<ClassId> = self.def_classes.values_in(file.0 as usize);
        classes.sort_unstable();
        for c in classes {
            let mut owner = self.syms.class(c).owner;
            while let Owner::Class(o) = owner {
                owner = self.syms.class(o).owner;
            }
            if owner != Owner::Local {
                self.complete_class(c);
            }
        }
    }

    /// The children of the sealed class `k` in its mirror's order (`mirror_shape`).
    pub(crate) fn sum_mirror_children(&mut self, k: ClassId) -> Vec<ClassId> {
        match self.mirror_shape(k) {
            Some(Shape::Sum(children)) => children,
            _ => Vec::new(),
        }
    }

    /// The type a child of a sum is tested as (`SyntheticMembers.ordinalBody`'s `rawRef`): an
    /// object's or a value's singleton, a class of any arguments.
    pub(crate) fn mirror_child_type(&mut self, child: ClassId) -> TypeId {
        let info = self.syms.class(child);
        if let Some(v) = info.singleton {
            return self.types.mk(Type::Term(v));
        }
        if let Some(s) = info.module_sym.filter(|_| info.kind == ClassKind::Object) {
            return self.types.mk(Type::Term(s));
        }
        let n = info.tparams.len();
        self.types.class(child, &vec![WILD; n])
    }

    /// Whether `o` extends `Mirror.Sum`, as a companion that is its sum's mirror does.
    pub(crate) fn extends_sum_mirror(&mut self, o: ClassId) -> bool {
        let Some(sm) = self.derive.scala else { return false };
        self.complete_class(o);
        self.syms.class(o).base_types.iter().any(|&(b, _)| b == sm.sum)
    }

    /// Whether `o`, the companion of `k`, decides its mirror parent as the companion of a sealed
    /// class: `Some(true)` where `k` is the program's (`useCompanionAsSumMirror`'s
    /// `isDefinedInSource`) and the file's classes decide it, `Some(false)` where `k` is a library's.
    pub(super) fn sealed_companion_class(&self, o: ClassId, k: ClassId) -> Option<bool> {
        let ki = self.syms.class(k);
        let sealed = ki.mods & mods::SEALED != 0 && (ki.kind == ClassKind::Trait || (ki.kind == ClassKind::Class && ki.mods & mods::ABSTRACT != 0));
        (sealed && self.mirror_module(o)).then(|| ki.def.is_some() && !self.is_library_class(k))
    }

    /// Whether the mirror parent of the object `o` waits for `o`'s written parents: the companion
    /// of a sealed class of the program, whose file's classes decide it and may look into `o`.
    pub(super) fn defers_mirror_parent(&mut self, o: ClassId) -> bool {
        let Some(k) = self.syms.class(o).companion else { return false };
        self.syms.class(k).kind != ClassKind::Enum && self.sealed_companion_class(o, k) == Some(true) && self.mirror_companion(k) == Some(o)
    }

    /// `isGenericProduct` of a companion's class, as `SyntheticMembers` asks it after typing: read
    /// off the definition of a class of the program not complete yet, whose completion may look
    /// into the companion being completed (`case class Owner(o: Owner.Id)` with `object Owner
    /// extends Typed[Owner]`); the case flag, `Abstract`, one clause and no value class, as
    /// `whyNotGenericProduct` reads them.
    fn is_generic_product_by_definition(&mut self, k: ClassId) -> bool {
        let (file, def, done) = {
            let info = self.syms.class(k);
            (info.file, info.def, info.state() == Completion::Done)
        };
        let Some(def) = def.filter(|_| !done) else { return self.is_generic_product_class(k) };
        let info = self.syms.class(k);
        if info.mods & mods::CASE == 0 || !matches!(info.kind, ClassKind::Class | ClassKind::EnumCase) || info.singleton.is_some() || info.mods & mods::ABSTRACT != 0 {
            return false;
        }
        let ast = self.ast(file);
        let ast::DefKind::Class(cls) = &ast.def(def).kind else { return false };
        let value_class = cls.parents.iter().any(|p| match ast.ty(p.ty) {
            TyExpr::Name(n) => n == names::ANY_VAL,
            TyExpr::Select(_, n) => n == names::ANY_VAL,
            _ => false,
        });
        cls.clauses.len() <= 1 && cls.clauses.iter().all(|cl| !cl.is_using) && !value_class
    }

    /// `SymUtils.isGenericProduct` but its access, which the summoning site checks: a case class,
    /// not abstract and no value class, of one parameter clause.
    pub(super) fn is_generic_product_class(&mut self, k: ClassId) -> bool {
        self.complete_class(k);
        let info = self.syms.class(k);
        info.mods & mods::CASE != 0
            && matches!(info.kind, ClassKind::Class | ClassKind::EnumCase)
            && info.singleton.is_none()
            && info.mods & mods::ABSTRACT == 0
            && !info.value_class
            && info.ctor.len() <= 1
            && info.ctor.iter().all(|cl| !cl.is_using)
    }

    fn mirror_names(&mut self) -> MirrorNames {
        if let Some(n) = self.derive.names {
            return n;
        }
        let n = MirrorNames {
            mono: self.interner.intern("MirroredMonoType"),
            mirrored: self.interner.intern("MirroredType"),
            label: self.interner.intern("MirroredLabel"),
            elem_types: self.interner.intern("MirroredElemTypes"),
            elem_labels: self.interner.intern("MirroredElemLabels"),
        };
        self.derive.names = Some(n);
        n
    }

    /// Whether every value of `t` is a `Product`: a class that extends the trait (a case class or
    /// object and an enum by their desugaring), or a tuple, whose traits the lean std leaves
    /// without the parent scala-library gives them.
    pub fn is_product(&mut self, t: TypeId) -> bool {
        self.every_value_by_rule(t, |w, c| {
            let b = &w.b;
            if [b.product, b.tuple_trait, b.non_empty_tuple].contains(&Some(c)) || w.is_tuple_class(c) {
                return true;
            }
            w.complete_class(c);
            let info = w.syms.class(c);
            info.base_types.iter().any(|&(base, _)| Some(base) == w.b.product || Some(base) == w.b.tuple_trait)
        })
    }

    /// Whether every value of `t` is of a class `class_by_rule` takes: both sides of a union, one of
    /// an intersection, a type parameter's bound, what a path or an alias stands for.
    fn every_value_by_rule(&mut self, t: TypeId, class_by_rule: impl Fn(&mut Self, ClassId) -> bool + Copy) -> bool {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Class(c, _) => class_by_rule(self, c),
            Type::Union(x, y, _) => self.every_value_by_rule(x, class_by_rule) && self.every_value_by_rule(y, class_by_rule),
            Type::Inter(x, y) => self.every_value_by_rule(x, class_by_rule) || self.every_value_by_rule(y, class_by_rule),
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                upper != ANY && self.every_value_by_rule(upper, class_by_rule)
            }
            Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..) | Type::Match(..) | Type::Alias(..) | Type::Nested(..) | Type::Rec(_) => {
                match self.dependent_underlying(t) {
                    Some(u) => self.every_value_by_rule(u, class_by_rule),
                    None => false,
                }
            }
            _ => false,
        }
    }

    /// Whether one of the parents `desugared_parents` gave `c` is a mirror's (`mirror_parent`).
    pub(super) fn has_desugared_mirror_parent(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        let from = info.parents.len() - (info.desugared_parents as usize).min(info.parents.len());
        info.parents[from..].iter().any(|&t| self.types.named_class(t).is_some_and(|k| self.is_mirror_class(k)))
    }

    /// `scala.deriving.Mirror.Singleton`, once the derivation protocol's classes are found.
    pub fn scala_singleton_mirror(&self) -> Option<ClassId> {
        self.derive.scala.map(|m| m.singleton)
    }

    /// Whether `b` is a parent dotty's `Desugar.classDef` gives a case class, a case object or an
    /// enum, or one of its bases: `Product` with `Equals`, `java.io.Serializable`,
    /// `scala.reflect.Enum`; or one `SyntheticMembers.addMirrorSupport` gives: `Mirror.Singleton`
    /// with `Mirror.Product` and `Mirror`, `Mirror.Sum`.
    #[inline(always)]
    pub fn is_desugared_parent(&self, b: ClassId) -> bool {
        if Some(b) == self.b.product || Some(b) == self.b.reflect_enum || Some(b) == self.b.equals {
            return true;
        }
        if let Some(m) = self.derive.scala {
            if b == m.mirror || b == m.product || b == m.singleton || b == m.sum {
                return true;
            }
        }
        // The one `java.io.Serializable` of its package, once looked up (`java_serializable`).
        match self.java_serializable {
            Some(found) => found == Some(b),
            None => self.syms.is_synthetic_parent(self.interner, b),
        }
    }

    /// A class of package `scala.reflect` as the standard library defines it.
    pub(super) fn entered_scala_reflect_class(&mut self, name: &str) -> Option<ClassId> {
        let reflect = self.interner.intern("reflect");
        let name = self.interner.intern(name);
        let pkg = self.syms.entry_at(self.b.scala_pkg, reflect).and_then(|e| e.pkg)?;
        self.syms.entry_at(pkg, name).and_then(|e| e.class)
    }

    /// A member of `Product` on a tuple, which the lean std's tuple traits do not extend;
    /// `productPrefix` of one is the builtin's (`prims.rs`).
    pub fn product_member(&mut self, recv_ty: TypeId, name: Name) -> Option<(SymId, TypeId)> {
        let product = self.b.product?;
        if name == names::PRODUCT_PREFIX || !self.syms.class(product).members.contains_key(&name) || !self.is_product(recv_ty) {
            return None;
        }
        let pt = self.types.class(product, &[]);
        self.find_member(pt, name)
    }

    /// Whether `c` is `scala.deriving.Mirror`, `Mirror.Product` or `Mirror.Sum`, whose givens the
    /// compiler synthesizes.
    pub fn derive_mirror_class(&self, c: ClassId) -> bool {
        matches!(self.derive.scala, Some(sm) if c == sm.mirror || c == sm.product || c == sm.sum)
    }

    /// The given for `Mirror.Of[T]`, `Mirror.ProductOf[T]`, `Mirror.SumOf[T]` and the other
    /// refinements of `Mirror`, `Mirror.Product` or `Mirror.Sum` that fix `MirroredType`: the
    /// mirror of `T` as scalac synthesizes it, a value of scala.runtime's mirror classes typed as
    /// the parent refined with the mirror's type members. None, with the reasons recorded for
    /// the error, where scalac refuses.
    /// The refinements of `t` and of the aliases under them: `Mirror.ProductOf[P] { type
    /// MirroredElemTypes = E }` refines an alias whose own refinement names `MirroredType`.
    fn refinements_through_aliases(&mut self, t: TypeId) -> Vec<RefineId> {
        let mut all = Vec::new();
        let mut at = t;
        loop {
            let (base, rs) = self.types.refinements_of(at);
            all.extend(rs);
            let under = self.dealias(base);
            if under == base {
                return all;
            }
            at = under;
        }
    }

    /// Whether `target` asks of the mirror what the mirror of the class may not have, which
    /// scalac checks: its element types and labels (`Mirror.ProductOf[P] { type
    /// MirroredElemTypes = E }`) beyond `Mirror`'s own `<: Tuple`, and a mono type other than the
    /// mirrored one, and a term member. scalac does not check the label, the mirrored type's
    /// bounds or the mono type's.
    pub fn mirror_target_fixes_members(&mut self, target: TypeId) -> bool {
        let names = self.mirror_names();
        let target = self.deref(target);
        let (_, rs) = self.types.refinements_of(target);
        let mirrored = rs.iter().find_map(|&r| match self.types.refinement(r) {
            Refinement::Alias(n, t) if n == names.mirrored => Some(t),
            _ => None,
        });
        let tuple = self.b.tuple_trait;
        rs.iter().any(|&r| match self.types.refinement(r) {
            Refinement::Alias(n, t) if n == names.mono => Some(self.canon(t)) != mirrored.map(|m| self.canon(m)),
            Refinement::Alias(n, _) => n == names.elem_types || n == names.elem_labels,
            Refinement::Bounds(n, lo, hi) if n == names.elem_types || n == names.elem_labels => {
                self.canon(lo) != NOTHING || !matches!(self.types.get(hi), Type::Class(c, _) if Some(c) == tuple)
            }
            Refinement::Bounds(..) => false,
            // A term member no mirror has.
            Refinement::Term(..) | Refinement::Val(..) => true,
        })
    }

    /// The element types and labels the target fixes, checked against the mirror's as dotty's
    /// `Synthesizer.checkRefinement` checks them: the target's member of the name, an alias or
    /// bounds, contains the mirror's (`Elems` of `fromProductTyped`'s using clause solved by it);
    /// where it does not, the error is reported, and the mirror is the search's all the same.
    /// Whether an error was reported.
    pub fn check_mirror_refinements(&mut self, target: TypeId, mirror: TypeId, span: Span) -> bool {
        let names = self.mirror_names();
        let target = self.deref(target);
        let wanted = self.refinements_through_aliases(target);
        let mirror = self.deref(mirror);
        let (_, own) = self.types.refinements_of(mirror);
        let mut reported = false;
        for name in [names.elem_types, names.elem_labels] {
            let Some(expected) = own.iter().find_map(|&r| match self.types.refinement(r) {
                Refinement::Alias(n, t) if n == name => Some(t),
                _ => None,
            }) else {
                continue;
            };
            // The member as `findMember` meets the refinements of the name, the alias's under
            // them first: an alias where one fixes it, else the bounds' union and intersection.
            let mut alias = None;
            let (mut lo, mut hi) = (NOTHING, ANY);
            for &r in wanted.iter().rev() {
                match self.types.refinement(r) {
                    Refinement::Alias(n, t) if n == name => alias = Some(t),
                    Refinement::Bounds(n, l, h) if n == name => {
                        if self.canon(l) != NOTHING {
                            lo = if lo == NOTHING { l } else { self.types.union(lo, l) };
                        }
                        if self.canon(h) != ANY {
                            hi = if hi == ANY { h } else { self.types.inter(hi, h) };
                        }
                    }
                    _ => {}
                }
            }
            if alias.is_none() && lo == NOTHING && hi == ANY {
                continue;
            }
            let (lo, hi) = alias.map_or((lo, hi), |t| (t, t));
            let mark = self.snapshot();
            if self.is_sub(lo, expected) && self.is_sub(expected, hi) {
                continue;
            }
            self.rollback(&mark);
            let found = match alias {
                Some(t) => format!(" = {}", self.show(t)),
                None if lo == NOTHING => format!(" <: {}", self.show(hi)),
                None => format!(" >: {} <: {}", self.show(lo), self.show(hi)),
            };
            let msg = format!("{} mismatch, expected: {}, found: {}.", self.name_str(name), self.show(expected), found);
            // Where the search's argument stands, the end of what is applied (`tree.span.endPos`).
            self.error(Span::new(span.end, span.end), msg);
            reported = true;
        }
        reported
    }

    pub fn scala_mirror_given(&mut self, class: ClassId, target: TypeId, site: Span) -> Option<(TExprId, TypeId)> {
        let sm = self.derive.scala?;
        let wants = if class == sm.mirror {
            Wants::Any
        } else if class == sm.product {
            Wants::Product
        } else if class == sm.sum {
            Wants::Sum
        } else {
            return None;
        };
        let names = self.mirror_names();
        // The mirrored type solved, the target's other variables left to the refinements' check
        // (`fromProductTyped`'s `Elems`, which `checkRefinement`'s `contains` constrains).
        let target = self.zonk(target);
        let rs = self.refinements_through_aliases(target);
        let mirrored = rs.iter().find_map(|&r| match self.types.refinement(r) {
            Refinement::Alias(n, t) if n == names.mirrored => Some(t),
            _ => None,
        })?;
        let mirrored = self.solve_bounded_in(mirrored);
        let mirrored = self.normalize(mirrored);
        if mirrored == ERROR || self.types.has_vars(mirrored) {
            return None;
        }
        let (c, subject, params) = match self.mirror_ctor_source(mirrored) {
            Some(hk) => hk,
            None => {
                let (c, subject) = self.mirror_source(mirrored)?;
                (c, subject, Vec::new())
            }
        };
        self.complete_class(c);
        let shape = self.mirror_shape(c).filter(|shape| match shape {
            Shape::Sum(children) => children.iter().all(|&child| self.mirror_ctor_accessible(child)),
            _ => self.mirror_ctor_accessible(c),
        });
        let fits = match (&shape, wants) {
            (None, _) => false,
            (Some(Shape::Sum(_)), Wants::Product) => false,
            (Some(Shape::Product | Shape::Singleton), Wants::Sum) => false,
            _ => true,
        };
        if !fits {
            self.refuse_mirror(c, wants, mirrored, None);
            return None;
        }
        let shape = shape?;
        if let Shape::Sum(children) = &shape {
            if let Some(why) = self.inaccessible_child(c, children, subject) {
                self.refuse_mirror(c, wants, mirrored, Some(why));
                return None;
            }
        }
        let ty = self.scala_mirror_type(c, &shape, mirrored, subject, &params, names, sm);
        // `Synthesizer`'s mirror: the companion, the singleton, a `TupleMirror` (`mirror_ref`), or
        // an anonymous mirror made at the site, cast to the mirror's type.
        let made = match self.mirror_ref(c, &shape, subject, sm) {
            Some(value) => Some(value),
            None => self.anonymous_mirror(c, &shape, subject, &params, sm, site),
        };
        if let Some(value) = made {
            let value = self.mirror_cast(value, ty, sm);
            return Some((value, ty));
        }
        // Where no anonymous class can be made (a quote's body) or no path names the object (an
        // enum's case of a class or a block), the mirror is one of the runtime's mirror classes made
        // at the site, a product nested in a class with the prefix's value as its enclosing
        // instance (`MirrorImpl.OfProduct(pre)`).
        let outer = match (self.types.get(subject), &shape) {
            (Type::Nested(p, _), Shape::Product) if self.outer_class(c).is_some() => self.prefix_value(p),
            _ => None,
        };
        let impl_class = match &shape {
            Shape::Product => sm.product_impl,
            Shape::Singleton => sm.singleton_impl,
            Shape::Sum(_) => sm.sum_impl,
        };
        let span = self.syms.class(c).span;
        let now = outer.is_some() || self.new_prefixed;
        let prefixed = std::mem::replace(&mut self.new_prefixed, now);
        let value = self.build_scala_mirror(c, &shape, impl_class, span, outer);
        self.new_prefixed = prefixed;
        let value = self.lower_cast(value, ty, ty, Span::default()).0;
        Some((value, ty))
    }

    /// The mirror `Synthesizer` hands out for `c`, as a path or a construction: of a product
    /// the companion (`companionPath`, `useCompanionAsProductMirror`), of a tuple a
    /// `TupleMirror` of its arity; of a sum the companion of an enum (`useCompanionAsSumMirror`);
    /// of a singleton the object (`singletonPath`), a Scala 2 one's in a `SingletonProxy`. None
    /// where teq's targets have no such companion: a class nested in a class or a block, whose
    /// companion they lay out by rule; a sealed class's written companion, which they make no
    /// `Mirror.Sum`; a class of Scala 2.
    /// The companion `o` of the mirrored type `subject` as `Synthesizer.companionPath` (304) reads
    /// it: the module of a static owner; a block's object's val; an object of a class's instance
    /// through the type's prefix, or through the enclosing instance where the site is in it.
    fn companion_path(&mut self, o: ClassId, subject: TypeId) -> Option<TExprId> {
        if let Some(this) = self.module_within(o) {
            return Some(this);
        }
        let (owner, local, inner) = {
            let info = self.syms.class(o);
            (info.owner, info.local_module, info.inner_object)
        };
        match (local, inner) {
            (Some(v), _) => Some(self.prog.add(TExpr::Local(v))),
            (None, Some(v)) => match self.types.get(subject) {
                Type::Nested(pre, _) => {
                    let recv = self.prefix_value(pre)?;
                    Some(self.prog.add(TExpr::Field(recv, v)))
                }
                // A local enum's class case's companion, an object of the enum's companion, a
                // block's object: through that object's val.
                _ => self.inner_object_ref(v).or_else(|| {
                    let Owner::Class(h) = owner else { return None };
                    let lv = self.syms.class(h).local_module?;
                    let recv = self.prog.add(TExpr::Local(lv));
                    Some(self.prog.add(TExpr::Field(recv, v)))
                }),
            },
            (None, None) if self.is_static_owner(owner) => Some(self.prog.add(TExpr::Module(o))),
            (None, None) => None,
        }
    }

    /// The module of the class `o` where the code stands in `o`'s body, as `tpd.ref` gives it there:
    /// `This(o)` through the outer links (`followOuterLinks`), not its accessor, which the module
    /// under construction would call again.
    fn module_within(&mut self, o: ClassId) -> Option<TExprId> {
        if !self.env.frames.iter().any(|f| matches!(f, super::Frame::Class(k) if *k == o)) {
            return None;
        }
        Some(self.this_ref(o))
    }

    fn mirror_ref(&mut self, c: ClassId, shape: &Shape, subject: TypeId, sm: ScalaMirrors) -> Option<TExprId> {
        let _ = subject;
        match shape {
            Shape::Singleton => {
                let (owner, singleton, local, inner) = {
                    let info = self.syms.class(c);
                    (info.owner, info.singleton, info.local_module, info.inner_object)
                };
                // The object's path: a local object's val, an inner one's through the prefix; in
                // its own body, the object itself.
                let within = self.module_within(c);
                let path = match (singleton, local, inner) {
                    _ if within.is_some() => within.unwrap(),
                    (Some(sym), _, _) if self.is_static_owner(owner) => self.prog.add(TExpr::Static(sym)),
                    (Some(_), _, _) => return None,
                    (None, Some(v), _) => self.prog.add(TExpr::Local(v)),
                    (None, None, Some(v)) => {
                        let Type::Nested(pre, _) = self.types.get(subject) else { return None };
                        let recv = self.prefix_value(pre)?;
                        self.prog.add(TExpr::Field(recv, v))
                    }
                    (None, None, None) if self.is_static_owner(owner) => self.prog.add(TExpr::Module(c)),
                    (None, None, None) => return None,
                };
                match sm.singleton_proxy.filter(|_| self.is_scala2x(c)) {
                    Some(proxy) => {
                        let args = self.prog.list(&[path]);
                        Some(self.prog.add(TExpr::New(proxy, args)))
                    }
                    None => Some(path),
                }
            }
            Shape::Product if self.is_tuple_class(c) => {
                let tm = sm.tuple_mirror?;
                self.complete_class(tm);
                let n = self.syms.class(c).ctor.first().map_or(0, |cl| cl.params.len());
                let arity = self.prog.add(TExpr::Int(n as i32));
                self.prog.set_type(arity, self.b.t_int);
                let args = self.prog.list(&[arity]);
                let made = self.prog.add(TExpr::New(tm, args));
                let tm_ty = self.types.class(tm, &[]);
                self.prog.set_type(made, tm_ty);
                Some(made)
            }
            Shape::Product => {
                let o = self.mirror_companion(c)?;
                self.companion_path(o, subject)
            }
            // The companion of an enum, or of a sealed class that is its mirror
            // (`useCompanionAsSumMirror`): one that extends `Mirror.Sum`.
            Shape::Sum(_) => {
                let o = self.mirror_companion(c).filter(|&o| self.extends_sum_mirror(o))?;
                self.companion_path(o, subject)
            }
        }
    }

    /// `Synthesizer.anonymousMirror`, the mirror of a class whose companion is none (`mirror_ref`):
    /// an instance of a class made at the site, `Mirror.Product` or `Mirror.Sum` with `type
    /// MirroredMonoType` the mirrored type, whose `fromProduct` makes the class of the product's
    /// elements, each cast to its field's type, and whose `ordinal` tests the value's case.
    fn anonymous_mirror(&mut self, c: ClassId, shape: &Shape, subject: TypeId, params: &[TypeId], sm: ScalaMirrors, span: Span) -> Option<TExprId> {
        let parent = match shape {
            Shape::Product => sm.product,
            Shape::Sum(_) => sm.sum,
            Shape::Singleton => return None,
        };
        if self.quote.level > 0 {
            return None;
        }
        // A type constructor's mono type is the class over its parameters' bounds
        // (`mkMirroredMonoType`), whose elements its `fromProduct` takes as `Any`.
        let (mono, subst): (TypeId, Option<Subst>) = if params.is_empty() {
            (subject, None)
        } else {
            let bounds: Vec<TypeId> = params.iter().map(|&p| match self.types.get(p) {
                Type::Param(id) => self.syms.tparam(id).upper,
                _ => ANY,
            }).collect();
            let anys: Subst = self.syms.class(c).tparams.iter().map(|&tp| (tp, ANY)).collect();
            (self.types.class(c, &bounds), Some(anys))
        };
        let trait_ty = self.types.class(parent, &[]);
        let shape = match shape {
            Shape::Sum(children) => Shape::Sum(children.clone()),
            Shape::Product => Shape::Product,
            Shape::Singleton => Shape::Singleton,
        };
        let is_enum = self.syms.class(c).kind == ClassKind::Enum;
        let mono_name = self.mirror_names().mono;
        let (te, _) = self.sam_instance_with_types(trait_ty, 1, span, &[(mono_name, mono)], |t, params| {
            let (p, _) = params[0];
            match &shape {
                Shape::Sum(children) => t.ordinal_of(p, children, is_enum),
                _ => {
                    let subst = subst.unwrap_or_else(|| t.owner_subst(subject));
                    t.product_of(c, p, subject, subst, span)
                }
            }
        })?;
        Some(te)
    }

    /// `new C(..)` of the elements of the product `p` holds, each cast to its field's type as
    /// `subject` sees it, the class nested in a class made in the instance its prefix names.
    fn product_of(&mut self, c: ClassId, p: SymId, subject: TypeId, subst: Subst, span: Span) -> TExprId {
        let element = self.b.product.and_then(|pc| {
            let name = self.interner.intern("productElement");
            self.syms.class(pc).members.get(&name).copied()
        });
        let template = element.and_then(|sym| self.syms.sym(sym).intrinsic.clone());
        let fields: Vec<TypeId> = self.syms.class(c).ctor.first().map_or(Vec::new(), |cl| cl.params.iter().map(|p| p.ty).collect());
        let values: Vec<TExprId> = fields
            .into_iter()
            .enumerate()
            .map(|(i, t)| {
                let p_ref = self.prog.add(TExpr::Local(p));
                let index = self.prog.add(TExpr::Int(i as i32));
                let value = match (&template, element) {
                    (Some(text), Some(sym)) => {
                        let s = self.prog.add_str(text);
                        self.prog.template_syms.insert(s, sym);
                        if self.forked && !self.prog.template_syms.to_shared {
                            self.pending_shared.push(super::PendingShared::Template(s, sym));
                        }
                        let l = self.prog.list(&[p_ref, index]);
                        self.prog.add(TExpr::Js(s, l))
                    }
                    (None, Some(sym)) => {
                        let args = self.prog.list(&[index]);
                        self.prog.add(TExpr::CallMethod(p_ref, sym, args))
                    }
                    _ => self.prog.add(TExpr::Unit),
                };
                let t = self.types.subst(t, &subst);
                self.lower_cast(value, ANY, t, span).0
            })
            .collect();
        let n = values.len();
        let xxl = if self.is_tuple_class(c) && n > 22 { self.tuple_xxl_value(&values, span) } else { None };
        let values = self.prog.list(&values);
        // A class nested in a class is made in the instance its prefix names (`MirrorImpl
        // .OfProduct(pre)`), which the creation takes in place of its own enclosing one.
        let outer = match self.types.get(subject) {
            Type::Nested(pre, _) if self.outer_class(c).is_some() => self.prefix_value(pre),
            _ => None,
        };
        let now = outer.is_some() || self.new_prefixed;
        let prefixed = std::mem::replace(&mut self.new_prefixed, now);
        let instance = match xxl {
            Some(te) => te,
            None => self.new_instance(c, values, span),
        };
        self.new_prefixed = prefixed;
        if let Some(outer) = outer {
            self.pass_outer(instance, outer);
        }
        instance
    }

    /// The ordinal of the value `a` holds among `children`: an enum value's own, else the index of
    /// the first case it is an instance of.
    fn ordinal_of(&mut self, a: SymId, children: &[ClassId], is_enum: bool) -> TExprId {
        let a_ref = self.prog.add(TExpr::Local(a));
        if is_enum {
            let template = self.prog.add_str("$0.$ordinal");
            let args = self.prog.list(&[a_ref]);
            return self.prog.add(TExpr::Js(template, args));
        }
        let mut ordinal = self.prog.add(TExpr::Int(children.len() as i32 - 1));
        for (i, &child) in children.iter().enumerate().rev().skip(1) {
            let test = self.class_test(child);
            let a_ref = self.prog.add(TExpr::Local(a));
            let is_child = self.prog.add(TExpr::TypeTest(a_ref, test));
            if self.capturing() {
                // The case's class, of any arguments (`case _: C[?] => i`).
                let n = self.syms.class(child).tparams.len();
                let t = self.types.class(child, &vec![ANY; n]);
                self.capture_form(is_child, crate::tir::capture::Form::Test(t));
            }
            let index = self.prog.add(TExpr::Int(i as i32));
            ordinal = self.prog.add(TExpr::If(is_child, index, Some(ordinal)));
        }
        ordinal
    }

    /// The companion of `k` that is its mirror (`SymUtils.useCompanionAsProductMirror`,
    /// `useCompanionAsSumMirror`): an object, no case object, of a class of Scala 3, placed
    /// statically, where the targets give it the mirror's members by rule
    /// (`SyntheticMembers.addMirrorSupport`, `mirror_members`).
    /// Whether `c` is a module's class: an object's, or a class teq makes of an object nested in a
    /// class or a block (`inner_object`, `local_module`).
    pub(super) fn mirror_module(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        info.kind == ClassKind::Object || info.inner_object.is_some() || info.local_module.is_some()
    }

    pub(super) fn mirror_companion(&mut self, k: ClassId) -> Option<ClassId> {
        let o = self.syms.class(k).companion?;
        let oi = self.syms.class(o);
        // A companion nested in a class or a block is the mirror too, read through its prefix
        // (`useCompanionAsProductMirror` 166 has no owner's restriction; `companionPath` 304).
        if !self.mirror_module(o) || oi.mods & mods::CASE != 0 {
            return None;
        }
        (!self.is_scala2x(k)).then_some(o)
    }

    /// The mirror cast to its type, `Synthesizer`'s `mirrorRef.cast(mirrorType)`: the companion
    /// or the object is the mirror on every target, so the cast tests nothing; the pickle writes
    /// it as scalac's (`Wrap::Mirror`).
    fn mirror_cast(&mut self, value: TExprId, ty: TypeId, _sm: ScalaMirrors) -> TExprId {
        // The cast of a path tests nothing: the companion's or the object's class extends the
        // mirror's trait on every target (`mirror_parent`), so its erasure conforms and `Erasure`
        // drops the `$asInstanceOf$` (`TypeTestsCasts.interceptTypeApply`, `CastOp::Erased`). An
        // anonymous mirror's instance is typed `Object & Serializable` (`anonymousMirror`), whose
        // erasure does not, so the mirror's trait is tested, as scalac's `checkcast` tests it. The
        // node is a cast's tree either way, of the mirror's type and no path (`MARK_CAST`), which
        // an inline argument's copy keeps: `p.MirroredElemTypes` of a summoned mirror is the
        // refinement's, not a member of the companion's singleton, and a macro's pattern reads the
        // refinement.
        // The operand's type as dotty's tree has it: an anonymous mirror's instance ascribed its
        // class's parents but the mirror's (`Object & Serializable`), an enum's value its declared
        // type (the enum), an object its class, a construction its class.
        let from = match self.prog.expr(value) {
            TExpr::New(c, _) if self.syms.class(c).kind == ClassKind::Anon => {
                let parents = self.syms.class(c).parents.clone();
                parents.into_iter().filter(|&p| !self.types.named_class(p).is_some_and(|k| self.is_mirror_class(k))).fold(self.b.t_any_ref, |acc, p| self.types.inter(acc, p))
            }
            TExpr::New(c, _) | TExpr::Module(c) => self.types.class(c, &[]),
            // A companion's own `this` in its body (`module_within`).
            TExpr::This => self.this_class().map_or(ANY, |c| self.types.class(c, &[])),
            TExpr::Static(v) | TExpr::Local(v) | TExpr::Field(_, v) => self.sig_of(v).ret,
            _ => self.prog.type_of(value).unwrap_or(ANY),
        };
        // The companion or the object has the mirror's trait from `addMirrorSupport`, after typing
        // (`ClassInfo::post_typer_bases`): `Erasure`, which sees it, drops the cast.
        let post_typer = self.class_of(from).is_some_and(|k| self.syms.class(k).post_typer_bases > 0);
        let op = match self.cast_lowering(from, ty, false) {
            _ if post_typer => CastOp::Erased,
            super::prims::CastLowering::Op(op) => op,
            _ => CastOp::Erased,
        };
        let value = self.prog.add(TExpr::Cast(value, op, ty));
        *self.expr_marks.entry(value).or_default() |= super::MARK_CAST | super::MARK_CALL | super::MARK_RETYPED;
        // The tree is of the mirror's type, which a quote's pattern reads (`$m: Mirror.SumOf[A]
        // { type MirroredLabel = l }`).
        self.prog.set_type(value, ty);
        self.end_chain(value, ty);
        if self.capturing() {
            self.capture_wrap(value, crate::tir::capture::Wrap::Mirror(ty));
        }
        value
    }

    /// A mirrored type constructor (`type MirroredType = Tree`, or `[X] =>> Tree[X]`), as
    /// scalac mirrors higher-kinded types: the class, the class type over the constructor's
    /// parameters, and those parameters.
    fn mirror_ctor_source(&mut self, mirrored: TypeId) -> Option<(ClassId, TypeId, Vec<TypeId>)> {
        let mirrored = self.deref(mirrored);
        match self.types.get(mirrored) {
            Type::Ctor(c) => {
                self.settle_class(c);
                let params: Vec<TypeId> = self.syms.class(c).tparams.clone().iter().map(|&p| {
                    let name = self.syms.tparam(p).name;
                    let fresh = self.syms.new_tparam(name, 0);
                    self.types.param(fresh)
                }).collect();
                let subject = self.types.class(c, &params);
                Some((c, subject, params))
            }
            // Only the class over the lambda's own parameters, as `[X] =>> Tree[X]` is `Tree`;
            // `[X] =>> List[Tree[X]]` is no class and has no mirror.
            Type::Lambda(ps, body) => match self.types.get(self.types.strip_nested(body)) {
                Type::Class(c, args)
                    if self.types.items(args).len() == self.types.items(ps).len()
                        && self.types.items(args).iter().zip(self.types.items(ps)).all(|(&a, &p)| self.same_type(a, p)) =>
                {
                    Some((c, body, self.types.items(ps).to_vec()))
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// The class a mirrored type is the mirror of, with the class type it is seen as: a class
    /// type, the class of an object or an enum value, and of an intersection the more specific
    /// side, as scalac's `MirrorSource` reduces it.
    fn mirror_source(&mut self, mirrored: TypeId) -> Option<(ClassId, TypeId)> {
        let mirrored = self.deref(mirrored);
        match self.types.get(self.types.strip_nested(mirrored)) {
            Type::Class(c, _) => Some((c, mirrored)),
            Type::Term(s) => match self.syms.sym(s).kind {
                SymKind::EnumValue(c) | SymKind::Object(c) => Some((c, self.types.class(c, &[]))),
                _ => None,
            },
            Type::Inter(a, b) => {
                let (left, right) = (self.mirror_source(a)?, self.mirror_source(b)?);
                let mark = self.snapshot();
                let picked = if self.is_sub(left.1, right.1) {
                    Some(left)
                } else if self.is_sub(right.1, left.1) {
                    Some(right)
                } else {
                    None
                };
                self.rollback(&mark);
                picked
            }
            _ => None,
        }
    }

    /// The mirror's type: the parent refined with `MirroredMonoType`, `MirroredType`,
    /// `MirroredLabel`, `MirroredElemTypes` and `MirroredElemLabels`.
    fn scala_mirror_type(&mut self, c: ClassId, shape: &Shape, mirrored: TypeId, subject: TypeId, params: &[TypeId], names: MirrorNames, sm: ScalaMirrors) -> TypeId {
        let parent = match shape {
            Shape::Sum(_) => sm.sum,
            Shape::Product => sm.product,
            // A Scala 2 singleton's mirror is its `SingletonProxy`, any other's the object itself.
            Shape::Singleton => match sm.singleton_proxy {
                Some(proxy) if self.is_scala2x(c) => proxy,
                _ => sm.singleton,
            },
        };
        let (elem_types, elem_names): (Vec<TypeId>, Vec<Name>) = match shape {
            Shape::Product => {
                let subst = self.owner_subst(subject);
                let fields: Vec<(TypeId, Name)> =
                    self.syms.class(c).ctor.first().map_or(Vec::new(), |clause| clause.params.iter().map(|p| (p.ty, p.name)).collect());
                fields.into_iter().map(|(t, n)| (self.types.subst(t, &subst), n)).unzip()
            }
            Shape::Singleton => (Vec::new(), Vec::new()),
            Shape::Sum(children) => children.iter().map(|&child| (self.case_type(child, subject), self.syms.class(child).name)).unzip(),
        };
        let mut elems_ty = self.tuple_of(&elem_types);
        // A type constructor's element types are a lambda over its parameters, and its mono
        // type the class over their bounds.
        let mut mono = mirrored;
        if !params.is_empty() {
            let ps = self.types.list(params);
            elems_ty = self.types.mk(Type::Lambda(ps, elems_ty));
            let bounds: Vec<TypeId> = params.iter().map(|&p| match self.types.get(p) {
                Type::Param(id) => self.syms.tparam(id).upper,
                _ => ANY,
            }).collect();
            mono = self.types.class(c, &bounds);
            // Through the subject's prefix, as `mkMirroredMonoType` applies the lambda.
            if let Type::Nested(p, _) = self.types.get(subject) {
                mono = self.nested_type(p, mono);
            }
        }
        let labels: Vec<TypeId> = elem_names.iter().map(|&n| self.types.lit(LitVal::Str(n))).collect();
        let labels_ty = self.tuple_of(&labels);
        let label = self.types.lit(LitVal::Str(self.syms.class(c).name));
        let mut t = self.types.class(parent, &[]);
        // A singleton's is `mirrorCore` alone: its parent fixes the elements.
        let members = if matches!(shape, Shape::Singleton) { 3 } else { 5 };
        for (n, rhs) in [(names.mono, mono), (names.mirrored, mirrored), (names.label, label), (names.elem_types, elems_ty), (names.elem_labels, labels_ty)].into_iter().take(members) {
            let r = self.types.refine(Refinement::Alias(n, rhs));
            t = self.types.mk(Type::Refined(t, r));
        }
        t
    }

    /// The mirror object, whose constructions of the product take `outer` as their enclosing
    /// instance where it is given, that of the summon site otherwise.
    fn build_scala_mirror(&mut self, c: ClassId, shape: &Shape, impl_class: ClassId, span: Span, outer: Option<TExprId>) -> TExprId {
        let arg = match shape {
            Shape::Product => {
                let product_ty = self.b.product.map_or(ANY, |p| self.types.class(p, &[]));
                let element = self.b.product.and_then(|p| {
                    let name = self.interner.intern("productElement");
                    self.syms.class(p).members.get(&name).copied()
                });
                let p = self.fresh_local("p", product_ty, span);
                let n = self.syms.class(c).ctor.first().map_or(0, |clause| clause.params.len());
                let template = element.and_then(|sym| self.syms.sym(sym).intrinsic.clone());
                let values: Vec<TExprId> = (0..n)
                    .map(|i| {
                        let p_ref = self.prog.add(TExpr::Local(p));
                        let index = self.prog.add(TExpr::Int(i as i32));
                        match (&template, element) {
                            (Some(t), _) => {
                                let s = self.prog.add_str(t);
                                // Named by its member, as a call's template is, which a macro's
                                // run answers on any target (`Product.productElement`).
                                if let Some(sym) = element {
                                    self.prog.template_syms.insert(s, sym);
                                    if self.forked && !self.prog.template_syms.to_shared {
                                        self.pending_shared.push(super::PendingShared::Template(s, sym));
                                    }
                                }
                                let l = self.prog.list(&[p_ref, index]);
                                self.prog.add(TExpr::Js(s, l))
                            }
                            (None, Some(sym)) => {
                                let args = self.prog.list(&[index]);
                                self.prog.add(TExpr::CallMethod(p_ref, sym, args))
                            }
                            (None, None) => self.prog.add(TExpr::Unit),
                        }
                    })
                    .collect();
                // A tuple past 22 elements is a TupleXXL (`typer/arity.rs`).
                let xxl = if self.is_tuple_class(c) && n > 22 { self.tuple_xxl_value(&values, span) } else { None };
                let values = self.prog.list(&values);
                // A case class nested in a class takes the enclosing instance the summon site
                // holds, as scalac's mirror, its companion, holds it.
                let instance = match xxl {
                    Some(te) => te,
                    None => self.new_instance(c, values, span),
                };
                if let Some(outer) = outer {
                    self.pass_outer(instance, outer);
                }
                let params = self.prog.syms(&[p]);
                self.prog.add(TExpr::Lambda(params, instance))
            }
            Shape::Singleton => match self.syms.class(c).singleton {
                Some(sym) => self.prog.add(TExpr::Static(sym)),
                None => self.prog.add(TExpr::Module(c)),
            },
            Shape::Sum(children) => {
                let is_enum = self.syms.class(c).kind == ClassKind::Enum;
                self.ordinal_function(children, is_enum, span)
            }
        };
        let args = self.prog.list(&[arg]);
        self.prog.add(TExpr::New(impl_class, args))
    }

    /// scalac's explanation of why no mirror exists, in the place of the message a missing
    /// given gets.
    #[cold]
    fn refuse_mirror(&mut self, c: ClassId, wants: Wants, mirrored: TypeId, sum_problem: Option<String>) {
        if self.implicit_depth != 1 {
            return;
        }
        let of = match wants {
            Wants::Any => "Of",
            Wants::Product => "ProductOf",
            Wants::Sum => "SumOf",
        };
        let shown = format!("Mirror.{}[{}]", of, self.show(mirrored));
        let desc = self.class_description(c);
        let mut msg = format!("No given instance of type {} was found.
Failed to synthesize an instance of type {}:", shown, shown);
        if wants != Wants::Sum {
            msg.push_str(&format!("
	* {} is not a generic product because {}", desc, self.why_not_generic_product(c)));
        }
        if wants != Wants::Product {
            let why = sum_problem.unwrap_or_else(|| self.why_not_generic_sum(c));
            msg.push_str(&format!("
	* {} is not a generic sum because {}", desc, why));
        }
        self.given_ambiguity = Some(msg);
    }

    /// dotty's `TypeOps.childPrefix` (TypeOps.scala 1008): the prefix of `child`, a child of
    /// `parent` seen through `pre`. The owners the two share are stripped; `pre` is unwound by
    /// as many steps as the parent has owners left (a `this` to its class's prefix, a path to
    /// the path it selects from), and the child's remaining owners, objects, are selected from
    /// it: `o.mid.T`'s child of `O` is seen through `o`, a child in `O`'s `object Kids` through
    /// `o.Kids`. None where no instance prefix comes of it (a static owner, a local value).
    pub(super) fn child_prefix(&mut self, pre: TypeId, parent: ClassId, child: ClassId) -> Option<TypeId> {
        let chain = |w: &Self, c: ClassId| -> Vec<Owner> {
            let mut owners = Vec::new();
            let mut at = w.syms.class(c).owner;
            loop {
                owners.push(at);
                at = match at {
                    Owner::Class(k) => w.syms.class(k).owner,
                    Owner::Package(p) => match w.syms.pkg(p).parent {
                        Some(q) => Owner::Package(q),
                        None => break,
                    },
                    Owner::Local => break,
                };
            }
            owners.reverse();
            owners
        };
        let (parent_owners, child_owners) = (chain(self, parent), chain(self, child));
        let common = parent_owners.iter().zip(&child_owners).take_while(|(a, b)| a == b).count();
        // `subPrefixAt`: a `this` steps to its class's prefix, a selection to what it selects
        // from; a value with no prefix ends the walk.
        let mut pre = pre;
        for _ in common..parent_owners.len() {
            pre = match self.types.get(pre) {
                Type::Select(q, _) => q,
                Type::This(k) => match self.syms.class(k).owner {
                    Owner::Class(o) if self.is_module_class(o) && self.syms.class(o).inner_object.is_none() => self.types.class(o, &[]),
                    Owner::Class(o) => self.types.mk(Type::This(o)),
                    _ => return None,
                },
                _ => return None,
            };
        }
        // `selectAll`: the child's owners below the common one, each an object of the prefix.
        for &owner in &child_owners[common..] {
            let Owner::Class(o) = owner else { return None };
            pre = match self.syms.class(o).inner_object {
                Some(val) => self.types.mk(Type::Select(pre, val)),
                None if self.syms.class(o).kind == ClassKind::Object => self.types.class(o, &[]),
                None => return None,
            };
        }
        Some(pre)
    }

    /// dotty's `whyNotGenericSum(pre)`: a child of the sum `c` seen as `subject` has to be
    /// reachable from the parent (its owner encloses `c`, or is an object reachable so) and, but
    /// where the parent's companion holds the mirror, from the summon site (its owner encloses the
    /// site, or is a base of the singleton prefix's class, or an object reachable so): the children
    /// of `Outer#Item` summoned outside `Outer` are not.
    fn inaccessible_child(&mut self, c: ClassId, children: &[ClassId], subject: TypeId) -> Option<String> {
        // Children of packages and of objects in packages, the common case, are reachable.
        let static_owner = |w: &Self, child: ClassId| {
            let mut owner = w.syms.class(child).owner;
            while let Owner::Class(o) = owner {
                if w.syms.class(o).kind != ClassKind::Object {
                    return false;
                }
                owner = w.syms.class(o).owner;
            }
            true
        };
        if children.iter().all(|&child| static_owner(self, child)) {
            return None;
        }
        let companion_mirror = self.syms.class(c).companion.is_some_and(|k| self.syms.class(k).mods & mods::CASE == 0);
        let prefix = match self.types.get(subject) {
            Type::Nested(p, _) => Some(p),
            _ => match self.syms.class(c).owner {
                Owner::Class(o) if self.is_inner_class(c) => Some(self.types.mk(Type::This(o))),
                _ => None,
            },
        };
        // The classes of a singleton prefix (`pre.classSymbols`).
        let scope: Vec<ClassId> = match prefix {
            Some(p) if self.types.is_path(p) => {
                let under = self.widen_path(p);
                self.class_of(under).into_iter().collect()
            }
            Some(p) => match self.types.get(p) {
                Type::Class(k, _) if self.is_module_class(k) => vec![k],
                _ => Vec::new(),
            },
            None => Vec::new(),
        };
        let encloses = |w: &Self, outer: ClassId, inner: ClassId| {
            let mut at = Owner::Class(inner);
            while let Owner::Class(k) = at {
                if k == outer {
                    return true;
                }
                at = w.syms.class(k).owner;
            }
            false
        };
        for &child in children {
            // A class owner (an object or a package recurses outward, as a module does).
            let mut owner = self.syms.class(child).owner;
            let mut to_parent = false;
            while let Owner::Class(o) = owner {
                if encloses(self, o, c) {
                    to_parent = true;
                    break;
                }
                if !self.is_module_class(o) && self.syms.class(o).local_module.is_none() {
                    break;
                }
                owner = self.syms.class(o).owner;
            }
            let to_parent = to_parent || !matches!(owner, Owner::Class(_));
            if !to_parent {
                return Some(format!("its child {} is not accessible to its parent {}", self.class_description(child), self.class_description(c)));
            }
            if companion_mirror {
                continue;
            }
            let mut owner = self.syms.class(child).owner;
            let mut to_site = false;
            while let Owner::Class(o) = owner {
                let reachable = self.env.frames.iter().any(|f| matches!(f, super::Frame::Class(k) if *k == o));
                let of_prefix = scope.iter().any(|&k| {
                    let mut at = Owner::Class(k);
                    while let Owner::Class(x) = at {
                        if x == o || self.syms.class(x).base_types.iter().any(|&(b, _)| b == o) {
                            return true;
                        }
                        at = self.syms.class(x).owner;
                    }
                    false
                });
                if reachable || of_prefix {
                    to_site = true;
                    break;
                }
                if !self.is_module_class(o) && self.syms.class(o).local_module.is_none() {
                    break;
                }
                owner = self.syms.class(o).owner;
            }
            if !to_site && matches!(owner, Owner::Class(_)) {
                return Some(format!("its child {} is not accessible to the call site", self.class_description(child)));
            }
        }
        None
    }

    /// A case class's companion holds its mirror and reaches its constructor; only where the
    /// companion is a case object, which cannot, the mirror stands at the call site, which
    /// scalac refuses when the constructor is out of its reach.
    fn mirror_ctor_accessible(&mut self, c: ClassId) -> bool {
        let companion_is_case = self.syms.class(c).companion.map_or(false, |k| self.syms.class(k).mods & mods::CASE != 0);
        !companion_is_case || self.ctor_accessible(c)
    }

    fn why_not_generic_product(&mut self, c: ClassId) -> String {
        let info = self.syms.class(c);
        if info.mods & mods::CASE == 0 {
            "it is not a case class".to_string()
        } else if info.mods & mods::ABSTRACT != 0 {
            "it is an abstract class".to_string()
        } else if info.ctor.len() > 1 {
            "it takes more than one parameter list".to_string()
        } else if !self.mirror_ctor_accessible(c) {
            format!("the constructor of {} is inaccessible from the calling scope.", self.class_description(c))
        } else {
            "it is not a case class".to_string()
        }
    }

    fn why_not_generic_sum(&mut self, c: ClassId) -> String {
        let info = self.syms.class(c);
        let sealed = info.kind == ClassKind::Enum || info.mods & mods::SEALED != 0;
        let is_abstract = matches!(info.kind, ClassKind::Trait | ClassKind::Enum) || info.mods & mods::ABSTRACT != 0;
        if !sealed {
            format!("it is not a sealed {}", if info.kind == ClassKind::Trait { "trait" } else { "class" })
        } else if !is_abstract {
            "it is not an abstract class".to_string()
        } else if info.children.is_empty() {
            "it does not have subclasses".to_string()
        } else {
            let children = info.children.clone();
            match children.into_iter().find(|&child| self.mirror_shape(child).is_none() || !self.mirror_ctor_accessible(child)) {
                Some(child) => format!("its child {} is not a generic product", self.class_description(child)),
                None => "it is not a sealed class".to_string(),
            }
        }
    }

    /// The signature of `derives TC` on `C[A, ...]` where `TC` takes a type constructor of
    /// `C`'s arity: the instance is `TC[C]` with no parameters, as scalac derives it, in the
    /// place of the parser's `[A: TC, ...]: TC[C[A, ...]]`.
    pub(super) fn derived_ctor_sig(&mut self, g: &ast::GivenDef) -> Option<MethodSig> {
        let ast = self.cur_ast();
        if g.tparams.is_empty() || !matches!(g.alias.map(|a| ast.expr(a)), Some(ast::Expr::Derived)) {
            return None;
        }
        let TyExpr::Apply(tc, args) = ast.ty(g.ty) else { return None };
        let &[subject] = ast.ty_list(args) else { return None };
        let TyExpr::Apply(head, _) = ast.ty(subject) else { return None };
        let tc_ty = self.resolve_type_ctor(tc);
        let Type::Ctor(k) = self.types.get(tc_ty) else { return None };
        self.complete_class(k);
        let &[p] = self.syms.class(k).tparams.as_slice() else { return None };
        if self.param_arity(p) != g.tparams.len() {
            return None;
        }
        let head_ty = self.resolve_type_ctor(head);
        if !matches!(self.types.get(head_ty), Type::Ctor(_)) {
            return None;
        }
        let ret = self.types.class(k, &[head_ty]);
        Some(MethodSig { tparams: Vec::new(), clauses: Vec::new(), ret, declared: crate::tir::NO_TYPE })
    }

    /// `TC.derived` for a given of type `TC[T]`.
    pub fn type_derived(&mut self, expected: Option<TypeId>, span: Span) -> (TExprId, TypeId) {
        let failed = (self.prog.add(TExpr::Unit), ERROR);
        match expected {
            Some(target) if target != ERROR => self.derived_call(target, span).unwrap_or(failed),
            _ => failed,
        }
    }

    fn derived_call(&mut self, target: TypeId, span: Span) -> Option<(TExprId, TypeId)> {
        let companion = self.class_of(target).and_then(|tc| {
            self.complete_class(tc);
            self.syms.class(tc).companion.filter(|&c| self.syms.class(c).kind == ClassKind::Object)
        });
        // The companion's own `derived`, or an extension on the companion in scope (kittens'
        // `extension (x: Eq.type) inline def derived[A]`, through `import cats.derived.*`).
        let provided = |t: &mut Self, c: ClassId| {
            let module_ty = t.types.class(c, &[]);
            t.module_term(c, names::DERIVED).is_some() || t.has_lexical_extension_for(module_ty, names::DERIVED)
        };
        let Some(companion) = companion.filter(|&c| provided(self, c)) else {
            let msg = format!(
                "{} cannot be derived: the companion of the type class has no derived method",
                self.show(target)
            );
            self.error(span, msg);
            return None;
        };
        let module = self.prog.add(TExpr::Module(companion));
        let module_ty = self.types.class(companion, &[]);
        Some(self.apply_member(module, module_ty, names::DERIVED, None, Vec::new(), span, Some(target)))
    }

    fn mirror_shape(&mut self, c: ClassId) -> Option<Shape> {
        self.complete_class(c);
        let info = self.syms.class(c);
        let is_case = info.mods & mods::CASE != 0;
        let is_abstract = info.kind == ClassKind::Trait || (info.kind == ClassKind::Class && info.mods & mods::ABSTRACT != 0);
        let is_sum = info.kind == ClassKind::Enum || (is_abstract && info.mods & mods::SEALED != 0);
        match info.kind {
            ClassKind::EnumCase if info.singleton.is_some() => Some(Shape::Singleton),
            ClassKind::Object if is_case => Some(Shape::Singleton),
            ClassKind::Class | ClassKind::EnumCase if is_case && info.mods & mods::ABSTRACT == 0 => {
                // `construct` can only fill in one explicit parameter list.
                let constructible = info.ctor.len() <= 1 && info.ctor.iter().all(|clause| !clause.is_using);
                constructible.then_some(Shape::Product)
            }
            _ if is_sum => {
                // Children are recorded as they get completed; `ordinal` follows the source, and
                // an enum's cases are numbered so, a product's by their places in its source.
                let mut children = info.children.clone();
                if info.kind == ClassKind::Enum {
                    children.sort_by_key(|&child| self.syms.class(child).ordinal);
                } else {
                    children.sort_by_key(|&child| self.syms.class(child).span.start);
                }
                let derivable = !children.is_empty() && children.iter().all(|&child| self.mirror_shape(child).is_some());
                derivable.then_some(Shape::Sum(children))
            }
            _ => None,
        }
    }

    fn class_test(&mut self, c: ClassId) -> TestId {
        let test = match self.syms.class(c).kind {
            ClassKind::Trait | ClassKind::Enum => TypeTest::Trait(c),
            _ => TypeTest::Class(c),
        };
        self.prog.add_test(test)
    }

    /// Enum values and enum class cases carry their ordinal; the children of a sealed trait are
    /// told apart by their classes, in declaration order.
    fn ordinal_function(&mut self, children: &[ClassId], is_enum: bool, span: Span) -> TExprId {
        let a = self.fresh_local("a", ANY, span);
        let a_ref = self.prog.add(TExpr::Local(a));
        let body = if is_enum {
            let template = self.prog.add_str("$0.$ordinal");
            let args = self.prog.list(&[a_ref]);
            self.prog.add(TExpr::Js(template, args))
        } else {
            let mut ordinal = self.prog.add(TExpr::Int(children.len() as i32 - 1));
            for (i, &child) in children.iter().enumerate().rev().skip(1) {
                let test = self.class_test(child);
                let is_child = self.prog.add(TExpr::TypeTest(a_ref, test));
                let index = self.prog.add(TExpr::Int(i as i32));
                ordinal = self.prog.add(TExpr::If(is_child, index, Some(ordinal)));
            }
            ordinal
        };
        let params = self.prog.syms(&[a]);
        self.prog.add(TExpr::Lambda(params, body))
    }

    /// The type of a case as seen from `parent`, the instance of the sum that is being derived.
    fn case_type(&mut self, child: ClassId, parent: TypeId) -> TypeId {
        self.complete_class(child);
        // An enum's singleton case is its value's type, `solve`'s `child.termRef`.
        if let Some(value) = self.syms.class(child).singleton {
            return self.types.mk(Type::Term(value));
        }
        let arity = self.syms.class(child).tparams.len();
        let vars: Vec<TypeId> = (0..arity).map(|_| self.fresh_var()).collect();
        let mut ty = self.types.class(child, &vars);
        // A child nested in a class is seen through the prefix `childPrefix` gives it, as
        // `sumMirror` sees it (`asSeenFrom(childPre, child.owner)`, Synthesizer.scala 567).
        if self.is_inner_class(child) {
            let parent_class = self.types.named_class(parent);
            let pre = match self.types.get(parent) {
                Type::Nested(p, _) => Some(p),
                _ => parent_class.and_then(|c| match self.syms.class(c).owner {
                    Owner::Class(o) if self.is_inner_class(c) => Some(self.types.mk(Type::This(o))),
                    _ => None,
                }),
            };
            if let Some(child_pre) = pre.zip(parent_class).and_then(|(pre, c)| self.child_prefix(pre, c, child)) {
                ty = self.nested_type(child_pre, ty);
            }
        }
        if arity > 0 {
            self.is_sub(ty, parent);
        }
        self.solve_in(ty)
    }
}
