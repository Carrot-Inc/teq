//! The schedule shaker of the parallel typer: with
//! `TEQ_SHAKE=<seed>` a worker stops for a moment at the named points where the order of two
//! workers' steps decides whether a rule of the sharing holds (a cell claimed and not yet
//! computed, a lock hold's records published and its cells not yet `Done`, a store grown and
//! not yet switched), so that a scenario of `tests/workers.sh` meets the interleaving a quiet
//! machine would reach once in thousands of runs. `TEQ_SHAKE=<seed>:<points>` shakes only the
//! points named, by their names below, separated by commas. Off, a point is one load and a
//! branch, and only the forked paths have points.

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

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
