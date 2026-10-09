//! Type members seen through a prefix: `x.T`, `C.this.T`, `C#T`, and the refinements that fix
//! them. A member that the prefix fixes is replaced by what it equals as soon as it is looked
//! up, so a `Type::Member` in the store is always abstract and is compared by its prefix and
//! name, as scalac compares type references.

use super::Worker;
use crate::intern::Name;
use crate::symbols::*;
use crate::types::*;

/// What a type member is through a prefix.
#[derive(Clone, Copy)]
pub enum MemberInfo {
    Alias(TypeId),
    Bounds(TypeId, TypeId),
}

/// The deepest nesting of bounds through bounds that a lookup follows before it gives up;
/// cyclic bounds are reported where they are declared.
/// The shape of a `Blocked` type standing for a member a path cannot resolve.
pub(super) const UNRESOLVED_MEMBER: &str = "unresolved member";

const MAX_DEPTH: u32 = 32;
const MAX_STEPS: u32 = 4096;

impl<'a> Worker<'a> {
    /// The prefix of the type members of `c` inside its body: `C.this`, or the object itself.
    pub fn this_prefix(&mut self, c: ClassId) -> TypeId {
        if self.syms.class(c).kind == ClassKind::Object {
            self.types.class(c, &[])
        } else {
            self.types.mk(Type::This(c))
        }
    }

    /// `p.Inner` for a class nested in a generic class that `p`'s type derives from.
    #[cold]
    fn inner_class_member(&mut self, prefix: TypeId, name: Name) -> Option<TypeId> {
        let under = if self.types.is_path(prefix) { self.path_underlying(prefix) } else { prefix };
        let under = self.dealias(under);
        let c = match self.types.get(under) {
            Type::Class(c, _) | Type::This(c) => c,
            _ => return None,
        };
        self.complete_class(c);
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        let inner = bases.into_iter().find_map(|b| self.syms.class(b).nested.get(&name).copied())?;
        if self.syms.class(inner).outer_tparams == 0 {
            return None;
        }
        self.inner_class_through(inner, prefix)
    }

    /// `p.A` for a type parameter `A` of the class of `p`'s type, which only a library body
    /// names: the argument `p`'s type gives it, a wildcard included, as scalac names the
    /// capture of a wildcard (`val v: kv._2.head.A` of a `Slot[?]`).
    #[cold]
    fn class_param_member(&mut self, prefix: TypeId, name: Name) -> Option<TypeId> {
        if !self.types.is_path(prefix) || !self.is_body_file(self.env.file) {
            return None;
        }
        let under = self.path_underlying(prefix);
        let under = self.dealias(under);
        let Type::Class(c, args) = self.types.get(under) else { return None };
        self.complete_class_tparams(c);
        let index = self.syms.class(c).tparams.iter().position(|&tp| self.syms.tparam(tp).name == name)?;
        self.types.items(args).get(index).copied()
    }

    /// A class nested in a generic class seen through `prefix`: the enclosing class's
    /// arguments as the prefix gives them, then the class's own parameters, over which it is a
    /// lambda when it has any. None where the prefix is no instance of the enclosing class.
    pub fn inner_class_through(&mut self, c: ClassId, prefix: TypeId) -> Option<TypeId> {
        let info = self.syms.class(c);
        let n = info.outer_tparams as usize;
        let Owner::Class(o) = info.owner else { return None };
        let own: Vec<TParamId> = info.tparams[n..].to_vec();
        let base = self.base_type(prefix, o)?;
        let Type::Class(_, args) = self.types.get(base) else { return None };
        let mut all = self.types.items(args).to_vec();
        if all.len() != n {
            return None;
        }
        let own_types: Vec<TypeId> = own.iter().map(|&p| self.types.param(p)).collect();
        all.extend_from_slice(&own_types);
        let class = self.types.class(c, &all);
        if own.is_empty() {
            return Some(class);
        }
        let l = self.types.list(&own_types);
        Some(self.types.mk(Type::Lambda(l, class)))
    }

    /// A type another module's class exports (`car.Fuel` of `class Car: export Engine.*`): its
    /// pickle's forwarder is no type member of the class, the class's export table holds it.
    fn product_exported_type(&mut self, prefix: TypeId, name: Name) -> Option<TypeId> {
        let under = match self.types.get(prefix) {
            Type::Class(..) => prefix,
            _ => self.path_underlying(prefix),
        };
        let Type::Class(c, _) = self.types.get(under) else { return None };
        if !self.syms.class(c).has_exports || !self.loaded.as_ref().map_or(false, |l| l.is_product_class(c)) {
            return None;
        }
        let t = *self.exports_of(c)?.types.get(&name)?;
        Some(self.type_ref_to_type(t, crate::source::Span::default(), false))
    }

    /// The type member `name` seen through `prefix`: what it equals where the prefix fixes
    /// it, the abstract member otherwise, None where the prefix has no such member.
    pub fn member_type(&mut self, prefix: TypeId, name: Name) -> Option<TypeId> {
        if !self.enter_lookup(prefix, name) {
            return None;
        }
        let prefix = self.stable_root(prefix);
        let found = match self.type_member(prefix, name) {
            Some(MemberInfo::Alias(t)) => Some(t),
            Some(MemberInfo::Bounds(..)) => Some(self.types.mk(Type::Member(prefix, name))),
            None => self.inner_class_member(prefix, name).or_else(|| self.class_param_member(prefix, name)).or_else(|| self.product_exported_type(prefix, name)),
        };
        self.leave_lookup();
        if self.dep_depth == 0 && self.dep_exhausted {
            return Some(ERROR);
        }
        found
    }

    fn leave_lookup(&mut self) {
        self.dep_depth -= 1;
        self.lookups.pop();
    }

    /// Whether the bounds of the member `name` through `prefix` are being resolved up the stack.
    pub(super) fn bounds_lookup_in_progress(&self, prefix: TypeId, name: Name) -> bool {
        self.lookups.iter().any(|&(p, n, bounds)| bounds && p == prefix && n == name)
    }

    /// Starts the step budget for a dependent comparison made outside any lookup; a cycle met
    /// under it is reported at the member on either side, if one is a member.
    pub(super) fn start_lookup_budget(&mut self, a: TypeId, b: TypeId) {
        self.dep_budget = MAX_STEPS;
        self.dep_exhausted = false;
        self.dep_root = match (self.types.get(a), self.types.get(b)) {
            (Type::Member(p, n), _) | (_, Type::Member(p, n)) => (p, n),
            _ => (a, crate::names::EMPTY),
        };
    }

    /// Counts a lookup against the depth and the step budget of the outermost one under way;
    /// false, with the cycle reported, once either is used up.
    fn enter_lookup(&mut self, prefix: TypeId, name: Name) -> bool {
        // A lookup that branches at every level (members defined through each other's paths)
        // is bounded by the number of steps as well as by the depth.
        if self.dep_depth == 0 {
            self.dep_budget = MAX_STEPS;
            self.dep_root = (prefix, name);
            self.dep_exhausted = false;
        }
        if self.dep_depth >= MAX_DEPTH || self.dep_budget == 0 {
            let (root, root_name) = self.dep_root;
            self.report_member_cycle(root, root_name, false);
            return false;
        }
        self.dep_budget -= 1;
        self.dep_depth += 1;
        self.lookups.push((prefix, name, false));
        true
    }

    /// A class nested in the object or class a path leads to: `q.reflect.Symbol` for
    /// `q: Quotes`, whose `reflect` is an object.
    pub fn nested_class_through(&mut self, prefix: TypeId, name: Name) -> Option<ClassId> {
        let under = if self.types.is_path(prefix) { self.path_underlying(prefix) } else { prefix };
        let under = self.dealias(under);
        let c = match self.types.get(under) {
            Type::Class(c, _) | Type::This(c) => c,
            _ => return None,
        };
        self.complete_class(c);
        if let Some(&n) = self.syms.class(c).nested.get(&name) {
            return Some(n);
        }
        // A class a base declares (`Cache` of `TypeModule`, through the object `Type`); an
        // opaque type of a trait is the object's own copy of it.
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).filter(|&b| b != c).collect();
        let n = bases.into_iter().find_map(|b| {
            self.complete_class(b);
            self.syms.class(b).nested.get(&name).copied()
        })?;
        if self.types.is_path_class(n) && self.syms.class(c).kind == ClassKind::Object {
            return Some(self.derived_opaque(n, c));
        }
        Some(n)
    }

    /// The path a path stands for: a val declared with a singleton type (`val b: a.type`)
    /// names that path, so `b.T` is `a.T`.
    fn stable_root(&mut self, p: TypeId) -> TypeId {
        let mut p = p;
        for _ in 0..MAX_DEPTH {
            if !matches!(self.types.get(p), Type::Term(_) | Type::Select(..)) {
                return p;
            }
            let under = self.path_underlying(p);
            if !self.types.is_path(under) {
                return p;
            }
            p = under;
        }
        p
    }

    /// The alias or bounds of the type member `name` through `prefix`, both as seen from it.
    pub fn type_member(&mut self, prefix: TypeId, name: Name) -> Option<MemberInfo> {
        if !self.enter_lookup(prefix, name) {
            return None;
        }
        let found = match self.types.get(prefix) {
            // `C.this` has the members of the self type as well, seen from `C.this`: they are
            // path-dependent through it, and only a receiver that is the self type resolves them.
            Type::This(c) => self.member_in_class(c, EMPTY_LIST, name, prefix).or_else(|| {
                let k = self.self_type_member_class(c, name)?;
                self.member_in_class(k, EMPTY_LIST, name, prefix)
            }),
            Type::Class(c, args) => self.member_in_class(c, args, name, prefix),
            _ => {
                let under = self.path_underlying(prefix);
                self.member_in_type(under, name, prefix)
            }
        };
        self.leave_lookup();
        // An alias that comes back to itself through the members it names (`type A = z.B` and
        // `type B = z.A` on two sides of `z`'s type) is a cycle.
        if matches!(found, Some(MemberInfo::Alias(x)) if matches!(self.types.get(x), Type::Member(p, n) if p == prefix && n == name)) {
            self.dep_exhausted = true;
            self.report_member_cycle(prefix, name, true);
        }
        if self.dep_depth == 0 && self.dep_exhausted {
            return Some(MemberInfo::Alias(ERROR));
        }
        found
    }

    /// A lookup that never ends: members defined through each other across the traits a class
    /// mixes in (`type A = B` with `type B = A`). Reported once per member, at its class.
    #[cold]
    fn report_member_cycle(&mut self, prefix: TypeId, name: Name, at_use: bool) {
        // The class is read off the prefix without another lookup, which the spent budget
        // would cut short again.
        let c = match self.types.get(prefix) {
            Type::This(c) | Type::Class(c, _) => c,
            Type::Term(_) | Type::Select(..) if at_use && self.cycle_at_use.is_some() => {
                self.cycle_at_use = Some(true);
                return;
            }
            Type::Term(s) | Type::Select(_, s) => match self.syms.sym(s).owner {
                Owner::Class(c) => c,
                _ => return,
            },
            _ => return,
        };
        if self.member_cycles.contains(&(c, name)) {
            return;
        }
        self.member_cycles.push((c, name));
        let (file, span) = {
            let i = self.syms.class(c);
            (i.file, i.span)
        };
        let msg = format!(
            "illegal cyclic type reference: alias {}.{} of type {} refers back to the type itself",
            self.name_str(self.syms.class(c).name),
            self.name_str(name),
            self.name_str(name)
        );
        self.diags.error(file, span, msg);
    }

    fn member_in_type(&mut self, t: TypeId, name: Name, prefix: TypeId) -> Option<MemberInfo> {
        let t = self.dealias(t);
        match self.types.get(t) {
            Type::Refined(parent, r) => match self.types.refinement(r) {
                Refinement::Alias(n, rhs) if n == name => Some(MemberInfo::Alias(rhs)),
                Refinement::Bounds(n, lo, hi) if n == name => Some(MemberInfo::Bounds(lo, hi)),
                _ => self.member_in_type(parent, name, prefix),
            },
            Type::Class(c, args) => self.member_in_class(c, args, name, prefix),
            Type::This(c) => self.member_in_class(c, EMPTY_LIST, name, prefix),
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                (upper != ANY).then(|| self.member_in_type(upper, name, prefix)).flatten()
            }
            Type::AppParam(p, args) => {
                let upper = self.app_param_upper(p, args)?;
                self.member_in_type(upper, name, prefix)
            }
            Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => {
                let (_, upper) = self.member_bounds(t);
                (upper != ANY).then(|| self.member_in_type(upper, name, prefix)).flatten()
            }
            Type::Term(_) | Type::Select(..) => {
                let under = self.path_underlying(t);
                self.member_in_type(under, name, prefix)
            }
            Type::Inter(a, b) => match self.member_in_type(a, name, prefix) {
                Some(MemberInfo::Bounds(lo, hi)) => {
                    let other = self.member_in_type(b, name, prefix);
                    Some(self.bounds_meet(lo, hi, other))
                }
                Some(alias @ MemberInfo::Alias(_)) => match self.member_in_type(b, name, prefix) {
                    Some(MemberInfo::Bounds(lo, hi)) => Some(self.bounds_meet(lo, hi, Some(alias))),
                    _ => Some(alias),
                },
                None => self.member_in_type(b, name, prefix),
            },
            Type::Union(..) => {
                let j = self.union_join(t)?;
                self.member_in_type(j, name, prefix)
            }
            _ => None,
        }
    }

    /// An intersection's member declared abstract on one side and `other` on the other: an
    /// alias within the bounds implements it, one outside them intersects with them
    /// (`>: x | lo <: x & hi`), as scalac's `&` of a `TypeBounds` and a `TypeAlias`, and two
    /// bounds meet (`>: lo | lo2 <: hi & hi2`).
    fn bounds_meet(&mut self, lo: TypeId, hi: TypeId, other: Option<MemberInfo>) -> MemberInfo {
        match other {
            Some(MemberInfo::Alias(x)) if self.alias_within(x, lo, hi) => MemberInfo::Alias(x),
            Some(MemberInfo::Alias(x)) => {
                let lo = if lo == NOTHING { x } else { self.types.union(x, lo) };
                MemberInfo::Bounds(lo, self.types.inter(x, hi))
            }
            Some(MemberInfo::Bounds(lo2, hi2)) => {
                let met_lo = if lo == NOTHING { lo2 } else if lo2 == NOTHING || lo2 == lo { lo } else { self.types.union(lo, lo2) };
                let met_hi = if hi == ANY { hi2 } else if hi2 == ANY || hi2 == hi { hi } else { self.types.inter(hi, hi2) };
                // Bounds shown to conflict (`L <: Nothing` and `L >: Any`, ground types) are no
                // type scalac lets a value have; the first side's stand. Bounds over parameters
                // or members meet, whether or not they can be compared here.
                let mark = self.trail.len();
                if met_lo == NOTHING || met_hi == ANY || !self.is_ground(met_lo) || !self.is_ground(met_hi) || self.is_sub(met_lo, met_hi) {
                    MemberInfo::Bounds(met_lo, met_hi)
                } else {
                    self.rollback(mark);
                    MemberInfo::Bounds(lo, hi)
                }
            }
            None => MemberInfo::Bounds(lo, hi),
        }
    }

    /// A type without parameters, variables or paths: one a comparison decides.
    fn is_ground(&mut self, t: TypeId) -> bool {
        !self.types.has_vars(t) && !self.types.has_paths(t) && !self.mentions_param(t)
    }

    fn alias_within(&mut self, x: TypeId, lo: TypeId, hi: TypeId) -> bool {
        let mark = self.trail.len();
        let within = (hi == ANY || self.is_sub(x, hi)) && (lo == NOTHING || self.is_sub(lo, x));
        if !within {
            self.rollback(mark);
        }
        within
    }

    /// The member `name` of the class `c` applied to `args`, through its own declarations and
    /// those it inherits, as seen from `prefix`.
    fn member_in_class(&mut self, c: ClassId, args: TList, name: Name, prefix: TypeId) -> Option<MemberInfo> {
        self.complete_class(c);
        // The first definition in the linearisation, unless a later one is an alias where it
        // is abstract: an alias implements the abstract members of every trait mixed in.
        let mut found = None;
        for &(b, bt) in &self.syms.class(c).base_types {
            if let Some(&a) = self.syms.class(b).type_aliases.get(&name) {
                let concrete = !self.syms.aliases[a.idx()].is_abstract();
                if found.is_none() || concrete {
                    found = Some((b, a, bt));
                }
                if concrete {
                    break;
                }
            }
        }
        let (b, a, bt) = found?;
        // A member whose own bounds this thread is resolving is named in them (`type T <:
        // Ordered[T]`): the bounds it has so far are what such a reference needs. Another worker's
        // completion is waited for, its record until then the namer's placeholder. An alias named
        // in its own right-hand side is a cycle, which completing it again reports.
        if self.syms.alias(a).state() != Completion::Done {
            if self.forked {
                self.order_alias_readers(a);
            }
            let info = self.syms.alias(a);
            if info.state() != Completion::InProgress || !info.state().mine() || !info.is_abstract() {
                self.complete_alias(a);
            }
        }
        let (mut rhs, mut bounds, tparams) = {
            let i = &self.syms.aliases[a.idx()];
            (i.rhs, i.bounds, i.tparams.clone())
        };
        if bounds.is_some() && self.syms.class(b).kind == ClassKind::Object {
            let decl = self.types.mk(Type::Decl(a));
            return Some(MemberInfo::Alias(decl));
        }
        // The declaration is in terms of `b`'s parameters, which `bt` states in terms of `c`'s,
        // and of `b`'s own `this`.
        let mut subst = self.owner_subst(bt);
        if args != EMPTY_LIST {
            self.settle_class(c);
            let outer: Subst = self.syms.class(c).tparams.iter().copied().zip(self.types.items(args).iter().copied()).collect();
            for (_, t) in subst.iter_mut() {
                *t = self.types.subst(*t, &outer);
            }
        }
        // A match-type alias is kept by name for its recursion, so one whose cases name the
        // class's parameters (`Kind`'s `LiftP` over `Mono`, `Head` and `Tail`) is copied with
        // them seen through the prefix, once per prefix.
        if self.is_match_alias(a) {
            let body = self.alias_body(a);
            let seen_body = self.types.subst(body, &subst);
            let seen_body = self.as_seen_from(seen_body, prefix, c);
            let alias = if seen_body != body { self.derived_match_alias(a, prefix, &subst, c) } else { a };
            return Some(MemberInfo::Alias(self.match_alias_ctor(alias)));
        }
        if !tparams.is_empty() && rhs != ERROR {
            // A bound of the alias's own parameters that names a parameter of the class
            // (`type ProductGeneric[O <: Kind.this.Up]` seen through `K0 extends Kind[Any, ..]`)
            // is seen through the prefix too, on fresh parameters.
            let bound_seen = |t: &mut Self, b: TypeId| -> TypeId {
                let b = t.types.subst(b, &subst);
                t.as_seen_from(b, prefix, c)
            };
            let bounds_through_prefix = tparams.iter().any(|&p| {
                let (lo, hi) = (self.syms.tparam(p).lower, self.syms.tparam(p).upper);
                bound_seen(self, lo) != lo || bound_seen(self, hi) != hi
            });
            let ps: Vec<TypeId> = if bounds_through_prefix {
                let fresh: Vec<TParamId> = tparams
                    .iter()
                    .map(|&p| {
                        let info = self.syms.tparam(p);
                        let (name, variance, arity, lo, hi) = (info.name, info.variance, info.arity, info.lower, info.upper);
                        let q = self.syms.new_tparam(name, variance);
                        self.syms.tparams[q.idx()].arity = arity;
                        self.syms.tparams[q.idx()].lower = bound_seen(self, lo);
                        self.syms.tparams[q.idx()].upper = bound_seen(self, hi);
                        q
                    })
                    .collect();
                let renaming: Subst = tparams.iter().zip(&fresh).map(|(&p, &q)| (p, self.types.param(q))).collect();
                rhs = self.types.subst(rhs, &renaming);
                fresh.iter().map(|&q| self.types.param(q)).collect()
            } else {
                tparams.iter().map(|&p| self.types.param(p)).collect()
            };
            let l = self.types.list(&ps);
            rhs = self.types.mk(Type::Lambda(l, rhs));
        }
        let own = b == c;
        let seen = |t: &mut Self, ty: TypeId| {
            let ty = t.types.subst(ty, &subst);
            let ty = if own { t.own_seen_from(ty, prefix, c) } else { t.as_seen_from(ty, prefix, c) };
            t.seen_from_outer_path(ty, prefix, c)
        };
        if let Some((lo, hi)) = bounds.as_mut() {
            if let Some(entry) = self.lookups.iter_mut().rev().find(|(_, n, _)| *n == name) {
                entry.2 = true;
            }
            *lo = seen(self, *lo);
            *hi = seen(self, *hi);
            return Some(MemberInfo::Bounds(*lo, *hi));
        }
        Some(MemberInfo::Alias(seen(self, rhs)))
    }

    /// `t`, a member of the object `c` nested in a class, seen through the path `p.o` to it: the
    /// enclosing class's `this` is `p` (`Exs.this.Type` in `Mod.this.ExistentialType.Bounded`
    /// for `Mod` whose self type derives from `Exs`), scalac's `asSeenFrom` one class out.
    fn seen_from_outer_path(&mut self, t: TypeId, prefix: TypeId, c: ClassId) -> TypeId {
        let Type::Select(p, s) = self.types.get(prefix) else { return t };
        let Owner::Class(k) = self.syms.class(c).owner else { return t };
        if self.syms.sym(s).owner != Owner::Class(k) || !self.types.has_paths(t) {
            return t;
        }
        self.as_seen_from(t, p, k)
    }

    /// The copy of the match-type alias `a` seen through `prefix`: its own parameters fresh
    /// with their bounds seen, its cases with the owner's parameters and `this` substituted,
    /// its recursion pointing at the copy.
    fn derived_match_alias(&mut self, a: AliasId, prefix: TypeId, subst: &Subst, c: ClassId) -> AliasId {
        if let Some(&d) = self.derived_aliases.get(&(a, prefix)) {
            debug_assert!(!(self.forked && self.syms.aliases.holding() && a.0 < crate::arena::LOCAL_BASE) || d.0 < crate::arena::LOCAL_BASE, "the loader's lock holder found a worker's own copy of a shared match alias");
            return d;
        }
        // In a forked build a shared alias's copy is the base's, made under the loader's lock over
        // the caller's types exported (the canonicalisation rule): one
        // alias for the worker and its holder, a shared record every view reads, which the memo
        // keeps under the caller's prefix as well; a worker's own copy would reach the library
        // bodies the holder types into the shared trees, which no merge renumbers.
        if self.forked && !self.syms.aliases.holding() && a.0 < crate::arena::LOCAL_BASE && c.0 < crate::arena::LOCAL_BASE {
            let d = self.with_loader(|w| {
                let view = w.types.view_here();
                let base_prefix = w.types.translate(view, prefix);
                let base_subst: Subst = subst.iter().map(|&(p, t)| (p, w.types.translate(view, t))).collect();
                w.derived_match_alias(a, base_prefix, &base_subst, c)
            });
            self.derived_aliases.insert((a, prefix), d);
            return d;
        }
        let (name, owner, file, tparams) = {
            let i = &self.syms.aliases[a.idx()];
            (i.name, i.owner, i.file, i.tparams.clone())
        };
        let rhs = self.alias_body(a);
        let d = AliasId(self.syms.aliases.len() as u32);
        self.syms.aliases.push(AliasInfo { name, owner, file, def: None, tparams: Vec::new(), rhs: ERROR, bounds: None });
        self.syms.alias_cells.set(d.0, Completion::Done);
        self.derived_aliases.insert((a, prefix), d);
        let seen = |t: &mut Self, ty: TypeId| -> TypeId {
            let ty = t.types.subst(ty, subst);
            t.as_seen_from(ty, prefix, c)
        };
        let fresh: Vec<TParamId> = tparams
            .iter()
            .map(|&p| {
                let info = self.syms.tparam(p);
                let (n, variance, arity, lo, hi) = (info.name, info.variance, info.arity, info.lower, info.upper);
                let q = self.syms.new_tparam(n, variance);
                self.syms.tparams[q.idx()].arity = arity;
                self.syms.tparams[q.idx()].lower = seen(self, lo);
                self.syms.tparams[q.idx()].upper = seen(self, hi);
                q
            })
            .collect();
        let renaming: Subst = tparams.iter().zip(&fresh).map(|(&p, &q)| (p, self.types.param(q))).collect();
        let body = self.types.subst(rhs, &renaming);
        let body = seen(self, body);
        let body = self.redirect_alias(body, a, d);
        let info = &mut self.syms.aliases[d.idx()];
        info.tparams = fresh;
        info.rhs = body;
        d
    }

    /// An opaque type declared in a class or trait is marked as naming its owner's `this`, so
    /// that a member's type seen from an object deriving from the owner has the object's own
    /// copy of it (`opaque_seen_from`).
    pub(super) fn mark_opaque_in_class(&mut self, c: ClassId) {
        if let Owner::Class(o) = self.syms.class(c).owner {
            if !matches!(self.syms.class(o).kind, ClassKind::Object | ClassKind::Builtin) {
                self.types.mark_path_class(c);
            }
        }
    }

    /// The opaque type `c` of a class or trait as the rebase sees it: through a prefix that is
    /// an object deriving from its owner, the object's copy, whose implicit scope is the
    /// object's, as scalac's `HtmlTagOf.Tag` has the prefix `HtmlTagOf` among its anchors.
    fn opaque_seen_from(&mut self, c: ClassId, r: &Rebase) -> ClassId {
        let Some((k, prefix)) = r.this_of else { return c };
        let Owner::Class(o) = self.syms.class(c).owner else { return c };
        let through = k == o || (!r.own && self.syms.class(k).base_types.iter().any(|&(b, _)| b == o));
        if !through {
            return c;
        }
        let under = if self.types.is_path(prefix) { self.path_underlying(prefix) } else { prefix };
        match self.types.get(under) {
            Type::Class(m, _) | Type::This(m) if self.syms.class(m).kind == ClassKind::Object => self.derived_opaque(c, m),
            _ => c,
        }
    }

    /// The copy of the opaque type `c`, declared in a class or trait, that the object `m`
    /// deriving from its owner has: owned by `m`, with the owner's parameters as `m` fixes
    /// them in its bounds and underlying type, and opaque everywhere, since scalac keeps an
    /// opaque type transparent in its defining template only.
    pub fn derived_opaque(&mut self, c: ClassId, m: ClassId) -> ClassId {
        // A copy found is every worker's (applied at a release after the class it names); a miss
        // is the lock's to answer, with the lookup made again under it.
        if self.forked {
            if let Some(&d) = self.derived_opaques.get(&(c, m)) {
                return d;
            }
        }
        self.with_loader(|w| w.derived_opaque_unlocked(c, m))
    }

    pub fn derived_opaque_unlocked(&mut self, c: ClassId, m: ClassId) -> ClassId {
        if let Some(&d) = self.derived_opaques.get(&(c, m)) {
            return d;
        }
        self.complete_class(c);
        self.complete_class(m);
        let Owner::Class(o) = self.syms.class(c).owner else { return c };
        let Some(bt) = self.syms.class(m).base_types.iter().find(|&&(b, _)| b == o).map(|&(_, bt)| bt) else { return c };
        let mut subst = self.owner_subst(bt);
        let info = self.syms.class(c);
        let (name, mods, file, span, tparams) = (info.name, info.mods, info.file, info.span, info.tparams.clone());
        let d = self.syms.new_class(name, ClassKind::Opaque, mods, Owner::Class(m), file, None, span);
        self.derived_opaques.insert((c, m), d);
        let fresh: Vec<TParamId> = tparams
            .iter()
            .map(|&p| {
                let info = self.syms.tparam(p);
                let (n, variance, arity) = (info.name, info.variance, info.arity);
                let q = self.syms.new_tparam(n, variance);
                self.syms.tparams[q.idx()].arity = arity;
                q
            })
            .collect();
        subst.extend(tparams.iter().zip(&fresh).map(|(&p, &q)| (p, self.types.param(q))));
        let this_of = |t: &mut Self, ty: TypeId| {
            let ty = t.types.subst(ty, &subst);
            let prefix = t.types.class(m, &[]);
            t.as_seen_from(ty, prefix, m)
        };
        for (&p, &q) in tparams.iter().zip(&fresh) {
            let (lo, hi) = (self.syms.tparam(p).lower, self.syms.tparam(p).upper);
            let (lo, hi) = (this_of(self, lo), this_of(self, hi));
            self.syms.tparams[q.idx()].lower = lo;
            self.syms.tparams[q.idx()].upper = hi;
        }
        let info = self.syms.class(c);
        let (underlying, parents, bases) = (info.underlying, info.parents.clone(), info.base_types.clone());
        let underlying = underlying.map(|u| this_of(self, u));
        let parents: Vec<TypeId> = parents.iter().map(|&p| this_of(self, p)).collect();
        let fresh_types: Vec<TypeId> = fresh.iter().map(|&q| self.types.param(q)).collect();
        let own = self.types.class(d, &fresh_types);
        let mut base_types = vec![(d, own)];
        for &(b, bt) in bases.iter().filter(|&&(b, _)| b != c) {
            base_types.push((b, this_of(self, bt)));
        }
        let mut info = self.syms.class_mut(d);
        info.tparams = fresh;
        info.underlying = underlying;
        info.parents = parents;
        info.base_types = base_types;
        info.state().set(Completion::Done);
        d
    }

    /// `t` with every application of a match-type alias of a generic class whose parameters
    /// `subst` binds (an inline body's `Kind.this.LiftP[F, T]` expanded on `K0`) applying
    /// the copy seen through the class instantiated by the substitution.
    pub fn specialise_aliases(&mut self, t: TypeId, subst: &Subst) -> TypeId {
        match self.types.get(t) {
            Type::Alias(a, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let n: Vec<TypeId> = items.iter().map(|&x| self.specialise_aliases(x, subst)).collect();
                let l = self.types.list(&n);
                let a = self.specialised_alias(a, subst).unwrap_or(a);
                self.types.mk(Type::Alias(a, l))
            }
            Type::Class(k, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let n: Vec<TypeId> = items.iter().map(|&x| self.specialise_aliases(x, subst)).collect();
                self.types.class(k, &n)
            }
            Type::AppParam(p, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let n: Vec<TypeId> = items.iter().map(|&x| self.specialise_aliases(x, subst)).collect();
                let l = self.types.list(&n);
                self.types.mk(Type::AppParam(p, l))
            }
            Type::Lambda(ps, b) => {
                let nb = self.specialise_aliases(b, subst);
                self.types.mk(Type::Lambda(ps, nb))
            }
            Type::Poly(ps, b) => {
                let nb = self.specialise_aliases(b, subst);
                let nps = self.map_poly_bounds(ps, &mut |w, x| w.specialise_aliases(x, subst));
                self.types.mk(Type::Poly(nps, nb))
            }
            Type::Union(x, y) => {
                let (nx, ny) = (self.specialise_aliases(x, subst), self.specialise_aliases(y, subst));
                self.types.union(nx, ny)
            }
            Type::Inter(x, y) => {
                let (nx, ny) = (self.specialise_aliases(x, subst), self.specialise_aliases(y, subst));
                self.types.inter(nx, ny)
            }
            _ => t,
        }
    }

    /// The copy of the match-type alias `a` for the owner class instantiated by `subst`, where
    /// the substitution binds a parameter of that class.
    fn specialised_alias(&mut self, a: AliasId, subst: &Subst) -> Option<AliasId> {
        let Owner::Class(k) = self.syms.aliases[a.idx()].owner else { return None };
        self.settle_class(k);
        let tparams = self.syms.class(k).tparams.clone();
        if tparams.is_empty() || !tparams.iter().any(|p| subst.iter().any(|&(q, _)| q == *p)) || !self.is_match_alias(a) {
            return None;
        }
        let args: Vec<TypeId> = tparams.iter().map(|&p| subst.iter().find(|&&(q, _)| q == p).map(|&(_, t)| t).unwrap_or_else(|| self.types.param(p))).collect();
        let prefix = self.types.class(k, &args);
        let owner: Subst = tparams.iter().copied().zip(args.iter().copied()).collect();
        Some(self.derived_match_alias(a, prefix, &owner, k))
    }

    /// `t` with every application of the alias `from` applying `to` instead.
    fn redirect_alias(&mut self, t: TypeId, from: AliasId, to: AliasId) -> TypeId {
        match self.types.get(t) {
            Type::Alias(a, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let n: Vec<TypeId> = items.iter().map(|&x| self.redirect_alias(x, from, to)).collect();
                let l = self.types.list(&n);
                self.types.mk(Type::Alias(if a == from { to } else { a }, l))
            }
            Type::Class(k, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let n: Vec<TypeId> = items.iter().map(|&x| self.redirect_alias(x, from, to)).collect();
                self.types.class(k, &n)
            }
            Type::AppParam(p, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let n: Vec<TypeId> = items.iter().map(|&x| self.redirect_alias(x, from, to)).collect();
                let l = self.types.list(&n);
                self.types.mk(Type::AppParam(p, l))
            }
            Type::AppMember(m, args) => {
                let m = self.redirect_alias(m, from, to);
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let n: Vec<TypeId> = items.iter().map(|&x| self.redirect_alias(x, from, to)).collect();
                let l = self.types.list(&n);
                self.types.mk(Type::AppMember(m, l))
            }
            Type::Lambda(ps, b) => {
                let nb = self.redirect_alias(b, from, to);
                self.types.mk(Type::Lambda(ps, nb))
            }
            Type::Poly(ps, b) => {
                let nb = self.redirect_alias(b, from, to);
                let nps = self.map_poly_bounds(ps, &mut |w, x| w.redirect_alias(x, from, to));
                self.types.mk(Type::Poly(nps, nb))
            }
            Type::Union(x, y) => {
                let (nx, ny) = (self.redirect_alias(x, from, to), self.redirect_alias(y, from, to));
                self.types.union(nx, ny)
            }
            Type::Inter(x, y) => {
                let (nx, ny) = (self.redirect_alias(x, from, to), self.redirect_alias(y, from, to));
                self.types.inter(nx, ny)
            }
            Type::Match(s, m) => {
                let ns = self.redirect_alias(s, from, to);
                let (cases, bound) = {
                    let info = self.types.match_info(m);
                    (info.cases.to_vec(), info.bound)
                };
                let nb = self.redirect_alias(bound, from, to);
                let ncases: Vec<MatchCase> = cases
                    .iter()
                    .map(|k| MatchCase { binders: k.binders, pattern: self.redirect_alias(k.pattern, from, to), body: self.redirect_alias(k.body, from, to) })
                    .collect();
                self.types.match_type(ns, &ncases, nb)
            }
            _ => t,
        }
    }

    /// The bounds of an abstract member type, `Any` and `Nothing` when it has none.
    pub fn member_bounds(&mut self, t: TypeId) -> (TypeId, TypeId) {
        match self.types.get(t) {
            Type::Member(prefix, name) => match self.type_member(prefix, name) {
                Some(MemberInfo::Bounds(lo, hi)) => (lo, hi),
                Some(MemberInfo::Alias(a)) => (a, a),
                None => (NOTHING, ANY),
            },
            Type::Decl(a) => {
                self.complete_alias(a);
                self.syms.aliases[a.idx()].bounds.unwrap_or((NOTHING, ANY))
            }
            Type::AppMember(m, args) => {
                let (lo, hi) = self.member_bounds(m);
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let apply = |t: &mut Self, bound: TypeId| match t.types.get(bound) {
                    Type::Lambda(..) | Type::Ctor(_) | Type::Param(_) => t.types.apply_ctor(bound, &items),
                    _ => bound,
                };
                (apply(self, lo), apply(self, hi))
            }
            _ => (NOTHING, ANY),
        }
    }

    /// The upper bound of an abstract member type, followed through members bounded by
    /// members, or the type itself when it is no member.
    pub fn member_upper(&mut self, t: TypeId) -> TypeId {
        let mut t = t;
        for _ in 0..MAX_DEPTH {
            if !matches!(self.types.get(t), Type::Member(..) | Type::AppMember(..) | Type::Decl(_)) {
                return t;
            }
            let (_, upper) = self.member_bounds(t);
            if upper == t {
                return ANY;
            }
            t = self.deref(upper);
        }
        ANY
    }

    /// The type a path stands for: the declared type of the val or parameter, the class of an
    /// object, `this` of a class.
    /// The literal type of a constant val that is declared wider (`final val x = 1`, `Int` in
    /// teq's signature, `1` in scalac's), which its singleton stands for.
    pub(super) fn constant_type(&mut self, s: SymId, ret: TypeId) -> Option<TypeId> {
        if !self.may_be_constant(s) || matches!(self.types.get(ret), Type::Lit(_)) {
            return None;
        }
        let value = match self.constant_value(s)? {
            crate::tir::TExpr::Int(v) => LitVal::Int(v),
            crate::tir::TExpr::Long(v) => LitVal::Long(v),
            crate::tir::TExpr::Double(v) => LitVal::Double(v.to_bits()),
            crate::tir::TExpr::Bool(v) => LitVal::Bool(v),
            crate::tir::TExpr::Char(v) => LitVal::Char(v),
            crate::tir::TExpr::Str(id) => LitVal::Str(self.interner.intern(&self.prog.strings[id.idx()])),
            _ => return None,
        };
        Some(self.types.lit(value))
    }

    /// Whether `s` can be a constant: a `final` or `inline` val that is no given.
    fn may_be_constant(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        info.kind == SymKind::Val && info.mods & (crate::ast::mods::FINAL | crate::ast::mods::INLINE) != 0 && info.mods & crate::ast::mods::GIVEN == 0
    }

    pub fn path_underlying(&mut self, t: TypeId) -> TypeId {
        match self.types.get(t) {
            Type::This(c) => {
                self.complete_class(c);
                self.syms.this_type(c)
            }
            Type::Term(s) => {
                let ret = self.sig_of(s).ret;
                self.constant_type(s, ret).unwrap_or_else(|| self.deref(ret))
            }
            Type::Select(p, s) => {
                if self.may_be_constant(s) {
                    let declared = self.sig_of(s).ret;
                    if let Some(lit) = self.constant_type(s, declared) {
                        return lit;
                    }
                }
                let name = self.syms.sym(s).name;
                let under = self.path_underlying(p);
                // A prefix whose type refines the member (`i: Insp { val q: x.type }`) gives
                // it the refinement's type.
                if let Some(Refinement::Val(_, _, ty)) = self.refined_term(under, name) {
                    return ty;
                }
                let ret = self.sig_of(s).ret;
                match self.find_member(under, name) {
                    Some((_, owner_ty)) => {
                        let subst = self.owner_subst(owner_ty);
                        let ty = self.types.subst(ret, &subst);
                        let ty = match self.class_of(under) {
                            Some(c) => self.as_seen_from(ty, p, c),
                            None => ty,
                        };
                        // The outer class's `this` left by a member of an inner class, as a
                        // selection of it is typed (`x.Underlying.type` of a pattern's `x`).
                        if self.types.has_paths(ty) && self.member_of_inner_class(s) {
                            self.seen_from_enclosing_this(ty)
                        } else {
                            ty
                        }
                    }
                    None => ret,
                }
            }
            _ => t,
        }
    }

    /// The widened type of a path or member type: what a value of it is at run time.
    pub fn widen_path(&mut self, t: TypeId) -> TypeId {
        let mut t = self.deref(t);
        for _ in 0..MAX_DEPTH {
            match self.types.get(t) {
                Type::This(_) | Type::Term(_) | Type::Select(..) => t = self.path_underlying(t),
                _ => return t,
            }
            t = self.deref(t);
        }
        t
    }

    /// `t` as seen from `prefix`, an instance of `c`: `C.this` of `c` and of its ancestors
    /// becomes the prefix, and a member seen through it is looked up anew, since the prefix may
    /// fix it.
    pub fn as_seen_from(&mut self, t: TypeId, prefix: TypeId, c: ClassId) -> TypeId {
        if !self.types.has_paths(t) {
            return t;
        }
        self.rebase(t, &Rebase { this_of: Some((c, prefix)), terms: &[], approx: false, own: false })
    }

    /// `t`, a member's own declaration in class `c`, as seen from `prefix`: `c.this` becomes
    /// the prefix, an ancestor's `this` stays what it is.
    pub(super) fn own_seen_from(&mut self, t: TypeId, prefix: TypeId, c: ClassId) -> TypeId {
        if !self.types.has_paths(t) {
            return t;
        }
        self.rebase(t, &Rebase { this_of: Some((c, prefix)), terms: &[], approx: false, own: true })
    }

    /// `t` with the paths of the parameters in `terms` replaced by the argument paths.
    pub fn subst_paths(&mut self, t: TypeId, terms: &[(SymId, TypeId)]) -> TypeId {
        if terms.is_empty() || !self.types.has_paths(t) {
            return t;
        }
        self.rebase(t, &Rebase { this_of: None, terms, approx: false, own: false })
    }

    /// `t` with every path over one of `params`, and every member seen through one
    /// (`fold.Out` in a candidate's result), replaced by a fresh inference variable; the
    /// members replaced and their variables are appended to `approx`, so that the caller can
    /// bind each variable once the argument's path is known.
    pub fn approx_paths(&mut self, t: TypeId, params: &[SymId], approx: &mut Vec<(TypeId, TypeId)>) -> TypeId {
        if params.is_empty() || !self.types.has_paths(t) {
            return t;
        }
        let terms: Vec<(SymId, TypeId)> = params.iter().map(|&p| (p, self.fresh_var())).collect();
        let mark = self.path_approx.len();
        let out = self.rebase(t, &Rebase { this_of: None, terms: &terms, approx: true, own: false });
        approx.extend(self.path_approx.drain(mark..));
        out
    }

    /// The variable standing for a member seen through a parameter's path, one per member.
    fn approx_member(&mut self, t: TypeId) -> TypeId {
        if let Some(&(_, v)) = self.path_approx.iter().find(|&&(m, _)| m == t) {
            return v;
        }
        let v = self.fresh_var();
        self.path_approx.push((t, v));
        v
    }

    /// The signature a term refinement gives its member: a `val`'s is the refinement's type. A
    /// generic member whose bounds the refinement's types no longer hold as its type parameters
    /// do (a path substituted in `A <: c.T`) takes type parameters of those bounds, the types
    /// over them (dotty's `derivedLambdaType` of the `PolyType` `TypeMap.mapOverLambda`
    /// mapped), the same ones for the same refinement.
    pub fn refinement_sig(&mut self, r: Refinement) -> std::sync::Arc<MethodSig> {
        match r {
            Refinement::Val(_, _, ty) => self.value_sig(ty),
            Refinement::Term(_, s, l) => {
                let sig = self.sig_arc(s);
                if self.sig_types(&sig) == l {
                    return sig;
                }
                if let Some(seen) = self.refinement_sigs.get(&(s, l)) {
                    return seen.clone();
                }
                let items = self.types.items(l).to_vec();
                let k = sig.tparams.len();
                let (bounds, rest) = items.split_at((2 * k).min(items.len()));
                let mut seen = (*sig).clone();
                let mut rebound: Subst = Vec::new();
                if let Some((fresh, subst)) = self.rebind_tparams(&sig.tparams, bounds) {
                    seen.tparams = fresh;
                    rebound = subst;
                }
                let mut rest = rest.iter().copied();
                for p in seen.clauses.iter_mut().flat_map(|c| c.params.iter_mut()) {
                    let t = rest.next().unwrap_or(p.ty);
                    p.ty = self.types.subst(t, &rebound);
                }
                let ret = rest.next().unwrap_or(seen.ret);
                seen.ret = self.types.subst(ret, &rebound);
                let seen = std::sync::Arc::new(seen);
                self.refinement_sigs.insert((s, l), seen.clone());
                seen
            }
            Refinement::Alias(..) | Refinement::Bounds(..) => unreachable!("a type refinement has no signature"),
        }
    }

    /// Type parameters of the bounds `bounds` (each one's lower and upper) for `tparams`, where
    /// those are not their own: fresh ones, with the renaming of `tparams` to them, which the
    /// bounds are read under too (an F-bound); None where the bounds are theirs. dotty's
    /// `derivedLambdaType` of a lambda whose parameter infos a map changed.
    fn rebind_tparams(&mut self, tparams: &[TParamId], bounds: &[TypeId]) -> Option<(Vec<TParamId>, Subst)> {
        let unchanged = tparams.iter().enumerate().all(|(i, &p)| {
            let info = self.syms.tparam(p);
            bounds.get(2 * i) == Some(&info.lower) && bounds.get(2 * i + 1) == Some(&info.upper)
        });
        if unchanged {
            return None;
        }
        let fresh: Vec<TParamId> = tparams
            .iter()
            .map(|&p| {
                let info = self.syms.tparam(p).clone();
                let q = self.syms.new_tparam(info.name, info.variance);
                let new = &mut self.syms.tparams[q.idx()];
                new.arity = info.arity;
                new.hk_variances = info.hk_variances;
                q
            })
            .collect();
        let renaming: Subst = tparams.iter().zip(&fresh).map(|(&p, &q)| (p, self.types.param(q))).collect();
        for (i, &q) in fresh.iter().enumerate() {
            let lower = bounds.get(2 * i).map_or(NOTHING, |&b| self.types.subst(b, &renaming));
            let upper = bounds.get(2 * i + 1).map_or(ANY, |&b| self.types.subst(b, &renaming));
            let info = &mut self.syms.tparams[q.idx()];
            info.lower = lower;
            info.upper = upper;
        }
        Some((fresh, renaming))
    }

    /// A polymorphic function type's list with its bounds mapped by `f`, its parameters as they
    /// are.
    pub fn map_poly_bounds(&mut self, ps: TList, f: &mut dyn FnMut(&mut Self, TypeId) -> TypeId) -> TList {
        let items = self.types.items(ps).to_vec();
        let k = items.len() / 3;
        let mut mapped = items.clone();
        for x in mapped.iter_mut().skip(k) {
            *x = f(self, *x);
        }
        if mapped == items { ps } else { self.types.list(&mapped) }
    }

    /// `[A <: B] => F`: the polymorphic function type over the parameters `params` (`Param`
    /// types) with the bounds their symbols hold, which the type keeps from then on.
    pub fn poly_type(&mut self, params: &[TypeId], fun: TypeId) -> TypeId {
        let mut items = params.to_vec();
        for &p in params {
            match self.types.get(p) {
                Type::Param(id) => {
                    let info = self.syms.tparam(id);
                    let (lower, upper) = (info.lower, info.upper);
                    items.extend([lower, upper]);
                }
                _ => items.extend([NOTHING, ANY]),
            }
        }
        let l = self.types.list(&items);
        self.types.mk(Type::Poly(l, fun))
    }

    /// The parameters of the polymorphic function type `t` and its function type over them,
    /// the parameters' symbols holding the type's bounds: rebound where the type's bounds are
    /// not theirs (`rebind_tparams`), the same ones for the same type.
    pub fn poly_binders(&mut self, t: TypeId) -> Option<(Vec<TParamId>, TypeId)> {
        let Type::Poly(l, fun) = self.types.get(t) else { return None };
        let params: Vec<TParamId> = self.types.poly_params(l).iter().filter_map(|&p| match self.types.get(p) {
            Type::Param(id) => Some(id),
            _ => None,
        }).collect();
        let bounds = self.types.poly_bounds(l).to_vec();
        if params.len() * 2 != bounds.len() {
            return Some((params, fun));
        }
        if let Some(seen) = self.poly_rebound.get(&t) {
            return Some(seen.clone());
        }
        let seen = match self.rebind_tparams(&params, &bounds) {
            Some((fresh, renaming)) => (fresh, self.types.subst(fun, &renaming)),
            None => (params, fun),
        };
        self.poly_rebound.insert(t, seen.clone());
        Some(seen)
    }

    /// A method signature's types as a refinement carries them: the bounds of its type
    /// parameters, the parameters clause by clause, then the result. The bounds stand where
    /// every map of the types reaches them, as a `PolyType`'s parameter infos do in dotty's
    /// `TypeMap.mapOverLambda`, and before the parameters, a parameter's position.
    pub fn sig_types(&mut self, sig: &MethodSig) -> TList {
        let bounds = sig.tparams.iter().flat_map(|&p| {
            let info = self.syms.tparam(p);
            [info.lower, info.upper]
        });
        let items: Vec<TypeId> = bounds.collect::<Vec<_>>().into_iter().chain(sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty))).chain([sig.ret]).collect();
        self.types.list(&items)
    }

    fn rebase(&mut self, t: TypeId, r: &Rebase) -> TypeId {
        if !self.types.has_paths(t) {
            return t;
        }
        let t = self.deref(t);
        match self.types.get(t) {
            Type::This(k) => match r.this_of {
                Some((c, prefix)) if k == c => prefix,
                Some((c, prefix)) if !r.own && (self.syms.class(c).base_types.iter().any(|&(b, _)| b == k) || self.self_type_derives(c, k)) => prefix,
                _ => t,
            },
            // The innermost binding of a parameter wins where an inline method expands inside
            // its own expansion (`mkInstances` for a field's type under `mkInstances`).
            Type::Term(s) => r.terms.iter().rev().find(|&&(p, _)| p == s).map_or(t, |&(_, path)| path),
            Type::Select(p, s) => {
                let np = self.rebase(p, r);
                if np == p {
                    t
                } else if r.approx && matches!(self.types.get(np), Type::Var(_)) {
                    self.approx_member(t)
                } else if self.types.is_path(np) {
                    self.types.mk(Type::Select(np, s))
                } else {
                    // The path was replaced by a type: the selection is its member's type, the
                    // one a refinement gives it first (`C { val q: p.type }`).
                    let name = self.syms.sym(s).name;
                    if let Some(refined) = self.refined_term(np, name) {
                        return self.refinement_sig(refined).ret;
                    }
                    match self.find_member(np, name) {
                        Some((_, owner_ty)) => {
                            let ret = self.sig_of(s).ret;
                            let subst = self.owner_subst(owner_ty);
                            let ty = self.types.subst(ret, &subst);
                            self.widen_path(ty)
                        }
                        None => ERROR,
                    }
                }
            }
            Type::Member(p, name) => {
                let np = self.rebase(p, r);
                if np == p {
                    t
                } else if r.approx && matches!(self.types.get(np), Type::Var(_)) {
                    self.approx_member(t)
                } else {
                    let np = self.projection_prefix(np);
                    // The member whose bounds are being resolved names itself through the new
                    // prefix (`type T <: Element[T]` seen from a self-typed trait): by name.
                    if !self.lookups.is_empty() && self.bounds_lookup_in_progress(np, name) {
                        return self.types.mk(Type::Member(np, name));
                    }
                    match self.member_type(np, name) {
                        Some(seen) => seen,
                        None => self.unresolved_member(np, name).unwrap_or(t),
                    }
                }
            }
            Type::AppMember(m, args) => {
                let nm = self.rebase(m, r);
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let nargs: Vec<TypeId> = items.iter().map(|&a| self.rebase(a, r)).collect();
                if nm == m && nargs == items {
                    t
                } else {
                    self.types.apply_ctor(nm, &nargs)
                }
            }
            Type::Class(c, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let nargs: Vec<TypeId> = items.iter().map(|&a| self.rebase(a, r)).collect();
                let seen = if self.types.is_path_class(c) { self.opaque_seen_from(c, r) } else { c };
                if nargs == items && seen == c {
                    t
                } else {
                    self.types.class(seen, &nargs)
                }
            }
            Type::AppParam(p, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let nargs: Vec<TypeId> = items.iter().map(|&a| self.rebase(a, r)).collect();
                let l = self.types.list(&nargs);
                self.types.mk(Type::AppParam(p, l))
            }
            Type::AppVar(v, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let nargs: Vec<TypeId> = items.iter().map(|&a| self.rebase(a, r)).collect();
                let l = self.types.list(&nargs);
                self.types.mk(Type::AppVar(v, l))
            }
            Type::Lambda(ps, body) => {
                let nb = self.rebase(body, r);
                self.types.mk(Type::Lambda(ps, nb))
            }
            // The bounds with the function type, as `TypeMap.mapOverLambda` maps both.
            Type::Poly(ps, body) => {
                let nb = self.rebase(body, r);
                let nps = self.map_poly_bounds(ps, &mut |w, x| w.rebase(x, r));
                self.types.mk(Type::Poly(nps, nb))
            }
            Type::Union(a, b) => {
                let (na, nb) = (self.rebase(a, r), self.rebase(b, r));
                self.types.union(na, nb)
            }
            Type::Inter(a, b) => {
                let (na, nb) = (self.rebase(a, r), self.rebase(b, r));
                self.types.inter(na, nb)
            }
            Type::Refined(p, rf) => {
                let np = self.rebase(p, r);
                let nrf = match self.types.refinement(rf) {
                    Refinement::Alias(n, rhs) => {
                        let a = Refinement::Alias(n, self.rebase(rhs, r));
                        self.types.refine(a)
                    }
                    Refinement::Bounds(n, lo, hi) => {
                        let b = Refinement::Bounds(n, self.rebase(lo, r), self.rebase(hi, r));
                        self.types.refine(b)
                    }
                    Refinement::Val(n, s, ty) => {
                        let v = Refinement::Val(n, s, self.rebase(ty, r));
                        self.types.refine(v)
                    }
                    Refinement::Term(n, s, l) => {
                        let items: Vec<TypeId> = self.types.items(l).to_vec();
                        let z: Vec<TypeId> = items.into_iter().map(|a| self.rebase(a, r)).collect();
                        let l = self.types.list(&z);
                        self.types.refine(Refinement::Term(n, s, l))
                    }
                };
                self.types.mk(Type::Refined(np, nrf))
            }
            Type::Alias(a, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let nargs: Vec<TypeId> = items.iter().map(|&x| self.rebase(x, r)).collect();
                let l = self.types.list(&nargs);
                self.types.mk(Type::Alias(a, l))
            }
            Type::Match(s, m) => {
                let ns = self.rebase(s, r);
                let (cases, bound) = {
                    let info = self.types.match_info(m);
                    (info.cases.to_vec(), info.bound)
                };
                let nb = self.rebase(bound, r);
                let ncases: Vec<MatchCase> = cases
                    .iter()
                    .map(|c| MatchCase { binders: c.binders, pattern: self.rebase(c.pattern, r), body: self.rebase(c.body, r) })
                    .collect();
                self.types.match_type(ns, &ncases, nb)
            }
            _ => t,
        }
    }

    /// A member seen through a path whose type has no such member (`bus.Subscriber` for a
    /// `bus: ManagedActorClassification` whose self type declares it): a type of its own that
    /// nothing conforms to, as scalac cannot resolve the reference. None where the path's type
    /// is not settled enough to tell.
    fn unresolved_member(&mut self, prefix: TypeId, name: Name) -> Option<TypeId> {
        if !self.types.is_path(prefix) {
            return None;
        }
        let under = self.path_underlying(prefix);
        let under = self.dealias(under);
        let mut parts = vec![under];
        while let Some(t) = parts.pop() {
            match self.types.get(t) {
                Type::Class(..) | Type::This(_) => {}
                Type::Inter(a, b) => parts.extend([a, b]),
                _ => return None,
            }
        }
        let spelling = format!("{}.{} (which cannot be resolved: {} has no member {})", self.show(prefix), self.name_str(name), self.show(under), self.name_str(name));
        Some(self.types.blocked(&format!("{}: {}", UNRESOLVED_MEMBER, spelling)))
    }

    /// The class of `c`'s declared self type that has the type member `name`, if any.
    pub fn self_type_member_class(&mut self, c: ClassId, name: Name) -> Option<ClassId> {
        let declared = self.syms.class(c).declared_self?;
        let mut parts = vec![declared];
        while let Some(t) = parts.pop() {
            let t = self.deref_alias(t);
            match self.types.get(t) {
                Type::Inter(a, b) => parts.extend([a, b]),
                Type::Class(k, _) if k != c => {
                    self.complete_class(k);
                    if self.syms.class(k).base_types.iter().any(|&(b, _)| self.syms.class(b).type_aliases.contains_key(&name)) {
                        return Some(k);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Whether `this` of `c` is a `k` through the self type `c` declares.
    pub(super) fn self_type_derives(&mut self, c: ClassId, k: ClassId) -> bool {
        let Some(declared) = self.syms.class(c).declared_self else { return false };
        let mut parts = vec![declared];
        while let Some(t) = parts.pop() {
            let t = self.deref_alias(t);
            match self.types.get(t) {
                Type::Inter(a, b) => parts.extend([a, b]),
                Type::Class(d, _) => {
                    self.complete_class(d);
                    if self.syms.class(d).base_types.iter().any(|&(b, _)| b == k) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// A prefix for a member seen through a value that is no path: the value's type, which
    /// makes the member the projection `T#A`.
    fn projection_prefix(&mut self, t: TypeId) -> TypeId {
        if self.types.is_path(t) {
            return t;
        }
        let t = self.widen_path(t);
        self.widen_lit(t)
    }

    /// The prefix through which the members of `recv`, of type `ty`, are seen: its path when
    /// it is one, its type otherwise.
    pub fn prefix_of(&mut self, recv: crate::tir::TExprId, ty: TypeId) -> TypeId {
        // A receiver typed as a path (`builder.addOne(x)`, of type `builder.type`) is that path.
        if self.types.is_path(ty) {
            return ty;
        }
        // A cast keeps the expression under it, whose path does not have the members; a
        // refinement the expression is typed with fixes members the path leaves abstract (an
        // inline proxy for a `Mirror.ProductOf[T]` bound to the companion object).
        let declared = self.deref(ty);
        let refined = matches!(self.types.get(declared), Type::Refined(..));
        let path = self.path_of(recv).filter(|&p| {
            let under = self.widen_path(p);
            if refined && under != ty {
                return false;
            }
            match self.class_of(ty) {
                Some(c) => self.base_type(under, c).is_some(),
                None => true,
            }
        });
        match path {
            Some(p) => p,
            None => self.projection_prefix(ty),
        }
    }

    /// The singleton type of a stable expression: a val, a parameter, an object, `this`, or a
    /// val selected from one of those.
    pub fn path_of(&mut self, te: crate::tir::TExprId) -> Option<TypeId> {
        use crate::tir::TExpr;
        if self.expr_marks.get(&te).map_or(false, |&m| m & super::MARK_CAST != 0) {
            return None;
        }
        // A given's reference still pending is the call its expansion makes, each a value of its
        // own, no path.
        if matches!(self.prog.expr(te), TExpr::Static(_) | TExpr::Field(..)) && self.is_pending_call(te) {
            return None;
        }
        if let Some(&(recv, s)) = self.folded_paths.get(&te) {
            return self.member_path(recv, s);
        }
        match self.prog.expr(te) {
            TExpr::Local(s) | TExpr::Static(s) => {
                let info = self.syms.sym(s);
                match info.kind {
                    // The local holding an enclosing class's `this` for a lifted class is that
                    // `this` as a path.
                    SymKind::Val if matches!(self.syms.sym(s).sig.as_ref().map(|sig| self.types.get(sig.ret)), Some(Type::This(_))) => {
                        self.syms.sym(s).sig.as_ref().map(|sig| sig.ret)
                    }
                    SymKind::Val | SymKind::Param | SymKind::Given | SymKind::EnumValue(_) if !info.by_name => {
                        Some(self.types.mk(Type::Term(s)))
                    }
                    SymKind::Object(c) => Some(self.types.class(c, &[])),
                    _ => None,
                }
            }
            TExpr::Module(c) => Some(self.types.class(c, &[])),
            TExpr::This | TExpr::Super(_) => {
                let c = self.this_class()?;
                Some(self.this_prefix(c))
            }
            // The enclosing instance of an inner class is its outer class's `this`.
            TExpr::CallMethod(_, s, _) if !self.outer_accessors.is_empty() => {
                let Owner::Class(k) = self.syms.sym(s).owner else { return None };
                if self.outer_accessors.get(&k) != Some(&s) {
                    return None;
                }
                let o = self.enclosing_instance_class(k)?;
                Some(self.this_prefix(o))
            }
            TExpr::Field(recv, s) => self.member_path(recv, s),
            _ => None,
        }
    }

    fn member_path(&mut self, recv: crate::tir::TExprId, s: SymId) -> Option<TypeId> {
        if !matches!(self.syms.sym(s).kind, SymKind::Val | SymKind::Given | SymKind::EnumValue(_)) {
            return None;
        }
        // A member of an object is one value wherever it is reached from.
        if let crate::tir::TExpr::Module(_) = self.prog.expr(recv) {
            return Some(self.types.mk(Type::Term(s)));
        }
        let p = self.path_of(recv)?;
        if matches!(self.types.get(p), Type::Class(c, _) if self.syms.class(c).kind == ClassKind::Object) {
            return Some(self.types.mk(Type::Term(s)));
        }
        Some(self.types.mk(Type::Select(p, s)))
    }

    /// The path a term reference names when it is stable, with why it is not otherwise.
    pub fn path_of_ref(&mut self, r: super::resolve::TermRef) -> Result<TypeId, &'static str> {
        use super::resolve::TermRef;
        let with_sym = |t: &mut Self, prefix: Option<TypeId>, s: SymId| {
            let info = t.syms.sym(s);
            match info.kind {
                SymKind::Object(c) => Ok(t.types.class(c, &[])),
                SymKind::Var => Err("a variable"),
                // A def with only using clauses (`quotes.reflect.Term`) stands for its result,
                // as scalac reads `transparent inline def quotes(using q: Quotes): q.type`.
                SymKind::Def if info.sig.as_ref().map_or(false, |sig| !sig.clauses.is_empty() && sig.clauses.iter().all(|c| c.is_using)) => {
                    let ret = t.sig_of(s).ret;
                    match t.types.get(ret) {
                        Type::Class(..) => Ok(ret),
                        _ => Err("a method"),
                    }
                }
                SymKind::Def | SymKind::Overloaded(_) => Err("a method"),
                _ if info.by_name => Err("a by-name parameter"),
                _ => Ok(match prefix {
                    Some(p) => t.types.mk(Type::Select(p, s)),
                    None => t.types.mk(Type::Term(s)),
                }),
            }
        };
        match r {
            TermRef::Local(s) | TermRef::Global(s) | TermRef::ModuleMember(_, s) => with_sym(self, None, s),
            TermRef::ValueMember(v, m) => {
                let p = self.import_value_prefix(v);
                with_sym(self, Some(p), m)
            }
            TermRef::This(c, s) => {
                if self.syms.class(c).kind == ClassKind::Object {
                    return with_sym(self, None, s);
                }
                let p = self.this_prefix(c);
                with_sym(self, Some(p), s)
            }
            TermRef::SelfAlias(c) => {
                self.complete_class(c);
                Ok(self.inline_this_of(c).unwrap_or_else(|| self.this_prefix(c)))
            }
            TermRef::Class(_) => Err("a class"),
            TermRef::Package(_) => Err("a package"),
        }
    }

    /// The bounds of a member that a refinement `r` states for the type `t`'s member of its
    /// name: what the refinement asks of `t`.
    pub fn refinement_holds(&mut self, t: TypeId, r: RefineId) -> bool {
        match self.types.refinement(r) {
            Refinement::Alias(name, rhs) => match self.member_in_type(t, name, t) {
                Some(MemberInfo::Alias(a)) => self.is_same(a, rhs),
                Some(MemberInfo::Bounds(lo, hi)) if lo == hi => self.is_same(lo, rhs),
                // The abstract member of a path is the path's own: `v.R` for a `v` whose
                // class declares `type R`.
                Some(MemberInfo::Bounds(..)) if self.types.is_path(t) => {
                    let own = self.types.mk(Type::Member(t, name));
                    self.is_same(own, rhs)
                }
                Some(MemberInfo::Bounds(..)) | None => false,
            },
            Refinement::Bounds(name, lo, hi) => match self.member_in_intersection(t, name) {
                Some(MemberInfo::Alias(a)) => self.is_sub(lo, a) && self.is_sub(a, hi),
                Some(MemberInfo::Bounds(mlo, mhi)) => self.is_sub(lo, mlo) && self.is_sub(mhi, hi),
                None => false,
            },
            wanted_ref @ (Refinement::Term(name, ..) | Refinement::Val(name, ..)) => {
                let wanted = self.refinement_sig(wanted_ref);
                let refined = if self.types.is_path(t) { self.widen_path(t) } else { t };
                let (sig, found, owner_ty) = match self.refined_term(refined, name) {
                    Some(r @ (Refinement::Term(_, s, _) | Refinement::Val(_, s, _))) => (self.refinement_sig(r), s, ANY),
                    _ => {
                        let Some((found, owner_ty)) = self.find_member(t, name) else { return false };
                        (self.sig_arc(found), found, owner_ty)
                    }
                };
                // A `val` refinement asks for a stable member: a `def` or a `var` is none.
                if matches!(wanted_ref, Refinement::Val(..)) && matches!(self.syms.sym(found).kind, SymKind::Def | SymKind::Var) && self.syms.sym(found).sig.as_ref().map_or(true, |sg| sg.clauses.is_empty()) {
                    return false;
                }
                let mut subst = self.owner_subst(owner_ty);
                if sig.tparams.len() != wanted.tparams.len() {
                    return false;
                }
                // A generic member's type parameters are the wanted one's, whose bounds lie
                // within its own (dotty's `TypeComparer.comparePoly`, `matchingPolyParams`).
                for (&p, &q) in sig.tparams.iter().zip(&wanted.tparams) {
                    if p != q {
                        let to = self.types.param(q);
                        subst.push((p, to));
                    }
                }
                for (&p, &q) in sig.tparams.iter().zip(&wanted.tparams) {
                    let (lo, hi) = (self.syms.tparam(p).lower, self.syms.tparam(p).upper);
                    let (lo, hi) = (self.types.subst(lo, &subst), self.types.subst(hi, &subst));
                    let (wlo, whi) = (self.syms.tparam(q).lower, self.syms.tparam(q).upper);
                    if !(self.is_sub(lo, wlo) && self.is_sub(whi, hi)) {
                        return false;
                    }
                }
                let shape = |s: &MethodSig| s.clauses.iter().map(|c| c.params.len()).collect::<Vec<_>>();
                if shape(&sig) != shape(&wanted) {
                    return false;
                }
                // The parameters are matched by position, as the override check matches them
                // (`(d: Ctx) => d.T` is `(c: Ctx) => c.T`). A function's `apply`, which the
                // refined class declares, takes them contravariantly, as scalac's
                // `hasMatchingMember` compares a refinement of a member the class has (`Any =>
                // Int` is a `(s: String) => Int`); another member's have to be the same.
                let renamed: Vec<(SymId, TypeId)> = sig
                    .clauses
                    .iter()
                    .zip(&wanted.clauses)
                    .flat_map(|(a, b)| a.params.iter().zip(&b.params).map(|(x, y)| (x.sym, y.sym)).collect::<Vec<_>>())
                    .filter(|&(x, y)| x != y)
                    .map(|(x, y)| (x, self.types.mk(Type::Term(y))))
                    .collect();
                for (a, b) in sig.clauses.iter().zip(&wanted.clauses) {
                    for (x, y) in a.params.iter().zip(&b.params) {
                        // A parameter's mode is part of its type, `=> T` and `T*` in dotty's
                        // method types (`TypeComparer.matchingMethodParams`).
                        if x.by_name != y.by_name || x.repeated != y.repeated {
                            return false;
                        }
                        let xt = self.subst_paths(x.ty, &renamed);
                        let xt = self.types.subst(xt, &subst);
                        let fits = if name == crate::names::APPLY { self.is_sub(y.ty, xt) } else { self.is_same(xt, y.ty) };
                        if !fits {
                            return false;
                        }
                    }
                }
                let ret = self.subst_paths(sig.ret, &renamed);
                let ret = self.types.subst(ret, &subst);
                self.is_sub(ret, wanted.ret)
            }
        }
    }

    /// Whether a refinement of the signature `sig` matches `member`, a member of the class seen
    /// as `owner_ty`, which it then refines rather than overloads (dotty's `Denotation.matches`
    /// of a refinement's symbol, `Typer.typedRefinedTypeTree`): as many type parameters, the
    /// same clauses, each parameter of the same mode and type, the member's type parameters and
    /// earlier parameters read as the refinement's.
    pub fn refinement_matches(&mut self, member: SymId, owner_ty: TypeId, sig: &MethodSig) -> bool {
        let theirs = self.sig_arc(member);
        let shape = |s: &MethodSig| s.clauses.iter().map(|c| c.params.len()).collect::<Vec<_>>();
        if theirs.tparams.len() != sig.tparams.len() || shape(&theirs) != shape(sig) {
            return false;
        }
        let mut subst = self.owner_subst(owner_ty);
        for (&p, &q) in theirs.tparams.iter().zip(&sig.tparams) {
            if p != q {
                let to = self.types.param(q);
                subst.push((p, to));
            }
        }
        let pairs: Vec<(ParamSig, ParamSig)> = theirs.clauses.iter().flat_map(|c| c.params.iter().cloned()).zip(sig.clauses.iter().flat_map(|c| c.params.iter().cloned())).collect();
        let renamed: Vec<(SymId, TypeId)> = pairs.iter().filter(|(p, q)| p.sym != q.sym).map(|(p, q)| (p.sym, self.types.mk(Type::Term(q.sym)))).collect();
        let mark = self.trail.len();
        for (p, q) in &pairs {
            if p.by_name != q.by_name || p.repeated != q.repeated {
                self.rollback(mark);
                return false;
            }
            let t = self.subst_paths(p.ty, &renamed);
            let t = self.types.subst(t, &subst);
            if !self.is_same(t, q.ty) {
                self.rollback(mark);
                return false;
            }
        }
        true
    }

    /// The member `name` of `t`, where an intersection's parts declare it as an alias and as
    /// bounds the alias does not meet: the two infos intersected, `>: x | lo <: x & hi`, as
    /// scalac's `&` of a `TypeAlias` and `TypeBounds` (`ProductOf[B] { type MirroredElemTypes
    /// = t } & Mirror.Of[B]` has `t & Tuple`).
    fn member_in_intersection(&mut self, t: TypeId, name: Name) -> Option<MemberInfo> {
        let d = self.dealias(t);
        let Type::Inter(a, b) = self.types.get(d) else { return self.member_in_type(t, name, t) };
        let (ma, mb) = (self.member_in_type(a, name, t), self.member_in_type(b, name, t));
        match (ma, mb) {
            (Some(MemberInfo::Bounds(lo, hi)), other) | (other @ Some(MemberInfo::Alias(_)), Some(MemberInfo::Bounds(lo, hi))) => Some(self.bounds_meet(lo, hi, other)),
            (Some(m), _) => Some(m),
            (None, m) => m,
        }
    }

    /// The structural member `name` of a refined type, with the refinement chain it sits in.
    pub fn refined_term(&mut self, t: TypeId, name: Name) -> Option<Refinement> {
        let mut t = self.deref(t);
        loop {
            match self.types.get(t) {
                Type::Refined(p, r) => {
                    let refinement = self.types.refinement(r);
                    if matches!(refinement, Refinement::Term(n, ..) | Refinement::Val(n, ..) if n == name) {
                        return Some(refinement);
                    }
                    t = p;
                }
                Type::Alias(..) => t = self.reduce_head(t)?,
                _ => return None,
            }
        }
    }
}

struct Rebase<'r> {
    this_of: Option<(ClassId, TypeId)>,
    terms: &'r [(SymId, TypeId)],
    /// A member seen through a term replaced by a variable becomes a variable of its own.
    approx: bool,
    /// The type is a member's own declaration: only the class's own `this` moves, since an
    /// ancestor's `C.this` written in it names an enclosing instance (an anonymous `Validate`
    /// inside `Validate.contramap` with `type R = self.R`).
    own: bool,
}
