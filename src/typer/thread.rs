//! The typing thread: one per session process (`teq compiler watch`, `teq lsp`), started by the first
//! phase, on which every phase of the typer runs (`Typer::run`, `Typer::retype`, the reach pass
//! that types library bodies on demand) with the calling thread blocked until the phase
//! returns. What the thread keeps between phases is its own and stays: the interpreter's caches
//! (`interp::take_caches`, the module instances of the quoted std, the layouts and indexes over
//! the program) and the allocator's free lists (`alloc.rs` keeps a batch of each per thread;
//! what a phase freed beyond that goes to the allocator's centre when the phase is over), which
//! a thread per phase would rebuild on every build and every retype of a watch session. The
//! parallel typer's workers are threads of this kind. The thread has
//! the compiler thread's stack (`main.rs`): the bodies nest as deeply as before.
//!
//! A one-shot build runs its phases on the compiler thread itself, which has the same stack
//! and keeps the same caches from one phase to the next, and whose free lists hold what the
//! parse freed, where a fresh thread takes chunks of the system: the type phase of `teq compiler check`
//! 5 to 12 ms shorter on `realistic-frontend` (of 550) and 0.05 ms on a hello world (of 0.27).
//! `TEQ_BODY_THREAD=1` gives a one-shot build the thread and `TEQ_BODY_THREAD=0` takes it from
//! a session, for measuring what the thread costs.
//!
//! A phase borrows the worker for its duration. The closure reaches the thread with the
//! lifetimes of its borrows erased (`'static`), which is sound because `run` does not return
//! before the closure has run to its end: nothing it borrows is read, moved or dropped by the
//! caller meanwhile, and the result comes back through a slot the same borrow protects. A
//! phase that itself calls `run` (the interpreter typing a body from inside a run on the
//! thread) runs the inner closure in place.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Mutex, OnceLock};

type Job = Box<dyn FnOnce() + Send + 'static>;

struct TypingThread {
    jobs: Sender<Job>,
    done: Receiver<()>,
}

static THREAD: OnceLock<Mutex<TypingThread>> = OnceLock::new();
static SESSION: AtomicBool = AtomicBool::new(false);
static SEPARATE: OnceLock<bool> = OnceLock::new();

thread_local! {
    static ON_THREAD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn start() -> Mutex<TypingThread> {
    let (jobs, inbox) = channel::<Job>();
    let (finished, done) = channel::<()>();
    let thread = std::thread::Builder::new().name("teq-typer".to_string()).stack_size(1 << 30);
    thread
        .spawn(move || {
            crate::alloc::enter();
            ON_THREAD.with(|on| on.set(true));
            for job in inbox {
                job();
                crate::alloc::settle();
                if finished.send(()).is_err() {
                    break;
                }
            }
        })
        .expect("cannot start the typing thread");
    Mutex::new(TypingThread { jobs, done })
}

/// Tells the typer that the process is a session, whose phases run on the typing thread;
/// called before the first phase.
pub fn session() {
    SESSION.store(true, Ordering::Relaxed);
}

/// Whether the phases run on a thread of their own, and not on the calling one: in a session,
/// unless `TEQ_BODY_THREAD` says otherwise.
pub fn separate() -> bool {
    *SEPARATE.get_or_init(|| match std::env::var_os("TEQ_BODY_THREAD") {
        Some(v) if v == "0" => false,
        Some(v) if v == "1" => true,
        _ => SESSION.load(Ordering::Relaxed),
    })
}

/// Runs `f` on the typing thread and returns what it returned, the caller blocked meanwhile;
/// in place where the phases run on the calling thread (`separate`).
pub fn run<'a, R: Send + 'a>(f: impl FnOnce() -> R + Send + 'a) -> R {
    if !separate() || ON_THREAD.with(|on| on.get()) {
        return f();
    }
    let mut result: Option<R> = None;
    {
        let slot = &mut result;
        let job: Box<dyn FnOnce() + Send + '_> = Box::new(move || *slot = Some(f()));
        // The borrows of `job` end before `run` returns: see the module's header.
        let job: Job = unsafe { std::mem::transmute(job) };
        let thread = THREAD.get_or_init(start).lock().unwrap_or_else(|e| e.into_inner());
        thread.jobs.send(job).expect("the typing thread is gone");
        if thread.done.recv().is_err() {
            // The phase panicked on the thread, which reported it.
            std::process::exit(101);
        }
    }
    result.expect("the typing thread returned without a result")
}
