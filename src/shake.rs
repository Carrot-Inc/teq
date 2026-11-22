//! The schedule shaker of the parallel typer: with
//! `TEQ_SHAKE=<seed>` a worker stops for a moment at the named points where the order of two
//! workers' steps decides whether a rule of the sharing holds (a cell claimed and not yet
//! computed, a lock hold's records published and its cells not yet `Done`, a store grown and
//! not yet switched), so that a scenario of `tests/workers.sh` meets the interleaving a quiet
//! machine would reach once in thousands of runs. `TEQ_SHAKE=<seed>:<points>` shakes only the
//! points named, by their names below, separated by commas. Off, a point is one load and a
//! branch, and only the forked paths have points.
//!
//! Beside it the forced refusal: with `TEQ_REFUSE_READ=<k>` the k-th read of a program's
//! signature made with the loader's lock held, in a build of several workers, is refused as a
//! wait there is (`Worker::wait_cell`: the build is given to one worker and the read answers as
//! a recursion's does), and every later such read of the same symbol with it. A wait under the
//! lock needs a schedule in which another worker holds the cell at that read; the refusal puts
//! the lock's holder on the same path at a read a scenario names, so that the path after each
//! read is walked without the schedule. How many such reads a build makes is its schedule's
//! too: with the trace on (`TEQ_SERIAL_TRACE`), a build given to one worker says how many it
//! counted. Off, one load and a branch where a signature is completed or its reads are hooked.
//!
//! And the forced refusal of a start: with `TEQ_REFUSE_START=<phase>:<k>` the k-th start of the
//! named phase's threads (`body`, a worker of the body phase; `inlining`, a worker of the inlining
//! phase; `merge`, a merge thread) is refused as the system refuses one, where it has no address
//! space for the thread's stack: the thread is not started, what it was to run is dropped, and the
//! site takes the path its refusal takes (the build is given to one worker). A refusal of the
//! system's needs a memory limit that falls in the window where that start is the first thing to
//! fail; the forced one puts the build on the same path at the start a test names, whatever the
//! machine. A value that names no one start (`body:1`, not `body` nor `body:1,merge:1`) stops the run
//! with a line that says so. Off, one load and a branch where a thread is started.

use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};

/// The places a worker may be held up at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Point {
    /// A completion cell claimed, its work not begun.
    Claimed = 0,
    /// The loader's lock taken, nothing of the hold done.
    LockTaken,
    /// A hold's staged records published, the maps that name them not yet applied.
    Staged,
    /// A hold's records and maps published, its cells not yet `Done`.
    Flushed,
    /// A shared store's entries copied or its chunk made, not yet in place.
    Grown,
    /// A worker about to take its next item.
    Item,
    /// A body, signature or class computed outside the lock, not yet published.
    Computed,
    /// A wait on a cell about to begin.
    Wait,
}

const NAMES: [&str; 8] = ["claimed", "lock", "staged", "flushed", "grown", "item", "computed", "wait"];

/// 0 not read yet, 1 off, 2 on.
static STATE: AtomicU8 = AtomicU8::new(0);
/// The points shaken, one bit each.
static POINTS: AtomicU64 = AtomicU64::new(0);
static SEED: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static RNG: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Holds this thread up for a moment at `p` when the shaker is on.
#[inline]
pub fn point(p: Point) {
    if STATE.load(Ordering::Relaxed) == 1 {
        return;
    }
    shake(p);
}

#[cold]
#[inline(never)]
fn shake(p: Point) {
    if STATE.load(Ordering::Acquire) == 0 {
        read_setting();
    }
    if STATE.load(Ordering::Relaxed) != 2 || POINTS.load(Ordering::Relaxed) & (1 << p as u64) == 0 {
        return;
    }
    let r = next();
    match r % 8 {
        0..=3 => {}
        4..=5 => std::thread::yield_now(),
        _ => std::thread::sleep(std::time::Duration::from_micros(20 + (r >> 8) % 400)),
    }
}

fn read_setting() {
    let setting = std::env::var("TEQ_SHAKE").unwrap_or_default();
    if setting.is_empty() {
        STATE.store(1, Ordering::Release);
        return;
    }
    let (seed, names) = setting.split_once(':').unwrap_or((setting.as_str(), ""));
    let mut points = 0u64;
    for name in names.split(',').filter(|n| !n.is_empty()) {
        match NAMES.iter().position(|&n| n == name) {
            Some(i) => points |= 1 << i,
            None => eprintln!("teq: TEQ_SHAKE names no point {}", name),
        }
    }
    if points == 0 {
        points = u64::MAX;
    }
    SEED.store(seed.parse().unwrap_or(1), Ordering::Relaxed);
    POINTS.store(points, Ordering::Relaxed);
    STATE.store(2, Ordering::Release);
}

/// This thread's next number: a xorshift seeded by the setting's seed and the thread.
fn next() -> u64 {
    RNG.with(|s| {
        let mut x = s.get();
        if x == 0 {
            x = SEED.load(Ordering::Relaxed).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (crate::shared::thread_number() as u64).wrapping_mul(0xD1B5_4A32_D192_ED03) | 1;
        }
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        x
    })
}

/// The forced refusal: 0 not read yet, 1 off, 2 on.
static REFUSAL: AtomicU8 = AtomicU8::new(0);
/// The number of the read refused first.
static REFUSED_READ: AtomicU64 = AtomicU64::new(0);
/// The reads counted so far.
static READS: AtomicU64 = AtomicU64::new(0);
/// The symbol whose read was refused, every later read of which is refused too.
static REFUSED_SYM: AtomicU32 = AtomicU32::new(u32::MAX);

/// What the forced refusal says of a read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Refused {
    No,
    /// The read the setting names, by its number.
    First(u64),
    /// A later read of the symbol refused first.
    Again,
}

/// Whether the forced refusal is on.
#[inline]
pub fn refusing() -> bool {
    match REFUSAL.load(Ordering::Relaxed) {
        1 => false,
        2 => true,
        _ => read_refusal(),
    }
}

#[cold]
#[inline(never)]
fn read_refusal() -> bool {
    let k = std::env::var("TEQ_REFUSE_READ").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    REFUSED_READ.store(k, Ordering::Relaxed);
    REFUSAL.store(if k == 0 { 1 } else { 2 }, Ordering::Release);
    k != 0
}

/// Counts a read of the signature of the symbol `sym` that the refusal may refuse (the caller's
/// to say: `Worker::read_refused`), and whether it is refused: the one the setting names, or a
/// later one of the same symbol.
pub fn refuse(sym: u32) -> Refused {
    let n = READS.fetch_add(1, Ordering::SeqCst) + 1;
    if n == REFUSED_READ.load(Ordering::Relaxed) {
        REFUSED_SYM.store(sym, Ordering::SeqCst);
        return Refused::First(n);
    }
    if REFUSED_SYM.load(Ordering::SeqCst) == sym {
        Refused::Again
    } else {
        Refused::No
    }
}

/// The reads `refuse` has counted in this process: those a build of several workers made before
/// it was given to one worker (`serial_again` traces their number), which a schedule decides, so
/// that a read named past them says how many there were.
pub fn reads() -> u64 {
    READS.load(Ordering::SeqCst)
}

/// The starts the forced refusal may refuse (`TEQ_REFUSE_START`): a thread of a phase of the typing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Start {
    /// A worker of the body phase (`Typer::run`).
    Body = 0,
    /// A worker of the inlining phase (`Worker::expand_in_parallel`).
    Inlining = 1,
    /// A merge thread (`crew::Crew::with`).
    Merge = 2,
}

/// 0 not read yet, 1 off, 2 on.
static START_REFUSAL: AtomicU8 = AtomicU8::new(0);
static REFUSED_PHASE: AtomicU8 = AtomicU8::new(0);
static REFUSED_START: AtomicU64 = AtomicU64::new(0);
static STARTS: [AtomicU64; 3] = [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];

#[cold]
#[inline(never)]
fn start_refusal() -> bool {
    // Unset or empty, the switch is off. Any other value is one start or the run stops here: a test that set it
    // wrongly would otherwise pass by refusing nothing.
    let named = std::env::var_os("TEQ_REFUSE_START").filter(|v| !v.is_empty()).map(|v| {
        let text = v.to_string_lossy().into_owned();
        named_start(&text).unwrap_or_else(|| {
            eprintln!("teq: TEQ_REFUSE_START={text:?} is not one <phase>:<k> (the phase body, inlining or merge, k a number from 1: merge:1)");
            std::process::exit(2);
        })
    });
    if let Some((phase, k)) = named {
        REFUSED_PHASE.store(phase as u8, Ordering::Relaxed);
        REFUSED_START.store(k, Ordering::Relaxed);
    }
    START_REFUSAL.store(if named.is_some() { 2 } else { 1 }, Ordering::Release);
    named.is_some()
}

/// The start a value of `TEQ_REFUSE_START` names: exactly one `<phase>:<k>`, the phase `body`, `inlining` or `merge`
/// and k digits alone, from 1.
fn named_start(v: &str) -> Option<(Start, u64)> {
    let (phase, k) = v.split_once(':')?;
    let phase = match phase {
        "body" => Start::Body,
        "inlining" => Start::Inlining,
        "merge" => Start::Merge,
        _ => return None,
    };
    if k.is_empty() || !k.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    k.parse::<u64>().ok().filter(|&k| k > 0).map(|k| (phase, k))
}

/// Counts a start of `start`'s phase, and whether the forced refusal refuses it: the one the setting names.
pub fn refuse_start(start: Start) -> bool {
    let on = match START_REFUSAL.load(Ordering::Acquire) {
        1 => false,
        2 => true,
        _ => start_refusal(),
    };
    if !on {
        return false;
    }
    let n = STARTS[start as usize].fetch_add(1, Ordering::SeqCst) + 1;
    REFUSED_PHASE.load(Ordering::Relaxed) == start as u8 && n == REFUSED_START.load(Ordering::Relaxed)
}

/// `Builder::spawn_scoped` that the forced refusal may refuse (`refuse_start`): the thread is not started, `f` is
/// dropped, and the error is the site's to take, as the system's own refusal is.
pub trait SpawnRefusable {
    fn spawn_scoped_refusable<'scope, 'env, F, T>(
        self,
        s: &'scope std::thread::Scope<'scope, 'env>,
        start: Start,
        f: F,
    ) -> std::io::Result<std::thread::ScopedJoinHandle<'scope, T>>
    where
        F: FnOnce() -> T + Send + 'scope,
        T: Send + 'scope;
}

impl SpawnRefusable for std::thread::Builder {
    fn spawn_scoped_refusable<'scope, 'env, F, T>(
        self,
        s: &'scope std::thread::Scope<'scope, 'env>,
        start: Start,
        f: F,
    ) -> std::io::Result<std::thread::ScopedJoinHandle<'scope, T>>
    where
        F: FnOnce() -> T + Send + 'scope,
        T: Send + 'scope,
    {
        if refuse_start(start) {
            drop(f);
            return Err(std::io::Error::other(format!("refused by TEQ_REFUSE_START at start {} of the phase", REFUSED_START.load(Ordering::Relaxed))));
        }
        self.spawn_scoped(s, f)
    }
}

#[cfg(test)]
mod tests {
    use super::{named_start, Start};

    #[test]
    fn a_refusal_of_a_start_names_one_start_exactly() {
        assert_eq!(named_start("body:1"), Some((Start::Body, 1)));
        assert_eq!(named_start("inlining:2"), Some((Start::Inlining, 2)));
        assert_eq!(named_start("merge:99"), Some((Start::Merge, 99)));
        for bad in ["unknown:1", "body:0", "body:no", "body", "body:", ":1", "body:1,merge:1", "body:+1", "body: 1", "Body:1", "body:99999999999999999999999"] {
            assert_eq!(named_start(bad), None, "{bad}");
        }
    }
}
