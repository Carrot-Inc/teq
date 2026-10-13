use super::{Redo, TVarInfo, Worker, Undo, DEP_CO, DEP_CONTRA};
use crate::intern::Name;
use crate::symbols::*;
use crate::types::*;
use crate::tir::{TExpr, TExprId};

const CLASS_OF_DEPTH: u32 = 64;

/// What an intersection's member of one name is, of the members of matching signatures its two
/// parts give (`Worker::merge_infos`, dotty's `mergeSingleDenot`): the first's, the second's, one
/// member of both whose symbol is the first's (`true`) or the second's, of their joint
/// information, or the two apart, an overload (`MultiDenotation`).
#[derive(Clone, Debug)]
pub(super) enum Meet {
    First,
    Second,
    Joint(bool, std::sync::Arc<MethodSig>),
    Apart,
}

/// The information an intersection's member carries where its parts' members make one member
/// (dotty's `JointRefDenotation` of the symbol the meet takes and `infoMeet`'s info): that
/// symbol's signature in its own terms with the results met, the other part's result seen from
/// its owner; `None` where the member is one part's as it stands.
pub type JointInfo = Option<std::sync::Arc<MethodSig>>;

/// A member of an intersection's part as dotty's `SingleDenotation` has it: its symbol, the
/// instance of its owner it is a member of, and its joint information.
pub(super) type Denot = ((SymId, TypeId), JointInfo);

impl<'a> Worker<'a> {
    pub fn fresh_var(&mut self) -> TypeId {
        let index = self.tvars.len();
        let v = self.tvars.id(index);
        let place = match self.var_frames.last() {
            Some(&(start, nested)) => index as u32 - start - nested,
            None => index as u32,
        };
        self.tvars.push(TVarInfo::open(self.app_base, (place + 1).min((1 << TVarId::TAG_SHIFT) - 1)));
        self.types.mk(Type::Var(v))
    }

    pub fn snapshot(&self) -> usize {
        self.trail.len()
    }

    pub fn rollback(&mut self, mark: usize) {
        if self.profile.on && self.inline.depth > 0 && self.trail.len() > mark {
            self.profile.inline.rollbacks += 1;
            self.profile.inline.rollback_entries += (self.trail.len() - mark) as u64;
        }
        while self.trail.len() > mark {
            match self.trail.pop().unwrap() {
                Undo::Inst(v) => self.tvars[v].inst = None,
                Undo::Lower(v) => {
                    self.tvars[v].lower.pop();
                }
                Undo::Upper(v) => {
                    self.tvars[v].upper.pop();
                }
                Undo::Dep(v) => {
                    self.tvars[v].pop_dep();
                }
                Undo::DropDeps(v) => {
                    let deps = self.dropped_deps.pop().expect("the dropped dependencies");
                    self.tvars[v].restore_deps(deps);
                }
            }
        }
    }

    pub fn replay(&mut self, undone: Vec<Redo>) {
        for redo in undone.into_iter().rev() {
            match redo {
                // The entries its instantiation added are replayed as they were (`Redo::Dep`).
                Redo::Inst(v, t) => {
                    self.tvars[v].inst = Some(t);
                    self.trail.push(Undo::Inst(v));
                }
                Redo::Lower(v, t) => {
                    self.tvars[v].lower.push(t);
                    self.trail.push(Undo::Lower(v));
                }
                Redo::Upper(v, t) => {
                    self.tvars[v].upper.push(t);
                    self.trail.push(Undo::Upper(v));
                }
                Redo::Dep(v, d) => {
                    self.tvars[v].push_dep(d);
                    self.trail.push(Undo::Dep(v));
                }
                Redo::DropDeps(v) => self.drop_deps(v),
            }
        }
    }

    pub(super) fn instantiate(&mut self, v: TVarId, t: TypeId) {
        self.tvars[v].inst = Some(t);
        self.trail.push(Undo::Inst(v));
        self.note_instance_deps(v, t);
        self.drop_deps(v);
    }

    /// `v` instantiated, its dependents moved to its instance's variables (`note_instance_deps`):
    /// its own are dropped, as dotty's `replace` drops an instantiated parameter's (`dropDeps`),
    /// and an undo gives them back (`Undo::DropDeps`).
    #[inline(always)]
    fn drop_deps(&mut self, v: TVarId) {
        if !self.tvars[v].deps().is_empty() {
            self.drop_deps_slow(v);
        }
    }

    #[cold]
    #[inline(never)]
    fn drop_deps_slow(&mut self, v: TVarId) {
        let deps = self.tvars[v].take_deps();
        self.dropped_deps.push(deps);
        self.trail.push(Undo::DropDeps(v));
    }

    /// The bound `b` just recorded for `by` (a lower one at `sign` 1, an upper one at -1): each
    /// open variable it names keeps `by` among the variables whose bounds name it, at the
    /// variance it is named at (dotty's `OrderingConstraint.adjustDeps`, its `Adjuster`). A bound
    /// that is a variable alone is an ordering, which both variables' bounds keep (the lenses).
    #[inline]
    fn note_bound_deps(&mut self, by: TVarId, b: TypeId, sign: i8) {
        if self.types.has_vars(b) {
            self.note_bound_deps_slow(by, b, sign);
        }
    }

    #[inline(never)]
    fn note_bound_deps_slow(&mut self, by: TVarId, b: TypeId, sign: i8) {
        let b = self.deref(b);
        if matches!(self.types.get(b), Type::Var(_)) {
            return;
        }
        let mut named = Vec::new();
        self.collect_var_variances(b, sign, &mut named);
        for (x, s) in named {
            if x != by {
                self.add_dep(x, by, dep_flags(s));
            }
        }
    }

    fn add_dep(&mut self, x: TVarId, by: TVarId, flags: u8) {
        self.tvars[x].push_dep((by, flags));
        self.trail.push(Undo::Dep(x));
    }

    /// `v` instantiated to `t`: whatever `v`'s instance could narrow, `t`'s open variables can
    /// now, as dotty's `replace` moves a parameter's dependents to its instance's
    /// (`adjustDeps`): each keeps them at the composed variances (`v <: w` makes `t` a lower bound
    /// of `w`, an entry `(w, DEP_CO)` of `v`'s).
    #[inline]
    fn note_instance_deps(&mut self, v: TVarId, t: TypeId) {
        if !self.tvars[v].deps().is_empty() && self.types.has_vars(t) {
            self.note_instance_deps_slow(v, t);
        }
    }

    #[inline(never)]
    fn note_instance_deps_slow(&mut self, v: TVarId, t: TypeId) {
        let dependents: Vec<(TVarId, u8)> = self.tvars[v].deps().iter().copied().filter(|&(by, _)| self.tvars[by].inst.is_none()).collect();
        if dependents.is_empty() {
            return;
        }
        let mut named = Vec::new();
        self.collect_var_variances(t, 1, &mut named);
        for (y, s) in named {
            for &(by, f) in &dependents {
                let g = compose_dep(f, s);
                if y != by && g != 0 {
                    self.add_dep(y, by, g);
                }
            }
        }
    }

    /// dotty's `OrderingConstraint.dependsOn(v, except, co)`: whether an open variable outside
    /// `excluded` names `v` in its bounds where instantiating `v` larger (`co`) or smaller
    /// narrows it (`coDeps`, `contraDeps`), or is ordered above (`co`) or below `v` (the upper
    /// and lower lenses), all of which `deps` keeps.
    pub(super) fn depends_on(&self, v: TVarId, excluded: &[TVarId], co: bool) -> bool {
        let want = if co { DEP_CO } else { DEP_CONTRA };
        self.tvars[v].deps().iter().any(|&(by, f)| f & want != 0 && by != v && self.tvars[by].inst.is_none() && !excluded.contains(&by))
    }

    /// Whether `v`'s instance can narrow an open variable but itself (`deps`).
    #[inline]
    fn has_open_dep(&self, v: TVarId) -> bool {
        let deps = self.tvars[v].deps();
        !deps.is_empty() && deps.iter().any(|&(by, _)| by != v && self.tvars[by].inst.is_none())
    }

    /// Follows instantiated inference variables at the head of the type.
    #[inline]
    pub fn deref(&mut self, t: TypeId) -> TypeId {
        match self.types.get(t) {
            Type::Var(_) | Type::AppVar(..) => self.deref_var(t),
            _ => t,
        }
    }

    /// Follows instantiated inference variables and expands an alias kept by name at the
    /// head of the type, for a use that reads the type's shape.
    #[inline]
    pub fn deref_alias(&mut self, t: TypeId) -> TypeId {
        match self.types.get(t) {
            Type::Var(_) | Type::AppVar(..) | Type::Alias(..) => self.deref_alias_slow(t),
            _ => t,
        }
    }

    #[inline(never)]
    fn deref_alias_slow(&mut self, t: TypeId) -> TypeId {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Alias(a, args) if !self.is_match_alias(a) && !self.is_curried_alias(a) => match self.alias_expansion(a, args) {
                Some(e) if e != t => self.deref_alias(e),
                _ => t,
            },
            _ => t,
        }
    }

    #[inline(never)]
    fn deref_var(&mut self, t: TypeId) -> TypeId {
        let mut t = t;
        loop {
            match self.types.get(t) {
                Type::Var(v) => match self.tvars[v].inst {
                    Some(i) => t = i,
                    None => return t,
                },
                Type::AppVar(v, args) => match self.tvars[v].inst {
                    Some(ctor) => {
                        let items: Vec<TypeId> = self.types.items(args).to_vec();
                        let ctor = self.deref(ctor);
                        t = self.types.apply_ctor(ctor, &items);
                    }
                    None => return t,
                },
                _ => return t,
            }
        }
    }

    /// Replaces all instantiated inference variables inside the type.
    pub fn zonk(&mut self, t: TypeId) -> TypeId {
        if !self.types.has_vars(t) {
            return t;
        }
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Class(c, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let z: Vec<TypeId> = items.iter().map(|&a| self.zonk(a)).collect();
                self.types.class(c, &z)
            }
            Type::AppParam(p, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let z: Vec<TypeId> = items.iter().map(|&a| self.zonk(a)).collect();
                let l = self.types.list(&z);
                self.types.mk(Type::AppParam(p, l))
            }
            Type::AppVar(v, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let z: Vec<TypeId> = items.iter().map(|&a| self.zonk(a)).collect();
                let l = self.types.list(&z);
                self.types.mk(Type::AppVar(v, l))
            }
            Type::Lambda(ps, b) => {
                let zb = self.zonk(b);
                self.types.mk(Type::Lambda(ps, zb))
            }
            Type::Union(a, b, soft) => {
                let (za, zb) = (self.zonk(a), self.zonk(b));
                self.types.union_as(soft, za, zb)
            }
            Type::Inter(a, b) => {
                let (za, zb) = (self.zonk(a), self.zonk(b));
                self.types.inter(za, zb)
            }
            Type::Select(p, s) => {
                let zp = self.zonk(p);
                self.types.mk(Type::Select(zp, s))
            }
            Type::Member(p, n) => {
                let zp = self.zonk(p);
                self.types.mk(Type::Member(zp, n))
            }
            Type::AppMember(m, args) => {
                let zm = self.zonk(m);
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let z: Vec<TypeId> = items.iter().map(|&a| self.zonk(a)).collect();
                self.types.apply_ctor(zm, &z)
            }
            Type::Refined(p, r) => {
                let zp = self.zonk(p);
                let zr = match self.types.refinement(r) {
                    Refinement::Alias(n, rhs) => Refinement::Alias(n, self.zonk(rhs)),
                    Refinement::Bounds(n, lo, hi) => Refinement::Bounds(n, self.zonk(lo), self.zonk(hi)),
                    Refinement::Val(n, s, ty) => Refinement::Val(n, s, self.zonk(ty)),
                    Refinement::Term(n, s, l) => {
                        let items: Vec<TypeId> = self.types.items(l).to_vec();
                        let z: Vec<TypeId> = items.into_iter().map(|a| self.zonk(a)).collect();
                        Refinement::Term(n, s, self.types.list(&z))
                    }
                };
                let zr = self.types.refine(zr);
                self.types.mk(Type::Refined(zp, zr))
            }
            Type::Alias(a, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let z: Vec<TypeId> = items.iter().map(|&x| self.zonk(x)).collect();
                let l = self.types.list(&z);
                self.types.mk(Type::Alias(a, l))
            }
            Type::Match(s, m) => {
                let zs = self.zonk(s);
                let (cases, bound) = {
                    let info = self.types.match_info(m);
                    (info.cases.to_vec(), info.bound)
                };
                let zb = self.zonk(bound);
                let zcases: Vec<MatchCase> = cases
                    .iter()
                    .map(|c| MatchCase { binders: c.binders, pattern: self.zonk(c.pattern), body: self.zonk(c.body) })
                    .collect();
                self.types.match_type(zs, &zcases, zb)
            }
            _ => t,
        }
    }

    pub(super) fn is_unresolved_member(&self, t: TypeId) -> bool {
        matches!(self.types.get(t), Type::Blocked(b) if self.types.blocked_description(b).starts_with(super::members::UNRESOLVED_MEMBER))
    }

    /// Whether the two sides are a wildcard and a capture, or two captures, which a library
    /// body (checked by scalac) may hold for one unknown argument.
    fn wildcards_agree(&self, a: TypeId, b: TypeId) -> bool {
        match (self.types.get(a), self.types.get(b)) {
            (Type::Wild | Type::BoundedWild(..), Type::Param(_)) => self.is_wildcard_capture(b),
            (Type::Param(_), Type::Wild | Type::BoundedWild(..)) => self.is_wildcard_capture(a),
            (Type::Param(_), Type::Param(_)) => self.is_wildcard_capture(a) && self.is_wildcard_capture(b),
            _ => false,
        }
    }

    /// A capture of a wildcard: the compiler's own, or a type variable scalac bound for an
    /// anonymous binder of a type pattern (`case v: Value[_$5]`, whose name it made up).
    fn is_wildcard_capture(&self, t: TypeId) -> bool {
        matches!(self.types.get(t), Type::Param(p) if self.syms.tparam(p).name == crate::names::WILDCARD || self.interner.get(self.syms.tparam(p).name).starts_with("_$"))
    }

    pub fn is_same(&mut self, a: TypeId, b: TypeId) -> bool {
        self.is_sub(a, b) && self.is_sub(b, a)
    }

    /// Sees through opaque types whose definition is visible from the current scope, through
    /// type parameters that a pattern of an enclosing match case has fixed, and from a literal
    /// type to its class, whose members and operators the literal has.
    pub fn dealias(&mut self, t: TypeId) -> TypeId {
        let mut t = self.deref(t);
        // A path whose underlying type is a path again is followed a bounded number of times.
        for _ in 0..64 {
            let next = match self.types.get(t) {
                Type::Class(c, args) => self.opaque_underlying(t).or_else(|| {
                    (args != EMPTY_LIST && self.types.is_op_class(c)).then(|| self.reduce_head(t)).flatten()
                }),
                Type::Param(_) => self.gadt_known(t),
                Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => {
                    self.path_seen_through(t)
                }
                Type::Match(..) | Type::Alias(..) => self.reduced_seen_through(t),
                _ => None,
            };
            let Some(u) = next else { break };
            let u = self.deref(u);
            if u == t {
                break;
            }
            t = u;
        }
        let t = self.widen_lit(t);
        if t == WILD {
            return ANY;
        }
        if let Type::BoundedWild(_, hi) = self.types.get(t) {
            return self.dealias(hi);
        }
        t
    }

    /// What a value of a path type or an abstract member type is used as: the path's type,
    /// the member's upper bound.
    fn path_seen_through(&mut self, t: TypeId) -> Option<TypeId> {
        match self.types.get(t) {
            Type::This(_) | Type::Term(_) | Type::Select(..) => Some(self.path_underlying(t)),
            Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => {
                let upper = self.member_upper(t);
                (upper != t).then_some(upper)
            }
            _ => None,
        }
    }

    /// What a value of a match type is used as: its reduction, or the bound of a stuck one;
    /// a type-level operation or a tuple cons as what it works out to.
    fn reduced_seen_through(&mut self, t: TypeId) -> Option<TypeId> {
        if !self.types.head_reducible(self.types.get(t)) {
            return None;
        }
        if let Some(r) = self.reduce_head(t) {
            return Some(r);
        }
        match self.types.get(t) {
            Type::Match(..) | Type::Alias(..) => {
                let bound = self.match_bound(t);
                (bound != t).then_some(bound)
            }
            _ => None,
        }
    }

    fn gadt_known(&self, t: TypeId) -> Option<TypeId> {
        if self.gadt.is_empty() {
            return None;
        }
        let Type::Param(p) = self.types.get(t) else { return None };
        self.gadt.iter().rev().find(|&&(q, _, variance)| q == p && variance <= 0).map(|&(_, known, _)| known)
    }

    /// The type an opaque type erases to, its underlying type seen from anywhere.
    pub(super) fn opaque_erasure(&mut self, t: TypeId) -> Option<TypeId> {
        let Type::Class(c, args) = self.types.get(t) else { return None };
        if self.syms.class(c).kind != ClassKind::Opaque {
            return None;
        }
        self.complete_class(c);
        let under = self.syms.class(c).underlying?;
        let subst: Subst = self.syms.class(c).tparams.iter().copied().zip(self.types.items(args).iter().copied()).collect();
        Some(self.types.subst(under, &subst))
    }

    /// The base types an opaque type has outside its scope through its bound `bound` that is no
    /// class (`opaque type O <: A & B`), which dotty reads as an abstract type's upper bound
    /// (`superType`): each class of the bound's base classes in dotty's order
    /// (`receiver_base_classes`) with its base type, as a class bound's base types are copied
    /// (`set_opaque_bound`).
    pub(super) fn opaque_bound_bases(&mut self, bound: TypeId) -> Vec<(ClassId, TypeId)> {
        let classes = self.receiver_base_classes(bound, 0);
        classes.into_iter().filter_map(|k| Some((k, self.base_type(bound, k)?))).collect()
    }

    pub(crate) fn opaque_underlying(&mut self, t: TypeId) -> Option<TypeId> {
        if let Type::Class(c, args) = self.types.get(t) {
            if self.syms.class(c).kind == ClassKind::Opaque && self.transparent.contains(&c) {
                self.complete_class(c);
                let under = self.syms.class(c).underlying?;
                let subst: Subst = self
                    .syms
                    .class(c)
                    .tparams
                    .iter()
                    .copied()
                    .zip(self.types.items(args).iter().copied())
                    .collect();
                return Some(self.types.subst(under, &subst));
            }
        }
        None
    }

    fn add_upper(&mut self, v: TVarId, b: TypeId) -> bool {
        if self.tvars[v].upper.contains(&b) {
            return true;
        }
        if let Type::Var(w) = self.types.get(b) {
            if w == v {
                return true;
            }
            // `v <: w` between two open variables is kept as a pair of bounds, and in either's
            // dependencies (the lenses): `v` growing narrows `w`, `w` shrinking narrows `v`.
            let v_ty = self.types.mk(Type::Var(v));
            if !self.tvars[w].lower.contains(&v_ty) {
                self.tvars[w].lower.push(v_ty);
                self.trail.push(Undo::Lower(w));
            }
            self.add_dep(v, w, DEP_CO);
            self.add_dep(w, v, DEP_CONTRA);
        }
        // Recorded before the lower bounds are checked so that cyclic constraints terminate.
        self.tvars[v].upper.push(b);
        self.trail.push(Undo::Upper(v));
        self.note_bound_deps(v, b, -1);
        // The lower bounds as they stand, by index (dotty's `addOneBound` compares the entry it
        // narrowed, copying no list); a comparison only records bounds after them.
        let n = self.tvars[v].lower.len();
        (0..n).all(|k| self.tvars[v].lower.get(k).copied().map_or(true, |l| self.is_sub(l, b)))
    }

    fn add_lower(&mut self, v: TVarId, a: TypeId) -> bool {
        if let Type::Var(w) = self.types.get(a) {
            let v_ty = self.types.mk(Type::Var(v));
            return self.add_upper(w, v_ty);
        }
        if self.tvars[v].lower.contains(&a) {
            return true;
        }
        self.tvars[v].lower.push(a);
        self.trail.push(Undo::Lower(v));
        self.note_bound_deps(v, a, 1);
        // The upper bounds as they stand, by index, as `add_upper` reads the lower ones.
        let n = self.tvars[v].upper.len();
        (0..n).all(|k| self.tvars[v].upper.get(k).copied().map_or(true, |u| self.is_sub(a, u)))
    }

    /// The upper bounds of `v` that are no open variables, through the variables among them:
    /// `A` of `Some(x)` for `orElse[B >: A]`'s argument against an `Option[CashFee]` is below
    /// `B`, which is below `CashFee`.
    pub fn upper_bounds_through_vars(&mut self, v: TVarId) -> Vec<TypeId> {
        let mut seen = vec![v];
        let mut out = Vec::new();
        let mut i = 0;
        while i < seen.len() {
            let w = seen[i];
            i += 1;
            for j in 0..self.tvars[w].upper.len() {
                let u = self.tvars[w].upper[j];
                let u = self.deref(u);
                match self.types.get(u) {
                    Type::Var(x) if !seen.contains(&x) => seen.push(x),
                    Type::Var(_) => {}
                    _ => out.push(u),
                }
            }
        }
        out
    }

    /// `a <: b`, keeping the constraints only when it holds.
    pub fn conforms_or_rollback(&mut self, a: TypeId, b: TypeId) -> bool {
        let mark = self.snapshot();
        let ok = self.is_sub(a, b);
        if !ok {
            self.rollback(mark);
        }
        ok
    }

    pub fn is_sub(&mut self, a: TypeId, b: TypeId) -> bool {
        let a = self.deref(a);
        let b = self.deref(b);
        // An unknown type argument is read as `Any` and written as `Nothing`; two of them
        // need not be the same type. A member a path cannot resolve conforms to nothing, not
        // even to another reference to it.
        if a == b {
            return a != WILD && !self.is_unresolved_member(a);
        }
        // A constant against its own class, the commonest comparison a literal meets.
        if let Type::Lit(_) = self.types.get(a) {
            if self.widen_lit(a) == b {
                return true;
            }
        }
        // A library body's `C[?]#T` (an unknown argument) against the capture of the same
        // receiver's wildcard, which scalac gave one name: the body was checked by scalac.
        if self.loaded.is_some() && self.wildcards_agree(a, b) && self.is_body_file(self.env.file) {
            return true;
        }
        if a == WILD {
            return self.is_sub(ANY, b);
        }
        if b == WILD {
            return self.is_sub(a, NOTHING);
        }
        if a == self.b.t_null && !matches!(self.types.get(b), Type::Var(_) | Type::Error | Type::BoundedWild(..)) {
            return self.null_conforms(b);
        }
        if b == self.b.t_any_ref && !matches!(self.types.get(a), Type::Var(_) | Type::Error | Type::BoundedWild(..)) {
            return self.is_reference(a);
        }
        if b == self.b.t_any_val && !matches!(self.types.get(a), Type::Var(_) | Type::Error | Type::BoundedWild(..)) {
            return self.is_value(a);
        }
        if b == self.b.t_singleton && b != ERROR && !matches!(self.types.get(a), Type::Var(_) | Type::Error | Type::BoundedWild(..)) {
            return self.is_singleton_type(a);
        }
        if (b == self.b.t_product || b == self.b.t_equals) && !matches!(self.types.get(a), Type::Var(_) | Type::Error | Type::BoundedWild(..)) && self.is_product(a) {
            return true;
        }
        let (ta, tb) = (self.types.get(a), self.types.get(b));
        if let Type::BoundedWild(_, hi) = ta {
            return self.is_sub(hi, b);
        }
        if let Type::BoundedWild(lo, _) = tb {
            return self.is_sub(a, lo);
        }
        // `F[T]` against an alias applied beyond its parameters (`(Eq |: Derived)[A]`) takes
        // the alias application as `F` and the rest as `T`, before the alias reduces, as
        // scalac unifies with the unreduced application.
        if let (Type::AppVar(v, xs), Type::Alias(al, ys)) | (Type::Alias(al, ys), Type::AppVar(v, xs)) = (ta, tb) {
            let own = self.syms.aliases[al.idx()].tparams.len();
            let (xs, ys) = (self.types.items(xs).to_vec(), self.types.items(ys).to_vec());
            if ys.len() == own + xs.len() && !xs.is_empty() && self.tvars[v].inst.is_none() {
                let head = self.types.list(&ys[..own]);
                let ctor = self.types.mk(Type::Alias(al, head));
                self.instantiate(v, ctor);
                return self.is_sub(a, b);
            }
        }
        if self.types.head_reducible(ta) || self.types.head_reducible(tb) {
            if let Some(holds) = self.is_sub_reducible(a, b) {
                return holds;
            }
        }
        if let Some(holds) = self.named_tuple_sub(a, b) {
            return holds;
        }
        if let Some(holds) = self.local_object_sub(a, b) {
            return holds;
        }
        match (ta, tb) {
            // A failed argument leaves no variable to solve to `Nothing` and report on again.
            (Type::Error, Type::Var(v)) | (Type::Var(v), Type::Error) => {
                self.instantiate(v, ERROR);
                return true;
            }
            (Type::Error, _) | (_, Type::Error) => return true,
            // A library type of an unsupported shape is known to nothing.
            (Type::Blocked(_), _) | (_, Type::Blocked(_)) => return false,
            (Type::Var(v), _) => return self.add_upper(v, b),
            (_, Type::Var(v)) => return self.add_lower(v, a),
            (_, Type::Any) | (Type::Nothing, _) => return true,
            (Type::Union(x, y, _), _) => return self.is_sub(x, b) && self.is_sub(y, b),
            (_, Type::Inter(x, y)) => return self.is_sub(a, x) && self.is_sub(a, y),
            // A recursive refinement on the right is its body with the left side's anchor as its
            // self, a path or a skolem of the left side (`TypeComparer.compareRec`:
            // `ensureStableSingleton`), the left side's own recursive types opened at the same
            // anchor (`fixRecs`); on the left, its body seen through a skolem of it (its
            // `RecThis`, a singleton of the type).
            (_, Type::Rec(body)) => {
                let anchor = self.rec_anchor(a);
                let left = self.fix_recs(anchor, anchor);
                let right = self.opened_rec(body, anchor);
                return self.is_sub(left, right);
            }
            (Type::Rec(body), _) => {
                let anchor = self.rec_anchor(a);
                let opened = self.opened_rec(body, anchor);
                return self.is_sub(opened, b);
            }
            _ => {}
        }
        if let Type::Union(..) = tb {
            // The members without variables take what conforms to them first, so that `Other`
            // conforms to `?T | Other` without becoming a bound of `?T`, and `X | Other` binds
            // `?T` to `X` alone.
            let mark = self.snapshot();
            if self.is_sub_union_member(a, b, false, mark)
                || (self.types.has_vars(b) && self.is_sub_union_member(a, b, true, mark))
            {
                return true;
            }
        }
        if let Type::Inter(x, y) = ta {
            // `b` one of the parts: no comparison (`A & B & C <: B`), but for an unknown argument
            // or an unresolved member, which nothing conforms to (the `a == b` rule above).
            if b != WILD && self.is_part_of_inter(b, a) && !self.is_unresolved_member(b) {
                return true;
            }
            let mark = self.snapshot();
            // dotty's `secondTry` sends an `AndType` on the left to `thirdTry`, whose
            // `compareRefined` takes it whole (its member a refinement of one part names the
            // other part's member through it); the parts are tried after (`fourthTry`).
            if let Type::Refined(parent, r) = tb {
                if self.is_sub(a, parent) && self.refinement_holds(a, parent, r) {
                    return true;
                }
                self.rollback(mark);
            }
            if self.necessary_either && self.types.has_vars(x) && self.types.has_vars(y) {
                if let Some(ok) = self.is_sub_inter_necessary(x, y, b, mark) {
                    return ok;
                }
            }
            // The whole intersection conforms to a parameter bounded below by it (a wildcard's
            // capture, whose lower bound is the `A & B` of a join), where neither part does.
            if let Type::Param(p) = tb {
                let lower = self.syms.tparam(p).lower;
                if lower != NOTHING && self.is_sub(a, lower) {
                    return true;
                }
                self.rollback(mark);
            }
            if self.is_sub(x, b) {
                return true;
            }
            self.rollback(mark);
            if self.is_sub(y, b) {
                return true;
            }
            self.rollback(mark);
            return self.is_sub_inter_laws(x, y, b, mark);
        }
        if let Some(u) = self.opaque_underlying(a) {
            return self.is_sub(u, b);
        }
        if let Some(u) = self.opaque_underlying(b) {
            return self.is_sub(a, u);
        }
        // A constant against a path whose underlying type is a constant (`C.x.type` of a
        // `final val x = 1`): the same atom, compared with that constant before the constant
        // widens to its class (dotty's `comparePaths`, `compareAtoms` with known singletons).
        if matches!(ta, Type::Lit(_)) && matches!(tb, Type::Term(_) | Type::Select(..)) {
            let under = self.widen_path(b);
            if matches!(self.types.get(under), Type::Lit(_)) {
                return self.is_sub(a, under);
            }
        }
        if let Type::Lit(_) = ta {
            let class = self.widen_lit(a);
            return self.is_sub(class, b);
        }
        let dependent = |t: Type| {
            matches!(t, Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..) | Type::Rec(_))
        };
        // A lambda against an abstract type constructor (`[A] =>> Impl.Type[A]` for `Impl.Type`)
        // reduces before the member rules see it.
        if matches!(ta, Type::Lambda(..)) || matches!(tb, Type::Lambda(..)) {
            return self.is_sub_lambda(a, b);
        }
        if matches!(ta, Type::Poly(..)) || matches!(tb, Type::Poly(..)) {
            return self.is_sub_poly(a, b);
        }
        if dependent(ta) || dependent(tb) {
            // Two paths to one term: the reflect API's `typeRef` of a val has an id of its own.
            if let (Type::Term(x), Type::Term(y)) = (ta, tb) {
                if x == y {
                    return true;
                }
            }
            return self.is_sub_dependent(a, b);
        }
        match (ta, tb) {
            (Type::AppVar(v, args), Type::Class(c, cargs))
            | (Type::Class(c, cargs), Type::AppVar(v, args)) => {
                let n = self.types.items(args).len();
                let m = self.types.items(cargs).len();
                if n > m {
                    // A class with fewer arguments than the application conforms through a
                    // base type that has enough: `None.type` as `F[A]` is `Option[Nothing]`.
                    return matches!(ta, Type::Class(..)) && self.is_sub_through_base(a, c, cargs, b, n);
                }
                // A constructor below the variable already (the expected type's `Option` for
                // `Kleisli[Option, Int, String]`) is the instantiation, not the argument's own
                // class (`Some`), where the relation holds through it. An application of the
                // variable below a class (`G[A] <: Dec[A]` of a result against the expected type)
                // leaves room above the bound for the arguments still to come.
                let var_below = matches!(ta, Type::AppVar(..));
                let bounds: Vec<TypeId> = if var_below { Vec::new() } else { self.tvars[v].lower.clone() };
                for bound in bounds {
                    let bound = self.deref(bound);
                    if !matches!(self.types.get(bound), Type::Ctor(_) | Type::Lambda(..)) {
                        continue;
                    }
                    let mark = self.snapshot();
                    self.instantiate(v, bound);
                    if self.is_sub(a, b) {
                        return true;
                    }
                    self.rollback(mark);
                }
                let mark = self.snapshot();
                if self.app_var_instance(a, b, v, c, cargs, n, var_below) {
                    return true;
                }
                self.rollback(mark);
                // Else the variable is a constructor of a base type of the class, the first in
                // its linearisation that fits, as dotty's `canInstantiate` lifts the argument to
                // its base classes (`liftToBase`): a `Map[K, V]` as `M[A]` with `M[A] <:
                // IterableOnce[A]` is an `Iterable[(K, V)]`.
                if var_below {
                    return false;
                }
                let bases: Vec<TypeId> = {
                    self.complete_class(c);
                    let subst: Subst = self.syms.class(c).tparams.iter().copied().zip(self.types.items(cargs).iter().copied()).collect();
                    self.syms.class(c).base_types.iter().skip(1).map(|&(_, bt)| bt).collect::<Vec<_>>().into_iter().map(|bt| self.types.subst(bt, &subst)).collect()
                };
                for base in bases {
                    let Type::Class(bc, bargs) = self.types.get(base) else { continue };
                    if self.types.items(bargs).len() < n {
                        continue;
                    }
                    let mark = self.snapshot();
                    if self.app_var_instance(base, b, v, bc, bargs, n, false) {
                        return true;
                    }
                    self.rollback(mark);
                }
                false
            }
            (Type::AppVar(v, _), Type::AppParam(p, _)) | (Type::AppParam(p, _), Type::AppVar(v, _)) => {
                let ctor = self.types.param(p);
                self.instantiate(v, ctor);
                self.is_sub(a, b)
            }
            (Type::AppVar(v, xs), Type::AppVar(w, ys)) if v == w => self.same_args(xs, ys),
            (Type::AppParam(p, xs), Type::AppParam(q, ys)) if p == q => {
                let variances = self.syms.tparam(p).hk_variances.clone();
                if variances.is_empty() {
                    return self.same_args(xs, ys);
                }
                let xs: Vec<TypeId> = self.types.items(xs).to_vec();
                let ys: Vec<TypeId> = self.types.items(ys).to_vec();
                xs.len() == ys.len()
                    && xs.iter().zip(&ys).enumerate().all(|(i, (&x, &y))| match variances.get(i).copied().unwrap_or(0) {
                        1 => self.is_sub(x, y),
                        -1 => self.is_sub(y, x),
                        _ => self.is_same(x, y),
                    })
            }
            (_, Type::Nested(p2, class2)) if !matches!(ta, Type::AppParam(..) | Type::AppVar(..)) => self.is_sub_nested(a, b, p2, class2),
            // An unapplied class below a class that takes no parameters is compared by its base
            // type there, as dotty's `TypeComparer.thirdTryNamed` (`tryBaseType`) compares a
            // class constructor that is no lambda: the class over its own parameters, a product
            // by rule as well (`Tuple2 <: Product`).
            (Type::Ctor(c1), Type::Class(c2, args)) if self.types.items(args).is_empty() && self.syms.class(c2).tparams.is_empty() => {
                self.complete_class(c1);
                let own: Vec<TypeId> = self.syms.class(c1).tparams.clone().into_iter().map(|p| self.types.param(p)).collect();
                let applied = self.types.class(c1, &own);
                self.is_sub(applied, b)
            }
            (_, Type::Class(c2, args2)) if !matches!(ta, Type::AppParam(..) | Type::AppVar(..)) => {
                // A parameter bounded by an intersection conforms through either side, which one
                // base type of the bound cannot say: `C <: LinearSeq[A] & LinearSeqOps[A, CC, C]`
                // is a `LinearSeqOps[A, CC, C]` by its second side alone.
                if let Type::Param(p) = ta {
                    let upper = self.syms.tparam(p).upper;
                    if matches!(self.types.get(upper), Type::Inter(..)) {
                        return self.is_sub(upper, b) || self.is_sub_by_gadt(p, b, -1);
                    }
                }
                let Some(base) = self.base_type(a, c2) else {
                    return matches!(ta, Type::Param(p) if self.is_sub_by_gadt(p, b, -1));
                };
                let args1 = match self.types.get(base) {
                    Type::Class(_, args1) => args1,
                    // `p.C` against the bare `C`, its type through its owner's `this`.
                    Type::Nested(p1, class1) => {
                        let (Type::Class(_, args1), Owner::Class(o)) = (self.types.get(class1), self.syms.class(c2).owner) else { return false };
                        let this = self.types.mk(Type::This(o));
                        if !self.is_sub_prefix(p1, this) {
                            return false;
                        }
                        args1
                    }
                    _ => return false,
                };
                let xs: Vec<TypeId> = self.types.items(args1).to_vec();
                let ys: Vec<TypeId> = self.types.items(args2).to_vec();
                let variances: Vec<i8> = self
                    .syms
                    .class(c2)
                    .tparams
                    .iter()
                    .map(|&p| self.syms.tparam(p).variance)
                    .collect();
                let bounded = self.types.has_wild(b);
                for i in 0..xs.len().min(ys.len()) {
                    if ys[i] == WILD {
                        continue;
                    }
                    if let (true, Type::BoundedWild(lo, hi)) = (bounded, self.types.get(ys[i])) {
                        let (xlo, xhi) = match self.types.wild_bounds(xs[i]) {
                            // A wildcard argument is bounded by its parameter's declared bounds too
                            // (`TypeComparer.isSubArg`: `tp1 & paramBounds(tparam)`).
                            Some((xlo, xhi)) => self.within_param_bounds(c2, i, xlo, xhi, &ys),
                            None => (xs[i], xs[i]),
                        };
                        if !(self.is_sub(lo, xlo) && self.is_sub(xhi, hi)) {
                            return false;
                        }
                        continue;
                    }
                    let ok = match variances.get(i).copied().unwrap_or(0) {
                        1 => self.is_sub(xs[i], ys[i]),
                        -1 => self.is_sub(ys[i], xs[i]),
                        _ => self.is_same(xs[i], ys[i]),
                    };
                    if !ok {
                        return false;
                    }
                }
                true
            }
            (Type::Param(p), _) => {
                let upper = self.syms.tparam(p).upper;
                if upper != ANY && self.is_sub(upper, b) {
                    return true;
                }
                let below_lower = match tb {
                    Type::Param(q) => {
                        let lower = self.syms.tparam(q).lower;
                        (lower != NOTHING && self.is_sub(a, lower)) || self.is_sub_by_gadt(q, a, 1)
                    }
                    _ => false,
                };
                below_lower || self.is_sub_by_gadt(p, b, -1)
            }
            (Type::AppParam(p, args), _) => {
                let mark = self.snapshot();
                if let Some(upper) = self.app_param_upper(p, args) {
                    if self.is_sub(upper, b) {
                        return true;
                    }
                    self.rollback(mark);
                }
                match tb {
                    Type::AppParam(q, bargs) => match self.app_param_lower(q, bargs) {
                        Some(lower) => self.is_sub(a, lower),
                        None => false,
                    },
                    _ => false,
                }
            }
            (_, Type::AppParam(q, args)) => match self.app_param_lower(q, args) {
                Some(lower) => self.is_sub(a, lower),
                None => false,
            },
            (_, Type::Param(p)) => {
                let lower = self.syms.tparam(p).lower;
                (lower != NOTHING && self.is_sub(a, lower)) || self.is_sub_by_gadt(p, a, 1)
            }
            (Type::Ctor(c1), Type::Ctor(c2)) => c1 == c2 || self.ctor_extends(c1, c2),
            _ => false,
        }
    }

    /// `a <: p.C[Ts]`, dotty's `compareNamed` of a class reference: `a`'s base type of `C` is
    /// the class through a prefix that conforms (`is_sub_prefix`), with conforming arguments; a
    /// bare `C` is the class through its owner's `this`.
    fn is_sub_nested(&mut self, a: TypeId, b: TypeId, p2: TypeId, class2: TypeId) -> bool {
        let Type::Class(c2, _) = self.types.get(class2) else { return false };
        let Some(base) = self.base_type(a, c2) else {
            return matches!(self.types.get(a), Type::Param(p) if self.is_sub_by_gadt(p, b, -1));
        };
        let (p1, class1) = match self.types.get(base) {
            Type::Nested(p1, class1) => (p1, class1),
            Type::Class(..) => {
                let Owner::Class(o) = self.syms.class(c2).owner else { return false };
                (self.types.mk(Type::This(o)), base)
            }
            _ => return false,
        };
        self.is_sub_prefix(p1, p2) && self.is_sub(class1, class2)
    }

    /// Whether the prefix `p1` of a class reference conforms to `p2`, dotty's `isSubPrefix`: the
    /// `this` of two classes whose self types derive from each other is one prefix, otherwise
    /// the prefixes conform as types (`a.C <: O#C` for an `a: O`, `b.C` is `a.C` for a `b: a.type`).
    fn is_sub_prefix(&mut self, p1: TypeId, p2: TypeId) -> bool {
        if p1 == p2 {
            return true;
        }
        if let (Type::This(c1), Type::This(c2)) = (self.types.get(p1), self.types.get(p2)) {
            let derives = |w: &mut Self, x: ClassId, y: ClassId| w.syms.class(x).base_types.iter().any(|&(b, _)| b == y) || w.self_type_derives(x, y);
            if derives(self, c1, c2) && derives(self, c2, c1) {
                return true;
            }
        }
        let mark = self.snapshot();
        let holds = self.is_sub(p1, p2);
        if !holds {
            self.rollback(mark);
        }
        holds
    }

    /// Conformance where a match type, an alias application, a type-level operation or a tuple
    /// cons heads either side: what reduces is compared reduced; a stuck match type conforms
    /// to its bound and to a match type with the same scrutinee and cases; a tuple conforms
    /// to `h *: t` element by element. None where the ordinary rules decide.
    #[inline(never)]
    fn is_sub_reducible(&mut self, a: TypeId, b: TypeId) -> Option<bool> {
        // Two applications of one alias with the same arguments stand for the same type,
        // whatever it reduces to; compared before reducing, an open argument is pinned to the
        // other's (`(Eq |: Derived)[?T]` against `(Eq |: Derived)[Slot]`).
        // An alias kept by name for the reflect API is compared as what it stands for, as
        // scalac dealiases it: `Lifted[U]` against `Lifted[Int]` for `Err | A` bounds `U`
        // rather than pinning it.
        if let (Type::Alias(a1, xs), Type::Alias(a2, ys)) = (self.types.get(a), self.types.get(b)) {
            if a1 == a2 && !self.alias_by_name(a1) {
                let mark = self.snapshot();
                if self.contained_args(xs, ys) {
                    return Some(true);
                }
                self.rollback(mark);
            }
        }
        let (na, nb) = (self.reduce_head(a).unwrap_or(a), self.reduce_head(b).unwrap_or(b));
        if na != a || nb != b {
            return Some(self.is_sub(na, nb));
        }
        // dotty's `compareS`: `S[N]` of an open `N` against a natural constant compares `N` with
        // the constant's predecessor (`2 <: S[?N]` makes `?N >: 1`), either way round.
        if let Some(arg) = self.successor_arg(b) {
            if let Some(n) = self.nat_value(a).filter(|&n| n > 0) {
                let pred = self.types.lit(LitVal::Int(n - 1));
                return Some(self.is_sub(pred, arg));
            }
        }
        if let Some(arg) = self.successor_arg(a) {
            if let Some(n) = self.nat_value(b).filter(|&n| n > 0) {
                let pred = self.types.lit(LitVal::Int(n - 1));
                return Some(self.is_sub(arg, pred));
            }
        }
        let (ta, tb) = (self.types.get(a), self.types.get(b));
        if matches!(ta, Type::Var(_) | Type::Error | Type::Blocked(_) | Type::Nothing) || matches!(tb, Type::Var(_) | Type::Error | Type::Blocked(_) | Type::Any) {
            return None;
        }
        let (ma, mb) = (self.as_match(a), self.as_match(b));
        match (ma, mb) {
            (Some(x), Some(y)) => {
                if x == y || self.same_match(x, y) {
                    return Some(true);
                }
                let bound = self.match_bound(a);
                Some(bound != ANY && self.is_sub(bound, b))
            }
            (Some(_), None) => {
                // A stuck match type is a member of a union that names it.
                if let Type::Union(x, y, _) = tb {
                    let mark = self.snapshot();
                    if self.is_sub(a, x) {
                        return Some(true);
                    }
                    self.rollback(mark);
                    if self.is_sub(a, y) {
                        return Some(true);
                    }
                    self.rollback(mark);
                }
                let bound = self.match_bound(a);
                Some(bound != ANY && self.is_sub(bound, b))
            }
            (None, Some(_)) => Some(match ta {
                Type::Param(p) => {
                    let upper = self.syms.tparam(p).upper;
                    (upper != ANY && self.is_sub(upper, b)) || self.is_sub_by_gadt(p, b, -1)
                }
                Type::AppParam(p, args) => match self.app_param_upper(p, args) {
                    Some(upper) => self.is_sub(upper, b),
                    None => false,
                },
                Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => {
                    let upper = self.member_upper(a);
                    upper != a && upper != ANY && self.is_sub(upper, b)
                }
                Type::This(_) | Type::Term(_) | Type::Select(..) => {
                    let under = self.path_underlying(a);
                    under != a && self.is_sub(under, b)
                }
                Type::Refined(parent, _) => self.is_sub(parent, b),
                Type::Rec(body) => {
                    let opened = self.opened_rec(body, a);
                    self.is_sub(opened, b)
                }
                Type::Union(x, y, _) => self.is_sub(x, b) && self.is_sub(y, b),
                Type::Inter(x, y) => {
                    let mark = self.snapshot();
                    if self.is_sub(x, b) {
                        return Some(true);
                    }
                    self.rollback(mark);
                    self.is_sub(y, b)
                }
                _ => false,
            }),
            (None, None) => {
                let cons = self.b.cons_tuple?;
                match (ta, tb) {
                    (Type::Class(c1, xs), Type::Class(c2, ys)) if c1 == cons && c2 == cons => {
                        let (xs, ys) = (self.types.items(xs).to_vec(), self.types.items(ys).to_vec());
                        Some(self.is_sub(xs[0], ys[0]) && self.is_sub(xs[1], ys[1]))
                    }
                    (Type::Class(c1, xs), Type::Class(c2, ys)) if c2 == cons && self.is_tuple_class(c1) => {
                        let elems = self.types.items(xs).to_vec();
                        let [h, t] = *self.types.items(ys) else { return Some(false) };
                        if elems.is_empty() {
                            return Some(false);
                        }
                        let rest = self.tuple_of(&elems[1..]);
                        Some(self.is_sub(elems[0], h) && self.is_sub(rest, t))
                    }
                    _ => None,
                }
            }
        }
    }

    /// Conformance where a path, an abstract type member or a refinement is on either side. A
    /// path conforms to what its type conforms to and only the same path conforms to it; an
    /// abstract member conforms through its upper bound and is conformed to through its lower
    /// one; a refinement asks its parent and its members of the other side.
    #[cold]
    #[inline(never)]
    /// `p.T` names the member `C#T` or `q.T` for a `p: C` or a `p: q.type`: the same member
    /// through a prefix that conforms.
    fn member_through_prefix(&mut self, m: TypeId, n: TypeId) -> bool {
        let (Type::Member(p, x), Type::Member(q, y)) = (self.types.get(m), self.types.get(n)) else { return false };
        if x != y || p == q || !self.types.is_path(p) {
            return false;
        }
        let mark = self.snapshot();
        let holds = self.is_sub(p, q);
        if !holds {
            self.rollback(mark);
        }
        holds
    }

    /// The path `t` with every val declared as a singleton type replaced by that type (`alias`
    /// declared `a.type` reads as `a`, `self` declared `this.type` as `this`).
    fn stable_path(&mut self, t: TypeId, depth: u32) -> Option<TypeId> {
        if depth > 8 {
            return None;
        }
        match self.types.get(t) {
            Type::This(_) => Some(t),
            // An object stands for itself as its class type.
            Type::Class(c, _) if self.syms.class(c).kind == ClassKind::Object => Some(t),
            Type::Term(v) => match self.declared_singleton(v) {
                Some(u) => self.stable_path(u, depth + 1),
                None => Some(t),
            },
            Type::Select(p, v) => {
                let np = self.stable_path(p, depth + 1)?;
                match self.declared_singleton(v) {
                    Some(u) => self.stable_path(u, depth + 1),
                    None if np == p => Some(t),
                    // Read on the object the prefix names (`alias.U` of an `alias: O.type` is `O.U`).
                    None => Some(match self.types.get(np) {
                        Type::Class(o, _) => self.object_member_path(o, v),
                        _ => self.types.mk(Type::Select(np, v)),
                    }),
                }
            }
            _ => None,
        }
    }

    /// dotty's `compareThis` of an object's `this`: the path to an object nested in a class
    /// (`Outer.this.R`, `p.R`) and the object's own `this` (`R.this`) are one value where the
    /// path's prefix conforms to the enclosing class's `this`, in both directions.
    fn object_this_holds(&mut self, ta: Type, tb: Type) -> bool {
        let (path, r, flipped) = match (ta, tb) {
            (Type::Select(p, v), Type::This(r)) if self.syms.class(r).inner_object == Some(v) => (p, r, false),
            (Type::This(r), Type::Select(p, v)) if self.syms.class(r).inner_object == Some(v) => (p, r, true),
            _ => return false,
        };
        let Owner::Class(o) = self.syms.class(r).owner else { return false };
        let this = self.types.mk(Type::This(o));
        let mark = self.snapshot();
        let holds = if flipped { self.is_sub(this, path) } else { self.is_sub(path, this) };
        if !holds {
            self.rollback(mark);
        }
        holds
    }

    /// dotty's `compareNamed` of a module class against its module's `TermRef` (`X$ <:< X.type`):
    /// the class of an object nested in a class, or of a given object, seen through the prefix
    /// `p1` (`p1.U$`, bare through its owner's `this`) conforms to the object's path `p2.U` where
    /// `p1` conforms to `p2`, so that `B.U` is no `C.U` of another instance; the bare class of a
    /// given object of an object or a package conforms to its path, one value (`isStaticOwner`).
    fn module_class_holds(&mut self, a: TypeId, tb: Type) -> bool {
        let (p2, v) = match tb {
            Type::Select(p, v) => (Some(p), v),
            Type::Term(v) => (None, v),
            _ => return false,
        };
        let (p1, class) = match self.types.get(a) {
            Type::Nested(p, class) => (Some(p), class),
            Type::Class(..) => (None, a),
            _ => return false,
        };
        let Type::Class(c, _) = self.types.get(class) else { return false };
        if self.module_val_of_class(c) != Some(v) {
            return false;
        }
        let Some(p2) = p2 else { return p1.is_none() };
        let p1 = match p1 {
            Some(p) => p,
            None => {
                let Owner::Class(o) = self.syms.class(c).owner else { return false };
                self.types.mk(Type::This(o))
            }
        };
        self.is_sub_prefix(p1, p2)
    }

    /// The val that holds the one instance of the class `c` where `c` is an object's: the lazy
    /// val of an object nested in a class, or a given object without parameters (`given u: T
    /// with {}`, a module in dotty), whose class has its name and owner.
    pub(super) fn module_val_of_class(&self, c: ClassId) -> Option<SymId> {
        let info = self.syms.class(c);
        if info.inner_object.is_some() {
            return info.inner_object;
        }
        if info.kind != ClassKind::GivenImpl {
            return None;
        }
        let g = match info.owner {
            Owner::Class(o) => self.syms.class(o).members.get(&info.name).copied(),
            Owner::Package(p) => self.syms.pkg(p).entries.get(&info.name).and_then(|e| e.term),
            Owner::Local => None,
        }?;
        let s = self.syms.sym(g);
        (s.kind == SymKind::Given && s.impl_class == Some(c) && s.mods & crate::ast::mods::LAZY != 0).then_some(g)
    }

    /// The singleton type a stable val is declared with, if any: a path, or an object's type
    /// (`val alias: O.type`), which is the object's one value.
    fn declared_singleton(&self, v: SymId) -> Option<TypeId> {
        let info = self.syms.sym(v);
        if !matches!(info.kind, SymKind::Val | SymKind::Param) || info.by_name || info.mods & crate::ast::mods::MUTABLE != 0 {
            return None;
        }
        let ret = info.sig.as_ref()?.ret;
        match self.types.get(ret) {
            Type::This(_) | Type::Term(_) | Type::Select(..) => Some(ret),
            Type::Class(c, args) if args == EMPTY_LIST && self.syms.class(c).kind == ClassKind::Object => Some(ret),
            _ => None,
        }
    }

    fn is_sub_dependent(&mut self, a: TypeId, b: TypeId) -> bool {
        if self.dep_depth >= 32 {
            return false;
        }
        // The outermost dependent comparison starts the step budget of the member lookups it
        // makes, as the outermost lookup does.
        if self.dep_depth == 0 {
            self.start_lookup_budget(a, b);
        }
        self.dep_depth += 1;
        let holds = self.is_sub_dependent_inner(a, b);
        self.dep_depth -= 1;
        holds
    }

    /// The variances of a type member a refinement declares, `type T[+A]`: its type lambda's
    /// parameters'.
    fn refined_member_variances(&mut self, m: TypeId) -> Vec<i8> {
        let Type::Member(prefix, name) = self.types.get(m) else { return Vec::new() };
        let info = match self.type_member(prefix, name) {
            Some(super::members::MemberInfo::Bounds(_, hi)) => hi,
            Some(super::members::MemberInfo::Alias(x)) => x,
            None => return Vec::new(),
        };
        match self.types.get(info) {
            Type::Lambda(ps, _) => self.types.items(ps).iter().filter_map(|&p| match self.types.get(p) {
                Type::Param(p) => Some(self.syms.tparam(p).variance),
                _ => None,
            }).collect(),
            _ => Vec::new(),
        }
    }

    fn is_sub_dependent_inner(&mut self, a: TypeId, b: TypeId) -> bool {
        let (ta, tb) = (self.types.get(a), self.types.get(b));
        match (ta, tb) {
            (Type::AppMember(m, xs), Type::AppMember(n, ys)) if m == n || self.member_through_prefix(m, n) => {
                let mut variances = self.member_variances(m);
                if variances.is_empty() {
                    variances = self.refined_member_variances(m);
                }
                if variances.is_empty() {
                    return self.same_args(xs, ys);
                }
                let xs: Vec<TypeId> = self.types.items(xs).to_vec();
                let ys: Vec<TypeId> = self.types.items(ys).to_vec();
                return xs.len() == ys.len()
                    && xs.iter().zip(&ys).enumerate().all(|(i, (&x, &y))| match variances.get(i).copied().unwrap_or(0) {
                        1 => self.is_sub(x, y),
                        -1 => self.is_sub(y, x),
                        _ => self.is_same(x, y),
                    });
            }
            (Type::AppVar(v, xs), Type::AppMember(m, ys)) | (Type::AppMember(m, ys), Type::AppVar(v, xs))
                if self.types.items(xs).len() <= self.types.items(ys).len() =>
            {
                let (n, k) = (self.types.items(xs).len(), self.types.items(ys).len());
                let ctor = if n == k {
                    m
                } else {
                    // Partial unification, as against a class: `F[Any, Any]` against an abstract
                    // `T[Any, Any, Any, Any]` makes `F` the `[X, Y] =>> T[?, ?, X, Y]` whose
                    // leftmost arguments the comparison then fixes (dotty's `canInstantiate`).
                    let mut full: Vec<TypeId> = (0..k - n).map(|_| self.fixed_var()).collect();
                    let params: Vec<TypeId> = (0..n)
                        .map(|_| {
                            let p = self.syms.new_tparam(crate::names::WILDCARD, 0);
                            self.types.param(p)
                        })
                        .collect();
                    full.extend(params.iter().copied());
                    let fl = self.types.list(&full);
                    let body = self.types.mk(Type::AppMember(m, fl));
                    let pl = self.types.list(&params);
                    self.types.mk(Type::Lambda(pl, body))
                };
                self.instantiate(v, ctor);
                return self.is_sub(a, b);
            }
            _ => {}
        }
        match tb {
            Type::This(_) | Type::Term(_) | Type::Select(..) => {
                return match ta {
                    Type::This(_) | Type::Term(_) | Type::Select(..) => {
                        // Two paths naming one value through vals declared as singletons
                        // (`val alias: a.type = a`, then `alias.R.type` for `a.R`).
                        if let (Some(x), Some(y)) = (self.stable_path(a, 0), self.stable_path(b, 0)) {
                            if x == y {
                                return true;
                            }
                        }
                        if self.object_this_holds(ta, tb) {
                            return true;
                        }
                        // One member through prefixes that conform, dotty's `compareNamed` of two
                        // `TermRef`s: of one symbol, or of one name where neither is private (an
                        // abstract val and the object implementing it, `Exprs.this.ExprPromise`).
                        if let (Type::Select(p1, v1), Type::Select(p2, v2)) = (ta, tb) {
                            let same = v1 == v2 || (self.syms.sym(v1).name == self.syms.sym(v2).name && !self.is_private(v1) && !self.is_private(v2));
                            if same && self.is_sub_prefix(p1, p2) {
                                return true;
                            }
                        }
                        let under = self.path_underlying(a);
                        under != a && self.is_sub(under, b)
                    }
                    Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => {
                        let (_, upper) = self.member_bounds(a);
                        upper != a && self.is_sub(upper, b)
                    }
                    Type::Param(p) => {
                        let upper = self.syms.tparam(p).upper;
                        upper != ANY && self.is_sub(upper, b)
                    }
                    Type::AppParam(p, args) => match self.app_param_upper(p, args) {
                        Some(upper) => self.is_sub(upper, b),
                        None => false,
                    },
                    Type::Inter(x, y) => {
                        let mark = self.snapshot();
                        if self.is_sub(x, b) {
                            return true;
                        }
                        self.rollback(mark);
                        self.is_sub(y, b)
                    }
                    // An object's type is the path of a val declared with it (`O` and an
                    // `alias: O.type`).
                    Type::Class(..) | Type::Nested(..) => self.stable_path(b, 0) == Some(a) || self.module_class_holds(a, tb),
                    _ => false,
                };
            }
            Type::Refined(parent, r) => {
                let mark = self.snapshot();
                if !self.is_sub(a, parent) {
                    return false;
                }
                if self.refinement_holds(a, parent, r) {
                    return true;
                }
                self.rollback(mark);
                return false;
            }
            Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => {
                let (lower, _) = self.member_bounds(b);
                let mark = self.snapshot();
                if lower != NOTHING && lower != b && self.is_sub(a, lower) {
                    return true;
                }
                self.rollback(mark);
                // Two members of one name through prefixes that conform: `p.T <: C#T` for a
                // `p: C`, `x.T <: y.T` for an `x: y.type`, and `C#T <: D#T` for a `C <: D`.
                if let (Type::Member(p, n), Type::Member(q, m)) = (ta, tb) {
                    if n == m && p != q && (self.types.is_path(p) || !self.types.is_path(q)) && self.is_sub(p, q) {
                        return true;
                    }
                    self.rollback(mark);
                }
            }
            // A path's type below a parameter bounded below by it: the capture of a pattern's
            // `Box[? >: h.Start]` takes `h.start`.
            Type::Param(p) if self.syms.tparam(p).lower != NOTHING => {
                let lower = self.syms.tparam(p).lower;
                let mark = self.snapshot();
                if self.is_sub(a, lower) {
                    return true;
                }
                self.rollback(mark);
            }
            _ => {}
        }
        match ta {
            Type::This(_) | Type::Term(_) | Type::Select(..) => {
                let under = self.path_underlying(a);
                under != a && self.is_sub(under, b)
            }
            Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => {
                let (_, upper) = self.member_bounds(a);
                upper != a && self.is_sub(upper, b)
            }
            Type::Refined(parent, _) => self.is_sub(parent, b),
            Type::Rec(body) => {
                let opened = self.opened_rec(body, a);
                self.is_sub(opened, b)
            }
            // A type parameter below a member type through its bound (`a <: c.Start` for a
            // pattern's `a`) or what the case fixed it to.
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                (upper != ANY && self.is_sub(upper, b)) || self.is_sub_by_gadt(p, b, -1)
            }
            _ => false,
        }
    }

    /// What an intersection conforms to beyond what either side conforms to alone: it
    /// distributes over a union in one of its sides, and two views of one class merge into
    /// that class with the arguments intersected (`Cov[A] & Cov[B] <: Cov[A & B]`).
    #[cold]
    #[inline(never)]
    fn is_sub_inter_laws(&mut self, x: TypeId, y: TypeId, b: TypeId, mark: usize) -> bool {
        for (side, other) in [(x, y), (y, x)] {
            let side = self.deref(side);
            if let Type::Union(p, q, _) = self.types.get(side) {
                let (op, oq) = (self.types.inter(other, p), self.types.inter(other, q));
                if self.is_sub(op, b) && self.is_sub(oq, b) {
                    return true;
                }
                self.rollback(mark);
            }
        }
        let Type::Class(c, _) = self.types.get(b) else { return false };
        let (Some(bx), Some(by)) = (self.base_type(x, c), self.base_type(y, c)) else { return false };
        let Some(merged) = self.glb_of_class(bx, by) else { return false };
        if self.is_sub(merged, b) {
            return true;
        }
        self.rollback(mark);
        false
    }

    /// The greatest lower bound of two instances of one class, argument by argument.
    fn glb_of_class(&mut self, x: TypeId, y: TypeId) -> Option<TypeId> {
        let (Type::Class(c, xa), Type::Class(_, ya)) = (self.types.get(x), self.types.get(y)) else { return None };
        let xs: Vec<TypeId> = self.types.items(xa).to_vec();
        let ys: Vec<TypeId> = self.types.items(ya).to_vec();
        self.settle_class(c);
        let tparams = self.syms.class(c).tparams.clone();
        let mut merged = Vec::with_capacity(xs.len());
        for i in 0..xs.len().min(ys.len()) {
            let variance = tparams.get(i).map_or(0, |&p| self.syms.tparam(p).variance);
            merged.push(match variance {
                1 => self.types.inter(xs[i], ys[i]),
                -1 => self.types.union(xs[i], ys[i]),
                _ if xs[i] == ys[i] => xs[i],
                _ => return None,
            });
        }
        Some(self.types.class(c, &merged))
    }

    fn is_sub_union_member(&mut self, a: TypeId, b: TypeId, with_vars: bool, mark: usize) -> bool {
        match self.types.get(b) {
            Type::Union(x, y, _) => {
                self.is_sub_union_member(a, x, with_vars, mark) || self.is_sub_union_member(a, y, with_vars, mark)
            }
            _ if self.types.has_vars(b) != with_vars => false,
            _ => {
                if self.is_sub(a, b) {
                    return true;
                }
                self.rollback(mark);
                false
            }
        }
    }

    /// Whether a bound that the enclosing match cases give `p` relates it to `other`: with
    /// `side` 1 `other` has to conform to a lower bound, with -1 an upper bound to `other`.
    fn is_sub_by_gadt(&mut self, p: TParamId, other: TypeId, side: i8) -> bool {
        !self.gadt.is_empty() && self.is_sub_by_gadt_bound(p, other, side)
    }

    #[cold]
    #[inline(never)]
    fn is_sub_by_gadt_bound(&mut self, p: TParamId, other: TypeId, side: i8) -> bool {
        const IN_USE: i8 = 2;
        for i in 0..self.gadt.len() {
            let (q, bound, variance) = self.gadt[i];
            if q != p || variance == IN_USE || variance == -side {
                continue;
            }
            self.gadt[i].2 = IN_USE;
            let mark = self.snapshot();
            let holds = if side == 1 { self.is_sub(other, bound) } else { self.is_sub(bound, other) };
            self.gadt[i].2 = variance;
            if holds {
                return true;
            }
            self.rollback(mark);
        }
        false
    }

    /// Type lambdas are compared up to the names of their parameters, and `[X] =>> F[X]` is `F`.
    fn is_sub_lambda(&mut self, a: TypeId, b: TypeId) -> bool {
        let (ra, rb) = (self.eta_reduce(a), self.eta_reduce(b));
        match (self.types.get(ra), self.types.get(rb)) {
            (Type::Lambda(ps, x), Type::Lambda(qs, y)) => {
                let ps: Vec<TypeId> = self.types.items(ps).to_vec();
                let qs: Vec<TypeId> = self.types.items(qs).to_vec();
                if ps.len() != qs.len() {
                    return false;
                }
                let mut renaming: Subst = Vec::with_capacity(qs.len());
                for (&q, &p) in qs.iter().zip(&ps) {
                    if let Type::Param(id) = self.types.get(q) {
                        renaming.push((id, p));
                    }
                }
                let y = self.types.subst(y, &renaming);
                self.is_sub_under_binders(x, y, &ps)
            }
            // `F <: [X] =>> B` when `F[X] <: B`, and `[X] =>> A <: G` when `A <: G[X]`. A class
            // inheriting the bound's trait twice with different arguments (`Seq` as
            // `Collection[Iterable, T]` and `Collection[Seq, T]`) is read through its first
            // base; deriving from the bound's class at all is taken as conforming.
            (_, Type::Lambda(qs, y)) if self.is_type_ctor(ra) => {
                let args: Vec<TypeId> = self.types.items(qs).to_vec();
                let applied = self.types.apply_ctor(ra, &args);
                applied != ERROR && (self.is_sub_under_binders(applied, y, &args) || self.derives_shape(applied, y))
            }
            (Type::Lambda(ps, x), _) if self.is_type_ctor(rb) => {
                let args: Vec<TypeId> = self.types.items(ps).to_vec();
                let applied = self.types.apply_ctor(rb, &args);
                applied != ERROR && self.is_sub_under_binders(x, applied, &args)
            }
            (Type::Lambda(..), _) | (_, Type::Lambda(..)) => false,
            _ => self.is_sub(ra, rb),
        }
    }

    /// The bodies of two lambdas compared under their binders `ps`: an inference variable from
    /// outside cannot take a type naming a binder (`Const[T] = [X] =>> T` is no `List`, as
    /// `T := List[X]` would let `X` escape).
    fn is_sub_under_binders(&mut self, a: TypeId, b: TypeId, ps: &[TypeId]) -> bool {
        let mark = self.snapshot();
        if !self.is_sub(a, b) {
            return false;
        }
        let binders: Vec<TParamId> = ps.iter().filter_map(|&p| match self.types.get(p) {
            Type::Param(id) => Some(id),
            _ => None,
        }).collect();
        let escaped = self.trail[mark..].iter().any(|u| {
            let bound = match *u {
                Undo::Inst(v) => self.tvars[v].inst,
                Undo::Lower(v) => self.tvars[v].lower.last().copied(),
                Undo::Upper(v) => self.tvars[v].upper.last().copied(),
                Undo::Dep(_) | Undo::DropDeps(_) => None,
            };
            bound.map_or(false, |t| self.mentions_tparam_of(t, Some(&binders)))
        });
        if escaped {
            self.rollback(mark);
            return false;
        }
        true
    }

    /// Whether the value parameters of two polymorphic functions' function types `x` and `y`
    /// erase to the same classes, a type parameter to its bound's (dotty's `Signature.paramsSig`);
    /// a parameter whose erasure holds an uninstantiated variable erases to nothing yet and matches
    /// any (`Signature.consistentParams`' wildcard, `tpnme.Uninstantiated`), the variable reached
    /// through a type parameter's bound too (`TypeErasure.sigName`: a `TypeProxy` is signed as its
    /// underlying type, an uninstantiated `TypeVar` as `Uninstantiated`). `[T <: U] => T => U`
    /// expected while `U` is inferred takes `[T <: String] => (t: T) => ..`.
    fn poly_params_consistent(&mut self, x: TypeId, y: TypeId) -> bool {
        let (Some((xs, _)), Some((ys, _))) = (self.as_function(x), self.as_function(y)) else { return true };
        xs.len() == ys.len()
            && xs.iter().zip(&ys).all(|(&p, &q)| {
                let (p, q) = (self.deref(p), self.deref(q));
                self.sig_uninstantiated(p, 0) || self.sig_uninstantiated(q, 0) || self.erased_class(p, 0) == self.erased_class(q, 0)
            })
    }

    /// Whether the signature of `t` is `tpnme.Uninstantiated`: it holds an uninstantiated
    /// variable, or it is a type parameter whose bound does (`TypeErasure.sigName` of a
    /// `TypeProxy`, its underlying type's).
    fn sig_uninstantiated(&mut self, t: TypeId, depth: u32) -> bool {
        let t = self.deref(t);
        if self.types.has_vars(t) {
            return true;
        }
        match self.types.get(t) {
            Type::Param(p) | Type::AppParam(p, _) if depth < 8 => {
                let upper = self.syms.tparam(p).upper;
                self.sig_uninstantiated(upper, depth + 1)
            }
            _ => false,
        }
    }

    /// Polymorphic function types are compared up to the names of their type parameters, by
    /// their function types; one conforms to `scala.PolyFunction` and to what its function type
    /// conforms to, as it erases to that.
    fn is_sub_poly(&mut self, a: TypeId, b: TypeId) -> bool {
        match (self.types.get(a), self.types.get(b)) {
            // The parameters renamed, the wanted bounds within the found ones (dotty's
            // `TypeComparer.comparePoly`, `matchingPolyParams`).
            (Type::Poly(..), Type::Poly(..)) => {
                let (Some((ps, x)), Some((qs, y))) = (self.poly_binders(a), self.poly_binders(b)) else { return false };
                if ps.len() != qs.len() {
                    return false;
                }
                // The parameters erase alike, each function's under its own binders' bounds
                // (`comparePoly`'s `signature.consistentParams`, TypeComparer.scala 838):
                // `[T <: Int | String] => T => T` is no `[T <: Int & String] => T => T`.
                if !self.poly_params_consistent(x, y) {
                    return false;
                }
                let renaming: Subst = qs.iter().zip(&ps).map(|(&q, &p)| (q, self.types.param(p))).collect();
                for (&p, &q) in ps.iter().zip(&qs) {
                    let (lo, hi) = (self.syms.tparam(p).lower, self.syms.tparam(p).upper);
                    let (wlo, whi) = (self.syms.tparam(q).lower, self.syms.tparam(q).upper);
                    let (wlo, whi) = (self.types.subst(wlo, &renaming), self.types.subst(whi, &renaming));
                    if !(self.is_sub(lo, wlo) && self.is_sub(whi, hi)) {
                        return false;
                    }
                }
                let y = self.types.subst(y, &renaming);
                self.is_sub(x, y)
            }
            (Type::Poly(_, x), Type::Class(c, _)) => {
                let poly_function = self.interner.get(self.syms.class(c).name) == "PolyFunction";
                poly_function || self.is_sub(x, b)
            }
            (Type::Poly(_, x), _) => self.is_sub(x, b),
            _ => false,
        }
    }

    /// A class or a higher-kinded parameter standing for a type constructor.
    fn is_type_ctor(&self, t: TypeId) -> bool {
        match self.types.get(t) {
            Type::Ctor(_) => true,
            Type::Param(p) => self.syms.tparam(p).arity > 0,
            Type::Decl(_) | Type::Member(..) | Type::Alias(..) => self.type_arity(t) > 0,
            _ => false,
        }
    }

    /// `Sub <: Base` as type constructors when `Sub[A] extends Base[A]` passes its parameters on
    /// in order.
    fn ctor_extends(&mut self, sub: ClassId, sup: ClassId) -> bool {
        self.complete_class(sub);
        let info = self.syms.class(sub);
        let Some(&(_, at_sup)) = info.base_types.iter().find(|&&(b, _)| b == sup) else { return false };
        let Type::Class(_, args) = self.types.get(at_sup) else { return false };
        let args = self.types.items(args);
        args.len() == info.tparams.len()
            && args.iter().zip(&info.tparams).all(|(&a, &p)| matches!(self.types.get(a), Type::Param(q) if q == p))
    }

    pub(super) fn eta_reduce(&mut self, t: TypeId) -> TypeId {
        let Type::Lambda(ps, body) = self.types.get(t) else { return t };
        match self.types.get(body) {
            Type::Class(c, args) if args == ps => self.types.mk(Type::Ctor(c)),
            Type::AppParam(p, args) if args == ps => self.types.param(p),
            // `[A] =>> Impl.Type[A]` is the abstract type constructor itself.
            Type::AppMember(m, args) if args == ps => m,
            _ => t,
        }
    }

    /// Invariant arguments, each the same type or within the other side's wildcard.
    fn contained_args(&mut self, xs: TList, ys: TList) -> bool {
        let xs: Vec<TypeId> = self.types.items(xs).to_vec();
        let ys: Vec<TypeId> = self.types.items(ys).to_vec();
        xs.len() == ys.len()
            && xs.iter().zip(&ys).all(|(&x, &y)| match self.types.wild_bounds(y) {
                Some((lo, hi)) => {
                    let (xlo, xhi) = self.types.wild_bounds(x).unwrap_or((x, x));
                    self.is_sub(lo, xlo) && self.is_sub(xhi, hi)
                }
                None => self.is_same(x, y),
            })
    }

    fn same_args(&mut self, xs: TList, ys: TList) -> bool {
        let xs: Vec<TypeId> = self.types.items(xs).to_vec();
        let ys: Vec<TypeId> = self.types.items(ys).to_vec();
        xs.len() == ys.len() && xs.iter().zip(&ys).all(|(&x, &y)| self.is_same(x, y))
    }

    /// `String` implements `java.lang.CharSequence` and `Comparable[String]`, whichever source
    /// declares them; its members stay the std's extensions.
    fn string_interface(&mut self, target: ClassId) -> Option<TypeId> {
        let info = self.syms.class(target);
        let Owner::Package(p) = info.owner else { return None };
        if !self.pkg_is(p, "java.lang") {
            return None;
        }
        match self.name_str(info.name).as_str() {
            "CharSequence" => Some(self.types.class(target, &[])),
            "Comparable" => {
                let string = self.types.class(self.b.string, &[]);
                Some(self.types.class(target, &[string]))
            }
            _ => None,
        }
    }

    /// On JavaScript a function is a JavaScript function, a `js.Function` and so a `js.Object`.
    #[cold]
    #[inline(never)]
    fn function_interface(&mut self, target: ClassId) -> Option<TypeId> {
        let info = self.syms.class(target);
        let Owner::Package(p) = info.owner else { return None };
        if !self.pkg_is(p, "scala.scalajs.js") {
            return None;
        }
        Some(self.types.class(target, &[]))
    }

    /// On JavaScript `Array` is the JS array, which Scala.js's `js.Array[A]` makes a
    /// `js.Iterable[A]`.
    fn array_interface(&mut self, target: ClassId) -> Option<TypeId> {
        if self.jvm {
            return None;
        }
        let info = self.syms.class(target);
        let Owner::Package(p) = info.owner else { return None };
        if info.name != crate::names::ITERABLE || !self.pkg_is(p, "scala.scalajs.js") {
            return None;
        }
        let a = *self.syms.class(self.b.array).tparams.first()?;
        let elem = self.types.param(a);
        Some(self.types.class(target, &[elem]))
    }

    /// The upper bound of the higher-kinded parameter `p` applied to `args`: `Semigroup[A]` for
    /// `S[A]` under `S[T] <: Semigroup[T]`.
    /// Whether the class of `t` derives from every class an intersection of classes names.
    fn derives_shape(&mut self, t: TypeId, shape: TypeId) -> bool {
        let Type::Class(c, _) = self.types.get(t) else { return false };
        match self.types.get(shape) {
            Type::Class(d, _) => c == d || self.derives_from(c, d),
            Type::Inter(x, y) => self.derives_shape(t, x) && self.derives_shape(t, y),
            _ => false,
        }
    }

    pub fn app_param_upper(&mut self, p: TParamId, args: TList) -> Option<TypeId> {
        let upper = self.syms.tparam(p).upper;
        if upper == ANY {
            return None;
        }
        let items: Vec<TypeId> = self.types.items(args).to_vec();
        let t = self.types.apply_ctor(upper, &items);
        // `A[X] <: A[X]` bounds the parameter by itself, which is reported where it is written.
        let cyclic = matches!(self.types.get(t), Type::AppParam(q, _) if q == p);
        (t != ERROR && !cyclic).then_some(t)
    }

    /// The lower bound of the higher-kinded parameter `p` applied to `args`: `F[A]` for `F2[A]`
    /// under `F2[X] >: F[X]`.
    pub fn app_param_lower(&mut self, p: TParamId, args: TList) -> Option<TypeId> {
        let lower = self.syms.tparam(p).lower;
        if lower == NOTHING {
            return None;
        }
        let items: Vec<TypeId> = self.types.items(args).to_vec();
        let t = self.types.apply_ctor(lower, &items);
        let cyclic = matches!(self.types.get(t), Type::AppParam(q, _) if q == p);
        (t != ERROR && !cyclic).then_some(t)
    }

    /// The view of `t` as an instance of class `target`, if it has that class as an ancestor.
    /// Whether `a`, the class `c` applied to `cargs`, conforms to `b` through the first of its
    /// base types that takes `n` arguments or more.
    fn is_sub_through_base(&mut self, a: TypeId, c: ClassId, cargs: TList, b: TypeId, n: usize) -> bool {
        self.complete_class(c);
        let subst: Subst = self.syms.class(c).tparams.iter().copied().zip(self.types.items(cargs).iter().copied()).collect();
        let bases: Vec<TypeId> = self.syms.class(c).base_types.iter().skip(1).map(|&(_, t)| t).collect();
        for bt in bases {
            let Type::Class(_, bargs) = self.types.get(bt) else { continue };
            if self.types.items(bargs).len() < n {
                continue;
            }
            let base = self.types.subst(bt, &subst);
            // `Tuple1[X]`'s base `X *: EmptyTuple` reduces back to it.
            if base == a || self.reduce_head(base) == Some(a) {
                continue;
            }
            let mark = self.snapshot();
            if self.is_sub(base, b) {
                return true;
            }
            self.rollback(mark);
        }
        false
    }

    pub fn base_type(&mut self, t: TypeId, target: ClassId) -> Option<TypeId> {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Class(c, args) => {
                if c == target {
                    return Some(t);
                }
                // dotty's `baseTypeCache`: the base type of a class type, kept per type and base
                // class once the class is complete and where the type has no inference variable.
                if let Some(&kept) = self.base_type_cache.get(&(t, target)) {
                    return kept;
                }
                if self.loaded.is_some() && self.jdk_class_unrelated(c, target) {
                    return None;
                }
                self.complete_class(c);
                let found = self.class_base_type(c, args, target);
                if self.syms.class(c).state() == Completion::Done && !self.types.has_vars(t) {
                    self.base_type_cache.insert((t, target), found);
                }
                found
            }
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                if upper == ANY {
                    None
                } else {
                    self.base_type(upper, target)
                }
            }
            Type::AppParam(p, args) => {
                let upper = self.app_param_upper(p, args)?;
                self.base_type(upper, target)
            }
            Type::Inter(a, b) => self.base_type(a, target).or_else(|| self.base_type(b, target)),
            Type::Union(..) => {
                let j = self.union_join(t)?;
                self.base_type(j, target)
            }
            Type::Lit(_) => {
                let class = self.widen_lit(t);
                self.base_type(class, target)
            }
            // `p.C`'s base type: the class's, as seen from the prefix (dotty's `baseType` of a
            // `TypeRef`, its parents seen from its prefix).
            Type::Nested(p, class) => {
                let Type::Class(c, _) = self.types.get(class) else { return None };
                if c == target {
                    return Some(t);
                }
                let bt = self.base_type(class, target)?;
                Some(self.outer_seen_from(bt, p, c))
            }
            Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..) | Type::Rec(_) | Type::Match(..) | Type::Alias(..) => {
                let under = self.dependent_underlying(t)?;
                self.base_type(under, target)
            }
            _ => None,
        }
    }

    /// `base_type` of the class type `c[args]`, `c` complete.
    fn class_base_type(&mut self, c: ClassId, args: TList, target: ClassId) -> Option<TypeId> {
        let found = self.syms.class(c).base_types.iter().find(|(bc, _)| *bc == target).map(|&(_, t)| t);
        let bt = match found {
            Some(bt) => bt,
            None if c == self.b.string => self.string_interface(target)?,
            None if c == self.b.array => self.array_interface(target)?,
            None if self.b.js_dynamic.is_some() && !self.jvm && matches!(self.syms.class(target).name, crate::names::FUNCTION | crate::names::OBJECT) && self.is_function_class(c) => self.function_interface(target)?,
            None => return None,
        };
        let subst: Subst = self.syms.class(c).tparams.iter().copied().zip(self.types.items(args).iter().copied()).collect();
        Some(self.types.subst(bt, &subst))
    }

    /// The type a path's value has, the upper bound of an abstract member, the parent of a
    /// refinement, the reduction or bound of a match type: what the members and base types of
    /// such a type are those of.
    #[inline(never)]
    pub fn dependent_underlying(&mut self, t: TypeId) -> Option<TypeId> {
        match self.types.get(t) {
            Type::This(_) | Type::Term(_) | Type::Select(..) => {
                let under = self.path_underlying(t);
                (under != t).then_some(under)
            }
            Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => {
                let (_, upper) = self.member_bounds(t);
                (upper != t && upper != ANY).then_some(upper)
            }
            Type::Refined(parent, _) => Some(parent),
            Type::Rec(body) => Some(self.opened_rec(body, t)),
            Type::Match(..) | Type::Alias(..) => {
                let under = self.reduce_head(t).unwrap_or_else(|| self.match_bound(t));
                (under != t && under != ANY).then_some(under)
            }
            // The class itself, its enclosing classes not seen from the prefix: where that
            // matters, `base_type` and `find_member` see it (`outer_seen_from`).
            Type::Nested(_, class) => Some(class),
            _ => None,
        }
    }

    /// Capture conversion: the wildcard arguments of a value's type stand for one unknown type
    /// each while the value is used, so that a `Box[?]` passes for the `Box[A]` a method takes
    /// and its members agree with each other.
    pub fn capture_wildcards(&mut self, t: TypeId) -> TypeId {
        if !self.wildcards_used.load(std::sync::atomic::Ordering::Relaxed) {
            return t;
        }
        self.capture_wildcards_of(t, None)
    }

    /// The capture of the value `e`: a local's is made once and kept.
    pub fn capture_wildcards_in(&mut self, e: TExprId, t: TypeId) -> TypeId {
        if !self.wildcards_used.load(std::sync::atomic::Ordering::Relaxed) {
            return t;
        }
        let local = match self.prog.expr(e) {
            TExpr::Local(s) => Some(s),
            _ => None,
        };
        self.capture_wildcards_of(t, local)
    }

    fn capture_wildcards_of(&mut self, t: TypeId, local: Option<SymId>) -> TypeId {
        let t = match self.types.get(t) {
            Type::Var(_) | Type::AppVar(..) => self.deref(t),
            // A path to a value with wildcard arguments captures them through its type.
            Type::Term(_) | Type::Select(..) => {
                let under = self.widen_path(t);
                if self.types.has_wild(under) { under } else { t }
            }
            _ => t,
        };
        if !self.types.has_wild(t) {
            return t;
        }
        let Type::Class(c, args) = self.types.get(t) else { return t };
        if !self.types.items(args).iter().any(|&a| self.types.is_wild(a)) {
            return t;
        }
        if let Some(&kept) = local.and_then(|s| self.captured_locals.get(&s)) {
            return kept;
        }
        let items: Vec<TypeId> = self.types.items(args).to_vec();
        self.settle_class(c);
        let params = self.syms.class(c).tparams.clone();
        let captured: Vec<TypeId> = items
            .into_iter()
            .enumerate()
            .map(|(i, a)| {
                let Some((lo, hi)) = self.types.wild_bounds(a) else { return a };
                let p = self.syms.new_tparam(crate::names::WILDCARD, 0);
                // The captured type lies within the wildcard's bounds, or else the parameter's
                // declared bound, which a member selected through it has (`x.get.bar` on a
                // `Box[?]` for `Box[D <: Foo]`).
                if hi != ANY {
                    self.syms.tparams[p.idx()].upper = hi;
                } else if let Some(upper) = params.get(i).map(|&q| self.syms.tparam(q).upper) {
                    if matches!(self.types.get(upper), Type::Class(_, bargs) if self.types.items(bargs).is_empty()) {
                        self.syms.tparams[p.idx()].upper = upper;
                    }
                }
                self.syms.tparams[p.idx()].lower = lo;
                self.types.param(p)
            })
            .collect();
        let captured = self.types.class(c, &captured);
        if let Some(s) = local {
            self.captured_locals.insert(s, captured);
        }
        captured
    }

    /// The class of a literal type, any other type left alone: Scala's widening of an inferred
    /// type. A union of literal types is kept, as it is when written out (`type Month = 1 | 2`).
    pub fn widen_lit(&self, t: TypeId) -> TypeId {
        let Type::Lit(l) = self.types.get(t) else { return t };
        match self.types.lit_val(l) {
            LitVal::Int(_) => self.b.t_int,
            LitVal::Long(_) => self.b.t_long,
            LitVal::Double(_) => self.b.t_double,
            LitVal::Float(_) => self.b.t_float,
            LitVal::Char(_) => self.b.t_char,
            LitVal::Bool(_) => self.b.t_boolean,
            LitVal::Str(_) => self.b.t_string,
        }
    }

    /// The type of a `val` or `def` without a declared type: fully defined and widened
    /// (`Namer.inferredResultType`'s `widenInferred(tp, pt, Widen.Unions)`).
    pub fn solve_inferred(&mut self, t: TypeId) -> TypeId {
        let t = self.fully_defined(t);
        let t = self.normalize(t);
        let t = self.widen_lit(t);
        self.widen_unions(t)
    }

    /// `widen_or` of a union or an intersection at the head of `t`, a definition's or a
    /// lambda's result inferred, whose paths and constants the definition widens itself.
    pub(super) fn widen_unions(&mut self, t: TypeId) -> TypeId {
        if matches!(self.types.get(t), Type::Union(..) | Type::Inter(..)) { self.widen_or(t, &[]) } else { t }
    }

    /// The bounds of a wildcard argument of `c`'s `i`-th parameter met with the parameter's own,
    /// instantiated with the arguments `args` of the type it is compared with
    /// (`TypeComparer.isSubArgs`' `paramBounds`, `tparam.info.substApprox(tparams2, args2)`): a
    /// wildcard argument stands for the end of its bounds its occurrence's variance takes
    /// (`subst_approx`), and a bound this cannot instantiate is not met.
    fn within_param_bounds(&mut self, c: ClassId, i: usize, lo: TypeId, hi: TypeId, args: &[TypeId]) -> (TypeId, TypeId) {
        let Some(&p) = self.syms.class(c).tparams.get(i) else { return (lo, hi) };
        let (upper, lower) = (self.syms.tparam(p).upper, self.syms.tparam(p).lower);
        if upper == ANY && lower == NOTHING {
            return (lo, hi);
        }
        let tparams = self.syms.class(c).tparams.clone();
        let hi = match upper {
            ANY => hi,
            u => match self.subst_approx(u, &tparams, args, 1, false) {
                Some(u) if hi == ANY => u,
                Some(u) => self.types.inter(hi, u),
                None => hi,
            },
        };
        let lo = match lower {
            NOTHING => lo,
            l => match self.subst_approx(l, &tparams, args, -1, false) {
                Some(l) if lo == NOTHING => l,
                Some(l) => self.types.union(lo, l),
                None => lo,
            },
        };
        (lo, hi)
    }

    /// `t` with the parameters `tparams` replaced by `args`, a wildcard argument by the end of its
    /// bounds the occurrence's variance `variance` takes: the upper in a covariant position, the
    /// lower in a contravariant one, the wildcard itself as an invariant class argument (`as_arg`).
    /// dotty's `Substituters.SubstApproxMap`, whose `range` an `ApproximatingTypeMap` composes
    /// through applications by their parameters' variances. `None` where a replaced parameter
    /// stands where this does not follow it (an invariant position outside an argument, a type
    /// it does not take apart).
    fn subst_approx(&mut self, t: TypeId, tparams: &[TParamId], args: &[TypeId], variance: i8, as_arg: bool) -> Option<TypeId> {
        match self.types.get(t) {
            Type::Param(p) => {
                let Some(i) = tparams.iter().position(|&q| q == p) else { return Some(t) };
                let a = *args.get(i)?;
                match self.types.wild_bounds(a) {
                    None => Some(a),
                    Some((lo, hi)) => match variance {
                        v if v > 0 => Some(hi),
                        v if v < 0 => Some(lo),
                        _ if as_arg => Some(a),
                        _ => None,
                    },
                }
            }
            Type::Class(c, list) => {
                let items = self.types.items(list).to_vec();
                if items.is_empty() {
                    return Some(t);
                }
                let variances: Vec<i8> = self.syms.class(c).tparams.iter().map(|&q| self.syms.tparam(q).variance).collect();
                let mut out = Vec::with_capacity(items.len());
                for (k, &x) in items.iter().enumerate() {
                    let v = variance * variances.get(k).copied().unwrap_or(0);
                    out.push(self.subst_approx(x, tparams, args, v, true)?);
                }
                Some(self.types.class(c, &out))
            }
            Type::Union(a, b, soft) => {
                let (a, b) = (self.subst_approx(a, tparams, args, variance, false)?, self.subst_approx(b, tparams, args, variance, false)?);
                Some(self.types.union_as(soft, a, b))
            }
            Type::Inter(a, b) => {
                let (a, b) = (self.subst_approx(a, tparams, args, variance, false)?, self.subst_approx(b, tparams, args, variance, false)?);
                Some(self.types.inter(a, b))
            }
            Type::Any | Type::Nothing | Type::Error | Type::Lit(_) | Type::Ctor(_) | Type::This(_) | Type::Term(_) | Type::Decl(_) | Type::Blocked(_) | Type::Wild => Some(t),
            _ => None,
        }
    }

    /// The rank of a numeric primitive type, a constant's by its class (dotty's
    /// `classSymbol.isNumericValueClass`, a `ConstantType` being a proxy of its class).
    #[inline]
    pub fn is_numeric(&self, t: TypeId) -> Option<u8> {
        match self.b.num_rank.get(t.idx()) {
            Some(&r) if r != super::NO_RANK => Some(r),
            Some(_) => None,
            None => self.is_numeric_constant(t),
        }
    }

    #[inline(never)]
    fn is_numeric_constant(&self, t: TypeId) -> Option<u8> {
        match self.types.get(t) {
            Type::Lit(_) => match self.b.num_rank.get(self.widen_lit(t).idx()) {
                Some(&r) if r != super::NO_RANK => Some(r),
                _ => None,
            },
            _ => None,
        }
    }

    /// `x & y <: b` where both parts hold open variables: when both conform, the constraint
    /// neither alone implies is dropped, so that `ZEnvironment(a, b)` against a
    /// `ZEnvironment[Base & Meta]` leaves `A` and `B` to the arguments. `None` when at most one
    /// part conforms, for the ordinary rule to decide.
    fn is_sub_inter_necessary(&mut self, x: TypeId, y: TypeId, b: TypeId, mark: usize) -> Option<bool> {
        let left = self.is_sub(x, b);
        self.rollback(mark);
        if !left {
            return None;
        }
        let right = self.is_sub(y, b);
        self.rollback(mark);
        right.then_some(true)
    }

    /// The join of two types: the wider one, the harmonised numeric type or the least common
    /// base type; unrelated types join to their union, member by member for unions, soft (dotty's
    /// `lub`, whose `isSoft` is true but where a written union is widened).
    pub fn lub(&mut self, a: TypeId, b: TypeId) -> TypeId {
        self.lub_as(UnionKind::Soft, a, b)
    }

    /// `lub` making a soft or a hard union (`TypeComparer.lub`'s `isSoft`).
    pub fn lub_as(&mut self, kind: UnionKind, a: TypeId, b: TypeId) -> TypeId {
        let a = self.zonk(a);
        let b = self.zonk(b);
        if a == b {
            return a;
        }
        if a == ERROR || b == ERROR {
            return ERROR;
        }
        let mut members = Vec::new();
        self.union_alternatives(a, &mut members);
        let mut others = Vec::new();
        self.union_alternatives(b, &mut others);
        for m in others {
            let mut joined = false;
            for i in 0..members.len() {
                // Two constants are atoms, disjoint unless equal: their lub is their union, soft
                // (dotty's `mergedLub`, `Atoms.Range`), which an inferred type widens to their
                // class (`widenInferred`'s `widenOr`); a union's join (`union_join`, `OrType.join`)
                // joins them.
                if !self.join_wild && members[i] != m && matches!(self.types.get(members[i]), Type::Lit(_)) && matches!(self.types.get(m), Type::Lit(_)) {
                    continue;
                }
                if let Some(j) = self.join(members[i], m) {
                    members[i] = j;
                    joined = true;
                    break;
                }
            }
            if !joined {
                members.push(m);
            }
        }
        let mut acc = members[0];
        for &m in &members[1..] {
            acc = self.types.union_as(kind, acc, m);
        }
        acc
    }

    /// dotty's `isTransparent()`: a type deriving only from transparent classes, `Any`,
    /// `Matchable`, `Object`, `AnyVal` and the transparent traits.
    fn transparent_join(&mut self, j: TypeId) -> bool {
        match self.types.get(j) {
            Type::Any => true,
            Type::Inter(a, b) => self.transparent_join(a) && self.transparent_join(b),
            Type::Class(c, _) => {
                c == self.b.any_ref
                    || c == self.b.any_val
                    || matches!(self.interner.get(self.syms.class(c).name), "Matchable" | "Object")
                    || self.is_transparent_trait_type(j)
            }
            _ => false,
        }
    }

    fn join(&mut self, a: TypeId, b: TypeId) -> Option<TypeId> {
        if a == b {
            return Some(a);
        }
        // Two constants of one class join to it (or the class and a constant of it).
        if matches!(self.types.get(a), Type::Lit(_)) || matches!(self.types.get(b), Type::Lit(_)) {
            let (wa, wb) = (self.widen_lit(a), self.widen_lit(b));
            if wa == wb {
                return Some(wa);
            }
        }
        let mark = self.snapshot();
        if self.is_sub(a, b) {
            return Some(b);
        }
        self.rollback(mark);
        if self.is_sub(b, a) {
            return Some(a);
        }
        self.rollback(mark);
        let ca = self.class_of(a)?;
        if self.lub_in_progress.contains(&(a, b)) {
            return None;
        }
        self.lub_in_progress.push((a, b));
        let joined = self.lub_of_base_types(a, b, ca);
        self.lub_in_progress.pop();
        joined
    }

    /// The intersection of the minimal base classes `a` and `b` share, each seen with the joined
    /// arguments; a base that another shared one derives from adds nothing, and a class that is
    /// not accessible here is left out, as scalac does. The intersection is written in reverse
    /// linearisation order, as scalac writes it (`Named & HasSize` for `extends Named, HasSize`).
    fn lub_of_base_types(&mut self, a: TypeId, b: TypeId, ca: ClassId) -> Option<TypeId> {
        self.complete_class(ca);
        let candidates: Vec<ClassId> = self.syms.class(ca).base_types.iter().map(|&(c, _)| c).collect();
        let mut shared: Vec<(ClassId, TypeId)> = Vec::new();
        for c in candidates {
            if !self.is_class_accessible(c) {
                continue;
            }
            // A base of a shared base adds nothing to the intersection.
            if shared.iter().any(|&(k, _)| self.syms.class(k).base_types.iter().any(|&(bc, _)| bc == c)) {
                continue;
            }
            let Some(t) = self.lub_as_class(a, b, c) else {
                // Two applications of one class differing in an invariant argument have no
                // least upper bound as a class type: dotc keeps their union, and joins it to
                // the class over a bounded wildcard where a member is selected (`union_join`).
                // An opaque type's applications join to its bound (`NamedTuple[N, V]` of two
                // name tuples is an `AnyNamedTuple`).
                if c == ca && !self.join_wild && self.syms.class(ca).kind != ClassKind::Opaque && self.base_type(b, ca).is_some() {
                    return None;
                }
                continue;
            };
            if c == ca {
                return Some(t);
            }
            shared.push((c, t));
        }
        let mut minimal: Vec<TypeId> = Vec::with_capacity(shared.len());
        for &(c, t) in &shared {
            let derived_from = shared
                .iter()
                .any(|&(other, _)| other != c && self.syms.class(other).base_types.iter().any(|&(bc, _)| bc == c));
            if !derived_from {
                minimal.push(t);
            }
        }
        // dotty's `dropTransparentTraits` in `widenInferred`: a transparent trait (`Product`,
        // `java.io.Serializable`, one declared `transparent`) leaves the intersection where a base
        // that is none remains, as `Set(a, b)` of two case objects of `S` is a `Set[S]`.
        if minimal.iter().any(|&t| !self.is_transparent_trait_type(t)) {
            minimal.retain(|&t| !self.is_transparent_trait_type(t));
        }
        // dotty's `widenOr`: a join that is transparent (`Product & Serializable` of two case
        // classes) leaves the union as it is, where a member's selection joins it (`OrType.join`).
        if !self.join_wild && minimal.iter().all(|&t| self.transparent_join(t)) {
            return None;
        }
        minimal.reverse();
        let (&first, rest) = minimal.split_first()?;
        Some(rest.iter().fold(first, |acc, &t| self.types.inter(acc, t)))
    }

    /// dotty's `isTransparent(traitOnly = true)`: a trait declared `transparent` (`Product`,
    /// `scala.reflect.Enum`) or assumed so (`java.io.Serializable`, `java.lang.Comparable`).
    fn is_transparent_trait_type(&self, t: TypeId) -> bool {
        let Type::Class(c, _) = self.types.get(t) else { return false };
        let info = self.syms.class(c);
        if info.kind != ClassKind::Trait {
            return false;
        }
        if info.mods & crate::ast::mods::TRANSPARENT != 0 || Some(c) == self.b.product || Some(c) == self.b.reflect_enum {
            return true;
        }
        let assumed = match self.interner.get(info.name) {
            "Serializable" => "java.io",
            "Comparable" => "java.lang",
            _ => return false,
        };
        matches!(info.owner, Owner::Package(p) if self.pkg_path(p) == assumed)
    }

    fn lub_as_class(&mut self, a: TypeId, b: TypeId, c: ClassId) -> Option<TypeId> {
        // `c` is a base of `a` already; whether `b` shares it is the question.
        let y = self.base_type(b, c)?;
        let x = self.base_type(a, c)?;
        // An inner class's occurrences join through the join of their prefixes (dotty's
        // `mergeRefinedOrApplied`, `pre1 | pre2`), the bare class's its owner's `this`.
        let (px, x) = match self.types.get(x) {
            Type::Nested(p, class) => (Some(p), class),
            _ => (None, x),
        };
        let (py, y) = match self.types.get(y) {
            Type::Nested(p, class) => (Some(p), class),
            _ => (None, y),
        };
        let (Type::Class(_, xa), Type::Class(_, ya)) = (self.types.get(x), self.types.get(y)) else { return None };
        let xs: Vec<TypeId> = self.types.items(xa).to_vec();
        let ys: Vec<TypeId> = self.types.items(ya).to_vec();
        self.settle_class(c);
        let tparams = self.syms.class(c).tparams.clone();
        let mut merged = Vec::with_capacity(xs.len());
        for i in 0..xs.len() {
            let variance = tparams.get(i).map_or(0, |&p| self.syms.tparam(p).variance);
            merged.push(match variance {
                1 if self.soft_lub => self.soft_lub(xs[i], ys[i]),
                1 => self.lub(xs[i], ys[i]),
                -1 => self.contravariant_meet(xs[i], ys[i]),
                _ if xs[i] == ys[i] => xs[i],
                // The join for member selection reads differing invariant arguments as a
                // bounded wildcard, dotc's `lubArgs`: `? >: glb <: lub` of the two, each read
                // through the bounds it may carry from an earlier join.
                _ if self.join_wild => self.wild_join(xs[i], ys[i]),
                _ => return None,
            });
        }
        let class = self.types.class(c, &merged);
        if px.is_none() && py.is_none() {
            return Some(class);
        }
        let Owner::Class(o) = self.syms.class(c).owner else { return Some(class) };
        let this = self.types.mk(Type::This(o));
        let (px, py) = (px.unwrap_or(this), py.unwrap_or(this));
        let prefix = if px == py { px } else { self.types.union(px, py) };
        Some(self.nested_type(prefix, class))
    }

    /// The meet of two contravariant arguments of a join: two by-name types meet in the by-name
    /// type of their meet (dotty's `glb` of two `ExprType`s), so that the join of `(=> T) => T`
    /// and `(=> Int) => Int` still takes its argument by name.
    fn contravariant_meet(&mut self, x: TypeId, y: TypeId) -> TypeId {
        if self.b.by_name.is_some() {
            if let (Some(a), Some(b)) = (self.by_name_arg(x), self.by_name_arg(y)) {
                let meet = self.types.inter(a, b);
                return self.by_name_type(meet);
            }
        }
        self.types.inter(x, y)
    }

    /// `? >: glb <: lub` of two invariant arguments: the narrower of the lower bounds where one
    /// conforms to the other, their intersection otherwise, and the wider of the upper bounds or
    /// their union (dotc's `lubArgs` for an invariant parameter).
    fn wild_join(&mut self, x: TypeId, y: TypeId) -> TypeId {
        let (xlo, xhi) = self.types.wild_bounds(x).unwrap_or((x, x));
        let (ylo, yhi) = self.types.wild_bounds(y).unwrap_or((y, y));
        let mark = self.snapshot();
        let lo = if self.is_sub(xlo, ylo) {
            xlo
        } else {
            self.rollback(mark);
            if self.is_sub(ylo, xlo) {
                ylo
            } else {
                self.rollback(mark);
                self.types.inter(xlo, ylo)
            }
        };
        let hi = self.soft_lub(xhi, yhi);
        self.types.bounded_wild(lo, hi)
    }

    /// The members of a union are those of its join, when the join is a single type. Every
    /// alternative is folded in: the members a union keeps apart (applications differing in an
    /// invariant argument) join here over a bounded wildcard.
    pub(super) fn union_join(&mut self, t: TypeId) -> Option<TypeId> {
        let Type::Union(..) = self.types.get(t) else { return None };
        let mut alts = Vec::new();
        self.union_alternatives(t, &mut alts);
        let outer = std::mem::replace(&mut self.join_wild, true);
        let outer_soft = std::mem::replace(&mut self.soft_lub, true);
        let mut j = alts[0];
        for &m in &alts[1..] {
            j = self.lub(j, m);
        }
        self.soft_lub = outer_soft;
        self.join_wild = outer;
        (!matches!(self.types.get(j), Type::Union(..))).then_some(j)
    }

    /// The least upper bound of two covariant arguments under a union's join: the wider of
    /// the two, or their union, as scalac's `lub` leaves unrelated types (`List[ValDef] |
    /// List[TypeDef]` joins to `List[ValDef | TypeDef]`).
    fn soft_lub(&mut self, a: TypeId, b: TypeId) -> TypeId {
        let mark = self.snapshot();
        if self.is_sub(a, b) {
            return b;
        }
        self.rollback(mark);
        if self.is_sub(b, a) {
            return a;
        }
        self.rollback(mark);
        self.types.soft_union(a, b)
    }

    /// Whether `t` conforms to `Singleton`, as scalac's stable types and those bounded by it
    /// do: a literal type, a path, `Nothing`, a parameter or member bounded by `Singleton`.
    fn is_singleton_type(&mut self, t: TypeId) -> bool {
        match self.types.get(t) {
            Type::Lit(_) | Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Nothing => true,
            Type::Class(c, _) => self.syms.class(c).kind == ClassKind::Object,
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                upper != ANY && self.bound_is_singleton(upper)
            }
            Type::Member(..) | Type::Decl(_) => {
                let upper = self.member_upper(t);
                upper != ANY && upper != t && self.bound_is_singleton(upper)
            }
            Type::Refined(p, _) => self.is_sub(p, self.b.t_singleton),
            Type::Rec(body) => self.is_sub(body, self.b.t_singleton),
            Type::Inter(x, y) => {
                let mark = self.trail.len();
                self.is_sub(x, self.b.t_singleton) || {
                    self.rollback(mark);
                    self.is_sub(y, self.b.t_singleton)
                }
            }
            Type::Union(x, y, _) => self.is_sub(x, self.b.t_singleton) && self.is_sub(y, self.b.t_singleton),
            _ => false,
        }
    }

    /// A bound followed from a parameter or a member is a lookup of its own, bounded as the
    /// member lookups are; the parts of one type are not.
    fn bound_is_singleton(&mut self, upper: TypeId) -> bool {
        if self.singleton_depth >= 32 {
            return false;
        }
        self.singleton_depth += 1;
        let holds = self.is_sub(upper, self.b.t_singleton);
        self.singleton_depth -= 1;
        holds
    }

    pub fn class_of(&mut self, t: TypeId) -> Option<ClassId> {
        self.class_of_within(t, CLASS_OF_DEPTH)
    }

    /// A bound that grows at every step (`M <: t.b.M & U` seen through `a.x.b.b...`) is
    /// followed a bounded number of times: each step is a lookup of its own, with a fresh budget.
    fn class_of_within(&mut self, t: TypeId, depth: u32) -> Option<ClassId> {
        let depth = depth.checked_sub(1)?;
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Class(c, _) => Some(c),
            Type::Match(..) | Type::Alias(..) => {
                let under = self.dependent_underlying(t)?;
                self.class_of_within(under, depth)
            }
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                if upper == ANY {
                    None
                } else {
                    self.class_of_within(upper, depth)
                }
            }
            Type::AppParam(p, args) => {
                let upper = self.app_param_upper(p, args)?;
                self.class_of_within(upper, depth)
            }
            Type::Inter(a, b) => self.class_of_within(a, depth).or_else(|| self.class_of_within(b, depth)),
            Type::Union(..) => {
                let j = self.union_join(t)?;
                self.class_of_within(j, depth)
            }
            Type::Lit(_) => {
                let class = self.widen_lit(t);
                self.class_of_within(class, depth)
            }
            Type::Nested(_, class) => self.class_of_within(class, depth),
            Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..) | Type::Rec(_) => {
                let under = self.dependent_underlying(t)?;
                self.class_of_within(under, depth)
            }
            _ => None,
        }
    }

    /// Instantiates a variable from its bounds. Returns false when the bounds conflict.
    pub fn solve_var(&mut self, v: TVarId) -> bool {
        self.solve_var_directed(v, None)
    }

    /// With the result type of the application, a variable that the result uses covariantly
    /// and that only other variables and `Any` bound from above keeps a choice of `Nothing`
    /// from its lower bounds, as scalac instantiates it from below: `LowerE` of zio's
    /// `@@[LowerE >: E, UpperE >: LowerE]` over a `Nothing`.
    pub fn solve_var_directed(&mut self, v: TVarId, result: Option<TypeId>) -> bool {
        if self.tvars[v].inst.is_some() || self.tvars[v].solving {
            return true;
        }
        self.tvars[v].solving = true;
        let ok = self.solve_var_inner(v, result);
        self.tvars[v].solving = false;
        ok
    }

    /// Numeric lower bounds of a parameter bounded by a union of numeric types join to the widest
    /// one, which is how the std spells what the JVM offers as numeric overloads (Math.min and
    /// friends); elsewhere mixed numeric types stay a union.
    fn harmonizes(&mut self, v: TVarId) -> bool {
        let uppers = self.tvars[v].upper.clone();
        uppers.into_iter().any(|u| self.is_numeric_union(u))
    }

    fn is_numeric_union(&mut self, t: TypeId) -> bool {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Union(a, b, _) => self.is_numeric_union(a) && self.is_numeric_union(b),
            _ => self.is_numeric(t).is_some(),
        }
    }

    fn solve_var_inner(&mut self, v: TVarId, result: Option<TypeId>) -> bool {
        // Nothing is known about it: the choice below would be `Nothing` after every step. Most
        // such variables are what a rolled-back candidate of a nested search left.
        if self.tvars[v].lower.is_empty() && self.tvars[v].upper.is_empty() {
            self.tvars[v].inst = Some(NOTHING);
            self.trail.push(Undo::Inst(v));
            self.drop_deps(v);
            return true;
        }
        // A bound that is the variable itself, left by another variable that was unified with
        // it and then instantiated to it, says nothing.
        let lowers = self.bounds_but_itself(v, true);
        let uppers = self.bounds_but_itself(v, false);
        // Bounded above and below by an open variable of an enclosing application, it is that
        // variable: `A` of `pure(x)` against a `Fr[B]`, which the other branches of an `if` join
        // into (`pure(Left(e))` and `pure(Right(r))` make `B` an `Either`, not a `Left`).
        for &l in &lowers {
            if let Some(w) = self.outer_var(v, l) {
                let w_ty = self.types.mk(Type::Var(w));
                if uppers.iter().any(|&u| self.deref(u) == w_ty) {
                    self.tvars[v].inst = Some(w_ty);
                    self.trail.push(Undo::Inst(v));
                    self.note_instance_deps(v, w_ty);
                    self.drop_deps(v);
                    return uppers.into_iter().all(|u| self.is_sub(w_ty, u));
                }
            }
        }
        let mut choice = NOTHING;
        let mut outer_lowers = Vec::new();
        for &l in &lowers {
            if let Some(w) = self.outer_var(v, l) {
                outer_lowers.push(w);
                continue;
            }
            let l = self.solve_own_in(v, l);
            if !matches!(self.types.get(l), Type::Var(_)) {
                choice = self.join_lower(v, choice, l);
            }
        }
        // An open variable of an enclosing application below the join stays open under it, and
        // is the choice itself when nothing else is known: `R1` of an inner `flatMap[R1 <: R]`
        // is then settled by the outer one.
        for w in outer_lowers {
            let w_ty = self.types.mk(Type::Var(w));
            if choice == NOTHING {
                choice = w_ty;
                continue;
            }
            if matches!(self.types.get(choice), Type::Var(_)) {
                choice = self.solve_in(choice);
            }
            let mark = self.snapshot();
            if !self.is_sub(w_ty, choice) {
                self.rollback(mark);
                let l = self.solve_in(w_ty);
                choice = self.join_lower(v, choice, l);
            }
        }
        // Lower bounds that amount to `Nothing` say nothing, as scalac reads them: the choice is
        // the upper bounds' then, where there are any. A kind alone, a type lambda ending in
        // `Any` (`F[X, Y <: X]`'s), is no upper bound (dotc's `hasUpperBound`, `isTopOfSomeKind`).
        if choice == NOTHING && !uppers.is_empty() {
            let kinds_only: Vec<TypeId>;
            let bounding: &[TypeId] = if uppers.iter().any(|&u| self.kind_alone(u)) {
                kinds_only = uppers.iter().copied().filter(|&u| !self.kind_alone(u)).collect();
                &kinds_only
            } else {
                &uppers
            };
            if !bounding.is_empty() && !result.map_or(false, |r| self.taken_from_below(v, bounding, r)) {
                choice = self.choice_from_uppers(v, bounding, false);
            }
        }
        // Bounded by nothing but itself (an inner application's variable unified with it) and
        // used only contravariantly in the result, it is maximised as an unconstrained one is:
        // `RIn2` of zio's `a ++ ZLayer.scoped(z)` is `Any`.
        if choice == NOTHING && lowers.is_empty() && uppers.is_empty() && result.map_or(false, |r| self.var_variances(r).iter().any(|&(w, s)| w == v && s == -1)) {
            choice = ANY;
        }
        // A join that escapes the upper bounds (e.g. `Any` for `Int | String`) gives way to the
        // bounds themselves; every lower bound was checked against them when it was recorded.
        let mark = self.snapshot();
        let fits = uppers.iter().all(|&u| self.is_sub(choice, u));
        let choice = if fits || uppers.is_empty() {
            choice
        } else {
            self.rollback(mark);
            self.choice_from_uppers(v, &uppers, true)
        };
        let fitted = fits.then_some(choice);
        // A union joined from several lower bounds is soft, as dotc's `lub` makes it, and
        // widens with the instance (`List(new Box[Int], new Box[String])`); a written one widens
        // its members' singletons (`widenUnion`).
        let choice = self.widen_inferred(choice, &uppers);
        // An F-bounded variable with nothing below it (`T <: Ord[T]`) is `Nothing`, never a
        // type that contains itself.
        let choice =
            if !uppers.is_empty() && self.types.has_vars(choice) && self.occurs(v, choice) { NOTHING } else { choice };
        // The choice that fitted the upper bounds, as widening left it, is not compared with them
        // again (dotty's `instantiateWith` compares nothing), unless a bound names the variable,
        // which reads its instance once it is made.
        let compared = fitted == Some(choice) && !uppers.iter().any(|&u| self.types.has_vars(u) && self.occurs(v, u));
        self.tvars[v].inst = Some(choice);
        self.trail.push(Undo::Inst(v));
        self.note_instance_deps(v, choice);
        self.drop_deps(v);
        compared || uppers.into_iter().all(|u| self.is_sub(choice, u))
    }

    /// Whether `t` is a type lambda ending in `Any`: a higher-kinded parameter's kind, bounding
    /// nothing (dotc's `isTopOfSomeKind`).
    fn kind_alone(&self, t: TypeId) -> bool {
        match self.types.get(t) {
            Type::Lambda(_, body) => body == ANY || self.kind_alone(body),
            _ => false,
        }
    }

    /// The lower (`lower`) or upper bounds of `v` but those that are `v` itself.
    fn bounds_but_itself(&mut self, v: TVarId, lower: bool) -> Vec<TypeId> {
        let n = if lower { self.tvars[v].lower.len() } else { self.tvars[v].upper.len() };
        let mut out = Vec::with_capacity(n);
        for k in 0..n {
            let b = if lower { self.tvars[v].lower[k] } else { self.tvars[v].upper[k] };
            if !self.is_var_itself(b, v) {
                out.push(b);
            }
        }
        out
    }

    /// Whether `t` is the open variable `v` (through instantiated ones).
    #[inline]
    fn is_var_itself(&mut self, t: TypeId, v: TVarId) -> bool {
        match self.types.get(t) {
            Type::Var(w) if w == v => true,
            Type::Var(_) | Type::AppVar(..) => {
                let t = self.deref_var(t);
                matches!(self.types.get(t), Type::Var(w) if w == v)
            }
            _ => false,
        }
    }

    fn taken_from_below(&mut self, v: TVarId, uppers: &[TypeId], result: TypeId) -> bool {
        let mut vars = false;
        for &u in uppers {
            let u = self.deref(u);
            match self.types.get(u) {
                Type::Var(_) => vars = true,
                _ if u == ANY => {}
                _ => return false,
            }
        }
        vars && self.var_variances(result).iter().any(|&(w, s)| w == v && s == 1)
    }

    pub(super) fn occurs(&self, v: TVarId, t: TypeId) -> bool {
        let mut vars = Vec::new();
        self.collect_vars(t, &mut vars);
        vars.contains(&v)
    }

    /// An instance widened as dotty's `widenInferred(inst, bound, Widen.Unions)` widens it, as
    /// long as it stays within the upper bounds: its singleton types (`widenSingle`, a soft union's
    /// members skipped), then a union by its members (`widenOr`: `widenUnion`, a soft union joined),
    /// so `id(c)` for a `c: 1` is an `Int` unless a `1` is expected of it, `id(own(x))` over `def
    /// own(x: B): x.type` a `B`, a join of `1` and `2` an `Int`, and a written `1 | "one"` an `Int |
    /// String`; an alias of a union is kept (`widen` does not dealias it).
    pub(super) fn widen_inferred(&mut self, choice: TypeId, uppers: &[TypeId]) -> TypeId {
        // A variable bounded by `Singleton` keeps its instance, as dotc's `isSingletonBounded`
        // does (`new A(r)` for `A[R <: Refl & Singleton]` is an `A[r.type]`).
        if self.widens_nothing(choice) || self.singleton_bounded(uppers) {
            return choice;
        }
        let single = self.widen_single(choice, true, uppers);
        self.widen_or(single, uppers)
    }

    /// Whether `widen_inferred` leaves `t` as it is: no singleton, no union at its head.
    #[inline]
    fn widens_nothing(&self, t: TypeId) -> bool {
        !self.types.is_path(t) && !matches!(self.types.get(t), Type::Lit(_) | Type::Union(..) | Type::Inter(..))
    }

    /// dotty's `widenSingle`: `t`'s singletons widened (`widenSingletons`), where that stays
    /// within the bounds.
    fn widen_single(&mut self, t: TypeId, skip_soft: bool, uppers: &[TypeId]) -> TypeId {
        let w = self.widen_singletons_of(t, skip_soft);
        if w == t {
            return t;
        }
        self.within_uppers(w, t, uppers)
    }

    /// `w` where it conforms to every upper bound, else `t`.
    fn within_uppers(&mut self, w: TypeId, t: TypeId, uppers: &[TypeId]) -> TypeId {
        let mark = self.snapshot();
        if uppers.iter().all(|&u| self.is_sub(w, u)) {
            w
        } else {
            self.rollback(mark);
            t
        }
    }

    /// dotty's `widenOr`: `t`'s `widenUnion`, a join that is transparent giving way to the soft
    /// union's members widened (`widenSingle(tp, skipSoftUnions = false)`: `'A' | Int` is a
    /// `Char | Int`).
    pub(super) fn widen_or(&mut self, t: TypeId, uppers: &[TypeId]) -> TypeId {
        let w = self.widen_union(t);
        if w == t {
            return t;
        }
        if self.transparent_join(w) {
            return self.widen_single(t, false, uppers);
        }
        self.within_uppers(w, t, uppers)
    }

    /// dotty's `widenSingletons`: a singleton widened to its underlying type, the parts of an
    /// intersection, the members of a soft union where they are not skipped (`OrType`'s
    /// `widenSingletons`, a hard one kept), read through an alias, which stays where nothing
    /// changes.
    fn widen_singletons_of(&mut self, t: TypeId, skip_soft: bool) -> TypeId {
        let d = self.deref_alias(t);
        if self.types.is_path(d) {
            return self.widen_path(d);
        }
        match self.types.get(d) {
            Type::Lit(_) => self.widen_lit(d),
            Type::Union(a, b, UnionKind::Soft) if !skip_soft => {
                let (wa, wb) = (self.widen_singletons_of(a, false), self.widen_singletons_of(b, false));
                if (wa, wb) == (a, b) { t } else { self.lub_as(UnionKind::Soft, wa, wb) }
            }
            Type::Inter(a, b) => {
                let (wa, wb) = (self.widen_singletons_of(a, skip_soft), self.widen_singletons_of(b, skip_soft));
                if (wa, wb) == (a, b) { t } else { self.types.inter(wa, wb) }
            }
            _ => t,
        }
    }

    /// dotty's `widenUnion`: a union, not through an alias (`widen` does not dealias), made again
    /// of its members widened (`widenUnionWithoutNull`), soft or hard as it was, a soft one
    /// joined (`OrType.join`); the parts of an intersection, a refinement's parent.
    pub fn widen_union(&mut self, t: TypeId) -> TypeId {
        if let Type::Alias(..) = self.types.get(t) {
            return t;
        }
        let t = self.deref(t);
        // A singleton widened through to its class (dotty's `widen`: `c.type` of a `c: 1` is an
        // `Int`).
        let t = if self.types.is_path(t) { let w = self.widen_path(t); self.widen_lit(w) } else { self.widen_lit(t) };
        match self.types.get(t) {
            Type::Union(_, _, UnionKind::Named) => t,
            Type::Union(_, _, kind) => {
                // The members widened, the union made again of them where one changed, and a soft
                // one joined once (dotty caches each nested `OrType`'s `widenUnion`; a nested
                // union is a member here, a named one an atom).
                let mut members = Vec::new();
                self.widened_members(t, &mut members);
                let changed = members.iter().any(|&(m, w)| m != w);
                // Members that widen to themselves: the union as it stands (its join keeps an
                // alias its members share, `IO[Either[String, Int]]`).
                let u = if !changed {
                    t
                } else {
                    let mut acc = members[0].1;
                    for &(_, w) in &members[1..] {
                        acc = self.lub_as(kind, acc, w);
                    }
                    acc
                };
                if kind == UnionKind::Soft && matches!(self.types.get(u), Type::Union(..)) {
                    match self.union_join(u) {
                        Some(j) if j != ERROR => j,
                        _ => u,
                    }
                } else {
                    u
                }
            }
            Type::Inter(a, b) => {
                let (wa, wb) = (self.widen_union(a), self.widen_union(b));
                if (wa, wb) == (a, b) { t } else { self.types.inter(wa, wb) }
            }
            _ => t,
        }
    }

    /// The members of a union and what each widens to (`widenUnionWithoutNull`): a singleton
    /// its class, an intersection its parts', a union nested in it its own members, a named one
    /// itself.
    fn widened_members(&mut self, t: TypeId, out: &mut Vec<(TypeId, TypeId)>) {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Union(a, b, k) if k != UnionKind::Named => {
                self.widened_members(a, out);
                self.widened_members(b, out);
            }
            _ => {
                let w = self.widen_union(t);
                out.push((t, w));
            }
        }
    }

    fn singleton_bounded(&mut self, uppers: &[TypeId]) -> bool {
        if self.b.t_singleton == ERROR {
            return false;
        }
        uppers.iter().any(|&u| {
            let u = self.deref(u);
            if u == ANY || matches!(self.types.get(u), Type::Var(_) | Type::AppVar(..) | Type::Error) {
                return false;
            }
            let mark = self.snapshot();
            let holds = self.is_sub(u, self.b.t_singleton);
            self.rollback(mark);
            holds
        })
    }

    fn join_lower(&mut self, v: TVarId, acc: TypeId, l: TypeId) -> TypeId {
        if acc == NOTHING {
            return l;
        }
        if let (Some(x), Some(y)) = (self.is_numeric(acc), self.is_numeric(l)) {
            if acc != l && self.harmonizes(v) {
                return self.rank_type(x.max(y).max(super::prims::R_INT));
            }
        }
        self.lub(acc, l)
    }

    /// The intersection of the upper bounds. An open variable of an enclosing application among
    /// them is left open, or is the choice when it is all there is, unless `force` solves it.
    fn choice_from_uppers(&mut self, v: TVarId, uppers: &[TypeId], force: bool) -> TypeId {
        let mut acc = ANY;
        let mut outer = Vec::new();
        for &u in uppers {
            match self.outer_var(v, u) {
                Some(w) if !force => outer.push(w),
                _ => {
                    let u = if force { self.solve_in(u) } else { self.solve_own_in(v, u) };
                    match self.types.get(u) {
                        // A bound being solved (`?fr <: ?R1` while `?R1` joins its lower bounds,
                        // `?fr` among them) offers what bounds it from above itself.
                        Type::Var(w) => {
                            let theirs: Vec<TypeId> = self.tvars[w].upper.clone();
                            for t in theirs {
                                let t = self.zonk(t);
                                if !self.types.has_vars(t) {
                                    acc = self.types.inter(acc, t);
                                }
                            }
                        }
                        _ => acc = self.types.inter(acc, u),
                    }
                }
            }
        }
        match outer.as_slice() {
            [w] if acc == ANY => self.types.mk(Type::Var(*w)),
            [_, _, ..] if acc == ANY => self.choice_from_uppers(v, uppers, true),
            _ => acc,
        }
    }

    /// Solves the variables of `t` that belong to the application of `v`, leaving those of the
    /// enclosing applications open inside it, as `T` in a bound `Page[T]` of an inner
    /// `List.empty[A]`: the enclosing call settles it once its own arguments are typed.
    fn solve_own_in(&mut self, v: TVarId, t: TypeId) -> TypeId {
        let t = self.zonk(t);
        if !self.types.has_vars(t) {
            return t;
        }
        let base = self.tvars[v].base();
        let mut vars = Vec::new();
        self.collect_vars(t, &mut vars);
        for w in vars {
            if w.index() as u32 >= base {
                self.solve_var(w);
            }
        }
        self.zonk(t)
    }

    /// A bound that is an open variable of an enclosing application: its owner solves it once
    /// all of its arguments are typed, so it is not forced here.
    fn outer_var(&mut self, v: TVarId, bound: TypeId) -> Option<TVarId> {
        let bound = self.deref(bound);
        match self.types.get(bound) {
            Type::Var(w) if (w.index() as u32) < self.tvars[v].base() => Some(w),
            _ => None,
        }
    }

    /// dotty's `Inferencing.instantiateTypeVars` over `vars`, the variables the tree of type `tp`
    /// expected to be `pt` made: one its type or expected type holds covariantly is instantiated
    /// from below, contravariantly from above, one it holds nonvariantly is left open for the trees
    /// around it (`open_vars`), and one it does not hold from below where something bounds it
    /// below and from above otherwise (`variances` decides, through the bounds). A variable from
    /// below waits for one below it, one from above for one above it (`doInstantiate`'s
    /// suspension). False where a variable's bounds conflict.
    pub(super) fn instantiate_vars(&mut self, vars: &[TVarId], var_mark: usize, tp: TypeId, pt: Option<TypeId>) -> bool {
        // Whether an open variable outside the tree's names one of them or is ordered against
        // one (`dependsOn`, either way): none, and the expected type (made before them) reaches
        // none of them through its variables' bounds, so their variances are the tree's type's
        // alone, read through the bounds of the variables made from `var_mark` on.
        let ordered = vars.iter().any(|&v| self.tvars[v].inst.is_none() && self.has_open_dep(v));
        let outside = ordered && vars.iter().any(|&v| self.tvars[v].inst.is_none() && (self.depends_on(v, vars, true) || self.depends_on(v, vars, false)));
        let vs = if outside { self.interp_variances(tp, pt, None) } else { self.interp_variances(tp, None, Some(var_mark)) };
        // A map where the variances are many; most types hold a few variables.
        let vmap: Option<crate::intern::FxMap<TVarId, i8>> = (vs.len() > 16).then(|| vs.iter().copied().collect());
        let variance = |v: TVarId| match &vmap {
            Some(m) => m.get(&v).copied(),
            None => vs.iter().find(|&&(w, _)| w == v).map(|&(_, s)| s),
        };
        let mut forced: Option<Vec<TVarId>> = None;
        let mut todo: Vec<(TVarId, i8)> = Vec::new();
        for &v in vars {
            if self.tvars[v].inst.is_some() {
                continue;
            }
            match variance(v) {
                // Neither held nor bounded (what a rolled-back candidate of a search left, most
                // often): from above, at once (`fromBelow` without a lower bound).
                None if self.tvars[v].lower.is_empty() && self.tvars[v].upper.is_empty() => self.instantiate(v, ANY),
                None => todo.push((v, 0)),
                Some(0) => {
                    let forced = forced.get_or_insert_with(|| self.match_alias_vars(tp));
                    if forced.contains(&v) {
                        todo.push((v, 0));
                    } else {
                        self.open_vars.push(v);
                        self.opened += 1;
                    }
                }
                Some(s) => todo.push((v, s)),
            }
        }
        if todo.is_empty() {
            return true;
        }
        // Nothing any of the tree's variables can narrow (the common application): no proposal is
        // dropped or turned, and none waits for another.
        let mut todo = if ordered { self.filter_by_deps(todo) } else { todo };
        if !ordered || !todo.iter().any(|&(v, _)| self.has_var_bound(v)) {
            let mut ok = true;
            for &(v, s) in &todo {
                if self.tvars[v].inst.is_none() {
                    let from_below = s == 1 || (s == 0 && self.below_first(v));
                    ok &= self.instantiate_from(v, from_below, tp);
                }
            }
            return ok;
        }
        let mut ok = true;
        while !todo.is_empty() {
            let mut suspended = Vec::new();
            for i in 0..todo.len() {
                let (v, s) = todo[i];
                if self.tvars[v].inst.is_some() {
                    continue;
                }
                let from_below = s == 1 || (s == 0 && self.below_first(v));
                if i + 1 < todo.len() && self.waits_for_later(&todo, i, from_below) {
                    suspended.push((v, s));
                    continue;
                }
                ok &= self.instantiate_from(v, from_below, tp);
            }
            todo = suspended;
        }
        ok
    }

    /// Whether a variable no variance decides is instantiated from below: where it is bounded
    /// below (`hasLowerBound`, `Nothing` none), and a partial unification's fixed argument where
    /// any lower bound is recorded, the argument it stands for (dotty's `canInstantiate` takes
    /// `ZIO[Log, Nothing, _]`'s `Nothing` as it is, which teq's `?e >: Nothing` stands for).
    fn below_first(&mut self, v: TVarId) -> bool {
        self.has_lower_bound(v) || (self.tvars[v].is_fixed() && !self.tvars[v].lower.is_empty())
    }

    /// A fresh variable for a fixed argument of a partial unification (`TVarExtra::fixed`).
    fn fixed_var(&mut self) -> TypeId {
        let t = self.fresh_var();
        if let Type::Var(v) = self.types.get(t) {
            self.tvars[v].set_fixed();
        }
        t
    }

    /// Whether a bound of `v` is an open variable (an ordering).
    fn has_var_bound(&mut self, v: TVarId) -> bool {
        let info = &self.tvars[v];
        if !info.lower.iter().chain(&info.upper).any(|&b| matches!(self.types.get(b), Type::Var(_) | Type::AppVar(..))) {
            return false;
        }
        let (nl, nu) = (self.tvars[v].lower.len(), self.tvars[v].upper.len());
        for k in 0..nl + nu {
            let b = if k < nl { self.tvars[v].lower[k] } else { self.tvars[v].upper[k - nl] };
            let b = self.deref(b);
            if matches!(self.types.get(b), Type::Var(_)) {
                return true;
            }
        }
        false
    }

    /// Whether the proposal `i`, instantiated from below (`from_below`) or above, waits for a
    /// later open one below it, or above it (`Constraint.isLess`, as a bound of either): dotty's
    /// `doInstantiate` suspension.
    fn waits_for_later(&mut self, todo: &[(TVarId, i8)], i: usize, from_below: bool) -> bool {
        let v = todo[i].0;
        let later = |t: &Self, w: TVarId| todo[i + 1..].iter().any(|&(x, _)| x == w) && t.tvars[w].inst.is_none();
        let own = if from_below { self.tvars[v].lower.len() } else { self.tvars[v].upper.len() };
        for k in 0..own {
            let b = if from_below { self.tvars[v].lower[k] } else { self.tvars[v].upper[k] };
            let b = self.deref(b);
            if let Type::Var(w) = self.types.get(b) {
                if later(self, w) {
                    return true;
                }
            }
        }
        let vt = self.types.mk(Type::Var(v));
        for &(w, _) in &todo[i + 1..] {
            if self.tvars[w].inst.is_some() {
                continue;
            }
            let theirs = if from_below { self.tvars[w].upper.len() } else { self.tvars[w].lower.len() };
            for k in 0..theirs {
                let b = if from_below { self.tvars[w].upper[k] } else { self.tvars[w].lower[k] };
                if self.deref(b) == vt {
                    return true;
                }
            }
        }
        false
    }

    /// The open variables made from `first` on, left open for the trees around (`open_vars`)
    /// without an interpolation of their own, but those a match alias holds in the result
    /// (`match_alias_vars`).
    pub(super) fn leave_vars_open(&mut self, first: usize, ret: TypeId) {
        let forced = self.match_alias_vars(ret);
        let from = self.open_vars.iter().rposition(|&w| !self.tvars.made_since(w, first)).map_or(0, |i| i + 1);
        let left: crate::intern::FxMap<TVarId, ()> = self.open_vars[from..].iter().map(|&w| (w, ())).collect();
        for i in first..self.tvars.len() {
            let info = &self.tvars[i];
            if info.inst.is_some() || info.guide {
                continue;
            }
            let v = self.tvars.id(i);
            if forced.contains(&v) || left.contains_key(&v) {
                continue;
            }
            self.open_vars.push(v);
            self.opened += 1;
        }
    }

    /// The variables of `tp` that a match alias's application holds (`M[?T]` of `type M[T] = T
    /// match ...`), which dotty's `simplify` of a tree's type makes fully defined before it
    /// normalizes the application (`TypeOps.simplify`, `isFullyDefined(tp, ForceDegree.all)` on
    /// an `isMatchAlias` application), so that an unreducible `M[?T]` is reduced over the
    /// instance (`M[Int]`); an application that reduces where the method is declared holds none.
    pub(super) fn match_alias_vars(&mut self, tp: TypeId) -> Vec<TVarId> {
        let mut out = Vec::new();
        self.collect_match_alias_vars(tp, &mut out);
        out
    }

    fn collect_match_alias_vars(&mut self, t: TypeId, out: &mut Vec<TVarId>) {
        let t = self.deref(t);
        if !self.types.has_vars(t) {
            return;
        }
        let parts: Vec<TypeId> = match self.types.get(t) {
            Type::Alias(a, _) if self.is_match_alias_application(a) => {
                let mut vars = Vec::new();
                self.collect_vars(t, &mut vars);
                for v in vars {
                    if !out.contains(&v) {
                        out.push(v);
                    }
                }
                return;
            }
            Type::Alias(_, _) => {
                let e = self.deref_alias(t);
                if e != t {
                    return self.collect_match_alias_vars(e, out);
                }
                match self.types.get(t) {
                    Type::Alias(_, args) => self.types.items(args).to_vec(),
                    _ => return,
                }
            }
            Type::Class(_, args) | Type::AppVar(_, args) | Type::AppParam(_, args) | Type::AppMember(_, args) => self.types.items(args).to_vec(),
            Type::Union(a, b, _) | Type::Inter(a, b) => vec![a, b],
            Type::Lambda(_, b) => vec![b],
            Type::Refined(p, _) => vec![p],
            _ => return,
        };
        for q in parts {
            self.collect_match_alias_vars(q, out);
        }
    }

    /// The argument of `S[arg]`, `scala.compiletime.ops.int`'s successor.
    fn successor_arg(&mut self, t: TypeId) -> Option<TypeId> {
        let Type::Class(c, args) = self.types.get(t) else { return None };
        let items = self.types.items(args);
        if items.len() != 1 || !self.types.is_op_class(c) || self.name_ref(self.syms.class(c).name) != "S" {
            return None;
        }
        Some(items[0])
    }

    /// dotty's `natValue`: the natural number a constant type of `Int` is.
    fn nat_value(&mut self, t: TypeId) -> Option<i32> {
        match self.fold_type(t) {
            Some(LitVal::Int(n)) if n >= 0 => Some(n),
            _ => None,
        }
    }

    /// dotty's `isMatchAlias`: an alias whose right-hand side is a match type, or a type lambda
    /// whose result is one.
    fn is_match_alias_application(&mut self, a: AliasId) -> bool {
        if !self.is_match_alias(a) {
            return false;
        }
        let body = self.alias_body(a);
        match self.types.get(body) {
            Type::Match(..) => true,
            Type::Lambda(_, b) => matches!(self.types.get(b), Type::Match(..)),
            _ => false,
        }
    }

    /// dotty's `filterByDeps`: a proposal that would narrow what an open variable outside the
    /// proposals allows is turned to the other side where it does not occur (`v == 0`), or
    /// dropped, the variable left open for the trees around: from above where one lies above it
    /// or holds it in its bounds so that a larger instance narrows it (`dependsOn(co = true)`),
    /// from below dually. Repeated with the smaller set until nothing changes.
    fn filter_by_deps(&mut self, todo: Vec<(TVarId, i8)>) -> Vec<(TVarId, i8)> {
        let mut todo = todo;
        loop {
            // The proposals' dependencies on one another do not count (`excluded`).
            let excluded: Vec<TVarId> = todo.iter().map(|&(v, _)| v).collect();
            let mut next = Vec::with_capacity(todo.len());
            for &(v, s) in &todo {
                let above_ok = !self.depends_on(v, &excluded, true);
                let below_ok = !self.depends_on(v, &excluded, false);
                let s = if s == 0 && !above_ok { 1 } else if s == 0 && !below_ok { -1 } else { s };
                if (s == -1 && !above_ok) || (s == 1 && !below_ok) {
                    continue;
                }
                next.push((v, s));
            }
            if next.len() == todo.len() {
                return next;
            }
            // The dropped ones stay open for the trees around.
            for &(v, _) in &todo {
                if !next.iter().any(|&(w, _)| w == v) && !self.open_vars.contains(&v) {
                    self.open_vars.push(v);
                    self.opened += 1;
                }
            }
            todo = next;
        }
    }

    /// `v` instantiated from its lower bounds (teq's solver, which widens as `instanceType` does;
    /// `Nothing` where nothing bounds it below) or from its upper ones (their intersection, `Any`
    /// without any).
    fn instantiate_from(&mut self, v: TVarId, from_below: bool, tp: TypeId) -> bool {
        if from_below {
            if !self.has_lower_bound(v) {
                self.instantiate(v, NOTHING);
                let uppers: Vec<TypeId> = self.tvars[v].upper.clone();
                return uppers.into_iter().all(|u| self.is_sub(NOTHING, u));
            }
            return self.solve_var_directed(v, Some(tp));
        }
        let uppers = self.bounds_but_itself(v, false);
        if uppers.is_empty() {
            self.instantiate(v, ANY);
            return true;
        }
        // An upper bound that holds the variable itself (`T <: Ord[T]`) is no instance: from below.
        let zonked: Vec<TypeId> = uppers.iter().map(|&u| self.zonk(u)).collect();
        if zonked.iter().any(|&u| self.occurs(v, u)) {
            return self.solve_var_directed(v, Some(tp));
        }
        // The meet of the upper bounds as they stand, open variables included (`approximation`'s
        // `fullUpperBound`, `_ & _`), which the instantiation of the rest settles; every lower
        // bound was checked against them as it was recorded (`add_lower`).
        let choice = zonked.into_iter().fold(ANY, |acc, u| self.glb(acc, u));
        self.instantiate(v, choice);
        true
    }

    /// dotty's `TypeComparer.glb`: the meet of `a` and `b`, a part of either that is a supertype
    /// of the other dropped (`dropIfSuper`, a frozen comparison; a part the other holds as a part
    /// is dropped without one), so `R & R` is `R` and `Bus & (Clock & Bus)` is `Clock & Bus`.
    /// `a`'s parts are dropped first, then `b`'s against what is left of `a` (`mergedGlb`).
    pub(super) fn glb(&mut self, a: TypeId, b: TypeId) -> TypeId {
        let (a, b) = (self.deref(a), self.deref(b));
        if a == b || b == ANY {
            return a;
        }
        if a == ANY {
            return b;
        }
        if a == ERROR || b == ERROR {
            return ERROR;
        }
        if a == NOTHING || b == NOTHING {
            return NOTHING;
        }
        let mut pa = Vec::new();
        self.inter_parts(a, &mut pa);
        let mut pb = Vec::new();
        self.inter_parts(b, &mut pb);
        let mut ka = Vec::with_capacity(pa.len());
        for &p in &pa {
            if !pb.contains(&p) && !self.frozen_sub(b, p) {
                ka.push(p);
            }
        }
        if ka.is_empty() {
            return b;
        }
        let a2 = if ka.len() == pa.len() { a } else { ka.iter().copied().reduce(|x, y| self.types.inter(x, y)).unwrap() };
        let mut kb = Vec::with_capacity(pb.len());
        for &q in &pb {
            if !ka.contains(&q) && !self.frozen_sub(a2, q) {
                kb.push(q);
            }
        }
        if kb.is_empty() {
            return a2;
        }
        let b2 = if kb.len() == pb.len() { b } else { kb.iter().copied().reduce(|x, y| self.types.inter(x, y)).unwrap() };
        self.types.inter(a2, b2)
    }

    /// The parts of the intersection `t`, in order; `t` itself where it is none.
    fn inter_parts(&mut self, t: TypeId, out: &mut Vec<TypeId>) {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Inter(x, y) => {
                self.inter_parts(x, out);
                self.inter_parts(y, out);
            }
            _ => out.push(t),
        }
    }

    /// Whether `part` is `t` or one of the parts of the intersection `t`.
    fn is_part_of_inter(&mut self, part: TypeId, t: TypeId) -> bool {
        let t = self.deref(t);
        if t == part {
            return true;
        }
        match self.types.get(t) {
            Type::Inter(a, b) => self.is_part_of_inter(part, a) || self.is_part_of_inter(part, b),
            _ => false,
        }
    }

    /// Whether something other than `Any` bounds `v` above (`TypeVar.hasUpperBound`).
    fn has_upper_bound(&mut self, v: TVarId) -> bool {
        for k in 0..self.tvars[v].upper.len() {
            let u = self.tvars[v].upper[k];
            let u = self.deref(u);
            if u != ANY && !matches!(self.types.get(u), Type::Var(w) if w == v) {
                return true;
            }
        }
        false
    }

    /// `t` with its open variables instantiated as dotty's `IsFullyDefinedAccumulator` does under
    /// `ForceDegree.all`, at a definition's inferred type (`Namer.inferredResultType`'s
    /// `fullyDefinedType`) or a class's parent, by `instDecision`: one something bounds below
    /// from below unless held contravariantly; one nothing bounds below from below where held
    /// covariantly, or nonvariantly with nothing above it; the others from above, after the
    /// rest, where `instDirection` (the side the constraint narrowed beyond the declared bounds)
    /// does not decide first.
    pub fn fully_defined(&mut self, t: TypeId) -> TypeId {
        let t = self.zonk(t);
        if !self.types.has_vars(t) {
            return t;
        }
        self.define_vars_in(t, None);
        self.solve_in(t)
    }

    /// dotty's `Inferencing.instDirection`: 1 where the constraint bounds `v` from above beyond
    /// the bounds its parameter declares and not from below, -1 the other way round, 0 where
    /// neither or both (each compared frozen, the lower bounds' join against the declared lower
    /// bound, the declared upper bound against each upper one, an ordering among them).
    fn inst_direction(&mut self, v: TVarId) -> i8 {
        let (lo, hi) = self.tvars[v].declared();
        let mut below = 0;
        for k in 0..self.tvars[v].lower.len() {
            let l = self.tvars[v].lower[k];
            if !self.is_var_itself(l, v) && !self.frozen_sub(l, lo) {
                below = 1;
                break;
            }
        }
        let mut above = 0;
        for k in 0..self.tvars[v].upper.len() {
            let u = self.tvars[v].upper[k];
            if !self.is_var_itself(u, v) && !self.frozen_sub(hi, u) {
                above = 1;
                break;
            }
        }
        above - below
    }

    /// `IsFullyDefinedAccumulator`'s `ForceDegree.all` over the open variables of `t` (made from
    /// `since` on, where given): `instDecision` of each, by `instDirection` first, the maximised
    /// ones by their variance (`ToMax`) last.
    fn define_vars_in(&mut self, t: TypeId, since: Option<usize>) {
        let mut maximize = Vec::new();
        for (v, s) in self.var_variances(t) {
            if self.tvars[v].inst.is_some() || since.is_some_and(|m| !self.tvars.made_since(v, m)) {
                continue;
            }
            let direction = self.inst_direction(v);
            if direction != 0 {
                self.instantiate_from(v, direction < 0, t);
                continue;
            }
            let min = if self.has_lower_bound(v) { s >= 0 } else { s > 0 || s == 0 && !self.has_upper_bound(v) };
            if min {
                self.instantiate_from(v, true, t);
            } else {
                maximize.push(v);
            }
        }
        for v in maximize {
            if self.tvars[v].inst.is_none() {
                self.instantiate_from(v, false, t);
            }
        }
    }

    /// dotty's `canDefineFurther`: the open variables of `t` instantiated as `isFullyDefined` does
    /// under `ForceDegree.failBottom`, true where every one could be and one was; none of them
    /// where one held covariantly or nonvariantly has nothing below it.
    pub fn can_define_further(&mut self, t: TypeId) -> bool {
        let t = self.zonk(t);
        if !self.types.has_vars(t) {
            return false;
        }
        let mark = self.snapshot();
        let mut maximize = Vec::new();
        let mut changed = false;
        for (v, s) in self.var_variances(t) {
            if self.tvars[v].inst.is_some() {
                continue;
            }
            let direction = self.inst_direction(v);
            if direction != 0 {
                self.instantiate_from(v, direction < 0, t);
                changed = true;
                continue;
            }
            if self.has_lower_bound(v) {
                if s >= 0 {
                    self.instantiate_from(v, true, t);
                    changed = true;
                } else {
                    maximize.push(v);
                }
            } else if s >= 0 {
                self.rollback(mark);
                return false;
            } else {
                maximize.push(v);
            }
        }
        for v in maximize {
            if self.tvars[v].inst.is_none() {
                self.instantiate_from(v, false, t);
                changed = true;
            }
        }
        changed
    }

    /// Whether something other than `Nothing` bounds `v` below (`TypeVar.hasLowerBound`).
    fn has_lower_bound(&mut self, v: TVarId) -> bool {
        let info = &self.tvars[v];
        let mut heads = false;
        for &l in &info.lower {
            match self.types.get(l) {
                Type::Var(_) | Type::AppVar(..) => heads = true,
                _ if l != NOTHING => return true,
                _ => {}
            }
        }
        if !heads {
            return false;
        }
        for k in 0..self.tvars[v].lower.len() {
            let l = self.tvars[v].lower[k];
            if self.deref(l) != NOTHING {
                return true;
            }
        }
        false
    }

    /// dotty's `Inferencing.variances`: the variances of the open variables of `tp` and of the
    /// expected type `pt`, and through the bounds of each the variables that decide its
    /// instance: below a covariant one at its variance, above a contravariant one, both ways for a
    /// nonvariant one, until nothing changes (`propagate`).
    pub(super) fn interp_variances(&mut self, tp: TypeId, pt: Option<TypeId>, within: Option<usize>) -> Vec<(TVarId, i8)> {
        let mut out = self.var_variances(tp);
        if let Some(pt) = pt {
            if pt != ERROR {
                self.collect_var_variances(pt, 1, &mut out);
            }
        }
        // The variance each entry's bounds were read at (2: unread), on the stack for the few a
        // type holds; one whose variance changed since (to nonvariant) is read again. Under
        // `within`, only the bounds of the variables made from it on are read.
        let mut read_small = [2i8; 16];
        let mut read_big: Vec<i8> = Vec::new();
        loop {
            let mut changed = false;
            let mut i = 0;
            while i < out.len() {
                let (v, s) = out[i];
                let was = if i < 16 {
                    std::mem::replace(&mut read_small[i], s)
                } else {
                    if read_big.len() <= i - 16 {
                        read_big.resize(i - 15, 2);
                    }
                    std::mem::replace(&mut read_big[i - 16], s)
                };
                if was == s || self.tvars[v].inst.is_some() || within.is_some_and(|m| !self.tvars.made_since(v, m)) {
                    i += 1;
                    continue;
                }
                changed = true;
                // A bound that mentions no variable holds none (most).
                if s >= 0 {
                    for k in 0..self.tvars[v].lower.len() {
                        let l = self.tvars[v].lower[k];
                        if self.types.has_vars(l) {
                            self.collect_var_variances(l, s, &mut out);
                        }
                    }
                }
                if s <= 0 {
                    for k in 0..self.tvars[v].upper.len() {
                        let u = self.tvars[v].upper[k];
                        if self.types.has_vars(u) {
                            self.collect_var_variances(u, s, &mut out);
                        }
                    }
                }
                i += 1;
            }
            if !changed {
                return out;
            }
        }
    }

    /// The tree of type `ty` expected to be `expected`, whose typing made the variables from
    /// `mark` on, interpolates those an inner interpolation left open (`Typer.simplify`'s
    /// `interpolateTypeVars`): its type constrained by the expected one first where that holds
    /// (`constrainResult`, which can bound a variable by a singleton), then instantiated as
    /// `instantiate_vars` says.
    #[inline(never)]
    pub(super) fn interpolate_open(&mut self, mark: usize, ty: TypeId, expected: Option<TypeId>) {
        let start = self.open_vars.iter().rposition(|&v| !self.tvars.made_since(v, mark)).map_or(0, |i| i + 1);
        let vars: Vec<TVarId> = self.open_vars.drain(start..).filter(|&v| self.tvars[v].inst.is_none()).collect();
        if vars.is_empty() || ty == ERROR {
            self.open_vars.extend(vars);
            return;
        }
        if let Some(exp) = self.concrete_expected(expected) {
            if exp != self.b.t_unit && exp != ANY && exp != ERROR {
                let snap = self.snapshot();
                if !self.is_sub(ty, exp) {
                    self.rollback(snap);
                }
            }
        }
        self.instantiate_vars(&vars, mark, ty, expected);
    }

    /// A lambda's body, of type `ty`, typed: the variables its typing made from `mark` on are
    /// instantiated, as dotty's closure is an anonymous method whose result type is inferred
    /// fully defined (`Namer.inferredResultType`): those `ty` holds as `fully_defined` says,
    /// the others by their bounds. Its parameters' paths do not reach the trees around.
    pub(super) fn define_body_vars(&mut self, mark: usize, ty: TypeId) -> TypeId {
        if !self.open_vars.last().is_some_and(|&v| self.tvars.made_since(v, mark)) {
            return ty;
        }
        let zonked = self.zonk(ty);
        self.define_vars_in(zonked, Some(mark));
        self.force_open_vars(mark);
        self.zonk(ty)
    }

    /// The recorded types of the nodes from `from` on, zonked: a variable they name that an
    /// interpolation left open is instantiated by now.
    pub(super) fn settle_node_types(&mut self, from: usize) {
        if !self.prog.record_types {
            return;
        }
        for i in from..self.prog.exprs.len() {
            let te = crate::tir::TExprId(i as u32);
            if let Some(t) = self.prog.type_of(te) {
                if self.types.has_vars(t) {
                    let z = self.zonk(t);
                    if z != t {
                        self.prog.set_type(te, z);
                    }
                }
            }
        }
    }

    /// The definition whose typing made the variables from `start` on is typed: what is still
    /// open is instantiated by its bounds (`fullyDefinedType`'s `ForceDegree.all`).
    pub(super) fn force_open_vars(&mut self, start: usize) {
        if !self.open_vars.last().is_some_and(|&v| self.tvars.made_since(v, start)) {
            return;
        }
        let from = self.open_vars.iter().rposition(|&v| !self.tvars.made_since(v, start)).map_or(0, |i| i + 1);
        let vars: Vec<TVarId> = self.open_vars.drain(from..).collect();
        for v in vars {
            if self.tvars[v].inst.is_none() {
                let from_below = self.has_lower_bound(v);
                self.instantiate_from(v, from_below, ANY);
            }
        }
    }

    /// Solves every variable mentioned in `t` and returns the variable-free type.
    pub fn solve_in(&mut self, t: TypeId) -> TypeId {
        let t = self.zonk(t);
        if !self.types.has_vars(t) {
            return t;
        }
        let mut vars = Vec::new();
        self.collect_vars(t, &mut vars);
        for v in vars {
            self.solve_var(v);
        }
        self.zonk(t)
    }

    /// A receiver whose type is still an open variable that something bounds below, as an
    /// operator finds no member on a variable's upper bound: instantiated from below, as dotty's
    /// selection retries with `couldInstantiateTypeVar` (`Typer.tryInstantiateTypeVar`), so
    /// `it + 1` of `fun(it + 1)` over a `Ctx[T] ?=> U` takes `T`'s `Int` (i10082).
    #[inline]
    pub(super) fn instantiate_receiver_var(&mut self, t: TypeId) -> TypeId {
        if self.types.has_vars(t) && self.could_instantiate_var(t, false) { self.zonk(t) } else { t }
    }

    /// dotty's `couldInstantiateTypeVar`: an open variable bounded below, instantiated from below,
    /// at the head of `t`, of an application's constructor (`?G[Unit]`, its arguments too) or of
    /// a match alias's arguments, of a refinement's parent, or of either side of a union or an
    /// intersection. Whether one was.
    pub(super) fn could_instantiate_var(&mut self, t: TypeId, applied: bool) -> bool {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Var(v) => {
                if !self.has_lower_bound(v) {
                    return false;
                }
                self.instantiate_from(v, true, t);
                true
            }
            Type::AppVar(v, args) => {
                let head = self.types.mk(Type::Var(v));
                self.could_instantiate_var(head, true) || self.types.items(args).to_vec().into_iter().any(|a| self.could_instantiate_var(a, true))
            }
            Type::Alias(a, args) if self.is_match_alias_application(a) => self.types.items(args).to_vec().into_iter().any(|a| self.could_instantiate_var(a, applied)),
            Type::Class(_, args) | Type::AppParam(_, args) | Type::AppMember(_, args) | Type::Alias(_, args) if applied => {
                self.types.items(args).to_vec().into_iter().any(|a| self.could_instantiate_var(a, applied))
            }
            Type::Refined(p, _) => self.could_instantiate_var(p, applied),
            Type::Union(a, b, _) | Type::Inter(a, b) => self.could_instantiate_var(a, applied) || self.could_instantiate_var(b, applied),
            _ => false,
        }
    }

    /// Solves the variables in `t` that already have bounds; unconstrained ones stay open so
    /// that an implicit search can still determine them.
    pub fn solve_bounded_in(&mut self, t: TypeId) -> TypeId {
        let t = self.zonk(t);
        if !self.types.has_vars(t) {
            return t;
        }
        let mut vars = Vec::new();
        self.collect_vars(t, &mut vars);
        for v in vars {
            let info = &self.tvars[v];
            if info.inst.is_none() && !(info.lower.is_empty() && info.upper.is_empty()) {
                self.solve_var(v);
            }
        }
        self.zonk(t)
    }

    /// Solves `t` as the parameter type of a function value. A variable nothing is known about
    /// becomes `Any` there, as in Scala: `Nothing` would leave a function that takes no argument.
    pub fn solve_maximized_in(&mut self, t: TypeId) -> TypeId {
        let t = self.zonk(t);
        if self.types.has_vars(t) {
            self.maximize_unbounded_vars(t);
        }
        self.solve_in(t)
    }

    fn maximize_unbounded_vars(&mut self, t: TypeId) {
        match self.types.get(t) {
            Type::Var(v) => {
                let info = &self.tvars[v];
                if info.inst.is_none() && info.lower.is_empty() && info.upper.is_empty() {
                    self.instantiate(v, ANY);
                }
            }
            Type::AppVar(_, args) | Type::Class(_, args) | Type::AppParam(_, args) => {
                for a in self.types.items(args).to_vec() {
                    self.maximize_unbounded_vars(a);
                }
            }
            Type::Lambda(_, b) => self.maximize_unbounded_vars(b),
            Type::Union(a, b, _) | Type::Inter(a, b) => {
                self.maximize_unbounded_vars(a);
                self.maximize_unbounded_vars(b);
            }
            _ => {}
        }
    }

    /// Solves the variables of `selected` that `t` mentions.
    pub fn solve_selected_in(&mut self, t: TypeId, selected: &[TVarId]) {
        if selected.is_empty() || !self.types.has_vars(t) {
            return;
        }
        let mut vars = Vec::new();
        self.collect_vars(t, &mut vars);
        for v in vars {
            if selected.contains(&v) {
                self.solve_var(v);
            }
        }
    }

    /// The variables of `t` with the variance each occurs with: `1` or `-1` when every
    /// occurrence agrees, `0` otherwise. One walk answers for every variable, where a walk per
    /// variable had cost an application as many walks as the variables its nested searches and
    /// expansions left behind.
    pub fn var_variances(&mut self, t: TypeId) -> Vec<(TVarId, i8)> {
        let mut out = Vec::new();
        self.collect_var_variances(t, 1, &mut out);
        out
    }

    fn collect_var_variances(&mut self, t: TypeId, sign: i8, out: &mut Vec<(TVarId, i8)>) {
        // A part that mentions no variable holds none (the store's flag, read once).
        if !self.types.has_vars(t) {
            return;
        }
        let t = self.deref(t);
        let note = |out: &mut Vec<(TVarId, i8)>, v: TVarId, sign: i8| match out.iter_mut().find(|(w, _)| *w == v) {
            Some((_, s)) if *s != sign => *s = 0,
            Some(_) => {}
            None => out.push((v, sign)),
        };
        match self.types.get(t) {
            Type::Var(v) => note(out, v, sign),
            Type::Class(c, args) => {
                self.settle_class(c);
                for i in 0..self.types.items(args).len() {
                    let a = self.types.items(args)[i];
                    if !self.types.has_vars(a) {
                        continue;
                    }
                    let declared = self.syms.class(c).tparams.get(i).map_or(0, |&p| self.syms.tparam(p).variance);
                    let inner = match declared {
                        1 => sign,
                        -1 => -sign,
                        _ => 0,
                    };
                    self.collect_var_variances(a, inner, out);
                }
            }
            Type::AppVar(w, args) => {
                note(out, w, sign);
                let items = self.types.items(args).to_vec();
                for a in items {
                    self.collect_var_variances(a, 0, out);
                }
            }
            Type::AppParam(_, args) => {
                let items = self.types.items(args).to_vec();
                for a in items {
                    self.collect_var_variances(a, 0, out);
                }
            }
            // An abstract type member's arguments at its declared variances (`type T[+A]`), as a
            // class's; an argument it declares none for, or of a member not found, is invariant.
            Type::AppMember(m, args) => {
                let m = self.deref(m);
                let variances = self.member_variances(m);
                let items = self.types.items(args).to_vec();
                for (i, a) in items.into_iter().enumerate() {
                    let inner = match variances.get(i).copied().unwrap_or(0) {
                        1 => sign,
                        -1 => -sign,
                        _ => 0,
                    };
                    self.collect_var_variances(a, inner, out);
                }
            }
            Type::Lambda(_, b) => self.collect_var_variances(b, sign, out),
            Type::Union(a, b, _) | Type::Inter(a, b) => {
                self.collect_var_variances(a, sign, out);
                self.collect_var_variances(b, sign, out);
            }
            Type::Alias(..) => {
                let e = self.deref_alias(t);
                if e != t {
                    self.collect_var_variances(e, sign, out);
                }
            }
            // A refinement's members count as the member types they are: a `val` and a method's
            // result where the refinement stands, a method's parameters and a lower bound the
            // other way, an alias both ways.
            Type::Refined(parent, r) => {
                self.collect_var_variances(parent, sign, out);
                match self.types.refinement(r) {
                    Refinement::Alias(_, ty) => self.collect_var_variances(ty, 0, out),
                    Refinement::Bounds(_, lo, hi) => {
                        self.collect_var_variances(lo, -sign, out);
                        self.collect_var_variances(hi, sign, out);
                    }
                    Refinement::Val(_, _, ty) => self.collect_var_variances(ty, sign, out),
                    Refinement::Term(_, _, l) => {
                        let items = self.types.items(l).to_vec();
                        if let Some((&ret, params)) = items.split_last() {
                            for &p in params {
                                self.collect_var_variances(p, -sign, out);
                            }
                            self.collect_var_variances(ret, sign, out);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Whether `t` names a type variable made at `from` or later.
    #[cfg(debug_assertions)]
    pub fn mentions_var_from(&self, t: TypeId, from: u32) -> bool {
        let mut vars = Vec::new();
        self.collect_vars(t, &mut vars);
        vars.iter().any(|v| v.tag() != 0 || v.index() as u32 >= from)
    }

    pub fn collect_vars(&self, t: TypeId, out: &mut Vec<TVarId>) {
        match self.types.get(t) {
            Type::Var(v) => out.push(v),
            Type::AppVar(v, args) => {
                out.push(v);
                for &a in self.types.items(args) {
                    self.collect_vars(a, out);
                }
            }
            Type::Class(_, args) | Type::AppParam(_, args) | Type::Alias(_, args) => {
                for &a in self.types.items(args) {
                    self.collect_vars(a, out);
                }
            }
            Type::Lambda(_, b) => self.collect_vars(b, out),
            Type::Union(a, b, _) | Type::Inter(a, b) => {
                self.collect_vars(a, out);
                self.collect_vars(b, out);
            }
            Type::Select(p, _) | Type::Member(p, _) => self.collect_vars(p, out),
            Type::AppMember(m, args) => {
                self.collect_vars(m, out);
                for &a in self.types.items(args) {
                    self.collect_vars(a, out);
                }
            }
            Type::Refined(p, r) => {
                self.collect_vars(p, out);
                match self.types.refinement(r) {
                    Refinement::Alias(_, rhs) => self.collect_vars(rhs, out),
                    Refinement::Bounds(_, lo, hi) => {
                        self.collect_vars(lo, out);
                        self.collect_vars(hi, out);
                    }
                    Refinement::Val(_, _, ty) => self.collect_vars(ty, out),
                    Refinement::Term(_, _, l) => {
                        for a in self.types.items(l).to_vec() {
                            self.collect_vars(a, out);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Finds a declared member by name, returning the symbol and the owner type as seen from `t`.
    #[inline]
    /// `a`, an application of class `c` to `cargs`, as the application `b` of the variable `v` to
    /// `n` arguments: `v` the constructor of `c` itself, or of `c` with its leftmost arguments
    /// fixed (partial unification), within the variable's higher-kinded bounds.
    #[allow(clippy::too_many_arguments)]
    fn app_var_instance(&mut self, a: TypeId, b: TypeId, v: TVarId, c: ClassId, cargs: TList, n: usize, var_below: bool) -> bool {
        let m = self.types.items(cargs).len();
        let ctor = if n == m {
            self.types.mk(Type::Ctor(c))
        } else {
            // Partial unification: the leftmost arguments are fixed, the rest abstracted
            // over. Each fixed argument is an open variable the class's own argument
            // bounds, so that later operands join it as scalac joins the constructors
            // it infers: `(Validated[E1, A], Validated[E2, B])` as `(F[A], F[B])` makes
            // `F` a `Validated[E1 | E2, _]`.
            let fixed: Vec<TypeId> = (0..m - n).map(|_| self.fixed_var()).collect();
            let params: Vec<TypeId> = (0..n)
                .map(|_| {
                    let p = self.syms.new_tparam(crate::names::WILDCARD, 0);
                    self.types.param(p)
                })
                .collect();
            let mut full = fixed;
            full.extend(params.iter().copied());
            let body = self.types.class(c, &full);
            let pl = self.types.list(&params);
            self.types.mk(Type::Lambda(pl, body))
        };
        // The constructor keeps the variable's higher-kinded bounds (`T[X] <: Iterable[X]`
        // of a given over `T[A]`), which rule a `Box` out.
        let uppers: Vec<TypeId> = self.tvars[v].upper.clone();
        if !uppers.into_iter().all(|u| self.is_sub(ctor, u)) {
            return false;
        }
        if var_below {
            let lowers: Vec<TypeId> = self.tvars[v].lower.clone();
            if !lowers.into_iter().all(|l| self.is_sub(l, ctor)) {
                return false;
            }
        }
        self.instantiate(v, ctor);
        self.is_sub(a, b)
    }

    pub fn find_member(&mut self, t: TypeId, name: Name) -> Option<(SymId, TypeId)> {
        let d = self.deref(t);
        if let Type::Class(c, args) = self.types.get(d) {
            return self.find_class_member(d, c, args, name, 1);
        }
        self.find_member_from(t, t, name, 1, &mut None)
    }

    /// The member `name` of `t` selected on a receiver whose type dotty's typer leaves as `pre`
    /// (`written_prefix`), the members of an intersection met against `pre`'s base classes,
    /// the joint information the meet gives it in `joint`: what a selection types the member by.
    #[inline]
    pub fn find_member_joint(&mut self, pre: TypeId, t: TypeId, name: Name, joint: &mut JointInfo) -> Option<(SymId, TypeId)> {
        let d = self.deref(t);
        if let Type::Class(c, args) = self.types.get(d) {
            return self.find_class_member(d, c, args, name, 1);
        }
        self.find_member_from(pre, t, name, 1, joint)
    }

    /// The type dotty's typer gives the qualifier `e` as written, typed as `te` of type `ty`,
    /// where teq's typing of a branching expression keeps the join of its branches:
    /// `TypeAssigner`'s rules by the written tree (`TypeAssigner.scala` at the 3.8.4 tag), an
    /// `if` the union of its branches' types (`thenp.tpe | elsep.tpe`, 428), a match its cases',
    /// a block its expression's, read through the typed nodes that correspond; any other tree
    /// the type it was typed as, an ascription its ascribed type (`Typed`, 413). A selection on
    /// it searches the join, which dotty's `findMember` searches too (`goOr`), with this as the
    /// prefix whose base classes an intersection's members meet against.
    pub fn written_prefix(&mut self, e: crate::ast::ExprId, te: TExprId, ty: TypeId) -> TypeId {
        self.written_type(e, te, 0).unwrap_or(ty)
    }

    fn written_type(&mut self, e: crate::ast::ExprId, te: TExprId, depth: u32) -> Option<TypeId> {
        use crate::ast::{Expr, Stmt};
        if depth > 8 {
            return None;
        }
        let ast = self.cur_ast();
        let branches: Vec<(crate::ast::ExprId, TExprId)> = match (ast.expr(e), self.prog.expr(te)) {
            (Expr::If(_, t, Some(f)), TExpr::If(_, tt, Some(tf))) => vec![(t, tt), (f, tf)],
            (Expr::Match(_, cases), TExpr::Match(_, tcases)) => {
                let bodies: Vec<crate::ast::ExprId> = ast.case_list(cases).iter().map(|c| c.body).collect();
                let typed: Vec<TExprId> = self.prog.case_list(tcases).iter().map(|c| c.body).collect();
                if bodies.len() != typed.len() {
                    return None;
                }
                bodies.into_iter().zip(typed).collect()
            }
            (Expr::Block(stmts), TExpr::Block(_, res)) => match ast.stmt_list(stmts).last() {
                Some(&Stmt::Expr(r)) => return self.written_type(r, res, depth + 1),
                _ => return None,
            },
            _ => return None,
        };
        let mut acc: Option<TypeId> = None;
        for (b, tb) in branches {
            let t = match self.written_type(b, tb, depth + 1) {
                Some(t) => t,
                None => self.prog.type_of(tb)?,
            };
            let d = self.deref(t);
            if matches!(self.types.get(d), Type::Nothing) {
                continue;
            }
            acc = Some(match acc {
                Some(a) if a == t => a,
                Some(a) => self.types.soft_union(a, t),
                None => t,
            });
        }
        acc
    }

    /// The member that is left when the private one of the class itself cannot be accessed: a
    /// constructor parameter may share its name with a member the class inherits.
    pub fn find_inherited_member(&mut self, t: TypeId, name: Name) -> Option<(SymId, TypeId)> {
        self.find_member_from(t, t, name, 0, &mut None)
    }

    fn find_member_in_bound(&mut self, pre: TypeId, p: TParamId, upper: TypeId, name: Name, private_until: usize, joint: &mut JointInfo) -> Option<(SymId, TypeId)> {
        if let Type::Inter(a, b) = self.types.get(upper) {
            let mentions = |t: &mut Self, part: TypeId| t.mentions_tparam_of(part, Some(&[p]));
            if !mentions(self, a) && mentions(self, b) {
                if let Some(found) = self.find_member_from(pre, b, name, 0, joint) {
                    return Some(found);
                }
            }
        }
        self.find_member_from(pre, upper, name, private_until, joint)
    }

    /// The parameter counts of a method's clauses.
    fn member_shape(&mut self, s: SymId) -> Vec<usize> {
        self.sig_of(s).clauses.iter().map(|cl| cl.params.len()).collect()
    }

    /// Whether the member `name` of `target` reaches `c` along its parents past every class
    /// declaring a member of that name and shape itself, which hides it there (dotty's
    /// `SymDenotations.addInherited` through `Denotations.filterDisjoint`).
    fn inherited_unshadowed(&mut self, c: ClassId, target: ClassId, name: Name, want: &[usize]) -> bool {
        let mut seen = vec![c];
        let mut stack = vec![c];
        while let Some(x) = stack.pop() {
            if x == target {
                return true;
            }
            if let Some(&m) = self.syms.class(x).members.get(&name) {
                let own: Vec<SymId> = match self.syms.alternatives(m) {
                    Some(alts) => alts.iter().copied().filter(|&a| self.syms.sym(a).owner == Owner::Class(x)).collect(),
                    None => vec![m],
                };
                if own.into_iter().any(|a| self.member_shape(a) == want) {
                    continue;
                }
            }
            for p in self.syms.class(x).parents.clone() {
                if let Some(pc) = self.class_of(p) {
                    if !seen.contains(&pc) {
                        seen.push(pc);
                        stack.push(pc);
                    }
                }
            }
        }
        false
    }

    /// Private members count in the first `private_until` classes of the linearisation; one of
    /// an ancestor is not inherited. `pre` is the receiver the lookup started from, which a
    /// join, a proxy's underlying type and an intersection's parts are searched for: the members
    /// of an intersection meet against its base classes (dotty's `findMember`, whose `goOr` and
    /// `goAnd` keep `pre` while they search `tp.join` and the parts).
    /// `joint` takes the joint information of the member found (`JointInfo`).
    fn find_member_from(&mut self, pre: TypeId, t: TypeId, name: Name, private_until: usize, joint: &mut JointInfo) -> Option<(SymId, TypeId)> {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Class(c, args) => self.find_class_member(t, c, args, name, private_until),
            // Of an F-bounded parameter, `C <: LinearSeq[A] & LinearSeqOps[A, CC, C]`, the part
            // that names `C` gives the member its most specific type (`tail: C`), as the
            // intersection's member has under scalac.
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                if upper == ANY || self.bound_path.contains(&p) {
                    return None;
                }
                self.bound_path.push(p);
                let found = self.find_member_in_bound(pre, p, upper, name, private_until, joint);
                self.bound_path.pop();
                found
            }
            Type::AppParam(p, args) => {
                let upper = self.app_param_upper(p, args)?;
                self.find_member_from(pre, upper, name, private_until, joint)
            }
            Type::Inter(a, b) => self.find_member_meet(pre, (a, b), name, private_until, false, joint),
            Type::Union(..) => {
                let j = self.union_join(t)?;
                self.find_member_from(pre, j, name, private_until, joint)
            }
            Type::Lit(_) => {
                let class = self.widen_lit(t);
                self.find_member_from(pre, class, name, private_until, joint)
            }
            // The class's member, its owner's type the base type of `p.C` (`base_type`), so
            // that the enclosing classes are seen from the prefix.
            Type::Nested(p, class) => {
                let (sym, owner_ty) = self.find_member_from(pre, class, name, private_until, joint)?;
                let Type::Class(c, _) = self.types.get(class) else { return Some((sym, owner_ty)) };
                let owner_ty = match self.types.get(owner_ty) {
                    Type::Class(k, _) if k == c => self.nested_type(p, owner_ty),
                    _ => self.outer_seen_from(owner_ty, p, c),
                };
                Some((sym, owner_ty))
            }
            // A class's `this` with a self type is teq's `cls & self`, dotty's `givenSelf & cls`
            // (`ClassInfo.selfType`): its members are merged in dotty's order.
            Type::This(c) if self.syms.class(c).declared_self.is_some() => {
                let under = self.dependent_underlying(t)?;
                let Type::Inter(cls, given) = self.types.get(under) else { return self.find_member_from(pre, under, name, private_until, joint) };
                self.find_member_meet(pre, (cls, given), name, private_until, true, joint)
            }
            Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..) | Type::Rec(_) | Type::Alias(..) | Type::Match(..) => {
                let under = self.dependent_underlying(t)?;
                self.find_member_from(pre, under, name, private_until, joint)
            }
            _ => None,
        }
    }

    /// The member `name` of the class type `t` (`c` applied to `args`), seen from `t`: the first
    /// class of its linearisation that declares it, a private member of an ancestor passed over.
    /// A class's lookup meets nothing: no prefix or joint information takes part.
    fn find_class_member(&mut self, t: TypeId, c: ClassId, args: TList, name: Name, private_until: usize) -> Option<(SymId, TypeId)> {
        self.complete_class(c);
        let info = self.syms.class(c);
        let mut found = None;
        for (i, &(bc, bt)) in info.base_types.iter().enumerate() {
            if let Some(&sym) = self.syms.class(bc).members.get(&name) {
                // A private member of an ancestor class is not inherited.
                if i >= private_until && self.is_private(sym) {
                    continue;
                }
                found = Some((bc, sym, bt));
                break;
            }
        }
        // An abstract method that a class further down the linearisation defines concretely
        // (`Cap extends Util, TagFactory` with `Util.tag` implementing `TagFactory.tag`) meets
        // it as dotty's class lookup meets the parents' denotations (`computeMembersNamed`, then
        // `toDenot`'s `meet`): `mergeSingleDenot` takes the concrete one (`merge_single`). Only an
        // inherited member meets a further one, told first; whether it is a lone abstract def (a
        // trait's or an abstract class's) is decided once per member, and the search for the
        // concrete one runs apart from the lookup's path. The concrete definition takes part only
        // where it reaches the class past every own declaration of its shape, as dotty's
        // `addInherited` keeps of a parent's members those no own one matches (`filterDisjoint`).
        if let Some((bc, sym, bt)) = found.filter(|&(bc, _, _)| bc != c) {
            if let Some(concrete) = self.inherited_meeting(t, c, args, name, (bc, sym, bt)) {
                found = Some(concrete);
            }
        }
        let (bc, mut sym, bt) = match found {
            Some(f) => f,
            None => {
                // A member of `Any` is `universal_member`'s: reading a Java class for it
                // would give a std class the JDK's members where the JDK is open alone.
                if self.loaded.is_some() && !self.bare_lookup && !crate::names::ANY_MEMBERS.contains(&self.interner.get(name)) && self.java_member_miss(c) {
                    if self.loaded.as_ref().unwrap().detail {
                        eprintln!("java members of {} read for {}", self.class_path(c), self.name_str(name));
                    }
                    return self.find_class_member(t, c, args, name, private_until);
                }
                return None;
            }
        };
        match self.settled(bc, name, sym) {
            Some(settled) => sym = settled,
            None => return self.find_class_member(t, c, args, name, private_until),
        }
        if self.types.items(args).is_empty() {
            return Some((sym, bt));
        }
        let subst: Subst = self
            .syms
            .class(c)
            .tparams
            .iter()
            .copied()
            .zip(self.types.items(args).iter().copied())
            .collect();
        Some((sym, self.types.subst(bt, &subst)))
    }

    /// The concrete member an inherited one meets further down `c`'s linearisation, where the
    /// inherited one is a lone abstract def (decided once per member): apart from the lookup's
    /// path, which takes it only for a member it found in a base.
    #[cold]
    #[inline(never)]
    fn inherited_meeting(&mut self, t: TypeId, c: ClassId, args: TList, name: Name, (bc, sym, bt): (ClassId, SymId, TypeId)) -> Option<(ClassId, SymId, TypeId)> {
        let deferred = match self.lone_abstract_defs.get(sym) {
            Some(d) => d,
            None => self.lone_abstract_def_now(bc, sym),
        };
        if !deferred {
            return None;
        }
        let at = self.syms.class(c).base_types.iter().position(|&(b, _)| b == bc)?;
        self.concrete_meeting(t, c, args, name, at, (sym, bt))
    }

    /// Whether `sym` of `bc` is a lone abstract def, which a concrete one further down a class's
    /// linearisation can meet: decided once per member (`lone_abstract_defs`).
    #[cold]
    #[inline(never)]
    fn lone_abstract_def_now(&mut self, bc: ClassId, sym: SymId) -> bool {
        let d = self.syms.sym(sym).kind == SymKind::Def && {
            let info = self.syms.class(bc);
            info.kind == ClassKind::Trait || info.mods & crate::ast::mods::ABSTRACT != 0
        } && self.syms.alternatives(sym).is_none() && self.is_deferred_member(sym);
        self.lone_abstract_defs.set(sym, d);
        d
    }

    /// The concrete member of `name` and the abstract one's shape that a base of `c` after the
    /// `at`-th defines, where the meet of the two takes it (`mergeSingleDenot`).
    #[cold]
    #[inline(never)]
    fn concrete_meeting(&mut self, t: TypeId, c: ClassId, args: TList, name: Name, at: usize, (sym, bt): (SymId, TypeId)) -> Option<(ClassId, SymId, TypeId)> {
        let want = self.member_shape(sym);
        let rest: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types.iter().skip(at + 1).copied().collect();
        for (bc2, bt2) in rest {
            let Some(&sym2) = self.syms.class(bc2).members.get(&name) else { continue };
            if self.syms.alternatives(sym2).is_some() || self.is_private(sym2) || self.is_deferred_member(sym2) || self.syms.sym(sym2).kind != SymKind::Def {
                continue;
            }
            if self.member_shape(sym2) != want {
                return None;
            }
            if self.inherited_unshadowed(c, bc2, name, &want) {
                return self.meets_as_second(t, c, args, (sym, bt), (sym2, bt2)).then_some((bc2, sym2, bt2));
            }
        }
        None
    }

    /// Whether, of a class's two inherited members of one name and shape, `second` found further
    /// down the linearisation than `first`, the meet of the two is the second's symbol, as
    /// `mergeSingleDenot` decides it (`merge_single`); each owner type seen from `c[args]`.
    fn meets_as_second(&mut self, t: TypeId, c: ClassId, args: TList, (s1, bt1): (SymId, TypeId), (s2, bt2): (SymId, TypeId)) -> bool {
        let (bt1, bt2) = if self.types.items(args).is_empty() {
            (bt1, bt2)
        } else {
            let subst: Subst = self.syms.class(c).tparams.iter().copied().zip(self.types.items(args).iter().copied()).collect();
            (self.types.subst(bt1, &subst), self.types.subst(bt2, &subst))
        };
        matches!(self.merge_single(t, &((s1, bt1), None), &((s2, bt2), None)), Meet::Second | Meet::Joint(false, _))
    }

    /// The member `name` of an intersection's parts `a` and `b`, merged as dotty's `goAnd`
    /// merges `go(a)` and `go(b)` (`Types.scala`, `findMember`, at the 3.8.4 tag) against the
    /// whole receiver `pre`'s base classes, a part that is an intersection itself folded in with
    /// them, the joint information of the member it makes in `joint`. Each part's member is seen
    /// from the whole receiver, as a class part's `ClassDenotation.findMember` sees it with `pre`
    /// (`seen_from_whole`): one symbol through two instances of its owner is one member then.
    /// Two members of matching signatures merge as `mergeSingleDenot` merges them
    /// (`merge_single`); members that do not merge, or whose signatures do not match, are an
    /// overload (dotty's `MultiDenotation`), whose alternatives an application takes from
    /// `intersection_denotations`. The second part is another class, whose private members are
    /// its own. `reversed`: `b` is dotty's first part (a self type's).
    fn find_member_meet(&mut self, pre: TypeId, (a, b): (TypeId, TypeId), name: Name, private_until: usize, reversed: bool, joint: &mut JointInfo) -> Option<(SymId, TypeId)> {
        let (mut ja, mut jb) = (None, None);
        let first = self.find_member_in_part(pre, a, name, private_until, &mut ja);
        let second = self.find_member_in_part(pre, b, name, 0, &mut jb);
        let (found, info) = match (first, second) {
            // The same denotation through both parts (dotty's `denot1 eq denot2`).
            (Some(fa), Some(fb)) if fa == fb => (Some(fa), ja),
            (Some(fa), Some(fb)) => {
                let da = self.seen_from_whole(pre, (fa, ja));
                let db = self.seen_from_whole(pre, (fb, jb));
                let ((sa, ta), _) = da;
                let ((sb, tb), _) = db;
                let overloaded = self.syms.alternatives(sa).is_some() || self.syms.alternatives(sb).is_some();
                // A part's refinement that stays an alternative beside the member makes the
                // member an overload, whose alternatives an application takes from
                // `intersection_denotations`.
                let refined = !overloaded && (self.refinement_alternative(a, name, &da).is_some() || self.refinement_alternative(b, name, &db).is_some());
                if refined {
                    (Some((self.merged_overload(sa, ta, sb, tb), ta)), None)
                } else if da.0 == db.0 {
                    // One symbol through two instances of its owner, seen from the whole
                    // receiver alike: one member.
                    (Some(da.0), da.1)
                } else if overloaded {
                    if self.overloads_across(sa, ta, sb, tb) {
                        (Some((self.merged_overload(sa, ta, sb, tb), ta)), None)
                    } else {
                        // Each alternative of the second matches one of the first's: the first's
                        // set, whose alternatives the receiver meets with their matches where it
                        // applies one (`intersection_denotations`).
                        (Some((sa, ta)), None)
                    }
                } else {
                    let (d1, d2) = if reversed { (db, da) } else { (da, db) };
                    let matched = {
                        let (i1, i2) = (self.denot_info(&d1), self.denot_info(&d2));
                        self.infos_match(&i1, &i2)
                    };
                    match matched.then(|| self.merge_single(pre, &d1, &d2)) {
                        Some(Meet::First) => (Some(d1.0), d1.1),
                        Some(Meet::Second) => (Some(d2.0), d2.1),
                        Some(Meet::Joint(true, info)) => (Some(d1.0), Some(info)),
                        Some(Meet::Joint(false, info)) => (Some(d2.0), Some(info)),
                        // Two members kept apart: an overload of the two.
                        Some(Meet::Apart) | None => (Some((self.merged_overload(sa, ta, sb, tb), ta)), None),
                    }
                }
            }
            (Some(f), _) => (Some(f), ja),
            (None, f) => (f, jb),
        };
        *joint = info;
        found
    }

    fn find_member_in_part(&mut self, pre: TypeId, t: TypeId, name: Name, private_until: usize, joint: &mut JointInfo) -> Option<(SymId, TypeId)> {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Inter(a, b) => self.find_member_meet(pre, (a, b), name, private_until, false, joint),
            _ => self.find_member_from(pre, t, name, private_until, joint),
        }
    }

    /// The member `s` a part of the receiver `pre` gives as a member of the instance `t` of its
    /// owner, with the joint information it carries, seen from the whole receiver as dotty's class
    /// lookup sees it (`ClassDenotation.findMember` with `pre`, whose denotations are `asSeenFrom`
    /// it): where the owner is a base class of the receiver under another instance than `t`, the
    /// one the receiver's base type is (`prefix_base_type`), the information seen from it
    /// (`seen_from_instance`). `R["k"] & R["v"]`'s `get(k: K)` takes `"k" | "v"`. An overloaded
    /// set's alternatives are seen so one by one (`part_denotations`).
    fn seen_from_whole(&mut self, pre: TypeId, ((s, t), joint): Denot) -> Denot {
        let Owner::Class(owner) = self.syms.sym(s).owner else { return ((s, t), joint) };
        if self.syms.alternatives(s).is_some() || !matches!(self.types.get(t), Type::Class(_, args) if args != EMPTY_LIST) {
            return ((s, t), joint);
        }
        let Some(whole) = self.prefix_base_type(pre, owner, &mut Vec::new()) else { return ((s, t), joint) };
        if whole == t {
            return ((s, t), joint);
        }
        match self.seen_from_instance(s, whole, &joint) {
            Some(info) => ((s, whole), Some(info)),
            None => ((s, whole), joint),
        }
    }

    /// The instance of the class `c` the receiver `pre` has as a whole, as dotty's `baseType`
    /// gives it (`Types.scala` at the 3.8.4 tag): an intersection's the meet of its parts'
    /// (`AndType`'s `baseTp1 & baseTp2`, `met_instances`), a refinement's its parent's, a type
    /// parameter's its bound's, a class's `this` with a self type its `cls & self`'s, any other
    /// type's `base_type`.
    fn prefix_base_type(&mut self, pre: TypeId, c: ClassId, seen: &mut Vec<TParamId>) -> Option<TypeId> {
        let t = self.deref(pre);
        if let Type::This(k) = self.types.get(t) {
            if self.syms.class(k).declared_self.is_some() {
                return match self.dependent_underlying(t) {
                    Some(under) if under != t => self.prefix_base_type(under, c, seen),
                    _ => self.base_type(t, c),
                };
            }
        }
        let t = self.dealias(t);
        match self.types.get(t) {
            // Every part, however many (`ClassDenotation.baseTypeOf` meets both operands).
            Type::Inter(a, b) => {
                let x = self.prefix_base_type(a, c, seen);
                let y = self.prefix_base_type(b, c, seen);
                match (x, y) {
                    (Some(x), Some(y)) => Some(self.met_instances(c, x, y)),
                    (x, y) => x.or(y),
                }
            }
            Type::Refined(parent, _) => self.prefix_base_type(parent, c, seen),
            // A recursive refinement as its body opened at itself, the proxy's underlying type.
            Type::Rec(body) => {
                let opened = self.opened_rec(body, pre);
                self.prefix_base_type(opened, c, seen)
            }
            // A parameter's bound, once on the way down: bounds that name each other end there.
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                if upper == ANY || seen.contains(&p) {
                    return None;
                }
                let upper = self.types.translate(self.types.view_here(), upper);
                seen.push(p);
                let found = self.prefix_base_type(upper, c, seen);
                seen.pop();
                found
            }
            // An applied parameter's bound applied (`F[Int]` of `F[X] <: R["k"] & R["v"]`), a
            // union's join and a proxy's underlying type, as `baseTypeOf` takes a `TypeProxy`'s
            // super type: the instance met through them as written receivers meet it.
            Type::AppParam(p, args) => {
                if seen.contains(&p) {
                    return None;
                }
                let upper = self.app_param_upper(p, args)?;
                seen.push(p);
                let found = self.prefix_base_type(upper, c, seen);
                seen.pop();
                found
            }
            Type::Union(..) => match self.union_join(t) {
                Some(j) if j != t => self.prefix_base_type(j, c, seen),
                _ => self.base_type(t, c),
            },
            Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Alias(..) | Type::Match(..) => match self.dependent_underlying(t) {
                Some(under) if under != t => self.prefix_base_type(under, c, seen),
                _ => self.base_type(t, c),
            },
            _ => self.base_type(t, c),
        }
    }

    /// The meet of two instances `x` and `y` of the class `c`, as dotty's `glb` makes it of two
    /// applications of one class and `NamedType.argForParam` reads it (`TypeComparer`'s
    /// `dropIfSuper` and `distributeAnd`'s `glbArgs`, `Types.scala`'s `argForParam` of an
    /// `AndType`, at the 3.8.4 tag): an instance conforming to the other is the meet; else the
    /// argument of a covariant parameter is the intersection of the two, of a contravariant one
    /// their union, of an invariant one the same argument where they are the same and else the
    /// wildcard whose bounds are both arguments' met (`TypeBounds.&`: one within the other, else
    /// the union of the lower bounds to the intersection of the upper ones).
    fn met_instances(&mut self, c: ClassId, x: TypeId, y: TypeId) -> TypeId {
        if x == y {
            return x;
        }
        let (Type::Class(cx, xs), Type::Class(cy, ys)) = (self.types.get(x), self.types.get(y)) else { return x };
        if cx != c || cy != c {
            return x;
        }
        if self.frozen_sub(x, y) {
            return x;
        }
        if self.frozen_sub(y, x) {
            return y;
        }
        let (xs, ys) = (self.types.items(xs).to_vec(), self.types.items(ys).to_vec());
        let tparams = self.syms.class(c).tparams.clone();
        let mut args = Vec::with_capacity(xs.len());
        for (i, (&a, &b)) in xs.iter().zip(&ys).enumerate() {
            let variance = tparams.get(i).map_or(0, |&p| self.syms.tparam(p).variance);
            let arg = match variance {
                _ if a == b => a,
                v if v > 0 => self.met_type(a, b),
                v if v < 0 => self.joined_type(a, b),
                _ => {
                    let wild = |w: &Self, t: TypeId| w.types.wild_bounds(t).unwrap_or((t, t));
                    let ((lo1, hi1), (lo2, hi2)) = (wild(self, a), wild(self, b));
                    if !self.types.is_wild(a) && !self.types.is_wild(b) && self.frozen_sub(a, b) && self.frozen_sub(b, a) {
                        a
                    } else if self.frozen_sub(lo1, lo2) && self.frozen_sub(hi2, hi1) {
                        b
                    } else if self.frozen_sub(lo2, lo1) && self.frozen_sub(hi1, hi2) {
                        a
                    } else {
                        let lo = self.joined_type(lo1, lo2);
                        let hi = self.met_type(hi1, hi2);
                        self.types.bounded_wild(lo, hi)
                    }
                }
            };
            args.push(arg);
        }
        self.types.class(c, &args)
    }

    /// The information of `s` (`joint` where a meet made it) seen from `base`, an instance of its
    /// owner some of whose arguments are wildcards (`met_instances`): dotty's `asSeenFrom`, the
    /// exact arguments substituted and each wildcard's parameter a range its positions approximate
    /// (`approx_ranges`, dotty's `ApproximatingTypeMap`): a parameter's type at a parameter's
    /// variance, the result at a result's. `None` where no argument is a wildcard, the instance's
    /// substitution being the information then.
    fn seen_from_instance(&mut self, s: SymId, base: TypeId, joint: &JointInfo) -> Option<std::sync::Arc<MethodSig>> {
        let Type::Class(c, list) = self.types.get(base) else { return None };
        let args = self.types.items(list).to_vec();
        if !args.iter().any(|&a| self.types.is_wild(a)) {
            return None;
        }
        let sig = match joint {
            Some(j) => j.clone(),
            None => self.sig_arc(s),
        };
        let tparams = self.syms.class(c).tparams.clone();
        let mut exact: Subst = Vec::new();
        let (mut ranged, mut ranges) = (Vec::new(), Vec::new());
        for (&p, &a) in tparams.iter().zip(&args) {
            match self.types.wild_bounds(a) {
                Some(bounds) => {
                    ranged.push(p);
                    ranges.push(bounds);
                }
                None => exact.push((p, a)),
            }
        }
        let mut out = (*sig).clone();
        for clause in &mut out.clauses {
            for p in &mut clause.params {
                let t = self.types.subst(p.ty, &exact);
                p.ty = self.approx_ranges(t, &ranged, &ranges, -1);
            }
        }
        let t = self.types.subst(sig.ret, &exact);
        out.ret = self.approx_ranges(t, &ranged, &ranges, 1);
        Some(std::sync::Arc::new(out))
    }

    /// The alternatives of `name` on the receiver `t` where it is an intersection, each with
    /// the instance of its owner it is seen as and its joint information: dotty's
    /// `Denotation.meet` of the parts' denotations (`Denotations.scala`, `meet` and `mergeDenot`,
    /// at the 3.8.4 tag), each seen from the whole receiver (`seen_from_whole`), each alternative
    /// of one part merged with the first of the other's whose signature it matches
    /// (`merge_denotation`), the unmatched and the unmergeable kept. What an overloaded selection
    /// on an intersection resolves among (`C & R` whose `f(x: Int): Any` and `f(x: Int): String`
    /// make one alternative of the met result beside the others). The receiver is followed to
    /// its intersection as `find_member_from` follows it (`Types.findMember`'s `go`: a type
    /// parameter's bound, `T <: R["k"] & R["v"]`, a path's underlying type, an alias, a
    /// refinement's parent, a union's join, a class's `this` with a self type as `givenSelf &
    /// cls`), the receiver as written the prefix throughout. None where it reaches no
    /// intersection.
    pub(super) fn intersection_denotations(&mut self, t: TypeId, name: Name) -> Option<Vec<Denot>> {
        self.denotations_under(t, t, name, &mut Vec::new())
    }

    fn denotations_under(&mut self, pre: TypeId, t: TypeId, name: Name, seen: &mut Vec<TParamId>) -> Option<Vec<Denot>> {
        let t = self.deref(t);
        if let Type::This(c) = self.types.get(t) {
            if self.syms.class(c).declared_self.is_some() {
                let under = self.dependent_underlying(t)?;
                let Type::Inter(cls, given) = self.types.get(under) else { return self.denotations_under(pre, under, name, seen) };
                // dotty's `givenSelf & cls`: the self type's members first, the class's own
                // private members its own.
                let mut acc = self.part_denotations(pre, given, name, 0);
                for d in self.part_denotations(pre, cls, name, 1) {
                    self.merge_denotation(pre, &mut acc, d);
                }
                return Some(acc);
            }
        }
        let t = self.dealias(t);
        match self.types.get(t) {
            Type::Inter(..) => Some(self.part_denotations(pre, t, name, 1)),
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                if upper == ANY || seen.contains(&p) {
                    return None;
                }
                let upper = self.types.translate(self.types.view_here(), upper);
                seen.push(p);
                let found = self.denotations_under(pre, upper, name, seen);
                seen.pop();
                found
            }
            Type::AppParam(p, args) => {
                let upper = self.app_param_upper(p, args)?;
                self.denotations_under(pre, upper, name, seen)
            }
            Type::Union(..) => {
                let j = self.union_join(t)?;
                self.denotations_under(pre, j, name, seen)
            }
            // A refined type whose refinements name an overloaded member's alternatives refines them
            // (`goRefined` of a `MultiDenotation`).
            Type::Refined(..) | Type::Rec(_) if self.types.term_refinements() && self.refines_overload(t, name) => Some(self.part_denotations(pre, t, name, 1)),
            Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..) | Type::Rec(_) | Type::Alias(..) | Type::Match(..) => {
                let under = self.dependent_underlying(t)?;
                self.denotations_under(pre, under, name, seen)
            }
            _ => None,
        }
    }

    /// The information of the method refinement `r` where its parameters are the denotation `d`'s
    /// (the same signature, `infos_match`).
    fn matching_refinement_info(&mut self, r: Refinement, d: &Denot) -> Option<std::sync::Arc<MethodSig>> {
        let rinfo = self.refinement_sig(r);
        let info = self.denot_info(d);
        if rinfo.tparams.len() != info.tparams.len() || !self.infos_match(&info, &rinfo) {
            return None;
        }
        Some(rinfo)
    }

    /// Every method refinement of `name` along the refined type `t`'s chain, the outermost first, a
    /// recursive one opened at `t` (the refinements of an anonymous class's overloads).
    fn term_refinements_named(&mut self, t: TypeId, name: Name) -> Vec<Refinement> {
        let mut out = Vec::new();
        let mut t = self.deref(t);
        let whole = t;
        for _ in 0..256 {
            match self.types.get(t) {
                Type::Rec(body) => t = self.opened_rec(body, whole),
                Type::Refined(p, r) => {
                    let refinement = self.types.refinement(r);
                    if matches!(refinement, Refinement::Term(n, ..) if n == name) {
                        out.push(refinement);
                    }
                    t = p;
                }
                _ => break,
            }
        }
        out
    }

    /// Whether `t` refines the term `name` and its parent has several alternatives of it.
    fn refines_overload(&mut self, t: TypeId, name: Name) -> bool {
        if self.term_refinements_named(t, name).is_empty() {
            return false;
        }
        let mut joint = None;
        self.find_member_from(t, t, name, 1, &mut joint).is_some_and(|(s, _)| self.syms.alternatives(s).is_some())
    }

    fn part_denotations(&mut self, pre: TypeId, t: TypeId, name: Name, private_until: usize) -> Vec<Denot> {
        let t = self.deref(t);
        let t = self.dealias(t);
        if let Type::Inter(a, b) = self.types.get(t) {
            let mut acc = self.part_denotations(pre, a, name, private_until);
            for d in self.part_denotations(pre, b, name, 0) {
                self.merge_denotation(pre, &mut acc, d);
            }
            return acc;
        }
        let mut joint = None;
        let Some((s, owner)) = self.find_member_from(pre, t, name, private_until, &mut joint) else { return Vec::new() };
        match self.syms.alternatives(s).map(|alts| alts.to_vec()) {
            // A class's alternatives in dotty's scope order, the last declared first.
            Some(mut alts) => {
                alts.sort_by_key(|&a| std::cmp::Reverse(self.syms.sym(a).span.start));
                let denots: Vec<Denot> = alts
                    .into_iter()
                    .map(|alt| {
                        let owner_ty = match self.syms.sym(alt).owner {
                            Owner::Class(c) => self.base_type(t, c).unwrap_or(owner),
                            _ => owner,
                        };
                        self.seen_from_whole(pre, ((alt, owner_ty), None))
                    })
                    .collect();
                // Each refinement of the name refines the alternative whose parameters it matches
                // (`goRefined`'s `pdenot & rinfo` of a `MultiDenotation`), the refinements an
                // anonymous class's overloads make (`TypeOps.classBound`).
                let refinements = self.term_refinements_named(t, name);
                if refinements.is_empty() {
                    return denots;
                }
                let mut out = Vec::with_capacity(denots.len());
                for d in denots {
                    // The refinement's information is the alternative's where it conforms (the meet
                    // of the two is it then, `infoMeet`); the alternative keeps its symbol and owner.
                    let refined = refinements.iter().find_map(|&r| self.matching_refinement_info(r, &d));
                    match refined {
                        Some(rinfo) if self.refined_info_narrows(&d, &rinfo) => out.push((d.0, Some(rinfo))),
                        _ => out.push(d),
                    }
                }
                out
            }
            None => {
                let d = self.seen_from_whole(pre, ((s, owner), joint));
                match self.refinement_alternative(t, name, &d) {
                    Some(r) => vec![d, r],
                    None => vec![d],
                }
            }
        }
    }

    /// Whether the refinement's information `rinfo` conforms to the denotation `d`'s: its result below
    /// the member's, the parameters being the same.
    fn refined_info_narrows(&mut self, d: &Denot, rinfo: &std::sync::Arc<MethodSig>) -> bool {
        let info = self.denot_info(d);
        let renamed = self.renamed_info(rinfo, &info);
        let mark = self.snapshot();
        let below = self.is_sub(renamed.ret, info.ret);
        self.rollback(mark);
        below
    }

    /// `mergeDenot` of the denotations so far and one more: the same denotation again dropped,
    /// else merged with the first whose signature it matches as `mergeSingleDenot` merges two
    /// (`merge_single`), kept beside that one where the two do not merge, kept apart at the end
    /// where none matches.
    fn merge_denotation(&mut self, pre: TypeId, acc: &mut Vec<Denot>, d: Denot) {
        for i in 0..acc.len() {
            if acc[i].0 == d.0 {
                return;
            }
            let (ia, id) = (self.denot_info(&acc[i]), self.denot_info(&d));
            if !self.infos_match(&ia, &id) {
                continue;
            }
            let first = acc[i].clone();
            match self.merge_single(pre, &first, &d) {
                Meet::First => {}
                Meet::Second => acc[i] = d,
                Meet::Joint(true, info) => acc[i] = (first.0, Some(info)),
                Meet::Joint(false, info) => acc[i] = (d.0, Some(info)),
                Meet::Apart => acc.insert(i + 1, d),
            }
            return;
        }
        acc.push(d);
    }

    /// The information of the denotation `d`: its joint information where a meet made one, else
    /// its symbol's signature, a structural given's with its class for the result (dotty's given
    /// module is a reference of its class, `given_object_type`), under the substitution of the
    /// instance of its owner it is a member of (dotty's `SingleDenotation.info`, `asSeenFrom` its
    /// prefix).
    pub(super) fn denot_info(&mut self, ((s, t), joint): &Denot) -> MethodSig {
        let sig = match joint {
            Some(j) => j.clone(),
            None => {
                let own = self.sig_arc(*s);
                match self.given_object_type(*s) {
                    Some(class) => std::sync::Arc::new(MethodSig { tparams: own.tparams.clone(), clauses: own.clauses.clone(), ret: class, declared: crate::tir::NO_TYPE }),
                    None => own,
                }
            }
        };
        let subst = self.owner_subst(*t);
        let mut out = (*sig).clone();
        if !subst.is_empty() {
            for clause in &mut out.clauses {
                for p in &mut clause.params {
                    p.ty = self.types.subst(p.ty, &subst);
                }
            }
            out.ret = self.types.subst(out.ret, &subst);
        }
        out
    }

    /// The whole information of the member `s` of the receiver `pre`, seen from the instance of
    /// its owner the receiver is as a whole (`prefix_base_type`, `seen_from_instance`): what a
    /// library document's index merges (`merge_infos`).
    pub(super) fn member_info_on(&mut self, pre: TypeId, s: SymId) -> MethodSig {
        let base = match self.syms.sym(s).owner {
            Owner::Class(c) => self.prefix_base_type(pre, c, &mut Vec::new()),
            _ => None,
        };
        let Some(base) = base else { return (*self.sig_arc(s)).clone() };
        let joint = self.seen_from_instance(s, base, &None);
        self.denot_info(&((s, base), joint))
    }

    /// Of two denotations of one name and matching signatures, found on an intersection's first
    /// and second part, the one dotty's `mergeSingleDenot` takes over their whole information
    /// (`merge_infos`), the second's type parameters read as the first's (`tp2.subst(tp2, tp1)`).
    /// A refinement's denotation, which has no symbol (`refinement_alternative`), scores as
    /// dotty's `!sym.exists`: 2 against the other, the first's checked first.
    fn merge_single(&mut self, pre: TypeId, d1: &Denot, d2: &Denot) -> Meet {
        let ((s1, _), _) = *d1;
        let ((s2, _), _) = *d2;
        let i1 = self.denot_info(d1);
        let i2 = self.denot_info(d2);
        if self.types.contains_error(i1.ret) || self.types.contains_error(i2.ret) {
            return Meet::First;
        }
        let i2 = self.renamed_info(&i2, &i1);
        let score = match (self.is_refinement_denot(d1), self.is_refinement_denot(d2)) {
            (true, _) => Some(-2),
            (_, true) => Some(2),
            _ => None,
        };
        let hidden = match score {
            Some(_) => (false, false),
            None => (!self.is_accessible(s1), !self.is_accessible(s2)),
        };
        let bases = self.receiver_base_classes(pre, 0);
        self.merge_scored(&bases, (s1, &i1), (s2, &i2), hidden, score)
    }

    /// Whether `d` is a refinement's denotation (`refinement_alternative`): the member it
    /// refines, the refined type its prefix in the place of an instance of its owner.
    pub(super) fn is_refinement_denot(&self, ((_, t), _): &Denot) -> bool {
        matches!(self.types.get(*t), Type::Refined(..))
    }

    /// The refinement of `name` that the part `t` of an intersection's receiver makes of the
    /// member `d` (seen from the whole receiver), where it stays a member of its own beside it:
    /// dotty's `goRefined` meets the refinement's information with the member's, which stay two
    /// alternatives where their signatures do not match (`Denotation.meet`'s
    /// `MultiDenotation`: `Rec["k", String] & Rec["v", Int]`'s `get(k: "k"): String` beside
    /// the member's `get(k: "k" | "v"): Any`). A method refinement alone, whose parameters erase
    /// as the member's: a call of it is a call of the member, as the reflective call dotty makes
    /// of a symbol-less alternative finds that method. Its denotation is the member's symbol
    /// with the refined type as its prefix (`is_refinement_denot`) and its information.
    fn refinement_alternative(&mut self, t: TypeId, name: Name, d: &Denot) -> Option<Denot> {
        let Some(r @ Refinement::Term(..)) = self.refined_term(t, name) else { return None };
        self.denotation_refined_by(t, r, d)
    }

    /// The denotation `d` refined by the term refinement `r` of `t`, where `r`'s parameters are
    /// `d`'s and its information another.
    fn denotation_refined_by(&mut self, t: TypeId, r: Refinement, d: &Denot) -> Option<Denot> {
        if !matches!(r, Refinement::Term(..)) {
            return None;
        }
        let rinfo = self.refinement_sig(r);
        let info = self.denot_info(d);
        if rinfo.clauses.is_empty() || rinfo.tparams.len() != info.tparams.len() || rinfo.clauses.len() != info.clauses.len() {
            return None;
        }
        let shape = |s: &MethodSig| s.clauses.iter().map(|c| c.params.len()).collect::<Vec<_>>();
        if shape(&rinfo) != shape(&info) {
            return None;
        }
        let pairs: Vec<(ParamSig, ParamSig)> = rinfo.clauses.iter().flat_map(|c| c.params.iter().cloned()).zip(info.clauses.iter().flat_map(|c| c.params.iter().cloned())).collect();
        for (p, q) in &pairs {
            if self.param_signature(p) != self.param_signature(q) {
                return None;
            }
        }
        if self.infos_match(&info, &rinfo) {
            return None;
        }
        let mut refined = self.deref(t);
        while let Type::Alias(..) = self.types.get(refined) {
            refined = self.reduce_head(refined)?;
        }
        // A recursive part is its refinements opened at itself (`goRec`).
        if let Type::Rec(body) = self.types.get(refined) {
            refined = self.opened_rec(body, refined);
        }
        if !matches!(self.types.get(refined), Type::Refined(..)) {
            return None;
        }
        Some(((d.0 .0, refined), Some(rinfo)))
    }

    /// `info` read in the binders of `to`, the information it is compared with: its type
    /// parameters as `to`'s where they are as many, and its term parameters' paths as `to`'s
    /// where the clauses take as many (`tp2.subst(tp2, tp1)` of `matchingMethodParams`,
    /// `compareMethod` and `infoMeet`: `y: x.T` of one method and `y: x.T` of the other are one
    /// type then). Its parameters' types and its result are rewritten; its `tparams` and its
    /// parameters' symbols stay its own, whose bounds `renamed_bounds` reads in `to`'s terms.
    pub(super) fn renamed_info(&mut self, info: &MethodSig, to: &MethodSig) -> MethodSig {
        let mut out = info.clone();
        let types: Subst = if !info.tparams.is_empty() && info.tparams.len() == to.tparams.len() && info.tparams != to.tparams {
            info.tparams.iter().zip(&to.tparams).map(|(&p, &q)| (p, self.types.param(q))).collect()
        } else {
            Vec::new()
        };
        let same_shape = info.clauses.len() == to.clauses.len() && info.clauses.iter().zip(&to.clauses).all(|(x, y)| x.params.len() == y.params.len());
        let terms: Vec<(SymId, TypeId)> = if same_shape {
            let pairs: Vec<(SymId, SymId)> = info.clauses.iter().zip(&to.clauses).flat_map(|(x, y)| x.params.iter().zip(&y.params).map(|(p, q)| (p.sym, q.sym))).filter(|(p, q)| p != q).collect();
            pairs.into_iter().map(|(p, q)| (p, self.types.mk(Type::Term(q)))).collect()
        } else {
            Vec::new()
        };
        if types.is_empty() && terms.is_empty() {
            return out;
        }
        let rename = |w: &mut Self, t: TypeId| {
            let t = if types.is_empty() { t } else { w.types.subst(t, &types) };
            w.subst_paths(t, &terms)
        };
        for clause in &mut out.clauses {
            for p in &mut clause.params {
                p.ty = rename(self, p.ty);
            }
        }
        out.ret = rename(self, out.ret);
        out
    }

    /// The bounds of the type parameter `q`, one of `from`, read in the terms of `to`, the type
    /// parameters `from`'s are compared with (`tp2.paramInfos.subst(tp2, tp1)`).
    fn renamed_bounds(&mut self, q: TParamId, from: &[TParamId], to: &[TParamId]) -> (TypeId, TypeId) {
        let view = self.types.view_here();
        let (lower, upper) = (self.syms.tparam(q).lower, self.syms.tparam(q).upper);
        let (lower, upper) = (self.types.translate(view, lower), self.types.translate(view, upper));
        if from == to || from.len() != to.len() {
            return (lower, upper);
        }
        let renaming: Subst = from.iter().zip(to).map(|(&p, &q)| (p, self.types.param(q))).collect();
        (self.types.subst(lower, &renaming), self.types.subst(upper, &renaming))
    }

    /// Which member of an intersection two members of one name and matching signatures make, as
    /// dotty's `mergeSingleDenot` decides it (`Denotations.scala`, `meet`, at the 3.8.4 tag):
    /// `s1` of the information `i1` from the first part, `s2` of `i2` from the second, the second's
    /// type parameters read as the first's, `bases` the whole receiver's base classes
    /// (`receiver_base_classes`). An inaccessible member gives way to an accessible one
    /// (`hidden`); else the symbol score prefers the member at least as concrete as the other
    /// (`isAsConcrete`), then the one whose owner derives from the other's or comes first in
    /// `bases` (`linearScore`), then the wider access boundary, then a method over a value; the
    /// member whose whole information conforms to the other's, unconstrained, wins where the score
    /// allows it (`overrides`); else the two make one member of the joint information
    /// (`infoMeet`), the scoring symbol's, the first's where the score is even; else, both
    /// symbols existing (no score of 2), they stay apart (`MultiDenotation`). The selection of the
    /// program's typer and of a library document's index alike.
    pub(super) fn merge_infos(&mut self, bases: &[ClassId], first: (SymId, &MethodSig), second: (SymId, &MethodSig), hidden: (bool, bool)) -> Meet {
        self.merge_scored(bases, first, second, hidden, None)
    }

    /// `merge_infos` where the symbol score is known (`Some(2)` or `Some(-2)`: one of the two has
    /// no symbol, whose information is taken alone where the two do not meet).
    fn merge_scored(&mut self, bases: &[ClassId], (s1, i1): (SymId, &MethodSig), (s2, i2): (SymId, &MethodSig), hidden: (bool, bool), score: Option<i32>) -> Meet {
        match hidden {
            (true, false) => return Meet::Second,
            (false, true) => return Meet::First,
            _ => {}
        }
        let score = match score {
            Some(score) => score,
            None => self.meet_score(bases, s1, s2),
        };
        // `matchNullaryLoosely`: `()T` and `=> T` alike where either member is Java's, `Any`'s or
        // a Scala 2 class's (teq's stand-in: the members a call takes with or without Java's
        // parentheses, `has_java_parens`); a member without a symbol is none of them.
        let loose = (score != -2 && self.has_java_parens(s1)) || (score != 2 && self.has_java_parens(s2));
        if score <= 0 && self.info_overrides_loosely(i2, i1, loose) {
            return Meet::Second;
        }
        if score >= 0 && self.info_overrides_loosely(i1, i2, loose) {
            return Meet::First;
        }
        match self.info_meet(i1, i2) {
            Some(mut joint) => {
                // The joint information is the first's (`tp1.derivedLambdaType`); the second's
                // symbol, where it is taken, has its own parameters, the joint's paths over the
                // first's rebound to them, and its own defaults.
                if score < 0 && score > -2 {
                    let own = self.sig_arc(s2);
                    let to = MethodSig { tparams: joint.tparams.clone(), clauses: own.clauses.clone(), ret: own.ret, declared: crate::tir::NO_TYPE };
                    joint = self.renamed_info(&joint, &to);
                    for (clause, theirs) in joint.clauses.iter_mut().zip(&own.clauses) {
                        for (p, q) in clause.params.iter_mut().zip(&theirs.params) {
                            p.has_default = q.has_default;
                            p.sym = q.sym;
                        }
                    }
                }
                Meet::Joint(score >= 0, std::sync::Arc::new(joint))
            }
            None if score == 2 => Meet::First,
            None if score == -2 => Meet::Second,
            None => Meet::Apart,
        }
    }

    /// Whether `t` is a value's information: no parameters and no type parameters (dotty's
    /// `ExprType` or value type, whose signature is `NotAMethod`).
    fn is_value_info(info: &MethodSig) -> bool {
        info.tparams.is_empty() && info.clauses.is_empty()
    }

    /// `a <: b` without constraining a type variable (dotty's `frozen_<:<`).
    pub(super) fn frozen_sub(&mut self, a: TypeId, b: TypeId) -> bool {
        if a == b {
            return true;
        }
        let mark = self.snapshot();
        let sub = self.is_sub(a, b) && self.snapshot() == mark;
        self.rollback(mark);
        sub
    }

    /// The meet of two types as dotty's `&` makes it: the one that conforms to the other, else
    /// their intersection.
    fn met_type(&mut self, a: TypeId, b: TypeId) -> TypeId {
        if self.frozen_sub(a, b) {
            a
        } else if self.frozen_sub(b, a) {
            b
        } else {
            self.types.inter(a, b)
        }
    }

    /// The join of two types as dotty's `|` makes it: the one the other conforms to, else their
    /// union.
    fn joined_type(&mut self, a: TypeId, b: TypeId) -> TypeId {
        if self.frozen_sub(a, b) {
            b
        } else if self.frozen_sub(b, a) {
            a
        } else {
            self.types.union(a, b)
        }
    }

    /// Whether two members' informations match as dotty's `matchesLoosely` has them for Scala
    /// definitions (`Denotations.scala` at the 3.8.4 tag, `Signature.matchDegree`): two values
    /// fully, a value and a method of no parameters at all (`MethodNotAMethodMatch`); two methods
    /// where the numbers of their type parameters and of each clause's parameters are the same
    /// and their parameters erase alike, and, where their results erase differently
    /// (`ParamMatch`), their parameter types are the same (`info.matches`, the second's type
    /// parameters read as the first's). teq compares the clauses' arities where dotty flattens
    /// them (`docs/internal/DIFFERENCES.md`).
    pub(super) fn infos_match(&mut self, a: &MethodSig, b: &MethodSig) -> bool {
        let (va, vb) = (Self::is_value_info(a), Self::is_value_info(b));
        if va || vb {
            let other = if va { b } else { a };
            return other.tparams.is_empty() && other.clauses.iter().all(|c| c.params.is_empty());
        }
        if a.tparams.len() != b.tparams.len() || a.clauses.len() != b.clauses.len() || a.clauses.iter().zip(&b.clauses).any(|(x, y)| x.params.len() != y.params.len()) {
            return false;
        }
        let pairs: Vec<(&ParamSig, &ParamSig)> = a.clauses.iter().flat_map(|c| c.params.iter()).zip(b.clauses.iter().flat_map(|c| c.params.iter())).collect();
        for &(p, q) in &pairs {
            if self.param_signature(p) != self.param_signature(q) {
                return false;
            }
        }
        if self.signature_class(a.ret) == self.signature_class(b.ret) {
            return true;
        }
        let b = self.renamed_info(b, a);
        let pairs: Vec<(TypeId, TypeId)> = a.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)).zip(b.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty))).collect();
        pairs.into_iter().all(|(x, y)| self.frozen_sub(x, y) && self.frozen_sub(y, x))
    }

    /// The class a parameter contributes to its method's signature: a by-name one's `Function0`,
    /// a repeated one's `Seq`, else its type's (`signature_class`).
    fn param_signature(&mut self, p: &ParamSig) -> ClassId {
        if p.by_name {
            return self.b.functions.first().copied().flatten().unwrap_or(self.b.any_ref);
        }
        if p.repeated {
            return self.b.seq.unwrap_or(self.b.any_ref);
        }
        self.signature_class(p.ty)
    }

    /// The class a type contributes to a signature (dotty's `sigName` of its erasure): a literal
    /// type's class, a union's the class both sides erase to (`erasedLub`, where it is one of
    /// them), else `erased_class`'s.
    fn signature_class(&mut self, t: TypeId) -> ClassId {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Lit(_) => {
                let class = self.widen_lit(t);
                self.erased_class(class, 0)
            }
            Type::Union(a, b, _) => {
                let (x, y) = (self.signature_class(a), self.signature_class(b));
                if x == y { x } else { self.b.any_ref }
            }
            _ => self.erased_class(t, 0),
        }
    }

    /// Whether the information `a` conforms to `b`, unconstrained: dotty's `a.overrides(b)`
    /// (`Types.scala` at the 3.8.4 tag: `widenExpr frozen_<:<`, a method type's `compareMethod`
    /// and a polymorphic one's `comparePoly`): two values by their types; two methods of the same
    /// shape whose type parameters' bounds in `b` lie within those in `a` (`matchingPolyParams`),
    /// whose parameter types are the same (`matchingMethodParams`), whose clause of `b` is
    /// implicit only where `a`'s is, and whose result in `a` conforms to `b`'s. `b`'s type
    /// parameters are read as `a`'s.
    fn info_overrides(&mut self, a: &MethodSig, b: &MethodSig) -> bool {
        let (va, vb) = (Self::is_value_info(a), Self::is_value_info(b));
        if va != vb {
            return false;
        }
        if va {
            return self.frozen_sub(a.ret, b.ret);
        }
        if a.tparams.len() != b.tparams.len() || a.clauses.len() != b.clauses.len() {
            return false;
        }
        for (&p, &q) in a.tparams.iter().zip(&b.tparams) {
            if p == q {
                continue;
            }
            let (pl, pu) = self.renamed_bounds(p, &a.tparams, &a.tparams);
            let (ql, qu) = self.renamed_bounds(q, &b.tparams, &a.tparams);
            if !(self.frozen_sub(pl, ql) && self.frozen_sub(qu, pu)) {
                return false;
            }
        }
        for (x, y) in a.clauses.iter().zip(&b.clauses) {
            if x.params.len() != y.params.len() || ((y.is_using || y.is_implicit) && !(x.is_using || x.is_implicit)) {
                return false;
            }
            for (p, q) in x.params.iter().zip(&y.params) {
                if p.by_name != q.by_name || p.repeated != q.repeated || !(self.frozen_sub(p.ty, q.ty) && self.frozen_sub(q.ty, p.ty)) {
                    return false;
                }
            }
        }
        self.frozen_sub(a.ret, b.ret)
    }

    /// `info_overrides`, and where the match is loose (`overrides`' `matchLoosely`) the two
    /// compared again with a method of one empty clause read as a value of its result
    /// (`widenNullaryMethod`), where one of them is such a method.
    fn info_overrides_loosely(&mut self, a: &MethodSig, b: &MethodSig, loose: bool) -> bool {
        if self.info_overrides(a, b) {
            return true;
        }
        if !loose {
            return false;
        }
        let nullary = |info: &MethodSig| match (info.tparams.is_empty(), info.clauses.as_slice()) {
            (true, [c]) if c.params.is_empty() && !c.is_using => Some(MethodSig::value(info.ret)),
            _ => None,
        };
        match (nullary(a), nullary(b)) {
            (None, None) => false,
            (wa, wb) => {
                let (wa, wb) = (wa.unwrap_or_else(|| a.clone()), wb.unwrap_or_else(|| b.clone()));
                self.info_overrides(&wa, &wb)
            }
        }
    }

    /// The joint information of two members' (`infoMeet`, `Denotations.scala` 541 at the 3.8.4
    /// tag), `b`'s type parameters read as `a`'s: two values' the meet of their types; two methods'
    /// of the same type parameters and clauses whose parameter types are the same and whose
    /// clauses are implicit alike (`matchingMethodParams`, `isImplicitMethod`) the first's
    /// parameters, a name the two disagree on synthetic (`mergeParamNames`: `x$0`), the type
    /// parameters' bounds met, the results met; none (`NoType`) for any other pair, a value and a
    /// method among them.
    fn info_meet(&mut self, a: &MethodSig, b: &MethodSig) -> Option<MethodSig> {
        let (va, vb) = (Self::is_value_info(a), Self::is_value_info(b));
        if va && vb {
            let ret = self.met_type(a.ret, b.ret);
            return Some(MethodSig::value(ret));
        }
        if va || vb || a.tparams.len() != b.tparams.len() || a.clauses.len() != b.clauses.len() {
            return None;
        }
        let mut clauses = a.clauses.clone();
        for (clause, y) in clauses.iter_mut().zip(&b.clauses) {
            if clause.params.len() != y.params.len() || (clause.is_using || clause.is_implicit) != (y.is_using || y.is_implicit) {
                return None;
            }
            for (i, (p, q)) in clause.params.iter_mut().zip(&y.params).enumerate() {
                if p.by_name != q.by_name || p.repeated != q.repeated || !(self.frozen_sub(p.ty, q.ty) && self.frozen_sub(q.ty, p.ty)) {
                    return None;
                }
                if p.name != q.name {
                    p.name = self.interner.intern(&format!("x${}", i));
                }
            }
        }
        let ret = self.met_type(a.ret, b.ret);
        let mut joint = MethodSig { tparams: a.tparams.clone(), clauses, ret, declared: crate::tir::NO_TYPE };
        self.meet_tparam_bounds(&mut joint, &b.tparams);
        Some(joint)
    }

    /// The bounds of `joint`'s type parameters met with those of `theirs` (`paramInfos & ...` of
    /// `infoMeet`'s polymorphic case): where they differ, fresh parameters of the met bounds stand
    /// for the first's.
    fn meet_tparam_bounds(&mut self, joint: &mut MethodSig, theirs: &[TParamId]) {
        let mut fresh: Subst = Vec::new();
        let mut bounds = Vec::new();
        let own = joint.tparams.clone();
        for (&p, &q) in own.iter().zip(theirs) {
            if p == q {
                continue;
            }
            let (pl, pu) = self.renamed_bounds(p, &own, &own);
            // The second's bounds as they stand, its own binders not read as the first's: the
            // tag's `infoMeet` meets `tp2.paramInfos` unsubstituted (`Denotations.scala` 575),
            // where `matchingPolyParams` substitutes them to compare (`info_overrides`).
            let (ql, qu) = self.renamed_bounds(q, theirs, theirs);
            if pl == ql && pu == qu {
                continue;
            }
            let lower = if self.frozen_sub(ql, pl) { pl } else if self.frozen_sub(pl, ql) { ql } else { self.types.union(pl, ql) };
            let upper = self.met_type(pu, qu);
            if lower == pl && upper == pu {
                continue;
            }
            bounds.push((p, lower, upper));
        }
        if bounds.is_empty() {
            return;
        }
        for &(p, _, _) in &bounds {
            let (name, variance) = (self.syms.tparam(p).name, self.syms.tparam(p).variance);
            let q = self.syms.new_tparam(name, variance);
            fresh.push((p, self.types.param(q)));
        }
        for (k, &(_, lower, upper)) in bounds.iter().enumerate() {
            let Type::Param(q) = self.types.get(fresh[k].1) else { continue };
            let (lower, upper) = (self.types.subst(lower, &fresh), self.types.subst(upper, &fresh));
            self.syms.tparams[q.idx()].lower = lower;
            self.syms.tparams[q.idx()].upper = upper;
        }
        for t in &mut joint.tparams {
            if let Some(&(_, q)) = fresh.iter().find(|(p, _)| p == t) {
                if let Type::Param(q) = self.types.get(q) {
                    *t = q;
                }
            }
        }
        for clause in &mut joint.clauses {
            for p in &mut clause.params {
                p.ty = self.types.subst(p.ty, &fresh);
            }
        }
        joint.ret = self.types.subst(joint.ret, &fresh);
    }
    /// `mergeSingleDenot`'s `symScore` of two existing symbols, neither a bridge: above zero for
    /// `s1`, below for `s2`.
    pub(super) fn meet_score(&mut self, bases: &[ClassId], s1: SymId, s2: SymId) -> i32 {
        // dotty's `Deferred`: a member without a body, which an `abstract override` one with its
        // body (a trait's stackable member, `ABSTRACT` and `OVERRIDE` from a pickle) is not.
        let (d1, d2) = (self.is_deferred_member(s1), self.is_deferred_member(s2));
        if d1 && !d2 {
            return -1;
        }
        if d2 && !d1 {
            return 1;
        }
        let owner = |w: &Self, s: SymId| match w.syms.sym(s).owner {
            Owner::Class(c) => Some(c),
            _ => None,
        };
        if let (Some(o1), Some(o2)) = (owner(self, s1), owner(self, s2)) {
            if o1 != o2 {
                self.complete_class(o1);
                self.complete_class(o2);
                let derives = |w: &Self, a: ClassId, b: ClassId| w.syms.class(a).base_types.iter().any(|&(c, _)| c == b);
                if derives(self, o1, o2) {
                    return 1;
                }
                if derives(self, o2, o1) {
                    return -1;
                }
                if let Some(&c) = bases.iter().find(|&&c| c == o1 || c == o2) {
                    return if c == o1 { 1 } else { -1 };
                }
            }
        }
        // `accessBoundary`: a private member's is its owner, a protected or qualified one's
        // a package, a public one's the root; the one properly contained in the other's loses.
        let boundary = |w: &Self, s: SymId| {
            let m = w.syms.sym(s).mods;
            if m & crate::ast::mods::PRIVATE != 0 && !w.has_access_scope(w.syms.sym(s).file, w.syms.sym(s).span.start) {
                2
            } else if m & (crate::ast::mods::PRIVATE | crate::ast::mods::PROTECTED) != 0 {
                1
            } else {
                0
            }
        };
        let (b1, b2) = (boundary(self, s1), boundary(self, s2));
        if b1 != b2 {
            return if b1 > b2 { -1 } else { 1 };
        }
        match (self.is_method_sym(s1), self.is_method_sym(s2)) {
            (false, true) => -1,
            (true, false) => 1,
            _ => 0,
        }
    }

    /// The base classes of a receiver's type in the order dotty's `baseClasses` gives them
    /// (`Types.scala` at the 3.8.4 tag: a class's linearisation; a proxy its super type's, here
    /// a path's underlying type, an alias's expansion, a parameter's bound, a refinement's
    /// parent, an opaque type's bound outside its scope; an intersection's, `AndType.baseClasses`,
    /// the second part's classes not among the first's, up to their first common class, then
    /// the first's; a union's, `OrType.baseClasses`, the second part's classes the first shares,
    /// traits up to the first common class, then the rest of the second's; a class's `this` with a
    /// self type its `givenSelf & cls`), none for a type with no class.
    pub(super) fn receiver_base_classes(&mut self, t: TypeId, depth: u32) -> Vec<ClassId> {
        if depth > 8 {
            return Vec::new();
        }
        if let Type::This(c) = self.types.get(t) {
            if let Some(given) = self.syms.class(c).declared_self {
                let cls = self.types.class(c, &[]);
                let inter = self.types.mk(Type::Inter(given, cls));
                return self.receiver_base_classes(inter, depth + 1);
            }
        }
        let t = self.widen_path(t);
        let t = self.dealias(t);
        match self.types.get(t) {
            Type::Class(c, _) if self.syms.class(c).kind == ClassKind::Opaque && !self.transparent.contains(&c) => match self.opaque_bound(t) {
                Some(bound) => self.receiver_base_classes(bound, depth + 1),
                None => vec![c],
            },
            Type::Class(c, _) | Type::Ctor(c) | Type::This(c) => {
                self.complete_class(c);
                self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect()
            }
            Type::Param(p) | Type::AppParam(p, _) => {
                let upper = self.syms.tparam(p).upper;
                self.receiver_base_classes(upper, depth + 1)
            }
            // A nested class seen through its prefix (`p.C`): the class's.
            Type::Nested(_, class) => self.receiver_base_classes(class, depth + 1),
            Type::Refined(parent, _) => self.receiver_base_classes(parent, depth + 1),
            Type::Inter(a, b) => {
                let bcs1 = self.receiver_base_classes(a, depth + 1);
                let bcs2 = self.receiver_base_classes(b, depth + 1);
                let mut out = Vec::new();
                for bc2 in bcs2 {
                    if bcs1.contains(&bc2) {
                        if self.syms.class(bc2).kind == ClassKind::Trait {
                            continue;
                        }
                        break;
                    }
                    out.push(bc2);
                }
                out.extend(bcs1);
                out
            }
            Type::Union(a, b, _) => {
                let bcs1 = self.receiver_base_classes(a, depth + 1);
                let bcs2 = self.receiver_base_classes(b, depth + 1);
                let mut out = Vec::new();
                for (i, &bc2) in bcs2.iter().enumerate() {
                    if bcs1.contains(&bc2) {
                        if self.syms.class(bc2).kind != ClassKind::Trait {
                            out.extend_from_slice(&bcs2[i..]);
                            break;
                        }
                        out.push(bc2);
                    }
                }
                out
            }
            _ => Vec::new(),
        }
    }

    /// Whether two methods found on the two sides of an intersection are overloads of one
    /// name rather than one overriding the other: some alternative of the second takes
    /// parameters that none of the first does (`body(file: File)` of a trait next to
    /// `body(b: SttpFile)` of its self type).
    fn overloads_across(&mut self, a: SymId, ta: TypeId, b: SymId, tb: TypeId) -> bool {
        let alts = |t: &mut Self, s: SymId| -> Vec<SymId> { t.syms.alternatives(s).map_or_else(|| vec![s], |l| l.to_vec()) };
        let (xs, ys) = (alts(self, a), alts(self, b));
        if !xs.iter().chain(&ys).all(|&s| self.is_method_sym(s)) {
            return false;
        }
        // Arities first, which tells most overloads apart without substituting a type.
        let arities: Vec<_> = xs.iter().map(|&s| self.arity_shape(s)).collect();
        if ys.iter().any(|&s| !arities.contains(&self.arity_shape(s))) {
            return true;
        }
        ys.iter().any(|&y| !self.matches_among(y, tb, &xs, ta))
    }

    /// Whether some alternative of `xs` (on `ta`) matches `y` (on `tb`): `members_match`.
    fn matches_among(&mut self, y: SymId, tb: TypeId, xs: &[SymId], ta: TypeId) -> bool {
        xs.iter().any(|&x| self.members_match(x, ta, y, tb))
    }

    /// Whether `x`, a member of the instance `ta` of its owner, and `y`, of `tb`, match as one
    /// member, as dotty's `SingleDenotation.matches` has it (`Denotations.scala`, `matchesLoosely`,
    /// at the 3.8.4 tag) before any information is merged: their signatures, the type parameters'
    /// number and the parameters' erasures (a type parameter's its bound's), are consistent, and
    /// where their results erase differently (`ParamMatch`) their parameter types are the same,
    /// `y`'s type parameters read as `x`'s (`matchingMethodParams`). `f[T](x: T)` and `f[U <:
    /// String](x: U)` do not match: their parameters erase to `Object` and `String`.
    pub(super) fn members_match(&mut self, x: SymId, ta: TypeId, y: SymId, tb: TypeId) -> bool {
        if x == y && ta == tb {
            return true;
        }
        if self.arity_shape(x) != self.arity_shape(y) {
            return false;
        }
        let (px, rx) = self.erased_signature_of(x, ta);
        let (py, ry) = self.erased_signature_of(y, tb);
        if px != py {
            return false;
        }
        if rx == ry {
            return true;
        }
        let own = self.parameters_of(x).tparams.clone();
        let shape = self.parameter_shape(x, ta, None);
        self.parameter_shape(y, tb, Some(&own)) == shape
    }

    /// The classes the parameters and the result of `s` erase to, seen from its owner's
    /// instance `owner_ty`: a by-name parameter a `Function0`, a repeated one a `Seq`, a type
    /// parameter its bound (dotty's `Signature`, `paramsSig` and `resSig`).
    fn erased_signature_of(&mut self, s: SymId, owner_ty: TypeId) -> (Vec<ClassId>, ClassId) {
        let sig = self.sig_arc(s);
        let subst = self.owner_subst(owner_ty);
        let mut params = Vec::new();
        for p in sig.clauses.iter().flat_map(|c| c.params.iter()) {
            let erased = if p.by_name {
                self.b.functions.first().copied().flatten().unwrap_or(self.b.any_ref)
            } else if p.repeated {
                self.b.seq.unwrap_or(self.b.any_ref)
            } else {
                let t = self.types.subst(p.ty, &subst);
                self.erased_class(t, 0)
            };
            params.push(erased);
        }
        let ret = self.types.subst(sig.ret, &subst);
        (params, self.erased_class(ret, 0))
    }

    fn arity_shape(&mut self, s: SymId) -> (usize, Vec<usize>) {
        let sig = self.parameters_of(s);
        (sig.tparams.len(), sig.clauses.iter().map(|c| c.params.len()).collect())
    }

    /// The parameter types of `s` seen from its owner's instance `owner_ty`, with the arity of
    /// its type parameters: what tells an overload from an override across an intersection.
    /// Every input of the shape is in the reader's view before the shape is made of it (the
    /// canonicalisation rule): the parameters through
    /// `parameters_of`, the owner's arguments by the caller, the owner's bounds, read raw from
    /// its type parameters, translated here. `renamed`: type parameters of another method its
    /// own are read as, by position, where they are as many.
    fn parameter_shape(&mut self, s: SymId, owner_ty: TypeId, renamed: Option<&[TParamId]>) -> (usize, Vec<(TypeId, bool, bool)>) {
        let sig = self.parameters_of(s);
        let tparams = sig.tparams.len();
        let own = sig.tparams.clone();
        let mut params: Vec<(TypeId, bool, bool)> = sig.clauses.iter().flat_map(|c| c.params.iter()).map(|p| (p.ty, p.by_name, p.repeated)).collect();
        let mut subst = self.owner_subst(owner_ty);
        if let Some(to) = renamed.filter(|to| to.len() == tparams && tparams > 0) {
            subst.extend(own.into_iter().zip(to).map(|(p, &q)| (p, self.types.param(q))));
        }
        let view = self.types.view_here();
        if view != View::None {
            for (_, t) in &mut subst {
                *t = self.types.translate(view, *t);
            }
        }
        for p in &mut params {
            p.0 = self.types.subst(p.0, &subst);
        }
        (tparams, params)
    }

    /// The overloaded set joining the alternatives of `a` (on `ta`) and `b` (on `tb`), made and
    /// published under the loader's lock as the shared allocation it is (`with_loader`).
    fn merged_overload(&mut self, a: SymId, ta: TypeId, b: SymId, tb: TypeId) -> SymId {
        self.with_loader(|w| w.merged_overload_unlocked(a, ta, b, tb))
    }

    /// Under the loader's lock, whose holder's namespace is the base: the owners' types a
    /// worker passes in are canonicalised into the base first (exported with their parts), so
    /// that the shapes compared below are made of base types alone, whichever worker holds the
    /// lock (the canonicalisation rule).
    fn merged_overload_unlocked(&mut self, a: SymId, ta: TypeId, b: SymId, tb: TypeId) -> SymId {
        if let Some(&set) = self.merged_overloads.get(&(a, b)) {
            return set;
        }
        // Two members of single names are the two alternatives whatever their instances (their
        // signatures do not match, or their informations do not merge); of an overloaded name,
        // the second's alternatives that match none of the first's join them.
        let alts = if a == b {
            vec![a]
        } else if self.syms.alternatives(a).is_none() && self.syms.alternatives(b).is_none() {
            vec![a, b]
        } else {
            let view = self.types.view_here();
            let (ta, tb) = (self.types.translate(view, ta), self.types.translate(view, tb));
            let mut alts: Vec<SymId> = self.syms.alternatives(a).map_or_else(|| vec![a], |l| l.to_vec());
            let own = alts.clone();
            for s in self.syms.alternatives(b).map_or_else(|| vec![b], |l| l.to_vec()) {
                if !self.matches_among(s, tb, &own, ta) {
                    alts.push(s);
                }
            }
            alts
        };
        let owner = self.syms.sym(a).owner;
        // A view of members of other classes, which neither marks them as overloads nor their
        // class as having some (their names in the output stay their own).
        let set = self.syms.new_overloaded_view(a, owner, alts);
        self.merged_overloads.insert((a, b), set);
        set
    }

    /// Substitution mapping the owner class's type parameters to the arguments of `owner_ty`;
    /// of `p.C`, the enclosing classes' too, to the arguments of the prefix's base types.
    pub fn owner_subst(&mut self, owner_ty: TypeId) -> Subst {
        match self.types.get(owner_ty) {
            Type::Nested(p, class) => {
                let mut subst = self.owner_subst(class);
                if let Type::Class(c, _) = self.types.get(class) {
                    subst.extend(self.outer_subst(p, c));
                }
                subst
            }
            Type::Class(c, args) => {
                self.settle_class(c);
                // A class of no type parameters, most receivers, substitutes nothing: neither its
                // parameters nor the type's arguments are copied.
                if self.syms.class(c).tparams.is_empty() {
                    return Subst::new();
                }
                // The parameters and the arguments read in place, one pair at a time.
                let n = self.syms.class(c).tparams.len().min(self.types.items(args).len());
                let mut subst = Vec::with_capacity(n);
                for i in 0..n {
                    let (p, a) = (self.syms.class(c).tparams[i], self.types.items(args)[i]);
                    // A member read through an open wildcard has the parameter's declared
                    // bound, as under scalac (`other.toLocalDate` on a `ChronoLocalDateTime[?]`
                    // is a `ChronoLocalDate`); the bound, read raw from the parameter's record,
                    // in the reader's view before it is looked at (the canonicalisation rule).
                    if a != WILD {
                        subst.push((p, a));
                        continue;
                    }
                    let upper = self.syms.tparam(p).upper;
                    let upper = self.types.translate(self.types.view_here(), upper);
                    let bounded = matches!(self.types.get(upper), Type::Class(_, bargs) if self.types.items(bargs).is_empty());
                    subst.push((p, if bounded { upper } else { a }));
                }
                subst
            }
            _ => Vec::new(),
        }
    }
}

/// The dependency flags of a variable named at variance `s` (`coDeps` where `s >= 0`,
/// `contraDeps` where `s <= 0`, the `Adjuster`'s test).
fn dep_flags(s: i8) -> u8 {
    match s {
        1 => DEP_CO,
        -1 => DEP_CONTRA,
        _ => DEP_CO | DEP_CONTRA,
    }
}

/// Flags `f` of a dependency on a variable, carried to a variable its instance names at `s`.
fn compose_dep(f: u8, s: i8) -> u8 {
    match s {
        1 => f,
        -1 => ((f & DEP_CO) << 1) | ((f & DEP_CONTRA) >> 1),
        _ if f != 0 => DEP_CO | DEP_CONTRA,
        _ => 0,
    }
}
