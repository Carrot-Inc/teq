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
/// with its items in the order dotty's typer meets them (`Worker::items_of`). A worker takes whole files from the front, one at a time, and
/// types their items from the head; a worker without a file takes single items from the tail of a
/// file another worker holds, so that the holder still meets its file's definitions in order.
pub struct Queue {
    /// The files and how many items each has.
    pub files: Vec<(FileId, u32)>,
    /// Per file, its items' indices in that order where it is not the written one
    /// (`super::inlining::desugared_order`).
    orders: Vec<Option<Vec<u32>>>,
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
        let orders = files.iter().map(|&(f, _)| w.desugared_items(f)).collect();
        Queue { files, orders, next_file: std::sync::atomic::AtomicUsize::new(0), cursors, stolen: std::sync::atomic::AtomicUsize::new(0) }
    }

    /// The index among its file's top-level definitions of the item at `i` of the file's order.
    pub fn item_index(&self, fi: usize, i: u32) -> u32 {
        self.orders[fi].as_ref().map_or(i, |o| o[i as usize])
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
        self.forwarders_on_completion = true;
        // The classes with export clauses first, in the order of the files and their places, as
        // dotty's typer meets them: where their forwarders read each other's (a cycle of two
        // objects exporting each other), what each gets does not depend on the demand order.
        for i in 0..self.syms.classes.len() {
            let info = &self.syms.classes[i];
            if info.has_exports && !self.in_jar(info.file) && self.std.slot_of(info.file).is_none() {
                self.complete_class(ClassId(i as u32));
            }
        }
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
        // Value exports make members, once every class they read is complete, before the
        // classes' export tables (`build_scope_tables`), which read them.
        self.add_value_forwarders();
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
            // The companion the typer made has no exports or givens.
            if self.syms.class(c).made_companion && self.syms.class(c).def.is_none() {
                continue;
            }
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
        self.begin_work();
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
                        self.check_queued(file, q.item_index(fi, i));
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
                    self.check_queued(q.files[fi].0, q.item_index(fi, i));
                }
                None => break,
            }
        }
        self.walk.done = true;
        self.end_work();
    }

    /// What a worker's thread sets before it types for its worker: its arena ids, its overlay of
    /// the type store, its views of the shared records.
    pub(super) fn begin_work(&mut self) {
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
    }

    /// What a worker's thread undoes as its work ends (`begin_work`).
    pub(super) fn end_work(&mut self) {
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

    /// Types the item `i` of `file` (its index among the file's top-level definitions) and records
    /// the own records it made (`ItemRange`), which the merge places in the order of the sources,
    /// by the file and the item.
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
        self.note_mixin_super_call(c, crate::tir::SuperCall::member(m));
    }

    /// Notes that trait `c` selects the getter of `m`'s parameter `index` through `super`
    /// (`super.m$default$N`, `Applications.findDefaultGetter` on a `super` receiver), which
    /// `SuperAccessors` gives an accessor before the member's.
    pub(super) fn note_mixin_super_getter(&mut self, c: ClassId, m: SymId, index: u32) {
        self.note_mixin_super_call(c, crate::tir::SuperCall { member: m, getter: Some(index) });
    }

    fn note_mixin_super_call(&mut self, c: ClassId, call: crate::tir::SuperCall) {
        let mut shared = self.shared_class(c).then(|| self.shared_mixin_supers.lock().unwrap_or_else(|e| e.into_inner()));
        let calls = match &mut shared {
            Some(shared) => shared.entry(c).or_default(),
            None => self.mixin_supers.entry(c).or_default(),
        };
        if calls.contains(&call) {
            return;
        }
        match call.getter.and_then(|_| calls.iter().position(|k| k.member == call.member && k.getter.is_none())) {
            Some(at) => calls.insert(at, call),
            None => calls.push(call),
        }
    }

    /// The member of class `b` a call of `member` through `super` may run: of an overloaded
    /// name, the alternative with the member's parameters; none private or abstract.
    pub(super) fn concrete_super_member(&mut self, b: ClassId, member: SymId) -> Option<SymId> {
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
    pub(super) fn mixin_super_target_in(&mut self, bases: &[ClassId], at: usize, member: SymId) -> Option<SymId> {
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
        let mut calls: FxMap<ClassId, Vec<crate::tir::SuperCall>> = self.mixin_supers.clone();
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
                for call in members {
                    let member = call.member;
                    // A getter selected on `super`: the one of the member the target takes the
                    // default from, as `findDefaultGetter` selects it after the trait.
                    if let Some(index) = call.getter {
                        let target = self.mixin_super_target_in(&bases, at, member).filter(|&t| self.syms.sym(t).kind == SymKind::Def);
                        if let Some(d) = target.and_then(|t| self.default_declarer(t, index as usize)) {
                            accessors.push(SuperAccessor { of_trait: tr, member, target: Some(d), getter: Some(index) });
                        }
                        continue;
                    }
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
                        Some(_) => accessors.push(SuperAccessor { of_trait: tr, member, target, getter: None }),
                        None if of_any => accessors.push(SuperAccessor { of_trait: tr, member, target, getter: None }),
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

    /// The work items of `file` in the order dotty's typer meets them: the file's classes and objects,
    /// then the members of its `<file>$package` (`super::inlining::desugared_order`).
    pub fn items_of(&self, file: FileId) -> impl Iterator<Item = Item> + 'a {
        let ast: &'a crate::ast::Ast = self.ast(file);
        let order = self.desugared_items(file);
        (0..ast.top_level.len()).map(move |i| Item { file, def: ast.top_level[order.as_ref().map_or(i, |o| o[i] as usize)] })
    }

    /// The order of `file`'s items where it is not the written one (`items_of`): a file of the
    /// program's, whose typing a macro of the run may suspend; the std's and a library's bodies'
    /// are typed in their written order, which nothing observes.
    pub(super) fn desugared_items(&self, file: FileId) -> Option<Vec<u32>> {
        if !self.program_file(file) {
            return None;
        }
        super::inlining::desugared_order(self.ast(file))
    }

    /// Types the items of a file, in order.
    pub(super) fn check_file(&mut self, file: FileId) {
        for item in self.items_of(file) {
            self.check_item(item);
        }
    }

    /// Types one work item: the whole of a top-level definition (`Item`).
    pub fn check_item(&mut self, item: Item) {
        // A unit suspended at typing is typed no further in this run (`TyperPhase.typeCheck`'s
        // `!unit.suspended`): its items after the one its typing was cut in, in the order dotty's
        // typer meets them (`super::inlining::top_level_key`). One before it, which another
        // worker's cut found untyped, is typed: dotty's traversal reached it first.
        if !self.inlining.typing_suspended.is_empty() && self.inlining.typing_suspended.contains(&item.file.0) {
            let key = super::inlining::top_level_key(self.ast(item.file), item.def);
            if self.inlining.typing_cuts.iter().any(|c| c.file == item.file.0 && c.key < key) {
                return;
            }
        }
        let pkg = self.file_pkgs[item.file.0 as usize];
        self.env.file = item.file;
        // The file's imports in scope: those before a class or object (`typedStats`), every one
        // for a member of the package object (`Env::import_limit`).
        self.env.import_limit = if super::inlining::wrapped_top_level(self.ast(item.file), item.def) { None } else { Some((item.file, self.ast(item.file).def(item.def).span.start)) };
        self.env.visible_imports = None;
        self.inlining.item_place = self.ast(item.file).def(item.def).span.start;
        let owner = crate::source::DiagOwner::new(item.file, self.inlining.item_place);
        let outer_demands = std::mem::replace(&mut self.inlining.item_demands, self.inlining.demands.len());
        let outer_owner = self.enter_owner(owner);
        self.inlining.in_item += 1;
        let frame = self.var_frame_begin();
        // The file's import statements before the item, which dotty's traversal reaches first.
        self.reach_file_imports(item.file, item.def);
        self.check_top_def(item.file, Owner::Package(pkg), item.def);
        // Those after the file's last item, reached after it.
        if !self.ast(item.file).imports.is_empty() && self.items_of(item.file).last().map(|i| i.def) == Some(item.def) {
            self.reach_trailing_file_imports(item.file);
        }
        self.var_frame_end(frame);
        self.inlining.in_item -= 1;
        self.leave_owner(outer_owner, false);
        self.inlining.item_demands = outer_demands;
        if self.inlining.in_item == 0 {
            self.inlining.unwinding = false;
        }
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
    /// `<stem>$package`; every name of the path encoded, its packages' included (dotty's
    /// `javaClassName`, `fullName.mangledString`).
    pub(crate) fn entry_point_class(&self, sym: SymId, object: Option<ClassId>) -> String {
        let info = self.syms.sym(sym);
        let in_package = |t: &Self, p: PkgId, name: &str| if p == ROOT_PKG { name.to_string() } else { format!("{}.{}", t.binary_package(p), name) };
        match (object, info.owner) {
            (Some(c), _) => self.binary_path(c),
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
            (None, Owner::Class(c)) => self.binary_path(c),
            (None, Owner::Package(p)) if info.is_main => in_package(self, p, &crate::jvm::names::encode(&self.name_str(info.name))),
            (None, Owner::Package(p)) => {
                let stem = crate::tasty::write::file_stem(&self.source(info.file).path);
                in_package(self, p, &format!("{}$package", crate::jvm::names::encode(&stem)))
            }
            (None, Owner::Local) => self.name_str(info.name),
        }
    }

    /// An entry point's object by its class path with each name encoded as the JVM's class is named
    /// (`a$minusb` for an object `` `a-b` ``, dotty's `javaClassName` of `Name.mangledString`).
    fn binary_path(&self, c: ClassId) -> String {
        let info = self.syms.class(c);
        let name = crate::jvm::names::encode(&self.name_str(info.name));
        match info.owner {
            Owner::Package(p) if p == ROOT_PKG => name,
            Owner::Package(p) => format!("{}.{}", self.binary_package(p), name),
            Owner::Class(o) => format!("{}.{}", self.binary_path(o), name),
            Owner::Local => name,
        }
    }

    /// A package's path with each name encoded, as `Names.mangledString` mangles a qualified name's
    /// every part and the JVM target places the package's classes (`x$minusy` for a directory `x-y`);
    /// empty for the root.
    pub(super) fn binary_package(&self, p: PkgId) -> String {
        if p == ROOT_PKG {
            return String::new();
        }
        let info = self.syms.pkg(p);
        let name = crate::jvm::names::encode(&self.name_str(info.name));
        match info.parent.filter(|&parent| parent != ROOT_PKG) {
            Some(parent) => format!("{}.{}", self.binary_package(parent), name),
            None => name,
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
    pub(super) fn is_java_main(&mut self, sym: SymId) -> bool {
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

    /// A `@main` method runs with the command line: without parameters, with them as `String*`, or
    /// through its proxy (dotty's `MainProxies`, `parser::add_main_proxies`), which parses them into
    /// the parameters' types (`CommandLineParser`); dotty's refusals of the others.
    fn enter_main_method(&mut self, sym: SymId) {
        // dotty's `MainProxies` takes the methods `scala.main` annotates (`defn.MainAnnot`): an annotation
        // class of the program's own of that name makes none.
        if !self.annotated_scala_main(sym) {
            return;
        }
        let sig = self.sig_arc(sym);
        let params: Vec<&ParamSig> = sig.clauses.iter().flat_map(|c| c.params.iter()).collect();
        let takes_strings = match params.as_slice() {
            [] => true,
            [p] => p.repeated && !p.by_name && p.ty == self.b.t_string,
            _ => false,
        };
        let refusal = if !sig.tparams.is_empty() {
            Some("@main method cannot have type parameters".to_string())
        } else if sig.clauses.iter().any(|c| c.is_using) {
            Some("@main method cannot have implicit parameters".to_string())
        } else if params.iter().rev().skip(1).any(|p| p.repeated) {
            Some("varargs parameter of @main method must come last".to_string())
        } else if let Some(p) = params.iter().find(|p| p.by_name) {
            // The proxy parses a by-name parameter as its by-name type, which has no `FromString`.
            Some(format!("No given instance of type scala.util.CommandLineParser.FromString[=> {}] was found for parameter fs of method parseArgument in object CommandLineParser", self.show(p.ty)))
        } else if let Some(clash) = self.main_proxy_clash(sym) {
            Some(clash)
        } else if !takes_strings && self.syms.main_adapter(self.interner, sym).is_none() {
            Some("@main method cannot be run: its proxy is missing".to_string())
        } else {
            None
        };
        if let Some(msg) = refusal {
            let (file, span) = {
                let s = self.syms.sym(sym);
                (s.file, s.span)
            };
            self.diags.error(file, span, msg);
            return;
        }
        self.entry_points.push((sym, None));
    }

    /// The `n`th parameter's type of the `@main` method `def` of the file being typed, as its signature has it:
    /// a repeated one's element type (`TyExpr::MainParam`, dotty's `MainProxies`' formal types).
    pub(super) fn main_param_type(&mut self, def: DefId, n: u32) -> TypeId {
        let file = self.env.file;
        let Some(&sym) = self.def_syms.get(file.0 as usize, &def) else { return crate::types::ERROR };
        let sig = self.sig_arc(sym);
        sig.clauses.iter().flat_map(|c| c.params.iter()).nth(n as usize).map_or(crate::types::ERROR, |p| p.ty)
    }

    /// Whether the proxy `id` of the file is one of a method `scala.main` does not annotate, which dotty's
    /// `MainProxies` makes none of: it is left untyped and runs nowhere.
    fn idle_main_proxy(&mut self, file: FileId, id: DefId) -> bool {
        let Some(main) = self.ast(file).main_proxies.iter().find(|&&(p, _)| p == id).map(|&(_, m)| m) else { return false };
        match self.def_syms.get(file.0 as usize, &main) {
            Some(&sym) => !self.annotated_scala_main(sym),
            None => true,
        }
    }

    /// Whether the `@main` of `sym` is `scala.main` as its type resolves where the method is defined, not a
    /// class of the program that takes its name.
    fn annotated_scala_main(&mut self, sym: SymId) -> bool {
        let info = self.syms.sym(sym);
        let (Some(d), file, owner, at) = (info.def, info.file, info.owner, info.span.start) else { return true };
        let Some(target) = self.std_class("main") else { return true };
        let ast = self.ast(file);
        let env = self.env_at(file, owner, at);
        self.with_env(env, |t| {
            let mark = t.diags.items.len();
            let found = ast.def(d).annots.iter().filter(|a| a.name == names::MAIN).any(|a| match ast.annot_new(a) {
                Some((ty, _)) => {
                    let resolved = t.resolve_type_ctor(ty);
                    matches!(t.types.get(resolved), Type::Class(k, _) | Type::Ctor(k) if k == target)
                }
                None => true,
            });
            t.drop_reported_since(mark);
            found
        })
    }

    /// dotty's proxy of a `@main` method is a class of the method's name in the package of its outermost
    /// object (`MainProxies`): a class of that name there is defined twice, an object's mirror class
    /// overwritten (the backend's refusals).
    fn main_proxy_clash(&self, sym: SymId) -> Option<String> {
        let info = self.syms.sym(sym);
        let mut owner = info.owner;
        while let Owner::Class(c) = owner {
            owner = self.syms.class(c).owner;
        }
        let Owner::Package(p) = owner else { return None };
        let entry = self.syms.scoped(self.interner, p, info.name)?;
        let n = self.name_str(info.name);
        match (entry.class, entry.term.map(|t| self.syms.sym(t).kind)) {
            (Some(c), _) if self.syms.class(c).kind != ClassKind::Object => Some(format!("{n} is already defined as class {n}")),
            (Some(_), _) | (None, Some(SymKind::Object(_))) => Some(format!("{n} and {n} produce classes that overwrite one another")),
            _ => None,
        }
    }

    pub(super) fn check_top_def(&mut self, file: FileId, owner: Owner, id: DefId) {
        self.restrict_def(file, id);
        self.check_def_warnings(file, id);
        if self.unused.on() {
            self.mark_annotations(file, id);
            self.mark_module_prefixes(file, id, owner, true);
        }
        let ast = self.ast(file);
        let def = ast.def(id);
        let lacks_body = match &def.kind {
            DefKind::Val { rhs, .. } => rhs.is_none(),
            DefKind::Fun(f) => f.body.is_none() && !def.annots.iter().any(|a| a.name == names::JS || (a.name == names::JVM && self.jvm)),
            _ => false,
        };
        // Both interop annotations report a missing body in their own terms.
        let is_interop = |a: &ast::Annot| a.name == names::JS_IMPORT || a.name == names::JS_EXPORT;
        let misplaced = self.misplaced_abstract_member(def, true);
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
                    // A std val is typed where the program names it (`std_val_body`), as dotty's
                    // package lookup completes a member by its name: the prelude's `Stream`
                    // enters no file of a program that does not name it.
                    if !self.defers_top_val(sym) {
                        self.walk_body(sym);
                        if let Some(init) = self.readable_init(sym) {
                            self.prog.top_vals.push((sym, init));
                        }
                    }
                }
            }
            DefKind::Fun(_) => {
                if let Some(&sym) = self.def_syms.get(file.0 as usize, &id) {
                    if !ast.main_proxies.is_empty() && self.idle_main_proxy(file, id) {
                        return;
                    }
                    if !self.defers_body(sym) {
                        self.walk_body(sym);
                    }
                    if let Some(f) = self.readable_fun(sym) {
                        self.register_top_fun(f);
                    }
                    if self.syms.sym(sym).is_main {
                        self.enter_main_method(sym);
                        self.credit_main_proxy(sym);
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
            DefKind::TypeAlias { .. } => self.reach_alias(file, id),
        }
        if self.capturing() && !matches!(def.kind, DefKind::Class(_)) {
            self.capture_def_annotations(file, owner, id, true);
        }
    }

    /// The annotations of the binders of `clause` (`SyntaxNotes::binder_annots`), typed where the typer resolves the
    /// clause, as dotty's `Typer.completeAnnotations` types a binder's `TypeDef`'s in `annotContext`: a lambda's before
    /// its binders exist, a type definition's own clause's with them (the callers' places); one whose binders a type
    /// definition owns (a lambda in its right side or bounds, `type_def_frames`) outside the definition's frames
    /// (`isInner`).
    #[cold]
    #[inline(never)]
    pub(super) fn binder_annotations(&mut self, clause: ast::BinderClause) {
        self.binder_annotations_of(clause);
    }

    fn binder_annotations_of(&mut self, clause: ast::BinderClause) {
        if self.kind_only_bound {
            return;
        }
        let file = self.env.file;
        let records: Vec<ast::BinderAnnot> = self.ast(file).notes.binder_annots.iter().filter(|r| r.clause == clause).copied().collect();
        if records.is_empty() {
            return;
        }
        let owned = match clause {
            ast::BinderClause::Lambda(_) => self.type_def_frames.filter(|&depth| depth <= self.env.frames.len()),
            _ => None,
        };
        match owned {
            Some(depth) => {
                let frames = self.env.frames[..depth].to_vec();
                let imports = self.env.imports.iter().copied().filter(|i| (i.depth as usize) <= depth).collect();
                // The file's imports visible here stay so (`import_limit`): the cut drops frames alone.
                let (import_limit, visible_imports) = (self.env.import_limit, self.env.visible_imports.clone());
                let env = super::Env { file, frames, imports, import_limit, visible_imports };
                self.with_env(env, |t| t.complete_binder_annotations(file, &records));
            }
            None => self.complete_binder_annotations(file, &records),
        }
    }

    fn complete_binder_annotations(&mut self, file: FileId, records: &[ast::BinderAnnot]) {
        // scalac's dependency extraction reads no annotation of a definition.
        let deps = self.deps.take();
        let outer = std::mem::replace(&mut self.typing_annotation, true);
        // What the annotation holds is the annotation's dummy owner's, no type definition's.
        let type_def = self.type_def_frames.take();
        for r in records {
            self.complete_binder_annotation(file, r);
        }
        self.type_def_frames = type_def;
        self.typing_annotation = outer;
        self.deps = deps;
    }

    /// The traversal reaches the type alias defined at `id` where it stands
    /// (`Typer.typedStats` → `typedTypeDef`): its completion, done or suspended, is one the
    /// definition being typed reached (`complete_alias`), as dotty completes it there.
    fn reach_alias(&mut self, file: FileId, id: DefId) {
        // A std alias is completed where a program names it (as the std's vals are,
        // `defers_top_val`): dotty's package lookup completes a library member by its name
        // (`SymDenotations.computeMembersNamed`), and the prelude's deprecated `Traversable` or
        // `Stream` enters no file of a program that does not name it.
        if self.std_source(file) {
            return;
        }
        if let Some(&a) = self.def_aliases.get(file.0 as usize, &id) {
            self.complete_alias(a);
        }
    }

    /// `TypeDefCompleter.completerTypeParams` (`Namer.scala` 1053-1073) over the binders `ids` a clause made (dotty's
    /// `index(tparams)`), each completed in turn (`typedAheadExpr(param)` → `ensureCompleted`) and then its annotations
    /// (`completeAnnotations`, an own clause's `records`), a binder a read forces completing then
    /// (`Worker::binder_info`). `bounds` holds each binder's lower and upper bound as written.
    #[cold]
    #[inline(never)]
    pub(super) fn complete_clause(&mut self, ids: &[TParamId], bounds: Vec<(TyExprId, TyExprId)>, records: Vec<ast::BinderAnnot>, keys: [Option<ast::BinderClause>; 2]) {
        let file = self.env.file;
        // A binder's bounds are its own (`typedTypeDef`'s context), no type definition's.
        let type_def = self.type_def_frames.take();
        let n = ids.len();
        self.binder_completing.push(super::ClauseCompletion { file, ids: ids.to_vec(), states: vec![super::BinderState::NotStarted; n], bounds, own: vec![crate::tir::NO_TYPE; n], records, keys, failed: Vec::new() });
        let at = self.binder_completing.len() - 1;
        for i in 0..n {
            if self.binder_completing[at].states[i] == super::BinderState::NotStarted {
                // A cycle a completion at the top meets is reported where it is read (`binder_info`'s callers).
                let _ = self.complete_binder(at, i);
            }
            // The binder's annotations installed on it (`addAnnotations`' `sym.addAnnotation`).
            let all: Vec<ast::BinderAnnot> = self.binder_completing[at].records.iter().filter(|r| r.index as usize == i).copied().collect();
            self.install_binder_annotations(file, &all, ids);
            // `completeAnnotations`: the binder's annotations typed whole, its completion over; one whose class's typing
            // failed is the error tree it made.
            let c = &self.binder_completing[at];
            let own: Vec<ast::BinderAnnot> = c.records.iter().filter(|r| r.index as usize == i && !c.failed.contains(&r.annot.instance)).copied().collect();
            if !own.is_empty() {
                self.complete_binder_annotations(file, &own);
            }
        }
        self.binder_completing.pop();
        self.type_def_frames = type_def;
    }

    /// `Namer.addAnnotations`' `sym.addAnnotation`: the records' annotations installed on their binders among `ids`
    /// (`Symbols::binder_annots`), which the pickle writes on a binder's `TYPEPARAM`.
    #[cold]
    #[inline(never)]
    pub(super) fn install_binder_annotations(&mut self, file: FileId, records: &[ast::BinderAnnot], ids: &[TParamId]) {
        for (i, &p) in ids.iter().enumerate() {
            let annots: Vec<ast::Annot> = records.iter().filter(|r| r.index as usize == i).map(|r| r.annot).collect();
            if !annots.is_empty() {
                self.syms.binder_annots.insert(p, (file, annots));
            }
        }
    }

    /// `SymDenotation.completeFrom` → `Namer.Completer.completeInCreationContext` of the binder `i` of the clause `at`:
    /// `Touched` while its annotations' classes are typed (`addAnnotations`' `typedAheadAnnotationClass`), `Provisional`
    /// (`dummyInfo1`) while its bounds are (`typeSig`'s `typedAheadType(rhs)`), `abstracted`'s `lo.isRef(Nothing)` a
    /// read of the lower bound, then `Checking` (`NoCompleter`) while `checkNonCyclic` checks them. A cycle met outside
    /// a tree of its own (`abstracted`'s read) leaves the completion: the binder keeps `dummyInfo1` and the cycle goes
    /// to the read that started the completion, which reports it.
    fn complete_binder(&mut self, at: usize, i: usize) -> Result<(), TParamId> {
        use super::BinderState::*;
        let p = self.binder_completing[at].ids[i];
        self.binder_completing[at].states[i] = Touched;
        // `addAnnotations`: each annotation's class, the `New`'s type expression alone; the arguments wait.
        let records: Vec<ast::BinderAnnot> = self.binder_completing[at].records.iter().filter(|r| r.index as usize == i).copied().collect();
        if !records.is_empty() {
            let outer = std::mem::replace(&mut self.typing_annotation, true);
            let deps = self.deps.take();
            let file = self.binder_completing[at].file;
            for r in &records {
                if let Some(tpt) = self.annotation_new_type(r.annot.instance) {
                    let mark = self.diags.items.len();
                    self.resolve_type(tpt);
                    // The typed-ahead tree an error, which the annotation's typing reuses (`TypedAhead`): typed no more.
                    let span = self.ast(file).expr_span(r.annot.instance);
                    if self.diags.items[mark..].iter().any(|d| !d.is_warning && d.file == file && d.span.start >= span.start && d.span.end <= span.end) {
                        self.binder_completing[at].failed.push(r.annot.instance);
                    }
                }
            }
            self.deps = deps;
            self.typing_annotation = outer;
        }
        self.binder_completing[at].states[i] = Provisional;
        // `typedAheadType(rhs)`: the bounds typed, the reads in them caught at their trees.
        let (lo, hi) = self.binder_completing[at].bounds[i];
        let lower = if lo == crate::ast::NO_BOUND { NOTHING } else { self.resolve_type(lo) };
        let upper = if hi == crate::ast::NO_BOUND { ANY } else { self.resolve_type(hi) };
        // `abstracted` → `boundsFromParams`' `lo.isRef(defn.NothingClass)`: its reads (its answer chooses whether the
        // lower bound is abstracted over the binder's own clause, which teq's bound keeps as written).
        if let Err(cycle) = self.is_ref_reads(lower, RefSym::Nothing) {
            self.binder_completing[at].states[i] = Done;
            return Err(cycle);
        }
        self.binder_completing[at].states[i] = Checking;
        let (lower, upper) = match self.check_non_cyclic(at, i, lower, upper) {
            Ok(()) => (lower, upper),
            Err(()) => (ERROR, ERROR),
        };
        let info = &mut self.syms.tparams[p.idx()];
        info.lower = lower;
        info.upper = upper;
        self.binder_completing[at].states[i] = Done;
        Ok(())
    }

    /// The type expression of the class an annotation instantiates (`typedAheadAnnotationClass`: through `Apply`,
    /// `TypeApply` and the constructor's selection to `New(tpt)`).
    fn annotation_new_type(&self, instance: ast::ExprId) -> Option<TyExprId> {
        let ast = self.cur_ast();
        let mut e = instance;
        loop {
            match ast.expr(e) {
                ast::Expr::Apply(f, _) | ast::Expr::TypeApply(f, _) => e = f,
                ast::Expr::New(tpt, _) => return Some(tpt),
                _ => return None,
            }
        }
    }

    /// The read of the binder `p`'s information (`TypeRef.info` → `completeFrom`): a binder of a clause in completion
    /// `NotStarted` completes now; one `Touched` or `Checking` is a cycle (`CyclicReference`), named by the binder its
    /// completion's read met; `Provisional` and `Done` read their bounds. No binder of a clause in completion: nothing.
    pub(super) fn binder_info(&mut self, p: TParamId) -> Result<(), TParamId> {
        use super::BinderState::*;
        for at in (0..self.binder_completing.len()).rev() {
            if let Some(i) = self.binder_completing[at].ids.iter().position(|&q| q == p) {
                return match self.binder_completing[at].states[i] {
                    NotStarted => self.complete_binder(at, i),
                    Touched | Checking => Err(p),
                    Provisional | Done => Ok(()),
                };
            }
        }
        Ok(())
    }

    /// The state of the binder `p` where a clause in completion has it.
    fn binder_state(&self, p: TParamId) -> Option<super::BinderState> {
        self.binder_completing.iter().rev().find_map(|c| c.ids.iter().position(|&q| q == p).map(|i| c.states[i]))
    }

    /// E046 at `at` for the cycle a read met (`CyclicReference`, caught by the tree whose typing read it:
    /// `typedUnadapted`'s `TypeError` handler).
    pub(super) fn report_cyclic(&mut self, cycle: TParamId, at: Span) {
        let msg = format!("Cyclic reference involving type {}", self.type_name_ref(self.syms.tparam(cycle).name));
        self.error(at, msg);
    }

    /// `Type.isRef(sym)` (`core/Types.scala` 198-218, `skipRefined` true), with its reads: whether the symbol the
    /// reference reaches (`ref_target_reads`) is `sym`.
    #[inline(always)]
    pub(super) fn is_ref_reads(&mut self, t: TypeId, sym: RefSym) -> Result<bool, TParamId> {
        Ok(self.ref_target_reads(t)? == Some(sym))
    }

    /// `Type.isRef(sym)` where no read can meet a cycle the caller reports (`is_ref_reads`'s cycle reads as no reference).
    #[inline(always)]
    pub(super) fn is_ref(&mut self, t: TypeId, sym: RefSym) -> bool {
        self.is_ref_reads(t, sym).unwrap_or(false)
    }

    /// The class (or `Nothing`) `isRef` finds `t` a reference to, none where it is no reference to a class: `isRef`'s
    /// walk (`core/Types.scala` 198-218), one chain whatever `sym` is, with the reads a `TypeRef`'s information makes
    /// (`this1.info`): a reference to a class that class (`Nothing` teq's own type); another reference its information's
    /// alias followed (`TypeAlias(tp) => tp.isRef`), else its own symbol, no class: a parameter's information is bounds,
    /// a declaration's or a member's an alias or bounds (one completing its own `typeSig` its completer); an application
    /// dealiased where its constructor is an alias (`AppliedType.dealias`: the alias applied), else its constructor
    /// (`underlying`); an alias kept by name: a curried one dealiased, a match alias's information a `MatchAlias`, no
    /// `TypeAlias`; a refinement, a recursive type, an annotated type their parent; a variable its instance
    /// (`instanceOpt`), an open one none; a polymorphic function type the refinement of `PolyFunction` it is.
    #[inline(always)]
    pub(super) fn ref_target_reads(&mut self, t: TypeId) -> Result<Option<RefSym>, TParamId> {
        self.ref_target_of::<false>(t)
    }

    /// `ref_target_reads` of `t`, or with `WIDEN` of `t.widen` (`preCheckKind`'s `arg.tpe.widen.isRef`) on the same
    /// dispatch: `widen`'s fast path a type that is no path, no variable and no annotated type itself
    /// (`case _: TypeRef => this`), else a path's underlying type, through variables and annotations (`widen_path`).
    #[inline(always)]
    fn ref_target_of<const WIDEN: bool>(&mut self, t: TypeId) -> Result<Option<RefSym>, TParamId> {
        match self.types.get(t) {
            Type::Nothing => Ok(Some(RefSym::Nothing)),
            Type::Class(c, _) | Type::Ctor(c) => Ok(Some(RefSym::Class(c))),
            // A parameter's information is bounds, its read nothing outside a binder's completion (`binder_info`).
            Type::Param(_) | Type::AppParam(..) if self.binder_completing.is_empty() => Ok(None),
            Type::This(_) | Type::Term(_) | Type::Select(..) | Type::Var(_) | Type::AppVar(..) | Type::Annotated(_) if WIDEN => {
                let widened = self.widen_path(t);
                self.ref_target_reads(widened)
            }
            _ => self.ref_target_other(t),
        }
    }

    /// `t.widen.isRef(sym)` with its reads (`ref_target_of`).
    #[inline(always)]
    pub(super) fn widened_is_ref_reads(&mut self, t: TypeId, sym: RefSym) -> Result<bool, TParamId> {
        Ok(self.ref_target_of::<true>(t)? == Some(sym))
    }

    /// `ref_target_reads` of a type no class's or `Nothing`'s own reference: out of the kind checks' line.
    #[cold]
    #[inline(never)]
    fn ref_target_other(&mut self, t: TypeId) -> Result<Option<RefSym>, TParamId> {
        let t = self.deref_keeping(t);
        match self.types.get(t) {
            Type::Nothing => Ok(Some(RefSym::Nothing)),
            Type::Class(c, _) | Type::Ctor(c) => Ok(Some(RefSym::Class(c))),
            Type::Nested(_, class) => self.ref_target_reads(class),
            Type::Param(p) | Type::AppParam(p, _) => {
                self.binder_info(p)?;
                Ok(None)
            }
            Type::Decl(_) | Type::Member(..) => match self.kind_info(t)? {
                KindInfo::Alias(alias) => self.ref_target_reads(alias),
                _ => Ok(None),
            },
            Type::AppMember(m, args) => match self.kind_info(m)? {
                KindInfo::Alias(alias) => {
                    let args: Vec<TypeId> = self.types.items(args).to_vec();
                    match self.apply_if_parameterized(alias, &args)? {
                        KindInfo::Bounds(applied) | KindInfo::Alias(applied) => self.ref_target_reads(applied),
                        KindInfo::Unknown => Ok(None),
                    }
                }
                _ => Ok(None),
            },
            Type::Alias(a, _) => {
                if self.is_match_alias(a) {
                    return Ok(None);
                }
                match self.kind_dealias(t) {
                    Some(e) => self.ref_target_reads(e),
                    None => Ok(None),
                }
            }
            Type::Refined(parent, _) | Type::Rec(parent) | Type::Annotated(parent) => self.ref_target_reads(parent),
            Type::Poly(..) => Ok(self.std_class("PolyFunction").map(RefSym::Class)),
            _ => Ok(None),
        }
    }

    /// `ref_target_reads` where no read can meet a cycle the caller reports.
    #[inline(always)]
    pub(super) fn ref_target(&mut self, t: TypeId) -> Option<RefSym> {
        self.ref_target_reads(t).unwrap_or(None)
    }

    /// The reads a member selection makes of its prefix (`prefix.member(name)`: the prefix's information).
    fn prefix_reads(&mut self, prefix: TypeId) -> Result<(), TParamId> {
        let prefix = self.deref_keeping(prefix);
        match self.types.get(prefix) {
            Type::Param(p) | Type::AppParam(p, _) => self.binder_info(p),
            Type::Member(q, _) | Type::AppMember(q, _) => self.prefix_reads(q),
            _ => Ok(()),
        }
    }

    /// A non-class reference's information as the kind queries read it (`TypeRef.info`), with its reads: a parameter's
    /// (`binder_info`; a binder whose bounds are being typed its `dummyInfo1`, `binder_own_clause`), a member's or a
    /// declaration's (its prefix's reads, then `type_member`; one still completing its `dummyInfo1` over its declared
    /// parameters, `typeSig`'s), each as `boundsFromParams` keeps it (`param_info_hi`, `member_info_hi`); an
    /// application's `superType` (`AppliedType.superType`, `core/Types.scala` 4742-4754: the constructor's applied,
    /// `apply_if_parameterized`); an open variable's `origin`'s (`TypeVar.underlying` 5199-5202, `ParamRef.underlying`
    /// 4885-4890: the parameter it was made for, else the bounds it keeps).
    fn kind_info(&mut self, t: TypeId) -> Result<KindInfo, TParamId> {
        match self.types.get(t) {
            Type::Param(p) => {
                self.binder_info(p)?;
                if self.binder_state(p).is_some_and(|s| s != super::BinderState::Done) {
                    return Ok(KindInfo::Bounds(self.binder_own_clause(p)));
                }
                Ok(KindInfo::Bounds(self.param_info_hi(p)))
            }
            Type::Decl(a) => {
                if self.member_completing(t) {
                    return Ok(KindInfo::Bounds(self.declared_clause(a)));
                }
                self.complete_alias(a);
                let info = &self.syms.aliases[a.idx()];
                Ok(match info.bounds {
                    Some((_, hi)) => KindInfo::Bounds(self.member_info_hi(Some(a), hi)),
                    None if info.rhs != ERROR => KindInfo::Alias(info.rhs),
                    None => KindInfo::Unknown,
                })
            }
            Type::Member(prefix, name) => {
                self.prefix_reads(prefix)?;
                let alias = self.type_member_alias(prefix, name);
                if self.member_completing(t) {
                    return Ok(KindInfo::Bounds(alias.map_or(ANY, |a| self.declared_clause(a))));
                }
                Ok(match self.type_member(prefix, name) {
                    Some(super::members::MemberInfo::Alias(a)) => KindInfo::Alias(a),
                    Some(super::members::MemberInfo::Bounds(_, hi)) => KindInfo::Bounds(self.member_info_hi(alias, hi)),
                    None => KindInfo::Unknown,
                })
            }
            // `AppliedType.superType` of a non-class constructor: its own `superType` applied (`applyIfParameterized`).
            Type::AppParam(..) | Type::AppMember(..) | Type::AppVar(..) => {
                let (head, args) = match self.types.get(t) {
                    Type::AppParam(f, args) => (self.types.param(f), args),
                    Type::AppMember(m, args) => (m, args),
                    Type::AppVar(v, args) => (self.types.mk(Type::Var(v)), args),
                    _ => unreachable!(),
                };
                let args: Vec<TypeId> = self.types.items(args).to_vec();
                match self.kind_info(head)? {
                    KindInfo::Bounds(hi) | KindInfo::Alias(hi) => self.apply_if_parameterized(hi, &args),
                    KindInfo::Unknown => Ok(KindInfo::Unknown),
                }
            }
            Type::Var(v) => match self.tvars[v].origin() {
                Some(p) => {
                    let origin = self.types.param(p);
                    self.kind_info(origin)
                }
                None => Ok(KindInfo::Bounds(self.tvars[v].declared().1)),
            },
            _ => Ok(KindInfo::Unknown),
        }
    }

    /// `applyIfParameterized(args)` (`core/TypeApplications.scala` 446-447) of a constructor's `superType` `hi`, with its
    /// reads: itself where it takes no parameters (`typeParams` empty); else `appliedTo` (374-441): a lambda
    /// instantiated, a class, a parameter, a member, an alias, a variable applied (`AppliedType(self, args)`); another
    /// proxy, an application or a wildcard's or annotated type's parent, its own `superType` applied in turn (the
    /// `AppliedType(self, args)` dotty makes reads that `superType` first).
    fn apply_if_parameterized(&mut self, hi: TypeId, args: &[TypeId]) -> Result<KindInfo, TParamId> {
        let hi = self.deref_keeping(hi);
        if hi == ERROR {
            return Ok(KindInfo::Unknown);
        }
        if let Type::Lambda(..) = self.types.get(hi) {
            let applied = self.types.apply_ctor(hi, args);
            return Ok(if applied == ERROR { KindInfo::Unknown } else { KindInfo::Bounds(applied) });
        }
        // `typeParams.nonEmpty`: the reference's own query, which guards its own walk (`type_params_reads`), so the
        // helper's call is no cycle of the reference's information.
        if self.type_params_reads(hi)?.is_empty() {
            return Ok(KindInfo::Bounds(hi));
        }
        match self.types.get(hi) {
            Type::Ctor(_) | Type::Param(_) | Type::Member(..) | Type::Decl(_) | Type::Alias(..) | Type::Var(_) => {
                let applied = self.types.apply_ctor(hi, args);
                Ok(if applied == ERROR { KindInfo::Unknown } else { KindInfo::Bounds(applied) })
            }
            // An application's own `superType`, applied in turn: the walk keeps a bound that reaches itself.
            Type::AppParam(..) | Type::AppMember(..) | Type::AppVar(..) => {
                if !self.kind_walk_enter(hi) {
                    return Ok(KindInfo::Unknown);
                }
                let r = match self.kind_info(hi) {
                    Ok(KindInfo::Bounds(s)) | Ok(KindInfo::Alias(s)) => self.apply_if_parameterized(s, args),
                    Ok(KindInfo::Unknown) => Ok(KindInfo::Unknown),
                    Err(e) => Err(e),
                };
                self.kind_walk.pop();
                r
            }
            Type::BoundedWild(_, u) | Type::Annotated(u) => self.apply_if_parameterized(u, args),
            _ => Ok(KindInfo::Unknown),
        }
    }

    /// A parameter's upper bound as its information holds it (`boundsFromParams`, `core/Types.scala` 4598-4620): one
    /// that did not resolve the error; one taking a clause the parser keeps by its arity and variances alone (`F[_]`,
    /// `F[+X]`: none of the clause's parameters bounded or of a kind of its own, `Parser::parse_type_params`, the
    /// loader's open bound) `[X..] =>> Any` over parameters each `TypeBounds.empty`, made once (`kind_clauses`); else
    /// the bound.
    fn param_info_hi(&mut self, p: TParamId) -> TypeId {
        let info = self.syms.tparam(p);
        let (upper, arity) = (info.upper, info.arity as usize);
        if arity == 0 || upper != ANY {
            return upper;
        }
        if let Some(&t) = self.kind_clauses.get(&p) {
            return t;
        }
        let variances = self.syms.tparam(p).hk_variances.clone();
        let ps: Vec<TypeId> = (0..arity)
            .map(|k| {
                let q = self.syms.new_tparam(crate::names::WILDCARD, variances.get(k).copied().unwrap_or(0));
                self.types.param(q)
            })
            .collect();
        let l = self.types.list(&ps);
        let t = self.types.mk(Type::Lambda(l, ANY));
        self.kind_clauses.insert(p, t);
        t
    }

    /// A member's upper bound as its information holds it: one declared without a lambda bound (`type F[G[_]]`)
    /// `[X..] =>> Any` over its declared parameters (`declared_clause`), each with its own information; else the bound.
    fn member_info_hi(&mut self, alias: Option<AliasId>, hi: TypeId) -> TypeId {
        match alias {
            Some(a) if hi == ANY => self.declared_clause(a),
            _ => hi,
        }
    }

    /// `boundsFromParams(tparamSyms, TypeBounds.empty)`'s upper bound over the declared parameters of the member `a`
    /// (`completerTypeParams`): `[X..] =>> Any`, `Any` where it declares none.
    fn declared_clause(&mut self, a: AliasId) -> TypeId {
        let ps: Vec<TypeId> = self.syms.aliases[a.idx()].tparams.clone().into_iter().map(|q| self.types.param(q)).collect();
        if ps.is_empty() {
            return ANY;
        }
        let l = self.types.list(&ps);
        self.types.mk(Type::Lambda(l, ANY))
    }

    /// The `dummyInfo1` of the binder `p` whose bounds are being typed: `[X..] =>> Any` over its own clause's parameters
    /// (`ClauseCompletion::own`), `TypeBounds.empty` where it has no clause of its own.
    fn binder_own_clause(&self, p: TParamId) -> TypeId {
        self.binder_completing
            .iter()
            .rev()
            .find_map(|c| c.ids.iter().position(|&q| q == p).map(|i| c.own[i]))
            .filter(|&t| t != crate::tir::NO_TYPE)
            .unwrap_or(ANY)
    }

    /// `AppliedType.superType` of an application of a non-class constructor (`kind_info`'s application arm), none where
    /// it is unknown or its reads meet a cycle.
    pub(super) fn applied_super_type(&mut self, t: TypeId) -> Option<TypeId> {
        match self.kind_info(t) {
            Ok(KindInfo::Bounds(s)) | Ok(KindInfo::Alias(s)) => Some(s),
            _ => None,
        }
    }

    /// `kind_dealias` for the typer's other readers.
    pub(super) fn alias_kept_expansion(&mut self, t: TypeId) -> Option<TypeId> {
        self.kind_dealias(t)
    }

    /// `dealias` of an alias kept by name (`Type::Alias`: a curried alias, a match alias): its expansion, none where it
    /// has none or is itself.
    fn kind_dealias(&mut self, t: TypeId) -> Option<TypeId> {
        let Type::Alias(a, args) = self.types.get(t) else { return None };
        self.alias_expansion(a, args).filter(|&e| e != t)
    }

    /// Enters the reference `t` on the kind queries' walk (`Worker::kind_walk`): false where it is on it already, a
    /// bound reaching itself (dotty's `CyclicReference`), which reads as no information.
    fn kind_walk_enter(&mut self, t: TypeId) -> bool {
        if self.kind_walk.contains(&t) {
            return false;
        }
        self.kind_walk.push(t);
        true
    }

    /// `hkResult` (`core/TypeApplications.scala` 269-282) with its reads, the result type where one exists: 269 the
    /// dealiased type; 270-273 a `TypeRef`: `AnyKind` itself, a class none, else `info.hkResult` (a `TypeBounds` its
    /// `superType`, `hi`, 2193-2196, recursively); 274-275 an `AppliedType`: a class constructor none, else
    /// `superType.hkResult`; 276 an `HKTypeLambda` its result; 277 a singleton, a refinement, a recursive type none; 278 a
    /// wildcard its bounds'; 279-282 a variable its instance, another proxy its `superType`'s, else none. A class
    /// constructor as an argument is eta-expanded (`adaptType`): its class applied to its parameters.
    pub(super) fn hk_result_reads(&mut self, t: TypeId) -> Result<Option<TypeId>, TParamId> {
        let t = self.deref_keeping(t);
        match self.types.get(t) {
            Type::Class(c, _) | Type::Ctor(c) if self.is_scala_class(c, "AnyKind") => Ok(Some(t)),
            // An unresolved bound, any kind as `AnyKind` is (`is_any_kind`).
            Type::Error => Ok(Some(t)),
            Type::Ctor(c) => {
                self.class_tparam_infos(c);
                let ps: Vec<TypeId> = self.syms.class(c).tparams.clone().into_iter().map(|q| self.types.param(q)).collect();
                if ps.is_empty() {
                    return Ok(None);
                }
                Ok(Some(self.types.class(c, &ps)))
            }
            Type::Lambda(_, body) => Ok(Some(body)),
            // 269: `dealias`, an alias kept by name (a curried one, `Const[T]` of `[X] =>> T`, a match alias) expanded.
            Type::Alias(..) => match self.kind_dealias(t) {
                Some(e) => self.hk_result_reads(e),
                None => Ok(None),
            },
            Type::Param(_) | Type::Decl(_) | Type::Member(..) | Type::AppParam(..) | Type::AppMember(..) | Type::Var(_) | Type::AppVar(..) => {
                if !self.kind_walk_enter(t) {
                    return Ok(None);
                }
                let r = match self.kind_info(t) {
                    Ok(KindInfo::Bounds(hi)) | Ok(KindInfo::Alias(hi)) => self.hk_result_reads(hi),
                    Ok(KindInfo::Unknown) => Ok(None),
                    Err(e) => Err(e),
                };
                self.kind_walk.pop();
                r
            }
            Type::BoundedWild(_, hi) => self.hk_result_reads(hi),
            Type::Annotated(u) => self.hk_result_reads(u),
            _ => Ok(None),
        }
    }

    /// `hasSameKindAs(other)` (`core/TypeApplications.scala` 290-305) with its reads, in its order (`other` the formal
    /// parameter itself, whose information is the `paramBounds` whose `hi` dotty passes): both `hkResult`s,
    /// `isAnyKind` of either (a symbol test); where the first exists, the second exists, the results' kinds, then
    /// `typeParams.corresponds(other.typeParams)(_.paramInfo.hasSameKindAs(_.paramInfo))`; else the second none. Each
    /// `&&` stops the reads after it.
    pub(super) fn has_same_kind_reads(&mut self, t: TypeId, other: TypeId) -> Result<bool, TParamId> {
        let self_result = self.hk_result_reads(t)?;
        let other_result = self.hk_result_reads(other)?;
        if self_result.is_some_and(|r| self.is_any_kind(r)) || other_result.is_some_and(|r| self.is_any_kind(r)) {
            return Ok(true);
        }
        let Some(self_result) = self_result else {
            return Ok(other_result.is_none());
        };
        let Some(other_result) = other_result else {
            return Ok(false);
        };
        if !self.has_same_kind_reads(self_result, other_result)? {
            return Ok(false);
        }
        let own = self.type_params_reads(t)?;
        let others = self.type_params_reads(other)?;
        for (&a, &b) in own.iter().zip(&others) {
            if !self.has_same_kind_reads(a, b)? {
                return Ok(false);
            }
        }
        Ok(own.len() == others.len())
    }

    /// `isAnyKind` of a `hkResult`: the `TypeRef` of `AnyKind`.
    fn is_any_kind(&self, t: TypeId) -> bool {
        match self.types.get(t) {
            Type::Class(c, _) | Type::Ctor(c) => self.is_scala_class(c, "AnyKind"),
            Type::Error => true,
            _ => false,
        }
    }

    /// `typeParams` (`core/TypeApplications.scala` 170-213) with its reads, each parameter as `hasSameKindAs` reads its
    /// `paramInfo` (a parameter itself, whose information is its bounds; `Any` for a clause known by its arity, its
    /// infos `TypeBounds.empty`): 182-197 a `TypeRef`: a class's parameters, else its information's (a binder still under
    /// its completer by its clause, `completerTypeParams`); 199-201 an `AppliedType`: a class constructor none, else
    /// `superType.typeParams`; 204-205 an `HKTypeLambda` its own; 206-211 a singleton, a refinement, a recursive type
    /// none, a wildcard its bounds', another proxy its `superType`'s; else none. A class constructor as an argument
    /// (eta-expanded) its class's.
    fn type_params_reads(&mut self, t: TypeId) -> Result<Vec<TypeId>, TParamId> {
        let t = self.deref_keeping(t);
        match self.types.get(t) {
            Type::Lambda(l, _) => Ok(self.types.items(l).to_vec()),
            Type::Alias(..) => match self.kind_dealias(t) {
                Some(e) => self.type_params_reads(e),
                None => Ok(Vec::new()),
            },
            Type::Ctor(c) => {
                self.class_tparam_infos(c);
                Ok(self.syms.class(c).tparams.clone().into_iter().map(|q| self.types.param(q)).collect())
            }
            Type::Param(_) | Type::Decl(_) | Type::Member(..) | Type::AppParam(..) | Type::AppMember(..) | Type::Var(_) | Type::AppVar(..) => {
                if !self.kind_walk_enter(t) {
                    return Ok(Vec::new());
                }
                let r = match self.kind_info(t) {
                    Ok(KindInfo::Bounds(hi)) | Ok(KindInfo::Alias(hi)) => self.type_params_reads(hi),
                    Ok(KindInfo::Unknown) => Ok(Vec::new()),
                    Err(e) => Err(e),
                };
                self.kind_walk.pop();
                r
            }
            Type::BoundedWild(_, hi) => self.type_params_reads(hi),
            Type::Annotated(u) => self.type_params_reads(u),
            _ => Ok(Vec::new()),
        }
    }

    /// `hasSimpleKind` (`core/TypeApplications.scala` 246-250) with its reads: `typeParams.isEmpty && !hasAnyKind`, else
    /// the dealiased type's where `dealias` changed it (an alias's information followed).
    pub(super) fn has_simple_kind_reads(&mut self, t: TypeId) -> Result<bool, TParamId> {
        let t = self.deref_keeping(t);
        if self.type_params_reads(t)?.is_empty() && !self.has_any_kind_reads(t)? {
            return Ok(true);
        }
        let dealiased = match self.types.get(t) {
            Type::Member(..) | Type::Decl(_) => match self.kind_info(t)? {
                KindInfo::Alias(a) => a,
                _ => t,
            },
            Type::Alias(..) => self.kind_dealias(t).unwrap_or(t),
            _ => t,
        };
        if dealiased != t && self.kind_walk_enter(t) {
            let r = self.has_simple_kind_reads(dealiased);
            self.kind_walk.pop();
            return r;
        }
        Ok(false)
    }

    /// `hasAnyKind`'s loop (`core/Types.scala` 371-381): a class `TypeRef` is `AnyKind` or not; another `TypeRef` its
    /// `translucentSuperType` (3034-3041: an alias's alias, bounds' `hi`); another proxy its `underlying` (an
    /// application its constructor, 4740; a lambda its result, 3975; an annotated, refined or recursive type its parent;
    /// a wildcard's bounds `hi`); else false.
    fn has_any_kind_reads(&mut self, t: TypeId) -> Result<bool, TParamId> {
        let t = self.deref_keeping(t);
        match self.types.get(t) {
            Type::Class(c, _) | Type::Ctor(c) => Ok(self.is_scala_class(c, "AnyKind")),
            Type::Param(_) | Type::Decl(_) | Type::Member(..) => {
                if !self.kind_walk_enter(t) {
                    return Ok(false);
                }
                let r = match self.kind_info(t) {
                    Ok(KindInfo::Bounds(hi)) | Ok(KindInfo::Alias(hi)) => self.has_any_kind_reads(hi),
                    Ok(_) => Ok(false),
                    Err(e) => Err(e),
                };
                self.kind_walk.pop();
                r
            }
            Type::AppParam(f, _) => {
                let head = self.types.param(f);
                self.has_any_kind_reads(head)
            }
            Type::AppMember(m, _) => self.has_any_kind_reads(m),
            // An open variable's `underlying`, its `origin`'s information (`TypeVar.underlying`); an application of one its
            // constructor.
            Type::Var(_) => {
                if !self.kind_walk_enter(t) {
                    return Ok(false);
                }
                let r = match self.kind_info(t) {
                    Ok(KindInfo::Bounds(hi)) | Ok(KindInfo::Alias(hi)) => self.has_any_kind_reads(hi),
                    Ok(KindInfo::Unknown) => Ok(false),
                    Err(e) => Err(e),
                };
                self.kind_walk.pop();
                r
            }
            Type::AppVar(v, _) => {
                let head = self.types.mk(Type::Var(v));
                self.has_any_kind_reads(head)
            }
            Type::Alias(..) => match self.kind_dealias(t) {
                Some(e) => self.has_any_kind_reads(e),
                None => Ok(false),
            },
            Type::Lambda(_, body) => self.has_any_kind_reads(body),
            Type::Annotated(u) | Type::Refined(u, _) | Type::Rec(u) | Type::BoundedWild(_, u) => self.has_any_kind_reads(u),
            _ => Ok(false),
        }
    }

    /// `Checking.preCheckKind(arg, paramBounds)` (`typer/Checking.scala` 189-193) with its reads, in its order:
    /// `arg.tpe.widen.isRef(defn.NothingClass) || !paramBounds.exists || arg.tpe.hasSameKindAs(paramBounds.bounds.hi)`,
    /// `hi` the formal's upper bound (`None`: an unbounded wildcard argument's `NoType`). The value is whether the
    /// argument is kept (an argument refused is an error tree, read no further).
    pub(super) fn pre_check_kind_reads(&mut self, arg: TypeId, hi: Option<TypeId>) -> Result<bool, TParamId> {
        // An error tree's own verdict says nothing (`HideNonSensicalMessages` hides a message of the error type).
        if self.types.contains_error(arg) {
            return Ok(true);
        }
        if self.widened_is_ref_reads(arg, RefSym::Nothing)? {
            return Ok(true);
        }
        match hi {
            Some(hi) => self.has_same_kind_reads(arg, hi),
            None => Ok(true),
        }
    }

    /// `preCheckKind` of the argument `arg` for the parameter `p`, outside a binder's completion (where its reads meet no
    /// cycle): the widened argument a reference to `Nothing`, then `hasSameKindAs` of the parameter's information
    /// (`has_same_kind`, which keeps an argument that did not resolve, teq's error tree).
    pub(super) fn pre_check_kind(&mut self, arg: TypeId, p: TParamId) -> bool {
        self.widened_is_ref_reads(arg, RefSym::Nothing).unwrap_or(false) || self.has_same_kind(arg, p)
    }

    /// `typedAppliedTypeTree`'s `tpt1.tpe.dealiasKeepAnnotsAndOpaques` (`typer/Typer.scala` 2725), before the arguments
    /// are typed: a non-class constructor's information (`Type.dealias`'s `tp.info`): a binder's; a member's prefix's.
    pub(super) fn dealias_reads(&mut self, ctor: TypeId) -> Result<(), TParamId> {
        match self.types.get(ctor) {
            Type::Param(f) => self.binder_info(f),
            Type::Member(prefix, _) => self.prefix_reads(prefix),
            _ => Ok(()),
        }
    }

    /// `Typer.adaptType`'s `etaExpand` (`typer/Typer.scala` 5051-5068, `TypeApplications.etaExpand`'s `self.dealias`) of
    /// a type expression outside a constructor's place: a bare binder of a clause in completion whose `typeParamSymbols`
    /// are its completer's (`completerTypeParams`: a clause of its own, the binder `NotStarted` or `Touched`) has its
    /// information read.
    pub(super) fn eta_reads(&mut self, t: TypeId) -> Result<(), TParamId> {
        let Type::Param(p) = self.types.get(t) else {
            return Ok(());
        };
        use super::BinderState::*;
        for at in (0..self.binder_completing.len()).rev() {
            let c = &self.binder_completing[at];
            if let Some(i) = c.ids.iter().position(|&q| q == p) {
                if !matches!(c.states[i], NotStarted | Touched) {
                    return Ok(());
                }
                let hi = c.bounds[i].1;
                let file = c.file;
                if hi == crate::ast::NO_BOUND || self.ast(file).notes.own_clause_bounds.binary_search(&hi).is_err() {
                    return Ok(());
                }
                return self.binder_info(p);
            }
        }
        Ok(())
    }

    /// `desugar.refinedTypeToClass`' `stripToCore` (`ast/Desugar.scala` 2500-2507) of a refinement's parent: an
    /// application or a class itself, another proxy's `underlying` (a binder's information, then its upper bound),
    /// an intersection both sides.
    pub(super) fn strip_to_core_reads(&mut self, t: TypeId) -> Result<(), TParamId> {
        let t = self.deref_keeping(t);
        match self.types.get(t) {
            Type::Param(p) => {
                self.binder_info(p)?;
                let upper = self.syms.tparam(p).upper;
                if upper == ANY || upper == ERROR || self.binder_state(p).is_some_and(|s| s != super::BinderState::Done) {
                    return Ok(());
                }
                self.strip_to_core_reads(upper)
            }
            Type::Inter(a, b) => {
                self.strip_to_core_reads(a)?;
                self.strip_to_core_reads(b)
            }
            Type::Annotated(u) | Type::Rec(u) => self.strip_to_core_reads(u),
            _ => Ok(()),
        }
    }

    /// `SelectionProto.hasUnknownMembers` (`typer/ProtoTypes.scala` 236-253) of a selection's qualifier, with its reads
    /// (`has_unknown_members` over the typer's types, `KnownTypes`): a binder's `info` read where dotty reads `tp.info`.
    pub(super) fn has_unknown_members_reads(&mut self, t: TypeId) -> Result<bool, TParamId> {
        let mut visiting = Vec::new();
        has_unknown_members(&mut KnownTypes { w: self }, t, &mut visiting)
    }

    /// A non-class reference's information as `hasUnknownMembers` reads it (`tp.info`: `loBound`, `hiBound`), with its
    /// read: a binder's (`binder_info`; one whose bounds are being typed its `dummyInfo1`), an abstract member's or an
    /// alias's (a member completing its provisional bounds, `typeSig`'s).
    fn reference_bounds(&mut self, t: TypeId) -> Result<(TypeId, TypeId), TParamId> {
        Ok(match self.types.get(t) {
            Type::Param(p) => {
                self.binder_info(p)?;
                if self.binder_state(p).is_some_and(|s| s != super::BinderState::Done) {
                    (NOTHING, self.binder_own_clause(p))
                } else {
                    (self.syms.tparam(p).lower, self.param_info_hi(p))
                }
            }
            Type::Decl(a) => {
                if self.member_completing(t) {
                    return Ok((NOTHING, self.declared_clause(a)));
                }
                self.complete_alias(a);
                let info = &self.syms.aliases[a.idx()];
                match info.bounds {
                    Some((lo, hi)) => (lo, hi),
                    None => (info.rhs, info.rhs),
                }
            }
            Type::Member(prefix, name) => {
                let alias = self.type_member_alias(prefix, name);
                if self.member_completing(t) {
                    return Ok((NOTHING, alias.map_or(ANY, |a| self.declared_clause(a))));
                }
                match self.type_member(prefix, name) {
                    Some(super::members::MemberInfo::Alias(a)) => (a, a),
                    Some(super::members::MemberInfo::Bounds(lo, hi)) => (lo, hi),
                    None => (NOTHING, ANY),
                }
            }
            _ => (NOTHING, ANY),
        })
    }

    /// `Type.isProvisional`, `testProvisional` (`core/Types.scala` 118-152), over the typer's types.
    pub(super) fn is_provisional(&mut self, t: TypeId) -> bool {
        let mut seen: Vec<TypeId> = Vec::new();
        self.test_provisional(t, &mut seen)
    }

    /// `testProvisional`'s `test`: a reference met again false (dotty marks it `mightBeProvisional = false` before it
    /// recurses, which breaks cycles). A parameter: a binder whose completion is under way true (its `dummyInfo1`'s
    /// `Provisional`, or its completer's `LazyType`), else its bounds' (`ParamRef.underlying`; a parameter `TypeRef`'s
    /// `TypeBounds`, not static, its prefix none). A member or a declaration (`TypeRef`): `alias_provisional`. A selected
    /// term (`TermRef`): not static, its prefix's. An application: its constructor's or an argument's (`fold`). A class:
    /// its arguments', a nested class's prefix. A variable: not permanently instantiated true (`permanent_instance`), else
    /// its permanent instance's. Else the parts' (`foldOver`).
    fn test_provisional(&mut self, t: TypeId, seen: &mut Vec<TypeId>) -> bool {
        if seen.contains(&t) {
            return false;
        }
        seen.push(t);
        match self.types.get(t) {
            Type::Param(p) => {
                if self.binder_state(p).is_some_and(|s| s != super::BinderState::Done) {
                    return true;
                }
                let (lo, hi) = (self.syms.tparam(p).lower, self.syms.tparam(p).upper);
                self.test_provisional(lo, seen) || self.test_provisional(hi, seen)
            }
            Type::Decl(a) => self.alias_provisional(a, None, seen),
            Type::Member(prefix, name) => match self.type_member_alias(prefix, name) {
                Some(a) => self.alias_provisional(a, Some(prefix), seen),
                // A member no declaration of the prefix's classes names (a refinement's): not static, its prefix's.
                None => self.test_provisional(prefix, seen),
            },
            Type::Select(prefix, s) => {
                let owner = self.syms.sym(s).owner;
                !self.is_static_owner(owner) && self.test_provisional(prefix, seen)
            }
            // `!t.isPermanentlyInstantiated || test(t.permanentInst)`: an instance a held mark or an open attempt can
            // still undo is provisional whatever it is (`instanceOpt` sees it; `hasUnknownMembers` reads that one).
            Type::Var(v) => match self.permanent_instance(v) {
                Some(inst) => self.test_provisional(inst, seen),
                None => true,
            },
            // An application of a variable: its constructor's or an argument's (`fold`).
            Type::AppVar(v, args) => {
                let head = self.types.mk(Type::Var(v));
                let args = self.types.items(args).to_vec();
                self.test_provisional(head, seen) || args.into_iter().any(|x| self.test_provisional(x, seen))
            }
            Type::AppParam(f, args) => {
                let head = self.types.param(f);
                let args = self.types.items(args).to_vec();
                self.test_provisional(head, seen) || args.into_iter().any(|x| self.test_provisional(x, seen))
            }
            Type::AppMember(m, args) => {
                let args = self.types.items(args).to_vec();
                self.test_provisional(m, seen) || args.into_iter().any(|x| self.test_provisional(x, seen))
            }
            Type::Alias(a, args) => {
                let args = self.types.items(args).to_vec();
                self.alias_provisional(a, None, seen) || args.into_iter().any(|x| self.test_provisional(x, seen))
            }
            Type::Nested(prefix, class) => self.test_provisional(prefix, seen) || self.test_provisional(class, seen),
            _ => {
                let mut parts = Vec::new();
                self.types.parts(t, &mut parts);
                parts.into_iter().any(|x| self.test_provisional(x, seen))
            }
        }
    }

    /// `testProvisional` of a `TypeRef` to the member `a`: its symbol `Provisional` (its `typeSig` under way) true; a
    /// static one (owned by a package or a static object) nothing more; else its prefix's, or its information's: not
    /// completed (`LazyType`) true, an alias its target's, bounds either side's. The member's own information, as
    /// declared (`infoOrCompleter`, no completion forced).
    fn alias_provisional(&mut self, a: AliasId, prefix: Option<TypeId>, seen: &mut Vec<TypeId>) -> bool {
        let state = self.syms.alias(a).state().get();
        if state == Completion::InProgress {
            return true;
        }
        if self.is_static_owner(self.syms.aliases[a.idx()].owner) {
            return false;
        }
        if prefix.is_some_and(|p| self.test_provisional(p, seen)) {
            return true;
        }
        if state != Completion::Done {
            return true;
        }
        let info = &self.syms.aliases[a.idx()];
        match info.bounds {
            Some((lo, hi)) => self.test_provisional(lo, seen) || self.test_provisional(hi, seen),
            None => {
                let rhs = info.rhs;
                self.test_provisional(rhs, seen)
            }
        }
    }

    /// `completeAnnotations`' step for one annotation of the binder `r`: `typedAnnotation` (a Java class's arguments
    /// checked first, `Namer.addAnnotations`' `checkNamedArgumentForJavaAnnotation`), then `checkAnnotArgs`,
    /// `checkAnnotClass` and `checkAnnotApplicable`. A `@nowarn`'s registration is `register_suppressions`', once
    /// typing is over.
    fn complete_binder_annotation(&mut self, file: FileId, r: &ast::BinderAnnot) {
        let instance = r.annot.instance;
        let span = self.ast(file).expr_span(instance);
        let mark = self.diags.items.len();
        let frame = self.var_frame_begin();
        let pending = self.attempts.pending_len();
        let (class, typed) = match self.java_annotation_class(instance) {
            Some(c) => {
                self.check_named_java_annotation_args(file, c, instance);
                (Some(c), self.type_java_annotation(c, instance))
            }
            None => {
                let (te, ty) = self.type_expr(instance, None);
                let ty = self.dealias(ty);
                let class = match self.types.get(ty) {
                    Type::Class(c, _) => Some(c),
                    _ => None,
                };
                (class, Some(te))
            }
        };
        if self.attempts.pending_len() > pending {
            self.flush_pending_from(pending);
        }
        self.var_frame_end(frame);
        // An annotation whose typing reported an error at it is checked no further (`checkAnnotClass`' `isError`); a
        // check's message at the same place would be hidden by scalac's reporter (`UniqueMessagePositions`).
        let failed = self.diags.items[mark..].iter().any(|d| !d.is_warning && d.file == file && d.span.start >= span.start && d.span.end <= span.end);
        let Some(class) = class else { return };
        if !failed {
            if typed.is_some() {
                self.check_annot_args(file, class, instance);
            }
            self.check_annot_class(class, span);
            // `checkAnnotApplicable`: a binder is a type, never a real method. Its second error (not accessed
            // statically) stands at the same place at the same level, which scalac's reporter hides; teq's hides an
            // error only by the same error.
            if self.class_path(class) == "scala.main" {
                let name = self.binder_name(file, r);
                let msg = format!("main annotation cannot be applied to type {}", self.name_str(name));
                self.error(span, msg);
            }
        }
        // The typed tree of the annotation installed on the binder, which the pickle writes (`prog.capture`).
        if let (false, Some(te)) = (failed, typed) {
            if let Some(c) = self.prog.capture.as_deref_mut().filter(|c| c.annotates(file)) {
                c.annotations.insert((file, instance), te);
            }
        }
        // `if Annotations.annotClass(annot1) == defn.NowarnAnnot then registerNowarn(annot1, mdef)`: by the class the
        // annotation resolved to (an import's rename is the class), from its argument typed here.
        if self.class_path(class) == "scala.annotation.nowarn" {
            self.binder_nowarn(file, r, span);
        }
    }

    /// `Typer.registerNowarn` of a binder's `@nowarn`: its filter the constant string of its first argument
    /// (`argumentConstantString(0)`, typed in the binder's context), `""` where none is written (the default getter's),
    /// else `none` with the warning at the argument; over the binder's `TypeDef` (`Worker::binder_nowarns`).
    fn binder_nowarn(&mut self, file: FileId, r: &ast::BinderAnnot, annot: Span) {
        let arg = self.annotation_argument(file, r.annot.instance);
        let t_string = self.b.t_string;
        let (conf, constant) = match arg {
            None => (String::new(), true),
            // `argumentConstant`: `stripCast(arg).tpe.widenTermRefExpr.normalized`, the operand of a cast as the
            // typed tree types it (a selection's qualifier: no type expected).
            Some(arg) => {
                let (operand, expected) = match self.strip_cast(file, arg) {
                    Some(operand) => (operand, None),
                    None => (arg, Some(t_string)),
                };
                let ty = self.annot_argument_type(file, operand, expected);
                match self.constant_of_type(ty) {
                    Some(crate::types::LitVal::Str(s)) => (self.name_str(s), true),
                    _ => ("none".to_string(), false),
                }
            }
        };
        let arg = arg.map(|a| self.ast(file).expr_span(a));
        let n = super::BinderNowarn { file, annot, range: r.span, conf, arg, constant };
        if !self.binder_nowarns.contains(&n) {
            self.binder_nowarns.push(n);
        }
    }

    /// `Annotation.argument(0)`: the first term argument of the annotation, its argument lists flattened
    /// (`allTermArguments`), a named one as written.
    fn annotation_argument(&self, file: FileId, instance: ast::ExprId) -> Option<ast::ExprId> {
        let ast = self.ast(file);
        let mut lists = Vec::new();
        let mut e = instance;
        loop {
            match ast.expr(e) {
                ast::Expr::Apply(f, l) => {
                    lists.push(l);
                    e = f;
                }
                ast::Expr::New(_, l) => {
                    lists.push(l);
                    break;
                }
                _ => return None,
            }
        }
        lists.iter().rev().flat_map(|&l| ast.expr_list(l).iter().copied()).next()
    }

    /// `Checking.checkNonCyclic` (`typer/Checking.scala` 425-437) of the binder `i` of the clause `at`, its bounds `lower`
    /// and `upper`: `CheckNonCyclicMap.checkInfo` over them; a cycle is `IllegalCyclicTypeReference` (E140) at the
    /// binder, whose information is then the error type.
    fn check_non_cyclic(&mut self, at: usize, i: usize, lower: TypeId, upper: TypeId) -> Result<(), ()> {
        let sym = self.binder_completing[at].ids[i];
        let mut m = NonCyclic { sym, locked: Vec::new(), cycle_ok: false, nested_cycle_ok: false, variance: 1, where_: "", last_checked: NOTHING };
        if self.nc_check_info(&mut m, lower, upper).is_ok() {
            return Ok(());
        }
        let (file, keys) = (self.binder_completing[at].file, self.binder_completing[at].keys);
        let named = self.ast(file).notes.binder_names.iter().find(|&&(cl, k, _)| k as usize == i && keys.contains(&Some(cl))).map(|&(_, _, s)| s);
        let bound = self.binder_completing[at].bounds[i];
        let span = named.unwrap_or_else(|| {
            let t = if bound.1 != crate::ast::NO_BOUND { bound.1 } else { bound.0 };
            self.ast(file).ty_spans[t.idx()]
        });
        let msg = format!(
            "illegal cyclic type reference: {} {} of type {} refers back to the type itself",
            m.where_,
            self.show(m.last_checked),
            self.type_name_ref(self.syms.tparam(sym).name)
        );
        let file_was = std::mem::replace(&mut self.env.file, file);
        self.error(span, msg);
        self.env.file = file_was;
        Err(())
    }

    /// `checkInfo` (296-310) of bounds `>: lo <: hi`: the lower part at variance -1, the upper one with nested cycles
    /// allowed (`checkUpper`).
    fn nc_check_info(&mut self, m: &mut NonCyclic, lo: TypeId, hi: TypeId) -> Result<(), ()> {
        let saved = m.variance;
        m.variance = -1;
        let r = self.nc_check_part(m, lo, "lower bound");
        m.variance = saved;
        r?;
        self.nc_check_upper(m, hi, "upper bound")
    }

    /// `checkInfo` of a `TypeAlias(alias)`: the alias as its lower part at variance -1, then as its upper one.
    fn nc_check_alias(&mut self, m: &mut NonCyclic, alias: TypeId) -> Result<(), ()> {
        let saved = m.variance;
        m.variance = -1;
        let r = self.nc_check_part(m, alias, "alias");
        m.variance = saved;
        r?;
        self.nc_check_upper(m, alias, "alias")
    }

    /// `checkPart`: the part checked, `where` and `lastChecked` set after it whatever came of it (`finally`).
    fn nc_check_part(&mut self, m: &mut NonCyclic, t: TypeId, w: &'static str) -> Result<(), ()> {
        let r = self.nc_apply(m, t);
        m.where_ = w;
        m.last_checked = t;
        r
    }

    /// `checkUpper`: `checkPart` with nested cycles allowed.
    fn nc_check_upper(&mut self, m: &mut NonCyclic, t: TypeId, w: &'static str) -> Result<(), ()> {
        let saved = m.nested_cycle_ok;
        m.nested_cycle_ok = true;
        let r = self.nc_check_part(m, t, w);
        m.nested_cycle_ok = saved;
        r
    }

    /// `apply(tp, cycleOK, nestedCycleOK)` (313-323).
    fn nc_apply_with(&mut self, m: &mut NonCyclic, t: TypeId, cycle_ok: bool, nested_cycle_ok: bool) -> Result<(), ()> {
        let saved = (m.cycle_ok, m.nested_cycle_ok);
        m.cycle_ok = cycle_ok;
        m.nested_cycle_ok = nested_cycle_ok;
        let r = self.nc_apply(m, t);
        (m.cycle_ok, m.nested_cycle_ok) = saved;
        r
    }

    /// `CheckNonCyclicMap.apply` (325-378): a path's information then its prefix (`TermRef`), a refinement's parent and
    /// its members' infos at the nested flag, a recursive type's parent, a reference with an interesting prefix
    /// explored (`TypeRef`), every other type mapped over (`TypeMap.mapOver`).
    fn nc_apply(&mut self, m: &mut NonCyclic, t: TypeId) -> Result<(), ()> {
        let t = self.deref_keeping(t);
        match self.types.get(t) {
            Type::Term(_) | Type::Select(..) => {
                let underlying = self.path_underlying(t);
                if underlying != t {
                    self.nc_apply(m, underlying)?;
                }
                match self.types.get(t) {
                    Type::Select(prefix, _) => self.nc_map_prefix(m, prefix),
                    _ => Ok(()),
                }
            }
            Type::Refined(parent, r) => {
                self.nc_apply(m, parent)?;
                // `this(rinfo, nestedCycleOK, nestedCycleOK)`.
                let saved = (m.cycle_ok, m.nested_cycle_ok);
                m.cycle_ok = m.nested_cycle_ok;
                let r = self.nc_refinement_info(m, r);
                (m.cycle_ok, m.nested_cycle_ok) = saved;
                r
            }
            Type::Rec(parent) => self.nc_apply(m, parent),
            Type::Param(q) => {
                let r = self.nc_binder_ref(m, t, q);
                if r.is_err() && m.cycle_ok {
                    return Ok(());
                }
                r
            }
            Type::Member(prefix, name) => {
                let r = self.nc_member_ref(m, t, prefix, name);
                if r.is_err() && m.cycle_ok {
                    return Ok(());
                }
                r
            }
            Type::Decl(a) => {
                let r = self.nc_decl_ref(m, t, a);
                if r.is_err() && m.cycle_ok {
                    return Ok(());
                }
                r
            }
            _ => self.nc_map_over(m, t),
        }
    }

    /// `mapOver(rinfo)` of a refinement member's information, as one type: a `TypeBounds` its lower bound at
    /// `-variance`, its upper at the variance; a `TypeAlias` (`AliasingBounds`) at variance 0; a method's
    /// (`mapOverLambda`) type parameters' bounds (a `TypeBounds` at `-variance`), value parameters at `-variance`, result
    /// at the variance; a value's type at the variance.
    fn nc_refinement_info(&mut self, m: &mut NonCyclic, r: crate::types::RefineId) -> Result<(), ()> {
        let saved = m.variance;
        let at = |w: &mut Self, m: &mut NonCyclic, t: TypeId, variance: i8| {
            m.variance = variance;
            let r = w.nc_apply(m, t);
            m.variance = saved;
            r
        };
        match self.types.refinement(r) {
            crate::types::Refinement::Bounds(_, lo, hi) => {
                at(self, m, lo, -saved)?;
                at(self, m, hi, saved)
            }
            crate::types::Refinement::Alias(_, a) => at(self, m, a, 0),
            crate::types::Refinement::Val(_, _, v) => at(self, m, v, saved),
            crate::types::Refinement::Term(_, sym, l) => {
                let items = self.types.items(l).to_vec();
                let k = self.syms.sym(sym).sig.as_ref().map_or(0, |sig| sig.tparams.len());
                let Some((&result, rest)) = items.split_last() else {
                    return Ok(());
                };
                let bounds = (2 * k).min(rest.len());
                for pair in rest[..bounds].chunks(2) {
                    at(self, m, pair[0], saved)?;
                    if let Some(&hi) = pair.get(1) {
                        at(self, m, hi, -saved)?;
                    }
                }
                for &p in &rest[bounds..] {
                    at(self, m, p, -saved)?;
                }
                at(self, m, result, saved)
            }
        }
    }

    /// Whether an alias's or an object's abstract member's `TypeRef` has an interesting prefix (`isInteresting`): a local
    /// one's `NoPrefix`; a top-level one's package object, or an object's (a static module class of this run's
    /// sources, `pcls.span.exists && pcls.defRunId == ctx.runId`), an object's nested in a class its prefix's (a
    /// `ThisType`, the binder's owner no class); a class's member its `ThisType`, none. teq keeps no prefix on
    /// `Type::Alias` or `Type::Decl`: an object's member is taken through the object's `TermRef`.
    fn nc_alias_interesting(&self, a: AliasId) -> bool {
        let info = &self.syms.aliases[a.idx()];
        let source = !self.in_jar(info.file) && !self.is_body_file(info.file);
        match info.owner {
            Owner::Local => true,
            Owner::Package(_) => source,
            Owner::Class(c) => {
                let mut c = c;
                loop {
                    if self.syms.class(c).kind != ClassKind::Object {
                        return false;
                    }
                    match self.syms.class(c).owner {
                        Owner::Class(o) => c = o,
                        Owner::Package(_) => return source,
                        Owner::Local => return true,
                    }
                }
            }
        }
    }

    /// The `TypeRef` case for an application of a match alias kept by name (`this(tycon)` of `mapOver`'s
    /// `AppliedType`): where interesting, `locked` by the alias, its information (`MatchAlias`) checked.
    fn nc_alias_ref(&mut self, m: &mut NonCyclic, a: AliasId) -> Result<(), ()> {
        if !self.nc_alias_interesting(a) {
            return Ok(());
        }
        let key = self.types.mk(Type::Alias(a, crate::types::EMPTY_LIST));
        if m.locked.contains(&key) {
            return Err(());
        }
        m.locked.push(key);
        self.complete_alias(a);
        let (rhs, tparams) = (self.syms.aliases[a.idx()].rhs, self.syms.aliases[a.idx()].tparams.clone());
        let r = self.nc_check_match_alias(m, &tparams, rhs);
        m.locked.pop();
        r
    }

    /// The `TypeRef` case for an object's abstract member (`O.T`): where interesting, `locked`, its bounds checked.
    fn nc_decl_ref(&mut self, m: &mut NonCyclic, t: TypeId, a: AliasId) -> Result<(), ()> {
        if !self.nc_alias_interesting(a) {
            return Ok(());
        }
        if m.locked.contains(&t) {
            return Err(());
        }
        m.locked.push(t);
        self.complete_alias(a);
        let r = match self.syms.aliases[a.idx()].bounds {
            Some((lo, hi)) if lo != ERROR && hi != ERROR => self.nc_check_info(m, lo, hi),
            Some(_) => Ok(()),
            None => {
                let rhs = self.syms.aliases[a.idx()].rhs;
                self.nc_check_member_alias(m, rhs)
            }
        };
        m.locked.pop();
        r
    }

    /// `checkInfo` of a member's alias: a `MatchAlias` (a match type, or a lambda over one) by `nc_check_match_alias`,
    /// another `TypeAlias` by `nc_check_alias`.
    fn nc_check_member_alias(&mut self, m: &mut NonCyclic, alias: TypeId) -> Result<(), ()> {
        match self.types.get(alias) {
            Type::Match(..) => self.nc_check_match_alias(m, &[], alias),
            Type::Lambda(ps, body) if matches!(self.types.get(body), Type::Match(..)) => {
                let params: Vec<TParamId> = self.types.items(ps).iter().filter_map(|&p| match self.types.get(p) {
                    Type::Param(q) => Some(q),
                    _ => None,
                }).collect();
                self.nc_check_match_alias(m, &params, body)
            }
            _ => self.nc_check_alias(m, alias),
        }
    }

    /// `checkInfo`'s `MatchAlias(alias)` (303-304): `atVariance(0)(checkUpper(alias, "match"))`, the alias a lambda
    /// over `tparams` (`mapOverLambda`: their bounds at `-0`) whose result is the match type `rhs`.
    fn nc_check_match_alias(&mut self, m: &mut NonCyclic, tparams: &[TParamId], rhs: TypeId) -> Result<(), ()> {
        let saved = (m.variance, m.nested_cycle_ok);
        m.variance = 0;
        m.nested_cycle_ok = true;
        let mut r = Ok(());
        for &q in tparams {
            let (lo, hi) = (self.syms.tparam(q).lower, self.syms.tparam(q).upper);
            r = self.nc_apply(m, lo).and_then(|_| self.nc_apply(m, hi));
            if r.is_err() {
                break;
            }
        }
        if r.is_ok() {
            r = self.nc_apply(m, rhs);
        }
        m.where_ = "match";
        m.last_checked = if tparams.is_empty() {
            rhs
        } else {
            let ps: Vec<TypeId> = tparams.iter().map(|&q| self.types.param(q)).collect();
            let l = self.types.list(&ps);
            self.types.mk(Type::Lambda(l, rhs))
        };
        (m.variance, m.nested_cycle_ok) = saved;
        r
    }

    /// The `TypeRef` case for a binder (`NoPrefix`, interesting): locked or the one checked in `NoCompleter` a cycle; a
    /// reference whose information is `NoCompleter` not explored; another's information read (`binder_info`,
    /// completing it) and checked.
    fn nc_binder_ref(&mut self, m: &mut NonCyclic, t: TypeId, q: TParamId) -> Result<(), ()> {
        let state = self.binder_state(q);
        if m.locked.contains(&t) || (state == Some(super::BinderState::Checking) && q == m.sym) {
            return Err(());
        }
        if state == Some(super::BinderState::Checking) {
            return Ok(());
        }
        m.locked.push(t);
        let r = match self.binder_info(q) {
            Err(_) => Err(()),
            Ok(()) => {
                let (lo, hi) = match self.binder_state(q) {
                    Some(super::BinderState::Provisional) => (NOTHING, ANY),
                    _ => (self.syms.tparam(q).lower, self.syms.tparam(q).upper),
                };
                if lo == ERROR || hi == ERROR {
                    Ok(())
                } else {
                    self.nc_check_info(m, lo, hi)
                }
            }
        };
        m.locked.pop();
        r
    }

    /// The `TypeRef` case for a type member `prefix.name`: its prefix mapped (at `variance max 0`, cycles not allowed);
    /// where the prefix is interesting, locked a cycle, else its alias or its bounds checked.
    fn nc_member_ref(&mut self, m: &mut NonCyclic, t: TypeId, prefix: TypeId, name: Name) -> Result<(), ()> {
        if !self.nc_interesting(prefix) {
            return Ok(());
        }
        let saved = m.variance;
        m.variance = m.variance.max(0);
        let r = self.nc_apply_with(m, prefix, false, false);
        m.variance = saved;
        r?;
        if m.locked.contains(&t) {
            return Err(());
        }
        m.locked.push(t);
        let r = match self.type_member(prefix, name) {
            Some(super::members::MemberInfo::Alias(a)) => self.nc_check_member_alias(m, a),
            Some(super::members::MemberInfo::Bounds(lo, hi)) => self.nc_check_info(m, lo, hi),
            _ => Ok(()),
        };
        m.locked.pop();
        r
    }

    /// A path prefix mapped over (`mapOver`'s `NamedType`: the prefix at `variance max 0`).
    fn nc_map_prefix(&mut self, m: &mut NonCyclic, prefix: TypeId) -> Result<(), ()> {
        let saved = m.variance;
        m.variance = m.variance.max(0);
        let r = self.nc_apply(m, prefix);
        m.variance = saved;
        r
    }

    /// `isInteresting(prefix)` (342-360) for a binder (owned by no class): a path into an object of this run's sources,
    /// or a local path (its prefix `NoPrefix`); an intersection either side, a union both; a refinement, a recursive
    /// type, an application; an annotated type its parent; a class's `this` none (the owner is no class).
    fn nc_interesting(&mut self, prefix: TypeId) -> bool {
        let prefix = self.deref_keeping(prefix);
        match self.types.get(prefix) {
            Type::Term(s) => match self.inner_object_of_sym(s) {
                Some(_) => self.nc_source_object(s),
                None => true,
            },
            Type::Select(q, s) => match self.inner_object_of_sym(s) {
                Some(_) => self.nc_source_object(s),
                None => self.nc_interesting(q),
            },
            Type::Inter(a, b) => self.nc_interesting(a) || self.nc_interesting(b),
            Type::Union(a, b, _) => self.nc_interesting(a) && self.nc_interesting(b),
            Type::Refined(..) | Type::Rec(_) | Type::AppParam(..) | Type::AppMember(..) => true,
            Type::Class(_, args) => !self.types.items(args).is_empty(),
            Type::Annotated(u) => self.nc_interesting(u),
            _ => false,
        }
    }

    /// A static object defined in this run (`pcls.span.exists && pcls.defRunId == ctx.runId`): of a program's source.
    fn nc_source_object(&self, s: SymId) -> bool {
        let file = self.syms.sym(s).file;
        !self.in_jar(file) && !self.is_body_file(file)
    }

    /// `TypeMap.mapOver` (`core/Types.scala` 6429-6539) for the map: an application's constructor and its arguments
    /// (`mapArgs` by `tyconTypeParams`), a lambda's parameters' bounds at the variance flipped and its result
    /// (`mapOverLambda`), a wildcard's bounds (the lower one flipped), a variable's instance (`mapOverTypeVar`), an
    /// annotated type's parent (`Annotation.mapWith`: `RuntimeChecked`, the one annotation teq keeps, has no argument),
    /// a match type's bound, scrutinee (variance 0) and cases, an intersection's and a union's sides, a class
    /// constant's class type.
    fn nc_map_over(&mut self, m: &mut NonCyclic, t: TypeId) -> Result<(), ()> {
        match self.types.get(t) {
            Type::Class(c, args) => {
                if self.types.items(args).is_empty() {
                    return Ok(());
                }
                let variances: Vec<i8> = self.syms.class(c).tparams.iter().map(|&p| self.syms.tparam(p).variance).collect();
                let args = self.types.items(args).to_vec();
                self.nc_map_args(m, &args, &variances)
            }
            // `mapOver(AppliedType)`: the constructor, then `mapArgs(args, tyconTypeParams(tp))`, each argument at its
            // parameter's `paramVarianceSign`.
            Type::AppParam(f, args) => {
                let head = self.types.param(f);
                self.nc_apply(m, head)?;
                let variances = self.tycon_type_params(head)?;
                let args = self.types.items(args).to_vec();
                self.nc_map_args(m, &args, &variances)
            }
            Type::AppMember(head, args) => {
                self.nc_apply(m, head)?;
                let args = self.types.items(args).to_vec();
                let variances = self.tycon_type_params(head)?;
                self.nc_map_args(m, &args, &variances)
            }
            Type::Alias(a, args) => {
                // `this(tycon)`: the alias's `TypeRef`, a cycle there under `cycle_ok` its `LazyRef`.
                let r = self.nc_alias_ref(m, a);
                if r.is_err() && !m.cycle_ok {
                    return r;
                }
                let variances: Vec<i8> = self.syms.aliases[a.idx()].tparams.iter().map(|&p| self.syms.tparam(p).variance).collect();
                let args = self.types.items(args).to_vec();
                self.nc_map_args(m, &args, &variances)
            }
            Type::Nested(prefix, class) => {
                self.nc_map_prefix(m, prefix)?;
                self.nc_map_over(m, class)
            }
            Type::Lambda(ps, body) | Type::Poly(ps, body) => {
                // `mapOverLambda`: the parameters' infos (`TypeBounds`) at `-variance`, a `PolyType`'s as the type stores
                // them (`poly_bounds`), a lambda's its binders'; then the result.
                let bounds: Vec<(TypeId, TypeId)> = match self.types.get(t) {
                    Type::Poly(..) => self.types.poly_bounds(ps).chunks(2).map(|b| (b[0], b.get(1).copied().unwrap_or(ANY))).collect(),
                    _ => self.types.items(ps).to_vec().into_iter().filter_map(|p| match self.types.get(p) {
                        Type::Param(q) => Some((self.syms.tparam(q).lower, self.syms.tparam(q).upper)),
                        _ => None,
                    }).collect(),
                };
                let saved = m.variance;
                m.variance = -saved;
                for (lo, hi) in bounds {
                    let inner = m.variance;
                    m.variance = -inner;
                    let r = self.nc_apply(m, lo);
                    m.variance = inner;
                    if r.is_err() {
                        m.variance = saved;
                        return r;
                    }
                    if let Err(e) = self.nc_apply(m, hi) {
                        m.variance = saved;
                        return Err(e);
                    }
                }
                m.variance = saved;
                self.nc_apply(m, body)
            }
            Type::BoundedWild(lo, hi) => {
                let saved = m.variance;
                m.variance = -saved;
                let r = self.nc_apply(m, lo);
                m.variance = saved;
                r?;
                self.nc_apply(m, hi)
            }
            Type::Var(_) | Type::AppVar(..) => {
                let inst = self.deref_keeping(t);
                if inst != t {
                    return self.nc_apply(m, inst);
                }
                // An open variable's application: its arguments as any application's.
                let Type::AppVar(v, args) = self.types.get(t) else { return Ok(()) };
                let head = self.types.mk(Type::Var(v));
                let args = self.types.items(args).to_vec();
                let variances = self.tycon_type_params(head)?;
                self.nc_map_args(m, &args, &variances)
            }
            Type::Annotated(u) => self.nc_apply(m, u),
            Type::Union(a, b, _) | Type::Inter(a, b) => {
                self.nc_apply(m, a)?;
                self.nc_apply(m, b)
            }
            Type::Match(scrut, mid) => {
                let info = self.types.match_info(mid).clone();
                self.nc_apply(m, info.bound)?;
                let saved = m.variance;
                m.variance = 0;
                let r = self.nc_apply(m, scrut);
                m.variance = saved;
                r?;
                // The cases at the variance restored: a case's binders (`mapOverLambda` of a `MatchCase` result: their
                // infos at 0), then `MatchCase[Pat, +Body]`'s arguments (`scala/runtime/MatchCase.scala`).
                for case in info.cases.iter() {
                    let binders = self.types.items(case.binders).to_vec();
                    if !binders.is_empty() {
                        m.variance = 0;
                        let r = binders.into_iter().try_for_each(|b| match self.types.get(b) {
                            Type::Param(q) => {
                                let (lo, hi) = (self.syms.tparam(q).lower, self.syms.tparam(q).upper);
                                self.nc_apply(m, lo).and_then(|_| self.nc_apply(m, hi))
                            }
                            _ => Ok(()),
                        });
                        m.variance = saved;
                        r?;
                    }
                    self.nc_map_args(m, &[case.pattern, case.body], &[0, 1])?;
                }
                Ok(())
            }
            Type::Lit(l) => match self.types.lit_val(l) {
                crate::types::LitVal::Class(class) => self.nc_apply(m, class),
                _ => Ok(()),
            },
            _ => Ok(()),
        }
    }

    /// `AppliedType.tyconTypeParams` (`core/Types.scala` 4831-4834) of an application's constructor `head`, their
    /// variances (`paramVarianceSign`): `tycon.typeParams` (`type_params_reads`: a parameter's or a member's information
    /// through its bounds, `O.G <: P#F` the parameters `P#F` declares, `G <: F` those of `F`'s clause); none,
    /// `HKTypeLambda.any`'s invariant ones (`nc_map_args` reads an absent variance as invariant). A read's cycle is the
    /// check's (`checkNonCyclic`'s `CyclicReference` catch).
    fn tycon_type_params(&mut self, head: TypeId) -> Result<Vec<i8>, ()> {
        let ps = self.type_params_reads(head).map_err(|_| ())?;
        Ok(ps.iter().map(|&q| match self.types.get(q) {
            Type::Param(q) => self.syms.tparam(q).variance,
            _ => 0,
        }).collect())
    }

    /// CheckNonCyclicMap's `mapArg` (380-388) over `mapArgs`: each argument at `variance * paramVarianceSign`, cycles
    /// allowed as nested ones are, nested ones where the variance differs from the parameter's (none past the
    /// parameters, an absent variance invariant).
    fn nc_map_args(&mut self, m: &mut NonCyclic, args: &[TypeId], variances: &[i8]) -> Result<(), ()> {
        for (k, &arg) in args.iter().enumerate() {
            let sign = variances.get(k).copied().unwrap_or(0).signum();
            let variance_diff = m.variance != sign;
            let saved = m.variance;
            m.variance = saved * sign;
            let nested1 = m.nested_cycle_ok || m.variance != 0 && variance_diff;
            let nested = m.nested_cycle_ok;
            let r = self.nc_apply_with(m, arg, nested, nested1);
            m.variance = saved;
            r?;
        }
        Ok(())
    }

    /// The name of the binder `r` annotates, by its clause's names.
    fn binder_name(&self, file: FileId, r: &ast::BinderAnnot) -> Name {
        let ast = self.ast(file);
        let names = match r.clause {
            ast::BinderClause::Lambda(t) | ast::BinderClause::Own(t) | ast::BinderClause::PolyFun(t) => match ast.ty(t) {
                TyExpr::Lambda(l, _) | TyExpr::PolyFun(l, _) => l,
                _ => return names::EMPTY,
            },
            ast::BinderClause::PolyLiteral(e) => match ast.expr(e) {
                ast::Expr::PolyLambda(l, _) => l,
                _ => return names::EMPTY,
            },
        };
        ast.name_lists[names.range()].get(r.index as usize).copied().unwrap_or(names::EMPTY)
    }

    /// `Checking.checkAnnotArgs`: the compiler's own annotations' arguments, its own tests. `@targetName`'s one
    /// argument (`Apply(tycon, arg :: Nil)`) a `Literal` of the typed tree: one written so, its parentheses dropped as
    /// the typed tree drops them, not empty; a named one, an ascription, a block, an operation (`ConstFold` gives it a
    /// constant type, not a `Literal`) or a constant's name is none. `@implicitNotFound`'s and `@implicitAmbiguous`'s
    /// argument of a constant type (`arg.tpe.widenTermRefExpr.normalized`), `argument_constant`.
    fn check_annot_args(&mut self, file: FileId, class: ClassId, instance: ast::ExprId) {
        let path = self.class_path(class);
        let name = match path.as_str() {
            "scala.annotation.targetName" => "targetName",
            "scala.annotation.implicitNotFound" => "implicitNotFound",
            "scala.annotation.implicitAmbiguous" => "implicitAmbiguous",
            _ => return,
        };
        let ast = self.ast(file);
        let ast::Expr::New(_, written) = ast.expr(instance) else { return };
        let &[arg] = ast.expr_list(written) else { return };
        let at = ast.expr_span(arg);
        if name == "targetName" {
            let mut lit = arg;
            while let ast::Expr::Parens(x) = ast.expr(lit) {
                lit = x;
            }
            match ast.expr(lit) {
                ast::Expr::StringLit(s) if ast.str(s).is_empty() => self.error(at, "target name cannot be empty"),
                ast::Expr::StringLit(_)
                | ast::Expr::IntLit(_)
                | ast::Expr::LongLit(_)
                | ast::Expr::DoubleLit(_)
                | ast::Expr::DecimalLit(_)
                | ast::Expr::FloatLit(_)
                | ast::Expr::BoolLit(_)
                | ast::Expr::CharLit(_)
                | ast::Expr::NullLit
                | ast::Expr::UnitLit => {}
                _ => self.error(at, format!("@{} needs a string literal as argument", name)),
            }
            return;
        }
        let t_string = self.b.t_string;
        let ty = self.annot_argument_type(file, arg, Some(t_string));
        if self.constant_of_type(ty).is_none() {
            self.error(at, format!("@{} requires constant expressions as a parameter", name));
        }
    }

    /// The type of the annotation argument `arg` as the annotation's typed tree has it (`arg.tpe`): typed where the
    /// annotation is, against `expected`, under a rollback: what its typing reports at it is the annotation's, which
    /// typed it already.
    fn annot_argument_type(&mut self, file: FileId, arg: ast::ExprId, expected: Option<TypeId>) -> TypeId {
        let span = self.ast(file).expr_span(arg);
        let recorded = self.index_mark();
        let mark = self.diags.items.len();
        let frame = self.var_frame_begin();
        let trail = self.snapshot();
        let pending = self.attempts.pending_len();
        let arg = match self.ast(file).expr(arg) {
            ast::Expr::NamedArg(_, v) => v,
            _ => arg,
        };
        let (_, ty) = self.type_expr(arg, expected);
        if self.attempts.pending_len() > pending {
            self.flush_pending_from(pending);
        }
        let ty = self.deref_keeping(ty);
        self.rollback(&trail);
        self.var_frame_end(frame);
        let own = |d: &crate::source::Diagnostic| d.file == file && d.span.start >= span.start && d.span.end <= span.end;
        let elsewhere: Vec<crate::source::Diagnostic> = self.diags.items.drain(mark..).filter(|d| !own(d)).collect();
        self.diags.items.extend(elsewhere);
        self.index_drop_records_within(recorded, file, span);
        ty
    }

    /// `tpd.stripCast` (`TreeInfo.scala` 845-855) of a written argument: `x.asInstanceOf[T]` (and `x.asInstanceOf[T]()`),
    /// the selection `isTypeCast`'s (`Any.asInstanceOf`, `Any.$asInstanceOf$`), its operand, recursively; none where
    /// `arg` is no cast.
    fn strip_cast(&mut self, file: FileId, arg: ast::ExprId) -> Option<ast::ExprId> {
        let ast = self.ast(file);
        let mut e = match ast.expr(arg) {
            ast::Expr::NamedArg(_, v) => v,
            _ => arg,
        };
        let mut stripped = None;
        loop {
            let ty_app = match ast.expr(e) {
                ast::Expr::Apply(f, args) if ast.expr_list(args).is_empty() => f,
                _ => e,
            };
            let ast::Expr::TypeApply(f, _) = ast.expr(ty_app) else { return stripped };
            let ast::Expr::Select(inner, n) = ast.expr(f) else { return stripped };
            // teq's typer resolves the selection to the universal cast by its name (`universal_member`).
            if n != names::AS_INSTANCE_OF && self.name_str(n) != "$asInstanceOf$" {
                return stripped;
            }
            stripped = Some(inner);
            e = inner;
        }
    }

    /// `Checking.checkAnnotClass`: a Java class an annotation interface, a Scala class an
    /// `scala.annotation.Annotation`. `@assumeSafe`'s branch is the safe mode's (`Feature.safeEnabled`), which teq
    /// does not read.
    fn check_annot_class(&mut self, class: ClassId, span: Span) {
        // `JavaDefined`: read from a class file, a std's stand-in for the JDK's (the JVM back end's `java_defined`), or
        // `java.lang.Object`, which teq's `AnyRef` is.
        let object = class == self.b.any_ref;
        let java = object || self.is_java_class(class) || {
            let std = self.source(self.syms.class(class).file).is_std;
            std && self.jdk_rooted(self.package_of_class(class))
        };
        if java {
            if self.syms.class(class).mods & mods::JAVA_ANNOTATION == 0 {
                let shown = if object { "class Object".to_string() } else { self.shown(super::referents::Referent::Class(class)).0 };
                self.error(span, format!("{} is not a valid Java annotation: it was not declared with `@interface`", shown));
            }
            return;
        }
        self.complete_class(class);
        let annotation = self.syms.class(class).base_types.iter().map(|&(b, _)| b).collect::<Vec<_>>().into_iter().any(|b| self.class_path(b) == "scala.annotation.Annotation");
        if !annotation {
            let (shown, _) = self.shown(super::referents::Referent::Class(class));
            self.error(span, format!("{} is not a valid Scala annotation: it does not extend `scala.annotation.Annotation`", shown));
        }
    }

    /// Types a given. Parameterless structural givens become a value holding the instance, which
    /// under the fork is the given's body cell's, as a val's initialiser is: one worker makes and
    /// publishes it, another that reads the given (a macro's run) waits for it (`walk_body`). One
    /// with type or term parameters is a def making an instance per call (dotty's
    /// `Parsers.givenDef`).
    fn check_given(&mut self, sym: SymId) {
        if self.unwinding_from(sym) {
            return;
        }
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
        // A member of a class of a body, at any depth of classes, is typed with the body: a
        // local trait lifts it with what the classes nested in it read (`proxies.rs`).
        let mut local = false;
        let mut owner = self.syms.sym(sym).owner;
        while let Owner::Class(c) = owner {
            owner = self.syms.class(c).owner;
            if owner == Owner::Local {
                local = true;
            }
        }
        self.in_std(sym) && !local && self.is_method_sym(sym)
    }

    /// Whether `sym` is a top-level val of the standard library, which its first reader types
    /// (`std_val_body`) rather than the walk over its file.
    pub fn defers_top_val(&self, sym: SymId) -> bool {
        let info = self.syms.sym(sym);
        // teq's own `std/` alone: a library's pseudo files of bodies, marked std too, are checked
        // with their files as before (a product's top-level vals among them).
        info.kind == SymKind::Val && matches!(info.owner, Owner::Package(_)) && self.std_source(info.file)
    }

    /// A top-level val of the standard library a reader names, typed then with the other vals of
    /// its file, which its initialiser runs together, and registered with the program's top-level
    /// values once, as the walk registers a program's.
    pub fn std_val_body(&mut self, sym: SymId) -> Option<TExprId> {
        let file = self.syms.sym(sym).file;
        self.with_loader_for(crate::measure::Hold::Body, |w| {
            for s in w.top_level_vals(file) {
                if !w.defers_top_val(s) || w.prog.top_vals.all().any(|(v, _)| v == s) {
                    continue;
                }
                w.ensure_body(s);
                if let Some(init) = w.readable_init(s) {
                    w.prog.top_vals.push((s, init));
                }
            }
            w.readable_init(sym)
        })
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
        } else if self.defers_top_val(sym) {
            self.std_val_body(sym).map(DeferredBody::Val)
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
    pub(super) fn register_top_fun(&mut self, f: FunId) {
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

    /// A member of `Product` or `Equals`, whose runtime templates a class implements without
    /// `override`, as the abstract members of scalac's `Product` and `Equals` are.
    fn is_product_member(&self, s: SymId) -> bool {
        matches!(self.syms.sym(s).owner, Owner::Class(c) if Some(c) == self.b.product || Some(c) == self.equals_class())
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
            // A read of a completion is a demand of the one in progress (`demand_sig_read`).
            if self.inlining.track_demands {
                self.demand_sig_read(sym);
            }
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
            if self.inlining.track_demands {
                self.demand_sig_read(sym);
            }
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
                self.wait_cell(CellKey::new(CELL_SIG, sym.0), |w| w.syms.sym(sym).state().get().is_complete());
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
            // An inferred one the unwinding typer of its suspended unit would start stays for its
            // readers (`unwinding_from`), and so does a default's getter, whose result its default
            // infers (`Desugar.defaultGetters` leaves its type to a `TypeTree()`); a declared one
            // types no body, and a class checked on the way reads its parents' (`Namer`'s
            // completers complete them alike).
            Completion::NotStarted if self.unwinding_from(sym) && (self.result_inferred(file, def_id, owner, kind) || self.getters.link.contains_key(&sym)) => return None,
            Completion::NotStarted if self.syms.sym_cells.claim(sym.0) => Completion::NotStarted,
            Completion::NotStarted => Completion::InProgress,
            other => other,
        };
        match state {
            Completion::Done => {}
            Completion::Suspended => return self.read_suspended_sig(sym),
            Completion::InProgress if !self.syms.sym(sym).state().mine() && self.wait_cell(CellKey::new(CELL_SIG, sym.0), |w| w.syms.sym(sym).state().get().is_complete()) => {
                if self.syms.sym(sym).state() == Completion::Suspended {
                    return self.read_suspended_sig(sym);
                }
            }
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
                if self.syms.sym(sym).state() == Completion::Suspended {
                    if self.inlining.track_demands {
                        self.demand_sig_read(sym);
                    }
                    return None;
                }
            }
        }
        // The completion this read made or waited for, or found made, is a demand of the one in
        // progress (`demand_sig_read`).
        if self.inlining.track_demands {
            self.demand_sig_read(sym);
        }
        self.syms.sym(sym).info.sig.as_ref()
    }

    /// A read of the signature of `sym`, suspended by a macro of the run (`read_suspended`): its
    /// reader depends on what the suspended completion reported before it suspended.
    #[cold]
    fn read_suspended_sig(&mut self, sym: SymId) -> Option<&Arc<MethodSig>> {
        if self.inlining.track_demands {
            self.demand_sig_read(sym);
        }
        self.read_suspended()
    }

    /// A read of a signature whose inference met a macro of the run at typing time
    /// (`Completion::Suspended`): the reader's unit suspends, and every signature being inferred
    /// around the read with it (`suspend_at_typing`), as dotty's `SuspendCompleter` suspends the
    /// unit that completes the definition again and its exception leaves every completer on the
    /// way, and reads an error value; in the inlining phase the expansion under way is given up,
    /// its file typed again (`suspended_hit`).
    #[cold]
    #[inline(never)]
    pub(super) fn read_suspended(&mut self) -> Option<&Arc<MethodSig>> {
        if !self.inlining.running {
            let unit = self.inline.sites.first().map_or(self.env.file, |s| s.file);
            self.suspend_at_typing(unit);
        } else if self.inlining.suspending {
            self.hit_suspension();
        }
        None
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
        let reader = self.inline.sites.first().map_or(self.env.file, |s| s.file);
        // Any completion that met a suspension is suspended, a declared signature's too (a
        // parameter's `x.type` of a val suspended), as dotty's completer installs a
        // `SuspendCompleter` on any `SuspendException` (`Namer.Completer.complete`).
        self.inlining.sig_marks.push(Some(self.inlining.typing_suspensions));
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
        // An inferred type across a macro of the run met at typing time (`typing_suspends`) stands
        // for none: published `Suspended` (`publish_sig`), its reader's unit suspends, and so does
        // each later reader's (`read_suspended`), as dotty's `SuspendCompleter` in place of the
        // completer that suspended (`Namer.Completer.complete`).
        self.inlining.sig_marks.pop();
        if self.syms.sym(sym).state() == Completion::Suspended {
            self.suspend_at_typing(reader);
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
        // A default's getter, a member of its own (`super::getters`).
        if let Some(&(of, index)) = self.getters.link.get(&sym) {
            let (sig, noted) = self.complete_getter(sym, of, index);
            let sig = Arc::new(sig);
            self.publish_sig_noted(sym, sig.clone(), noted);
            self.outermost_completion_done();
            return sig;
        }
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
            if let Some(field) = self.selector_field(sym) {
                let sig = Arc::new(MethodSig::value(self.sig_arc(field).ret));
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
        // What the completion reports and demands is the signature's own (`DiagOwner::sig`): its
        // type parameters, its parameters, its written result, the overridden members an
        // inferred result reads and the body it is inferred from (`Namer.valOrDefDefSig`,
        // `defDefSig`, `inferredResultType`).
        let sig_owner = self.sig_owner(sym);
        let outer_owner = sig_owner.map(|o| self.enter_owner(o));
        let marks = (self.inlining.demands.len(), self.diags.items.len());
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
        let mut noted = false;
        if let Some(outer) = outer_owner {
            noted = self.inlining.demands.len() > marks.0 || self.diags.items.len() > marks.1;
            self.leave_owner(outer, false);
        }
        let sig = Arc::new(sig);
        self.publish_sig_noted(sym, sig.clone(), noted);
        // What waited for the outermost completion runs once this signature is done.
        self.outermost_completion_done();
        sig
    }

    /// Whether `owner` is a block, a class made in one (an anonymous class among them) or a
    /// class nested in such a class: what it defines is typed where the block is, inside the
    /// expansion under way if the block is an inline method's. Any other definition's body is
    /// typed on its own, whatever asked for it (`outside_inline`, `outside_quotes`).
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
        self.publish_sig_noted(sym, sig, false)
    }

    /// `publish_sig`, with whether the completion recorded a demand or a report
    /// (`SymInfo::completion_noted`), published with the signature.
    fn publish_sig_noted(&mut self, sym: SymId, sig: Arc<MethodSig>, noted: bool) {
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
            let done = self.completion_done();
            self.with_loader_for(crate::measure::Hold::Publication, |w| {
                let sig = w.published_sig(sig, "a shared symbol's signature (publish_sig)");
                w.syms.sym_mut(sym).sig = Some(sig);
                if noted {
                    w.syms.sym_mut(sym).completion_noted = true;
                }
                w.publish_class_bodies();
                w.syms.sym(sym).state().set(done);
            });
        } else {
            let done = self.completion_done();
            let mut s = self.syms.sym_mut(sym);
            s.sig = Some(sig);
            if noted {
                s.completion_noted = true;
            }
            s.state().set(done);
        }
    }

    /// The state a signature being completed is published in: `Suspended` where its inference met
    /// a macro of the run at typing time (`typing_suspends`), `Done` otherwise.
    pub(super) fn completion_done(&self) -> Completion {
        match self.inlining.sig_marks.last() {
            Some(&Some(mark)) if mark != self.inlining.typing_suspensions => Completion::Suspended,
            _ => Completion::Done,
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

    /// `infer_body` with the body's plain inline calls expanded once the result type is set as
    /// the member's partial signature, which an expansion reading the member then finds rather
    /// than the recursion (dotty types the body, the member's type with it, before it inlines).
    fn infer_body_then_expand(&mut self, sym: SymId, partial: &MethodSig, inherited: Option<TypeId>, sig_of: impl Fn(TypeId) -> Arc<MethodSig>) -> TypeId {
        let pending = self.attempts.pending_len();
        let outer = std::mem::replace(&mut self.inferring_depth, self.body_depth + 1);
        let inferred = self.infer_body(sym, partial, inherited);
        self.inferring_depth = outer;
        if self.attempts.pending_len() > pending {
            if inferred != ERROR {
                self.set_partial_sig(sym, sig_of(inferred));
            }
            // A local definition inferred inside another member's inference leaves its calls to
            // that member's expansion, once the member's own type is set: dotty inlines after
            // typer, every type of the unit known (`Inlining`), so an expansion that reads the
            // enclosing `val` (macwire's eligible values) finds its type, not the recursion.
            let nested_local = outer != 0 && self.syms.sym(sym).owner == Owner::Local;
            if !nested_local {
                let erased_mark = self.erased_values.len();
                let file = self.syms.sym(sym).file;
                self.end_unit(pending, file);
                // What the expansions made that the body still holds is used
                // (`Erasure.checkNotErased`), as the body's own typing's is (`type_body`).
                if self.erased_values.len() > erased_mark {
                    let roots: Vec<TExprId> = self.fun_of_sym.get(&sym).and_then(|&f| self.prog.funs[f.idx()].body).into_iter().chain(self.val_init.get(&sym).copied()).collect();
                    self.unit_erased_values(erased_mark, &roots, file);
                }
            }
        }
        inferred
    }

    /// The clauses of a method a bound of whose type parameters names a parameter: the other
    /// bounds before the clauses, those after them, once the parameters are bound.
    #[cold]
    #[inline(never)]
    fn compute_dependent_bounds_sig(&mut self, f: &'a ast::FunDef, ids: &[(Name, TParamId)]) -> Vec<ClauseSig> {
        let ast = self.cur_ast();
        let is_param = |n: Name| f.clauses.iter().any(|c| c.params.iter().any(|p| p.name == n));
        let dependent = |tp: &ast::TypeParam| [tp.upper, tp.lower].into_iter().flatten().any(|b| super::resolve::ty_selects_from(ast, b, &is_param));
        self.resolve_tparam_bounds_of(&f.tparams, ids, |tp| !dependent(tp));
        let clauses = self.resolve_clauses(&f.clauses, None, &f.tparams, ids, None);
        self.resolve_tparam_bounds_of(&f.tparams, ids, dependent);
        clauses
    }

    fn compute_sig_inner(&mut self, sym: SymId, def: &'a ast::Def) -> MethodSig {
        let t = self;
        match &def.kind {
            DefKind::Val { ty, .. } => match ty {
                Some(ty) => {
                    let (ty, declared) = t.declared_view(*ty, |w| w.resolve_declared_type(*ty));
                    MethodSig { declared, ..MethodSig::value(t.types.unannotated(t.declared_chain(ty, declared))) }
                }
                // An incomplete val's type is unknown unless it was declared.
                None if def.mods & mods::INCOMPLETE != 0 => MethodSig::value(ERROR),
                None => {
                    let partial = Arc::new(MethodSig::value(ERROR));
                    t.set_partial_sig(sym, partial.clone());
                    let inherited = t.inherited_result_type(sym, &partial);
                    if inherited.is_none() && t.syms.sym(sym).owner != Owner::Local {
                        t.missing_implicit_type(sym, def, "");
                    }
                    let inferred = t.infer_body_then_expand(sym, &partial, inherited, |ret| Arc::new(MethodSig::value(ret)));
                    if t.shared_sym(sym) {
                        t.publish_body(sym);
                    }
                    MethodSig::value(t.types.unannotated(inferred))
                }
            },
            DefKind::Fun(f) if def.name == names::INIT => {
                // A secondary constructor: the class's type parameters, and the class as result.
                let Owner::Class(c) = t.syms.sym(sym).owner else { return MethodSig::value(ERROR) };
                let clauses = t.resolve_clauses(&f.clauses, None, &[], &[], None);
                let info = t.syms.class(c);
                MethodSig { tparams: info.tparams.clone(), clauses, ret: info.base_types[0].1, declared: crate::tir::NO_TYPE }
            }
            DefKind::Fun(f) => {
                let ids = t.make_tparams(&f.tparams);
                t.env.frames.push(Frame::Locals { names: Vec::new(), tparams: ids.clone(), givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: t.sites.owners.len() as u32 });
                // A bound of a clause after a term clause may name its parameters (an extension's
                // `Elems <: p.MirroredElemTypes`, `Mirror.fromProductTyped`): resolved once they
                // are, the others before them.
                let ast = t.cur_ast();
                let is_param = |n: Name| f.clauses.iter().any(|c| c.params.iter().any(|p| p.name == n));
                let dependent_bounds = !f.clauses.is_empty()
                    && f.tparams.iter().any(|tp| [tp.upper, tp.lower].into_iter().flatten().any(|b| super::resolve::ty_selects_from(ast, b, &is_param)));
                let clauses = if dependent_bounds {
                    t.compute_dependent_bounds_sig(f, &ids)
                } else {
                    t.resolve_tparam_bounds(&f.tparams, &ids);
                    t.resolve_clauses(&f.clauses, None, &f.tparams, &ids, None)
                };
                if let Some(tags) = t.erased_tags.take() {
                    t.shared_write(sym, |w| w.syms.sym_mut(sym).erased_tags = tags);
                }
                let tparams: Vec<TParamId> = ids.iter().map(|&(_, p)| p).collect();
                let (ret, declared) = match f.ret {
                    Some(r) => {
                        let (ty, declared) = t.declared_view(r, |w| w.resolve_declared_type(r));
                        (Some(t.declared_chain(ty, declared)), declared)
                    }
                    None => (None, NO_TYPE),
                };
                // A method of no parameters is no application's: its reference is the `TermRef`,
                // which carries no annotation of its result (`T @RuntimeChecked`).
                let ret = ret.map(|r| if clauses.is_empty() { t.types.unannotated(r) } else { r });
                t.env.frames.pop();
                let mut sig = MethodSig { tparams, clauses, ret: ret.unwrap_or(ERROR), declared };
                // A macro without a declared result type has the type its splice's `Expr` holds,
                // against which a call infers its type arguments before the macro runs.
                if ret.is_none() && def.mods & mods::INLINE != 0 && def.mods & (mods::TRANSPARENT | mods::INCOMPLETE) == 0 {
                    if let Some(body) = f.body.filter(|&b| matches!(t.ast(t.syms.sym(sym).file).expr(b), ast::Expr::Splice(_))) {
                        let _ = body;
                        sig.ret = t.inferred_macro_result(sym, &sig);
                    }
                }
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
                        let shape = sig.clone();
                        sig.ret = t.infer_body_then_expand(sym, &sig, inherited, |ret| Arc::new(MethodSig { ret, ..shape.clone() }));
                        if sig.clauses.is_empty() {
                            sig.ret = t.types.unannotated(sig.ret);
                        }
                        if t.shared_sym(sym) {
                            t.publish_body(sym);
                        }
                    }
                }
                sig
            }
            // A given class's def answers an instance of the class, of its own type parameters, as
            // dotty's desugaring of a structural given with parameters makes it (`Parsers.givenDef`'s
            // `TypeDef`, `Desugar.classDef`'s implicit wrapper `def t(using ..): t = new t`): the
            // members its body adds are the result's.
            DefKind::Given(g) => match t.syms.sym(sym).impl_class {
                Some(c) => {
                    t.complete_class(c);
                    let info = t.syms.class(c);
                    let (tparams, clauses) = (info.tparams.clone(), info.ctor.clone());
                    // A given object, of no parameters, is read as its object
                    // (`given_object_type`); its value's type stays the one declared.
                    let ret = if tparams.is_empty() && clauses.iter().all(|c| c.params.is_empty()) {
                        info.parents.first().copied().unwrap_or(ERROR)
                    } else {
                        let own: Vec<TypeId> = tparams.iter().map(|&p| t.types.param(p)).collect();
                        t.types.class(c, &own)
                    };
                    MethodSig { tparams, clauses, ret, declared: crate::tir::NO_TYPE }
                }
                None => {
                    if let Some(sig) = t.derived_ctor_sig(g) {
                        return sig;
                    }
                    let ids = t.make_tparams(&g.tparams);
                    t.env.frames.push(Frame::Locals { names: Vec::new(), tparams: ids.clone(), givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: t.sites.owners.len() as u32 });
                    t.resolve_tparam_bounds(&g.tparams, &ids);
                    let clauses = t.resolve_clauses(&g.clauses, None, &g.tparams, &ids, None);
                    let (ret, declared) = t.declared_view(g.ty, |w| w.resolve_declared_type(g.ty));
                    let ret = t.declared_chain(ret, declared);
                    t.env.frames.pop();
                    if t.deferred_candidate(sym) && g.alias.is_some_and(|a| t.is_deferred_marker(a)) {
                        t.shared_write(sym, |w| w.syms.sym_mut(sym).mods |= mods::DEFERRED);
                    }
                    MethodSig { tparams: ids.iter().map(|&(_, p)| p).collect(), clauses, ret, declared }
                }
            },
            _ => MethodSig::value(ERROR),
        }
    }

    /// scalac's diagnostic of a `return` that leaves a lambda or a by-name argument on its way
    /// out of the method, by the unit's version (`NonLocalReturns.transformReturn`).
    fn warn_nonlocal_returns(&mut self, body: TExprId, returns: &[(TExprId, crate::source::Span)]) {
        let mut found = Vec::new();
        self.nonlocal_returns(body, false, &mut found);
        for e in found {
            if let Some(&(_, span)) = returns.iter().find(|&&(r, _)| r == e) {
                if !self.restrict_nonlocal_return(span) {
                    // The phase after erasure that expands them.
                    self.migration_of_phase(crate::source_version::Migration::NonLocalReturns, span, crate::warnings::NO_ID, crate::source::PHASE_NONLOCAL_RETURNS, || {
                        "Non local returns are no longer supported; use `boundary` and `boundary.break` in `scala.util` instead".to_string()
                    });
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
            | TExpr::Js(_, args) | TExpr::SeqLit(args) | TExpr::ArrayLit(args) | TExpr::ObjLit(args) | TExpr::Default(_, _, args) => {
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
            | TExpr::Module(_) | TExpr::ClassOf(..) | TExpr::JsImport(_) | TExpr::JsGlobal(..) => {}
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
            let b = self.syms.class(c).base_types[i].0;
            let bt = self.base_instance_at(c, i);
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
        if self.unwinding_from(sym) {
            return;
        }
        // A body typed before the walk reaches it (a completion another definition demanded) is
        // one the walk demands all the same (`note_reached`).
        if self.fun_of_sym.contains_key(&sym) || self.val_init.contains_key(&sym) {
            // The traversal reads the signature where it reaches the definition
            // (`typedValDef`, `typedDefDef`), a demand of it, and a suspended one suspends the
            // walk's unit (`read_suspended`).
            self.sig_of(sym);
            self.note_reached(sym);
            self.reach_getters(crate::tir::DefaultOf::Method(sym));
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
            self.note_reached(sym);
            self.reach_getters(crate::tir::DefaultOf::Method(sym));
            if !self.is_retained_inline(sym) {
                self.type_inline_getters(sym);
                return;
            }
        }
        if self.bodies_in_progress.contains(&sym) {
            return;
        }
        // Kept for the body's typing below.
        let sig = self.sig_arc(sym);
        if self.fun_of_sym.contains_key(&sym) || self.val_init.contains_key(&sym) {
            self.note_reached(sym);
            self.reach_getters(crate::tir::DefaultOf::Method(sym));
            self.fill_own_getters(sym);
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
        // The traversal reaches the method's getters after it (`Desugar.addDefaultGetters`'s
        // `Thicket`), their trees its record's before the record is every worker's.
        self.reach_getters(crate::tir::DefaultOf::Method(sym));
        self.fill_own_getters(sym);
        if self.shared_sym(sym) {
            self.publish_body(sym);
        }
    }

    /// The ordinary default getters of the inline method `sym`, which `Desugar.addDefaultGetters`
    /// makes methods of their own that a call applies (an inline parameter's getter is inline):
    /// a function of no body holding those defaults, typed as a method's are. What they report
    /// the definition check reported already.
    fn type_inline_getters(&mut self, sym: SymId) {
        let info = self.syms.sym(sym);
        if info.kind != SymKind::Def || info.def.is_none() || info.owner == Owner::Local || self.fun_of_sym.contains_key(&sym) {
            return;
        }
        let sig = self.sig_arc(sym);
        let ordinary = sig.clauses.iter().flat_map(|c| c.params.iter()).any(|p| p.has_default && self.syms.sym(p.sym).mods & mods::INLINE == 0);
        if !ordinary {
            return;
        }
        let mark = self.diags.items.len();
        let outer = std::mem::replace(&mut self.inline_getters_only, true);
        self.type_member_body_with(sym, sig);
        self.inline_getters_only = outer;
        self.drop_reported_since(mark);
    }

    /// A local inline method's ordinary getters, local functions beside it with its captures,
    /// as a member's are its class's (`Desugar.addDefaultGetters` makes them for a local method
    /// too): a function of no body the block holds where the method stands.
    pub(super) fn type_local_inline_getters(&mut self, sym: SymId, sig: &Arc<MethodSig>) {
        if self.fun_of_sym.contains_key(&sym) {
            return;
        }
        let ordinary = sig.clauses.iter().flat_map(|c| c.params.iter()).any(|p| p.has_default && self.syms.sym(p.sym).mods & mods::INLINE == 0);
        if !ordinary {
            return;
        }
        let mark = self.diags.items.len();
        let outer = std::mem::replace(&mut self.inline_getters_only, true);
        self.type_body(sym, sig, Some(sig.ret));
        self.inline_getters_only = outer;
        self.drop_reported_since(mark);
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
        let (file, owner, span, selector) = {
            let s = self.syms.sym(sym);
            (s.file, s.owner, s.span, s.mods & mods::SELECTOR != 0)
        };
        // A case class's selector its class's check left to its first use (`selector_bodies`).
        if selector {
            if let Some(field) = self.selector_field(sym) {
                self.selector_body(sym, field);
                return;
            }
        }
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
    pub(super) fn publish_body(&mut self, sym: SymId) {
        self.publish_body_bundle(sym);
        self.syms.sym(sym).body().set(Completion::Done);
    }

    /// Enters the parameters of `sig` into the locals frame `frame` clause by clause while the
    /// defaults are typed: a default sees the earlier clauses, as in scalac, so `size = size`
    /// names the enclosing member. Returns the typed defaults, one slot per parameter.
    pub(super) fn enter_params(&mut self, sig: &MethodSig, default_exprs: &[Option<ast::ExprId>], frame: usize, getters: Option<crate::tir::DefaultOf>, record: bool) -> Vec<Option<TExprId>> {
        let mut defaults: Vec<Option<TExprId>> = Vec::new();
        for clause in &sig.clauses {
            for p in &clause.params {
                let default = default_exprs.get(defaults.len()).copied().flatten();
                // A program member's default is its getter's, a completion of its own, never its
                // method's body's (`Desugar.addDefaultGetters`): a function's slot holds its tree
                // where it is complete, filled in later otherwise (`fill_getters`); an inline
                // method's record (`record`) holds the body alone, as the inliner reads it, a call's
                // omission being the getter's application among the arguments (`Inliner`).
                if let (Some(of), Some(_)) = (getters, default) {
                    if self.getter_of(of, defaults.len()).is_some() {
                        defaults.push(if record { None } else { self.getter_tree(of, defaults.len()) });
                        continue;
                    }
                }
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
            self.inline_val_error(span, "inline value must have a literal constant type");
            return ty;
        }
        // `inline val k = (0: Int)` is an `Int`, no constant, as scalac types the ascription, and
        // so is one of a plain inline call or an operation over one, which scalac's typer keeps
        // as a call; in a body the expansion checks it, as scalac checks it where the body
        // expands, after the inlining phase (`expanded_widening`).
        let widened = if in_body { self.expanded_widening(init) } else { self.widened_constant(init) };
        if let Some(v) = self.fold_constant(init).filter(|_| !widened) {
            let v = self.folded_as(v, ty);
            let lit = self.types.lit(v);
            self.prog.set_type(init, lit);
            return lit;
        }
        // A `classOf[T]`'s constant is the class constant of its type, `Class[T]` (`ConstFold.apply`).
        let class_constant = |t: &mut Self| matches!(t.prog.expr(init), TExpr::ClassOf(..)).then(|| crate::types::LitVal::Class(t.widen_lit(ty)));
        if let Some(v) = self.fold_type(ty).or_else(|| self.constant_by_type(init, None)).or_else(|| class_constant(self)).filter(|_| !widened) {
            // A constant of the initialiser's type (a call's declared result `true` included) is
            // the val's where the initialiser is pure (scalac's `exprPurity`); in a body, the
            // expansion checks it.
            if !in_body && !self.is_pure_value(init) {
                let msg = format!("inline value must be pure but was: {}", self.code_of_tree(init));
                self.inline_val_error(span, msg);
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
            self.inline_val_error(span, "`inline val` of type `Unit` is not supported.\n\nTo inline a `Unit` consider using `inline def`");
        } else if t == self.b.t_string || self.is_primitive(t) {
            self.inline_val_error(span, "inline value must have a literal constant type");
        } else {
            self.inline_val_error(span, "inline value must contain a literal constant value.\n\nTo inline more complex types consider using `inline def`");
        }
        ty
    }

    /// An error of scalac's `InlineVals`, which runs after `Inlining` (`Compiler.transformPhases`):
    /// late, so that it holds back no expansion, presented where the build has no other error.
    pub(super) fn inline_val_error(&mut self, span: Span, msg: impl Into<String>) {
        let file = self.env.file;
        self.diags.late_error_of(file, span, msg, crate::source::PHASE_FIRST_TRANSFORM);
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
        self.env.frames.push(Frame::Locals { names: Vec::new(), tparams: Vec::new(), givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: self.sites.owners.len() as u32 });
        // A class nested in a class takes the enclosing instance as every constructor's first
        // parameter, which stands for that instance's `this` in the body and is passed on to the
        // constructor called first (dotty's `ExplicitOuter`, `OuterOps.args` of a `This`), and in
        // the defaults, whose getters are the companion's (`Desugar.classDef`), read with no
        // instance of the class.
        let outer_param = self.outer_class(c).map(|o| (o, self.ctor_outer_param(sym, o)));
        let outer_ctor = std::mem::replace(&mut self.ctor_outer, outer_param);
        let default_exprs: Vec<Option<ast::ExprId>> = f.clauses.iter().flat_map(|cl| cl.params.iter().map(|p| p.default)).collect();
        let defaults = self.enter_params(&sig, &default_exprs, frame, Some(crate::tir::DefaultOf::Method(sym)), false);
        let mut params: Vec<SymId> = sig.clauses.iter().flat_map(|cl| cl.params.iter().map(|p| p.sym)).collect();
        if let Some((_, p)) = outer_param {
            params.insert(0, p);
        }
        let outer_return = self.return_to.take();
        let outer_returns = std::mem::take(&mut self.returns);
        let outer_excluded = self.excluded_ctor.replace(sym);
        let outer_args = self.parent_args_of.replace(c);
        let (call, _) = self.type_expr(self_call, None);
        self.parent_args_of = outer_args;
        self.excluded_ctor = outer_excluded;
        let (after, _) = if rest.len == 0 { (self.prog.add(TExpr::Unit), self.b.t_unit) } else { self.type_block(rest, None, None) };
        self.ctor_outer = outer_ctor;
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
        let fid = self.prog.add_fun(TFun { sym, params, defaults, body: Some(body), body_unconsuming: false, captures: 0, lifted: false });
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
        let erased_mark = self.erased_values.len();
        let pending = self.attempts.pending_len();
        // The unboxings the demanding unit's expansions wait for are that unit's, applied at its
        // own end; the body's unit (its typing and its end) applies its own alone: a std body an
        // inlining phase's worker demands is typed under the loader's lock, which would publish
        // the worker's records with its own types.
        let outer_unboxes = (!self.inline_unboxes.is_empty()).then(|| std::mem::take(&mut self.inline_unboxes));
        let p = self.body_prof(sym);
        // A member's body is typed as the walk types it, whichever expansion or search
        // demanded it: the anonymous classes it makes are named by their own sites, not by the
        // expansion's, and the search's state is the demanding body's.
        let retained = self.is_inline_callee(sym) as u32;
        let ty = if self.made_in_block(self.syms.sym(sym).owner) {
            self.as_own_unit(|t| t.type_retained(retained, |t| t.type_body_in(sym, sig, expected)))
        } else {
            self.as_own_unit(|t| t.outside_search(|t| t.outside_inline(|t| t.outside_quotes(|t| t.type_retained(retained, |t| t.type_body_in(sym, sig, expected))))))
        };
        self.phase_end(p);
        // The body's typing is kept whatever demanded it (an attempt abandoned after it inferred
        // the member's result, an extension given up for a conversion): its marks with it.
        if self.unused.on() {
            self.unused.settle_since(marks);
        }
        // A body is a unit of its own, published when typed: its plain inline calls are kept for
        // the inlining phase at its end (`end_unit`); the body of a member whose type is inferred
        // keeps them once the type is set (`infer_body_then_expand`).
        let file = self.syms.sym(sym).file;
        if self.inferring_depth != self.body_depth {
            self.end_unit(pending, file);
        }
        if self.erased_values.len() > erased_mark {
            let roots: Vec<TExprId> = self.fun_of_sym.get(&sym).and_then(|&f| self.prog.funs[f.idx()].body).into_iter().chain(self.val_init.get(&sym).copied()).collect();
            self.unit_erased_values(erased_mark, &roots, file);
        }
        self.promote_end(promoted);
        if let Some(outer) = outer_unboxes {
            let left = std::mem::replace(&mut self.inline_unboxes, outer);
            self.inline_unboxes.extend(left);
        }
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

    fn type_body_in(&mut self, sym: SymId, sig: &MethodSig, expected: Option<TypeId>) -> TypeId {
        if self.deps.is_none() {
            return self.type_body_in_now(sym, sig, expected);
        }
        self.type_body_recorded(sym, sig, expected)
    }

    #[cold]
    #[inline(never)]
    fn type_body_recorded(&mut self, sym: SymId, sig: &MethodSig, expected: Option<TypeId>) -> TypeId {
        let outer = self.deps_enter_sym(sym, super::deps::Comp::Body(sym));
        let ty = self.type_body_in_now(sym, sig, expected);
        self.deps_leave(outer);
        ty
    }

    fn type_body_in_now(&mut self, sym: SymId, sig: &MethodSig, expected: Option<TypeId>) -> TypeId {
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
        self.env.frames.push(Frame::Locals { names: Vec::new(), tparams, givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: self.sites.owners.len() as u32 + 1 });
        let outer_ext_scope = self.ext_scope;
        if let Some(scope) = self.ext_scope_of(sym, sig, def) {
            self.ext_scope = Some(scope);
        }
        let owners_depth = self.sites.owners.len();
        self.sites.owners.push(SiteOwner::Sym(sym));
        // What the body's typing reports is its own (`Diagnostic::owner`).
        let outer_owner = self.entered_body(sym);
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
        let (body, default_exprs): (Option<ast::ExprId>, Vec<Option<ast::ExprId>>) = match &def.kind {
            DefKind::Val { rhs, .. } => (*rhs, Vec::new()),
            // An inline method's ordinary getters alone: an inline parameter's getter is inline
            // and stays with the stored body.
            DefKind::Fun(f) if self.inline_getters_only => (
                None,
                f.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.default.filter(|_| p.mods & mods::INLINE == 0))).collect(),
            ),
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
        let defaults = self.enter_params(sig, &default_exprs, frame, Some(crate::tir::DefaultOf::Method(sym)), false);
        let is_method = matches!(def.kind, DefKind::Fun(_)) || given_def;
        // The interpreter runs the body of a templated def where it has no builtin for it.
        let has_intrinsic = (self.syms.sym(sym).intrinsic.is_some() || self.body_is_native(sym)) && !((self.interp || self.for_interpreter) && body.is_some());
        let mut result_ty = expected.unwrap_or(ERROR);
        // The body's own type, before its adaptation to the declared result (`rhs.tpe`).
        let mut body_ty = None;
        let outer_inline_val = std::mem::replace(&mut self.inline_val_rhs, def.mods & mods::INLINE != 0 && matches!(def.kind, DefKind::Val { .. }));
        let typed = match body {
            Some(b) if !has_intrinsic => Some(match expected {
                Some(t) if t == self.b.t_unit => {
                    let (e, ty) = self.type_expr(b, None);
                    self.warn_discarded(b, e, ty);
                    body_ty = Some(ty);
                    e
                }
                Some(t) if self.declared_type_inferred(&def.kind) => self.type_inferred_val(b, t).0,
                Some(t) => {
                    let (e, own) = self.check_expr_own(b, t);
                    body_ty = Some(own);
                    e
                }
                None => {
                    let (e, t) = self.type_expr(b, None);
                    body_ty = Some(t);
                    let (mut e, t) = self.apply_inferred_context(b, e, t);
                    // A `final val` of an operation over constants is the constant, as scalac's
                    // `ConstFold` makes it (`final val k = 0 + 0`, an ascribed operand too: its
                    // `ConstantTree` looks through an ascription), which its reads take; one an
                    // ascription widened (`final val k = (0: Int)`) is no constant.
                    let plain_final = def.mods & (mods::FINAL | mods::INLINE | mods::LAZY) == mods::FINAL && matches!(def.kind, DefKind::Val { .. });
                    if plain_final && matches!(self.prog.expr(e), TExpr::Prim(..) | TExpr::Unary(..)) && !self.widened_constant(e) {
                        if let Some(v) = self.fold_constant_as_typed(e) {
                            let v = self.folded_as(v, t);
                            let lit = self.literal_expr(v);
                            self.prog.copy_span(e, lit);
                            let lit_ty = self.types.lit(v);
                            self.prog.set_type(lit, lit_ty);
                            e = lit;
                        }
                    }
                    // A `final val` keeps a literal type, as in Scala: dotty's `inferredResultType`
                    // keeps a `ConstantType` of an `isInlineVal` (final or inline, no method, not
                    // mutable) and widens every other (`widenInferred`).
                    let keeps_constant = def.mods & mods::FINAL != 0 && def.mods & mods::MUTABLE == 0 && matches!(def.kind, DefKind::Val { .. });
                    result_ty = if keeps_constant {
                        // A constant kept, a union widened by its members (`widenInferred`).
                        let t = self.solve_in(t);
                        if matches!(self.types.get(t), Type::Union(..)) { self.widen_unions(t) } else { t }
                    } else {
                        self.solve_inferred(t)
                    };
                    // A path type inferred (`sing(c)` for a `[T <: Singleton]`) widens as well,
                    // but for an object's, which stays its singleton as dotty keeps a module's
                    // `TermRef` (`val chosen = summon[B.u.type]` of a given object `u` is a
                    // `B.u.type`, `val w = summon[B.W.type]` of an object `W` too).
                    if !keeps_constant {
                        result_ty = self.widen_annotated_singleton(result_ty);
                    }
                    let module = match self.types.get(result_ty) {
                        Type::Term(s) | Type::Select(_, s) => self.inner_object_of_sym(s).is_some() || self.given_object_type(s).is_some(),
                        _ => false,
                    };
                    if !keeps_constant && !module && matches!(self.types.get(result_ty), Type::Term(_) | Type::Select(..)) {
                        let widened = self.widen_path(result_ty);
                        result_ty = self.widen_lit(widened);
                    }
                    // A result of a primitive type an erased call gave is the primitive.
                    self.unbox_erased(e, result_ty)
                }
            }),
            _ => None,
        };
        self.inline_val_rhs = outer_inline_val;
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
        if is_method {
            if !has_intrinsic {
                let body_unconsuming = self.unused.defs_on() && body_ty.is_some_and(|t| t == NOTHING || self.fold_type(t).is_some());
                let f = self.prog.add_fun(TFun { sym, params, defaults, body: typed, body_unconsuming, captures: 0, lifted: false });
                self.fun_of_sym.insert(sym, f);
                if def.annots.iter().any(|a| a.name == names::TAILREC) {
                    // The check reads the calls the body's plain inline calls expand to, as
                    // scalac's `TailRec` runs after its `Inlining`: after the phase where the body
                    // keeps calls for it.
                    let file = self.syms.sym(sym).file;
                    if self.attempts.pending_len() > pending && self.program_file(file) {
                        self.inlining.tailrec.push((sym, f, self.env.file, def.span));
                    } else {
                        self.flush_pending_since(pending);
                        self.check_tailrec(sym, f, def.span);
                    }
                }
            }
        } else if let Some(e) = typed {
            #[cfg(debug_assertions)]
            if self.forked && crate::types::view::probing() && self.shared_sym(sym) && e.0 >= crate::arena::LOCAL_BASE && self.source(self.syms.sym(sym).file).is_std {
                crate::types::view::probe(|| format!("the initialiser of the std member {} ({:?}) goes to worker {}'s chunk, depth {}", self.sym_path(sym), sym, self.worker, crate::shared::lock_depth()));
            }
            self.val_init.insert(sym, e);
        }
        self.left_body(outer_owner);
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
                Completion::Suspended => unreachable!("only a signature suspends"),
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
        self.read_jdk_superclasses(c);
        let pending = self.attempts.pending_len();
        let erased_mark = self.erased_values.len();
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
            synthesized: 0,
            retracted_unapply: false,
            lists: None,
            init: Vec::new(),
            methods: Vec::new(),
            ctors: Vec::new(),
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
        let (synthesized, existing_defs) = self.synthesized_record(c);
        self.check_synthesized_conflicts(c, synthesized);
        self.check_companion_apply_defaults(c);
        tclass.synthesized = synthesized;
        // A check that only reports leaves the program's to the interpreter's first use, a macro's
        // (`ensure_mirror_members`), as it leaves the selectors' bodies (`emits_bodies`); a shared
        // class's are made here, under the loader's lock.
        let mirror = if self.emits_bodies || self.shared_class(c) { self.mirror_members(c) } else { Vec::new() };
        if !mirror.is_empty() {
            tclass.lists_mut().mirror_members = mirror;
        }
        if !existing_defs.is_empty() {
            tclass.lists_mut().existing_defs = existing_defs;
        }
        if tclass.synthesized & synthetic::EQUALS != 0 && crate::tir::equals_reorders(&self.syms, &self.types, c) {
            let order = crate::tir::equals_order(&self.syms, &self.types, c);
            tclass.lists_mut().equals_order = order.into_iter().map(|k| k as u16).collect();
        }
        // A pickle's synthesized companion of a case class, its body read, and the one the parser
        // makes for a `derives` clause: the rule's `toString`.
        if def_id.is_some() && (self.syms.class(c).made_companion || self.syms.class(c).synthetic_companion || self.derives_companion(c)) {
            if let Some(f) = self.case_companion_to_string(c) {
                tclass.methods.push(f);
            }
        }
        let Some(def_id) = def_id else {
            // The companion the namer made (an enum case's, a nested class's): its template is its
            // class's constructor getters (`Desugar.classDef`'s `companionMembers`), reached here
            // as a written companion's are below, their trees the class's record's at `fill_getters`.
            if let Some(k) = self.syms.class(c).companion.filter(|&k| self.syms.class(k).kind != ClassKind::Object) {
                self.reach_class_getters(k);
            }
            // A companion an enum made holds its values, and the outer accessor its cases read
            // their enum's enclosing instance through.
            let values = self.instance_enum_values(c);
            tclass.init.splice(0..0, values);
            if let Some(f) = self.case_companion_to_string(c) {
                tclass.methods.push(f);
            }
            if kind != ClassKind::Trait && self.syms.class(c).inner_object.is_some() {
                let span = self.syms.class(c).span;
                let env = self.env_at(file, Owner::Class(c), span.start);
                let accessors = self.with_env(env, |t| t.define_outer_accessors(c));
                tclass.methods.extend(accessors);
            }
            self.push_class_body(tclass);
            return;
        };
        let def = self.ast(file).def(def_id);
        let (body, clauses, parents): (&[Stmt], &[ast::ParamClause], &[ast::Parent]) = match &def.kind {
            DefKind::Class(cls) => (&cls.body, &cls.clauses, &cls.parents),
            DefKind::Given(g) => (&g.body, &g.clauses, &g.parents[..]),
            _ => (&[], &[], &[]),
        };
        if kind == ClassKind::EnumCase && self.unused.on() {
            self.mark_enum_case_parents(c, file, parents);
        }
        if let DefKind::Class(cls) = &def.kind {
            // Errors in export clauses are reported even when nothing looks the exports up.
            if !cls.exports.is_empty() {
                self.exports_of(c);
            }
        }
        // A call of the class's header (a parent's arguments, a parameter's default) and of each
        // statement is the inlining phase's at that place, as `InliningTreeMap` walks the template.
        let statement = std::mem::replace(&mut self.inlining.statement_place, Some((c, super::inlining::StatementPlace::At(def.span.start))));
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
            tclass.retracted_unapply = self.synthetic_unapply_retracted(c);
        }
        self.selector_bodies(c, &mut tclass);
        // A companion object's template begins with its class's constructor getters
        // (`Desugar.classDef`'s `companionMembers`, merged before its own statements): a
        // top-level or nested object's, and an inner object's (one nested in a class, whose class
        // is an inner class with its module val, `inner_object`).
        if self.syms.class(c).kind == ClassKind::Object || self.syms.class(c).inner_object.is_some() {
            if let Some(k) = self.syms.class(c).companion.filter(|&k| self.syms.class(k).kind != ClassKind::Object && self.syms.class(k).inner_object.is_none()) {
                self.reach_class_getters(k);
            }
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
            t.env.frames.push(Frame::Locals { names, tparams: Vec::new(), givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: t.sites.owners.len() as u32 });
            // A default stands by its own parameter, after the outer instance an inner class
            // takes first (`ExplicitOuter`), as `Desugar.classDef`'s getters are its parameters'.
            let lead = tclass.captures;
            let mut i = 0;
            for clause in clauses {
                for p in &clause.params {
                    if let Some(d) = p.default {
                        // A program class's default is its getter's, completed at its companion's
                        // template (`Desugar.classDef`'s `companionMembers`), never here: its tree
                        // where it is complete, filled in later otherwise (`fill_getters`).
                        if t.getter_of(crate::tir::DefaultOf::Ctor(c), i).is_some() {
                            tclass.ctor_defaults[lead + i] = t.getter_tree(crate::tir::DefaultOf::Ctor(c), i);
                            i += 1;
                            continue;
                        }
                        let (ty, by_name) = param_types[i];
                        let te = t.check_expr(d, ty);
                        tclass.ctor_defaults[lead + i] =
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
                t.inlining.statement_place = Some((c, match *stmt {
                    Stmt::Expr(e) => super::inlining::StatementPlace::Expr(file, e),
                    Stmt::Def(d) => super::inlining::StatementPlace::Def(file, d),
                    Stmt::Import(_) => super::inlining::StatementPlace::At(def.span.start),
                }));
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
                if tclass.synthesized & synthetic::EQUALS != 0 && t.syms.class(c).mods & mods::CASE != 0 {
                    let own = t.outer_accessors.get(&c).copied();
                    if let Some(own) = own.filter(|&own| accessors.iter().any(|&f| t.prog.funs[f.idx()].sym == own)) {
                        tclass.lists_mut().eq_outer = Some(own);
                    }
                }
                tclass.methods.extend(accessors);
            }
            let forwarders = t.define_value_forwarders(c);
            tclass.methods.extend(forwarders);
            t.sites.owners.truncate(owners_depth);
            t.return_to = outer_return;
        });
        self.inlining.statement_place = statement;
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
        // A local enum's cases and companion are checked once the enum's captures are known
        // (`Worker::finish_local_enum`).
        if kind == ClassKind::Enum && self.syms.class(c).owner != Owner::Local {
            let children = self.syms.class(c).children.clone();
            for child in children {
                self.check_class(child);
            }
            // A synthesized companion holds the values of a stateful enum, and the `$values` and
            // lookups of any (dotty's `enumScaffolding`); it is emitted where it is reached. One
            // read from the products is the library's, its body converted where it is reached.
            let info = self.syms.class(c);
            if let (false, Some(companion)) = (info.children.is_empty(), info.companion) {
                if self.syms.class(companion).def.is_none() && !self.is_library_class(companion) {
                    self.check_class(companion);
                }
            }
        }
        self.check_overrides(c);
        if self.jvm && kind != ClassKind::Trait {
            self.complete_forwarded_members(c);
        }
        self.check_variances(c);
        if !matches!(kind, ClassKind::Trait | ClassKind::Enum) && def.mods & mods::ABSTRACT == 0 && !self.is_body_file(file) && !self.syms.class(c).parent_failed {
            let implemented = tclass.synthesized | self.enum_value_parent_members(c) | self.inherited_synthesized(c);
            self.check_implements_all(c, Abstracts::Declared, implemented);
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
        if self.capturing() && self.syms.class(c).owner != Owner::Local {
            self.capture_class_annotations(c, file, def_id);
        }
        // The values of an enum whose companion is an instance (a local object, an object
        // nested in a class), made first in its constructor as dotty's
        // `DesugarEnums.expandSimpleEnumCase` places them.
        let values = self.instance_enum_values(c);
        tclass.init.splice(0..0, values);
        // The class's `TClass` is published once pushed: the plain inline calls of its statements
        // are kept for the inlining phase first.
        self.end_unit(pending, file);
        if self.erased_values.len() > erased_mark {
            let roots: Vec<TExprId> = tclass.init.iter().filter_map(|i| match *i {
                TInit::Field(_, e) | TInit::Stmt(e) => Some(e),
                TInit::Parent(..) => None,
            }).chain(tclass.parent_args.iter().flat_map(|&l| self.prog.expr_list(l).to_vec())).collect();
            self.unit_erased_values(erased_mark, &roots, file);
        }
        // A class whose companion is the one desugaring makes (the parser's synthetic one, or the
        // namer's without a definition, an enum case's or a nested class's, which no walk checks)
        // has its constructor getters there, right after the class (`Desugar.classDef`'s
        // `companionDefs`); their trees are the record's before it is published.
        let made = |t: &Self, k: ClassId| t.syms.class(k).synthetic_companion || (t.syms.class(k).made_companion && t.syms.class(k).def.is_none());
        if self.syms.class(c).kind != ClassKind::Object && self.syms.class(c).companion.map_or(true, |k| made(self, k)) {
            self.reach_class_getters(c);
            let lead = tclass.captures;
            for i in 0..tclass.ctor_defaults.len().saturating_sub(lead) {
                if tclass.ctor_defaults[lead + i].is_none() {
                    if let Some(tree) = self.getter_tree(crate::tir::DefaultOf::Ctor(c), i) {
                        tclass.ctor_defaults[lead + i] = Some(tree);
                    }
                }
            }
            for &f in &tclass.ctors {
                let ctor = self.prog.funs[f.idx()].sym;
                for i in 0..self.prog.funs[f.idx()].defaults.len() {
                    if self.prog.funs[f.idx()].defaults[i].is_none() {
                        if let Some(tree) = self.getter_tree(crate::tir::DefaultOf::Method(ctor), i) {
                            self.prog.funs[f.idx()].defaults[i] = Some(tree);
                        }
                    }
                }
            }
        }
        self.push_class_body(tclass);
        // A local trait's members take the values of the block they read, which the classes
        // mixing it in pass (`proxies.rs`): after the inlining phase where its trees hold calls
        // kept for it (`defer_trait_lifting`), as dotty's `LambdaLift` runs after `Inlining`.
        if kind == ClassKind::Trait {
            if !self.defer_trait_lifting(c) {
                self.lift_trait_proxies(c);
            }
        } else {
            self.add_trait_forwarders(c);
        }
    }

    /// The initialisers of the values of the enum whose companion the class `m` is, when it is an
    /// instance: each a creation of its case with this instance as its outer; and of the lazy
    /// vals of its class cases' companions (`enter_enum_case_companion`), objects of this
    /// instance as a case class's made companion in a class body is.
    pub(super) fn instance_enum_values(&mut self, m: ClassId) -> Vec<TInit> {
        let info = self.syms.class(m);
        let Some(e) = info.companion.filter(|&e| self.syms.class(e).kind == ClassKind::Enum) else { return Vec::new() };
        if info.local_module.is_none() && info.inner_object.is_none() {
            return Vec::new();
        }
        let children = self.syms.class(e).children.clone();
        let mut values = Vec::new();
        let mut companions = Vec::new();
        self.env.frames.push(Frame::Class(m));
        for case in children {
            let span = self.syms.class(case).span;
            let Some(v) = self.syms.class(case).singleton else {
                let made = self.syms.class(case).companion.filter(|&o| self.syms.class(o).made_companion && self.syms.class(o).def.is_none());
                if let Some((o, val)) = made.and_then(|o| self.syms.class(o).inner_object.map(|v| (o, v))) {
                    // A stored body's enum keeps it with its cases (`finish_local_enum`).
                    self.note_nested_stored_class(m, o);
                    self.check_class(o);
                    let init = self.new_instance(o, ast::ListRef::EMPTY, span);
                    companions.push(TInit::Field(val, init));
                }
                continue;
            };
            let made = self.new_instance(case, ast::ListRef::EMPTY, span);
            let ty = self.syms.this_type(case);
            self.prog.set_type(made, ty);
            values.push(TInit::Field(v, made));
        }
        self.env.frames.pop();
        // The cases' companions after the values, which come first (above), as the companion's
        // template read from a pickle runs them.
        values.extend(companions);
        values
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

    /// The members dotty's `SyntheticMembers.caseAndValueMethods` gives `c` (`TClass::synthesized`),
    /// with an enum class case's `ordinal`: of its kind's set, those `existingDef` finds no
    /// definition of.
    pub(super) fn synthesized_members(&mut self, c: ClassId) -> u16 {
        self.synthesized_record(c).0
    }

    /// `synthesized_members` with, for each member of the kind's set that is not synthesized, the
    /// existing definition `existingDef` found (`TClass::existing_defs`).
    pub(super) fn synthesized_record(&mut self, c: ClassId) -> (u16, Vec<(u16, SymId)>) {
        let (kind, is_case, singleton, value_class, is_module) = {
            let i = self.syms.class(c);
            let is_module = i.kind == ClassKind::Object || i.local_module.is_some() || i.inner_object.is_some();
            (i.kind, i.mods & mods::CASE != 0, i.singleton.is_some(), i.value_class, is_module)
        };
        let wanted = if kind == ClassKind::EnumCase && singleton {
            if self.is_java_enum_value(c) { synthetic::PRODUCT_PREFIX } else { synthetic::ENUM_VALUE }
        } else if kind == ClassKind::EnumCase {
            // A class case's `ordinal` is `Desugar.enumCaseMeths`'.
            synthetic::CASE | synthetic::ORDINAL
        } else if is_case {
            if is_module { synthetic::CASE & !synthetic::EQUALS } else { synthetic::CASE }
        } else if value_class {
            synthetic::VALUE
        } else {
            0
        };
        if wanted == 0 {
            return (0, Vec::new());
        }
        let superclass = self.syms.class(c).superclass;
        let held = self.held_rule_names(c);
        // A member no base can hold a definition of is synthesized without a lookup.
        let mut synthesized = wanted & !held;
        let mut existing = Vec::new();
        let lookups: &[(u16, Name, synthetic::Param)] = if wanted & held == 0 { &[] } else { &synthetic::MEMBERS };
        for &(bit, name, param) in lookups {
            if wanted & held & bit == 0 {
                continue;
            }
            let found = self.existing_def_among(c, name, param);
            if let Some(s) = found {
                existing.push((bit, s));
            }
            match found {
                None => synthesized |= bit,
                Some(s) if bit & (synthetic::HASH_CODE | synthetic::EQUALS | synthetic::TO_STRING) != 0 => {
                    let definer = match self.syms.sym(s).owner {
                        Owner::Class(k) => Some(k),
                        _ => None,
                    };
                    let inherited = definer.is_some_and(|k| {
                        k != c && superclass.is_some_and(|p| self.syms.class(p).base_types.iter().any(|&(x, _)| x == k))
                    });
                    if inherited {
                        synthesized |= bit << synthetic::INHERITED;
                    }
                }
                Some(_) => {}
            }
        }
        if synthesized & synthetic::EQUALS != 0 {
            // A value class is final (`Namer`).
            let is_final = self.syms.class(c).mods & mods::FINAL != 0 || value_class;
            if !is_final || (held & synthetic::CAN_EQUAL != 0 && self.existing_def_among(c, names::CAN_EQUAL_METHOD, synthetic::Param::Any).is_some()) {
                synthesized |= synthetic::EQUALS_CALLS_CAN_EQUAL;
            }
        }
        (synthesized, existing)
    }

    /// The `toString` member of the companion `m` the typer or the parser makes for a case class,
    /// entered when the companion is, so that a selection of it types alike whenever the
    /// companion is checked (its body, `case_companion_to_string`).
    pub(super) fn made_companion_to_string(&mut self, m: ClassId) -> SymId {
        if let Some(&s) = self.syms.class(m).members.get(&names::TO_STRING) {
            if self.syms.sym(s).owner == Owner::Class(m) {
                return s;
            }
        }
        let (file, span) = (self.syms.class(m).file, self.syms.class(m).span);
        let ret = self.b.t_string;
        let s = self.syms.new_sym(names::TO_STRING, SymKind::Def, mods::OVERRIDE, Owner::Class(m), file, None, span);
        let mut info = self.syms.sym_mut(s);
        info.sig = Some(Arc::new(MethodSig { tparams: Vec::new(), clauses: vec![ClauseSig { params: Vec::new(), is_using: false, is_implicit: false }], ret, declared: crate::tir::NO_TYPE }));
        info.state().set(Completion::Done);
        self.syms.add_member(m, s);
        s
    }

    /// Whether the object `m` is the companion the parser makes for a case class's `derives`
    /// clause where the source writes none (`add_derived_givens`, at the class's place): the
    /// companion `Desugar.classDef` gives, holding the derived givens.
    pub(super) fn derives_companion(&self, m: ClassId) -> bool {
        let info = self.syms.class(m);
        let (Some(d), Some(k)) = (info.def, info.companion) else { return false };
        let ki = self.syms.class(k);
        ki.kind == ClassKind::Class && ki.mods & mods::CASE != 0 && !self.in_jar(info.file) && self.ast(info.file).def(d).span == ki.span
    }

    /// The `toString` of the companion `Desugar.classDef` gives a case class without a written one,
    /// the class's name (`companionDefs`' `toString`): a member of the object `m`, defined here.
    fn case_companion_to_string(&mut self, m: ClassId) -> Option<FunId> {
        if !self.syms.class(m).made_companion && !self.syms.class(m).synthetic_companion && !self.derives_companion(m) {
            return None;
        }
        let k = self.syms.class(m).companion?;
        let info = self.syms.class(k);
        if !matches!(info.kind, ClassKind::Class | ClassKind::EnumCase) || info.mods & mods::CASE == 0 {
            return None;
        }
        let name = info.name;
        // A pickle's synthesized companion declares it, its body left out with the rule's; the
        // typer's has it from its making (`made_companion_to_string`).
        let declared = self.syms.class(m).members.get(&names::TO_STRING).copied().filter(|&s| self.syms.sym(s).owner == Owner::Class(m) && self.syms.sym(s).kind == SymKind::Def);
        let s = match declared {
            Some(s) => s,
            None if self.shared_class(m) => self.with_loader(|w| w.made_companion_to_string(m)),
            None => self.made_companion_to_string(m),
        };
        let text = self.name_str(name).to_string();
        let lit = self.prog.add_str(&text);
        let body = self.prog.add(TExpr::Str(lit));
        let f = self.prog.add_fun(TFun { sym: s, params: Vec::new(), defaults: Vec::new(), body: Some(body), body_unconsuming: false, captures: 0, lifted: false });
        self.fun_of_sym.insert(s, f);
        Some(f)
    }

    /// dotty's `checkNoDoubleDeclaration` over the written companion `o` of a case class whose
    /// constructor has defaults: the `apply` `Desugar.classDef` gives the companion takes them, so
    /// a written `apply` of other parameters with defaults of its own is a second overloaded variant
    /// with defaults; one of the constructor's parameter types replaces the synthesized one.
    fn check_companion_apply_defaults(&mut self, o: ClassId) {
        let info = self.syms.class(o);
        let Some(k) = info.companion.filter(|_| info.def.is_some() && (info.kind == ClassKind::Object || info.inner_object.is_some())) else { return };
        let ki = self.syms.class(k);
        if ki.kind != ClassKind::Class || ki.mods & mods::CASE == 0 || ki.mods & mods::ABSTRACT != 0 {
            return;
        }
        // The synthesized `apply` takes the class's type parameters and every constructor clause,
        // its defaults those of any clause (`Desugar.classDef`).
        let ctor: Vec<Vec<TypeId>> = ki.ctor.iter().map(|c| c.params.iter().map(|p| p.ty).collect()).collect();
        if !ki.ctor.iter().any(|c| c.params.iter().any(|p| p.has_default)) {
            return;
        }
        let class_tparams = ki.tparams.clone();
        let mut k_alt = 0;
        while let Some(a) = self.own_alternative(o, names::APPLY, k_alt) {
            k_alt += 1;
            let sig = self.parameters_of(a).clone();
            if !sig.clauses.iter().any(|c| c.params.iter().any(|p| p.has_default)) {
                continue;
            }
            // A written `apply` that matches the synthesized one loosely retracts it
            // (`Namer.invalidateIfClashingSynthetic`'s `matchesLoosely`): as many type parameters,
            // every clause's parameter types the same, the method's parameters read as the class's.
            let subst: Subst = sig.tparams.iter().zip(&class_tparams).map(|(&m, &c)| (m, self.types.param(c))).collect();
            let replaces = sig.tparams.len() == class_tparams.len()
                && sig.clauses.len() == ctor.len()
                && sig.clauses.iter().zip(&ctor).all(|(clause, types)| {
                    clause.params.len() == types.len()
                        && clause.params.iter().zip(types).all(|(p, &t)| {
                            let pt = self.types.subst(p.ty, &subst);
                            pt == t || {
                                let mark = self.snapshot();
                                let same = self.is_sub(pt, t) && self.is_sub(t, pt);
                                self.rollback(&mark);
                                same
                            }
                        })
                });
            if replaces {
                continue;
            }
            let (file, span) = (self.syms.sym(a).file, self.syms.sym(a).span);
            self.diags.error(file, span, "two or more overloaded variants of method apply have default arguments".to_string());
        }
    }

    /// A member the rule synthesizes beside a deferred declaration of `c`'s own that it matches:
    /// `existingDef` takes no deferred definition, so dotty enters the synthesized one next to the
    /// declaration and erasure's `checkNoDoubleDeclaration` refuses the pair (E120). A concrete
    /// class with such a declaration is refused by `RefChecks` first ("needs to be abstract"),
    /// after which scalac's erasure never runs.
    fn check_synthesized_conflicts(&mut self, c: ClassId, synthesized: u16) {
        if synthesized == 0 || self.syms.class(c).mods & mods::ABSTRACT == 0 {
            return;
        }
        for &(bit, name, param) in &synthetic::MEMBERS {
            if synthesized & bit == 0 {
                continue;
            }
            let mut k = 0;
            while let Some(s) = self.own_alternative(c, name, k) {
                k += 1;
                if self.is_abstract_member(s) && !self.is_private(s) && self.erases_as_synthesized(s, name, param) {
                    self.report_synthesized_double_definition(c, bit, s);
                }
            }
        }
    }

    /// Whether the rule gives `c` a member named `name` (`synthesized_members`), which is then the
    /// class's own and overrides what it inherits of the name (dotty's synthesized
    /// `productPrefix` over `Product`'s and a trait's).
    pub(super) fn record_synthesizes(&mut self, c: ClassId, name: Name) -> bool {
        let Some(&(bit, _, _)) = synthetic::MEMBERS.iter().find(|&&(_, n, _)| n == name) else { return false };
        let info = self.syms.class(c);
        if info.mods & mods::CASE == 0 && !info.value_class && info.kind != ClassKind::EnumCase {
            return false;
        }
        self.synthesized_members(c) & bit != 0
    }

    /// The members the rule gives a case class, value class or enum case `c` derives from
    /// (`class D extends K` of a case class `K`), which implement what `Product` declares.
    /// Such a base is a class, an enum case or an object being no parent, so it is one of the
    /// superclasses of `c`.
    fn inherited_synthesized(&mut self, c: ClassId) -> u16 {
        let mut by_rule = 0;
        let mut next = self.syms.class(c).superclass;
        for _ in 1..self.syms.class(c).base_types.len() {
            let Some(b) = next else { break };
            let info = self.syms.class(b);
            next = info.superclass;
            let synthesizes = matches!(info.kind, ClassKind::Class | ClassKind::Object | ClassKind::EnumCase)
                && (info.mods & mods::CASE != 0 || info.value_class || info.kind == ClassKind::EnumCase);
            if synthesizes {
                by_rule |= self.synthesized_members(b) | self.enum_value_parent_members(b);
            }
        }
        by_rule
    }

    /// The members `scala.runtime.EnumValue` gives a value case, a parent of every one
    /// (`DesugarEnums.enumValueCreator`), which teq leaves to the runtime's rule.
    fn enum_value_parent_members(&self, c: ClassId) -> u16 {
        let info = self.syms.class(c);
        if info.kind == ClassKind::EnumCase && info.singleton.is_some() {
            synthetic::CAN_EQUAL | synthetic::PRODUCT_ARITY | synthetic::PRODUCT_ELEMENT | synthetic::PRODUCT_ELEMENT_NAME
        } else {
            0
        }
    }

    /// dotty's `existingDef`: the member of `c` the synthesized one would be, `matchingMember`'s
    /// first concrete definition of its name and parameter in the linearisation (a lookup takes a
    /// concrete member over an abstract one of an unrelated parent, while an abstract
    /// redeclaration hides what it overrides), unless that is the declaration the rule
    /// implements (`Any`'s, `Product`'s, `Equals`'s, `Enum`'s).
    /// Asked only for a name some base that can hold a definition has (`held_rule_names`).
    fn existing_def_among(&mut self, c: ClassId, name: Name, param: synthetic::Param) -> Option<SymId> {
        let mut abstract_in: Vec<ClassId> = Vec::new();
        for i in 0..self.syms.class(c).base_types.len() {
            let b = self.syms.class(c).base_types[i].0;
            let bt = self.base_instance_at(c, i);
            let mut k = 0;
            while let Some(s) = self.own_alternative(b, name, k) {
                k += 1;
                if self.is_private(s) || !self.matches_synthesized_at(s, param, b, bt) {
                    continue;
                }
                if self.is_abstract_member(s) {
                    abstract_in.push(b);
                    continue;
                }
                // `trait R extends H { override def hashCode: Int }`: the member is R's, deferred.
                if abstract_in.iter().any(|&a| self.syms.class(a).base_types.iter().any(|&(x, _)| x == b)) {
                    return None;
                }
                return (!self.declares_by_rule(b)).then_some(s);
            }
        }
        None
    }

    /// Which of the names `synthetic::MEMBERS` gives (their bits) a base of `c` that can hold an
    /// existing definition has: none whose declarations the rule implements, none without members
    /// (`java.io.Serializable`). `existingDef` of another name is none (a case class over `Product`
    /// and `Any` alone), which is told without a lookup per name.
    fn held_rule_names(&self, c: ClassId) -> u16 {
        // The rule's names as a names mask: a base whose mask has none of them holds none.
        const RULE_NAMES: u64 = {
            let (mut mask, mut i) = (0, 0);
            while i < synthetic::MEMBERS.len() {
                mask |= crate::symbols::name_bit(synthetic::MEMBERS[i].1);
                i += 1;
            }
            mask
        };
        let mut held = 0;
        // The leading classes `Desugar.classDef` gives (`rule_prefix`) are the rule's own.
        let info = self.syms.class(c);
        for &(b, _) in std::iter::once(&info.base_types[0]).chain(&info.base_types[1 + info.rule_prefix as usize..]) {
            let bi = self.syms.class(b);
            if bi.names_mask & RULE_NAMES == 0 || self.declares_by_rule(b) {
                continue;
            }
            for &(bit, name, _) in &synthetic::MEMBERS {
                if bi.may_have(name) && bi.members.contains_key(&name) {
                    held |= bit;
                }
            }
        }
        held
    }

    /// Whether `b` is a class whose declarations the rule implements (`Any`'s, `Object`'s,
    /// `Product`'s, `Equals`'s, `Enum`'s).
    fn declares_by_rule(&self, b: ClassId) -> bool {
        b == self.b.any_ref || Some(b) == self.b.product || Some(b) == self.equals_class() || Some(b) == self.b.reflect_enum || self.is_java_lang_object(b)
    }

    /// Whether `m` has the name and the signature, at its own class, of a member of `Product` or
    /// `Equals` the rule gives (`productElement(n: Int)`, `canEqual(that: Any)`): the member the
    /// runtime's product functions call by its plain name (`$productElement` and kin).
    pub(super) fn matches_product_member(&mut self, m: SymId) -> bool {
        let name = self.syms.sym(m).name;
        let product = synthetic::PRODUCT_ARITY | synthetic::PRODUCT_ELEMENT | synthetic::PRODUCT_PREFIX | synthetic::PRODUCT_ELEMENT_NAME | synthetic::CAN_EQUAL;
        synthetic::MEMBERS.iter().any(|&(bit, n, param)| bit & product != 0 && n == name && self.matches_synthesized(m, param))
    }

    /// Whether `s` has the signature of a member taking `param` by rule: no type parameters, no
    /// parameter or one empty list (`toString()`), or the one parameter of that type.
    fn matches_synthesized(&mut self, s: SymId, param: synthetic::Param) -> bool {
        self.matches_synthesized_under(s, param, &Subst::default())
    }

    /// `matches_synthesized` of `s`, a member of the base `b` that the class sees as `bt`: its
    /// signature at the class site, `b`'s type parameters taken as `bt`'s arguments (dotty's
    /// `matchingMember`, `trait T[A] { def productElement(i: A) }` under `T[Int]`).
    /// Whether the declaration `s` has the erased signature of the member the rule synthesizes
    /// under `name` (`ElimErasedValueType.checkNoClashes`' `sym1.signature == sym2.signature`): its
    /// parameter lists joined and erased, and its result erased, type parameters gone
    /// (`def toString[A]: String` erases as `toString(): String`).
    fn erases_as_synthesized(&mut self, s: SymId, name: Name, param: synthetic::Param) -> bool {
        // The pairs are of one effective name: a declaration `@targetName` renames meets none.
        if self.declared_target_name(s).is_some_and(|t| t != name) {
            return false;
        }
        let erased = |w: &mut Self, t: TypeId| {
            let mut out = String::new();
            w.erased_name(t, true, 0, &mut out);
            out
        };
        let params = match param {
            synthetic::Param::None => "0".to_string(),
            synthetic::Param::Any => erased(self, ANY),
            synthetic::Param::Int => erased(self, self.b.t_int),
        };
        let result = match name {
            names::EQUALS | names::CAN_EQUAL_METHOD => self.b.t_boolean,
            names::TO_STRING | names::PRODUCT_PREFIX | names::PRODUCT_ELEMENT_NAME => self.b.t_string,
            names::PRODUCT_ELEMENT => ANY,
            _ => self.b.t_int,
        };
        let result = erased(self, result);
        self.erased_signature(s, true) == params && self.erased_result(s, true) == result
    }

    fn matches_synthesized_at(&mut self, s: SymId, param: synthetic::Param, b: ClassId, bt: TypeId) -> bool {
        let subst: Subst = match self.types.get(bt) {
            Type::Class(_, args) => self.syms.class(b).tparams.iter().copied().zip(self.types.items(args).iter().copied()).collect(),
            _ => Subst::default(),
        };
        self.matches_synthesized_under(s, param, &subst)
    }

    fn matches_synthesized_under(&mut self, s: SymId, param: synthetic::Param, subst: &Subst) -> bool {
        let sig = self.sig_arc(s);
        if !sig.tparams.is_empty() {
            return false;
        }
        let param_ty = |w: &mut Self, t: TypeId| {
            let t = if subst.is_empty() { t } else { w.types.subst(t, subst) };
            w.deref_alias(t)
        };
        match (param, sig.clauses.as_slice()) {
            (synthetic::Param::None, []) => true,
            (synthetic::Param::None, [cl]) => cl.params.is_empty(),
            (synthetic::Param::Any, [cl]) if !cl.is_using && cl.params.len() == 1 => {
                let t = param_ty(self, cl.params[0].ty);
                t == ANY || t == self.b.t_any_ref || self.class_of(t).is_some_and(|k| self.is_java_lang_object(k))
            }
            (synthetic::Param::Int, [cl]) if !cl.is_using && cl.params.len() == 1 => param_ty(self, cl.params[0].ty) == self.b.t_int,
            _ => false,
        }
    }

    /// Whether the value case `c` is one of an enum extending `java.lang.Enum`.
    pub(super) fn is_java_enum_value(&self, c: ClassId) -> bool {
        let Owner::Class(companion) = self.syms.class(c).owner else { return false };
        let Some(e) = self.syms.class(companion).companion else { return false };
        self.syms.class(e).superclass.is_some_and(|s| {
            let info = self.syms.class(s);
            self.interner.get(info.name) == "Enum" && matches!(info.owner, Owner::Package(p) if self.pkg_is(p, "java.lang"))
        })
    }

    /// Whether `Any`'s concrete `equals`, `hashCode` or `toString` implements the abstract member
    /// `m` (scala's `Equals.equals`), as the member a lookup finds over a deferred one.
    pub(super) fn implemented_by_any(&mut self, m: SymId) -> bool {
        self.synthesized_by_rule(m, synthetic::HASH_CODE | synthetic::EQUALS | synthetic::TO_STRING)
    }

    /// Whether the member `m` is one of `synthesized`, by its name and parameter.
    fn synthesized_by_rule(&mut self, m: SymId, synthesized: u16) -> bool {
        if synthesized == 0 {
            return false;
        }
        let name = self.syms.sym(m).name;
        synthetic::MEMBERS.iter().any(|&(bit, n, param)| synthesized & bit != 0 && n == name && self.matches_synthesized(m, param))
    }

    /// `java.lang.Object`, read from a jar or the std's.
    fn is_java_lang_object(&self, k: ClassId) -> bool {
        let info = self.syms.class(k);
        info.name == names::OBJECT && matches!(info.owner, Owner::Package(p) if self.pkg_is(p, "java.lang"))
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
        // The parents the class writes, which the desugaring's follow (`desugared_parents`).
        let written = {
            let info = self.syms.class(c);
            info.parents.len() - info.desugared_parents as usize
        };
        let all_accepted = written >= parents.len();
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
        let parent_ty = self.base_type_of(c, superclass)?;
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
        outside.frames.push(Frame::Locals { names, tparams, givens, classes: Vec::new(), aliases: Vec::new(), owner: self.sites.owners.len() as u32 });
        let enum_class = match (kind, owner) {
            (ClassKind::EnumCase, Owner::Class(companion)) => self.syms.class(companion).companion,
            _ => None,
        };
        let filler = enum_class.or(self.syms.class(c).superclass);
        let filled_above =
            |t: &Self, b: ClassId| filler.map_or(false, |e| t.syms.class(e).base_types.iter().any(|&(x, _)| x == b));
        let bases: Vec<(ClassId, TypeId)> = self.met_base_types(c).into_iter().skip(1).filter(|&(b, _)| self.has_trait_params(b)).collect();
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
            // A trait through a prefix (`H.o.I`) takes its own arguments as the bare trait does.
            let class_ty = match self.types.get(base_ty) {
                Type::Nested(_, class) => class,
                _ => base_ty,
            };
            let Type::Class(_, targs) = self.types.get(class_ty) else { continue };
            self.settle_class(b);
            let owner_subst: Subst =
                self.syms.class(b).tparams.iter().copied().zip(self.types.items(targs).iter().copied()).collect();
            let sig = Arc::new(MethodSig { tparams: Vec::new(), clauses: self.syms.class(b).ctor.clone(), ret: base_ty, declared: crate::tir::NO_TYPE });
            let span = clause.map_or(class_span, |p| ast.ty_spans[p.ty.idx()]);
            let mut lists: Vec<ArgList> = clause
                .map(|p| {
                    p.args
                        .iter()
                        .enumerate()
                        .filter(|&(_, &(_, using))| using || !evidence_only)
                        .map(|(k, &(l, using))| ArgList {
                            args: ast.expr_list(l).iter().map(|&a| ArgSrc::Ast(a)).collect(),
                            using,
                            span,
                            fun: super::apply::FunEnd::at(p.fun_end(ast, k)),
                        })
                        .collect()
                })
                .unwrap_or_default();
            if !evidence_only && !lists.iter().any(|l| !l.using) {
                lists.insert(0, ArgList { args: Vec::new(), using: false, span, fun: ArgList::NO_FUN });
            }
            let call = MethodCall { recv: None, sym: SymId(u32::MAX), owner_subst, ext_recv: None, prefix: None, refined: None };
            let applied = self.with_env(outside.clone(), |t| {
                let outer = t.parent_args_of.replace(c);
                let r = t.apply_method(call, Some((b, sig.clone(), None)), None, lists, span, None, false);
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
    pub(super) fn trait_has_statements(&self, b: ClassId) -> bool {
        let info = self.syms.class(b);
        info.kind == ClassKind::Trait && info.has_statements && info.js == JsKind::Scala
    }

    pub(super) fn inherits_trait_statements(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        info.base_types[1 + info.rule_prefix as usize..].iter().any(|&(b, _)| self.trait_has_statements(b))
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
        // The traits `Desugar.classDef` gives (`rule_prefix`) take no arguments and run nothing.
        for &(b, _) in info.base_types[1 + info.rule_prefix as usize..].iter().rev() {
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
        let class_of_parent = |w: &Self, t: TypeId| match w.types.get(t) {
            Type::Nested(_, class) => w.types.get(class),
            other => other,
        };
        let (sig_tparams, owner_subst, ret) = match parent_ty.map(|t| (t, class_of_parent(self, t))) {
            Some((t, Type::Class(_, targs))) => {
                let tparams = self.syms.class(parent).info.tparams.iter().copied();
                (Vec::new(), tparams.zip(self.types.items(targs).iter().copied()).collect(), t)
            }
            _ => (self.syms.class(parent).tparams.clone(), Vec::new(), self.syms.class(parent).base_types[0].1),
        };
        let sig = Arc::new(MethodSig { tparams: sig_tparams, clauses: self.syms.class(parent).ctor.clone(), ret, declared: crate::tir::NO_TYPE });
        // A parent through a prefix (`Ops` of `Num[T]` in `Integ[T] extends Num[T]`, which is
        // `Integ.this.Ops`) takes its constructor's parameters seen from that prefix, the
        // enclosing classes' arguments the prefix's, as dotty's parent constructor type.
        let prefix = match parent_ty.map(|t| self.types.get(t)) {
            Some(Type::Nested(p, _)) => Some(p),
            _ => None,
        };
        let sig = match prefix {
            Some(p) => self.ctor_sig_through(sig, p, parent),
            None => sig,
        };
        let span = clause.map_or(class_span, |p| ast.ty_spans[p.ty.idx()]);
        let mut lists: Vec<ArgList> = clause
            .map(|p| {
                p.args
                    .iter()
                    .enumerate()
                    .map(|(k, &(l, using))| ArgList {
                        args: ast.expr_list(l).iter().map(|&a| ArgSrc::Ast(a)).collect(),
                        using,
                        span,
                        fun: super::apply::FunEnd::at(p.fun_end(ast, k)),
                    })
                    .collect()
            })
            .unwrap_or_default();
        // A Java class has its constructors as alternatives only, each with a parameter list.
        let has_normal_clause = sig.clauses.iter().any(|cl| !cl.is_using)
            || (self.syms.class(parent).mods & crate::ast::mods::PRIVATE_CTOR != 0 && !self.syms.class(parent).ctors.is_empty() && self.is_java_class(parent));
        if lists.is_empty() && has_normal_clause {
            lists.push(ArgList { args: Vec::new(), using: false, span, fun: ArgList::NO_FUN });
        }
        // `extends E()` for a parent declared without a parameter list.
        if !has_normal_clause && lists.first().map_or(false, |l| !l.using && l.args.is_empty()) {
            lists.remove(0);
        }
        let (te, ty) = if in_place {
            self.construct_parent(parent, sig, owner_subst, prefix, lists, span)
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
            env.frames.push(Frame::Locals { names, tparams, givens, classes: Vec::new(), aliases: Vec::new(), owner: self.sites.owners.len() as u32 });
            self.with_env(env, |t| {
                let outer_case = t.parent_args_of.replace(c);
                let r = t.construct_parent(parent, sig, owner_subst, prefix, lists, span);
                t.parent_args_of = outer_case;
                r
            })
        };
        // A parent's type is fully defined (`fullyDefinedType` of the parent in `typedParent`).
        let ty = self.fully_defined(ty);
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
    fn construct_parent(&mut self, parent: ClassId, sig: Arc<MethodSig>, owner_subst: Subst, prefix: Option<TypeId>, lists: Vec<ArgList>, span: Span) -> (TExprId, TypeId) {
        // A class nested in a class through a path (`extends H.o.I`) takes the path's value as
        // its enclosing instance, as a `new` through it does (dotty's `ExplicitOuter.OuterOps.args`
        // for `Super` as for `New`, the outer from `outerPrefix` of the parent's type); through
        // a `this` the enclosing class's instance is the one `new_outer_arg` finds.
        let path = prefix.filter(|&p| !matches!(self.types.get(p), Type::This(_)) && self.outer_class(parent).is_some());
        let prefixed = self.new_prefixed;
        self.new_prefixed |= path.is_some();
        let r = self.construct_parent_now(parent, sig, owner_subst, prefix, lists, span);
        self.new_prefixed = prefixed;
        if let Some(outer) = path.and_then(|p| self.prefix_value(p)) {
            self.pass_outer(r.0, outer);
        }
        r
    }

    fn construct_parent_now(&mut self, parent: ClassId, sig: Arc<MethodSig>, owner_subst: Subst, prefix: Option<TypeId>, lists: Vec<ArgList>, span: Span) -> (TExprId, TypeId) {
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
            let call = MethodCall { recv: None, sym: SymId(u32::MAX), owner_subst, ext_recv: None, prefix: None, refined: None };
            return self.apply_method(call, Some((parent, sig, prefix)), None, lists, span, None, false).unwrap();
        }
        // Every alternative, the primary included, takes the parent's type arguments.
        // `sig` is seen from the prefix already, a secondary constructor's own signature not.
        let fix = |t: &mut Self, own: Arc<MethodSig>, through: bool| -> Arc<MethodSig> {
            let own = match prefix.filter(|_| through) {
                Some(p) => t.ctor_sig_through(own, p, parent),
                None => own,
            };
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
            Arc::new(MethodSig { tparams: own.tparams.clone(), clauses, ret: sig.ret, declared: crate::tir::NO_TYPE })
        };
        let primary = self.syms.class(parent).primary_ctor.filter(|_| self.ctor_accessible(parent)).map(|s| (s, fix(self, sig.clone(), false)));
        let mut alts = Vec::with_capacity(secondaries.len());
        for s in secondaries {
            let own = self.sig_arc(s);
            let fixed = fix(self, own, true);
            alts.push((s, fixed));
        }
        self.construct_overloaded(parent, primary, alts, prefix, None, lists, span, None)
    }

    fn check_member(&mut self, file: FileId, d: DefId, tclass: &mut TClass) {
        self.restrict_def(file, d);
        self.check_def_warnings(file, d);
        self.check_deprecated_overrides(file, d, tclass.id);
        if self.unused.on() {
            self.mark_annotations(file, d);
            self.mark_module_prefixes(file, d, Owner::Class(tclass.id), false);
        }
        let def = self.ast(file).def(d);
        match &def.kind {
            DefKind::Val { rhs, .. } => {
                let Some(&sym) = self.def_syms.get(file.0 as usize, &d) else { return };
                if rhs.is_none() && self.no_deferred_members(tclass.id) {
                    self.misplaced_abstract_member(def, false);
                }
                // A val of a jar's class is typed when the program reaches it and read
                // through a getter, so that only what the program uses is compiled; one that
                // implements a def of an ancestor is an accessor instead and stays eager. A
                // product's class is another module's program and initialises as the whole
                // program does.
                // A subclass's val that overrides it assigns the field its getter stands for, which a
                // setter beside the getter takes (scalacheck's `Test.Parameters.useLegacyShrinking`
                // overridden in `cpy`'s anonymous class).
                if rhs.is_some() && self.is_library_class(tclass.id) && !self.is_product_class(tclass.id) && self.syms.sym(sym).kind == SymKind::Val && !self.jvm && !self.implements_def(tclass.id, sym) {
                    self.with_loader(|w| {
                        let mut info = w.syms.sym_mut(sym);
                        info.mods |= mods::LAZY;
                        info.needs_setter = true;
                    });
                    return;
                }
                self.class_member_body(sym);
                if let (Some(_), Some(init)) = (rhs, self.readable_init(sym)) {
                    tclass.init.push(TInit::Field(sym, init));
                    // dotty's `Memoize`: a constant val's getter answers the constant whatever
                    // the constructor has run; the record gives the targets that have no type
                    // store its literal (`TClass::constants`), the right side staying the
                    // constructor's work where the val stands (`Constructors`).
                    match crate::tir::constant_val(&self.prog, &self.syms, &self.types, &self.asts, sym, init) {
                        Some(crate::tir::ConstVal::Declared(l)) => {
                            let lit = self.literal_expr(self.types.lit_val(l));
                            tclass.lists_mut().constants.push((sym, lit));
                        }
                        Some(crate::tir::ConstVal::Literal) => tclass.lists_mut().constants.push((sym, init)),
                        None => {}
                    }
                }
            }
            DefKind::Fun(_) if def.name == names::INIT => {
                let Some(&sym) = self.def_syms.get(file.0 as usize, &d) else { return };
                self.type_secondary_ctor(tclass.id, sym, tclass);
            }
            DefKind::Fun(_) => {
                let Some(&sym) = self.def_syms.get(file.0 as usize, &d) else { return };
                if self.no_deferred_members(tclass.id) {
                    self.misplaced_abstract_member(def, false);
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
                    // A local enum's cases are checked once its captures are known
                    // (`finish_local_enum`).
                    let local_case = self.syms.class(nested).kind == ClassKind::EnumCase && self.syms.class(tclass.id).owner == Owner::Local;
                    if local_case {
                        return;
                    }
                    self.note_nested_stored_class(tclass.id, nested);
                    self.check_class(nested);
                    // The instance of an object nested here is made on the first read of its
                    // lazy val, with this instance as its outer.
                    if let Some(val) = self.syms.class(nested).inner_object {
                        let init = self.new_instance(nested, ast::ListRef::EMPTY, span);
                        tclass.init.push(TInit::Field(val, init));
                    }
                    // The companion an enum made: here, eager in a class (`LazyVals
                    // .transformSyntheticModule`), lazy in a trait; a case class's, lazy.
                    let made_for = |t: &Self, k: ClassId| t.syms.class(k).kind == ClassKind::Enum || t.syms.class(k).kind == ClassKind::Class && t.syms.class(k).mods & mods::CASE != 0;
                    let made = self.syms.class(nested).companion.filter(|&m| made_for(self, nested) && self.syms.class(m).def.is_none());
                    if let Some((m, val)) = made.and_then(|m| self.syms.class(m).inner_object.map(|v| (m, v))) {
                        self.check_class(m);
                        let init = self.new_instance(m, ast::ListRef::EMPTY, span);
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
            DefKind::TypeAlias { .. } => self.reach_alias(file, d),
        }
    }

    /// The field the case class selector `s` (`mods::SELECTOR`) reads: the `_N`'s parameter of
    /// the first clause.
    pub(super) fn selector_field(&self, s: SymId) -> Option<SymId> {
        let info = self.syms.sym(s);
        let Owner::Class(c) = info.owner else { return None };
        if info.mods & mods::SELECTOR == 0 {
            return None;
        }
        let n: usize = self.name_str(info.name).strip_prefix('_')?.parse().ok()?;
        self.syms.class(c).ctor_syms.first()?.get(n.checked_sub(1)?).copied()
    }

    /// The name of a case class's `n`th selector, `_n`; the first five are the predefined names.
    pub(super) fn selector_name(&self, n: usize) -> Name {
        match n {
            1..=5 => [names::TUPLE_1, names::TUPLE_2, names::TUPLE_3, names::TUPLE_4, names::TUPLE_5][n - 1],
            _ => self.interner.intern(&format!("_{n}")),
        }
    }

    /// Whether the namer entered selectors in `c`: a case class or an enum's class case.
    pub(super) fn has_selectors(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        info.mods & mods::CASE != 0 || info.kind == ClassKind::EnumCase
    }

    /// The case class's selector reading its `i`th field of the first clause, where the namer
    /// entered one (`mods::SELECTOR`).
    pub(super) fn selector_of_field(&self, c: ClassId, i: usize) -> Option<SymId> {
        self.syms.class(c).selectors.get(i).copied().flatten()
    }

    /// The bodies of the case class's selectors, each reading its field (`this.x`), among the
    /// class's methods, which every target defines; a check that only reports makes one where
    /// it is first used (`emits_bodies`, `type_member_body_with`).
    fn selector_bodies(&mut self, c: ClassId, tclass: &mut TClass) {
        if !self.emits_bodies || !self.has_selectors(c) {
            return;
        }
        let arity = self.syms.class(c).selectors.len();
        for i in 0..arity {
            let Some(s) = self.selector_of_field(c, i) else { continue };
            let f = match self.fun_of_sym.get(&s) {
                Some(&f) => f,
                None => self.selector_body(s, self.syms.class(c).ctor_syms[0][i]),
            };
            if !tclass.methods.contains(&f) {
                tclass.methods.push(f);
            }
        }
    }

    /// The body of the selector `s`, reading `field` (`this.x`).
    fn selector_body(&mut self, s: SymId, field: SymId) -> FunId {
        let ty = self.sig_arc(s).ret;
        let this = self.prog.add(TExpr::This);
        let read = self.prog.add(TExpr::Field(this, field));
        self.prog.set_type(read, ty);
        let f = self.prog.add_fun(crate::tir::TFun { sym: s, params: Vec::new(), defaults: Vec::new(), body: Some(read), body_unconsuming: false, captures: 0, lifted: false });
        self.fun_of_sym.insert(s, f);
        f
    }

    /// scalac's E067 for an abstract member where nothing can implement it (dotty's
    /// `Checking.checkWellFormed`: a deferred term whose owner is no class, a module or an
    /// anonymous class): a member of an object or an anonymous class or a local, a def, a val or a
    /// var, and of a package a given (`givens_only`), whose other members say they need a body;
    /// reported when `def` is one. A JavaScript or JVM template or `@native` stands for a body.
    pub(super) fn misplaced_abstract_member(&mut self, def: &ast::Def, givens_only: bool) -> bool {
        let kind = match &def.kind {
            DefKind::Fun(f) if f.body.is_none() && def.mods & mods::GIVEN != 0 => "given instance",
            _ if givens_only => return false,
            DefKind::Fun(f) if f.body.is_none() => "method",
            DefKind::Val { rhs: None, .. } if def.mods & mods::MUTABLE != 0 => "variable",
            DefKind::Val { rhs: None, .. } => "value",
            _ => return false,
        };
        let template = |a: &ast::Annot| matches!(a.name, names::JS | names::JS_IMPORT | names::JS_EXPORT | names::JVM | names::NATIVE);
        if kind != "given instance" && def.annots.iter().any(template) {
            return false;
        }
        let msg = format!("Declaration of {} {} not allowed here: only classes can have declared but undefined members", kind, self.name_str(def.name));
        // An incomplete definition's body is unknown, not missing: dotty's parser gives it an
        // error tree (`Parsers.funDefOrDcl`'s `expr()` after the missing `=`).
        if def.mods & mods::INCOMPLETE != 0 {
            self.dependent_error(def.span, msg);
        } else {
            self.error(def.span, msg);
        }
        true
    }

    /// Whether no member of `c` can be abstract: an object's or an anonymous class's of the program,
    /// a Scala class (a JavaScript type's members are its own).
    fn no_deferred_members(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        matches!(info.kind, ClassKind::Object | ClassKind::Anon) && info.js == JsKind::Scala && !self.is_library_class(c)
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

    /// Whether `sym` is deferred, as dotty's `Deferred` flag says: abstract, and not an
    /// `abstract override` read from a pickle with its body, which `TreeUnpickler` leaves
    /// concrete (only a definition that lacks a body is `Deferred`).
    pub(crate) fn is_deferred_member(&self, sym: SymId) -> bool {
        if !self.is_abstract_member(sym) {
            return false;
        }
        let both = mods::ABSTRACT | mods::OVERRIDE;
        !(self.syms.sym(sym).mods & both == both && self.loaded.as_ref().map_or(false, |l| l.has_body(sym)))
    }

    /// Whether the parameters of `msig` and of `p` erase apart by what a declaration states beyond
    /// its type (`MethodSig::declared`): their signatures differ, dotty's `Signature` by `sigName`,
    /// so that the one overrides or implements nothing of the other (`Denotation.matches`):
    /// `x: V | V` is `Object` where `x: V` is `V`, `x: C` of `type C = Int *: EmptyTuple`
    /// `Product` where `x: Tuple1[Int]` is `Tuple1`. Only parameters both declare without a type
    /// parameter are compared: one seen through a binding erases at the binding's place.
    fn declared_params_apart(&mut self, msig: &MethodSig, p: SymId) -> bool {
        let declared = |w: &Self, s: SymId| s != SymId(u32::MAX) && declared_of(w, s) != NO_TYPE;
        let psig = self.sig_arc(p);
        let params = |sig: &MethodSig| {
            sig.clauses.iter().flat_map(|c| c.params.iter()).any(|q| declared(self, q.sym))
                || sig.tparams.iter().any(|&t| self.syms.tparam(t).declared_upper != NO_TYPE)
        };
        if !params(msig) && !params(&psig) {
            return false;
        }
        let pairs: Vec<(&ParamSig, &ParamSig)> = msig.clauses.iter().flat_map(|c| c.params.iter()).zip(psig.clauses.iter().flat_map(|c| c.params.iter())).collect();
        pairs.into_iter().any(|(a, b)| {
            let (x, y) = (self.declared_sig_name(a), self.declared_sig_name(b));
            matches!((x, y), (Some(x), Some(y)) if x != y)
        })
    }

    /// The class a parameter erases to by its declaration, as dotty's `sigName` names it: by the
    /// declared view where there is one (`symbols::expand_declared`), a type parameter by its
    /// bound as declared, a union of a value class `java.lang.Object` (`erasedLub` of
    /// `ErasedValueType`s); none for another type naming a type parameter.
    fn declared_sig_name(&mut self, q: &ParamSig) -> Option<String> {
        if q.by_name || q.repeated {
            return None;
        }
        let view = if q.sym != SymId(u32::MAX) { declared_of(self, q.sym) } else { NO_TYPE };
        let mut t = if view == NO_TYPE { q.ty } else { crate::symbols::expand_declared(&self.syms, &self.types, view) };
        // The bound chain followed to its end, as `sigName`'s erasure follows it
        // (`checkedSuperType`); a cycle, which the bounds' check refuses, names nothing.
        let mut seen: Vec<TParamId> = Vec::new();
        while let Type::Param(p) = self.types.get(t) {
            if seen.contains(&p) {
                return None;
            }
            seen.push(p);
            let info = self.syms.tparam(p);
            t = if info.declared_upper != NO_TYPE { crate::symbols::expand_declared(&self.syms, &self.types, info.declared_upper) } else { info.upper };
        }
        if self.mentions_param(t) {
            return None;
        }
        // A `*:` chain as `erasePair` names it, before `sig_name` dealiases it to its tuple class:
        // the class of its arity where it ends in one, `Product` where it ends in the `EmptyTuple`
        // alias (`tupleArity` -1), as the JVM's erasure has it (`Gen::erase`).
        let chain = |w: &Self, t: TypeId| match w.types.get(t) {
            Type::Class(c, _) if Some(c) == w.b.cons_tuple => {
                let tuples = &w.b.tuples;
                Some(match w.types.cons_arity(t, c, &|k| tuples.iter().position(|&n| n == Some(k))) {
                    Some(n) if n <= 22 => format!("scala.Tuple{}", n),
                    Some(_) => "scala.runtime.TupleXXL".to_string(),
                    None => "scala.Product".to_string(),
                })
            }
            _ => None,
        };
        if let Some(name) = chain(self, t) {
            return Some(name);
        }
        if let Type::Class(c, args) = self.types.get(t) {
            if c == self.b.array {
                if let Some(name) = self.types.items(args).first().and_then(|&e| chain(self, e)) {
                    return Some(format!("{}[]", name));
                }
            }
        }
        if let Type::Union(a, b, _) = self.types.get(t) {
            let value_class = |w: &Self, t: TypeId| matches!(w.types.get(t), Type::Class(c, _) if w.syms.class(c).value_class);
            if value_class(self, a) || value_class(self, b) {
                return Some("java.lang.Object".to_string());
            }
            let (x, y) = (self.sig_name(a, false, 0, None)?, self.sig_name(b, false, 0, None)?);
            return (x == y).then_some(x);
        }
        self.sig_name(t, false, 0, None)
    }

    /// A member of a std class standing for a JDK class that the JDK declares abstract
    /// (`absorb_java_class_now`): implemented by a member of its signature alone, as any other
    /// class's, not by a std member of its name.
    fn abstract_by_jdk(&self, sym: SymId) -> bool {
        let s = self.syms.sym(sym);
        s.intrinsic.is_some() && s.mods & mods::ABSTRACT != 0
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
            // is abstract to a class that extends the trait itself, as scalac declares it, and
            // so is `PartialFunction.apply`, scala-library's abstract `Function1.apply`, whose
            // template calls the function a literal or a class instance is. A Java interface's
            // method without a body is abstract too, its template being how a call reaches the
            // JS primitives that implement it; a default method has a body.
            return match s.owner {
                Owner::Class(c) if Some(c) == self.b.reflect_enum || Some(c) == self.b.partial_function => true,
                // So are `Product`'s `productArity` and `productElement` and `Equals`'s
                // `canEqual`, whose templates answer for the classes the rule gives them
                // (`SyntheticMembers`): a trait's own `canEqual` implements it.
                Owner::Class(c) if Some(c) == self.b.product || Some(c) == self.equals_class() => {
                    matches!(s.name, names::PRODUCT_ARITY | names::PRODUCT_ELEMENT | names::CAN_EQUAL_METHOD) && declared_abstract()
                }
                // A member the JDK declares abstract is (`absorb_java_class_now`).
                Owner::Class(c) => (s.mods & mods::ABSTRACT != 0 || s.java_defined && self.syms.class(c).kind == ClassKind::Trait) && declared_abstract(),
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

    /// Completes the signatures of the members of `Object`'s names `c`'s bases declare, which
    /// nothing may have called: the JVM's mixin forwarders of them in `c` are chosen and bridged
    /// by their types before erasure (`Gen::object_method_forwarders`, dotty's
    /// `MixinOps.needsMixinForwarder` and `Bridges.addBridgeIfNeeded`), a polymorphic
    /// `clone[A]()` overriding no `Object.clone()`.
    fn complete_forwarded_members(&mut self, c: ClassId) {
        let bases: Vec<ClassId> = self.syms.class(c).base_types[1..].iter().map(|&(b, _)| b).collect();
        let finalize = self.interner.lookup("finalize");
        let names: Vec<Name> = [names::TO_STRING, names::HASH_CODE, names::EQUALS, names::CLONE].into_iter().chain(finalize).collect();
        for b in bases {
            for &name in &names {
                let Some(&m) = self.syms.class(b).members.get(&name) else { continue };
                let alternatives: Vec<SymId> = match self.syms.alternatives(m) {
                    Some(alts) => alts.to_vec(),
                    None => vec![m],
                };
                for a in alternatives {
                    if self.syms.sym(a).owner == Owner::Class(b) {
                        self.sig_of(a);
                    }
                }
            }
        }
    }

    /// Completes the signatures of the members `c`'s own members and extensions override, which
    /// nothing may have called: the JVM's bridges of `c`'s methods read them (dotty's
    /// `Bridges.addBridgeIfNeeded`, the overridden member completed on demand). The override
    /// check completes them for a class of the program's files; a class an expansion copies from
    /// a library's or a product's template, whose file is that library's, is not checked
    /// (`check_overrides`), and a module's products write it whole (`reach::materialised`).
    pub(crate) fn complete_overridden_members(&mut self, c: ClassId) {
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        for i in 0..self.own_member_count(c) {
            let m = self.own_member(c, i);
            let (name, is_ext) = (self.syms.sym(m).name, self.syms.sym(m).is_extension);
            for &b in &bases {
                let mut k = 0;
                while let Some(p) = self.inherited_member(b, name, is_ext, k) {
                    k += 1;
                    if matches!(self.syms.sym(p).kind, SymKind::Def | SymKind::Val | SymKind::Var | SymKind::Given) {
                        self.sig_of(p);
                    }
                }
            }
        }
    }

    /// Decides again, from the flags every body left, what the override checks decided from
    /// the flags they read (`override_pairs`): whether a method is an alternative is set by the
    /// merge of an entry with the inherited alternatives, which a lookup makes when it gets
    /// there, so what a check read depended on the merges made before it, the workers' order.
    /// The flags only ever turn on, and every merge a body makes is made by the end, so the
    /// decisions made again contain the ones made on the way whatever the order was.
    /// The names `c`'s bases enter as a mask (their `names_mask`): all a walk of `c`'s own members
    /// over its bases can meet in their tables, gathered once per class as dotty's `memberNames`
    /// is, so that a member none of them names goes without a probe of each base.
    pub(super) fn base_member_names(&self, c: ClassId) -> BaseNames {
        let mut mask = 0;
        for &(b, _) in &self.syms.class(c).base_types[1..] {
            debug_assert!(self.names_mask_covers(b), "the names mask of {} lacks a name of its table", self.class_path(b));
            mask |= self.syms.class(b).names_mask;
        }
        BaseNames(mask)
    }

    /// Whether `b`'s `names_mask` has the bit of every name of its table and extensions.
    pub(super) fn names_mask_covers(&self, b: ClassId) -> bool {
        let bi = self.syms.class(b);
        bi.members.keys().all(|&n| bi.may_have(n)) && bi.extensions.iter().all(|&e| bi.may_have(self.syms.sym(e).name))
    }

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
        let mut inherited: Option<BaseNames> = None;
        let mut targets: Option<Vec<(ClassId, SymId, Name)>> = None;
        for i in 0..self.own_member_count(c) {
            let m = self.own_member(c, i);
            let (name, is_ext, m_mods, has_def) = {
                let s = self.syms.sym(m);
                (s.name, s.is_extension, s.mods, s.def.is_some())
            };
            // A plain constructor parameter is local to its class and overrides nothing, and
            // neither does a private val, which scalac lets share its name with an inherited member.
            // A case class's selector is a definition of the class (`Desugar.productElemMeths`).
            // A member object overrides as a value of its type does (`RefChecks.checkOverride`),
            // its symbol made apart from its definition's.
            if !has_def && m_mods & mods::SELECTOR == 0 && (m_mods & mods::FIELD == 0 || !self.is_ctor_param(c, m)) && !self.is_member_object(m) {
                continue;
            }
            // A selector is checked where it meets something: dotty's `OverridingPairs.Cursor`
            // pairs a member with the members its ancestors declare of its name, and it carries
            // no modifier the checks below read alone.
            if m_mods & mods::SELECTOR != 0 && !self.syms.sym(m).alternative && !self.selector_meets_inherited(c, name, &mut inherited, &mut targets) {
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
            // A name none of the bases has meets nothing there (the bases' names gathered once).
            let named = inherited.get_or_insert_with(|| self.base_member_names(c)).has(name);
            for j in (1..self.syms.class(c).base_types.len()).filter(|_| named) {
                let b = self.syms.class(c).base_types[j].0;
                let bt = self.base_instance_at(c, j);
                // A base that declares nothing (`java.io.Serializable`) has nothing to override.
                let bi = self.syms.class(b);
                if if is_ext { bi.extensions.is_empty() } else { bi.members.is_empty() } {
                    continue;
                }
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
                    // A root beside `Any`'s member of the name is named apart (`dispatch_name`),
                    // its name maybe still pending: the override takes that name too.
                    let beside_any = overridden
                        && !(alternative || meets_inherited || named)
                        && matches!(name, names::TO_STRING | names::HASH_CODE | names::EQUALS)
                        && !is_ext
                        && self.syms.sym(p).kind == SymKind::Def
                        && self.any_member_agreement(p) == Agreement::Params;
                    named_root |= overridden && (alternative || meets_inherited || named || beside_any);
                    let inherited = self.syms.sym(p);
                    // An inherited value a method does not override is an alternative of it too.
                    overloads_inherited |= !overridden && matches!(inherited.kind, SymKind::Def | SymKind::Val | SymKind::Var | SymKind::Given) && !is_ext;
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
                self.check_target_name_clashes(c, m, &mut targets);
            }
            if overrides {
                continue;
            }
            // A `val toString` or `val hashCode` implements the method of `Any` too, called as
            // a method through its accessor; one of the parameters of `Any`'s and another type
            // overrides it incompatibly (E164).
            let any = if matches!(name, names::TO_STRING | names::HASH_CODE | names::EQUALS | names::CLONE)
                && !is_ext
                && matches!(self.syms.sym(m).kind, SymKind::Def | SymKind::Val)
            {
                self.any_member_agreement(m)
            } else {
                Agreement::Params
            };
            let overrides_any = any == Agreement::Same;
            // Another parameter list beside `Any`'s member of the name is an alternative of it,
            // named apart in the output (`dispatch_name`).
            let beside_any = matches!(name, names::TO_STRING | names::HASH_CODE | names::EQUALS)
                && !is_ext
                && self.syms.sym(m).kind == SymKind::Def
                && any == Agreement::Params;
            if beside_any {
                if self.alternatives_named {
                    self.dispatch_name(m);
                } else {
                    self.dispatch_pending.push(m);
                }
            }
            if any == Agreement::Result {
                let msg = format!(
                    "error overriding method {} in class Any of type {}: {} of type {} has incompatible type",
                    self.name_str(name),
                    Self::any_member_type(name),
                    self.member_description(m, None),
                    self.sig_string(m)
                );
                self.error_at(m, msg);
            } else if overrides_any && m_mods & mods::OVERRIDE == 0 && !self.is_abstract_member(m) {
                let msg = format!("method {} needs `override` modifier to override the {} of Any", self.name_str(name), self.name_str(name));
                self.error_at(m, msg);
            } else if !overrides_any && m_mods & mods::OVERRIDE != 0 {
                let msg = format!("{} overrides nothing", self.member_description(m, None));
                self.error_at(m, msg);
            }
        }
        // A pair of members of two bases is checked here only where no one parent derives from
        // both owners (dotty's `OverridingPairs.isHandledByParent`): the precheck tells.
        let shared = self.syms.class(c).parents.len() > 1 && self.bases_share_a_name(c, true);
        if shared {
            self.check_inherited_pairs(c);
        }
        if shared && self.syms.class(c).base_types.len() > 2 {
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
        let bases: Vec<(ClassId, TypeId)> = self.met_base_types(c)[1..].to_vec();
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
            self.rollback(&mark);
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

    /// Whether `m` is a parameter of a `using` clause of its class's constructor, which dotty flags `Given`.
    fn is_using_ctor_param(&self, m: SymId) -> bool {
        let Owner::Class(c) = self.syms.sym(m).owner else { return false };
        let info = self.syms.class(c);
        info.ctor.iter().zip(&info.ctor_syms).any(|(clause, syms)| clause.is_using && syms.contains(&m))
    }

    fn is_ctor_param(&self, c: ClassId, m: SymId) -> bool {
        self.syms.class(c).ctor_syms.iter().any(|clause| clause.contains(&m))
    }

    /// Whether `m`, a member of `c`, is a parameter accessor whose chain of like-named parameter
    /// accessors in the superclasses reaches `p` (`RefChecks.isInheritedAccessor`, which follows
    /// `ParamForwarding.inheritedAccessor` until it meets `p`): the superclass's own immutable parameter
    /// accessor of the name, unless private, in which case one it passes to its superclass's like-named
    /// parameter (`PostTyper.forwardParamAccessors`' `SuperParamAlias`) stands for the next.
    fn is_inherited_accessor(&mut self, c: ClassId, m: SymId, p: SymId) -> bool {
        if !self.is_ctor_param(c, m) {
            return false;
        }
        let name = self.syms.sym(m).name;
        let mut sup = self.syms.class(c).superclass;
        while let Some(s) = sup {
            let Some(candidate) = self.syms.class(s).ctor_syms.iter().flatten().copied().find(|&q| self.syms.sym(q).name == name) else { return false };
            if self.syms.sym(candidate).kind != SymKind::Val {
                return false;
            }
            if self.syms.sym(candidate).mods & mods::FIELD != 0 && !self.is_private(candidate) {
                if candidate == p {
                    return true;
                }
            } else if !self.forwards_to_super_param(s, candidate) {
                return false;
            }
            sup = self.syms.class(s).superclass;
        }
        false
    }

    /// Whether the private parameter accessor `q` of the program's class `s` is an alias of its superclass's
    /// like-named parameter, `PostTyper.forwardParamAccessors`' `SuperParamAlias`, read off the typed superclass
    /// call as dotty reads it: the call, typed again here and dropped (`type_parent_ctor`), applies the primary
    /// constructor (one resolved to a secondary constructor, `ParentCall::via`, makes no alias); its outermost
    /// application is the constructor's last clause (but the `()` `normalizeIfConstructor` appends to one of `using`
    /// clauses alone), the method type whose names `superParamNames` holds, and the
    /// argument whose symbol (`Tree.symbol`) is `q` stands at a parameter of `q`'s name. A typed argument's symbol is
    /// its reference's, none for any other node (a conversion's call, `int2long(x)`) and none for a node the typing
    /// marked as dotty's `Block`, `NamedArg` or `Inlined` (`Worker::denotation_marks`). A by-name parameter is none
    /// (`!sym.info.isInstanceOf[ExprType]`). A library class's alias (TASTy's `PARAMalias`) is not read.
    fn forwards_to_super_param(&mut self, s: ClassId, q: SymId) -> bool {
        let (file, def_id, sup) = {
            let i = self.syms.class(s);
            (i.file, i.def, i.superclass)
        };
        let (Some(def_id), Some(sup)) = (def_id, sup) else { return false };
        if self.source(file).is_std || self.is_body_file(file) {
            return false;
        }
        if self.syms.class(s).ctor.iter().flat_map(|cl| cl.params.iter()).any(|p| p.sym == q && p.by_name) {
            return false;
        }
        // The constructor's method type as `NamerOps.normalizeIfConstructor` makes it: one of `using` clauses alone ends
        // in an added `()`, the outermost application, which passes nothing (`superArgs.nonEmpty`).
        let ctor = &self.syms.class(sup).ctor;
        if ctor.iter().all(|cl| cl.is_using && !cl.is_implicit) {
            return false;
        }
        let DefKind::Class(cls) = &self.ast(file).def(def_id).kind else { return false };
        let clause = self.superclass_clause(s, &cls.parents);
        let Some(names) = self.syms.class(sup).ctor.last().map(|cl| cl.params.iter().map(|p| p.name).collect::<Vec<_>>()) else { return false };
        let Some(parent_ty) = self.base_type_of(s, sup) else { return false };
        let outer = self.denotation_marks.replace(Default::default());
        let mark = self.attempt();
        let call = self.type_parent_ctor(s, clause, sup, Some(parent_ty), false).map(|(call, _)| call);
        let marks = std::mem::replace(&mut self.denotation_marks, outer).map(|m| m.nodes).unwrap_or_default();
        let aliased = call.is_some_and(|call| {
            let args = self.prog.expr_list(call.args).to_vec();
            if call.via.is_some() || args.len() < names.len() {
                return false;
            }
            let symbol = |te: TExprId| match self.prog.expr(te) {
                TExpr::Local(x) | TExpr::Field(_, x) if !marks.contains(&te) => Some(x),
                _ => None,
            };
            // `superArgs.indexWhere(_.symbol == sym)`, then `superParamNames(idx) == stat.name`.
            let outermost = &args[args.len() - names.len()..];
            outermost.iter().position(|&te| symbol(te) == Some(q)).is_some_and(|i| names[i] == self.syms.sym(q).name)
        });
        self.retract(mark);
        aliased
    }

    /// Whether `m` overrides `Any`'s member of its name (`any_member_agreement`).
    pub(super) fn matches_any_member(&mut self, m: SymId) -> bool {
        self.any_member_agreement(m) == Agreement::Same
    }

    /// How `m` stands to the member of its name every class has, `Any`'s `equals(x$0: Any):
    /// Boolean`, `hashCode(): Int` and `toString(): String` and `Object`'s `clone(): Object`,
    /// compared as an inherited member is (`compare_sig_pair`, dotty's `RefChecks.checkOverride`
    /// on `Any`'s real members) with the complete method type: a parameterless member matches
    /// the `()` one, as `matchNullaryLoosely` holds of a member of `Any`.
    pub(super) fn any_member_agreement(&mut self, m: SymId) -> Agreement {
        let Owner::Class(c) = self.syms.sym(m).owner else { return Agreement::Params };
        let name = self.syms.sym(m).name;
        let msig = self.sig_arc(m);
        let (params, ret) = match name {
            names::EQUALS => {
                // `m`'s own parameter stands for `Any`'s, whose type names no path to rename.
                let sym = msig.clauses.iter().flat_map(|cl| cl.params.iter()).next().map_or(m, |p| p.sym);
                (vec![ParamSig { name: self.interner.intern("x$0"), ty: ANY, by_name: false, repeated: false, has_default: false, sym }], self.b.t_boolean)
            }
            names::HASH_CODE => (Vec::new(), self.b.t_int),
            names::TO_STRING => (Vec::new(), self.b.t_string),
            names::CLONE => (Vec::new(), self.b.t_any_ref),
            _ => return Agreement::Params,
        };
        let clauses = if msig.clauses.is_empty() && params.is_empty() { Vec::new() } else { vec![ClauseSig { params, is_using: false, is_implicit: false }] };
        let psig = Arc::new(MethodSig { tparams: Vec::new(), clauses, ret, declared: crate::tir::NO_TYPE });
        self.compare_sig_pair(c, msig, None, psig, ANY, false, false)
    }

    /// `Any`'s member of the name as scalac shows its type.
    fn any_member_type(name: Name) -> &'static str {
        match name {
            names::EQUALS => "(x$0: Any): Boolean",
            names::HASH_CODE => "(): Int",
            names::CLONE => "(): Object",
            _ => "(): String",
        }
    }

    /// Two ancestors defining one name: a concrete member has to fit the abstract one it
    /// implements. Two concrete ones are the conflict that `set_parents` reports. A pair that a
    /// direct parent inherits as well was checked with that parent.
    #[cold]
    /// Asked where two parents' bases share a name (`bases_share_a_name`).
    fn check_inherited_pairs(&mut self, c: ClassId) {
        let bases: Vec<(ClassId, TypeId)> = self.met_base_types(c)[1..].to_vec();
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
                if earlier_ones.is_empty() {
                    earlier_ones.push((m, b, bt));
                    continue;
                }
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
                        (true, false) if !self.overridden_before(c, m, b, bt) => {
                            self.check_override(c, m, Some(bt), earlier, ebt);
                            self.reach_through_accessor(m, earlier);
                        }
                        (false, true) if !self.overridden_before(c, earlier, eb, ebt) => {
                            self.check_override(c, earlier, Some(ebt), m, bt);
                            self.reach_through_accessor(earlier, m);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Whether the concrete member `s` of the ancestor `b` (of type `bt` in `c`) is overridden by
    /// a concrete member of its name and parameters that a class before `b` in `c`'s
    /// linearization declares, `c` itself included: dotty's `OverridingPairs` cursor has visited
    /// `s` as overridden then, and pairs it as overriding nothing (`visited`), so that a class
    /// overriding both of two inherited members checks each against its own alone.
    fn overridden_before(&mut self, c: ClassId, s: SymId, b: ClassId, bt: TypeId) -> bool {
        let name = self.syms.sym(s).name;
        let bases: Vec<(ClassId, TypeId)> = self.met_base_types(c);
        for (i, &(k, kt)) in bases.iter().enumerate() {
            if k == b {
                return false;
            }
            let mut j = 0;
            while let Some(o) = self.inherited_member(k, name, false, j) {
                j += 1;
                if self.syms.sym(o).owner != Owner::Class(k) || self.is_private(o) || self.is_abstract_member(o) {
                    continue;
                }
                let osig = self.sig_arc(o);
                if self.compare_sigs(c, osig, (i > 0).then_some(kt), s, bt) != Agreement::Params {
                    return true;
                }
            }
        }
        false
    }

    /// Checks `m` against `p`, a member of an ancestor that it overrides or implements, both seen
    /// from `c`: `m_owner` is the ancestor `m` comes from, or `None` for a member of `c` itself.
    /// Returns whether `m` counts as overriding `p`; different parameters make the two unrelated,
    /// which is an overload scalac would accept and teq rejects.
    /// Whether `m` in class `c` keeps the access `p` it overrides grants, dotty's
    /// `RefChecks.isOverrideAccessOK`: `m` public, or protected where `p` is, and relaxing `p`'s
    /// access boundary (`p`'s within `m`'s, or `m`'s that of the companion object of the class
    /// that bounds `p`, a class not local to a block).
    fn override_access_ok(&mut self, c: ClassId, m: SymId, p: SymId) -> bool {
        let mb = self.access_boundary(m, c);
        if mb == Boundary::Root {
            return true;
        }
        let (m_mods, p_mods) = (self.syms.sym(m).mods, self.syms.sym(p).mods);
        let protected_ok = p_mods & mods::PROTECTED == 0 || m_mods & mods::PROTECTED != 0;
        if !protected_ok {
            return false;
        }
        let ob = self.access_boundary(p, c);
        let companion_ok = match (ob, mb) {
            (Boundary::Class(k), Boundary::Class(o)) => {
                // `mb.is(Module)`: an object nested in a class is a module class too.
                self.syms.class(k).owner != Owner::Local && self.is_module_class(o) && self.syms.class(k).companion == Some(o)
            }
            _ => false,
        };
        let java_protected = p_mods & mods::PROTECTED != 0 && matches!(self.syms.sym(p).owner, Owner::Class(k) if self.is_java_class(k));
        self.boundary_within(ob, mb) || companion_ok || java_protected
    }

    /// The access boundary of the member `s` seen from the class `base`, dotty's
    /// `accessBoundary`: its class where it is private, what its qualifier names (`private[p]`,
    /// `protected[p]`), `base` where it is protected, the root otherwise.
    fn access_boundary(&mut self, s: SymId, base: ClassId) -> Boundary {
        let info = self.syms.sym(s);
        let (mods_s, owner, file, at) = (info.mods, info.owner, info.file, info.span.start);
        let qualifier = if self.in_jar(file) || self.is_body_file(file) {
            (mods_s & mods::QUALIFIED != 0).then(|| self.loaded.as_ref().and_then(|l| l.access_within.get(&s).map(|&(n, _)| n))).flatten()
        } else {
            self.ast(file).access_scopes.iter().find(|&&(a, _)| a == at).map(|&(_, n)| n)
        };
        if let Some(scope) = qualifier {
            let mut o = owner;
            loop {
                match o {
                    Owner::Class(k) if self.syms.class(k).name == scope => return Boundary::Class(k),
                    Owner::Class(k) => o = self.syms.class(k).owner,
                    Owner::Package(p) => {
                        let mut q = Some(p);
                        while let Some(pk) = q {
                            if self.syms.pkg(pk).name == scope {
                                return Boundary::Package(pk);
                            }
                            q = self.syms.pkg(pk).parent;
                        }
                        return Boundary::Root;
                    }
                    Owner::Local => return Boundary::Root,
                }
            }
        }
        match owner {
            Owner::Class(k) if mods_s & mods::PRIVATE != 0 => Boundary::Class(k),
            _ if mods_s & mods::PROTECTED != 0 => Boundary::Class(base),
            _ => Boundary::Root,
        }
    }

    /// Whether the boundary `inner` lies within `outer`, dotty's `isContainedIn`.
    fn boundary_within(&self, inner: Boundary, outer: Boundary) -> bool {
        if outer == Boundary::Root || inner == outer {
            return true;
        }
        let mut at = match inner {
            Boundary::Root => return false,
            Boundary::Class(k) => self.syms.class(k).owner,
            Boundary::Package(p) => match self.syms.pkg(p).parent {
                Some(q) => Owner::Package(q),
                None => return false,
            },
        };
        loop {
            match at {
                Owner::Class(k) => {
                    if outer == Boundary::Class(k) {
                        return true;
                    }
                    at = self.syms.class(k).owner;
                }
                Owner::Package(p) => {
                    if outer == Boundary::Package(p) {
                        return true;
                    }
                    match self.syms.pkg(p).parent {
                        Some(q) => at = Owner::Package(q),
                        None => return false,
                    }
                }
                Owner::Local => return false,
            }
        }
    }

    fn check_override(&mut self, c: ClassId, m: SymId, m_owner: Option<TypeId>, p: SymId, p_owner: TypeId) -> bool {
        let (m_kind, m_mods, m_lazy) = {
            let s = self.syms.sym(m);
            (s.kind, s.mods, s.mods & mods::LAZY != 0)
        };
        let (p_kind, p_mods, p_lazy, mut pb) = {
            let s = self.syms.sym(p);
            let Owner::Class(pb) = s.owner else { return false };
            // A jar's val that teq reads through a getter is lazy by teq's own mark, which a session
            // keeps from one build to the next: its pickle declares no `lazy` (`needs_setter`).
            (s.kind, s.mods, s.mods & mods::LAZY != 0 && !s.needs_setter, pb)
        };
        // An object over an inherited object is the namer's error (E069), a class definition
        // that cannot be overridden.
        if self.is_member_object(m) && self.is_member_object(p) {
            return true;
        }
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
        let declared_apart = self.declared_params_apart(&msig, p);
        let agreement = match self.compare_sigs_of(c, Some(m), msig, m_owner, p, p_owner) {
            Agreement::Same | Agreement::Result if declared_apart => Agreement::Params,
            agreement => agreement,
        };
        // A val next to the alternatives of an overloaded name other than the one it implements,
        // a method beside a value whose parameters it does not take, and a value beside a method
        // that takes parameters: no pair (dotty's `matches` of their signatures fails,
        // `Signature.matchDegree`'s `NoMatch`, so neither overrides the other).
        let overloads = (p_kind == SymKind::Def && (m_kind == SymKind::Def || self.syms.sym(p).alternative))
            || (m_kind == SymKind::Def && matches!(p_kind, SymKind::Val | SymKind::Var | SymKind::Given))
            || (matches!(m_kind, SymKind::Val | SymKind::Var) && p_kind == SymKind::Def);
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
        } else if own && !self.override_access_ok(c, m, p) {
            // `RefChecks.overrideAccessError`: what the overridden member's own access asks.
            if p_mods & mods::PROTECTED != 0 {
                "{m} has weaker access privileges than {p}; it should be at least protected"
            } else {
                "{m} has weaker access privileges than {p}; it should be public"
            }
        } else if own && m_mods & (mods::OVERRIDE | mods::SELECTOR) == 0 && (!p_abstract || p_deferred) && !self.is_product_member(p) {
            // A case class's selector overrides as if marked (dotty's `RefChecks.autoOverride`).
            "{m} needs `override` modifier to override {p}"
        } else {
            if !own {
                return true;
            }
            if self.declared_target_name(m) != self.declared_target_name(p) {
                return self.check_target_names(m, p);
            }
            // (1.12) A val parameter is overridden by its inherited accessor alone, or by a
            // member the version lets pass (`RefChecks.checkOverride` 626-632).
            if p_kind == SymKind::Val && self.is_ctor_param(pb, p) && !self.is_inherited_accessor(c, m, p) {
                let (file, span) = (self.syms.sym(m).file, self.syms.sym(m).span);
                let msg = format!("cannot override val parameter {}", self.member_description(p, Some(pb)));
                self.migration_in(file, crate::source_version::Migration::OverrideValParameter, span, crate::warnings::NO_ID, crate::source::PHASE_REFCHECKS, || msg);
            }
            return true;
        };
        let m_class = m_owner.and_then(|t| self.class_of(t));
        // An object is shown without its type (`infoString` of a module).
        let object = self.is_member_object(m);
        let problem = if object { problem.replace("{m} of type {mt}", "{m}") } else { problem.to_string() };
        let shown_m = if object { format!("object {}", self.name_str(self.syms.sym(m).name)) } else { self.member_description(m, m_class) };
        let msg = problem
            .replace("{m}", &shown_m)
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

    /// Whether an ancestor of `c` declares a member called `name`, or an alternative whose
    /// `@targetName` is `name`, which `check_target_name_clashes` compares: the bases' names
    /// (`base_member_names`) and the ancestors' target names (`inherited_targets`) gathered once
    /// per class, a base whose names lack `name` passed over.
    fn selector_meets_inherited(&mut self, c: ClassId, name: Name, names: &mut Option<BaseNames>, targets: &mut Option<Vec<(ClassId, SymId, Name)>>) -> bool {
        if names.get_or_insert_with(|| self.base_member_names(c)).has(name) {
            for j in 1..self.syms.class(c).base_types.len() {
                let bi = self.syms.class(self.syms.class(c).base_types[j].0);
                if bi.may_have(name) && bi.members.contains_key(&name) {
                    return true;
                }
            }
        }
        self.inherited_targets(c, targets);
        targets.as_ref().is_some_and(|t| t.iter().any(|&(_, _, t)| t == name))
    }

    /// The alternatives of `c`'s ancestors that declare a `@targetName`, with their bases and
    /// names, gathered into `targets` where it is empty.
    fn inherited_targets(&mut self, c: ClassId, targets: &mut Option<Vec<(ClassId, SymId, Name)>>) {
        if targets.is_some() {
            return;
        }
        let mut found = Vec::new();
        // The classes `Desugar.classDef` gives (`rule_prefix`) declare none.
        for j in 1 + self.syms.class(c).rule_prefix as usize..self.syms.class(c).base_types.len() {
            let b = self.syms.class(c).base_types[j].0;
            if !self.file_declares_target_name(self.syms.class(b).file) {
                continue;
            }
            for k in 0..self.syms.class(b).member_order.len() {
                let q = self.syms.class(b).member_order[k];
                if self.syms.sym(q).alternative {
                    if let Some(t) = self.declared_target_name(q) {
                        found.push((b, q, t));
                    }
                }
            }
        }
        *targets = Some(found);
    }

    /// Whether a definition of `f` carries `@targetName` (`Worker::target_name_files`), read
    /// once per file.
    pub(super) fn file_declares_target_name(&mut self, f: crate::source::FileId) -> bool {
        let i = f.0 as usize;
        match self.target_name_files.get(i) {
            Some(1) => return false,
            Some(2) => return true,
            _ => {}
        }
        let declares = match self.interner.lookup("targetName") {
            Some(name) => self.ast(f).defs.iter().any(|d| d.annots.iter().any(|a| a.name == name)),
            None => false,
        };
        if self.target_name_files.len() <= i {
            self.target_name_files.resize(i + 1, 0);
        }
        self.target_name_files[i] = 1 + declares as u8;
        declares
    }

    pub(super) fn declared_target_name(&mut self, m: SymId) -> Option<Name> {
        let info = self.syms.sym(m);
        let ast = self.ast(info.file);
        let annot = ast.def(info.def?).annots.iter().find(|a| self.interner.get(a.name) == "targetName")?;
        let text = ast.str(*ast.annot_args(annot).first()?).to_string();
        Some(self.interner.intern(&text))
    }

    /// scalac's E120 for an inherited member: a method whose `@targetName` is the name of an
    /// inherited method of the same erased signature. `inherited` holds the bases' alternatives
    /// that declare a target name, gathered once per class.
    fn check_target_name_clashes(&mut self, c: ClassId, m: SymId, inherited: &mut Option<Vec<(ClassId, SymId, Name)>>) {
        let own_name = self.syms.sym(m).name;
        let mut candidates: Vec<(ClassId, SymId, Name)> = Vec::new();
        if let Some(target) = self.declared_target_name(m) {
            // A declaration renamed to a member the rule synthesizes meets it under that one name
            // after erasure (`checkNoDoubleDeclaration` pairs the effective names): E120 where the
            // two erase alike.
            if target != own_name && self.record_synthesizes(c, target) {
                if let Some(&(bit, _, param)) = synthetic::MEMBERS.iter().find(|&&(_, n, _)| n == target) {
                    if self.erases_as_synthesized(m, target, param) {
                        self.report_synthesized_double_definition(c, bit, m);
                        return;
                    }
                }
            }
            for j in 1..self.syms.class(c).base_types.len() {
                let b = self.syms.class(c).base_types[j].0;
                let bi = self.syms.class(b);
                if let Some(&q) = bi.may_have(target).then(|| bi.members.get(&target)).flatten() {
                    candidates.push((b, q, target));
                }
            }
        }
        self.inherited_targets(c, inherited);
        candidates.extend(inherited.as_ref().unwrap().iter().copied().filter(|&(_, _, t)| t == own_name));
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

    /// scalac's check along the linearisation: what an earlier ancestor defines concretely cannot
    /// override a final member of a later one.
    fn check_final_overrides(&mut self, c: ClassId) {
        // The names of the bases before the one at hand: a final member no earlier base names
        // meets nothing there (their names masks).
        let mut earlier = self.syms.class(self.syms.class(c).base_types[1].0).names_mask;
        for j in 2..self.syms.class(c).base_types.len() {
            let b = self.syms.class(c).base_types[j].0;
            let bt = self.base_instance_at(c, j);
            let mask = self.syms.class(b).names_mask;
            let meets = mask & earlier != 0;
            earlier |= mask;
            if !meets {
                continue;
            }
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
                    let a = self.syms.class(c).base_types[i].0;
                    let at = self.base_instance_at(c, i);
                    let Some(&m) = self.syms.class(a).members.get(&name) else { continue };
                    if self.syms.sym(m).owner != Owner::Class(a) || self.syms.alternatives(m).is_some() || self.syms.sym(m).kind != SymKind::Def {
                        continue;
                    }
                    // An abstract member is implemented by the final one, not overriding it: dotty's
                    // overriding pairs take the concrete of the two as the overriding member
                    // (`OverridingPairsChecker.checkAll`, "concrete overrides abstract").
                    if self.is_abstract_member(m) {
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
    pub(super) fn compare_sigs(&mut self, c: ClassId, msig: Arc<MethodSig>, m_owner: Option<TypeId>, p: SymId, p_owner: TypeId) -> Agreement {
        self.compare_sigs_of(c, None, msig, m_owner, p, p_owner)
    }

    /// `compare_sigs` of the member `m` where it is known, whose own and overridden symbols count
    /// for the loose match of a nullary pair as `p`'s do.
    pub(super) fn compare_sigs_of(&mut self, c: ClassId, m: Option<SymId>, mut msig: Arc<MethodSig>, m_owner: Option<TypeId>, p: SymId, p_owner: TypeId) -> Agreement {
        let mut psig = self.sig_arc(p);
        if let Some(mono) = self.post_typer_mono(c, p) {
            psig = Arc::new(self.mono_replaced_sig(&psig, mono));
        }
        // `def x` and `def x()` are one member, either overriding the other, where either or a
        // member either overrides is Java's, `Any`'s or `Object.clone`, or of a Scala 2 class
        // (dotty's `matchNullaryLoosely`, `OverridingPairs.isOverridingPair`).
        let empty = |sig: &MethodSig| matches!(&sig.clauses[..], [cl] if cl.params.is_empty() && !cl.is_using);
        let nullary_pair = (msig.clauses.is_empty() && empty(&psig)) || (psig.clauses.is_empty() && empty(&msig));
        let java_member = nullary_pair && (self.matches_nullary_loosely(p) || m.is_some_and(|m| self.matches_nullary_loosely(m)));
        if msig.clauses.is_empty() && empty(&psig) && java_member {
            psig = Arc::new(MethodSig { tparams: psig.tparams.clone(), clauses: Vec::new(), ret: psig.ret, declared: crate::tir::NO_TYPE });
        } else if psig.clauses.is_empty() && empty(&msig) && java_member && self.syms.sym(p).kind == SymKind::Def {
            msig = Arc::new(MethodSig { tparams: msig.tparams.clone(), clauses: Vec::new(), ret: msig.ret, declared: crate::tir::NO_TYPE });
        }
        let lenient_using = self.is_std_scala_member(p);
        self.compare_sig_pair(c, msig, m_owner, psig, p_owner, false, lenient_using)
    }

    /// The `MirroredMonoType` that `RefChecks` reads a member `p` of a mirror's trait through in the
    /// companion `c`, which `SyntheticMembers.addMirrorSupport` makes its class's mirror after
    /// typing: the alias it adds beside the parent where `c` has none (`monoType`, 683–699), the
    /// class over wildcards. The typer's tables hold neither (`ClassInfo::post_typer_bases`).
    fn post_typer_mono(&mut self, c: ClassId, p: SymId) -> Option<TypeId> {
        let Owner::Class(pb) = self.syms.sym(p).owner else { return None };
        if !self.is_mirror_class(pb) || !self.has_desugared_mirror_parent(c) {
            return None;
        }
        let k = self.syms.class(c).companion?;
        if self.mirror_companion(k) != Some(c) {
            return None;
        }
        // `monoType`: the member the companion has of the name, its own or inherited, where it is
        // no `Deferred` one, which the override reads as it stands; else the alias it enters.
        let mono = self.interner.intern("MirroredMonoType");
        if self.typer_type_member(c, mono).is_some_and(|a| !self.syms.aliases[a.idx()].is_abstract()) {
            return None;
        }
        let n = self.syms.class(k).tparams.len();
        Some(self.types.class(k, &vec![WILD; n]))
    }

    /// The type member `name` of the complete class `c` as the typer finds it: its own, then its
    /// bases' in linearisation order but the mirror's traits it gets after typing
    /// (`ClassInfo::post_typer_bases`), a concrete one before an abstract one, as a member's
    /// denotation prefers it.
    pub(super) fn typer_type_member(&self, c: ClassId, name: Name) -> Option<AliasId> {
        let info = self.syms.class(c);
        let post_typer = 1..1 + info.post_typer_bases as usize;
        let found: Vec<AliasId> = info.base_types.iter().enumerate().filter(|(i, _)| !post_typer.contains(i)).filter_map(|(_, &(b, _))| self.syms.class(b).type_aliases.get(&name).copied()).collect();
        found.iter().copied().find(|a| !self.syms.aliases[a.idx()].is_abstract()).or(found.first().copied())
    }

    /// `sig` with the mirror's abstract `MirroredMonoType` read as `mono` (`post_typer_mono`).
    fn mono_replaced_sig(&mut self, sig: &MethodSig, mono: TypeId) -> MethodSig {
        let name = self.interner.intern("MirroredMonoType");
        let (types, syms) = (&*self.types, &self.syms);
        fn replaced(types: &crate::types::TypeStore, syms: &crate::symbols::Symbols, t: TypeId, name: Name, mono: TypeId) -> TypeId {
            match types.get(t) {
                Type::Decl(a) if syms.aliases[a.idx()].name == name => mono,
                Type::Member(_, n) if n == name => mono,
                _ => types.map_parts(t, &mut |x| replaced(types, syms, x, name, mono)),
            }
        }
        let mut out = sig.clone();
        out.ret = replaced(types, syms, out.ret, name, mono);
        for cl in out.clauses.iter_mut() {
            for p in cl.params.iter_mut() {
                p.ty = replaced(types, syms, p.ty, name, mono);
            }
        }
        out
    }

    /// A member of the lean std, whose overrides may add a using clause the ancestor lacks
    /// (`SortedSet.map` asks for an `Ordering`; a call through the ancestor passes no evidence,
    /// which the body allows for).
    pub(super) fn is_std_member(&self, s: SymId) -> bool {
        self.std.slot_of(self.syms.sym(s).file).is_some()
    }

    /// dotty's `SymDenotation.matchNullaryLoosely`: `s` or a member it overrides is Java's (the
    /// std's classes of `java.*` stand for the JDK's), `Any`'s (`toString()`, `hashCode()`),
    /// `Object.clone`, or a member of a Scala 2 class (scala-library's compiled from Scala 2's
    /// sources, the std's standing for one, a Scala 2 jar's), by where the symbol comes from and
    /// never by its package's name: the one rule of an application's parentheses
    /// (`takes_java_parens`), an override's and a member meet's loose match.
    pub(super) fn matches_nullary_loosely(&mut self, s: SymId) -> bool {
        if s.0 == u32::MAX {
            return false;
        }
        let mut chain = vec![s];
        self.overridden_chain(s, 0, &mut chain);
        chain.into_iter().any(|o| self.nullary_provenance(o) != Nullary::Strict)
    }

    /// What `matchNullaryLoosely`'s `test` finds of the one symbol `s` (`allOverriddenSymbols`
    /// asks it of each it overrides).
    pub(super) fn nullary_provenance(&mut self, s: SymId) -> Nullary {
        let info = self.syms.sym(s);
        // `JavaDefined`: a class file's, the std's standing for the JDK (its `java.*` classes,
        // `@javaDefined`, its extensions on `String` standing for `java.lang.String`'s members,
        // its ports of the test interfaces).
        let (file, owner, name) = (info.file, info.owner, info.name);
        if info.java_defined || (self.is_std_member(s) && !self.is_std_scala_member(s)) || self.string_member_of_std(s) || self.scala2_std_member(s) {
            return Nullary::Java;
        }
        // A jar's class of a `java.*` package standing for the JDK's class of its name, which
        // scalac typed the library against (scala-java-time's `java.time` under Scala.js): the
        // member is Java's where the JDK's class declares it so (`Duration.isZero()`), the
        // declaration's own, Scala's, otherwise: of a class of the package that is no JDK class's
        // (`java.review.C`), and wherever no JDK is there to tell.
        if self.in_jar(file) && self.under_java_package(s) {
            let binary = self.jdk_binary_name(owner);
            if binary.and_then(|binary| self.jdk_declares_nullary_method(&binary, &self.name_str(name))) == Some(true) {
                return Nullary::Java;
            }
        }
        // `Any.toString()`, `Any.hashCode()` and `Object.clone()`, which a class's member of that
        // name and no parameters, type parameters included, overrides; a local or top-level
        // definition overrides nothing.
        if matches!(name, names::TO_STRING | names::HASH_CODE | names::CLONE) && matches!(owner, Owner::Class(_)) {
            let sig = self.sig_arc(s);
            if sig.tparams.is_empty() && sig.clauses.iter().all(|cl| cl.params.is_empty() && !cl.is_using) && sig.clauses.len() <= 1 {
                return Nullary::Java;
            }
        }
        if self.scala2_member(s) {
            Nullary::Scala2
        } else {
            Nullary::Strict
        }
    }

    /// The JDK binary name of the class owning a member (`java/time/Duration`, `java/util/Map$Entry`),
    /// an object's the class's (the JDK's static members are its class's).
    fn jdk_binary_name(&self, owner: Owner) -> Option<String> {
        let Owner::Class(mut c) = owner else { return None };
        let mut names = Vec::new();
        let pkg = loop {
            let info = self.syms.class(c);
            names.push(self.name_str(info.name).trim_end_matches('$').to_string());
            match info.owner {
                Owner::Class(o) => c = o,
                Owner::Package(p) => break p,
                _ => return None,
            }
        };
        names.reverse();
        Some(format!("{}/{}", self.pkg_path(pkg).replace('.', "/"), names.join("$")))
    }

    /// Every member `m` overrides, down the linearisation: `root_declarations`' walk, keeping
    /// each step and not only the roots.
    pub(super) fn overridden_chain(&mut self, m: SymId, depth: u32, out: &mut Vec<SymId>) {
        let (name, owner) = (self.syms.sym(m).name, self.syms.sym(m).owner);
        let (Owner::Class(c), true) = (owner, depth < 64) else { return };
        self.complete_class(c);
        let bases: Vec<(ClassId, TypeId)> = self.met_base_types(c);
        for &(b, bt) in bases.iter().skip(1) {
            let mut k = 0;
            while let Some(p) = self.own_alternative(b, name, k) {
                k += 1;
                if self.syms.sym(p).kind == SymKind::Def && !self.is_private(p) && !out.contains(&p) && self.same_parameters(c, &bases, m, p, bt) {
                    out.push(p);
                    self.overridden_chain(p, depth + 1, out);
                }
            }
        }
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
            // The bounds are seen from `c` as the signatures are (`X <: Z.this.Y` of `Z` is
            // `X <: A.this.Y` in `A`).
            let (pu, pl) = (seen(self, pu), seen(self, pl));
            let (mu, ml) = if m_owner.is_some() { (seen(self, mu), seen(self, ml)) } else { (mu, ml) };
            if (pu != mu && !self.is_same(pu, mu)) || (pl != ml && !self.is_same(pl, ml)) {
                return Agreement::Params;
            }
        }
        // The using clauses count as scalac counts them: `compose[G](using Applicative[G])`
        // and `compose[G]` are two members.
        let using_count = |sig: &MethodSig| sig.clauses.iter().filter(|cl| cl.is_using).count();
        let explicit_only = lenient_using && using_count(&msig) != using_count(&psig);
        let mclauses: Vec<&ClauseSig> = msig.clauses.iter().filter(|cl| !explicit_only || !cl.is_using).collect();
        let pclauses: Vec<&ClauseSig> = psig.clauses.iter().filter(|cl| !explicit_only || !cl.is_using).collect();
        // The clauses pair up one by one; where one method has clauses past the other's, they
        // have to be empty: such a pair is a member whose type differs (`def f` against `def
        // f()`, `f(x: Any)()` against `f(x: Any)`), an incompatible type (E164), as dotty's
        // `RefChecks` reports one only of a pair whose types match loosely (`trueMatch`,
        // `Type.matches`, relaxed); clauses that divide the parameters otherwise are no override.
        let paired = mclauses.len().min(pclauses.len());
        let same_clause = |a: &ClauseSig, b: &ClauseSig| {
            a.is_using == b.is_using && a.params.len() == b.params.len() && a.params.iter().zip(&b.params).all(|(x, y)| x.by_name == y.by_name && x.repeated == y.repeated)
        };
        let empty_rest = mclauses[paired..].iter().chain(&pclauses[paired..]).all(|cl| cl.params.is_empty() && !cl.is_using);
        if !empty_rest || !mclauses[..paired].iter().zip(&pclauses[..paired]).all(|(a, b)| same_clause(a, b)) {
            return Agreement::Params;
        }
        let same_shape = mclauses.len() == pclauses.len();
        // The parameters a later one's type names are the other method's by position, as
        // scalac matches `f(d: Ctx)(x: d.T)` with `f(c: Ctx)(x: c.T)`.
        let renamed: Vec<(SymId, TypeId)> = mclauses[..paired]
            .iter()
            .zip(&pclauses[..paired])
            .flat_map(|(a, b)| a.params.iter().zip(&b.params).map(|(x, y)| (x.sym, y.sym)).collect::<Vec<_>>())
            .filter(|&(x, y)| x != y)
            .map(|(x, y)| (x, self.types.mk(Type::Term(y))))
            .collect();
        for (a, b) in mclauses[..paired].iter().zip(&pclauses[..paired]) {
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
        if !same_shape {
            return Agreement::Result;
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
        Arc::new(MethodSig { tparams: sig.tparams.clone(), clauses, ret, declared: crate::tir::NO_TYPE })
    }

    /// "method f", "value x in trait T".
    /// A member object: a static one's symbol, or the val of one nested in a class.
    fn is_member_object(&self, m: SymId) -> bool {
        matches!(self.syms.sym(m).kind, SymKind::Object(_)) || self.inner_object_of_sym(m).is_some()
    }

    pub(super) fn member_description(&self, m: SymId, in_class: Option<ClassId>) -> String {
        let info = self.syms.sym(m);
        // `PlainPrinter.kindString`: `Given` before `Lazy`, a given definition and a `using` clause's accessor
        // (`class A(using val x: Int)`) alike.
        let kind = match info.kind {
            SymKind::Def | SymKind::Overloaded(_) => "method",
            SymKind::Val if self.inner_object_of_sym(m).is_some() => "object",
            SymKind::Val if info.mods & mods::GIVEN != 0 || self.is_using_ctor_param(m) => "given instance",
            SymKind::Val if info.mods & mods::LAZY != 0 => "lazy value",
            SymKind::Val | SymKind::Param => "value",
            SymKind::Var => "variable",
            SymKind::Object(_) => "object",
            SymKind::EnumValue(_) => "case",
            SymKind::Given => "given instance",
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
                let (upper, lower, arity) = (self.syms.tparam(tp).upper, self.syms.tparam(tp).lower, self.syms.tparam(tp).arity);
                // A higher-kinded parameter shows its kind (`PlainPrinter.decomposeLambdas`): its
                // lambda bound's parameters, or as many wildcards as it takes.
                let upper = match self.types.get(upper) {
                    Type::Lambda(ps, body) => {
                        out.push('[');
                        for (j, &q) in self.types.items(ps).to_vec().iter().enumerate() {
                            if j > 0 {
                                out.push(',');
                            }
                            self.show_param_binder(q, &mut out);
                        }
                        out.push(']');
                        body
                    }
                    _ if arity > 0 => {
                        out.push('[');
                        out.push_str(&vec!["_"; arity as usize].join(","));
                        out.push(']');
                        upper
                    }
                    _ => upper,
                };
                let lower = match self.types.get(lower) {
                    Type::Lambda(_, body) => body,
                    _ => lower,
                };
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
                self.check_implements_all(c, Abstracts::OuterAccessors, 0);
            }
        }
    }

    /// Reports the abstract members `c` leaves unimplemented, an export's forwarders implementing
    /// what they forward as the class's members do. A member of `synthesized`
    /// (`TClass::synthesized`) is implemented by rule.
    fn check_implements_all(&mut self, c: ClassId, which: Abstracts, synthesized: u16) {
        // The base types as a list of the check's own, made where an abstract member is to be
        // matched: most classes inherit none.
        let mut copied: Option<Vec<(ClassId, TypeId)>> = None;
        let mut missing: Vec<(String, SymId, TypeId)> = Vec::new();
        // A member left unimplemented beside an incomplete definition of its name, whose body
        // is unknown rather than missing.
        let mut unknown = false;
        // The class's own deferred declarations of what the rule gives it.
        let mut own_deferred: Vec<SymId> = Vec::new();
        // A concrete class's own members without a body too (`class C { given x: Int }`), as
        // scalac's "needs to be abstract"; an object's are E067 (`misplaced_abstract_member`).
        let own = (which == Abstracts::Declared && self.syms.class(c).kind == ClassKind::Class) as usize;
        // A case class or object and an enum case are given every abstract member of `Product`,
        // `Equals` and `Enum` by rule or have an existing definition of it (`SyntheticMembers`),
        // so those parents need no walk.
        // So is a companion that is its class's mirror (`SyntheticMembers.addMirrorSupport`):
        // `fromProduct` or `ordinal` by rule.
        let completed_by_rule = {
            let info = self.syms.class(c);
            which == Abstracts::Declared
                && (info.mods & mods::CASE != 0 || info.kind == ClassKind::EnumCase || (self.mirror_module(c) && info.desugared_parents > 0))
        };
        // Of a class completed by rule, the leading classes `Desugar.classDef` gives
        // (`rule_prefix`) are passed over without a visit.
        let prefix = if completed_by_rule { 1 + self.syms.class(c).rule_prefix as usize } else { 1 };
        for j in ((1 - own)..1.min(self.syms.class(c).base_types.len())).chain(prefix..self.syms.class(c).base_types.len()) {
            let b = self.syms.class(c).base_types[j].0;
            let bt = self.base_instance_at(c, j);
            if completed_by_rule && self.is_desugared_parent(b) {
                continue;
            }
            // The base's members and then its extensions, read in place.
            let mut i = 0;
            while let Some(m) = {
                let bi = self.syms.class(b);
                let declared = bi.member_order.len();
                let at = if i < declared { bi.member_order.get(i) } else { bi.extensions.get(i - declared) };
                i += 1;
                at.copied()
            } {
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
                if !is_ext && self.synthesized_by_rule(m, synthesized) {
                    // The member the rule gives implements what the class inherits; its own
                    // deferred declaration stays one (`RefChecks.checkNoAbstractDecls`).
                    if b == c {
                        own_deferred.push(m);
                    }
                    continue;
                }
                // `Any`'s `toString`, `hashCode` and `equals` implement an abstract redeclaration of
                // theirs (scala-library's `Equals.equals`), as dotty's `RefChecks.isImplemented`
                // finds a concrete member matching it loosely with the parameter types compared
                // (`matchesLoosely(.., alwaysCompareTypes = true)`, relaxed on the result): one of
                // `Any`'s parameters, not an `equals(s: String)`, a type incompatible otherwise
                // being the override check's error. A trait's: its deferred member meets the
                // concrete one of the superclass chain among the class's members, the concrete
                // preferred (`Denotations.mergeSingleDenot`); the class's or a superclass's own
                // hides `Any`'s (`computeMembersNamed`).
                let from_trait = self.syms.class(b).kind == ClassKind::Trait;
                if from_trait && matches!(name, names::TO_STRING | names::HASH_CODE | names::EQUALS) && self.any_member_agreement(m) != Agreement::Params {
                    continue;
                }
                // The class's own member is the most derived: whatever of its name it inherits it
                // overrides, which implements nothing of it (`class C extends T { override def x:
                // Int }` over a concrete `T.x`).
                if copied.is_none() {
                    copied = Some(self.met_base_types(c));
                }
                let bases: &Vec<(ClassId, TypeId)> = copied.as_ref().unwrap();
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
                            && ((self.in_std(s) && self.in_std(m) && !self.abstract_by_jdk(m)) || {
                                let ssig = self.sig_arc(s);
                                let apart = self.declared_params_apart(&ssig, m);
                                self.compare_sigs(c, ssig, (i > 0).then_some(ot), m, bt) != Agreement::Params
                                    && !apart
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
                missing.push((self.name_str(name).to_string(), m, bt));
            }
        }
        if !missing.is_empty() {
            // An abstract var left undefined is one member, its getter and setter together.
            let vars: Vec<SymId> = missing.iter().map(|&(_, m, _)| m).filter(|&m| self.syms.sym(m).kind == SymKind::Var).collect();
            missing.retain(|&(_, m, _)| self.syms.sym(m).mods & mods::SETTER == 0 || !super::setters::var_of_setter(&self.syms, self.interner, m).is_some_and(|v| vars.contains(&v)));
            missing.sort_by(|a, b| a.0.cmp(&b.0));
            missing.dedup_by(|a, b| a.0 == b.0);
            self.report_unimplemented(c, &missing, unknown);
        } else if !own_deferred.is_empty() {
            self.report_deferred_declarations(c, &own_deferred);
        }
    }

    /// dotty's `RefChecks.checkNoAbstractDecls`, which runs where every member is implemented: a
    /// concrete class's own deferred declaration that nothing below it implements, here one of a
    /// member the rule gives the class beside it (`case class C(x: Int) { override def toString:
    /// String }`).
    #[cold]
    fn report_deferred_declarations(&mut self, c: ClassId, decls: &[SymId]) {
        let (span, file) = {
            let i = self.syms.class(c);
            (i.span, i.file)
        };
        let mut errors = vec![format!("{} needs to be abstract, since", self.class_description(c))];
        for &m in decls {
            // dotty's `infoString`: a method without parameters is of type `=> T`.
            let parameterless = self.syms.sym(m).kind == SymKind::Def && self.sig_of(m).clauses.is_empty();
            let ty = self.sig_string(m);
            let what = format!("{} of type {}{}", self.member_description(m, None), if parameterless { "=> " } else { "" }, ty);
            errors.push(format!("there is a deferred declaration of {} which is not implemented in a subclass", what));
        }
        // dotty's `abstractErrorMessage`.
        let msg = if errors.len() <= 2 { errors.join(" ") } else { format!("{}:\n{}", errors[0], errors[1..].join("\n")) };
        self.diags.error(file, span, msg);
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
                let as_setter = Arc::new(MethodSig { tparams: Vec::new(), clauses: vec![ClauseSig { params, is_using: false, is_implicit: false }], ret: unit, declared: crate::tir::NO_TYPE });
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
    /// invariant. Plain constructor parameters are construction-only and not checked; a private
    /// member is local to `this` until a selection through another instance (or a companion's)
    /// makes it a member of every instance, which checks it (`note_private_selection`), as
    /// dotty's `PrivateLocal` (VarianceChecker.relativeVariance) loses `Local` in
    /// `isAccessibleFrom`; a qualified private member and a private class parameter are never
    /// local. The standard library keeps `Array[A]` in its covariant collections and is not
    /// checked.
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
        // The class's own info, its parents (`ClassInfo`), shown as dotty shows it: `Object with Cell[T] {...}`.
        let parents = self.syms.class(c).parents.clone();
        if let Some((p, required)) = parents.iter().find_map(|&t| self.variance_violation(t, 1)) {
            let shown = self.class_info_text(c);
            self.report_variance_at(p, required, &shown, file, span, &class_desc);
        }
        for i in 0..self.own_member_count(c) {
            let m = self.own_member(c, i);
            let (m_mods, has_def) = {
                let s = self.syms.sym(m);
                (s.mods, s.def.is_some())
            };
            let is_field_param = m_mods & mods::FIELD != 0 && self.is_ctor_param(c, m);
            if !(has_def || is_field_param) || self.local_private(c, m) {
                continue;
            }
            self.check_member_variance(c, m, false);
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

    /// Whether the member `m` of `c` is private to `this` until another instance selects it:
    /// private, not qualified, no class parameter (dotty's `canBeLocal`).
    fn local_private(&self, c: ClassId, m: SymId) -> bool {
        let info = self.syms.sym(m);
        info.mods & mods::PRIVATE != 0 && !self.has_access_scope(info.file, info.span.start) && !self.is_ctor_param(c, m)
    }

    /// The variance check of the member `m` of `c`, by its signature's positions; a `var` stands
    /// in an invariant position unless it is private and only read through other instances
    /// (`assigned` false), as dotty's `validateDefinition` keeps a private var covariant unless
    /// `AssignedNonLocally`. Whether an error was reported.
    fn check_member_variance(&mut self, c: ClassId, m: SymId, read_only: bool) -> bool {
        let (kind, m_mods, m_file, m_span) = {
            let s = self.syms.sym(m);
            (s.kind, s.mods, s.file, s.span)
        };
        self.variance_skip.clear();
        if !self.unchecked_variance.is_empty() {
            let tys = self.member_type_exprs(c, m);
            self.collect_unchecked_variance(m_file, &tys);
        }
        let sign = match kind {
            SymKind::Var if read_only => 1,
            SymKind::Var => 0,
            SymKind::Val | SymKind::Def | SymKind::Given => 1,
            _ => return false,
        };
        self.sig_of(m);
        let sig = self.sig_arc(m);
        let mut what = self.member_description(m, None);
        // dotty's `privateParamAddendum`: a private class parameter is never local.
        if m_mods & mods::PRIVATE != 0 && m_mods & mods::FIELD != 0 && self.is_ctor_param(c, m) {
            let (var_or_val, instead) = if kind == SymKind::Var { ("var", " and add\na field inside the class instead") } else { ("val", "") };
            what.push_str(&format!(
                "\n\nImplementation limitation: private {var_or_val} parameters cannot be inferred to be local\nand therefore are always variance-checked.\n\nPotential fix: remove the private {var_or_val} modifiers on the parameter {}{instead}.",
                self.name_str(self.syms.sym(m).name)
            ));
        }
        // dotty's `Traverser`: the member's own info first (`checkInfo`, a method's result), reported at the
        // definition and shown as its info (`=> R`, `(x: Int): R`); its span holds the parameters', whose errors
        // `UniqueMessagePositions` then hides. Else each type parameter's bounds (`type B`, shown ` <: A`) and
        // each parameter's type, at its own definition, in the order written.
        if kind == SymKind::Def {
            if let Some((p, required)) = self.variance_violation(sig.ret, sign) {
                let shown = self.show_method_info(&sig);
                self.report_variance_at(p, required, &shown, m_file, m_span, &what);
                return true;
            }
        } else if self.report_variance(sig.ret, sign, m_file, m_span, &what) {
            return true;
        }
        let written: Vec<(crate::intern::Name, crate::source::Span)> = match self.syms.sym(m).def.map(|d| &self.ast(m_file).def(d).kind) {
            Some(DefKind::Fun(f)) => f.tparams.iter().map(|tp| (tp.name, tp.span)).collect(),
            Some(DefKind::Given(g)) => g.tparams.iter().map(|tp| (tp.name, tp.span)).collect(),
            _ => Vec::new(),
        };
        let mut reported = false;
        for &tp in &sig.tparams {
            let (name, upper, lower) = {
                let info = self.syms.tparam(tp);
                (info.name, info.upper, info.lower)
            };
            let span = written.iter().find(|&&(n, _)| n == name).map_or(m_span, |&(_, sp)| sp);
            // `TypeBounds`' fold: the lower bound flipped first, then the upper.
            if let Some((p, required)) = self.variance_violation(lower, 1).or_else(|| self.variance_violation(upper, -1)) {
                let text = self.bounds_text(lower, upper);
                let what = format!("type {}", self.name_str(name));
                self.report_variance_at(p, required, &format!(" {}", text.trim_start()), m_file, span, &what);
                reported = true;
            }
        }
        for p in sig.clauses.iter().flat_map(|cl| cl.params.iter()) {
            let what = format!("parameter {}", self.name_str(p.name));
            if self.report_variance(p.ty, -1, m_file, self.syms.sym(p.sym).span, &what) {
                reported = true;
            }
        }
        reported
    }

    /// A class's info as dotty shows a `ClassInfo` in a message: its parents, `Object` before a first
    /// that is a trait, then `{...}` (`Object with Cell[T] {...}`).
    fn class_info_text(&mut self, c: ClassId) -> String {
        let parents = self.syms.class(c).parents.clone();
        let mut shown: Vec<String> = Vec::with_capacity(parents.len() + 1);
        let first = parents.first().map(|&t| self.deref(t));
        let first_trait = first.map_or(true, |t| match self.types.get(t) {
            Type::Class(k, _) => self.syms.class(k).kind == ClassKind::Trait,
            _ => false,
        });
        if first_trait {
            shown.push("Object".to_string());
        }
        for t in parents {
            let text = self.show(t);
            shown.push(if text == "AnyRef" { "Object".to_string() } else { text });
        }
        format!("{} {{...}}", shown.join(" with "))
    }

    /// Whether the local `l` stands for `c.this` in a class nested in `c` (`this_ref`, a
    /// constructor's outer parameter), being of that singleton's type: dotty's prefix `c.this`.
    fn local_is_this(&mut self, l: SymId, c: ClassId) -> bool {
        let ty = self.sig_of(l).ret;
        matches!(self.types.get(ty), Type::This(k) if k == c)
    }

    /// A private member of a class selected through `recv`, typed `recv_ty`, or assigned
    /// through it: where `recv` is no `this` of the member's class (nor the object itself), the
    /// member is every instance's, and its variance is checked as any member's (dotty's
    /// `isAccessibleFrom` resetting `Local`, `typedAssign`'s `AssignedNonLocally`), with the
    /// other alternatives of its name, which the selection's denotation filtered alike. An error
    /// is reported once however many selections meet it; a member found sound is not checked
    /// again.
    #[inline]
    pub(super) fn note_private_selection(&mut self, s: SymId, recv: TExprId, recv_ty: TypeId, assigned: bool) {
        if self.syms.sym(s).mods & mods::PRIVATE != 0 {
            self.note_private_selection_of(s, recv, recv_ty, assigned);
        }
    }

    #[inline(never)]
    fn note_private_selection_of(&mut self, s: SymId, recv: TExprId, recv_ty: TypeId, assigned: bool) {
        let (owner, s_mods, s_file, name) = {
            let info = self.syms.sym(s);
            (info.owner, info.mods, info.file, info.name)
        };
        let Owner::Class(c) = owner else { return };
        if s_mods & mods::PRIVATE == 0 || self.source(s_file).is_std || !self.in_variant_scope(c) || !self.local_private(c, s) {
            return;
        }
        match self.prog.expr(recv) {
            TExpr::This => return,
            TExpr::Module(o) if o == c => return,
            TExpr::Local(l) if self.local_is_this(l, c) => return,
            _ => {}
        }
        let recv_ty = self.deref(recv_ty);
        if matches!(self.types.get(recv_ty), Type::This(k) if k == c) {
            return;
        }
        let mut alts: Vec<SymId> = (0..self.own_member_count(c)).map(|i| self.own_member(c, i)).filter(|&m| self.syms.sym(m).name == name && self.local_private(c, m)).collect();
        if !alts.contains(&s) {
            alts.push(s);
        }
        let skip = std::mem::take(&mut self.variance_skip);
        for m in alts {
            let read_only = !(assigned && m == s);
            if self.variance_checked.contains_key(&(m, read_only)) {
                continue;
            }
            let mark = self.diags.items.len();
            if self.check_member_variance(c, m, read_only) {
                if self.diags.items[..mark].contains(&self.diags.items[mark]) {
                    self.diags.items.truncate(mark);
                }
            } else {
                self.variance_checked.insert((m, read_only), ());
            }
        }
        self.variance_skip = skip;
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
                TyExpr::ByName(inner) | TyExpr::Repeated(inner) | TyExpr::Unchecked(inner) | TyExpr::Annotated(inner, _) | TyExpr::Lambda(_, inner) | TyExpr::PolyFun(_, inner) => {
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
                if v != 0 && v != sign && !self.variance_skip.contains(&self.types.param(p)) {
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
            Type::Union(a, b, _) | Type::Inter(a, b) => {
                self.variance_violation(a, sign).or_else(|| self.variance_violation(b, sign))
            }
            // A lambda's parameters are no class's: `Validator` checks a `TypeRef` whose symbol the base contains
            // (`base.isContainedIn(sym.owner)`), so the variances a binder's own clause declares (`F[+X]`) are not read.
            Type::Lambda(ps, body) => {
                let mark = self.variance_skip.len();
                let own = self.types.items(ps).to_vec();
                self.variance_skip.extend(own);
                let found = self.variance_violation(body, sign);
                self.variance_skip.truncate(mark);
                found
            }
            // `TypeAccumulator.foldOver` of a `RefinedType`, which `VarianceChecker.Validator`
            // folds: the parent, then the refinement's info, an alias invariantly, bounds the
            // lower contravariantly and the upper covariantly, a method its type parameters'
            // bounds and its parameters flipped (`LambdaType`'s `paramInfos`) and its result as
            // it stands; a `RecType` its parent.
            Type::Refined(parent, r) => {
                if let Some(found) = self.variance_violation(parent, sign) {
                    return Some(found);
                }
                match self.types.refinement(r) {
                    Refinement::Alias(_, x) => self.variance_violation(x, 0),
                    Refinement::Bounds(_, lo, hi) => self.variance_violation(lo, -sign).or_else(|| self.variance_violation(hi, sign)),
                    Refinement::Val(_, _, x) => self.variance_violation(x, sign),
                    refinement @ Refinement::Term(..) => {
                        let sig = self.refinement_sig(refinement);
                        for &q in &sig.tparams {
                            let (lo, hi) = (self.syms.tparam(q).lower, self.syms.tparam(q).upper);
                            if let Some(found) = self.variance_violation(lo, sign).or_else(|| self.variance_violation(hi, -sign)) {
                                return Some(found);
                            }
                        }
                        for p in sig.clauses.iter().flat_map(|c| c.params.iter()) {
                            if let Some(found) = self.variance_violation(p.ty, -sign) {
                                return Some(found);
                            }
                        }
                        self.variance_violation(sig.ret, sign)
                    }
                }
            }
            Type::Rec(body) => self.variance_violation(body, sign),
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
    pub(super) fn mark_accessors(&mut self, c: ClassId) {
        // The members' flags are records of the shared region when the class is a prefix one.
        // The flags are written under the loader's lock where the members are the shared
        // region's (`shared_write`); the signatures are read outside it.
        self.mark_accessors_unlocked(c)
    }

    fn mark_accessors_unlocked(&mut self, c: ClassId) {
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).skip(1).collect();
        let members = self.syms.class(c).member_order.clone();
        let mut inherited: Option<BaseNames> = None;
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
            // A name none of the bases has meets nothing there (the bases' names gathered once).
            let named = inherited.get_or_insert_with(|| self.base_member_names(c)).has(name);
            for &b in bases.iter().filter(|_| named) {
                let Some(p) = self.syms.class(b).members.get(&name).copied() else { continue };
                if self.syms.sym(p).kind != SymKind::Def {
                    let info = self.syms.sym(p);
                    over_accessor |= info.kind == SymKind::Val && info.needs_accessor && p != m && info.owner == Owner::Class(b) && !self.is_private(p);
                    continue;
                }
                // A def without parameters, or of one empty clause, which a Scala val matches as
                // dotty's `matchesLoosely` does ("a Scala zero-parameter method and a Scala
                // non-method always match"; a Java one does not).
                if self.zero_parameter_def(p) {
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
                let implemented = bases.iter().filter(|_| named).find_map(|&b| {
                    let mut k = 0;
                    while let Some(p) = self.own_alternative(b, name, k) {
                        k += 1;
                        let is_def = self.syms.sym(p).kind == SymKind::Def && self.syms.sym(p).alternative;
                        if is_def && self.zero_parameter_def(p) {
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

    /// Whether a def takes no parameters: no clause but using ones, or, not Java's, one empty
    /// clause besides them (`def name(): String` of a Scala trait, which a val implements).
    fn zero_parameter_def(&mut self, p: SymId) -> bool {
        let sig = self.sig_of(p);
        let plain: Vec<&ClauseSig> = sig.clauses.iter().filter(|cl| !cl.is_using).collect();
        match plain.as_slice() {
            [] => true,
            [only] => only.params.is_empty() && !self.syms.sym(p).java_defined,
            _ => false,
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
/// A parameter's declared view (`MethodSig::declared` of its symbol's signature).
fn declared_of(w: &Worker, s: SymId) -> TypeId {
    w.syms.sym(s).sig.as_ref().map_or(NO_TYPE, |sig| sig.declared)
}

fn sig_types(sig: &MethodSig) -> impl Iterator<Item = TypeId> + '_ {
    sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)).chain([sig.ret]).chain((sig.declared != NO_TYPE).then_some(sig.declared))
}

/// Whether a worker reads the shared records and the signatures in its view with the overlays
/// on (`TEQ_VIEW_READS=0` reads them raw, for the measurement of what the views cost and change).
fn view_reads_wanted() -> bool {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    !*OFF.get_or_init(|| std::env::var_os("TEQ_VIEW_READS").is_some_and(|v| v == "0"))
}

/// Where a member is accessible: everywhere, within a class, within a package.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Boundary {
    Root,
    Class(ClassId),
    Package(PkgId),
}

/// The names of a class's bases, a bit each (`Worker::base_member_names`).
#[derive(Clone, Copy)]
pub(super) struct BaseNames(u64);

impl BaseNames {

    /// Whether a base may have the name: false only where none has.
    pub(super) fn has(self, n: Name) -> bool {
        self.0 & crate::symbols::name_bit(n) != 0
    }
}

/// What dotty's `matchNullaryLoosely` finds of a symbol (`Worker::nullary_provenance`): Java's,
/// `Any`'s or `Object.clone` (`JavaDefined`, the owner `Any`), a Scala 2 class's (`Scala2x`),
/// or neither.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Nullary {
    Strict,
    Java,
    Scala2,
}

/// What `hasUnknownMembers` asks of a type (`typer/ProtoTypes.scala` 236-253): its case.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum UnknownShape {
    /// A `WildcardType`, `NoType`, a `TypeRef` to `Nothing`: true.
    Unknown,
    /// A class's `TypeRef`, a static symbol's: false.
    Known,
    /// A non-class, non-static `TypeRef`, with its prefix (none: `NoPrefix`).
    Reference(Option<TypeId>),
    /// An uninstantiated `TypeVar`.
    OpenVar,
    /// An `AppliedType`, with its constructor.
    Applied(TypeId),
    /// Another `TypeProxy`.
    Proxy,
    /// Else (an intersection, a union: false, as dotty leaves them).
    Other,
}

/// The type tests and queries `hasUnknownMembers` makes, so that its walk is one over the typer's types (`KnownTypes`)
/// and a test's.
pub(super) trait UnknownMembers {
    fn shape(&mut self, t: TypeId) -> UnknownShape;
    /// `tp.info`'s `loBound` and `hiBound` of a reference: the read, a cycle its error.
    fn bounds(&mut self, t: TypeId) -> Result<(TypeId, TypeId), TParamId>;
    fn is_provisional(&mut self, t: TypeId) -> bool;
    /// `TypeComparer.bounds(tp.origin).lo` of an open variable.
    fn var_lower(&mut self, t: TypeId) -> TypeId;
    /// `superType` of a proxy or an application, with its reads.
    fn super_type(&mut self, t: TypeId) -> Result<Option<TypeId>, TParamId>;
}

/// `SelectionProto.hasUnknownMembers` (`typer/ProtoTypes.scala` 236-253), each `||` and `&&` stopping the reads after
/// it: a wildcard and `Nothing` true; a class or a static symbol false; another reference its prefix's, or its
/// information's upper bound provisional with unknown members and its lower bound with unknown members; an open variable
/// its constraint's lower bound's; an application its constructor's or its `superType`'s; another proxy its
/// `superType`'s; else false. A type met again on the walk reads as false (dotty's `CyclicReference`).
pub(super) fn has_unknown_members<U: UnknownMembers>(u: &mut U, t: TypeId, visiting: &mut Vec<TypeId>) -> Result<bool, TParamId> {
    if visiting.contains(&t) {
        return Ok(false);
    }
    visiting.push(t);
    let r = has_unknown_members_of(u, t, visiting);
    visiting.pop();
    r
}

fn has_unknown_members_of<U: UnknownMembers>(u: &mut U, t: TypeId, visiting: &mut Vec<TypeId>) -> Result<bool, TParamId> {
    match u.shape(t) {
        UnknownShape::Unknown => Ok(true),
        UnknownShape::Known | UnknownShape::Other => Ok(false),
        UnknownShape::Reference(prefix) => {
            if let Some(p) = prefix {
                if has_unknown_members(u, p, visiting)? {
                    return Ok(true);
                }
            }
            let (lo, hi) = u.bounds(t)?;
            Ok(u.is_provisional(hi) && has_unknown_members(u, hi, visiting)? && has_unknown_members(u, lo, visiting)?)
        }
        UnknownShape::OpenVar => {
            let lo = u.var_lower(t);
            has_unknown_members(u, lo, visiting)
        }
        UnknownShape::Applied(tycon) => {
            if has_unknown_members(u, tycon, visiting)? {
                return Ok(true);
            }
            match u.super_type(t)? {
                Some(s) => has_unknown_members(u, s, visiting),
                None => Ok(false),
            }
        }
        UnknownShape::Proxy => match u.super_type(t)? {
            Some(s) => has_unknown_members(u, s, visiting),
            None => Ok(false),
        },
    }
}

/// The typer's types as `hasUnknownMembers` reads them.
struct KnownTypes<'w, 'a> {
    w: &'w mut Worker<'a>,
}

impl UnknownMembers for KnownTypes<'_, '_> {
    fn shape(&mut self, t: TypeId) -> UnknownShape {
        let w = &mut *self.w;
        if t == crate::tir::NO_TYPE {
            return UnknownShape::Unknown;
        }
        let t = w.deref_keeping(t);
        let static_owner = |w: &Worker, a: Option<AliasId>| a.is_some_and(|a| w.is_static_owner(w.syms.aliases[a.idx()].owner));
        match w.types.get(t) {
            Type::Wild | Type::BoundedWild(..) | Type::Nothing => UnknownShape::Unknown,
            Type::Class(..) | Type::Ctor(_) | Type::Nested(..) => UnknownShape::Known,
            Type::Param(_) => UnknownShape::Reference(None),
            Type::Decl(a) => {
                if static_owner(w, Some(a)) { UnknownShape::Known } else { UnknownShape::Reference(None) }
            }
            Type::Member(prefix, name) => {
                let alias = w.type_member_alias(prefix, name);
                if static_owner(w, alias) { UnknownShape::Known } else { UnknownShape::Reference(Some(prefix)) }
            }
            Type::Var(_) => UnknownShape::OpenVar,
            Type::AppParam(p, _) => UnknownShape::Applied(w.types.param(p)),
            Type::AppMember(m, _) => UnknownShape::Applied(m),
            Type::AppVar(v, _) => UnknownShape::Applied(w.types.mk(Type::Var(v))),
            Type::Alias(..) | Type::Term(_) | Type::Select(..) | Type::This(_) | Type::Lit(_) | Type::Annotated(_) | Type::Refined(..) | Type::Rec(_) | Type::Lambda(..) | Type::Match(..) => UnknownShape::Proxy,
            _ => UnknownShape::Other,
        }
    }

    fn bounds(&mut self, t: TypeId) -> Result<(TypeId, TypeId), TParamId> {
        let t = self.w.deref_keeping(t);
        self.w.reference_bounds(t)
    }

    fn is_provisional(&mut self, t: TypeId) -> bool {
        self.w.is_provisional(t)
    }

    fn var_lower(&mut self, t: TypeId) -> TypeId {
        let w = &mut *self.w;
        let t = w.deref_keeping(t);
        let Type::Var(v) = w.types.get(t) else { return NOTHING };
        let lower = w.tvars[v].lower.clone();
        lower.into_iter().fold(NOTHING, |acc, l| if acc == NOTHING { l } else { w.types.soft_union(acc, l) })
    }

    fn super_type(&mut self, t: TypeId) -> Result<Option<TypeId>, TParamId> {
        let w = &mut *self.w;
        let t = w.deref_keeping(t);
        Ok(match w.types.get(t) {
            // `AppliedType.superType`: the constructor's own applied (`kind_info`'s application arm).
            Type::AppParam(..) | Type::AppMember(..) | Type::AppVar(..) => match w.kind_info(t)? {
                KindInfo::Bounds(s) | KindInfo::Alias(s) => Some(s),
                KindInfo::Unknown => None,
            },
            Type::Alias(..) => w.kind_dealias(t),
            Type::Term(_) | Type::Select(..) | Type::This(_) => {
                let s = w.widen_path(t);
                (s != t).then_some(s)
            }
            Type::Lit(_) => Some(w.widen_lit(t)),
            Type::Annotated(u) | Type::Refined(u, _) | Type::Rec(u) | Type::Lambda(_, u) => Some(u),
            Type::Match(_, m) => Some(w.types.match_info(m).bound),
            _ => None,
        })
    }
}

#[cfg(test)]
mod unknown_members_tests {
    use super::*;

    const PROGRAM: &str = "object O:\n  type S <: Any\ntrait P:\n  type R\n  def f[T <: Int](x: T): T = x\n@main def run(): Unit = println(1)\n";

    /// `testProvisional` and `hasUnknownMembers` over the typer's own types (`KnownTypes`), a typed program's worker.
    #[test]
    fn provisional_information_through_the_real_types() {
        crate::with_typed_program("provisional", PROGRAM, &[], |w| {
            let name = w.interner.intern("q");
            // A parameter whose upper bound is an open variable: provisional through its information (`ParamRef.underlying`),
            // and no more once the variable is instantiated.
            let q = w.syms.new_tparam(name, 0);
            let (v, var) = w.fresh_var_id();
            w.syms.tparams[q.idx()].upper = v;
            let pq = w.types.param(q);
            assert!(w.is_provisional(pq));
            // An application of it: provisional through its constructor, its argument a class.
            let int = w.b.t_int;
            let l = w.types.list(&[int]);
            let applied = w.types.mk(Type::AppParam(q, l));
            assert!(w.is_provisional(applied));
            w.tvars[var].inst = Some(int);
            assert!(!w.is_provisional(pq));
            assert!(!w.is_provisional(applied));
            // A parameter bounding itself: the cycle reads as not provisional.
            let r = w.syms.new_tparam(name, 0);
            let pr = w.types.param(r);
            w.syms.tparams[r.idx()].upper = pr;
            assert!(!w.is_provisional(pr));
            // A member of a trait (not static) through its bounds, both sides read; a static object's member stops there.
            let alias = |w: &mut Worker, n: &str| {
                let n = w.interner.intern(n);
                (0..w.syms.aliases.len()).map(|i| AliasId(i as u32)).find(|&a| w.syms.aliases[a.idx()].name == n).expect("the member")
            };
            let (rr, ss) = (alias(w, "R"), alias(w, "S"));
            w.complete_alias(rr);
            w.complete_alias(ss);
            let (dr, ds) = (w.types.mk(Type::Decl(rr)), w.types.mk(Type::Decl(ss)));
            assert!(!w.is_provisional(dr) && !w.is_provisional(ds));
            let (open, _) = w.fresh_var_id();
            w.syms.aliases[rr.idx()].bounds = Some((NOTHING, open));
            w.syms.aliases[ss.idx()].bounds = Some((NOTHING, open));
            assert!(w.is_provisional(dr));
            assert!(!w.is_provisional(ds));
        });
    }

    /// `testProvisional`'s variable: provisional while its instance is one a rollback or a retraction can still undo
    /// (`isPermanentlyInstantiated`), whatever the instance; `hasUnknownMembers` reads the instance as it stands
    /// (`isInstantiated`, `instanceOpt`).
    #[test]
    fn provisional_until_an_instance_is_permanent() {
        crate::with_typed_program("permanent", PROGRAM, &[], |w| {
            let int = w.b.t_int;
            // A trail mark held: the instance a rollback to it undoes is provisional, while `hasUnknownMembers` sees the
            // class; rolled back, the variable is open.
            let (var, v) = w.fresh_var_id();
            let mark = w.snapshot();
            w.instantiate(v, int);
            assert!(w.is_provisional(var));
            assert_eq!(w.has_unknown_members_reads(var), Ok(false));
            w.rollback(&mark);
            drop(mark);
            assert!(w.tvars[v].inst.is_none());
            assert!(w.is_provisional(var));
            // Instantiated under a mark that then goes with its region kept (committed): permanent.
            let mark = w.snapshot();
            w.instantiate(v, int);
            assert!(w.is_provisional(var));
            drop(mark);
            assert!(!w.is_provisional(var));
            // No mark held: permanent at once; a mark taken after the instance does not reach it.
            let (var2, v2) = w.fresh_var_id();
            w.instantiate(v2, int);
            assert!(!w.is_provisional(var2));
            let mark = w.snapshot();
            assert!(!w.is_provisional(var2));
            drop(mark);
            // An open attempt begun after the variable was made: its retraction undoes the instance, provisional; a
            // variable made inside it keeps its instance (its owning state's), permanent.
            let (var3, v3) = w.fresh_var_id();
            let m = w.attempt();
            w.instantiate(v3, int);
            assert!(w.is_provisional(var3));
            let (var4, v4) = w.fresh_var_id();
            w.instantiate(v4, int);
            assert!(!w.is_provisional(var4));
            w.retract(m);
            assert!(w.tvars[v3].inst.is_none());
            assert_eq!(w.tvars[v4].inst, Some(int));
            assert!(!w.is_provisional(var4));
        });
    }

    #[test]
    fn unknown_members_through_a_qualifiers_variable() {
        crate::with_typed_program("unknown", PROGRAM, &[], |w| {
            let name = w.interner.intern("q");
            // The qualifier an open variable whose lower bound is a parameter bounded by an open variable from above and
            // by `Nothing` from below: its upper bound provisional with unknown members (an open variable's lower bound
            // `Nothing`), its lower bound `Nothing`: unknown members, read through the variable's lower bound.
            let q = w.syms.new_tparam(name, 0);
            let (hi, _) = w.fresh_var_id();
            w.syms.tparams[q.idx()].upper = hi;
            let pq = w.types.param(q);
            let (qual, qv) = w.fresh_var_id();
            w.tvars[qv].lower.push(pq);
            assert_eq!(w.has_unknown_members_reads(qual), Ok(true));
            // Its upper bound a class instead: no provisional bound, no unknown members.
            let int = w.b.t_int;
            w.syms.tparams[q.idx()].upper = int;
            assert_eq!(w.has_unknown_members_reads(qual), Ok(false));
            // A class: known; `Nothing`: unknown.
            assert_eq!(w.has_unknown_members_reads(int), Ok(false));
            assert_eq!(w.has_unknown_members_reads(NOTHING), Ok(true));
        });
    }
}

/// The symbol `Type.isRef` compares a reference with: `Nothing` (teq's own type, dotty's `NothingClass`) or a class.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum RefSym {
    Nothing,
    Class(ClassId),
}

/// A non-class reference's information as the kind queries read it (`Worker::kind_info`).
#[derive(Clone, Copy)]
enum KindInfo {
    /// `TypeBounds(_, hi)`: `hi` its `superType` (a clause's `[X..] =>> hi` over its parameters, `boundsFromParams`).
    Bounds(TypeId),
    /// `TypeAlias(alias)`: what `dealias` follows.
    Alias(TypeId),
    /// No information (an error, an overloaded or missing member).
    Unknown,
}

/// `CheckNonCyclicMap`'s state (`typer/Checking.scala` 257-295): the binder checked, the references whose information is
/// being checked (`locked`), whether a cycle is allowed here and in nested refinements, the variance, and where the
/// last part checked was (`where`, `lastChecked`) for the message.
struct NonCyclic {
    sym: TParamId,
    locked: Vec<TypeId>,
    cycle_ok: bool,
    nested_cycle_ok: bool,
    variance: i8,
    where_: &'static str,
    last_checked: TypeId,
}
