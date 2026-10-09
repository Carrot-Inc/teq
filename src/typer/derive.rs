//! Scala's derivation protocol. `derives TC` reaches the typer as a given whose right-hand side
//! is `Expr::Derived`, and the givens of `scala.deriving.Mirror` and its refinements are
//! synthesized here, as scalac synthesizes them.

use super::{resolve, Worker};
use crate::ast::{self, mods, TyExpr};
use crate::intern::{FxMap, Name};
use crate::names;
use crate::source::{FileId, Span};
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
    /// The lazily initialised top-level val that holds the Scala mirror of a class: a cell,
    /// made under the loader's lock and shared.
    scala_mirrors: crate::arena::Layered<ClassId, SymId>,
    /// Scala mirror vals of a file being re-typed (`incremental`), whose initialisers are built
    /// anew under the same symbol, since other files refer to it.
    pub scala_reserved: FxMap<ClassId, SymId>,
}

enum Shape {
    Product,
    Singleton,
    Sum(Vec<ClassId>),
}

impl Derivation {
    pub(super) fn fork(&mut self) {
        self.scala_mirrors.fork();
    }

    pub(super) fn to_shared(&mut self, on: bool) {
        self.scala_mirrors.to_shared = on;
    }

    pub(super) fn merge_own(&mut self) {
        self.scala_mirrors.merge_own();
    }

    pub(super) fn apply_pending(&mut self) {
        self.scala_mirrors.apply_pending();
    }

    pub(super) fn absorb(&mut self, other: &mut Derivation) {
        self.scala_mirrors.absorb(other.scala_mirrors.take_local());
        if self.scala.is_none() {
            self.scala = other.scala;
        }
        if self.names.is_none() {
            self.names = other.names;
        }
    }

    /// Another worker's derivation state over the same mirrors.
    pub(super) fn attach(&self) -> Derivation {
        Derivation { scala: self.scala, names: self.names, scala_mirrors: self.scala_mirrors.attach(), scala_reserved: FxMap::default() }
    }

    /// The mirrors under the merge's ids (`merge.rs`).
    pub(super) fn remap(&mut self, sym: impl Fn(SymId) -> SymId, class: impl Fn(ClassId) -> ClassId) {
        if let Some(m) = &mut self.scala {
            for c in [&mut m.mirror, &mut m.product, &mut m.sum, &mut m.singleton, &mut m.product_impl, &mut m.sum_impl, &mut m.singleton_impl] {
                *c = class(*c);
            }
        }
        for table in [&mut self.scala_mirrors.local, &mut self.scala_reserved] {
            let old = std::mem::take(table);
            table.extend(old.into_iter().map(|(c, s)| (class(c), sym(s))));
        }
    }
}

impl<'a> Worker<'a> {
    /// `scala.deriving.Mirror` and its cases among the std files entered so far, or
    /// scala-library's under `--std=scala-library`, where the jar's package and class are entered
    /// on demand; the values behind the mirrors (`std/mirrors.scala`) are the std's in both modes.
    pub fn find_mirror_classes(&mut self) {
        let library = self.scala_library_std();
        let pkg = |t: &mut Self, path: [&str; 2]| {
            let mut names = path.map(|n| t.interner.intern(n)).into_iter();
            names.try_fold(ROOT_PKG, |p, n| match t.syms.pkg(p).entries.get(&n).and_then(|e| e.pkg) {
                Some(sub) => Some(sub),
                None if library => match t.pkg_term(p, n) {
                    Some(resolve::TermRef::Package(p)) => Some(p),
                    _ => None,
                },
                None => None,
            })
        };
        let (Some(deriving), Some(runtime)) = (pkg(self, ["scala", "deriving"]), pkg(self, ["scala", "runtime"])) else { return };
        let class = |t: &mut Self, p: PkgId, name: &str| {
            let name = t.interner.intern(name);
            if let Some(c) = t.syms.pkg(p).entries.get(&name).and_then(|e| e.class) {
                return Some(c);
            }
            if !library {
                return None;
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
        self.derive.scala = Some(ScalaMirrors { mirror, product, sum, singleton, product_impl, sum_impl, singleton_impl });
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

    /// Whether every value of `t` is a `Product`: a case class, a case object, an enum or one of
    /// its cases, a tuple, or a class that extends the trait.
    pub fn is_product(&mut self, t: TypeId) -> bool {
        self.every_value_by_rule(t, |w, c| {
            let b = &w.b;
            if [b.product, b.tuple_trait, b.non_empty_tuple].contains(&Some(c)) || w.is_tuple_class(c) {
                return true;
            }
            w.complete_class(c);
            let info = w.syms.class(c);
            info.is_product_by_rule() || info.base_types.iter().any(|&(base, _)| Some(base) == w.b.product || Some(base) == w.b.tuple_trait)
        })
    }

    /// Whether every value of `t` is a `java.io.Serializable` by rule: a product by rule
    /// (`ClassInfo::is_product_by_rule`) or a class that extends one, as `Desugar.classDef` makes
    /// a case class extend the trait; not `Product`, `Tuple` or a class that extends them alone.
    pub fn is_serializable_by_rule(&mut self, t: TypeId) -> bool {
        self.every_value_by_rule(t, |w, c| {
            w.complete_class(c);
            let info = w.syms.class(c);
            info.is_product_by_rule() || info.base_types.iter().any(|&(base, _)| w.syms.class(base).is_product_by_rule())
        })
    }

    /// Whether every value of `t` is of a class `class_by_rule` takes: both sides of a union, one of
    /// an intersection, a type parameter's bound, what a path or an alias stands for.
    fn every_value_by_rule(&mut self, t: TypeId, class_by_rule: impl Fn(&mut Self, ClassId) -> bool + Copy) -> bool {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Class(c, _) => class_by_rule(self, c),
            Type::Union(x, y) => self.every_value_by_rule(x, class_by_rule) && self.every_value_by_rule(y, class_by_rule),
            Type::Inter(x, y) => self.every_value_by_rule(x, class_by_rule) || self.every_value_by_rule(y, class_by_rule),
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                upper != ANY && self.every_value_by_rule(upper, class_by_rule)
            }
            Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..) | Type::Match(..) | Type::Alias(..) => {
                match self.dependent_underlying(t) {
                    Some(u) => self.every_value_by_rule(u, class_by_rule),
                    None => false,
                }
            }
            _ => false,
        }
    }

    /// Whether every value of the type is an enum value: an enum class or case, which is a
    /// `scala.reflect.Enum` by rule, as it is a `Product`.
    pub fn is_enum_value(&mut self, t: TypeId) -> bool {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Class(c, _) => {
                if Some(c) == self.b.reflect_enum {
                    return true;
                }
                self.complete_class(c);
                let info = self.syms.class(c);
                matches!(info.kind, ClassKind::Enum | ClassKind::EnumCase)
                    || info.base_types.iter().any(|&(base, _)| Some(base) == self.b.reflect_enum)
            }
            Type::Union(x, y) => self.is_enum_value(x) && self.is_enum_value(y),
            Type::Inter(x, y) => self.is_enum_value(x) || self.is_enum_value(y),
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                upper != ANY && self.is_enum_value(upper)
            }
            Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..) | Type::Match(..) | Type::Alias(..) => {
                match self.dependent_underlying(t) {
                    Some(u) => self.is_enum_value(u),
                    None => false,
                }
            }
            _ => false,
        }
    }

    /// A class of package `scala.reflect` as the standard library defines it.
    pub(super) fn entered_scala_reflect_class(&mut self, name: &str) -> Option<ClassId> {
        let reflect = self.interner.intern("reflect");
        let name = self.interner.intern(name);
        let pkg = self.syms.pkg(self.b.scala_pkg).entries.get(&reflect).and_then(|e| e.pkg)?;
        self.syms.pkg(pkg).entries.get(&name).and_then(|e| e.class)
    }

    /// A member of `Product` on a value that is one without extending the trait; `productPrefix`
    /// of such a value is the builtin's (`prims.rs`).
    pub fn product_member(&mut self, recv_ty: TypeId, name: Name) -> Option<(SymId, TypeId)> {
        let product = self.b.product?;
        if name == names::PRODUCT_PREFIX || !self.syms.class(product).members.contains_key(&name) || !self.is_product(recv_ty) {
            return None;
        }
        let pt = self.types.class(product, &[]);
        self.find_member(pt, name)
    }

    /// The std function that stands in for a concrete member of `Product` (`productIterator`,
    /// `canEqual`, the latter declared by `Equals`) when a call resolves to the trait's own definition: the receiver is a product
    /// by rule or is typed as `Product`, and neither has a body of the trait to call.
    pub fn product_helper(&self, member: SymId) -> Option<SymId> {
        let s = self.syms.sym(member);
        if !matches!(s.owner, Owner::Class(c) if Some(c) == self.b.product || Some(c) == self.equals_class()) {
            return None;
        }
        self.b.product_helpers.iter().find(|&&(n, _)| n == s.name).map(|&(_, h)| h)
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
            Refinement::Alias(n, t) if n == names.mono => Some(t) != mirrored,
            Refinement::Alias(n, _) => n == names.elem_types || n == names.elem_labels,
            Refinement::Bounds(n, lo, hi) if n == names.elem_types || n == names.elem_labels => {
                lo != NOTHING || !matches!(self.types.get(hi), Type::Class(c, _) if Some(c) == tuple)
            }
            Refinement::Bounds(..) => false,
            // A term member no mirror has.
            Refinement::Term(..) | Refinement::Val(..) => true,
        })
    }

    pub fn scala_mirror_given(&mut self, class: ClassId, target: TypeId) -> Option<(TExprId, TypeId)> {
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
        let target = self.solve_bounded_in(target);
        let rs = self.refinements_through_aliases(target);
        let mirrored = rs.iter().find_map(|&r| match self.types.refinement(r) {
            Refinement::Alias(n, t) if n == names.mirrored => Some(t),
            _ => None,
        })?;
        let mirrored = self.zonk(mirrored);
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
            self.refuse_mirror(c, wants, mirrored);
            return None;
        }
        let shape = shape?;
        let ty = self.scala_mirror_type(c, &shape, mirrored, subject, &params, names, sm);
        // The one mirror of a class without parameters is declared with its refined type in link
        // mode, so that a path to it keeps the type members: scala-library's `summon` is inline
        // and gives `x.type`, where the std's gives the expected type. So whatever type names the
        // class (`Red.type`, `A & B`), which keeps its accessor's descriptor that of the
        // mirror's parent in every build (`complete_program_mirrors`).
        let plain = self.scala_library_std() && params.is_empty() && self.syms.class(c).tparams.is_empty();
        let value = self.scala_mirror_value(c, &shape, sm, plain.then_some(ty))?;
        Some((value, ty))
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
            Type::Lambda(ps, body) => match self.types.get(body) {
                Type::Class(c, args) if self.types.items(args) == self.types.items(ps) => Some((c, body, self.types.items(ps).to_vec())),
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
        match self.types.get(mirrored) {
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
                self.rollback(mark);
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
            // scala-library's `Singleton` is the object's own type; the std's mirror object
            // is a `Product` refined to the object (`std/library/mirrors.scala`).
            Shape::Singleton if self.scala_library_std() => sm.product,
            Shape::Singleton => sm.singleton,
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
        }
        let labels: Vec<TypeId> = elem_names.iter().map(|&n| self.types.lit(LitVal::Str(n))).collect();
        let labels_ty = self.tuple_of(&labels);
        let label = self.types.lit(LitVal::Str(self.syms.class(c).name));
        let mut t = self.types.class(parent, &[]);
        for (n, rhs) in [(names.mono, mono), (names.mirrored, mirrored), (names.label, label), (names.elem_types, elems_ty), (names.elem_labels, labels_ty)] {
            let r = self.types.refine(Refinement::Alias(n, rhs));
            t = self.types.mk(Type::Refined(t, r));
        }
        t
    }

    /// The mirror object of a class, one per class whatever the type arguments, held by a
    /// lazily initialised top-level val.
    fn scala_mirror_value(&mut self, c: ClassId, shape: &Shape, sm: ScalaMirrors, declared: Option<TypeId>) -> Option<TExprId> {
        // A mirror's val found is every worker's (applied at a release after the val); a miss is
        // the lock's to answer, with the lookup made again under it.
        if self.forked {
            if let Some(&sym) = self.derive.scala_mirrors.get(&c) {
                return Some(self.prog.add(TExpr::Static(sym)));
            }
        }
        self.with_loader(|w| w.scala_mirror_value_unlocked(c, shape, sm, declared))
    }

    /// Under the loader's lock: the declared type a worker passes in is canonicalised into the
    /// base first, since the signature it becomes is the shared symbol's (the canonicalisation
    /// rule).
    fn scala_mirror_value_unlocked(&mut self, c: ClassId, shape: &Shape, sm: ScalaMirrors, declared: Option<TypeId>) -> Option<TExprId> {
        let view = self.types.view_here();
        let declared = declared.map(|t| self.types.translate(view, t));
        if let Some(&sym) = self.derive.scala_mirrors.get(&c) {
            return Some(self.prog.add(TExpr::Static(sym)));
        }
        let (file, owner, span) = {
            let info = self.syms.class(c);
            (info.file, info.owner, info.span)
        };
        let impl_class = match shape {
            Shape::Product => sm.product_impl,
            Shape::Singleton => sm.singleton_impl,
            Shape::Sum(_) => sm.sum_impl,
        };
        // The mirror of a class nested in a class holds the enclosing instance, as scalac's
        // (the companion) does: it is made where it is summoned, not held by a top-level val.
        if self.outer_class(c).is_some() {
            return Some(self.build_scala_mirror(c, shape, impl_class, span));
        }
        let depth = std::mem::replace(&mut self.implicit_depth, 0);
        let owners = std::mem::take(&mut self.sites.owners);
        let env = self.env_for(file, owner);
        let sym = match self.derive.scala_reserved.remove(&c) {
            Some(sym) => sym,
            None => self.mirror_val_sym(c, "derivingMirror", impl_class, span),
        };
        // A product class's mirror stands with the class's file, which its conversion makes
        // later (`rank_files`).
        if self.loaded.as_ref().map_or(false, |l| l.is_product_class(c)) {
            self.loaded_mut().product_mirror_vals.push((sym, c));
        }
        if let Some(ty) = declared {
            let sig = self.value_sig(ty);
            self.syms.sym_mut(sym).sig = Some(sig);
        }
        self.derive.scala_mirrors.insert(c, sym);
        let init = self.with_env(env, |t| t.build_scala_mirror(c, shape, impl_class, span));
        self.prog.top_vals.push((sym, init));
        self.sites.owners = owners;
        self.implicit_depth = depth;
        Some(self.prog.add(TExpr::Static(sym)))
    }

    fn build_scala_mirror(&mut self, c: ClassId, shape: &Shape, impl_class: ClassId, span: Span) -> TExprId {
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
    fn refuse_mirror(&mut self, c: ClassId, wants: Wants, mirrored: TypeId) {
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
            msg.push_str(&format!("
	* {} is not a generic sum because {}", desc, self.why_not_generic_sum(c)));
        }
        self.given_ambiguity = Some(msg);
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
        Some(MethodSig { tparams: Vec::new(), clauses: Vec::new(), ret })
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

    /// The class whose Scala mirror the synthesized top-level val `s` holds
    /// (`scala_mirror_value`), which no pickle of scalac's names: a writer states its type.
    pub fn scala_mirror_class(&self, s: SymId) -> Option<ClassId> {
        let info = self.syms.sym(s);
        if info.kind != SymKind::Val || info.def.is_some() || !matches!(info.owner, Owner::Package(_)) || !self.interner.get(info.name).starts_with("derivingMirror$") {
            return None;
        }
        let file = info.file;
        (0..self.syms.classes.len() as u32).map(ClassId).find(|&c| self.syms.class(c).file == file && self.derive.scala_mirrors.get(&c) == Some(&s))
    }

    pub(super) fn scala_mirrors_of_file(&self, file: FileId) -> Vec<(ClassId, SymId)> {
        self.derive.scala_mirrors.local.iter().filter(|(c, _)| self.syms.class(**c).file == file).map(|(&c, &s)| (c, s)).collect()
    }

    pub(super) fn reserve_scala_mirror(&mut self, c: ClassId, sym: SymId) {
        self.derive.scala_mirrors.remove(&c);
        self.derive.scala_reserved.insert(c, sym);
    }

    pub(super) fn rebuild_scala_mirror(&mut self, c: ClassId) {
        let Some(sm) = self.derive.scala else { return };
        if let Some(shape) = self.mirror_shape(c) {
            self.scala_mirror_value(c, &shape, sm, None);
        }
    }

    /// On the JVM every class of the program's sources with a mirror has its val, summoned or
    /// not: another project built over the class files (sbt's residents, `--own`) may summon it,
    /// and the val stands in the class's file, which that project does not write, as scalac's
    /// mirror, the companion, is in every build of the class. Declared as a summon of the class
    /// declares it, so that both projects name the accessor alike. Products keep their own
    /// rule: a downstream over them holds the mirrors it summons.
    pub(super) fn complete_program_mirrors(&mut self, only: Option<&[FileId]>) {
        let Some(sm) = self.derive.scala else { return };
        if !self.jvm || self.open_world {
            return;
        }
        let mut classes: Vec<ClassId> = (0..self.syms.classes.len() as u32)
            .map(ClassId)
            .filter(|&c| self.is_program_mirror_class(c) && only.map_or(true, |files| files.contains(&self.syms.class(c).file)))
            .collect();
        classes.sort_by_key(|&c| (self.syms.class(c).file, self.syms.class(c).span.start));
        let names = self.mirror_names();
        for c in classes {
            if self.derive.scala_mirrors.contains_key(&c) || self.outer_class(c).is_some() {
                continue;
            }
            let Some(shape) = self.mirror_shape(c) else { continue };
            let accessible = match &shape {
                Shape::Sum(children) => children.iter().all(|&child| !self.in_local_class(child) && self.mirror_ctor_accessible(child)),
                _ => self.mirror_ctor_accessible(c),
            };
            if !accessible {
                continue;
            }
            let declared = (self.scala_library_std() && self.syms.class(c).tparams.is_empty()).then(|| {
                let mirrored = self.types.class(c, &[]);
                self.scala_mirror_type(c, &shape, mirrored, mirrored, &[], names, sm)
            });
            self.scala_mirror_value(c, &shape, sm, declared);
        }
    }

    /// The vals `complete_program_mirrors` writes, which link mode's reach takes as roots: those
    /// of the program's own classes, summoned or not, outside the product mode. A val a summon
    /// made for another class is reached where it is used, as before.
    pub(crate) fn scala_mirror_vals(&self) -> Vec<SymId> {
        if !self.jvm || self.open_world {
            return Vec::new();
        }
        let mut vals: Vec<SymId> =
            self.derive.scala_mirrors.local.iter().filter(|(&c, _)| self.is_program_mirror_class(c)).map(|(_, &s)| s).collect();
        vals.sort();
        vals
    }

    /// A class of the program's own sources whose mirror the JVM build writes summoned or not:
    /// not the std's, a jar's or a product's, and not inside a local class.
    fn is_program_mirror_class(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        info.def.is_some()
            && !self.in_local_class(c)
            && !self.source(info.file).is_std
            && !self.in_jar(info.file)
            && !self.is_body_file(info.file)
            && !self.is_product_class(c)
    }

    fn in_local_class(&self, c: ClassId) -> bool {
        let mut owner = self.syms.class(c).owner;
        while let Owner::Class(o) = owner {
            owner = self.syms.class(o).owner;
        }
        owner == Owner::Local
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

    /// The top-level val holding a mirror of `c`, typed as `class[Any]`, named after the class's
    /// path.
    fn mirror_val_sym(&mut self, c: ClassId, prefix: &str, class: ClassId, span: Span) -> SymId {
        let mut name = String::from(prefix);
        let mut owner = Owner::Class(c);
        let mut path = Vec::new();
        while let Owner::Class(o) = owner {
            path.push(o);
            owner = self.syms.class(o).owner;
        }
        // An object is named apart from a class of the same name, as scalac's module class.
        for &o in path.iter().rev() {
            name.push('$');
            name.push_str(self.interner.get(self.syms.class(o).name));
            if self.syms.class(o).kind == ClassKind::Object {
                name.push('$');
            }
        }
        let name = self.interner.intern(&name);
        let file = self.syms.class(c).file;
        let sym = self.syms.new_sym(name, SymKind::Val, 0, owner, file, None, span);
        let ty = self.types.class(class, &[ANY]);
        let sig = self.value_sig(ty);
        let mut info = self.syms.sym_mut(sym);
        info.sig = Some(sig);
        info.state().set(Completion::Done);
        sym
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
        let arity = self.syms.class(child).tparams.len();
        let vars: Vec<TypeId> = (0..arity).map(|_| self.fresh_var()).collect();
        let ty = self.types.class(child, &vars);
        if arity > 0 {
            self.is_sub(ty, parent);
        }
        self.solve_in(ty)
    }
}
