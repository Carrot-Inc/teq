//! The worker-view checks of the type store. A read is checked only in a worker's
//! context, between `enter_overlay` and
//! `leave_overlay` with the overlays on. Under the loader's lock the holder's view is the base,
//! whole. Outside it the worker's view is the base whole and its own overlay, a base entry made
//! after the fork being foreign where the worker holds its own equivalent of it (made before the
//! base's), which the import reconciles. Nothing is checked
//! after the join or on a thread with no worker. The checks are compiled
//! into the assertion-enabled builds alone (`debug_assertions`: the `checks` profile and the
//! tests), so the production release and the fast profile carry none of them; the classification
//! below is compiled everywhere for its unit tests.
//!
//! `TEQ_VIEW_CHECKS` says what a violation does: a panic by default; `report` prints the first
//! foreign id of each cascade with the frames that led to its read, counts the rest, and prints
//! the cascades and the sites at the join; `off` nothing. `TEQ_VIEW_CHECKS_LOG=<file>` appends to
//! the file a line at each invocation's start, one per worker as it begins and one per worker's slot
//! the fork left empty, one per fork at its join (the namespace, the workers, the mode and how many
//! checks ran) and one at the invocation's end (whether it typed a
//! program, how many forks and how many with the overlays), the evidence `tests/fork-one.sh`
//! reconciles per invocation.

// The classification is the assertion-enabled builds' and the tests'; the production build
// compiles the checks out and uses none of it.
#![cfg_attr(not(debug_assertions), allow(dead_code))]

use super::*;

/// Where an id lies that the reader's namespace does not hold (a foreign id).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Foreign {
    /// A base entry made after the fork (at or past its sub-store's bound), read by a worker
    /// outside the loader's lock.
    LateBase,
    /// Another worker's overlay entry.
    Peer,
    /// The reading worker's own overlay entry, read under the loader's lock, whose namespace is
    /// the base's.
    OwnOverlay,
    /// In the shared namespace, a base entry made after the fork that is not the reading worker's
    /// canonical entry: the worker holds its own equivalent of it or of one of its descendants.
    Shadowed,
}

impl Foreign {
    pub fn name(self) -> &'static str {
        match self {
            Foreign::LateBase => "the base past the bound",
            Foreign::Peer => "a peer's overlay",
            Foreign::OwnOverlay => "the holder's own overlay",
            Foreign::Shadowed => "the base past the bound, shadowed by the worker's own or over a part that is",
        }
    }
}

/// Where an id of a sub-store whose fork bound is `bound` lies for the worker whose overlay
/// starts at `own`, read under the loader's lock (`hold`) or outside it; `None` when the reader's
/// namespace holds it.
pub fn foreign(bound: u32, own: u32, id: u32, hold: bool) -> Option<Foreign> {
    let mine = id >= own && id - own < WORKER_SPAN;
    if hold {
        return (id >= LOCAL_BASE).then_some(if mine { Foreign::OwnOverlay } else { Foreign::Peer });
    }
    if id < bound || mine {
        None
    } else if id < LOCAL_BASE {
        Some(Foreign::LateBase)
    } else {
        Some(Foreign::Peer)
    }
}

/// `foreign` in the shared namespace: outside the lock the worker's view is the base whole and
/// its own overlay, a base entry at or past the bound foreign where it is not the worker's
/// canonical entry of its structure: the worker holds its own equivalent of it or of one of its
/// descendants (`shadowed`, `TypeStore::not_canonical`).
pub fn foreign_shared(bound: u32, own: u32, id: u32, hold: bool, shadowed: impl FnOnce() -> bool) -> Option<Foreign> {
    if hold || id >= LOCAL_BASE {
        return foreign(bound, own, id, hold);
    }
    (id >= bound && shadowed()).then_some(Foreign::Shadowed)
}

/// Whether a shared record may hold `id`: a base id alone, whoever publishes it.
pub fn publishable(id: u32) -> bool {
    id < LOCAL_BASE
}

#[cfg(debug_assertions)]
/// The entries an entry of sub-store `sub` names, read raw: a type's list, refinement,
/// match record, literal or description and its types, a list's items, a refinement's types and a
/// term refinement's list, a match record's binders' lists and types.
pub(super) fn parts_of(store: &TypeStore, sub: Sub, id: u32) -> Vec<(u8, u32)> {
    let mut out = Vec::new();
    let ty = |t: TypeId| (Sub::Types as u8, t.0);
    match sub {
        Sub::Types => {
            match store.entry(TypeId(id)) {
                Type::Class(_, l) | Type::AppParam(_, l) | Type::AppVar(_, l) | Type::Alias(_, l) => out.push((Sub::Lists as u8, l.0)),
                Type::Lambda(l, b) | Type::Poly(l, b) => out.extend([(Sub::Lists as u8, l.0), ty(b)]),
                Type::Refined(p, r) => out.extend([ty(p), (Sub::Refinements as u8, r.0)]),
                Type::Match(s, m) => out.extend([ty(s), (Sub::Matches as u8, m.0)]),
                Type::AppMember(m, l) => out.extend([ty(m), (Sub::Lists as u8, l.0)]),
                Type::Lit(l) => out.push((Sub::Lits as u8, l.0)),
                Type::Blocked(b) => out.push((Sub::Blocked as u8, b.0)),
                _ => {}
            }
            let mut types = Vec::new();
            store.parts(TypeId(id), &mut types);
            out.extend(types.into_iter().map(ty));
        }
        Sub::Lists => out.extend(store.entry_items(TList(id)).iter().map(|&t| ty(t))),
        Sub::Refinements => {
            if let Refinement::Term(_, _, l) = store.entry_refinement(RefineId(id)) {
                out.push((Sub::Lists as u8, l.0));
            }
            out.extend(store.refinement_types_raw(RefineId(id)).into_iter().map(ty));
        }
        Sub::Matches => {
            let info = store.entry_match(MatchId(id));
            for c in info.cases.iter() {
                out.extend([(Sub::Lists as u8, c.binders.0), ty(c.pattern), ty(c.body)]);
            }
            out.push(ty(info.bound));
        }
        Sub::Lits | Sub::Blocked => {}
    }
    out
}

#[cfg(debug_assertions)]
pub use checked::*;

#[cfg(debug_assertions)]
mod checked {
    use super::*;
    use std::panic::Location;
    use std::sync::Mutex;

    /// What the check was made at: a read through an accessor, a part of an entry being made,
    /// a record's type read through its normalised accessor, a publication into a shared
    /// record, a comparison or a cache of the interpreter.
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub enum At {
        Read,
        Built,
        Record(&'static str),
        Published(&'static str),
        Compared(&'static str),
        Cached(&'static str),
    }

    impl At {
        fn describe(self) -> String {
            match self {
                At::Read => "read".to_string(),
                At::Built => "made a part of".to_string(),
                At::Record(r) => format!("read through {}", r),
                At::Published(r) => format!("published into {}", r),
                At::Compared(r) => format!("compared in {}", r),
                At::Cached(r) => format!("cached in {}", r),
            }
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    struct Key {
        at: At,
        loc: Option<&'static Location<'static>>,
        sub: Sub,
        foreign: Foreign,
        hold: bool,
    }

    /// Where a foreign id came into the reader's hands (`TEQ_VIEW_CHECKS=report`): a raw record
    /// that held it (`origin`), or the site where the first id of its cascade was read, its
    /// parts carrying the origin of the type they are parts of.
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    enum Origin {
        Record(&'static str, bool),
        Entry(usize),
    }

    struct Site {
        key: Key,
        /// The distinct foreign ids first read at the site, and every read there.
        first: u64,
        reads: u64,
    }

    #[derive(Default)]
    struct Report {
        sites: Vec<Site>,
        index: crate::intern::FxMap<Key, usize>,
        /// Per foreign id, where it came from.
        origin: crate::intern::FxMap<(u8, u32), Origin>,
        seen: crate::intern::FxMap<(u8, u32), ()>,
        /// Per origin, in the order of the first id each gave, the distinct foreign ids it gave
        /// the reader (its cascade) and the site of the first.
        cascades: Vec<(Origin, u64, usize)>,
        cascade_of: crate::intern::FxMap<Origin, usize>,
    }

    static REPORT: Mutex<Option<Report>> = Mutex::new(None);

    /// The checks made in a worker's context since the fork: the reads and constructions, and
    /// the publications (`TEQ_VIEW_CHECKS_LOG`, the evidence that the enforcement ran).
    static CHECKED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static PUBLISHED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    thread_local! {
        /// The checks suspended on this thread (`unchecked`, the tests' raw constructions).
        static SUSPENDED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    /// `f` with the checks suspended on this thread: a test that builds what closure at
    /// construction refuses, to measure the translations over it.
    #[cfg(test)]
    pub fn unchecked<R>(f: impl FnOnce() -> R) -> R {
        SUSPENDED.with(|s| s.set(true));
        let r = f();
        SUSPENDED.with(|s| s.set(false));
        r
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Mode {
        Panic,
        Report,
        Off,
    }

    fn mode() -> Mode {
        static MODE: std::sync::OnceLock<Mode> = std::sync::OnceLock::new();
        *MODE.get_or_init(|| match std::env::var("TEQ_VIEW_CHECKS").as_deref() {
            Ok("report") => Mode::Report,
            Ok("off") => Mode::Off,
            _ => Mode::Panic,
        })
    }

    /// The store this thread's worker reads (`store`, or its overlay's where the caller has
    /// none), its overlay, the sub-store's bound and whether it holds the loader's lock: `None`
    /// where nothing is checked.
    fn context<'s>(store: Option<&'s TypeStore>, sub: Sub) -> Option<(&'s TypeStore, &'static Overlay, u32, bool)> {
        let here = HERE.with(|h| h.get());
        if here.is_null() || SUSPENDED.with(|s| s.get()) {
            return None;
        }
        let ov = unsafe { &*here };
        let store = store.unwrap_or_else(|| unsafe { &*ov.store });
        if !store.overlaid.load(Ordering::Relaxed) {
            return None;
        }
        let apart = store.apart_ref()?;
        if !apart.workers.get(ov.worker).is_some_and(|w| std::ptr::eq(&**w, ov)) {
            return None;
        }
        Some((store, ov, apart.bounds[sub as usize], lock_depth() > 0))
    }

    /// A read of `id` of sub-store `sub` through an accessor.
    #[track_caller]
    #[inline]
    pub fn read(store: &TypeStore, sub: Sub, id: u32) {
        if id >= crate::arena::LOCAL_BASE && store.overlays_reclaimed() {
            panic!("an overlay's entry of the {} ({}) read after the merge gave the overlays back, at {}", sub.name(), id, Location::caller());
        }
        check(Some(store), At::Read, sub, id, Location::caller());
    }

    /// A part `id` of an entry about to be made in this thread's namespace: closure at
    /// construction, the worker's parts in its view and the holder's in the base.
    #[track_caller]
    #[inline]
    pub fn built(store: &TypeStore, sub: Sub, id: u32) {
        check(Some(store), At::Built, sub, id, Location::caller());
    }

    /// A type of the record `what` read through its normalised accessor (a signature, a
    /// partial one), which hands the reader its view's ids alone.
    #[track_caller]
    pub fn record(store: &TypeStore, what: &'static str, t: TypeId) {
        check(Some(store), At::Record(what), Sub::Types, t.0, Location::caller());
    }

    /// A type compared at the boundary `what`, where no accessor sees it (`Value::Type`'s
    /// equality): the comparison has to be in the reader's view.
    #[track_caller]
    pub fn compared(what: &'static str, t: TypeId) {
        check(None, At::Compared(what), Sub::Types, t.0, Location::caller());
    }

    /// A type put into or found in the interpreter's cache `what`, which later runs read by id.
    #[track_caller]
    pub fn cached(what: &'static str, t: TypeId) {
        check(None, At::Cached(what), Sub::Types, t.0, Location::caller());
    }

    /// A type published into the shared record `what` (a signature, a class's info): a base id
    /// alone, which every worker's view holds.
    #[track_caller]
    pub fn published(store: &TypeStore, what: &'static str, t: TypeId) {
        if context(Some(store), Sub::Types).is_some() {
            PUBLISHED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        if publishable(t.0) || t == crate::tir::NO_TYPE {
            return;
        }
        let Some((_, ov, _, hold)) = context(Some(store), Sub::Types) else { return };
        let foreign = if t.0 >= ov.base && t.0 - ov.base < WORKER_SPAN { Foreign::OwnOverlay } else { Foreign::Peer };
        let key = Key { at: At::Published(what), loc: Some(Location::caller()), sub: Sub::Types, foreign, hold };
        violation(store, key, t.0);
        if mode() == Mode::Report {
            tag(what, true, t);
        }
    }

    /// A type of the raw record `what` handed to the reader (a class's info, a type parameter's
    /// bounds, an alias's right-hand side), with `TEQ_VIEW_CHECKS=report` alone: not a check,
    /// but where a foreign id the reader goes on to read came from.
    pub fn origin(what: &'static str, t: TypeId) {
        if mode() != Mode::Report || t == crate::tir::NO_TYPE {
            return;
        }
        let Some((_, ov, bound, hold)) = context(None, Sub::Types) else { return };
        if foreign(bound, ov.base, t.0, hold).is_some() {
            tag(what, false, t);
        }
    }

    fn tag(what: &'static str, published: bool, t: TypeId) {
        let mut guard = REPORT.lock().unwrap_or_else(|e| e.into_inner());
        guard.get_or_insert_with(Report::default).origin.entry((Sub::Types as u8, t.0)).or_insert(Origin::Record(what, published));
    }

    #[inline]
    fn check(store: Option<&TypeStore>, at: At, sub: Sub, id: u32, loc: &'static Location<'static>) {
        let Some((store, ov, bound, hold)) = context(store, sub) else { return };
        CHECKED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let foreign = foreign_shared(bound, ov.base, id, hold, || store.not_canonical(ov, sub, id));
        if let Some(foreign) = foreign {
            violation(store, Key { at, loc: Some(loc), sub, foreign, hold }, id);
        }
    }

    #[cold]
    #[inline(never)]
    fn violation(store: &TypeStore, key: Key, id: u32) {
        match mode() {
            Mode::Off => {}
            Mode::Panic => panic!("view check: {} (id {}); TEQ_VIEW_CHECKS=report lists every site", describe(&key), id),
            Mode::Report => {
                let mut guard = REPORT.lock().unwrap_or_else(|e| e.into_inner());
                let report = guard.get_or_insert_with(Report::default);
                let at = match report.index.get(&key) {
                    Some(&i) => i,
                    None => {
                        let i = report.sites.len();
                        report.index.insert(key, i);
                        report.sites.push(Site { key, first: 0, reads: 0 });
                        i
                    }
                };
                report.sites[at].reads += 1;
                if report.seen.insert((key.sub as u8, id), ()).is_some() {
                    return;
                }
                report.sites[at].first += 1;
                let origin = *report.origin.entry((key.sub as u8, id)).or_insert(Origin::Entry(at));
                let c = match report.cascade_of.get(&origin) {
                    Some(&c) => c,
                    None => {
                        let c = report.cascades.len();
                        report.cascade_of.insert(origin, c);
                        report.cascades.push((origin, 0, at));
                        print_cascade(c + 1, origin, &key, id);
                        c
                    }
                };
                report.cascades[c].1 += 1;
                for part in parts_of(store, key.sub, id) {
                    report.origin.entry(part).or_insert(origin);
                }
            }
        }
    }

    fn reader(key: &Key) -> &'static str {
        if key.hold { "the loader's lock holder" } else { "a worker" }
    }

    fn describe(key: &Key) -> String {
        format!("{} {} {} from {}{}", reader(key), key.at.describe(), key.sub.name(), key.foreign.name(), key.loc.map_or(String::new(), |l| format!(" at {}", l)))
    }

    fn describe_origin(origin: Origin) -> String {
        match origin {
            Origin::Record(what, false) => format!("handed over in {}", what),
            Origin::Record(what, true) => format!("published by a worker into {}", what),
            Origin::Entry(i) => format!("first read at site {}", i + 1),
        }
    }

    /// A cascade's first foreign id with the frames of teq's own sources that led to its read,
    /// the checks' and the accessors' frames left out.
    fn print_cascade(n: usize, origin: Origin, key: &Key, id: u32) {
        let trace = std::backtrace::Backtrace::force_capture().to_string();
        let mut frames: Vec<String> = Vec::new();
        let mut name = String::new();
        let root = concat!("at ", env!("CARGO_MANIFEST_DIR"), "/src/");
        for line in trace.lines() {
            let line = line.trim_start();
            match line.strip_prefix("at ./src/").or_else(|| line.strip_prefix(root)) {
                Some(at) if !at.starts_with("types/view.rs") => {
                    if frames.len() < 12 && !(frames.is_empty() && at.starts_with("types.rs")) {
                        frames.push(format!("    {} (src/{})", name, at));
                    }
                }
                Some(_) => {}
                None if !line.starts_with("at ") => {
                    let full = line.split_once(": ").map_or(line, |(_, n)| n);
                    name = full.split('<').next().unwrap_or(full).to_string();
                }
                None => {}
            }
        }
        let interp = frames.iter().any(|f| f.contains("(src/interp/"));
        let from = match origin {
            Origin::Record(what, false) => format!(", handed over in {}", what),
            Origin::Record(what, true) => format!(", published by a worker into {}", what),
            Origin::Entry(_) => String::new(),
        };
        eprintln!("view check: cascade {}: {} (id {}){}{}\n{}", n, describe(key), id, from, if interp { ", in the interpreter" } else { "" }, frames.join("\n"));
    }

    /// Whether this invocation typed a program, how many times it forked the body phase and how
    /// many of those forks had the overlays (`TEQ_VIEW_CHECKS_LOG`'s end line).
    static TYPED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    static FORKS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static OVERLAID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    /// The items the workers typed since the fork: a fork with none makes no check.
    static ITEMS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    /// A worker typed an item of the body phase's queue.
    pub fn item_typed() {
        ITEMS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// The workers that began their work in a context the checks apply in (a worker's overlay
    /// entered, the overlays on, a checked namespace): the proof that the checks were active in a
    /// build whatever it read.
    static ACTIVE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    /// A worker begins its work: counted where its reads are checked, and said in the checks'
    /// log at once, so that an invocation that ends before its join (an abort) has its proof too:
    /// which worker of how many, in which fork of the invocation.
    pub fn worker_entered(store: &TypeStore) {
        let context = context(Some(store), Sub::Types);
        if context.is_some() {
            ACTIVE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        // The namespace the line names is the shared base's, every forked build's, which the
        // runner's reconciliation reads (tests/support/fork_one.py).
        let namespace = if store.apart_ref().is_some() { "the Shared" } else { "no" };
        let here = HERE.with(|h| h.get());
        let worker = if here.is_null() { "?".to_string() } else { unsafe { &*here }.worker.to_string() };
        let workers = store.apart_ref().map_or(0, |a| a.workers.len());
        let fork = FORKS.load(std::sync::atomic::Ordering::Relaxed);
        log_line(&format!("worker {}: {} namespace, mode {}, {}; worker {} of {} in fork {}", std::process::id(), namespace, mode_name(), if context.is_some() { "in a checked context" } else { "outside the checked context" }, worker, workers, fork));
    }

    /// The workers' slots the fork left empty (the queue had no first work for them, so they were
    /// neither attached nor started), since the last join.
    static EMPTY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    /// A worker's slot the fork left empty, said in the checks' log as its worker would be, which of
    /// how many in which fork, and kept out of the join's count of the workers that began.
    pub fn slot_left_empty(store: &TypeStore, worker: usize) {
        EMPTY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let workers = store.apart_ref().map_or(0, |a| a.workers.len());
        let fork = FORKS.load(std::sync::atomic::Ordering::Relaxed);
        log_line(&format!("empty {}: worker {} of {} in fork {}", std::process::id(), worker, workers, fork));
    }

    /// The crossings a checked build exercised: a checked context
    /// proves nothing about whether a peer's record was read, so each boundary counts its reads
    /// of a foreign type, said at the join.
    #[derive(Clone, Copy)]
    pub enum Crossing {
        /// A peer's record read through a view, by kind (a signature, a class's info, a type
        /// parameter's bounds, an alias's record), by the worker or by the loader's lock holder.
        PeerSignature,
        PeerClass,
        PeerTypeParam,
        PeerAlias,
        /// Of those, the ones a view translated (its first read of the record).
        PeerTranslated,
        /// A recorded expression type read outside the reader's view and translated.
        ExprType,
        /// A type a consumer met in a record raw (a quote's, a pattern's, a stored body's) and
        /// took into its view (`TypeStore::in_view_here`).
        Consumer,
        /// The loader's lock holder's read of its own worker's record, exported.
        HolderOwn,
        /// An interpreter's run under the loader's lock with its caches kept apart by view, the
        /// holder's (`interp::take_caches`).
        HolderRun,
    }

    const CROSSINGS: usize = 9;
    static CROSSED: [std::sync::atomic::AtomicU64; CROSSINGS] = [const { std::sync::atomic::AtomicU64::new(0) }; CROSSINGS];

    /// `n` crossings of kind `c`.
    pub fn crossed(c: Crossing, n: u64) {
        CROSSED[c as usize].fetch_add(n, std::sync::atomic::Ordering::Relaxed);
    }

    fn crossings() -> String {
        let n: Vec<u64> = CROSSED.iter().map(|c| c.swap(0, std::sync::atomic::Ordering::Relaxed)).collect();
        format!("crossings: peer records {} signatures, {} classes, {} type parameters, {} aliases, {} translated; {} expression types; {} consumers' types; {} holder's own; {} holder's runs", n[0], n[1], n[2], n[3], n[4], n[5], n[6], n[7], n[8])
    }

    /// The parallel attempt gives way to one worker, whose build the process's image is replaced
    /// by (`serial_again`): its workers were checked, its fork never joined.
    pub fn gave_way(why: &str) {
        log_line(&format!("gave way {}: fork {}, {}; {}; {}", std::process::id(), FORKS.load(std::sync::atomic::Ordering::Relaxed), crossings(), crate::typer::bundle::summary_line(), why.replace('\n', " ")));
    }

    fn mode_name() -> &'static str {
        match mode() {
            Mode::Panic => "panic",
            Mode::Report => "report",
            Mode::Off => "off",
        }
    }

    /// A body-time probe's report (a signature published naming a worker's id, an interpreter's
    /// run of a peer's initialisers): a diagnostic of the assertion-enabled builds for the checks'
    /// log alone, never printed and never fatal, the merge's check deciding what survives it
    /// (`Worker::check_merge`). `line` is made only where a log is named.
    pub fn probe(line: impl FnOnce() -> String) {
        if probing() {
            log_line(&format!("probe {}: {}", std::process::id(), line()));
        }
    }

    /// Whether a checks' log is named, which the probes write to (`probe`).
    pub fn probing() -> bool {
        static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *ON.get_or_init(|| std::env::var_os("TEQ_VIEW_CHECKS_LOG").is_some())
    }

    /// A line appended to `TEQ_VIEW_CHECKS_LOG`, where one is named.
    fn log_line(line: &str) {
        let Some(path) = std::env::var_os("TEQ_VIEW_CHECKS_LOG") else { return };
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = f.write_all(format!("view checks: {}\n", line).as_bytes());
        }
    }

    /// The invocation's start in the checks' log, and its end registered for the process's exit:
    /// a line each, so that the log accounts for every compiler invocation, the ones that typed
    /// no program or forked nothing among them (`tests/fork-one.sh` reconciles them).
    pub fn invocation_begin() {
        if std::env::var_os("TEQ_VIEW_CHECKS_LOG").is_none() {
            return;
        }
        let all = crate::argfile::args();
        let args: Vec<String> = all.iter().take(2).cloned().collect();
        let again = if std::env::var_os("TEQ_SERIAL").is_some() { " (typed again by one worker)" } else { "" };
        // The worker count asked for and where from: the command line's, `TEQ_THREADS`, or none
        // (the automatic count, `frontend::automatic_threads`).
        let threads = match all.iter().position(|a| a == "--threads").and_then(|i| all.get(i + 1)) {
            Some(n) => format!("{} (--threads)", n),
            None => match std::env::var("TEQ_THREADS") {
                Ok(n) => format!("{} (TEQ_THREADS)", n),
                Err(_) => "automatic".to_string(),
            },
        };
        log_line(&format!("start {} {}{}; threads {}", std::process::id(), args.join(" "), again, threads));
        extern "C" {
            fn atexit(f: extern "C" fn()) -> std::os::raw::c_int;
        }
        unsafe { atexit(invocation_end) };
    }

    extern "C" fn invocation_end() {
        let o = std::sync::atomic::Ordering::Relaxed;
        log_line(&format!("end {}: {}, {} forks, {} with the overlays", std::process::id(), if TYPED.load(o) { "typed" } else { "not typed" }, FORKS.load(o), OVERLAID.load(o)));
    }

    /// The typer runs on a program.
    pub fn typing() {
        TYPED.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// The body phase forked, with the overlays or without.
    pub fn forked(overlays: bool) {
        FORKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if overlays {
            OVERLAID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// What the checks met since the fork (`TEQ_VIEW_CHECKS=report`), printed at the join: each
    /// cascade in the order of its first foreign id, with where its ids came from and how many
    /// distinct ones it gave; then every site with the ids first read there and all its reads.
    /// The counts start again for the next fork.
    pub fn summary(workers: usize) {
        let checked = CHECKED.swap(0, std::sync::atomic::Ordering::Relaxed);
        let published = PUBLISHED.swap(0, std::sync::atomic::Ordering::Relaxed);
        let mode_name = mode_name();
        let items = ITEMS.swap(0, std::sync::atomic::Ordering::Relaxed);
        let active = ACTIVE.swap(0, std::sync::atomic::Ordering::Relaxed);
        let began = workers.saturating_sub(EMPTY.swap(0, std::sync::atomic::Ordering::Relaxed) as usize);
        log_line(&format!("join {}: the Shared namespace, {} workers, mode {}, {} reads and constructions checked, {} publications checked, {} items typed by the workers, {} of {} workers in a checked context; fork {}; {}; {}", std::process::id(), workers, mode_name, checked, published, items, active, began, FORKS.load(std::sync::atomic::Ordering::Relaxed), crossings(), crate::typer::bundle::summary_line()));
        if mode() != Mode::Report {
            return;
        }
        let Some(report) = REPORT.lock().unwrap_or_else(|e| e.into_inner()).take() else { return };
        let (first, reads) = report.sites.iter().fold((0, 0), |(f, r), s| (f + s.first, r + s.reads));
        eprintln!("view check: {} cascades, {} sites, {} distinct foreign ids, {} violations", report.cascades.len(), report.sites.len(), first, reads);
        for (i, &(origin, ids, at)) in report.cascades.iter().enumerate() {
            eprintln!("view check: cascade {}: {} ids, {}; first at site {}: {}", i + 1, ids, describe_origin(origin), at + 1, describe(&report.sites[at].key));
        }
        for (i, s) in report.sites.iter().enumerate() {
            eprintln!("view check: site {}: {} ids first, {} in all: {}", i + 1, s.first, s.reads, describe(&s.key));
        }
    }
}

#[cfg(not(debug_assertions))]
pub fn summary(_workers: usize) {}

#[cfg(not(debug_assertions))]
pub fn invocation_begin() {}

#[cfg(not(debug_assertions))]
pub fn typing() {}

#[cfg(not(debug_assertions))]
pub fn forked(_overlays: bool) {}

#[cfg(not(debug_assertions))]
pub fn item_typed() {}

#[cfg(not(debug_assertions))]
pub fn worker_entered(_store: &TypeStore) {}

#[cfg(not(debug_assertions))]
pub fn slot_left_empty(_store: &TypeStore, _worker: usize) {}

#[cfg(not(debug_assertions))]
pub fn gave_way(_why: &str) {}

#[cfg(not(debug_assertions))]
pub fn probe(_line: impl FnOnce() -> String) {}

#[cfg(all(test, not(debug_assertions)))]
pub fn unchecked<R>(f: impl FnOnce() -> R) -> R {
    f()
}
