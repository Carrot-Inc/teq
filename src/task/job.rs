//! A child process as a job of its own, for the verbs whose child is the developer's program
//! (`dev`'s dev command, `run`'s JVM): on Unix it leads a process group, holds the terminal while
//! this process does, so that Ctrl-C and the keys reach it and not this process, and the signals
//! that end this process are passed on to the group, which then has `STOP_BOUND` to end before
//! it is killed.

use std::process::{Child, Command, ExitStatus};

#[cfg(unix)]
pub use unix::{received, start, stop, take_signals, wait_for};
#[cfg(not(unix))]
pub use console::{received, start, stop, take_signals, wait_for};

/// The exit code a shell reports for the child: 128 and the signal that ended it, else its code.
pub fn code_of(status: ExitStatus) -> i32 {
    #[cfg(unix)]
    if let Some(signal) = std::os::unix::process::ExitStatusExt::signal(&status) {
        return 128 + signal;
    }
    status.code().unwrap_or(1)
}

#[cfg(unix)]
mod unix {
    use super::{Child, Command};
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::time::{Duration, Instant};

    const POLL: Duration = Duration::from_millis(100);
    /// How long a child has to end on SIGTERM before its group is killed.
    const STOP_BOUND: Duration = Duration::from_secs(5);

    #[allow(clashing_extern_declarations)]
    extern "C" {
        fn _exit(code: i32) -> !;
        fn kill(pid: i32, sig: i32) -> i32;
        fn signal(sig: i32, handler: usize) -> usize;
        fn tcsetpgrp(fd: i32, pgrp: i32) -> i32;
        fn tcgetpgrp(fd: i32) -> i32;
        fn getpgrp() -> i32;
        fn isatty(fd: i32) -> i32;
    }
    const SIG_DFL: usize = 0;
    const SIG_IGN: usize = 1;
    const SIGHUP: i32 = 1;
    const SIGINT: i32 = 2;
    const SIGKILL: i32 = 9;
    const SIGTERM: i32 = 15;
    const SIGTTOU: i32 = 22;
    #[cfg(target_os = "macos")]
    const SIGCONT: i32 = 19;
    #[cfg(not(target_os = "macos"))]
    const SIGCONT: i32 = 18;

    /// The group of the child running, for the handler to pass a signal on to.
    static GROUP: AtomicI32 = AtomicI32::new(0);
    /// The last signal passed on, which the loop's exit code reports.
    static RECEIVED: AtomicI32 = AtomicI32::new(0);

    extern "C" fn pass_on(sig: i32) {
        RECEIVED.store(sig, Ordering::Relaxed);
        let group = GROUP.load(Ordering::Relaxed);
        // SAFETY: a signal to the process group this process started, or the end of this one
        // when none runs.
        unsafe {
            if group > 0 {
                kill(-group, sig);
            } else {
                _exit(128 + sig);
            }
        }
    }

    /// Ctrl-C, a hangup and a termination end the child, and with it this process (at
    /// once while none runs); a write to the terminal while the child holds it is not
    /// stopped.
    pub fn take_signals() {
        // SAFETY: installs a handler that only signals a process group, and ignores SIGTTOU.
        unsafe {
            for sig in [SIGHUP, SIGINT, SIGTERM] {
                signal(sig, pass_on as extern "C" fn(i32) as usize);
            }
            signal(SIGTTOU, SIG_IGN);
        }
    }

    pub fn received() -> Option<i32> {
        Some(RECEIVED.load(Ordering::Relaxed)).filter(|&sig| sig != 0)
    }

    /// Whether this process is in the foreground of the terminal on its stdin.
    fn holds_terminal() -> bool {
        // SAFETY: queries of the terminal on fd 0.
        unsafe { isatty(0) == 1 && tcgetpgrp(0) == getpgrp() }
    }

    pub fn start(mut command: Command) -> std::io::Result<Child> {
        use std::os::unix::process::CommandExt;
        let terminal = holds_terminal();
        command.process_group(0);
        // SAFETY: async-signal-safe calls between fork and exec: the child takes the terminal
        // for its group, as this process does below (whichever runs first), and SIGTTOU back.
        unsafe {
            command.pre_exec(move || {
                if terminal {
                    tcsetpgrp(0, std::process::id() as i32);
                }
                signal(SIGTTOU, SIG_DFL);
                Ok(())
            });
        }
        let child = command.spawn()?;
        let group = child.id() as i32;
        GROUP.store(group, Ordering::Relaxed);
        if terminal {
            // SAFETY: hands the terminal to the group just started, and wakes it in case it
            // read the terminal before it had it.
            unsafe {
                tcsetpgrp(0, group);
                kill(-group, SIGCONT);
            }
        }
        Ok(child)
    }

    /// The child has ended: the terminal back to this process's group.
    fn stopped(child: &Child) {
        let group = child.id() as i32;
        let _ = GROUP.compare_exchange(group, 0, Ordering::Relaxed, Ordering::Relaxed);
        // SAFETY: takes the terminal back for this group, if it went to the child's.
        unsafe {
            if isatty(0) == 1 && tcgetpgrp(0) == group {
                tcsetpgrp(0, getpgrp());
            }
        }
    }

    /// Ends the child's group, or what is left of it once the child ended: SIGTERM,
    /// then SIGKILL to what is left after `STOP_BOUND`.
    pub fn stop(child: &mut Child) {
        // SAFETY: a signal to the process group this process started.
        unsafe {
            kill(-(child.id() as i32), SIGTERM);
        }
        wait_for(child);
    }

    /// Waits `STOP_BOUND` for the group to end, as a signal passed on to it asks, then kills what
    /// is left of it.
    pub fn wait_for(child: &mut Child) {
        let group = child.id() as i32;
        let until = Instant::now() + STOP_BOUND;
        // The group is empty once kill(-group, 0) finds nobody; the leader is reaped first.
        while Instant::now() < until {
            let _ = child.try_wait();
            // SAFETY: a probe of the group.
            if unsafe { kill(-group, 0) } != 0 {
                break;
            }
            std::thread::sleep(POLL);
        }
        // SAFETY: as above.
        unsafe {
            kill(-group, SIGKILL);
        }
        let _ = child.wait();
        stopped(child);
    }
}

/// Without process groups: the child shares the console, which gives it Ctrl-C, and a stop ends
/// its tree with `taskkill`.
#[cfg(not(unix))]
mod console {
    use super::{Child, Command};
    use std::process::Stdio;

    pub fn take_signals() {}

    pub fn received() -> Option<i32> {
        None
    }

    pub fn start(mut command: Command) -> std::io::Result<Child> {
        command.spawn()
    }

    pub fn stop(child: &mut Child) {
        // Its report goes nowhere: once the child ended it is `ERROR: The process "<pid>" not
        // found.`, on the user's console after every run.
        let _ = Command::new("taskkill").args(["/T", "/F", "/PID", &child.id().to_string()]).stdout(Stdio::null()).stderr(Stdio::null()).status();
        let _ = child.kill();
        let _ = child.wait();
    }

    pub fn wait_for(child: &mut Child) {
        stop(child)
    }
}
