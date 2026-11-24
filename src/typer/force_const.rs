//! Forced schedules of a constant's reference, for tests: `TEQ_FORCE_CONST=<mode>:<name>` makes one interleaving of
//! a reader of `<name>` and the worker that defines it happen every time, where a build of several workers would
//! otherwise meet it by chance.
//! - `unpublished`: the worker publishing `<name>`'s typed initialiser withholds it until another thread has
//!   decided `<name>`'s constant (a selection's `constant_of`, or a top-level reference), so that the reader's
//!   decision is made while the initialiser is unpublished.
//! - `unclaimed`: the first thread to reach `<name>`'s signature cell waits there until another thread reaches it,
//!   which meets the cell unclaimed and claims it.
//! - `claimed`: the thread that claims `<name>`'s signature cell waits, before completing it, until another thread
//!   waits on the cell, which meets it claimed and unpublished.
//! - `refuse`: the thread that claims `<name>`'s signature cell holds it as in `claimed`, and the first wait on the
//!   cell is refused as a cyclic one is, so that the build is given to one worker.
//! Each wait is bounded (`TEQ_FORCE_CONST_MS`, 5000 by default). Each hook appends one line to
//! `TEQ_FORCE_CONST_LOG` when it fires: the mode, the name, this thread, the other thread or `none`, and `met`,
//! `not met` or `refused`. The variables are read once; unset, each hook is one load of a static.
use std::sync::{Mutex, OnceLock};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Mode {
    Unpublished,
    Unclaimed,
    Claimed,
    Refuse,
}

struct Config {
    mode: Mode,
    name: String,
    bound: Duration,
    log: Option<String>,
}

fn config() -> Option<&'static Config> {
    static CONFIG: OnceLock<Option<Config>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            let v = std::env::var("TEQ_FORCE_CONST").ok()?;
            let (mode, name) = v.split_once(':')?;
            let mode = match mode {
                "unpublished" => Mode::Unpublished,
                "unclaimed" => Mode::Unclaimed,
                "claimed" => Mode::Claimed,
                "refuse" => Mode::Refuse,
                _ => return None,
            };
            let ms = std::env::var("TEQ_FORCE_CONST_MS").ok().and_then(|m| m.parse().ok()).unwrap_or(5000);
            Some(Config { mode, name: name.to_string(), bound: Duration::from_millis(ms), log: std::env::var("TEQ_FORCE_CONST_LOG").ok() })
        })
        .as_ref()
}

/// Whether a forced schedule is set: the one test every hook makes on its hot path.
#[inline]
pub(crate) fn on() -> bool {
    config().is_some()
}

/// Whether the schedule set is `mode` of `name`.
pub(crate) fn is(mode: Mode, name: &str) -> bool {
    config().is_some_and(|c| c.mode == mode && c.name == name)
}

/// What the other side has done so far: the threads that asked for the constant, reached the signature cell, or
/// waited on it, and whether the refusal was given.
#[derive(Default)]
struct Seen {
    asked: Vec<ThreadId>,
    reached: Vec<ThreadId>,
    waited: Vec<ThreadId>,
    refused: bool,
}

fn seen() -> &'static Mutex<Seen> {
    static SEEN: OnceLock<Mutex<Seen>> = OnceLock::new();
    SEEN.get_or_init(|| Mutex::new(Seen::default()))
}

#[derive(Clone, Copy)]
pub(crate) enum Side {
    Asked,
    Reached,
    Waited,
}

/// Notes that this thread did `side` for `name`.
#[cold]
#[inline(never)]
pub(crate) fn note(side: Side, name: &str) {
    let Some(c) = config() else { return };
    if c.name != name {
        return;
    }
    let me = std::thread::current().id();
    let mut s = seen().lock().unwrap_or_else(|e| e.into_inner());
    let list = match side {
        Side::Asked => &mut s.asked,
        Side::Reached => &mut s.reached,
        Side::Waited => &mut s.waited,
    };
    if !list.contains(&me) {
        list.push(me);
    }
}

/// Holds this thread, bounded, until another thread has done `side` for `name`; writes the hook's line.
#[cold]
#[inline(never)]
pub(crate) fn hold_until(mode: Mode, side: Side, name: &str) {
    let Some(c) = config() else { return };
    let me = std::thread::current().id();
    // Never under the loader's lock, whose holder another worker may wait for.
    if crate::shared::lock_depth() > 0 {
        write(mode, name, me, None, "not met (under the loader's lock)");
        return;
    }
    let start = Instant::now();
    let other = loop {
        let found = {
            let s = seen().lock().unwrap_or_else(|e| e.into_inner());
            let list = match side {
                Side::Asked => &s.asked,
                Side::Reached => &s.reached,
                Side::Waited => &s.waited,
            };
            list.iter().copied().find(|&t| t != me)
        };
        if found.is_some() || start.elapsed() >= c.bound {
            break found;
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    let status = if other.is_some() { "met" } else { "not met" };
    write(mode, name, me, other, status);
}

/// Whether no thread but this one has done `side` for `name` yet.
fn first(side: Side, name: &str) -> bool {
    let me = std::thread::current().id();
    let s = seen().lock().unwrap_or_else(|e| e.into_inner());
    let list = match side {
        Side::Asked => &s.asked,
        Side::Reached => &s.reached,
        Side::Waited => &s.waited,
    };
    let _ = name;
    list.iter().all(|&t| t == me)
}

/// Whether this wait on `name`'s signature cell is the one the `refuse` mode refuses: the first, once.
#[cold]
#[inline(never)]
pub(crate) fn refuse_now(name: &str) -> bool {
    if !is(Mode::Refuse, name) {
        return false;
    }
    let mut s = seen().lock().unwrap_or_else(|e| e.into_inner());
    if s.refused {
        return false;
    }
    s.refused = true;
    drop(s);
    let me = std::thread::current().id();
    write(Mode::Refuse, name, me, None, "refused");
    true
}

fn write(mode: Mode, name: &str, me: ThreadId, other: Option<ThreadId>, status: &str) {
    let Some(path) = config().and_then(|c| c.log.as_deref()) else { return };
    let mode = match mode {
        Mode::Unpublished => "unpublished",
        Mode::Unclaimed => "unclaimed",
        Mode::Claimed => "claimed",
        Mode::Refuse => "refuse",
    };
    let other_s = other.map_or_else(|| "none".to_string(), |t| format!("{t:?}"));
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{mode}\t{name}\t{me:?}\t{other_s}\t{status}");
    }
}

impl<'a> super::Worker<'a> {
    fn force_const_name(&self, s: crate::types::SymId) -> String {
        self.name_str(self.syms.sym(s).name)
    }

    /// A reader has decided the constant of `s` (a selection's `constant_of`, or a top-level reference).
    #[cold]
    #[inline(never)]
    pub(super) fn force_const_asked(&self, s: crate::types::SymId) {
        note(Side::Asked, &self.force_const_name(s));
    }

    /// The initialiser of `s` is about to be published.
    #[cold]
    #[inline(never)]
    pub(super) fn force_const_unpublished(&self, s: crate::types::SymId) {
        let n = self.force_const_name(s);
        if is(Mode::Unpublished, &n) {
            hold_until(Mode::Unpublished, Side::Asked, &n);
        }
    }

    /// A thread reaches the signature cell of `s`, not yet complete.
    #[cold]
    #[inline(never)]
    pub(super) fn force_const_reached(&self, s: crate::types::SymId) {
        let n = self.force_const_name(s);
        note(Side::Reached, &n);
        if is(Mode::Unclaimed, &n) && first(Side::Reached, &n) {
            hold_until(Mode::Unclaimed, Side::Reached, &n);
        }
    }

    /// This thread has claimed the signature cell of `s` and is about to complete it.
    #[cold]
    #[inline(never)]
    pub(super) fn force_const_claimed(&self, s: crate::types::SymId) {
        let n = self.force_const_name(s);
        // The refuse mode holds the claim as the claimed mode does, so that a reader waits on the cell; that wait is the
        // one refused (`force_const_wait`).
        if is(Mode::Claimed, &n) {
            hold_until(Mode::Claimed, Side::Waited, &n);
        } else if is(Mode::Refuse, &n) {
            hold_until(Mode::Refuse, Side::Waited, &n);
        }
    }

    /// A wait on the cell `key`: noted for a signature's; whether the `refuse` mode refuses it.
    #[cold]
    #[inline(never)]
    pub(super) fn force_const_wait(&self, key: crate::shared::CellKey) -> bool {
        if key.kind() != super::check::CELL_SIG {
            return false;
        }
        let n = self.force_const_name(crate::types::SymId(key.id()));
        note(Side::Waited, &n);
        refuse_now(&n)
    }
}
