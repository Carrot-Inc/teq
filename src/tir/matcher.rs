//! The match plans: one record per match, per try's cases and per pattern definition, which states as tests and lets
//! the decisions the writers make of its patterns (dotty's `PatternMatcher`'s plan: tests, lets, alternatives, the
//! cases in order). A plan is derived from its owner's typed patterns, never copied, and made where it is read, as
//! dotty's are made in PatternMatcher after typing: for the writers, by one pass once reach has typed everything
//! (`prepare`); for the interpreter, at a plan's first use (`build`, by the reader's own records). Every reader obtains
//! a plan for its current typed owner. A plan holds no writer's choice: each step carries the pattern it comes from and
//! its role in it, which a writer's own printing rules read.
//!
//! This first form states what the writers do today, no more: a class pattern's test and its fields, an extractor's
//! call and the pattern of its result, the sequence helper. The selector is variable 0, bound by the writer.

use crate::ast::ListRef;
use crate::types::{ClassId, SymId};

use super::{Program, TExpr, TExprId, TPat, TPatId, TStmt, TestId};

/// What a plan belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Owner {
    /// A `TExpr::Match` node.
    Match(TExprId),
    /// A try's cases, by the try's index in `Program::tries`.
    Try(u32),
    /// A pattern definition's pattern (`TStmt::Pat`).
    PatDef(TPatId),
}

/// A value the plan names: 0 is the selector.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Var(pub u32);

pub const SELECTOR: Var = Var(0);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TestKind {
    /// A type test, as `TPat::Test` holds it.
    Type(TestId),
    /// A class pattern's test of its class.
    Class(ClassId),
    /// The value compared with the expression, by `===` when the flag is set.
    Equal(TExprId, bool),
    /// The sequence helper's result is not null (the legacy helper's; T2's seqPattern row).
    SeqMatched,
    /// A case's guard.
    Guard(TExprId),
}

/// The value a let binds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rhs {
    /// The variable itself: a source binder, or an extractor's argument binder.
    Alias(Var),
    /// A class pattern's field of the variable.
    Field(Var, SymId),
    /// The sequence helper's element.
    Elem(Var, u32),
    /// An extractor's call, the typed node the typer made.
    Call(TExprId),
    /// The legacy sequence helper over the variable: its first `items` elements, and the rest where `rest`.
    SeqHelper { of: Var, items: u32, rest: bool },
}

/// What a let binds to: a variable of the plan's, or a symbol a typed expression names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bound {
    Var(Var),
    Sym(SymId),
}

/// A step's place in the pattern it comes from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// The pattern's own test, or its binder.
    Own,
    /// A class pattern's field, by its index.
    Field(u32),
    /// The sequence helper's element, by its index (the rest is the last).
    Elem(u32),
    /// An extractor's argument binder.
    UnapplyArg,
    /// An extractor's call.
    UnapplyCall,
    /// The sequence helper's call.
    SeqHelper,
}

/// A step of a case, in pre-order: a test or a let is followed by its continuation, the `scope` steps after it that
/// hold only where it does (dotty's TestPlan.onSuccess and LetPlan.body); the steps after those are its siblings.
/// So a writer takes a step's subtree whole: declares a binder after its pattern's own steps, or leaves a class
/// pattern's fields out where its test cannot pass.
#[derive(Clone, PartialEq, Debug)]
pub enum Step {
    Test { kind: TestKind, on: Var, origin: TPatId, scope: u32 },
    Let { bound: Bound, rhs: Rhs, origin: TPatId, role: Role, scope: u32 },
    /// Alternatives: each its own steps, on the same variable, none binding.
    Alts { alts: Vec<Vec<Step>>, origin: TPatId },
}

impl Step {
    /// How many of the steps after it are its continuation.
    pub fn scope(&self) -> usize {
        match self {
            Step::Test { scope, .. } | Step::Let { scope, .. } => *scope as usize,
            Step::Alts { .. } => 0,
        }
    }
}

impl MatchPlan {
    /// The steps of the `k`th case's pattern, its guard's test left out: for a writer that tests the guard itself.
    pub fn pattern_of(&self, k: usize) -> &[Step] {
        let c = &self.cases[k];
        let steps = &self.steps[c.start as usize..c.end as usize];
        match steps.last() {
            Some(Step::Test { kind: TestKind::Guard(_), .. }) => &steps[..steps.len() - 1],
            _ => steps,
        }
    }
}

/// For a case whose pattern is a class pattern of `n` fields (a tuple's), each field's continuation as a range of the
/// plan's steps, empty for a field its pattern leaves (a wildcard's): for a writer that holds the fields' values
/// apart, as the interpreter's match of a case lambda's parameters does.
pub fn component_ranges(plan: &MatchPlan, k: usize, n: usize) -> Vec<(u32, u32)> {
    let base = plan.cases[k].start as usize;
    let steps = plan.pattern_of(k);
    let mut ranges = vec![(0, 0); n];
    let Some(Step::Test { kind: TestKind::Class(_), .. }) = steps.first() else { return ranges };
    let fields = 1 + steps[0].scope();
    let mut j = 1;
    while j < fields {
        let end = j + 1 + steps[j].scope();
        if let Step::Let { role: Role::Field(m), .. } = steps[j] {
            ranges[m as usize] = ((base + j + 1) as u32, (base + end) as u32);
        }
        j = end;
    }
    ranges
}

/// A case: its steps, its pattern's and then its guard's test, as a range of the plan's steps. A pattern definition's
/// one case has no case index.
#[derive(Clone, PartialEq, Debug)]
pub struct CasePlan {
    pub case: Option<u32>,
    pub start: u32,
    pub end: u32,
}

/// The plan of a match, a try's cases or a pattern definition: the cases in order, each taken when its steps hold;
/// when none does, the owner's failure (a MatchError, or the caught value thrown again).
#[derive(Clone, PartialEq, Debug)]
pub struct MatchPlan {
    pub owner: Owner,
    pub cases: Vec<CasePlan>,
    /// The cases' steps, one case's after another's.
    pub steps: Vec<Step>,
    /// How many variables the plan names, the selector among them.
    pub vars: u32,
}

/// The plan of `owner` as its typed patterns are now, or None where the owner is no longer a match, a try or a pattern
/// definition's pattern, or is no record the reader may read. The owner is looked up among the records the reader can
/// read (`Arena::try_get`): a worker's own, the shared region's published ones and the other workers' published ones,
/// whatever the reader's next allocation is. It reads the program alone: it makes no node, symbol or name.
pub fn build(prog: &Program, owner: Owner) -> Option<MatchPlan> {
    let mut b = Builder { prog, vars: 1 };
    let mut steps = Vec::new();
    let cases = match owner {
        Owner::Match(e) => match *prog.exprs.try_get(e.0)? {
            TExpr::Match(_, cases) => b.cases(cases, &mut steps),
            _ => return None,
        },
        Owner::Try(i) => b.cases(prog.tries.try_get(i)?.cases, &mut steps),
        Owner::PatDef(p) => {
            prog.pats.try_get(p.0)?;
            b.pattern(p, SELECTOR, &mut steps);
            vec![CasePlan { case: None, start: 0, end: steps.len() as u32 }]
        }
    };
    Some(MatchPlan { owner, cases, steps, vars: b.vars })
}


struct Builder<'p> {
    prog: &'p Program,
    vars: u32,
}

impl Builder<'_> {
    fn var(&mut self) -> Var {
        self.vars += 1;
        Var(self.vars - 1)
    }

    fn cases(&mut self, list: ListRef, steps: &mut Vec<Step>) -> Vec<CasePlan> {
        let prog = self.prog;
        (0..list.len)
            .map(|k| {
                let case = &prog.cases[(list.start + k) as usize];
                let start = steps.len() as u32;
                self.pattern(case.pat, SELECTOR, steps);
                if let Some(g) = case.guard {
                    steps.push(Step::Test { kind: TestKind::Guard(g), on: SELECTOR, origin: case.pat, scope: 0 });
                }
                CasePlan { case: Some(list.start + k), start, end: steps.len() as u32 }
            })
            .collect()
    }

    /// The steps of `pat` against `on`, in the order the writers take them today.
    fn pattern(&mut self, pat: TPatId, on: Var, out: &mut Vec<Step>) {
        let prog = self.prog;
        match prog.pats[pat.idx()] {
            TPat::Wildcard => {}
            TPat::Bind(sym, inner) => {
                out.push(Step::Let { bound: Bound::Sym(sym), rhs: Rhs::Alias(on), origin: pat, role: Role::Own, scope: 0 });
                let at = out.len() - 1;
                if let Some(i) = inner {
                    self.pattern(i, on, out);
                }
                close(out, at);
            }
            TPat::Test(test, _, inner) => {
                out.push(Step::Test { kind: TestKind::Type(test), on, origin: pat, scope: 0 });
                let at = out.len() - 1;
                self.pattern(inner, on, out);
                close(out, at);
            }
            TPat::Equals(e, strict) => out.push(Step::Test { kind: TestKind::Equal(e, strict), on, origin: pat, scope: 0 }),
            TPat::Class(c, _, fields, subs) => {
                out.push(Step::Test { kind: TestKind::Class(c), on, origin: pat, scope: 0 });
                let class_at = out.len() - 1;
                let fields = prog.sym_list(fields);
                for (i, (&f, &sub)) in fields.iter().zip(prog.pat_list(subs)).enumerate() {
                    if matches!(prog.pats[sub.idx()], TPat::Wildcard) {
                        continue;
                    }
                    let v = self.var();
                    out.push(Step::Let { bound: Bound::Var(v), rhs: Rhs::Field(on, f), origin: sub, role: Role::Field(i as u32), scope: 0 });
                    let at = out.len() - 1;
                    self.pattern(sub, v, out);
                    close(out, at);
                }
                close(out, class_at);
            }
            TPat::Alt(alts) => {
                let alts = prog
                    .pat_list(alts)
                    .iter()
                    .map(|&a| {
                        let mut steps = Vec::new();
                        self.pattern(a, on, &mut steps);
                        steps
                    })
                    .collect();
                out.push(Step::Alts { alts, origin: pat });
            }
            TPat::Unapply(sym, call, inner) => {
                out.push(Step::Let { bound: Bound::Sym(sym), rhs: Rhs::Alias(on), origin: pat, role: Role::UnapplyArg, scope: 0 });
                let arg_at = out.len() - 1;
                let u = self.var();
                out.push(Step::Let { bound: Bound::Var(u), rhs: Rhs::Call(call), origin: pat, role: Role::UnapplyCall, scope: 0 });
                let call_at = out.len() - 1;
                self.pattern(inner, u, out);
                close(out, call_at);
                close(out, arg_at);
            }
            TPat::Seq(items, rest) => {
                let q = self.var();
                out.push(Step::Let {
                    bound: Bound::Var(q),
                    rhs: Rhs::SeqHelper { of: on, items: items.len, rest: rest.is_some() },
                    origin: pat,
                    role: Role::SeqHelper,
                    scope: 0,
                });
                let helper_at = out.len() - 1;
                out.push(Step::Test { kind: TestKind::SeqMatched, on: q, origin: pat, scope: 0 });
                let matched_at = out.len() - 1;
                let subs: Vec<TPatId> = prog.pat_list(items).iter().copied().chain(rest).collect();
                for (i, sub) in subs.into_iter().enumerate() {
                    if matches!(prog.pats[sub.idx()], TPat::Wildcard) {
                        continue;
                    }
                    let v = self.var();
                    out.push(Step::Let { bound: Bound::Var(v), rhs: Rhs::Elem(q, i as u32), origin: sub, role: Role::Elem(i as u32), scope: 0 });
                    let at = out.len() - 1;
                    self.pattern(sub, v, out);
                    close(out, at);
                }
                close(out, matched_at);
                close(out, helper_at);
            }
        }
    }
}

/// The steps pushed after the one at `at` are its continuation.
fn close(out: &mut [Step], at: usize) {
    let n = (out.len() - at - 1) as u32;
    if let Step::Test { scope, .. } | Step::Let { scope, .. } = &mut out[at] {
        *scope = n;
    }
}

/// THE PASS: every plan of the program made from its patterns, into the table the writers read, the table's earlier
/// plans dropped. It runs once reach and every other typing are done and immediately before a writer (the JavaScript
/// emitter, the JVM's lowering, a resident session's build), on the program one worker's again: its records are found by
/// a scan of the arenas, every match, every try and every pattern definition's statement, whatever block holds it.
pub fn prepare(prog: &mut Program) {
    assert!(!prog.exprs.is_shared() && !prog.stmts.is_shared() && !prog.tries.is_shared(), "the plans' pass on a program of several workers");
    let mut table = crate::intern::FxMap::default();
    let mut put = |owner: Owner| {
        if let Some(plan) = build(prog, owner) {
            table.insert(owner, plan);
        }
    };
    for (i, e) in prog.exprs.iter().enumerate() {
        if let TExpr::Match(..) = e {
            put(Owner::Match(TExprId(i as u32)));
        }
    }
    for st in prog.stmts.iter() {
        if let TStmt::Pat(p, _, _) = *st {
            put(Owner::PatDef(p));
        }
    }
    for i in 0..prog.tries.len() as u32 {
        put(Owner::Try(i));
    }
    prog.matchers = table;
}

/// The writers' table released once its writer has read it (a resident session's builds): the next build's pass makes
/// it again (`prepare`), so that no plan is held between builds.
pub fn release(prog: &mut Program) {
    prog.matchers = Default::default();
}

/// The plans' checker, for a reader's entry: under `TEQ_CHECK_MATCH_PLANS=<file>` the plan held for `owner` is compared
/// with one built now, and a line is appended to the file: `ok`, `missing` (none held), `stale` (another held) or
/// `stray` (one held for an owner that is none), with the reader, the owner and the command's arguments. Nothing is
/// written without the variable, and nothing else changes with it.
pub fn check(prog: &Program, owner: Owner, reader: &str) {
    let Some(path) = checking() else { return };
    let outcome = outcome(prog, owner);
    tell(path, outcome, reader, owner);
}

/// The checker for a plan a reader holds, a copy it made at its first use (the interpreter's): under
/// `TEQ_CHECK_MATCH_PLANS` it is compared, where it is used, with the plan its owner's patterns give now: `ok`, `stale`
/// (another), or `stray` (its owner is none now).
pub fn check_held(prog: &Program, held: &MatchPlan, reader: &str) {
    let Some(path) = checking() else { return };
    let outcome = match build(prog, held.owner) {
        Some(now) if now == *held => "ok",
        Some(_) => "stale",
        None => "stray",
    };
    tell(path, outcome, reader, held.owner);
}

thread_local! {
    /// Under the checker: the owners this thread's interpreter made a held plan of, and the patterns those plans read,
    /// since its caches last went (`note_held`, `rewriting`). Its lowered snapshots take their plans from the held ones.
    static HELD: std::cell::RefCell<(crate::intern::FxMap<Owner, ()>, crate::intern::FxMap<TPatId, ()>)> = Default::default();
}

/// Under the checker, the interpreter's note of a plan it has made and holds (`Interp::held_plan`).
#[cold]
#[inline(never)]
pub fn note_held(plan: &MatchPlan) {
    fn origins(steps: &[Step], pats: &mut crate::intern::FxMap<TPatId, ()>) {
        for s in steps {
            match s {
                Step::Test { origin, .. } | Step::Let { origin, .. } => {
                    pats.insert(*origin, ());
                }
                Step::Alts { alts, origin } => {
                    pats.insert(*origin, ());
                    for a in alts {
                        origins(a, pats);
                    }
                }
            }
        }
    }
    HELD.with(|h| {
        let mut h = h.borrow_mut();
        h.0.insert(plan.owner, ());
        origins(&plan.steps, &mut h.1);
    });
}

/// The interpreter's caches went on this thread, both views': no plan is held on it any more.
pub fn forget_held_notes() {
    HELD.with(|h| *h.borrow_mut() = Default::default());
}

/// Under the checker, at an in-place rewrite of a match's patterns (`owner`: substitution's walk of the match it
/// copied) or of one pattern (`pat`: a copy's specialisation). Only a copy that no interpreter path has run yet is
/// rewritten so, which is checked here: a plan of it held on this thread, in either view's caches, a running
/// interpreter's or a lowered snapshot, is a fault, `held-at-rewrite`.
#[inline]
pub fn rewriting(owner: Option<Owner>, pat: Option<TPatId>, site: &str) {
    if let Some(path) = checking() {
        rewriting_checked(path, owner, pat, site);
    }
}

#[cold]
#[inline(never)]
fn rewriting_checked(path: &str, owner: Option<Owner>, pat: Option<TPatId>, site: &str) {
    let held = HELD.with(|h| {
        let h = h.borrow();
        owner.is_some_and(|o| h.0.contains_key(&o)) || pat.is_some_and(|p| h.1.contains_key(&p))
    });
    if held {
        let named = owner.or(pat.map(Owner::PatDef)).expect("an owner or a pattern");
        tell(path, "held-at-rewrite", site, named);
    }
}

/// An outcome: `ok` counted by reader and written once at the process's exit (`count=N`), any other written at once
/// with its owner and the command's arguments.
fn tell(path: &str, outcome: &str, reader: &str, owner: Owner) {
    if outcome == "ok" {
        let mut counts = OK_COUNTS.lock().unwrap_or_else(|e| e.into_inner());
        match counts.iter_mut().find(|(r, _)| r == reader) {
            Some((_, n)) => *n += 1,
            None => counts.push((reader.to_string(), 1)),
        }
        return;
    }
    let args: String = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    append(path, &format!("{} {} {:?} {}\n", outcome, reader, owner, args));
}

fn append(path: &str, text: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        use std::io::Write;
        let _ = f.write_all(text.as_bytes());
    }
}

static OK_COUNTS: std::sync::Mutex<Vec<(String, u64)>> = std::sync::Mutex::new(Vec::new());

/// The `ok` counts, written when the process ends.
extern "C" fn write_ok_counts() {
    let Some(path) = checking() else { return };
    let counts = OK_COUNTS.lock().unwrap_or_else(|e| e.into_inner());
    let text: String = counts.iter().map(|(reader, n)| format!("ok {} count={}\n", reader, n)).collect();
    append(path, &text);
}

/// Whether the plans' checker is on, for a reader that reads it once.
pub fn checking_on() -> bool {
    checking().is_some()
}

fn checking() -> Option<&'static str> {
    static PATH: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    PATH.get_or_init(|| {
        let path = std::env::var("TEQ_CHECK_MATCH_PLANS").ok().filter(|p| !p.is_empty());
        if path.is_some() {
            extern "C" {
                fn atexit(f: extern "C" fn()) -> std::os::raw::c_int;
            }
            unsafe { atexit(write_ok_counts) };
        }
        path
    })
    .as_deref()
}

/// The checker's outcome for `owner`: `ok` where the plan held is the one its patterns give now.
pub fn outcome(prog: &Program, owner: Owner) -> &'static str {
    match (prog.matchers.get(&owner), &build(prog, owner)) {
        (Some(held), Some(now)) if held == now => "ok",
        (Some(_), Some(_)) => "stale",
        (None, Some(_)) => "missing",
        (Some(_), None) => "stray",
        (None, None) => "no-owner",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tir::TCase;

    /// The steps of the plan's `k`th case.
    fn steps_of(plan: &MatchPlan, k: usize) -> &[Step] {
        let c = &plan.cases[k];
        &plan.steps[c.start as usize..c.end as usize]
    }

    /// `s match { case E(_) => () }`: an extractor's pattern, its call a node of its own.
    fn program() -> (Program, TExprId, TPatId) {
        let mut prog = Program::default();
        let scrut = prog.add(TExpr::Unit);
        let call = prog.add(TExpr::Unit);
        let inner = prog.add_pat(TPat::Wildcard);
        let pat = prog.add_pat(TPat::Unapply(SymId(1), call, inner));
        let body = prog.add(TExpr::Unit);
        let cases = prog.cases.push_slice(&[TCase { pat, guard: None, body }]);
        let m = prog.add(TExpr::Match(scrut, cases));
        (prog, m, pat)
    }

    #[test]
    fn a_match_has_no_plan_until_the_pass_and_the_passs_is_ok() {
        let (mut prog, m, _) = program();
        // Typing makes no plan: the writers' are made by the pass.
        assert_eq!(outcome(&prog, Owner::Match(m)), "missing");
        prepare(&mut prog);
        assert_eq!(outcome(&prog, Owner::Match(m)), "ok");
        let plan = prog.matchers.get(&Owner::Match(m)).unwrap();
        assert!(matches!(steps_of(plan, 0), [Step::Let { rhs: Rhs::Alias(SELECTOR), .. }, Step::Let { rhs: Rhs::Call(_), .. }]));
    }

    #[test]
    fn each_step_holds_its_continuation_as_its_scope() {
        // `s match { case x @ C(a, _) => () }`
        let mut prog = Program::default();
        let scrut = prog.add(TExpr::Unit);
        let a = prog.add_pat(TPat::Bind(SymId(2), None));
        let wild = prog.add_pat(TPat::Wildcard);
        let fields = prog.sym_lists.push_slice(&[SymId(3), SymId(4)]);
        let subs = prog.pat_lists.push_slice(&[a, wild]);
        let class = prog.add_pat(TPat::Class(crate::types::ClassId(1), crate::types::TypeId(0), fields, subs));
        let pat = prog.add_pat(TPat::Bind(SymId(1), Some(class)));
        let body = prog.add(TExpr::Unit);
        let cases = prog.cases.push_slice(&[TCase { pat, guard: None, body }]);
        let m = prog.add(TExpr::Match(scrut, cases));
        prepare(&mut prog);
        let steps = steps_of(prog.matchers.get(&Owner::Match(m)).unwrap(), 0);
        let scope = |s: &Step| match s {
            Step::Test { scope, .. } | Step::Let { scope, .. } => *scope as usize,
            Step::Alts { .. } => 0,
        };
        let shape: Vec<(&str, usize)> = steps
            .iter()
            .map(|s| match s {
                Step::Let { rhs: Rhs::Alias(_), .. } => ("alias", scope(s)),
                Step::Test { kind: TestKind::Class(_), .. } => ("class", scope(s)),
                Step::Let { rhs: Rhs::Field(..), .. } => ("field", scope(s)),
                _ => ("other", scope(s)),
            })
            .collect();
        assert_eq!(shape, [("alias", 3), ("class", 2), ("field", 1), ("alias", 0)]);
    }

    #[test]
    fn a_pattern_rewritten_after_the_build_makes_its_plan_stale() {
        let (mut prog, m, pat) = program();
        prepare(&mut prog);
        let TPat::Unapply(sym, _, inner) = prog.pats[pat.idx()] else { unreachable!() };
        let other = prog.add(TExpr::Unit);
        prog.pats[pat.idx()] = TPat::Unapply(sym, other, inner);
        assert_eq!(outcome(&prog, Owner::Match(m)), "stale");
    }

    #[test]
    fn a_plan_whose_owner_is_no_match_any_more_is_stray() {
        let (mut prog, m, _) = program();
        prepare(&mut prog);
        prog.exprs[m.idx()] = TExpr::Unit;
        assert_eq!(outcome(&prog, Owner::Match(m)), "stray");
        assert_eq!(outcome(&prog, Owner::Try(7)), "no-owner");
    }

    /// A match, a try and a pattern definition's pattern, each an extractor's (`program`'s shape), made in `prog`.
    fn owners(prog: &mut Program) -> (TExprId, u32, TPatId) {
        let make_cases = |prog: &mut Program| {
            let call = prog.add(TExpr::Unit);
            let inner = prog.add_pat(TPat::Wildcard);
            let pat = prog.add_pat(TPat::Unapply(SymId(1), call, inner));
            let body = prog.add(TExpr::Unit);
            prog.cases.push_slice(&[TCase { pat, guard: None, body }])
        };
        let scrut = prog.add(TExpr::Unit);
        let cases = make_cases(prog);
        let m = prog.add(TExpr::Match(scrut, cases));
        let body = prog.add(TExpr::Unit);
        let cases = make_cases(prog);
        let t = prog.tries.push(crate::tir::TTry { body, cases, finalizer: None, wraps: false });
        let call = prog.add(TExpr::Unit);
        let inner = prog.add_pat(TPat::Wildcard);
        let d = prog.add_pat(TPat::Unapply(SymId(1), call, inner));
        (m, t, d)
    }

    #[test]
    fn a_pattern_definition_outside_any_block_has_its_plan() {
        let mut prog = Program::default();
        let (_, _, d) = owners(&mut prog);
        let init = prog.add(TExpr::Unit);
        // A statement list no Block holds, as a class's prelude is.
        prog.stmts.push_slice(&[TStmt::Pat(d, init, crate::tir::PatDefForm::General)]);
        prepare(&mut prog);
        assert_eq!(outcome(&prog, Owner::PatDef(d)), "ok");
    }

    #[test]
    fn a_plan_is_made_for_a_readable_owner_of_another_worker_either_way() {
        let mut first = Program::default();
        first.add(TExpr::Unit);
        first.fork();
        let mut seventh = first.attach(7);
        let (m7, t7, d7) = owners(&mut seventh);
        let (m0, t0, d0) = owners(&mut first);
        // Each worker's records published to the other, as a publication's walk escapes and seals them.
        #[cfg(debug_assertions)]
        for (p, w) in [(&first, 0), (&seventh, 7)] {
            p.escape_own();
            p.tries.escape_own();
            let base = crate::arena::worker_base(w);
            for i in base..p.exprs.len() as u32 {
                p.exprs.seal(i);
            }
            for i in base..p.pats.len() as u32 {
                p.pats.seal(i);
            }
            for i in base..p.cases.len() as u32 {
                p.cases.seal(i);
            }
            for i in base..p.tries.len() as u32 {
                p.tries.seal(i);
            }
        }
        // Worker 0 reads worker 7's owners, whose ids are past worker 0's next allocation, and worker 7 reads worker 0's.
        for (reader, m, t, d) in [(&first, m7, t7, d7), (&seventh, m0, t0, d0)] {
            assert!(build(reader, Owner::Match(m)).is_some(), "a match of another worker");
            assert!(build(reader, Owner::Try(t)).is_some(), "a try of another worker");
            assert!(build(reader, Owner::PatDef(d)).is_some(), "a pattern definition of another worker");
        }
        assert!(build(&first, Owner::Match(TExprId(m7.0 + 1000))).is_none(), "an id no record has");
    }

    #[test]
    fn the_loader_locks_holder_has_plans_of_its_staged_owners_and_its_own() {
        let mut first = Program::default();
        first.add(TExpr::Unit);
        first.fork();
        let (m0, t0, d0) = owners(&mut first);
        let lock = crate::shared::ReentrantLock::new();
        lock.lock();
        first.exprs.lock_taken();
        first.pats.lock_taken();
        first.cases.lock_taken();
        first.tries.lock_taken();
        for a in [&mut first.exprs.alloc_shared, &mut first.pats.alloc_shared, &mut first.cases.alloc_shared, &mut first.tries.alloc_shared] {
            *a = true;
        }
        let (ms, ts, ds) = owners(&mut first);
        let local = crate::arena::LOCAL_BASE;
        assert!(ms.0 < local && ts < local && ds.0 < local, "the owners made under the lock are staged in the shared region");
        assert!(m0.0 >= local && t0 >= local && d0.0 >= local, "the owners made before the lock are the worker's own");
        for (m, t, d) in [(ms, ts, ds), (m0, t0, d0)] {
            assert!(build(&first, Owner::Match(m)).is_some(), "a match of the lock holder's");
            assert!(build(&first, Owner::Try(t)).is_some(), "a try of the lock holder's");
            assert!(build(&first, Owner::PatDef(d)).is_some(), "a pattern definition of the lock holder's");
        }
        for a in [&mut first.exprs.alloc_shared, &mut first.pats.alloc_shared, &mut first.cases.alloc_shared, &mut first.tries.alloc_shared] {
            *a = false;
        }
        lock.unlock();
    }
}
