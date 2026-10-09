//! The merge's parallel parts: the listing of the types made after
//! the fork that the records hand over, each thread of the crew over its contiguous ranges of the
//! merged arenas; the interning of what was listed on the main thread, the store's one owner, in
//! the serial walk's order, with the walk's maps at their places; the promotion's mapping
//! published; the renumbering of the records by the crew, over the same ranges, reading the
//! published mappings and interning nothing.
//!
//! The order the interning takes is the serial walk's (`Remap::apply`): kind by kind, the work
//! items' records in the merged arenas' order and then the shared region's. A record is one
//! range's, and a range's types are listed in the order the record's visitor hands them over
//! (`records`), so the position compared between two threads' lists is the kind, the span (the
//! work items' or the shared region's) and the record's place, which is the order of the threads'
//! shares; a thread lists a type once, at the first of its places, the later ones handing over a
//! type promoted by then.

use super::records::{self, Mapping, SigMemo};
use super::{Remap, Table, Tables, UNMET, VISITING};
use crate::crew::{Crew, Shared};
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::cell::{Cell, RefCell};
use std::sync::Mutex;

/// The kinds of record whose visitors hand types over, in the walk's order; the deferred tests'
/// map stands between the patterns and the recorded types (`Remap::deferred_tests`). An
/// expression hands over the type a cast names (`TExpr::Cast`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Kind {
    Syms,
    Classes,
    Aliases,
    TParams,
    Exprs,
    Pats,
    ExprTypes,
    Quotes,
    QuotePats,
}

pub(super) const KINDS: [Kind; 9] = [Kind::Syms, Kind::Classes, Kind::Aliases, Kind::TParams, Kind::Exprs, Kind::Pats, Kind::ExprTypes, Kind::Quotes, Kind::QuotePats];

/// The first kind after the deferred tests.
pub(super) const AFTER_TESTS: usize = 6;

/// The arenas the renumbering walks besides the kinds that hold types, none of whose records
/// hands a type over.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Other {
    Pkgs,
    Overloads,
    ExprLists,
    PatLists,
    SymLists,
    Stmts,
    Cases,
    Tries,
    Tests,
    Funs,
    TClasses,
}

pub(super) const OTHERS: [Other; 11] = [Other::Pkgs, Other::Overloads, Other::ExprLists, Other::PatLists, Other::SymLists, Other::Stmts, Other::Cases, Other::Tries, Other::Tests, Other::Funs, Other::TClasses];

impl Other {
    pub(super) fn name(self) -> &'static str {
        match self {
            Other::Pkgs => "packages",
            Other::Overloads => "overload sets",
            Other::ExprLists => "expression lists",
            Other::PatLists => "pattern lists",
            Other::SymLists => "symbol lists",
            Other::Stmts => "statements",
            Other::Cases => "cases",
            Other::Tries => "tries",
            Other::Tests => "type tests",
            Other::Funs => "functions",
            Other::TClasses => "class bodies",
        }
    }

    /// The records the walk takes: every package (a package of the prefix gets a body's entries),
    /// of the others what the walk's prefix leaves.
    pub(super) fn range(self, walk: &super::Prefix, l: &super::Prefix) -> std::ops::Range<usize> {
        let (from, to) = match self {
            Other::Pkgs => (0, l.pkgs),
            Other::Overloads => (walk.overloads, l.overloads),
            Other::ExprLists => (walk.expr_lists, l.expr_lists),
            Other::PatLists => (walk.pat_lists, l.pat_lists),
            Other::SymLists => (walk.sym_lists, l.sym_lists),
            Other::Stmts => (walk.stmts, l.stmts),
            Other::Cases => (walk.cases, l.cases),
            Other::Tries => (walk.tries, l.tries),
            Other::Tests => (walk.tests, l.tests),
            Other::Funs => (walk.funs, l.funs),
            Other::TClasses => (walk.tclasses, l.tclasses),
        };
        from as usize..to as usize
    }
}

/// The record `i` of the arena `o` through `m`; a package's entries counted, for the profile.
#[inline]
pub(super) fn visit_other<M: Mapping>(o: Other, m: &M, a: &Arenas, walk: &super::Prefix, i: usize) -> u64 {
    // SAFETY: the caller's share of the arena's records, which no other thread visits.
    unsafe {
        match o {
            Other::Pkgs => return records::package(m, at(&a.pkgs, i), i >= walk.pkgs as usize),
            Other::Overloads => records::overload(m, at(&a.overloads, i)),
            Other::ExprLists => {
                let e = at(&a.expr_lists, i);
                *e = m.expr(*e);
            }
            Other::PatLists => {
                let p = at(&a.pat_lists, i);
                *p = m.pat(*p);
            }
            Other::SymLists => {
                let s = at(&a.sym_lists, i);
                *s = m.sym(*s);
            }
            Other::Stmts => records::stmt(m, at(&a.stmts, i)),
            Other::Cases => records::case(m, at(&a.cases, i)),
            Other::Tries => records::tri(m, at(&a.tries, i)),
            Other::Tests => records::type_test(m, at(&a.tests, i)),
            Other::Funs => records::fun(m, at(&a.funs, i)),
            Other::TClasses => records::tclass(m, at(&a.tclasses, i)),
        }
    }
    0
}

impl Kind {
    pub(super) fn name(self) -> &'static str {
        match self {
            Kind::Syms => "symbols",
            Kind::Classes => "classes",
            Kind::Aliases => "aliases",
            Kind::TParams => "type parameters",
            Kind::Exprs => "expressions",
            Kind::Pats => "patterns",
            Kind::ExprTypes => "recorded expression types",
            Kind::Quotes => "quotes",
            Kind::QuotePats => "quote patterns",
        }
    }
}

/// The merged arenas as the crew's threads reach them, each record by one thread alone: what
/// `Arenas::of` takes from the worker stays the worker's, its vectors neither grown nor moved
/// while the crew runs.
pub(super) struct Arenas {
    syms: Shared<SymInfo>,
    classes: Shared<ClassInfo>,
    tparams: Shared<TParamInfo>,
    aliases: Shared<AliasInfo>,
    pkgs: Shared<PkgInfo>,
    overloads: Shared<(SymId, Vec<SymId>)>,
    exprs: Shared<TExpr>,
    pats: Shared<TPat>,
    expr_lists: Shared<TExprId>,
    pat_lists: Shared<TPatId>,
    sym_lists: Shared<SymId>,
    stmts: Shared<TStmt>,
    cases: Shared<TCase>,
    tries: Shared<TTry>,
    tests: Shared<TypeTest>,
    funs: Shared<TFun>,
    tclasses: Shared<TClass>,
    quotes: Shared<TQuote>,
    quote_pats: Shared<TQuotePat>,
    expr_types: Shared<TypeId>,
    pub lens: super::Prefix,
}

macro_rules! arena_ptr {
    ($v:expr) => {
        Shared($v.as_mut_ptr())
    };
}

impl Arenas {
    pub(super) fn of(w: &mut super::Worker) -> Arenas {
        let lens = lens(&w.syms, &w.prog);
        let s = &mut w.syms;
        let p = &mut w.prog;
        Arenas {
            syms: arena_ptr!(s.syms.own_mut()),
            classes: arena_ptr!(s.classes.own_mut()),
            tparams: arena_ptr!(s.tparams.own_mut()),
            aliases: arena_ptr!(s.aliases.own_mut()),
            pkgs: arena_ptr!(s.pkgs.own_mut()),
            overloads: arena_ptr!(s.overloads.own_mut()),
            exprs: arena_ptr!(p.exprs.own_mut()),
            pats: arena_ptr!(p.pats.own_mut()),
            expr_lists: arena_ptr!(p.expr_lists.own_mut()),
            pat_lists: arena_ptr!(p.pat_lists.own_mut()),
            sym_lists: arena_ptr!(p.sym_lists.own_mut()),
            stmts: arena_ptr!(p.stmts.own_mut()),
            cases: arena_ptr!(p.cases.own_mut()),
            tries: arena_ptr!(p.tries.own_mut()),
            tests: arena_ptr!(p.tests.own_mut()),
            funs: arena_ptr!(p.funs.own_mut()),
            tclasses: arena_ptr!(p.classes.own_mut()),
            quotes: arena_ptr!(p.quotes.own_mut()),
            quote_pats: arena_ptr!(p.quote_pats.own_mut()),
            expr_types: arena_ptr!(p.expr_types.own_mut()),
            lens,
        }
    }
}

/// The lengths of the merged arenas.
fn lens(s: &Symbols, p: &Program) -> super::Prefix {
    super::Prefix {
        syms: s.syms.len() as u32,
        classes: s.classes.len() as u32,
        tparams: s.tparams.len() as u32,
        aliases: s.aliases.len() as u32,
        pkgs: s.pkgs.len() as u32,
        overloads: s.overloads.len() as u32,
        exprs: p.exprs.len() as u32,
        pats: p.pats.len() as u32,
        strings: p.strings.len() as u32,
        expr_lists: p.expr_lists.len() as u32,
        pat_lists: p.pat_lists.len() as u32,
        sym_lists: p.sym_lists.len() as u32,
        stmts: p.stmts.len() as u32,
        cases: p.cases.len() as u32,
        tries: p.tries.len() as u32,
        tests: p.tests.len() as u32,
        funs: p.funs.len() as u32,
        tclasses: p.classes.len() as u32,
        quotes: p.quotes.len() as u32,
        quote_pats: p.quote_pats.len() as u32,
        types: 0,
        names: 0,
        journal: 0,
    }
}

/// A record of the crew's range by its index: its thread's alone.
#[inline]
unsafe fn at<'a, T>(p: &Shared<T>, i: usize) -> &'a mut T {
    &mut *p.0.add(i)
}

/// The record `i` of `kind` through `m`.
#[inline]
pub(super) fn visit<M: Mapping>(kind: Kind, m: &M, memo: &mut SigMemo, a: &Arenas, i: usize) {
    // SAFETY: the caller's share of the kind's records, which no other thread visits.
    unsafe {
        match kind {
            Kind::Syms => records::symbol(m, memo, at(&a.syms, i)),
            Kind::Classes => {
                let c = at(&a.classes, i);
                records::subclasses(m, c);
                records::class(m, c);
            }
            Kind::Aliases => records::alias(m, at(&a.aliases, i)),
            Kind::TParams => records::tparam(m, at(&a.tparams, i)),
            Kind::Exprs => records::texpr(m, at(&a.exprs, i)),
            Kind::Pats => records::tpat(m, at(&a.pats, i)),
            Kind::ExprTypes => {
                let t = at(&a.expr_types, i);
                *t = m.ty(*t);
            }
            Kind::Quotes => records::quote(m, at(&a.quotes, i)),
            Kind::QuotePats => records::quote_pat(m, at(&a.quote_pats, i)),
        }
    }
}

/// The dense index of a type made after the fork among the promotion's tables (`Promoted::slot`):
/// the base's later growth first, then each overlay's.
pub(super) struct Slots {
    from: u32,
    late: usize,
    overlays: Vec<(usize, usize)>,
    pub total: usize,
}

impl Slots {
    /// The promotion's tables' slots, sized (`size_promotion`) or published.
    pub(super) fn of(r: &Remap) -> Slots {
        let p = r.promoted.borrow();
        let (late, overlays): (usize, Vec<usize>) = match &r.published {
            Some(x) => (x.late.len(), x.overlays.iter().map(|o| o.len()).collect()),
            None => (p.late.len(), p.overlays.iter().map(|o| o.len()).collect()),
        };
        let mut at = late;
        let overlays = overlays
            .into_iter()
            .map(|n| {
                let first = at;
                at += n;
                (first, n)
            })
            .collect();
        Slots { from: r.types_from, late, overlays, total: at }
    }

    pub(super) fn from(&self) -> u32 {
        self.from
    }

    #[inline]
    pub(super) fn index(&self, t: TypeId) -> Option<usize> {
        match crate::arena::worker_of(t.0) {
            Some(k) => {
                let (first, len) = *self.overlays.get(k)?;
                let i = (t.0 - crate::arena::worker_base(k)) as usize;
                (i < len).then_some(first + i)
            }
            None => {
                let i = (t.0 - self.from) as usize;
                (i < self.late).then_some(i)
            }
        }
    }
}

/// The listing's mapping: every id kept, every type handed over kept, the types made after the
/// fork listed at the first place of the thread's ranges that hands each over.
struct Lister<'r> {
    ids: &'r Tables,
    slots: &'r Slots,
    seen: RefCell<Vec<u64>>,
    out: RefCell<Vec<TypeId>>,
}

impl Mapping for Lister<'_> {
    #[inline]
    fn ids(&self) -> &Tables {
        self.ids
    }

    #[inline]
    fn ty(&self, t: TypeId) -> TypeId {
        if t.0 >= self.slots.from && t != NO_TYPE {
            self.list(t);
        }
        t
    }
}

impl Lister<'_> {
    #[inline]
    fn list(&self, t: TypeId) {
        match self.slots.index(t) {
            Some(i) => {
                let mut seen = self.seen.borrow_mut();
                let (w, b) = (i / 64, 1u64 << (i % 64));
                if seen[w] & b == 0 {
                    seen[w] |= b;
                    self.out.borrow_mut().push(t);
                }
            }
            // A type the promotion's tables were not sized for: listed every time (the
            // promotion makes it once).
            None => self.out.borrow_mut().push(t),
        }
    }
}

/// What the crew listed: per thread, the types in the order its shares handed them over, and
/// where the share of each kind's span begins (two spans a kind, the work items' and the shared
/// region's, and the end).
pub(super) struct Listing {
    threads: Vec<(Vec<TypeId>, Vec<usize>)>,
    pub bits: usize,
}

impl Listing {
    /// The bytes the listing holds: its entries and its threads' bits.
    pub(super) fn held(&self) -> usize {
        self.threads.iter().map(|(t, m)| t.capacity() * 4 + m.capacity() * 8).sum::<usize>() + self.bits / 8
    }

    /// The entries listed, over every thread.
    #[cfg(test)]
    pub(super) fn entries(&self) -> usize {
        self.threads.iter().map(|(t, _)| t.len()).sum()
    }

    /// The types listed for the kind at `kind`'s index in `KINDS`, in the walk's order.
    pub(super) fn of_kind(&self, kind: usize) -> impl Iterator<Item = TypeId> + '_ {
        (0..2).flat_map(move |span| self.threads.iter().flat_map(move |(out, marks)| out[marks[2 * kind + span]..marks[2 * kind + span + 1]].iter().copied()))
    }
}

/// The promotion's mapping, published: read by every thread of the crew, changed by none.
pub(super) struct Published {
    from: u32,
    late: Vec<u32>,
    overlays: Vec<Vec<u32>>,
}

impl Published {
    /// What the merge made of `t`, made after the fork: none when no record handed it over.
    #[inline]
    pub(super) fn get(&self, t: TypeId) -> Option<TypeId> {
        let s = match crate::arena::worker_of(t.0) {
            Some(k) => self.overlays.get(k).and_then(|v| v.get((t.0 - crate::arena::worker_base(k)) as usize)),
            None => self.late.get((t.0 - self.from) as usize),
        };
        match s {
            None => Some(t),
            Some(&UNMET) | Some(&VISITING) => None,
            Some(&m) => Some(TypeId(m - 1)),
        }
    }

    pub(super) fn held(&self) -> usize {
        self.late.capacity() * 4 + self.overlays.iter().map(|o| o.capacity() * 4).sum::<usize>()
    }
}

/// The renumbering's mapping: the arenas' new ids and the published promotion, read alone; a type
/// made after the fork that no listing handed over is the merge's defect, fatal, named with its
/// record.
struct Renumber<'r> {
    ids: &'r Tables,
    types: &'r Published,
    record: Cell<(&'static str, usize, u32)>,
}

impl Mapping for Renumber<'_> {
    #[inline]
    fn ids(&self) -> &Tables {
        self.ids
    }

    #[inline]
    fn ty(&self, t: TypeId) -> TypeId {
        if t.0 < self.types.from || t == NO_TYPE {
            return t;
        }
        let (what, i, nth) = self.record.get();
        self.record.set((what, i, nth + 1));
        match self.types.get(t) {
            Some(m) => m,
            None => self.unmapped(t),
        }
    }
}

impl Renumber<'_> {
    #[inline]
    fn at(&self, what: &'static str, i: usize) {
        self.record.set((what, i, 0));
    }

    #[cold]
    #[inline(never)]
    fn unmapped(&self, t: TypeId) -> ! {
        let (what, i, nth) = self.record.get();
        panic!("the merge's renumbering met type {} made after the fork, which no listing handed over: in record {} of the {}, its type number {}", t.0, i, what, nth)
    }
}

impl Remap {
    /// The spans of `kind`'s records the walk takes, in its order: the work items' and then the
    /// shared region's (`Remap::ranges`).
    pub(super) fn spans(&self, kind: Kind, lens: &super::Prefix) -> [std::ops::Range<usize>; 2] {
        let (walk, shared, len) = match kind {
            Kind::Syms => (self.walk.syms, self.shared.syms, lens.syms),
            Kind::Classes => (self.walk.classes, self.shared.classes, lens.classes),
            Kind::Aliases => (self.walk.aliases, self.shared.aliases, lens.aliases),
            Kind::TParams => (self.walk.tparams, self.shared.tparams, lens.tparams),
            Kind::Exprs => (self.walk.exprs, self.shared.exprs, lens.exprs),
            Kind::Pats => (self.walk.pats, self.shared.pats, lens.pats),
            Kind::ExprTypes => (self.walk.exprs, self.shared.exprs, lens.exprs),
            Kind::Quotes => (self.walk.quotes, self.shared.quotes, lens.quotes),
            Kind::QuotePats => (self.walk.quote_pats, self.shared.quote_pats, lens.quote_pats),
        };
        Remap::ranges(walk, shared, len as usize)
    }

    /// The listing: each thread of the crew over its share of every
    /// kind's spans, in the walk's order, listing each type made after the fork the first time its
    /// share hands it over. The promotion's tables are sized.
    pub(super) fn listing(&mut self, w: &mut super::Worker, crew: &Crew) -> Listing {
        assert!(self.walk.classes == 0, "the classes are walked whole");
        self.size_promotion();
        let arenas = Arenas::of(w);
        let slots = Slots::of(self);
        let identity = Tables::each(|_| Table::keep());
        let lists: Vec<Mutex<(Vec<TypeId>, Vec<usize>)>> = (0..crew.threads()).map(|_| Mutex::new((Vec::new(), Vec::new()))).collect();
        let spans: Vec<[std::ops::Range<usize>; 2]> = KINDS.iter().map(|&kind| self.spans(kind, &arenas.lens)).collect();
        let (spans, arenas, slots, identity, lists) = (&spans, &arenas, &slots, &identity, &lists);
        crew.run(&|k| {
            let lister = Lister { ids: identity, slots, seen: RefCell::new(vec![0u64; slots.total / 64 + 1]), out: RefCell::new(Vec::new()) };
            let mut memo = SigMemo::default();
            let mut marks = Vec::with_capacity(2 * KINDS.len() + 1);
            for (kind, spans) in KINDS.into_iter().zip(spans) {
                for span in spans.clone() {
                    marks.push(lister.out.borrow().len());
                    for i in crew.share(k, span) {
                        visit(kind, &lister, &mut memo, arenas, i);
                    }
                }
            }
            marks.push(lister.out.borrow().len());
            *lists[k].lock().unwrap_or_else(|e| e.into_inner()) = (lister.out.into_inner(), marks);
        });
        let mut threads: Vec<(Vec<TypeId>, Vec<usize>)> = lists.iter().map(|l| std::mem::take(&mut *l.lock().unwrap_or_else(|e| e.into_inner()))).collect();
        if super::trace::fault("stream") && threads[0].0.len() > 1 {
            threads[0].0.swap(0, 1);
        }
        Listing { bits: (slots.total / 64 + 1) * 64 * crew.threads(), threads }
    }

    /// The interning, on the main thread: the listed types promoted in
    /// the serial walk's order (`apply`), the deferred tests' map at its place among the kinds,
    /// then the walk's maps, the worker's tables, the profile and the variables, as the serial walk
    /// takes them.
    pub(super) fn intern(&mut self, w: &mut super::Worker, listing: &Listing, clock: &super::Clock) {
        if super::trace::fault("order") {
            if let Some(second) = listing.of_kind(0).nth(1) {
                self.ty(second);
            }
        }
        for (k, _) in KINDS.iter().enumerate() {
            if k == AFTER_TESTS {
                self.deferred_tests(&mut w.prog);
            }
            for t in listing.of_kind(k) {
                self.ty(t);
            }
        }
        clock.part("interning");
        self.registers(&mut w.prog);
        self.program_maps(w);
        self.map(&mut w.syms.dispatch_names.local, |r, s| r.sym(s), |_, _| {});
        clock.part("interning of the maps and the variables");
        self.tables_typed(w);
        clock.part("interning of the maps and the variables");
        w.profile.remap(|s| self.sym(s), |c| self.class(c), |t| self.ty(t), |e| self.expr(e));
        self.variables(w);
        self.map_segment(None);
        clock.part("interning of the maps and the variables");
    }

    /// The promotion's mapping published for the crew's renumbering: from here the promotion
    /// makes nothing.
    pub(super) fn publish(&mut self) {
        let p = self.promoted.get_mut();
        self.published = Some(Published { from: self.types_from, late: std::mem::take(&mut p.late), overlays: std::mem::take(&mut p.overlays) });
    }

    /// The renumbering: each thread of the crew over its share of
    /// every arena, the kinds with types over the listing's shares, reading the arenas' new ids and
    /// the published promotion; then the templates' quote indices, which read the expressions and
    /// their lists renumbered.
    pub(super) fn renumber(&mut self, w: &mut super::Worker, crew: &Crew) {
        let arenas = Arenas::of(w);
        let types = self.published.as_ref().expect("the promotion published");
        let templates: Vec<Mutex<Vec<usize>>> = (0..crew.threads()).map(|_| Mutex::new(Vec::new())).collect();
        let counted: Vec<Mutex<u64>> = (0..crew.threads()).map(|_| Mutex::new(0)).collect();
        let spans: Vec<[std::ops::Range<usize>; 2]> = KINDS.iter().map(|&kind| self.spans(kind, &arenas.lens)).collect();
        let (walk, ids) = (&self.walk, &self.ids);
        let (spans, arenas, templates, counted) = (&spans, &arenas, &templates, &counted);
        crew.run(&|k| {
            let m = Renumber { ids, types, record: Cell::new(("", 0, 0)) };
            let mut memo = SigMemo::default();
            let mut mine = Vec::new();
            for (kind, spans) in KINDS.into_iter().zip(spans) {
                for span in spans.clone() {
                    for i in crew.share(k, span) {
                        // SAFETY: this thread's share.
                        if kind == Kind::Exprs && matches!(unsafe { at(&arenas.exprs, i) }, TExpr::Js(..)) {
                            mine.push(i);
                        }
                        m.at(kind.name(), i);
                        visit(kind, &m, &mut memo, arenas, i);
                    }
                }
            }
            let mut count = 0u64;
            for o in OTHERS {
                for i in crew.share(k, o.range(walk, &arenas.lens)) {
                    count += visit_other(o, &m, arenas, walk, i);
                }
            }
            *templates[k].lock().unwrap_or_else(|e| e.into_inner()) = mine;
            *counted[k].lock().unwrap_or_else(|e| e.into_inner()) = count + memo.records;
        });
        // The worker's id-only tables, a task each on the crew (`untyped_tables`).
        let mode = self.tables_mode();
        let tasks: Vec<Mutex<Option<super::TableTask>>> = super::untyped_tables(w, mode).into_iter().map(|t| Mutex::new(Some(t))).collect();
        let ids = &self.ids;
        crew.each(tasks.len(), &|i| {
            let task = tasks[i].lock().unwrap_or_else(|e| e.into_inner()).take();
            if let Some(task) = task {
                task(&Renumber { ids, types, record: Cell::new(("the worker's tables", i, 0)) });
            }
        });
        drop(tasks);
        let templates: Vec<usize> = templates.iter().flat_map(|t| std::mem::take(&mut *t.lock().unwrap_or_else(|e| e.into_inner()))).collect();
        if super::trace::fault("renumber") {
            let at = self.walk.expr_lists as usize;
            if let Some(e) = w.prog.expr_lists.own_mut().get_mut(at) {
                *e = TExprId(e.0 + 1);
            }
        }
        self.quote_indices(&mut w.prog, &templates);
        let l = &arenas.lens;
        let walk = &self.walk;
        self.walked += counted.iter().map(|c| *c.lock().unwrap_or_else(|e| e.into_inner())).sum::<u64>()
            + (l.syms - walk.syms) as u64
            + l.classes as u64
            + (l.aliases - walk.aliases) as u64
            + (l.tparams - walk.tparams) as u64
            + (l.overloads - walk.overloads) as u64
            + [(l.exprs, walk.exprs), (l.pats, walk.pats), (l.expr_lists, walk.expr_lists), (l.pat_lists, walk.pat_lists), (l.sym_lists, walk.sym_lists), (l.stmts, walk.stmts), (l.cases, walk.cases), (l.tries, walk.tries), (l.tests, walk.tests), (l.funs, walk.funs), (l.tclasses, walk.tclasses), (l.quotes, walk.quotes), (l.quote_pats, walk.quote_pats)].iter().map(|&(n, b)| (n - b) as u64).sum::<u64>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::{worker_base, LOCAL_BASE};

    fn slots() -> Slots {
        Slots { from: 100, late: 10, overlays: vec![(10, 5), (15, 5)], total: 20 }
    }

    #[test]
    fn the_stream_takes_every_threads_share_of_a_span_before_the_next_span() {
        let t = |n: u32| TypeId(n);
        // Two threads, two kinds: each thread's marks bound its share of (kind 0, items),
        // (kind 0, shared), (kind 1, items), (kind 1, shared).
        let listing = Listing { threads: vec![(vec![t(1), t(2), t(3), t(4)], vec![0, 1, 2, 3, 4]), (vec![t(5), t(6), t(7)], vec![0, 2, 2, 3, 3])], bits: 0 };
        assert_eq!(listing.of_kind(0).map(|t| t.0).collect::<Vec<_>>(), [1, 5, 6, 2]);
        assert_eq!(listing.of_kind(1).map(|t| t.0).collect::<Vec<_>>(), [3, 7, 4]);
        assert_eq!(listing.entries(), 7);
    }

    #[test]
    fn a_thread_lists_a_type_made_after_the_fork_once_at_its_first_place() {
        let slots = slots();
        let identity = Tables::each(|_| Table::keep());
        let lister = Lister { ids: &identity, slots: &slots, seen: RefCell::new(vec![0; 1]), out: RefCell::new(Vec::new()) };
        let overlay = TypeId(worker_base(1) + 2);
        for t in [TypeId(3), TypeId(104), overlay, TypeId(104), NO_TYPE, overlay, TypeId(105), TypeId(LOCAL_BASE + 99)] {
            assert_eq!(lister.ty(t), t, "the listing keeps every type");
        }
        // The overlay type past the tables' size is listed every time it is met.
        assert_eq!(lister.out.into_inner(), [TypeId(104), overlay, TypeId(105), TypeId(LOCAL_BASE + 99)]);
    }

    #[test]
    fn the_published_mapping_tells_a_type_never_promoted_from_one_promoted() {
        let p = Published { from: 100, late: vec![UNMET, 7 + 1, VISITING], overlays: vec![vec![UNMET, 42 + 1]] };
        assert_eq!(p.get(TypeId(100)), None);
        assert_eq!(p.get(TypeId(101)), Some(TypeId(7)));
        assert_eq!(p.get(TypeId(102)), None);
        assert_eq!(p.get(TypeId(worker_base(0) + 1)), Some(TypeId(42)));
        assert_eq!(p.get(TypeId(103)), Some(TypeId(103)), "a type the merge made itself");
        let identity = Tables::each(|_| Table::keep());
        let r = Renumber { ids: &identity, types: &p, record: Cell::new(("", 0, 0)) };
        assert_eq!((r.ty(TypeId(5)), r.ty(NO_TYPE), r.ty(TypeId(101))), (TypeId(5), NO_TYPE, TypeId(7)));
    }

    #[test]
    #[should_panic(expected = "which no listing handed over: in record 12 of the symbols, its type number 2")]
    fn the_renumbering_refuses_a_type_no_listing_handed_over() {
        let p = Published { from: 100, late: vec![UNMET, 7 + 1], overlays: Vec::new() };
        let identity = Tables::each(|_| Table::keep());
        let r = Renumber { ids: &identity, types: &p, record: Cell::new(("", 0, 0)) };
        r.at("symbols", 12);
        r.ty(TypeId(101));
        r.ty(TypeId(100));
    }
}
