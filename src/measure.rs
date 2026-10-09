//! The parallel typer's measurement: the
//! phases of the type phase, and the diagnostic switches its experiments run under, which `--time`
//! reports as its "workers" section. A build that gives way to one worker prints its attempt's
//! section before the process is replaced (`serial_again`), so that what the attempt measured is
//! not lost with its process image.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Whether the counters and clocks run: with `--time` (`TEQ_WORKERS_TIME=0` turns them off, for
/// their own cost). Off, a counted site is this load and a branch.
static ON: AtomicBool = AtomicBool::new(false);

pub fn enable(on: bool) {
    let on = on && std::env::var_os("TEQ_WORKERS_TIME").map_or(true, |v| v != "0");
    if on {
        // The clock every interval is measured from, set before any is taken.
        epoch();
    }
    ON.store(on, Ordering::Relaxed);
}

/// `TEQ_SESSION_INVENTORY=<file>`, in the assertion-enabled build: lines naming what a session's
/// full builds and their workers leave behind, by name and size,
/// appended. Elsewhere off, and every site of it compiled out.
pub fn inventory_on() -> bool {
    static PATH: std::sync::OnceLock<Option<std::ffi::OsString>> = std::sync::OnceLock::new();
    cfg!(debug_assertions) && PATH.get_or_init(|| std::env::var_os("TEQ_SESSION_INVENTORY")).is_some()
}

pub fn inventory_line(what: &str, items: &[(&str, usize)]) {
    let Some(path) = std::env::var_os("TEQ_SESSION_INVENTORY") else { return };
    let mut line = format!("{} {}", std::process::id(), what);
    for (name, value) in items {
        line.push_str(&format!("; {}={}", name, value));
    }
    line.push('\n');
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        use std::io::Write;
        let _ = f.write_all(line.as_bytes());
    }
}

#[inline(always)]
pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// Whether `--time` prints the "workers" section at one worker too (`TEQ_WORKERS_TIME=1`), for
/// the controls of a ladder; a build that forks or gave way prints it anyway.
pub fn section_wanted() -> bool {
    std::env::var_os("TEQ_WORKERS_TIME").is_some_and(|v| v == "1") || std::env::var_os("TEQ_FORK").is_some_and(|v| v == "1")
}

/// `TEQ_STATE_GIVEWAY=off`, a diagnostic: a macro's run or a
/// constant's folding that changed state other runs share no longer gives the build to one
/// worker (`Worker::shared_state_changed`); each is counted by its reason, and what the build
/// then prints or writes may differ from one worker's, which is what the diagnostic is run to
/// count. Nothing of it is the compiler's behaviour.
pub fn state_giveway_off() -> bool {
    static STATE: AtomicU8 = AtomicU8::new(0);
    match STATE.load(Ordering::Relaxed) {
        1 => false,
        2 => true,
        _ => {
            let off = std::env::var("TEQ_STATE_GIVEWAY").is_ok_and(|v| v == "off");
            STATE.store(if off { 2 } else { 1 }, Ordering::Relaxed);
            off
        }
    }
}

static STATE_BYPASSED: Mutex<Vec<(String, u64)>> = Mutex::new(Vec::new());

/// `--macro-state per-worker`, for the report's note on the changes let through.
static PER_WORKER: AtomicBool = AtomicBool::new(false);

pub fn set_macro_state_per_worker(on: bool) {
    PER_WORKER.store(on, Ordering::Relaxed);
}

/// A change of shared state that would have given the build away, let through under
/// `TEQ_STATE_GIVEWAY=off`, counted by its reason.
pub fn state_bypassed(why: String) {
    let mut b = STATE_BYPASSED.lock().unwrap_or_else(|e| e.into_inner());
    match b.iter_mut().find(|(w, _)| *w == why) {
        Some((_, n)) => *n += 1,
        None => b.push((why, 1)),
    }
}

/// The identity hashes asked while the counters run (`interp::identity`): every request, a kept
/// hash's included; per kind of value the first computations, the nodes their walks read, the
/// cuts by reason and the time; and the collisions, distinct live values of one class asked in
/// one run with one hash.
static IDENT_REQUESTS: AtomicU64 = AtomicU64::new(0);
static IDENT_FIRST: [AtomicU64; 5] = [const { AtomicU64::new(0) }; 5];
static IDENT_NODES: AtomicU64 = AtomicU64::new(0);
static IDENT_MOST_NODES: AtomicU64 = AtomicU64::new(0);
static IDENT_CUTS: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
static IDENT_NS: AtomicU64 = AtomicU64::new(0);
static IDENT_COLLISIONS: AtomicU64 = AtomicU64::new(0);

pub fn identity_requested() {
    IDENT_REQUESTS.fetch_add(1, Ordering::Relaxed);
}

pub fn identity_computed(kind: crate::interp::identity::Kind, nodes: u32, cuts: [u32; 4], ns: u64) {
    IDENT_FIRST[kind as usize].fetch_add(1, Ordering::Relaxed);
    IDENT_NODES.fetch_add(nodes as u64, Ordering::Relaxed);
    IDENT_MOST_NODES.fetch_max(nodes as u64, Ordering::Relaxed);
    for (c, n) in IDENT_CUTS.iter().zip(cuts) {
        c.fetch_add(n as u64, Ordering::Relaxed);
    }
    IDENT_NS.fetch_add(ns, Ordering::Relaxed);
}

pub fn identity_collided() {
    IDENT_COLLISIONS.fetch_add(1, Ordering::Relaxed);
}

/// The identity hashes' rows of the "workers" section, when any was asked.
fn report_identity(section: &mut crate::report::Section) {
    let requests = IDENT_REQUESTS.load(Ordering::Relaxed);
    if requests == 0 {
        return;
    }
    let first: Vec<u64> = IDENT_FIRST.iter().map(|c| c.load(Ordering::Relaxed)).collect();
    let computed: u64 = first.iter().sum();
    let b = crate::interp::identity::bounds();
    section.row("identity hashes asked").count(requests as usize).note(format!("{} computed: objects {}, functions {}, arrays {}, maps {}, tries {}", computed, first[0], first[1], first[2], first[3], first[4]));
    let cuts: Vec<u64> = IDENT_CUTS.iter().map(|c| c.load(Ordering::Relaxed)).collect();
    section.row("identity hashes' walks").time(Duration::from_nanos(IDENT_NS.load(Ordering::Relaxed))).note(format!(
        "{} nodes, at most {} in one; cut by depth {}, cycle {}, nodes {}, elements {}; bounds depth {}, elements {}, nodes {}, bytes {}",
        IDENT_NODES.load(Ordering::Relaxed),
        IDENT_MOST_NODES.load(Ordering::Relaxed),
        cuts[0],
        cuts[1],
        cuts[2],
        cuts[3],
        b.depth,
        b.elements,
        b.nodes,
        b.bytes
    ));
    section.row("identity hash collisions").count(IDENT_COLLISIONS.load(Ordering::Relaxed) as usize).note("distinct live values of one class, one hash, one run".to_string());
}

/// The type phase's parts in their order, each with its wall time, and the moment a worker first
/// asked for the build to be typed again by one (`Worker::need_serial_for_state`).
struct Phases {
    start: Option<Instant>,
    last: Option<Instant>,
    parts: Vec<(&'static str, Duration)>,
    gave_way: Option<Duration>,
}

static PHASES: Mutex<Phases> = Mutex::new(Phases { start: None, last: None, parts: Vec::new(), gave_way: None });

fn phases() -> std::sync::MutexGuard<'static, Phases> {
    PHASES.lock().unwrap_or_else(|e| e.into_inner())
}

/// The type phase begins: the parts are timed from here.
pub fn phases_begin() {
    let now = Instant::now();
    let mut p = phases();
    *p = Phases { start: Some(now), last: Some(now), parts: Vec::new(), gave_way: None };
    MERGE_PARTS.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

/// The fork's and the merge's parts (`Worker::fork_parts`, `merge.rs`), each with its wall time,
/// in their order: rows of the "workers" section after the phases, the `fork` and `merge` phases'
/// totals kept.
static MERGE_PARTS: Mutex<Vec<(String, Duration)>> = Mutex::new(Vec::new());

/// The merge's part `name` took `d`; a part met again adds to its row.
pub fn merge_part(name: &str, d: Duration) {
    part_done("merge", name, d)
}

/// The fork's part `name` took `d`.
pub fn fork_part(name: &str, d: Duration) {
    part_done("fork", name, d)
}

fn part_done(phase: &str, name: &str, d: Duration) {
    if !on() {
        return;
    }
    let name = format!("{} {}", phase, name);
    let mut parts = MERGE_PARTS.lock().unwrap_or_else(|e| e.into_inner());
    match parts.iter_mut().find(|(n, _)| *n == name) {
        Some((_, t)) => *t += d,
        None => parts.push((name, d)),
    }
}

/// The part `name` ends now, begun where the last one ended.
pub fn phase_done(name: &'static str) {
    let now = Instant::now();
    let mut p = phases();
    let Some(last) = p.last else { return };
    p.parts.push((name, now - last));
    p.last = Some(now);
    PHASE_NO.store(p.parts.len(), Ordering::Relaxed);
}

/// The part under way: its index in `Phases::parts`, the parts' count after the last.
static PHASE_NO: AtomicUsize = AtomicUsize::new(0);

fn phase_name(parts: &[(&'static str, Duration)], i: usize) -> &'static str {
    parts.get(i).map_or("after the type phase", |p| p.0)
}

/// A worker asked for the build to be typed again by one; the first ask is kept.
pub fn gave_way() {
    let now = Instant::now();
    let mut p = phases();
    if let (Some(start), None) = (p.start, p.gave_way) {
        p.gave_way = Some(now - start);
    }
}

/// What this process measured, as sections of the `--time` report: the "workers" section (the
/// parts of the type phase, the moment of the give-way, what the diagnostics let through) and,
/// while the counters ran, the macro reach, the bodies typed and the workers' waits and holds.
/// `attempt` names them as the parallel attempt's that gave way.
pub fn report(report: &mut crate::report::Report, attempt: bool, lead: impl FnOnce(&mut crate::report::Section)) {
    let p = phases();
    let section = report.section(if attempt { "workers, the attempt that gave way" } else { "workers" });
    lead(section);
    for &(name, d) in &p.parts {
        section.row(&format!("phase: {}", name)).time(d);
    }
    for (name, d) in MERGE_PARTS.lock().unwrap_or_else(|e| e.into_inner()).iter() {
        section.row(name).time(*d);
    }
    if let Some(d) = p.gave_way {
        section.row("give-way asked").time(d).note("into the type phase".to_string());
    }
    let prep = PREP.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if let Some(kinds) = &prep.0 {
        let (n, done) = kinds.iter().fold((0, 0), |(n, d), k| (n + k.1, d + k.2));
        let what: Vec<String> = kinds.iter().map(|(k, n, d)| format!("{} {} of {}", k, d, n)).collect();
        section.row("preparation replayed").count(done).note(format!("of {} keys: {}", n, what.join(", ")));
    }
    if let Some(n) = prep.1 {
        section.row(if prep.0.is_some() { "preparation demanded after the replay" } else { "preparation captured" }).count(n).note("keys, TEQ_PREP_CAPTURE".to_string());
    }
    report_identity(section);
    let per_worker = PER_WORKER.load(Ordering::Relaxed);
    if state_giveway_off() || per_worker {
        let mut b = STATE_BYPASSED.lock().unwrap_or_else(|e| e.into_inner()).clone();
        b.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let total: u64 = b.iter().map(|(_, n)| n).sum();
        let why = if per_worker { "--macro-state per-worker" } else { "TEQ_STATE_GIVEWAY=off, a diagnostic" };
        section.row("shared state changes let through").count(total as usize).note(why.to_string());
        for (why, n) in b {
            section.detail(format!("{} x {}", n, why));
        }
    }
    report_counters(report, attempt, &p.parts);
    report_overlays(report);
}

/// Where a body typed stands: the program's, the std's or a library's (a jar's member, or a
/// definition of a library body's pseudo file, the anonymous classes an expansion copied from a
/// library's quoted code among them).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum BodyOrigin {
    Program,
    Std,
    Library,
}

/// Why a body was typed: the walk over the files (the items, and what their typing types on the
/// way), an inline method's body typed again for an expansion, for an inferred result type
/// another body read, or on demand: by a macro's run or a constant's folding under way on the
/// thread, or by the reach pass after the type phase.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum BodyReason {
    Walk,
    Inline,
    Inferred,
    Demand,
}

/// The bodies of one kind a thread typed: how many, their own time (without the bodies typed
/// inside them) and their time inclusive.
#[derive(Clone, Default)]
struct BodyCount {
    n: u64,
    self_ns: u64,
    incl_ns: u64,
}

/// What a thread measured, flushed under its worker's number (`flush`). The waits and the holds
/// are counted while the thread works the body phase's queue (`work_begin` to `work_end`).
#[derive(Clone, Default)]
struct Stats {
    bodies: Vec<((usize, BodyOrigin, BodyReason), BodyCount)>,
    /// Per `Wait`, how many and their time.
    waits: [(u64, u64); WAITS],
    /// Per `Hold`, the loader's lock held for it, without what nested categories took: how many
    /// stretches and their time.
    holds: [(u64, u64); HOLDS],
    /// The outermost holds of the loader's lock: how many and their time, acquisition excluded.
    held: (u64, u64),
    /// The waits inside those holds (the holder waiting for the type store's lock): how many and
    /// their time, which the held time holds too.
    waits_in_hold: (u64, u64),
    /// The outermost holds by the category under way when the lock was taken: how many and
    /// their whole time, nested categories included.
    holds_by_entry: [(u64, u64); HOLDS],
    /// The thread's part of the body phase: its wall time and its CPU time, and the items it took.
    elapsed_ns: u64,
    cpu_ns: u64,
    items: u64,
    /// On Linux, the thread's time runnable and waiting for a CPU over its work.
    run_delay_ns: u64,
    /// The trace's count of the loader's lock's parks, and of its releases: all, those that
    /// found a thread waiting, those that woke one.
    lock_parks: u64,
    lock_releases: (u64, u64, u64),
    /// The items' records (`item_end`).
    item_rows: Vec<ItemRow>,
    /// The bounded trace's events, and how many did not fit.
    trace: Vec<Event>,
    trace_dropped: u64,
    /// The thread's number (`shared::thread_number`), which a cell's claimant is known by.
    thread: usize,
    /// Per `Lookup`, under the loader's lock: the calls, how many entered or found something,
    /// and how many took the lock themselves (the outermost hold); and the calls answered
    /// outside the lock, with nothing to enter.
    lookups: [(u64, u64, u64); LOOKUPS],
    unlocked: [u64; LOOKUPS],
    /// With `TEQ_WORKERS_TYPES=1`, per store of `shared::Serial` (the type store, the interner,
    /// the shared maps): how many holds of its insertion lock, their time, and how many took
    /// under 2^k ns, per k.
    serial_holds: [(u64, u64, [u64; 32], [u64; 32]); 3],
}

impl Stats {
    fn absorb(&mut self, other: Stats) {
        for (k, c) in other.bodies {
            let e = self.body(k);
            e.n += c.n;
            e.self_ns += c.self_ns;
            e.incl_ns += c.incl_ns;
        }
        for i in 0..WAITS {
            self.waits[i].0 += other.waits[i].0;
            self.waits[i].1 += other.waits[i].1;
        }
        for i in 0..HOLDS {
            self.holds[i].0 += other.holds[i].0;
            self.holds[i].1 += other.holds[i].1;
        }
        self.held.0 += other.held.0;
        self.held.1 += other.held.1;
        self.waits_in_hold.0 += other.waits_in_hold.0;
        self.waits_in_hold.1 += other.waits_in_hold.1;
        for i in 0..HOLDS {
            self.holds_by_entry[i].0 += other.holds_by_entry[i].0;
            self.holds_by_entry[i].1 += other.holds_by_entry[i].1;
        }
        self.elapsed_ns += other.elapsed_ns;
        self.cpu_ns += other.cpu_ns;
        self.items += other.items;
        self.run_delay_ns += other.run_delay_ns;
        self.lock_parks += other.lock_parks;
        self.lock_releases.0 += other.lock_releases.0;
        self.lock_releases.1 += other.lock_releases.1;
        self.lock_releases.2 += other.lock_releases.2;
        self.item_rows.extend(other.item_rows);
        self.trace.extend(other.trace);
        self.trace_dropped += other.trace_dropped;
        self.thread = self.thread.max(other.thread);
        for i in 0..LOOKUPS {
            self.lookups[i].0 += other.lookups[i].0;
            self.lookups[i].1 += other.lookups[i].1;
            self.lookups[i].2 += other.lookups[i].2;
            self.unlocked[i] += other.unlocked[i];
        }
        for i in 0..3 {
            self.serial_holds[i].0 += other.serial_holds[i].0;
            self.serial_holds[i].1 += other.serial_holds[i].1;
            for k in 0..32 {
                self.serial_holds[i].2[k] += other.serial_holds[i].2[k];
                self.serial_holds[i].3[k] += other.serial_holds[i].3[k];
            }
        }
    }

    /// The counts without the rows of the items and the trace.
    fn counts(&self) -> Stats {
        Stats { bodies: self.bodies.clone(), waits: self.waits, holds: self.holds, held: self.held, waits_in_hold: self.waits_in_hold, holds_by_entry: self.holds_by_entry, serial_holds: self.serial_holds, lookups: self.lookups, unlocked: self.unlocked, elapsed_ns: self.elapsed_ns, cpu_ns: self.cpu_ns, items: self.items, run_delay_ns: self.run_delay_ns, lock_parks: self.lock_parks, lock_releases: self.lock_releases, trace_dropped: self.trace_dropped, thread: self.thread, ..Stats::default() }
    }

    fn waited_ns(&self) -> u64 {
        self.waits.iter().map(|w| w.1).sum()
    }

    /// Wall time neither waiting outside the loader's lock nor holding it: the waits inside a
    /// hold are the hold's time already.
    fn unattributed_ns(&self) -> u64 {
        self.elapsed_ns.saturating_sub(self.waited_ns() - self.waits_in_hold.1 + self.held.1)
    }

    fn body(&mut self, k: (usize, BodyOrigin, BodyReason)) -> &mut BodyCount {
        let i = match self.bodies.iter().position(|(key, _)| *key == k) {
            Some(i) => i,
            None => {
                self.bodies.push((k, BodyCount::default()));
                self.bodies.len() - 1
            }
        };
        &mut self.bodies[i].1
    }
}

/// What a worker waits for: the loader's lock, the type store's, the interner's and the shared
/// maps' insertion locks (`shared::Serial`), a completion cell of each kind (the wait graph's
/// own mutex inside the cell's time), a wait the graph refused (a cycle, which gives the build
/// away), and a record another worker staged under the loader's lock, waited for until the
/// lock's release publishes it (`SharedArena::wait_published`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wait {
    Loader,
    TypeStore,
    Interner,
    SharedMap,
    CellSignature,
    CellBody,
    CellClass,
    CellAlias,
    CellCheck,
    CellRefused,
    Published,
}

const WAITS: usize = 11;

const WAIT_NAMES: [&str; WAITS] = [
    "the loader's lock",
    "the type store's lock",
    "the interner's lock",
    "a shared map's lock",
    "a signature's cell",
    "a body's cell",
    "a class completion's cell",
    "an alias's cell",
    "a class check's cell",
    "a wait refused (a cycle)",
    "a staged record's publication",
];

impl Wait {
    /// The wait on a completion cell of the kind `check::CELL_*`.
    pub fn cell(kind: u8) -> Wait {
        match kind {
            1 => Wait::CellSignature,
            2 => Wait::CellBody,
            3 => Wait::CellClass,
            4 => Wait::CellAlias,
            _ => Wait::CellCheck,
        }
    }
}

/// What the loader's lock is held for, each stretch of a hold charged to the innermost category
/// under way on the thread: taking it (the refresh of what other workers entered, the working
/// copies made), a class of the std checked, a jar's class converted or checked, a class or an
/// alias of the std or a jar completed, a signature of the std or a jar completed, a std or
/// library body typed, a name asked of the std's files (`enter_std_for`, which enters the file
/// that defines it, if any), a std file entered otherwise, a name asked of a jar's package
/// (`load_pkg_member`), a jar's or the JDK's class or package objects entered otherwise, a
/// program's result (a body, a signature, a class check) published, releasing it (the staged
/// records published, the maps applied, the working copies flushed), and the rest.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hold {
    Entry,
    StdClassCheck,
    LibraryConversion,
    LibraryClassCheck,
    ClassCompletion,
    AliasCompletion,
    Signature,
    Body,
    StdLookup,
    StdEntry,
    JarLookup,
    JarEntry,
    Publication,
    Release,
    Other,
}

const HOLDS: usize = 15;

const HOLD_NAMES: [&str; HOLDS] = [
    "taking it: refresh and working copies",
    "a std class checked",
    "a jar class converted",
    "a jar class checked",
    "a class completed (std or jar)",
    "an alias completed (std or jar)",
    "a signature completed (std or jar)",
    "a std or library body typed",
    "a name asked of the std's files",
    "a std file entered otherwise",
    "a name asked of a jar's package",
    "a jar's class entered otherwise",
    "a program result published",
    "releasing it: publish, apply, flush",
    "other",
];

/// A lookup the loader's lock is taken for: a name asked of a jar's package (`load_pkg_member`),
/// of the std's files (`enter_std_for`), a Java class file asked of a package, a package's
/// objects asked for, a Java class's members or constructors asked for (`absorb_java_class`,
/// `absorb_java_ctors`, `enter_java_builtin_members`). Each is counted with whether it entered
/// or found something.
#[derive(Clone, Copy)]
pub enum Lookup {
    JarMember,
    StdName,
    JavaClassFile,
    PackageObjects,
    JavaClassEntry,
}

const LOOKUPS: usize = 5;

const LOOKUP_NAMES: [&str; LOOKUPS] = [
    "a name asked of a jar's package",
    "a name asked of the std's files",
    "a Java class file asked of a package",
    "a package's objects asked for",
    "a Java class's members asked for",
];

/// A lookup under the loader's lock ended, having entered or found something or not.
pub fn looked_up(what: Lookup, found: bool) {
    if !on() || !in_work() {
        return;
    }
    let outermost = crate::shared::lock_depth() == 1;
    STATS.with(|s| {
        let l = &mut s.borrow_mut().lookups[what as usize];
        l.0 += 1;
        l.1 += found as u64;
        l.2 += outermost as u64;
    });
}

/// A lookup answered outside the loader's lock, with nothing to enter.
pub fn looked_up_unlocked(what: Lookup) {
    if !on() || !in_work() {
        return;
    }
    STATS.with(|s| s.borrow_mut().unlocked[what as usize] += 1);
}

/// A row of the bounded trace (`TEQ_WORKERS_TRACE`): what, with its detail, from `t0` to `t1`
/// in ns since the process's measurement began.
#[derive(Clone, Copy)]
struct Event {
    what: u8,
    detail: u8,
    arg: u32,
    t0: u64,
    t1: u64,
}

/// The kinds of trace rows.
const EV_WAIT: u8 = 1;
const EV_HELD: u8 = 2;
const EV_HOLD: u8 = 3;
const EV_ITEM: u8 = 4;
const EV_WORK: u8 = 5;
/// The loader's lock: a waiter parked, from the park to its wake;
/// a waiter took it, from its wait's start to the acquisition, `arg` its parks; a holder
/// released it, from the release to the wake's return, `arg` the threads waiting then and
/// `detail` whether one was woken; and, on Linux, the thread's time runnable and not running
/// over its work (`t0`, the scheduler's run delay) with its CPU time (`t1`) and its slices
/// (`arg`).
const EV_PARK: u8 = 6;
const EV_TAKEN: u8 = 7;
const EV_RELEASE: u8 = 8;
const EV_SCHED: u8 = 9;

/// The events a thread keeps; the rest are counted.
const TRACE_CAP: usize = 1 << 21;

/// One item of the queue a worker typed: its file and its index in the file, when it began and
/// ended, the waits inside it and its CPU time.
#[derive(Clone, Copy)]
struct ItemRow {
    worker: u32,
    file: u32,
    /// The file's size in bytes, by which the queue orders the files for several workers.
    bytes: u32,
    index: u32,
    t0: u64,
    t1: u64,
    waited_ns: u64,
    cpu_ns: u64,
}

fn epoch() -> Instant {
    static EPOCH: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    *EPOCH.get_or_init(Instant::now)
}

fn since(t: Instant) -> u64 {
    t.saturating_duration_since(epoch()).as_nanos() as u64
}

fn tracing() -> bool {
    static STATE: AtomicU8 = AtomicU8::new(0);
    match STATE.load(Ordering::Relaxed) {
        1 => false,
        2 => true,
        _ => {
            let on = std::env::var_os("TEQ_WORKERS_TRACE").is_some();
            STATE.store(if on { 2 } else { 1 }, Ordering::Relaxed);
            on
        }
    }
}

fn trace(s: &mut Stats, e: Event) {
    if s.trace.len() < TRACE_CAP {
        s.trace.push(e);
    } else {
        s.trace_dropped += 1;
    }
}

/// This thread's CPU time, in ns.
#[cfg(unix)]
fn thread_cpu_ns() -> u64 {
    #[repr(C)]
    struct Timespec {
        sec: i64,
        nsec: i64,
    }
    extern "C" {
        fn clock_gettime(clock: i32, tp: *mut Timespec) -> i32;
    }
    #[cfg(target_os = "macos")]
    const CLOCK_THREAD_CPUTIME_ID: i32 = 16;
    #[cfg(not(target_os = "macos"))]
    const CLOCK_THREAD_CPUTIME_ID: i32 = 3;
    let mut t = Timespec { sec: 0, nsec: 0 };
    if unsafe { clock_gettime(CLOCK_THREAD_CPUTIME_ID, &mut t) } != 0 {
        return 0;
    }
    t.sec as u64 * 1_000_000_000 + t.nsec as u64
}

/// This thread's CPU time, in ns: its kernel and user times, which Windows counts in 100 ns
/// units but advances at the clock's interrupt (15.6 ms by default), so a short stretch reads
/// as nothing or as a whole tick.
#[cfg(windows)]
fn thread_cpu_ns() -> u64 {
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentThread() -> *mut std::ffi::c_void;
        fn GetThreadTimes(thread: *mut std::ffi::c_void, creation: *mut FileTime, exit: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
    }
    let mut t = [FileTime::default(); 4];
    let [creation, exit, kernel, user] = &mut t;
    // SAFETY: the calling thread's pseudo-handle, four times written by the call.
    if unsafe { GetThreadTimes(GetCurrentThread(), creation, exit, kernel, user) } == 0 {
        return 0;
    }
    let ticks = |f: &FileTime| (f.high as u64) << 32 | f.low as u64;
    (ticks(kernel) + ticks(user)) * 100
}

/// This thread's scheduler counts on Linux (`/proc/thread-self/schedstat`): its time on a CPU,
/// its time runnable and waiting for one, and its slices.
fn sched_counts() -> Option<(u64, u64, u64)> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let text = std::fs::read_to_string("/proc/thread-self/schedstat").ok()?;
    let mut f = text.split_whitespace().map(|n| n.parse::<u64>().ok());
    Some((f.next()??, f.next()??, f.next()??))
}

/// The thread's part of the body phase: when it began, its CPU time then, and the worker.
struct Work {
    worker: u32,
    start: Instant,
    cpu: u64,
    sched: Option<(u64, u64, u64)>,
    /// The item under way: when it began, the waits before it, its CPU time then.
    item: Option<(Instant, u64, u64)>,
}

/// The loader's lock as this thread holds it: when the outermost hold began and when its last
/// stretch was charged, the category under way when it was taken, and the categories under way,
/// innermost last.
struct Holding {
    since: Option<(Instant, Instant)>,
    entry: Hold,
    cats: Vec<Hold>,
}

thread_local! {
    /// Set by `infer_body` for the body typed next, while the counters run: typed for an
    /// inferred result type (`BodyReason::Inferred`).
    static TYPING_INFERRED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// How many macro runs are under way on this thread: a body typed meanwhile is the run's
    /// demand (`BodyReason::Demand`).
    static MACRO_RUNS: std::cell::Cell<i32> = const { std::cell::Cell::new(0) };
    static STATS: RefCell<Stats> = RefCell::new(Stats::default());
    static WORK: RefCell<Option<Work>> = const { RefCell::new(None) };
    static HOLDING: RefCell<Holding> = const { RefCell::new(Holding { since: None, entry: Hold::Other, cats: Vec::new() }) };
    /// The bodies being typed on this thread, outermost first: when each began and the time of
    /// the bodies typed inside it so far.
    static OPEN_BODIES: RefCell<Vec<(Instant, u64)>> = const { RefCell::new(Vec::new()) };
}

/// The threads' measurements by worker, in the order they were flushed.
static FLUSHED: Mutex<Vec<(usize, Stats)>> = Mutex::new(Vec::new());

/// Hands this thread's measurements over as worker `worker`'s: at the end of its part of the body
/// phase, and on the typing thread at the end of the type phase.
pub fn flush(worker: usize) {
    if !on() {
        return;
    }
    let stats = STATS.with(|s| std::mem::take(&mut *s.borrow_mut()));
    let mut f = FLUSHED.lock().unwrap_or_else(|e| e.into_inner());
    match f.iter_mut().find(|(w, _)| *w == worker) {
        Some((_, s)) => s.absorb(stats),
        None => f.push((worker, stats)),
    }
}

/// A body's typing begins (`Worker::type_body`); `body_end` with its kind ends it.
pub fn body_begin() {
    OPEN_BODIES.with(|o| o.borrow_mut().push((Instant::now(), 0)));
}

pub fn body_end(origin: BodyOrigin, reason: BodyReason) {
    let Some((start, child)) = OPEN_BODIES.with(|o| o.borrow_mut().pop()) else { return };
    let incl = start.elapsed().as_nanos() as u64;
    OPEN_BODIES.with(|o| {
        if let Some(parent) = o.borrow_mut().last_mut() {
            parent.1 += incl;
        }
    });
    let phase = PHASE_NO.load(Ordering::Relaxed);
    STATS.with(|s| {
        let mut s = s.borrow_mut();
        let c = s.body((phase, origin, reason));
        c.n += 1;
        c.self_ns += incl.saturating_sub(child);
        c.incl_ns += incl;
    });
}

/// The macro reach's make-up (`Worker::macro_reach`): its mode, the program's files, the files
/// by their distance from the files with quoted code, and the names that carried the most.
pub struct Reach {
    pub mode: &'static str,
    pub program_files: usize,
    /// Files per ring: the seed (the files with quoted code), then the files a ring's mentions
    /// reach first, to the fixed point.
    pub rings: Vec<usize>,
    /// Per name that carried a file into a ring: the files it carried, and the ones no other
    /// name carried. A file of ring k+1 is carried by every name a file of ring k mentions and
    /// it defines at its top level.
    pub names: Vec<(String, usize, usize)>,
    /// The files typed before the fork, in the order of the walk.
    pub files: usize,
    /// Per file whose mentions reached files of the next ring: its path, its ring and how many
    /// of the next ring's files it reached, the most first.
    pub fanout: Vec<(String, usize, usize)>,
}

static REACH: Mutex<Option<Reach>> = Mutex::new(None);

pub fn reach_measured(r: Reach) {
    *REACH.lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
}

fn ms(ns: u64) -> String {
    format!("{:.1} ms", ns as f64 / 1e6)
}

fn origin_name(o: BodyOrigin) -> &'static str {
    match o {
        BodyOrigin::Program => "program",
        BodyOrigin::Std => "std",
        BodyOrigin::Library => "library",
    }
}

fn reason_name(r: BodyReason) -> &'static str {
    match r {
        BodyReason::Walk => "walk",
        BodyReason::Inline => "inline expansion",
        BodyReason::Inferred => "inferred",
        BodyReason::Demand => "demand",
    }
}

/// The counters' sections: the reach, the bodies typed, the workers' waits and holds.
fn report_counters(report: &mut crate::report::Report, attempt: bool, parts: &[(&'static str, Duration)]) {
    if let Some(r) = REACH.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let section = report.section(if attempt { "the attempt's macro reach" } else { "macro reach" });
        let rings: Vec<String> = r.rings.iter().enumerate().map(|(i, n)| if i == 0 { format!("seed {}", n) } else { format!("ring {} {}", i, n) }).collect();
        section.row(&format!("files typed before the fork ({})", r.mode)).count(r.files).note(format!("of {} program files; {}", r.program_files, rings.join(", ")));
        for (name, carried, alone) in r.names.iter().take(12) {
            section.detail(format!("name {}: carried {} files, {} alone", name, carried, alone));
        }
        for (path, ring, n) in r.fanout.iter().take(12) {
            section.detail(format!("file {} (ring {}): reached {} files of ring {}", path, ring, n, ring + 1));
        }
    }
    let flushed = FLUSHED.lock().unwrap_or_else(|e| e.into_inner());
    if flushed.is_empty() {
        return;
    }
    let mut total = Stats::default();
    for (_, s) in flushed.iter() {
        total.absorb(s.counts());
    }
    let mut kinds = total.bodies.clone();
    kinds.sort_by_key(|(k, _)| *k);
    let (n, self_ns) = kinds.iter().fold((0, 0), |(n, t), (_, c)| (n + c.n, t + c.self_ns));
    let section = report.section(if attempt { "the attempt's bodies typed" } else { "bodies typed" });
    section.row("bodies").count(n as usize).note(format!("{} of their own time", ms(self_ns)));
    for ((phase, origin, reason), c) in &kinds {
        section.detail(format!("{} / {} / {}: {} bodies, {} own, {} inclusive", phase_name(parts, *phase), origin_name(*origin), reason_name(*reason), c.n, ms(c.self_ns), ms(c.incl_ns)));
    }
    let mut workers: Vec<&(usize, Stats)> = flushed.iter().collect();
    workers.sort_by_key(|(w, _)| *w);
    for (w, s) in &workers {
        let demand: Vec<&((usize, BodyOrigin, BodyReason), BodyCount)> = s.bodies.iter().filter(|((_, _, r), _)| *r == BodyReason::Demand).collect();
        let (n, t) = demand.iter().fold((0, 0), |(n, t), (_, c)| (n + c.n, t + c.self_ns));
        let all = s.bodies.iter().fold(0, |n, (_, c)| n + c.n);
        section.detail(format!("worker {}: {} bodies, {} of them on demand ({} own)", w, all, n, ms(t)));
    }
    let sorted: Vec<(usize, Stats)> = workers.iter().map(|(w, s)| (*w, s.counts())).collect();
    report_workers(report.section(if attempt { "the attempt's waits and holds" } else { "waits and holds" }), &sorted);
    report_types(report, &sorted);
}

/// This thread begins its part of the body phase as worker `worker`: its waits and holds are
/// counted from here.
pub fn work_begin(worker: usize) {
    if !on() {
        return;
    }
    let sched = sched_counts();
    WORK.with(|w| *w.borrow_mut() = Some(Work { worker: worker as u32, start: Instant::now(), cpu: thread_cpu_ns(), sched, item: None }));
    STATS.with(|s| s.borrow_mut().thread = crate::shared::thread_number());
}

/// This thread's part of the body phase ends.
pub fn work_end() {
    let Some(w) = WORK.with(|w| w.borrow_mut().take()) else { return };
    let now = Instant::now();
    let cpu = thread_cpu_ns();
    let sched = w.sched.zip(sched_counts()).map(|(a, b)| (b.0.saturating_sub(a.0), b.1.saturating_sub(a.1), b.2.saturating_sub(a.2)));
    STATS.with(|s| {
        let mut s = s.borrow_mut();
        s.elapsed_ns += (now - w.start).as_nanos() as u64;
        s.cpu_ns += cpu.saturating_sub(w.cpu);
        if let Some((_, delay, _)) = sched {
            s.run_delay_ns += delay;
        }
        if tracing() {
            trace(&mut s, Event { what: EV_WORK, detail: 0, arg: w.worker, t0: since(w.start), t1: since(now) });
            if let Some((run, delay, slices)) = sched {
                trace(&mut s, Event { what: EV_SCHED, detail: 0, arg: slices as u32, t0: delay, t1: run });
            }
        }
    });
}

/// A waiter for the loader's lock parks: the clock, while the trace runs.
#[inline]
pub fn lock_park_begin() -> Option<Instant> {
    (on() && tracing()).then(Instant::now)
}

/// The park begun at `parked` ended: the waiter woke.
pub fn lock_park_end(parked: Option<Instant>) {
    let Some(parked) = parked else { return };
    if !in_work() {
        return;
    }
    let now = Instant::now();
    STATS.with(|s| {
        let mut s = s.borrow_mut();
        s.lock_parks += 1;
        trace(&mut s, Event { what: EV_PARK, detail: 0, arg: 0, t0: since(parked), t1: since(now) });
    });
}

/// A waiter whose wait began at `start` took the loader's lock.
pub fn lock_taken_waiting(start: Option<Instant>, taken: crate::shared::Taken) {
    let Some(start) = start else { return };
    if !tracing() || !in_work() {
        return;
    }
    let now = Instant::now();
    STATS.with(|s| trace(&mut s.borrow_mut(), Event { what: EV_TAKEN, detail: 0, arg: taken.parks, t0: since(start), t1: since(now) }));
}

/// The loader's lock's holder releases it: the clock and the threads waiting, while the trace
/// runs.
#[inline]
pub fn lock_release_begin(waiting: &AtomicU32) -> Option<(Instant, u32)> {
    if !on() || !tracing() {
        return None;
    }
    Some((Instant::now(), waiting.load(Ordering::Relaxed)))
}

/// The release begun at `releasing` ended, having woken a waiter or not.
pub fn lock_release_end(releasing: Option<(Instant, u32)>, woke: bool) {
    let Some((at, waiting)) = releasing else { return };
    if !in_work() {
        return;
    }
    let now = Instant::now();
    STATS.with(|s| {
        let mut s = s.borrow_mut();
        s.lock_releases.0 += 1;
        s.lock_releases.1 += (waiting > 0) as u64;
        s.lock_releases.2 += woke as u64;
        trace(&mut s, Event { what: EV_RELEASE, detail: woke as u8, arg: waiting, t0: since(at), t1: since(now) });
    });
}

/// An item of the queue begins on this thread.
pub fn item_begin() {
    if !on() {
        return;
    }
    let waited = STATS.with(|s| s.borrow().waited_ns());
    let cpu = thread_cpu_ns();
    WORK.with(|w| {
        if let Some(w) = w.borrow_mut().as_mut() {
            w.item = Some((Instant::now(), waited, cpu));
        }
    });
}

/// The item `index` of file `file`, of `bytes` bytes, ends on this thread.
pub fn item_end(file: u32, bytes: u32, index: u32) {
    if !on() {
        return;
    }
    let Some((worker, (start, waited, cpu))) = WORK.with(|w| w.borrow_mut().as_mut().and_then(|w| Some((w.worker, w.item.take()?)))) else { return };
    let now = Instant::now();
    let cpu_now = thread_cpu_ns();
    STATS.with(|s| {
        let mut s = s.borrow_mut();
        let waited_ns = s.waited_ns() - waited;
        s.items += 1;
        s.item_rows.push(ItemRow { worker, file, bytes, index, t0: since(start), t1: since(now), waited_ns, cpu_ns: cpu_now.saturating_sub(cpu) });
        if tracing() {
            trace(&mut s, Event { what: EV_ITEM, detail: 0, arg: file, t0: since(start), t1: since(now) });
        }
    });
}

fn in_work() -> bool {
    WORK.with(|w| w.borrow().is_some())
}

/// A wait may begin: its clock, while the counters run.
#[inline]
pub fn wait_begin() -> Option<Instant> {
    on().then(Instant::now)
}

/// The wait begun at `start` ended; `holder` is the thread that held what was waited for, where
/// it is known (a cell's claimant), 0 otherwise.
#[inline]
pub fn wait_end(start: Option<Instant>, what: Wait, holder: usize) {
    if let Some(start) = start {
        waited(start, what, holder);
    }
}

#[cold]
#[inline(never)]
fn waited(start: Instant, what: Wait, holder: usize) {
    if !in_work() {
        return;
    }
    let now = Instant::now();
    let in_hold = HOLDING.with(|h| h.borrow().since.is_some());
    STATS.with(|s| {
        let mut s = s.borrow_mut();
        let w = &mut s.waits[what as usize];
        w.0 += 1;
        w.1 += (now - start).as_nanos() as u64;
        if in_hold {
            s.waits_in_hold.0 += 1;
            s.waits_in_hold.1 += (now - start).as_nanos() as u64;
        }
        if tracing() {
            trace(&mut s, Event { what: EV_WAIT, detail: what as u8, arg: holder as u32, t0: since(start), t1: since(now) });
        }
    });
}

/// Charges the stretch of the hold since its last charge to the innermost category.
fn charge(h: &mut Holding) -> Option<Instant> {
    let (began, last) = h.since?;
    let now = Instant::now();
    let cat = h.cats.last().copied().unwrap_or(Hold::Other);
    h.since = Some((began, now));
    STATS.with(|s| {
        let mut s = s.borrow_mut();
        let c = &mut s.holds[cat as usize];
        c.0 += 1;
        c.1 += (now - last).as_nanos() as u64;
        if tracing() {
            trace(&mut s, Event { what: EV_HOLD, detail: cat as u8, arg: 0, t0: since(last), t1: since(now) });
        }
    });
    Some(now)
}

/// The outermost hold of the loader's lock begins on this thread, the lock just taken.
pub fn hold_begin() {
    if !on() || !in_work() {
        return;
    }
    let now = Instant::now();
    HOLDING.with(|h| {
        let mut h = h.borrow_mut();
        h.since = Some((now, now));
        h.entry = h.cats.last().copied().unwrap_or(Hold::Other);
    });
}

/// The outermost hold ends, the lock just given up.
pub fn hold_end() {
    HOLDING.with(|h| {
        let mut h = h.borrow_mut();
        let Some((began, _)) = h.since else { return };
        let now = charge(&mut h).unwrap_or_else(Instant::now);
        h.since = None;
        let entry = h.entry;
        STATS.with(|s| {
            let mut s = s.borrow_mut();
            s.held.0 += 1;
            s.held.1 += (now - began).as_nanos() as u64;
            let e = &mut s.holds_by_entry[entry as usize];
            e.0 += 1;
            e.1 += (now - began).as_nanos() as u64;
            if tracing() {
                trace(&mut s, Event { what: EV_HELD, detail: 0, arg: 0, t0: since(began), t1: since(now) });
            }
        });
    });
}

/// A category of the loader's work under way on this thread until the guard goes; the lock's
/// time while it is innermost is its own.
pub struct HoldFor(());

impl Drop for HoldFor {
    fn drop(&mut self) {
        HOLDING.with(|h| {
            let mut h = h.borrow_mut();
            charge(&mut h);
            h.cats.pop();
        });
    }
}

/// Begins `cat`, while the counters run.
#[inline]
pub fn hold_for(cat: Hold) -> Option<HoldFor> {
    if !on() {
        return None;
    }
    Some(hold_for_measured(cat))
}

#[cold]
#[inline(never)]
fn hold_for_measured(cat: Hold) -> HoldFor {
    HOLDING.with(|h| {
        let mut h = h.borrow_mut();
        charge(&mut h);
        h.cats.push(cat);
    });
    HoldFor(())
}

/// The workers' slots the fork left empty: the queue had no first work to reserve for them, so they
/// were neither attached nor started.
static EMPTY_SLOTS: AtomicUsize = AtomicUsize::new(0);

pub fn empty_slots(n: usize) {
    EMPTY_SLOTS.store(n, Ordering::Relaxed);
}

/// The workers' rows: per worker its wall and CPU time, its items, its waits by kind and the
/// loader's lock it held, the rest unattributed; then the holds by category over the workers.
fn report_workers(section: &mut crate::report::Section, flushed: &[(usize, Stats)]) {
    let working: Vec<&(usize, Stats)> = flushed.iter().filter(|(_, s)| s.elapsed_ns > 0).collect();
    if working.is_empty() {
        return;
    }
    let mut total = Stats::default();
    for (_, s) in &working {
        total.absorb(s.counts());
    }
    let waited = total.waited_ns();
    section.row("workers' time").count(working.len()).note(format!(
        "{} wall summed, {} CPU; waiting {} ({} of it inside the loader's holds), holding the loader's lock {}, unattributed {}",
        ms(total.elapsed_ns),
        ms(total.cpu_ns),
        ms(waited),
        ms(total.waits_in_hold.1),
        ms(total.held.1),
        ms(total.unattributed_ns())
    ));
    let empty = EMPTY_SLOTS.load(Ordering::Relaxed);
    if empty > 0 {
        section.detail(format!("{} more workers' slots left empty: the queue had no work to reserve for them", empty));
    }
    for (i, name) in WAIT_NAMES.iter().enumerate() {
        let (n, t) = total.waits[i];
        if n > 0 {
            section.detail(format!("waited for {}: {} times, {}", name, n, ms(t)));
        }
    }
    section.detail(format!("held the loader's lock: {} holds, {}", total.held.0, ms(total.held.1)));
    if total.lock_releases.0 > 0 {
        let (all, waited, woke) = total.lock_releases;
        section.detail(format!("released it {} times (the trace's), {} with threads waiting, {} waking one; {} parks", all, waited, woke, total.lock_parks));
    }
    if total.run_delay_ns > 0 {
        section.detail(format!("runnable and waiting for a CPU (the scheduler's run delay): {}", ms(total.run_delay_ns)));
    }
    for (i, name) in HOLD_NAMES.iter().enumerate() {
        let (n, t) = total.holds[i];
        if n > 0 {
            section.detail(format!("  for {}: {} stretches, {}", name, n, ms(t)));
        }
    }
    for (i, name) in LOOKUP_NAMES.iter().enumerate() {
        let (n, found, outermost) = total.lookups[i];
        let unlocked = total.unlocked[i];
        if n > 0 || unlocked > 0 {
            section.detail(format!("{}: {} calls under the lock, {} of them entered or found something, {} took the lock themselves; {} answered without it", name, n, found, outermost, unlocked));
        }
    }
    section.detail("taken for (the category under way when the lock was taken; whole holds):".to_string());
    for (i, name) in HOLD_NAMES.iter().enumerate() {
        let (n, t) = total.holds_by_entry[i];
        if n > 0 {
            section.detail(format!("  {}: {} holds, {}", name, n, ms(t)));
        }
    }
    for (w, s) in working {
        let waited = s.waited_ns();
        let mut kinds: Vec<(u64, usize)> = s.waits.iter().enumerate().filter(|(_, w)| w.0 > 0).map(|(i, w)| (w.1, i)).collect();
        kinds.sort_by(|a, b| b.cmp(a));
        let top: Vec<String> = kinds.iter().take(3).map(|&(t, i)| format!("{} {}", WAIT_NAMES[i], ms(t))).collect();
        section.detail(format!(
            "worker {}: {} items, {} wall, {} CPU, waited {} ({}), held {}, unattributed {}",
            w,
            s.items,
            ms(s.elapsed_ns),
            ms(s.cpu_ns),
            ms(waited),
            top.join(", "),
            ms(s.held.1),
            ms(s.unattributed_ns())
        ));
    }
    if total.trace_dropped > 0 {
        section.detail(format!("the trace dropped {} events past {} a thread", total.trace_dropped, TRACE_CAP));
    }
}

/// Writes what was measured to the files `TEQ_WORKERS_JSON` and `TEQ_WORKERS_TRACE` name, with
/// `suffix` after the name (the attempt that gave way writes `.attempt`).
pub fn write_files(suffix: &str) {
    let flushed = FLUSHED.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(path) = std::env::var_os("TEQ_WORKERS_TRACE") {
        let mut out = format!("# waits: {}\n# holds: {}\nthread worker what detail arg t0 t1\n", WAIT_NAMES.join(";"), HOLD_NAMES.join(";"));
        for (w, s) in flushed.iter() {
            for e in &s.trace {
                out.push_str(&format!("{} {} {} {} {} {} {}\n", s.thread, w, e.what, e.detail, e.arg, e.t0, e.t1));
            }
        }
        let mut p = path.clone();
        p.push(suffix);
        let _ = std::fs::write(p, out);
    }
    let Some(path) = std::env::var_os("TEQ_WORKERS_JSON") else { return };
    let p = phases();
    let mut j = String::from("{");
    j.push_str("\"phases\":[");
    j.push_str(&p.parts.iter().map(|(n, d)| format!("[{},{}]", js(n), d.as_nanos())).collect::<Vec<_>>().join(","));
    j.push_str("],");
    j.push_str(&format!("\"gave_way_ns\":{},", p.gave_way.map_or(-1, |d| d.as_nanos() as i64)));
    if let Some(r) = REACH.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        j.push_str(&format!(
            "\"reach\":{{\"mode\":\"{}\",\"program_files\":{},\"files\":{},\"rings\":{:?},\"names\":[{}],\"fanout\":[{}]}},",
            r.mode,
            r.program_files,
            r.files,
            r.rings,
            r.names.iter().map(|(n, c, a)| format!("[{},{},{}]", js(n), c, a)).collect::<Vec<_>>().join(","),
            r.fanout.iter().map(|(p, r, n)| format!("[{},{},{}]", js(p), r, n)).collect::<Vec<_>>().join(",")
        ));
    }
    j.push_str("\"workers\":[");
    let mut first = true;
    for (w, s) in flushed.iter() {
        if !first {
            j.push(',');
        }
        first = false;
        j.push_str(&format!(
            "{{\"worker\":{},\"elapsed_ns\":{},\"cpu_ns\":{},\"run_delay_ns\":{},\"items\":{},\"held\":[{},{}],\"waits_in_hold\":[{},{}],\"waits\":{{{}}},\"holds\":{{{}}},\"holds_by_entry\":{{{}}},\"bodies\":[{}],\"item_rows\":[{}]}}",
            w,
            s.elapsed_ns,
            s.cpu_ns,
            s.run_delay_ns,
            s.items,
            s.held.0,
            s.held.1,
            s.waits_in_hold.0,
            s.waits_in_hold.1,
            WAIT_NAMES.iter().enumerate().map(|(i, n)| format!("{}:[{},{}]", js(n), s.waits[i].0, s.waits[i].1)).collect::<Vec<_>>().join(","),
            HOLD_NAMES.iter().enumerate().map(|(i, n)| format!("{}:[{},{}]", js(n), s.holds[i].0, s.holds[i].1)).collect::<Vec<_>>().join(","),
            HOLD_NAMES.iter().enumerate().map(|(i, n)| format!("{}:[{},{}]", js(n), s.holds_by_entry[i].0, s.holds_by_entry[i].1)).collect::<Vec<_>>().join(","),
            s.bodies
                .iter()
                .map(|((ph, o, r), c)| format!("[{},\"{}\",\"{}\",{},{},{}]", js(phase_name(&p.parts, *ph)), origin_name(*o), reason_name(*r), c.n, c.self_ns, c.incl_ns))
                .collect::<Vec<_>>()
                .join(","),
            s.item_rows.iter().map(|r| format!("[{},{},{},{},{},{},{},{}]", r.worker, r.file, r.bytes, r.index, r.t0, r.t1, r.waited_ns, r.cpu_ns)).collect::<Vec<_>>().join(",")
        ));
    }
    j.push_str("]}");
    let mut p = path.clone();
    p.push(suffix);
    let _ = std::fs::write(p, j);
}

/// `s` as a JSON string.
fn js(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `TEQ_WORKERS_TYPES=1`: the types the forked workers made, which the merge's records reach
/// (`Worker::types_escaping`), and the insertion locks' holds timed one by one.
#[inline]
pub fn types_wanted() -> bool {
    match TYPES_WANTED.load(Ordering::Relaxed) {
        1 => false,
        2 => true,
        _ => types_wanted_first(),
    }
}

static TYPES_WANTED: AtomicU8 = AtomicU8::new(0);

#[cold]
#[inline(never)]
fn types_wanted_first() -> bool {
    let on = std::env::var_os("TEQ_WORKERS_TYPES").is_some_and(|v| v == "1");
    TYPES_WANTED.store(if on { 2 } else { 1 }, Ordering::Relaxed);
    on
}

fn serial_index(what: Wait) -> Option<usize> {
    match what {
        Wait::TypeStore => Some(0),
        Wait::Interner => Some(1),
        Wait::SharedMap => Some(2),
        _ => None,
    }
}

/// An insertion lock of a store just taken: the hold's clock, when holds are timed.
#[inline]
pub fn serial_hold_begin(what: Wait) -> Option<(Instant, Wait)> {
    (on() && types_wanted()).then(|| (Instant::now(), what))
}

/// The insertion lock taken at `start` is given up.
pub fn serial_held(what: Wait, start: Instant) {
    let Some(i) = serial_index(what) else { return };
    if !in_work() {
        return;
    }
    let ns = start.elapsed().as_nanos() as u64;
    let k = (64 - ns.leading_zeros()).min(31) as usize;
    STATS.with(|s| {
        let mut s = s.borrow_mut();
        let h = &mut s.serial_holds[i];
        h.0 += 1;
        h.1 += ns;
        h.2[k] += 1;
        h.3[k] += ns;
    });
}

/// The types the body phase made and which of them outlive it (`Worker::types_escaping`).
#[derive(Clone, Copy, Default)]
pub struct TypeCounts {
    /// The store's length at the fork, at the workers' join and after the merge.
    pub at_fork: u32,
    pub at_join: u32,
    pub after_merge: u32,
    /// Of the workers' types (from the fork to the join), those a record kept past the merge
    /// reaches (a symbol's signature, a class's parents, base types and constructor, a type
    /// parameter's bounds, an alias, a pattern's type, an expression's recorded type, and what
    /// those types and the variables they name reach), directly or through the type the merge
    /// made of it again.
    pub escaping: u32,
    /// The records' roots walked: expressions with a type, symbols with a signature.
    pub exprs: u32,
    pub sigs: u32,
}

static TYPES: Mutex<Option<TypeCounts>> = Mutex::new(None);

pub fn types_counted(c: TypeCounts) {
    *TYPES.lock().unwrap_or_else(|e| e.into_inner()) = Some(c);
}

/// The "types" section: the workers' types, the escaping share, and the insertion locks' holds.
fn report_types(report: &mut crate::report::Report, flushed: &[(usize, Stats)]) {
    let counts = *TYPES.lock().unwrap_or_else(|e| e.into_inner());
    let mut holds = [(0u64, 0u64, [0u64; 32], [0u64; 32]); 3];
    for (_, s) in flushed {
        for i in 0..3 {
            holds[i].0 += s.serial_holds[i].0;
            holds[i].1 += s.serial_holds[i].1;
            for k in 0..32 {
                holds[i].2[k] += s.serial_holds[i].2[k];
                holds[i].3[k] += s.serial_holds[i].3[k];
            }
        }
    }
    if counts.is_none() && holds.iter().all(|h| h.0 == 0) {
        return;
    }
    let section = report.section("types made by the workers");
    if let Some(c) = counts {
        let made = c.at_join.saturating_sub(c.at_fork);
        section.row("types inserted after the fork").count(made as usize).note(format!("{} at the fork, {} more made again by the merge", c.at_fork, c.after_merge.saturating_sub(c.at_join)));
        section.row("  reached from a record kept past the merge").count(c.escaping as usize).note(format!("{:.1}%; roots: {} expressions' types, {} signatures", 100.0 * c.escaping as f64 / made.max(1) as f64, c.exprs, c.sigs));
        section.row("  reached from nothing kept").count(made.saturating_sub(c.escaping) as usize);
    }
    for (i, name) in ["the type store's", "the interner's", "the shared maps'"].iter().enumerate() {
        let (n, ns, hist, hist_ns) = &holds[i];
        if *n == 0 {
            continue;
        }
        let quantile = |q: f64| -> String {
            let target = (*n as f64 * q).ceil() as u64;
            let mut seen = 0;
            for (k, &c) in hist.iter().enumerate() {
                seen += c;
                if seen >= target {
                    return format!("<{} ns", 1u64 << k);
                }
            }
            "?".to_string()
        };
        section.row(&format!("{} insertion lock held", name)).count(*n as usize).note(format!(
            "{} in all, mean {:.0} ns; median {}, p90 {}, p99 {}, p99.9 {}, the longest {}",
            ms(*ns),
            *ns as f64 / *n as f64,
            quantile(0.5),
            quantile(0.9),
            quantile(0.99),
            quantile(0.999),
            quantile(1.0)
        ));
        // Bucket k holds the holds under 2^k ns: 11 and up are 1 µs and longer, 21 and up 1 ms.
        for (from, what) in [(11, "1 µs"), (21, "1 ms")] {
            let (c, t): (u64, u64) = (from..32).fold((0, 0), |(c, t), k| (c + hist[k], t + hist_ns[k]));
            section.detail(format!("{} holds of {} lock of {} or longer: {} ({:.1}% of the time held)", c, name, what, ms(t), 100.0 * t as f64 / (*ns).max(1) as f64));
        }
    }
}

/// A row of the "type store's overlays" section (`types::TypeStore::overlay_rows`).
pub struct OverlayRow {
    pub label: String,
    pub count: Option<u64>,
    pub time: Option<Duration>,
    pub note: String,
}

static OVERLAYS: Mutex<Vec<OverlayRow>> = Mutex::new(Vec::new());

/// Rows for the overlays' section, after the ones given so far.
pub fn overlays_measured(rows: Vec<OverlayRow>) {
    OVERLAYS.lock().unwrap_or_else(|e| e.into_inner()).extend(rows);
}

fn report_overlays(report: &mut crate::report::Report) {
    let rows = std::mem::take(&mut *OVERLAYS.lock().unwrap_or_else(|e| e.into_inner()));
    if rows.is_empty() {
        return;
    }
    let section = report.section("the type store's overlays");
    for r in rows {
        let row = section.row(&r.label);
        if let Some(n) = r.count {
            row.count(n as usize);
        }
        if let Some(d) = r.time {
            row.time(d);
        }
        row.note(r.note);
    }
}

/// `TEQ_WORKERS_MEMORY=1`: the type store's memory by component at the fork, the join and the
/// merge's end, sampled by the owner at those barriers.
pub fn memory_wanted() -> bool {
    static STATE: AtomicU8 = AtomicU8::new(0);
    match STATE.load(Ordering::Relaxed) {
        1 => false,
        2 => true,
        _ => {
            let on = std::env::var_os("TEQ_WORKERS_MEMORY").is_some_and(|v| v == "1");
            STATE.store(if on { 2 } else { 1 }, Ordering::Relaxed);
            on
        }
    }
}

/// What `typer::prep` replayed (per kind of key: how many, how many found) and captured.
static PREP: Mutex<(Option<Vec<(char, usize, usize)>>, Option<usize>)> = Mutex::new((None, None));

pub fn prep_measured(replayed: Option<Vec<(char, usize, usize)>>, captured: Option<usize>) {
    *PREP.lock().unwrap_or_else(|e| e.into_inner()) = (replayed, captured);
}

/// Marks the body typed next as typed for an inferred result type (`set`), or takes the mark
/// (`!set`), which it returns.
#[inline]
pub fn typing_inferred(set: bool) -> bool {
    if !on() {
        return false;
    }
    TYPING_INFERRED.with(|t| if set { t.set(true); true } else { t.replace(false) })
}

/// A macro's run begins (`1`) or ends (`-1`) on this thread.
#[inline]
pub fn macro_run(step: i32) {
    if on() {
        MACRO_RUNS.with(|m| m.set(m.get() + step));
    }
}

pub fn macro_runs() -> i32 {
    MACRO_RUNS.with(|m| m.get())
}

/// The passes after the type phase whose parts `--time` reports, and a session's answer under
/// `TEQ_SESSION_PARTS=1`: the reach (`emit::reach::compute`), the emit, and what a session does
/// after it (the write, a JVM session's analysis).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pass {
    Reach,
    Emit,
    After,
}

/// A part of a pass: in which round of the reach's typing and walking, how many things it handled
/// and its time; a part without a time is a count of what the pass did inside its timed parts.
#[derive(Clone)]
pub struct Part {
    pub name: &'static str,
    pub round: u32,
    pub n: usize,
    pub time: Option<Duration>,
}

static PARTS_ON: AtomicBool = AtomicBool::new(false);
static PARTS: Mutex<[Vec<Part>; 3]> = Mutex::new([Vec::new(), Vec::new(), Vec::new()]);

/// Whether the passes' parts are timed: `TEQ_PART_TIMES=0` turns them off, the control for their
/// own cost.
pub fn enable_parts(on: bool) {
    let on = on && std::env::var_os("TEQ_PART_TIMES").map_or(true, |v| v != "0");
    PARTS_ON.store(on, Ordering::Relaxed);
}

pub fn parts_on() -> bool {
    PARTS_ON.load(Ordering::Relaxed)
}

fn with_parts<R>(pass: Pass, f: impl FnOnce(&mut Vec<Part>) -> R) -> R {
    let mut parts = PARTS.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut parts[pass as usize])
}

/// The parts of `pass`, in the order first met.
pub fn parts(pass: Pass) -> Vec<Part> {
    with_parts(pass, |p| p.clone())
}

/// The clock of a pass's parts, each part timed from where the last one ended.
pub struct Lap {
    pass: Pass,
    at: Option<Instant>,
    pub round: u32,
}

impl Lap {
    /// A clock that times nothing: a pass run again for a check, whose parts are not the build's.
    #[cfg(debug_assertions)]
    pub fn off(pass: Pass) -> Lap {
        Lap { pass, at: None, round: 0 }
    }

    /// The pass begins, the parts of its last run dropped.
    pub fn begin(pass: Pass) -> Lap {
        if !parts_on() {
            return Lap { pass, at: None, round: 0 };
        }
        with_parts(pass, |p| p.clear());
        Lap { pass, at: Some(Instant::now()), round: 0 }
    }

    /// The pass goes on from here, after parts timed elsewhere.
    pub fn resume(pass: Pass) -> Lap {
        Lap { pass, at: parts_on().then(Instant::now), round: 0 }
    }

    /// The part `name` ends now, having handled `n` things.
    pub fn done(&mut self, name: &'static str, n: usize) {
        let Some(at) = self.at else { return };
        let round = self.round;
        with_parts(self.pass, |parts| match parts.iter_mut().find(|p| p.name == name && p.round == round && p.time.is_some()) {
            Some(p) => {
                p.n += n;
                p.time = Some(p.time.unwrap_or_default() + at.elapsed());
            }
            None => parts.push(Part { name, round, n, time: Some(at.elapsed()) }),
        });
        self.at = Some(Instant::now());
    }

    /// A count of what the timed parts did, `name`, outside the clock.
    pub fn count(&mut self, name: &'static str, n: usize) {
        if self.at.is_none() {
            return;
        }
        with_parts(self.pass, |parts| parts.push(Part { name, round: 0, n, time: None }));
    }
}

/// A part of `pass` timed outside its lap: `name` took `d`, having handled `n` things.
pub fn part(pass: Pass, name: &'static str, n: usize, d: Duration) {
    if parts_on() {
        with_parts(pass, |parts| parts.push(Part { name, round: 0, n, time: Some(d) }));
    }
}

/// The parts of `pass` summed over the rounds: per name in the order first met, the rounds it
/// ran in, what it handled and its time.
pub fn parts_summed(pass: Pass) -> Vec<(&'static str, u32, usize, Option<Duration>)> {
    let mut out: Vec<(&'static str, u32, usize, Option<Duration>)> = Vec::new();
    for p in parts(pass) {
        match out.iter_mut().find(|(name, _, _, t)| *name == p.name && t.is_some() == p.time.is_some()) {
            Some(o) => {
                o.1 += 1;
                o.2 += p.n;
                o.3 = o.3.zip(p.time).map(|(a, b)| a + b);
            }
            None => out.push((p.name, 1, p.n, p.time)),
        }
    }
    out
}

/// The section of `pass`'s parts in `--time`: a row per part summed over the rounds, then the
/// rows `outside` names (the time of the pass's row the parts do not cover), the rounds' detail
/// and the counts.
pub fn report_parts(report: &mut crate::report::Report, pass: Pass, inside: Duration, outside: &[(&str, Duration)]) {
    if !parts_on() {
        return;
    }
    let all = parts(pass);
    let section = report.section(match pass {
        Pass::Reach => "reach",
        Pass::Emit => "emit",
        Pass::After => "after the emit",
    });
    let mut covered = Duration::ZERO;
    for (name, rounds, n, time) in parts_summed(pass) {
        let Some(time) = time else { continue };
        covered += time;
        let row = section.row(name);
        if n > 0 {
            row.count(n);
        }
        row.time(time);
        if rounds > 1 {
            row.note(format!("in {} rounds", rounds));
        }
    }
    section.row("unattributed, inside").time(inside.saturating_sub(covered));
    for (name, d) in outside {
        section.row(name).time(*d);
    }
    let rounds = all.iter().filter_map(|p| p.time.map(|_| p.round)).max().unwrap_or(0);
    if rounds > 1 {
        for r in 1..=rounds {
            let line: Vec<String> = all
                .iter()
                .filter(|p| p.round == r && p.time.is_some_and(|t| t >= Duration::from_micros(50) || p.n > 0))
                .map(|p| if p.n > 0 { format!("{} {} in {}", p.name, p.n, ms(p.time.unwrap_or_default().as_nanos() as u64)) } else { format!("{} {}", p.name, ms(p.time.unwrap_or_default().as_nanos() as u64)) })
                .collect();
            section.detail(format!("round {}: {}", r, line.join(", ")));
        }
    }
    let counts: Vec<String> = all.iter().filter(|p| p.time.is_none()).map(|p| format!("{} {}", p.name, p.n)).collect();
    if !counts.is_empty() {
        section.detail(counts.join(", "));
    }
}

/// `pass`'s parts as the members of a JSON object, a session's answer's: per part summed over the
/// rounds `"<name>": [<n>, <ms>]`, a count `"<name>": <n>`.
pub fn parts_json(pass: Pass) -> String {
    let mut out = String::from("{");
    for (i, (name, _, n, time)) in parts_summed(pass).into_iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        match time {
            Some(t) => out.push_str(&format!("\"{}\":[{},{:.3}]", name, n, t.as_secs_f64() * 1000.0)),
            None => out.push_str(&format!("\"{}\":{}", name, n)),
        }
    }
    out.push('}');
    out
}
