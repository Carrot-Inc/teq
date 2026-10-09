//! The merge after the body phase: what the bodies made (the chunk, every record of the arenas
//! past the signature phase's prefix) renumbered into the order of the sources, the worker's
//! tables rebuilt under the new ids, and the types the workers
//! made promoted into the base. With one worker the chunk
//! is the arenas'
//! suffix and every id maps to itself, so a build does not run the walk; under `--profile` it
//! runs as the measurement of what the parallel typer's merge costs (the `merge` row).
//!
//! The order. The workers' records are placed by their work items' keys (`ItemRange`: the file
//! and the definition in it), the shared region's before them. One walk renumbers every record
//! that can hold a worker's id (`Remap::apply`) and promotes each type made after the fork the
//! first time it hands one over (`Remap::promote`, parts first): the work items' records first
//! in the merged arenas' order, then the shared region's, whose order is the loader's lock's,
//! then the registers and the tables, a map whose values hold types in the order of its keys
//! under the new ids and a register the workers appended to in the order of its sites
//! (`map_sorted`, `promote_in_order`), then the variables' tables (`Remap::variables`). So what
//! the base grows by follows the sources' order and not the workers'; the base's own numbering
//! of what it held at the join (the completions' and the exports' types) is the loader's.
//! Then the check (`Worker::check_merge`: the roots the promotion handed over, or the same walk
//! again over every record from the first of each arena, the sweep), then the overlays go
//! (`TypeStore::reclaim_overlays`).
//!
//! What the walk covers: the symbol table's records (a symbol's owner, kind, signature and
//! implementing class; a class's owner, parameters, parents, members, nested classes,
//! constructors, companions and the rest of its info; an alias's owner, parameters, right-hand
//! side and bounds; a type parameter's bounds; a package's entries, givens and package object),
//! the `subclasses` of every class, the overload sets and the dispatch names; the program's
//! records (expressions, patterns, lists, statements, cases, tries, type tests, functions,
//! class bodies, quotes and quote patterns, and the quotes' indices the `$quote` and
//! `$quoteMatch` templates carry as `Int` arguments) and its registers (the top-level functions
//! and vals, the entry point, the exports, the template calls and symbols, `Program::overrides`,
//! the expansions and leaf tests, the deferred tests, the recorded expression types, the
//! capture's tables, the dependency recorder's); the worker's per-thread tables keyed by an id,
//! the deferred checks, bounds and inline registrations, the stored
//! inline bodies, the export tables, the mirrors, the quote state's deferred calls and copies,
//! the builtins made on first use and the std's types found on the way, the site classes, the
//! files' opaque types, the inline state's folded literals and arguments, the match types'
//! reported recursions, the profile's records, the navigation index's records and tables (the
//! records joined in the work items' order, `merge_index_records`).
//!
//! What it leaves, by design: the per-expression bit sets and spans, placed by the arenas' merge
//! with their expressions; names and files, which are every worker's (`FileId`s of pseudo files
//! are appended under the loader's lock); the loader's tables and the references of converted
//! ASTs, filled under the lock with the shared region's ids, which the check reads
//! (`loader_left_local`); the memos and indexes over types and ids, which the merge drops and
//! the build makes again (`merge_workers`' end); the working state of a typing under way, which
//! the next one sets before it reads, dropped at the merge.

mod parallel;
mod records;
mod trace;

pub(super) use records::Mapping;
use records::SigMemo;
use super::{Env, ImportTarget, ResolvedImport, TVarInfo, TVars, Worker};
use crate::source::FileId;
use crate::ast::ListRef;
use crate::intern::FxMap;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::sync::Arc;

/// The sizes of the arenas at the end of the signature phase: what the body phase adds past
/// them is the chunk.
pub struct Prefix {
    pub syms: u32,
    pub classes: u32,
    pub tparams: u32,
    pub aliases: u32,
    pub pkgs: u32,
    pub overloads: u32,
    pub exprs: u32,
    pub pats: u32,
    pub strings: u32,
    pub expr_lists: u32,
    pub pat_lists: u32,
    pub sym_lists: u32,
    pub stmts: u32,
    pub cases: u32,
    pub tries: u32,
    pub tests: u32,
    pub funs: u32,
    pub tclasses: u32,
    pub quotes: u32,
    pub quote_pats: u32,
    pub types: u32,
    pub names: u32,
    /// The navigation index's journal (`index::Index`).
    pub journal: u32,
}

impl Prefix {
    pub fn of(w: &Worker) -> Prefix {
        let p = &w.prog;
        Prefix {
            syms: w.syms.syms.len() as u32,
            classes: w.syms.classes.len() as u32,
            tparams: w.syms.tparams.len() as u32,
            aliases: w.syms.aliases.len() as u32,
            pkgs: w.syms.pkgs.len() as u32,
            overloads: w.syms.overloads.len() as u32,
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
            types: w.types.len().0 as u32,
            names: w.interner.len() as u32,
            journal: w.index.as_ref().map_or(0, |ix| ix.journal_len()),
        }
    }
}

/// The new id of every chunk id of one arena, per worker: `new[worker][old - worker's base]`;
/// an id below the base is the prefix's and keeps it. With one table the ids past the base
/// are all its. With none (`Table::checking`), an id past the base is a worker's that the merge
/// left, kept and counted in `refused`.
pub(super) struct Table {
    base: u32,
    new: Vec<Vec<u32>>,
    refused: std::sync::atomic::AtomicU32,
}

impl Table {
    fn held(&self) -> usize {
        self.new.iter().map(|v| v.capacity() * 4).sum::<usize>()
    }

    fn identity(base: u32, len: usize) -> Table {
        Table::of(base, vec![(base..base + len as u32).collect()])
    }

    fn of(base: u32, new: Vec<Vec<u32>>) -> Table {
        Table { base, new, refused: std::sync::atomic::AtomicU32::new(0) }
    }

    /// The table of the merge's check: every id stays, and one of a worker's is counted.
    fn checking() -> Table {
        Table::of(crate::arena::LOCAL_BASE, Vec::new())
    }

    #[inline]
    fn map(&self, id: u32) -> u32 {
        if id < self.base {
            id
        } else if self.new.len() == 1 {
            self.new[0][(id - self.base) as usize]
        } else if self.new.is_empty() {
            self.refuse(id)
        } else {
            let off = id - self.base;
            self.new[(off / crate::arena::WORKER_SPAN) as usize][(off % crate::arena::WORKER_SPAN) as usize]
        }
    }

    #[cold]
    #[inline(never)]
    fn refuse(&self, id: u32) -> u32 {
        self.refused.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        id
    }

    fn refused(&self) -> u32 {
        self.refused.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Every id kept, none counted.
    fn keep() -> Table {
        Table::of(u32::MAX, Vec::new())
    }
}

/// The new id of every record of each arena (`Table`): the merge's positional numbering, read by
/// every visitor of the records (`Mapping`) and, published, by the crew's threads at once.
pub(super) struct Tables {
    syms: Table,
    classes: Table,
    tparams: Table,
    aliases: Table,
    pkgs: Table,
    overloads: Table,
    exprs: Table,
    pats: Table,
    strings: Table,
    expr_lists: Table,
    pat_lists: Table,
    sym_lists: Table,
    stmts: Table,
    cases: Table,
    tries: Table,
    tests: Table,
    funs: Table,
    tclasses: Table,
    quotes: Table,
    quote_pats: Table,
}

impl Tables {
    /// Every table made by `table`, given the arena's name.
    fn each(mut table: impl FnMut(&'static str) -> Table) -> Tables {
        Tables {
            syms: table("syms"),
            classes: table("classes"),
            tparams: table("tparams"),
            aliases: table("aliases"),
            pkgs: table("pkgs"),
            overloads: table("overloads"),
            exprs: table("exprs"),
            pats: table("pats"),
            strings: table("strings"),
            expr_lists: table("expr_lists"),
            pat_lists: table("pat_lists"),
            sym_lists: table("sym_lists"),
            stmts: table("stmts"),
            cases: table("cases"),
            tries: table("tries"),
            tests: table("tests"),
            funs: table("funs"),
            tclasses: table("tclasses"),
            quotes: table("quotes"),
            quote_pats: table("quote_pats"),
        }
    }

    /// The tables with their arenas' names.
    fn all(&self) -> [(&'static str, &Table); 20] {
        [
            ("symbols", &self.syms),
            ("classes", &self.classes),
            ("type parameters", &self.tparams),
            ("aliases", &self.aliases),
            ("packages", &self.pkgs),
            ("overload sets", &self.overloads),
            ("expressions", &self.exprs),
            ("patterns", &self.pats),
            ("strings", &self.strings),
            ("expression lists", &self.expr_lists),
            ("pattern lists", &self.pat_lists),
            ("symbol lists", &self.sym_lists),
            ("statements", &self.stmts),
            ("cases", &self.cases),
            ("tries", &self.tries),
            ("type tests", &self.tests),
            ("functions", &self.funs),
            ("class bodies", &self.tclasses),
            ("quotes", &self.quotes),
            ("quote patterns", &self.quote_pats),
        ]
    }

    #[inline]
    fn sym(&self, s: SymId) -> SymId {
        SymId(self.syms.map(s.0))
    }
    #[inline]
    fn class(&self, c: ClassId) -> ClassId {
        ClassId(self.classes.map(c.0))
    }
    #[inline]
    fn tparam(&self, p: TParamId) -> TParamId {
        TParamId(self.tparams.map(p.0))
    }
    #[inline]
    fn alias(&self, a: AliasId) -> AliasId {
        AliasId(self.aliases.map(a.0))
    }
    #[inline]
    fn pkg(&self, p: PkgId) -> PkgId {
        PkgId(self.pkgs.map(p.0))
    }
    #[inline]
    fn expr(&self, e: TExprId) -> TExprId {
        TExprId(self.exprs.map(e.0))
    }
    #[inline]
    fn pat(&self, p: TPatId) -> TPatId {
        TPatId(self.pats.map(p.0))
    }
    #[inline]
    fn string(&self, s: StrRef) -> StrRef {
        StrRef(self.strings.map(s.0))
    }
    #[inline]
    fn test(&self, t: TestId) -> TestId {
        TestId(self.tests.map(t.0))
    }
    #[inline]
    fn fun(&self, f: FunId) -> FunId {
        FunId(self.funs.map(f.0))
    }
    /// A list is a range of one of the program's list arenas; a chunk's list keeps its length
    /// and moves with the arena.
    #[inline]
    fn list(table: &Table, l: ListRef) -> ListRef {
        if l.len == 0 {
            return l;
        }
        ListRef { start: table.map(l.start), len: l.len }
    }
    #[inline]
    fn expr_list(&self, l: ListRef) -> ListRef {
        Self::list(&self.expr_lists, l)
    }
    #[inline]
    fn sym_list(&self, l: ListRef) -> ListRef {
        Self::list(&self.sym_lists, l)
    }
    #[inline]
    fn pat_list(&self, l: ListRef) -> ListRef {
        Self::list(&self.pat_lists, l)
    }
    #[inline]
    fn stmt_list(&self, l: ListRef) -> ListRef {
        Self::list(&self.stmts, l)
    }
    #[inline]
    fn case_list(&self, l: ListRef) -> ListRef {
        Self::list(&self.cases, l)
    }
}

/// A work item a worker typed and the own records it made: the run of each of its arenas
/// between the two marks (`Worker::own_marks`). The key is the item's place in the sources: the
/// file's id, which is its place in the one-worker walk (`check::file_order` without
/// `TEQ_FILE_ORDER`), then the definition's index in the file; the queue's order, which above one
/// worker is the largest files first, is the schedule's and no part of it.
pub struct ItemRange {
    pub key: (u32, u32),
    pub start: Prefix,
    pub end: Prefix,
}

/// The merge's renumbering, per arena, and what it makes of the types made after the fork
/// (`promote`). The tables map the ids past `base` (a worker's own, from `arena::LOCAL_BASE`
/// after a fork); `walk` says from which id on each arena's records are walked: the shared
/// region's records are walked as well where a body's demand may have filled one with a
/// worker's id (a signature completed lazily, the subclasses of a parent, a package's entries).
/// `checking` is the merge's check over the same walk (`Remap::checking`).
pub struct Remap {
    walk: Prefix,
    /// Where each arena's shared region ends in the merged arenas: the walk takes the work
    /// items' records first, in the items' order, and the shared region's after them, whose
    /// order is the loader's lock's (`Remap::ranges`).
    shared: Prefix,
    types: std::sync::Arc<TypeStore>,
    /// Where the promotion reads the types made after the fork and their parts, when not from
    /// `types`: the merge-order check's shadow store interns apart from the store it reads
    /// (`trace`).
    source: Option<std::sync::Arc<TypeStore>>,
    /// The types the walk's maps hand over, recorded for the merge-order check (`trace`): the
    /// segment under way (the deferred tests', or the maps' after the quote patterns) and each
    /// segment's hand-overs in their order.
    map_roots: Option<std::cell::RefCell<trace::MapRoots>>,
    /// The types from this id on were made after the fork: `ty` promotes them (`promote`).
    types_from: u32,
    promoted: std::cell::RefCell<Promoted>,
    /// The promotion's mapping once the interning is done (`publish`), which the crew reads.
    published: Option<parallel::Published>,
    /// The check's count of what it met, when this remap is the merge's check.
    checking: Option<std::cell::RefCell<Violations>>,
    /// Whether this is the workers' merge: the one that reorders the tables whose values hold
    /// types (`map_sorted`), keeps of the variables' tables what its records reach (`variables`)
    /// and drops a provisional export table; an identity merge (`--profile`, a retype's) leaves
    /// every table as it was.
    forked: bool,
    ids: Tables,
    /// The signatures renumbered so far (`SigMemo`), and the records they were walked in.
    sigs: SigMemo,
    /// The records walked, for the profile, the signatures' apart (`records`).
    walked: u64,
    /// The merge's parts timed (`Clock`), the workers' merge's alone.
    clock: Clock,
}

/// What the merge makes of the types made after the fork, met as the walk hands them over
/// (`Remap::promote`): the base's later growth by its id less `types_from` and each overlay's by
/// its offset, `UNMET` until met, `VISITING` while its parts are, then the id the merge gives it.
/// The closure of the variables the promoted types name, and the counts.
#[derive(Default)]
struct Promoted {
    late: Vec<u32>,
    overlays: Vec<Vec<u32>>,
    /// The one path the merge gives each unshared path's origin (`TypeStore::unshared_origin`).
    unshared: FxMap<u32, TypeId>,
    /// The variables of the other workers' tables the promoted types name, to be kept with
    /// their instances and bounds (`Remap::variables`).
    vars: Vec<TVarId>,
    vars_seen: FxMap<u32, ()>,
    stack: Vec<(TypeId, bool)>,
    parts: Vec<TypeId>,
    pub counts: PromotionCounts,
    /// The time inside `promote`, under `TEQ_WORKERS_TYPES=1` alone (two clock reads a call).
    interning: std::time::Duration,
}

const UNMET: u32 = 0;
const VISITING: u32 = u32::MAX;

impl Promoted {
    /// The slot of a type made after the fork from `from` on, or none for one the merge made
    /// itself (past the base's length when the tables were sized): `UNMET`, `VISITING`, or the
    /// id the merge gives it plus one.
    #[inline]
    fn slot(&mut self, from: u32, t: TypeId) -> Option<&mut u32> {
        match crate::arena::worker_of(t.0) {
            Some(k) => self.overlays.get_mut(k)?.get_mut((t.0 - crate::arena::worker_base(k)) as usize),
            None => self.late.get_mut((t.0 - from) as usize),
        }
    }

    fn set(&mut self, from: u32, t: TypeId, made: TypeId) {
        *self.slot(from, t).expect("a type made after the fork") = made.0 + 1;
    }

    /// What the merge made of `t`: itself for a type the merge made, none while it is unmet or
    /// being made.
    #[inline]
    fn made(&self, from: u32, t: TypeId) -> Option<TypeId> {
        let s = match crate::arena::worker_of(t.0) {
            Some(k) => self.overlays.get(k).and_then(|v| v.get((t.0 - crate::arena::worker_base(k)) as usize)),
            None => self.late.get((t.0 - from) as usize),
        };
        match s {
            None => Some(t),
            Some(&UNMET) | Some(&VISITING) => None,
            Some(&m) => Some(TypeId(m - 1)),
        }
    }

    fn note_var(&mut self, v: TVarId) {
        if self.vars_seen.insert(v.0, ()).is_none() {
            self.vars.push(v);
        }
    }
}

/// What the merge's promotion did, for the measurement (`TEQ_WORKERS_TYPES=1`).
#[derive(Default, Clone, Copy)]
pub(super) struct PromotionCounts {
    /// The overlays' types interned into the base, and of them the ones the base held already.
    pub overlay_types: u64,
    pub found: u64,
    /// The base's types made after the fork that name a worker's arena id, made again; those
    /// met that stand as they are.
    pub late_remade: u64,
    pub late_kept: u64,
    /// The types the base grew by.
    pub appended: u64,
    /// The variables: the merged worker's own, kept whole; the other workers', and of them the
    /// ones kept.
    pub own_vars: u64,
    pub own_kept: u64,
    pub other_vars: u64,
    pub vars_kept: u64,
    /// The promoted types handed to a record that are an overlay's or name a worker's id.
    pub roots_left: u64,
}

/// What the merge's check met (`Remap::checking`): a type made after the fork that is an
/// overlay's or names one, or names a worker's arena id; per arena, a worker's id.
#[derive(Default)]
pub(super) struct Violations {
    pub overlay_types: u64,
    pub local_types: u64,
    pub first: Vec<String>,
    /// The base's types made after the fork whose parts were walked, each once: a bit per type
    /// from the first made after the fork.
    seen: Vec<u64>,
    todo: Vec<TypeId>,
    pub types_checked: u64,
}

impl Violations {
    fn note(&mut self, what: String) {
        if self.first.len() < 8 {
            self.first.push(what);
        }
    }
}

/// What the merge did, for `--profile`.
#[derive(Default, Clone, Copy)]
pub struct MergeStats {
    pub records: u64,
    pub syms: u32,
    pub classes: u32,
    pub exprs: u32,
    pub funs: u32,
    pub tclasses: u32,
    pub names: u32,
    /// The types made during the body phase, and how many of them the chunk's records hold
    /// directly (the signatures of its symbols, the infos of its classes, the types of its
    /// expressions): the bounds of what the parallel typer's merge re-interns.
    pub types_made: u32,
    pub types_held: u32,
}

impl Remap {
    /// The identity: one worker's chunk stays where it is.
    pub fn identity(w: &Worker, p: &Prefix) -> Remap {
        let prog = &w.prog;
        Remap {
            walk: Prefix { ..*p },
            shared: Prefix { ..*p },
            types: w.types.clone(),
            types_from: w.types.len().0 as u32,
            promoted: Default::default(),
            published: None,
            source: None,
            map_roots: None,
            checking: None,
            forked: false,
            ids: Tables {
                syms: Table::identity(p.syms, w.syms.syms.len() - p.syms as usize),
                classes: Table::identity(p.classes, w.syms.classes.len() - p.classes as usize),
                tparams: Table::identity(p.tparams, w.syms.tparams.len() - p.tparams as usize),
                aliases: Table::identity(p.aliases, w.syms.aliases.len() - p.aliases as usize),
                pkgs: Table::identity(p.pkgs, w.syms.pkgs.len() - p.pkgs as usize),
                overloads: Table::identity(p.overloads, w.syms.overloads.len() - p.overloads as usize),
                exprs: Table::identity(p.exprs, prog.exprs.len() - p.exprs as usize),
                pats: Table::identity(p.pats, prog.pats.len() - p.pats as usize),
                strings: Table::identity(p.strings, prog.strings.len() - p.strings as usize),
                expr_lists: Table::identity(p.expr_lists, prog.expr_lists.len() - p.expr_lists as usize),
                pat_lists: Table::identity(p.pat_lists, prog.pat_lists.len() - p.pat_lists as usize),
                sym_lists: Table::identity(p.sym_lists, prog.sym_lists.len() - p.sym_lists as usize),
                stmts: Table::identity(p.stmts, prog.stmts.len() - p.stmts as usize),
                cases: Table::identity(p.cases, prog.cases.len() - p.cases as usize),
                tries: Table::identity(p.tries, prog.tries.len() - p.tries as usize),
                tests: Table::identity(p.tests, prog.tests.len() - p.tests as usize),
                funs: Table::identity(p.funs, prog.funs.len() - p.funs as usize),
                tclasses: Table::identity(p.tclasses, prog.classes.len() - p.tclasses as usize),
                quotes: Table::identity(p.quotes, prog.quotes.len() - p.quotes as usize),
                quote_pats: Table::identity(p.quote_pats, prog.quote_pats.len() - p.quote_pats as usize),
            },
            sigs: SigMemo::default(),
            walked: 0,
            clock: Clock::default(),
        }
    }

    /// The merge's check: the walk of the merge over
    /// every record from the first of each arena, every id kept, a worker's arena id counted where
    /// it is met (`Table::checking`), and every type checked (`check_type`): none an overlay's,
    /// none naming a worker's arena id, and a type the base made after the fork made of base types
    /// alone, part for part. `from` is the first type made after the fork.
    pub(super) fn checking(types: std::sync::Arc<TypeStore>, from: u32) -> Remap {
        let zero = Prefix { syms: 0, classes: 0, tparams: 0, aliases: 0, pkgs: 0, overloads: 0, exprs: 0, pats: 0, strings: 0, expr_lists: 0, pat_lists: 0, sym_lists: 0, stmts: 0, cases: 0, tries: 0, tests: 0, funs: 0, tclasses: 0, quotes: 0, quote_pats: 0, types: 0, names: 0, journal: 0 };
        Remap {
            walk: Prefix { ..zero },
            shared: zero,
            types,
            types_from: from,
            promoted: Default::default(),
            published: None,
            source: None,
            map_roots: None,
            checking: Some(Default::default()),
            forked: false,
            ids: Tables::each(|_| Table::checking()),
            sigs: SigMemo::default(),
            walked: 0,
            clock: Clock::default(),
        }
    }

    /// What the merge's check counted: the overlays' entries, the types naming a worker's arena
    /// id, the workers' ids in records.
    #[cfg(test)]
    pub(super) fn left(&self) -> (u64, u64, u32) {
        let v = self.checking.as_ref().expect("the check").borrow();
        (v.overlay_types, v.local_types, self.ids.all().iter().map(|(_, t)| t.refused()).sum())
    }

    /// The order the walk takes an arena's records in: the work items' (from the shared region's
    /// end), then the shared region's from `walk`. A table shorter than the walk's start (one a
    /// worker emptied after an error, met under `--profile`) has nothing to walk.
    fn ranges(walk: u32, shared: u32, len: usize) -> [std::ops::Range<usize>; 2] {
        let walk = (walk as usize).min(len);
        let shared = (shared as usize).clamp(walk, len);
        [shared..len, walk..shared]
    }

    /// Sizes the promotion's tables for the types made after the fork: the base's later growth
    /// up to its length now and every overlay's.
    fn size_promotion(&mut self) {
        let types = &*self.types;
        let src = self.source.as_deref().unwrap_or(types);
        let p = self.promoted.get_mut();
        let len = types.len().0 as u32;
        p.late = vec![UNMET; len.saturating_sub(self.types_from) as usize];
        p.overlays = src.overlays().iter().map(|o| vec![UNMET; o.lens()[0]]).collect();
    }

    /// What the promotion made of `t`, without promoting it: none while it is unmet.
    pub(super) fn promoted_of(&self, t: TypeId) -> Option<TypeId> {
        if t.0 < self.types_from {
            return Some(t);
        }
        match &self.published {
            Some(p) => p.get(t),
            None => self.promoted.borrow().made(self.types_from, t),
        }
    }

    /// The type under the new ids: itself when it was made before the fork, else what the merge
    /// makes of it (`promote`), or under the merge's check the type itself, checked.
    #[inline]
    pub(super) fn ty(&self, t: TypeId) -> TypeId {
        if t.0 < self.types_from || t == crate::tir::NO_TYPE {
            return t;
        }
        if self.map_roots.is_some() {
            self.map_root(t);
        }
        // A type met before: what the promotion made of it, read without the cell's guard, which
        // no one holds while the walk hands a type over (`promote` reads its parts' itself).
        if self.checking.is_none() {
            if let Some(m) = unsafe { self.promoted.try_borrow_unguarded() }.ok().and_then(|p| p.made(self.types_from, t)) {
                return m;
            }
        }
        self.after_fork(t)
    }

    #[cold]
    #[inline(never)]
    fn after_fork(&self, t: TypeId) -> TypeId {
        match &self.checking {
            Some(v) => {
                self.check_type(&mut v.borrow_mut(), t);
                t
            }
            None => self.promote(t),
        }
    }

    pub(super) fn tlist(&self, types: &TypeStore, l: TList) -> TList {
        if self.checking.is_some() {
            if l.0 >= crate::arena::LOCAL_BASE {
                let mut v = self.checking.as_ref().expect("the check").borrow_mut();
                v.overlay_types += 1;
                v.note(format!("a list of an overlay ({})", l.0));
            }
            for &t in types.items(l) {
                self.ty(t);
            }
            return l;
        }
        let items: Vec<TypeId> = types.items(l).iter().map(|&t| self.ty(t)).collect();
        if l.0 < crate::arena::LOCAL_BASE && items.iter().zip(types.items(l)).all(|(a, b)| a == b) {
            return l;
        }
        types.list(&items)
    }

    /// What the merge makes of `root`, made after the fork, parts first: an overlay's type
    /// interned into the base, a type of the base's later growth
    /// that names a worker's arena id made again under the new ids, any other of the base's as it
    /// is; an unshared path as its origin's one path (`unshared_made`). Each type once, at the first
    /// record that hands it over, so that what the base grows by follows the walk's order and not
    /// the workers'; the variables a promoted type names noted for `variables`.
    #[cold]
    #[inline(never)]
    fn promote(&self, root: TypeId) -> TypeId {
        let started = (self.forked && crate::measure::types_wanted()).then(std::time::Instant::now);
        let m = self.promote_timed(root);
        if let Some(t) = started {
            self.promoted.borrow_mut().interning += t.elapsed();
        }
        m
    }

    fn promote_timed(&self, root: TypeId) -> TypeId {
        let types = &*self.types;
        let src = self.source.as_deref().unwrap_or(types);
        let from = self.types_from;
        let mut cell = self.promoted.borrow_mut();
        let p = &mut *cell;
        if let Some(m) = p.made(from, root) {
            return m;
        }
        p.stack.push((root, false));
        while let Some((t, expanded)) = p.stack.pop() {
            if expanded {
                let before = types.len().0;
                let m = self.make(src, types, p, t);
                // The roots the merge registers, validated as they are made: a base type that
                // names no worker's id (`check_merge`).
                if m.0 >= crate::arena::LOCAL_BASE || types.mentions_local(m) {
                    p.counts.roots_left += 1;
                }
                p.counts.appended += (types.len().0 - before) as u64;
                if t.0 >= crate::arena::LOCAL_BASE {
                    p.counts.overlay_types += 1;
                    p.counts.found += (types.len().0 == before) as u64;
                } else {
                    p.counts.late_remade += 1;
                }
                p.set(from, t, m);
                continue;
            }
            match p.slot(from, t) {
                Some(s) if *s == UNMET => {}
                _ => continue,
            }
            if Self::stands(src, t) {
                p.set(from, t, t);
                p.counts.late_kept += 1;
                continue;
            }
            *p.slot(from, t).expect("a type made after the fork") = VISITING;
            p.stack.push((t, true));
            let mark = p.parts.len();
            src.parts(t, &mut p.parts);
            for i in (mark..p.parts.len()).rev() {
                let q = p.parts[i];
                if q.0 >= from && matches!(p.slot(from, q), Some(s) if *s == UNMET) {
                    p.stack.push((q, false));
                }
            }
            p.parts.truncate(mark);
        }
        p.made(from, root).expect("the promoted type")
    }

    /// Whether a type made after the fork stays what it is through the merge: a base type that
    /// names no worker's arena id and no variable (whose table the merge prunes, `variables`),
    /// whose parts, by the closure of what the base holds, are base types of the same kind.
    fn stands(types: &TypeStore, t: TypeId) -> bool {
        t.0 < crate::arena::LOCAL_BASE && !types.mentions_local(t) && !types.has_vars(t)
    }

    /// `t` made again in the base from its parts' promotions (`promote` made them first), the
    /// arena ids under the new ids, every entry of a sub-store interned in the base with its
    /// value, an unshared path as its origin's (`unshared_made`).
    fn make(&self, src: &TypeStore, types: &TypeStore, p: &mut Promoted, t: TypeId) -> TypeId {
        let from = self.types_from;
        let part = |p: &Promoted, q: TypeId| -> TypeId { if q.0 < from { q } else { p.made(from, q).unwrap_or(q) } };
        let list = |p: &Promoted, l: TList| -> TList {
            let items = src.items(l);
            if l.0 < crate::arena::LOCAL_BASE && items.iter().all(|&q| part(p, q) == q) {
                return l;
            }
            let mapped: Vec<TypeId> = items.iter().map(|&q| part(p, q)).collect();
            types.list(&mapped)
        };
        match src.get(t) {
            Type::Class(c, args) => {
                let l = list(p, args);
                types.mk(Type::Class(self.class(c), l))
            }
            Type::Ctor(c) => types.mk(Type::Ctor(self.class(c))),
            Type::This(c) => types.mk(Type::This(self.class(c))),
            Type::Term(_) if src.is_unshared(t) => self.unshared_made(src, types, p, t),
            Type::Term(s) => types.mk(Type::Term(self.sym(s))),
            Type::Select(q, s) => types.mk(Type::Select(part(p, q), self.sym(s))),
            Type::Member(q, name) => types.mk(Type::Member(part(p, q), name)),
            Type::Param(x) => types.mk(Type::Param(self.tparam(x))),
            Type::AppParam(x, args) => {
                let l = list(p, args);
                types.mk(Type::AppParam(self.tparam(x), l))
            }
            Type::Decl(a) => types.mk(Type::Decl(self.alias(a))),
            Type::Alias(a, args) => {
                let l = list(p, args);
                types.mk(Type::Alias(self.alias(a), l))
            }
            Type::Lambda(ps, b) => {
                let ps = list(p, ps);
                types.mk(Type::Lambda(ps, part(p, b)))
            }
            Type::Poly(ps, b) => {
                let ps = list(p, ps);
                types.mk(Type::Poly(ps, part(p, b)))
            }
            Type::Union(a, b) => types.mk(Type::Union(part(p, a), part(p, b))),
            Type::Inter(a, b) => types.mk(Type::Inter(part(p, a), part(p, b))),
            Type::BoundedWild(lo, hi) => types.mk(Type::BoundedWild(part(p, lo), part(p, hi))),
            Type::AppVar(v, args) => {
                p.note_var(v);
                let l = list(p, args);
                types.mk(Type::AppVar(v, l))
            }
            Type::AppMember(m, args) => {
                let m = part(p, m);
                let l = list(p, args);
                types.mk(Type::AppMember(m, l))
            }
            Type::Refined(q, r) => {
                let q = part(p, q);
                let r = match src.refinement(r) {
                    Refinement::Alias(n, x) => Refinement::Alias(n, part(p, x)),
                    Refinement::Bounds(n, lo, hi) => Refinement::Bounds(n, part(p, lo), part(p, hi)),
                    Refinement::Term(n, s, l) => Refinement::Term(n, self.sym(s), list(p, l)),
                    Refinement::Val(n, s, x) => Refinement::Val(n, self.sym(s), part(p, x)),
                };
                let r = types.refine(r);
                types.mk(Type::Refined(q, r))
            }
            Type::Match(s, m) => {
                let s = part(p, s);
                let info = src.match_info(m).clone();
                let cases: Vec<MatchCase> = info.cases.iter().map(|c| MatchCase { binders: list(p, c.binders), pattern: part(p, c.pattern), body: part(p, c.body) }).collect();
                types.match_type(s, &cases, part(p, info.bound))
            }
            Type::Lit(l) => types.lit(src.lit_val(l)),
            Type::Blocked(b) => {
                let d = src.blocked_description(b).to_string();
                types.blocked(&d)
            }
            Type::Var(v) => {
                p.note_var(v);
                types.mk(Type::Var(v))
            }
            leaf @ (Type::Any | Type::Nothing | Type::Error | Type::Wild) => types.mk(leaf),
        }
    }

    /// The unshared path `t` through the merge: its origin's one path (`TypeStore::unshared_origin`),
    /// made once for the origin and every copy of it: under the symbol's new id where the symbol
    /// is a worker's, else the base's representative of the origin where it holds one (an export
    /// made it), else a path of the base's made for it.
    fn unshared_made(&self, src: &TypeStore, types: &TypeStore, p: &mut Promoted, t: TypeId) -> TypeId {
        let origin = src.unshared_origin(t);
        if let Some(&m) = p.unshared.get(&origin.0) {
            return m;
        }
        let Type::Term(s) = src.get(t) else { unreachable!("an unshared path is a Term") };
        let moved = self.sym(s);
        let m = match src.base_representative(origin) {
            Some(r) if moved == s && !src.mentions_local(r) => r,
            _ => {
                let m = types.term_unshared(moved);
                types.note_unshared_original(m);
                m
            }
        };
        p.unshared.insert(origin.0, m);
        m
    }

    /// The merge's check of a type made after the fork (`checking`): an overlay's id, an id that
    /// names a worker's, and for the base's later growth its parts, each walked once.
    fn check_type(&self, v: &mut Violations, root: TypeId) {
        let types = &*self.types;
        let from = self.types_from;
        let mut todo = std::mem::take(&mut v.todo);
        todo.push(root);
        while let Some(t) = todo.pop() {
            if t.0 < from {
                continue;
            }
            if t.0 >= crate::arena::LOCAL_BASE {
                v.overlay_types += 1;
                v.note(format!("an overlay's type ({})", t.0));
                continue;
            }
            let bit = (t.0 - from) as usize;
            if v.seen.len() <= bit / 64 {
                v.seen.resize(types.len().0.saturating_sub(from as usize) / 64 + 1, 0);
            }
            if v.seen[bit / 64] & 1 << (bit % 64) != 0 {
                continue;
            }
            v.seen[bit / 64] |= 1 << (bit % 64);
            v.types_checked += 1;
            if types.mentions_local(t) {
                v.local_types += 1;
                v.note(format!("a type that names a worker's id ({})", t.0));
            }
            let (mut overlays, mut first) = (0, None);
            types.sub_entries(t, &mut |sub, id| {
                if id >= crate::arena::LOCAL_BASE {
                    overlays += 1;
                    first.get_or_insert((sub, id));
                }
            });
            if let Some((sub, id)) = first {
                v.overlay_types += overlays;
                v.note(format!("an overlay's {} ({}) in type {}", sub.name(), id, t.0));
            }
            types.parts(t, &mut todo);
        }
        v.todo = todo;
    }
    #[inline]
    fn sym(&self, s: SymId) -> SymId {
        self.ids.sym(s)
    }
    #[inline]
    fn class(&self, c: ClassId) -> ClassId {
        self.ids.class(c)
    }
    #[inline]
    fn tparam(&self, p: TParamId) -> TParamId {
        self.ids.tparam(p)
    }
    #[inline]
    fn alias(&self, a: AliasId) -> AliasId {
        self.ids.alias(a)
    }
    #[inline]
    fn pkg(&self, p: PkgId) -> PkgId {
        self.ids.pkg(p)
    }
    #[inline]
    fn expr(&self, e: TExprId) -> TExprId {
        self.ids.expr(e)
    }
    #[inline]
    pub(super) fn pat(&self, p: TPatId) -> TPatId {
        self.ids.pat(p)
    }
    #[inline]
    fn test(&self, t: TestId) -> TestId {
        self.ids.test(t)
    }
    #[inline]
    fn fun(&self, f: FunId) -> FunId {
        self.ids.fun(f)
    }

    /// The signature under the new ids (`records::sig`), with the walk's memo.
    fn sig(&mut self, sig: &mut Arc<MethodSig>) {
        let mut memo = std::mem::take(&mut self.sigs);
        records::sig(&*self, &mut memo, sig);
        self.sigs = memo;
    }

    fn sig_moves(&self, sig: &MethodSig) -> bool {
        records::sig_moves(self, sig)
    }

    /// The records walked, for the profile: the signatures among them.
    pub fn records(&self) -> u64 {
        self.walked + self.sigs.records
    }

    fn env(&self, env: &mut Env) {
        records::env(self, env)
    }

    /// Rebuilds a map keyed by ids where a key moved, and renumbers the values in place.
    fn map<K: Copy + Eq + std::hash::Hash, V>(&self, m: &mut FxMap<K, V>, key: impl Fn(&Self, K) -> K, value: impl Fn(&Self, &mut V)) {
        records::map(self, m, key, value)
    }

    /// `map` for a map whose values hold types: rebuilt in the order of its keys under the new
    /// ids, so that the types its values hand the promotion follow the keys and not the order the
    /// workers' entries hash to (`promote`). Under the merge's check, `map`.
    fn map_sorted<K: Copy + Eq + std::hash::Hash + Ord, V>(&self, m: &mut FxMap<K, V>, key: impl Fn(&Self, K) -> K, value: impl Fn(&Self, &mut V)) {
        if !self.forked || self.checking.is_some() {
            return self.map(m, key, value);
        }
        let mut entries: Vec<(K, V)> = std::mem::take(m).into_iter().map(|(k, v)| (key(self, k), v)).collect();
        entries.sort_unstable_by_key(|e| e.0);
        m.reserve(entries.len());
        for (k, mut v) in entries {
            value(self, &mut v);
            m.insert(k, v);
        }
    }

    /// A map's entries under the new keys, in their order unless this is the merge's check, for
    /// a table outside this module whose values hold types (`map_sorted`).
    pub(super) fn entries_in_order<K: Copy + Ord, V>(&self, m: FxMap<K, V>, key: impl Fn(K) -> K) -> Vec<(K, V)>
    where
        K: Eq + std::hash::Hash,
    {
        let mut entries: Vec<(K, V)> = m.into_iter().map(|(k, v)| (key(k), v)).collect();
        if self.forked && self.checking.is_none() {
            entries.sort_unstable_by_key(|e| e.0);
        }
        entries
    }

    /// The bytes the merge's own tables hold (`TEQ_WORKERS_MEMORY=1`): the arenas' renumbering
    /// and the promotion's dense tables, a word per id or type made after the fork.
    pub(super) fn held(&self) -> usize {
        let p = self.promoted.borrow();
        self.ids.all().iter().map(|(_, t)| t.held()).sum::<usize>() + p.late.capacity() * 4 + p.overlays.iter().map(|o| o.capacity() * 4).sum::<usize>() + self.published.as_ref().map_or(0, |p| p.held())
    }

    /// The types of a register the workers appended to in their order, promoted in the order of
    /// the keys given, before the register is walked (`promote`). Nothing under the merge's check.
    fn promote_in_order<K: Ord, T: IntoIterator<Item = TypeId>>(&self, entries: impl Iterator<Item = (K, T)>) {
        if !self.forked || self.checking.is_some() {
            return;
        }
        let mut entries: Vec<(K, T)> = entries.collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        for (_, types) in entries {
            for t in types {
                self.ty(t);
            }
        }
    }

    fn syms_in(&self, v: &mut Vec<SymId>) {
        records::syms_in(self, v)
    }

    /// Renumbers every record and table of the worker that holds an id of the chunk, promoting
    /// the types made after the fork as the walk meets them (`ty`), the profile's among them,
    /// then the variables' tables (`variables`), whose closure every root the walk handed over
    /// reaches into. Under the merge's check (`checking`) the same walk changes nothing and
    /// counts what it meets.
    pub fn apply(&mut self, w: &mut Worker) {
        if self.checking.is_none() {
            self.size_promotion();
        }
        self.clock.part("walk: the promotion's tables sized");
        self.symbols(w);
        self.clock.part("walk: symbols");
        self.program(w);
        self.clock.part("walk: the program's maps, capture, dependencies, index");
        self.tables(w);
        self.clock.part("walk: the worker's tables");
        w.profile.remap(|s| self.sym(s), |c| self.class(c), |t| self.ty(t), |e| self.expr(e));
        self.clock.part("walk: the profile");
        self.variables(w);
        self.map_segment(None);
        self.clock.part("walk: the variables");
    }

    /// What the variables were solved to and bounded by, which the JVM backend erases a recorded
    /// variable's type through and the interpreter and the capture's finalisation zonk through:
    /// the merged worker's own table whole, `fresh_var` allocating after it; of the other
    /// workers' tables the variables the promoted types name, with what their instances and
    /// bounds name in turn, the rest dropped (`TVars::keep_others`): another worker's variable
    /// that no kept record reaches reads open and unbounded, as it did during the bodies.
    fn variables(&mut self, w: &mut Worker) {
        let prefix = w.tvars_at_fork;
        self.variables_of(&mut w.tvars, prefix);
    }

    /// `variables` over `tvars`, whose first `prefix` variables are the signature phase's, every
    /// worker's: kept with their types promoted. Of the rest the ones the promoted types name, and
    /// what their instances and bounds name in turn; the merged worker's own others keep their
    /// place, read open (`fresh_var` allocates after them), the other workers' go.
    fn variables_of(&mut self, tvars: &mut TVars, prefix: u32) {
        if self.checking.is_some() {
            tvars.each_type(|t| {
                self.ty(t);
            });
            return;
        }
        if !self.forked {
            return;
        }
        let promote = |r: &Self, info: &mut TVarInfo| {
            info.inst = info.inst.map(|t| r.ty(t));
            for t in info.lower.iter_mut().chain(info.upper.iter_mut()) {
                *t = r.ty(*t);
            }
        };
        let own = tvars.len();
        let prefix = (prefix as usize).min(own);
        for i in 0..prefix {
            promote(self, &mut tvars[i]);
        }
        let others = tvars.others_len();
        let mut reached = vec![false; own];
        let mut kept = Vec::new();
        loop {
            let next = self.promoted.borrow_mut().vars.pop();
            let Some(v) = next else { break };
            if tvars.is_own(v) {
                let i = v.index();
                if i >= prefix && !reached[i] {
                    reached[i] = true;
                    promote(self, &mut tvars[v]);
                }
                continue;
            }
            let Some(info) = tvars.other_info(v) else { continue };
            let mut info = info.clone();
            promote(self, &mut info);
            kept.push((v, info));
        }
        let mut own_kept = 0;
        for (i, &r) in reached.iter().enumerate().skip(prefix) {
            if r {
                own_kept += 1;
            } else {
                let info = &mut tvars[i];
                info.inst = None;
                info.lower = Vec::new();
                info.upper = Vec::new();
            }
        }
        let c = &mut self.promoted.get_mut().counts;
        (c.own_vars, c.own_kept, c.other_vars, c.vars_kept) = (own as u64, (prefix + own_kept) as u64, others as u64, kept.len() as u64);
        tvars.keep_others(kept);
    }

    fn symbols(&mut self, w: &mut Worker) {
        let mut memo = std::mem::take(&mut self.sigs);
        let n = w.syms.syms.len();
        for i in Self::ranges(self.walk.syms, self.shared.syms, n).into_iter().flatten() {
            records::symbol(&*self, &mut memo, &mut w.syms.syms[i]);
        }
        self.sigs = memo;
        self.walked += (n - self.walk.syms as usize) as u64;
        let n = w.syms.classes.len();
        for i in 0..n {
            records::subclasses(&*self, &mut w.syms.classes[i]);
        }
        for i in Self::ranges(self.walk.classes, self.shared.classes, n).into_iter().flatten() {
            records::class(&*self, &mut w.syms.classes[i]);
        }
        self.walked += n as u64;
        let n = w.syms.aliases.len();
        let aliases = w.syms.aliases.own_mut();
        for i in Self::ranges(self.walk.aliases, self.shared.aliases, n).into_iter().flatten() {
            records::alias(&*self, &mut aliases[i]);
        }
        self.walked += (n - self.walk.aliases as usize) as u64;
        let n = w.syms.tparams.len();
        let tparams = w.syms.tparams.own_mut();
        for i in Self::ranges(self.walk.tparams, self.shared.tparams, n).into_iter().flatten() {
            records::tparam(&*self, &mut tparams[i]);
        }
        self.walked += (n - self.walk.tparams as usize) as u64;
        let base = self.walk.pkgs as usize;
        for (i, p) in w.syms.pkgs.iter_mut().enumerate() {
            self.walked += records::package(&*self, p, i >= base);
        }
        let base = self.walk.overloads as usize;
        for o in &mut w.syms.overloads.own_mut()[base..] {
            records::overload(&*self, o);
        }
        self.walked += (w.syms.overloads.len() - base) as u64;
        self.map(&mut w.syms.dispatch_names.local, |r, s| r.sym(s), |_, _| {});
    }

    fn program(&mut self, w: &mut Worker) {
        let clock = std::mem::take(&mut self.clock);
        let p = &mut w.prog;
        // In the kinds' order (`parallel::KINDS`): a cast names a type.
        let n = p.exprs.len();
        let mut templates = Vec::new();
        for i in Self::ranges(self.walk.exprs, self.shared.exprs, n).into_iter().flatten() {
            let e = &mut p.exprs[i];
            if let TExpr::Js(..) = e {
                templates.push(i);
            }
            records::texpr(&*self, e);
        }
        self.walked += (n - self.walk.exprs as usize) as u64;
        clock.part("walk: expressions");
        let n = p.pats.len();
        for i in Self::ranges(self.walk.pats, self.shared.pats, n).into_iter().flatten() {
            records::tpat(&*self, &mut p.pats[i]);
        }
        self.walked += (n - self.walk.pats as usize) as u64;
        let base = self.walk.expr_lists as usize;
        for e in &mut p.expr_lists[base..] {
            *e = self.expr(*e);
        }
        self.walked += (p.expr_lists.len() - base) as u64;
        // After the expressions and their lists, whose ids a template's arguments are.
        self.quote_indices(p, &templates);
        let base = self.walk.pat_lists as usize;
        for pat in &mut p.pat_lists[base..] {
            *pat = self.pat(*pat);
        }
        self.walked += (p.pat_lists.len() - base) as u64;
        let base = self.walk.sym_lists as usize;
        for s in &mut p.sym_lists[base..] {
            *s = self.sym(*s);
        }
        self.walked += (p.sym_lists.len() - base) as u64;
        let base = self.walk.stmts as usize;
        for s in &mut p.stmts[base..] {
            records::stmt(&*self, s);
        }
        self.walked += (p.stmts.len() - base) as u64;
        let base = self.walk.cases as usize;
        for c in &mut p.cases[base..] {
            records::case(&*self, c);
        }
        self.walked += (p.cases.len() - base) as u64;
        let base = self.walk.tries as usize;
        for t in &mut p.tries[base..] {
            records::tri(&*self, t);
        }
        self.walked += (p.tries.len() - base) as u64;
        let base = self.walk.tests as usize;
        for t in &mut p.tests[base..] {
            records::type_test(&*self, t);
        }
        self.walked += (p.tests.len() - base) as u64;
        self.deferred_tests(p);
        let base = self.walk.funs as usize;
        for f in &mut p.funs[base..] {
            records::fun(&*self, f);
        }
        self.walked += (p.funs.len() - base) as u64;
        let base = self.walk.tclasses as usize;
        for tc in &mut p.classes[base..] {
            records::tclass(&*self, tc);
        }
        self.walked += (p.classes.len() - base) as u64;
        clock.part("walk: patterns, lists, statements, cases, tests, functions, class bodies");
        self.registers(p);
        clock.part("walk: registers");
        let n = p.expr_types.own().len();
        let recorded = p.expr_types.own_mut();
        for i in Self::ranges(self.walk.exprs, self.shared.exprs, n).into_iter().flatten() {
            recorded[i] = self.ty(recorded[i]);
        }
        clock.part("walk: recorded expression types");
        let n = p.quotes.len();
        for i in Self::ranges(self.walk.quotes, self.shared.quotes, n).into_iter().flatten() {
            records::quote(&*self, &mut p.quotes[i]);
        }
        self.walked += (n - self.walk.quotes as usize) as u64;
        let n = p.quote_pats.len();
        for i in Self::ranges(self.walk.quote_pats, self.shared.quote_pats, n).into_iter().flatten() {
            records::quote_pat(&*self, &mut p.quote_pats[i]);
        }
        self.walked += (n - self.walk.quote_pats as usize) as u64;
        clock.part("walk: quotes and quote patterns");
        self.clock = clock;
        self.program_maps(w);
    }

    /// The deferred type tests' types, in the order of the tests under their new ids
    /// (`map_sorted`): between the patterns and the recorded types in the walk's order.
    fn deferred_tests(&mut self, p: &mut Program) {
        self.map_segment(Some(0));
        self.map_sorted(&mut p.deferred_tests, |r, t| r.test(t), |r, ty| *ty = r.ty(*ty));
        self.map_segment(None);
    }

    /// The registers: what the walk added stands anywhere in them.
    fn registers(&mut self, p: &mut Program) {
        for f in p.top_funs.own_mut() {
            *f = self.fun(*f);
        }
        for (s, e) in p.top_vals.own_mut() {
            *s = self.sym(*s);
            *e = self.expr(*e);
        }
        p.main = p.main.map(|s| self.sym(s));
        p.main_object = p.main_object.map(|c| self.class(c));
        for (s, _) in &mut p.js_exports {
            *s = self.sym(*s);
        }
        for (s, _) in &mut p.template_calls {
            *s = self.sym(*s);
        }
        self.map(&mut p.template_syms.local, |r, s| r.ids.string(s), |r, s| *s = r.sym(*s));
        self.walked += (p.top_funs.own().len() + p.top_vals.own().len() + p.js_exports.len() + p.template_calls.len()) as u64;
    }

    /// The program's maps, the capture's tables, the dependency recorder's and the navigation
    /// index's: after the quote patterns in the walk's order.
    fn program_maps(&mut self, w: &mut Worker) {
        // The maps the walk takes after the quote patterns, to its end (`map_segment`).
        self.map_segment(Some(1));
        let p = &mut w.prog;
        self.map(&mut p.overrides.local, |r, s| r.sym(s), |r, ps| r.syms_in(ps));
        self.map(&mut p.expansions, |r, e| r.expr(e), |r, x| x.callee = r.sym(x.callee));
        self.map(&mut p.leaf_tests, |r, t| r.test(t), |_, _| {});
        self.map(&mut p.stored_tests, |r, t| r.test(t), |_, _| {});
        self.map(&mut p.stored_pats, |r, t| r.pat(t), |_, _| {});
        self.walked += (p.expansions.len() + p.leaf_tests.len()) as u64;
        if let Some(c) = p.capture.as_deref_mut() {
            self.capture(&w.types, c);
        }
        if let Some(d) = w.deps.as_deref_mut() {
            d.remap(|s| self.sym(s), |c| self.class(c), |a| self.alias(a), |p| self.pkg(p), |e| self.expr(e));
        }
        if let Some(ix) = w.index.as_deref_mut() {
            ix.remap(&|s| self.sym(s), &|c| self.class(c), &|a| self.alias(a), &|p| self.tparam(p), &|e| self.expr(e));
        }
    }

    /// The quotes' and the quote patterns' indices, which the `$quote` and `$quoteMatch`
    /// templates carry as `Int` arguments (`quoted.rs`, `quote_template` and
    /// `quote_match_pattern`), under the arenas' new ids: of the templates the walk met.
    fn quote_indices(&self, p: &mut Program, templates: &[usize]) {
        for &i in templates {
            let TExpr::Js(s, l) = p.exprs[i] else { continue };
            let (table, at) = match p.strings[s.idx()].as_str() {
                "$quote" => (&self.ids.quotes, 0),
                "$quoteMatch" => (&self.ids.quote_pats, 1),
                _ => continue,
            };
            let Some(&arg) = p.expr_list(l).get(at) else { continue };
            if let TExpr::Int(q) = p.exprs[arg.idx()] {
                p.exprs[arg.idx()] = TExpr::Int(table.map(q as u32) as i32);
            }
        }
    }

    fn inline_definition(&self, d: &mut InlineDefinition) {
        d.body = d.body.map(|e| self.expr(e));
        d.ty = self.ty(d.ty);
        self.syms_in(&mut d.params);
        for e in d.defaults.iter_mut().flatten() {
            *e = self.expr(*e);
        }
        self.syms_in(&mut d.binders);
        for p in &mut d.pattern_tparams {
            *p = self.tparam(*p);
        }
        for list in [&mut d.reducible, &mut d.deferred, &mut d.splices, &mut d.leaves] {
            for e in list.iter_mut() {
                *e = self.expr(*e);
            }
        }
        for (e, ts) in &mut d.type_args {
            *e = self.expr(*e);
            for t in ts.iter_mut() {
                *t = self.ty(*t);
            }
        }
        for t in &mut d.leaf_tests {
            *t = self.test(*t);
        }
        self.syms_in(&mut d.hoisted);
        for e in d.widened.iter_mut().chain(d.opaque.iter_mut()).chain(d.spread.iter_mut()) {
            *e = self.expr(*e);
        }
        for tc in &mut d.classes {
            records::tclass(self, tc);
        }
        self.syms_in(&mut d.inline_vals);
        self.syms_in(&mut d.inferred_vals);
        for a in &mut d.aliases {
            a.block = self.expr(a.block);
            a.local = self.sym(a.local);
            a.tree = self.expr(a.tree);
        }
        for i in &mut d.imports {
            i.block = self.expr(i.block);
            let mut env = Env { file: FileId(0), frames: Vec::new(), imports: vec![i.import] };
            self.env(&mut env);
            i.import = env.imports[0];
        }
        if !d.node_types.is_empty() {
            self.map_sorted(&mut d.node_types, |r, e| r.expr(e), |r, t| *t = r.ty(*t));
        }
    }

    /// The worker's tables: the part that hands types over, in the walk's order, then the
    /// id-only maps (`untyped_tables`), which the parallel merge's crew takes apart after the
    /// interning (`parallel::renumber`) and the serial walk takes here.
    fn tables(&mut self, w: &mut Worker) {
        self.tables_typed(w);
        let mode = self.tables_mode();
        for task in untyped_tables(w, mode) {
            task(&*self);
        }
    }

    /// How the id-only tables are taken: whether this is the workers' merge (a provisional export
    /// table dropped, a file's opaque types sorted) and whether it is the merge's check.
    fn tables_mode(&self) -> TablesMode {
        TablesMode { forked: self.forked && self.checking.is_none(), checking: self.checking.is_some() }
    }

    /// The worker's tables whose values hold types, in the walk's order (the deferred bounds and
    /// matches by their sites, the sorted maps, the inline definitions, the builtins' types and
    /// the quote state's deferred calls), and the builtins' and the sites' classes beside them.
    fn tables_typed(&mut self, w: &mut Worker) {
        self.promote_in_order(w.deferred_bounds.iter().enumerate().map(|(i, b)| ((b.site(), i), b.types().collect::<Vec<_>>())));
        for b in &mut w.deferred_bounds {
            b.remap(|c| self.class(c), |t| self.ty(t));
            b.definition = b.definition.map(|s| self.sym(s));
        }
        self.promote_in_order(w.deferred_matches.iter().enumerate().map(|(i, m)| ((m.file, m.span.start, m.span.end, i), std::iter::once(m.sty).chain(m.gadt.iter().map(|g| g.1)).collect::<Vec<_>>())));
        for m in &mut w.deferred_matches {
            m.sty = self.ty(m.sty);
            m.definition = m.definition.map(|s| self.sym(s));
            for c in &mut m.cases {
                c.pat = self.pat(c.pat);
                c.guard = c.guard.map(|g| self.expr(g));
                c.body = self.expr(c.body);
            }
            for c in &mut m.transparent {
                *c = self.class(*c);
            }
            for g in &mut m.gadt {
                *g = (self.tparam(g.0), self.ty(g.1), g.2);
            }
        }
        self.map_sorted(&mut w.outer_prefixes, |r, e| r.expr(e), |r, v| {
            for (c, t) in v {
                *c = r.class(*c);
                *t = r.ty(*t);
            }
        });
        self.map_sorted(&mut w.captured_locals, |r, s| r.sym(s), |r, t| *t = r.ty(*t));
        self.map_sorted(&mut w.unchecked_variance.local, |_, k| k, |r, t| *t = r.ty(*t));
        self.map_sorted(&mut w.inline_definitions.local, |r, s| r.sym(s), |r, d| r.inline_definition(Arc::make_mut(d)));
        for d in &mut w.superseded_definitions {
            self.inline_definition(Arc::make_mut(d));
        }
        self.map(&mut w.inline.evaluated, |r, e| r.expr(e), |_, _| {});
        self.map_sorted(&mut w.inline.args, |r, s| r.sym(s), |r, a| {
            a.expr = r.expr(a.expr);
            a.ty = r.ty(a.ty);
        });
        // The types the std's classes give, which a worker finds on its way (`find_std_classes`).
        for t in [&mut w.b.t_product, &mut w.b.t_equals, &mut w.b.t_enum, &mut w.b.t_singleton] {
            *t = self.ty(*t);
        }
        w.b.repeated = w.b.repeated.map(|c| self.class(c));
        w.quote.remap(self);
        // The classes the bodies made on first use, which the builtins name.
        let b = &mut w.b;
        for c in b.functions.iter_mut().chain(b.context_functions.iter_mut()).chain(b.tuples.iter_mut()) {
            *c = c.map(|c| self.class(c));
        }
        for c in [
            &mut b.by_name,
            &mut b.seq,
            &mut b.string_context,
            &mut b.partial_function,
            &mut b.conversion,
            &mut b.sub_evidence,
            &mut b.eq_evidence,
            &mut b.option,
            &mut b.named_tuple,
            &mut b.either,
            &mut b.set,
            &mut b.map,
            &mut b.can_equal,
            &mut b.value_of,
            &mut b.selectable,
            &mut b.number,
            &mut b.js_dynamic,
            &mut b.throwable,
            &mut b.js_exception,
            &mut b.cons_tuple,
            &mut b.product,
            &mut b.reflect_enum,
            &mut b.tuple_trait,
            &mut b.non_empty_tuple,
            &mut b.empty_tuple,
        ] {
            *c = c.map(|c| self.class(c));
        }
        for (_, s) in &mut b.product_helpers {
            *s = self.sym(*s);
        }
        w.sites.class_tag = w.sites.class_tag.map(|c| self.class(c));
        w.sites.not_given_class = w.sites.not_given_class.map(|c| self.class(c));
        w.unit_ops = w.unit_ops.map(|o| o.map(|c| self.class(c)));
        w.prog.partial_function = w.prog.partial_function.map(|c| self.class(c));
        w.prog.throwable = w.prog.throwable.map(|c| self.class(c));
        w.prog.js_exception = w.prog.js_exception.map(|c| self.class(c));
    }

    /// The quote state's renumbering, called from `quoted.rs` with its private tables.
    pub(super) fn deferred_inline(&mut self, d: &mut Arc<super::quoted::DeferredInline>) {
        let moves = self.sym(d.sym) != d.sym
            || self.sig_moves(&d.sig)
            || d.owner_subst.iter().chain(d.subst.iter()).any(|&(p, t)| self.tparam(p) != p || self.ty(t) != t)
            || self.ty(d.ret_ty) != d.ret_ty
            || d.prefix.map_or(false, |t| self.ty(t) != t)
            || d.expected.map_or(false, |t| self.ty(t) != t);
        if !moves {
            return;
        }
        let mut moved = (**d).clone();
        moved.sym = self.sym(moved.sym);
        self.sig(&mut moved.sig);
        for (p, t) in moved.owner_subst.iter_mut().chain(moved.subst.iter_mut()) {
            *p = self.tparam(*p);
            *t = self.ty(*t);
        }
        moved.ret_ty = self.ty(moved.ret_ty);
        moved.prefix = moved.prefix.map(|t| self.ty(t));
        moved.expected = moved.expected.map(|t| self.ty(t));
        *d = Arc::new(moved);
    }

    pub(super) fn map_expr(&self, e: TExprId) -> TExprId {
        self.expr(e)
    }

    pub(super) fn map_sym(&self, s: SymId) -> SymId {
        self.sym(s)
    }

    pub(super) fn map_class(&self, c: ClassId) -> ClassId {
        self.class(c)
    }

    /// The types made during the body phase, and how many of them the chunk's records name
    /// directly (the result and parameter types of its symbols' signatures, its classes'
    /// parents, base types, self and underlying types and constructor parameters, its type
    /// parameters' bounds, its aliases' right-hand sides and bounds, the recorded types of its
    /// expressions and the types in its patterns; not the quotes' types nor the deferred
    /// tests, and nothing of what those types are made of), under `--profile`. The first is the
    /// most the parallel typer's merge could re-intern, the second the roots of the graph it
    /// walks, not the graph.
    pub fn count_types(&self, w: &Worker, p: &Prefix) -> (u32, u32) {
        let made = w.types.len().0 as u32 - p.types;
        let mut held = vec![false; made as usize];
        let mut note = |t: TypeId| {
            if t.0 >= p.types && t.0 < p.types + made {
                held[(t.0 - p.types) as usize] = true;
            }
        };
        for s in &w.syms.syms.own()[p.syms as usize..] {
            if let Some(sig) = &s.sig {
                note(sig.ret);
                for c in &sig.clauses {
                    for param in &c.params {
                        note(param.ty);
                    }
                }
            }
        }
        for c in &w.syms.classes.own()[p.classes as usize..] {
            for &t in &c.parents {
                note(t);
            }
            for &(_, t) in &c.base_types {
                note(t);
            }
            for t in [c.underlying, c.declared_self, c.this_type].into_iter().flatten() {
                note(t);
            }
            for clause in &c.ctor {
                for param in &clause.params {
                    note(param.ty);
                }
            }
        }
        for tp in &w.syms.tparams.own()[p.tparams as usize..] {
            note(tp.upper);
            note(tp.lower);
        }
        for a in &w.syms.aliases.own()[p.aliases as usize..] {
            note(a.rhs);
            if let Some((l, u)) = a.bounds {
                note(l);
                note(u);
            }
        }
        for &t in w.prog.expr_types.own().iter().skip(p.exprs as usize) {
            if t != NO_TYPE {
                note(t);
            }
        }
        for pat in &w.prog.pats[p.pats as usize..] {
            match *pat {
                TPat::Test(_, t, _) | TPat::Class(_, t, _, _) => note(t),
                _ => {}
            }
        }
        for e in &w.prog.exprs[p.exprs as usize..] {
            if let TExpr::Cast(_, _, t) = *e {
                note(t);
            }
        }
        (made, held.iter().filter(|&&h| h).count() as u32)
    }
}

impl Mapping for Remap {
    #[inline]
    fn ids(&self) -> &Tables {
        &self.ids
    }
    #[inline]
    fn ty(&self, t: TypeId) -> TypeId {
        Remap::ty(self, t)
    }
}

/// How the id-only tables are taken (`Remap::tables_mode`).
#[derive(Clone, Copy)]
pub(super) struct TablesMode {
    /// The workers' merge: a provisional export table dropped, a file's opaque types sorted.
    forked: bool,
    /// The merge's check: an export table's references counted, nothing changed.
    checking: bool,
}

/// A task of the id-only tables (`untyped_tables`), run with the merge's mapping.
pub(super) type TableTask<'w> = Box<dyn FnOnce(&dyn Mapping) + Send + 'w>;

/// The worker's tables none of whose entries hands a type over, renumbered by the arenas' new ids
/// alone: tasks over disjoint fields, in no order of their own, which the parallel merge's crew
/// runs after the interning and the serial walk runs in turn (`Remap::tables`).
pub(super) fn untyped_tables<'w>(w: &'w mut Worker, mode: TablesMode) -> Vec<TableTask<'w>> {
    use records::{map, syms_in};
    let mut tasks: Vec<TableTask<'w>> = Vec::new();
    let (fun_of_sym, val_init, interpolations, soft_exprs, soft_syms, class_done) = (&mut w.fun_of_sym.local, &mut w.val_init.local, &mut w.interpolations, &mut w.soft_exprs, &mut w.soft_syms, &mut w.class_done.local);
    tasks.push(Box::new(move |m| {
        map(m, fun_of_sym, |r, s| r.sym(s), |r, f| *f = r.fun(*f));
        map(m, val_init, |r, s| r.sym(s), |r, e| *e = r.expr(*e));
        map(m, interpolations, |r, e| r.expr(e), |_, _| {});
        map(m, soft_exprs, |r, e| r.expr(e), |_, _| {});
        map(m, soft_syms, |r, s| r.sym(s), |_, _| {});
        map(m, class_done, |r, c| r.class(c), |_, _| {});
    }));
    let (class_imports, anon_envs, anon_captures, anon_parent_args, inferred_parent_args) = (&mut w.class_imports.local, &mut w.anon_envs, &mut w.anon_captures, &mut w.anon_parent_args, &mut w.inferred_parent_args.local);
    let inferred_trait_args = &mut w.inferred_trait_args.local;
    let (local_news, outer_this, sam_classes, sam_arity, entry_points) = (&mut w.local_news, &mut w.outer_this.local, &mut w.sam_classes, &mut w.sam_arity, &mut w.entry_points);
    tasks.push(Box::new(move |m| {
        map(m, class_imports, |r, c| r.class(c), |r, imports| {
            if imports.iter().any(|(_, i)| records::env_moves(r, &Env { file: FileId(0), frames: Vec::new(), imports: vec![*i] })) {
                let mut moved: Vec<(u32, ImportTarget)> = Vec::new();
                let mut env = Env { file: FileId(0), frames: Vec::new(), imports: imports.iter().map(|(_, i)| *i).collect() };
                records::env(r, &mut env);
                for ((at, _), i) in imports.iter().zip(env.imports.iter()) {
                    moved.push((*at, i.target));
                }
                let renumbered: Vec<(u32, ResolvedImport)> = imports.iter().zip(moved).map(|((_, i), (at, target))| (at, ResolvedImport { target, ..*i })).collect();
                *imports = Arc::from(renumbered);
            }
        });
        map(m, anon_envs, |r, c| r.class(c), |r, env| {
            if records::env_moves(r, env) {
                let mut moved = (**env).clone();
                records::env(r, &mut moved);
                *env = Arc::new(moved);
            }
        });
        map(m, anon_captures, |r, c| r.class(c), |r, syms| syms_in(r, syms));
        map(m, anon_parent_args, |r, c| r.class(c), |r, pc| records::parent_call(r, pc));
        map(m, inferred_parent_args, |r, c| r.class(c), |r, pc| records::parent_call(r, pc));
        map(m, inferred_trait_args, |r, c| r.class(c), |r, calls| {
            let mut moved = calls.to_vec();
            for (t, pc) in moved.iter_mut() {
                *t = r.class(*t);
                records::parent_call(r, pc);
            }
            *calls = std::sync::Arc::from(moved);
        });
        for (c, e, _, captures) in local_news.iter_mut() {
            *c = m.class(*c);
            *e = m.expr(*e);
            for (s, e) in captures {
                *s = m.sym(*s);
                *e = m.expr(*e);
            }
        }
        map(m, outer_this, |r, c| r.class(c), |r, s| *s = r.sym(*s));
        map(m, sam_classes, |r, c| r.class(c), |_, _| {});
        map(m, sam_arity, |r, (c, n)| (r.class(c), n), |_, _| {});
        for (s, c) in entry_points.iter_mut() {
            *s = m.sym(*s);
            *c = c.map(|c| m.class(c));
        }
    }));
    let (override_pairs, late_methods, inherited_pairs, member_cycles, unsettled_std_entries) = (&mut w.override_pairs, &mut w.late_methods, &mut w.inherited_pairs, &mut w.member_cycles, &mut w.unsettled_std_entries);
    let (super_problems_told, inference_failed, mixin_supers, any_members, dispatch_pending, named_like, suffixed_roots) = (&mut w.super_problems_told, &mut w.inference_failed, &mut w.mixin_supers, &mut w.any_members.local, &mut w.dispatch_pending, &mut w.named_like, &mut w.suffixed_roots);
    tasks.push(Box::new(move |m| {
        for (a, p) in override_pairs.iter_mut() {
            *a = m.sym(*a);
            *p = m.sym(*p);
        }
        for (c, f) in late_methods.iter_mut() {
            *c = m.class(*c);
            *f = m.fun(*f);
        }
        for (c, _, a, b, _) in inherited_pairs.iter_mut() {
            *c = m.class(*c);
            *a = m.class(*a);
            *b = m.class(*b);
        }
        for (c, _) in member_cycles.iter_mut() {
            *c = m.class(*c);
        }
        for (c, s) in unsettled_std_entries.iter_mut() {
            *c = m.class(*c);
            *s = m.sym(*s);
        }
        map(m, super_problems_told, |r, (c, s)| (r.class(c), r.sym(s)), |_, _| {});
        map(m, inference_failed, |r, s| r.sym(s), |_, _| {});
        map(m, mixin_supers, |r, c| r.class(c), |r, syms| syms_in(r, syms));
        map(m, any_members, |r, (c, n)| (r.class(c), n), |r, s| *s = r.sym(*s));
        syms_in(m, dispatch_pending);
        for (a, b) in named_like.iter_mut() {
            *a = m.sym(*a);
            *b = m.sym(*b);
        }
        map(m, suffixed_roots, |r, s| r.sym(s), |_, _| {});
    }));
    let (expr_marks, eta_expansions, folded_paths, irrefutable_pats) = (&mut w.expr_marks, &mut w.eta_expansions, &mut w.folded_paths, &mut w.irrefutable_pats);
    tasks.push(Box::new(move |m| {
        map(m, expr_marks, |r, e| r.expr(e), |_, _| {});
        map(m, eta_expansions, |r, e| r.expr(e), |_, _| {});
        map(m, folded_paths, |r, e| r.expr(e), |r, (recv, s)| {
            *recv = r.expr(*recv);
            *s = r.sym(*s);
        });
        map(m, irrefutable_pats, |r, p| r.pat(p), |_, _| {});
    }));
    let (poisoned, poisoned_withheld, outer_accessors, merged_overloads, derived_aliases, derived_opaques) = (&mut w.poisoned, &mut w.poisoned_withheld, &mut w.outer_accessors.local, &mut w.merged_overloads.local, &mut w.derived_aliases, &mut w.derived_opaques.local);
    let withheld_notes = &mut w.withheld_notes;
    tasks.push(Box::new(move |m| {
        map(m, poisoned, |r, f| r.fun(f), |_, _| {});
        map(m, poisoned_withheld, |r, f| r.fun(f), |_, _| {});
        map(m, withheld_notes, |r, e| r.expr(e), |_, _| {});
        map(m, outer_accessors, |r, c| r.class(c), |r, s| *s = r.sym(*s));
        map(m, merged_overloads, |r, (a, b)| (r.sym(a), r.sym(b)), |r, s| *s = r.sym(*s));
        map(m, derived_aliases, |r, (a, t)| (r.alias(a), t), |r, a| *a = r.alias(*a));
        map(m, derived_opaques, |r, (c, k)| (r.class(c), r.class(k)), |r, c| *c = r.class(*c));
    }));
    // The program files' definition maps: the local and anonymous classes and the local
    // definitions a body enters under its file.
    let def_syms = w.def_syms.own_mut();
    tasks.push(Box::new(move |m| {
        for f in def_syms.iter_mut() {
            for s in f.values_mut() {
                *s = m.sym(*s);
            }
        }
    }));
    let def_classes = w.def_classes.own_mut();
    tasks.push(Box::new(move |m| {
        for f in def_classes.iter_mut() {
            for c in f.values_mut() {
                *c = m.class(*c);
            }
        }
    }));
    let def_aliases = w.def_aliases.own_mut();
    tasks.push(Box::new(move |m| {
        for f in def_aliases.iter_mut() {
            for a in f.values_mut() {
                *a = m.alias(*a);
            }
        }
    }));
    let (reflect_ctor_params, reflect_param_of, retained_bodies, check_capture_keys, renamed_roots) = (&mut w.reflect_ctor_params, &mut w.reflect_param_of, &mut w.retained_bodies.local, &mut w.check_capture_keys, &mut w.renamed_roots);
    let (import_values, file_opaques) = (&mut w.import_values, w.file_opaques.own_mut());
    tasks.push(Box::new(move |m| {
        map(m, reflect_ctor_params, |r, s| r.sym(s), |r, s| *s = r.sym(*s));
        map(m, reflect_param_of, |r, s| r.sym(s), |r, s| *s = r.sym(*s));
        map(m, retained_bodies, |r, s| r.sym(s), |_, _| {});
        for k in check_capture_keys.iter_mut() {
            use crate::tir::capture::Key;
            *k = match *k {
                Key::Targs(e) => Key::Targs(m.expr(e)),
                Key::Form(e) => Key::Form(m.expr(e)),
                Key::Wrap(e) => Key::Wrap(m.expr(e)),
                Key::Inline(e) => Key::Inline(m.expr(e)),
                Key::Builtin(e) => Key::Builtin(m.expr(e)),
                Key::Receiver(e) => Key::Receiver(m.expr(e)),
                Key::Evidence(e) => Key::Evidence(m.expr(e)),
                Key::Tree(e) => Key::Tree(m.expr(e)),
                Key::Pat(p) => Key::Pat(m.pat(p)),
                Key::Local(s) => Key::Local(m.sym(s)),
                Key::Class(c) => Key::Class(m.class(c)),
                Key::BlockClasses(e) => Key::BlockClasses(m.expr(e)),
            };
        }
        syms_in(m, renamed_roots);
        // The third component is an index into the table itself, which every worker shares.
        for i in 0..import_values.len() {
            let (s, c, on) = *import_values.get(i);
            let moved = (m.sym(s), c.map(|c| m.class(c)), on);
            if (moved.0, moved.1) != (s, c) {
                unsafe { *import_values.get_mut(i) = moved };
            }
        }
        for opaques in file_opaques.iter_mut() {
            for c in opaques.iter_mut() {
                *c = m.class(*c);
            }
            if mode.forked {
                opaques.sort();
                opaques.dedup();
            }
        }
    }));
    let classes = w.class_exports.own_mut();
    tasks.push(Box::new(move |m| map(m, classes, |r, c| r.class(c), |r, s| export_slot(r, s, mode))));
    let pkgs = w.pkg_exports.own_mut();
    tasks.push(Box::new(move |m| map(m, pkgs, |r, p| r.pkg(p), |r, s| export_slot(r, s, mode))));
    let (derive, reported) = (&mut w.derive, &mut w.matches.reported);
    tasks.push(Box::new(move |m| {
        derive.remap(|s| m.sym(s), |c| m.class(c));
        for a in reported.iter_mut() {
            *a = m.alias(*a);
        }
    }));
    tasks
}

/// An export table (`exports.rs`) under the new ids: its references renumbered, a provisional
/// one, never final, gone in the workers' merge (its lookups made again); under the merge's check
/// its references counted.
fn export_slot(m: &dyn Mapping, s: &mut super::exports::ExportSlot, mode: TablesMode) {
    use super::exports::ExportSlot;
    if let ExportSlot::Provisional(..) = s {
        if mode.forked {
            *s = ExportSlot::Unknown;
        }
    }
    match s {
        ExportSlot::Ready(scope) | ExportSlot::Provisional(Some(scope), _) => {
            if mode.checking {
                scope.each_ref(
                    &mut |s| {
                        m.sym(s);
                    },
                    &mut |c| {
                        m.class(c);
                    },
                    &mut |a| {
                        m.alias(a);
                    },
                    &mut |p| {
                        m.tparam(p);
                    },
                    &mut |p| {
                        m.pkg(p);
                    },
                );
            } else {
                Arc::make_mut(scope).remap(&|s| m.sym(s), &|c| m.class(c), &|a| m.alias(a), &|p| m.tparam(p), &|p| m.pkg(p));
            }
        }
        _ => {}
    }
    if let ExportSlot::Provisional(_, awaited) = s {
        for c in awaited.iter_mut() {
            *c = m.class(*c);
        }
    }
}

/// Whether the merge's check sweeps every record (`Worker::check_merge`): in the
/// assertion-enabled builds always, in the others under `TEQ_MERGE_SWEEP=1` (the determinism
/// suites'), where the roots the promotion hands the records are validated as they are handed
/// over and the sweep's walk, some 40 ms on the frontend, is not paid.
fn sweep_wanted() -> bool {
    cfg!(debug_assertions) || std::env::var_os("TEQ_MERGE_SWEEP").is_some_and(|v| v == "1")
}

/// The merge's parts timed for `--time` (`measure::merge_part`), each from the last one's end.
#[derive(Default)]
pub(super) struct Clock(std::cell::Cell<Option<std::time::Instant>>);

impl Clock {
    fn start(on: bool) -> Clock {
        Clock(std::cell::Cell::new((on && crate::measure::on()).then(std::time::Instant::now)))
    }

    fn part(&self, name: &'static str) {
        if let Some(t) = self.0.get() {
            let now = std::time::Instant::now();
            crate::measure::merge_part(name, now - t);
            self.0.set(Some(now));
        }
    }
}

/// Takes every worker's own records of the arenas named, into a vector per name (the names
/// are the caller's, so that `merge_arenas` finds them).
macro_rules! take_arenas {
    ($self:ident, $others:ident; $($name:ident: $($path:ident).+),*) => {
        $(let mut $name = vec![$self.$($path).+.take_own()]; for o in $others.iter_mut() { $name.push(o.as_mut().map_or_else(Vec::new, |o| o.$($path).+.take_own())); })*
    };
}

/// The shared region's retained records of each arena named, beside its own records
/// `take_arenas` took, each name bound to the pair (the retention).
macro_rules! retain_arenas {
    ($self:ident, $crew:ident; $($name:ident: $($path:ident).+),*) => {
        $(let $name = {
            let records = $self.$($path).+.retained(|_| false, $crew);
            ($name, records)
        };)*
    };
}

/// Places the records `retain_arenas` bound by the arenas' numberings (`NUMBERED`'s order) and
/// leaves each name bound to the new ids, by worker and own index.
macro_rules! place_arenas {
    ($self:ident, $numbering:ident, $crew:ident; $($name:ident: $($path:ident).+ = $i:expr),*) => {
        $(let $name = {
            let (owns, records) = $name;
            $self.$($path).+.place(records, owns, &$numbering[$i], $crew)
        };)*
    };
}

/// The arenas the merge numbers, by their places in `Prefix`, in the order of `numberings`.
const NUMBERED: [fn(&Prefix) -> u32; 20] = [
    |p| p.syms,
    |p| p.classes,
    |p| p.tparams,
    |p| p.aliases,
    |p| p.pkgs,
    |p| p.overloads,
    |p| p.exprs,
    |p| p.pats,
    |p| p.strings,
    |p| p.expr_lists,
    |p| p.pat_lists,
    |p| p.sym_lists,
    |p| p.stmts,
    |p| p.cases,
    |p| p.tries,
    |p| p.tests,
    |p| p.funs,
    |p| p.tclasses,
    |p| p.quotes,
    |p| p.quote_pats,
];

/// Every arena's numbering (`NUMBERED`'s order): the items' segments in their canonical order
/// (`order`), every arena's taken at once by each thread of the crew over its share of the items,
/// and each worker's in its own order (`items`, the order it typed them), over the retained shared
/// length and the workers' counts of own records in `sizes`; each arena's runs a task of the crew.
fn numberings(items: &[Vec<ItemRange>], order: &[((u32, u32), u32, u32)], sizes: &[(u32, Vec<usize>)], crew: &crate::crew::Crew) -> Vec<crate::arena::Numbering> {
    use crate::arena::Segment;
    let shares: Vec<std::sync::Mutex<Vec<Vec<Segment>>>> = (0..crew.threads()).map(|_| std::sync::Mutex::new(Vec::new())).collect();
    crew.run(&|t| {
        let mine = crew.share(t, 0..order.len());
        let mut segments: Vec<Vec<Segment>> = (0..NUMBERED.len()).map(|_| Vec::with_capacity(mine.len())).collect();
        for &(_, k, i) in &order[mine] {
            let r = &items[k as usize][i as usize];
            for (a, of) in NUMBERED.iter().enumerate() {
                let (start, end) = (of(&r.start), of(&r.end));
                if start < end {
                    segments[a].push(Segment { worker: k as usize, start, end });
                }
            }
        }
        *shares[t].lock().unwrap_or_else(|e| e.into_inner()) = segments;
    });
    let shares: Vec<Vec<Vec<Segment>>> = shares.into_iter().map(|s| s.into_inner().unwrap_or_else(|e| e.into_inner())).collect();
    let out: Vec<std::sync::Mutex<Option<crate::arena::Numbering>>> = (0..sizes.len()).map(|_| std::sync::Mutex::new(None)).collect();
    let shares = &shares;
    crew.each(sizes.len(), &|a| {
        let of = NUMBERED[a];
        let segments: Vec<Segment> = shares.iter().flat_map(|s| s[a].iter().copied()).collect();
        let covered: Vec<Vec<(u32, u32)>> = items.iter().map(|v| v.iter().map(|r| (of(&r.start), of(&r.end))).filter(|&(s, e)| s < e).collect()).collect();
        *out[a].lock().unwrap_or_else(|e| e.into_inner()) = Some(crate::arena::Numbering::of(sizes[a].0, &segments, covered, &sizes[a].1));
    });
    out.into_iter().map(|n| n.into_inner().unwrap_or_else(|e| e.into_inner()).expect("a numbering")).collect()
}

/// The class bodies' retention: a body of the shared region, typed under the lock for a demand
/// of a class whose walk is another worker's, goes when that walk's own copy is among the
/// workers' records (`ensure_body`); the walk's is the one the build emits.
macro_rules! retain_class_bodies {
    ($self:ident, $crew:ident, $name:ident) => {
        let $name = {
            let walked: std::collections::HashSet<ClassId> = $name.iter().flatten().map(|tc| tc.id).collect();
            if std::env::var_os("TEQ_MERGE_TRACE").is_some_and(|v| v == "print") {
                let mut seen: FxMap<ClassId, Vec<String>> = FxMap::default();
                for (k, own) in $name.iter().enumerate() {
                    for tc in own {
                        seen.entry(tc.id).or_default().push(format!("worker {}", k));
                    }
                }
                for (i, tc) in $self.prog.classes.entries() {
                    if i as u32 >= $self.prog.classes.own_base() {
                        break;
                    }
                    seen.entry(tc.id).or_default().push("shared".to_string());
                }
                for (c, from) in seen {
                    if from.len() > 1 {
                        eprintln!("merge check: class {} has {} bodies: {}", $self.name_str($self.syms.class(c).name), from.len(), from.join(", "));
                    }
                }
            }
            let records = $self.prog.classes.retained(|tc| walked.contains(&tc.id), $crew);
            ($name, records)
        };
    };
}

impl<'a> Worker<'a> {
    /// Takes over what another worker made during the bodies that lives outside the arenas:
    /// the per-thread tables under the ids of the other worker,
    /// which the remap renumbers with this worker's own. The other worker's arenas stay with it
    /// (`merge_workers` takes their records).
    fn absorb(&mut self, o: &mut Worker<'a>) {
        self.diags.absorb(std::mem::take(&mut o.diags));
        self.absorb_unused(o);
        self.sealed_std_files.append(&mut o.sealed_std_files);
        self.profile.absorb(std::mem::take(&mut o.profile));
        self.def_syms.absorb(o.def_syms.take_own());
        self.def_classes.absorb(o.def_classes.take_own());
        self.def_aliases.absorb(o.def_aliases.take_own());
        self.file_imports.absorb_some(o.file_imports.take_own());
        self.fun_of_sym.absorb(o.fun_of_sym.take_local());
        self.val_init.absorb(o.val_init.take_local());
        self.class_done.absorb(o.class_done.take_local());
        self.class_imports.absorb(o.class_imports.take_local());
        self.retained_bodies.absorb(o.retained_bodies.take_local());
        let theirs = o.inline_definitions.take_local();
        // A capture's census drops the records of every body a worker checked, those of the
        // records another worker's replaces included; nothing else reads a replaced one.
        if self.prog.capture.is_some() {
            for s in theirs.keys() {
                if let Some(mine) = self.inline_definitions.local.get(s) {
                    self.superseded_definitions.push(mine.clone());
                }
            }
            self.superseded_definitions.append(&mut o.superseded_definitions);
        }
        self.check_capture_keys.append(&mut o.check_capture_keys);
        self.inline_definitions.absorb(theirs);
        self.any_members.absorb(o.any_members.take_local());
        self.outer_accessors.absorb(o.outer_accessors.take_local());
        self.merged_overloads.absorb(o.merged_overloads.take_local());
        self.derived_opaques.absorb(o.derived_opaques.take_local());
        self.arity_classes.absorb(o.arity_classes.take_local());
        self.derive.absorb(&mut o.derive);
        self.syms.dispatch_names.absorb(o.syms.dispatch_names.take_local());
        self.quote.absorb(&mut o.quote);
        fn union<K: Copy + Eq + std::hash::Hash, V>(mine: &mut FxMap<K, V>, theirs: &mut FxMap<K, V>) {
            for (k, v) in std::mem::take(theirs) {
                mine.entry(k).or_insert(v);
            }
        }
        union(&mut self.interpolations, &mut o.interpolations);
        union(&mut self.soft_exprs, &mut o.soft_exprs);
        union(&mut self.soft_syms, &mut o.soft_syms);
        for (body, users) in std::mem::take(&mut o.inline_deps) {
            self.inline_deps.entry(body).or_default().extend(users);
        }
        union(&mut self.anon_envs, &mut o.anon_envs);
        union(&mut self.anon_captures, &mut o.anon_captures);
        union(&mut self.anon_parent_args, &mut o.anon_parent_args);
        self.inferred_parent_args.absorb(o.inferred_parent_args.take_local());
        self.inferred_trait_args.absorb(o.inferred_trait_args.take_local());
        union(&mut self.sam_classes, &mut o.sam_classes);
        union(&mut self.sam_arity, &mut o.sam_arity);
        union(&mut self.super_problems_told, &mut o.super_problems_told);
        union(&mut self.suffixed_roots, &mut o.suffixed_roots);
        union(&mut self.expr_marks, &mut o.expr_marks);
        union(&mut self.eta_expansions, &mut o.eta_expansions);
        union(&mut self.folded_paths, &mut o.folded_paths);
        union(&mut self.irrefutable_pats, &mut o.irrefutable_pats);
        union(&mut self.outer_prefixes, &mut o.outer_prefixes);
        union(&mut self.captured_locals, &mut o.captured_locals);
        self.unchecked_variance.absorb(o.unchecked_variance.take_local());
        union(&mut self.poisoned, &mut o.poisoned);
        union(&mut self.poisoned_withheld, &mut o.poisoned_withheld);
        union(&mut self.withheld_notes, &mut o.withheld_notes);
        union(&mut self.reflect_ctor_params, &mut o.reflect_ctor_params);
        union(&mut self.reflect_param_of, &mut o.reflect_param_of);
        union(&mut self.anon_sites, &mut o.anon_sites);
        union(&mut self.macro_files, &mut o.macro_files);
        // A body's local opaque types, which the worker entered under the file it typed.
        for (mine, theirs) in self.file_opaques.own_mut().iter_mut().zip(o.file_opaques.take_own()) {
            for c in theirs {
                if !mine.contains(&c) {
                    mine.push(c);
                }
            }
        }
        // What the inline expansions keep past their item: the literals the interpreter folded,
        // and the arguments of the locals an `inline val` or a binder stands for.
        union(&mut self.inline.evaluated, &mut o.inline.evaluated);
        union(&mut self.inline.args, &mut o.inline.args);
        for a in std::mem::take(&mut o.matches.reported) {
            if !self.matches.reported.contains(&a) {
                self.matches.reported.push(a);
            }
        }
        // The export tables another worker built, where this one has none: a table reports its
        // conflicts as it is built, once.
        for (c, slot) in o.class_exports.take_own() {
            if matches!(slot, super::exports::ExportSlot::Ready(_) | super::exports::ExportSlot::Empty) && !matches!(self.class_exports.get(&c), Some(super::exports::ExportSlot::Ready(_) | super::exports::ExportSlot::Empty)) {
                self.class_exports.insert(c, slot);
            }
        }
        for (k, slot) in o.pkg_exports.take_own() {
            if matches!(slot, super::exports::ExportSlot::Ready(_) | super::exports::ExportSlot::Empty) && !matches!(self.pkg_exports.get(&k), Some(super::exports::ExportSlot::Ready(_) | super::exports::ExportSlot::Empty)) {
                self.pkg_exports.insert(k, slot);
            }
        }
        if let (Some(mine), Some(theirs)) = (&mut self.infos, &mut o.infos) {
            mine.append(theirs);
        }
        union(&mut self.temp_names, &mut o.temp_names);
        self.prog.template_syms.absorb(o.prog.template_syms.take_local());
        self.prog.overrides.absorb(o.prog.overrides.take_local());
        union(&mut self.prog.expansions, &mut o.prog.expansions);
        union(&mut self.prog.leaf_tests, &mut o.prog.leaf_tests);
        union(&mut self.prog.stored_tests, &mut o.prog.stored_tests);
        union(&mut self.prog.stored_pats, &mut o.prog.stored_pats);
        union(&mut self.prog.deferred_tests, &mut o.prog.deferred_tests);
        if let (Some(mine), Some(theirs)) = (self.prog.capture.as_deref_mut(), o.prog.capture.as_deref_mut()) {
            mine.absorb(theirs);
        }
        if let (Some(mine), Some(theirs)) = (self.deps.as_deref_mut(), o.deps.as_deref_mut()) {
            mine.absorb(theirs);
        }
        if let (Some(mine), Some(theirs)) = (self.index.as_deref_mut(), o.index.take()) {
            mine.absorb(*theirs);
        }
        for (tr, syms) in std::mem::take(&mut o.mixin_supers) {
            let mine = self.mixin_supers.entry(tr).or_default();
            for s in syms {
                if !mine.contains(&s) {
                    mine.push(s);
                }
            }
        }
        let news = self.lists_at_fork;
        self.local_news.extend(o.local_news.drain(news.min(o.local_news.len())..));
        self.outer_this.absorb(o.outer_this.take_local());
        self.entry_points.append(&mut o.entry_points);
        // What an inline definition's check left, each worker that asked for the check having
        // made it, is checked once, as by one worker; an expansion's own checks all stay.
        let met: std::collections::HashSet<(SymId, (FileId, u32, u32))> = self.deferred_bounds.iter().filter_map(|b| Some((b.definition?, b.site()))).collect();
        self.deferred_bounds.extend(o.deferred_bounds.drain(..).filter(|b| b.definition.map_or(true, |d| !met.contains(&(d, b.site())))));
        let site = |m: &crate::typer::space::DeferredMatch| (m.file, m.span.start, m.span.end);
        let met: std::collections::HashSet<(SymId, (FileId, u32, u32))> = self.deferred_matches.iter().filter_map(|m| Some((m.definition?, site(m)))).collect();
        self.deferred_matches.extend(o.deferred_matches.drain(..).filter(|m| m.definition.map_or(true, |d| !met.contains(&(d, site(m))))));
        self.late_methods.append(&mut o.late_methods);
        self.override_pairs.append(&mut o.override_pairs);
        self.inherited_pairs.append(&mut o.inherited_pairs);
        self.member_cycles.append(&mut o.member_cycles);
        self.unsettled_std_entries.append(&mut o.unsettled_std_entries);
        self.dispatch_pending.append(&mut o.dispatch_pending);
        self.named_like.append(&mut o.named_like);
        self.vals_by_accessor |= o.vals_by_accessor;
        self.renamed_roots.append(&mut o.renamed_roots);
        self.prog.top_funs.absorb(o.prog.top_funs.take_own());
        self.prog.top_vals.absorb(o.prog.top_vals.take_own());
        self.prog.js_exports.append(&mut o.prog.js_exports);
        self.prog.template_calls.append(&mut o.prog.template_calls);
        for unit in std::mem::take(&mut o.prog.get_class_units) {
            if !self.prog.get_class_units.contains(&unit) {
                self.prog.get_class_units.push(unit);
            }
        }
        if self.prog.main.is_none() {
            self.prog.main = o.prog.main;
            self.prog.main_object = o.prog.main_object;
        }
        let b = &mut self.b;
        let ob = &o.b;
        for (mine, theirs) in [(&mut b.functions, &ob.functions), (&mut b.context_functions, &ob.context_functions), (&mut b.tuples, &ob.tuples)] {
            if mine.len() < theirs.len() {
                mine.resize(theirs.len(), None);
            }
            for (m, t) in mine.iter_mut().zip(theirs.iter()) {
                if m.is_none() {
                    *m = *t;
                }
            }
        }
        for (mine, theirs) in [
            (&mut b.by_name, ob.by_name),
            (&mut b.seq, ob.seq),
            (&mut b.string_context, ob.string_context),
            (&mut b.partial_function, ob.partial_function),
            (&mut b.conversion, ob.conversion),
            (&mut b.sub_evidence, ob.sub_evidence),
            (&mut b.eq_evidence, ob.eq_evidence),
            (&mut b.option, ob.option),
            (&mut b.named_tuple, ob.named_tuple),
            (&mut b.either, ob.either),
            (&mut b.set, ob.set),
            (&mut b.map, ob.map),
            (&mut b.can_equal, ob.can_equal),
            (&mut b.value_of, ob.value_of),
            (&mut b.selectable, ob.selectable),
            (&mut b.number, ob.number),
            (&mut b.js_dynamic, ob.js_dynamic),
            (&mut b.throwable, ob.throwable),
            (&mut b.js_exception, ob.js_exception),
            (&mut b.cons_tuple, ob.cons_tuple),
            (&mut b.product, ob.product),
            (&mut b.reflect_enum, ob.reflect_enum),
            (&mut b.tuple_trait, ob.tuple_trait),
            (&mut b.non_empty_tuple, ob.non_empty_tuple),
            (&mut b.empty_tuple, ob.empty_tuple),
        ] {
            if mine.is_none() {
                *mine = theirs;
            }
        }
        for (k, s) in &ob.product_helpers {
            if !b.product_helpers.iter().any(|(k2, _)| k2 == k) {
                b.product_helpers.push((k.clone(), *s));
            }
        }
        if self.sites.class_tag.is_none() {
            self.sites.class_tag = o.sites.class_tag;
        }
        if self.sites.not_given_class.is_none() {
            self.sites.not_given_class = o.sites.not_given_class;
        }
        if self.unit_ops.is_none() {
            self.unit_ops = o.unit_ops;
        }
        if self.prog.partial_function.is_none() {
            self.prog.partial_function = o.prog.partial_function;
        }
        if self.prog.throwable.is_none() {
            self.prog.throwable = o.prog.throwable;
        }
        if self.prog.js_exception.is_none() {
            self.prog.js_exception = o.prog.js_exception;
        }
        if self.reflective_jar_found.is_none() {
            self.reflective_jar_found = o.reflective_jar_found.take();
        }
    }

    /// A place where two different diagnostics of the workers stand, by file and offset. They
    /// are presented in the order they were reported in, which the workers' schedule decides
    /// (two workers', or one worker's items taken from a file's tail): a signature's error is
    /// reported by whichever worker completes it first. One worker's build reports them in
    /// its walk's order.
    pub fn diagnostics_meeting(&self, others: &[Option<Worker<'a>>]) -> Option<(FileId, u32)> {
        use std::collections::hash_map::Entry;
        let mut at: std::collections::HashMap<(FileId, u32), &crate::source::Diagnostic> = std::collections::HashMap::new();
        for d in self.diags.items.iter().chain(others.iter().flatten().flat_map(|o| o.diags.items.iter())) {
            match at.entry((d.file, d.span.start)) {
                Entry::Vacant(e) => {
                    e.insert(d);
                }
                Entry::Occupied(e) if *e.get() != d => return Some((d.file, d.span.start)),
                Entry::Occupied(_) => {}
            }
        }
        None
    }

    /// The merge after the body phase over the workers: every worker's
    /// own records placed after the shared region in the order of the work items (`ItemRange`), every id
    /// of theirs renumbered to where they went, the tables taken over, and the arenas one
    /// worker's again. `others` are the other workers, this one the first; with none it is
    /// the fork's own merge.
    pub fn merge_workers(&mut self, others: Vec<Option<Worker<'a>>>, at_fork: &Prefix) {
        #[cfg(debug_assertions)]
        crate::typer::loader::compile::body_crossings::report();
        let threads = others.len() + 1;
        let t = std::time::Instant::now();
        let ended = crate::crew::Crew::with(threads, |crew| {
            crate::measure::merge_part("crew", t.elapsed());
            self.merge_with(others, at_fork, crew);
            std::time::Instant::now()
        });
        crate::measure::merge_part("crew", ended.elapsed());
    }

    /// `merge_workers` with its crew, as many threads as there were workers.
    fn merge_with(&mut self, mut others: Vec<Option<Worker<'a>>>, at_fork: &Prefix, crew: &crate::crew::Crew) {
        use crate::arena::LOCAL_BASE;
        let p = self.phase(super::profile::Phase::Merge);
        let mut clock = Clock::start(true);
        self.overlays_joined();
        let types_at_join = self.types.len().0 as u32;
        // The work items, by worker in the order each typed them, and their canonical order by
        // their keys (`ItemRange`), a key, a worker and an index each, sorted: a tie of keys keeps
        // the workers' order and each worker's.
        let items: Vec<Vec<ItemRange>> = std::iter::once(std::mem::take(&mut self.items)).chain(others.iter_mut().map(|o| o.as_mut().map_or_else(Vec::new, |o| std::mem::take(&mut o.items)))).collect();
        let mut order: Vec<((u32, u32), u32, u32)> = items.iter().enumerate().flat_map(|(k, v)| v.iter().enumerate().map(move |(i, r)| (r.key, k as u32, i as u32))).collect();
        order.sort_unstable();
        clock.part("join");
        if crate::measure::on() && std::env::var_os("TEQ_FORK_INVENTORY").is_some_and(|v| v == "1") {
            self.fork_inventory_at_join(&others);
            clock.part("measurement");
        }
        self.merge_index_records(&mut others, &items, &order, at_fork);
        clock.part("tables");
        for o in others.iter_mut().flatten() {
            self.absorb(o);
        }
        // The super calls of the shared region's traits, which every worker registered in one
        // table (`note_mixin_super`), are this worker's from here on.
        let shared = std::mem::take(&mut *self.shared_mixin_supers.lock().unwrap_or_else(|e| e.into_inner()));
        for (tr, syms) in shared {
            let mine = self.mixin_supers.entry(tr).or_default();
            for s in syms {
                if !mine.contains(&s) {
                    mine.push(s);
                }
            }
        }
        let shared = Prefix {
            syms: self.syms.syms.shared_len() as u32,
            classes: self.syms.classes.shared_len() as u32,
            tparams: self.syms.tparams.shared_len() as u32,
            aliases: self.syms.aliases.shared_len() as u32,
            pkgs: 0,
            overloads: 0,
            exprs: self.prog.exprs.shared_len() as u32,
            pats: self.prog.pats.shared_len() as u32,
            strings: 0,
            expr_lists: 0,
            pat_lists: 0,
            sym_lists: 0,
            stmts: 0,
            cases: 0,
            tries: 0,
            tests: 0,
            funs: 0,
            tclasses: 0,
            quotes: self.prog.quotes.shared_len() as u32,
            quote_pats: self.prog.quote_pats.shared_len() as u32,
            types: 0,
            names: 0,
            journal: 0,
        };
        clock.part("tables");
        take_arenas!(self, others; syms: syms.syms, classes: syms.classes, tparams: syms.tparams, aliases: syms.aliases, pkgs: syms.pkgs, overloads: syms.overloads, exprs: prog.exprs, pats: prog.pats, strings: prog.strings, expr_lists: prog.expr_lists, pat_lists: prog.pat_lists, sym_lists: prog.sym_lists, stmts: prog.stmts, cases: prog.cases, tries: prog.tries, tests: prog.tests, funs: prog.funs, tclasses: prog.classes, quotes: prog.quotes, quote_pats: prog.quote_pats);
        clock.part("retention");
        if crate::measure::on() && crate::measure::types_wanted() {
            fn size<T>(name: &str, shared: usize, own: &[Vec<T>]) -> String {
                let n: usize = own.iter().map(|v| v.len()).sum();
                format!("{} {}+{} x {} B", name, shared, n, std::mem::size_of::<T>())
            }
            let sizes = vec![
                size("syms", self.syms.syms.shared_len(), &syms),
                size("classes", self.syms.classes.shared_len(), &classes),
                size("tparams", self.syms.tparams.shared_len(), &tparams),
                size("aliases", self.syms.aliases.shared_len(), &aliases),
                size("pkgs", self.syms.pkgs.shared_len(), &pkgs),
                size("overloads", self.syms.overloads.shared_len(), &overloads),
                size("exprs", self.prog.exprs.shared_len(), &exprs),
                size("pats", self.prog.pats.shared_len(), &pats),
                size("strings", self.prog.strings.shared_len(), &strings),
                size("expr_lists", self.prog.expr_lists.shared_len(), &expr_lists),
                size("pat_lists", self.prog.pat_lists.shared_len(), &pat_lists),
                size("sym_lists", self.prog.sym_lists.shared_len(), &sym_lists),
                size("stmts", self.prog.stmts.shared_len(), &stmts),
                size("cases", self.prog.cases.shared_len(), &cases),
                size("tries", self.prog.tries.shared_len(), &tries),
                size("tests", self.prog.tests.shared_len(), &tests),
                size("funs", self.prog.funs.shared_len(), &funs),
                size("tclasses", self.prog.classes.shared_len(), &tclasses),
                size("quotes", self.prog.quotes.shared_len(), &quotes),
                size("quote_pats", self.prog.quote_pats.shared_len(), &quote_pats),
            ];
            crate::measure::overlays_measured(vec![crate::measure::OverlayRow { label: "merge: the arenas' records, shared+own x size".to_string(), count: None, time: None, note: sizes.join(", ") }]);
        }
        // What the others' variables were solved to, which the types their bodies recorded
        // name.
        for o in others.iter_mut().flatten() {
            let tvars = std::mem::replace(&mut o.tvars, TVars::new());
            self.tvars.absorb(tvars);
        }
        clock.part("tables");
        // The other workers' arenas over the shared regions go, so that the regions are this
        // worker's alone to merge: dropped by the crew, each worker's remains on one thread.
        let remains: Vec<std::sync::Mutex<Option<Worker<'a>>>> = others.into_iter().map(std::sync::Mutex::new).collect();
        crew.each(remains.len(), &|k| drop(remains[k].lock().unwrap_or_else(|e| e.into_inner()).take()));
        drop(remains);
        clock.part("workers dropped");
        retain_arenas!(self, crew; syms: syms.syms, classes: syms.classes, tparams: syms.tparams, aliases: syms.aliases, pkgs: syms.pkgs, overloads: syms.overloads, exprs: prog.exprs, pats: prog.pats, strings: prog.strings, expr_lists: prog.expr_lists, pat_lists: prog.pat_lists, sym_lists: prog.sym_lists, stmts: prog.stmts, cases: prog.cases, tries: prog.tries, tests: prog.tests, funs: prog.funs, quotes: prog.quotes, quote_pats: prog.quote_pats);
        retain_class_bodies!(self, crew, tclasses);
        clock.part("retention");
        macro_rules! size {
            ($name:ident) => {
                ($name.1.len() as u32, $name.0.iter().map(|v| v.len()).collect::<Vec<usize>>())
            };
        }
        let sizes = [size!(syms), size!(classes), size!(tparams), size!(aliases), size!(pkgs), size!(overloads), size!(exprs), size!(pats), size!(strings), size!(expr_lists), size!(pat_lists), size!(sym_lists), size!(stmts), size!(cases), size!(tries), size!(tests), size!(funs), size!(tclasses), size!(quotes), size!(quote_pats)];
        let numbering = numberings(&items, &order, &sizes, crew);
        let uncovered: usize = numbering.iter().map(|n| n.uncovered).sum();
        clock.part("numbering");
        // The ordering's buffers: the items' order and every arena's runs.
        self.note_merge_memory("the numbering", order.capacity() * std::mem::size_of::<((u32, u32), u32, u32)>() + numbering.iter().map(|n| n.runs.capacity() * std::mem::size_of::<crate::arena::Run>()).sum::<usize>());
        place_arenas!(self, numbering, crew; syms: syms.syms = 0, classes: syms.classes = 1, tparams: syms.tparams = 2, aliases: syms.aliases = 3, pkgs: syms.pkgs = 4, overloads: syms.overloads = 5, exprs: prog.exprs = 6, pats: prog.pats = 7, strings: prog.strings = 8, expr_lists: prog.expr_lists = 9, pat_lists: prog.pat_lists = 10, sym_lists: prog.sym_lists = 11, stmts: prog.stmts = 12, cases: prog.cases = 13, tries: prog.tries = 14, tests: prog.tests = 15, funs: prog.funs = 16, tclasses: prog.classes = 17, quotes: prog.quotes = 18, quote_pats: prog.quote_pats = 19);
        self.syms.carry_cells(&classes, &syms, &aliases, crew);
        clock.part("placement");
        let prog = &mut self.prog;
        let expr_numbering = &numbering[6];
        prog.expr_types.place(expr_numbering, crew);
        prog.expr_spans.place(expr_numbering, crew);
        // The bit sets, a task of the crew each.
        let bits: Vec<std::sync::Mutex<&mut crate::arena::Bits>> = [&mut prog.expansion_bits, &mut prog.leaf_bits, &mut prog.chain_end_bits, &mut prog.taken_then_bits, &mut prog.taken_else_bits, &mut prog.widened_bits, &mut prog.opaque_bits, &mut prog.spread_bits].into_iter().map(std::sync::Mutex::new).collect();
        crew.each(bits.len(), &|i| bits[i].lock().unwrap_or_else(|e| e.into_inner()).place(expr_numbering));
        drop(bits);
        clock.part("placement of the side tables");
        let table = |new: Vec<Vec<u32>>| Table::of(LOCAL_BASE, new);
        let mut remap = Remap {
            types: self.types.clone(),
            types_from: self.types_at_fork,
            promoted: Default::default(),
            published: None,
            source: None,
            map_roots: None,
            checking: None,
            forked: true,
            shared,
            // The class bodies are walked whole: one of the shared region gets the members a
            // worker's walk typed.
            walk: Prefix { syms: 0, classes: 0, tparams: 0, aliases: 0, pkgs: 0, overloads: 0, tclasses: 0, types: 0, names: 0, ..*at_fork },
            ids: Tables {
                syms: table(syms),
                classes: table(classes),
                tparams: table(tparams),
                aliases: table(aliases),
                pkgs: table(pkgs),
                overloads: table(overloads),
                exprs: table(exprs),
                pats: table(pats),
                strings: table(strings),
                expr_lists: table(expr_lists),
                pat_lists: table(pat_lists),
                sym_lists: table(sym_lists),
                stmts: table(stmts),
                cases: table(cases),
                tries: table(tries),
                tests: table(tests),
                funs: table(funs),
                tclasses: table(tclasses),
                quotes: table(quotes),
                quote_pats: table(quote_pats),
            },
            sigs: SigMemo::default(),
            walked: 0,
            clock: Clock::default(),
        };
        self.fun_of_sym.merge_own();
        self.val_init.merge_own();
        self.class_done.merge_own();
        self.class_imports.merge_own();
        self.retained_bodies.merge_own();
        self.inline_definitions.merge_own();
        self.any_members.merge_own();
        self.outer_accessors.merge_own();
        self.merged_overloads.merge_own();
        self.derived_opaques.merge_own();
        self.arity_classes.merge_own();
        self.class_exports.merge_own();
        self.pkg_exports.merge_own();
        self.prog.template_syms.merge_own();
        // Every body is placed now; the registry of other workers' is done with.
        self.prog.class_bodies = Default::default();
        self.prog.overrides.merge_own();
        self.inferred_parent_args.merge_own();
        self.inferred_trait_args.merge_own();
        self.unchecked_variance.merge_own();
        self.outer_this.merge_own();
        self.quote.merge_own();
        self.derive.merge_own();
        self.syms.dispatch_names.merge_own();
        if let Some(r) = self.js_registry.take() {
            let r = std::sync::Arc::try_unwrap(r).unwrap_or_else(|_| panic!("the JS registry is still shared"));
            self.prog.js_imports = r.list.into_vec();
            self.js_import_ids = self.prog.js_imports.iter().enumerate().map(|(i, &imp)| (imp, i as u32)).collect();
        }
        self.def_syms.merge_own();
        self.def_classes.merge_own();
        self.def_aliases.merge_own();
        self.file_pkgs.merge_own();
        self.file_imports.merge_own();
        self.file_opaques.merge_own();
        self.prog.file_tags.merge_own();
        self.forked = false;
        self.types.set_exclusive(true);
        self.interner.set_exclusive(true);
        self.types.carry_local_marks(|c| remap.class(c));
        clock.part("tables");
        let started = std::time::Instant::now();
        let mut traced = Ok(());
        if std::env::var_os("TEQ_MERGE_SERIAL").is_some_and(|v| v == "1") {
            // The serial walk, listing, interning and renumbering at once (the merge-order
            // check's reference, docs/DEVELOPING.md).
            let trace = trace::wanted().then(|| {
                remap.size_promotion();
                remap.trace(self)
            });
            remap.clock = clock;
            remap.apply(self);
            clock = std::mem::take(&mut remap.clock);
            if let Some(trace) = trace {
                remap.publish();
                traced = remap.check_trace(self, trace, None);
            }
        } else {
            let listing = remap.listing(self, crew);
            clock.part("listing");
            self.note_merge_memory("the listing", remap.held() + listing.held());
            let trace = trace::wanted().then(|| remap.trace(self));
            remap.intern(self, &listing, &clock);
            self.note_merge_memory("the interning", remap.held() + listing.held());
            remap.publish();
            remap.renumber(self, crew);
            clock.part("renumbering");
            if let Some(trace) = trace {
                traced = remap.check_trace(self, trace, Some(&listing));
                clock.part("the merge-order check");
            }
            crew.give_back(listing);
        }
        let promoted = started.elapsed();
        if crate::measure::types_wanted() {
            crate::measure::merge_part("walk: of it inside the promotion (TEQ_WORKERS_TYPES=1)", remap.promoted.get_mut().interning);
        }
        self.note_merge_memory("the promotion and the renumbering", remap.held());
        if crate::measure::on() && crate::measure::types_wanted() {
            self.types_escaping(remap.types_from, |t| remap.promoted_of(t), types_at_join);
        }
        clock.part("measurement");
        self.sort_registers();
        clock.part("cleanup");
        // The interpreter's caches index the program by the ids of the body phase: both views'.
        crate::interp::drop_caches();
        // The indexes over the records are built again from the merged arenas, and the
        // memos keyed by the types of the bodies go with the worker's chunk.
        self.prog_index = super::check::ProgIndex::default();
        self.roots_memo.clear();
        self.subclasses = super::overload::SubclassIndex::default();
        self.given_fast.clear();
        self.given_fits.clear();
        self.head_sigs.clear();
        self.given_preferences.clear();
        self.given_orders.clear();
        self.implicit_scopes.clear();
        self.implicit_scope_givens.clear();
        self.package_levels.clear();
        self.value_sigs.clear();
        self.derived_aliases.clear();
        self.inferred_outcomes.clear();
        self.inferred_val_outcomes.clear();
        self.matches.forget();
        self.class_given_indexes.clear();
        self.conversion_indexes.clear();
        // What the last typing of an item left in the working state, which the next typing sets
        // before it reads.
        self.case_binders.clear();
        self.last_typed_pattern = None;
        self.gadt.clear();
        self.enum_case_new = None;
        self.ascribed_constant = None;
        self.arg_types_seen.clear();
        self.erroneous_args.clear();
        self.erroneous_lambdas.clear();
        self.quote.hole_ranges.clear();
        self.inline.clear_leftovers();
        clock.part("cleanup");
        let started = std::time::Instant::now();
        let roots_left = remap.promoted.get_mut().counts.roots_left;
        let swept = sweep_wanted();
        let checked = if let Err(problem) = traced {
            Err(format!("the merge-order check: {}", problem))
        } else if swept {
            self.check_merge(remap.types_from)
        } else if roots_left > 0 {
            Err(format!("the merge left the workers' ids in the merged program: {} of the types it handed the records are an overlay's or name a worker's id", roots_left))
        } else {
            Ok(0)
        };
        let checked_in = started.elapsed();
        clock.part("check");
        self.note_merge_memory("the check", remap.held());
        if let Err(problem) = &checked {
            self.merge_failed = true;
            self.diags.error(crate::source::NO_FILE, crate::source::Span::default(), problem.clone());
        }
        let started = std::time::Instant::now();
        self.types.reclaim_overlays();
        let reclaimed_in = started.elapsed();
        clock.part("reclamation");
        self.note_merge_memory("the reclamation", remap.held());
        let records = remap.records();
        self.merge_measured(remap.promoted.get_mut().counts, uncovered, [promoted, checked_in, reclaimed_in], checked.as_ref().map_or(0, |n| *n), swept);
        if self.profile.on {
            self.profile.merge = MergeStats { records, ..Default::default() };
        }
        if !self.merge_failed {
            self.complete_pending_sealed_files();
        }
        // The merge's own tables, the numbering and the items, freed off the main thread with
        // the rest the merge gives back.
        crew.give_back(crate::crew::Unshared((remap, numbering, items, order)));
        clock.part("end");
        self.phase_end(p);
    }

    /// The merge's check, after the promotion and the
    /// renumbering and before the overlays go and any reader of the merged program runs: the
    /// merge's walk again over every record from the first of each arena, every id kept
    /// (`Remap::checking`), with the loader's tables beside it. A worker's arena id left in a
    /// record, an overlay's entry, or a type naming either is a defect of the merge's closure:
    /// the build ends with an error naming what was found (`merge_failed`). The types checked.
    fn check_merge(&mut self, types_from: u32) -> Result<u64, String> {
        let mut check = Remap::checking(self.types.clone(), types_from);
        check.apply(self);
        // The library bodies' converted trees, which the loader's lock holder resolves types
        // into (`TyExpr::Resolved`) and a session's retype reads again.
        for i in 0..self.asts.len() {
            for t in &self.ast(FileId(i as u32)).tys {
                if let crate::ast::TyExpr::Resolved(ty) = *t {
                    check.ty(ty);
                }
            }
        }
        let mut found: Vec<String> = check.ids.all().iter().filter(|(_, t)| t.refused() > 0).map(|(what, t)| format!("{} ids of {}", t.refused(), what)).collect();
        found.extend(self.loader_left_local());
        let v = check.checking.take().expect("the check").into_inner();
        if v.overlay_types > 0 {
            found.push(format!("{} overlays' entries", v.overlay_types));
        }
        if v.local_types > 0 {
            found.push(format!("{} types naming a worker's id", v.local_types));
        }
        if found.is_empty() {
            return Ok(v.types_checked);
        }
        Err(format!("the merge left the workers' ids in the merged program: {}; first: {}", found.join(", "), v.first.join("; ")))
    }

    /// The loader's tables that a worker's id keys (`check_merge`): the loader's entries, its side
    /// tables and the TASTy files' tables, filled under the loader's lock, where ids are the
    /// shared region's.
    fn loader_left_local(&self) -> Vec<String> {
        let is_local = |id: u32| id >= crate::arena::LOCAL_BASE;
        let Some(loaded) = &self.loaded else { return Vec::new() };
        let mut out = Vec::new();
        let n = loaded.classes.keys().filter(|c| is_local(c.0)).count() + loaded.syms.keys().filter(|s| is_local(s.0)).count() + loaded.aliases.keys().filter(|a| is_local(a.0)).count();
        if n > 0 {
            out.push(format!("{} loader entries keyed by a worker's id", n));
        }
        let n = loaded.bridged.keys().filter(|c| is_local(c.0)).count() + loaded.builtin_of.iter().filter(|(a, b)| is_local(a.0) || is_local(b.0)).count();
        if n > 0 {
            out.push(format!("{} loader side tables keyed by a worker's id", n));
        }
        let n: usize = loaded.tables.iter().map(|f| f.classes.values().filter(|c| is_local(c.0)).count() + f.terms.values().filter(|s| is_local(s.0)).count() + f.tparams.values().filter(|p| is_local(p.0)).count()).sum();
        if n > 0 {
            out.push(format!("{} entries of the TASTy files' tables holding a worker's id", n));
        }
        out
    }

    /// The navigation index's records the workers made, placed as the arenas' are: the prefix's
    /// first, then each work item's from the worker that typed it in the items' order
    /// (`ItemRange`), then what no item made, worker by worker; the index's last record of a span
    /// is the one it keeps (`index_order`), as one worker's walk would leave it.
    fn merge_index_records(&mut self, others: &mut [Option<Worker<'a>>], items: &[Vec<ItemRange>], order: &[((u32, u32), u32, u32)], at_fork: &Prefix) {
        if self.index.is_none() {
            return;
        }
        let mut journals: Vec<Vec<Option<(FileId, super::index::Record)>>> = std::iter::once(Some(&mut *self))
            .chain(others.iter_mut().map(Option::as_mut))
            .map(|w| w.and_then(|w| w.index.as_mut()).map_or_else(Vec::new, |ix| ix.take_journal()).into_iter().map(Some).collect())
            .collect();
        let prefix = (at_fork.journal as usize).min(journals[0].len());
        let mut merged: Vec<(FileId, super::index::Record)> = journals[0][..prefix].iter_mut().filter_map(Option::take).collect();
        for &(_, worker, i) in order {
            let r = &items[worker as usize][i as usize];
            let journal = &mut journals[worker as usize];
            let (start, end) = ((r.start.journal as usize).min(journal.len()), (r.end.journal as usize).min(journal.len()));
            merged.extend(journal[start..end].iter_mut().filter_map(Option::take));
        }
        for journal in journals.iter_mut() {
            merged.extend(journal.iter_mut().filter_map(Option::take));
        }
        if let Some(ix) = self.index.as_deref_mut() {
            ix.set_journal(merged);
        }
    }

    /// The registers the bodies append to, in the order of the merged ids: the workers
    /// appended in the order they were scheduled, which is not canonical.
    fn sort_registers(&mut self) {
        self.prog.top_funs.merge_own();
        self.prog.top_vals.merge_own();
        // A function the walk of one worker registered, another's demand did too.
        self.prog.top_funs.sort();
        self.prog.top_funs.own_mut().dedup();
        self.prog.top_vals.sort();
        self.prog.top_vals.own_mut().dedup();
        self.prog.js_exports.sort_by_key(|&(s, _)| s);
        self.prog.template_calls.sort();
        self.entry_points.sort();
        self.dispatch_pending.sort();
        self.named_like.sort();
        self.renamed_roots.sort();
        self.inherited_pairs.sort();
        self.member_cycles.sort();
        self.unsettled_std_entries.sort();
        self.local_news.sort_by_key(|&(c, e, _, _)| (c, e));
        self.deferred_bounds.sort_by_key(|b| b.site());
        for syms in self.mixin_supers.values_mut() {
            syms.sort();
        }
    }

    /// The merge of what the body phase made into the arenas the signature phase left, with
    /// one worker: the renumbering is the identity and the walk is what the merge costs.
    pub fn merge(&mut self, prefix: &Prefix) {
        let p = self.phase(super::profile::Phase::Merge);
        let mut remap = Remap::identity(self, prefix);
        remap.apply(self);
        if self.profile.on {
            let (types_made, types_held) = remap.count_types(self, prefix);
            self.profile.merge = MergeStats {
                records: remap.records(),
                syms: self.syms.syms.len() as u32 - prefix.syms,
                classes: self.syms.classes.len() as u32 - prefix.classes,
                exprs: self.prog.exprs.len() as u32 - prefix.exprs,
                funs: self.prog.funs.len() as u32 - prefix.funs,
                tclasses: self.prog.classes.len() as u32 - prefix.tclasses,
                names: self.interner.len() as u32 - prefix.names,
                types_made,
                types_held,
            };
        }
        self.phase_end(p);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ranges_past_a_shortened_table() {
        // A worker emptied the table after the walk's start was taken: nothing to walk.
        let [items, shared] = super::Remap::ranges(2, 2, 0);
        assert!(items.is_empty() && shared.is_empty());
        let [items, shared] = super::Remap::ranges(1, 3, 5);
        assert_eq!((items, shared), (3..5, 1..3));
    }

    use super::super::quoted::DeferredInline;
    use super::*;
    use crate::arena::LOCAL_BASE;
    use crate::shared::ReentrantLock;
    use std::sync::Arc as Shared;

    fn record(deferred: Vec<TExprId>) -> InlineDefinition {
        InlineDefinition {
            state: DefinitionState::Checked,
            body: Some(TExprId(1)),
            ty: ANY,
            params: vec![SymId(2)],
            defaults: Vec::new(),
            binders: Vec::new(),
            pattern_tparams: Vec::new(),
            reducible: Vec::new(),
            reducible_sources: Vec::new(),
            deferred,
            type_args: Vec::new(),
            splices: Vec::new(),
            leaves: Vec::new(),
            leaf_tests: Vec::new(),
            diagnostics: Vec::new(),
            widened: Vec::new(),
            opaque: Vec::new(),
            spread: Vec::new(),
            hoisted: Vec::new(),
            imports: Vec::new(),
            node_types: FxMap::default(),
            aliases: Vec::new(),
            classes: Vec::new(),
            inline_vals: Vec::new(),
            inferred_vals: Vec::new(),
            walk_lack: None,
        }
    }

    fn call(subst: Subst, ret_ty: TypeId) -> Arc<DeferredInline> {
        Arc::new(DeferredInline { sym: SymId(3), owner_subst: Vec::new(), prefix: None, sig: Arc::new(MethodSig::value(ANY)), subst, ret_ty, span: crate::source::Span::default(), expected: None, at_end: false })
    }

    fn zero() -> Prefix {
        Prefix { syms: 0, classes: 0, tparams: 0, aliases: 0, pkgs: 0, overloads: 0, exprs: 0, pats: 0, strings: 0, expr_lists: 0, pat_lists: 0, sym_lists: 0, stmts: 0, cases: 0, tries: 0, tests: 0, funs: 0, tclasses: 0, quotes: 0, quote_pats: 0, types: 0, names: 0, journal: 0 }
    }

    fn keep() -> Table {
        Table::of(u32::MAX, Vec::new())
    }

    /// A worker's ids renumbered by `new` (`new[i]` for `LOCAL_BASE + i` of worker 0).
    fn moved(new: &[(u32, u32)]) -> Table {
        let mut table = vec![u32::MAX; 16];
        for &(i, to) in new {
            table[i as usize] = to;
        }
        Table::of(LOCAL_BASE, vec![table])
    }

    /// A renumbering of the symbols by `syms` and the classes by `classes`, every other id kept,
    /// the types from `types_from` on made after the fork, its promotion sized over the store.
    fn renumbering(types: &Shared<TypeStore>, types_from: u32, syms: Table, classes: Table) -> Remap {
        let mut remap = Remap {
            walk: zero(),
            shared: zero(),
            types: types.clone(),
            types_from,
            promoted: Default::default(),
            published: None,
            source: None,
            map_roots: None,
            checking: None,
            forked: true,
            ids: Tables { syms, classes, ..Tables::each(|_| keep()) },
            sigs: SigMemo::default(),
            walked: 0,
            clock: Clock::default(),
        };
        remap.size_promotion();
        remap
    }

    /// A store forked for `workers` workers in the shared namespace, as the merge finds it after
    /// `body` ran (the workers' overlays entered in turn) and the workers joined.
    fn joined(workers: usize, body: impl FnOnce(&TypeStore, &ReentrantLock)) -> Shared<TypeStore> {
        let s = Shared::new(TypeStore::new());
        s.set_exclusive(false);
        assert!(s.fork_overlays(workers));
        body(&s, &ReentrantLock::new());
        s.leave_overlay();
        s.join_overlays();
        s.set_exclusive(true);
        s
    }

    /// The workers' merge keeps an unshared path's identity: two originals over one worker's
    /// symbol, each with its export and a peer's copy, merge into two paths under the symbol's
    /// new id, each copy onto its origin's, and types over the copies with them; a retype's
    /// merge after it leaves them as they are.
    #[test]
    fn the_merge_keeps_one_path_per_unshared_origin() {
        let local = SymId(LOCAL_BASE + 7);
        let mut made = Vec::new();
        let s = joined(2, |s, lock| {
            s.enter_overlay(0);
            let (u1, u2) = (s.term_unshared(local), s.term_unshared(local));
            let over_u1 = s.union(u1, ANY);
            lock.lock();
            let (e1, e2) = (s.export_at(Route::Publish, u1), s.export_at(Route::Publish, u2));
            lock.unlock();
            s.enter_overlay(1);
            let (p1, p2) = (s.import(u1), s.import(e2));
            let over_p1 = s.union(p1, ANY);
            made = vec![u1, u2, over_u1, e1, e2, p1, p2, over_p1];
        });
        let [u1, u2, over_u1, e1, e2, p1, p2, over_p1] = made[..] else { unreachable!() };
        let merge = renumbering(&s, s.bound(Sub::Types), moved(&[(7, 500)]), keep());
        let a = merge.ty(u1);
        let b = merge.ty(u2);
        assert!([e1, p1].iter().all(|&t| merge.ty(t) == a), "the first origin's copies");
        assert!([e2, p2].iter().all(|&t| merge.ty(t) == b), "the second origin's copies");
        assert!(a != b && s.is_unshared(a) && s.is_unshared(b));
        assert_eq!((s.get(a), s.get(b)), (Type::Term(SymId(500)), Type::Term(SymId(500))));
        assert!(a != s.mk(Type::Term(SymId(500))) && a.0 < LOCAL_BASE);
        assert_eq!(merge.ty(over_u1), merge.ty(over_p1));
        assert_eq!(merge.ty(over_u1), s.union(a, ANY));
        let len = s.len().0 as u32;
        let retype = renumbering(&s, len, keep(), keep());
        assert_eq!((retype.ty(a), retype.ty(b), s.len().0 as u32), (a, b, len));
        assert!(s.is_unshared(a) && s.is_unshared(b));
    }

    /// An unshared path over a shared symbol keeps the base's one representative of its origin
    /// through the merge: the export's, for the worker's original and every copy of it.
    #[test]
    fn an_unshared_path_over_a_shared_symbol_merges_onto_the_bases_representative() {
        let shared = SymId(30);
        let mut made = Vec::new();
        let s = joined(2, |s, lock| {
            s.enter_overlay(0);
            let u = s.term_unshared(shared);
            lock.lock();
            let e = s.export_at(Route::Publish, u);
            lock.unlock();
            s.enter_overlay(1);
            let p = s.import(u);
            made = vec![u, e, p];
        });
        let [u, e, p] = made[..] else { unreachable!() };
        assert!(u.0 >= LOCAL_BASE && e.0 < LOCAL_BASE);
        let merge = renumbering(&s, s.bound(Sub::Types), keep(), keep());
        assert_eq!((merge.ty(u), merge.ty(e), merge.ty(p)), (e, e, e));
    }

    /// The overlays' types a record holds are interned into the base, parts first: structurally
    /// equal types two workers made apart become one, an overlay's list, literal and blocked
    /// description the base's, so that nothing past the merge names an overlay's entry.
    #[test]
    fn the_merge_promotes_the_overlays_types_into_the_base() {
        let c = ClassId(40);
        let mut made = Vec::new();
        let s = joined(2, |s, _| {
            s.enter_overlay(0);
            let lit = s.lit(LitVal::Int(3));
            made.push(s.class(c, &[lit, ANY]));
            made.push(s.blocked("a reason"));
            s.enter_overlay(1);
            let lit = s.lit(LitVal::Int(3));
            made.push(s.class(c, &[lit, ANY]));
        });
        let [a0, why, a1] = made[..] else { unreachable!() };
        assert!(a0.0 >= LOCAL_BASE && a1.0 >= LOCAL_BASE && a0 != a1 && why.0 >= LOCAL_BASE);
        let merge = renumbering(&s, s.bound(Sub::Types), keep(), keep());
        let (m0, m1) = (merge.ty(a0), merge.ty(a1));
        assert!(m0 == m1 && m0.0 < LOCAL_BASE);
        let Type::Class(mc, l) = s.get(m0) else { panic!("a class type") };
        assert!(mc == c && l.0 < LOCAL_BASE);
        let Type::Lit(lit) = s.get(s.items(l)[0]) else { panic!("a literal type") };
        assert!(s.items(l)[0].0 < LOCAL_BASE && lit.0 < LOCAL_BASE && s.lit_val(lit) == LitVal::Int(3));
        let m = merge.ty(why);
        let Type::Blocked(b) = s.get(m) else { panic!("a blocked type") };
        assert!(m.0 < LOCAL_BASE && b.0 < LOCAL_BASE && s.blocked_description(b) == "a reason");
        let counts = merge.promoted.borrow().counts;
        assert_eq!((counts.overlay_types, counts.found), (5, 2), "the two classes and lists of one worker found at the other's");
    }

    /// The base's types made after the fork that name a worker's arena id are made again under its
    /// new id; the ones over the shared region's ids alone stand as they are.
    #[test]
    fn the_merge_makes_a_base_type_over_a_workers_class_again() {
        let mut made = Vec::new();
        let s = joined(1, |s, lock| {
            s.enter_overlay(0);
            lock.lock();
            made.push(s.class(ClassId(LOCAL_BASE + 5), &[]));
            made.push(s.class(ClassId(41), &[ANY]));
            lock.unlock();
        });
        let [late, plain] = made[..] else { unreachable!() };
        assert!(late.0 >= s.bound(Sub::Types) && late.0 < LOCAL_BASE && s.mentions_local(late));
        let merge = renumbering(&s, s.bound(Sub::Types), keep(), moved(&[(5, 77)]));
        let m = merge.ty(late);
        assert!(s.get(m) == Type::Class(ClassId(77), EMPTY_LIST) && !s.mentions_local(m));
        assert_eq!(merge.ty(plain), plain);
    }

    /// A stored inline body's result type, its calls' type arguments and its deferred call's
    /// record are promoted with the rest, and the merge's check finds nothing of an overlay left
    /// in them; the check counts a record the merge did not walk.
    #[test]
    fn a_stored_inline_bodys_types_are_promoted_and_checked() {
        let mut made = Vec::new();
        let s = joined(1, |s, _| {
            s.enter_overlay(0);
            made.push(s.class(ClassId(40), &[ANY]));
            made.push(s.class(ClassId(41), &[ANY]));
        });
        let [t1, t2] = made[..] else { unreachable!() };
        let merge = renumbering(&s, s.bound(Sub::Types), keep(), keep());
        let mut d = record(Vec::new());
        d.ty = t1;
        d.type_args = vec![(TExprId(5), vec![t2, ANY])];
        let left = d.clone();
        merge.inline_definition(&mut d);
        assert!(d.ty.0 < LOCAL_BASE && d.type_args[0].1.iter().all(|t| t.0 < LOCAL_BASE));
        let mut deferred = call(vec![(TParamId(1), t1)], t2);
        let mut merge = merge;
        merge.deferred_inline(&mut deferred);
        assert!(deferred.ret_ty.0 < LOCAL_BASE && deferred.subst[0].1.0 < LOCAL_BASE);
        let check = Remap::checking(s.clone(), s.bound(Sub::Types));
        check.inline_definition(&mut d.clone());
        assert_eq!(check.checking.as_ref().unwrap().borrow().overlay_types, 0);
        check.inline_definition(&mut left.clone());
        assert_eq!(check.checking.as_ref().unwrap().borrow().overlay_types, 2);
    }

    /// The merge's check counts an overlay's type, a type naming a worker's arena id, a base type
    /// made over an overlay's part and a worker's id in a record, and keeps every id.
    #[test]
    fn the_merge_check_counts_what_the_merge_left() {
        let mut made = Vec::new();
        let s = joined(1, |s, _| {
            s.enter_overlay(0);
            made.push(s.class(ClassId(40), &[ANY]));
        });
        let o = made[0];
        let local = s.class(ClassId(LOCAL_BASE + 3), &[]);
        let over = s.mk(Type::Union(o, NOTHING));
        let check = Remap::checking(s.clone(), s.bound(Sub::Types));
        assert_eq!((check.ty(o), check.ty(local), check.ty(over)), (o, local, over));
        assert_eq!(check.sym(SymId(LOCAL_BASE + 1)), SymId(LOCAL_BASE + 1));
        assert_eq!(check.sym(SymId(12)), SymId(12));
        let v = check.checking.as_ref().unwrap().borrow();
        assert_eq!((v.overlay_types, v.local_types, check.ids.syms.refused()), (2, 1, 1));
    }

    /// The variables' tables after the merge: the signature phase's whole, of the rest the
    /// variables the promoted types name and what those name in turn, the rest reading open.
    #[test]
    fn the_merge_keeps_the_variables_its_records_reach() {
        let info = |inst: Option<TypeId>| TVarInfo { inst, ..Default::default() };
        let s = Shared::new(TypeStore::new());
        let from = s.len().0 as u32;
        let mut main = TVars::new();
        main.push(info(Some(ANY)));
        let mut other = TVars::attach(&main, 1, 1);
        let (v1, v2, v3) = (other.id(1), other.id(2), other.id(3));
        let t2 = s.mk(Type::Var(v2));
        other.push(info(Some(t2)));
        other.push(info(Some(NOTHING)));
        other.push(info(Some(ANY)));
        main.absorb(other);
        let t1 = s.mk(Type::Var(v1));
        let mut merge = renumbering(&s, from, keep(), keep());
        merge.ty(t1);
        merge.variables_of(&mut main, 1);
        assert_eq!((main.inst(v1), main.inst(v2), main.inst(v3)), (Some(t2), Some(NOTHING), None));
        assert_eq!(main.inst(main.id(0)), Some(ANY));
        let c = merge.promoted.borrow().counts;
        assert_eq!((c.own_vars, c.own_kept, c.other_vars, c.vars_kept), (1, 1, 4, 2));
    }

    /// The overlays given back: an overlay's entry read after it is refused in the
    /// assertion-enabled builds.
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "read after the merge gave the overlays back")]
    fn an_overlays_entry_read_after_the_reclamation_is_refused() {
        let mut made = Vec::new();
        let s = joined(1, |s, _| {
            s.enter_overlay(0);
            made.push(s.class(ClassId(40), &[ANY]));
        });
        s.reclaim_overlays();
        s.get(made[0]);
    }
}
