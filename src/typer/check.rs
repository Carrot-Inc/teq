use super::apply::{ArgList, ArgSrc, MethodCall};
use super::profile::Phase;
use super::site::SiteOwner;
use super::{Frame, Worker};
use crate::ast::{self, mods, DefId, DefKind, Stmt, TyExpr, TyExprId};
use crate::intern::{FxMap, Name};
use crate::names;
use crate::shared::CellKey;
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::sync::Arc;

/// How the signature of a member relates to one of an ancestor with the same name.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Agreement {
    Same,
    /// The same parameters, a result that does not conform.
    Result,
    /// Different parameters: unrelated members under scalac, an overload.
    Params,
}

/// How deep bodies may be typed inside one another before a cycle is reported.
const MAX_BODY_DEPTH: u32 = 256;

/// The kinds of cells the wait graph names (`CellKey`).
pub const CELL_SIG: u8 = 1;
pub const CELL_BODY: u8 = 2;
pub const CELL_CLASS: u8 = 3;
pub const CELL_ALIAS: u8 = 4;
pub const CELL_CHECK: u8 = 5;

/// A work item of the body phase: a top-level definition of a file, typed as a whole. For a
/// class that is its check (parents, overrides, variances, implements, the class-body
/// statements, the constructor defaults and parent arguments) with its member bodies and its
/// nested classes; for a def, val or given its body; the anonymous and local classes a body
/// makes are that body's. The parallel typer queues a file's items together in source order
/// and steals them one at a time.
#[derive(Clone, Copy)]
pub struct Item {
    pub file: FileId,
    pub def: DefId,
}

/// `TEQ_IDLE_WORKERS=1` keeps every worker but the main one from taking work, `=0` keeps the
/// main one from it: the threads, the lock and the cells with nothing migrating, or every
/// item on an attached worker, for the determinism test's bisection. The workers it keeps idle
/// are attached and started all the same (`idle_workers_asked`).
fn idle_worker(worker: usize) -> bool {
    match std::env::var("TEQ_IDLE_WORKERS").ok().as_deref() {
        Some("0") => worker == 0,
        Some(_) => worker != 0,
        None => false,
    }
}

/// Whether `TEQ_IDLE_WORKERS` is set: every worker is then attached and started, work or none.
pub fn idle_workers_asked() -> bool {
    std::env::var_os("TEQ_IDLE_WORKERS").is_some()
}

/// A worker's work: a file it holds with the file's first item where it has one left, or an item from
/// the tail of a file another worker holds.
#[derive(Clone, Copy)]
pub enum Reserved {
    File(usize, Option<u32>),
    Item(usize, u32),
}

/// The body phase's queue: the files in `Worker::file_order`, each
/// with its items in source order. A worker takes whole files from the front, one at a time, and
/// types their items from the head; a worker without a file takes single items from the tail of a
/// file another worker holds, so that the holder still meets its file's definitions in order.
pub struct Queue {
    /// The files and how many items each has.
    pub files: Vec<(FileId, u32)>,
    next_file: std::sync::atomic::AtomicUsize,
    /// Per file, the head and the tail of the items not taken yet, packed as `head | tail << 32`.
    cursors: Vec<std::sync::atomic::AtomicU64>,
    /// How many steals were attempted (`TEQ_STEAL_LAST`).
    stolen: std::sync::atomic::AtomicUsize,
}

impl Queue {
    /// The files of the walk's order still to type: not the ones typed before the fork
    /// (`check_before_fork`), which stay the prefix's.
    pub fn new(w: &Worker, order: &[FileId]) -> Queue {
        let files: Vec<(FileId, u32)> = order.iter().copied().filter(|&f| !w.walk.passed(f)).map(|f| (f, w.ast(f).top_level.len() as u32)).collect();
        let cursors = files.iter().map(|&(_, n)| std::sync::atomic::AtomicU64::new((n as u64) << 32)).collect();
        Queue { files, next_file: std::sync::atomic::AtomicUsize::new(0), cursors, stolen: std::sync::atomic::AtomicUsize::new(0) }
    }

    /// `TEQ_QUEUE_SHAPE=<file>`, a diagnostic of the count at the fork:
    /// a line per file of the queue, its items' bytes and expressions, `bytes/expressions` each.
    pub fn write_shape(&self, w: &Worker, path: &std::ffi::OsStr) {
        let mut out = String::new();
        for &(f, _) in &self.files {
            let ast = w.ast(f);
            let mut items: Vec<(u32, u32)> = ast.top_level.iter().map(|&d| ast.def_range(d)).map(|r| (r.start, r.end)).collect();
            items.sort_unstable();
            let mut exprs = vec![0u32; items.len()];
            for span in &ast.expr_spans {
                let i = items.partition_point(|&(start, _)| start <= span.start);
                if i > 0 && span.start < items[i - 1].1 {
                    exprs[i - 1] += 1;
                }
            }
            out.push_str(&format!("{}", w.source(f).path));
            for (&(start, end), n) in items.iter().zip(&exprs) {
                out.push_str(&format!(" {}/{}", end - start, n));
            }
            out.push('\n');
        }
        let _ = std::fs::write(path, out);
    }

    /// The next work of `worker`: a whole file, else an item stolen from a held file's tail; none
    /// once the queue is exhausted.
    pub fn reserve(&self, worker: usize) -> Option<Reserved> {
        self.reserve_file(worker).or_else(|| self.steal(worker).map(|(fi, i)| Reserved::Item(fi, i)))
    }

    /// A whole file for `worker`, with its first item where it has one: asked before the workers
    /// start, a worker's first file (`Typer::run`).
    pub fn reserve_file(&self, worker: usize) -> Option<Reserved> {
        self.take_file(worker).map(|fi| Reserved::File(fi, self.take_head(fi)))
    }

    /// The items no worker has taken yet, which the workers without a file steal as they run.
    pub fn items_left(&self) -> usize {
        self.cursors
            .iter()
            .map(|c| {
                let v = c.load(std::sync::atomic::Ordering::Acquire);
                ((v >> 32) as u32).saturating_sub(v as u32) as usize
            })
            .sum()
    }

    fn take_file(&self, worker: usize) -> Option<usize> {
        if idle_worker(worker) {
            return None;
        }
        let i = self.next_file.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        (i < self.files.len()).then_some(i)
    }

    /// The next item of file `fi` from the head, for the worker that holds the file.
    fn take_head(&self, fi: usize) -> Option<u32> {
        let c = &self.cursors[fi];
        let mut v = c.load(std::sync::atomic::Ordering::Acquire);
        loop {
            let (head, tail) = (v as u32, (v >> 32) as u32);
            if head >= tail {
                return None;
            }
            match c.compare_exchange_weak(v, v + 1, std::sync::atomic::Ordering::AcqRel, std::sync::atomic::Ordering::Acquire) {
                Ok(_) => return Some(head),
                Err(now) => v = now,
            }
        }
    }

    /// The last item of file `fi` not taken yet, for a worker without a file.
    fn take_tail(&self, fi: usize) -> Option<u32> {
        let c = &self.cursors[fi];
        let mut v = c.load(std::sync::atomic::Ordering::Acquire);
        loop {
            let (head, tail) = (v as u32, (v >> 32) as u32);
            if head >= tail {
                return None;
            }
            match c.compare_exchange_weak(v, v - (1 << 32), std::sync::atomic::Ordering::AcqRel, std::sync::atomic::Ordering::Acquire) {
                Ok(_) => return Some(tail - 1),
                Err(now) => v = now,
            }
        }
    }

    /// An item of the file with the most items left, from its tail. `TEQ_STEAL_LAST=m` lets
    /// the workers without a file take at most `m` items between them, for the determinism
    /// test's bisection.
    fn steal(&self, worker: usize) -> Option<(usize, u32)> {
        if idle_worker(worker) {
            return None;
        }
        if let Some(m) = std::env::var("TEQ_STEAL_LAST").ok().and_then(|m| m.parse::<usize>().ok()) {
            if self.stolen.fetch_add(1, std::sync::atomic::Ordering::AcqRel) >= m {
                return None;
            }
        }
        loop {
            let mut best: Option<(usize, u32)> = None;
            for (fi, c) in self.cursors.iter().enumerate() {
                let v = c.load(std::sync::atomic::Ordering::Acquire);
                let left = ((v >> 32) as u32).saturating_sub(v as u32);
                if left > best.map_or(0, |(_, n)| n) {
                    best = Some((fi, left));
                }
            }
            let (fi, _) = best?;
            if let Some(i) = self.take_tail(fi) {
                return Some((fi, i));
            }
        }
    }
}

/// What the typer gives the reach pass for a definition it asked for.
#[derive(Clone, Copy)]
pub enum DeferredBody {
    Fun(FunId),
    /// The initialiser of a val of a library class, now a lazy field of the class.
    Val(TExprId),
}

/// `TEQ_DEMAND_ORDER=reversed` turns the signature phase around: the classes and aliases are
/// completed and the scope tables built last to first, the naming order and the walk over the
/// bodies as they are. What a completion or a table reads of another must then not depend on
/// which was demanded first, as it must not once the parallel typer's workers demand them in
/// any order; tests/split.sh compares the diagnostics and the bytes with the walk's order.
fn demand_reversed() -> bool {
    std::env::var_os("TEQ_DEMAND_ORDER").map_or(false, |v| v == "reversed")
}

fn demand_order(n: usize) -> Box<dyn Iterator<Item = usize>> {
    if demand_reversed() {
        Box::new((0..n).rev())
    } else {
        Box::new(0..n)
    }
}

/// Which abstract members of its ancestors a check of what a class implements looks at: those
/// declared, at the class's check, or the outer accessors of the traits nested in a class, once
/// every body is typed (`check_outer_accessors_implemented`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Abstracts {
    Declared,
    OuterAccessors,
}

impl<'a> Worker<'a> {
    /// The eager part of the signature phase: the completion of every class and alias of the
    /// program's files, and the export tables of the packages.
    pub fn complete_program(&mut self) {
        // What a jar or the std holds completes when the program reaches it.
        for i in demand_order(self.syms.classes.len()) {
            let file = self.syms.classes[i].file;
            if !self.in_jar(file) && self.std.slot_of(file).is_none() {
                self.complete_class(ClassId(i as u32));
            }
        }
        for i in demand_order(self.syms.aliases.len()) {
            let file = self.syms.aliases[i].file;
            if !self.in_jar(file) && self.std.slot_of(file).is_none() {
                self.complete_alias(AliasId(i as u32));
            }
        }
        // Errors in top-level export clauses are reported even when nothing looks the exports up.
        for i in 0..self.syms.pkgs.len() {
            self.pkg_exports_of(PkgId(i as u32));
        }
    }

    /// The tables of the program's scopes that the bodies read, built at the end of the
    /// signature phase so that the bodies find them ready: read-only for the parallel typer's
    /// threads, where a table built on demand would be a cell.
    /// What a jar or the std holds keeps completing on demand, since tabling it eagerly would
    /// enter it. The export tables of the program's classes (the packages' are built above);
    /// the given indexes of the packages the program's files declare (the levels a search
    /// walks through the package clauses; `scala` and `java.lang` behind them are the std's,
    /// entered on the first search) and of the program's classes; the conversion indexes of
    /// the same scopes.
    pub fn build_scope_tables(&mut self) {
        use super::conversions::ScopeKey;
        let prof = self.phase(Phase::Tables);
        let saved = self.env.file;
        let mut packages = self.program_packages();
        let mut classes = self.program_classes();
        if demand_reversed() {
            packages.reverse();
            classes.reverse();
        }
        for (p, file) in packages {
            self.env.file = file;
            let index = self.given_index_of(p);
            self.conversion_index(ScopeKey::Pkg(p), &index);
        }
        // One pass per class: its export table, which its given index reads, then the index
        // and the conversions among its givens; most classes have none and cost a lookup.
        for c in classes {
            self.env.file = self.syms.class(c).file;
            self.exports_of(c);
            if let Some(index) = self.class_given_index_of(c) {
                self.conversion_index(ScopeKey::Class(c), &index);
            }
        }
        self.env.file = saved;
        self.phase_end(prof);
    }

    /// The packages the program's files declare, in the order of the files, each with the
    /// first file declaring it: a file's package clauses, not the packages enclosing them
    /// (`pkg_chain`).
    fn program_packages(&mut self) -> Vec<(PkgId, FileId)> {
        let mut out: Vec<(PkgId, FileId)> = Vec::new();
        for i in 0..self.files.len() {
            let f = FileId(i as u32);
            if self.source(f).is_std || self.in_jar(f) {
                continue;
            }
            self.env.file = f;
            let chain = self.pkg_chain();
            let own = self.ast(f).package_clauses.len().max(1);
            for &p in &chain[..own] {
                if !out.iter().any(|&(q, _)| q == p) {
                    out.push((p, f));
                }
            }
        }
        out
    }

    /// The classes the program's files define, local ones aside (those stand in bodies), in
    /// the order they were entered: the ones the signature phase completes and tables.
    fn program_classes(&self) -> Vec<ClassId> {
        (0..self.syms.classes.len())
            .map(|i| ClassId(i as u32))
            .filter(|&c| {
                let info = self.syms.class(c);
                info.owner != Owner::Local && !self.in_jar(info.file) && self.std.slot_of(info.file).is_none()
            })
            .collect()
    }

    /// The body phase of one worker: the walk over the files there are when it starts, in the
    /// order of their ids (`file_order`), each file's items typed (`check_file`) with what
    /// they demand of the signatures on the way. A file a body appends (a library class
    /// converted) is no part of the walk, as it never was.
    pub fn check_files(&mut self) {
        self.walk.reset(self.asts.len());
        for f in self.file_order() {
            self.walk.take(f);
            self.check_file(f);
        }
        self.walk.done = true;
    }

    /// The program files with quoted code, a macro's splices and implementations, in the walk's
    /// order: what is typed before the fork, as scalac compiles a
    /// macro's implementation before the program that expands it. A body a run reaches beyond
    /// them is typed on demand by the worker whose run needs it first, into its chunk, which every
    /// worker reads by id (`ensure_body`).
    pub fn quoted_files(&self, order: &[FileId]) -> Vec<FileId> {
        let n = self.files.len().min(self.asts.len());
        order.iter().copied().filter(|f| (f.0 as usize) < n && !self.source(*f).is_std && self.asts[f.0 as usize].has_quotes).collect()
    }

    /// What is typed on this thread before the fork, into the prefix: the std files (their
    /// top-level vals and givens are what every worker's interpreter and bodies read by id;
    /// the walk had them first anyway) and the files with quoted code (`prefix_reach`). A std file entered
    /// later is behind the walk and is checked where it enters, under the loader's lock.
    /// `order` is the walk's, taken before anything is typed, as one worker's walk takes it:
    /// a pseudo file a library body appends on the way is no part of it.
    pub fn check_before_fork(&mut self, order: &[FileId]) {
        let std_files: Vec<FileId> = order.iter().copied().filter(|&f| self.source(f).is_std).collect();
        for f in std_files {
            self.walk.take_for_prefix(f);
            self.check_file(f);
        }
        crate::measure::phase_done("prefix, the std files");
        for f in self.prefix_reach(order) {
            // A build that gives way to one worker stops here (`need_serial`).
            if self.serial.is_needed() {
                return;
            }
            if std::env::var_os("TEQ_MERGE_TRACE").is_some_and(|v| v == "print") {
                eprintln!("merge check: {} is typed before the fork", self.source(f).path);
            }
            self.walk.take_for_prefix(f);
            self.check_file(f);
        }
    }

    /// Types the items of the queue as one of its workers: whole files
    /// while there are files, then single items from the tail of the files other workers hold.
    pub fn work(&mut self, q: &Queue, first: Option<Reserved>) {
        crate::arena::set_thread_worker(self.worker);
        self.types.enter_overlay(self.worker);
        crate::types::view::worker_entered(&self.types);
        self.views_on = view_reads_wanted() && self.types.view_here() == View::Worker;
        if self.views_on {
            self.syms.view_records(true);
            self.prog.expr_types.shared_reads = Some(crate::types::expr_type_in_view);
            crate::interp::partition_by_view(true);
        }
        if crate::types::noting() {
            self.syms.classes.peer_reads = Some(crate::types::note_peer_class_record);
            if !self.views_on {
                self.prog.expr_types.shared_reads = Some(crate::types::expr_type_noted);
            }
        }
        crate::measure::work_begin(self.worker);
        self.walk.started = true;
        let mut first = first;
        loop {
            crate::shake::point(crate::shake::Point::Item);
            if self.serial.is_needed() {
                break;
            }
            match first.take().or_else(|| q.reserve(self.worker)) {
                Some(Reserved::File(fi, first)) => {
                    let file = q.files[fi].0;
                    self.walk.take(file);
                    let mut next = first;
                    while let Some(i) = next {
                        self.check_queued(file, i);
                        if self.serial.is_needed() {
                            break;
                        }
                        next = q.take_head(fi);
                    }
                }
                Some(Reserved::Item(fi, i)) => {
                    if std::env::var_os("TEQ_MERGE_TRACE").is_some_and(|v| v == "print") {
                        eprintln!("merge check: worker {} steals item {} of {}", self.worker, i, self.source(q.files[fi].0).path);
                    }
                    self.check_queued(q.files[fi].0, i);
                }
                None => break,
            }
        }
        self.walk.done = true;
        // The views and the signatures' copies are the worker's reads' alone: they go with its
        // work, the first worker's too, whose tables the merge goes on with.
        if crate::types::noting() {
            self.types.count_views(self.syms.view_counts());
        }
        #[cfg(debug_assertions)]
        if self.views_on {
            use crate::types::view::{crossed, Crossing};
            let [sigs, classes, tparams, aliases] = self.syms.view_counts();
            crossed(Crossing::PeerSignature, sigs.peer_reads);
            crossed(Crossing::PeerClass, classes.peer_reads);
            crossed(Crossing::PeerTypeParam, tparams.peer_reads);
            crossed(Crossing::PeerAlias, aliases.peer_reads);
            crossed(Crossing::PeerTranslated, sigs.peer_translated + classes.peer_translated + tparams.peer_translated + aliases.peer_translated);
        }
        self.views_on = false;
        self.syms.view_records(false);
        self.prog.expr_types.shared_reads = None;
        crate::interp::partition_by_view(false);
        self.other_view_memos = Default::default();
        self.types.leave_overlay();
        crate::measure::work_end();
        if self.worker != 0 {
            crate::measure::flush(self.worker);
        }
    }

    /// Types the item `i` of `file` and records the own records it made (`ItemRange`), which the
    /// merge places in the order of the sources, by the file and the item.
    fn check_queued(&mut self, file: FileId, i: u32) {
        // `TEQ_SERIAL_ITEMS=1` types one item at a time across the workers, the assignment
        // of items to workers unchanged: a failure that stays is one of state, one that goes
        // is a race between two items' typing.
        static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
        static SERIAL_ITEMS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        let serial = *SERIAL_ITEMS.get_or_init(|| std::env::var_os("TEQ_SERIAL_ITEMS").is_some());
        let _serial = serial.then(|| SERIAL.lock().unwrap_or_else(|e| e.into_inner()));
        // The working state of one solve or one match starts every item anew, whichever
        // worker types it and whatever that worker typed before.
        self.joined_soft = false;
        self.soft_scrutinee = None;
        self.ascribed_constant = None;
        self.refresh_std_if_stale();
        self.sync_arity_tables();
        let def = self.ast(file).top_level[i as usize];
        let start = self.own_marks();
        crate::types::view::item_typed();
        crate::measure::item_begin();
        self.check_item(Item { file, def });
        crate::measure::item_end(file.0, self.source(file).text.len() as u32, i);
        let end = self.own_marks();
        self.items.push(super::merge::ItemRange { key: (file.0, i), start, end });
    }

    /// The order the body phase takes the files in: by id for one worker; for several, the std
    /// files by id, then the program's largest first, ties by id. `TEQ_FILE_ORDER` (`id`,
    /// `reverse`, `size`, `seed:<n>`) names the order for the determinism test.
    pub fn file_order(&self) -> Vec<FileId> {
        let mut files: Vec<FileId> = (0..self.asts.len() as u32).map(FileId).collect();
        let mut order = std::env::var("TEQ_FILE_ORDER").unwrap_or_default();
        if order.is_empty() {
            // One worker walks the files by id, as the walk always did; several take the
            // largest first, the parser's rule.
            order = if self.walk_by_size { "size" } else { "id" }.to_string();
        }
        let rank = |f: FileId| -> (u8, u64, u32) {
            let src = self.source(f);
            if order == "id" {
                return (0, 0, f.0);
            }
            if src.is_std {
                return (0, 0, f.0);
            }
            match order.as_str() {
                "reverse" => (1, 0, u32::MAX - f.0),
                seed if seed.starts_with("seed:") => {
                    let seed: u64 = seed["seed:".len()..].parse().unwrap_or(1);
                    let x = (f.0 as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(seed).wrapping_mul(0xD1B5_4A32_D192_ED03);
                    (1, x ^ (x >> 29), f.0)
                }
                _ => (1, u64::MAX - src.text.len() as u64, f.0),
            }
        };
        files.sort_by_key(|&f| rank(f));
        files
    }

    /// The passes after the bodies, over the whole program.
    pub fn final_passes(&mut self) {
        let p = self.phase(Phase::Final);
        self.bind_mixin_supers();
        self.complete_library_trait_fields();
        self.once(|t| t.check_inherited_conflicts());
        self.decide_override_pairs();
        self.settle_val_accessors();
        self.name_all_alternatives();
        self.choose_entry_point();
        self.complete_program_mirrors(None);
        self.phase_end(p);
    }

    /// The vals, vars and givens of the traits read from TASTy (the products' and the jars')
    /// that the program's classes mix in, with their signatures: a JVM class lays out their
    /// fields and bridges their accessors by them (`jvm::classes`' `class_file_trait_fields`),
    /// referenced or not.
    fn complete_library_trait_fields(&mut self) {
        if !self.jvm {
            return;
        }
        let mut traits: Vec<ClassId> = Vec::new();
        let mut seen: std::collections::HashSet<ClassId, crate::intern::FxBuild> = Default::default();
        for i in 0..self.prog.classes.len() {
            let c = self.prog.classes[i].id;
            for &(b, _) in self.syms.class(c).base_types.iter().skip(1) {
                if self.syms.class(b).kind == ClassKind::Trait && self.is_library_class(b) && seen.insert(b) {
                    traits.push(b);
                }
            }
        }
        for b in traits {
            let members: Vec<SymId> = self.syms.class(b).member_order.clone();
            for m in members {
                if matches!(self.syms.sym(m).kind, SymKind::Val | SymKind::Var | SymKind::Given) && self.syms.sym(m).owner == Owner::Class(b) {
                    self.sig_of(m);
                }
            }
        }
    }

    /// `super.m` inside a trait is the `m` that follows the trait in the linearisation of each
    /// class that mixes it in. The first class of a chain to do so names it; what its subclasses
    /// add comes in front of the trait and changes nothing behind it. A class bound before is
    /// bound again only when the calls of one of its traits changed: the binding reads those
    /// and the classes' members, which a later typing does not change.
    pub fn bind_mixin_supers(&mut self) {
        if self.mixin_supers.is_empty() && self.shared_mixin_supers.lock().unwrap_or_else(|e| e.into_inner()).is_empty() {
            return;
        }
        // One thread binds them, after the workers' merge (the final passes, the reach, a
        // session's retype): a published `TClass` is not changed before the merge, and a
        // worker's interpreter asks the binding's rule instead (`mixin_super_target_of`).
        debug_assert!(!self.forked, "the mixins' super calls bound during the body phase");
        self.bind_mixin_supers_unlocked()
    }

    /// Notes that trait `c` calls `m` through `super`: a trait of the shared region, the
    /// program's or a library's, for every worker, whichever typed its body; a worker's own
    /// local trait for this worker until the merge.
    pub(super) fn note_mixin_super(&mut self, c: ClassId, m: SymId) {
        let mut shared = self.shared_class(c).then(|| self.shared_mixin_supers.lock().unwrap_or_else(|e| e.into_inner()));
        let calls = match &mut shared {
            Some(shared) => shared.entry(c).or_default(),
            None => self.mixin_supers.entry(c).or_default(),
        };
        if !calls.contains(&m) {
            calls.push(m);
        }
    }

    /// The member of class `b` a call of `member` through `super` may run: of an overloaded
    /// name, the alternative with the member's parameters; none private or abstract.
    fn concrete_super_member(&mut self, b: ClassId, member: SymId) -> Option<SymId> {
        let name = self.syms.sym(member).name;
        let m = *self.syms.class(b).members.get(&name)?;
        let alts: Option<Vec<SymId>> = self.syms.alternatives(m).map(|alts| alts.to_vec());
        let m = match alts {
            Some(alts) => alts.into_iter().find(|&a| self.identical_parameters(member, a))?,
            None => m,
        };
        (!self.is_private(m) && !self.is_abstract_member(m)).then_some(m)
    }

    /// The target of a call of `member` through `super` in the trait at `at` of a class's
    /// linearisation `bases`: the first class after the trait that has it concrete.
    fn mixin_super_target_in(&mut self, bases: &[ClassId], at: usize, member: SymId) -> Option<SymId> {
        bases[at + 1..].iter().copied().find_map(|b| self.concrete_super_member(b, member))
    }

    /// Where a call of `member` through `super` in trait `tr` goes for an instance of `class`,
    /// by the rule `bind_mixin_supers` binds it with, read from the class records alone: in the
    /// class of `class`'s superclass chain that mixes `tr` in, the class after `tr` that has the
    /// member concrete; `Some(None)` for a member of `Any` none has; `None` where nothing binds
    /// it. What the interpreter asks where the class's `TClass` has no binding: a class another
    /// worker checked, or one checked before the trait's body was typed, whose published record
    /// is not changed for it.
    pub fn mixin_super_target_of(&mut self, class: ClassId, tr: ClassId, member: SymId) -> Option<Option<SymId>> {
        let mut at = Some(class);
        let mut steps = 0;
        while let Some(k) = at {
            let info = self.syms.class(k);
            let superclass = info.superclass.filter(|_| info.kind != ClassKind::Trait);
            let bases: Vec<ClassId> = info.base_types.iter().map(|&(b, _)| b).collect();
            let inherited = superclass.is_some_and(|s| self.syms.class(s).base_types.iter().any(|&(b, _)| b == tr));
            if let Some(pos) = bases.iter().skip(1).position(|&b| b == tr).filter(|_| !inherited) {
                let name = self.syms.sym(member).name;
                let of_any = self.syms.sym(member).def.is_none() && matches!(name, names::TO_STRING | names::HASH_CODE | names::EQUALS | names::CLONE);
                return match self.mixin_super_target_in(&bases, pos + 1, member) {
                    Some(t) if self.syms.sym(t).kind != SymKind::Def => None,
                    Some(t) => Some(Some(t)),
                    None if of_any => Some(None),
                    None => None,
                };
            }
            at = superclass;
            steps += 1;
            if steps > self.syms.classes.len() {
                break;
            }
        }
        None
    }

    fn bind_mixin_supers_unlocked(&mut self) {
        let mut calls: FxMap<ClassId, Vec<SymId>> = self.mixin_supers.clone();
        for (&tr, theirs) in self.shared_mixin_supers.lock().unwrap_or_else(|e| e.into_inner()).iter() {
            let mine = calls.entry(tr).or_default();
            for &m in theirs {
                if !mine.contains(&m) {
                    mine.push(m);
                }
            }
        }
        let (upto, seen) = std::mem::take(&mut self.mixins_bound);
        let changed: FxMap<ClassId, ()> = calls.iter().filter(|&(tr, m)| seen.get(tr) != Some(m)).map(|(&tr, _)| (tr, ())).chain(seen.keys().filter(|tr| !calls.contains_key(tr)).map(|&tr| (tr, ()))).collect();
        let syms = &self.syms;
        let ids: Vec<u32> = self
            .prog
            .classes
            .entries()
            .enumerate()
            .filter(|&(k, (_, tc))| k >= upto || (!changed.is_empty() && syms.class(tc.id).base_types.iter().skip(1).any(|(b, _)| changed.contains_key(b))))
            .map(|(_, (i, _))| i)
            .collect();
        for i in ids {
            let i = i as usize;
            let c = self.prog.classes[i].id;
            let info = self.syms.class(c);
            if info.kind == ClassKind::Trait || !info.base_types.iter().skip(1).any(|(b, _)| calls.contains_key(b)) {
                continue;
            }
            let bases: Vec<ClassId> = info.base_types.iter().map(|&(b, _)| b).collect();
            let inherited: Vec<ClassId> =
                info.superclass.map_or(Vec::new(), |s| self.syms.class(s).base_types.iter().map(|&(b, _)| b).collect());
            let mut accessors = Vec::new();
            for (at, &tr) in bases.iter().enumerate().skip(1) {
                if inherited.contains(&tr) {
                    continue;
                }
                let Some(members) = calls.get(&tr).cloned() else { continue };
                for member in members {
                    let name = self.syms.sym(member).name;
                    let concrete = |t: &mut Self, b: ClassId| t.concrete_super_member(b, member);
                    let target = self.mixin_super_target_in(&bases, at, member);
                    // A class without any implementation has been told that it needs one.
                    let implemented = bases.iter().copied().any(|b| match concrete(self, b) {
                        Some(m) => !self.is_abstract_override(m),
                        None => false,
                    });
                    // The binding is redone as library traits are typed; a problem is told once.
                    let told = |t: &mut Self| !t.super_problems_told.insert((c, member), ()).is_none();
                    let of_any = self.syms.sym(member).def.is_none()
                        && matches!(name, names::TO_STRING | names::HASH_CODE | names::EQUALS | names::CLONE);
                    match target {
                        Some(t) if self.syms.sym(t).kind != SymKind::Def => {
                            let Owner::Class(owner) = self.syms.sym(t).owner else { continue };
                            let msg = format!(
                                "parent {} has a super call which binds to the value {}.{}. Super calls can only target methods.",
                                self.class_description(tr),
                                self.name_str(self.syms.class(owner).name),
                                self.name_str(name)
                            );
                            let (file, span) = (self.syms.class(c).file, self.syms.class(c).span);
                            if !told(self) {
                                self.diags.error(file, span, msg);
                            }
                        }
                        Some(_) => accessors.push(SuperAccessor { of_trait: tr, member, target }),
                        None if of_any => accessors.push(SuperAccessor { of_trait: tr, member, target }),
                        None if !implemented => {}
                        None => {
                            let msg = format!(
                                "{} is accessed from super in {}; {} has no concrete {} behind that trait",
                                self.member_description(member, None),
                                self.class_description(tr),
                                self.class_description(c),
                                self.name_str(name)
                            );
                            let (file, span) = (self.syms.class(c).file, self.syms.class(c).span);
                            if !told(self) {
                                self.diags.error(file, span, msg);
                            }
                        }
                    }
                }
            }
            // Written only where they changed: a shared record is copied to be changed.
            if self.prog.classes[i].super_accessors != accessors {
                self.prog.classes[i].super_accessors = accessors;
            }
        }
        self.mixins_bound = (self.prog.classes.len(), calls);
    }

    /// The work items of a file, in source order.
    pub fn items_of(&self, file: FileId) -> impl Iterator<Item = Item> + 'a {
        let top: &'a [DefId] = &self.ast(file).top_level;
        top.iter().map(move |&def| Item { file, def })
    }

    /// Types the items of a file, in order.
    pub(super) fn check_file(&mut self, file: FileId) {
        for item in self.items_of(file) {
            self.check_item(item);
        }
    }

    /// Types one work item: the whole of a top-level definition (`Item`).
    pub fn check_item(&mut self, item: Item) {
        let pkg = self.file_pkgs[item.file.0 as usize];
        self.env.file = item.file;
        let frame = self.var_frame_begin();
        self.check_top_def(item.file, Owner::Package(pkg), item.def);
        self.var_frame_end(frame);
    }

    /// A `@main` method wins over `main(args: Array[String])` methods of objects; among several
    /// of one kind the one named with `--main` runs, and without it the program is rejected as
    /// scala-cli rejects it. An entry is the method and, for a `main` an object inherits, the
    /// object it runs on.
    pub(super) fn choose_entry_point(&mut self) {
        if !self.choose_entry {
            return;
        }
        let mut candidates: Vec<(SymId, Option<ClassId>)> =
            self.entry_points.iter().copied().filter(|&(s, _)| self.syms.sym(s).is_main).collect();
        // A name chooses among every entry point, the `main` an object inherits included.
        if candidates.is_empty() || self.main_name.is_some() {
            candidates = self.entry_points.clone();
        }
        if let Some(name) = self.main_name {
            // A qualified name is the entry point's class as `java` runs it (`app.Main`, or
            // `app.run` for a `@main def run`).
            let qualified = self.name_str(name).contains('.').then(|| self.name_str(name).to_string());
            let named: Vec<(SymId, Option<ClassId>)> = candidates
                .iter()
                .copied()
                .filter(|&(s, object)| {
                    let info = self.syms.sym(s);
                    let object_class = match (object, info.owner) {
                        (Some(c), _) | (None, Owner::Class(c)) => Some(c),
                        _ => None,
                    };
                    match &qualified {
                        Some(q) => &self.entry_point_class(s, object) == q,
                        // The simple name, the source's or the class's (`hello$minusworld` for a
                        // `@main def hello-world`, `Main$package` for a package's `main`).
                        None => {
                            info.name == name
                                || object_class.map(|c| self.syms.class(c).name) == Some(name)
                                || self.entry_point_class(s, object).rsplit('.').next() == Some(self.name_str(name).as_str())
                        }
                    }
                })
                .collect();
            if named.is_empty() {
                let msg = format!("no entry point named {}", self.name_str(name));
                let first = candidates.first().map(|&(s, object)| self.entry_point_site(s, object));
                match first {
                    None => eprintln!("error: {}", msg),
                    Some((file, span)) => self.diags.error(file, span, msg),
                }
                return;
            }
            candidates = named;
        }
        match candidates.as_slice() {
            [] => {}
            [(one, object)] => {
                self.prog.main = Some(*one);
                self.prog.main_object = *object;
            }
            many => {
                let names: Vec<String> = many.iter().map(|&(s, object)| self.entry_point_name(s, object)).collect();
                let msg = format!("several entry points: {}; choose one with --main", names.join(", "));
                let (file, span) = self.entry_point_site(many[0].0, many[0].1);
                self.diags.error(file, span, msg);
            }
        }
    }

    /// Whether the members of `c` are reached statically, as a `@main` method's proxy calls
    /// one: `c` is an object of a package or nested in such objects alone.
    fn statically_accessible(&self, c: ClassId) -> bool {
        let mut k = c;
        loop {
            let info = self.syms.class(k);
            if info.kind != ClassKind::Object || info.local_module.is_some() || info.inner_object.is_some() {
                return false;
            }
            match info.owner {
                Owner::Package(_) => return true,
                Owner::Class(o) => k = o,
                Owner::Local => return false,
            }
        }
    }

    /// The class `java` runs for the entry point `sym`, by its qualified name: the object's
    /// (`app.Main`), the one a `@main` method makes in its package (`app.run`, the method's name
    /// encoded as the class's: `hello$minusworld`), or, for a package's own `main`, its file's
    /// `<stem>$package`.
    pub(crate) fn entry_point_class(&self, sym: SymId, object: Option<ClassId>) -> String {
        let info = self.syms.sym(sym);
        let in_package = |t: &Self, p: PkgId, name: &str| if p == ROOT_PKG { name.to_string() } else { format!("{}.{}", t.pkg_description(p), name) };
        match (object, info.owner) {
            (Some(c), _) => self.class_path(c),
            (None, Owner::Class(c)) if info.is_main => {
                let mut owner = Owner::Class(c);
                while let Owner::Class(k) = owner {
                    owner = self.syms.class(k).owner;
                }
                let p = match owner {
                    Owner::Package(p) => p,
                    _ => ROOT_PKG,
                };
                in_package(self, p, &crate::jvm::names::encode(&self.name_str(info.name)))
            }
            (None, Owner::Class(c)) => self.class_path(c),
            (None, Owner::Package(p)) if info.is_main => in_package(self, p, &crate::jvm::names::encode(&self.name_str(info.name))),
            (None, Owner::Package(p)) => {
                let stem = crate::tasty::write::file_stem(&self.source(info.file).path);
                in_package(self, p, &format!("{}$package", crate::jvm::names::encode(&stem)))
            }
            (None, Owner::Local) => self.name_str(info.name),
        }
    }

    fn entry_point_site(&self, sym: SymId, object: Option<ClassId>) -> (FileId, Span) {
        match object {
            Some(c) => (self.syms.class(c).file, self.syms.class(c).span),
            None => (self.syms.sym(sym).file, self.syms.sym(sym).span),
        }
    }

    fn entry_point_name(&self, sym: SymId, object: Option<ClassId>) -> String {
        let info = self.syms.sym(sym);
        match (object, info.owner) {
            (Some(c), _) | (None, Owner::Class(c)) => format!("{}.{}", self.name_str(self.syms.class(c).name), self.name_str(info.name)),
            (None, Owner::Package(_)) if !info.is_main => {
                let class = self.entry_point_class(sym, object);
                format!("{}.{}", class.rsplit('.').next().unwrap_or(&class), self.name_str(info.name))
            }
            _ => self.name_str(info.name),
        }
    }

    /// A concrete `main(args: Array[String]): Unit` that the object `c` inherits from a trait or
    /// class of the program, the std or a jar: the object's entry point, as the JVM's static
    /// forwarder makes it for scalac.
    fn inherited_java_main(&mut self, c: ClassId) -> Option<SymId> {
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(bc, _)| bc).collect();
        for bc in bases {
            let mut k = 0;
            while let Some(m) = self.own_alternative(bc, names::MAIN, k) {
                k += 1;
                if self.is_java_main(m) && !self.is_abstract_member(m) {
                    return Some(m);
                }
            }
        }
        None
    }

    /// `def main(args: Array[String]): Unit`, the entry point of the JVM.
    fn is_java_main(&mut self, sym: SymId) -> bool {
        if self.syms.sym(sym).kind != SymKind::Def || self.syms.sym(sym).is_extension {
            return false;
        }
        // The signature and `Array[String]` both in the reader's view, compared by id.
        let sig = self.sig_arc(sym);
        let [clause] = sig.clauses.as_slice() else { return false };
        let [param] = clause.params.as_slice() else { return false };
        let array_of_string = self.types.class(self.b.array, &[self.b.t_string]);
        sig.tparams.is_empty()
            && !clause.is_using
            && !param.repeated
            && !param.by_name
            && param.ty == array_of_string
            && sig.ret == self.b.t_unit
    }

    /// A `@main` method runs with the command line: without parameters, or with them as `String*`.
    fn enter_main_method(&mut self, sym: SymId) {
        let sig = self.sig_arc(sym);
        let params: Vec<&ParamSig> = sig.clauses.iter().flat_map(|c| c.params.iter()).collect();
        let takes_strings = match params.as_slice() {
            [] => true,
            [p] => p.repeated && !p.by_name && p.ty == self.b.t_string,
            _ => false,
        };
        if !takes_strings || sig.clauses.iter().any(|c| c.is_using) {
            let (file, span) = {
                let s = self.syms.sym(sym);
                (s.file, s.span)
            };
            self.diags.error(file, span, "a @main method takes no parameters or a single String*");
            return;
        }
        self.entry_points.push((sym, None));
    }

    pub(super) fn check_top_def(&mut self, file: FileId, owner: Owner, id: DefId) {
        self.restrict_def(file, id);
        if self.unused.on() {
            self.mark_annotations(file, id);
            self.mark_module_prefixes(file, id, owner, true);
        }
        let def = self.ast(file).def(id);
        let lacks_body = match &def.kind {
            DefKind::Val { rhs, .. } => rhs.is_none(),
            DefKind::Fun(f) => f.body.is_none() && !def.annots.iter().any(|a| a.name == names::JS || (a.name == names::JVM && self.jvm)),
            _ => false,
        };
        // Both interop annotations report a missing body in their own terms.
        let is_interop = |a: &ast::Annot| a.name == names::JS_IMPORT || a.name == names::JS_EXPORT;
        let misplaced = self.misplaced_abstract_given(def);
        if lacks_body && !misplaced && !def.annots.iter().any(is_interop) {
            let msg = format!(
                "{} needs a body; only @js and @jsImport definitions go without one",
                self.name_str(def.name)
            );
            // An incomplete definition's body is unknown, not missing.
            if def.mods & mods::INCOMPLETE != 0 {
                self.dependent_error(def.span, msg);
            } else {
                self.error(def.span, msg);
            }
        }
        match &def.kind {
            DefKind::Val { .. } => {
                if let Some(&sym) = self.def_syms.get(file.0 as usize, &id) {
                    self.walk_body(sym);
                    if let Some(init) = self.readable_init(sym) {
                        self.prog.top_vals.push((sym, init));
                    }
                }
            }
            DefKind::Fun(_) => {
                if let Some(&sym) = self.def_syms.get(file.0 as usize, &id) {
                    if !self.defers_body(sym) {
                        self.walk_body(sym);
                    }
                    if let Some(f) = self.readable_fun(sym) {
                        self.register_top_fun(f);
                    }
                    if self.syms.sym(sym).is_main {
                        self.enter_main_method(sym);
                    } else if self.syms.sym(sym).name == names::MAIN && matches!(owner, Owner::Package(_)) && self.is_java_main(sym) {
                        // A package's own `main(args: Array[String])`, which `java` runs through
                        // the static forwarder of its file's `<stem>$package`, as scalac finds it.
                        self.entry_points.push((sym, None));
                    }
                }
            }
            DefKind::Class(_) => {
                // A class of the standard library is checked when the program reaches it
                // (`emit::reach`) or a macro instantiates it, as its methods are typed.
                if let Some(&c) = self.def_classes.get(file.0 as usize, &id).filter(|_| !self.source(file).is_std) {
                    self.check_class(c);
                }
            }
            DefKind::Given(_) => {
                let Some(&sym) = self.def_syms.get(file.0 as usize, &id) else { return };
                self.check_given(sym);
                if let Some(init) = self.readable_init(sym) {
                    self.prog.top_vals.push((sym, init));
                }
                if let Some(f) = self.readable_fun(sym) {
                    self.register_top_fun(f);
                }
            }
            DefKind::TypeAlias { .. } => {}
        }
        if self.capturing() && !matches!(def.kind, DefKind::Class(_)) {
            self.capture_def_annotations(file, owner, id, true);
        }
    }

    /// Types a given. Parameterless structural givens become a value holding the instance, which
    /// under the fork is the given's body cell's, as a val's initialiser is: one worker makes and
    /// publishes it, another that reads the given (a macro's run) waits for it (`walk_body`). One
    /// with type or term parameters is a def making an instance per call (dotty's
    /// `Parsers.givenDef`).
    fn check_given(&mut self, sym: SymId) {
        let parameterless = {
            let sig = self.sig_of(sym);
            sig.clauses.is_empty() && sig.tparams.is_empty()
        };
        match self.syms.sym(sym).impl_class {
            Some(impl_class) => {
                self.check_class(impl_class);
                if parameterless && !self.val_init.contains_key(&sym) {
                    let cell = self.forked && self.shared_sym(sym);
                    if cell && !self.syms.body_cells.claim(sym.0) {
                        if !self.syms.sym(sym).body().mine() {
                            self.wait_cell(CellKey::new(CELL_BODY, sym.0), |w| w.syms.sym(sym).body() == Completion::Done);
                        }
                        return;
                    }
                    // One of a class or trait instance is made in its initialiser, of `this`.
                    let args = if self.outer_class(impl_class).is_some() {
                        let this = self.prog.add(TExpr::This);
                        self.prog.list(&[this])
                    } else {
                        ast::ListRef::EMPTY
                    };
                    let init = self.prog.add(TExpr::New(impl_class, args));
                    if self.capturing() {
                        self.settle_class(impl_class);
                        let own: Vec<TypeId> = self.syms.class(impl_class).tparams.iter().map(|&p| self.types.param(p)).collect();
                        self.capture_targs(init, &own);
                        self.capture_form(init, crate::tir::capture::Form::GivenCall(sym));
                    }
                    self.val_init.insert(sym, init);
                    if cell {
                        self.publish_body(sym);
                    }
                }
            }
            None => {
                if !self.defers_body(sym) {
                    self.walk_body(sym);
                }
            }
        }
    }

    /// A method of the standard library is typed when the program reaches it (`emit::reach`), the
    /// library being trusted; a program's methods are typed whether they are used or not. The
    /// methods of an anonymous class are typed with it, since what they capture is settled then.
    pub(super) fn defers_body(&mut self, sym: SymId) -> bool {
        let local = matches!(self.syms.sym(sym).owner, Owner::Class(c) if self.syms.class(c).owner == Owner::Local);
        self.in_std(sym) && !local && self.is_method_sym(sym)
    }

    /// The body of a definition the reach pass asked for: a method of the standard library, or
    /// a method or val of a library on the classpath, whose class is compiled from its TASTy
    /// first.
    pub fn deferred_body(&mut self, sym: SymId) -> Option<DeferredBody> {
        let p = self.phase(Phase::Deferred);
        // The body is typed on its own, not inside the inline expansion that asked for it
        // (a macro run by the interpreter).
        let sites = std::mem::take(&mut self.inline.sites);
        // A product's top-level definition, a given object the loader made among them.
        let product_top = self.loaded.as_ref().map_or(false, |l| l.product_package_members.contains_key(&sym));
        let body = if product_top {
            self.library_body(sym)
        } else if self.is_library_member(sym) {
            // A jar method whose signature holds a type of an unsupported shape has no call
            // site the typer accepts, so a reach by its name alone leaves it out.
            if self.syms.sym(sym).kind == SymKind::Def && self.blocked_in_sig(sym).is_some() {
                None
            } else {
                self.library_body(sym)
            }
        } else {
            let outer = std::mem::replace(&mut self.for_interpreter, self.jvm);
            let body = self.std_body(sym).map(DeferredBody::Fun);
            self.for_interpreter = outer;
            body
        };
        self.inline.sites = sites;
        self.phase_end(p);
        body
    }

    /// A top-level function is registered once, whichever of the program's check, a std file's
    /// or the reach pass (a macro's run in the interpreter among them) gets to it first.
    fn register_top_fun(&mut self, f: FunId) {
        if !self.prog_index.has_top_fun(&self.prog, f) {
            self.prog.top_funs.push(f);
        }
    }

    /// Types the body of a method of the standard library that the program reaches, and registers
    /// the function with its class or the top level as the check of a program method does.
    pub fn std_body(&mut self, sym: SymId) -> Option<FunId> {
        // A program method is typed outside the loader's lock, into the typing worker's chunk,
        // which every worker's interpreter reads; the std's under it, into the shared
        // region.
        if self.forked && self.program_file(self.syms.sym(sym).file) {
            return self.program_body(sym);
        }
        self.with_loader_for(crate::measure::Hold::Body, |w| w.std_body_unlocked(sym))
    }

    /// A program method the interpreter runs, typed where it is not yet and registered as the
    /// walk registers it: at the top level, or with the class check that is this worker's; the
    /// check of another worker registers its methods itself.
    fn program_body(&mut self, sym: SymId) -> Option<FunId> {
        self.ensure_body(sym);
        let f = self.readable_fun(sym)?;
        match self.syms.sym(sym).owner {
            Owner::Package(_) => self.register_top_fun(f),
            Owner::Class(c) => {
                if let Some(i) = self.prog_index.class(&self.prog, c) {
                    let tc = &self.prog.classes[i];
                    if !tc.methods.contains(&f) && !tc.ctors.contains(&f) {
                        if self.tclass_unpublished(i) {
                            self.prog.classes[i].methods.push(f);
                        } else {
                            self.late_methods.push((c, f));
                        }
                    }
                }
            }
            Owner::Local => {}
        }
        Some(f)
    }

    /// The vals a file defines at its top level, in source order.
    pub fn top_level_vals(&self, file: FileId) -> Vec<SymId> {
        let ast = self.ast(file);
        ast.top_level.iter().filter(|&&d| matches!(ast.def(d).kind, DefKind::Val { .. })).filter_map(|d| self.def_syms.get(file.0 as usize, d).copied()).collect()
    }

    fn std_body_unlocked(&mut self, sym: SymId) -> Option<FunId> {
        self.ensure_body(sym);
        let f = self.readable_fun(sym)?;
        match self.syms.sym(sym).owner {
            Owner::Package(_) => self.register_top_fun(f),
            // A class still being checked registers the method itself when it gets there; one
            // checked just before, by the reach pass, has it already.
            Owner::Class(c) => {
                let i = self.prog_index.class(&self.prog, c);
                if let Some(i) = i {
                    let tc = &mut self.prog.classes[i];
                    if !tc.methods.contains(&f) && !tc.ctors.contains(&f) {
                        tc.methods.push(f);
                    }
                }
            }
            Owner::Local => {}
        }
        Some(f)
    }

    /// Checks a class of the program that a macro's run met before the check got to it, in the
    /// scope of its own file and outside the expansion that runs the macro: what its body makes
    /// (an anonymous class) is its file's, whichever expansion asked first.
    pub fn check_class_for_run(&mut self, c: ClassId) {
        let (file, owner) = (self.syms.class(c).file, self.syms.class(c).owner);
        let env = self.env_for(file, owner);
        self.outside_inline(|t| t.with_env(env, |t| t.check_class(c)));
    }

    /// Whether the check of `c` has begun and not ended where the asking worker cannot wait for
    /// it: on this worker, which asks from inside the check, or on another whose wait for this
    /// one would close a cycle. The class has no IR yet: `class_done` comes with it.
    pub fn check_under_way(&self, c: ClassId) -> bool {
        self.syms.check_cells.get(c.0) == Completion::InProgress
    }

    /// Checks a class of the standard library the reach pass met without its `TClass`: as
    /// `check_all` would have, with the names of its overloaded members and its bridges settled
    /// as the final passes settled the program's.
    pub fn check_std_class(&mut self, c: ClassId) {
        if self.class_done.contains_key(&c) {
            return;
        }
        let p = self.phase(Phase::Deferred);
        self.outside_inline(|t| t.check_class(c));
        // The alternatives of an overloaded name are named where the name is declared: the
        // bases of a class entered after the final passes are named with it.
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().rev().map(|&(b, _)| b).collect();
        for b in bases {
            if !self.in_jar(self.syms.class(b).file) {
                self.name_library_members(b);
            }
        }
        self.bridge_class(c);
        self.phase_end(p);
    }

    /// A member of `Product`, whose defaults a class implements without `override`, as the
    /// abstract members of scalac's `Product` are.
    fn is_product_member(&self, s: SymId) -> bool {
        matches!(self.syms.sym(s).owner, Owner::Class(c) if Some(c) == self.b.product)
    }

    /// The signature of `sym`, completed on the way, as a borrow for the reads that follow at
    /// once; a caller that keeps it while the worker changes underneath takes `sig_arc`, one that
    /// reads other fields of the worker meanwhile `syms.sig` after this call.
    pub fn sig_of(&mut self, sym: SymId) -> &MethodSig {
        self.completed_sig(sym).map_or(&ERROR_SIG, |sig| &**sig)
    }

    /// The signature of `sym` as a clone of the table's `Arc`; counted under `--profile`.
    pub fn sig_arc(&mut self, sym: SymId) -> Arc<MethodSig> {
        let sig = self.completed_sig(sym).map_or_else(|| Arc::new(MethodSig::value(ERROR)), Arc::clone);
        if self.profile.on {
            self.profile.sig_cloned(&sig);
        }
        sig
    }

    /// Completes the signature of `sym` where it is not yet and borrows it from the table: `None`
    /// for a recursive reference to a definition whose result type is being inferred, reported
    /// here, which reads as an error value.
    #[inline]
    fn completed_sig(&mut self, sym: SymId) -> Option<&Arc<MethodSig>> {
        if self.profile.sig_hook {
            return self.completed_sig_hooked(sym);
        }
        if self.syms.sym_cells.get(sym.0) == Completion::Done {
            return self.syms.sym(sym).info.sig.as_ref();
        }
        self.uncompleted_sig(sym)
    }

    /// `completed_sig` with its reads hooked: counted under `--profile` and noted as the routes'
    /// crossings (`TEQ_WORKERS_TYPES=1`). With the overlays on the table's read is the reader's
    /// view already: a shared symbol's and a peer's through the views of their records, the
    /// worker's own as it is, and under the loader's lock the holder's of its own worker's
    /// exported (`Symbols::view_records`).
    #[cold]
    #[inline(never)]
    fn completed_sig_hooked(&mut self, sym: SymId) -> Option<&Arc<MethodSig>> {
        self.sig_read_noted(sym);
        if self.views_on && crate::types::noting() {
            self.types.count_sig_read(|c| c.reads += 1);
        }
        if self.syms.sym_cells.get(sym.0) == Completion::Done {
            return self.syms.sym(sym).info.sig.as_ref();
        }
        self.uncompleted_sig(sym)
    }

    /// The partial signature of `s`, whose parameters are known while its result is inferred
    /// (`parameters_of`), read as `completed_sig_hooked` reads a complete one.
    #[cold]
    #[inline(never)]
    pub(super) fn partial_sig_hooked(&mut self, s: SymId) -> &Arc<MethodSig> {
        self.syms.sym(s).info.sig.as_ref().expect("a partial signature")
    }

    #[cold]
    #[inline(never)]
    fn sig_read_noted(&mut self, sym: SymId) {
        if self.profile.on {
            self.profile.sig_read();
            self.note_sig_read(sym);
        }
        #[cfg(debug_assertions)]
        if self.syms.sym_cells.get(sym.0) == Completion::Done {
            if let Some(sig) = &self.syms.sym(sym).info.sig {
                for t in sig_types(sig) {
                    crate::types::view::origin("a signature (sig_of, sig_arc)", t);
                }
            }
        }
        if crate::types::noting() && self.syms.sym_cells.get(sym.0) == Completion::Done {
            if let Some(sig) = &self.syms.sym(sym).info.sig {
                let types: Vec<TypeId> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)).chain([sig.ret]).collect();
                self.types.note_signature(sym.0, &types);
            }
        }
    }

    /// The signature's completion, or the wait for it, as a call apart from the check that
    /// most reads end at.
    #[inline(never)]
    fn uncompleted_sig(&mut self, sym: SymId) -> Option<&Arc<MethodSig>> {
        let (state, file, def_id, owner, kind, span) = {
            let s = self.syms.sym(sym);
            (s.state().get(), s.file, s.def, s.owner, s.kind, s.span)
        };
        // A signature of the std or a library is claimed and completed within one hold of the
        // loader's lock, whose holder then never meets another thread's claim;
        // outside the lock, a claimed one is waited for.
        if self.forked && self.shared_sym(sym) && state != Completion::Done && crate::shared::lock_depth() == 0 && !self.program_file(file) {
            if state == Completion::InProgress {
                self.wait_cell(CellKey::new(CELL_SIG, sym.0), |w| w.syms.sym(sym).state() == Completion::Done);
            } else {
                self.with_loader_for(crate::measure::Hold::Signature, |w| {
                    w.uncompleted_sig(sym);
                });
            }
            return self.syms.sym(sym).info.sig.as_ref();
        }
        // The cell: claimed by the first thread to find it
        // unclaimed, which completes it; another thread finding it claimed waits for the
        // signature, unless the wait would close a cycle, which reads as the recursion it is on one
        // thread.
        let state = match state {
            Completion::NotStarted if self.syms.sym_cells.claim(sym.0) => Completion::NotStarted,
            Completion::NotStarted => Completion::InProgress,
            other => other,
        };
        match state {
            Completion::Done => {}
            Completion::InProgress if !self.syms.sym(sym).state().mine() && self.wait_cell(CellKey::new(CELL_SIG, sym.0), |w| w.syms.sym(sym).state() == Completion::Done) => {}
            Completion::InProgress => {
                if !self.syms.sym(sym).sig.as_ref().is_some_and(|sig| sig.ret != ERROR) {
                    // Which member of the cycle reports it is the member the walk met first,
                    // which several workers' walks do not keep to.
                    self.need_serial(|| format!("a recursive signature ({})", self.name_str(self.syms.sym(sym).name)));
                    let msg = match kind {
                        SymKind::Val | SymKind::Var => {
                            format!("recursive value {} needs type", self.name_str(self.syms.sym(sym).name))
                        }
                        _ => format!(
                            "recursive use of {} needs an explicit result type",
                            self.name_str(self.syms.sym(sym).name)
                        ),
                    };
                    self.error(span, msg);
                    return None;
                }
            }
            Completion::NotStarted => {
                if self.forked {
                    crate::shake::point(crate::shake::Point::Claimed);
                }
                if self.profile.on && self.walk.taken_elsewhere(file) {
                    self.profile.stolen += 1;
                }
                let p = self.phase_split(Phase::Signature, Phase::LibrarySignature, |t| t.is_library_member(sym));
                self.complete_sig(sym, file, def_id, owner, kind, span);
                self.phase_end(p);
            }
        }
        self.syms.sym(sym).info.sig.as_ref()
    }

    fn complete_sig(&mut self, sym: SymId, file: FileId, def_id: Option<DefId>, owner: Owner, kind: SymKind, span: Span) -> Arc<MethodSig> {
        self.completing += 1;
        // A retype completes again the signatures it opened, those with an inferred result,
        // and with its body what a block defines; any other signature is resolved once.
        let outer = self.diags.of_bodies;
        let inferred = !self.made_in_block(owner) && self.result_inferred(file, def_id, owner, kind);
        if !self.made_in_block(owner) {
            self.diags.of_bodies = inferred;
        }
        let mark = self.failure_mark();
        let promoted = self.promote_begin(sym);
        // A signature of the std or a library is completed under the loader's lock, into the
        // shared region; a program member's outside it, into this
        // worker's chunk, which the other workers read by id, its cell claimed already and
        // published with the signature (the lock is held by work that never waits on a cell).
        let sig = if self.forked && self.shared_sym(sym) && !self.program_file(file) {
            self.with_loader_for(crate::measure::Hold::Signature, |w| w.complete_sig_inner(sym, file, def_id, owner, kind, span))
        } else {
            self.complete_sig_inner(sym, file, def_id, owner, kind, span)
        };
        if inferred && self.failed_since(mark) {
            self.inference_failed.insert(sym, ());
        }
        self.promote_end(promoted);
        self.diags.of_bodies = outer;
        self.leave_completion();
        if super::prep::capturing() {
            self.prep_note_member('S', sym);
        }
        sig
    }

    /// Whether the definition is a val, var or def outside blocks whose type is left to
    /// inference: what `open_inferred` opens for a retype.
    pub(super) fn result_inferred(&self, file: FileId, def_id: Option<DefId>, owner: Owner, kind: SymKind) -> bool {
        let Some(d) = def_id else { return false };
        if owner == Owner::Local || !matches!(kind, SymKind::Val | SymKind::Var | SymKind::Def) {
            return false;
        }
        match &self.ast(file).def(d).kind {
            DefKind::Val { ty, .. } => ty.is_none(),
            DefKind::Fun(f) => f.ret.is_none(),
            _ => false,
        }
    }

    /// Types the body a signature's result type is inferred from, where the signature is
    /// completed: a program member's outside the loader's lock into this worker's chunk, the
    /// body of another worker's file among them (a steal), which that
    /// worker's walk then finds typed; a std member's under the lock into the shared region.
    fn infer_body(&mut self, sym: SymId, sig: &MethodSig, expected: Option<TypeId>) -> TypeId {
        crate::measure::typing_inferred(true);
        self.type_body(sym, sig, expected)
    }

    fn complete_sig_inner(&mut self, sym: SymId, file: FileId, def_id: Option<DefId>, owner: Owner, kind: SymKind, span: Span) -> Arc<MethodSig> {
        let Some(def_id) = def_id else {
            match (kind, owner) {
                (SymKind::EnumValue(c), _) => self.complete_class(c),
                (_, Owner::Class(c)) => self.complete_class(c),
                _ => {}
            }
            if let Some(sig) = self.syms.sym(sym).sig.clone() {
                self.syms.sym(sym).state().set(Completion::Done);
                return sig;
            }
            if self.syms.sym(sym).mods & mods::SETTER != 0 && !self.is_library_member(sym) {
                let sig = Arc::new(self.setter_sig(sym));
                self.publish_sig(sym, sig.clone());
                self.outermost_completion_done();
                return sig;
            }
            return self.with_loader(|w| {
                let sig = w.loaded_sig(sym).unwrap_or_else(|| Arc::new(MethodSig::value(ERROR)));
                w.publish_sig(sym, sig.clone());
                sig
            });
        };
        if let Owner::Class(c) = owner {
            self.complete_class(c);
        }
        let def = self.ast(file).def(def_id);
        let sig = if owner == Owner::Local {
            self.compute_sig(sym, def)
        } else {
            let env = self.env_at(file, owner, span.start);
            if self.made_in_block(owner) {
                self.with_env(env, |t| t.compute_sig(sym, def))
            } else {
                // The declared types name the class's type parameters and `this`, which are
                // the definition's and not the receiver's of the expansion that was the first
                // to ask for the signature.
                self.outside_inline(|t| t.with_env(env, |t| t.compute_sig(sym, def)))
            }
        };
        let sig = Arc::new(sig);
        self.publish_sig(sym, sig.clone());
        // What waited for the outermost completion runs once this signature is done.
        self.outermost_completion_done();
        sig
    }

    /// Whether `owner` is a block, a class made in one (an anonymous class among them) or a
    /// class nested in such a class: what it defines is typed where the block is, inside the
    /// expansion under way if the block is an inline method's. Any other definition's body is
    /// typed on its own, whatever asked for it (`outside_inline`).
    pub(super) fn made_in_block(&self, mut owner: Owner) -> bool {
        loop {
            match owner {
                Owner::Local => return true,
                Owner::Class(c) => owner = self.syms.class(c).owner,
                Owner::Package(_) => return false,
            }
        }
    }

    /// Fills the cell of `sym` with its signature and publishes it, for every worker to read.
    /// A type variable this worker instantiated is replaced by its instance first: another
    /// worker cannot follow it (`TVars`).
    fn publish_sig(&mut self, sym: SymId, sig: Arc<MethodSig>) {
        let sig = if self.forked && self.shared_sym(sym) { self.zonked_sig(sig) } else { sig };
        #[cfg(debug_assertions)]
        if self.forked && self.shared_sym(sym) && crate::types::view::probing() {
            let local = |t: TypeId| self.types.mentions_local(t);
            if local(sig.ret) || sig.clauses.iter().any(|c| c.params.iter().any(|p| local(p.ty))) {
                crate::types::view::probe(|| format!("the published signature of {} names a worker's id, which the merge renumbers: {} on worker {}", self.name_str(self.syms.sym(sym).name), self.show(sig.ret), self.worker));
            }
            if self.tvars.len() as u32 > self.tvars_at_fork && self.mentions_var_from(sig.ret, self.tvars_at_fork) {
                crate::types::view::probe(|| format!("the published signature of {} holds an open type variable of the body phase", self.name_str(self.syms.sym(sym).name)));
            }
        }
        if self.shared_sym(sym) {
            // The signature, its parameters and the `Done` state become every worker's
            // together, when the lock is released (`lock_released`).
            if crate::shared::lock_depth() == 0 {
                crate::shake::point(crate::shake::Point::Computed);
            }
            self.with_loader_for(crate::measure::Hold::Publication, |w| {
                let sig = w.published_sig(sig, "a shared symbol's signature (publish_sig)");
                w.syms.sym_mut(sym).sig = Some(sig);
                w.publish_class_bodies();
                w.syms.sym(sym).state().set(Completion::Done);
            });
        } else {
            let mut s = self.syms.sym_mut(sym);
            s.sig = Some(sig);
            s.state().set(Completion::Done);
        }
    }

    /// A signature published into a shared symbol, for the loader's lock holder: with the
    /// overlays on, each
    /// type a worker's overlay holds exported into the base, so that the record names the base
    /// alone, which every worker's view holds; checked (`view::published`) and counted as the
    /// export it is (`TEQ_WORKERS_TYPES=1`).
    #[cfg_attr(not(debug_assertions), allow(unused_variables))]
    fn published_sig(&mut self, sig: Arc<MethodSig>, what: &'static str) -> Arc<MethodSig> {
        if self.types.view_here() != View::Hold {
            return sig;
        }
        let sig = crate::symbols::sig_mapped(&sig, &mut |t| self.types.export_at(crate::types::Route::Publish, t));
        #[cfg(debug_assertions)]
        for t in sig_types(&sig) {
            crate::types::view::published(&self.types, what, t);
        }
        if crate::types::noting() {
            let types: Vec<TypeId> = sig_types(&sig).collect();
            self.types.note_export(crate::types::Route::Publish, &types);
        }
        sig
    }

    /// The signature with every instantiated type variable replaced by its instance.
    fn zonked_sig(&mut self, sig: Arc<MethodSig>) -> Arc<MethodSig> {
        let ret = self.zonk(sig.ret);
        let mut params: Vec<TypeId> = Vec::new();
        for c in &sig.clauses {
            for p in &c.params {
                params.push(self.zonk(p.ty));
            }
        }
        let moved = ret != sig.ret || sig.clauses.iter().flat_map(|c| c.params.iter()).zip(params.iter()).any(|(p, &t)| p.ty != t);
        if !moved {
            return sig;
        }
        let mut moved_sig = (*sig).clone();
        moved_sig.ret = ret;
        let mut i = 0;
        for c in &mut moved_sig.clauses {
            for p in &mut c.params {
                p.ty = params[i];
                i += 1;
            }
        }
        Arc::new(moved_sig)
    }

    /// Waits for the cell `key`, claimed by another thread, until `done` holds: `true` once it
    /// does, `false` when the wait would close a cycle through this thread's own cells, which
    /// the caller treats as the recursion it is on one thread. Never under the loader's lock:
    /// the lock is held by the std's and the libraries' work alone, whose cells are claimed
    /// and completed within one hold, so its holder never meets another thread's claim;
    /// a wait there is an error (`shared::wait_until`).
    pub(super) fn wait_cell(&mut self, key: CellKey, done: impl Fn(&Self) -> bool) -> bool {
        if done(self) {
            return true;
        }
        super::bundle::waited_for(key);
        let start = std::time::Instant::now();
        let measured = crate::measure::wait_begin();
        let holder = if measured.is_some() { self.cell_owner(key).unwrap_or(0) } else { 0 };
        // The cycle check is the wait's own, under its lock (`shared::wait_until`).
        let waited = crate::shared::wait_until(key, |k| self.cell_owner(k), || done(self));
        crate::measure::wait_end(measured, if waited { crate::measure::Wait::cell(key.kind()) } else { crate::measure::Wait::CellRefused }, holder);
        if !waited {
            // Which of the cycle's members reports it is the order's to say.
            self.need_serial(|| format!("a cycle of completions through another worker's cell ({:?})", key));
            return false;
        }
        if self.profile.on {
            self.profile.waited += 1;
            self.profile.wait_ns += start.elapsed().as_nanos() as u64;
        }
        // What the other worker made on the way to the cell's result is this worker's to see.
        self.refresh_std_if_stale();
        self.sync_arity_if_stale();
        true
    }

    /// Whether waiting on `key` would close a cycle: the thread that holds it waits, directly
    /// or through other threads, on a cell this thread holds.
    /// The thread holding the cell, while it is in progress.
    fn cell_owner(&self, key: CellKey) -> Option<usize> {
        match key.kind() {
            CELL_SIG => self.syms.sym(SymId(key.id())).state().owner(),
            CELL_BODY => self.syms.sym(SymId(key.id())).body().owner(),
            CELL_CLASS => self.syms.class(ClassId(key.id())).state().owner(),
            CELL_ALIAS => self.syms.alias_cells.owner(key.id()),
            CELL_CHECK => self.syms.check_cells.owner(key.id()),
            _ => None,
        }
    }

    /// The signature as far as it is known while the body is typed for the result type: what
    /// a recursive reference reads.
    fn set_partial_sig(&mut self, sym: SymId, partial: Arc<MethodSig>) {
        if self.shared_sym(sym) {
            self.with_loader(|w| {
                let partial = w.published_sig(partial, "a shared symbol's partial signature (set_partial_sig)");
                w.syms.sym_mut(sym).sig = Some(partial);
            });
        } else {
            self.syms.sym_mut(sym).sig = Some(partial);
        }
    }

    fn compute_sig(&mut self, sym: SymId, def: &'a ast::Def) -> MethodSig {
        self.completing += 1;
        let sig = if self.deps.is_none() { self.compute_sig_inner(sym, def) } else { self.compute_sig_recorded(sym, def) };
        self.completing -= 1;
        sig
    }

    #[cold]
    #[inline(never)]
    fn compute_sig_recorded(&mut self, sym: SymId, def: &'a ast::Def) -> MethodSig {
        let outer = self.deps_enter_sym(sym, super::deps::Comp::Sig(sym));
        let sig = self.compute_sig_inner(sym, def);
        self.deps_leave(outer);
        sig
    }

    fn compute_sig_inner(&mut self, sym: SymId, def: &'a ast::Def) -> MethodSig {
        let t = self;
        match &def.kind {
            DefKind::Val { ty, .. } => match ty {
                Some(ty) => MethodSig::value(t.resolve_declared_type(*ty)),
                // An incomplete val's type is unknown unless it was declared.
                None if def.mods & mods::INCOMPLETE != 0 => MethodSig::value(ERROR),
                None => {
                    let partial = Arc::new(MethodSig::value(ERROR));
                    t.set_partial_sig(sym, partial.clone());
                    let inherited = t.inherited_result_type(sym, &partial);
                    if inherited.is_none() && t.syms.sym(sym).owner != Owner::Local {
                        t.missing_implicit_type(sym, def, "");
                    }
                    let inferred = t.infer_body(sym, &partial, inherited);
                    if t.shared_sym(sym) {
                        t.publish_body(sym);
                    }
                    MethodSig::value(inferred)
                }
            },
            DefKind::Fun(f) if def.name == names::INIT => {
                // A secondary constructor: the class's type parameters, and the class as result.
                let Owner::Class(c) = t.syms.sym(sym).owner else { return MethodSig::value(ERROR) };
                let clauses = t.resolve_clauses(&f.clauses, None, &[], &[], None);
                let info = t.syms.class(c);
                MethodSig { tparams: info.tparams.clone(), clauses, ret: info.base_types[0].1 }
            }
            DefKind::Fun(f) => {
                let ids = t.make_tparams(&f.tparams);
                t.env.frames.push(Frame::Locals { names: Vec::new(), tparams: ids.clone(), givens: Vec::new(), classes: Vec::new(), aliases: Vec::new() });
                t.resolve_tparam_bounds(&f.tparams, &ids);
                t.erased_tags = (t.syms.sym(sym).jvm_evidence && t.erases_evidence()).then_some(0);
                let clauses = t.resolve_clauses(&f.clauses, None, &f.tparams, &ids, None);
                if let Some(tags) = t.erased_tags.take() {
                    t.shared_write(sym, |w| w.syms.sym_mut(sym).erased_tags = tags);
                }
                let tparams: Vec<TParamId> = ids.iter().map(|&(_, p)| p).collect();
                let ret = f.ret.map(|r| t.resolve_declared_type(r));
                t.env.frames.pop();
                let mut sig = MethodSig { tparams, clauses, ret: ret.unwrap_or(ERROR) };
                // An inline method's body is typed where it expands, and the call takes the
                // type found there. An incomplete method's result is unknown unless declared.
                if ret.is_none() && def.mods & (mods::INLINE | mods::INCOMPLETE) == 0 {
                    if f.body.is_none() {
                        t.error(def.span, "an abstract def needs a result type");
                    } else {
                        let partial = Arc::new(sig.clone());
                        t.set_partial_sig(sym, partial.clone());
                        let inherited = t.inherited_result_type(sym, &partial);
                        if inherited.is_none() {
                            t.missing_implicit_type(sym, def, "result ");
                        }
                        sig.ret = t.infer_body(sym, &sig, inherited);
                        if t.shared_sym(sym) {
                            t.publish_body(sym);
                        }
                    }
                }
                sig
            }
            DefKind::Given(g) => match t.syms.sym(sym).impl_class {
                Some(c) => {
                    t.complete_class(c);
                    let info = t.syms.class(c);
                    MethodSig {
                        tparams: info.tparams.clone(),
                        clauses: info.ctor.clone(),
                        ret: info.parents.first().copied().unwrap_or(ERROR),
                    }
                }
                None => {
                    if let Some(sig) = t.derived_ctor_sig(g) {
                        return sig;
                    }
                    let ids = t.make_tparams(&g.tparams);
                    t.env.frames.push(Frame::Locals { names: Vec::new(), tparams: ids.clone(), givens: Vec::new(), classes: Vec::new(), aliases: Vec::new() });
                    t.resolve_tparam_bounds(&g.tparams, &ids);
                    let clauses = t.resolve_clauses(&g.clauses, None, &g.tparams, &ids, None);
                    let ret = t.resolve_declared_type(g.ty);
                    t.env.frames.pop();
                    if t.deferred_candidate(sym) && g.alias.is_some_and(|a| t.is_deferred_marker(a)) {
                        t.shared_write(sym, |w| w.syms.sym_mut(sym).mods |= mods::DEFERRED);
                    }
                    MethodSig { tparams: ids.iter().map(|&(_, p)| p).collect(), clauses, ret }
                }
            },
            _ => MethodSig::value(ERROR),
        }
    }

    /// scalac's warning for a `return` that leaves a lambda or a by-name argument on its way
    /// out of the method.
    fn warn_nonlocal_returns(&mut self, body: TExprId, returns: &[(TExprId, crate::source::Span)]) {
        let mut found = Vec::new();
        self.nonlocal_returns(body, false, &mut found);
        for e in found {
            if let Some(&(_, span)) = returns.iter().find(|&&(r, _)| r == e) {
                if !self.restrict_nonlocal_return(span) {
                    self.warn(span, "Non local returns are no longer supported; use `boundary` and `boundary.break` in `scala.util` instead");
                }
            }
        }
    }

    fn nonlocal_returns(&self, e: TExprId, in_lambda: bool, out: &mut Vec<TExprId>) {
        let prog = &self.prog;
        match prog.expr(e) {
            TExpr::Return(v) => {
                if in_lambda {
                    out.push(e);
                }
                self.nonlocal_returns(v, in_lambda, out);
            }
            TExpr::Lambda(_, body) => self.nonlocal_returns(body, true, out),
            TExpr::Block(stmts, res) => {
                for s in &prog.stmts[stmts.range()] {
                    match *s {
                        TStmt::Expr(x) | TStmt::Val(_, x) | TStmt::Pat(_, x) => self.nonlocal_returns(x, in_lambda, out),
                        TStmt::Fun(_) => {}
                    }
                }
                self.nonlocal_returns(res, in_lambda, out);
            }
            TExpr::If(c, t, els) => {
                self.nonlocal_returns(c, in_lambda, out);
                self.nonlocal_returns(t, in_lambda, out);
                if let Some(x) = els {
                    self.nonlocal_returns(x, in_lambda, out);
                }
            }
            TExpr::Match(scrut, cases) => {
                self.nonlocal_returns(scrut, in_lambda, out);
                for case in &prog.cases[cases.range()] {
                    if let Some(g) = case.guard {
                        self.nonlocal_returns(g, in_lambda, out);
                    }
                    self.nonlocal_returns(case.body, in_lambda, out);
                }
            }
            TExpr::While(a, b) | TExpr::Assign(a, b) | TExpr::Prim(_, a, b) => {
                self.nonlocal_returns(a, in_lambda, out);
                self.nonlocal_returns(b, in_lambda, out);
            }
            TExpr::Field(a, _) | TExpr::Unary(_, a) | TExpr::ToStr(a, _) | TExpr::TypeTest(a, _) | TExpr::Cast(a, ..) | TExpr::Index(a, _)
            | TExpr::Spread(a) | TExpr::JsSelect(a, _) => self.nonlocal_returns(a, in_lambda, out),
            TExpr::CallMethod(r, _, args) | TExpr::CallClosure(r, args) => {
                self.nonlocal_returns(r, in_lambda, out);
                for &a in prog.expr_list(args) {
                    self.nonlocal_returns(a, in_lambda, out);
                }
            }
            TExpr::CallStatic(_, args) | TExpr::New(_, args) | TExpr::NewVia(_, args) | TExpr::StrConcat(args)
            | TExpr::Js(_, args) | TExpr::SeqLit(args) | TExpr::ArrayLit(args) | TExpr::ObjLit(args) => {
                for &a in prog.expr_list(args) {
                    self.nonlocal_returns(a, in_lambda, out);
                }
            }
            TExpr::Throw(a, _) | TExpr::Splice(a) => self.nonlocal_returns(a, in_lambda, out),
            TExpr::Try(i) => {
                let t = &prog.tries[i as usize];
                self.nonlocal_returns(t.body, in_lambda, out);
                for case in &prog.cases[t.cases.range()] {
                    if let Some(g) = case.guard {
                        self.nonlocal_returns(g, in_lambda, out);
                    }
                    self.nonlocal_returns(case.body, in_lambda, out);
                }
                if let Some(f) = t.finalizer {
                    self.nonlocal_returns(f, in_lambda, out);
                }
            }
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_)
            | TExpr::Unit | TExpr::Null | TExpr::Local(_) | TExpr::This | TExpr::Super(_) | TExpr::Static(_)
            | TExpr::Module(_) | TExpr::ClassOf(_) | TExpr::JsImport(_) | TExpr::JsGlobal(..) => {}
        }
    }

    /// scalac's rule that an implicit definition declares its type; the definition stays, as an
    /// ordinary one.
    fn missing_implicit_type(&mut self, sym: SymId, def: &ast::Def, what: &str) {
        if def.mods & mods::IMPLICIT == 0 {
            return;
        }
        self.error(def.span, format!("{}type of implicit definition needs to be given explicitly", what));
        self.shared_write(sym, |w| w.syms.sym_mut(sym).mods &= !mods::IMPLICIT);
    }

    /// The result type of the ancestor member that a member without a declared type overrides,
    /// seen from the member's class: it is the type scalac gives the member. The member's own
    /// signature holds its parameters already, with `ERROR` for the result.
    pub(super) fn inherited_result_type(&mut self, sym: SymId, msig: &Arc<MethodSig>) -> Option<TypeId> {
        let (name, is_ext, owner) = {
            let s = self.syms.sym(sym);
            (s.name, s.is_extension, s.owner)
        };
        let Owner::Class(c) = owner else { return None };
        for i in 1..self.syms.class(c).base_types.len() {
            let (b, bt) = self.syms.class(c).base_types[i];
            let mut k = 0;
            while let Some(p) = self.inherited_member(b, name, is_ext, k) {
                k += 1;
                if self.is_private(p) || self.compare_sigs(c, msig.clone(), None, p, bt) == Agreement::Params {
                    continue;
                }
                let psig = self.sig_arc(p);
                let mut subst = self.owner_subst(bt);
                for (&pt, &mt) in psig.tparams.iter().zip(&msig.tparams) {
                    let as_m = self.types.param(mt);
                    subst.push((pt, as_m));
                }
                let mut ret = self.types.subst(psig.ret, &subst);
                if self.types.has_paths(ret) {
                    let this = self.this_prefix(c);
                    ret = self.as_seen_from(ret, this, c);
                }
                return (ret != ERROR).then_some(ret);
            }
        }
        None
    }

    /// `private` without a qualifier: a `private[scope]` member takes part in overriding.
    pub(super) fn is_private(&self, sym: SymId) -> bool {
        let s = self.syms.sym(sym);
        s.mods & mods::PRIVATE != 0 && !self.ast(s.file).access_scopes.iter().any(|&(at, _)| at == s.span.start)
    }

    pub(super) fn ext_scope_of(&self, sym: SymId, sig: &MethodSig, def: &ast::Def) -> Option<super::ExtScope> {
        let info = self.syms.sym(sym);
        let DefKind::Fun(f) = &def.kind else { return None };
        if !info.is_extension {
            return None;
        }
        let receiver_clause = sig.clauses.iter().take(info.ext_clauses as usize).find(|c| !c.is_using)?;
        let receiver = receiver_clause.params.first()?.sym;
        Some(super::ExtScope { owner: info.owner, file: info.file, group: f.ext_group, receiver })
    }

    /// Types the body of `sym` for a use of its function or initialiser by id, unless it is
    /// typed already. A program member's body is the walk's (`walk_body`), claimed by its body
    /// cell outside the loader's lock and typed into the claiming worker's chunk, which every
    /// worker reads by id; a std or library member's is typed under the lock into the shared
    /// region.
    pub fn ensure_body(&mut self, sym: SymId) {
        if !self.forked || !self.shared_sym(sym) || self.program_file(self.syms.sym(sym).file) {
            return self.walk_body(sym);
        }
        if self.body_readable(sym) || self.bodies_in_progress.contains(&sym) {
            return;
        }
        if self.is_inline_callee(sym) && !self.is_retained_inline(sym) {
            return;
        }
        // A body of the std or a library: typed under the lock, where its cell is claimed and
        // done within the one hold.
        self.with_loader_for(crate::measure::Hold::Body, |w| {
            if w.body_readable(sym) {
                return;
            }
            if !w.syms.body_cells.claim(sym.0) {
                if !w.syms.sym(sym).body().mine() {
                    w.wait_cell(CellKey::new(CELL_BODY, sym.0), |w| w.syms.sym(sym).body() == Completion::Done);
                }
                return;
            }
            w.type_member_body(sym);
            w.publish_body(sym);
        })
    }

    /// Whether the function or initialiser of `sym` is typed: this worker's, the shared
    /// region's or another worker's, which reads it through its chunk.
    fn body_readable(&self, sym: SymId) -> bool {
        self.fun_of_sym.contains_key(&sym) || self.val_init.contains_key(&sym)
    }

    /// The function of `sym`, when it is typed.
    pub(super) fn readable_fun(&mut self, sym: SymId) -> Option<FunId> {
        let f = self.fun_of_sym.get(&sym).copied()?;
        super::bundle::entered_at(&self.prog.funs, f.0, super::bundle::Entry::Fun);
        Some(f)
    }

    /// The initialiser of `sym`, when it is typed.
    pub(super) fn readable_init(&mut self, sym: SymId) -> Option<TExprId> {
        let e = self.val_init.get(&sym).copied()?;
        super::bundle::entered_at(&self.prog.exprs, e.0, super::bundle::Entry::Val);
        Some(e)
    }

    /// A member's body as a class check types it: the walk's own when the class is this
    /// worker's item, a demand's under the loader's lock (a class the interpreter reaches).
    fn class_member_body(&mut self, sym: SymId) {
        if self.forked && crate::shared::lock_depth() > 0 {
            self.ensure_body(sym)
        } else {
            self.walk_body(sym)
        }
    }

    /// Types the body of `sym` as the walk over its file does, into this worker's chunk: the
    /// body cell says whether another worker has it already, in which
    /// case its function is taken from the cell.
    pub fn walk_body(&mut self, sym: SymId) {
        if self.fun_of_sym.contains_key(&sym) || self.val_init.contains_key(&sym) {
            return;
        }
        if self.forked && self.syms.sym(sym).impl_class.is_some() && self.syms.sym(sym).kind == SymKind::Given {
            return self.check_given(sym);
        }
        // An inline method's body is typed where it expands; the definition check types it
        // once more at the definition and stores it. One that overrides a method that is not
        // inline is typed here as well, as a method of its class.
        if self.is_inline_callee(sym) {
            self.check_inline_definition(sym);
            if !self.is_retained_inline(sym) {
                return;
            }
        }
        if self.bodies_in_progress.contains(&sym) {
            return;
        }
        // Kept for the body's typing below.
        let sig = self.sig_arc(sym);
        if self.fun_of_sym.contains_key(&sym) || self.val_init.contains_key(&sym) {
            return;
        }
        if self.syms.sym(sym).owner == Owner::Local {
            return;
        }
        // The body cell of a shared member: one thread types the body, the others wait for
        // the function or initialiser it publishes.
        if self.shared_sym(sym) && !self.syms.body_cells.claim(sym.0) {
            if self.syms.sym(sym).body().mine() {
                return;
            }
            self.wait_cell(CellKey::new(CELL_BODY, sym.0), |w| w.syms.sym(sym).body() == Completion::Done);
            return;
        }
        self.type_member_body_with(sym, sig);
        if self.shared_sym(sym) {
            self.publish_body(sym);
        }
    }

    fn type_member_body(&mut self, sym: SymId) {
        let sig = self.sig_arc(sym);
        // The completion of an inferred result type typed the body.
        if self.body_readable(sym) || self.syms.sym(sym).owner == Owner::Local {
            return;
        }
        self.type_member_body_with(sym, sig);
    }

    fn type_member_body_with(&mut self, sym: SymId, sig: Arc<MethodSig>) {
        let (file, owner, span) = {
            let s = self.syms.sym(sym);
            (s.file, s.owner, s.span)
        };
        let env = self.env_at(file, owner, span.start);
        let ret = sig.ret;
        let outer_defining = self.defining.take();
        self.with_env(env, |t| {
            t.type_body(sym, &sig, Some(ret));
        });
        self.defining = outer_defining;
    }

    /// Makes the body of the shared symbol `sym` every worker's, then its cell done: off the
    /// loader's lock as a bundle, or by the release of a hold.
    fn publish_body(&mut self, sym: SymId) {
        self.publish_body_bundle(sym);
        self.syms.sym(sym).body().set(Completion::Done);
    }

    /// Enters the parameters of `sig` into the locals frame `frame` clause by clause while the
    /// defaults are typed: a default sees the earlier clauses, as in scalac, so `size = size`
    /// names the enclosing member. Returns the typed defaults, one slot per parameter.
    pub(super) fn enter_params(&mut self, sig: &MethodSig, default_exprs: &[Option<ast::ExprId>], frame: usize) -> Vec<Option<TExprId>> {
        let mut defaults: Vec<Option<TExprId>> = Vec::new();
        for clause in &sig.clauses {
            for p in &clause.params {
                let default = default_exprs.get(defaults.len()).copied().flatten();
                defaults.push(default.map(|e| {
                    // A default is checked with the method's own type parameters open, as
                    // scalac's default getter is; `def f[T](x: T = "abc")` fixes `T` at the call.
                    let te = if self.mentions_tparam_of(p.ty, Some(&sig.tparams)) {
                        let first_var = self.tvars.len();
                        let subst: Subst = sig.tparams.iter().map(|&tp| (tp, self.fresh_var())).collect();
                        let open = self.types.subst(p.ty, &subst);
                        let te = self.check_expr(e, open);
                        for v in first_var..self.tvars.len() {
                            self.solve_var(self.tvars.id(v));
                        }
                        te
                    } else {
                        self.check_expr(e, p.ty)
                    };
                    if p.by_name { self.prog.add(TExpr::Lambda(ast::ListRef::EMPTY, te)) } else { te }
                }));
            }
            if let Frame::Locals { names, givens, .. } = &mut self.env.frames[frame] {
                names.extend(clause.params.iter().map(|p| (p.name, p.sym)));
                if clause.is_using {
                    givens.extend(clause.params.iter().map(|p| p.sym));
                }
            }
        }
        defaults
    }

    /// scalac's rules for an `inline val`: a literal constant, whose literal type it takes. In a
    /// body under the definition check, where scalac's `InlineVals` does not look, one whose value
    /// is no constant there is the expansion's to check (`Worker::stand_for_inline_val`): a call
    /// kept for the expansion, an `inline` parameter, any other call, which a branch the
    /// expansion discards never reaches.
    pub(super) fn check_inline_val(&mut self, init: TExprId, ty: TypeId, declared: bool, rhs: ast::ExprId) -> TypeId {
        let span = self.cur_ast().expr_span(rhs);
        let in_body = self.checks_inline_definition();
        let declared_ty = self.deref(ty);
        // A declared type has to be a literal type, or the singleton type of a constant
        // (`Int.MaxValue.type`).
        let declared_constant = match self.types.get(declared_ty) {
            Type::Lit(_) => true,
            Type::Term(s) => self.fold_value(s).is_some(),
            _ => false,
        };
        if declared && !declared_constant && !in_body {
            self.error(span, "inline value must have a literal constant type");
            return ty;
        }
        // `inline val k = (0: Int)` is an `Int`, no constant, as scalac types the ascription, and
        // so is one of a plain inline call or an operation over one, which scalac's typer keeps
        // as a call; in a body the expansion checks it, as scalac checks it where the body
        // expands, after the inlining phase (`expanded_widening`).
        let widened = if in_body { self.expanded_widening(init) } else { self.widened_constant(init) };
        if let Some(v) = self.fold_constant(init).filter(|_| !widened) {
            let lit = self.types.lit(v);
            self.prog.set_type(init, lit);
            return lit;
        }
        if let Some(v) = self.fold_type(ty).or_else(|| self.constant_by_type(init, None)).filter(|_| !widened) {
            // A constant of the initialiser's type (a call's declared result `true` included) is
            // the val's where the initialiser is pure (scalac's `exprPurity`); in a body, the
            // expansion checks it.
            if !in_body && !self.is_pure_value(init) {
                let msg = format!("inline value must be pure but was: {}", self.code_of_tree(init));
                self.error(span, msg);
            }
            if in_body && !self.is_pure_value(init) {
                return self.widen_lit(ty);
            }
            return self.types.lit(v);
        }
        if in_body {
            return ty;
        }
        let t = self.deref(ty);
        if t == self.b.t_unit {
            self.error(span, "`inline val` of type `Unit` is not supported.\n\nTo inline a `Unit` consider using `inline def`");
        } else if t == self.b.t_string || self.is_primitive(t) {
            self.error(span, "inline value must have a literal constant type");
        } else {
            self.error(span, "inline value must contain a literal constant value.\n\nTo inline more complex types consider using `inline def`");
        }
        ty
    }

    /// `def this(params) = { this(args); stmts }`: the self call is typed as a constructor call
    /// of the class, without `this` and the members as the arguments of a parent are, and the
    /// statements after it with them. The body of the resulting function is a block whose
    /// last statement is that call and whose result is the rest.
    fn type_secondary_ctor(&mut self, c: ClassId, sym: SymId, tclass: &mut TClass) {
        // Kept for the parameters' entry below.
        let sig = self.sig_arc(sym);
        let (file, def_id, span) = {
            let s = self.syms.sym(sym);
            (s.file, s.def, s.span)
        };
        let Some(def_id) = def_id else { return };
        let def = self.ast(file).def(def_id);
        let DefKind::Fun(f) = &def.kind else { return };
        let Some(body) = f.body else { return };
        let ast = self.ast(file);
        let (self_call, rest) = match ast.expr(body) {
            ast::Expr::Block(stmts) => match ast.stmt_list(stmts).first() {
                Some(&Stmt::Expr(e)) => (e, ast::ListRef { start: stmts.start + 1, len: stmts.len - 1 }),
                _ => return,
            },
            _ => (body, ast::ListRef::EMPTY),
        };
        let frame = self.env.frames.len();
        self.env.frames.push(Frame::Locals { names: Vec::new(), tparams: Vec::new(), givens: Vec::new(), classes: Vec::new(), aliases: Vec::new() });
        let default_exprs: Vec<Option<ast::ExprId>> = f.clauses.iter().flat_map(|cl| cl.params.iter().map(|p| p.default)).collect();
        let defaults = self.enter_params(&sig, &default_exprs, frame);
        let params: Vec<SymId> = sig.clauses.iter().flat_map(|cl| cl.params.iter().map(|p| p.sym)).collect();
        let outer_return = self.return_to.take();
        let outer_returns = std::mem::take(&mut self.returns);
        let outer_excluded = self.excluded_ctor.replace(sym);
        let outer_args = self.parent_args_of.replace(c);
        let (call, _) = self.type_expr(self_call, None);
        self.parent_args_of = outer_args;
        self.excluded_ctor = outer_excluded;
        let (after, _) = if rest.len == 0 { (self.prog.add(TExpr::Unit), self.b.t_unit) } else { self.type_block(rest, None, None) };
        self.env.frames.pop();
        self.return_to = outer_return;
        self.returns = outer_returns;
        // Temporaries of named arguments written out of order stand in front of the call.
        let mut stmts: Vec<TStmt> = Vec::new();
        let call = match self.prog.expr(call) {
            TExpr::Block(prelude, inner) => {
                stmts.extend_from_slice(&self.prog.stmts[prelude.range()]);
                inner
            }
            _ => call,
        };
        if !matches!(self.prog.expr(call), TExpr::New(..) | TExpr::NewVia(..)) && self.diags.error_count() == 0 {
            self.error(span, "a secondary constructor has to call another constructor of its class first");
        }
        if let TExpr::NewVia(target, _) = self.prog.expr(call) {
            let position = |t: &Self, s: SymId| t.syms.class(c).ctors.iter().position(|&x| x == s);
            if position(self, target) > position(self, sym) {
                let at = self.cur_ast().expr_span(self_call);
                self.error(at, "secondary constructor must call a preceding constructor");
            }
        }
        stmts.push(TStmt::Expr(call));
        let l = self.prog.stmts.push_slice(&stmts);
        let body = self.prog.add(TExpr::Block(l, after));
        let fid = self.prog.add_fun(TFun { sym, params, defaults, body: Some(body) });
        self.fun_of_sym.insert(sym, fid);
        tclass.ctors.push(fid);
    }

    /// scalac's rules on the constructors of a class as an overload set: default arguments on
    /// one alternative, and no two that erasure does not tell apart.
    fn check_ctor_alternatives(&mut self, c: ClassId) {
        let secondaries = self.syms.class(c).ctors.clone();
        let Some(primary) = self.syms.class(c).primary_ctor.filter(|_| !secondaries.is_empty()) else { return };
        let all: Vec<SymId> = std::iter::once(primary).chain(secondaries).collect();
        let mut with_defaults = Vec::new();
        for &s in &all {
            if self.sig_of(s).clauses.iter().any(|cl| cl.params.iter().any(|p| p.has_default)) {
                with_defaults.push(s);
            }
        }
        if let Some(&second) = with_defaults.get(1) {
            let (file, span) = (self.syms.sym(second).file, self.syms.sym(second).span);
            let msg = format!(
                "two or more overloaded variants of constructor {} have default arguments",
                self.name_str(self.syms.class(c).name)
            );
            self.diags.error(file, span, msg);
        }
        for i in 1..all.len() {
            for j in 0..i {
                if self.report_double_definition(all[j], all[i]) {
                    break;
                }
            }
        }
    }

    /// Types the right-hand side of a val, def or alias given in the current environment and
    /// records the resulting IR. Returns the type of the body.
    fn declared_type_inferred(&self, kind: &DefKind) -> bool {
        if !self.in_converted_body() {
            return false;
        }
        #[cfg(debug_assertions)]
        if self.forked {
            super::loader::compile::body_crossings::read(self.env.file, self.worker);
        }
        let inferred = &self.cur_ast().inferred_types;
        let declared = match kind {
            DefKind::Fun(f) => f.ret,
            DefKind::Val { ty, .. } => *ty,
            _ => None,
        };
        !inferred.is_empty() && declared.map_or(false, |t| inferred.binary_search(&t.0).is_ok())
    }

    pub fn type_body(&mut self, sym: SymId, sig: &MethodSig, expected: Option<TypeId>) -> TypeId {
        self.outside_annotation(|t| t.type_body_now(sym, sig, expected))
    }

    fn type_body_now(&mut self, sym: SymId, sig: &MethodSig, expected: Option<TypeId>) -> TypeId {
        if crate::measure::on() || super::prep::capturing() {
            return self.type_body_counted(sym, sig, expected);
        }
        self.type_body_plain(sym, sig, expected)
    }

    pub(super) fn type_body_plain(&mut self, sym: SymId, sig: &MethodSig, expected: Option<TypeId>) -> TypeId {
        if self.forked {
            // What other workers entered or made since this worker last looked (a std file, an
            // arity class): every body starts with it, as one worker's would have it.
            self.refresh_std_if_stale();
            self.sync_arity_tables();
        }
        if self.body_depth >= MAX_BODY_DEPTH {
            let msg = format!("the definition of {} is nested in the typing of {} other bodies, which is a cycle", self.sym_path(sym), MAX_BODY_DEPTH);
            self.error(self.syms.sym(sym).span, msg);
            return ERROR;
        }
        if self.body_depth + 40 >= MAX_BODY_DEPTH && std::env::var_os("TEQ_CLASSPATH_DETAIL").is_some() {
            eprintln!("body depth {}: {}", self.body_depth, self.sym_path(sym));
        }
        // `val a, b = e` types `e` once per name: whichever name is asked for first, the names
        // before it have their copies typed first, so that what a copy makes (an anonymous
        // class, numbered per site in `anon.rs`) is named as in the walk's order.
        if let Some(previous) = self.previous_initialiser_copy(sym) {
            self.walk_body(previous);
        }
        self.body_depth += 1;
        let marks = self.unused.journal_len();
        // The body's typing is its definition's whatever becomes of an attempt that asked for
        // it.
        let promoted = self.promote_begin(sym);
        let pending = self.attempts.pending_len();
        let p = self.body_prof(sym);
        // A member's body is typed as the walk types it, whichever expansion or search
        // demanded it: the anonymous classes it makes are named by their own sites, not by the
        // expansion's, and the search's state is the demanding body's.
        let retained = self.is_inline_callee(sym) as u32;
        let ty = if self.made_in_block(self.syms.sym(sym).owner) {
            self.as_own_unit(|t| t.type_retained(retained, |t| t.type_body_in(sym, sig, expected, true)))
        } else {
            self.as_own_unit(|t| t.outside_search(|t| t.outside_inline(|t| t.type_retained(retained, |t| t.type_body_in(sym, sig, expected, false)))))
        };
        self.phase_end(p);
        // The body's typing is kept whatever demanded it (an attempt abandoned after it inferred
        // the member's result, an extension given up for a conversion): its marks with it.
        if self.unused.on() {
            self.unused.settle_since(marks);
        }
        // A body is a unit of its own, published when typed: its plain inline calls expand at its
        // end, what they write its definition's.
        self.flush_pending_since(pending);
        self.promote_end(promoted);
        self.body_depth -= 1;
        ty
    }

    fn type_retained<T>(&mut self, retained: u32, f: impl FnOnce(&mut Self) -> T) -> T {
        self.inline.retained += retained;
        let result = f(self);
        self.inline.retained -= retained;
        result
    }

    /// Types a member's body outside the given search that demanded it: the search's stack,
    /// context and budget are the demanding body's, not the member's, and a body typed on
    /// demand (from a signature inference, a macro, another worker's item) must type as it
    /// would in the walk.
    pub(super) fn outside_search<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        if self.search_state_clear() {
            let result = f(self);
            self.clear_search_state();
            return result;
        }
        let given_stack = std::mem::take(&mut self.given_stack);
        let given_context = std::mem::take(&mut self.given_context);
        let given_consulted = std::mem::take(&mut self.given_consulted);
        let given_opened = std::mem::take(&mut self.given_opened);
        let given_tree = std::mem::take(&mut self.given_tree);
        let trace_starts = std::mem::take(&mut self.trace_starts);
        let implicit_depth = std::mem::replace(&mut self.implicit_depth, 0);
        let implicit_budget = std::mem::replace(&mut self.implicit_budget, 0);
        let given_ambiguity = self.given_ambiguity.take();
        let failed_givens = std::mem::take(&mut self.failed_givens);
        let search_byname = std::mem::replace(&mut self.search_byname, false);
        let measured_target = self.measured_target.take();
        let function_targets = std::mem::take(&mut self.function_targets);
        let blocked_conversion = self.blocked_conversion.take();
        let converting_receiver = std::mem::replace(&mut self.converting_receiver, false);
        // What a search under way took is its own; the body's searches attribute theirs.
        let given_winners = if self.unused.on() { std::mem::take(&mut self.unused.given_winners) } else { Vec::new() };
        let result = f(self);
        if self.unused.on() {
            self.unused.given_winners = given_winners;
        }
        self.given_stack = given_stack;
        self.given_context = given_context;
        self.given_consulted = given_consulted;
        self.given_opened = given_opened;
        self.given_tree = given_tree;
        self.trace_starts = trace_starts;
        self.implicit_depth = implicit_depth;
        self.implicit_budget = implicit_budget;
        self.given_ambiguity = given_ambiguity;
        self.failed_givens = failed_givens;
        self.search_byname = search_byname;
        self.measured_target = measured_target;
        self.function_targets = function_targets;
        self.blocked_conversion = blocked_conversion;
        self.converting_receiver = converting_receiver;
        result
    }

    /// Whether no search is under way: the state `outside_search` saves is its empty one, and
    /// clearing it after the body gives it back without giving up the vectors' room.
    fn search_state_clear(&self) -> bool {
        let tree = &self.given_tree;
        self.given_stack.is_empty()
            && self.given_context.is_empty()
            && self.given_consulted.is_empty()
            && self.given_opened.is_empty()
            && tree.in_progress == 0
            && tree.restricted == 0
            && tree.deepest == 0
            && tree.byname_requests == 0
            && self.trace_starts.is_empty()
            && self.implicit_depth == 0
            && self.implicit_budget == 0
            && self.given_ambiguity.is_none()
            && self.failed_givens.is_empty()
            && !self.search_byname
            && self.measured_target.is_none()
            && self.function_targets.is_empty()
            && self.blocked_conversion.is_none()
            && !self.converting_receiver
    }

    fn clear_search_state(&mut self) {
        self.given_stack.clear();
        self.given_context.clear();
        self.given_consulted.clear();
        self.given_opened.clear();
        self.given_tree = Default::default();
        self.trace_starts.clear();
        self.implicit_depth = 0;
        self.implicit_budget = 0;
        self.given_ambiguity = None;
        self.failed_givens.clear();
        self.search_byname = false;
        self.measured_target = None;
        self.function_targets.clear();
        self.blocked_conversion = None;
        self.converting_receiver = false;
    }

    /// The name declared before `sym`'s in one `val a, b = e`, whose copy of `e` is typed before
    /// `sym`'s (`Ast::val_copies`).
    fn previous_initialiser_copy(&self, sym: SymId) -> Option<SymId> {
        let s = self.syms.sym(sym);
        let d = s.def?;
        let previous = *self.ast(s.file).val_copies.get(&d)?;
        self.def_syms.get(s.file.0 as usize, &previous).copied()
    }

    /// The `@nowarn` annotations on the classes enclosing `sym`, which suppress its body's
    /// warnings wherever the body is typed from: its class's walk, or another body's demand.
    pub(super) fn enclosing_nowarn(&self, sym: SymId) -> u32 {
        let mut n = 0;
        let mut owner = self.syms.sym(sym).owner;
        while let Owner::Class(c) = owner {
            let info = self.syms.class(c);
            if let Some(d) = info.def {
                n += self.ast(info.file).def(d).annots.iter().any(|a| a.name == names::NOWARN) as u32;
            }
            owner = info.owner;
        }
        n
    }

    /// The `@nowarn` annotations on `c` and the classes enclosing it.
    pub(super) fn class_nowarn(&self, c: ClassId) -> u32 {
        let mut n = 0;
        let mut at = Some(c);
        while let Some(k) = at {
            let info = self.syms.class(k);
            if let Some(d) = info.def {
                n += self.ast(info.file).def(d).annots.iter().any(|a| a.name == names::NOWARN) as u32;
            }
            at = match info.owner {
                Owner::Class(o) => Some(o),
                _ => None,
            };
        }
        n
    }

    /// `in_block` is `made_in_block` of the owner: the context of the body typing the block
    /// stands for the lexical one then (a local class asked for before its walk from a body
    /// outside its scope reads that body's).
    fn type_body_in(&mut self, sym: SymId, sig: &MethodSig, expected: Option<TypeId>, in_block: bool) -> TypeId {
        if self.deps.is_none() {
            return self.type_body_in_now(sym, sig, expected, in_block);
        }
        self.type_body_recorded(sym, sig, expected, in_block)
    }

    #[cold]
    #[inline(never)]
    fn type_body_recorded(&mut self, sym: SymId, sig: &MethodSig, expected: Option<TypeId>, in_block: bool) -> TypeId {
        let outer = self.deps_enter_sym(sym, super::deps::Comp::Body(sym));
        let ty = self.type_body_in_now(sym, sig, expected, in_block);
        self.deps_leave(outer);
        ty
    }

    fn type_body_in_now(&mut self, sym: SymId, sig: &MethodSig, expected: Option<TypeId>, in_block: bool) -> TypeId {
        let (file, def_id) = {
            let s = self.syms.sym(sym);
            (s.file, s.def)
        };
        let Some(def_id) = def_id else { return ERROR };
        let def = self.ast(file).def(def_id);
        let params: Vec<SymId> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.sym)).collect();
        let tparams = sig.tparams.iter().map(|&p| (self.syms.tparam(p).name, p)).collect();
        // The parameters join the frame clause by clause while the defaults are typed: a default
        // sees the earlier clauses, as in scalac, so `size = size` names the enclosing member.
        let frame = self.env.frames.len();
        self.env.frames.push(Frame::Locals { names: Vec::new(), tparams, givens: Vec::new(), classes: Vec::new(), aliases: Vec::new() });
        let outer_ext_scope = self.ext_scope;
        if let Some(scope) = self.ext_scope_of(sym, sig, def) {
            self.ext_scope = Some(scope);
        }
        let owners_depth = self.sites.owners.len();
        self.sites.owners.push(SiteOwner::Sym(sym));
        self.bodies_in_progress.push(sym);
        let var_frame = self.var_frame_begin();
        let pending = self.attempts.pending_len();
        let outer_defining = self.defining;
        if let DefKind::Given(g) = &def.kind {
            self.defining = g.alias.map(|_| sym);
        }
        // A given alias with type or term parameters is a def, one without a lazy val (dotty's
        // `Parsers.givenDef`, `hasParams`).
        let given_def = matches!(&def.kind, DefKind::Given(_)) && (!sig.clauses.is_empty() || !sig.tparams.is_empty());
        let returns_to = match &def.kind {
            DefKind::Fun(f) => Some((def.name, f.ret.map(|_| sig.ret), sym)),
            DefKind::Given(_) if given_def => Some((def.name, Some(sig.ret), sym)),
            _ => None,
        };
        let outer_return = std::mem::replace(&mut self.return_to, returns_to);
        let outer_returns = std::mem::take(&mut self.returns);
        let own_nowarn = def.annots.iter().any(|a| a.name == names::NOWARN) as u32;
        let outer_nowarn = self.nowarn;
        self.nowarn = if in_block { outer_nowarn } else { self.enclosing_nowarn(sym) } + own_nowarn;
        let (body, default_exprs): (Option<ast::ExprId>, Vec<Option<ast::ExprId>>) = match &def.kind {
            DefKind::Val { rhs, .. } => (*rhs, Vec::new()),
            DefKind::Fun(f) => (
                if self.inline.retained > 0 { self.retained_bodies.get(&sym).copied().or(f.body) } else { f.body },
                f.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.default)).collect(),
            ),
            // A deferred given is abstract: its marker is never typed (dotty's
            // `excludeDeferredGiven`).
            DefKind::Given(g) => (
                g.alias.filter(|_| self.syms.sym(sym).mods & mods::DEFERRED == 0),
                g.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.default)).collect(),
            ),
            _ => (None, Vec::new()),
        };
        let defaults = self.enter_params(sig, &default_exprs, frame);
        let is_method = matches!(def.kind, DefKind::Fun(_)) || given_def;
        // The interpreter runs the body of a templated def where it has no builtin for it.
        let has_intrinsic = (self.syms.sym(sym).intrinsic.is_some() || self.body_is_native(sym)) && !((self.interp || self.for_interpreter) && body.is_some());
        let mut result_ty = expected.unwrap_or(ERROR);
        let first_expr = self.prog.exprs.len();
        let typed = match body {
            Some(b) if !has_intrinsic => Some(match expected {
                Some(t) if t == self.b.t_unit => {
                    let (e, ty) = self.type_expr(b, None);
                    self.warn_discarded(b, e, ty);
                    e
                }
                Some(t) if self.declared_type_inferred(&def.kind) => self.type_inferred_val(b, t).0,
                Some(t) => self.check_expr(b, t),
                None => {
                    let (e, t) = self.type_expr(b, None);
                    let (mut e, t) = self.apply_inferred_context(b, e, t);
                    // A `final val` of an operation over constants is the constant, as scalac's
                    // `ConstFold` makes it (`final val k = 0 + 0`, an ascribed operand too: its
                    // `ConstantTree` looks through an ascription), which its reads take; one an
                    // ascription widened (`final val k = (0: Int)`) is no constant.
                    let plain_final = def.mods & (mods::FINAL | mods::INLINE | mods::LAZY) == mods::FINAL && matches!(def.kind, DefKind::Val { .. });
                    if plain_final && matches!(self.prog.expr(e), TExpr::Prim(..) | TExpr::Unary(..)) && !self.widened_constant(e) {
                        if let Some(v) = self.fold_constant_as_typed(e) {
                            let lit = self.literal_expr(v);
                            self.prog.copy_span(e, lit);
                            e = lit;
                        }
                    }
                    // A `final val` keeps a literal type, as in Scala.
                    result_ty = if def.mods & mods::FINAL != 0 { self.solve_in(t) } else { self.solve_inferred(t) };
                    result_ty = self.widen_soft(e, result_ty);
                    // A path type inferred (`sing(c)` for a `[T <: Singleton]`) widens as well.
                    if def.mods & mods::FINAL == 0 && matches!(self.types.get(result_ty), Type::Term(_) | Type::Select(..)) {
                        let widened = self.widen_path(result_ty);
                        result_ty = self.widen_lit(widened);
                    }
                    e
                }
            }),
            _ => None,
        };
        if let (true, Some(e)) = (def.mods & mods::INLINE != 0 && matches!(def.kind, DefKind::Val { .. }), typed) {
            result_ty = self.check_inline_val(e, result_ty, expected.is_some(), body.unwrap());
        }
        self.env.frames.pop();
        self.ext_scope = outer_ext_scope;
        self.sites.owners.truncate(owners_depth);
        self.bodies_in_progress.pop();
        self.var_frame_end(var_frame);
        self.defining = outer_defining;
        self.return_to = outer_return;
        let returns = std::mem::replace(&mut self.returns, outer_returns);
        if let (false, Some(body)) = (returns.is_empty(), typed) {
            self.warn_nonlocal_returns(body, &returns);
        }
        self.nowarn = outer_nowarn;
        if is_method {
            if !has_intrinsic {
                let f = self.prog.add_fun(TFun { sym, params, defaults, body: typed });
                self.fun_of_sym.insert(sym, f);
                if def.annots.iter().any(|a| a.name == names::TAILREC) {
                    // The check reads the calls the body's plain inline calls expand to, as
                    // scalac's runs after its `Inlining` phase.
                    self.attempts.tracking_moved += 1;
                    let moved_at = self.attempts.moved.len();
                    self.flush_pending_since(pending);
                    self.attempts.tracking_moved -= 1;
                    let moved = self.attempts.moved.split_off(moved_at);
                    self.check_tailrec(sym, f, first_expr, &moved, def.span);
                }
            }
        } else if let Some(e) = typed {
            #[cfg(debug_assertions)]
            if self.forked && crate::types::view::probing() && self.shared_sym(sym) && e.0 >= crate::arena::LOCAL_BASE && self.source(self.syms.sym(sym).file).is_std {
                crate::types::view::probe(|| format!("the initialiser of the std member {} ({:?}) goes to worker {}'s chunk, depth {}", self.sym_path(sym), sym, self.worker, crate::shared::lock_depth()));
            }
            self.val_init.insert(sym, e);
        }
        result_ty
    }

    pub fn check_class(&mut self, c: ClassId) {
        self.outside_annotation(|t| t.check_class_once(c))
    }

    fn check_class_once(&mut self, c: ClassId) {
        if self.syms.class(c).replaced {
            return;
        }
        // The check cell: the worker that claims it checks the class,
        // and another that needs the class checked waits for the cell outside the loader's lock; a
        // check that asks for its own class, a member's typing, returns. A shared class's
        // `class_done` and its `TClass` (`Program::class_bodies`) are published before the cell is
        // done, and a class another worker checked is read through that worker's chunk.
        loop {
            if self.class_done.contains_key(&c) {
                self.entered_done(c);
                return;
            }
            match self.syms.check_cells.get(c.0) {
                // Checked by another worker, which published the class with its check (a
                // shared class) or with what reached it (another worker's own).
                Completion::Done if self.forked => return self.entered_done(c),
                Completion::Done => break,
                Completion::InProgress => {
                    // A wait the graph refuses would close a cycle, reported by the wait: the
                    // check returns as one that asks for its own class does.
                    if !self.syms.check_cells.mine(c.0) && self.wait_cell(CellKey::new(CELL_CHECK, c.0), |w| w.syms.check_cells.get(c.0) == Completion::Done) {
                        continue;
                    }
                    return;
                }
                Completion::NotStarted => {
                    if self.syms.check_cells.claim(c.0) {
                        if self.forked {
                            crate::shake::point(crate::shake::Point::Claimed);
                        }
                        break;
                    }
                }
            }
        }
        let outer = if self.deps.is_some() { Some(self.deps_enter_class_body(c)) } else { None };
        let frame = self.var_frame_begin();
        if crate::measure::on() || super::prep::capturing() {
            self.check_class_counted(c);
        } else {
            self.check_class_timed(c);
        }
        self.var_frame_end(frame);
        if let Some(outer) = outer {
            self.deps_leave(outer);
        }
        self.class_done.insert(c, ());
        // A shared class checked outside the loader's lock is every worker's once its body and
        // the check are published, before its cell is done.
        if self.shared_class(c) && crate::shared::lock_depth() == 0 {
            self.publish_check_bundle(c);
        }
        self.syms.check_cells.set(c.0, Completion::Done);
    }

    pub(super) fn check_class_timed(&mut self, c: ClassId) {
        let p = self.phase_split(Phase::ClassCheck, Phase::LibraryClassCheck, |t| t.is_library_class(c));
        // A retype checks the classes of the file again, whatever asked for the check first.
        self.again(|t| t.check_class_now(c));
        self.phase_end(p);
    }

    /// Publishes, under the loader's lock, the classes this worker checked into its own chunk
    /// since the last publication: another worker that reaches one through a body or a
    /// signature published with it finds its `TClass` by `Program::class_bodies` and reads it
    /// through this worker's chunk, and the check done (chunks other workers read).
    pub(super) fn publish_class_bodies(&mut self) {
        if !self.forked {
            return;
        }
        for p in std::mem::take(&mut self.pending_shared) {
            match p {
                // A worker's registration, made the base's as it is published.
                super::PendingShared::Variance(k, t) => {
                    let t = self.types.translate(self.types.view_here(), t);
                    if crate::types::noting() {
                        self.types.note_export(crate::types::Route::ClassCheck, &[t]);
                    }
                    self.unchecked_variance.insert_shared(k, t)
                }
                super::PendingShared::Template(s, sym) => self.prog.template_syms.insert_shared(s, sym),
            }
        }
        let base = self.prog.classes.own_base();
        for k in self.tclasses_to_register() {
            let c = self.prog.classes.own()[k].id;
            self.prog.class_bodies.insert_shared(c, base + k as u32);
            if self.class_done.local.contains_key(&c) {
                self.class_done.insert_shared(c, ());
            }
            self.tclass_registered(k);
        }
    }

    /// Whether the `TClass` at `i` is this worker's and no other worker reads it yet: a
    /// published record is not changed under its readers.
    fn tclass_unpublished(&self, i: usize) -> bool {
        let base = self.prog.classes.own_base() as usize;
        i >= base && !self.tclass_is_registered(i) && self.prog.classes.writable(i as u32)
    }

    /// Registers the methods kept in `late_methods` with their classes, the workers merged.
    pub(super) fn register_late_methods(&mut self) {
        for (c, f) in std::mem::take(&mut self.late_methods) {
            if let Some(i) = self.prog_index.class(&self.prog, c) {
                let tc = &mut self.prog.classes[i];
                if !tc.methods.contains(&f) && !tc.ctors.contains(&f) {
                    tc.methods.push(f);
                }
            }
        }
    }

    /// A class nested in `c` whose name an ancestor's nested class or trait has: scalac's
    /// "class definitions cannot be overridden".
    fn check_nested_class_overrides(&mut self, c: ClassId) {
        if self.syms.class(c).kind == ClassKind::Object {
            return;
        }
        let nested: Vec<(Name, ClassId)> = self.syms.class(c).nested.iter().map(|(&n, &k)| (n, k)).collect();
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        for (name, k) in nested {
            if !matches!(self.syms.class(k).kind, ClassKind::Class | ClassKind::Trait) || self.syms.class(k).def.is_none() {
                continue;
            }
            let Some((b, other)) = bases.iter().find_map(|&b| {
                let other = *self.syms.class(b).nested.get(&name)?;
                (self.syms.class(b).kind != ClassKind::Object && matches!(self.syms.class(other).kind, ClassKind::Class | ClassKind::Trait)).then_some((b, other))
            }) else {
                continue;
            };
            let word = |t: &Self, x: ClassId| if t.syms.class(x).kind == ClassKind::Trait { "trait" } else { "class" };
            let msg = format!(
                "{} {} cannot have the same name as {} {} in {} -- class definitions cannot be overridden",
                word(self, k),
                self.name_str(name),
                word(self, other),
                self.name_str(name),
                self.class_description(b)
            );
            let (file, span) = (self.syms.class(k).file, self.syms.class(k).span);
            self.diags.error(file, span, msg);
        }
    }

    fn check_class_now(&mut self, c: ClassId) {
        self.complete_class(c);
        let pending = self.attempts.pending_len();
        let (file, def_id, kind, js) = {
            let i = self.syms.class(c);
            (i.file, i.def, i.kind, i.js)
        };
        // JavaScript defines a native type; only its declarations are checked.
        if js == JsKind::Native {
            self.check_native_body(c);
            if let (true, Some(d)) = (self.capturing() && self.syms.class(c).owner != Owner::Local, def_id) {
                self.capture_class_annotations(c, file, d);
            }
            return;
        }
        if def_id.is_some() && !self.is_library_class(c) {
            self.check_nested_class_overrides(c);
        }
        let ctor_params: Vec<SymId> =
            self.syms.class(c).ctor.iter().flat_map(|cl| cl.params.iter().map(|p| p.sym)).collect();
        let mut tclass = TClass {
            id: c,
            ctor_defaults: vec![None; ctor_params.len()],
            ctor_params,
            captures: 0,
            parent_args: None,
            parent_via: None,
            parent_prelude: ast::ListRef::EMPTY,
            inherited_case_members: 0,
            init: Vec::new(),
            methods: Vec::new(),
            ctors: Vec::new(),
            forwarders: Vec::new(),
            bridges: Vec::new(),
            super_accessors: Vec::new(),
            deferred_givens: Vec::new(),
        };
        if let Some(o) = self.outer_class(c) {
            let outer = self.outer_this_sym(o);
            tclass.ctor_params.insert(0, outer);
            tclass.ctor_defaults.insert(0, None);
            tclass.captures = 1;
        }
        let Some(def_id) = def_id else {
            self.push_class_body(tclass);
            return;
        };
        let def = self.ast(file).def(def_id);
        let (body, clauses, parents): (&[Stmt], &[ast::ParamClause], &[ast::Parent]) = match &def.kind {
            DefKind::Class(cls) => (&cls.body, &cls.clauses, &cls.parents),
            DefKind::Given(g) => (&g.body, &g.clauses, &[]),
            _ => (&[], &[], &[]),
        };
        let nowarn = def.annots.iter().any(|a| a.name == names::NOWARN) as u32;
        self.nowarn += nowarn;
        if kind == ClassKind::EnumCase && self.unused.on() {
            self.mark_enum_case_parents(c, file, parents);
        }
        if let DefKind::Class(cls) = &def.kind {
            // Errors in export clauses are reported even when nothing looks the exports up.
            if !cls.exports.is_empty() {
                self.exports_of(c);
            }
        }
        let parent_call = match &def.kind {
            DefKind::Class(cls) if kind == ClassKind::EnumCase => self.check_enum_parent_args(c, cls),
            _ => self.check_parent_args(c, parents),
        };
        if let Some(call) = parent_call {
            tclass.parent_args = Some(call.args);
            tclass.parent_via = call.via;
            tclass.parent_prelude = call.prelude;
        }
        if def.mods & mods::CASE != 0 {
            tclass.inherited_case_members = self.inherited_case_members(c);
        }
        let env = self.env_at(file, Owner::Class(c), def.span.start);
        self.with_env(env, |t| {
            let imports_scope = t.env.imports.len();
            let owners_depth = t.sites.owners.len();
            t.sites.owners.push(SiteOwner::Dummy(c));
            // Defaults see earlier constructor parameters as plain locals.
            let names: Vec<(crate::intern::Name, SymId)> =
                tclass.ctor_params.iter().map(|&s| (t.syms.sym(s).name, s)).collect();
            let param_types: Vec<(TypeId, bool)> =
                t.syms.class(c).ctor.iter().flat_map(|cl| cl.params.iter().map(|p| (p.ty, p.by_name))).collect();
            t.env.frames.push(Frame::Locals { names, tparams: Vec::new(), givens: Vec::new(), classes: Vec::new(), aliases: Vec::new() });
            let mut i = 0;
            for clause in clauses {
                for p in &clause.params {
                    if let Some(d) = p.default {
                        let (ty, by_name) = param_types[i];
                        let te = t.check_expr(d, ty);
                        tclass.ctor_defaults[i] =
                            Some(if by_name { t.prog.add(TExpr::Lambda(ast::ListRef::EMPTY, te)) } else { te });
                    }
                    i += 1;
                }
            }
            t.env.frames.pop();

            if kind != ClassKind::Trait {
                let mut params = Vec::new();
                if t.has_evidence_traits(c) {
                    params = t.fill_trait_params(c, parents, &mut tclass);
                }
                let inits = t.parent_inits(c, &mut params);
                tclass.init.extend(inits);
            }
            let outer_return = t.return_to.take();
            for stmt in body {
                match *stmt {
                    Stmt::Expr(e) => {
                        let mark = t.failure_mark();
                        let (te, ty) = t.type_expr(e, None);
                        t.warn_pure_statement(e, te, ty, mark);
                        tclass.init.push(TInit::Stmt(te));
                    }
                    Stmt::Def(d) => t.check_member(file, d, &mut tclass),
                    Stmt::Import(i) => t.enter_class_imports(c, i, imports_scope),
                }
            }
            if kind != ClassKind::Trait {
                let accessors = t.define_outer_accessors(c);
                tclass.methods.extend(accessors);
            }
            t.sites.owners.truncate(owners_depth);
            t.return_to = outer_return;
        });
        if kind != ClassKind::Trait {
            self.implement_deferred_givens(c, &mut tclass);
        }
        self.check_ctor_alternatives(c);
        if kind == ClassKind::Object && matches!(self.syms.class(c).owner, Owner::Package(_)) {
            let mut k = 0;
            let mut own = false;
            while let Some(m) = self.own_alternative(c, names::MAIN, k) {
                k += 1;
                if self.is_java_main(m) {
                    self.entry_points.push((m, None));
                    own = true;
                }
            }
            if !own {
                if let Some(m) = self.inherited_java_main(c) {
                    self.entry_points.push((m, Some(c)));
                }
            }
        }
        if kind == ClassKind::Enum {
            let children = self.syms.class(c).children.clone();
            for child in children {
                self.check_class(child);
            }
            // A synthesized companion is only emitted when it holds the values.
            let info = self.syms.class(c);
            if let (true, Some(companion)) = (info.stateful, info.companion) {
                if self.syms.class(companion).def.is_none() {
                    self.check_class(companion);
                }
            }
        }
        self.check_overrides(c);
        self.check_variances(c);
        if !matches!(kind, ClassKind::Trait | ClassKind::Enum) && def.mods & mods::ABSTRACT == 0 && !self.is_body_file(file) {
            tclass.forwarders = self.check_implements_all(c, Abstracts::Declared);
        }
        if js == JsKind::Object {
            // scalac checked a jar's trait, whose vars come with their setters.
            if kind == ClassKind::Trait {
                if !self.is_body_file(file) {
                    self.check_js_trait_body(c, body);
                }
            } else {
                let defaults = self.js_trait_defaults(c);
                tclass.init.splice(0..0, defaults.into_iter().map(|s| TInit::Field(s, self.prog.add(TExpr::Unit))));
            }
        } else {
            self.mark_accessors(c);
        }
        self.nowarn -= nowarn;
        if self.capturing() && self.syms.class(c).owner != Owner::Local {
            self.capture_class_annotations(c, file, def_id);
        }
        // The class's `TClass` is published once pushed: the plain inline calls of its statements
        // expand first.
        self.flush_pending_since(pending);
        self.push_class_body(tclass);
    }

    /// The annotations of a class of an owned file for its pickle: its own, where its owner sees
    /// them, its parameters' and type parameters', and those of its members but the classes,
    /// whose own checks type theirs.
    fn capture_class_annotations(&mut self, c: ClassId, file: FileId, d: DefId) {
        let outer = self.syms.class(c).owner;
        let ast = self.ast(file);
        let def = ast.def(d);
        if matches!(def.kind, DefKind::Class(_)) {
            self.capture_annotations(file, outer, def.span.start, &def.annots, &[]);
            self.capture_def_annotations(file, Owner::Class(c), d, false);
        }
        let body: &[Stmt] = match &def.kind {
            DefKind::Class(cls) => &cls.body,
            DefKind::Given(g) => &g.body,
            _ => &[],
        };
        for stmt in body {
            let Stmt::Def(m) = *stmt else { continue };
            if !matches!(ast.def(m).kind, DefKind::Class(_)) {
                self.capture_def_annotations(file, Owner::Class(c), m, true);
            }
        }
    }

    /// A JS trait declares the shape of a plain JS object: abstract members, and vals set to
    /// `js.undefined` that every instance carries as a property.
    fn check_js_trait_body(&mut self, c: ClassId, body: &[Stmt]) {
        let file = self.syms.class(c).file;
        let ast = self.ast(file);
        for stmt in body {
            let Stmt::Def(d) = stmt else { continue };
            let def = ast.def(*d);
            let concrete = match &def.kind {
                DefKind::Val { rhs: Some(e), .. } => !super::interop::is_js_undefined(ast, *e),
                DefKind::Fun(f) => f.body.is_some(),
                DefKind::Given(_) => true,
                _ => false,
            };
            if concrete {
                let msg = format!(
                    "{} is a member of a JS trait, which can only hold abstract members and vals set to js.undefined",
                    self.name_str(def.name)
                );
                self.diags.error(file, def.span, msg);
            }
        }
    }

    /// The `js.undefined` vals of the JS traits that `c` mixes in and does not define itself:
    /// an instance carries them as properties whose value is `undefined`.
    pub(super) fn js_trait_defaults(&mut self, c: ClassId) -> Vec<SymId> {
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        let mut out = Vec::new();
        let mut seen: Vec<crate::intern::Name> = self.syms.class(c).members.keys().copied().collect();
        for b in bases {
            if self.syms.class(b).js != JsKind::Object || self.syms.class(b).kind != ClassKind::Trait {
                continue;
            }
            for m in self.syms.class(b).member_order.clone() {
                let name = self.syms.sym(m).name;
                if seen.contains(&name) {
                    continue;
                }
                seen.push(name);
                if self.syms.sym(m).kind == SymKind::Val && !self.is_abstract_member(m) {
                    out.push(m);
                }
            }
        }
        out
    }

    /// The arguments an enum case passes to the constructor of its enum.
    fn check_enum_parent_args(&mut self, case: ClassId, cls: &'a ast::ClassDef) -> Option<ParentCall> {
        let (file, Owner::Class(companion)) = ({
            let i = self.syms.class(case);
            (i.file, i.owner)
        }) else {
            return None;
        };
        let e = self.syms.class(companion).companion?;
        let ast = self.ast(file);
        let enum_parent = self.enum_parent_clause(file, cls, e);
        for p in &cls.parents {
            if !self.passes_evidence_only(case, p) && !enum_parent.map_or(false, |ep| std::ptr::eq(ep, p)) {
                self.diags.error(file, ast.ty_spans[p.ty.idx()], "only the enum itself takes constructor arguments");
            }
        }
        if !self.syms.class(e).stateful && enum_parent.map_or(true, |p| p.args.is_empty()) {
            return None;
        }
        if let Some(&call) = self.inferred_parent_args.get(&case) {
            return Some(call);
        }
        let parents = self.syms.class(case).parents.clone();
        let parent_ty = parents.into_iter().find(|&t| self.class_of(t) == Some(e))?;
        let call = self.type_enum_parent(case, cls, e, Some(parent_ty)).map(|(call, _)| call);
        call.filter(|_| self.syms.class(e).stateful)
    }

    /// Which of `toString`, `equals` and `hashCode` (bits 0 to 2) a case class takes from its
    /// superclass: it defines none itself, and the first concrete one behind it in the
    /// linearisation belongs to the superclass or to what that class extends.
    fn inherited_case_members(&self, c: ClassId) -> u8 {
        let info = self.syms.class(c);
        let Some(parent) = info.superclass else { return 0 };
        let mut kept = 0;
        for (bit, name) in [names::TO_STRING, names::EQUALS, names::HASH_CODE].into_iter().enumerate() {
            if info.members.contains_key(&name) {
                continue;
            }
            let definer = info.base_types.iter().skip(1).map(|&(b, _)| b).find(|&b| {
                self.syms.class(b).members.get(&name).map_or(false, |&m| !self.is_abstract_member(m))
            });
            let from_superclass =
                definer.map_or(false, |b| self.syms.class(parent).base_types.iter().any(|&(x, _)| x == b));
            kept |= (from_superclass as u8) << bit;
        }
        kept
    }

    /// The parent clause that names the superclass of `c`, when the class names one itself rather
    /// than getting it from a trait.
    pub(super) fn superclass_clause(&mut self, c: ClassId, parents: &'a [ast::Parent]) -> Option<&'a ast::Parent> {
        let first_parent = *self.syms.class(c).parents.first()?;
        let first = self.class_of(first_parent)?;
        let is_class = self.syms.class(first).kind == ClassKind::Class && self.syms.class(first).js != JsKind::Native;
        parents.first().filter(|_| is_class)
    }

    /// The arguments `c` passes to the constructor of its superclass, typed where the extends
    /// clause stands: the scope around the class with its constructor parameters, without `this`
    /// and the members. An anonymous class evaluates them where it is created.
    fn check_parent_args(&mut self, c: ClassId, parents: &'a [ast::Parent]) -> Option<ParentCall> {
        let (file, kind, superclass) = {
            let i = self.syms.class(c);
            (i.file, i.kind, i.superclass)
        };
        let clause = self.superclass_clause(c, parents);
        let all_accepted = self.syms.class(c).parents.len() >= parents.len();
        for p in parents.iter().filter(|_| all_accepted) {
            let of_trait = self.evidence_trait_of(c, p);
            let span = self.ast(file).ty_spans[p.ty.idx()];
            let passes_args = p.args.iter().any(|(l, _)| !l.is_empty());
            if !clause.map_or(false, |sc| std::ptr::eq(sc, p)) && passes_args && of_trait.is_none() {
                let what = self.ast(file).ty(match self.ast(file).ty(p.ty) {
                    ast::TyExpr::Apply(f, _) => f,
                    _ => p.ty,
                });
                let name = match what {
                    ast::TyExpr::Name(n) | ast::TyExpr::Select(_, n) => self.name_str(n),
                    _ => String::new(),
                };
                self.diags.error(file, span, format!("too many arguments for constructor {0} in trait {0}: (): {0}", name));
            } else if let Some(tr) = of_trait.filter(|_| kind == ClassKind::Trait && !p.args.is_empty()) {
                let msg =
                    format!("{} may not call constructor of {}", self.class_description(c), self.class_description(tr));
                self.diags.error(file, span, msg);
            }
        }
        let superclass = superclass?;
        if let Some(p) = clause.filter(|p| kind == ClassKind::Trait && !p.args.is_empty()) {
            let msg = format!(
                "{} may not call constructor of {}",
                self.class_description(c),
                self.class_description(superclass)
            );
            let span = self.ast(file).ty_spans[p.ty.idx()];
            self.diags.error(file, span, msg);
        }
        if kind == ClassKind::Anon || kind == ClassKind::Trait {
            return None;
        }
        if let Some(&call) = self.inferred_parent_args.get(&c) {
            return Some(call);
        }
        let parent_ty = self.syms.class(c).base_types.iter().find(|&&(b, _)| b == superclass).map(|&(_, t)| t)?;
        let call = self.type_parent_ctor(c, clause, superclass, Some(parent_ty), false).map(|(call, _)| call)?;
        (!call.args.is_empty() || call.via.is_some()).then_some(call)
    }

    /// The trait with using clauses or context bounds that a parent clause of `c` names.
    fn evidence_trait_of(&self, c: ClassId, parent: &ast::Parent) -> Option<ClassId> {
        let info = self.syms.class(c);
        let ast = self.ast(info.file);
        let head = match ast.ty(parent.ty) {
            ast::TyExpr::Apply(f, _) => f,
            _ => parent.ty,
        };
        let mut bases = info.base_types.iter().skip(1).map(|&(b, _)| b);
        match ast.ty(head) {
            ast::TyExpr::Name(n) | ast::TyExpr::Select(_, n) => bases.find(|&b| self.syms.class(b).name == n && self.has_trait_params(b)),
            // A converted library body names the parent by its resolved type.
            ast::TyExpr::Resolved(t) => match self.types.get(t) {
                Type::Class(k, _) | Type::Ctor(k) => bases.find(|&b| b == k && self.has_trait_params(b)),
                _ => None,
            },
            _ => None,
        }
    }

    /// A trait with ordinary parameters, which its body takes as arguments.
    fn takes_arguments(&self, c: ClassId) -> bool {
        self.has_trait_params(c) && !self.is_evidence_trait(c)
    }

    pub(super) fn has_trait_params(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        info.kind == ClassKind::Trait && !info.ctor.is_empty()
    }

    /// Whether the arguments of a parent clause, if any, go to a trait with parameters.
    fn passes_evidence_only(&self, c: ClassId, parent: &ast::Parent) -> bool {
        parent.args.is_empty() || self.evidence_trait_of(c, parent).is_some()
    }

    /// A trait leaves its parameters to the first class that mixes it in. Using clauses and
    /// context bounds are resolved where the extends clause stands, as scalac types the arguments
    /// of a parent: in the scope around the class with the constructor parameters visible, not in
    /// the class body, whose members would be read before they exist. The fields of a trait are
    /// set before its body runs: evidence ahead of every trait body, which goes into `tclass`,
    /// while a trait with ordinary parameters takes all of them as the arguments of its body,
    /// evaluated right before it runs as under scalac; those are returned by trait. An enum does
    /// all that for its cases.
    fn fill_trait_params(
        &mut self,
        c: ClassId,
        parents: &'a [ast::Parent],
        tclass: &mut TClass,
    ) -> Vec<(ClassId, ParentCall)> {
        let (file, class_span, kind, owner) = {
            let i = self.syms.class(c);
            (i.file, i.span, i.kind, i.owner)
        };
        let mut outside = self.env.clone();
        if matches!(outside.frames.last(), Some(Frame::Class(k)) if *k == c) {
            outside.frames.pop();
        }
        let mut names = Vec::new();
        let mut givens = Vec::new();
        for clause in &self.syms.class(c).ctor {
            for p in &clause.params {
                names.push((p.name, p.sym));
                if clause.is_using {
                    givens.push(p.sym);
                }
            }
        }
        self.settle_class(c);
        let tparams = self.syms.class(c).tparams.iter().map(|&p| (self.syms.tparam(p).name, p)).collect();
        outside.frames.push(Frame::Locals { names, tparams, givens, classes: Vec::new(), aliases: Vec::new() });
        let enum_class = match (kind, owner) {
            (ClassKind::EnumCase, Owner::Class(companion)) => self.syms.class(companion).companion,
            _ => None,
        };
        let filler = enum_class.or(self.syms.class(c).superclass);
        let filled_above =
            |t: &Self, b: ClassId| filler.map_or(false, |e| t.syms.class(e).base_types.iter().any(|&(x, _)| x == b));
        let bases: Vec<(ClassId, TypeId)> = self
            .syms
            .class(c)
            .base_types
            .iter()
            .skip(1)
            .copied()
            .filter(|&(b, _)| self.has_trait_params(b))
            .collect();
        let ast = self.ast(file);
        let mut by_trait = Vec::new();
        for (b, base_ty) in bases {
            let clause = parents.iter().find(|p| self.evidence_trait_of(c, p) == Some(b));
            let evidence_only = self.is_evidence_trait(b);
            if filled_above(self, b) {
                if let (Some(p), Some(above)) = (clause.filter(|p| !p.args.is_empty()), filler) {
                    let msg = format!(
                        "{} is already implemented by super{}, its constructor cannot be called again",
                        self.class_description(b),
                        self.class_description(above)
                    );
                    self.diags.error(file, ast.ty_spans[p.ty.idx()], msg);
                }
                continue;
            }
            if clause.is_none() && !evidence_only {
                let msg = format!(
                    "parameterized {} is indirectly implemented, needs to be implemented directly so that arguments can be passed",
                    self.class_description(b)
                );
                self.diags.error(file, class_span, msg);
                continue;
            }
            // A trait whose type arguments its arguments inferred was applied then.
            let inferred = self.inferred_trait_args.get(&c).and_then(|calls| calls.iter().find(|(t, _)| *t == b).map(|&(_, call)| call));
            if let Some(call) = inferred {
                by_trait.push((b, call));
                continue;
            }
            let Type::Class(_, targs) = self.types.get(base_ty) else { continue };
            self.settle_class(b);
            let owner_subst: Subst =
                self.syms.class(b).tparams.iter().copied().zip(self.types.items(targs).iter().copied()).collect();
            let sig = Arc::new(MethodSig { tparams: Vec::new(), clauses: self.syms.class(b).ctor.clone(), ret: base_ty });
            let span = clause.map_or(class_span, |p| ast.ty_spans[p.ty.idx()]);
            let mut lists: Vec<ArgList> = clause
                .map(|p| {
                    p.args
                        .iter()
                        .filter(|&&(_, using)| using || !evidence_only)
                        .map(|&(l, using)| ArgList {
                            args: ast.expr_list(l).iter().map(|&a| ArgSrc::Ast(a)).collect(),
                            using,
                            span,
                        })
                        .collect()
                })
                .unwrap_or_default();
            if !evidence_only && !lists.iter().any(|l| !l.using) {
                lists.insert(0, ArgList { args: Vec::new(), using: false, span });
            }
            let call = MethodCall { recv: None, sym: SymId(u32::MAX), owner_subst, ext_recv: None, prefix: None };
            let applied = self.with_env(outside.clone(), |t| {
                let outer = t.parent_args_of.replace(c);
                let r = t.apply_method(call, Some((b, sig.clone())), None, lists, span, None, false);
                t.parent_args_of = outer;
                r
            });
            let Some((te, _)) = applied else { continue };
            let (prelude, new) = match self.prog.expr(te) {
                TExpr::Block(stmts, new) => (stmts, new),
                _ => (ast::ListRef::EMPTY, te),
            };
            let TExpr::New(_, args) = self.prog.expr(new) else { continue };
            if evidence_only {
                let args = self.prog.expr_list(args).to_vec();
                let fields = sig.clauses.iter().flat_map(|cl| cl.params.iter().map(|p| p.sym));
                tclass.init.extend(fields.zip(args).map(|(field, arg)| TInit::Field(field, arg)));
            } else {
                by_trait.push((b, ParentCall { prelude, args, via: None }));
            }
        }
        by_trait
    }

    /// Whether the body of the trait `b` has statements to run.
    fn trait_has_statements(&self, b: ClassId) -> bool {
        let info = self.syms.class(b);
        info.kind == ClassKind::Trait && info.has_statements && info.js == JsKind::Scala
    }

    pub(super) fn inherits_trait_statements(&self, c: ClassId) -> bool {
        self.syms.class(c).base_types.iter().skip(1).any(|&(b, _)| self.trait_has_statements(b))
    }

    /// The trait bodies a constructor of `c` runs, ancestors first as scalac orders them, each
    /// behind the arguments `params` has for its trait. The `$init` of an enum runs those of the
    /// enum for its cases, and the constructor of a superclass those of its own traits.
    pub(super) fn parent_inits(&self, c: ClassId, params: &mut Vec<(ClassId, ParentCall)>) -> Vec<TInit> {
        let info = self.syms.class(c);
        let run_above: Option<&ClassInfo> = match (info.kind, info.owner) {
            (ClassKind::EnumCase, Owner::Class(companion)) => {
                self.syms.class(companion).companion.map(|e| self.syms.class(e).info)
            }
            _ => info.superclass.map(|s| self.syms.class(s).info),
        };
        let mut inits = Vec::new();
        for &(b, _) in info.base_types.iter().skip(1).rev() {
            if let Some(at) = params.iter().position(|(tr, _)| *tr == b) {
                inits.push(TInit::Parent(b, params.swap_remove(at).1));
            } else if self.trait_has_statements(b)
                && !self.takes_arguments(b)
                && !run_above.map_or(false, |e| e.base_types.iter().any(|&(x, _)| x == b))
            {
                inits.push(TInit::Parent(b, ParentCall::NONE));
            }
        }
        inits
    }

    /// The parent clause of an enum case that names the enum, `cls` the case's definition in
    /// `file`: the enum's own file where both are source, another module's pseudo file each
    /// where they come from a product.
    pub(super) fn enum_parent_clause(&self, file: FileId, cls: &'a ast::ClassDef, e: ClassId) -> Option<&'a ast::Parent> {
        let info = self.syms.class(e);
        let ast = self.ast(file);
        cls.parents.iter().find(|p| {
            let head = match ast.ty(p.ty) {
                ast::TyExpr::Apply(f, _) => f,
                _ => p.ty,
            };
            match ast.ty(head) {
                ast::TyExpr::Name(n) | ast::TyExpr::Select(_, n) => n == info.name,
                // A case converted from a jar names the enum by its resolved type.
                ast::TyExpr::Resolved(t) => matches!(self.types.get(t), Type::Class(k, _) if k == e),
                _ => false,
            }
        })
    }

    /// Types the application of the enum's constructor in the extends clause of a case, the
    /// evidence of context bounds included. The arguments belong to the scope of the companion,
    /// where a class case also sees its own parameters, and are evaluated by the constructor of
    /// the case. Without `parent_ty` the type arguments of the enum are inferred from them.
    pub(super) fn type_enum_parent(
        &mut self,
        case: ClassId,
        cls: &'a ast::ClassDef,
        e: ClassId,
        parent_ty: Option<TypeId>,
    ) -> Option<(ParentCall, TypeId)> {
        let clause = self.enum_parent_clause(self.syms.class(case).file, cls, e);
        self.type_parent_ctor(case, clause, e, parent_ty, false)
    }

    /// Types the call of the constructor of `parent` that the parent clause `clause` of `c`
    /// stands for; without a clause the constructor gets no arguments. The constructor
    /// parameters and type parameters of `c` are in scope, its members and `this` are not, unless
    /// the arguments are evaluated `in_place`, which is where an anonymous class is created.
    /// Without `parent_ty` the type arguments of the parent are inferred from the arguments.
    pub(super) fn type_parent_ctor(
        &mut self,
        c: ClassId,
        clause: Option<&'a ast::Parent>,
        parent: ClassId,
        parent_ty: Option<TypeId>,
        in_place: bool,
    ) -> Option<(ParentCall, TypeId)> {
        let (file, class_span) = {
            let i = self.syms.class(c);
            (i.file, i.span)
        };
        let ast = self.ast(file);
        self.settle_class(parent);
        let (sig_tparams, owner_subst, ret) = match parent_ty.map(|t| (t, self.types.get(t))) {
            Some((t, Type::Class(_, targs))) => {
                let tparams = self.syms.class(parent).info.tparams.iter().copied();
                (Vec::new(), tparams.zip(self.types.items(targs).iter().copied()).collect(), t)
            }
            _ => (self.syms.class(parent).tparams.clone(), Vec::new(), self.syms.class(parent).base_types[0].1),
        };
        let sig = Arc::new(MethodSig { tparams: sig_tparams, clauses: self.syms.class(parent).ctor.clone(), ret });
        let span = clause.map_or(class_span, |p| ast.ty_spans[p.ty.idx()]);
        let mut lists: Vec<ArgList> = clause
            .map(|p| {
                p.args
                    .iter()
                    .map(|&(l, using)| ArgList {
                        args: ast.expr_list(l).iter().map(|&a| ArgSrc::Ast(a)).collect(),
                        using,
                        span,
                    })
                    .collect()
            })
            .unwrap_or_default();
        // A Java class has its constructors as alternatives only, each with a parameter list.
        let has_normal_clause = sig.clauses.iter().any(|cl| !cl.is_using)
            || (self.syms.class(parent).mods & crate::ast::mods::PRIVATE_CTOR != 0 && !self.syms.class(parent).ctors.is_empty() && self.is_java_class(parent));
        if lists.is_empty() && has_normal_clause {
            lists.push(ArgList { args: Vec::new(), using: false, span });
        }
        // `extends E()` for a parent declared without a parameter list.
        if !has_normal_clause && lists.first().map_or(false, |l| !l.using && l.args.is_empty()) {
            lists.remove(0);
        }
        let (te, ty) = if in_place {
            self.construct_parent(parent, sig, owner_subst, lists, span)
        } else {
            let mut names = Vec::new();
            let mut givens = Vec::new();
            for clause in &self.syms.class(c).ctor {
                for p in &clause.params {
                    names.push((p.name, p.sym));
                    if clause.is_using {
                        givens.push(p.sym);
                    }
                }
            }
            self.settle_class(c);
            let tparams: Vec<(crate::intern::Name, TParamId)> =
                self.syms.class(c).tparams.iter().map(|&p| (self.syms.tparam(p).name, p)).collect();
            let mut env = self.env_for(file, Owner::Class(c));
            env.frames.push(Frame::Locals { names, tparams, givens, classes: Vec::new(), aliases: Vec::new() });
            self.with_env(env, |t| {
                let outer_case = t.parent_args_of.replace(c);
                let r = t.construct_parent(parent, sig, owner_subst, lists, span);
                t.parent_args_of = outer_case;
                r
            })
        };
        // Named arguments out of parameter order are bound to temporaries in front of the call.
        let (prelude, call) = match self.prog.expr(te) {
            TExpr::Block(stmts, call) => (stmts, call),
            _ => (ast::ListRef::EMPTY, te),
        };
        match self.prog.expr(call) {
            TExpr::New(_, args) => Some((ParentCall { prelude, args, via: None }, ty)),
            TExpr::NewVia(s, args) => Some((ParentCall { prelude, args, via: Some(s) }, ty)),
            _ => None,
        }
    }

    /// The constructor call of a parent clause, an overload resolution where the parent has
    /// secondary constructors: their signatures under the parent's type arguments, as `sig` is.
    fn construct_parent(&mut self, parent: ClassId, sig: Arc<MethodSig>, owner_subst: Subst, lists: Vec<ArgList>, span: Span) -> (TExprId, TypeId) {
        // A parent whose header the parser could not complete takes any arguments.
        if self.syms.class(parent).mods & mods::INCOMPLETE != 0 {
            self.type_args_for_errors(&lists);
            return (self.prog.add(TExpr::New(parent, ast::ListRef::EMPTY)), sig.ret);
        }
        if self.loaded.is_some() {
            self.absorb_java_ctors(parent);
        }
        let secondaries: Vec<SymId> =
            self.syms.class(parent).ctors.iter().copied().filter(|&s| Some(s) != self.excluded_ctor && self.ctor_callable_here(s)).collect();
        if secondaries.is_empty() {
            let call = MethodCall { recv: None, sym: SymId(u32::MAX), owner_subst, ext_recv: None, prefix: None };
            return self.apply_method(call, Some((parent, sig)), None, lists, span, None, false).unwrap();
        }
        // Every alternative, the primary included, takes the parent's type arguments.
        let fix = |t: &mut Self, own: Arc<MethodSig>| -> Arc<MethodSig> {
            if owner_subst.is_empty() {
                return own;
            }
            let clauses: Vec<ClauseSig> = own
                .clauses
                .iter()
                .map(|cl| ClauseSig {
                    params: cl.params.iter().map(|p| ParamSig { ty: t.types.subst(p.ty, &owner_subst), ..p.clone() }).collect(),
                    is_using: cl.is_using,
                    is_implicit: cl.is_implicit,
                })
                .collect();
            Arc::new(MethodSig { tparams: own.tparams.clone(), clauses, ret: sig.ret })
        };
        let primary = self.syms.class(parent).primary_ctor.filter(|_| self.ctor_accessible(parent)).map(|s| (s, fix(self, sig.clone())));
        let mut alts = Vec::with_capacity(secondaries.len());
        for s in secondaries {
            let own = self.sig_arc(s);
            let fixed = fix(self, own);
            alts.push((s, fixed));
        }
        self.construct_overloaded(parent, primary, alts, None, lists, span, None)
    }

    fn check_member(&mut self, file: FileId, d: DefId, tclass: &mut TClass) {
        self.restrict_def(file, d);
        if self.unused.on() {
            self.mark_annotations(file, d);
            self.mark_module_prefixes(file, d, Owner::Class(tclass.id), false);
        }
        let def = self.ast(file).def(d);
        match &def.kind {
            DefKind::Val { rhs, .. } => {
                let Some(&sym) = self.def_syms.get(file.0 as usize, &d) else { return };
                // A val of a jar's class is typed when the program reaches it and read
                // through a getter, so that only what the program uses is compiled; one that
                // implements a def of an ancestor is an accessor instead and stays eager. A
                // product's class is another module's program and initialises as the whole
                // program does.
                if rhs.is_some() && self.is_library_class(tclass.id) && !self.is_product_class(tclass.id) && self.syms.sym(sym).kind == SymKind::Val && !self.jvm && !self.implements_def(tclass.id, sym) {
                    self.with_loader(|w| w.syms.sym_mut(sym).mods |= mods::LAZY);
                    return;
                }
                self.class_member_body(sym);
                if let (Some(_), Some(init)) = (rhs, self.readable_init(sym)) {
                    tclass.init.push(TInit::Field(sym, init));
                }
            }
            DefKind::Fun(_) if def.name == names::INIT => {
                let Some(&sym) = self.def_syms.get(file.0 as usize, &d) else { return };
                self.type_secondary_ctor(tclass.id, sym, tclass);
            }
            DefKind::Fun(_) => {
                let Some(&sym) = self.def_syms.get(file.0 as usize, &d) else { return };
                if self.syms.class(tclass.id).kind == ClassKind::Object {
                    self.misplaced_abstract_given(def);
                }
                if !self.defers_body(sym) {
                    // A member of an anonymous class in a library body is compiled with the
                    // body, whether the program calls it or not: what does not type is
                    // reported only when the reach marks it.
                    let lenient = self.loaded.as_ref().map_or(false, |l| l.is_jar(file))
                        && matches!(self.syms.class(tclass.id).kind, ClassKind::Anon)
                        && !self.jvm;
                    let (mark, withheld) = (self.diags.items.len(), self.withheld_met);
                    self.class_member_body(sym);
                    if lenient && self.diags.items.len() > mark {
                        let taken: Vec<crate::source::Diagnostic> = self.diags.items.drain(mark..).collect();
                        if let Some(f) = self.readable_fun(sym) {
                            self.poisoned.insert(f, taken);
                            if self.withheld_met != withheld {
                                self.poisoned_withheld.insert(f, ());
                            }
                        }
                        self.withheld_met = withheld;
                    }
                }
                if let Some(f) = self.readable_fun(sym) {
                    tclass.methods.push(f);
                }
                if self.syms.sym(sym).is_main {
                    if self.statically_accessible(tclass.id) {
                        self.enter_main_method(sym);
                    } else {
                        let msg = format!("method {} cannot be a main method since it cannot be accessed statically", self.name_str(def.name));
                        self.error_at(sym, msg);
                    }
                }
            }
            DefKind::Class(_) => {
                let span = def.span;
                if let Some(&nested) = self.def_classes.get(file.0 as usize, &d) {
                    self.note_nested_stored_class(tclass.id, nested);
                    self.check_class(nested);
                    // The instance of an object nested here is made on the first read of its
                    // lazy val, with this instance as its outer.
                    if let Some(val) = self.syms.class(nested).inner_object {
                        let init = self.new_instance(nested, ast::ListRef::EMPTY, span);
                        tclass.init.push(TInit::Field(val, init));
                    }
                }
            }
            DefKind::Given(_) => {
                let Some(&sym) = self.def_syms.get(file.0 as usize, &d) else { return };
                self.check_given(sym);
                if let Some(&init) = self.val_init.get(&sym) {
                    // Traits have no initialisers, but a lazy member is mixed in as a getter, and
                    // a given is initialised on first use anyway.
                    if self.syms.class(tclass.id).kind == ClassKind::Trait {
                        self.with_loader(|w| w.syms.sym_mut(sym).mods |= mods::LAZY);
                    }
                    tclass.init.push(TInit::Field(sym, init));
                }
                if let Some(f) = self.readable_fun(sym) {
                    tclass.methods.push(f);
                }
            }
            DefKind::TypeAlias { .. } => {}
        }
    }

    /// scalac's E067 for an abstract given (`given x: T`, a def flagged `Given` without a body)
    /// where nothing can implement it: a member of an object or a package, or a local; reported
    /// when `def` is one.
    pub(super) fn misplaced_abstract_given(&mut self, def: &ast::Def) -> bool {
        if def.mods & mods::GIVEN == 0 {
            return false;
        }
        let DefKind::Fun(f) = &def.kind else { return false };
        if f.body.is_some() {
            return false;
        }
        let msg = format!("Declaration of given instance {} not allowed here: only classes can have declared but undefined members", self.name_str(def.name));
        self.error(def.span, msg);
        true
    }

    /// Whether the definition of `m` carries `@native` (a JVM method whose body is native code).
    fn annotated_native(&self, m: SymId) -> bool {
        let info = self.syms.sym(m);
        info.def.is_some_and(|d| self.ast(info.file).def(d).annots.iter().any(|a| a.name == names::NATIVE))
    }

    /// Whether an ancestor of `c` declares a def of the val `sym`'s name.
    pub(super) fn implements_def(&self, c: ClassId, sym: SymId) -> bool {
        let name = self.syms.sym(sym).name;
        self.syms.class(c).base_types.iter().skip(1).any(|&(b, _)| {
            self.syms.class(b).members.get(&name).map_or(false, |&p| matches!(self.syms.sym(p).kind, SymKind::Def | SymKind::Overloaded(_)))
        })
    }

    /// `abstract override def`: a member of a trait that still needs the implementation its
    /// `super` call goes to.
    fn is_abstract_override(&self, sym: SymId) -> bool {
        let stackable = mods::ABSTRACT | mods::OVERRIDE;
        self.syms.sym(sym).mods & stackable == stackable
    }

    /// Per symbol, whether it is a member without a body, for the reach pass.
    pub fn abstract_members(&self) -> Vec<bool> {
        (0..self.syms.syms.len()).map(|i| self.is_abstract_member(SymId(i as u32))).collect()
    }

    /// `abstract_members` extended over the symbols made since `known` was: what a symbol was
    /// answered stays its answer.
    pub fn extend_abstract_members(&self, known: &mut Vec<bool>) {
        let first = known.len();
        known.extend((first..self.syms.syms.len()).map(|i| self.is_abstract_member(SymId(i as u32))));
    }

    /// Whether `sym` is declared without a body, whatever template it carries; nothing is typed.
    pub fn declared_without_body(&self, sym: SymId) -> bool {
        let s = self.syms.sym(sym);
        let Some(d) = s.def else { return false };
        matches!(&self.ast(s.file).def(d).kind, DefKind::Fun(f) if f.body.is_none())
    }

    pub(crate) fn is_abstract_member(&self, sym: SymId) -> bool {
        let s = self.syms.sym(sym);
        let declared_abstract = || {
            // A local symbol of a body typed again in a session keeps its place in the tree the
            // retype replaced (`move_symbols`): dead, it may have none in the new one.
            let Some(def) = s.def.and_then(|d| self.ast(s.file).defs.get(d.idx())) else { return s.mods & mods::ABSTRACT != 0 };
            match &def.kind {
                DefKind::Fun(f) => f.body.is_none(),
                DefKind::Val { rhs, .. } => rhs.is_none(),
                // A deferred given read from a pickle is `ABSTRACT` as well.
                DefKind::Given(_) => s.mods & mods::DEFERRED != 0,
                _ => false,
            }
        };
        if s.intrinsic.is_some() {
            // `scala.reflect.Enum.ordinal` carries the template every enum answers with and
            // is abstract to a class that extends the trait itself, as scalac declares it. A
            // Java interface's method without a body is abstract too, its template being how a
            // call reaches the JS primitives that implement it; a default method has a body.
            return match s.owner {
                Owner::Class(c) if Some(c) == self.b.reflect_enum => true,
                Owner::Class(c) => s.java_defined && self.syms.class(c).kind == ClassKind::Trait && declared_abstract(),
                _ => false,
            };
        }
        declared_abstract()
    }

    /// The `k`-th member of `b` called `name` that a member of a subclass may override or
    /// implement: several when the name is overloaded in `b`.
    pub(super) fn inherited_member(&self, b: ClassId, name: Name, is_ext: bool, k: usize) -> Option<SymId> {
        if is_ext {
            let info = self.syms.class(b);
            info.extensions.iter().copied().filter(|&s| self.syms.sym(s).name == name).nth(k)
        } else {
            self.own_alternative(b, name, k)
        }
    }

    fn in_std(&self, sym: SymId) -> bool {
        self.source(self.syms.sym(sym).file).is_std
    }

    /// Every member of `c` against the members of its ancestors that share its name, and, when
    /// `c` has several parents, what they bring in against each other. Members without a
    /// definition are the compiler's own (`copy`, `values`) and are not checked. The standard
    /// library narrows signatures by name (`Map.map`, `Range.reverse`) and is not checked.
    /// What the check settles of the members (accessors, the parameters that override a val,
    /// the names of the alternatives) is written to records of the shared region when the
    /// class is a prefix one, each write a copy under the loader's lock (`shared_write`); the
    /// comparisons run outside it, where a signature another worker holds is waited for.
    fn check_overrides(&mut self, c: ClassId) {
        self.check_overrides_unlocked(c)
    }

    /// Decides again, from the flags every body left, what the override checks decided from
    /// the flags they read (`override_pairs`): whether a method is an alternative is set by the
    /// merge of an entry with the inherited alternatives, which a lookup makes when it gets
    /// there, so what a check read depended on the merges made before it, the workers' order.
    /// The flags only ever turn on, and every merge a body makes is made by the end, so the
    /// decisions made again contain the ones made on the way whatever the order was.
    pub(super) fn decide_override_pairs(&mut self) {
        let pairs = std::mem::take(&mut self.override_pairs);
        if pairs.is_empty() {
            return;
        }
        let mut pending: FxMap<SymId, ()> = self.dispatch_pending.iter().map(|&m| (m, ())).collect();
        for (m, p) in pairs {
            let (alternative, meets_inherited, named) = {
                let inherited = self.syms.sym(p);
                (inherited.alternative, inherited.meets_inherited, inherited.dispatch == Dispatch::Named)
            };
            if alternative || meets_inherited || self.syms.sym(m).alternative {
                self.push_override(m, p);
            }
            if (alternative || meets_inherited || named) && pending.insert(m, ()).is_none() {
                self.dispatch_pending.push(m);
            }
        }
    }

    fn check_overrides_unlocked(&mut self, c: ClassId) {
        if self.source(self.syms.class(c).file).is_std {
            // A class of a converted library body (an anonymous `Codec`) is not checked, but
            // its methods that implement an alternative of an overloaded name take that name.
            if self.is_body_file(self.syms.class(c).file) {
                self.name_body_overrides(c);
            }
            return;
        }
        for i in 0..self.own_member_count(c) {
            let m = self.own_member(c, i);
            let (name, is_ext, m_mods, has_def) = {
                let s = self.syms.sym(m);
                (s.name, s.is_extension, s.mods, s.def.is_some())
            };
            // A plain constructor parameter is local to its class and overrides nothing, and
            // neither does a private val, which scalac lets share its name with an inherited member.
            if !has_def && (m_mods & mods::FIELD == 0 || !self.is_ctor_param(c, m)) {
                continue;
            }
            if m_mods & (mods::LAZY | mods::OVERRIDE) == 0 && self.syms.sym(m).kind == SymKind::Val && self.is_private(m) {
                continue;
            }
            if m_mods & mods::FINAL != 0 && self.syms.sym(m).kind == SymKind::Def && self.is_abstract_member(m) {
                let msg = format!("abstract method {} may not have `final` modifier", self.name_str(name));
                self.error_at(m, msg);
            }
            let mut overrides = false;
            // Whether `m` overrides a method whose name in the output may not be the plain one,
            // and whether it is overloaded with a method it inherits.
            let (mut named_root, mut overloads_inherited) = (false, false);
            for j in 1..self.syms.class(c).base_types.len() {
                let (b, bt) = self.syms.class(c).base_types[j];
                let mut k = 0;
                while let Some(p) = self.inherited_member(b, name, is_ext, k) {
                    k += 1;
                    if self.is_private(p) {
                        continue;
                    }
                    let overridden = self.check_override(c, m, None, p, bt);
                    if overridden && self.index.is_some() {
                        self.index_override(m, p);
                    }
                    overrides |= overridden;
                    let (alternative, meets_inherited, named) = {
                        let inherited = self.syms.sym(p);
                        (inherited.alternative, inherited.meets_inherited, inherited.dispatch == Dispatch::Named)
                    };
                    if overridden && (alternative || meets_inherited || self.syms.sym(m).alternative) {
                        self.shared_write(m, |w| w.push_override(m, p));
                    }
                    if overridden && !self.alternatives_named {
                        self.override_pairs.push((m, p));
                    }
                    named_root |= overridden && (alternative || meets_inherited || named);
                    let inherited = self.syms.sym(p);
                    overloads_inherited |= !overridden && inherited.kind == SymKind::Def && !is_ext;
                    if !inherited.alternative && !is_ext {
                        break;
                    }
                }
            }
            if named_root {
                if self.alternatives_named {
                    self.dispatch_name(m);
                } else {
                    self.dispatch_pending.push(m);
                }
            }
            if overloads_inherited && self.syms.sym(m).kind == SymKind::Def {
                self.settle_member(c, name);
            }
            if self.is_abstract_override(m) && self.syms.class(c).kind != ClassKind::Trait {
                self.error_at(m, "abstract override modifier only allowed for members of traits".to_string());
            }
            if self.syms.sym(m).kind == SymKind::Def && !is_ext {
                self.check_target_name_clashes(c, m);
            }
            if overrides {
                continue;
            }
            // A `val toString` or `val hashCode` implements the method of `Any` too, called as
            // a method through its accessor.
            let overrides_any = matches!(name, names::TO_STRING | names::HASH_CODE | names::EQUALS | names::CLONE)
                && !is_ext
                && matches!(self.syms.sym(m).kind, SymKind::Def | SymKind::Val)
                && self.matches_any_member(m);
            if overrides_any && m_mods & mods::OVERRIDE == 0 && !self.is_abstract_member(m) {
                let msg = format!("method {} needs `override` modifier to override the {} of Any", self.name_str(name), self.name_str(name));
                self.error_at(m, msg);
            } else if !overrides_any && m_mods & mods::OVERRIDE != 0 {
                let msg = format!("{} overrides nothing", self.member_description(m, None));
                self.error_at(m, msg);
            }
        }
        if self.syms.class(c).parents.len() > 1 {
            self.check_inherited_pairs(c);
        }
        if self.syms.class(c).base_types.len() > 2 {
            self.check_final_overrides(c);
        }
        if self.syms.class(c).inherits_types {
            if !self.syms.class(c).type_aliases.is_empty() {
                self.check_type_member_overrides(c);
            }
            self.check_inherited_type_members(c);
        }
    }

    /// Every type member `c` inherits is looked up through `c` once, which finds the members
    /// that the mixed-in traits define through each other (`type A = B` with `type B = A`).
    #[cold]
    fn name_body_overrides(&mut self, c: ClassId) {
        if self.syms.class(c).base_types.len() <= 1 {
            return;
        }
        for i in 0..self.own_member_count(c) {
            let m = self.own_member(c, i);
            let info = self.syms.sym(m);
            if info.kind != SymKind::Def || info.is_extension || info.dispatch != Dispatch::Unknown {
                continue;
            }
            if self.alternatives_named {
                self.dispatch_name(m);
            } else {
                self.dispatch_pending.push(m);
            }
        }
    }

    fn check_inherited_type_members(&mut self, c: ClassId) {
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        let mut names: Vec<Name> = Vec::new();
        for b in bases {
            for &n in self.syms.class(b).type_aliases.keys() {
                if !names.contains(&n) {
                    names.push(n);
                }
            }
        }
        names.sort();
        let this = self.this_prefix(c);
        for n in names {
            if let Some(t) = self.member_type(this, n) {
                self.member_upper(t);
            }
        }
    }

    /// Every type member of `c` against the one of its name an ancestor declares: an alias
    /// has to lie within the bounds it implements, bounds within the bounds, and an alias that
    /// overrides an alias with `override` has to equal it. Both are seen from `c`, whose other
    /// members may fix what the bounds mention.
    #[cold]
    fn check_type_member_overrides(&mut self, c: ClassId) {
        let mut own: Vec<(Name, AliasId)> = self.syms.class(c).type_aliases.iter().map(|(&n, &a)| (n, a)).collect();
        own.sort_by_key(|&(_, a)| a);
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types[1..].to_vec();
        let this = self.this_prefix(c);
        for (name, m) in own {
            let Some((b, bt, p)) = bases.iter().find_map(|&(b, bt)| self.syms.class(b).type_aliases.get(&name).map(|&p| (b, bt, p))) else { continue };
            self.complete_alias(m);
            self.complete_alias(p);
            let (m_def, m_mods, m_file) = {
                let i = &self.syms.aliases[m.idx()];
                (i.def, i.def.map_or(0, |d| self.ast(i.file).def(d).mods), i.file)
            };
            let Some(m_def) = m_def else { continue };
            let seen = |t: &mut Self, ty: TypeId, subst: &Subst| {
                let ty = t.types.subst(ty, subst);
                t.as_seen_from(ty, this, c)
            };
            let psubst = self.owner_subst(bt);
            let over = |t: &mut Self, a: AliasId| {
                let i = &t.syms.aliases[a.idx()];
                let (rhs, bounds, tparams) = (i.rhs, i.bounds, i.tparams.clone());
                if tparams.is_empty() || rhs == ERROR {
                    return (rhs, bounds);
                }
                let ps: Vec<TypeId> = tparams.iter().map(|&p| t.types.param(p)).collect();
                let l = t.types.list(&ps);
                let lambda = t.types.mk(Type::Lambda(l, rhs));
                (t.eta_reduce(lambda), bounds)
            };
            let (p_rhs, p_bounds) = over(self, p);
            let (m_rhs, m_bounds) = over(self, m);
            // A member takes as many type arguments as the one it overrides.
            let arity = |t: &mut Self, a: AliasId, rhs: TypeId| {
                let n = t.syms.aliases[a.idx()].tparams.len();
                if n > 0 || rhs == ERROR { n } else { t.type_arity(rhs) }
            };
            let kinds_differ = arity(self, p, p_rhs) != arity(self, m, m_rhs);
            let (p_rhs, p_bounds) = (
                seen(self, p_rhs, &psubst),
                p_bounds.map(|(lo, hi)| (seen(self, lo, &psubst), seen(self, hi, &psubst))),
            );
            let (m_rhs, m_bounds) = (
                seen(self, m_rhs, &Vec::new()),
                m_bounds.map(|(lo, hi)| (seen(self, lo, &Vec::new()), seen(self, hi, &Vec::new()))),
            );
            if m_rhs == ERROR && m_bounds.is_none() || p_rhs == ERROR && p_bounds.is_none() {
                continue;
            }
            let mark = self.snapshot();
            let ok = !kinds_differ && match (p_bounds, m_bounds) {
                // scalac lets an alias take the place of an alias without a word unless
                // `override` asks for the check.
                (None, None) => m_mods & mods::OVERRIDE == 0 || self.is_same(m_rhs, p_rhs),
                (None, Some((lo, hi))) => self.is_sub(lo, p_rhs) && self.is_sub(p_rhs, hi) && self.is_same(lo, hi),
                (Some((lo, hi)), None) => self.is_sub(lo, m_rhs) && self.is_sub(m_rhs, hi),
                (Some((plo, phi)), Some((mlo, mhi))) => self.is_sub(plo, mlo) && self.is_sub(mhi, phi),
            };
            self.rollback(mark);
            if ok {
                continue;
            }
            let describe = |t: &mut Self, rhs: TypeId, bounds: Option<(TypeId, TypeId)>| match bounds {
                Some((lo, hi)) => {
                    let text = t.bounds_text(lo, hi);
                    if text.is_empty() { String::new() } else { format!(" with bounds{}", text) }
                }
                None => format!(", which equals {}", t.show(rhs)),
            };
            let p_text = describe(self, p_rhs, p_bounds);
            let m_text = describe(self, m_rhs, m_bounds);
            let msg = format!(
                "error overriding type {} in {}{};\n  type {}{} has incompatible type",
                self.name_str(name),
                self.class_description(b),
                p_text,
                self.name_str(name),
                m_text
            );
            let span = self.ast(m_file).def(m_def).span;
            self.diags.error(m_file, span, msg);
        }
    }

    /// ` >: L <: U`, over `[X]` for bounds that take parameters; empty for no bounds.
    pub(super) fn bounds_text(&mut self, lo: TypeId, hi: TypeId) -> String {
        let mut out = String::new();
        let params = match (self.types.get(lo), self.types.get(hi)) {
            (Type::Lambda(ps, _), _) | (_, Type::Lambda(ps, _)) => Some(ps),
            _ => None,
        };
        let body = |t: &mut Self, bound: TypeId| match t.types.get(bound) {
            Type::Lambda(_, b) => b,
            _ => bound,
        };
        if let Some(ps) = params {
            out.push('[');
            for (i, &p) in self.types.items(ps).to_vec().iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&self.show(p));
            }
            out.push(']');
        }
        if lo != NOTHING {
            let b = body(self, lo);
            out.push_str(&format!(" >: {}", self.show(b)));
        }
        if hi != ANY {
            let b = body(self, hi);
            out.push_str(&format!(" <: {}", self.show(b)));
        }
        out
    }

    /// The members `c` defines itself, extension methods after the others.
    fn own_member_count(&self, c: ClassId) -> usize {
        let info = self.syms.class(c);
        info.member_order.len() + info.extensions.len()
    }

    fn own_member(&self, c: ClassId, i: usize) -> SymId {
        let info = self.syms.class(c);
        match info.member_order.get(i) {
            Some(&m) => m,
            None => info.extensions[i - info.member_order.len()],
        }
    }

    fn is_ctor_param(&self, c: ClassId, m: SymId) -> bool {
        self.syms.class(c).ctor_syms.iter().any(|clause| clause.contains(&m))
    }

    /// `toString`, `hashCode` and `equals(that: Any)` with the shapes they have on `Any`.
    pub(super) fn matches_any_member(&mut self, m: SymId) -> bool {
        let name = self.syms.sym(m).name;
        let sig = self.sig_of(m);
        if !sig.tparams.is_empty() || sig.clauses.iter().any(|cl| cl.is_using) {
            return false;
        }
        let params: Vec<&ParamSig> = sig.clauses.iter().flat_map(|cl| cl.params.iter()).collect();
        match name {
            names::TO_STRING | names::HASH_CODE | names::CLONE => params.is_empty(),
            names::EQUALS => matches!(params.as_slice(), [p] if p.ty == ANY && !p.by_name && !p.repeated),
            _ => false,
        }
    }

    /// Two ancestors defining one name: a concrete member has to fit the abstract one it
    /// implements. Two concrete ones are the conflict that `set_parents` reports. A pair that a
    /// direct parent inherits as well was checked with that parent.
    #[cold]
    fn check_inherited_pairs(&mut self, c: ClassId) {
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types[1..].to_vec();
        let direct: Vec<ClassId> = self.syms.class(c).parents.clone().iter().filter_map(|&p| self.class_of(p)).collect();
        let extends = |t: &Self, sub: ClassId, sup: ClassId| t.syms.class(sub).base_types.iter().any(|&(x, _)| x == sup);
        let mut seen: crate::intern::FxMap<Name, Vec<(SymId, ClassId, TypeId)>> = Default::default();
        for &(b, bt) in &bases {
            let members: Vec<SymId> =
                self.syms.class(b).member_order.iter().chain(&self.syms.class(b).extensions).copied().collect();
            for m in members {
                if self.is_private(m) {
                    continue;
                }
                let name = self.syms.sym(m).name;
                let earlier_ones = seen.entry(name).or_default();
                let earlier_ones: Vec<(SymId, ClassId, TypeId)> = {
                    earlier_ones.push((m, b, bt));
                    earlier_ones[..earlier_ones.len() - 1].to_vec()
                };
                for (earlier, eb, ebt) in earlier_ones {
                    if eb == b
                        || extends(self, eb, b)
                        || direct.iter().any(|&p| extends(self, p, eb) && extends(self, p, b))
                        || (self.in_std(earlier) && self.in_std(m))
                    {
                        continue;
                    }
                    match (self.is_abstract_member(earlier), self.is_abstract_member(m)) {
                        (true, false) => {
                            self.check_override(c, m, Some(bt), earlier, ebt);
                            self.reach_through_accessor(m, earlier);
                        }
                        (false, true) => {
                            self.check_override(c, earlier, Some(ebt), m, bt);
                            self.reach_through_accessor(earlier, m);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Checks `m` against `p`, a member of an ancestor that it overrides or implements, both seen
    /// from `c`: `m_owner` is the ancestor `m` comes from, or `None` for a member of `c` itself.
    /// Returns whether `m` counts as overriding `p`; different parameters make the two unrelated,
    /// which is an overload scalac would accept and teq rejects.
    fn check_override(&mut self, c: ClassId, m: SymId, m_owner: Option<TypeId>, p: SymId, p_owner: TypeId) -> bool {
        let (m_kind, m_mods, m_lazy) = {
            let s = self.syms.sym(m);
            (s.kind, s.mods, s.mods & mods::LAZY != 0)
        };
        let (p_kind, p_mods, p_lazy, mut pb) = {
            let s = self.syms.sym(p);
            let Owner::Class(pb) = s.owner else { return false };
            (s.kind, s.mods, s.mods & mods::LAZY != 0, pb)
        };
        // A deferred given is checked as a concrete member but for its finality (dotty's
        // `RefChecks`, 533-546).
        let p_deferred = self.is_deferred_given(p);
        // One without parameters is a lazy value, overridden as one is (`RefChecks` 1.6, 1.7).
        let p_lazy_given = p_deferred && p_lazy && p_kind == SymKind::Given;
        let p_abstract = self.is_abstract_member(p);
        let own = m_owner.is_none();
        if own && !p_abstract && matches!(p_kind, SymKind::Val | SymKind::Var) && self.is_ctor_param(c, m) && !self.syms.sym(p).overridden_by_param {
            self.shared_write(p, |w| w.syms.sym_mut(p).overridden_by_param = true);
        }
        let msig = self.sig_arc(m);
        let agreement = self.compare_sigs(c, msig, m_owner, p, p_owner);
        // A val next to the alternatives of an overloaded name other than the one it implements.
        let overloads = p_kind == SymKind::Def && (m_kind == SymKind::Def || self.syms.sym(p).alternative);
        if agreement == Agreement::Params && (p_abstract || overloads) {
            return false;
        }
        // A member with `override` over a var or its setter, which scalac names by the var
        // (`RefChecks.checkOverride`, `other.isMutableVarOrAccessor`).
        let overrides_var = own && m_mods & mods::OVERRIDE != 0 && (p_kind == SymKind::Var || (p_kind == SymKind::Def && p_mods & mods::SETTER != 0));
        let overridden_var = match p_kind {
            SymKind::Var if overrides_var => Some(p),
            SymKind::Def if overrides_var => super::setters::var_of_setter(&self.syms, self.interner, p),
            _ => None,
        };
        let shown_p = overridden_var.unwrap_or(p);
        // A concrete given is final, an alias's and a given class's def alike (dotty's
        // `Parsers.givenDef` 4583, `Desugar` 1093); a `deferred` one is abstract, and the
        // implementation a class above `c` was given for it final.
        let mut p_final = p_mods & mods::FINAL != 0 || (!p_abstract && (p_kind == SymKind::Given || p_mods & mods::GIVEN != 0));
        if let Some(k) = if p_deferred { self.deferred_implemented_above(c, p) } else { None } {
            (p_final, pb) = (true, k);
        }
        let problem = if p_final {
            "{m} cannot override final member {p}"
        } else if overridden_var.is_some() {
            "error overriding {p} of type {pt};\n  {m} of type {mt} cannot override a mutable variable"
        } else if agreement == Agreement::Params {
            "{m} of type {mt} does not match {p} of type {pt}; only methods can be overloaded"
        } else if p_kind == SymKind::Var && !p_abstract {
            "{m} cannot override mutable {p}"
        } else if (p_kind == SymKind::Val || p_lazy_given) && m_kind == SymKind::Def {
            "{m} needs to be a stable, immutable value to override {p}"
        } else if (p_kind == SymKind::Val || p_lazy_given) && (!p_abstract || p_deferred) && p_lazy && !m_lazy {
            "{m} must be declared lazy to override {p}"
        } else if p_kind == SymKind::Val && !p_abstract && !p_lazy && m_lazy {
            "{m} may not override non-lazy {p}"
        } else if agreement == Agreement::Result {
            "error overriding {p} of type {pt}: {m} of type {mt} has incompatible type"
        } else if own && self.is_private(m) {
            "private {m} cannot override {p}"
        } else if own && m_mods & mods::PROTECTED != 0 && p_mods & (mods::PROTECTED | mods::PRIVATE) == 0 {
            "{m} has weaker access privileges than {p}; it should be public"
        } else if own && m_mods & mods::OVERRIDE == 0 && (!p_abstract || p_deferred) && !self.is_product_member(p) {
            "{m} needs `override` modifier to override {p}"
        } else {
            return !own || self.check_target_names(m, p);
        };
        let m_class = m_owner.and_then(|t| self.class_of(t));
        let msg = problem
            .replace("{m}", &self.member_description(m, m_class))
            .replace("{p}", &self.member_description(shown_p, Some(pb)))
            .replace("{mt}", &self.sig_string(m))
            .replace("{pt}", &self.sig_string(shown_p));
        if own {
            self.error_at(m, msg);
        } else {
            let (file, span) = {
                let i = self.syms.class(c);
                (i.file, i.span)
            };
            self.diags.error(file, span, msg);
        }
        true
    }

    /// scalac's rule on `@targetName` across an override: the same annotation on both, or none.
    /// Two different annotations make `m` no implementation of `p`, which is what is returned.
    fn check_target_names(&mut self, m: SymId, p: SymId) -> bool {
        let (mine, theirs) = (self.declared_target_name(m), self.declared_target_name(p));
        let (problem, implements) = match (mine, theirs) {
            (Some(_), None) => ("should not have a @targetName annotation since the overridden member hasn't one either".to_string(), true),
            (Some(a), Some(b)) if a != b => (format!("has a different target name annotation; it should be @targetName({})", self.name_str(b)), false),
            (None, Some(b)) => (format!("misses a target name annotation @targetName({})", self.name_str(b)), true),
            _ => return true,
        };
        let Owner::Class(pb) = self.syms.sym(p).owner else { return true };
        let msg = format!(
            "error overriding {} of type {};\n  {} of type {} {}",
            self.member_description(p, Some(pb)),
            self.sig_string(p),
            self.member_description(m, None),
            self.sig_string(m),
            problem
        );
        self.error_at(m, msg);
        implements
    }

    fn declared_target_name(&mut self, m: SymId) -> Option<Name> {
        let info = self.syms.sym(m);
        let ast = self.ast(info.file);
        let annot = ast.def(info.def?).annots.iter().find(|a| self.interner.get(a.name) == "targetName")?;
        let text = ast.str(*ast.annot_args(annot).first()?).to_string();
        Some(self.interner.intern(&text))
    }

    /// scalac's E120 for an inherited member: a method whose `@targetName` is the name of an
    /// inherited method of the same erased signature.
    fn check_target_name_clashes(&mut self, c: ClassId, m: SymId) {
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types[1..].to_vec();
        let own_name = self.syms.sym(m).name;
        let mut candidates: Vec<(ClassId, SymId, Name)> = Vec::new();
        if let Some(target) = self.declared_target_name(m) {
            for &(b, _) in &bases {
                if let Some(&q) = self.syms.class(b).members.get(&target) {
                    candidates.push((b, q, target));
                }
            }
        }
        for &(b, _) in &bases {
            let alternatives: Vec<SymId> = self.syms.class(b).member_order.iter().copied().filter(|&s| self.syms.sym(s).alternative).collect();
            for q in alternatives {
                if self.declared_target_name(q) == Some(own_name) {
                    candidates.push((b, q, own_name));
                }
            }
        }
        for (b, q, target) in candidates {
            if self.syms.sym(q).kind != SymKind::Def || self.is_private(q) {
                continue;
            }
            // scalac reads a private member's target name as an override of the inherited one.
            if self.is_private(m) {
                let msg = format!("private {} cannot override {}", self.member_description(m, None), self.member_description(q, Some(b)));
                self.error_at(m, msg);
                return;
            }
            if self.erased_signature(m, true) != self.erased_signature(q, true) || self.erased_result(m, true) != self.erased_result(q, true) {
                continue;
            }
            let line = |t: &Self, s: SymId| {
                let info = t.syms.sym(s);
                crate::source::locate(&t.files[info.file.0 as usize].text, info.span.start as usize).0
            };
            let shown = |t: &mut Self, s: SymId| {
                let sig = t.sig_string(s);
                if sig.starts_with('(') || sig.starts_with('[') { sig } else { format!(": {}", sig) }
            };
            let (q_sig, m_sig) = (shown(self, q), shown(self, m));
            let msg = format!(
                "Name clash between defined and inherited member:\ndef {}{} in {} at line {} and\ndef {}{} in {} at line {}\nhave the same name and type {} after erasure.",
                self.name_str(target),
                q_sig,
                self.class_description(b),
                line(self, q),
                self.name_str(self.syms.sym(m).name),
                m_sig,
                self.class_description(c),
                line(self, m),
                q_sig.trim_start_matches(": "),
            );
            let (file, span) = (self.syms.class(c).file, self.syms.class(c).span);
            self.diags.error(file, span, msg);
            return;
        }
    }

    /// scalac's check along the linearisation: what an earlier ancestor defines cannot override
    /// a final member of a later one.
    fn check_final_overrides(&mut self, c: ClassId) {
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types.clone();
        for j in 2..bases.len() {
            let (b, bt) = bases[j];
            let finals: Vec<SymId> = self
                .syms
                .class(b)
                .member_order
                .iter()
                .copied()
                .filter(|&s| {
                    let i = self.syms.sym(s);
                    i.mods & mods::FINAL != 0 && i.kind == SymKind::Def && i.mods & mods::PRIVATE == 0
                })
                .collect();
            for p in finals {
                let name = self.syms.sym(p).name;
                for i in 1..j {
                    let (a, at) = bases[i];
                    let Some(&m) = self.syms.class(a).members.get(&name) else { continue };
                    if self.syms.sym(m).owner != Owner::Class(a) || self.syms.alternatives(m).is_some() || self.syms.sym(m).kind != SymKind::Def {
                        continue;
                    }
                    let msig = self.sig_arc(m);
                    if matches!(self.compare_sigs(c, msig, Some(at), p, bt), Agreement::Same | Agreement::Result) {
                        let msg = format!(
                            "error overriding {} of type {};\n  {} of type {} cannot override final member {}",
                            self.member_description(p, Some(b)),
                            self.sig_string(p),
                            self.member_description(m, Some(a)),
                            self.sig_string(m),
                            self.member_description(p, Some(b)),
                        );
                        let (file, span) = (self.syms.class(c).file, self.syms.class(c).span);
                        self.diags.error(file, span, msg);
                        return;
                    }
                }
            }
        }
    }

    fn error_at(&mut self, sym: SymId, msg: String) {
        let (file, span) = {
            let s = self.syms.sym(sym);
            (s.file, s.span)
        };
        self.diags.error(file, span, msg);
    }

    /// How a member with the signature `msig` relates to `p`, both in terms of the class `c` that
    /// inherits them: `m_owner` and `p_owner` are the ancestor types each comes from, `None` for
    /// `c` itself. The type parameters of `p` are read as those of the member. A member of an
    /// ancestor whose result is that ancestor's own type is read as returning `this.type`.
    pub(super) fn compare_sigs(&mut self, c: ClassId, mut msig: Arc<MethodSig>, m_owner: Option<TypeId>, p: SymId, p_owner: TypeId) -> Agreement {
        let mut psig = self.sig_arc(p);
        // A Java method's `()` matches a parameterless member or a val, and a parameterless
        // one (the std's `Iterator.hasNext`) a `()`, as scalac's `matchNullaryLoosely` has it for
        // Java-defined members; the std's classes of `java.*` stand for the JDK's.
        let empty = |sig: &MethodSig| matches!(&sig.clauses[..], [cl] if cl.params.is_empty() && !cl.is_using);
        let java_member = self.syms.sym(p).java_defined || (self.is_std_member(p) && !self.is_std_scala_member(p));
        if msig.clauses.is_empty() && empty(&psig) && java_member {
            psig = Arc::new(MethodSig { tparams: psig.tparams.clone(), clauses: Vec::new(), ret: psig.ret });
        } else if psig.clauses.is_empty() && empty(&msig) && java_member && self.syms.sym(p).kind == SymKind::Def {
            msig = Arc::new(MethodSig { tparams: msig.tparams.clone(), clauses: Vec::new(), ret: msig.ret });
        }
        let lenient_using = self.is_std_scala_member(p);
        self.compare_sig_pair(c, msig, m_owner, psig, p_owner, false, lenient_using)
    }

    /// A member of the lean std, whose overrides may add a using clause the ancestor lacks
    /// (`SortedSet.map` asks for an `Ordering`; a call through the ancestor passes no evidence,
    /// which the body allows for).
    pub(super) fn is_std_member(&self, s: SymId) -> bool {
        self.std.slot_of(self.syms.sym(s).file).is_some()
    }

    /// A member of a Scala class of the lean std: a JDK class the std models has no using
    /// clause to leave out, so a trailing one makes an overload of its method.
    pub(super) fn is_std_scala_member(&self, s: SymId) -> bool {
        if !self.is_std_member(s) {
            return false;
        }
        let Owner::Class(c) = self.syms.sym(s).owner else { return true };
        let mut at = self.syms.class(c).owner;
        while let Owner::Class(o) = at {
            at = self.syms.class(o).owner;
        }
        let Owner::Package(mut p) = at else { return true };
        while let Some(parent) = self.syms.pkg(p).parent.filter(|&q| q != ROOT_PKG) {
            p = parent;
        }
        !matches!(self.interner.get(self.syms.pkg(p).name), "java" | "javax")
    }

    /// `params_only` leaves the results out, which may still be unknown; `lenient_using` leaves
    /// the using clauses out where their counts differ.
    pub(super) fn compare_sig_pair(
        &mut self,
        c: ClassId,
        msig: Arc<MethodSig>,
        m_owner: Option<TypeId>,
        psig: Arc<MethodSig>,
        p_owner: TypeId,
        params_only: bool,
        lenient_using: bool,
    ) -> Agreement {
        if msig.tparams.len() != psig.tparams.len() {
            return Agreement::Params;
        }
        let msubst: Subst = m_owner.map(|t| self.owner_subst(t)).unwrap_or_default();
        let mut psubst = self.owner_subst(p_owner);
        for (&pt, &mt) in psig.tparams.iter().zip(&msig.tparams) {
            let as_m = self.types.param(mt);
            psubst.push((pt, as_m));
        }
        // Both are seen from `c`, whose `this` fixes the type members they mention. A member of
        // `c` itself is already written from `c`: an ancestor's `C.this` in it is an enclosing
        // instance (`type State = (Schedule.this.State, that.State)` in an anonymous `Schedule`).
        let this = self.this_prefix(c);
        let seen = |t: &mut Self, ty: TypeId| if t.types.has_paths(ty) { t.as_seen_from(ty, this, c) } else { ty };
        let msig = if m_owner.is_some() { self.sig_seen(msig, &seen) } else { msig };
        let psig = self.sig_seen(psig, &seen);
        // The bounds of the type parameters have to match exactly, as scalac asks.
        for (&pt, &mt) in psig.tparams.iter().zip(&msig.tparams) {
            let (pu, pl) = (self.syms.tparam(pt).upper, self.syms.tparam(pt).lower);
            let (mu, ml) = (self.syms.tparam(mt).upper, self.syms.tparam(mt).lower);
            let (pu, pl) = (self.types.subst(pu, &psubst), self.types.subst(pl, &psubst));
            let (mu, ml) = (self.types.subst(mu, &msubst), self.types.subst(ml, &msubst));
            if (pu != mu && !self.is_same(pu, mu)) || (pl != ml && !self.is_same(pl, ml)) {
                return Agreement::Params;
            }
        }
        let parameterless = |sig: &MethodSig| sig.clauses.iter().all(|cl| !cl.is_using && cl.params.is_empty());
        // The using clauses count as scalac counts them: `compose[G](using Applicative[G])`
        // and `compose[G]` are two members.
        let using_count = |sig: &MethodSig| sig.clauses.iter().filter(|cl| cl.is_using).count();
        let explicit_only = lenient_using && using_count(&msig) != using_count(&psig);
        let mclauses: Vec<&ClauseSig> = msig.clauses.iter().filter(|cl| !explicit_only || !cl.is_using).collect();
        let pclauses: Vec<&ClauseSig> = psig.clauses.iter().filter(|cl| !explicit_only || !cl.is_using).collect();
        let same_shape = mclauses.len() == pclauses.len()
            && mclauses.iter().zip(&pclauses).all(|(a, b)| {
                a.is_using == b.is_using
                    && a.params.len() == b.params.len()
                    && a.params.iter().zip(&b.params).all(|(x, y)| x.by_name == y.by_name && x.repeated == y.repeated)
            });
        if !same_shape {
            // `def f` against `def f()`: the same member with incompatible types, as under scalac.
            return if parameterless(&msig) && parameterless(&psig) { Agreement::Result } else { Agreement::Params };
        }
        // The parameters a later one's type names are the other method's by position, as
        // scalac matches `f(d: Ctx)(x: d.T)` with `f(c: Ctx)(x: c.T)`.
        let renamed: Vec<(SymId, TypeId)> = mclauses
            .iter()
            .zip(&pclauses)
            .flat_map(|(a, b)| a.params.iter().zip(&b.params).map(|(x, y)| (x.sym, y.sym)).collect::<Vec<_>>())
            .filter(|&(x, y)| x != y)
            .map(|(x, y)| (x, self.types.mk(Type::Term(y))))
            .collect();
        for (a, b) in mclauses.iter().zip(&pclauses) {
            for (x, y) in a.params.iter().zip(&b.params) {
                let xt = self.subst_paths(x.ty, &renamed);
                let xt = self.types.subst(xt, &msubst);
                let yt = self.types.subst(y.ty, &psubst);
                if xt == ERROR || yt == ERROR {
                    continue;
                }
                if !self.is_same(xt, yt) && !self.quoted_param_match(xt, yt) {
                    return Agreement::Params;
                }
            }
        }
        if params_only {
            return Agreement::Same;
        }
        let self_of = |t: &Self, k: ClassId| t.syms.class(k).base_types[0].1;
        let returns_this = m_owner.and_then(|t| self.class_of(t)).map_or(false, |k| msig.ret == self_of(self, k));
        let mret = if returns_this {
            self_of(self, c)
        } else {
            let ret = self.subst_paths(msig.ret, &renamed);
            self.types.subst(ret, &msubst)
        };
        let pret = self.types.subst(psig.ret, &psubst);
        if mret == ERROR || pret == ERROR || self.is_sub(mret, pret) {
            Agreement::Same
        } else {
            Agreement::Result
        }
    }

    fn sig_seen(&mut self, sig: Arc<MethodSig>, seen: &dyn Fn(&mut Self, TypeId) -> TypeId) -> Arc<MethodSig> {
        let depends = self.types.has_paths(sig.ret) || sig.clauses.iter().any(|cl| cl.params.iter().any(|p| self.types.has_paths(p.ty)));
        if !depends {
            return sig;
        }
        let ret = seen(self, sig.ret);
        let clauses: Vec<ClauseSig> = sig
            .clauses
            .iter()
            .map(|cl| ClauseSig {
                params: cl.params.iter().map(|p| ParamSig { ty: seen(self, p.ty), ..p.clone() }).collect(),
                is_using: cl.is_using,
                is_implicit: cl.is_implicit,
            })
            .collect();
        Arc::new(MethodSig { tparams: sig.tparams.clone(), clauses, ret })
    }

    /// "method f", "value x in trait T".
    pub(super) fn member_description(&self, m: SymId, in_class: Option<ClassId>) -> String {
        let info = self.syms.sym(m);
        let kind = match info.kind {
            SymKind::Def | SymKind::Overloaded(_) => "method",
            SymKind::Val if info.mods & mods::LAZY != 0 => "lazy value",
            SymKind::Val | SymKind::Param => "value",
            SymKind::Var => "variable",
            SymKind::Object(_) => "object",
            SymKind::EnumValue(_) => "case",
            SymKind::Given => "given",
        };
        match in_class {
            Some(c) => format!("{} {} in {}", kind, self.name_str(info.name), self.class_description(c)),
            None => format!("{} {}", kind, self.name_str(info.name)),
        }
    }

    /// `(x: Int)(using s: Show[Int]): String`, or the type of a value.
    pub(super) fn sig_string(&mut self, m: SymId) -> String {
        self.sig_string_in(m, &Vec::new())
    }

    /// The same with the type parameters of the member's class replaced as `subst` says.
    fn sig_string_in(&mut self, m: SymId, subst: &Subst) -> String {
        let sig = self.sig_arc(m);
        self.sig_text_in(&sig, subst)
    }

    pub(super) fn sig_text(&mut self, sig: &MethodSig) -> String {
        self.sig_text_in(sig, &Vec::new())
    }

    pub(super) fn sig_text_in(&mut self, sig: &MethodSig, subst: &Subst) -> String {
        let mut out = String::new();
        if !sig.tparams.is_empty() {
            out.push('[');
            for (i, &tp) in sig.tparams.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&self.name_str(self.syms.tparam(tp).name));
                let (upper, lower) = (self.syms.tparam(tp).upper, self.syms.tparam(tp).lower);
                if lower != NOTHING {
                    let t = self.types.subst(lower, subst);
                    let _ = std::fmt::Write::write_fmt(&mut out, format_args!(" >: {}", self.show(t)));
                }
                if upper != ANY {
                    let t = self.types.subst(upper, subst);
                    let _ = std::fmt::Write::write_fmt(&mut out, format_args!(" <: {}", self.show(t)));
                }
            }
            out.push(']');
        }
        for cl in &sig.clauses {
            out.push('(');
            if cl.is_using {
                out.push_str("using ");
            }
            for (i, p) in cl.params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                let ty = self.types.subst(p.ty, subst);
                let ty = self.show(ty);
                let _ = std::fmt::Write::write_fmt(&mut out, format_args!("{}: {}{}{}", self.name_str(p.name), if p.by_name { "=> " } else { "" }, ty, if p.repeated { "*" } else { "" }));
            }
            out.push(')');
        }
        if !out.is_empty() {
            out.push_str(": ");
        }
        let ret = self.types.subst(sig.ret, subst);
        out.push_str(&self.show(ret));
        out
    }

    /// The outer accessors of the traits nested in a class that a class mixes in, checked
    /// implemented once every body is typed, of every class checked (of the files given, after a
    /// retype): a trait's accessor is declared by the first body that reads its enclosing
    /// instance, which another worker may type after the class's own check (the outer accessors).
    pub(super) fn check_outer_accessors_implemented(&mut self, files: Option<&[FileId]>) {
        if self.outer_accessors.is_empty() {
            return;
        }
        let classes: Vec<ClassId> = self.prog.classes.iter().map(|tc| tc.id).collect();
        for c in classes {
            let (kind, abstract_class, file, def) = {
                let i = self.syms.class(c);
                (i.kind, i.mods & mods::ABSTRACT != 0, i.file, i.def)
            };
            if def.is_none() || matches!(kind, ClassKind::Trait | ClassKind::Enum) || abstract_class || self.is_body_file(file) || files.is_some_and(|fs| !fs.contains(&file)) {
                continue;
            }
            let nested = self.syms.class(c).base_types.iter().skip(1).any(|&(b, _)| self.outer_accessors.get(&b).is_some());
            if nested {
                self.check_implements_all(c, Abstracts::OuterAccessors);
            }
        }
    }

    /// Returns the abstract members that an export clause of `c` implements, with their targets.
    fn check_implements_all(&mut self, c: ClassId, which: Abstracts) -> Vec<(SymId, SymId)> {
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types.clone();
        let is_case = self.syms.class(c).mods & mods::CASE != 0;
        let mut missing: Vec<(String, SymId, TypeId)> = Vec::new();
        // A member left unimplemented beside an incomplete definition of its name, whose body
        // is unknown rather than missing.
        let mut unknown = false;
        let mut forwarders = Vec::new();
        // A concrete class's own members without a body too (`class C { given x: Int }`), as
        // scalac's "needs to be abstract"; an object's are E067 (`misplaced_abstract_given`).
        let own = (which == Abstracts::Declared && self.syms.class(c).kind == ClassKind::Class) as usize;
        for &(b, bt) in bases.iter().skip(1 - own) {
            let members = self.syms.class(b).member_order.clone();
            for m in members.into_iter().chain(self.syms.class(b).extensions.clone()) {
                // A deferred given is implemented by the search, or reported there.
                if !self.is_abstract_member(m) || self.is_deferred_given(m) {
                    continue;
                }
                // An own `@native` method has its body elsewhere.
                if b == c && self.annotated_native(m) {
                    continue;
                }
                if (self.outer_accessors.get(&b) == Some(&m)) != (which == Abstracts::OuterAccessors) {
                    continue;
                }
                let name = self.syms.sym(m).name;
                let is_ext = self.syms.sym(m).is_extension;
                if is_case && matches!(name, names::TO_STRING | names::HASH_CODE | names::EQUALS) {
                    continue;
                }
                // The class's own member is the most derived: whatever of its name it inherits it
                // overrides, which implements nothing of it (`class C extends T { override def x:
                // Int }` over a concrete `T.x`).
                let implemented = bases.iter().enumerate().filter(|&(i, _)| b != c || i == 0).any(|(i, &(other, ot))| {
                    let mut k = 0;
                    while let Some(s) = self.inherited_member(other, name, is_ext, k) {
                        k += 1;
                        // A deferred given is implemented by the search, or reported there; one
                        // that `m` overrides, an ancestor's, implements nothing of `m`.
                        // A concrete var's setter is the var's, which `var_implements_setter` reads.
                        let implements = s != m
                            && !super::setters::is_concrete_setter(&self.syms, s)
                            && (!self.is_abstract_member(s) || (self.is_deferred_given(s) && !self.overrides_declaration_of(m, s)))
                            && !self.is_private(s)
                            && !self.is_abstract_override(s)
                            && ((self.in_std(s) && self.in_std(m)) || {
                                let ssig = self.sig_arc(s);
                                self.compare_sigs(c, ssig, (i > 0).then_some(ot), m, bt) != Agreement::Params
                                    && self.declared_target_name(s) == self.declared_target_name(m)
                            });
                        if implements {
                            return true;
                        }
                    }
                    false
                });
                if implemented || self.var_implements_setter(c, &bases, m, bt) {
                    continue;
                }
                if self.recovered && !unknown {
                    unknown = bases.iter().any(|&(other, _)| {
                        let mut k = 0;
                        while let Some(s) = self.inherited_member(other, name, is_ext, k) {
                            k += 1;
                            if s != m && self.syms.sym(s).mods & mods::INCOMPLETE != 0 {
                                return true;
                            }
                        }
                        false
                    });
                }
                let exported = (!is_ext && self.syms.class(c).has_exports)
                    .then(|| self.exports_of(c))
                    .flatten()
                    .and_then(|e| e.terms.get(&name).copied())
                    .and_then(|r| r.sym())
                    .filter(|&s| s != m && !self.is_abstract_member(s))
                    .filter(|&s| {
                        let ssig = self.sig_arc(s);
                        self.compare_sigs(c, ssig, None, m, bt) != Agreement::Params
                    });
                match exported {
                    Some(target) => forwarders.push((m, target)),
                    None => missing.push((self.name_str(name).to_string(), m, bt)),
                }
            }
        }
        if !missing.is_empty() && self.link_mode() {
            missing.retain(|&(_, m, bt)| !self.inherited_export_implements(c, &bases, m, bt));
        }
        if !missing.is_empty() {
            // An abstract var left undefined is one member, its getter and setter together.
            let vars: Vec<SymId> = missing.iter().map(|&(_, m, _)| m).filter(|&m| self.syms.sym(m).kind == SymKind::Var).collect();
            missing.retain(|&(_, m, _)| self.syms.sym(m).mods & mods::SETTER == 0 || !super::setters::var_of_setter(&self.syms, self.interner, m).is_some_and(|v| vars.contains(&v)));
            missing.sort_by(|a, b| a.0.cmp(&b.0));
            missing.dedup_by(|a, b| a.0 == b.0);
            self.report_unimplemented(c, &missing, unknown);
        }
        forwarders
    }

    /// Whether an export of a class above `c` implements the abstract member `m`, which nothing else
    /// implements, `c` inheriting its forwarder as scalac writes it (`class Child extends Car`): on the
    /// JVM alone, whose class files carry the forwarders, JavaScript having none to inherit; and only
    /// where its result conforms, one that does not being no implementation, as scalac's "has
    /// incompatible type" has it.
    #[cold]
    #[inline(never)]
    fn inherited_export_implements(&mut self, c: ClassId, bases: &[(ClassId, TypeId)], m: SymId, bt: TypeId) -> bool {
        let (name, is_ext) = {
            let info = self.syms.sym(m);
            (info.name, info.is_extension)
        };
        !is_ext && bases.iter().skip(1).any(|&(other, _)| {
            self.syms.class(other).has_exports
                && self
                    .exports_of(other)
                    .and_then(|e| e.terms.get(&name).copied())
                    .and_then(|r| r.sym())
                    .filter(|&s| s != m && !self.is_abstract_member(s))
                    .is_some_and(|s| {
                        let ssig = self.sig_arc(s);
                        self.compare_sigs(c, ssig, None, m, bt) == Agreement::Same
                    })
        })
    }

    /// Whether a var that `c` has implements the abstract setter `m` (`def x_=(v: T): Unit`):
    /// `var x: T` is the getter and the setter. The var, `c`'s own or inherited from an ancestor
    /// that knows nothing of `m`, is then reached through its accessors, and recorded as
    /// overriding `m`, whose name its setter takes.
    fn var_implements_setter(&mut self, c: ClassId, bases: &[(ClassId, TypeId)], m: SymId, m_owner: TypeId) -> bool {
        let (kind, name) = {
            let info = self.syms.sym(m);
            (info.kind, info.name)
        };
        if kind != SymKind::Def {
            return false;
        }
        let Some(var_name) = self.interner.get(name).strip_suffix("_=").and_then(|n| self.interner.lookup(n)) else { return false };
        let psig = self.sig_arc(m);
        let ([clause], true) = (psig.clauses.as_slice(), psig.tparams.is_empty()) else { return false };
        let [param] = clause.params.as_slice() else { return false };
        let param_sym = param.sym;
        for (i, &(other, ot)) in bases.iter().enumerate() {
            let mut k = 0;
            while let Some(s) = self.inherited_member(other, var_name, false, k) {
                k += 1;
                if self.syms.sym(s).kind != SymKind::Var || self.is_abstract_member(s) || self.is_private(s) {
                    continue;
                }
                let ty = self.sig_of(s).ret;
                let unit = self.b.t_unit;
                let params = vec![ParamSig { name: var_name, ty, by_name: false, repeated: false, has_default: false, sym: param_sym }];
                let as_setter = Arc::new(MethodSig { tparams: Vec::new(), clauses: vec![ClauseSig { params, is_using: false, is_implicit: false }], ret: unit });
                if self.compare_sig_pair(c, as_setter, (i > 0).then_some(ot), psig.clone(), m_owner, false, false) == Agreement::Same {
                    if !self.syms.js_member(s) {
                        self.shared_write(s, |w| {
                            let mut info = w.syms.sym_mut(s);
                            info.needs_accessor = true;
                            info.needs_setter = true;
                            w.push_override(s, m);
                        });
                    }
                    return true;
                }
            }
        }
        false
    }

    #[cold]
    fn report_unimplemented(&mut self, c: ClassId, missing: &[(String, SymId, TypeId)], unknown: bool) {
        let (span, file, kind) = {
            let i = self.syms.class(c);
            (i.span, i.file, i.kind)
        };
        let problem = match kind {
            ClassKind::Class | ClassKind::EnumCase => format!("{} needs to be abstract", self.class_description(c)),
            _ => "object creation impossible".to_string(),
        };
        let msg = match missing {
            [(_, m, owner_ty)] => {
                let subst = self.owner_subst(*owner_ty);
                let sig = self.sig_string_in(*m, &subst);
                let owner = self.class_of(*owner_ty).map_or(String::new(), |k| self.class_description(k));
                let info = self.syms.sym(*m);
                let keyword = match info.kind {
                    SymKind::Def if info.mods & mods::SETTER != 0 => "var",
                    SymKind::Def if info.mods & mods::GIVEN != 0 => "given def",
                    SymKind::Def => "def",
                    SymKind::Var => "var",
                    SymKind::Given => "given",
                    _ => "val",
                };
                // scalac shows the declaration's `override` (`override given def x: Int`).
                let keyword = if info.mods & mods::OVERRIDE != 0 { format!("override {}", keyword) } else { keyword.to_string() };
                let sep = if sig.starts_with('(') || sig.starts_with('[') { "" } else { ": " };
                let name = self.name_str(info.name);
                format!("{}, since {} {}{}{} in {} is not defined", problem, keyword, name, sep, sig, owner)
            }
            _ => {
                let names: Vec<&str> = missing.iter().map(|(n, ..)| n.as_str()).collect();
                format!("{}, since it has {} unimplemented members: {}", problem, missing.len(), names.join(", "))
            }
        };
        // A class whose header, or an ancestor's, the parser could not complete may have had
        // what implements the members, and so may an incomplete definition of a member's name.
        // A member whose signature holds the error type is about what malformed syntax left
        // unknown (`Diagnostic::unknown`).
        let self_ty = self.syms.class(c).base_types[0].1;
        if unknown || self.incomplete_ancestry(self_ty) {
            self.diags.dependent_error(file, span, msg);
        } else {
            let erroneous = self.recovered && missing.iter().any(|&(_, m, _)| self.sig_holds_error(m));
            self.diags.error(file, span, msg);
            if erroneous {
                if let Some(d) = self.diags.items.last_mut() {
                    d.unknown = true;
                }
            }
        }
    }

    /// Whether a member's signature holds the error type, in a parameter or its result.
    fn sig_holds_error(&mut self, m: SymId) -> bool {
        let sig = self.sig_arc(m);
        self.types.contains_error(sig.ret) || sig.clauses.iter().any(|c| c.params.iter().any(|p| self.types.contains_error(p.ty)))
    }

    /// Whether `c` or a class enclosing it declares a variant type parameter, which is what the
    /// variance check looks for; most classes have none and skip it.
    fn in_variant_scope(&self, c: ClassId) -> bool {
        let mut k = c;
        loop {
            let info = self.syms.class(k);
            if info.tparams.iter().any(|&p| self.syms.tparam(p).variance != 0) {
                return true;
            }
            match info.owner {
                Owner::Class(outer) => k = outer,
                _ => return false,
            }
        }
    }

    /// SLS 4.5: a covariant type parameter may occur only in covariant positions, a
    /// contravariant one only in contravariant positions. Results, vals and parents are
    /// covariant positions; method parameters and the upper bounds of method type parameters
    /// flip the position, a `var`, an alias and the argument of an invariant parameter make it
    /// invariant. Private members and plain constructor parameters are construction-only and
    /// not checked, as under scalac. The standard library keeps `Array[A]` in its covariant
    /// collections and is not checked.
    fn check_variances(&mut self, c: ClassId) {
        let (file, span) = {
            let i = self.syms.class(c);
            (i.file, i.span)
        };
        if self.source(file).is_std || !self.in_variant_scope(c) {
            return;
        }
        let class_desc = self.class_description(c);
        let class_def = self.syms.class(c).def;
        self.variance_skip.clear();
        if let (Some(d), false) = (class_def, self.unchecked_variance.is_empty()) {
            if let DefKind::Class(cls) = &self.ast(file).def(d).kind {
                let parents: Vec<TyExprId> = cls.parents.iter().map(|p| p.ty).collect();
                self.collect_unchecked_variance(file, &parents);
            }
        }
        for i in 0..self.syms.class(c).parents.len() {
            let parent = self.syms.class(c).parents[i];
            self.report_variance(parent, 1, file, span, &class_desc);
        }
        for i in 0..self.own_member_count(c) {
            let m = self.own_member(c, i);
            let (kind, m_mods, has_def, m_file, m_span) = {
                let s = self.syms.sym(m);
                (s.kind, s.mods, s.def.is_some(), s.file, s.span)
            };
            let is_field_param = m_mods & mods::FIELD != 0 && self.is_ctor_param(c, m);
            if m_mods & mods::PRIVATE != 0 || !(has_def || is_field_param) {
                continue;
            }
            self.variance_skip.clear();
            if !self.unchecked_variance.is_empty() {
                let tys = self.member_type_exprs(c, m);
                self.collect_unchecked_variance(m_file, &tys);
            }
            let sign = match kind {
                SymKind::Var => 0,
                SymKind::Val | SymKind::Def | SymKind::Given => 1,
                _ => continue,
            };
            self.sig_of(m);
            let sig = self.syms.sig(m);
            let mut positions: Vec<(TypeId, i8, crate::source::Span, String)> = Vec::new();
            for &tp in &sig.tparams {
                let (upper, lower) = (self.syms.tparam(tp).upper, self.syms.tparam(tp).lower);
                let what = format!("type parameter {}", self.name_str(self.syms.tparam(tp).name));
                positions.push((upper, -1, m_span, what.clone()));
                positions.push((lower, 1, m_span, what));
            }
            for p in sig.clauses.iter().flat_map(|cl| cl.params.iter()) {
                let what = format!("parameter {}", self.name_str(p.name));
                positions.push((p.ty, -1, self.syms.sym(p.sym).span, what));
            }
            positions.push((sig.ret, sign, m_span, self.member_description(m, None)));
            for (t, sign, span, what) in positions {
                if self.report_variance(t, sign, m_file, span, &what) {
                    break;
                }
            }
        }
        let mut aliases: Vec<AliasId> = self.syms.class(c).type_aliases.values().copied().collect();
        aliases.sort();
        for a in aliases {
            self.complete_alias(a);
            let (rhs, bounds, def_id, name) = {
                let i = &self.syms.aliases[a.idx()];
                (i.rhs, i.bounds, i.def, i.name)
            };
            let Some(def_id) = def_id else { continue };
            let a_span = self.ast(file).def(def_id).span;
            let what = format!("type {}", self.name_str(name));
            self.variance_skip.clear();
            if let (false, DefKind::TypeAlias { rhs: Some(rhs_ty), .. }) = (self.unchecked_variance.is_empty(), &self.ast(file).def(def_id).kind) {
                self.collect_unchecked_variance(file, &[*rhs_ty]);
            }
            match bounds {
                Some((lo, hi)) => {
                    let text = self.bounds_text(lo, hi);
                    for (bound, sign) in [(hi, 1), (lo, -1)] {
                        if let Some((p, required)) = self.variance_violation(bound, sign) {
                            self.report_variance_at(p, required, &format!(" {}", text.trim_start()), file, a_span, &what);
                            break;
                        }
                    }
                }
                None => {
                    if let Some((p, required)) = self.variance_violation(rhs, 0) {
                        let shown = format!(" = {}", self.show(rhs));
                        self.report_variance_at(p, required, &shown, file, a_span, &what);
                    }
                }
            }
        }
        self.variance_skip.clear();
    }

    /// The type expressions of a member's signature: parameters, bounds and result, or the
    /// declared type of a val, or that of the constructor parameter it is.
    fn member_type_exprs(&self, c: ClassId, m: SymId) -> Vec<TyExprId> {
        let info = self.syms.sym(m);
        let mut out = Vec::new();
        let params_of = |clauses: &[ast::ParamClause], out: &mut Vec<TyExprId>| {
            out.extend(clauses.iter().flat_map(|cl| cl.params.iter()).map(|p| p.ty));
        };
        let bounds_of = |tps: &[ast::TypeParam], out: &mut Vec<TyExprId>| {
            out.extend(tps.iter().flat_map(|tp| tp.upper.into_iter().chain(tp.lower)));
        };
        match info.def.map(|d| &self.ast(info.file).def(d).kind) {
            Some(DefKind::Fun(f)) => {
                params_of(&f.clauses, &mut out);
                bounds_of(&f.tparams, &mut out);
                out.extend(f.ret);
            }
            Some(DefKind::Val { ty, .. }) => out.extend(*ty),
            Some(DefKind::Given(g)) => {
                params_of(&g.clauses, &mut out);
                bounds_of(&g.tparams, &mut out);
                out.push(g.ty);
            }
            Some(_) => {}
            None => {
                let class = self.syms.class(c);
                if let Some(DefKind::Class(cls)) = class.def.map(|d| &self.ast(class.file).def(d).kind) {
                    let name = info.name;
                    out.extend(cls.clauses.iter().flat_map(|cl| cl.params.iter()).filter(|p| p.name == name).map(|p| p.ty));
                }
            }
        }
        out
    }

    /// Adds what the `@uncheckedVariance` annotations inside `tys` resolved to.
    fn collect_unchecked_variance(&mut self, file: FileId, tys: &[TyExprId]) {
        let ast = self.ast(file);
        let mut stack: Vec<TyExprId> = tys.to_vec();
        while let Some(t) = stack.pop() {
            match ast.ty(t) {
                TyExpr::UncheckedVariance(inner) => {
                    if let Some(&resolved) = self.unchecked_variance.get(&(file, t)) {
                        self.variance_skip.push(resolved);
                    }
                    stack.push(inner);
                }
                TyExpr::Apply(f, args) => {
                    stack.push(f);
                    stack.extend(ast.ty_list(args).iter().copied());
                }
                TyExpr::Fun(params, ret) => {
                    stack.extend(ast.ty_list(params).iter().copied());
                    stack.push(ret);
                }
                TyExpr::Tuple(items) => stack.extend(ast.ty_list(items).iter().copied()),
                TyExpr::Union(a, b) | TyExpr::Inter(a, b) => {
                    stack.push(a);
                    stack.push(b);
                }
                TyExpr::ByName(inner) | TyExpr::Repeated(inner) | TyExpr::Unchecked(inner) | TyExpr::Lambda(_, inner) | TyExpr::PolyFun(_, inner) => {
                    stack.push(inner)
                }
                _ => {}
            }
        }
    }

    fn report_variance(&mut self, t: TypeId, sign: i8, file: FileId, span: crate::source::Span, what: &str) -> bool {
        let Some((p, required)) = self.variance_violation(t, sign) else { return false };
        let shown = self.show(t);
        self.report_variance_at(p, required, &shown, file, span, what);
        true
    }

    fn report_variance_at(&mut self, p: TParamId, required: i8, in_type: &str, file: FileId, span: crate::source::Span, what: &str) {
        let label = |v: i8| match v {
            1 => "covariant",
            -1 => "contravariant",
            _ => "invariant",
        };
        let msg = format!(
            "{} type {} occurs in {} position in type {} of {}",
            label(self.syms.tparam(p).variance),
            self.name_str(self.syms.tparam(p).name),
            label(required),
            in_type,
            what
        );
        self.diags.error(file, span, msg);
    }

    /// The first variant type parameter of `t` that stands in a position of another sign, with
    /// the sign the position asks for. The arguments of a higher-kinded parameter are not
    /// checked, since the parameter's own variances are not recorded.
    fn variance_violation(&mut self, t: TypeId, sign: i8) -> Option<(TParamId, i8)> {
        let t = self.deref(t);
        if self.variance_skip.contains(&t) {
            return None;
        }
        match self.types.get(t) {
            Type::Param(p) => {
                let v = self.syms.tparam(p).variance;
                (v != 0 && v != sign).then_some((p, sign))
            }
            // `G[A]` for a `G[_]`: an argument stands where `G`'s own parameter puts it.
            Type::AppParam(p, args) => {
                let v = self.syms.tparam(p).variance;
                if v != 0 && v != sign {
                    return Some((p, sign));
                }
                let n = self.types.items(args).len();
                for i in 0..n {
                    let arg = self.types.items(args)[i];
                    let inner = sign * self.syms.tparam(p).hk_variances.get(i).copied().unwrap_or(0);
                    if let Some(found) = self.variance_violation(arg, inner) {
                        return Some(found);
                    }
                }
                None
            }
            Type::Class(k, args) => {
                let n = self.types.items(args).len();
                for i in 0..n {
                    let arg = self.types.items(args)[i];
                    self.settle_class(k);
                    let Some(&tp) = self.syms.class(k).tparams.get(i) else { break };
                    let v = self.syms.tparam(tp).variance;
                    let inner = if v == 0 { 0 } else { sign * v };
                    if let Some(found) = self.variance_violation(arg, inner) {
                        return Some(found);
                    }
                }
                None
            }
            Type::Union(a, b) | Type::Inter(a, b) => {
                self.variance_violation(a, sign).or_else(|| self.variance_violation(b, sign))
            }
            Type::Lambda(_, body) => self.variance_violation(body, sign),
            _ => None,
        }
    }

    /// A val that a class inherits next to the def it implements, from an ancestor that knows
    /// nothing of that def, is reached through a method as well.
    fn reach_through_accessor(&mut self, implementation: SymId, declared: SymId) {
        let is_field = matches!(self.syms.sym(implementation).kind, SymKind::Val | SymKind::Var);
        if is_field && self.syms.sym(declared).kind == SymKind::Def && !self.syms.sym(implementation).needs_accessor {
            self.shared_write(implementation, |w| w.syms.sym_mut(implementation).needs_accessor = true);
            self.vals_by_accessor = true;
        }
    }

    /// The abstract setter of the var `var` that one of `bases` declares, which the var's class
    /// then defines as a method: the one taking the var's type.
    fn implemented_abstract_setter(&mut self, bases: &[ClassId], var: SymId) -> Option<SymId> {
        let setter = self.interner.lookup(&format!("{}_=", self.interner.get(self.syms.sym(var).name)))?;
        let ty = self.sig_of(var).ret;
        let mut one_param = None;
        for &b in bases {
            let mut k = 0;
            while let Some(p) = self.own_alternative(b, setter, k) {
                k += 1;
                if self.syms.sym(p).kind != SymKind::Def || !self.is_abstract_member(p) {
                    continue;
                }
                let param = match self.sig_of(p).clauses.as_slice() {
                    [cl] if cl.params.len() == 1 => cl.params[0].ty,
                    _ => continue,
                };
                if self.syms.sym(p).mods & mods::SETTER != 0 || self.is_same(param, ty) {
                    return Some(p);
                }
                one_param.get_or_insert(p);
            }
        }
        one_param
    }

    /// A val implementing a parameterless def of a parent trait is reached through a method.
    fn mark_accessors(&mut self, c: ClassId) {
        // The members' flags are records of the shared region when the class is a prefix one.
        // The flags are written under the loader's lock where the members are the shared
        // region's (`shared_write`); the signatures are read outside it.
        self.mark_accessors_unlocked(c)
    }

    fn mark_accessors_unlocked(&mut self, c: ClassId) {
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).skip(1).collect();
        let members = self.syms.class(c).member_order.clone();
        for m in members {
            // The setter of an abstract var is declared in the output whether or not a call
            // has completed it.
            if self.syms.sym(m).mods & (mods::SETTER | mods::ABSTRACT) == mods::SETTER | mods::ABSTRACT {
                self.sig_of(m);
            }
            // A given without parameters is a lazy val, which may implement an abstract given
            // (`given x: T`, a def).
            let (kind, lazy) = (self.syms.sym(m).kind, self.syms.sym(m).mods & mods::LAZY != 0);
            if !(matches!(kind, SymKind::Val | SymKind::Var) || (kind == SymKind::Given && lazy)) {
                continue;
            }
            let name = self.syms.sym(m).name;
            if let Some(setter) = (self.syms.sym(m).kind == SymKind::Var).then(|| self.implemented_abstract_setter(&bases, m)).flatten() {
                self.shared_write(m, |w| {
                    let mut info = w.syms.sym_mut(m);
                    info.needs_accessor = true;
                    info.needs_setter = true;
                    w.push_override(m, setter);
                });
            }
            // The runtime calls `toString()` and `hashCode()`, which a val of that name
            // (`case class Shown(override val toString: String)`) implements.
            let mut implemented_def = None;
            let mut beside_method = false;
            // On JavaScript a val overriding one read through a method is read so too: a class
            // the reach checks after the final passes (a library's subclass) takes it from its base
            // here, the ones checked before in `settle_val_accessors`.
            let mut over_accessor = false;
            for &b in &bases {
                let Some(p) = self.syms.class(b).members.get(&name).copied() else { continue };
                if self.syms.sym(p).kind != SymKind::Def {
                    let info = self.syms.sym(p);
                    over_accessor |= info.kind == SymKind::Val && info.needs_accessor && p != m && info.owner == Owner::Class(b) && !self.is_private(p);
                    continue;
                }
                if self.sig_of(p).clauses.iter().all(|cl| cl.is_using) {
                    implemented_def = Some(p);
                    break;
                }
                beside_method = true;
            }
            let overrides_def = matches!(name, names::TO_STRING | names::HASH_CODE) || implemented_def.is_some();
            let overrides_accessor = over_accessor && !overrides_def && !self.jvm && self.syms.sym(m).kind == SymKind::Val && !self.is_private(m);
            if overrides_def || beside_method || overrides_accessor {
                self.vals_by_accessor = true;
            }
            if overrides_def || overrides_accessor {
                self.shared_write(m, |w| w.syms.sym_mut(m).needs_accessor = true);
                // The def may go by a suffixed name (one told apart from an unrelated method
                // of its name in a class below), which the accessor takes.
                if let Some(p) = implemented_def {
                    self.name_like(m, p);
                }
            } else if self.syms.class(c).has_overloads {
                // The alternative without parameters of an overloaded name, whose name in the
                // output the accessor takes.
                let implemented = bases.iter().find_map(|&b| {
                    let mut k = 0;
                    while let Some(p) = self.own_alternative(b, name, k) {
                        k += 1;
                        let is_def = self.syms.sym(p).kind == SymKind::Def && self.syms.sym(p).alternative;
                        if is_def && self.sig_of(p).clauses.iter().all(|cl| cl.is_using) {
                            return Some(p);
                        }
                    }
                    None
                });
                if let Some(p) = implemented {
                    self.shared_write(m, |w| w.syms.sym_mut(m).needs_accessor = true);
                    self.vals_by_accessor = true;
                    self.name_like_always(m, p);
                    continue;
                }
            }
            // A val beside an inherited method of its name that takes parameters overloads it
            // (sttp's `PartialRequestExtensions.body(file)`): the accessor goes by the name of an
            // alternative without parameters, so that the method keeps its own.
            if beside_method && !overrides_def {
                self.shared_write(m, |w| w.syms.sym_mut(m).needs_accessor = true);
                self.name_beside_method(m);
            }
        }
    }

    /// Whether the val `m` overrides a val of one of `bases` that is read through a method.
    fn overrides_accessor_val(&self, bases: &[ClassId], m: SymId) -> bool {
        let name = self.syms.sym(m).name;
        bases.iter().any(|&b| {
            self.syms.class(b).members.get(&name).is_some_and(|&p| {
                let info = self.syms.sym(p);
                p != m && info.owner == Owner::Class(b) && info.kind == SymKind::Val && info.needs_accessor && !self.is_private(p)
            })
        })
    }

    /// On JavaScript a val is a field, or a method over a field where it is read through one
    /// (`needs_accessor`: it implements a def, `mark_accessors`, or a class inherits it beside
    /// one, `reach_through_accessor`). A read through any val of a class's linearisation reaches
    /// the one member of that name, whichever val defines it, so all of them go by the method
    /// once one does (a val implementing a def of one trait and an abstract val of another, read
    /// through the second; a val that a class inherits from one trait for another's, a library
    /// trait's included; a lazy val or an object, whose method reads its lazy getter). A library
    /// val that the reach checks later (after these passes) and gives an accessor is not seen.
    fn settle_val_accessors(&mut self) {
        if self.jvm || !self.vals_by_accessor {
            return;
        }
        let val = |t: &Self, m: SymId| t.syms.sym(m).kind == SymKind::Val && !t.is_private(m);
        let own = |t: &Self, c: ClassId| !t.is_library_class(c) && !t.source(t.syms.class(c).file).is_std;
        // The names a val of the program goes by a method under: only such a name can need the
        // method elsewhere (none in most programs, so this first look allocates nothing else).
        let classes: Vec<ClassId> = (0..self.prog.classes.len()).map(|i| self.prog.classes[i].id).filter(|&c| own(self, c)).collect();
        let mut hot: FxMap<Name, ()> = FxMap::default();
        for &c in &classes {
            for &m in &self.syms.class(c).member_order {
                let info = self.syms.sym(m);
                if info.needs_accessor && info.owner == Owner::Class(c) && val(self, m) {
                    hot.insert(info.name, ());
                }
            }
        }
        if hot.is_empty() {
            return;
        }
        // Each program class's own vals of those names.
        let mut vals: FxMap<ClassId, Vec<(Name, SymId)>> = FxMap::default();
        for &c in &classes {
            for &m in &self.syms.class(c).member_order {
                let name = self.syms.sym(m).name;
                if hot.contains_key(&name) && self.syms.sym(m).owner == Owner::Class(c) && val(self, m) {
                    vals.entry(c).or_default().push((name, m));
                }
            }
        }
        let mut parent: FxMap<SymId, SymId> = FxMap::default();
        fn root(parent: &FxMap<SymId, SymId>, mut s: SymId) -> SymId {
            while let Some(&p) = parent.get(&s) {
                if p == s {
                    break;
                }
                s = p;
            }
            s
        }
        // The vals of a name across a class's linearisation are one member, a library's (a jar's,
        // a product's, the std's) included where the class's own vals carry its name: its body is
        // emitted with the program's, so it takes the method as well.
        for &c in &classes {
            if !self.syms.class(c).base_types.iter().any(|(b, _)| vals.contains_key(b)) {
                continue;
            }
            let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
            let mut members: Vec<(Name, SymId)> = bases.iter().filter_map(|b| vals.get(b)).flatten().copied().collect();
            let names: Vec<Name> = members.iter().map(|&(n, _)| n).collect();
            for &b in bases.iter().filter(|&&b| !own(self, b)) {
                for &n in &names {
                    let Some(&m) = self.syms.class(b).members.get(&n) else { continue };
                    if self.syms.sym(m).owner == Owner::Class(b) && val(self, m) && !self.syms.sym(m).java_defined && !members.contains(&(n, m)) {
                        members.push((n, m));
                    }
                }
            }
            if members.len() < 2 {
                continue;
            }
            let mut by_name: FxMap<Name, SymId> = FxMap::default();
            for (name, m) in members {
                match by_name.get(&name) {
                    Some(&first) => {
                        let (a, z) = (root(&parent, m), root(&parent, first));
                        if a != z {
                            parent.insert(a, z);
                        }
                    }
                    None => {
                        by_name.insert(name, m);
                    }
                }
            }
        }
        if parent.is_empty() {
            return;
        }
        let mut kin: FxMap<SymId, Vec<SymId>> = FxMap::default();
        let mut linked: Vec<SymId> = parent.iter().flat_map(|(&a, &b)| [a, b]).collect();
        linked.sort();
        linked.dedup();
        for s in linked {
            kin.entry(root(&parent, s)).or_default().push(s);
        }
        let mut roots: Vec<SymId> = kin.keys().copied().collect();
        roots.sort();
        let mut library_marked: FxMap<Name, ()> = FxMap::default();
        for r in roots {
            let group = &kin[&r];
            if !group.iter().any(|&s| self.syms.sym(s).needs_accessor) {
                continue;
            }
            for s in group.clone() {
                if !self.syms.sym(s).needs_accessor {
                    self.shared_write(s, |w| w.syms.sym_mut(s).needs_accessor = true);
                    if let Owner::Class(o) = self.syms.sym(s).owner {
                        if !own(self, o) {
                            library_marked.insert(self.syms.sym(s).name, ());
                        }
                    }
                }
            }
        }
        // A library class checked already that overrides a library val given the method here
        // takes it too, a subclass the program's classes do not extend included (one the reach
        // checks later takes it in `mark_accessors`).
        if library_marked.is_empty() {
            return;
        }
        for i in 0..self.prog.classes.len() {
            let c = self.prog.classes[i].id;
            if own(self, c) {
                continue;
            }
            let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
            let members: Vec<SymId> = self.syms.class(c).member_order.clone();
            for m in members {
                let info = self.syms.sym(m);
                if info.owner != Owner::Class(c) || info.needs_accessor || !library_marked.contains_key(&info.name) || !val(self, m) {
                    continue;
                }
                if self.overrides_accessor_val(&bases, m) {
                    self.shared_write(m, |w| w.syms.sym_mut(m).needs_accessor = true);
                }
            }
        }
    }
}

/// Where the program's classes and top-level functions stand in `TProgram`, which the reach pass
/// asks for every body it adds. Both lists only grow between edits, so the index catches up with
/// what was pushed since it last looked.
#[derive(Default)]
pub struct ProgIndex {
    /// Per class the id of its `TClass` record.
    classes: FxMap<ClassId, u32>,
    shared_seen: usize,
    own_seen: usize,
    top_funs: FxMap<FunId, ()>,
    top_funs_shared_seen: usize,
    top_funs_own_seen: usize,
}

impl ProgIndex {
    pub fn class(&mut self, prog: &Program, c: ClassId) -> Option<usize> {
        if let Some(&i) = self.classes.get(&c) {
            if prog.classes.try_get(i).map_or(false, |tc| tc.id == c) {
                return Some(i as usize);
            }
        }
        let shared = prog.classes.shared_len();
        for i in self.shared_seen..shared {
            self.classes.entry(prog.classes[i].id).or_insert(i as u32);
        }
        self.shared_seen = shared;
        let base = prog.classes.own_base();
        for (k, tc) in prog.classes.own().iter().enumerate().skip(self.own_seen) {
            self.classes.entry(tc.id).or_insert(base + k as u32);
        }
        self.own_seen = prog.classes.own().len();
        if let Some(i) = self.classes.get(&c).map(|&i| i as usize).filter(|&i| prog.classes[i].id == c) {
            return Some(i);
        }
        // Another worker's, which it published.
        let i = *prog.class_bodies.get(&c)?;
        super::bundle::entered_at(&prog.classes, i, super::bundle::Entry::Class);
        Some(i as usize)
    }

    pub fn has_top_fun(&mut self, prog: &Program, f: FunId) -> bool {
        for i in self.top_funs_shared_seen..prog.top_funs.shared_len() {
            self.top_funs.insert(prog.top_funs.shared_get(i), ());
        }
        self.top_funs_shared_seen = prog.top_funs.shared_len();
        for &g in prog.top_funs.own().iter().skip(self.top_funs_own_seen) {
            self.top_funs.insert(g, ());
        }
        self.top_funs_own_seen = prog.top_funs.own().len();
        self.top_funs.contains_key(&f)
    }
}

/// The types of a signature that its readers translate: its parameters' and its result.
fn sig_types(sig: &MethodSig) -> impl Iterator<Item = TypeId> + '_ {
    sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)).chain([sig.ret])
}

/// Whether a worker reads the shared records and the signatures in its view with the overlays
/// on (`TEQ_VIEW_READS=0` reads them raw, for the measurement of what the views cost and change).
fn view_reads_wanted() -> bool {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    !*OFF.get_or_init(|| std::env::var_os("TEQ_VIEW_READS").is_some_and(|v| v == "0"))
}


