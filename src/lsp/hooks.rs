//! The language server's test hooks (`tests/lsp/driver.mjs`), so that a test forces the schedules
//! of edits and builds rather than sleeping for them: `TEQ_LSP_TRACE_FILE` names a file both the
//! server and its sessions append a line to for each build's identity, start, cancel and outcome,
//! and `TEQ_LSP_TEST_BARRIER=<prefix>` holds a session's build of named files (an edit's, never
//! the idle build of every file) at its start, at its commit point and before its answer while
//! `<prefix>.start`, `<prefix>.commit` or `<prefix>.answer` exists (a file that names process ids
//! holds those sessions alone). Unset, each costs the test of a value read once.

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The identity of the session's build under way that the barriers hold, zero for none.
static HELD: AtomicU64 = AtomicU64::new(0);

fn trace_file() -> Option<&'static PathBuf> {
    static FILE: OnceLock<Option<PathBuf>> = OnceLock::new();
    FILE.get_or_init(|| std::env::var_os("TEQ_LSP_TRACE_FILE").filter(|v| !v.is_empty()).map(PathBuf::from)).as_ref()
}

/// Appends `<who> <pid> <milliseconds since the epoch> <what>` to the trace, in one write; `what`
/// is an event and the identity of the build (or query) it is about, then what else it says.
pub(crate) fn trace(who: &str, what: impl FnOnce() -> String) {
    let Some(path) = trace_file() else { return };
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis());
    let line = format!("{} {} {} {}\n", who, std::process::id(), ms, what());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// A session's build taken, told to the trace and held at its start: one that names files is
/// the one the barriers hold.
pub(crate) fn began(build: Option<u64>, named: bool) {
    let Some(id) = build else { return };
    HELD.store(if named { id } else { 0 }, Ordering::Relaxed);
    trace("session", || format!("start {}", id));
    hold("start");
}

/// Holds the build the barriers hold at `point` while the barrier's file for it exists and names
/// this process or nothing, told in the trace; at most a minute, so that a test cut short leaves
/// no session behind.
pub(crate) fn hold(point: &str) {
    static PREFIX: OnceLock<Option<String>> = OnceLock::new();
    let build = HELD.load(Ordering::Relaxed);
    let Some(prefix) = PREFIX.get_or_init(|| std::env::var("TEQ_LSP_TEST_BARRIER").ok().filter(|v| !v.is_empty())) else { return };
    if build == 0 {
        return;
    }
    let file = format!("{}.{}", prefix, point);
    let me = std::process::id().to_string();
    let holds = || std::fs::read_to_string(&file).is_ok_and(|names| names.split_whitespace().next().is_none() || names.split_whitespace().any(|n| n == me));
    if !holds() {
        return;
    }
    trace("session", || format!("held {} {}", build, point));
    let until = Instant::now() + Duration::from_secs(60);
    while holds() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(5));
    }
    trace("session", || format!("released {} {}", build, point));
}
