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
        let Some(conversion) = self.b.conversion else { return false };
        let sig = self.sig_of(g);
        if sig.clauses.iter().any(|c| !c.is_using) {
            return false;
        }
        let ret = sig.ret;
        match self.class_of(ret) {
            Some(c) => {
                self.complete_class(c);
                self.syms.class(c).base_types.iter().any(|&(b, _)| b == conversion)
            }
            None => false,
        }
    }

    pub(super) fn shape_of(&mut self, g: SymId) -> Option<Shape> {
        let is_def = self.syms.sym(g).kind == SymKind::Def;
        let sig = self.sig_of(g);
        if is_def && sig.clauses.first().map_or(false, |c| !c.is_using) {
            let [param] = sig.clauses[0].params.as_slice() else { return None };
            return Some(Shape { tparams: sig.tparams.clone(), from: param.ty, to: sig.ret });
        }
        let ret = sig.ret;
        let conversion = self.b.conversion?;
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
            let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
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

    /// The candidate conversions level by level, as the given search orders its levels: the
    /// enclosing blocks and classes from the inside out with their imports, the package
    /// clauses, and last the implicit scopes of `scope_tys`.
    fn conversion_levels(&mut self, scope_tys: &[TypeId], pick: Pick<'_>) -> Vec<Vec<ConvRef>> {
        let mut levels: Vec<Vec<ConvRef>> = Vec::new();
        // One buffer for the conversions of a level, which the level's list takes as refs.
        let mut level: Vec<SymId> = Vec::new();
        let scoped = |level: &mut Vec<SymId>| -> Vec<ConvRef> { level.drain(..).map(|g| (g, Via::Scope)).collect() };
        for frame in (0..self.env.frames.len()).rev() {
            match &self.env.frames[frame] {
                Frame::Locals { givens, .. } => {
                    let locals: Vec<SymId> = givens.iter().rev().copied().collect();
                    for g in locals {
                        if self.syms.is_given(g) && (self.is_conversion_def(g) || self.is_conversion_given(g)) {
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
            let mut from_values = Vec::new();
            for i in 0..self.env.imports.len() {
                let imp = self.env.imports[i];
                if imp.depth == depth {
                    self.import_conversions(imp, pick, &mut level);
                    self.value_import_conversions(imp, pick, &mut from_values);
                }
            }
            if !level.is_empty() || !from_values.is_empty() {
                let mut refs = scoped(&mut level);
                refs.append(&mut from_values);
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
            if i == 0 {
                let n = self.import_count();
                for k in 0..n {
                    let imp = self.import_at(k);
                    if imp.depth == 0 {
                        self.import_conversions(imp, pick, &mut level);
                    }
                }
            }
            if !level.is_empty() {
                levels.push(scoped(&mut level));
            }
        }
        // `scala.Predef` of a classpath is a level of its own outside the package clauses.
        if let Some(predef) = self.loaded.as_ref().and_then(|l| l.predef).filter(|_| !self.predef_unimported()) {
            self.class_conversions(predef, pick, &mut level);
            if !level.is_empty() {
                levels.push(scoped(&mut level));
            }
        }
        let mut scope = Vec::new();
        for &t in scope_tys {
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
        let key = |&(g, via): &ConvRef| match via {
            Via::Value(v) => (g, 2, values[v.0 as usize].0 .0),
            Via::Module(m) => (g, 1, m.0),
            Via::Path(p) => (g, 3, p.0),
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
            if upper != ANY || lower != NOTHING {
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
            self.rollback(mark);
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
            self.rollback(mark);
            return None;
        };
        let from = self.types.subst(from, &subst);
        // A conversion from a singleton type takes a stable receiver of that type, which the
        // receiver's widened type does not show (`Ok("body")` for http4s's `Ok.type` syntax).
        let from_head = self.deref(from);
        let singleton_from = matches!(self.types.get(from_head), Type::Term(_) | Type::Select(..));
        let widened = self.snapshot();
        let mut recv = recv;
        if !self.is_sub(recv_ty, from) {
            self.rollback(widened);
            // The receiver is the conversion's argument, and a receiver of a numeric constant type
            // is converted to the parameter's type as scalac adapts a constant (`44.usd` for an
            // `implicit class Usd(n: Double)`, a `val n: 44`): a literal has its class type here
            // unless the expected type asks for the singleton, so the literal itself counts, but
            // not one ascribed a wider type (`44: Int`), whose type is the ascription's.
            // A reference is typed with its literal type widened, so a `val n: 44` is read by
            // its declared type.
            let numeric_lit = |t: &Self, ty: TypeId| matches!(t.types.get(ty), Type::Lit(l) if matches!(t.types.lit_val(l), LitVal::Int(_) | LitVal::Long(_) | LitVal::Double(_) | LitVal::Char(_)));
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
                    self.rollback(mark);
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
            self.rollback(mark);
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
                            self.rollback(mark);
                            return None;
                        }
                    }
                }
            }
        }
        // As scalac interpolates the conversion's result: a variable it holds covariantly and
        // only open variables bound from above is `Nothing` (`stringToNode[M](s): Node[M]`
        // against a `Document[?M]`), not the outer variable.
        for v in first_var..self.tvars.len() {
            self.solve_var_directed(self.tvars.id(v), Some(to));
        }
        let mut given_result = None;
        let te = if is_def {
            let l = self.prog.list(&args);
            match self.syms.sym(g).owner {
                Owner::Class(c) if self.syms.class(c).kind == ClassKind::Object => {
                    let m = self.prog.add(TExpr::Module(c));
                    if self.syms.sym(g).intrinsic.is_some() {
                        let call = super::apply::MethodCall { recv: Some(m), sym: g, owner_subst: Vec::new(), ext_recv: None, prefix: None };
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
                                    self.rollback(mark);
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
            }
        } else {
            // The receiver and the result the caller accepted fix the given's parameters before
            // its using clauses are resolved (`defaultToNoBackend[X, P, S](using ev: X =>
            // Step2[P, S]): Conversion[X, ..]`).
            let conversion = self.b.conversion;
            let target = match conversion {
                Some(conversion) if self.is_conversion_given(g) => {
                    let result = self.fresh_var();
                    let bound = self.snapshot();
                    if !accept(self, result) {
                        self.rollback(bound);
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
                Via::Module(m) => self.instantiate_given_with((g, GivenScope::Module(m)), target, span, true),
                Via::Path(p) => self.instantiate_given_with((g, GivenScope::Path(p)), target, span, true),
                Via::Scope => self.instantiate_given_with((g, GivenScope::Lexical), target, span, true),
            };
            let Some((instance, inst_ty)) = instance else {
                self.rollback(mark);
                return None;
            };
            let lists = vec![ArgList { args: vec![ArgSrc::Typed(recv, recv_ty)], using: false, span }];
            let blocked = std::mem::replace(&mut self.no_receiver_conversion, true);
            let (te, applied) = self.apply_member(instance, inst_ty, names::APPLY, None, lists, span, None);
            self.no_receiver_conversion = blocked;
            // What the instance's using clauses fixed (`Step3[P, S, Unit]` with `P` and `S` from
            // `ev`) is the result, where the shape alone left them open.
            let applied = self.zonk(applied);
            if !self.types.has_vars(applied) && applied != ERROR {
                if !accept(self, applied) {
                    self.rollback(mark);
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
            self.rollback(mark);
            return None;
        }
        if self.inline.checking > 0 && is_def && !shape.tparams.is_empty() {
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
        self.try_conversion((g, via), arg, from, span, &mut accept, true).map(|(te, _)| te)
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
            return Some(Ok((g, te, ty)));
        }
        None
    }

    /// The conversion `conv` was committed with the tree `te`: the import credited is one that
    /// brings it on the object or value it was read on (CheckUnused's prefix test), as for a
    /// given (`Unused::receiver`).
    fn attribute_conversion(&mut self, (g, via): ConvRef, te: TExprId) {
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
            if upper != ANY || lower != NOTHING {
                let var = self.types.param(tp);
                let var = self.types.subst(var, &subst_b);
                let (upper, lower) = (self.types.subst(upper, &subst_b), self.types.subst(lower, &subst_b));
                ok = self.is_sub(var, upper) && self.is_sub(lower, var);
            }
        }
        self.rollback(mark);
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
            self.rollback(mark);
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
        self.rollback(mark);
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

    /// `te` converted to `expected`, when a conversion in scope or in the implicit scopes of
    /// the two types takes it there; dotc's `inferView`.
    /// A view is never applied to `null` or to an expression of type `Nothing`, as under scalac.
    fn is_bottom(&mut self, t: TypeId) -> bool {
        let t = self.deref(t);
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

    pub fn convert_to_var_bound_typed(&mut self, te: TExprId, actual: TypeId, v: TVarId, expected: TypeId, span: Span) -> Option<(TExprId, TypeId)> {
        for u in self.upper_bounds_through_vars(v) {
            let u = self.zonk(u);
            if u == ANY || self.types.has_vars(u) {
                continue;
            }
            let mark = self.attempt();
            if let Some((converted, ty)) = self.convert_to_typed(te, actual, u, span, false) {
                if ty != ERROR && self.is_sub(ty, expected) {
                    self.close(mark);
                    return Some((converted, ty));
                }
            }
            self.retract(mark);
        }
        None
    }

    pub(super) fn convert_to_typed(&mut self, te: TExprId, actual: TypeId, expected: TypeId, span: Span, report: bool) -> Option<(TExprId, TypeId)> {
        if actual == ERROR || expected == ERROR || self.types.contains_error(expected) || self.is_bottom(actual) {
            return None;
        }
        let target = self.deref(expected);
        if matches!(self.types.get(target), Type::Var(_) | Type::AppVar(..)) || target == ANY {
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
                Type::Union(a, b) => {
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
