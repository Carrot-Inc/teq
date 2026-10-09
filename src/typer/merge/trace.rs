//! The merge-order check (`TEQ_MERGE_TRACE=1`, the assertion builds):
//! the parallel merge against the serial walk's order, derived apart from the store the merge
//! mutates. Before the interning, with nothing changed: the trace walks the records in the serial
//! walk's order with a mapping that keeps every id and type, recording each type made after the
//! fork at its first place (the kind, the span, the record, its type's number) and every record's
//! references as they are; a shadow store copies the base (`TypeStore::shadow`). During the
//! interning the walk's maps record the types they hand over (`MapRoots`). After the renumbering:
//! the merged stream's first occurrences against the trace's, kind by kind; the shadow's promotion
//! over the trace's order and the maps' against the published mapping, type by type; every
//! record's references against the trace's under the arenas' new ids and the shadow's mapping.
//! The first difference ends the build, named.

use super::parallel::{self, Arenas, Listing, Slots, KINDS, OTHERS};
use super::records::{Mapping, SigMemo};
use super::{Remap, Table, Tables, UNMET, VISITING};
use crate::ast::ListRef;
use crate::tir::*;
use crate::types::*;
use std::cell::{Cell, RefCell};
use std::sync::Arc;

/// Whether the merge-order check runs: `TEQ_MERGE_TRACE=1` in the assertion builds.
pub(super) fn wanted() -> bool {
    cfg!(debug_assertions) && std::env::var_os("TEQ_MERGE_TRACE").is_some_and(|v| v == "1")
}

/// `TEQ_MERGE_FAULT` in the assertion builds, the merge-order check's own tests: `stream` swaps
/// two types of the first thread's listing, `order` interns one listed type before the one
/// listed ahead of it, `renumber` changes one reference of an expression list after the
/// renumbering. Each is a difference the check names.
pub(super) fn fault(which: &str) -> bool {
    cfg!(debug_assertions) && std::env::var_os("TEQ_MERGE_FAULT").is_some_and(|v| v == which)
}

/// The types the walk's maps hand over during the interning, for the shadow's promotion: the
/// deferred tests' (between the patterns and the recorded types) and the rest's (after the quote
/// patterns: the program's maps, the capture, the worker's tables, the profile, the variables).
#[derive(Default)]
pub(super) struct MapRoots {
    segment: Option<usize>,
    seqs: [Vec<TypeId>; 2],
}

/// A place of the walk: the kind or arena, the span (the work items' or the shared region's),
/// the record, and the reference's number in the record.
#[derive(Clone, Copy, Debug)]
pub(super) struct Pos {
    what: &'static str,
    span: u8,
    record: u32,
    nth: u32,
}

/// What a recorded reference is: the table that renumbers it, or a type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Ref {
    Sym,
    Class,
    TParam,
    Alias,
    Pkg,
    Overload,
    Expr,
    Pat,
    Str,
    Test,
    Fun,
    Try,
    ExprList,
    SymList,
    PatList,
    StmtList,
    CaseList,
    EmptyList,
    Type,
}

/// What the trace took before the interning.
pub(super) struct Trace {
    /// The types made after the fork the record kinds hand over, each at its first place in the
    /// serial walk's order, and where each kind's begin.
    order: Vec<(Pos, TypeId)>,
    kind_starts: Vec<usize>,
    /// Every renumbered record's references as they were, in the comparison's order.
    refs: Vec<(Ref, u32)>,
    shadow: Arc<TypeStore>,
}

/// The trace's and the reader's mapping: every id and type kept, each recorded (the trace) or
/// compared with the trace's under the merge (the reader).
struct Tracer<'r> {
    ids: &'r Tables,
    at: Cell<Pos>,
    /// The trace's: the types' first places, while the record kinds are walked.
    slots: Option<&'r Slots>,
    seen: RefCell<Vec<u64>>,
    order: RefCell<Vec<(Pos, TypeId)>>,
    kind_starts: RefCell<Vec<usize>>,
    refs: RefCell<Vec<(Ref, u32)>>,
    /// The reader's: the trace's references, the next one, what the merge should have made of
    /// each, and the first difference.
    against: Option<(&'r [(Ref, u32)], &'r dyn Fn(Ref, u32) -> u32)>,
    next: Cell<usize>,
    first: RefCell<Option<String>>,
}

impl<'r> Tracer<'r> {
    fn new(ids: &'r Tables) -> Tracer<'r> {
        Tracer { ids, at: Cell::new(Pos { what: "", span: 0, record: 0, nth: 0 }), slots: None, seen: RefCell::new(Vec::new()), order: RefCell::new(Vec::new()), kind_starts: RefCell::new(Vec::new()), refs: RefCell::new(Vec::new()), against: None, next: Cell::new(0), first: RefCell::new(None) }
    }

    fn record(&self, what: &'static str, span: u8, record: usize) {
        self.at.set(Pos { what, span, record: record as u32, nth: 0 });
    }

    #[inline]
    fn note(&self, r: Ref, v: u32) {
        let mut at = self.at.get();
        at.nth += 1;
        self.at.set(at);
        match self.against {
            None => self.refs.borrow_mut().push((r, v)),
            Some((refs, expected)) => {
                let i = self.next.get();
                self.next.set(i + 1);
                if self.first.borrow().is_some() {
                    return;
                }
                let found = match refs.get(i) {
                    None => Some("past the trace's references".to_string()),
                    Some(&(tr, old)) if tr != r => Some(format!("a {:?} where the trace had a {:?} ({})", r, tr, old)),
                    Some(&(tr, old)) => {
                        let want = expected(tr, old);
                        (want != v).then(|| format!("{:?} {} where the serial walk makes {} of {}", r, v, want, old))
                    }
                };
                if let Some(what) = found {
                    let span = match at.span {
                        0 => " (the work items' span)",
                        1 => " (the shared region's span)",
                        _ => "",
                    };
                    *self.first.borrow_mut() = Some(format!("{} in record {} of the {}{}, reference {}", what, at.record, at.what, span, at.nth));
                }
            }
        }
    }

    #[inline]
    fn list(&self, r: Ref, l: ListRef) -> ListRef {
        if l.len == 0 {
            self.note(Ref::EmptyList, 0);
        } else {
            self.note(r, l.start);
        }
        l
    }
}

impl Mapping for Tracer<'_> {
    fn ids(&self) -> &Tables {
        self.ids
    }
    fn ty(&self, t: TypeId) -> TypeId {
        self.note(Ref::Type, t.0);
        if let Some(slots) = self.slots {
            if t.0 >= slots.from() && t != NO_TYPE {
                let first = match slots.index(t) {
                    Some(i) => {
                        let mut seen = self.seen.borrow_mut();
                        let (w, b) = (i / 64, 1u64 << (i % 64));
                        let first = seen[w] & b == 0;
                        seen[w] |= b;
                        first
                    }
                    None => true,
                };
                if first {
                    self.order.borrow_mut().push((self.at.get(), t));
                }
            }
        }
        t
    }
    fn sym(&self, s: SymId) -> SymId {
        self.note(Ref::Sym, s.0);
        s
    }
    fn class(&self, c: ClassId) -> ClassId {
        self.note(Ref::Class, c.0);
        c
    }
    fn tparam(&self, p: TParamId) -> TParamId {
        self.note(Ref::TParam, p.0);
        p
    }
    fn alias(&self, a: AliasId) -> AliasId {
        self.note(Ref::Alias, a.0);
        a
    }
    fn pkg(&self, p: PkgId) -> PkgId {
        self.note(Ref::Pkg, p.0);
        p
    }
    fn overload(&self, i: u32) -> u32 {
        self.note(Ref::Overload, i);
        i
    }
    fn expr(&self, e: TExprId) -> TExprId {
        self.note(Ref::Expr, e.0);
        e
    }
    fn pat(&self, p: TPatId) -> TPatId {
        self.note(Ref::Pat, p.0);
        p
    }
    fn string(&self, s: StrRef) -> StrRef {
        self.note(Ref::Str, s.0);
        s
    }
    fn test(&self, t: TestId) -> TestId {
        self.note(Ref::Test, t.0);
        t
    }
    fn fun(&self, f: FunId) -> FunId {
        self.note(Ref::Fun, f.0);
        f
    }
    fn tri(&self, i: u32) -> u32 {
        self.note(Ref::Try, i);
        i
    }
    fn expr_list(&self, l: ListRef) -> ListRef {
        self.list(Ref::ExprList, l)
    }
    fn sym_list(&self, l: ListRef) -> ListRef {
        self.list(Ref::SymList, l)
    }
    fn pat_list(&self, l: ListRef) -> ListRef {
        self.list(Ref::PatList, l)
    }
    fn stmt_list(&self, l: ListRef) -> ListRef {
        self.list(Ref::StmtList, l)
    }
    fn case_list(&self, l: ListRef) -> ListRef {
        self.list(Ref::CaseList, l)
    }
}

impl Remap {
    #[cold]
    #[inline(never)]
    pub(super) fn map_root(&self, t: TypeId) {
        let mut r = self.map_roots.as_ref().expect("the maps' roots").borrow_mut();
        if let Some(s) = r.segment {
            r.seqs[s].push(t);
        }
    }

    /// The walk's maps' segment under way, for the merge-order check: the deferred tests' (0), the
    /// maps' after the quote patterns (1), or none.
    pub(super) fn map_segment(&self, segment: Option<usize>) {
        if let Some(r) = &self.map_roots {
            r.borrow_mut().segment = segment;
        }
    }

    /// Every renumbered record through `m`, in the comparison's order: the record kinds that hold
    /// types in the walk's order and spans, then the other arenas.
    fn each_record(&self, m: &Tracer, a: &Arenas) {
        let mut memo = SigMemo::default();
        let l = &a.lens;
        for kind in KINDS {
            m.kind_starts.borrow_mut().push(m.order.borrow().len());
            for (span, range) in self.spans(kind, l).into_iter().enumerate() {
                for i in range {
                    m.record(kind.name(), span as u8, i);
                    parallel::visit(kind, m, &mut memo, a, i);
                }
            }
        }
        m.kind_starts.borrow_mut().push(m.order.borrow().len());
        for o in OTHERS {
            for i in o.range(&self.walk, l) {
                m.record(o.name(), 2, i);
                parallel::visit_other(o, m, a, &self.walk, i);
            }
        }
    }

    /// The trace, before the interning (the promotion's tables sized): the serial walk's order of
    /// the types made after the fork, every renumbered record's references, the shadow store; and
    /// the maps' hand-overs recorded from here.
    pub(super) fn trace(&mut self, w: &mut super::Worker) -> Trace {
        let a = Arenas::of(w);
        let slots = Slots::of(self);
        let identity = Tables::each(|_| Table::keep());
        let mut t = Tracer::new(&identity);
        t.slots = Some(&slots);
        *t.seen.get_mut() = vec![0u64; slots.total / 64 + 1];
        self.each_record(&t, &a);
        let (order, kind_starts) = (t.order.into_inner(), t.kind_starts.into_inner());
        self.map_roots = Some(RefCell::new(MapRoots::default()));
        Trace { order, kind_starts, refs: t.refs.into_inner(), shadow: Arc::new(self.types.shadow()) }
    }

    /// The check, after the renumbering: the merged stream's first occurrences (none under the
    /// serial walk, `listing`'s absence), the published mapping and the records against the trace.
    pub(super) fn check_trace(&mut self, w: &mut super::Worker, trace: Trace, listing: Option<&Listing>) -> Result<(), String> {
        let roots = self.map_roots.take().expect("the maps' roots").into_inner();
        if let Some(listing) = listing {
            let slots = Slots::of(self);
            // The first occurrences over the whole stream, as the trace's are over the walk.
            let mut seen = vec![0u64; slots.total / 64 + 1];
            for (k, kind) in KINDS.iter().enumerate() {
                let mut firsts = listing.of_kind(k).filter(|&t| match slots.index(t) {
                    Some(i) => {
                        let (w, b) = (i / 64, 1u64 << (i % 64));
                        let first = seen[w] & b == 0;
                        seen[w] |= b;
                        first
                    }
                    None => true,
                });
                for &(at, want) in &trace.order[trace.kind_starts[k]..trace.kind_starts[k + 1]] {
                    match firsts.next() {
                        Some(got) if got == want => {}
                        got => return Err(format!("the merged stream of the {} hands over {} where the serial walk first meets type {} in record {} of the {} span, its type number {}", kind.name(), got.map_or("nothing more".to_string(), |t| format!("type {}", t.0)), want.0, at.record, if at.span == 0 { "work items'" } else { "shared region's" }, at.nth)),
                    }
                }
                if let Some(extra) = firsts.next() {
                    return Err(format!("the merged stream of the {} hands over type {} the serial walk never meets", kind.name(), extra.0));
                }
            }
        }
        // The shadow's promotion, in the serial walk's order with the maps' at their places.
        let mut shadow = Remap {
            walk: super::Prefix { ..self.walk },
            shared: super::Prefix { ..self.shared },
            types: trace.shadow.clone(),
            source: Some(self.types.clone()),
            map_roots: None,
            types_from: self.types_from,
            promoted: Default::default(),
            published: None,
            checking: None,
            forked: true,
            ids: self.ids.copy(),
            sigs: SigMemo::default(),
            walked: 0,
            clock: Default::default(),
        };
        shadow.size_promotion();
        for (k, _) in KINDS.iter().enumerate() {
            if k == parallel::AFTER_TESTS {
                for &t in &roots.seqs[0] {
                    shadow.ty(t);
                }
            }
            for &(_, t) in &trace.order[trace.kind_starts[k]..trace.kind_starts[k + 1]] {
                shadow.ty(t);
            }
        }
        for &t in &roots.seqs[1] {
            shadow.ty(t);
        }
        let published = self.published.as_ref().expect("the promotion published");
        let p = shadow.promoted.borrow();
        let slot = |v: u32| match v {
            UNMET => None,
            VISITING => Some(u32::MAX),
            m => Some(m - 1),
        };
        for (i, &s) in p.late.iter().enumerate() {
            let t = TypeId(self.types_from + i as u32);
            let (want, got) = (slot(s), published.get(t).map(|m| m.0));
            if want != got {
                return Err(format!("the merge made {:?} of type {} where the shadow's serial promotion made {:?}", got, t.0, want));
            }
        }
        for (k, o) in p.overlays.iter().enumerate() {
            for (i, &s) in o.iter().enumerate() {
                let t = TypeId(crate::arena::worker_base(k) + i as u32);
                let (want, got) = (slot(s), published.get(t).map(|m| m.0));
                if want != got {
                    return Err(format!("the merge made {:?} of worker {}'s type {} where the shadow's serial promotion made {:?}", got, k, t.0, want));
                }
            }
        }
        drop(p);
        // The records: what the merge left against what the serial walk makes of the trace's.
        let from = self.types_from;
        let ids = &self.ids;
        let expected = |r: Ref, old: u32| -> u32 {
            match r {
                Ref::Sym => ids.sym(SymId(old)).0,
                Ref::Class => ids.class(ClassId(old)).0,
                Ref::TParam => ids.tparam(TParamId(old)).0,
                Ref::Alias => ids.alias(AliasId(old)).0,
                Ref::Pkg => ids.pkg(PkgId(old)).0,
                Ref::Overload => ids.overloads.map(old),
                Ref::Expr => ids.expr(TExprId(old)).0,
                Ref::Pat => ids.pat(TPatId(old)).0,
                Ref::Str => ids.string(StrRef(old)).0,
                Ref::Test => ids.test(TestId(old)).0,
                Ref::Fun => ids.fun(FunId(old)).0,
                Ref::Try => ids.tries.map(old),
                Ref::ExprList => ids.expr_lists.map(old),
                Ref::SymList => ids.sym_lists.map(old),
                Ref::PatList => ids.pat_lists.map(old),
                Ref::StmtList => ids.stmts.map(old),
                Ref::CaseList => ids.cases.map(old),
                Ref::EmptyList => 0,
                Ref::Type if old < from || old == NO_TYPE.0 => old,
                Ref::Type => shadow.promoted_of(TypeId(old)).map_or(u32::MAX, |t| t.0),
            }
        };
        let identity = Tables::each(|_| Table::keep());
        let mut reader = Tracer::new(&identity);
        reader.against = Some((&trace.refs, &expected));
        let a = Arenas::of(w);
        self.each_record(&reader, &a);
        if let Some(first) = reader.first.into_inner() {
            return Err(format!("the merge renumbered a record otherwise than the serial walk: {}", first));
        }
        if reader.next.get() != trace.refs.len() {
            return Err(format!("the merge's records hold {} references where the trace recorded {}", reader.next.get(), trace.refs.len()));
        }
        Ok(())
    }
}

impl Tables {
    /// A copy of every table, for the merge-order check's shadow.
    pub(super) fn copy(&self) -> Tables {
        let all = self.all();
        let mut i = 0;
        Tables::each(|_| {
            let t = all[i].1.copy();
            i += 1;
            t
        })
    }
}

impl Table {
    fn copy(&self) -> Table {
        Table::of(self.base, self.new.clone())
    }
}
