use super::{Redo, TVarInfo, Worker, Undo};
use crate::intern::Name;
use crate::symbols::*;
use crate::types::*;
use crate::tir::{TExpr, TExprId};

const CLASS_OF_DEPTH: u32 = 64;

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
            }
        }
    }

    pub fn replay(&mut self, undone: Vec<Redo>) {
        for redo in undone.into_iter().rev() {
            match redo {
                Redo::Inst(v, t) => self.instantiate(v, t),
                Redo::Lower(v, t) => {
                    self.tvars[v].lower.push(t);
                    self.trail.push(Undo::Lower(v));
                }
                Redo::Upper(v, t) => {
                    self.tvars[v].upper.push(t);
                    self.trail.push(Undo::Upper(v));
                }
            }
        }
    }

    pub(super) fn instantiate(&mut self, v: TVarId, t: TypeId) {
        self.tvars[v].inst = Some(t);
        self.trail.push(Undo::Inst(v));
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
            Type::Union(a, b) => {
                let (za, zb) = (self.zonk(a), self.zonk(b));
                self.types.union(za, zb)
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
            // `v <: w` between two open variables is kept as a pair of bounds.
            let v_ty = self.types.mk(Type::Var(v));
            if !self.tvars[w].lower.contains(&v_ty) {
                self.tvars[w].lower.push(v_ty);
                self.trail.push(Undo::Lower(w));
            }
        }
        // Recorded before the lower bounds are checked so that cyclic constraints terminate.
        self.tvars[v].upper.push(b);
        self.trail.push(Undo::Upper(v));
        let lowers = self.tvars[v].lower.clone();
        lowers.into_iter().all(|l| self.is_sub(l, b))
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
        let uppers = self.tvars[v].upper.clone();
        uppers.into_iter().all(|u| self.is_sub(a, u))
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
        if self.is_serializable_type(b) && !matches!(self.types.get(a), Type::Var(_) | Type::Error | Type::BoundedWild(..)) && self.is_serializable_by_rule(a) {
            return true;
        }
        if b == self.b.t_enum && !matches!(self.types.get(a), Type::Var(_) | Type::Error | Type::BoundedWild(..)) && self.is_enum_value(a) {
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
            (Type::Union(x, y), _) => return self.is_sub(x, b) && self.is_sub(y, b),
            (_, Type::Inter(x, y)) => return self.is_sub(a, x) && self.is_sub(a, y),
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
            let mark = self.snapshot();
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
        if let Type::Lit(_) = ta {
            let class = self.widen_lit(a);
            return self.is_sub(class, b);
        }
        let dependent = |t: Type| {
            matches!(t, Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..))
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
                let ctor = if n == m {
                    self.types.mk(Type::Ctor(c))
                } else {
                    // Partial unification: the leftmost arguments are fixed, the rest abstracted
                    // over. Each fixed argument is an open variable the class's own argument
                    // bounds, so that later operands join it as scalac joins the constructors
                    // it infers: `(Validated[E1, A], Validated[E2, B])` as `(F[A], F[B])` makes
                    // `F` a `Validated[E1 | E2, _]`.
                    let fixed: Vec<TypeId> = (0..m - n).map(|_| self.fresh_var()).collect();
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
                let Type::Class(_, args1) = self.types.get(base) else { return false };
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
                        let (xlo, xhi) = self.types.wild_bounds(xs[i]).unwrap_or((xs[i], xs[i]));
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
                if let Type::Union(x, y) = tb {
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
                Type::Union(x, y) => self.is_sub(x, b) && self.is_sub(y, b),
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
            Type::Term(v) => match self.declared_singleton(v) {
                Some(u) => self.stable_path(u, depth + 1),
                None => Some(t),
            },
            Type::Select(p, v) => {
                let np = self.stable_path(p, depth + 1)?;
                match self.declared_singleton(v) {
                    Some(u) => self.stable_path(u, depth + 1),
                    None if np == p => Some(t),
                    None => Some(self.types.mk(Type::Select(np, v))),
                }
            }
            _ => None,
        }
    }

    /// The singleton type a stable val is declared with, if any.
    fn declared_singleton(&self, v: SymId) -> Option<TypeId> {
        let info = self.syms.sym(v);
        if !matches!(info.kind, SymKind::Val | SymKind::Param) || info.by_name || info.mods & crate::ast::mods::MUTABLE != 0 {
            return None;
        }
        let ret = info.sig.as_ref()?.ret;
        matches!(self.types.get(ret), Type::This(_) | Type::Term(_) | Type::Select(..)).then_some(ret)
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

    fn is_sub_dependent_inner(&mut self, a: TypeId, b: TypeId) -> bool {
        let (ta, tb) = (self.types.get(a), self.types.get(b));
        match (ta, tb) {
            (Type::AppMember(m, xs), Type::AppMember(n, ys)) if m == n || self.member_through_prefix(m, n) => {
                let variances = self.member_variances(m);
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
                if self.types.items(xs).len() == self.types.items(ys).len() =>
            {
                self.instantiate(v, m);
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
                    _ => false,
                };
            }
            Type::Refined(parent, r) => {
                let mark = self.snapshot();
                if !self.is_sub(a, parent) {
                    return false;
                }
                if self.refinement_holds(a, r) {
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
            if let Type::Union(p, q) = self.types.get(side) {
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
            Type::Union(x, y) => {
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
            };
            bound.map_or(false, |t| self.mentions_tparam_of(t, Some(&binders)))
        });
        if escaped {
            self.rollback(mark);
            return false;
        }
        true
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
                if self.loaded.is_some() && self.jdk_class_unrelated(c, target) {
                    return None;
                }
                self.complete_class(c);
                let found = self.syms.class(c).base_types.iter().find(|(bc, _)| *bc == target).map(|&(_, t)| t);
                let bt = match found {
                    Some(bt) => bt,
                    None if c == self.b.string => self.string_interface(target)?,
                    None if c == self.b.array => self.array_interface(target)?,
                    None if self.b.js_dynamic.is_some() && !self.jvm && matches!(self.syms.class(target).name, crate::names::FUNCTION | crate::names::OBJECT) && self.is_function_class(c) => self.function_interface(target)?,
                    None => return None,
                };
                let subst: Subst = self
                    .syms
                    .class(c)
                    .tparams
                    .iter()
                    .copied()
                    .zip(self.types.items(args).iter().copied())
                    .collect();
                Some(self.types.subst(bt, &subst))
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
            Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..) | Type::Match(..) | Type::Alias(..) => {
                let under = self.dependent_underlying(t)?;
                self.base_type(under, target)
            }
            _ => None,
        }
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
            Type::Match(..) | Type::Alias(..) => {
                let under = self.reduce_head(t).unwrap_or_else(|| self.match_bound(t));
                (under != t && under != ANY).then_some(under)
            }
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
            LitVal::Char(_) => self.b.t_char,
            LitVal::Bool(_) => self.b.t_boolean,
            LitVal::Str(_) => self.b.t_string,
        }
    }

    /// The type of a `val` or `def` without a declared type: solved and widened.
    pub fn solve_inferred(&mut self, t: TypeId) -> TypeId {
        let t = self.solve_in(t);
        let t = self.normalize(t);
        self.widen_lit(t)
    }

    /// Whether a literal type occurs in `t`, looking through the bounds of an open variable.
    pub fn mentions_lit(&mut self, t: TypeId) -> bool {
        let mut seen = Vec::new();
        self.mentions_lit_in(t, &mut seen)
    }

    /// `seen` holds the variables whose bounds are being looked through: an F-bounded variable
    /// (`?T <: Ord[?T]`) names itself in them.
    fn mentions_lit_in(&mut self, t: TypeId, seen: &mut Vec<TVarId>) -> bool {
        let t = self.deref_alias(t);
        match self.types.get(t) {
            Type::Lit(_) => true,
            Type::Class(_, args) if self.opaque_underlying(t).is_some() => {
                let u = self.opaque_underlying(t).unwrap();
                self.mentions_lit_in(u, seen) || self.list_mentions_lit(args, seen)
            }
            Type::Class(_, args) | Type::AppVar(_, args) | Type::AppParam(_, args) => self.list_mentions_lit(args, seen),
            Type::Union(a, b) | Type::Inter(a, b) => self.mentions_lit_in(a, seen) || self.mentions_lit_in(b, seen),
            Type::Var(v) => {
                let (upper, lower) = (self.tvars[v].upper.len(), self.tvars[v].lower.len());
                if upper + lower == 0 || seen.contains(&v) {
                    return false;
                }
                seen.push(v);
                (0..upper + lower).any(|i| {
                    let info = &self.tvars[v];
                    let b = if i < upper { info.upper[i] } else { info.lower[i - upper] };
                    let b = self.deref(b);
                    !matches!(self.types.get(b), Type::Var(_)) && self.mentions_lit_in(b, seen)
                })
            }
            _ => false,
        }
    }

    fn list_mentions_lit(&mut self, l: TList, seen: &mut Vec<TVarId>) -> bool {
        (0..self.types.items(l).len()).any(|i| {
            let a = self.types.items(l)[i];
            self.mentions_lit_in(a, seen)
        })
    }

    /// The rank of a numeric primitive type (`prims::R_BYTE` to `R_DOUBLE`).
    #[inline]
    /// Whether `t` is `java.io.Serializable`, which a product by rule is (`is_serializable_by_rule`):
    /// its name compared first, as an id.
    fn is_serializable_type(&self, t: TypeId) -> bool {
        let Type::Class(c, args) = self.types.get(t) else { return false };
        if args != EMPTY_LIST {
            return false;
        }
        let name = match self.b.serializable_name.get() {
            Some(n) => n,
            None => match self.interner.lookup("Serializable") {
                Some(n) => {
                    self.b.serializable_name.set(Some(n));
                    n
                }
                None => return false,
            },
        };
        self.syms.class(c).name == name && self.syms.is_synthetic_parent(&self.interner, c)
    }

    pub fn is_numeric(&self, t: TypeId) -> Option<u8> {
        match self.b.num_rank.get(t.idx()) {
            Some(&r) if r != super::NO_RANK => Some(r),
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
    /// base type; unrelated types join to their union, member by member for unions.
    pub fn lub(&mut self, a: TypeId, b: TypeId) -> TypeId {
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
            acc = self.types.union(acc, m);
        }
        acc
    }

    /// Marks `e` soft where `lub` gave it a union: the join of an `if`'s branches, a match's
    /// cases or a `try`'s.
    pub fn note_soft(&mut self, e: TExprId, t: TypeId) {
        if let Type::Union(..) = self.types.get(t) {
            self.soft_exprs.insert(e, ());
        }
    }

    /// Whether `e` is a soft union: a join expression, a block ending in one, or a binder of a
    /// soft scrutinee.
    pub fn is_soft(&self, e: TExprId) -> bool {
        let mut e = e;
        loop {
            if self.soft_exprs.contains_key(&e) {
                return true;
            }
            match self.prog.expr(e) {
                TExpr::Block(_, res) => e = res,
                TExpr::Local(s) => return self.soft_syms.contains_key(&s),
                _ => return false,
            }
        }
    }

    /// dotc's `widenInferred` on a union: the soft union of a join expression widens to its
    /// join at an inference boundary, a val's or def's inferred type, a lambda's inferred
    /// result, unless the join is transparent (`Any`, `AnyVal`, `Matchable`, `Product`,
    /// `Serializable`), where the union stays (`Int | String`). A union written in the program
    /// is kept (`val copy = hard`).
    pub fn widen_soft(&mut self, e: TExprId, t: TypeId) -> TypeId {
        if !self.is_soft(e) {
            return t;
        }
        self.widen_union_join(t)
    }

    /// The join a soft union widens to, unless it is transparent.
    pub fn widen_union_join(&mut self, t: TypeId) -> TypeId {
        match self.union_join(t) {
            Some(j) if j != ERROR && !self.transparent_join(j) => j,
            _ => t,
        }
    }

    fn transparent_join(&mut self, j: TypeId) -> bool {
        match self.types.get(j) {
            Type::Any => true,
            Type::Inter(a, b) => self.transparent_join(a) && self.transparent_join(b),
            Type::Class(c, _) => {
                c == self.b.any_ref
                    || c == self.b.any_val
                    || Some(c) == self.b.product
                    || matches!(self.interner.get(self.syms.class(c).name), "Matchable" | "Serializable" | "Equals" | "Object")
            }
            _ => false,
        }
    }

    fn join(&mut self, a: TypeId, b: TypeId) -> Option<TypeId> {
        if a == b {
            return Some(a);
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
        minimal.reverse();
        let (&first, rest) = minimal.split_first()?;
        Some(rest.iter().fold(first, |acc, &t| self.types.inter(acc, t)))
    }

    fn lub_as_class(&mut self, a: TypeId, b: TypeId, c: ClassId) -> Option<TypeId> {
        // `c` is a base of `a` already; whether `b` shares it is the question.
        let y = self.base_type(b, c)?;
        let x = self.base_type(a, c)?;
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
                -1 => self.types.inter(xs[i], ys[i]),
                _ if xs[i] == ys[i] => xs[i],
                // The join for member selection reads differing invariant arguments as a
                // bounded wildcard, dotc's `lubArgs`: `? >: glb <: lub` of the two, each read
                // through the bounds it may carry from an earlier join.
                _ if self.join_wild => self.wild_join(xs[i], ys[i]),
                _ => return None,
            });
        }
        Some(self.types.class(c, &merged))
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
        self.types.union(a, b)
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
            Type::Inter(x, y) => {
                let mark = self.trail.len();
                self.is_sub(x, self.b.t_singleton) || {
                    self.rollback(mark);
                    self.is_sub(y, self.b.t_singleton)
                }
            }
            Type::Union(x, y) => self.is_sub(x, self.b.t_singleton) && self.is_sub(y, self.b.t_singleton),
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
            Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..) => {
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
            Type::Union(a, b) => self.is_numeric_union(a) && self.is_numeric_union(b),
            _ => self.is_numeric(t).is_some(),
        }
    }

    fn solve_var_inner(&mut self, v: TVarId, result: Option<TypeId>) -> bool {
        // Nothing is known about it: the choice below would be `Nothing` after every step. Most
        // such variables are what a rolled-back candidate of a nested search left.
        if self.tvars[v].lower.is_empty() && self.tvars[v].upper.is_empty() {
            self.tvars[v].inst = Some(NOTHING);
            self.trail.push(Undo::Inst(v));
            return true;
        }
        // A bound that is the variable itself, left by another variable that was unified with
        // it and then instantiated to it, says nothing.
        let itself = self.types.mk(Type::Var(v));
        let lowers: Vec<TypeId> = self.tvars[v].lower.clone().into_iter().filter(|&l| self.deref(l) != itself).collect();
        let uppers: Vec<TypeId> = self.tvars[v].upper.clone().into_iter().filter(|&u| self.deref(u) != itself).collect();
        // Bounded above and below by an open variable of an enclosing application, it is that
        // variable: `A` of `pure(x)` against a `Fr[B]`, which the other branches of an `if` join
        // into (`pure(Left(e))` and `pure(Right(r))` make `B` an `Either`, not a `Left`).
        for &l in &lowers {
            if let Some(w) = self.outer_var(v, l) {
                let w_ty = self.types.mk(Type::Var(w));
                if uppers.iter().any(|&u| self.deref(u) == w_ty) {
                    self.tvars[v].inst = Some(w_ty);
                    self.trail.push(Undo::Inst(v));
                    return uppers.into_iter().all(|u| self.is_sub(w_ty, u));
                }
            }
        }
        let mut choice = NOTHING;
        let mut outer_lowers = Vec::new();
        let outer_joined = std::mem::replace(&mut self.joined_soft, false);
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
        // the upper bounds' then, where there are any.
        if choice == NOTHING && !uppers.is_empty() && !result.map_or(false, |r| self.taken_from_below(v, &uppers, r)) {
            choice = self.choice_from_uppers(v, &uppers, false);
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
        let choice = self.widen_inferred(choice, &uppers);
        // A union joined from several lower bounds is soft, as dotc's `lub` makes it, and
        // widens with the instance (`List(new Box[Int], new Box[String])`); one lower bound
        // that is a union written in the program stays.
        let joined_soft = std::mem::replace(&mut self.joined_soft, outer_joined);
        let choice = {
            let widened = if joined_soft { self.widen_union_join(choice) } else { choice };
            if widened == choice {
                choice
            } else {
                let mark = self.snapshot();
                if uppers.iter().all(|&u| self.is_sub(widened, u)) {
                    widened
                } else {
                    self.rollback(mark);
                    choice
                }
            }
        };
        // An F-bounded variable with nothing below it (`T <: Ord[T]`) is `Nothing`, never a
        // type that contains itself.
        let choice =
            if !uppers.is_empty() && self.types.has_vars(choice) && self.occurs(v, choice) { NOTHING } else { choice };
        self.tvars[v].inst = Some(choice);
        self.trail.push(Undo::Inst(v));
        uppers.into_iter().all(|u| self.is_sub(choice, u))
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

    fn occurs(&self, v: TVarId, t: TypeId) -> bool {
        let mut vars = Vec::new();
        self.collect_vars(t, &mut vars);
        vars.contains(&v)
    }

    /// An instance widens its literal and singleton types as long as it stays within the upper
    /// bounds, as dotc's `widenInferred` does, so `id(c)` for a `c: 1` is an `Int` unless a `1`
    /// is expected of it, and `id(own(x))` over `def own(x: B): x.type` a `B`.
    fn widen_inferred(&mut self, choice: TypeId, uppers: &[TypeId]) -> TypeId {
        // A variable bounded by `Singleton` keeps its instance, as dotc's `isSingletonBounded`
        // does (`new A(r)` for `A[R <: Refl & Singleton]` is an `A[r.type]`).
        if self.singleton_bounded(uppers) {
            return choice;
        }
        let widened = self.widen_singletons(choice);
        let widened = self.widen_lit(widened);
        if widened == choice {
            return choice;
        }
        let mark = self.snapshot();
        if uppers.iter().all(|&u| self.is_sub(widened, u)) {
            widened
        } else {
            self.rollback(mark);
            choice
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

    /// `t` with its singleton types widened, the sides of an intersection too (dotc's
    /// `widenSingletons`): `x.type & B` over `x: B` is a `B`.
    fn widen_singletons(&mut self, t: TypeId) -> TypeId {
        if self.types.is_path(t) {
            return self.widen_path(t);
        }
        match self.types.get(t) {
            Type::Inter(a, b) => {
                let (wa, wb) = (self.widen_singletons(a), self.widen_singletons(b));
                if wa == a && wb == b { t } else { self.types.inter(wa, wb) }
            }
            _ => t,
        }
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
        let joined = self.lub(acc, l);
        if let Type::Union(..) = self.types.get(joined) {
            self.joined_soft = true;
        }
        joined
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
            Type::Union(a, b) | Type::Inter(a, b) => {
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
                let variances: Vec<i8> = self.syms.class(c).tparams.iter().map(|&p| self.syms.tparam(p).variance).collect();
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
                let variances: Vec<i8> = match self.types.get(m) {
                    Type::Member(p, n) => self.type_member_alias(p, n).map(|a| self.syms.aliases[a.idx()].tparams.iter().map(|&q| self.syms.tparam(q).variance).collect()).unwrap_or_default(),
                    _ => Vec::new(),
                };
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
            Type::Union(a, b) | Type::Inter(a, b) => {
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
            Type::Union(a, b) | Type::Inter(a, b) => {
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
    pub fn find_member(&mut self, t: TypeId, name: Name) -> Option<(SymId, TypeId)> {
        self.find_member_from(t, name, 1)
    }

    /// The member that is left when the private one of the class itself cannot be accessed: a
    /// constructor parameter may share its name with a member the class inherits.
    pub fn find_inherited_member(&mut self, t: TypeId, name: Name) -> Option<(SymId, TypeId)> {
        self.find_member_from(t, name, 0)
    }

    fn find_member_in_bound(&mut self, p: TParamId, upper: TypeId, name: Name, private_until: usize) -> Option<(SymId, TypeId)> {
        if let Type::Inter(a, b) = self.types.get(upper) {
            let mentions = |t: &mut Self, part: TypeId| t.mentions_tparam_of(part, Some(&[p]));
            if !mentions(self, a) && mentions(self, b) {
                if let Some(found) = self.find_member_from(b, name, 0) {
                    return Some(found);
                }
            }
        }
        self.find_member_from(upper, name, private_until)
    }

    /// Private members count in the first `private_until` classes of the linearisation; one of
    /// an ancestor is not inherited.
    fn find_member_from(&mut self, t: TypeId, name: Name, private_until: usize) -> Option<(SymId, TypeId)> {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Class(c, args) => {
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
                let (bc, mut sym, bt) = match found {
                    Some(f) => f,
                    None => {
                        // A member of `Any` is `universal_member`'s: reading a Java class for it
                        // would give a std class the JDK's members where the JDK is open alone.
                        if self.loaded.is_some() && !self.bare_lookup && !crate::names::ANY_MEMBERS.contains(&self.interner.get(name)) && self.java_member_miss(c) {
                            if self.loaded.as_ref().unwrap().detail {
                                eprintln!("java members of {} read for {}", self.class_path(c), self.name_str(name));
                            }
                            return self.find_member_from(t, name, private_until);
                        }
                        return None;
                    }
                };
                match self.settled(bc, name, sym) {
                    Some(settled) => sym = settled,
                    None => return self.find_member_from(t, name, private_until),
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
            // Of an F-bounded parameter, `C <: LinearSeq[A] & LinearSeqOps[A, CC, C]`, the part
            // that names `C` gives the member its most specific type (`tail: C`), as the
            // intersection's member has under scalac.
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                if upper == ANY || self.bound_path.contains(&p) {
                    return None;
                }
                self.bound_path.push(p);
                let found = self.find_member_in_bound(p, upper, name, private_until);
                self.bound_path.pop();
                found
            }
            Type::AppParam(p, args) => {
                let upper = self.app_param_upper(p, args)?;
                self.find_member_from(upper, name, private_until)
            }
            // The second member of a self type's intersection is another class, whose private
            // members are its own; a method of another shape there overloads the first's.
            Type::Inter(a, b) => {
                let first = self.find_member_from(a, name, private_until);
                let second = self.find_member_from(b, name, 0);
                match (first, second) {
                    (Some((sa, ta)), Some((sb, tb))) if sa != sb && self.overloads_across(sa, ta, sb, tb) => {
                        Some((self.merged_overload(sa, ta, sb, tb), ta))
                    }
                    (Some(fa), Some(fb)) if fa.0 != fb.0 => Some(self.narrower_part_member(fa, fb)),
                    (Some(f), _) => Some(f),
                    (None, f) => f,
                }
            }
            Type::Union(..) => {
                let j = self.union_join(t)?;
                self.find_member_from(j, name, private_until)
            }
            Type::Lit(_) => {
                let class = self.widen_lit(t);
                self.find_member_from(class, name, private_until)
            }
            Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Refined(..) | Type::Alias(..) | Type::Match(..) => {
                let under = self.dependent_underlying(t)?;
                self.find_member_from(under, name, private_until)
            }
            _ => None,
        }
    }

    /// Whether two methods found on the two sides of an intersection are overloads of one
    /// name rather than one overriding the other: some alternative of the second takes
    /// parameters that none of the first does (`body(file: File)` of a trait next to
    /// `body(b: SttpFile)` of its self type).
    /// Of a member both parts of an intersection declare with one shape, the part whose result
    /// type is the narrower (`value: String` of `A` over `value: Any` of `B` in a `B & A`), as
    /// scalac's member of an intersection has the meet of the two; the first part's where
    /// neither result conforms to the other.
    fn narrower_part_member(&mut self, (sa, ta): (SymId, TypeId), (sb, tb): (SymId, TypeId)) -> (SymId, TypeId) {
        let result = |t: &mut Self, s: SymId, owner: TypeId| {
            let ret = t.sig_of(s).ret;
            let subst = t.owner_subst(owner);
            t.types.subst(ret, &subst)
        };
        let (ra, rb) = (result(self, sa, ta), result(self, sb, tb));
        if ra == rb || self.types.contains_error(ra) || self.types.contains_error(rb) {
            return (sa, ta);
        }
        let mark = self.snapshot();
        let b_narrower = self.is_sub(rb, ra) && self.snapshot() == mark;
        self.rollback(mark);
        let a_narrower = b_narrower && self.is_sub(ra, rb);
        self.rollback(mark);
        if b_narrower && !a_narrower { (sb, tb) } else { (sa, ta) }
    }

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
        let shapes: Vec<_> = xs.iter().map(|&s| self.parameter_shape(s, ta)).collect();
        ys.iter().any(|&s| {
            let sh = self.parameter_shape(s, tb);
            !shapes.contains(&sh)
        })
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
    /// its type parameters, translated here.
    fn parameter_shape(&mut self, s: SymId, owner_ty: TypeId) -> (usize, Vec<(TypeId, bool, bool)>) {
        let sig = self.parameters_of(s);
        let tparams = sig.tparams.len();
        let mut params: Vec<(TypeId, bool, bool)> = sig.clauses.iter().flat_map(|c| c.params.iter()).map(|p| (p.ty, p.by_name, p.repeated)).collect();
        let mut subst = self.owner_subst(owner_ty);
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
        let view = self.types.view_here();
        let (ta, tb) = (self.types.translate(view, ta), self.types.translate(view, tb));
        let mut alts: Vec<SymId> = self.syms.alternatives(a).map_or_else(|| vec![a], |l| l.to_vec());
        let shapes: Vec<_> = alts.iter().map(|&s| self.parameter_shape(s, ta)).collect();
        for s in self.syms.alternatives(b).map_or_else(|| vec![b], |l| l.to_vec()) {
            if !shapes.contains(&self.parameter_shape(s, tb)) {
                alts.push(s);
            }
        }
        let owner = self.syms.sym(a).owner;
        let set = self.syms.new_overloaded(a, owner, alts);
        self.merged_overloads.insert((a, b), set);
        set
    }

    /// Substitution mapping the owner class's type parameters to the arguments of `owner_ty`.
    pub fn owner_subst(&mut self, owner_ty: TypeId) -> Subst {
        match self.types.get(owner_ty) {
            Type::Class(c, args) => {
                self.settle_class(c);
                let tparams = self.syms.class(c).tparams.clone();
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                tparams
                    .into_iter()
                    .zip(items)
                    .map(|(p, a)| {
                        // A member read through an open wildcard has the parameter's declared
                        // bound, as under scalac (`other.toLocalDate` on a `ChronoLocalDateTime[?]`
                        // is a `ChronoLocalDate`); the bound, read raw from the parameter's record,
                        // in the reader's view before it is looked at (the canonicalisation rule).
                        if a != WILD {
                            return (p, a);
                        }
                        let upper = self.syms.tparam(p).upper;
                        let upper = self.types.translate(self.types.view_here(), upper);
                        let bounded = matches!(self.types.get(upper), Type::Class(_, bargs) if self.types.items(bargs).is_empty());
                        (p, if bounded { upper } else { a })
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }
}
