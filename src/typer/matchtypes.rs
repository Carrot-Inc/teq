//! Match types, `S match { case P => T ... }`. A match type is kept as written and reduced
//! where a use needs it: `dealias` and `is_sub` reduce the head, `normalize` the whole type
//! before it is stored or shown. An application of an alias whose right-hand side is a match
//! type stays a `Type::Alias` by name until then, which is what lets a case body name the alias
//! itself and what an error prints. Reduction follows scalac's `MatchReducer`: a case reduces
//! when the scrutinee is an instance of its pattern, is skipped when the two are provably
//! disjoint, and stops the reduction otherwise.

use super::{Frame, Worker};
use crate::ast::{DefKind, ListRef, TyExpr, TyExprId};
use crate::intern::{FxMap, Name};
use crate::source::Span;
use crate::symbols::*;
use crate::types::*;

/// How deep reductions nest through the types they produce before the recursion is reported.
const MAX_DEPTH: usize = 512;
const MAX_DISJOINT_DEPTH: u32 = 24;

impl MatchState {
    /// Forgets the reductions, which the merge's renumbering of the types leaves behind.
    pub(super) fn forget(&mut self) {
        self.cache.clear();
        self.expansions.clear();
    }
}

#[derive(Default)]
pub struct MatchState {
    /// Reductions of match types without inference variables: the reduced type, or None where
    /// the match type is stuck.
    cache: FxMap<TypeId, Option<TypeId>>,
    /// The match types being reduced, outermost first, as they are shown.
    stack: Vec<(TypeId, TypeId)>,
    /// Whether a failed reduction records why, for the note of a type mismatch.
    recording: bool,
    trace: Option<String>,
    /// Aliases whose reduction ran into itself, reported once each.
    pub(super) reported: Vec<AliasId>,
    /// The right-hand side of each alias application seen, over its arguments.
    expansions: FxMap<(AliasId, TList), TypeId>,
    /// How many structural comparisons of stuck match types are nested.
    comparing: u32,
}

impl MatchState {
    /// Exchanges the reductions and the alias expansions with `other`'s: the memos of the other
    /// view (`Worker::swap_view_memos`).
    pub fn swap_memos(&mut self, other: &mut MatchMemos) {
        std::mem::swap(&mut self.cache, &mut other.cache);
        std::mem::swap(&mut self.expansions, &mut other.expansions);
    }
}

/// `MatchState`'s memos, which hold types in the view they were made in.
#[derive(Default)]
pub struct MatchMemos {
    cache: FxMap<TypeId, Option<TypeId>>,
    expansions: FxMap<(AliasId, TList), TypeId>,
}

enum CaseResult {
    Reduced(TypeId),
    Disjoint,
    Stuck,
    Empty,
    NoInstance(Vec<(Name, TypeId, TypeId)>),
}

impl<'a> Worker<'a> {
    // ---- type expressions ----

    /// `S match { case P => T ... }` as a type: each case binds the type variables of its
    /// pattern, which its body may name; a variable in an argument position takes the bound of
    /// the parameter it stands for. Without a declared bound the match type is bounded by the
    /// union of its bodies, or by `Any` where a body reduces further.
    pub(super) fn match_type_expr(&mut self, scrut: TyExprId, cases: ListRef) -> TypeId {
        let case_ids: Vec<TyExprId> = self.cur_ast().ty_list(cases).to_vec();
        let scrutinee = self.resolve_type(scrut);
        let mut out = Vec::with_capacity(case_ids.len());
        let mut bound = NOTHING;
        for c in case_ids {
            let TyExpr::MatchCase(pat, body) = self.cur_ast().ty(c) else { continue };
            let mut names = Vec::new();
            self.collect_type_vars(pat, &mut names);
            let mark = self.case_binders.len();
            let mut binders = Vec::with_capacity(names.len());
            let mut frame_tparams = Vec::with_capacity(names.len());
            for n in names {
                let p = self.syms.new_tparam(n, 0);
                let t = self.types.param(p);
                binders.push(t);
                self.case_binders.push((n, t));
                frame_tparams.push((n, p));
            }
            let pattern = self.resolve_type(pat);
            self.bind_binder_bounds(pattern, &binders);
            self.env.frames.push(Frame::Locals { names: Vec::new(), tparams: frame_tparams, givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: self.sites.owners.len() as u32 });
            let body = self.resolve_type(body);
            self.env.frames.pop();
            self.case_binders.truncate(mark);
            if bound != ANY {
                let widened: Subst = binders
                    .iter()
                    .map(|&b| match self.types.get(b) {
                        Type::Param(p) => (p, self.syms.tparam(p).upper),
                        _ => unreachable!(),
                    })
                    .collect();
                let body_bound = self.types.subst(body, &widened);
                bound = if self.types.is_reducible(body_bound) { ANY } else { self.lub(bound, body_bound) };
            }
            let binders = self.types.list(&binders);
            out.push(MatchCase { binders, pattern, body });
        }
        if out.is_empty() {
            return ERROR;
        }
        self.types.match_type(scrutinee, &out, bound)
    }

    /// A binder that is the argument of a class in the pattern is bounded like that parameter
    /// (`case h *: t` gives `t <: Tuple`), so that the body may use it as one.
    fn bind_binder_bounds(&mut self, pattern: TypeId, binders: &[TypeId]) {
        let Type::Class(c, args) = self.types.get(pattern) else { return };
        let items: Vec<TypeId> = self.types.items(args).to_vec();
        self.complete_class(c);
        let params = self.syms.class(c).tparams.clone();
        let subst: Subst = params.iter().copied().zip(items.iter().copied()).collect();
        for (&p, &a) in params.iter().zip(&items) {
            if binders.contains(&a) {
                let Type::Param(b) = self.types.get(a) else { continue };
                let upper = self.syms.tparam(p).upper;
                if upper != ANY && self.syms.tparam(b).upper == ANY {
                    let upper = self.types.subst(upper, &subst);
                    self.syms.tparams[b.idx()].upper = upper;
                }
            } else {
                self.bind_binder_bounds(a, binders);
            }
        }
    }

    /// Whether an application of the alias is kept by name: its right-hand side is a match
    /// type, or a type lambda of its own (`type |:[F[_], G[_]] = [x] =>> OrElse[F[x], G[F[x]]]`),
    /// whose application scalac leaves unreduced as well, so that `F[T]` unifies with
    /// `(Eq |: Derived)[A]` as `F := Eq |: Derived`.
    pub fn is_match_alias(&self, a: AliasId) -> bool {
        if let Some(info) = self.syms.alias_done(a) {
            let body = self.alias_body(a);
            return matches!(self.types.get(body), Type::Match(..)) || (!info.tparams.is_empty() && matches!(self.types.get(body), Type::Lambda(..)));
        }
        let info = self.syms.alias(a);
        match info.def {
            Some(d) => match &self.ast(info.file).def(d).kind {
                DefKind::TypeAlias { rhs: Some(r), tparams, .. } => {
                    matches!(self.ast(info.file).ty(*r), TyExpr::Match(..)) || (!tparams.is_empty() && matches!(self.ast(info.file).ty(*r), TyExpr::Lambda(..)))
                }
                _ => false,
            },
            None => self.loaded_alias_is_match(a),
        }
    }

    /// Whether an application of a parameterised alias is kept by name, as scalac keeps one,
    /// and expanded where a use needs its right-hand side. Kept: an alias of a package or an
    /// object, whose right-hand side names no enclosing class's parameters or `this`. Expanded
    /// where it is named: one declared in a class or trait, seen through its prefix; one that
    /// only renames a type constructor (`type List[+A] = immutable.List[A]`), which scalac
    /// reduces as it applies it; and one over a type-level operation (`type Doubled[N <: Int]
    /// = N * 2`), which works out to a value.
    pub fn alias_by_name(&self, a: AliasId) -> bool {
        let Some(info) = self.syms.alias_done(a) else { return false };
        if info.tparams.is_empty() || info.rhs == ERROR || info.is_abstract() {
            return false;
        }
        let owned = match info.owner {
            Owner::Package(_) => true,
            Owner::Class(c) => self.syms.class(c).kind == ClassKind::Object && self.syms.class(c).tparams.is_empty() && !self.nested_in_generic(c),
            Owner::Local => false,
        };
        owned && !self.renames_constructor(a) && !self.types.head_reducible(self.types.get(self.alias_body(a)))
    }

    fn renames_constructor(&self, a: AliasId) -> bool {
        let info = &self.syms.aliases[a.idx()];
        let args = match self.types.get(self.alias_body(a)) {
            Type::Class(_, args) | Type::AppParam(_, args) | Type::AppMember(_, args) | Type::Alias(_, args) => args,
            _ => return false,
        };
        let items = self.types.items(args);
        items.len() == info.tparams.len() && items.iter().zip(&info.tparams).all(|(&t, &p)| matches!(self.types.get(t), Type::Param(q) if q == p))
    }

    fn nested_in_generic(&self, c: ClassId) -> bool {
        let mut owner = self.syms.class(c).owner;
        while let Owner::Class(o) = owner {
            if !self.syms.class(o).tparams.is_empty() || self.syms.class(o).kind != ClassKind::Object {
                return true;
            }
            owner = self.syms.class(o).owner;
        }
        false
    }

    /// The type an alias application stands for: its right-hand side over the arguments. None
    /// while the alias is being completed, which its own cases are.
    /// An alias whose right-hand side is a type lambda of its own (`type |:[F[_], G[_]] = [x] =>> ...`).
    pub fn is_curried_alias(&self, a: AliasId) -> bool {
        self.syms.alias_done(a).map_or(false, |info| !info.tparams.is_empty() && matches!(self.types.get(self.alias_body(a)), Type::Lambda(..)))
    }

    /// The right-hand side of an alias over its parameters: the lambda over exactly those
    /// parameters stripped where one was stored, a lambda of the alias's own (`type |:[F, G]
    /// = [x] =>> ...`) kept.
    pub fn alias_body(&self, a: AliasId) -> TypeId {
        let info = &self.syms.aliases[a.idx()];
        match self.types.get(info.rhs) {
            Type::Lambda(ps, b) => {
                let own = self.types.items(ps).iter().zip(&info.tparams).all(|(&p, &q)| matches!(self.types.get(p), Type::Param(r) if r == q));
                if own && self.types.items(ps).len() == info.tparams.len() { b } else { info.rhs }
            }
            _ => info.rhs,
        }
    }

    pub fn alias_expansion(&mut self, a: AliasId, args: TList) -> Option<TypeId> {
        if let Some(&t) = self.matches.expansions.get(&(a, args)) {
            return Some(t);
        }
        let t = self.alias_expansion_uncached(a, args)?;
        self.matches.expansions.insert((a, args), t);
        Some(t)
    }

    fn alias_expansion_uncached(&mut self, a: AliasId, args: TList) -> Option<TypeId> {
        // An alias this thread is completing has no expansion yet; one another worker is
        // completing is waited for (`complete_alias`), so that no reduction reads it as
        // incomplete and keeps that answer.
        if self.syms.alias(a).state() == Completion::InProgress && self.syms.alias(a).state().mine() {
            return None;
        }
        self.complete_alias(a);
        let (rhs, tparams) = {
            let info = &self.syms.aliases[a.idx()];
            (info.rhs, info.tparams.clone())
        };
        if rhs == ERROR {
            return Some(ERROR);
        }
        let body = self.alias_body(a);
        let items = self.types.items(args).to_vec();
        if items.len() < tparams.len() {
            return Some(ERROR);
        }
        let subst: Subst = tparams.iter().copied().zip(items.iter().copied()).collect();
        let expanded = self.types.subst(body, &subst);
        // A curried alias applied further (`(Eq |: Derived)[A]`): the rest goes to its lambda.
        if items.len() > tparams.len() {
            let rest = &items[tparams.len()..];
            return Some(self.types.apply_ctor(expanded, rest));
        }
        Some(expanded)
    }

    // ---- reduction ----

    /// The match type `t`, shown as `shown`, reduced to the body of the case its scrutinee
    /// selects; None where it is stuck.
    pub fn reduce_match(&mut self, t: TypeId, shown: TypeId) -> Option<TypeId> {
        let Type::Match(scrut, m) = self.types.get(t) else { return None };
        let cacheable = !self.types.has_vars(t) && self.gadt.is_empty() && !self.matches.recording;
        if cacheable {
            if let Some(&r) = self.matches.cache.get(&t) {
                return r;
            }
        }
        if self.matches.stack.iter().any(|&(open, _)| open == t) || self.matches.stack.len() >= MAX_DEPTH {
            self.report_match_recursion(shown);
            return None;
        }
        let prof = self.prof(super::profile::Kind::MatchType, Span::default(), super::profile::About::Type(shown));
        self.matches.stack.push((t, shown));
        let result = self.reduce_cases(scrut, m, shown);
        self.matches.stack.pop();
        if let Some(p) = prof {
            let outcome = if result.is_some() { super::profile::Outcome::Found } else { super::profile::Outcome::NotFound };
            self.profile.exit(p, outcome);
        }
        if cacheable {
            self.matches.cache.insert(t, result);
        }
        result
    }

    fn reduce_cases(&mut self, scrut: TypeId, m: MatchId, shown: TypeId) -> Option<TypeId> {
        let scrut = self.normalize(scrut);
        let scrut = self.zonk(scrut);
        if scrut == ERROR {
            return Some(ERROR);
        }
        // A scrutinee with open inference variables is provisional: matching it against the
        // patterns would constrain them.
        if self.types.has_vars(scrut) {
            return None;
        }
        if self.provably_empty(scrut) {
            self.record_failure(shown, |t| format!("failed since selector {}\n  is uninhabited (there are no values of that type).", t.show(scrut)));
            return None;
        }
        let cases = self.types.match_info(m).cases.to_vec();
        for (i, case) in cases.iter().enumerate() {
            match self.match_case(scrut, *case) {
                CaseResult::Reduced(t) => return Some(t),
                CaseResult::Disjoint => {}
                CaseResult::Empty => {
                    self.record_failure(shown, |t| format!("failed since selector {}\n  is uninhabited (there are no values of that type).", t.show(scrut)));
                    return None;
                }
                CaseResult::Stuck => {
                    let rest = &cases[i + 1..];
                    self.record_failure(shown, |t| {
                        let mut msg = format!(
                            "failed since selector {}\n  does not match  {}\n  and cannot be shown to be disjoint from it either.",
                            t.show(scrut),
                            t.show_case(*case)
                        );
                        if !rest.is_empty() {
                            let s = if rest.len() == 1 { "" } else { "s" };
                            msg.push_str(&format!("\n  Therefore, reduction cannot advance to the remaining case{}\n\n", s));
                            for c in rest {
                                msg.push_str(&format!("    {}\n", t.show_case(*c)));
                            }
                        }
                        msg
                    });
                    return None;
                }
                CaseResult::NoInstance(fails) => {
                    self.record_failure(shown, |t| {
                        let params = if fails.len() == 1 { "parameter" } else { "parameters" };
                        let names: Vec<String> = fails.iter().map(|&(n, _, _)| t.name_str(n)).collect();
                        let mut msg = format!(
                            "failed since selector {}\n  does not uniquely determine {} {} in\n    {}\n  The computed bounds for the {} are:",
                            t.show(scrut),
                            params,
                            names.join(", "),
                            t.show_case(*case),
                            params
                        );
                        for &(n, lo, hi) in &fails {
                            msg.push_str(&format!("\n    {}", t.name_str(n)));
                            if lo != NOTHING {
                                msg.push_str(&format!(" >: {}", t.show(lo)));
                            }
                            if hi != ANY {
                                msg.push_str(&format!(" <: {}", t.show(hi)));
                            }
                        }
                        msg
                    });
                    return None;
                }
            }
        }
        self.record_failure(shown, |t| {
            let mut msg = format!("failed since selector {}\n  matches none of the cases\n", t.show(scrut));
            for c in &cases {
                msg.push_str(&format!("\n    {}", t.show_case(*c)));
            }
            msg
        });
        None
    }

    fn record_failure(&mut self, shown: TypeId, message: impl FnOnce(&mut Self) -> String) {
        if !self.matches.recording || self.matches.trace.is_some() {
            return;
        }
        let mut text = String::new();
        let stack: Vec<TypeId> = self.matches.stack.iter().map(|&(_, s)| s).collect();
        for s in stack {
            text.push_str(&format!("  trying to reduce  {}\n", self.show(s)));
        }
        if !self.matches.stack.iter().any(|&(_, s)| s == shown) {
            text.push_str(&format!("  trying to reduce  {}\n", self.show(shown)));
        }
        text.push_str("  ");
        text.push_str(&message(self));
        self.matches.trace = Some(text);
    }

    /// The note scalac adds to a type mismatch in which a match type could not be reduced.
    pub fn match_type_note(&mut self, found: TypeId, expected: TypeId) -> Option<String> {
        if !self.types.is_reducible(found) && !self.types.is_reducible(expected) {
            return None;
        }
        self.matches.recording = true;
        self.matches.trace = None;
        for t in [expected, found] {
            let mark = self.snapshot();
            self.normalize(t);
            self.rollback(mark);
            if self.matches.trace.is_some() {
                break;
            }
        }
        self.matches.recording = false;
        let trace = self.matches.trace.take()?;
        Some(format!("\n\nNote: a match type could not be fully reduced:\n\n{}", trace.trim_end()))
    }

    #[cold]
    fn report_match_recursion(&mut self, shown: TypeId) {
        let Type::Alias(a, _) = self.types.get(shown) else { return };
        if self.matches.reported.contains(&a) {
            return;
        }
        self.matches.reported.push(a);
        let (file, span) = {
            let info = &self.syms.aliases[a.idx()];
            (info.file, info.def.map_or(Span::default(), |d| self.ast(info.file).def(d).span))
        };
        let msg = format!(
            "Recursion limit exceeded.\nMaybe there is an illegal cyclic reference?\nIf that's not the case, you could try to increase the fuel and stack size: https://docs.scala-lang.org/overviews/compiler-options/compiling-deeply-nested-code.html\nA recurring operation is (inner to outer):\n\n  reduce match type {}",
            self.show(shown)
        );
        self.diags.error(file, span, msg);
    }

    /// A type without values: `Nothing`, or an intersection of provably disjoint types.
    fn provably_empty(&mut self, t: TypeId) -> bool {
        match self.types.get(t) {
            Type::Nothing => true,
            Type::Inter(a, b) => self.provably_empty(a) || self.provably_empty(b) || self.provably_disjoint(a, b),
            Type::Union(a, b) => self.provably_empty(a) && self.provably_empty(b),
            _ => false,
        }
    }

    fn match_case(&mut self, scrut: TypeId, case: MatchCase) -> CaseResult {
        let binders: Vec<TParamId> = self
            .types
            .items(case.binders)
            .iter()
            .filter_map(|&b| match self.types.get(b) {
                Type::Param(p) => Some(p),
                _ => None,
            })
            .collect();
        let mark = self.snapshot();
        let vars: Vec<TypeId> = binders.iter().map(|_| self.fresh_var()).collect();
        let fresh: Subst = binders.iter().copied().zip(vars.iter().copied()).collect();
        let pattern = self.types.subst(case.pattern, &fresh);
        let widened: Subst = binders.iter().map(|&b| (b, self.syms.tparam(b).upper)).collect();
        let boundary = self.types.subst(case.pattern, &widened);
        if self.pattern_matches(scrut, pattern) {
            // A scrutinee that matches a pattern it is also disjoint from has no values
            // (`(Nothing, String)`), and reducing past it would be unsound.
            if self.provably_disjoint(scrut, boundary) {
                self.rollback(mark);
                return CaseResult::Empty;
            }
            let abstract_scrut = self.is_abstract_scrutinee(scrut);
            let mut insts: Subst = Vec::with_capacity(binders.len());
            let mut fails = Vec::new();
            for (&b, &v) in binders.iter().zip(&vars) {
                match self.binder_instance(v, abstract_scrut) {
                    Ok(t) => insts.push((b, t)),
                    Err((lo, hi)) => fails.push((self.syms.tparam(b).name, lo, hi)),
                }
            }
            // The instances are read before the variables they were solved from are undone.
            let insts: Subst = insts.into_iter().map(|(b, t)| (b, self.zonk(t))).collect();
            let fails: Vec<(Name, TypeId, TypeId)> = fails.into_iter().map(|(n, lo, hi)| (n, self.zonk(lo), self.zonk(hi))).collect();
            self.rollback(mark);
            if !fails.is_empty() {
                return CaseResult::NoInstance(fails);
            }
            return CaseResult::Reduced(self.types.subst(case.body, &insts));
        }
        self.rollback(mark);
        if self.provably_disjoint(scrut, boundary) {
            CaseResult::Disjoint
        } else {
            CaseResult::Stuck
        }
    }

    /// Whether the scrutinee is an instance of the pattern, binding the pattern's variables.
    /// `S[n]` of `compiletime.ops.int` matches a literal above zero with its predecessor.
    fn pattern_matches(&mut self, scrut: TypeId, pattern: TypeId) -> bool {
        if let Type::Class(c, args) = self.types.get(pattern) {
            if self.types.is_op_class(c) && self.types.items(args).len() == 1 && self.name_ref(self.syms.class(c).name) == "S" {
                let inner = self.types.items(args)[0];
                return match self.fold_type(scrut) {
                    Some(LitVal::Int(n)) if n > 0 => {
                        let pred = self.types.lit(LitVal::Int(n - 1));
                        self.pattern_matches(pred, inner)
                    }
                    _ => false,
                };
            }
        }
        self.is_sub(scrut, pattern)
    }

    /// A scrutinee whose values are only known through an upper bound, as scalac's
    /// `scrutIsWidenedAbstract`: a binder in a variant position of the pattern is not
    /// determined by it.
    fn is_abstract_scrutinee(&mut self, scrut: TypeId) -> bool {
        matches!(self.types.get(scrut), Type::Param(_) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_))
    }

    /// What a pattern variable stands for after the match: the type it was bound to, or the
    /// lower bound, or the upper bound; both when they differ and the scrutinee is abstract is
    /// no instance, and the bounds are returned.
    fn binder_instance(&mut self, v: TypeId, abstract_scrut: bool) -> Result<TypeId, (TypeId, TypeId)> {
        let Type::Var(id) = self.types.get(v) else { return Ok(v) };
        if let Some(inst) = self.tvars[id].inst {
            return Ok(inst);
        }
        let lowers = self.tvars[id].lower.clone();
        let uppers = self.tvars[id].upper.clone();
        let lower = lowers.into_iter().reduce(|a, b| self.lub(a, b));
        let upper = uppers.into_iter().reduce(|a, b| self.types.inter(a, b));
        match (lower, upper) {
            (Some(lo), Some(hi)) => {
                if self.is_same(lo, hi) {
                    Ok(lo)
                } else if abstract_scrut {
                    Err((lo, hi))
                } else {
                    Ok(lo)
                }
            }
            (Some(lo), None) => {
                if abstract_scrut {
                    Err((NOTHING, lo))
                } else {
                    Ok(lo)
                }
            }
            (None, Some(hi)) => {
                if abstract_scrut {
                    Err((hi, ANY))
                } else {
                    Ok(hi)
                }
            }
            (None, None) => Err((NOTHING, ANY)),
        }
    }

    // ---- disjointness ----

    /// Whether no value has both types, by scalac's rules: single inheritance of classes, final
    /// classes, distinct constants, distinct objects, the children of a sealed class.
    ///
    /// `pending` holds the pairs whose common base types are being compared; a pair met again
    /// inside its own comparison (an F-bounded base such as `SeqOps[A, CC, C]`) proves nothing.
    /// A match type reduced for a boundary starts queries of its own.
    pub fn provably_disjoint(&mut self, a: TypeId, b: TypeId) -> bool {
        self.disjoint_at(a, b, 0, &mut Vec::new())
    }

    fn disjoint_at(&mut self, a: TypeId, b: TypeId, depth: u32, pending: &mut Vec<(TypeId, TypeId)>) -> bool {
        if depth > MAX_DISJOINT_DEPTH {
            return false;
        }
        let a = self.disjointness_boundary(a);
        let b = self.disjointness_boundary(b);
        if a == NOTHING || b == NOTHING {
            return true;
        }
        if pending.contains(&(a, b)) {
            return false;
        }
        let (ta, tb) = (self.types.get(a), self.types.get(b));
        match (ta, tb) {
            (_, Type::Union(x, y)) => return self.disjoint_at(a, x, depth + 1, pending) && self.disjoint_at(a, y, depth + 1, pending),
            (_, Type::Inter(x, y)) => return self.disjoint_at(a, x, depth + 1, pending) || self.disjoint_at(a, y, depth + 1, pending),
            (Type::Union(x, y), _) => return self.disjoint_at(x, b, depth + 1, pending) && self.disjoint_at(y, b, depth + 1, pending),
            (Type::Inter(x, y), _) => return self.disjoint_at(x, b, depth + 1, pending) || self.disjoint_at(y, b, depth + 1, pending),
            (Type::Any, _) | (_, Type::Any) => return false,
            // Every stable value is a `Singleton`: the class shares no subclass, but its values.
            _ if self.b.t_singleton != ERROR && (a == self.b.t_singleton || b == self.b.t_singleton) => return false,
            (Type::Lit(x), Type::Lit(y)) => return x != y,
            (Type::Lit(_), _) => {
                let class = self.widen_lit(a);
                return self.disjoint_at(class, b, depth + 1, pending) || self.literal_outside(a, b);
            }
            (_, Type::Lit(_)) => {
                let class = self.widen_lit(b);
                return self.disjoint_at(a, class, depth + 1, pending) || self.literal_outside(b, a);
            }
            _ => {}
        }
        let (Type::Class(c1, args1), Type::Class(c2, args2)) = (ta, tb) else { return false };
        // A tuple against `h *: t`: element by element.
        if let Some(cons) = self.b.cons_tuple {
            let (tuple, cons_args) = if c2 == cons && self.is_tuple_class(c1) {
                (args1, args2)
            } else if c1 == cons && self.is_tuple_class(c2) {
                (args2, args1)
            } else {
                (EMPTY_LIST, EMPTY_LIST)
            };
            if cons_args != EMPTY_LIST {
                let elems: Vec<TypeId> = self.types.items(tuple).to_vec();
                let [h, t] = *self.types.items(cons_args) else { return false };
                let rest = self.tuple_of(&elems[1..]);
                return self.disjoint_at(elems[0], h, depth + 1, pending) || self.disjoint_at(rest, t, depth + 1, pending);
            }
        }
        if b == self.b.t_any_ref {
            return self.is_value(a);
        }
        if a == self.b.t_any_ref {
            return self.is_value(b);
        }
        if b == self.b.t_any_val {
            return self.is_reference(a);
        }
        if a == self.b.t_any_val {
            return self.is_reference(b);
        }
        if self.disjoint_classes(c1, c2) {
            return true;
        }
        // Two views of one class with arguments no value can have both of.
        if c1 == c2 {
            return self.disjoint_args(c1, args1, args2, depth, pending);
        }
        pending.push((a, b));
        let result = self.common_base_with_disjoint_args(a, b, c1, c2, depth, pending);
        pending.pop();
        result
    }

    /// Whether some common base class, not an ancestor of another common one, sees the two
    /// types with arguments no value has both of.
    fn common_base_with_disjoint_args(&mut self, a: TypeId, b: TypeId, c1: ClassId, c2: ClassId, depth: u32, pending: &mut Vec<(TypeId, TypeId)>) -> bool {
        self.complete_class(c1);
        self.complete_class(c2);
        let bases2: Vec<ClassId> = self.syms.class(c2).base_types.iter().map(|&(b, _)| b).collect();
        let common: Vec<ClassId> = self.syms.class(c1).base_types.iter().map(|&(b, _)| b).filter(|b| bases2.contains(b)).collect();
        for &bc in &common {
            if self.syms.class(bc).tparams.is_empty() || common.iter().any(|&other| other != bc && self.class_derives(other, bc)) {
                continue;
            }
            let (Some(x), Some(y)) = (self.base_type(a, bc), self.base_type(b, bc)) else { continue };
            if let (Type::Class(_, xs), Type::Class(_, ys)) = (self.types.get(x), self.types.get(y)) {
                if self.disjoint_args(bc, xs, ys, depth, pending) {
                    return true;
                }
            }
        }
        false
    }

    /// A literal that lies outside an enumeration of literals a type spells out (`1 | 2`)
    /// never meets it.
    fn literal_outside(&mut self, lit: TypeId, other: TypeId) -> bool {
        let mut alts = Vec::new();
        self.union_alternatives(other, &mut alts);
        !alts.is_empty() && alts.iter().all(|&alt| matches!(self.types.get(alt), Type::Lit(_)) && alt != lit)
    }

    /// Arguments of one class that no value has both of: disjoint arguments of a covariant
    /// parameter that a field has the type of, or of an invariant one where that holds or one
    /// of the arguments cannot be `Nothing`; a contravariant parameter proves nothing.
    fn disjoint_args(&mut self, c: ClassId, xs: TList, ys: TList, depth: u32, pending: &mut Vec<(TypeId, TypeId)>) -> bool {
        let xs: Vec<TypeId> = self.types.items(xs).to_vec();
        let ys: Vec<TypeId> = self.types.items(ys).to_vec();
        self.settle_class(c);
        let tparams = self.syms.class(c).tparams.clone();
        for i in 0..xs.len().min(ys.len()) {
            let Some(&p) = tparams.get(i) else { break };
            let variance = self.syms.tparam(p).variance;
            if variance < 0 || !self.disjoint_at(xs[i], ys[i], depth + 1, pending) {
                continue;
            }
            let is_field = self.param_is_field(c, p);
            if is_field || (variance == 0 && (self.cannot_be_nothing(xs[i]) || self.cannot_be_nothing(ys[i]))) {
                return true;
            }
        }
        false
    }

    /// Whether a constructor parameter of the class has exactly the type parameter as its type.
    fn param_is_field(&mut self, c: ClassId, p: TParamId) -> bool {
        self.complete_class(c);
        let param = self.types.param(p);
        self.syms.class(c).ctor.iter().any(|clause| clause.params.iter().any(|f| f.ty == param))
    }

    fn cannot_be_nothing(&mut self, t: TypeId) -> bool {
        let t = self.deref(t);
        !matches!(self.types.get(t), Type::Nothing | Type::Param(_) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Var(_) | Type::Wild | Type::BoundedWild(..))
    }

    /// Whether two classes have no common instance: neither derives from the other, and one is
    /// final, or their nearest non-trait ancestors are unrelated, or every child of a sealed one
    /// is disjoint from the other.
    fn disjoint_classes(&mut self, c1: ClassId, c2: ClassId) -> bool {
        if c1 == c2 || self.class_derives(c1, c2) || self.class_derives(c2, c1) {
            return false;
        }
        if self.is_final_class(c1) || self.is_final_class(c2) {
            return true;
        }
        let (n1, n2) = (self.non_trait_base(c1), self.non_trait_base(c2));
        if let (Some(n1), Some(n2)) = (n1, n2) {
            if n1 != n2 && !self.class_derives(n1, n2) && !self.class_derives(n2, n1) {
                return true;
            }
        }
        for (sealed, other, flip) in [(c1, c2, false), (c2, c1, true)] {
            if let Some(children) = self.sealed_children(sealed) {
                return children.iter().all(|&ch| if flip { self.disjoint_classes(other, ch) } else { self.disjoint_classes(ch, other) });
            }
        }
        false
    }

    fn class_derives(&mut self, sub: ClassId, sup: ClassId) -> bool {
        if Some(sup) == self.b.cons_tuple && self.is_tuple_class(sub) {
            return true;
        }
        self.complete_class(sub);
        self.syms.class(sub).base_types.iter().any(|&(b, _)| b == sup)
    }

    fn is_final_class(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        match info.kind {
            ClassKind::Object | ClassKind::Builtin => true,
            ClassKind::EnumCase => true,
            _ => info.mods & crate::ast::mods::FINAL != 0,
        }
    }

    fn non_trait_base(&mut self, c: ClassId) -> Option<ClassId> {
        self.complete_class(c);
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        bases.into_iter().find(|&b| !matches!(self.syms.class(b).kind, ClassKind::Trait | ClassKind::Enum))
    }

    fn sealed_children(&mut self, c: ClassId) -> Option<Vec<ClassId>> {
        self.complete_class(c);
        let info = self.syms.class(c);
        let sealed = info.kind == ClassKind::Enum || info.mods & crate::ast::mods::SEALED != 0;
        sealed.then(|| info.children.clone())
    }

    /// The type disjointness is decided on: an abstract type through its upper bound, a path
    /// through its type, a match type through its reduction or bound, a type-level operation
    /// through its value.
    fn disjointness_boundary(&mut self, t: TypeId) -> TypeId {
        let mut t = self.deref(t);
        for _ in 0..16 {
            let next = match self.types.get(t) {
                Type::Param(p) => self.syms.tparam(p).upper,
                Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => self.member_upper(t),
                Type::This(_) | Type::Term(_) | Type::Select(..) => self.path_underlying(t),
                Type::Refined(parent, _) => parent,
                Type::Wild => ANY,
                Type::BoundedWild(_, hi) => hi,
                Type::Var(_) | Type::AppVar(..) => ANY,
                Type::Class(c, _) if self.syms.class(c).kind == ClassKind::Opaque => {
                    self.complete_class(c);
                    let info = self.syms.class(c);
                    match info.base_types.get(1) {
                        Some(&(_, parent)) => parent,
                        None => ANY,
                    }
                }
                _ => match self.reduce_head(t) {
                    Some(r) => r,
                    None => match self.types.get(t) {
                        Type::Match(..) | Type::Alias(..) => self.match_bound(t),
                        _ => return t,
                    },
                },
            };
            if next == t {
                return t;
            }
            t = self.deref(next);
        }
        t
    }

    // ---- normalization ----

    /// The type with every match type that reduces replaced by its reduction, every type-level
    /// operation on literals by its value and every `h *: t` with a known tail by the tuple.
    pub fn normalize(&mut self, t: TypeId) -> TypeId {
        if !self.types.is_reducible(t) {
            return t;
        }
        if self.matches.stack.len() >= MAX_DEPTH {
            return t;
        }
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Match(..) => match self.reduce_match(t, t) {
                Some(r) => self.after_reduction(t, t, r, true),
                None => t,
            },
            Type::Alias(a, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let nargs: Vec<TypeId> = items.iter().map(|&x| self.normalize(x)).collect();
                let l = self.types.list(&nargs);
                let shown = self.types.mk(Type::Alias(a, l));
                // A curried alias application stays as written, as scalac keeps it, and reduces
                // where a use needs it: `(Eq |: Derived)[A]` unifies with an `F[T]` as such.
                if self.is_curried_alias(a) {
                    return shown;
                }
                let Some(expansion) = self.alias_expansion(a, l) else { return shown };
                match self.types.get(expansion) {
                    Type::Match(..) => match self.reduce_match(expansion, shown) {
                        Some(r) => self.after_reduction(expansion, shown, r, true),
                        None => shown,
                    },
                    _ if self.alias_by_name(a) => shown,
                    _ => self.normalize(expansion),
                }
            }
            Type::Class(c, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let nargs: Vec<TypeId> = items.iter().map(|&x| self.normalize(x)).collect();
                let nt = self.types.class(c, &nargs);
                self.reduce_head(nt).unwrap_or(nt)
            }
            Type::AppParam(p, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let nargs: Vec<TypeId> = items.iter().map(|&x| self.normalize(x)).collect();
                let l = self.types.list(&nargs);
                self.types.mk(Type::AppParam(p, l))
            }
            Type::AppVar(v, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let nargs: Vec<TypeId> = items.iter().map(|&x| self.normalize(x)).collect();
                let l = self.types.list(&nargs);
                self.types.mk(Type::AppVar(v, l))
            }
            Type::AppMember(m, args) => {
                let items: Vec<TypeId> = self.types.items(args).to_vec();
                let nargs: Vec<TypeId> = items.iter().map(|&x| self.normalize(x)).collect();
                self.types.apply_ctor(m, &nargs)
            }
            Type::Lambda(ps, body) => {
                let nb = self.normalize(body);
                self.types.mk(Type::Lambda(ps, nb))
            }
            Type::Union(a, b) => {
                let (na, nb) = (self.normalize(a), self.normalize(b));
                self.types.union(na, nb)
            }
            Type::Inter(a, b) => {
                let (na, nb) = (self.normalize(a), self.normalize(b));
                self.types.inter(na, nb)
            }
            Type::Select(p, s) => {
                let np = self.normalize(p);
                self.types.mk(Type::Select(np, s))
            }
            Type::Member(p, n) => {
                let np = self.normalize(p);
                self.types.mk(Type::Member(np, n))
            }
            Type::Refined(p, r) => {
                let np = self.normalize(p);
                let nr = match self.types.refinement(r) {
                    Refinement::Alias(n, rhs) => Refinement::Alias(n, self.normalize(rhs)),
                    Refinement::Bounds(n, lo, hi) => Refinement::Bounds(n, self.normalize(lo), self.normalize(hi)),
                    Refinement::Val(n, s, ty) => Refinement::Val(n, s, self.normalize(ty)),
                    Refinement::Term(n, s, l) => {
                        let items: Vec<TypeId> = self.types.items(l).to_vec();
                        let z: Vec<TypeId> = items.into_iter().map(|a| self.normalize(a)).collect();
                        Refinement::Term(n, s, self.types.list(&z))
                    }
                };
                let nr = self.types.refine(nr);
                self.types.mk(Type::Refined(np, nr))
            }
            _ => t,
        }
    }

    /// What the reduction `r` of the match type `t` goes on to: reduced again at its head, or
    /// normalized throughout with `deep`; the match type stays on the stack meanwhile, so that
    /// a reduction that comes back to it is reported and stops.
    fn after_reduction(&mut self, t: TypeId, shown: TypeId, r: TypeId, deep: bool) -> TypeId {
        if self.matches.stack.iter().any(|&(open, _)| open == t) || self.matches.stack.len() >= MAX_DEPTH {
            self.report_match_recursion(shown);
            return shown;
        }
        self.matches.stack.push((t, shown));
        let out = if deep { self.normalize(r) } else { self.reduce_head(r).unwrap_or(r) };
        self.matches.stack.pop();
        out
    }

    /// The type reduced at its head as far as it goes: a match type to its reduction, an alias
    /// application to its right-hand side, an operation on literals to its value, `h *: t` with
    /// a known tail to the tuple. None where the head stays as it is.
    pub fn reduce_head(&mut self, t: TypeId) -> Option<TypeId> {
        match self.types.get(t) {
            Type::Match(..) => {
                let r = self.reduce_match(t, t)?;
                let out = self.after_reduction(t, t, r, false);
                (out != t).then_some(out)
            }
            Type::Alias(a, args) => {
                let expansion = self.alias_expansion(a, args)?;
                match self.types.get(expansion) {
                    Type::Match(..) => {
                        let r = self.reduce_match(expansion, t)?;
                        let out = self.after_reduction(expansion, t, r, false);
                        (out != t).then_some(out)
                    }
                    _ => Some(expansion),
                }
            }
            Type::Class(c, args) if args != EMPTY_LIST && self.types.is_op_class(c) => {
                if Some(c) == self.b.cons_tuple {
                    // Flattening a cons walks its tail, which reduces the tails below: each
                    // is flattened once, as a match type is reduced once.
                    let cacheable = !self.types.has_vars(t) && self.gadt.is_empty() && !self.matches.recording;
                    if cacheable {
                        if let Some(&r) = self.matches.cache.get(&t) {
                            return r;
                        }
                    }
                    let [h, tail] = *self.types.items(args) else { return None };
                    let result = self.tuple_elements(tail).map(|elems| {
                        let mut all = vec![h];
                        all.extend(elems);
                        self.tuple_type(&all)
                    });
                    if cacheable {
                        self.matches.cache.insert(t, result);
                    }
                    return result;
                }
                self.fold_type_op(c, args)
            }
            _ => None,
        }
    }

    /// The upper bound a stuck match type is used as: its declared bound, or the union of its
    /// case bodies.
    pub fn match_bound(&mut self, t: TypeId) -> TypeId {
        let m = match self.types.get(t) {
            Type::Match(_, m) => m,
            Type::Alias(a, args) => {
                let Some(expansion) = self.alias_expansion(a, args) else { return ANY };
                match self.types.get(expansion) {
                    Type::Match(_, m) => m,
                    _ => return expansion,
                }
            }
            _ => return t,
        };
        self.types.match_info(m).bound
    }

    /// The match type behind an alias application, or the match type itself.
    pub(super) fn as_match(&mut self, t: TypeId) -> Option<TypeId> {
        match self.types.get(t) {
            Type::Match(..) => Some(t),
            Type::Alias(a, args) => {
                let expansion = self.alias_expansion(a, args)?;
                matches!(self.types.get(expansion), Type::Match(..)).then_some(expansion)
            }
            _ => None,
        }
    }

    /// Two stuck match types are the same when their scrutinees are and their cases agree
    /// pairwise, the first having at least the cases of the second.
    pub(super) fn same_match(&mut self, x: TypeId, y: TypeId) -> bool {
        if self.matches.comparing >= 8 {
            return false;
        }
        self.matches.comparing += 1;
        let same = self.same_match_cases(x, y);
        self.matches.comparing -= 1;
        same
    }

    fn same_match_cases(&mut self, x: TypeId, y: TypeId) -> bool {
        let (Type::Match(s1, m1), Type::Match(s2, m2)) = (self.types.get(x), self.types.get(y)) else { return false };
        if !self.is_same(s1, s2) {
            return false;
        }
        let cases1 = self.types.match_info(m1).cases.to_vec();
        let cases2 = self.types.match_info(m2).cases.to_vec();
        if cases1.len() < cases2.len() {
            return false;
        }
        cases1.iter().zip(&cases2).all(|(a, b)| {
            let n = self.types.items(a.binders).len();
            if n != self.types.items(b.binders).len() {
                return false;
            }
            let renaming: Subst = self
                .types
                .items(b.binders)
                .to_vec()
                .into_iter()
                .zip(self.types.items(a.binders).to_vec())
                .filter_map(|(from, to)| match self.types.get(from) {
                    Type::Param(p) => Some((p, to)),
                    _ => None,
                })
                .collect();
            let (bp, bb) = (self.types.subst(b.pattern, &renaming), self.types.subst(b.body, &renaming));
            self.is_same(a.pattern, bp) && self.is_sub(a.body, bb)
        })
    }

    /// `1 + 2`, `S[N]`, `"a" + "b"` as types: the operation of `scala.compiletime.ops` on its
    /// literal arguments, None where an argument is no literal.
    pub(super) fn fold_type_op(&mut self, op: ClassId, args: TList) -> Option<TypeId> {
        let module = self.compiletime_op_module(op)?;
        let items: Vec<TypeId> = self.types.items(args).to_vec();
        let mut values = Vec::with_capacity(items.len());
        for a in items {
            let a = self.normalize(a);
            values.push(self.fold_type(a)?);
        }
        let module = self.name_str(module);
        let op_name = self.name_str(self.syms.class(op).name);
        match super::inline::eval_type_op(&module, &op_name, &values, |n| self.name_str(n))? {
            super::inline::TypeOpResult::Lit(v) => Some(self.types.lit(v)),
            super::inline::TypeOpResult::Str(s) => {
                let n = self.interner.intern(&s);
                Some(self.types.lit(LitVal::Str(n)))
            }
        }
    }

    /// The classes whose applications the store flags as reducible: the operations of
    /// `scala.compiletime.ops` and the tuple cons `*:`.
    pub(super) fn mark_reducible_classes(&mut self) {
        if let Some(c) = self.b.cons_tuple {
            self.types.mark_op_class(c);
        }
        let Some(compiletime) = self.compiletime_pkg() else { return };
        let ops = self.interner.intern("ops");
        let Some(ops) = self.syms.pkg(compiletime).entries.get(&ops).and_then(|e| e.pkg) else { return };
        let modules: Vec<ClassId> = self
            .syms
            .pkg(ops)
            .entries
            .values()
            .filter_map(|e| match self.syms.sym(e.term?).kind {
                SymKind::Object(c) => Some(c),
                _ => None,
            })
            .collect();
        for m in modules {
            let nested: Vec<ClassId> = self.syms.class(m).nested.values().copied().collect();
            for c in nested {
                if self.syms.class(c).kind == ClassKind::Opaque {
                    self.types.mark_op_class(c);
                }
            }
        }
    }

    /// `case P => T` as scalac prints it.
    fn show_case(&mut self, c: MatchCase) -> String {
        if c.pattern == ANY {
            return format!("case _ => {}", self.show(c.body));
        }
        format!("case {} => {}", self.show(c.pattern), self.show(c.body))
    }
}
