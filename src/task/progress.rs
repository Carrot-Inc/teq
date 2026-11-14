//! How a project verb says where it is and how long it took (docs/CLI.md, "teq fix"): one reporter, on stderr, and
//! one way to write a duration. Where stderr is no terminal the reporter writes whole lines, at a unit's start and at
//! its end; on a terminal a unit's start is a status line that replaces itself, redrawn with the seconds it has stood,
//! held within the terminal's row, and erased before any whole line is written and where a signal ends the process.

use std::io::Write;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A duration as a line shows it: whole milliseconds below ten seconds (`355 ms`), seconds with one decimal from ten
/// seconds up (`75.0 s`).
pub fn duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 10_000 {
        format!("{} ms", ms)
    } else {
        format!("{}.{} s", ms / 1000, ms % 1000 / 100)
    }
}

/// `1 file`, `2 files`: a count with its noun, the plural for every count but one.
pub fn counted(n: usize, one: &str, many: &str) -> String {
    format!("{} {}", n, if n == 1 { one } else { many })
}

/// The reporter of one verb's run.
pub struct Progress {
    inner: Mutex<Inner>,
}

struct Inner {
    out: Box<dyn Write + Send>,
    terminal: bool,
    now: Box<dyn Fn() -> Instant + Send>,
    /// The terminal's width in columns, asked each time a status is drawn (a window changes its size).
    width: Box<dyn Fn() -> usize + Send>,
    /// The status line showing: its text, when it was set, and the whole seconds its last drawing told.
    status: Option<(String, Instant, u64)>,
}

/// What erases the line the cursor is on, the cursor back at its start.
const ERASE: &str = "\r\x1b[K";

/// What stands where a status was cut to its row.
const CUT: &str = "...";

/// A status as its row takes it: the text and, from the first second on, the seconds it has stood (`, 3 s`), within
/// one column less than the terminal's `width`, so that it never wraps: a row it wrapped into would stay when the
/// status is erased. Where it is longer the text gives way, by whole characters and with `CUT` in their place, and
/// the seconds stay. A character outside ASCII is counted as two columns, the widest it can take.
fn fitted(text: &str, seconds: u64, width: usize) -> String {
    let suffix = if seconds == 0 { String::new() } else { format!(", {} s", seconds) };
    let room = width.saturating_sub(1);
    let columns = |c: char| if c.is_ascii() { 1 } else { 2 };
    if text.chars().map(columns).sum::<usize>() + suffix.len() <= room {
        return format!("{}{}", text, suffix);
    }
    let keep = room.saturating_sub(CUT.len() + suffix.len());
    let (mut kept, mut used) = (String::new(), 0);
    for c in text.chars() {
        if used + columns(c) > keep {
            break;
        }
        kept.push(c);
        used += columns(c);
    }
    let all = format!("{}{}{}", kept, CUT, suffix);
    // A row narrower than the mark and the seconds takes what fits of them.
    if kept.is_empty() {
        all.chars().take(room).collect()
    } else {
        all
    }
}

/// The terminal under a status line: its width, and the signals that would leave a status on its row.
#[cfg(unix)]
mod terminal {
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Whether a status line shows on stderr: what a signal's handler erases.
    pub static STATUS_UP: AtomicBool = AtomicBool::new(false);

    /// Set by the handler before it erases: no status is drawn after it, in the moment the process still has.
    pub static ENDING: AtomicBool = AtomicBool::new(false);

    #[allow(clashing_extern_declarations)]
    extern "C" {
        fn signal(sig: i32, handler: usize) -> usize;
        fn raise(sig: i32) -> i32;
        fn write(fd: i32, bytes: *const std::ffi::c_void, count: usize) -> isize;
        fn ioctl(fd: i32, request: std::ffi::c_ulong, ...) -> i32;
    }
    const SIG_DFL: usize = 0;
    const SIGINT: i32 = 2;
    const SIGTERM: i32 = 15;
    #[cfg(any(target_os = "linux", target_os = "android"))]
    const TIOCGWINSZ: std::ffi::c_ulong = 0x5413;
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    const TIOCGWINSZ: std::ffi::c_ulong = 0x4008_7468;

    /// The window's size as the terminal's `TIOCGWINSZ` gives it: rows, columns, and its two sizes in pixels.
    #[repr(C)]
    struct Size(u16, u16, u16, u16);

    /// The columns of the terminal on stderr; eighty where it does not say.
    pub fn width() -> usize {
        let mut size = Size(0, 0, 0, 0);
        // SAFETY: the terminal's own query of fd 2, which fills the four fields it is handed.
        let answered = unsafe { ioctl(2, TIOCGWINSZ, &mut size as *mut Size) } == 0;
        if answered && size.1 > 0 {
            size.1 as usize
        } else {
            80
        }
    }

    extern "C" fn erase_and_end(sig: i32) {
        // SAFETY: calls a handler may make, and nothing that takes a lock or allocates: one write of fixed bytes to
        // the terminal where a status shows, then the signal's own action put back and the signal raised again, so
        // that the process ends by it as it would have without the handler.
        unsafe {
            ENDING.store(true, Ordering::SeqCst);
            if STATUS_UP.load(Ordering::SeqCst) {
                write(2, super::ERASE.as_ptr().cast(), super::ERASE.len());
            }
            signal(sig, SIG_DFL);
            raise(sig);
        }
    }

    /// An interrupt and a termination erase the status line before they end the process.
    pub fn take_signals() {
        // SAFETY: installs a handler that only writes, restores and raises (`erase_and_end`).
        unsafe {
            for sig in [SIGINT, SIGTERM] {
                signal(sig, erase_and_end as extern "C" fn(i32) as usize);
            }
        }
    }
}

/// Elsewhere no status line is drawn on stderr (`Progress::stderr`).
#[cfg(not(unix))]
mod terminal {
    pub static STATUS_UP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    pub static ENDING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    pub fn width() -> usize {
        80
    }

    pub fn take_signals() {}
}

impl Progress {
    /// The reporter a verb writes with: stderr, in a terminal's form where stderr is one that takes the erasing
    /// sequence: not a `TERM=dumb` one, and not a Windows console, which takes it only in a mode a process must ask
    /// for; those get the whole lines.
    /// The status line is held within the terminal's width, and an interrupt or a termination erases it before it
    /// ends the process (`terminal`).
    pub fn stderr() -> Progress {
        let dumb = std::env::var_os("TERM").is_some_and(|t| t == "dumb");
        let terminal = cfg!(unix) && !dumb && std::io::IsTerminal::is_terminal(&std::io::stderr());
        if terminal {
            terminal::take_signals();
        }
        Progress::over(Box::new(std::io::stderr()), terminal, Box::new(Instant::now), Box::new(terminal::width))
    }

    /// The reporter in the form of no terminal, whatever stderr is: for a run whose own switches write to stderr
    /// while it goes, each of whose lines then keeps a row of its own.
    pub fn stderr_lines() -> Progress {
        Progress::over(Box::new(std::io::stderr()), false, Box::new(Instant::now), Box::new(terminal::width))
    }

    /// A reporter over a writer, a clock and a terminal's width handed in.
    pub fn over(out: Box<dyn Write + Send>, terminal: bool, now: Box<dyn Fn() -> Instant + Send>, width: Box<dyn Fn() -> usize + Send>) -> Progress {
        Progress { inner: Mutex::new(Inner { out, terminal, now, width, status: None }) }
    }

    /// Whether a status line is drawn at all: a wait redraws it (`tick`) only then.
    pub fn is_terminal(&self) -> bool {
        self.lock().terminal
    }

    /// A unit begins: a whole line where stderr is no terminal, the status line on one.
    pub fn begin(&self, text: &str) {
        let mut inner = self.lock();
        if inner.terminal {
            inner.show(text);
        } else {
            inner.write(&format!("{}\n", text));
        }
    }

    /// Where the unit is: the status line on a terminal, nothing elsewhere.
    pub fn status(&self, text: &str) {
        let mut inner = self.lock();
        if inner.terminal {
            inner.show(text);
        }
    }

    /// The status line's text replaced, its seconds going on: how far the unit has come.
    pub fn advance(&self, text: &str) {
        let mut inner = self.lock();
        if !inner.terminal {
            return;
        }
        let Some((_, since, told)) = inner.status.take() else { return inner.show(text) };
        inner.status = Some((text.to_string(), since, told));
        inner.draw();
    }

    /// The status line drawn again where the whole seconds it has stood have changed.
    pub fn tick(&self) {
        let mut inner = self.lock();
        let at = (inner.now)();
        let Some((_, since, told)) = inner.status.as_mut() else { return };
        let stood = at.saturating_duration_since(*since).as_secs();
        if stood == *told {
            return;
        }
        *told = stood;
        inner.draw();
    }

    /// A whole line, in the status line's place where one shows.
    pub fn line(&self, text: &str) {
        let mut inner = self.lock();
        let erase = if inner.status.take().is_some() { ERASE } else { "" };
        inner.write(&format!("{}{}\n", erase, text));
        terminal::STATUS_UP.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// The status line erased: before the process writes to stderr by itself.
    pub fn clear(&self) {
        let mut inner = self.lock();
        if inner.status.take().is_some() {
            inner.write(ERASE);
            terminal::STATUS_UP.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Inner {
    fn show(&mut self, text: &str) {
        self.status = Some((text.to_string(), (self.now)(), 0));
        self.draw();
    }

    /// The status line written over what its row holds, within the terminal's width (`fitted`).
    fn draw(&mut self) {
        let Some((text, _, told)) = self.status.as_ref() else { return };
        let drawn = format!("{}{}", ERASE, fitted(text, *told, (self.width)()));
        // Up before its first byte: a signal between the two erases a row that holds nothing yet.
        terminal::STATUS_UP.store(true, std::sync::atomic::Ordering::SeqCst);
        if terminal::ENDING.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        self.write(&drawn);
    }

    fn write(&mut self, bytes: &str) {
        let _ = self.out.write_all(bytes.as_bytes());
        let _ = self.out.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// A writer whose bytes the test reads back.
    #[derive(Clone, Default)]
    struct Kept(Arc<Mutex<Vec<u8>>>);

    impl Write for Kept {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Kept {
        fn taken(&self) -> String {
            String::from_utf8(std::mem::take(&mut *self.0.lock().unwrap())).unwrap()
        }
    }

    /// A reporter over a kept writer and a clock the test moves, in milliseconds.
    fn reporter(terminal: bool) -> (Progress, Kept, Arc<Mutex<u64>>) {
        let (kept, moved) = (Kept::default(), Arc::new(Mutex::new(0u64)));
        let (start, clock) = (Instant::now(), moved.clone());
        let now = move || start + Duration::from_millis(*clock.lock().unwrap());
        (Progress::over(Box::new(kept.clone()), terminal, Box::new(now), Box::new(|| 80)), kept, moved)
    }

    #[test]
    fn a_duration_is_milliseconds_below_ten_seconds_and_seconds_from_there() {
        let shown = |ms: u64| duration(Duration::from_millis(ms));
        assert_eq!(shown(0), "0 ms");
        assert_eq!(shown(355), "355 ms");
        assert_eq!(shown(9_999), "9999 ms");
        assert_eq!(shown(10_000), "10.0 s");
        assert_eq!(shown(75_049), "75.0 s");
        assert_eq!(shown(75_950), "75.9 s");
        assert_eq!(shown(140_000), "140.0 s");
        assert_eq!(duration(Duration::from_micros(999)), "0 ms");
    }

    #[test]
    fn a_status_is_held_within_its_row() {
        let typing = "a/compile: typing (1 of 3)";
        // It fits: as it is, with its seconds from the first on.
        assert_eq!(fitted(typing, 0, 80), typing);
        assert_eq!(fitted(typing, 14, 80), "a/compile: typing (1 of 3), 14 s");
        assert_eq!(fitted(typing, 0, 27), typing);
        // One column short of it: the text gives way, never the last column of the row.
        assert_eq!(fitted(typing, 0, 26), "a/compile: typing (1 o...");
        assert_eq!(fitted(typing, 0, 20), "a/compile: typin...");
        assert_eq!(fitted("teq fix: preparing the rules", 0, 20), "teq fix: prepari...");
        // The seconds stay where the text is cut.
        assert_eq!(fitted(typing, 14, 20), "a/compile:..., 14 s");
        assert_eq!(fitted("a/compile: rules over 12 of 40 files", 3, 40), "a/compile: rules over 12 of 40 ..., 3 s");
        for width in [20, 40, 80] {
            for seconds in [0, 3, 75, 4000] {
                for text in [typing, "teq fix: preparing the rules", "a/compile: typing on one worker, since Asking may ask a symbol (1 of 3)", "a/compile: rules over 1200 of 4000 files"] {
                    assert!(fitted(text, seconds, width).len() < width, "{} at {} s in {} columns", text, seconds, width);
                }
            }
        }
        // A configuration's name longer than the row.
        let long = "a-project-with-a-name-longer-than-any-row-of-the-terminal/compile: typing (1 of 1)";
        assert_eq!(fitted(long, 0, 20), "a-project-with-a...");
        assert_eq!(fitted(long, 2, 40), "a-project-with-a-name-longer-th..., 2 s");
        // A character outside ASCII is whole or not there, and counts as two columns.
        let wide = "päckchen/compile: typing (1 of 1)";
        assert_eq!(fitted(wide, 0, 80), wide);
        assert_eq!(fitted(wide, 0, 12), "päckche...");
        assert_eq!(fitted(wide, 0, 6), "p...");
        assert_eq!(fitted("日本語/compile: typing (1 of 1)", 0, 12), "日本語/c...");
        assert_eq!(fitted("日本語/compile: typing (1 of 1)", 0, 11), "日本語/...");
        assert_eq!(fitted("日本語/compile: typing (1 of 1)", 0, 10), "日本語...");
        // A row narrower than the mark and the seconds.
        assert_eq!(fitted(typing, 14, 6), "..., ");
        assert_eq!(fitted(typing, 0, 3), "..");
        assert_eq!(fitted(typing, 0, 1), "");
        assert_eq!(fitted(typing, 0, 0), "");
    }

    #[test]
    fn a_terminal_status_is_cut_to_the_width_asked_at_each_drawing() {
        let (kept, moved, columns) = (Kept::default(), Arc::new(Mutex::new(0u64)), Arc::new(Mutex::new(20usize)));
        let (start, clock, width) = (Instant::now(), moved.clone(), columns.clone());
        let now = move || start + Duration::from_millis(*clock.lock().unwrap());
        let p = Progress::over(Box::new(kept.clone()), true, Box::new(now), Box::new(move || *width.lock().unwrap()));
        p.begin("a/compile: typing (1 of 3)");
        assert_eq!(kept.taken(), "\r\x1b[Ka/compile: typin...");
        *moved.lock().unwrap() = 2_000;
        p.tick();
        assert_eq!(kept.taken(), "\r\x1b[Ka/compile: ..., 2 s");
        // The window is made wider: the next drawing takes the new width.
        *columns.lock().unwrap() = 80;
        p.advance("a/compile: rules over 0 of 1 file");
        assert_eq!(kept.taken(), "\r\x1b[Ka/compile: rules over 0 of 1 file, 2 s");
        // A whole line is never cut.
        *columns.lock().unwrap() = 20;
        p.line("a/compile: 1 file, 0 lints, 0 to rewrite in 2100 ms");
        assert_eq!(kept.taken(), "\r\x1b[Ka/compile: 1 file, 0 lints, 0 to rewrite in 2100 ms\n");
    }

    #[test]
    fn a_count_takes_the_plural_but_for_one() {
        assert_eq!(counted(0, "file", "files"), "0 files");
        assert_eq!(counted(1, "file", "files"), "1 file");
        assert_eq!(counted(2, "file", "files"), "2 files");
    }

    #[test]
    fn a_terminal_shows_a_status_line_that_replaces_itself() {
        let (p, kept, clock) = reporter(true);
        p.begin("a/compile: typing (1 of 3)");
        assert_eq!(kept.taken(), "\r\x1b[Ka/compile: typing (1 of 3)");
        // The same second: nothing is drawn again.
        *clock.lock().unwrap() = 900;
        p.tick();
        assert_eq!(kept.taken(), "");
        *clock.lock().unwrap() = 1_000;
        p.tick();
        assert_eq!(kept.taken(), "\r\x1b[Ka/compile: typing (1 of 3), 1 s");
        *clock.lock().unwrap() = 1_700;
        p.tick();
        assert_eq!(kept.taken(), "");
        *clock.lock().unwrap() = 3_100;
        p.tick();
        assert_eq!(kept.taken(), "\r\x1b[Ka/compile: typing (1 of 3), 3 s");
        // Another status counts its own seconds.
        p.status("a/compile: rules over 12 of 40 files");
        assert_eq!(kept.taken(), "\r\x1b[Ka/compile: rules over 12 of 40 files");
        *clock.lock().unwrap() = 4_200;
        p.tick();
        assert_eq!(kept.taken(), "\r\x1b[Ka/compile: rules over 12 of 40 files, 1 s");
        // How far the unit has come: the text replaced, its seconds kept and counted on.
        p.advance("a/compile: rules over 13 of 40 files");
        assert_eq!(kept.taken(), "\r\x1b[Ka/compile: rules over 13 of 40 files, 1 s");
        *clock.lock().unwrap() = 5_100;
        p.tick();
        assert_eq!(kept.taken(), "\r\x1b[Ka/compile: rules over 13 of 40 files, 2 s");
        // The unit's end takes the status line's place.
        p.line("a/compile: 40 files, 0 lints, 0 to rewrite in 4200 ms");
        assert_eq!(kept.taken(), "\r\x1b[Ka/compile: 40 files, 0 lints, 0 to rewrite in 4200 ms\n");
        // No status shows: a line is written as it is, and a tick draws nothing.
        p.line("teq fix: 1 configuration, 40 files, 0 lints, 0 files rewritten in 4300 ms");
        assert_eq!(kept.taken(), "teq fix: 1 configuration, 40 files, 0 lints, 0 files rewritten in 4300 ms\n");
        *clock.lock().unwrap() = 9_000;
        p.tick();
        assert_eq!(kept.taken(), "");
    }

    #[test]
    fn a_terminal_status_is_erased_before_the_process_writes_itself() {
        let (p, kept, _) = reporter(true);
        p.status("teq fix: preparing the rules");
        assert_eq!(kept.taken(), "\r\x1b[Kteq fix: preparing the rules");
        p.clear();
        assert_eq!(kept.taken(), "\r\x1b[K");
        p.clear();
        assert_eq!(kept.taken(), "");
    }

    #[test]
    fn off_a_terminal_whole_lines_alone() {
        let (p, kept, clock) = reporter(false);
        p.status("teq fix: preparing the rules");
        p.begin("a/compile: typing (1 of 3)");
        *clock.lock().unwrap() = 5_000;
        p.tick();
        p.status("a/compile: rules over 1 of 2 files");
        p.advance("a/compile: rules over 2 of 2 files");
        p.clear();
        p.line("a/compile: 2 files, 0 lints, 0 to rewrite in 5000 ms");
        assert_eq!(kept.taken(), "a/compile: typing (1 of 3)\na/compile: 2 files, 0 lints, 0 to rewrite in 5000 ms\n");
        assert!(!p.is_terminal());
    }
}
