//! Exhaustivity and reachability of matches through a bounded space algebra (after scalac's
//! `transform/patmat/Space`): a space is a set of values, patterns and scrutinee types project
//! into spaces, a case is unreachable when the cases before it leave nothing of its space, and
//! what the scrutinee's space keeps after every case is taken away is what the match misses.
//!
//! The spaces of a match live in an arena that the next match reuses: a space is a small copyable
//! value, and the parts of a product or an alternative are a run of the arena.

use super::Worker;
use crate::ast::{mods, CaseClause};
use crate::intern::FxMap;
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::ops::Range;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Space {
    Empty,
    /// Every value of the type.
    Typ(TypeId),
    Lit(Lit),
    /// The values of a case class or tuple whose fields lie in the given spaces.
    Prod(ClassId, TypeId, Items),
    Or(Items),
    /// A pattern the algebra does not follow, such as a sequence pattern or a stable identifier
    /// that is no enum case: it covers nothing and is never unreachable.
    Opaque,
}

/// A case's pattern as the reachability of later cases sees it (`Spaces::case_kind`).
#[derive(Clone, Copy, PartialEq, Debug)]
enum CaseKind {
    Typ,
    Shape,
    Extractor(SymId, Path),
}

impl CaseKind {
    /// Whether an earlier case of the kind `earlier` takes away what a case of this kind
    /// covers: every case for a type test or a wildcard, all but the extractors for a
    /// constructor pattern, the type tests and the same `unapply`'s for an extractor.
    fn judged_by(self, earlier: CaseKind) -> bool {
        match self {
            CaseKind::Typ => true,
            CaseKind::Shape => !matches!(earlier, CaseKind::Extractor(..)),
            CaseKind::Extractor(..) => earlier == CaseKind::Typ || earlier == self,
        }
    }
}

/// The path an extractor's `unapply` is called on: two calls are of the same `unapply` where
/// the method and the path are the same.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Path {
    Static,
    /// A stable path by its digest (`Spaces::stable_path`).
    Stable(u64),
    /// What is no path: the case's own.
    Case(u32),
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Lit {
    Int(i32),
    Long(i64),
    Double(u64),
    Bool(bool),
    Char(u16),
    Str(StrRef),
}

/// A run of an arena.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Items {
    start: u32,
    len: u32,
}

impl Items {
    fn range(self) -> Range<usize> {
        self.start as usize..(self.start + self.len) as usize
    }

    fn is_empty(self) -> bool {
        self.len == 0
    }
}

/// How many types a match may split into their cases before its analysis is given up.
const DECOMPOSITIONS: u32 = 64;
const SHOWN_MISSING: usize = 6;

#[derive(Default)]
pub struct SpaceScratch {
    arena: Vec<Space>,
    /// The parts of the spaces under construction; a space's parts are moved from the top of
    /// the stack into the arena once they are all built.
    stack: Vec<Space>,
    decomposed: Vec<(TypeId, Option<Items>)>,
    fields: Vec<(TypeId, Option<Items>)>,
    field_arena: Vec<TypeId>,
    /// Subtype answers of this match that needed the typer, which the parts of a space ask for
    /// again and again.
    subs: FxMap<(TypeId, TypeId), bool>,
    /// The spaces of the unguarded cases so far.
    covered: Vec<Space>,
    plain: Vec<PlainCase>,
    classes: Vec<ClassId>,
    covered_classes: Vec<ClassId>,
    /// The array type of the test `plain` classified last, where the target tests an array by
    /// its elements, and those of the unguarded cases so far.
    array_test: Option<TypeId>,
    arrays: Vec<TypeId>,
    /// The classes the cases have to cover: the sealed scrutinee class, or the members of a
    /// union scrutinee, each with the scrutinee type when it fixes the type arguments.
    roots: Vec<(ClassId, Option<TypeId>)>,
    /// The class of a scrutinee that is no sealed type: what a case has to relate to.
    scrut_class: Option<ClassId>,
    types: Vec<TypeId>,
}

impl SpaceScratch {
    fn clear(&mut self) {
        self.arena.clear();
        self.stack.clear();
        self.decomposed.clear();
        self.fields.clear();
        self.field_arena.clear();
        self.subs.clear();
        self.covered.clear();
        self.plain.clear();
        self.classes.clear();
        self.covered_classes.clear();
        self.array_test = None;
        self.arrays.clear();
        self.roots.clear();
        self.scrut_class = None;
        self.types.clear();
    }
}

/// A case the classes alone describe, with the classes it names as a range of `classes`.
struct PlainCase {
    shape: Plain,
    classes: Range<usize>,
    /// The type of a test against an array that looks at the elements.
    array: Option<TypeId>,
}

#[derive(Clone, Copy, PartialEq)]
enum Plain {
    /// Takes every value.
    Total,
    /// Takes every value of its classes.
    Covers,
    /// Has a literal (or a value the algebra does not follow) somewhere inside, so it takes no
    /// whole class and covers nothing; its classes are the ones it needs values of.
    Literal,
}

struct Spaces<'t, 'a> {
    t: &'t mut Worker<'a>,
    sc: SpaceScratch,
    budget: u32,
    /// Set when a pattern's coverage was approximated, which rules out exhaustivity warnings.
    inexact: bool,
    /// Whether a type pattern over an abstract type takes its bound's children from what it is
    /// subtracted from (dotc's `tryDecompose2` in `minus`, Space.scala 269): on for the
    /// exhaustivity of the whole match, off while the cases are checked for reachability, where
    /// dotc's `isSubspace` of a case against a single type pattern is a subtype test alone
    /// (`computeIsSubspace`, `rhsDecompositionAllowed`).
    decompose_abstract: bool,
}

/// A match's check of its cases (`Worker::check_match`), kept until every body is typed: what
/// its cases cover and leave depends on the classes that extend its scrutinee's, which bodies
/// make (an anonymous class, a local one), whatever order they are typed in, as scalac checks
/// a match after its typer.
#[derive(Clone)]
pub struct DeferredMatch {
    pub sty: TypeId,
    pub cases: Vec<TCase>,
    pub clauses: Vec<CaseClause>,
    pub partial: bool,
    pub span: Span,
    pub file: FileId,
    pub quiet: bool,
    /// A case's pattern or guard is malformed (`Ast::broken_cases`): what the check finds
    /// depends on it.
    pub broken: bool,
    /// The opaque types seen through where the match is, and the facts its enclosing matches'
    /// cases give of type parameters (`Worker::gadt`).
    pub transparent: Vec<ClassId>,
    pub gadt: Vec<(TParamId, TypeId, i8)>,
    /// The inline definition whose check left it, reported at the definition once: the same
    /// check by each worker that asked for it.
    pub definition: Option<SymId>,
}

impl<'a> Worker<'a> {
    /// The check of a match's cases, kept for `run_deferred_matches` in the program's files;
    /// in the std's and the libraries' bodies, which warn about nothing, run at once.
    pub(super) fn defer_match_check(&mut self, sty: TypeId, cases: Vec<TCase>, clauses: Vec<CaseClause>, partial: bool, span: Span, broken: bool) {
        if self.inline.lenient_match > 0 || self.match_checked_at_definition(span) {
            return;
        }
        let file = self.env.file;
        if self.source(file).is_std || self.is_body_file(file) {
            return self.check_match(sty, &cases, &clauses, partial, span);
        }
        let (transparent, gadt) = (self.transparent.clone(), self.gadt.clone());
        self.note_rare(super::state::Rare::DeferredMatches, self.deferred_matches.len());
        self.deferred_matches.push(DeferredMatch { sty, cases, clauses, partial, span, file, quiet: self.nowarn > 0, broken, transparent, gadt, definition: None });
    }

    /// Runs the matches' checks the bodies kept, over the program as it stands once every body
    /// is typed, in the order of their files and positions.
    pub fn run_deferred_matches(&mut self) {
        let mut matches = std::mem::take(&mut self.deferred_matches);
        matches.sort_by_key(|m| (m.file, m.span.start, m.span.end));
        let saved_file = self.env.file;
        let saved_nowarn = self.nowarn;
        let saved_transparent = std::mem::take(&mut self.transparent);
        let saved_gadt = std::mem::take(&mut self.gadt);
        let saved_dependent = self.dependent_checks;
        for m in matches {
            self.env.file = m.file;
            self.nowarn = m.quiet as u32;
            self.dependent_checks = m.broken as u32;
            self.transparent = m.transparent;
            self.gadt = m.gadt;
            self.check_match(m.sty, &m.cases, &m.clauses, m.partial, m.span);
        }
        self.dependent_checks = saved_dependent;
        self.env.file = saved_file;
        self.nowarn = saved_nowarn;
        self.transparent = saved_transparent;
        self.gadt = saved_gadt;
    }

    /// Warns about cases no value reaches and, for a scrutinee of a sealed type, `Boolean`, a
    /// union or a case class of such fields, about the values no case takes. Cases made of plain
    /// constructors, values and type tests are counted by class, and so are patterns with a
    /// literal inside, which cover nothing; a nested constructor, a `Boolean` literal or a
    /// union with a member that is no class goes through the algebra.
    pub fn check_match(&mut self, sty: TypeId, cases: &[TCase], clauses: &[CaseClause], partial: bool, span: Span) {
        if self.inline.lenient_match > 0 {
            return;
        }
        let sty = self.match_type(sty);
        let scrut = self.deref(sty);
        if matches!(self.types.get(scrut), Type::Error | Type::Nothing) {
            return;
        }
        self.demand_match_hierarchy(scrut, cases);
        let mut sc = std::mem::take(&mut self.spaces);
        sc.clear();
        let mut s = Spaces { t: self, sc, budget: DECOMPOSITIONS, inexact: false, decompose_abstract: false };
        s.check(scrut, cases, clauses, partial, span);
        let Spaces { sc, .. } = s;
        self.spaces = sc;
    }
}

impl<'a> Worker<'a> {
    /// The std classes the scrutinee and the patterns name know their subclasses across the
    /// std's files before the spaces are compared.
    fn demand_match_hierarchy(&mut self, scrut: TypeId, cases: &[TCase]) {
        if self.std.slots.is_empty() {
            return;
        }
        let mut classes: Vec<ClassId> = Vec::new();
        self.classes_of_type(scrut, &mut classes);
        let mut pats: Vec<TPatId> = cases.iter().map(|c| c.pat).collect();
        while let Some(p) = pats.pop() {
            match self.prog.pats[p.idx()] {
                TPat::Wildcard | TPat::Equals(..) | TPat::Bind(_, None) => {}
                TPat::Bind(_, Some(q)) | TPat::Unapply(_, _, q) => pats.push(q),
                TPat::Test(_, ty, q) => {
                    self.classes_of_type(ty, &mut classes);
                    pats.push(q);
                }
                TPat::Class(c, ty, _, subs) => {
                    classes.push(c);
                    self.classes_of_type(ty, &mut classes);
                    pats.extend(self.prog.pat_lists[subs.range()].iter().copied());
                }
                TPat::Alt(alts) => pats.extend(self.prog.pat_lists[alts.range()].iter().copied()),
                TPat::Seq(items, rest) => {
                    pats.extend(self.prog.pat_lists[items.range()].iter().copied());
                    pats.extend(rest);
                }
            }
        }
        classes.sort_unstable();
        classes.dedup();
        for c in classes {
            self.demand_extenders_transitive(c);
        }
    }

    pub(super) fn classes_of_type(&mut self, t: TypeId, out: &mut Vec<ClassId>) {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Class(c, args) => {
                out.push(c);
                for a in self.types.items(args).to_vec() {
                    self.classes_of_type(a, out);
                }
            }
            Type::Union(a, b) | Type::Inter(a, b) => {
                self.classes_of_type(a, out);
                self.classes_of_type(b, out);
            }
            _ => {}
        }
    }
}

impl<'t, 'a> Spaces<'t, 'a> {
    fn check(&mut self, scrut: TypeId, cases: &[TCase], clauses: &[CaseClause], partial: bool, span: Span) {
        if self.collect_roots(scrut) {
            let all_plain = cases.iter().all(|c| {
                let start = self.sc.classes.len();
                self.sc.array_test = None;
                let shape = self.plain(c.pat);
                let classes = start..self.sc.classes.len();
                let array = self.sc.array_test.take();
                self.sc.plain.push(PlainCase { shape: shape.unwrap_or(Plain::Total), classes, array });
                shape.is_some()
            });
            if all_plain {
                self.check_plain(scrut, cases, clauses, partial, span);
                return;
            }
        }
        // What the cases so far leave of the scrutinee: a case that meets none of it is
        // unreachable, and what is left at the end is missing. scalac's `minus` keeps a
        // constructor pattern or an extractor whole against an extractor of another `unapply`
        // (Space.scala, `Prod` against `Prod`), so a constructor pattern meets what the cases
        // other than irrefutable extractors leave (`remaining_shapes`: `case Apply(f, as)` after
        // `case Arity(n)` over its input), and such an extractor what the type tests, the
        // wildcards and the extractors of its own `unapply` leave; a type test or a wildcard
        // meets what every case leaves (`case _: Apply` after it).
        let mut remaining = Space::Typ(scrut);
        let mut remaining_shapes = Space::Typ(scrut);
        // The kinds of the earlier unguarded cases, an alternative's per arm, and of the cases
        // whole (a repetition of a case is judged on those).
        let mut kinds: Vec<(CaseKind, Space)> = Vec::new();
        let mut wholes: Vec<(CaseKind, Space)> = Vec::new();
        for (i, case) in cases.iter().enumerate() {
            let space = self.project(case.pat, scrut);
            let arms = self.case_arms(case.pat, space, scrut, i);
            let kind = match arms.as_slice() {
                [(k, _)] => *k,
                _ if arms.iter().all(|&(k, _)| k == CaseKind::Shape) => CaseKind::Shape,
                _ => CaseKind::Typ,
            };
            let extractors = kinds.iter().any(|(k, _)| matches!(k, CaseKind::Extractor(..)));
            // An alternative reaches where one of its arms does, each judged by its kind.
            let mut reachable = false;
            for &(arm_kind, arm) in &arms {
                let left = match arm_kind {
                    CaseKind::Shape if extractors => remaining_shapes,
                    CaseKind::Extractor(..) if kinds.iter().any(|&(k, _)| !arm_kind.judged_by(k)) => {
                        let mut left = Space::Typ(scrut);
                        for (k, taken) in kinds.clone() {
                            if arm_kind.judged_by(k) {
                                if let Some(l) = self.minus(left, taken) {
                                    left = self.simplify(l);
                                }
                            }
                        }
                        left
                    }
                    _ => remaining,
                };
                if self.meets(arm, left) {
                    reachable = true;
                    break;
                }
            }
            let reachable = reachable || (self.catch_all_on_reference(case, scrut) && self.covered_whole(scrut, 16) != Some(true));
            let taken = if case.guard.is_none() { self.minus(remaining, space) } else { None };
            // A case that takes nothing away (a literal, a guard, a type the algebra cannot
            // split) repeats an earlier one only if their spaces are the same; a case that took
            // something away leaves nothing for a repetition to meet.
            let duplicate = reachable && taken.is_none() && !self.has_opaque(space) && wholes.iter().any(|&(k, c)| kind.judged_by(k) && self.same(c, space));
            if (!reachable || duplicate) && self.budget > 0 {
                self.unreachable(clauses, i);
            }
            if case.guard.is_none() {
                if let Some(left) = taken {
                    remaining = self.simplify(left);
                }
                let has_extractor = arms.iter().any(|(k, _)| matches!(k, CaseKind::Extractor(..)));
                if !extractors && !has_extractor {
                    remaining_shapes = remaining;
                } else {
                    // A constructor pattern is not taken by an extractor's arm.
                    for &(arm_kind, arm) in &arms {
                        if !matches!(arm_kind, CaseKind::Extractor(..)) {
                            if let Some(left) = self.minus(remaining_shapes, arm) {
                                remaining_shapes = self.simplify(left);
                            }
                        }
                    }
                }
                kinds.extend(arms);
                wholes.push((kind, space));
                self.sc.covered.push(space);
            }
        }
        if partial || self.inexact || self.budget == 0 || remaining == Space::Empty || !self.is_checkable(scrut, 0) {
            return;
        }
        // A type pattern over an abstract type covers its bound's children (`decompose_abstract`):
        // what the cases leave is taken again with that, as dotc's `checkExhaustivity` subtracts
        // every case at once.
        let abstract_tests = self.sc.covered.iter().any(|&c| matches!(c, Space::Typ(t) if self.abstract_type(t)));
        let remaining = if abstract_tests {
            self.decompose_abstract = true;
            let mut left = Space::Typ(scrut);
            for c in self.sc.covered.clone() {
                if let Some(taken) = self.minus(left, c) {
                    left = self.simplify(taken);
                }
            }
            self.decompose_abstract = false;
            left
        } else {
            remaining
        };
        if remaining == Space::Empty {
            return;
        }
        let shown = self.show_missing(remaining);
        self.t.warn(span, format!("match may not be exhaustive; missing: {}", shown));
    }

    // ---- the arena ----

    /// Moves the parts built on the stack since `base` into the arena.
    fn seal(&mut self, base: usize) -> Items {
        let start = self.sc.arena.len();
        self.sc.arena.extend_from_slice(&self.sc.stack[base..]);
        self.sc.stack.truncate(base);
        Items { start: start as u32, len: (self.sc.arena.len() - start) as u32 }
    }

    fn part(&self, items: Items, i: usize) -> Space {
        self.sc.arena[items.start as usize + i]
    }

    /// Structural equality: two runs are the same when their parts are.
    fn same(&self, a: Space, b: Space) -> bool {
        match (a, b) {
            (Space::Prod(c1, t1, f1), Space::Prod(c2, t2, f2)) => c1 == c2 && t1 == t2 && self.same_items(f1, f2),
            (Space::Or(xs), Space::Or(ys)) => self.same_items(xs, ys),
            _ => a == b,
        }
    }

    fn same_items(&self, xs: Items, ys: Items) -> bool {
        xs.len == ys.len && (0..xs.len as usize).all(|i| self.same(self.part(xs, i), self.part(ys, i)))
    }

    fn covered_before(&self, space: Space) -> bool {
        self.sc.covered.iter().any(|&c| self.same(c, space))
    }

    /// Whether an earlier unguarded test against an array takes every array the test against
    /// `t` takes, on a target whose test looks at the elements: the types say it, as scalac
    /// has it (`Array[Int]` after `Array[? <: Int]`, any array after `Array[?]`).
    fn array_taken(&mut self, t: TypeId) -> bool {
        (0..self.sc.arrays.len()).any(|i| {
            let earlier = self.sc.arrays[i];
            self.sub(t, earlier)
        })
    }

    fn covered_by_base(&self, c: ClassId) -> bool {
        self.t.syms.class(c).base_types.iter().skip(1).any(|(b, _)| self.sc.covered_classes.contains(b))
    }

    // ---- plain cases ----

    /// Fills `roots` and `scrut_class`; false for a union with a member that is no class,
    /// which only the algebra follows.
    fn collect_roots(&mut self, scrut: TypeId) -> bool {
        let fixed = |s: &Self, t: TypeId, args: TList| (args != EMPTY_LIST && s.is_ground(t)).then_some(t);
        match self.t.types.get(scrut) {
            Type::Class(c, args) => {
                self.sc.scrut_class = Some(c);
                let info = self.t.syms.class(c);
                let sealed = splits_into_children(&info);
                if sealed && !info.children.is_empty() {
                    let fixed = fixed(self, scrut, args);
                    self.sc.roots.push((c, fixed));
                }
                true
            }
            Type::Union(..) => {
                let mut alts = std::mem::take(&mut self.sc.types);
                alts.clear();
                self.t.union_alternatives(scrut, &mut alts);
                let mut classes = true;
                for &a in &alts {
                    match self.t.types.get(a) {
                        Type::Class(c, args) => {
                            let fixed = fixed(self, a, args);
                            self.sc.roots.push((c, fixed));
                        }
                        _ => classes = false,
                    }
                }
                self.sc.types = alts;
                classes
            }
            _ => true,
        }
    }

    /// The shape of a pattern of constructors, values, type tests and literals, with its classes
    /// pushed to `classes`; None for a nested pattern or another shape the classes alone do not
    /// describe.
    fn plain(&mut self, p: TPatId) -> Option<Plain> {
        let wild = |t: &Worker, p: TPatId| matches!(t.prog.pats[p.idx()], TPat::Wildcard | TPat::Bind(_, None));
        match self.t.prog.pats[p.idx()] {
            TPat::Wildcard | TPat::Bind(_, None) => Some(Plain::Total),
            TPat::Bind(_, Some(inner)) => self.plain(inner),
            TPat::Test(_, t, inner) if wild(self.t, inner) => match self.t.types.get(t) {
                Type::Any => Some(Plain::Total),
                Type::Class(c, _) if c == self.t.b.array && self.t.jvm => {
                    self.sc.classes.push(c);
                    self.sc.array_test = Some(t);
                    Some(Plain::Covers)
                }
                Type::Class(c, _) if c != self.t.b.boolean => {
                    self.sc.classes.push(c);
                    Some(Plain::Covers)
                }
                _ => None,
            },
            TPat::Test(..) | TPat::Seq(..) | TPat::Unapply(..) => None,
            TPat::Equals(e, _) => match self.t.prog.expr(e) {
                TExpr::Module(c) => {
                    self.sc.classes.push(c);
                    Some(Plain::Covers)
                }
                TExpr::Local(_) if self.t.local_module_class(e).is_some() => {
                    let c = self.t.local_module_class(e).unwrap();
                    self.sc.classes.push(c);
                    Some(Plain::Covers)
                }
                // An object nested in a class is one per enclosing instance: `R` inside the
                // class (`this.R`) covers the class, `a.R` takes that path's value alone.
                TExpr::Field(r, s) if self.t.inner_object_of_sym(s).is_some() => {
                    if !matches!(self.t.prog.expr(r), TExpr::This) {
                        return Some(Plain::Literal);
                    }
                    let c = self.t.inner_object_of_sym(s).unwrap();
                    self.sc.classes.push(c);
                    Some(Plain::Covers)
                }
                TExpr::Static(s) => match self.t.syms.sym(s).kind {
                    SymKind::EnumValue(c) => {
                        self.sc.classes.push(c);
                        Some(Plain::Covers)
                    }
                    _ => Some(Plain::Literal),
                },
                TExpr::Bool(_) | TExpr::Unit => None,
                TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Char(_) | TExpr::Str(_) => {
                    let Type::Class(c, _) = self.t.types.get(self.lit_class(e)) else { return None };
                    self.sc.classes.push(c);
                    Some(Plain::Literal)
                }
                _ => Some(Plain::Literal),
            },
            TPat::Class(c, _, _, subs) => {
                let subs = subs.range();
                if subs.clone().all(|i| wild(self.t, self.t.prog.pat_lists[i])) {
                    self.sc.classes.push(c);
                    return Some(Plain::Covers);
                }
                if !subs.clone().any(|i| self.literal_inside(self.t.prog.pat_lists[i])) {
                    return None;
                }
                self.sc.classes.push(c);
                Some(Plain::Literal)
            }
            TPat::Alt(items) => {
                let mut shape = None;
                for i in items.range() {
                    let item = self.t.prog.pat_lists[i];
                    let item_shape = self.plain(item)?;
                    // One array type per case is what the reachability of arrays follows.
                    if self.sc.array_test.take().is_some() {
                        return None;
                    }
                    shape = Some(match (shape, item_shape) {
                        (Some(Plain::Total), _) | (_, Plain::Total) => Plain::Total,
                        (None, s) => s,
                        (Some(s), t) if s == t => s,
                        _ => return None,
                    });
                }
                shape
            }
        }
    }

    /// The class of the literal a pattern compares with.
    fn lit_class(&self, e: TExprId) -> TypeId {
        let b = &self.t.b;
        match self.t.prog.expr(e) {
            TExpr::Int(_) => b.t_int,
            TExpr::Long(_) => b.t_long,
            TExpr::Double(_) => b.t_double,
            TExpr::Char(_) => b.t_char,
            _ => b.t_string,
        }
    }

    /// Whether the pattern's space would be one the algebra cannot take from a type (see
    /// `cannot_cover`): a literal of a type with more than two values, or a value it does not
    /// follow, at any depth. False where that is not certain.
    fn literal_inside(&self, p: TPatId) -> bool {
        match self.t.prog.pats[p.idx()] {
            TPat::Wildcard | TPat::Bind(_, None) | TPat::Test(..) | TPat::Seq(..) | TPat::Unapply(..) => false,
            TPat::Bind(_, Some(inner)) => self.literal_inside(inner),
            TPat::Equals(e, _) => match self.t.prog.expr(e) {
                TExpr::Module(_) | TExpr::Bool(_) | TExpr::Unit => false,
                TExpr::Static(s) => !matches!(self.t.syms.sym(s).kind, SymKind::EnumValue(_)),
                TExpr::Local(_) => self.t.local_module_class(e).is_none(),
                TExpr::Field(_, s) => self.t.inner_object_of_sym(s).is_none(),
                _ => true,
            },
            TPat::Class(_, _, _, subs) => subs.range().any(|i| self.literal_inside(self.t.prog.pat_lists[i])),
            TPat::Alt(items) => items.range().all(|i| self.literal_inside(self.t.prog.pat_lists[i])),
        }
    }

    fn class_covered(&self, c: ClassId) -> bool {
        let covered = &self.sc.covered_classes;
        covered.contains(&c)
            || self.t.syms.class(c).base_types.iter().skip(1).any(|(b, _)| covered.contains(b))
            // `Int` and `Double` share a run-time type, so a `Double` case takes the `Int`s too.
            || (c == self.t.b.int && covered.contains(&self.t.b.double))
    }

    /// Whether a value of the scrutinee type can be a `c`: `c` is a class of the scrutinee or
    /// under one, or above one, or a `Double` for an `Int` and the other way round.
    fn relates_to_scrutinee(&self, c: ClassId) -> bool {
        if self.sc.roots.is_empty() {
            return self.sc.scrut_class.map_or(true, |s| self.related(c, s));
        }
        self.sc.roots.iter().any(|&(r, _)| self.related(c, r))
    }

    /// Whether `t` is a tuple class past 22 elements and `xxl` `scala.runtime.TupleXXL`, named
    /// rather than looked up, which would enter its std file.
    fn xxl_tuple_of(&self, t: ClassId, xxl: ClassId) -> bool {
        if !self.t.is_tuple_class(t) || self.t.syms.class(t).tparams.len() <= 22 {
            return false;
        }
        let info = self.t.syms.class(xxl);
        let Owner::Package(p) = info.owner else { return false };
        self.t.interner.get(info.name) == "TupleXXL" && self.t.interner.get(self.t.syms.pkg(p).name) == "runtime" && self.t.syms.pkg(p).parent == Some(self.t.b.scala_pkg)
    }

    fn related(&self, c: ClassId, r: ClassId) -> bool {
        let b = &self.t.b;
        // The literals of a `Float` scrutinee are `Double`s.
        if c == r || (c == b.int && r == b.double) || (c == b.double && r == b.int) || (c == b.double && r == b.float) {
            return true;
        }
        let value_types = [b.int, b.long, b.double, b.byte, b.short, b.float, b.boolean, b.char, b.unit];
        // `AnyRef` is above every class that is no value type, `AnyVal` above the value types.
        if r == b.any_ref {
            return !value_types.contains(&c);
        }
        if r == b.any_val || c == b.any_val {
            let other = if r == b.any_val { c } else { r };
            return value_types.contains(&other) || self.t.syms.class(other).value_class;
        }
        let bases = |k: ClassId| self.t.syms.class(k).info.base_types.iter().skip(1);
        if bases(c).any(|&(x, _)| x == r) || bases(r).any(|&(x, _)| x == c) {
            return true;
        }
        // A tuple class past 22 elements is a type alone, whose values are TupleXXLs.
        if self.xxl_tuple_of(c, r) || self.xxl_tuple_of(r, c) {
            return true;
        }
        // Two traits may be mixed into one class, and a trait into a class below the other.
        let is_trait = |k: ClassId| matches!(self.t.syms.class(k).kind, ClassKind::Trait | ClassKind::Enum);
        match (is_trait(c), is_trait(r)) {
            (true, true) => return true,
            (true, false) if self.extended_with(r, c) => return true,
            (false, true) if self.extended_with(c, r) => return true,
            _ => {}
        }
        let info = self.t.syms.class(r);
        let sealed = splits_into_children(&info);
        sealed && info.children.iter().any(|&child| self.related(c, child))
    }

    /// Whether the covered classes take every value of `c`.
    fn covers_all(&mut self, c: ClassId, fixed: Option<TypeId>) -> bool {
        if self.class_covered(c) {
            return true;
        }
        if c == self.t.b.array {
            return fixed.map_or(false, |sty| self.array_taken(sty));
        }
        let info = self.t.syms.class(c);
        let sealed = splits_into_children(&info);
        if sealed && !info.children.is_empty() {
            return (0..info.children.len()).all(|i| {
                let child = self.t.syms.class(c).children[i];
                self.covers_all(child, fixed)
            });
        }
        fixed.map_or(false, |sty| {
            self.t.settle_class(c);
            let arity = self.t.syms.class(c).tparams.len();
            !self.t.solve_pattern_class(c, arity, sty).1
        })
    }

    fn check_plain(&mut self, scrut: TypeId, cases: &[TCase], clauses: &[CaseClause], partial: bool, span: Span) {
        let mut total = false;
        for (i, case) in cases.iter().enumerate() {
            let shape = self.sc.plain[i].shape;
            let classes = self.sc.plain[i].classes.clone();
            let nothing_left = |s: &mut Self| {
                total
                    || (!s.sc.covered_classes.is_empty()
                        && !s.sc.roots.is_empty()
                        && (0..s.sc.roots.len()).all(|k| {
                            let (root, fixed) = s.sc.roots[k];
                            s.covers_all(root, fixed)
                        }))
            };
            let classes_covered =
                |s: &Self| !classes.is_empty() && classes.clone().all(|k| s.class_covered(s.sc.classes[k]));
            let unrelated =
                |s: &Self| !classes.is_empty() && !classes.clone().any(|k| s.relates_to_scrutinee(s.sc.classes[k]));
            let array = self.sc.plain[i].array;
            let unreachable = match shape {
                Plain::Total => nothing_left(self),
                Plain::Covers => match array {
                    Some(t) => total || unrelated(self) || self.covered_by_base(self.t.b.array) || self.array_taken(t),
                    None => total || classes_covered(self) || unrelated(self),
                },
                Plain::Literal => {
                    nothing_left(self) || classes_covered(self) || unrelated(self) || self.repeats_literal(case, scrut)
                }
            };
            if unreachable {
                self.unreachable(clauses, i);
            }
            if case.guard.is_none() {
                match shape {
                    Plain::Total => total = true,
                    Plain::Covers => {
                        // A test against an array type covers the arrays of that type alone.
                        if array.is_none() {
                            for k in classes {
                                let c = self.sc.classes[k];
                                self.sc.covered_classes.push(c);
                            }
                        }
                        self.sc.arrays.extend(array);
                    }
                    Plain::Literal => {}
                }
            }
        }
        if partial || total || self.sc.roots.is_empty() {
            return;
        }
        let mut missing = Vec::new();
        for k in 0..self.sc.roots.len() {
            let (root, fixed) = self.sc.roots[k];
            self.missing_children(root, fixed, &mut missing);
        }
        if !missing.is_empty() {
            let mut shown: Vec<String> = Vec::new();
            for (c, fixed) in missing {
                self.t.settle_class(c);
                let arity = self.t.syms.class(c).tparams.len();
                let t = match fixed {
                    Some(sty) if arity > 0 => {
                        let (cargs, _) = self.t.solve_pattern_class(c, arity, sty);
                        self.t.types.class(c, &cargs)
                    }
                    _ => {
                        let cargs = vec![ANY; arity];
                        self.t.types.class(c, &cargs)
                    }
                };
                let text = self.show_type_space(t);
                if !shown.contains(&text) {
                    shown.push(text);
                }
            }
            self.t.warn(span, format!("match may not be exhaustive; missing: {}", shown.join(", ")));
        }
    }

    /// Whether an earlier unguarded literal case has the same space, which is the one way a
    /// case that covers nothing makes a later one unreachable. The spaces of the unguarded
    /// literal cases are kept in `covered`.
    fn repeats_literal(&mut self, case: &TCase, scrut: TypeId) -> bool {
        let space = self.project(case.pat, scrut);
        let repeated = !self.has_opaque(space) && self.covered_before(space);
        if case.guard.is_none() {
            self.sc.covered.push(space);
        }
        repeated
    }

    /// The classes under `c` that `covered_classes` leaves out, each with its root's fixed type.
    fn missing_children(&mut self, c: ClassId, fixed: Option<TypeId>, out: &mut Vec<(ClassId, Option<TypeId>)>) {
        if self.class_covered(c) || (c == self.t.b.array && fixed.map_or(false, |sty| self.array_taken(sty))) {
            return;
        }
        let info = self.t.syms.class(c);
        let sealed = splits_into_children(&info);
        if sealed && !info.children.is_empty() {
            for i in 0..info.children.len() {
                let child = self.t.syms.class(c).children[i];
                self.missing_children(child, fixed, out);
            }
        } else if fixed.map_or(true, |sty| {
            self.t.settle_class(c);
            let arity = self.t.syms.class(c).tparams.len();
            self.t.solve_pattern_class(c, arity, sty).1
        }) {
            out.push((c, fixed));
        }
    }

    /// A catch-all case (`_`, `x`, `x @ _`, guarded or not) on a reference scrutinee takes
    /// `null`, which no other case does: dotc reports it unreachable (except for null) only
    /// when the cases before cover the type as `isSubspace` sees it, without splitting a
    /// product by its fields; a guard changes what the case leaves, not what it takes.
    fn catch_all_on_reference(&self, case: &TCase, scrut: TypeId) -> bool {
        self.is_catch_all(case.pat) && !self.t.is_primitive(scrut)
    }

    fn is_catch_all(&self, p: TPatId) -> bool {
        match self.t.prog.pats[p.idx()] {
            TPat::Wildcard | TPat::Bind(_, None) => true,
            TPat::Bind(_, Some(inner)) => self.is_catch_all(inner),
            _ => false,
        }
    }

    /// dotc's `isSubspace` of a type against the cases before: one case's space takes it as a
    /// whole, or each child of a sealed type is taken so. A product is not split by its fields:
    /// `(Some(_), _)` and `(None, _)` cover `(Option[Int], Int)` only by splitting, so a `case
    /// _` after them is left alone, while `Some(_)` and `None` cover `Option[Int]`. Where the
    /// depth runs out the answer is unknown (`None`): neither the null exemption nor the
    /// warning follows from it, and the case is left alone.
    fn covered_whole(&mut self, t: TypeId, depth: u32) -> Option<bool> {
        let covered = self.sc.covered.clone();
        let mut unknown = false;
        for &c in &covered {
            match self.space_takes(c, t, depth) {
                Some(true) => return Some(true),
                Some(false) => {}
                None => unknown = true,
            }
        }
        if depth == 0 {
            return None;
        }
        let Some(parts) = self.decompose(t) else { return if unknown { None } else { Some(false) } };
        let mut all = true;
        for i in 0..parts.len as usize {
            let part = match self.part(parts, i) {
                Space::Typ(u) => self.covered_whole(u, depth - 1),
                part @ Space::Lit(l) => {
                    let lt = self.lit_type(l);
                    Some(covered.iter().any(|&c| self.same(c, part) || matches!(c, Space::Typ(u) if self.sub(lt, u))))
                }
                _ => Some(false),
            };
            match part {
                Some(true) => {}
                Some(false) => all = false,
                None => unknown = true,
            }
        }
        if all && !unknown {
            Some(true)
        } else if unknown {
            None
        } else {
            Some(false)
        }
    }

    /// Whether the space of one case takes every value of `t` as one shape: a type above it,
    /// or a product of its class whose fields each take their type wholly; `None` where the
    /// depth ran out before the answer.
    fn space_takes(&mut self, s: Space, t: TypeId, depth: u32) -> Option<bool> {
        match s {
            Space::Typ(u) => Some(self.sub(t, u)),
            Space::Or(items) => {
                let mut unknown = false;
                for i in 0..items.len as usize {
                    let x = self.part(items, i);
                    match self.space_takes(x, t, depth) {
                        Some(true) => return Some(true),
                        Some(false) => {}
                        None => unknown = true,
                    }
                }
                if unknown { None } else { Some(false) }
            }
            Space::Prod(_, pt, fields) => {
                if !(self.same_class(t, pt) || self.sub(t, pt)) {
                    return Some(false);
                }
                let Some(types) = self.field_types(t) else { return Some(false) };
                if types.len != fields.len {
                    return Some(false);
                }
                let mut unknown = false;
                for i in 0..fields.len as usize {
                    let ft = self.sc.field_arena[types.start as usize + i];
                    let taken = match self.part(fields, i) {
                        Space::Typ(u) => Some(self.sub(ft, u)),
                        inner @ (Space::Prod(..) | Space::Or(_)) => {
                            if depth == 0 { None } else { self.space_takes(inner, ft, depth - 1) }
                        }
                        _ => Some(false),
                    };
                    match taken {
                        Some(true) => {}
                        Some(false) => return Some(false),
                        None => unknown = true,
                    }
                }
                if unknown { None } else { Some(true) }
            }
            _ => Some(false),
        }
    }

    /// What a case's pattern is to the reachability of the cases after it: a constructor
    /// pattern of a case class (a product space, or alternatives of them; a tuple's is a type
    /// to scalac here, covered by `a -> b`), an extractor that cannot fail (by its `unapply` and
    /// the path it is called on, scalac's `isSameUnapply`; through a binder and its input's
    /// test), or a type, a wildcard or anything else. `case_index` tells apart the extractors
    /// called on what is no path.
    /// A case's pattern as arms with their kinds: an alternative's each (`Ex(_) | Ex2(_)`), else
    /// the pattern whole.
    fn case_arms(&mut self, p: TPatId, space: Space, scrut: TypeId, case_index: usize) -> Vec<(CaseKind, Space)> {
        let mut q = p;
        while let TPat::Bind(_, Some(inner)) = self.t.prog.pats[q.idx()] {
            q = inner;
        }
        match self.t.prog.pats[q.idx()] {
            TPat::Alt(items) => items
                .range()
                .map(|k| self.t.prog.pat_lists[k])
                .collect::<Vec<_>>()
                .into_iter()
                .map(|arm| {
                    let arm_space = self.project(arm, scrut);
                    (self.case_kind(arm, arm_space, case_index), arm_space)
                })
                .collect(),
            _ => vec![(self.case_kind(p, space, case_index), space)],
        }
    }

    fn case_kind(&self, p: TPatId, space: Space, case_index: usize) -> CaseKind {
        let mut p = p;
        for _ in 0..8 {
            match self.t.prog.pats[p.idx()] {
                TPat::Bind(_, Some(q)) | TPat::Test(_, _, q) => p = q,
                // A constructor pattern whatever its space (`Leaf()` projects to its type).
                TPat::Class(c, ..) if !self.t.is_tuple_class(c) => return CaseKind::Shape,
                TPat::Unapply(_, call, _) if self.t.irrefutable_pats.contains_key(&p) => {
                    let mut call = call;
                    for _ in 0..8 {
                        match self.t.prog.expr(call) {
                            TExpr::Block(_, r) => call = r,
                            TExpr::CallStatic(m, _) => return CaseKind::Extractor(m, Path::Static),
                            TExpr::CallMethod(r, m, _) => {
                                let path = self.stable_path(r, 8).map_or(Path::Case(case_index as u32), Path::Stable);
                                return CaseKind::Extractor(m, path);
                            }
                            _ => break,
                        }
                    }
                    return CaseKind::Typ;
                }
                _ => break,
            }
        }
        let shape = |s: &Self, sp: Space| matches!(sp, Space::Prod(c, ..) if !s.t.is_tuple_class(c));
        match space {
            Space::Prod(..) if shape(self, space) => CaseKind::Shape,
            Space::Or(items) if (0..items.len as usize).all(|i| shape(self, self.part(items, i))) => CaseKind::Shape,
            _ => CaseKind::Typ,
        }
    }

    /// A digest of the stable path `e` (an object, `this`, a value, a field of a stable path:
    /// `h.ex`), `None` for what is no path.
    fn stable_path(&self, e: TExprId, depth: u32) -> Option<u64> {
        use std::hash::{Hash, Hasher};
        let mut h = crate::intern::FxHasher::default();
        match self.t.prog.expr(e) {
            TExpr::Module(c) => (0u8, c.0).hash(&mut h),
            TExpr::This => 1u8.hash(&mut h),
            TExpr::Static(v) | TExpr::Local(v) => (2u8, v.0).hash(&mut h),
            TExpr::Field(q, v) if depth > 0 => (3u8, self.stable_path(q, depth - 1)?, v.0).hash(&mut h),
            _ => return None,
        }
        Some(h.finish())
    }

    fn unreachable(&mut self, clauses: &[CaseClause], i: usize) {
        let pat_span = self.t.cur_ast().pat_spans[clauses[i].pat.idx()];
        self.t.warn(pat_span, "unreachable case");
    }

    // ---- projection ----

    fn project(&mut self, p: TPatId, sty: TypeId) -> Space {
        match self.t.prog.pats[p.idx()] {
            TPat::Wildcard | TPat::Bind(_, None) => Space::Typ(sty),
            TPat::Bind(_, Some(inner)) => self.project(inner, sty),
            TPat::Test(test, t, inner) => {
                // A test a copy derives again, of an inline body's type parameter, is the
                // expansion's to judge, where the parameter has its argument.
                if self.t.types.contains_error(t) || self.t.prog.deferred_tests.contains_key(&test) {
                    return Space::Opaque;
                }
                // An abstract type is the space of itself, as scalac's `Typ(T)` (Space.scala,
                // `project`), whatever its bound: a wildcard after `case _: T` is reachable.
                let t = match self.t.types.get(t) {
                    Type::Param(_) | Type::AppParam(..) => t,
                    _ => self.t.match_type(t),
                };
                let tested = self.narrow(t, sty);
                let tested = self.simplify(tested);
                match self.t.prog.pats[inner.idx()] {
                    TPat::Seq(items, rest) => self.project_seq(items, rest, tested),
                    TPat::Wildcard | TPat::Bind(_, None) => tested,
                    // A class pattern under the test takes the tested type's fields apart.
                    _ => match tested {
                        Space::Typ(nt) => self.project(inner, nt),
                        other => other,
                    },
                }
            }
            TPat::Equals(e, _) => self.project_value(e),
            TPat::Class(c, t, _, subs) => {
                if self.t.types.contains_error(t) {
                    return Space::Opaque;
                }
                let subs = subs.range();
                let Some(fields) = self.field_types(t).filter(|_| !subs.is_empty()) else {
                    return Space::Typ(t);
                };
                let base = self.sc.stack.len();
                for (i, k) in subs.zip(fields.range()) {
                    let s = self.t.prog.pat_lists[i];
                    let f = self.sc.field_arena[k];
                    let space = self.project(s, f);
                    self.sc.stack.push(space);
                }
                Space::Prod(c, t, self.seal(base))
            }
            TPat::Alt(items) => {
                let base = self.sc.stack.len();
                for i in items.range() {
                    let item = self.t.prog.pat_lists[i];
                    let space = self.project(item, sty);
                    self.sc.stack.push(space);
                }
                Space::Or(self.seal(base))
            }
            TPat::Seq(items, rest) => self.project_seq(items, rest, Space::Typ(sty)),
            TPat::Unapply(_, _, inner) if self.t.irrefutable_pats.contains_key(&p) => self.project_irrefutable(inner, sty),
            TPat::Unapply(..) => Space::Opaque,
        }
    }

    /// An extractor typed as a `Some` (`->`) covers its scrutinee where the pattern matched
    /// against its result covers that result, as scalac's irrefutable unapply.
    fn project_irrefutable(&mut self, inner: TPatId, sty: TypeId) -> Space {
        let TPat::Class(_, result, _, _) = self.t.prog.pats[inner.idx()] else { return Space::Opaque };
        let space = self.project(inner, result);
        match self.minus(Space::Typ(result), space).map(|left| self.simplify(left)) {
            Some(Space::Empty) => Space::Typ(sty),
            _ => Space::Opaque,
        }
    }

    fn project_value(&mut self, e: TExprId) -> Space {
        match self.t.prog.expr(e) {
            TExpr::Module(c) => Space::Typ(self.t.types.class(c, &[])),
            TExpr::Local(_) if self.t.local_module_class(e).is_some() => {
                let c = self.t.local_module_class(e).unwrap();
                Space::Typ(self.t.types.class(c, &[]))
            }
            TExpr::Field(r, s) if self.t.inner_object_of_sym(s).is_some() => {
                if !matches!(self.t.prog.expr(r), TExpr::This) {
                    return Space::Opaque;
                }
                let c = self.t.inner_object_of_sym(s).unwrap();
                Space::Typ(self.t.types.class(c, &[]))
            }
            TExpr::Static(s) => match self.t.syms.sym(s).kind {
                SymKind::EnumValue(c) => Space::Typ(self.t.types.class(c, &[])),
                _ => Space::Opaque,
            },
            TExpr::Int(i) => Space::Lit(Lit::Int(i)),
            TExpr::Long(l) => Space::Lit(Lit::Long(l)),
            TExpr::Double(d) => Space::Lit(Lit::Double(d.to_bits())),
            TExpr::Bool(b) => Space::Lit(Lit::Bool(b)),
            TExpr::Char(c) => Space::Lit(Lit::Char(c)),
            TExpr::Str(s) => Space::Lit(Lit::Str(s)),
            TExpr::Unit => Space::Typ(self.t.b.t_unit),
            _ => Space::Opaque,
        }
    }

    /// `Seq(rest*)` covers its class, `Seq(x, rest*)` the cases with elements; any other shape
    /// depends on lengths, which the algebra leaves alone.
    fn project_seq(&mut self, items: crate::ast::ListRef, rest: Option<TPatId>, class_space: Space) -> Space {
        let items = items.range();
        let plain = |t: &Worker, p: TPatId| matches!(t.prog.pats[p.idx()], TPat::Wildcard | TPat::Bind(_, None));
        let covers = rest.map_or(false, |r| plain(self.t, r))
            && items.clone().all(|i| plain(self.t, self.t.prog.pat_lists[i]));
        let non_empty_cases = match (class_space, items.len()) {
            (_, 0) if covers => return class_space,
            (Space::Typ(t), 1) if covers => match self.decompose(t) {
                Some(parts) => {
                    let base = self.sc.stack.len();
                    for i in 0..parts.len as usize {
                        let p = self.part(parts, i);
                        if !matches!(p, Space::Typ(t) if self.is_singleton_type(t)) {
                            self.sc.stack.push(p);
                        }
                    }
                    Some(self.seal(base))
                }
                None => None,
            },
            _ => None,
        };
        match non_empty_cases {
            Some(parts) => Space::Or(parts),
            None => {
                self.inexact = true;
                Space::Opaque
            }
        }
    }

    // ---- the algebra ----

    fn sub(&mut self, a: TypeId, b: TypeId) -> bool {
        if a == b || b == ANY || a == NOTHING {
            return true;
        }
        // `Int` and `Double` share a run-time type, so a `Double` case takes the `Int`s too;
        // the literals of a `Float` scrutinee are `Double`s.
        if (a == self.t.b.t_int && b == self.t.b.t_double) || (a == self.t.b.t_double && b == self.t.b.t_float) {
            return true;
        }
        // Two classes are related through the base types alone; the typer settles their
        // arguments, and what stands below `AnyVal` and `AnyRef`.
        let tops = [self.t.b.t_any_val, self.t.b.t_any_ref];
        match (self.t.types.get(a), self.t.types.get(b)) {
            (Type::Class(c1, xs), Type::Class(c2, ys)) if self.plain_class(c1) && self.plain_class(c2) && !tops.contains(&b) => {
                self.t.complete_class(c1);
                if !self.t.syms.class(c1).base_types.iter().any(|&(base, _)| base == c2) {
                    return false;
                }
                if xs == EMPTY_LIST && ys == EMPTY_LIST {
                    return true;
                }
            }
            (Type::Union(x, y), _) => return self.sub(x, b) && self.sub(y, b),
            (Type::Class(..), Type::Union(x, y)) => return self.sub(a, x) || self.sub(a, y),
            _ => {}
        }
        if let Some(&r) = self.sc.subs.get(&(a, b)) {
            return r;
        }
        let mark = self.t.snapshot();
        let r = self.t.is_sub(a, b);
        self.t.rollback(mark);
        self.sc.subs.insert((a, b), r);
        r
    }

    /// A class whose values are exactly its instances: not an opaque type, which the typer
    /// sees through.
    fn plain_class(&self, c: ClassId) -> bool {
        self.t.syms.class(c).kind != ClassKind::Opaque
    }

    fn lit_type(&self, l: Lit) -> TypeId {
        let b = &self.t.b;
        match l {
            Lit::Int(_) => b.t_int,
            Lit::Long(_) => b.t_long,
            Lit::Double(_) => b.t_double,
            Lit::Bool(_) => b.t_boolean,
            Lit::Char(_) => b.t_char,
            Lit::Str(_) => b.t_string,
        }
    }

    /// The run time tells classes apart, not their type arguments: `Box[Int]` and `Box[String]`
    /// are one space.
    fn same_class(&self, t1: TypeId, t2: TypeId) -> bool {
        match (self.t.types.get(t1), self.t.types.get(t2)) {
            (Type::Class(c1, _), Type::Class(c2, _)) => c1 == c2,
            _ => false,
        }
    }

    /// Two classes no value belongs to both of: classes of which neither extends the other, and
    /// a class and a trait that neither it nor a class below it extends.
    fn disjoint(&self, t1: TypeId, t2: TypeId) -> bool {
        let (Type::Class(c1, _), Type::Class(c2, _)) = (self.t.types.get(t1), self.t.types.get(t2)) else {
            return false;
        };
        let is_trait = |c: ClassId| matches!(self.t.syms.class(c).kind, ClassKind::Trait | ClassKind::Enum);
        match (is_trait(c1), is_trait(c2)) {
            (true, true) => false,
            (false, false) => c1 != c2 && !self.derives(c1, c2) && !self.derives(c2, c1) && !self.xxl_tuple_of(c1, c2) && !self.xxl_tuple_of(c2, c1),
            (false, true) => !self.extended_with(c1, c2),
            (true, false) => !self.extended_with(c2, c1),
        }
    }

    fn derives(&self, sub: ClassId, sup: ClassId) -> bool {
        self.t.syms.class(sub).base_types.iter().any(|&(b, _)| b == sup)
    }

    /// Whether `class` or a class below it extends `tr`.
    fn extended_with(&self, class: ClassId, tr: ClassId) -> bool {
        self.derives(class, tr) || self.t.syms.class(class).subclasses.iter().any(|&d| self.extended_with(d, tr))
    }

    fn is_singleton_type(&self, t: TypeId) -> bool {
        let Type::Class(c, _) = self.t.types.get(t) else { return false };
        let info = self.t.syms.class(c);
        info.kind == ClassKind::Object || (info.kind == ClassKind::EnumCase && info.singleton.is_some())
    }

    /// The field types of a case class or tuple instance, as a run of `field_arena`, remembered
    /// per match.
    fn field_types(&mut self, t: TypeId) -> Option<Items> {
        if let Some(&(_, f)) = self.sc.fields.iter().find(|(k, _)| *k == t) {
            return f;
        }
        let fields = self.t.class_field_types(t).map(|tys| {
            let start = self.sc.field_arena.len();
            self.sc.field_arena.extend_from_slice(&tys);
            Items { start: start as u32, len: tys.len() as u32 }
        });
        self.sc.fields.push((t, fields));
        fields
    }

    /// The parts a type splits into: the cases of a sealed type or enum, `true` and `false`,
    /// the members of a union. None for a type without parts.
    fn decompose(&mut self, t: TypeId) -> Option<Items> {
        let t = self.t.deref(t);
        if let Some(&(_, parts)) = self.sc.decomposed.iter().find(|(k, _)| *k == t) {
            return parts;
        }
        let parts = self.decompose_fresh(t);
        self.sc.decomposed.push((t, parts));
        parts
    }

    /// A type parameter or an abstract type member, whose space is itself (`Typ(T)`) and whose
    /// parts are its upper bound's, as dotc's `classSymbol` of such a type is its bound's class
    /// (Space.scala 664, `decompose`, `isDecomposableToChildren`).
    fn abstract_type(&self, t: TypeId) -> bool {
        matches!(self.t.types.get(t), Type::Param(_) | Type::AppParam(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_))
    }

    fn decompose_fresh(&mut self, t: TypeId) -> Option<Items> {
        let base = self.sc.stack.len();
        match self.t.types.get(t) {
            Type::Param(p) => {
                let hi = self.t.syms.tparam(p).upper;
                let hi = self.t.deref_alias(hi);
                if hi == t || !matches!(self.t.types.get(hi), Type::Class(..)) {
                    return None;
                }
                self.decompose(hi)
            }
            // `F[Int]` under `F[X] <: A`: the applied bound's parts.
            Type::AppParam(p, args) => {
                let hi = self.t.app_param_upper(p, args)?;
                let hi = self.t.deref_alias(hi);
                if hi == t || !matches!(self.t.types.get(hi), Type::Class(..)) {
                    return None;
                }
                self.decompose(hi)
            }
            Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => {
                let (_, hi) = self.t.member_bounds(t);
                let hi = self.t.deref_alias(hi);
                if hi == t || !matches!(self.t.types.get(hi), Type::Class(..)) {
                    return None;
                }
                self.decompose(hi)
            }
            Type::Nothing => Some(self.seal(base)),
            Type::Union(..) => {
                let mut alts = std::mem::take(&mut self.sc.types);
                alts.clear();
                self.t.union_alternatives(t, &mut alts);
                self.sc.stack.extend(alts.iter().map(|&a| Space::Typ(a)));
                self.sc.types = alts;
                Some(self.seal(base))
            }
            Type::Class(c, args) => {
                if c == self.t.b.boolean {
                    self.sc.stack.push(Space::Lit(Lit::Bool(true)));
                    self.sc.stack.push(Space::Lit(Lit::Bool(false)));
                    return Some(self.seal(base));
                }
                self.t.complete_class(c);
                let info = self.t.syms.class(c);
                if !splits_into_children(&info) || info.children.is_empty() {
                    return None;
                }
                if self.budget == 0 {
                    return None;
                }
                self.budget -= 1;
                // A case that fixes other type arguments than the scrutinee's cannot occur.
                let ground = args != EMPTY_LIST && self.is_ground(t);
                for i in 0..self.t.syms.class(c).children.len() {
                    let child = self.t.syms.class(c).children[i];
                    self.t.settle_class(child);
                    let arity = self.t.syms.class(child).tparams.len();
                    let child_ty = if arity == 0 {
                        let ct = self.t.types.mk(Type::Class(child, EMPTY_LIST));
                        if ground && !self.sub(ct, t) {
                            continue;
                        }
                        ct
                    } else {
                        let (cargs, conforms) = self.t.solve_pattern_class(child, arity, t);
                        if ground && !conforms {
                            continue;
                        }
                        self.t.types.class(child, &cargs)
                    };
                    self.sc.stack.push(Space::Typ(child_ty));
                }
                Some(self.seal(base))
            }
            _ => None,
        }
    }

    fn is_ground(&self, t: TypeId) -> bool {
        match self.t.types.get(t) {
            Type::Any | Type::Nothing | Type::Ctor(_) => true,
            Type::Class(_, args) => self.t.types.items(args).iter().all(|&a| self.is_ground(a)),
            Type::Union(a, b) | Type::Inter(a, b) => self.is_ground(a) && self.is_ground(b),
            _ => false,
        }
    }

    /// `Typ(t)` as the product of its fields, when `t` is a case class or tuple.
    fn as_prod(&mut self, t: TypeId) -> Option<Space> {
        let Type::Class(c, _) = self.t.types.get(t) else { return None };
        let fields = self.field_types(t)?;
        let base = self.sc.stack.len();
        for k in fields.range() {
            let f = self.sc.field_arena[k];
            self.sc.stack.push(Space::Typ(f));
        }
        Some(Space::Prod(c, t, self.seal(base)))
    }

    /// A space that covers no whole type, because a literal (of a type with more than two
    /// values) or an unanalysed pattern sits in it: taking it from a type changes nothing the
    /// algebra can express.
    fn cannot_cover(&self, s: Space) -> bool {
        match s {
            Space::Empty | Space::Opaque => true,
            Space::Lit(l) => !matches!(l, Lit::Bool(_)),
            Space::Prod(_, _, fields) => (0..fields.len as usize).any(|i| self.cannot_cover(self.part(fields, i))),
            Space::Or(items) => (0..items.len as usize).all(|i| self.cannot_cover(self.part(items, i))),
            Space::Typ(_) => false,
        }
    }

    fn simplify(&mut self, s: Space) -> Space {
        match s {
            Space::Prod(c, t, fields) => {
                let base = self.sc.stack.len();
                let mut changed = false;
                for i in 0..fields.len as usize {
                    let field = self.part(fields, i);
                    let simple = self.simplify(field);
                    changed |= simple != field;
                    self.sc.stack.push(simple);
                }
                if self.sc.stack[base..].contains(&Space::Empty) {
                    self.sc.stack.truncate(base);
                    Space::Empty
                } else if changed {
                    Space::Prod(c, t, self.seal(base))
                } else {
                    self.sc.stack.truncate(base);
                    s
                }
            }
            Space::Or(items) => {
                let base = self.sc.stack.len();
                let mut changed = false;
                for i in 0..items.len as usize {
                    let item = self.part(items, i);
                    match self.simplify(item) {
                        Space::Empty => changed = true,
                        Space::Or(inner) => {
                            changed = true;
                            for j in inner.range() {
                                let part = self.sc.arena[j];
                                self.sc.stack.push(part);
                            }
                        }
                        other => {
                            changed |= other != item;
                            self.sc.stack.push(other);
                        }
                    }
                }
                match self.sc.stack.len() - base {
                    0 => Space::Empty,
                    1 => self.sc.stack.pop().unwrap(),
                    _ if changed => Space::Or(self.seal(base)),
                    _ => {
                        self.sc.stack.truncate(base);
                        s
                    }
                }
            }
            Space::Typ(t) if self.t.deref(t) == NOTHING => Space::Empty,
            other => other,
        }
    }

    /// What `a` keeps once every space of `bs` is taken away; None when none of them touches it.
    fn minus_all(&mut self, a: Space, bs: Items) -> Option<Space> {
        let mut acc: Option<Space> = None;
        for i in 0..bs.len as usize {
            let current = acc.unwrap_or(a);
            if current == Space::Empty {
                break;
            }
            let b = self.part(bs, i);
            if let Some(left) = self.minus(current, b) {
                acc = Some(self.simplify(left));
            }
        }
        acc
    }

    /// The values of type `sty` that a test for `t` accepts.
    fn narrow(&mut self, t: TypeId, sty: TypeId) -> Space {
        if self.sub(t, sty) || self.same_class(t, sty) {
            return Space::Typ(t);
        }
        if self.sub(sty, t) {
            return Space::Typ(sty);
        }
        if let Some(parts) = self.decompose(sty) {
            return self.narrow_parts(parts, t, true);
        }
        if let Some(parts) = self.decompose(t) {
            return self.narrow_parts(parts, sty, false);
        }
        if self.disjoint(t, sty) {
            Space::Empty
        } else {
            Space::Typ(t)
        }
    }

    fn narrow_parts(&mut self, parts: Items, other: TypeId, tested: bool) -> Space {
        let base = self.sc.stack.len();
        for i in 0..parts.len as usize {
            let part = self.part(parts, i);
            let narrowed = self.narrow_part(part, other, tested);
            self.sc.stack.push(narrowed);
        }
        Space::Or(self.seal(base))
    }

    /// A part of a decomposition (a type or a `Boolean` literal) against the other side of the
    /// test; `tested` says whether the part comes from the scrutinee.
    fn narrow_part(&mut self, part: Space, other: TypeId, tested: bool) -> Space {
        match part {
            Space::Typ(pt) => {
                if tested {
                    self.narrow(other, pt)
                } else {
                    self.narrow(pt, other)
                }
            }
            Space::Lit(l) => {
                let lt = self.lit_type(l);
                if self.sub(lt, other) {
                    part
                } else {
                    Space::Empty
                }
            }
            _ => part,
        }
    }

    fn any_part(&mut self, items: Items, mut f: impl FnMut(&mut Self, Space) -> bool) -> bool {
        (0..items.len as usize).any(|i| {
            let part = self.part(items, i);
            f(self, part)
        })
    }

    /// Whether the two spaces share a value.
    fn meets(&mut self, a: Space, b: Space) -> bool {
        match (a, b) {
            (Space::Empty, _) | (_, Space::Empty) => false,
            (Space::Opaque, _) | (_, Space::Opaque) => true,
            (Space::Or(xs), _) => self.any_part(xs, |s, x| s.meets(x, b)),
            (_, Space::Or(ys)) => self.any_part(ys, |s, y| s.meets(a, y)),
            (Space::Typ(t1), Space::Typ(t2)) => {
                self.sub(t1, t2)
                    || self.sub(t2, t1)
                    || self.same_class(t1, t2)
                    || match self.decompose(t1) {
                        Some(parts) => self.any_part(parts, |s, p| s.meets(p, b)),
                        None => match self.decompose(t2) {
                            Some(parts) => self.any_part(parts, |s, p| s.meets(a, p)),
                            None => !self.disjoint(t1, t2),
                        },
                    }
            }
            (Space::Typ(t), Space::Prod(_, pt, _)) => {
                if self.sub(pt, t) {
                    return true;
                }
                if self.same_class(t, pt) || self.sub(t, pt) {
                    if let Some(p) = self.as_prod(t) {
                        return self.meets(p, b);
                    }
                }
                match self.decompose(t) {
                    Some(parts) => self.any_part(parts, |s, p| s.meets(p, b)),
                    None => !self.disjoint(t, pt),
                }
            }
            (Space::Prod(..), Space::Typ(_)) => self.meets(b, a),
            (Space::Lit(x), Space::Lit(y)) => x == y,
            (Space::Lit(l), Space::Typ(t)) => {
                let lt = self.lit_type(l);
                self.sub(lt, t)
                    || match self.decompose(t) {
                        Some(parts) => self.any_part(parts, |s, p| s.meets(a, p)),
                        None => false,
                    }
            }
            (Space::Typ(_), Space::Lit(_)) => self.meets(b, a),
            (Space::Lit(_), Space::Prod(..)) | (Space::Prod(..), Space::Lit(_)) => false,
            (Space::Prod(c1, _, f1), Space::Prod(c2, _, f2)) => {
                c1 == c2
                    && f1.len == f2.len
                    && (0..f1.len as usize).all(|i| {
                        let (x, y) = (self.part(f1, i), self.part(f2, i));
                        self.meets(x, y)
                    })
            }
        }
    }

    /// What `a` keeps once `b` is taken away; None when `b` touches none of it.
    fn minus(&mut self, a: Space, b: Space) -> Option<Space> {
        match (a, b) {
            (Space::Empty, _) | (Space::Opaque, _) | (_, Space::Empty) | (_, Space::Opaque) => None,
            (Space::Typ(_), _) if self.cannot_cover(b) => None,
            (Space::Or(xs), _) => self.minus_each(xs, b),
            (_, Space::Or(ys)) => self.minus_all(a, ys),
            (Space::Typ(t1), Space::Typ(t2)) => {
                if self.sub(t1, t2) || self.same_class(t1, t2) {
                    Some(Space::Empty)
                } else if let Some(left) = self.minus_by_parts(t1, b) {
                    Some(left)
                } else if self.decompose_abstract && self.abstract_type(t2) {
                    // dotc's `tryDecompose2` (Space.scala 269, `minus`): a type pattern over an
                    // abstract type takes what its bound's children are (`decompose`).
                    let parts = self.decompose(t2)?;
                    self.minus_all(a, parts)
                } else {
                    None
                }
            }
            (Space::Typ(t), Space::Prod(_, pt, _)) => {
                let within = self.same_class(t, pt) || self.sub(t, pt);
                match within.then(|| self.as_prod(t)).flatten() {
                    Some(p) => self.minus(p, b),
                    None => self.minus_by_parts(t, b),
                }
            }
            (Space::Typ(t), Space::Lit(_)) => self.minus_by_parts(t, b),
            (Space::Lit(x), Space::Lit(y)) => (x == y).then_some(Space::Empty),
            (Space::Lit(l), Space::Typ(t)) => {
                let lt = self.lit_type(l);
                self.sub(lt, t).then_some(Space::Empty)
            }
            (Space::Lit(_), Space::Prod(..)) | (Space::Prod(..), Space::Lit(_)) => None,
            (Space::Prod(_, pt, _), Space::Typ(t)) => {
                if self.sub(pt, t) || self.same_class(pt, t) {
                    Some(Space::Empty)
                } else {
                    let parts = self.decompose(t)?;
                    self.minus_all(a, parts)
                }
            }
            (Space::Prod(c1, t1, f1), Space::Prod(c2, _, f2)) => {
                if c1 != c2 || f1.len != f2.len {
                    return None;
                }
                let base = self.sc.stack.len();
                for i in 0..f1.len as usize {
                    let (x, y) = (self.part(f1, i), self.part(f2, i));
                    // A field b leaves whole keeps the whole product.
                    let Some(left) = self.minus(x, y) else {
                        self.sc.stack.truncate(base);
                        return None;
                    };
                    let left = self.simplify(left);
                    self.sc.stack.push(left);
                }
                if self.sc.stack[base..].iter().all(|s| *s == Space::Empty) {
                    self.sc.stack.truncate(base);
                    return Some(Space::Empty);
                }
                let subs = self.seal(base);
                // `(_, _) - (true, _)` is `(false, _)`; `(_, _) - (true, false)` is
                // `(false, _) | (_, true)`.
                let mut alternatives = Vec::new();
                for i in 0..subs.len as usize {
                    alternatives.clear();
                    self.flatten_into(self.part(subs, i), &mut alternatives);
                    for &alt in &alternatives {
                        let fields = self.sc.stack.len();
                        for j in 0..f1.len as usize {
                            let field = if j == i { alt } else { self.part(f1, j) };
                            self.sc.stack.push(field);
                        }
                        let prod = Space::Prod(c1, t1, self.seal(fields));
                        self.sc.stack.push(prod);
                    }
                }
                Some(Space::Or(self.seal(base)))
            }
        }
    }

    /// Takes `b` from the cases of `t`; None when `t` has no cases or `b` touches none of them.
    fn minus_by_parts(&mut self, t: TypeId, b: Space) -> Option<Space> {
        let parts = self.decompose(t)?;
        self.minus_each(parts, b)
    }

    /// Takes `b` from every space; None when it touches none of them.
    fn minus_each(&mut self, xs: Items, b: Space) -> Option<Space> {
        let base = self.sc.stack.len();
        let mut changed = false;
        for i in 0..xs.len as usize {
            let x = self.part(xs, i);
            let left = self.minus(x, b);
            changed |= left.is_some();
            self.sc.stack.push(left.unwrap_or(x));
        }
        if changed {
            Some(Space::Or(self.seal(base)))
        } else {
            self.sc.stack.truncate(base);
            None
        }
    }

    fn has_opaque(&self, s: Space) -> bool {
        match s {
            Space::Opaque => true,
            Space::Prod(_, _, items) | Space::Or(items) => {
                (0..items.len as usize).any(|i| self.has_opaque(self.part(items, i)))
            }
            _ => false,
        }
    }

    fn flatten_into(&self, s: Space, out: &mut Vec<Space>) {
        match s {
            Space::Or(items) => {
                for i in 0..items.len as usize {
                    self.flatten_into(self.part(items, i), out);
                }
            }
            Space::Empty => {}
            other => out.push(other),
        }
    }

    /// Whether a match over the type is checked for missing cases: a sealed type or enum, a
    /// `Boolean`, a union, or a case class or tuple with such a field.
    fn is_checkable(&mut self, t: TypeId, depth: u32) -> bool {
        let t = self.t.deref(t);
        match self.t.types.get(t) {
            Type::Union(..) => true,
            Type::Class(c, _) => {
                if c == self.t.b.boolean {
                    return true;
                }
                self.t.complete_class(c);
                let info = self.t.syms.class(c);
                let sealed = splits_into_children(&info);
                if sealed && !info.children.is_empty() {
                    return true;
                }
                if depth >= 2 {
                    return false;
                }
                match self.field_types(t) {
                    Some(fields) => fields.range().any(|k| {
                        let f = self.sc.field_arena[k];
                        self.is_checkable(f, depth + 1)
                    }),
                    None => false,
                }
            }
            _ => false,
        }
    }

    // ---- messages ----

    fn show_missing(&mut self, s: Space) -> String {
        let mut parts = Vec::new();
        self.flatten_into(s, &mut parts);
        // A whole type left over is named by its cases.
        let mut expanded = Vec::with_capacity(parts.len());
        for p in parts {
            match p {
                Space::Typ(t) => match self.decompose(t) {
                    Some(cases) if !cases.is_empty() => {
                        expanded.extend((0..cases.len as usize).map(|i| self.part(cases, i)))
                    }
                    _ => expanded.push(p),
                },
                other => expanded.push(other),
            }
        }
        let mut shown: Vec<String> = Vec::new();
        for &p in &expanded {
            let text = self.show_space(p);
            if !shown.contains(&text) {
                shown.push(text);
            }
        }
        let more = shown.len() > SHOWN_MISSING;
        shown.truncate(SHOWN_MISSING);
        let mut out = shown.join(", ");
        if more {
            out.push_str(", ...");
        }
        out
    }

    fn show_space(&mut self, s: Space) -> String {
        match s {
            Space::Empty => String::new(),
            Space::Opaque => "_".to_string(),
            Space::Lit(l) => self.show_lit(l),
            Space::Typ(t) => self.show_type_space(t),
            Space::Prod(c, _, fields) => {
                let fields: Vec<String> =
                    (0..fields.len as usize).map(|i| self.show_field(self.part(fields, i))).collect();
                self.wrap_fields(c, fields)
            }
            Space::Or(items) => {
                let items: Vec<String> =
                    (0..items.len as usize).map(|i| self.show_space(self.part(items, i))).collect();
                items.join(" | ")
            }
        }
    }

    fn show_field(&mut self, f: Space) -> String {
        match f {
            Space::Typ(t) if self.is_singleton_type(t) => self.show_type_space(t),
            Space::Lit(_) | Space::Prod(..) => self.show_space(f),
            _ => "_".to_string(),
        }
    }

    fn show_type_space(&mut self, t: TypeId) -> String {
        let Type::Class(c, _) = self.t.types.get(t) else { return self.t.show(t) };
        if self.is_singleton_type(t) {
            return self.t.name_str(self.t.syms.class(c).name);
        }
        match self.field_types(t) {
            Some(fields) => self.wrap_fields(c, fields.range().map(|_| "_".to_string()).collect()),
            None => self.t.show(t),
        }
    }

    fn wrap_fields(&self, c: ClassId, fields: Vec<String>) -> String {
        if self.t.is_tuple_class(c) {
            format!("({})", fields.join(", "))
        } else {
            format!("{}({})", self.t.name_str(self.t.syms.class(c).name), fields.join(", "))
        }
    }

    fn show_lit(&self, l: Lit) -> String {
        match l {
            Lit::Int(i) => i.to_string(),
            Lit::Long(l) => format!("{}L", l),
            Lit::Double(d) => f64::from_bits(d).to_string(),
            Lit::Bool(b) => b.to_string(),
            Lit::Char(c) => format!("'{}'", char::from_u32(c as u32).unwrap_or('?')),
            Lit::Str(s) => format!("\"{}\"", self.t.prog.strings[s.idx()]),
        }
    }
}

/// Whether the children of a sealed type are all of its values: a sealed class that is not
/// abstract has instances of its own.
impl<'a> Worker<'a> {
    /// What a match sees of a scrutinee or a tested type: a path as its type, an abstract
    /// member as its upper bound, a refinement as its parent.
    pub fn match_type(&mut self, t: TypeId) -> TypeId {
        let mut t = self.deref(t);
        for _ in 0..32 {
            match self.dependent_underlying(t) {
                Some(u) => t = self.deref(u),
                None => return t,
            }
        }
        t
    }
}

fn splits_into_children(info: &ClassInfo) -> bool {
    info.kind == ClassKind::Enum
        || (info.mods & mods::SEALED != 0 && (info.kind == ClassKind::Trait || info.mods & mods::ABSTRACT != 0))
}
