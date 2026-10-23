//! Implicit conversions: `implicit def f(a: A): B` and givens of type `Conversion[A, B]`.
//! A member selection that fails on the receiver's own type is retried through a conversion
//! whose result has the member, and an expression that does not conform to its expected type
//! through a conversion to that type. Both are searched only where the program would be an
//! error otherwise, and the conversions of a scope are indexed by the member names of their
//! result class and by that class, on the first lookup in the scope.
use super::apply::{ArgList, ArgSrc};
use super::implicits::GivenIndex;
use super::profile::{About, Kind, Outcome};
use super::implicits::GivenScope;
use super::{Frame, ImportTarget, ResolvedImport, Worker, ValueImport};
use crate::ast::mods;
use crate::intern::{FxMap, Name};
use crate::names;
use crate::source::Span;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeKey {
    Pkg(PkgId),
    Class(ClassId),
}

fn conversion_outcome(picked: &Option<Result<(SymId, TExprId, TypeId), ()>>) -> Outcome {
    match picked {
        Some(Ok(_)) => Outcome::Found,
        Some(Err(())) => Outcome::Ambiguous,
        None => Outcome::NotFound,
    }
}

#[derive(Default)]
pub struct ConversionIndex {
    /// Conversions whose result class, or one of its bases, has a member of that name.
    pub by_member: FxMap<Name, Vec<SymId>>,
    /// Conversions whose result class derives from that class.
    pub by_result: FxMap<ClassId, Vec<SymId>>,
    /// Conversions whose result type has no class at its head: a type parameter, an abstract
    /// type. They are tried when the index has nothing.
    pub unindexed: Vec<SymId>,
}

/// What is asked of a conversion: a member of its result, or a result of a class; or, for a
/// completion, any.
#[derive(Clone, Copy)]
enum Pick<'a> {
    Member(Name),
    Result(Option<ClassId>),
    /// A result of one of these classes: the members of a union expected, which a conversion's
    /// result conforms to where it conforms to one (dotty's `inferView` takes any result that
    /// conforms to the expected type).
    Results(&'a [ClassId]),
    Any,
}

/// The operators a builtin type has without a member symbol, under which a conversion to it
/// is indexed; `prim_op_applies` says which of them the type has.
const PRIM_OPS: &[Name] = &[
    names::UNARY_BANG,
    names::UNARY_MINUS,
    names::UNARY_PLUS,
    names::UNARY_TILDE,
    names::AMPAMP,
    names::BARBAR,
    names::AMP,
    names::BAR,
    names::CARET,
    names::PLUS,
    names::MINUS,
    names::STAR,
    names::SLASH,
    names::PERCENT,
    names::LT,
    names::LE,
    names::GT,
    names::GE,
    names::SHL,
    names::SHR,
    names::USHR,
];

impl ConversionIndex {
    fn select(&self, pick: Pick<'_>, out: &mut Vec<SymId>) {
        let list = match pick {
            Pick::Member(n) => self.by_member.get(&n),
            Pick::Result(Some(c)) => self.by_result.get(&c),
            Pick::Result(None) => None,
            Pick::Results(cs) => {
                for c in cs {
                    for &g in self.by_result.get(c).into_iter().flatten() {
                        if !out.contains(&g) {
                            out.push(g);
                        }
                    }
                }
                None
            }
            Pick::Any => {
                let mut all: Vec<SymId> = self.by_member.values().chain(self.by_result.values()).flatten().copied().collect();
                all.sort_by_key(|s| s.0);
                all.dedup();
                out.extend(all.into_iter().filter(|s| !self.unindexed.contains(s)));
                None
            }
        };
        out.extend(list.into_iter().flatten().copied());
        out.extend(self.unindexed.iter().copied());
    }
}

/// A conversion's type parameters, the type it takes and the type it gives.
pub(super) struct Shape {
    tparams: Vec<TParamId>,
    from: TypeId,
    pub(super) to: TypeId,
}

/// A conversion and what it is called on.
type ConvRef = (SymId, Via);

/// The binding precedences `shadow_conversions` compares (dotty's `BindingPrec`).
const PREC_WILD_IMPORT: u8 = 1;
const PREC_NAMED_IMPORT: u8 = 2;
const PREC_DEFINITION: u8 = 3;

/// What `Worker::conversion_site` answers.
struct ConversionSite {
    site: Option<(super::exports::TraitMemberSite, ClassId)>,
    subst: Subst,
    from: TypeId,
    to: TypeId,
}

/// Where a conversion was found, which decides what it is called on: in scope (its object, or
/// the enclosing instance of a class that has it), as a member of an object of an implicit
/// scope, or as a member of a value an import opens.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Via {
    Scope,
    Module(ClassId),
    Value(ValueImport),
    /// A path of the implicit scope whose members are candidates (dotty's `addPath`).
    Path(TypeId),
}

struct Candidate {
    sym: SymId,
    via: Via,
    expr: TExprId,
    ty: TypeId,
}

impl<'a> Worker<'a> {
    /// Whether an open variable's bounds meet in one type, which then is its value: a lower bound
    /// that is also one of its upper bounds.
    #[inline(never)]
    fn fixed_by_bounds(&mut self, v: TVarId) -> bool {
        let info = &self.tvars[v];
        if info.inst.is_some() || info.lower.is_empty() || info.upper.is_empty() {
            return false;
        }
        for i in 0..self.tvars[v].lower.len() {
            let l = self.tvars[v].lower[i];
            let l = self.zonk(l);
            let l = self.canon(l);
            for j in 0..self.tvars[v].upper.len() {
                let u = self.tvars[v].upper[j];
                let u = self.zonk(u);
                if self.same_type(u, l) {
                    return true;
                }
            }
        }
        false
    }

    /// The type parameters the conversion's declared result `declared` holds nonvariantly, read off
    /// `to`, that result over the conversion's variables (`subst`'s, made in `made`); kept by
    /// `declared` where every one of them was open, so that the variances are the parameters'.
    #[inline(never)]
    fn nonvariant_params(&mut self, declared: TypeId, to: TypeId, subst: &Subst, made: &std::ops::Range<usize>) -> std::sync::Arc<[TParamId]> {
        let key = declared;
        if let Some(ps) = self.conversion_nonvariant.get(&key) {
            return ps.clone();
        }
        let open = made.clone().all(|v| self.tvars[self.tvars.id(v)].inst.is_none());
        let to_now = self.zonk(to);
        let mut ps: Vec<TParamId> = Vec::new();
        for (w, s) in self.var_variances(to_now) {
            if s != 0 {
                continue;
            }
            if let Some(&(p, _)) = subst.iter().find(|&&(_, t)| matches!(self.types.get(t), Type::Var(x) if x == w)) {
                ps.push(p);
            }
        }
        let ps: std::sync::Arc<[TParamId]> = ps.into();
        if open && !self.types.has_vars(declared) {
            self.conversion_nonvariant.insert(key, ps.clone());
        }
        ps
    }

    /// `implicit def f[..](a: A)(implicit ..): B`: a Scala 2 implicit def whose first clause has
    /// one plain parameter. Such a def is a conversion and no given.
    pub fn is_conversion_def(&mut self, g: SymId) -> bool {
        let info = self.syms.sym(g);
        if info.mods & mods::IMPLICIT == 0 || info.kind != SymKind::Def || info.is_extension {
            return false;
        }
        let sig = self.sig_of(g);
        matches!(sig.clauses.first(), Some(c) if !c.is_using && c.params.len() == 1 && !c.params[0].repeated)
            && sig.clauses[1..].iter().all(|c| c.is_using)
    }

    /// A given, or a Scala 2 implicit without plain parameters, of a type that derives from
    /// `Conversion`.
    pub fn is_conversion_given(&mut self, g: SymId) -> bool {
        self.conversion_class_of(g).is_some()
    }

    /// The class a given is a conversion through: `Conversion`, or `<:<`, which dotty's
    /// `viewCandidateKind` counts as a view too (`isConforms`: evidence `ev: A <:< B` or
    /// `A =:= B` in scope converts an `A`).
    fn conversion_class_of(&mut self, g: SymId) -> Option<ClassId> {
        let sig = self.sig_of(g);
        if sig.clauses.iter().any(|c| !c.is_using) {
            return None;
        }
        let ret = sig.ret;
        let c = self.class_of(ret)?;
        self.complete_class(c);
        [self.b.conversion, self.b.sub_evidence].into_iter().flatten().find(|&k| self.syms.class(c).base_types.iter().any(|&(b, _)| b == k))
    }

    pub(super) fn shape_of(&mut self, g: SymId) -> Option<Shape> {
        let is_def = self.syms.sym(g).kind == SymKind::Def;
        let sig = self.sig_of(g);
        if is_def && sig.clauses.first().map_or(false, |c| !c.is_using) {
            let [param] = sig.clauses[0].params.as_slice() else { return None };
            return Some(Shape { tparams: sig.tparams.clone(), from: param.ty, to: sig.ret });
        }
        let ret = sig.ret;
        let conversion = self.conversion_class_of(g)?;
        let bt = self.base_type(ret, conversion)?;
        let Type::Class(_, args) = self.types.get(bt) else { return None };
        let args = self.types.items(args);
        if args.len() != 2 {
            return None;
        }
        Some(Shape { tparams: self.syms.sig(g).tparams.clone(), from: args[0], to: args[1] })
    }

    /// Whether the conversion method `g` can be the implicit function value `target` by the
    /// classes alone: the target's parameter derives from the method's parameter class and the
    /// method's result class from the target's result class, where both sides name classes. The
    /// given search asks it once per candidate and target, before instantiating anything.
    pub(super) fn conversion_method_may_fit(&mut self, g: SymId, target: TypeId) -> bool {
        let t = self.deref(target);
        let Type::Class(_, args) = self.types.get(t) else { return false };
        let (want_from, want_to) = (self.types.items(args)[0], self.types.items(args)[1]);
        let Some(shape) = self.shape_of(g) else { return false };
        // A refinement is its parent's class (cats' `Ops[F, A] { type TypeClassType = .. }`).
        let head = |me: &mut Self, t: TypeId| -> TypeId {
            let mut t = me.deref(t);
            while let Type::Refined(parent, _) = me.types.get(t) {
                t = me.deref(parent);
            }
            t
        };
        let derives = |me: &mut Self, sub: TypeId, sup: TypeId| -> bool {
            let (sub, sup) = (head(me, sub), head(me, sup));
            match (me.types.get(sub), me.types.get(sup)) {
                (Type::Class(a, _), Type::Class(b, _)) => {
                    if a == b {
                        return true;
                    }
                    // What base types do not state passes: the top classes as the wanted side
                    // (the builtin and value classes' base types leave `AnyVal` out), `Null` as
                    // the offered one, below every reference type.
                    let bi = me.syms.class(b);
                    let top = b == me.b.any_ref || b == me.b.any_val || (bi.owner == Owner::Package(me.b.scala_pkg) && me.interner.get(bi.name) == "Matchable");
                    if top || a == me.b.null {
                        return true;
                    }
                    // A Java class nothing has read yet has no base types to test: it passes.
                    if me.is_java_placeholder(a) {
                        return true;
                    }
                    me.complete_class(a);
                    me.syms.class(a).base_types.iter().any(|&(k, _)| k == b)
                        || me.syms.class(a).kind == ClassKind::Opaque
                        || me.syms.class(b).kind == ClassKind::Opaque
                }
                _ => true,
            }
        };
        derives(self, want_from, shape.from) && derives(self, shape.to, want_to)
    }

    /// The conversions of a scope, indexed on the first lookup in it or ahead of any by the
    /// signature phase (`build_scope_tables`); `None` for a scope without conversions.
    pub(super) fn conversion_index(&mut self, key: ScopeKey, index: &GivenIndex) -> Option<Arc<ConversionIndex>> {
        if index.conversions.is_empty() {
            return None;
        }
        if let Some(i) = self.conversion_indexes.get(&key) {
            return Some(i.clone());
        }
        let prof = self.phase(super::profile::Phase::ConversionIndex);
        let built = self.build_conversion_index(key, index);
        self.phase_end(prof);
        Some(built)
    }

    fn build_conversion_index(&mut self, key: ScopeKey, index: &GivenIndex) -> Arc<ConversionIndex> {
        let mut out = ConversionIndex::default();
        for &g in &index.conversions {
            let Some(shape) = self.shape_of(g) else { continue };
            let Some(c) = self.class_of(shape.to) else {
                out.unindexed.push(g);
                continue;
            };
            self.complete_class(c);
            if self.syms.class(c).kind == ClassKind::Builtin {
                for &op in PRIM_OPS {
                    out.by_member.entry(op).or_default().push(g);
                }
                for name in super::prims::NUMERIC_CONVERSIONS {
                    let n = self.interner.intern(name);
                    out.by_member.entry(n).or_default().push(g);
                }
            }
            let mut bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
            // A value class's base types leave `AnyVal` out, which its values conform to: a view
            // to `AnyVal` is one to any of them (`Predef.Ensuring` from `Any`).
            if (self.syms.class(c).value_class || self.is_primitive_class(c) || c == self.b.unit) && !bases.contains(&self.b.any_val) {
                bases.push(self.b.any_val);
            }
            for b in bases {
                out.by_result.entry(b).or_default().push(g);
                for i in 0..self.syms.class(b).member_order.len() {
                    let m = self.syms.class(b).member_order[i];
                    let list = out.by_member.entry(self.syms.sym(m).name).or_default();
                    if list.last() != Some(&g) {
                        list.push(g);
                    }
                }
            }
        }
        let rc = Arc::new(out);
        // Kept only over a kept given index: one built over a provisional export table is not.
        let kept = match key {
            ScopeKey::Pkg(p) => self.given_indexes.get(&p).map_or(false, |k| std::ptr::eq(Arc::as_ptr(k), index)),
            ScopeKey::Class(c) => self.class_given_indexes.get(&c).and_then(|k| k.as_ref()).map_or(false, |k| std::ptr::eq(Arc::as_ptr(k), index)),
        };
        if kept {
            self.conversion_indexes.insert(key, rc.clone());
        }
        rc
    }

    fn pkg_conversions(&mut self, p: PkgId, pick: Pick<'_>, out: &mut Vec<SymId>) {
        let index = self.given_index_of(p);
        if let Some(ci) = self.conversion_index(ScopeKey::Pkg(p), &index) {
            ci.select(pick, out);
        }
    }

    fn class_conversions(&mut self, c: ClassId, pick: Pick<'_>, out: &mut Vec<SymId>) {
        let Some(index) = self.class_given_index_of(c) else { return };
        if let Some(ci) = self.conversion_index(ScopeKey::Class(c), &index) {
            ci.select(pick, out);
        }
    }

    fn import_conversions(&mut self, imp: ResolvedImport, pick: Pick<'_>, out: &mut Vec<SymId>) {
        if matches!(imp.target, ImportTarget::UnimportPredef) {
            return;
        }
        let from = out.len();
        match imp.target {
            ImportTarget::ClassGivens(c) | ImportTarget::ClassAll(c) => self.class_conversions(c, pick, out),
            ImportTarget::PkgGivens(p) | ImportTarget::PkgAll(p) => self.pkg_conversions(p, pick, out),
            ImportTarget::ClassMember(c, n) => {
                if let Some(s) = self.module_term(c, n).and_then(|r| r.sym()) {
                    if self.is_conversion_def(s) || self.is_conversion_given(s) {
                        out.push(s);
                    }
                }
            }
            ImportTarget::PkgMember(p, n) => {
                if let Some(s) = self.pkg_term(p, n).and_then(|r| r.sym()) {
                    if self.is_conversion_def(s) || self.is_conversion_given(s) {
                        out.push(s);
                    }
                }
            }
            ImportTarget::ValueAll(_) | ImportTarget::ValueMember(..) | ImportTarget::ValueGivens(_) | ImportTarget::UnimportPredef | ImportTarget::Unresolved => {}
        }
        if !imp.hidden.is_empty() {
            let hidden = &self.import_hidden.as_slice()[imp.hidden.range()];
            let mut i = from;
            while i < out.len() {
                if hidden.contains(&self.syms.sym(out[i]).name) {
                    out.remove(i);
                } else {
                    i += 1;
                }
            }
        }
    }

    /// The conversions of the class of a value an import opens (`import dsl.*`), called on it.
    fn value_import_conversions(&mut self, imp: ResolvedImport, pick: Pick<'_>, out: &mut Vec<ConvRef>) {
        let (v, givens) = match imp.target {
            ImportTarget::ValueAll(v) => (v, false),
            ImportTarget::ValueGivens(v) => (v, true),
            _ => return,
        };
        let ty = self.import_value_ret(v);
        let ty = self.zonk(ty);
        let Some(c) = self.class_of(ty) else { return };
        let mut found = Vec::new();
        match imp.name {
            None => self.class_conversions(c, pick, &mut found),
            Some(_) => return,
        }
        // A wildcard leaves the value's givens out, as it does an object's: its Scala 2 implicit
        // conversions come in; `import v.given` brings both, as scalac's migration rule has it.
        found.retain(|&g| {
            (givens || !self.syms.is_scala3_given(g))
                && (self.is_conversion_def(g) || self.is_conversion_given(g))
                && !self.import_hides(imp, self.syms.sym(g).name)
        });
        out.extend(found.into_iter().map(|g| (g, Via::Value(v))));
    }

    /// dotty's `ContextualImplicits.combineEligibles` within a level: the conversions `added` of one
    /// more binding of precedence `prec` (an import inside the definitions of its scope) beside
    /// those of the bindings before it, by name: where an earlier binding of a higher precedence
    /// has the name (a definition over an import, a named import over a wildcard), the new ones
    /// of the name go; else the earlier ones of the name go, a later import of the same
    /// precedence being the inner one. One binding's own alternatives all stay. An export's
    /// forwarder and what it forwards, of one name, are so one candidate as scalac has them.
    fn shadow_conversions(&self, scope: &mut Vec<(ConvRef, u8)>, values: &mut Vec<(ConvRef, u8)>, mut added: Vec<ConvRef>, prec: u8, to_values: bool) {
        if added.is_empty() {
            return;
        }
        // The level's first binding shadows nothing.
        if scope.is_empty() && values.is_empty() {
            let into = if to_values { values } else { scope };
            into.extend(added.into_iter().map(|r| (r, prec)));
            return;
        }
        let mut names: Vec<Name> = added.iter().map(|&(g, _)| self.syms.sym(g).name).collect();
        names.sort_by_key(|n| n.0);
        names.dedup();
        for n in names {
            let named = |&((g, _), _): &(ConvRef, u8)| self.syms.sym(g).name == n;
            let higher = scope.iter().chain(values.iter()).any(|e| named(e) && e.1 > prec);
            if higher {
                added.retain(|&(g, _)| self.syms.sym(g).name != n);
            } else {
                scope.retain(|e| !named(e));
                values.retain(|e| !named(e));
            }
        }
        let into = if to_values { values } else { scope };
        into.extend(added.into_iter().map(|r| (r, prec)));
    }

    /// The candidate conversions level by level, as the given search orders its levels: the
    /// enclosing blocks and classes from the inside out with their imports, the package
    /// clauses, and last the implicit scopes of `scope_tys`: the argument's type, widened, and
    /// the expected type where there is one, as it is (dotty's `wildProto`, a `ViewProto` of
    /// `argument.tpe.widen` and `pt`).
    fn conversion_levels(&mut self, scope_tys: &[TypeId], pick: Pick<'_>) -> Vec<Vec<ConvRef>> {
        let mut levels: Vec<Vec<ConvRef>> = Vec::new();
        // One buffer for the conversions of a level, which the level's list takes as refs.
        let mut level: Vec<SymId> = Vec::new();
        let scoped = |level: &mut Vec<SymId>| -> Vec<ConvRef> { level.drain(..).map(|g| (g, Via::Scope)).collect() };
        for frame in (0..self.env.frames.len()).rev() {
            match &self.env.frames[frame] {
                // Every given of the scope, a using parameter's included, as dotty's view
                // candidates are the context's implicit references (`Implicits.candidateKind`).
                Frame::Locals { givens, .. } => {
                    let locals: Vec<SymId> = givens.iter().rev().copied().collect();
                    for g in locals {
                        if self.is_conversion_def(g) || self.is_conversion_given(g) {
                            level.push(g);
                        }
                    }
                }
                Frame::Class(c) if self.parent_args_of == Some(*c) => {}
                Frame::Class(c) => {
                    let c = *c;
                    self.class_conversions(c, pick, &mut level);
                }
            }
            let depth = frame as u32 + 1;
            // A level without imports has its definitions alone, which shadow nothing.
            if !self.env.imports.iter().any(|imp| imp.depth == depth) {
                if !level.is_empty() {
                    levels.push(scoped(&mut level));
                }
                continue;
            }
            let mut scope: Vec<(ConvRef, u8)> = level.drain(..).map(|g| ((g, Via::Scope), PREC_DEFINITION)).collect();
            let mut values: Vec<(ConvRef, u8)> = Vec::new();
            let mut from_values = Vec::new();
            for i in 0..self.env.imports.len() {
                let imp = self.env.imports[i];
                if imp.depth == depth {
                    let prec = if imp.name.is_some() { PREC_NAMED_IMPORT } else { PREC_WILD_IMPORT };
                    self.import_conversions(imp, pick, &mut level);
                    let added: Vec<ConvRef> = scoped(&mut level);
                    self.shadow_conversions(&mut scope, &mut values, added, prec, false);
                    self.value_import_conversions(imp, pick, &mut from_values);
                    let added: Vec<ConvRef> = std::mem::take(&mut from_values);
                    self.shadow_conversions(&mut scope, &mut values, added, prec, true);
                }
            }
            if !scope.is_empty() || !values.is_empty() {
                let refs: Vec<ConvRef> = scope.into_iter().chain(values).map(|(r, _)| r).collect();
                levels.push(refs);
            }
        }
        let chain = self.pkg_chain();
        for (i, &p) in chain.iter().enumerate() {
            self.pkg_conversions(p, pick, &mut level);
            // An explicit import of `Predef` takes its root import's conversions away too; the
            // lean std defines them in package `scala`.
            if p == self.b.scala_pkg && level.iter().any(|&g| self.b.predef_names.contains(&self.syms.sym(g).name)) && self.predef_unimported() {
                level.retain(|&g| !self.b.predef_names.contains(&self.syms.sym(g).name));
            }
            // The root import of `Predef` leaves the deprecated `any2stringadd` conversion out
            // (`ImportInfo`'s `any2stringadd as _` on `rootImport`): an explicit import brings it.
            if p == self.b.scala_pkg {
                level.retain(|&g| self.syms.sym(g).name != crate::names::ANY2STRINGADD);
            }
            if i == 0 {
                let mut scope: Vec<(ConvRef, u8)> = level.drain(..).map(|g| ((g, Via::Scope), PREC_DEFINITION)).collect();
                let mut values: Vec<(ConvRef, u8)> = Vec::new();
                let n = self.import_count();
                for k in 0..n {
                    let imp = self.import_at(k);
                    if imp.depth == 0 {
                        let prec = if imp.name.is_some() { PREC_NAMED_IMPORT } else { PREC_WILD_IMPORT };
                        self.import_conversions(imp, pick, &mut level);
                        let added: Vec<ConvRef> = scoped(&mut level);
                        self.shadow_conversions(&mut scope, &mut values, added, prec, false);
                    }
                }
                level.extend(scope.into_iter().map(|((g, _), _)| g));
            }
            if !level.is_empty() {
                levels.push(scoped(&mut level));
            }
        }
        // `scala.Predef` of a classpath is a level of its own outside the package clauses.
        if let Some(predef) = self.loaded.as_ref().and_then(|l| l.predef).filter(|_| !self.predef_unimported()) {
            self.class_conversions(predef, pick, &mut level);
            // The root import leaves `any2stringadd` out here too (`ImportInfo`'s `rootImport`).
            level.retain(|&g| self.syms.sym(g).name != crate::names::ANY2STRINGADD);
            if !level.is_empty() {
                levels.push(scoped(&mut level));
            }
        }
        let mut scope = Vec::new();
        for (i, &t) in scope_tys.iter().enumerate() {
            let t = if i == 0 { self.view_scope_type(t) } else { t };
            // The objects of the implicit scope, and the paths it holds, whose members are
            // read on them (dotty's `OfTypeImplicits.refs` of the `addPath` references).
            for found in self.implicit_scope_objects(t) {
                match found {
                    GivenScope::Module(m) => {
                        self.class_conversions(m, pick, &mut level);
                        scope.extend(level.drain(..).map(|g| (g, Via::Module(m))));
                    }
                    GivenScope::Path(p) if self.scope_accessible(found) => {
                        let under = self.path_underlying(p);
                        if let Some(c) = self.class_of(under) {
                            self.class_conversions(c, pick, &mut level);
                            scope.extend(level.drain(..).map(|g| (g, Via::Path(p))));
                        }
                    }
                    _ => {}
                }
            }
        }
        if !scope.is_empty() {
            levels.push(scope);
        }
        // One method reached through one value is one candidate, however many imports of the
        // value (`import v.*` twice) bring it.
        let values = &self.import_values;
        let types = &self.types;
        let key = |&(g, via): &ConvRef| match via {
            Via::Value(v) => (g, 2, values[v.0 as usize].0 .0),
            Via::Module(m) => (g, 1, m.0),
            Via::Path(p) => (g, 3, types.canonical(p).0),
            Via::Scope => (g, 0, 0),
        };
        for level in &mut levels {
            level.sort_by_key(key);
            level.dedup_by_key(|r| key(r));
        }
        for level in &mut levels {
            level.retain(|&(g, _)| self.is_accessible(g));
        }
        levels
    }

    /// Where the conversion `g` found through `via` is called and what it takes and gives
    /// there: the site of an inherited one, the substitution of its owner's parameters and of its
    /// own, fresh, and the types it takes and gives as seen from the object or value it was
    /// found in, before the substitution; `None` where its own parameters' bounds fail, the
    /// constraints made the caller's to roll back.
    fn conversion_site(&mut self, g: SymId, via: Via, shape: &Shape) -> Option<ConversionSite> {
        let trait_owner = match self.syms.sym(g).owner {
            Owner::Class(c) if self.syms.class(c).kind != ClassKind::Object => Some(c),
            _ => None,
        };
        let site = match (trait_owner, via) {
            (Some(c), Via::Module(m)) => Some((super::exports::TraitMemberSite::Module(m), c)),
            (Some(c), Via::Path(p)) => Some((super::exports::TraitMemberSite::Path(p), c)),
            (Some(c), Via::Scope) => Some((self.trait_member_site(g, c), c)),
            _ => None,
        };
        let mut subst: Subst = match (site, via, trait_owner) {
            (Some((site, c)), _, _) => self.trait_member_subst(site, c),
            (None, Via::Value(v), Some(c)) => {
                let vty = self.import_value_ret(v);
                match self.base_type(vty, c) {
                    Some(bt) => self.owner_subst(bt),
                    None => Vec::new(),
                }
            }
            _ => Vec::new(),
        };
        for &tp in &shape.tparams {
            let v = self.fresh_var();
            subst.push((tp, v));
        }
        for &tp in &shape.tparams {
            let info = self.syms.tparam(tp);
            let (upper, lower) = (info.upper, info.lower);
            if self.canon(upper) != ANY || self.canon(lower) != NOTHING {
                let var = self.types.param(tp);
                let var = self.types.subst(var, &subst);
                let (upper, lower) = (self.types.subst(upper, &subst), self.types.subst(lower, &subst));
                if !(self.is_sub(var, upper) && self.is_sub(lower, var)) {
                    return None;
                }
            }
        }
        let (mut from, mut to) = (shape.from, shape.to);
        let prefix = match via {
            Via::Module(m) => Some(self.types.class(m, &[])),
            Via::Value(v) => Some(self.import_value_type(v)),
            Via::Path(p) => Some(p),
            // One a class in scope inherits is seen from that class's `this` (`~` of a
            // `ParsersBase` conversion in a subclass is the subclass's).
            Via::Scope => match site {
                Some((super::exports::TraitMemberSite::This(Some(k)), _)) => Some(self.this_prefix(k)),
                // One an imported object inherits is seen from that object, as its member is
                // (`all.SeqNode[A]` of a `Util[Builder]`'s conversion, the class nested in the
                // trait taking the object's arguments).
                Some((super::exports::TraitMemberSite::Module(m), _)) => Some(self.types.class(m, &[])),
                _ => None,
            },
        };
        if let (Some(prefix), Some(c)) = (prefix, trait_owner) {
            from = self.as_seen_from(from, prefix, c);
            to = self.as_seen_from(to, prefix, c);
        }
        Some(ConversionSite { site, subst, from, to })
    }

    /// The conversions in scope and in the receiver's implicit scope that take a receiver of
    /// type `t` where they are found, each with the type it gives: what a completion offers the
    /// members of. One that needs givens of its own is left out, since whether they resolve is
    /// the search's to say, which records what it finds. The constraints the checks make are
    /// rolled back.
    pub(super) fn conversion_targets(&mut self, t: TypeId) -> Vec<(SymId, TypeId, usize)> {
        if t == ERROR || self.types.contains_error(t) {
            return Vec::new();
        }
        let levels = self.conversion_levels(&[t], Pick::Any);
        let mut out: Vec<(SymId, TypeId, usize)> = Vec::new();
        for (level, (g, via)) in levels.into_iter().enumerate().flat_map(|(i, l)| l.into_iter().map(move |c| (i, c))) {
            if out.iter().any(|&(k, _, _)| k == g) || self.sig_of(g).clauses.iter().any(|c| c.is_using) {
                continue;
            }
            let Some(shape) = self.shape_of(g) else { continue };
            let mark = self.snapshot();
            let first_var = self.tvars.len();
            if let Some(ConversionSite { subst, from, to, .. }) = self.conversion_site(g, via, &shape) {
                let from = self.types.subst(from, &subst);
                if self.is_sub(t, from) {
                    let to = self.types.subst(to, &subst);
                    // As the conversion's application solves them: what the receiver bounds.
                    for v in first_var..self.tvars.len() {
                        self.solve_var_directed(self.tvars.id(v), Some(to));
                    }
                    let to = self.zonk(to);
                    if !self.types.contains_error(to) && !self.types.has_vars(to) {
                        out.push((g, to, level));
                    }
                }
            }
            self.rollback(&mark);
        }
        out
    }

    /// Applies the conversion `g` to `recv` when it takes the receiver's type and `accept`
    /// takes its result type; the bindings stay in place on success, and what a failure
    /// captured goes with it.
    fn try_conversion(
        &mut self,
        conv: ConvRef,
        recv: TExprId,
        recv_ty: TypeId,
        span: Span,
        accept: &mut dyn FnMut(&mut Self, TypeId) -> bool,
        to_type: bool,
    ) -> Option<(TExprId, TypeId)> {
        // A conversion tried is an attempt of its own: what it wrote
        // goes with it where it does not apply.
        let mark = self.attempt();
        let captured = self.capture_mark();
        let converted = self.try_conversion_in(conv, recv, recv_ty, span, accept, to_type);
        if converted.is_none() {
            self.retract(mark);
            if captured.is_some() {
                self.capture_drop_since(captured);
            }
        } else {
            self.close(mark);
            // The language server's index steps through a conversion no source wrote.
            if let (Some((te, _)), Some(ix)) = (converted, self.index.as_mut()) {
                ix.conversions.insert(te, ());
            }
        }
        converted
    }

    fn try_conversion_in(
        &mut self,
        (g, via): ConvRef,
        recv: TExprId,
        recv_ty: TypeId,
        span: Span,
        accept: &mut dyn FnMut(&mut Self, TypeId) -> bool,
        to_type: bool,
    ) -> Option<(TExprId, TypeId)> {
        let shape = self.shape_of(g)?;
        self.profile.tries += self.profile.on as u32;
        let mark = self.snapshot();
        let first_var = self.tvars.len();
        // Kept until the candidate's using clauses are resolved below.
        let sig = self.sig_arc(g);
        let is_def = self.syms.sym(g).kind == SymKind::Def && sig.clauses.first().map_or(false, |c| !c.is_using);
        let Some(ConversionSite { site, subst, from, to }) = self.conversion_site(g, via, &shape) else {
            self.rollback(&mark);
            return None;
        };
        let declared_to = to;
        let from = self.types.subst(from, &subst);
        // A conversion from a singleton type takes a stable receiver of that type, which the
        // receiver's widened type does not show (`Ok("body")` for http4s's `Ok.type` syntax).
        let from_head = self.deref(from);
        let singleton_from = matches!(self.types.get(from_head), Type::Term(_) | Type::Select(..));
        let widened = self.snapshot();
        let mut recv = recv;
        // The receiver is the view's argument, a path's type compared as the path's
        // (`inferView`'s `argument.tpe`).
        self.alias_bound_as_path = self.is_stable_path(recv);
        if !self.is_sub(recv_ty, from) {
            self.rollback(&widened);
            // The receiver is the conversion's argument, and a receiver of a numeric constant type
            // is converted to the parameter's type as scalac adapts a constant (`44.usd` for an
            // `implicit class Usd(n: Double)`, a `val n: 44`): a literal has its class type here
            // unless the expected type asks for the singleton, so the literal itself counts, but
            // not one ascribed a wider type (`44: Int`), whose type is the ascription's.
            // A reference is typed with its literal type widened, so a `val n: 44` is read by
            // its declared type.
            let numeric_lit = |t: &Self, ty: TypeId| matches!(t.types.get(ty), Type::Lit(l) if matches!(t.types.lit_val(l), LitVal::Int(_) | LitVal::Long(_) | LitVal::Double(_) | LitVal::Float(_) | LitVal::Char(_)));
            let declared = match self.prog.expr(recv) {
                TExpr::Local(s) | TExpr::Static(s) | TExpr::Field(_, s) => self.syms.sym(s).sig.as_ref().map(|sig| sig.ret),
                _ => None,
            };
            let constant = numeric_lit(self, recv_ty)
                || declared.map_or(false, |t| numeric_lit(self, t))
                || (self.constant_expr(recv) && self.ascribed_constant != Some(recv));
            let number = self.deref(recv_ty);
            let number = self.widen_lit(number);
            let widened = if constant { self.widen_numeric(recv, number, from) } else { None };
            match widened {
                Some(w) => recv = w,
                None if singleton_from && self.conforms_as_path(recv, from) => {}
                None => {
                    self.rollback(&mark);
                    return None;
                }
            }
        }
        let to = self.types.subst(to, &subst);
        if !accept(self, to) {
            // A library conversion to a shape teq cannot express is what blocks the use.
            if self.blocked_conversion.is_none() && super::loader::types::first_blocked(&self.types, to).is_some() {
                self.blocked_conversion = Some(to);
            }
            self.rollback(&mark);
            return None;
        }
        // A by-name parameter takes the receiver as a thunk (`implicit def f(fa: => F[A])`).
        let by_name_recv = is_def && sig.clauses.first().map_or(false, |c| c.params.first().map_or(false, |p| p.by_name));
        let recv_arg = if by_name_recv { self.by_name_thunk(recv) } else { recv };
        let mut args = vec![recv_arg];
        if is_def && sig.clauses.len() > 1 {
            // The receiver's type fixes what it bounds from below before the using clauses are
            // searched, as scalac instantiates the variables of the parameters: `fromPut(o)`
            // for an `Obj` looks for a `Put[Obj]`, not for any `Put[A >: Obj]`.
            for v in first_var..self.tvars.len() {
                let info = &self.tvars[v];
                if info.inst.is_none() && !info.lower.is_empty() {
                    self.solve_var(self.tvars.id(v));
                }
            }
        }
        if is_def {
            for clause in sig.clauses.iter().skip(1) {
                for p in &clause.params {
                    let pty = self.types.subst(p.ty, &subst);
                    match self.resolve_given(pty, span) {
                        Some(arg) => args.push(if p.by_name { self.by_name_thunk(arg) } else { arg }),
                        None => {
                            self.given_ambiguity = None;
                            self.rollback(&mark);
                            return None;
                        }
                    }
                }
            }
        }
        // As scalac interpolates the conversion's result: a variable it holds covariantly and
        // only open variables bound from above is `Nothing` (`stringToNode[M](s): Node[M]`
        // against a `Document[?M]`), not the outer variable.
        // A variable the result holds nonvariantly stays open for what follows when the conversion
        // gives a member its receiver (dotty's `interpolateTypeVars`: "no interpolation for
        // nonvariant" variables): cats' `catsSyntaxApplicativeId(NoFile(..))` is an
        // `ApplicativeIdOps[?A]`, whose `pure[F]` the expected `F[Result[A]]` then fixes.
        // One the receiver bounds from both sides (an invariant `Box[A]` taking a `Box[String]`)
        // is fixed by its constraint, and instantiated as dotty instantiates it at the end. Only the
        // conversion's own variables qualify (`interpolateTypeVars`' owned variables, an enclosing
        // application's locked): none where the conversion made none.
        let made = first_var..self.tvars.len();
        let open_from = self.conversion_open_vars.len();
        if !to_type && !made.is_empty() {
            let params = self.nonvariant_params(declared_to, to, &subst, &made);
            for &p in params.iter() {
                let Some(&(_, t)) = subst.iter().find(|&&(q, _)| q == p) else { continue };
                let t = self.deref(t);
                let Type::Var(w) = self.types.get(t) else { continue };
                if made.clone().any(|v| self.tvars.id(v) == w) && !self.fixed_by_bounds(w) {
                    self.conversion_open_vars.push(w);
                }
            }
        }
        for v in first_var..self.tvars.len() {
            let id = self.tvars.id(v);
            if !self.conversion_open_vars[open_from..].contains(&id) {
                self.solve_var_directed(id, Some(to));
            }
        }
        let mut given_result = None;
        let te = if is_def {
            let l = self.prog.list(&args);
            let call = match self.syms.sym(g).owner {
                Owner::Class(c) if self.syms.class(c).kind == ClassKind::Object => {
                    let m = self.prog.add(TExpr::Module(c));
                    if self.syms.sym(g).intrinsic.is_some() {
                        let call = super::apply::MethodCall { recv: Some(m), sym: g, owner_subst: Vec::new(), ext_recv: None, prefix: None, refined: None };
                        self.build_call(&call, l)
                    } else {
                        self.prog.add(TExpr::CallMethod(m, g, l))
                    }
                }
                Owner::Class(owner) => {
                    // A conversion an imported object inherits (`Predef`'s `intWrapper` from
                    // `LowPriorityImplicits`) is called on that object where no enclosing class
                    // has it.
                    let r = match site {
                        None if matches!(via, Via::Value(_)) => {
                            let Via::Value(v) = via else { unreachable!() };
                            match self.import_value_ref(v, span) {
                                Some((r, _)) => r,
                                None => {
                                    self.rollback(&mark);
                                    return None;
                                }
                            }
                        }
                        Some((super::exports::TraitMemberSite::This(None), _)) | None => match self.imported_module_with_base(owner) {
                            Some(obj) => self.prog.add(TExpr::Module(obj)),
                            None => self.prog.add(TExpr::This),
                        },
                        Some((site, _)) => self.trait_member_receiver(site),
                    };
                    self.prog.add(TExpr::CallMethod(r, g, l))
                }
                Owner::Package(_) | Owner::Local => self.prog.add(TExpr::CallStatic(g, l)),
            };
            if self.is_inline_callee(g) {
                self.inline_conversion(call, g, &sig, &subst, &args, span)
            } else {
                call
            }
        } else {
            // The receiver and the result the caller accepted fix the given's parameters before
            // its using clauses are resolved (`defaultToNoBackend[X, P, S](using ev: X =>
            // Step2[P, S]): Conversion[X, ..]`).
            let instance_vars = self.tvars.len();
            let target = match self.conversion_class_of(g) {
                Some(conversion) => {
                    let result = self.fresh_var();
                    let bound = self.snapshot();
                    if !accept(self, result) {
                        self.rollback(&bound);
                    }
                    Some(self.types.class(conversion, &[recv_ty, result]))
                }
                _ => None,
            };
            // The instance is reached as it was found: through the value an import opens, or the
            // object of an implicit scope that inherits it.
            let instance = match via {
                Via::Value(v) => self.import_value_ref(v, span).map(|(r, rty)| {
                    let name = self.syms.sym(g).name;
                    let (te, ty) = self.apply_member(r, rty, name, None, Vec::new(), span, None);
                    (te, ty)
                }),
                Via::Module(m) => self.instantiate_given_with((g, GivenScope::Module(m)), target, span, to_type),
                Via::Path(p) => self.instantiate_given_with((g, GivenScope::Path(p)), target, span, to_type),
                Via::Scope => self.instantiate_given_with((g, GivenScope::Lexical), target, span, to_type),
            };
            let Some((instance, inst_ty)) = instance else {
                self.rollback(&mark);
                return None;
            };
            let lists = vec![ArgList { args: vec![ArgSrc::Typed(recv, recv_ty)], using: false, span, fun: ArgList::NO_FUN }];
            let blocked = std::mem::replace(&mut self.no_receiver_conversion, true);
            let recorded = self.index_mark();
            let (te, applied) = self.apply_member(instance, inst_ty, names::APPLY, None, lists, span, None);
            self.no_receiver_conversion = blocked;
            // The application to the value is no list the program wrote (signature help's).
            if self.index.is_some() {
                self.index_forget_applied(recorded);
            }
            // What the instance's using clauses fixed (`Step3[P, S, Unit]` with `P` and `S` from
            // `ev`) is the result: dotty's `typedImplicit` types the view as the instance applied
            // to the receiver (`tryConversion`, Implicits.scala 1204-1217), of that application's
            // type with its variables and their constraints, an enclosing application's (the
            // receiver's nonvariant `S` of `initialState[S]`, which `ev` made one with the
            // instance's) and the instance's own alike (`conv[B]: Conversion[Raw, Box[B]]`'s `B`,
            // which the member selected on the view then fixes: `put("three")` makes it a
            // `String`).
            // Where the view gives a member its receiver, the instance's variables were left open
            // and are interpolated as dotty's `interpolateTypeVars` takes the application's
            // (`instantiate_vars`): their variances read through the bounds (`variances`'
            // `propagate`, Inferencing.scala 555-597: `Box[+A, B <: A]`'s nonvariant `B` makes
            // `A` nonvariant too), one held nonvariantly stays open for the member ("no
            // interpolation for nonvariant", 745), which the selection's interpolation settles
            // after it (`conversion_open_vars`, as an `implicit def`'s), and the others are
            // instantiated in their direction, a variable waiting for the one it depends on
            // (`instantiateTypeVars`' filtering and ordering): `Sink[-A]`'s `A` is `Any`.
            if !to_type {
                let applied_z = self.zonk(applied);
                // The instance's variables in the order dotty's owned variables are met, the view's
                // target variable (teq's stand-in for the `ViewProto`, which no type of dotty's
                // holds) after them: its lower bound holds them, which its instantiation would
                // otherwise settle before their own direction does.
                let occurring: Vec<TVarId> = self.var_variances(applied_z).into_iter().map(|(v, _)| v).collect();
                let mut vars: Vec<TVarId> = (instance_vars..self.tvars.len()).map(|k| self.tvars.id(k)).filter(|&v| self.tvars[v].inst.is_none()).collect();
                vars.sort_by_key(|v| !occurring.contains(v));
                let open_mark = self.open_vars.len();
                self.instantiate_vars(&vars, instance_vars, applied_z, None);
                let left: Vec<TVarId> = self.open_vars[open_mark.min(self.open_vars.len())..].to_vec();
                self.conversion_open_vars.extend(left);
            }
            let applied = self.zonk(applied);
            if applied != ERROR {
                if !accept(self, applied) {
                    self.rollback(&mark);
                    return None;
                }
                given_result = Some(applied);
            }
            te
        };
        // What the variables were solved to is checked once more where the caller wants a type:
        // the receiver still conforms to the parameter and the result to that type. A member
        // the result was found to have is the class's, which solving does not change.
        let ty = given_result.unwrap_or_else(|| self.zonk(to));
        if to_type && {
            let from = self.zonk(from);
            !self.is_sub(recv_ty, from) || !accept(self, ty)
        } {
            self.rollback(&mark);
            return None;
        }
        if self.notes_type_trees() && is_def && !shape.tparams.is_empty() {
            let own: Subst = subst.iter().filter(|(p, _)| shape.tparams.contains(p)).copied().collect();
            self.note_type_args(te, &own);
        }
        if self.capturing() && is_def && !shape.tparams.is_empty() {
            let own: Vec<TypeId> = subst.iter().filter(|(p, _)| shape.tparams.contains(p)).map(|&(_, t)| t).collect();
            self.capture_call_targs(te, &own);
        }
        self.prog.set_type(te, ty);
        Some((te, ty))
    }

    /// The call `te` of an `implicit inline def` conversion, expanded where it is inserted as an
    /// inline method's call is (dotty's `Inlines.needsInlining` of the adapted tree, the conversion
    /// typed as an application of its reference): a macro among them runs there, a plain one in the
    /// later phase; quoted code and the definition check keep the call, as for an inline given.
    #[cold]
    #[inline(never)]
    fn inline_conversion(&mut self, te: TExprId, g: SymId, sig: &Arc<MethodSig>, subst: &Subst, args: &[TExprId], span: Span) -> TExprId {
        let recv = match self.prog.expr(te) {
            TExpr::CallMethod(r, _, _) => Some(r),
            _ => None,
        };
        let call = super::apply::MethodCall { recv, sym: g, owner_subst: Vec::new(), ext_recv: None, prefix: None, refined: None };
        let ret_ty = self.types.subst(sig.ret, subst);
        let ret_ty = self.zonk(ret_ty);
        let kept = match self.prog.expr(te) {
            TExpr::CallMethod(..) => self.keeps_inline_calls(),
            _ => self.checks_inline_definition(),
        };
        if kept {
            self.defer_inline(&call, sig, subst, te, ret_ty, span, Some(ret_ty));
            self.quote.deferred_at_end(te);
            return te;
        }
        if self.defers_plain_inline(g) && !self.types.contains_error(ret_ty) && !self.eager_inline_conversion {
            self.defer_plain_inline(te, &call, sig, subst, args, Vec::new(), ret_ty, ret_ty, span, Some(ret_ty));
            return te;
        }
        match self.expand_inline(&call, sig, subst, args, ret_ty, span, Some(ret_ty)) {
            Some((e, _)) => e,
            None => te,
        }
    }

    /// The conversion method `given` applied to `arg`, where its result conforms to `to`; the
    /// given search's candidate of a function type.
    pub(super) fn conversion_method_body(&mut self, (g, scope): super::implicits::GivenRef, arg: TExprId, from: TypeId, to: TypeId, span: Span) -> Option<TExprId> {
        let via = match scope {
            GivenScope::Module(m) => Via::Module(m),
            GivenScope::Value(v) => Via::Value(v),
            GivenScope::Lexical => Via::Scope,
            GivenScope::Path(p) => Via::Path(p),
        };
        let mut accept = |t: &mut Self, result: TypeId| t.is_sub(result, to);
        // A candidate is typed as dotty's search types it, exploring: one whose application fails
        // where it is typed, an inline conversion's expansion reporting `compiletime.error` among
        // them, is no candidate (`uhOh` of i19862 gives way to `$conforms`).
        let mark = self.attempt();
        // Expanded here, not in the later phase: the candidate's expansion is its typing, as dotty
        // inlines in the typer.
        let outer = std::mem::replace(&mut self.eager_inline_conversion, true);
        let tried = self.try_conversion((g, via), arg, from, span, &mut accept, true);
        self.eager_inline_conversion = outer;
        match tried {
            Some((te, _)) if !self.attempt_failed(&mark) => {
                self.close(mark);
                Some(te)
            }
            _ => {
                self.retract(mark);
                None
            }
        }
    }

    /// The conversion of the first level with an applicable one, the most specific where
    /// several apply. Reported as ambiguous when `report` is set.
    fn pick_conversion(
        &mut self,
        levels: Vec<Vec<ConvRef>>,
        recv: TExprId,
        recv_ty: TypeId,
        span: Span,
        report: bool,
        wanted: &dyn Fn(&mut Self) -> String,
        provided: &dyn Fn(&mut Self) -> String,
        accept: &mut dyn FnMut(&mut Self, TypeId) -> bool,
        to_type: bool,
    ) -> Option<Result<(SymId, TExprId, TypeId), ()>> {
        if self.implicit_depth > 0 {
            self.read_in_progress(super::implicits::InProgress::ViewLookup);
        }
        for level in levels {
            if level.len() == 1 {
                if let Some((te, ty)) = self.try_conversion(level[0], recv, recv_ty, span, accept, to_type) {
                    self.attribute_conversion(level[0], te);
                    self.watch_for_map_conversion(te, ty);
                    return Some(Ok((level[0].0, te, ty)));
                }
                continue;
            }
            // Each conversion of the level is an attempt of its own; a success is set aside while
            // the others are tried, and the winner's is restored with the tree it typed, as dotty
            // commits the winning candidate's state.
            let mut found: Vec<Candidate> = Vec::new();
            let mut aside: Vec<Option<super::state::SetAside>> = Vec::new();
            for &g in &level {
                let mark = self.attempt();
                if let Some((expr, ty)) = self.try_conversion(g, recv, recv_ty, span, accept, to_type) {
                    found.push(Candidate { sym: g.0, via: g.1, expr, ty });
                    aside.push(Some(self.set_aside(mark)));
                } else {
                    self.retract(mark);
                }
            }
            if found.is_empty() {
                continue;
            }
            let best = self.most_specific_conversion(&found);
            let Some(best) = best else {
                if report {
                    let described: Vec<String> = found
                        .iter()
                        .map(|c| {
                            let what = if self.syms.sym(c.sym).kind == SymKind::Def { "method" } else { "value" };
                            format!("{} {}", what, self.method_description(c.sym))
                        })
                        .collect();
                    let msg = format!(
                        "type mismatch: found {}, required {}. Note that implicit extension methods cannot be applied because they are ambiguous; both {} and {} provide {}",
                        self.show(recv_ty),
                        wanted(self),
                        described[0],
                        described[1],
                        provided(self),
                    );
                    self.error(span, msg);
                }
                return Some(Err(()));
            };
            let g = found[best].sym;
            if let Some(state) = aside[best].take() {
                self.restore(state);
            }
            let (te, ty) = (found[best].expr, found[best].ty);
            self.attribute_conversion((g, found[best].via), te);
            self.watch_for_map_conversion(te, ty);
            return Some(Ok((g, te, ty)));
        }
        None
    }

    /// A committed conversion's node and type, kept while a comprehension's trailing `map` is typed
    /// (`Worker::for_map_watch`), whose `DropForMap` compares the receiver it converted.
    #[inline]
    fn watch_for_map_conversion(&mut self, te: TExprId, ty: TypeId) {
        if let Some(w) = self.for_map_watch.as_mut() {
            w.conversions.push((te, ty));
        }
    }

    /// The conversion `conv` was committed with the tree `te`: the import credited is one that
    /// brings it on the object or value it was read on (CheckUnused's prefix test), as for a
    /// given (`Unused::receiver`).
    fn attribute_conversion(&mut self, (g, via): ConvRef, te: TExprId) {
        // The conversion's reference reads it (`refUsage` of the tree the typer inserts).
        self.use_sym(g, Span::default());
        if !self.unused.on() {
            return;
        }
        let scope = match via {
            Via::Module(m) => GivenScope::Module(m),
            Via::Value(v) => GivenScope::Value(v),
            Via::Path(p) => GivenScope::Path(p),
            Via::Scope => match self.conversion_receiver(te, g) {
                Some(m) => GivenScope::Module(m),
                None => GivenScope::Lexical,
            },
        };
        self.attribute_via(g, scope);
    }

    /// The object the tree of a conversion reads `g` on: `O.g(x)`, or `O.g.apply(x)` of a given
    /// `Conversion`.
    fn conversion_receiver(&self, te: TExprId, g: SymId) -> Option<ClassId> {
        match self.prog.expr(te) {
            TExpr::Field(r, s) | TExpr::CallMethod(r, s, _) if s == g => match self.prog.expr(r) {
                TExpr::Module(m) => Some(m),
                _ => None,
            },
            TExpr::Field(r, _) | TExpr::CallMethod(r, _, _) => self.conversion_receiver(r, g),
            TExpr::Block(_, last) => self.conversion_receiver(last, g),
            _ => None,
        }
    }

    fn most_specific_conversion(&mut self, found: &[Candidate]) -> Option<usize> {
        let mut best = 0;
        for i in 1..found.len() {
            if self.compare_conversions((found[i].sym, found[i].via), (found[best].sym, found[best].via)) > 0 {
                best = i;
            }
        }
        let beats_all = (0..found.len()).all(|i| i == best || self.compare_conversions((found[best].sym, found[best].via), (found[i].sym, found[i].via)) > 0);
        beats_all.then_some(best)
    }

    /// dotc's `compare` for two conversions: the one whose owner derives from the other's wins
    /// unless the types say otherwise, and the one that takes the more specific type wins.
    fn compare_conversions(&mut self, a: (SymId, Via), b: (SymId, Via)) -> i32 {
        let owners = self.compare_given_owners(a.0, b.0);
        let a_wins = self.takes_more_specific(a, b);
        let b_wins = self.takes_more_specific(b, a);
        match owners {
            1 => (a_wins || !b_wins) as i32,
            -1 => -((b_wins || !a_wins) as i32),
            _ => a_wins as i32 - b_wins as i32,
        }
    }

    /// Whether `b` could be applied to `a`'s parameter, `a`'s type parameters read as abstract
    /// types within their bounds and `b`'s inferred (SLS 6.26.3): a conversion from
    /// `Either[A, B]` is as specific as one from `F[A, B]`, not the reverse.
    fn takes_more_specific(&mut self, a: (SymId, Via), b: (SymId, Via)) -> bool {
        let (Some(sa), Some(sb)) = (self.shape_seen_from(a), self.shape_seen_from(b)) else { return false };
        let mark = self.snapshot();
        let subst_b: Subst = sb.tparams.iter().map(|&tp| (tp, self.fresh_var())).collect();
        let from_b = self.types.subst(sb.from, &subst_b);
        // A number widens to `b`'s parameter as it does to a conversion's receiver, so the one
        // taking the number as it is wins (`durationInt` over `durationLong` for `5.seconds`).
        let mut ok = self.is_sub(sa.from, from_b) || self.numeric_widening(sa.from, from_b);
        // `b`'s type parameters keep their bounds: `wrapRefArray[T <: AnyRef]` does not take
        // what `genericWrapArray[T]` takes.
        for &tp in &sb.tparams {
            if !ok {
                break;
            }
            let info = self.syms.tparam(tp);
            let (upper, lower) = (info.upper, info.lower);
            if self.canon(upper) != ANY || self.canon(lower) != NOTHING {
                let var = self.types.param(tp);
                let var = self.types.subst(var, &subst_b);
                let (upper, lower) = (self.types.subst(upper, &subst_b), self.types.subst(lower, &subst_b));
                ok = self.is_sub(var, upper) && self.is_sub(lower, var);
            }
        }
        self.rollback(&mark);
        ok
    }

    /// A numeric literal, with or without a sign: one constant to scalac's parser.
    pub(super) fn constant_expr(&self, e: TExprId) -> bool {
        match self.prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Char(_) => true,
            TExpr::Unary(UnOp::IntNeg | UnOp::LongNeg | UnOp::DoubleNeg | UnOp::FloatNeg, inner) => {
                matches!(self.prog.expr(inner), TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_))
            }
            _ => false,
        }
    }

    /// The shape of a conversion as the object it was found in sees it, as scalac compares the
    /// candidates' types under their prefixes: the `toTwiddleOpTwo` that `object Codec extends
    /// TwiddleSyntax[Codec]` inherits takes a `Codec[B]`, the one of `Decoder` a `Decoder[B]`.
    fn shape_seen_from(&mut self, (g, via): (SymId, Via)) -> Option<Shape> {
        let shape = self.shape_of(g)?;
        let (Via::Module(m), Owner::Class(owner)) = (via, self.syms.sym(g).owner) else { return Some(shape) };
        self.settle_class(owner);
        if owner == m || self.syms.class(owner).tparams.is_empty() {
            return Some(shape);
        }
        let module_ty = self.types.class(m, &[]);
        let Some(bt) = self.base_type(module_ty, owner) else { return Some(shape) };
        let subst = self.owner_subst(bt);
        Some(Shape { tparams: shape.tparams, from: self.types.subst(shape.from, &subst), to: self.types.subst(shape.to, &subst) })
    }

    /// Whether the member `name` of `to`, applied to `lists`, gives what `expected` asks for;
    /// an alternative whose result its type parameters decide fits.
    fn member_result_fits(&mut self, to: TypeId, name: Name, lists: &[ArgList], expected: TypeId) -> bool {
        let to = self.zonk(to);
        let Some((sym, owner_ty)) = self.find_member(to, name) else { return true };
        let alts = match self.syms.alternatives(sym) {
            Some(alts) => alts.to_vec(),
            None => vec![sym],
        };
        let given = lists.iter().filter(|l| !l.using).count();
        let subst = self.owner_subst(owner_ty);
        alts.into_iter().any(|alt| {
            let sig = self.sig_of(alt);
            if sig.clauses.iter().filter(|c| !c.is_using).count() != given {
                return false;
            }
            if !sig.tparams.is_empty() {
                return true;
            }
            let ret = sig.ret;
            let ret = self.types.subst(ret, &subst);
            // `a.Out` of a result reached through the path `a.type`, which the path's type fixes.
            let ret = self.seen_from_prefix(ret, to, alt);
            let mark = self.snapshot();
            let fits = self.is_sub(ret, expected);
            self.rollback(&mark);
            fits
        })
    }

    /// Whether `to` has a member `name` that takes the arguments, as the selection through a
    /// conversion needs; extension methods of the result do not count.
    fn result_has_member(&mut self, to: TypeId, name: Name, lists: &[ArgList], expected: Option<TypeId>) -> bool {
        let to = self.zonk(to);
        if let Some((sym, _)) = self.find_member(to, name) {
            // A member the selection could not access is none: a class parameter of the
            // conversion's class (`TraversableOnceExt(as)`) leaves `as` to other conversions.
            if !self.is_accessible(sym) {
                return false;
            }
            if let Some(alts) = self.syms.alternatives(sym) {
                return alts.to_vec().into_iter().any(|a| self.is_accessible(a));
            }
            if self.syms.sym(sym).kind != SymKind::Def {
                return true;
            }
            // `t === (1, "a")` through `catsSyntaxEq`: the member takes the tuple auto-tupling packs.
            return self.member_accepts(sym, lists, false, expected) || self.member_takes_tuple(sym, to, lists);
        }
        if name == names::APPLY && self.as_function(to).is_some() {
            return true;
        }
        // A conversion to its own type parameter (refined's `autoUnwrap[F[_, _], T, P]: T`)
        // gives what the receiver fixed the parameter to, a builtin's operators included; the
        // variable is solved for the check alone.
        let Type::Var(v) = self.types.get(to) else { return self.prim_op_applies(to, name) || self.has_member_like(to, name) };
        let mark = self.snapshot();
        self.solve_var(v);
        let solved = self.zonk(to);
        let has = self.prim_op_applies(solved, name) || self.has_member_like(solved, name);
        self.rollback(&mark);
        has
    }

    /// The receiver `recv.name(args)` converted so that the selection can go on, when a
    /// conversion in scope or in the implicit scope of the receiver gives a type with such a
    /// member; dotc's `tryInsertImplicitOnQualifier`.
    pub fn convert_for_member(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        lists: &[ArgList],
        expected: Option<TypeId>,
        span: Span,
    ) -> Option<Result<(TExprId, TypeId), ()>> {
        if recv_ty == ERROR || self.types.contains_error(recv_ty) {
            return None;
        }
        let levels = self.conversion_levels(&[recv_ty], Pick::Member(name));
        if levels.is_empty() {
            return None;
        }
        let prof = self.prof(Kind::Conversion, span, About::Member(recv_ty, name));
        let mut accept = |t: &mut Self, to: TypeId| t.result_has_member(to, name, lists, expected);
        let wanted = |t: &mut Self| format!("?{{ {}: ? }}", t.name_str(name));
        let provided = |t: &mut Self| format!("an extension method `{}` on {}", t.name_str(name), t.show(recv_ty));
        self.blocked_conversion = None;
        let mut picked = self.pick_conversion(levels.clone(), recv, recv_ty, span, false, &wanted, &provided, &mut accept, false);
        // dotc's `inferImplicit` searches once more on an ambiguity with the expected type of the
        // member revealed (`deepenProto`): `7.days` for a `FiniteDuration` takes scala's
        // `DurationInt`, whose `days` gives one, over zio's `durationInt`.
        if matches!(picked, Some(Err(()))) {
            if let Some(exp) = self.concrete_expected(expected).filter(|&t| t != ANY && t != self.b.t_unit) {
                let mut deep = |t: &mut Self, to: TypeId| t.result_has_member(to, name, lists, expected) && t.member_result_fits(to, name, lists, exp);
                if let Some(Ok(found)) = self.pick_conversion(levels.clone(), recv, recv_ty, span, false, &wanted, &provided, &mut deep, false) {
                    picked = Some(Ok(found));
                }
            }
            if matches!(picked, Some(Err(()))) {
                picked = self.pick_conversion(levels, recv, recv_ty, span, true, &wanted, &provided, &mut accept, false);
            }
        }
        let picked = match (picked, self.blocked_conversion.take()) {
            (None, Some(blocked)) => {
                self.report_blocked_type(blocked, span);
                Some(Err(()))
            }
            (picked, _) => picked,
        };
        if let Some(Ok((g, _, _))) = &picked {
            self.restrict_conversion_use(*g, span);
        }
        if let Some(p) = prof {
            self.profile.exit(p, conversion_outcome(&picked));
        }
        picked.map(|r| r.map(|(_, te, ty)| (te, ty)))
    }

    /// Whether a conversion of `recv_ty` gives a type with the member `name` that `accept` takes:
    /// dotty's `viewExists` to a deeper selection's prototype (Implicits.scala 885), `inferView`
    /// testing each conversion's result against it, an ambiguity being no view. What the search
    /// does is the caller's to take back.
    pub(super) fn view_selects(&mut self, recv_ty: TypeId, name: Name, span: Span, accept: &mut dyn FnMut(&mut Self, TypeId) -> bool) -> bool {
        if recv_ty == ERROR || self.types.contains_error(recv_ty) {
            return false;
        }
        let levels = self.conversion_levels(&[recv_ty], Pick::Member(name));
        if levels.is_empty() {
            return false;
        }
        let probe = self.prog.add(TExpr::Unit);
        let wanted = |t: &mut Self| format!("?{{ {}: ? }}", t.name_str(name));
        let provided = |t: &mut Self| format!("an extension method `{}` on {}", t.name_str(name), t.show(recv_ty));
        let mut deep = |t: &mut Self, to: TypeId| t.result_has_member(to, name, &[], None) && accept(t, to);
        self.blocked_conversion = None;
        let picked = self.pick_conversion(levels, probe, recv_ty, span, false, &wanted, &provided, &mut deep, false);
        self.blocked_conversion = None;
        matches!(picked, Some(Ok(_)))
    }

    /// `te` converted to `expected`, when a conversion in scope or in the implicit scopes of
    /// the two types takes it there; dotc's `inferView`.
    /// A view is never applied to `null` or to an expression of type `Nothing`, as under scalac.
    fn is_bottom(&mut self, t: TypeId) -> bool {
        let t = self.deref_canonical(t);
        t == NOTHING || t == self.b.t_null
    }

    /// An object among the imports in scope, `scala.Predef` included, that extends `base`.
    fn imported_module_with_base(&self, base: ClassId) -> Option<ClassId> {
        let extends = |t: &Self, o: ClassId| t.syms.class(o).kind == ClassKind::Object && t.syms.class(o).base_types.iter().any(|&(b, _)| b == base);
        let imported = self.env.imports.iter().rev().filter_map(|imp| match imp.target {
            ImportTarget::ClassAll(o) | ImportTarget::ClassMember(o, _) => Some(o),
            _ => None,
        });
        imported.chain(self.loaded.as_ref().and_then(|l| l.predef)).find(|&o| extends(self, o))
    }

    pub fn convert_to(&mut self, te: TExprId, actual: TypeId, expected: TypeId, span: Span, report: bool) -> Option<TExprId> {
        self.convert_to_typed(te, actual, expected, span, report).map(|(te, _)| te)
    }

    /// A conversion into an upper bound of the open variable `v`, which is what scalac finds for
    /// an argument typed against a type parameter that the expected type has bounded
    /// (`Some(10)` against an `Option[Money]` with an `implicit def int2money`); the result of
    /// the conversion is then a lower bound of the variable.
    pub fn convert_to_var_bound(&mut self, te: TExprId, actual: TypeId, v: TVarId, expected: TypeId, span: Span) -> Option<TExprId> {
        self.convert_to_var_bound_typed(te, actual, v, expected, span).map(|(te, _)| te)
    }

    /// dotty's view into an open variable (`Implicits.inferView`, whose `ViewProto` takes the
    /// expected type's `wildApprox`): one search into the meet of its upper bounds, through the
    /// variables among them, each with the open variables it holds approximated by wildcards of
    /// their own bounds (`ProtoTypes.wildApprox`), so `Raw` finds a view to `List[String]` for a
    /// variable below `List[?X]`; the conversion's result then bounds the variable below.
    pub fn convert_to_var_bound_typed(&mut self, te: TExprId, actual: TypeId, v: TVarId, expected: TypeId, span: Span) -> Option<(TExprId, TypeId)> {
        let mut target = ANY;
        for u in self.upper_bounds_through_vars(v) {
            let u = self.zonk(u);
            let u = self.wild_approx_vars(u, &mut vec![v]);
            target = if self.canon(target) == ANY { u } else { self.types.inter(target, u) };
        }
        if self.canon(target) == ANY {
            return None;
        }
        let mark = self.attempt();
        if let Some((converted, ty)) = self.convert_to_typed(te, actual, target, span, false) {
            if ty != ERROR && self.is_sub(ty, expected) {
                self.close(mark);
                return Some((converted, ty));
            }
        }
        self.retract(mark);
        None
    }

    /// `t` with the open variables it holds approximated by wildcards of their bounds, as
    /// dotty's `wildApprox` approximates a constrained parameter by `WildcardType` of its
    /// constraint's bounds, themselves approximated: a variable met again on the way (`seen`, the
    /// variables being approximated) is a plain wildcard, the only cut, as dotty cuts a cycle. A
    /// bound that approximates to a wildcard bounds the variable by its own bound on that side
    /// (`B <: A` with `A <: Int` is below `Int`), and an intersection or a union with a wildcard part
    /// is the wildcard of the parts' bounds combined (`approxAnd`'s `wildBounds(tp1a) &
    /// wildBounds(tp2a)`, `approxOr`'s `tp1a.bounds | tp2a.bounds`, ProtoTypes.scala 1022-1042).
    fn wild_approx_vars(&mut self, t: TypeId, seen: &mut Vec<TVarId>) -> TypeId {
        if !self.types.has_vars(t) {
            return t;
        }
        let t = self.deref(t);
        if self.semantic {
            if let Some((under, r)) = self.types.form_of(t) {
                return self.wild_approx_vars_form(t, seen, under, r);
            }
        }
        match self.types.get(t) {
            Type::Var(v) => {
                if seen.contains(&v) {
                    return WILD;
                }
                seen.push(v);
                let mut lo = NOTHING;
                for k in 0..self.tvars[v].lower.len() {
                    let l = self.tvars[v].lower[k];
                    let a = self.wild_approx_vars(l, seen);
                    let l = self.wild_side(a, true);
                    if l != NOTHING {
                        lo = if lo == NOTHING { l } else { self.types.union_as(crate::types::UnionKind::Soft, lo, l) };
                    }
                }
                let mut hi = ANY;
                for k in 0..self.tvars[v].upper.len() {
                    let u = self.tvars[v].upper[k];
                    let a = self.wild_approx_vars(u, seen);
                    let u = self.wild_side(a, false);
                    if u != ANY {
                        hi = if hi == ANY { u } else { self.types.inter(hi, u) };
                    }
                }
                seen.pop();
                self.types.bounded_wild(lo, hi)
            }
            Type::Class(c, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let approx: Vec<TypeId> = items.iter().map(|&a| self.wild_approx_vars(a, seen)).collect();
                self.types.class(c, &approx)
            }
            Type::Union(a, b, soft) => {
                let (a, b) = (self.wild_approx_vars(a, seen), self.wild_approx_vars(b, seen));
                if !self.is_wild(a) && !self.is_wild(b) {
                    return self.types.union_as(soft, a, b);
                }
                // `TypeBounds.|`: the lower bounds met, the upper ones joined, a plain type its own
                // two bounds (`TypeAlias`).
                let (la, ha) = self.wild_part_bounds(a, a);
                let (lb, hb) = self.wild_part_bounds(b, b);
                let lo = if self.canon(la) == NOTHING || self.canon(lb) == NOTHING { NOTHING } else { self.types.inter(la, lb) };
                let hi = if self.canon(ha) == ANY || self.canon(hb) == ANY { ANY } else { self.types.union_as(soft, ha, hb) };
                self.types.bounded_wild(lo, hi)
            }
            Type::Inter(a, b) => {
                let (a, b) = (self.wild_approx_vars(a, seen), self.wild_approx_vars(b, seen));
                if !self.is_wild(a) && !self.is_wild(b) {
                    return self.types.inter(a, b);
                }
                // `TypeBounds.&` of `wildBounds`: a plain type an upper bound alone.
                let (la, ha) = self.wild_part_bounds(a, NOTHING);
                let (lb, hb) = self.wild_part_bounds(b, NOTHING);
                let lo = if la == NOTHING { lb } else if lb == NOTHING { la } else { self.types.union_as(crate::types::UnionKind::Soft, la, lb) };
                let hi = if ha == ANY { hb } else if hb == ANY { ha } else { self.types.inter(ha, hb) };
                self.types.bounded_wild(lo, hi)
            }
            _ => t,
        }
    }

    #[cold]
    #[inline(never)]
    fn wild_approx_vars_form(&mut self, t: TypeId, seen: &mut Vec<TVarId>, under: TypeId, r: crate::types::RetainId) -> TypeId {
        let mut ctx = vec![super::FORM_WILD_APPROX_VARS];
        ctx.extend(seen.iter().map(|v| v.0));
        self.map_form(&ctx, t, under, r, &mut |w, x| w.wild_approx_vars(x, seen))
    }

    fn is_wild(&self, t: TypeId) -> bool {
        matches!(self.types.get(t), Type::Wild | Type::BoundedWild(..))
    }

    /// The bounds a wildcard gives, a plain type `(lo, itself)`.
    fn wild_part_bounds(&self, t: TypeId, lo: TypeId) -> (TypeId, TypeId) {
        match self.types.get(t) {
            Type::Wild => (NOTHING, ANY),
            Type::BoundedWild(l, h) => (l, h),
            _ => (lo, t),
        }
    }

    /// An approximated bound as the side of a variable it bounds reads it: a wildcard by its
    /// own bound on that side.
    fn wild_side(&self, t: TypeId, lower: bool) -> TypeId {
        match self.types.get(t) {
            Type::Wild => if lower { NOTHING } else { ANY },
            Type::BoundedWild(l, h) => if lower { l } else { h },
            _ => t,
        }
    }

    pub(super) fn convert_to_typed(&mut self, te: TExprId, actual: TypeId, expected: TypeId, span: Span, report: bool) -> Option<(TExprId, TypeId)> {
        if actual == ERROR || expected == ERROR || self.types.contains_error(expected) || self.is_bottom(actual) {
            return None;
        }
        let target = self.deref_canonical(expected);
        if matches!(self.types.get(target), Type::Var(_) | Type::AppVar(..)) || target == ANY {
            return None;
        }
        // An adaptation tries no conversion to `AnyVal` or `AnyRef` itself, whose result would be
        // no more specific than the value (`Typer.adapt`'s `TooUnspecific`); a view to them is
        // still explored where overloads are resolved (`viewExists`, which reports nothing).
        if report && (target == self.b.t_any_val || target == self.b.t_any_ref) {
            return None;
        }
        let members;
        let pick = match self.types.get(target) {
            Type::Union(..) => {
                members = self.union_member_classes(target);
                Pick::Results(&members)
            }
            _ => Pick::Result(self.class_of(target)),
        };
        let levels = self.conversion_levels(&[actual, expected], pick);
        if levels.is_empty() {
            return None;
        }
        let prof = self.prof(Kind::Conversion, span, About::Type(expected));
        let mut accept = |t: &mut Self, to: TypeId| t.is_sub(to, expected);
        let wanted = |t: &mut Self| t.show(expected);
        let provided = |t: &mut Self| format!("a conversion from {} to {}", t.show(actual), t.show(expected));
        let picked = self.pick_conversion(levels, te, actual, span, report, &wanted, &provided, &mut accept, true);
        if let (Some(Ok((g, _, _))), true) = (&picked, report) {
            self.restrict_conversion_use(*g, span);
        }
        if let Some(p) = prof {
            self.profile.exit(p, conversion_outcome(&picked));
        }
        match picked? {
            Ok((_, converted, ty)) => Some((converted, ty)),
            Err(()) => Some((te, ERROR)),
        }
    }

    /// The classes of the members of the union `t`, through nested unions.
    fn union_member_classes(&mut self, t: TypeId) -> Vec<ClassId> {
        let mut out = Vec::new();
        let mut stack = vec![t];
        while let Some(t) = stack.pop() {
            let t = self.deref(t);
            match self.types.get(t) {
                Type::Union(a, b, _) => {
                    stack.push(b);
                    stack.push(a);
                }
                _ => {
                    if let Some(c) = self.class_of(t).filter(|c| !out.contains(c)) {
                        out.push(c);
                    }
                }
            }
        }
        out
    }

    /// Whether some conversion takes a value of `actual` to `expected`, nothing applied.
    /// A probe, dotty's `viewExists` under `explore`: what it writes goes with it.
    pub fn view_exists(&mut self, actual: TypeId, expected: TypeId, span: Span) -> bool {
        let mark = self.attempt();
        let probe = self.prog.add(TExpr::Unit);
        let found = self.convert_to(probe, actual, expected, span, false).is_some();
        self.retract(mark);
        found
    }
}
