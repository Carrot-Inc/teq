//! The logical threads of a program under `teq interp`: `java.lang.Thread` as the JDK runs it, on the
//! interpreter's one OS thread. This module is the layer between the evaluator and the threads, and the
//! only one that knows there are several: the scheduler, the timers, the monitors, park and unpark and the
//! blocking waits sit behind the functions below, which the natives call; the evaluator calls `quantum`
//! once a slice of its steps is spent (`Interp::steps_out`), and `teq interp` calls `run_program`.
//!
//! Each logical thread is a stackful coroutine (corosensei, behind the `threads` feature) with a native
//! stack of its own, and an interpreter of its own (`Slot::interp`) whose `exec` is the thread's execution
//! state: its step slice, its depth and stack bounds, its modules and concatenations under way. The state
//! the threads share, every other field of `Interp` (the program's caches and objects, the typer, the
//! output, this scheduler), is in one interpreter at a time: the running thread's, or the holder's
//! (`Sched::holder`) while the root's loop chooses the next. A switch hands it over (`Interp::trade`):
//! the outgoing thread gives it to the holder before it suspends, the incoming one takes it back after it
//! resumes. So the frames on a suspended thread's stack hold `&mut` borrows of that thread's own
//! interpreter alone, which nothing else reads or writes until the thread resumes, and the shared state
//! is never borrowed across a switch: a frame calls the evaluator only with its whole `self`, which the
//! borrow checker allows only once every borrow of a field has ended, and a switch happens only inside
//! such a call (a native that blocks, or `quantum` at the end of a slice). A `RefCell` borrow held over an
//! evaluation was already one held over arbitrary program code, which could reach the same cell; the
//! natives that block hold none. The typer is a field of the shared state too, moved with it, so one
//! `&mut Worker` is in use at a time. The interpreters of the suspended threads keep a reference to the
//! worker each (made from the running one's when the thread was made), which none of them reads.
//!
//! The layer's state is complete for the coroutine runtime: what a logical thread has of its own is its
//! `Exec`, and the scheduler, the timers and the monitors are `Sched`'s. The interpreter's thread-locals
//! are the OS thread's, which every logical thread shares: what the interpreter keeps of the program and
//! its values (`builtins.rs`'s table and clock origin, `code.rs`'s descriptor generations, `files.rs`'s
//! sources, `identity.rs`'s census, `registry.rs`'s registry, `value.rs`'s string index, `mod.rs`'s
//! caches), `mod.rs`'s epochs, which watch a macro's run, where no thread starts (`start`), and the
//! program-wide tables of handles, `process.rs`'s children, streams, stdin and shutdown hooks, `net.rs`'s
//! servers and `archive.rs`'s zips, digests and inflaters. Those tables still assume one OS thread: they
//! move into the layer's shared state with the second part, which rebuilds them as resumable handles.
//!
//! Scheduling: a thread runs until it blocks (a sleep, a join, a park, a monitor or a wait) or its slice
//! of 2^20 steps is spent while another thread can run or a timer is due, then the next runnable one runs,
//! in order. A deadline (a timed wait, a sleep) is a timer of the thread with the generation it blocked
//! in, so that a timer of an earlier wait never wakes a later one; when no thread can run, the root waits
//! for the earliest deadline, looking for a signal every 50 ms; with none and no deadline the program
//! waits for a signal, as the JVM's deadlock hangs. Nothing evaluates Scala outside a thread.
//!
//! The end: when main has returned and no thread but daemons runs, at `System.exit` from any thread, or at
//! a signal, the shutdown begins, once; its hooks run one after the other on a thread of their own that
//! has main's identity (`Thread.currentThread` is main's object: a departure from the JDK, which starts
//! each hook on its own thread at once, docs/internal/DIFFERENCES.md), while the program's threads go on;
//! then every thread still suspended is resumed with a stop that unwinds it as `System.exit` does, with
//! no `finally` run (a suspended coroutine is never dropped: with `panic = "abort"` that aborts), and the
//! process exits with the shutdown's status. A native that blocks the OS thread itself (a child's wait, a
//! read of a pipe) still ends its thread at a signal, which has to run the hooks.
//!
//! Without the `threads` feature main runs on the root alone, as before the layer: the blocking waits are
//! the OS thread's, and a program that starts a thread is refused (`start`).

// Without the `threads` feature the parts of the scheduler that run coroutines stand unused.
#![cfg_attr(not(feature = "threads"), allow(dead_code))]

use super::builtins::{reg, Table};
use super::value::*;
use super::*;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::time::{Duration, Instant};

#[cfg(feature = "threads")]
use corosensei::{Coroutine, CoroutineResult, Yielder};
#[cfg(all(feature = "threads", not(any(target_os = "macos", target_os = "linux"))))]
use corosensei::stack::DefaultStack;

type A<'a> = &'a [Value];

/// A logical thread's number, its slot in `Sched::slots`: main is 0.
pub(super) type Tid = u32;
const MAIN: Tid = 0;
const NOBODY: Tid = u32::MAX;

/// Main's native stack and the part of it the interpreter's recursion may take: the gigabyte and the
/// budget of the compiler's thread it ran on before (`STACK_BUDGET`). Another thread's are smaller.
#[cfg(feature = "threads")]
const MAIN_STACK: usize = 1 << 30;
#[cfg(feature = "threads")]
const THREAD_STACK: usize = 256 << 20;
#[cfg(feature = "threads")]
const THREAD_BUDGET: usize = 160 << 20;

/// How long the root waits, when no thread can run, before it looks for a signal again.
const IDLE: Duration = Duration::from_millis(50);

/// What a logical thread has of its own: the state of its execution, the rest of an interpreter being
/// shared by every thread.
pub(super) struct Exec {
    pub(super) steps: u64,
    /// The budget past `steps`, granted a slice at a time when the run watches for signals.
    pub(super) reserve: u64,
    /// The address of the stack where the thread started, against which `enter` measures the native
    /// stack its recursion takes, and how much it may take.
    pub(super) stack_base: usize,
    pub(super) stack_budget: usize,
    /// Whether the `StackOverflowError` the budget threw is being made.
    pub(super) throwing_overflow: bool,
    /// The modules being made whose superclass constructors have not returned: a read of one
    /// is `null`, as `MODULE$` is on the JVM until the super call returns (`BCodeSkelBuilder`),
    /// rather than a second instance made in a recursion.
    pub(super) unpublished: Vec<(ClassId, Rc<Object>)>,
    pub(super) depth: u32,
    pub(super) max_depth: u32,
    /// The binders of the quotes being run, innermost last, each with the fresh local of that
    /// run: what a quote of a reference to one of them (`'y`) resolves to.
    pub(super) quote_renames: Vec<FxMap<SymId, SymId>>,
    /// The chains of `+` the lowered code under way builds (`Op::ConcatBegin`): each one's text and
    /// the operands kept to render at their places.
    pub(super) concats: Vec<(String, Vec<(usize, Value)>)>,
    /// The thread this is.
    tid: Tid,
    /// Where the shared state is while this thread does not run, and the thread's handle to suspend
    /// itself: null outside a coroutine.
    holder: *mut Interp<'static, 'static>,
    #[cfg(feature = "threads")]
    yielder: *const Yielder<Resume, Suspend>,
}

impl Exec {
    pub(super) fn new(limits: &Limits) -> Exec {
        Exec {
            steps: limits.steps,
            reserve: 0,
            stack_base: stack_address(),
            stack_budget: STACK_BUDGET,
            throwing_overflow: false,
            unpublished: Vec::new(),
            depth: 0,
            max_depth: limits.depth,
            quote_renames: Vec::new(),
            concats: Vec::new(),
            tid: MAIN,
            holder: std::ptr::null_mut(),
            #[cfg(feature = "threads")]
            yielder: std::ptr::null(),
        }
    }
}

/// What the root hands a thread it resumes: run on, or stop with the shutdown's status.
#[derive(Clone, Copy)]
pub(super) enum Resume {
    Run,
    Stop(i32),
}

/// What a thread tells the root it suspends with: it can run again, or it waits for a wake.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Suspend {
    Yield,
    Block,
}

#[cfg(feature = "threads")]
type Co = Coroutine<Resume, Suspend, (), ThreadStack>;

/// A logical thread's stack: its address space reserved, not committed, each page the system's as the
/// thread first touches it (`alloc::stack_pages`, `MAP_NORESERVE`), its lowest 64 KiB the guard (a whole
/// number of pages on every system). Elsewhere corosensei's own.
#[cfg(all(feature = "threads", any(target_os = "macos", target_os = "linux")))]
struct ThreadStack {
    at: *mut u8,
    len: usize,
}

#[cfg(all(feature = "threads", any(target_os = "macos", target_os = "linux")))]
impl ThreadStack {
    const GUARD: usize = 64 << 10;

    fn new(size: usize) -> std::io::Result<ThreadStack> {
        let len = size + Self::GUARD;
        let at = crate::alloc::stack_pages(len, Self::GUARD);
        if at.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        Ok(ThreadStack { at, len })
    }
}

#[cfg(all(feature = "threads", any(target_os = "macos", target_os = "linux")))]
impl Drop for ThreadStack {
    fn drop(&mut self) {
        // SAFETY: the mapping `stack_pages` made, which no coroutine runs on once it is dropped.
        unsafe { crate::alloc::free_zeroed_pages(self.at, self.len) }
    }
}

// SAFETY: the mapping is the stack's alone, `len` bytes from `at`, page-aligned at both ends, so its
// base and limit are aligned to `STACK_ALIGNMENT`; its limit includes the guard.
#[cfg(all(feature = "threads", any(target_os = "macos", target_os = "linux")))]
unsafe impl corosensei::stack::Stack for ThreadStack {
    fn base(&self) -> corosensei::stack::StackPointer {
        corosensei::stack::StackPointer::new(self.at as usize + self.len).expect("a stack's base")
    }

    fn limit(&self) -> corosensei::stack::StackPointer {
        corosensei::stack::StackPointer::new(self.at as usize).expect("a stack's limit")
    }
}

#[cfg(all(feature = "threads", not(any(target_os = "macos", target_os = "linux"))))]
type ThreadStack = DefaultStack;

/// What a blocked thread waits in, which `Thread.getState` reports.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Wait {
    Sleep,
    Park,
    Join,
    /// The entry of a monitor another thread holds: `BLOCKED`.
    Enter,
    /// `Object.wait`.
    Notify,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Runnable,
    Blocked(Wait),
    Done,
}

/// Why a blocked thread was made runnable.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Wake {
    None,
    Timeout,
    /// An unpark, a notify, a monitor handed over, a joined thread's end.
    Signalled,
    Interrupted,
}

/// What a thread runs.
enum Entry {
    Main(Vec<String>),
    /// A `java.lang.Thread`'s `run`.
    Run(Value),
    /// The shutdown hooks, one after the other.
    Hooks,
    /// The dispatch of a signal the program handles, as the JVM's "Signal Dispatcher" thread's
    /// (`process::dispatch_signal`).
    Dispatch(i32),
}

struct Slot {
    #[cfg(feature = "threads")]
    co: Option<Co>,
    /// The thread's own interpreter, null for main without coroutines (it runs on the root's).
    interp: *mut Interp<'static, 'static>,
    /// Its `java.lang.Thread`: `Null` for main, whose object is the std's, and for the hooks' runner.
    obj: Value,
    state: State,
    /// Whether the thread holds the program alive: main and every thread not a daemon.
    counted: bool,
    timed: bool,
    interrupted: bool,
    /// `LockSupport`'s permit.
    permit: bool,
    wake: Wake,
    /// The count of the thread's waits, which a timer names to wake only the wait it was set for.
    gen: u64,
    /// The threads blocked in a join of this one.
    joiners: Vec<Tid>,
    /// Its `ThreadLocal` values: the local and the value, by the local's identity.
    locals: FxMap<Identity, (Value, Value)>,
}

impl Slot {
    fn new(obj: Value, counted: bool) -> Slot {
        Slot {
            #[cfg(feature = "threads")]
            co: None,
            interp: std::ptr::null_mut(),
            obj,
            state: State::Runnable,
            counted,
            timed: false,
            interrupted: false,
            permit: false,
            wake: Wake::None,
            gen: 0,
            joiners: Vec::new(),
            locals: FxMap::default(),
        }
    }
}

/// A lock while a thread holds it or waits for it: an object's monitor (`synchronized`, `Object.wait`)
/// or a `ReentrantLock`'s (keyed by an object of its own): who holds it and how many times, the threads
/// waiting to enter it (each with the count it enters with), and those waiting in one of its
/// conditions (0 is the monitor's own, `Object.wait`'s; a `Condition` has its number).
struct Monitor {
    /// The object, held so that its identity is not another's while the lock lives.
    #[allow(dead_code)]
    obj: Value,
    owner: Tid,
    count: u32,
    entrants: VecDeque<(Tid, u32)>,
    /// Each waiter, its condition and the count it holds the lock with again.
    waiters: Vec<(Tid, u32, u32)>,
}

/// An object's identity, which `eq` compares: its address, or a string's or class value's text.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) enum Identity {
    At(usize),
    Text(Rc<str>),
}

fn identity(v: &Value) -> Option<Identity> {
    Some(match v {
        Value::Obj(o) => Identity::At(Rc::as_ptr(o) as *const u8 as usize),
        Value::Fun(f) => Identity::At(Rc::as_ptr(f) as *const u8 as usize),
        Value::Array(a) => Identity::At(Rc::as_ptr(a) as *const u8 as usize),
        Value::Map(m) => Identity::At(Rc::as_ptr(m) as *const u8 as usize),
        Value::Trie(t) => Identity::At(Rc::as_ptr(t) as *const u8 as usize),
        Value::Match(m) => Identity::At(Rc::as_ptr(m) as *const u8 as usize),
        Value::Str(s) => Identity::Text(s.clone()),
        Value::Class(c) => Identity::Text(c.qname.clone()),
        _ => return None,
    })
}

/// An initialisation one thread runs and others may read meanwhile: a module's, as the JVM initialises
/// a class; an object's lazy val's (by the object's identity and the field), as dotty's
/// `LazyVals.mkThreadSafeDef` evaluates it; a local lazy val's (by its thunk's identity), as
/// `LazyVals.transformLocalDef` runs it synchronized on its holder.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum InitKey {
    Module(ClassId),
    Lazy(usize, FieldKey),
    Local(usize),
}

/// An initialisation running: its thread and how deep that thread is in it (a lazy val's initialiser
/// reading it again), the object, held so that its identity is not another's meanwhile, and the
/// threads waiting for its end.
struct Init {
    owner: Tid,
    depth: u32,
    #[allow(dead_code)]
    obj: Value,
    waiters: Vec<Tid>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Hooks {
    NotStarted,
    Running(Tid),
    Done,
}

/// The logical threads of a program's run, a part of the state they share.
pub(super) struct Sched {
    slots: Vec<Slot>,
    current: Tid,
    runnable: VecDeque<Tid>,
    /// The deadlines of the timed waits: when, an order among equal ones, the thread and its wait's
    /// generation.
    timers: BinaryHeap<Reverse<(Instant, u64, Tid, u64)>>,
    seq: u64,
    monitors: FxMap<Identity, Monitor>,
    /// The initialisations running (`begin_init`).
    inits: FxMap<InitKey, Init>,
    /// Whether the threads are coroutines; without the feature main runs alone on the root.
    threaded: bool,
    holder: *mut Interp<'static, 'static>,
    /// The shutdown's status once it began, and how far its hooks are.
    shutdown: Option<i32>,
    hooks: Hooks,
    /// Main's status had the program ended at its return: 1 after an uncaught exception.
    main_status: i32,
    /// A hook's `System.exit`, which is the process's status.
    hook_exit: Option<i32>,
    /// Why this run has no logical threads, main on the OS stack: the system refused main's stack.
    unavailable: Option<String>,
}

impl Sched {
    fn new(threaded: bool) -> Sched {
        Sched {
            slots: vec![Slot::new(Value::Null, true)],
            current: MAIN,
            runnable: VecDeque::new(),
            timers: BinaryHeap::new(),
            seq: 0,
            monitors: FxMap::default(),
            inits: FxMap::default(),
            threaded,
            holder: std::ptr::null_mut(),
            shutdown: None,
            hooks: Hooks::NotStarted,
            main_status: 0,
            hook_exit: None,
            unavailable: None,
        }
    }

    /// Makes the blocked thread `tid` runnable, with the reason.
    fn wake(&mut self, tid: Tid, why: Wake) {
        let slot = &mut self.slots[tid as usize];
        if let State::Blocked(_) = slot.state {
            slot.state = State::Runnable;
            slot.wake = why;
            slot.gen += 1;
            self.runnable.push_back(tid);
        }
    }

    /// Wakes the threads whose deadline is past.
    fn expire(&mut self, now: Instant) {
        while let Some(&Reverse((at, _, tid, gen))) = self.timers.peek() {
            if at > now {
                break;
            }
            self.timers.pop();
            if self.slots[tid as usize].gen == gen {
                self.wake(tid, Wake::Timeout);
            }
        }
    }

    /// Drops the timers at the heap's head that their thread's later waits made stale, so that the
    /// head is a live deadline (a stale one would hide those behind it from `due`).
    fn prune(&mut self) {
        while let Some(&Reverse((_, _, tid, gen))) = self.timers.peek() {
            if self.slots[tid as usize].gen == gen {
                break;
            }
            self.timers.pop();
        }
    }

    /// Whether the head's deadline has passed, `prune` having made it a live one.
    fn due(&self, now: Instant) -> bool {
        self.timers.peek().is_some_and(|Reverse((at, ..))| *at <= now)
    }

    /// Whether the thread running should give way: another can run, a timer is due, or the shutdown
    /// waits for its hooks to start.
    fn contended(&mut self) -> bool {
        if !self.runnable.is_empty() || (self.shutdown.is_some() && self.hooks == Hooks::NotStarted) || process::program_signal_pending() {
            return true;
        }
        self.prune();
        !self.timers.is_empty() && self.due(Instant::now())
    }

    fn alive(&self) -> bool {
        self.slots.iter().any(|s| s.counted && s.state != State::Done)
    }
}

impl<'a, 't> Interp<'a, 't> {
    /// Hands the state the threads share between this interpreter and `other`, each keeping its own
    /// thread's `exec`.
    fn trade(&mut self, other: &mut Interp<'a, 't>) {
        std::mem::swap(self, other);
        std::mem::swap(&mut self.exec, &mut other.exec);
    }

    /// A new interpreter for a thread: its own `exec`, every shared field empty until it takes the
    /// shared state (`trade`), its worker reference made from this one's and never read (module doc).
    #[cfg(feature = "threads")]
    fn shell(&mut self) -> Interp<'a, 't> {
        // SAFETY: the reference is made from the running thread's own and is read only by
        // `Interp::new` here, while this thread runs; after that it is moved with the shell's
        // fields and never read (the module's doc).
        let typer: &'a mut Worker<'t> = unsafe { &mut *(&mut *self.typer as *mut Worker<'t>) };
        Interp::new(typer, Limits::unlimited())
    }

    fn sched(&mut self) -> &mut Sched {
        self.sched.as_deref_mut().expect("a program's run")
    }

    /// Suspends the running thread, the shared state handed to the holder, until the root resumes it.
    #[cfg(feature = "threads")]
    fn switch(&mut self, why: Suspend) -> R<()> {
        let holder = self.exec.holder as *mut Interp<'a, 't>;
        let yielder = self.exec.yielder;
        // SAFETY: the holder is the root's heap interpreter, which nothing else borrows while a thread
        // runs; the yielder is this coroutine's own, alive while it runs.
        unsafe { self.trade(&mut *holder) };
        let resumed = unsafe { &*yielder }.suspend(why);
        unsafe { self.trade(&mut *holder) };
        match resumed {
            Resume::Run => Ok(()),
            Resume::Stop(status) => Err(Control::Fail(Failure::Exit(status))),
        }
    }

    /// `teq interp`'s run of the program: main, the threads it starts, the shutdown hooks; the
    /// process's status.
    pub fn run_program(&mut self, args: &[String]) -> i32 {
        #[cfg(feature = "threads")]
        return self.run_threads(args);
        #[cfg(not(feature = "threads"))]
        return self.run_alone(args);
    }

    /// The run on the root alone: main, then the hooks.
    #[cfg_attr(feature = "threads", allow(dead_code))]
    fn run_alone(&mut self, args: &[String]) -> i32 {
        self.run_unthreaded(args, None)
    }

    /// `run_alone`, with why the run has no logical threads where the system refused them.
    fn run_unthreaded(&mut self, args: &[String], unavailable: Option<String>) -> i32 {
        let mut sched = Sched::new(false);
        sched.unavailable = unavailable;
        self.sched = Some(Box::new(sched));
        let result = self.run_main(args);
        self.flush();
        let mut code = match result {
            Ok(()) => 0,
            // A program's own status passes through, as the JVM's process exits with it, and so does
            // a signal's (128 + its number).
            Err(Failure::Exit(status)) => status,
            Err(f) => {
                let message = self.describe(&f);
                eprintln!("{}", message);
                match f {
                    Failure::Thrown(_) | Failure::Withheld => 1,
                    _ => 3,
                }
            }
        };
        // The shutdown hooks, after an uncaught exception's report as the JVM runs them after its
        // main thread ended.
        if let Some(status) = self.shut_down() {
            code = status;
        }
        code
    }

    #[cfg(feature = "threads")]
    fn run_threads(&mut self, args: &[String]) -> i32 {
        let mut sched = Sched::new(true);
        let holder: *mut Interp<'a, 't> = Box::into_raw(Box::new(self.shell()));
        sched.holder = holder as *mut Interp<'static, 'static>;
        self.sched = Some(Box::new(sched));
        // SAFETY: `holder` is the box just made; the root reaches it only between resumes, the
        // running thread only while the root waits in `resume` (`switch`, `body`).
        unsafe {
            self.trade(&mut *holder);
            let h = &mut *holder;
            if let Err(f) = make_coroutine(h, MAIN, Entry::Main(args.to_vec()), MAIN_STACK, STACK_BUDGET) {
                self.trade(&mut *holder);
                drop(Box::from_raw(holder));
                // The system refuses the address space of main's stack (a limit of it, `ulimit -v`): main
                // runs on the OS stack as without the layer, and a thread is refused, naming why.
                let why = match f {
                    Failure::Unsupported(m) => m,
                    other => self.describe(&other),
                };
                return self.run_unthreaded(args, Some(why));
            }
        }
        let status = loop {
            // SAFETY: as above; this borrow ends before the resume below.
            let h = unsafe { &mut *holder };
            if let Some(status) = process::take_signal() {
                begin_shutdown(h, status);
            }
            // A signal the program handles: a dispatcher for it, a daemon, as the JVM's.
            for n in process::take_program_signals() {
                let s = h.sched();
                let tid = s.slots.len() as Tid;
                s.slots.push(Slot::new(Value::Null, false));
                if make_coroutine(h, tid, Entry::Dispatch(n), THREAD_STACK, THREAD_BUDGET).is_err() {
                    h.sched().slots[tid as usize].state = State::Done;
                }
            }
            let s = h.sched();
            s.expire(Instant::now());
            if s.shutdown.is_none() && s.slots[MAIN as usize].state == State::Done && !s.alive() {
                s.shutdown = Some(s.main_status);
            }
            if let Some(status) = s.shutdown {
                match s.hooks {
                    Hooks::NotStarted => {
                        let tid = s.slots.len() as Tid;
                        s.slots.push(Slot::new(Value::Null, false));
                        s.hooks = Hooks::Running(tid);
                        if make_coroutine(h, tid, Entry::Hooks, THREAD_STACK, THREAD_BUDGET).is_err() {
                            h.sched().hooks = Hooks::Done;
                        }
                        continue;
                    }
                    Hooks::Running(tid) if s.slots[tid as usize].state == State::Done => s.hooks = Hooks::Done,
                    Hooks::Done => break s.hook_exit.unwrap_or(status),
                    Hooks::Running(_) => {}
                }
            }
            let Some(tid) = s.runnable.pop_front() else {
                h.flush();
                let s = h.sched();
                let now = Instant::now();
                let wait = s.timers.peek().map_or(IDLE, |Reverse((at, ..))| at.saturating_duration_since(now).min(IDLE));
                std::thread::sleep(wait);
                continue;
            };
            s.current = tid;
            let slot = &mut s.slots[tid as usize];
            let Some(mut co) = slot.co.take() else { continue };
            let r = co.resume(Resume::Run);
            let h = unsafe { &mut *holder };
            let s = h.sched();
            s.current = NOBODY;
            match r {
                CoroutineResult::Yield(why) => {
                    s.slots[tid as usize].co = Some(co);
                    if why == Suspend::Yield {
                        s.runnable.push_back(tid);
                    }
                }
                CoroutineResult::Return(()) => {
                    drop(co);
                    let own = std::mem::replace(&mut s.slots[tid as usize].interp, std::ptr::null_mut());
                    // SAFETY: the thread's interpreter, made by `make_coroutine`, which its finished
                    // coroutine no longer borrows.
                    drop(unsafe { Box::from_raw(own as *mut Interp<'a, 't>) });
                }
            }
        };
        // The halt: every thread still suspended is resumed with the stop, until it returns.
        let stop = Resume::Stop(status);
        let n = unsafe { &mut *holder }.sched().slots.len();
        for tid in 0..n {
            let co = unsafe { &mut *holder }.sched().slots[tid].co.take();
            let Some(mut co) = co else { continue };
            unsafe { &mut *holder }.sched().current = tid as Tid;
            let mut turns = 0;
            while let CoroutineResult::Yield(_) = co.resume(stop) {
                turns += 1;
                if turns > 1000 {
                    // A thread that keeps suspending after its stop; leaving it suspended would abort
                    // at its drop, so the process ends here.
                    unsafe { &mut *holder }.flush();
                    process::exit_now(status);
                }
            }
            let own = std::mem::replace(&mut unsafe { &mut *holder }.sched().slots[tid].interp, std::ptr::null_mut());
            if !own.is_null() {
                drop(unsafe { Box::from_raw(own as *mut Interp<'a, 't>) });
            }
        }
        unsafe {
            self.trade(&mut *holder);
            drop(Box::from_raw(holder));
        }
        self.flush();
        status
    }

    /// Main's run: the program's entry point, its uncaught exception reported as the JVM's main thread
    /// reports it.
    fn main_body(&mut self, args: &[String]) {
        let result = self.run_main(args);
        self.flush();
        match result {
            Ok(()) => {}
            Err(Failure::Exit(status)) => begin_shutdown(self, status),
            Err(f) => {
                let message = self.describe(&f);
                eprintln!("{}", message);
                match f {
                    Failure::Thrown(_) => self.sched().main_status = 1,
                    Failure::Withheld => begin_shutdown(self, 1),
                    _ => halt(self, 3),
                }
            }
        }
    }

    /// A thread's run: its `run`, an exception it does not catch handed to its handler
    /// (`Thread.dispatchUncaughtException`), whose own exception is ignored as the JDK ignores it.
    fn thread_body(&mut self, obj: Value) {
        let mut r = self.call_by_name(obj.clone(), "run", Vec::new()).map(|_| ());
        if let Err(Control::Throw(e)) = r {
            r = match self.call_by_name(obj, "dispatchUncaughtException", vec![e]) {
                Err(Control::Throw(_)) => Ok(()),
                other => other.map(|_| ()),
            };
        }
        self.flush();
        match r {
            Err(Control::Fail(Failure::Exit(status))) => begin_shutdown(self, status),
            Err(Control::Fail(f)) => {
                let message = self.describe(&f);
                eprintln!("{}", message);
                if let Failure::Withheld = f {
                    begin_shutdown(self, 1)
                } else {
                    halt(self, 3)
                }
            }
            _ => {}
        }
    }

    /// A signal's dispatch, on a daemon thread of its own: its handler's thread started.
    fn dispatch_body(&mut self, n: i32) {
        let r = process::dispatch_signal(self, n);
        self.flush();
        match r {
            Err(Control::Fail(Failure::Exit(status))) => begin_shutdown(self, status),
            Err(Control::Fail(f)) => {
                let message = self.describe(&f);
                eprintln!("{}", message);
                halt(self, 3)
            }
            _ => {}
        }
    }

    /// The shutdown hooks' run, on the thread made for them.
    fn hooks_body(&mut self) {
        if let Some(status) = self.shut_down() {
            self.sched().hook_exit = Some(status);
        }
    }

    /// The end of the running thread: joiners woken. The locks it holds stay its own: no monitor, which
    /// `synchronized` releases on every way out of its block (`Template::Monitor`), and a `ReentrantLock`
    /// keeps its owner until an unlock, as the JDK's `Sync` does when its owner ends.
    fn thread_ended(&mut self) {
        let s = self.sched();
        let tid = s.current;
        let slot = &mut s.slots[tid as usize];
        slot.state = State::Done;
        slot.locals = FxMap::default();
        let joiners = std::mem::take(&mut slot.joiners);
        for j in joiners {
            s.wake(j, Wake::Signalled);
        }
    }
}

/// The body of a thread's coroutine: it takes the shared state, runs its entry, ends, and gives the
/// state back; resumed first with a stop (the halt before it ever ran), it returns at once.
#[cfg(feature = "threads")]
fn body(me: *mut Interp<'static, 'static>, yielder: &Yielder<Resume, Suspend>, first: Resume, entry: Entry) {
    // SAFETY: the thread's own interpreter, which only this coroutine borrows (the module's doc).
    let me = unsafe { &mut *me };
    me.exec.yielder = yielder;
    if let Resume::Stop(_) = first {
        return;
    }
    let holder = me.exec.holder;
    unsafe { me.trade(&mut *holder) };
    me.exec.stack_base = stack_address();
    match entry {
        Entry::Main(args) => me.main_body(&args),
        Entry::Run(obj) => me.thread_body(obj),
        Entry::Hooks => me.hooks_body(),
        Entry::Dispatch(n) => me.dispatch_body(n),
    }
    me.thread_ended();
    unsafe { me.trade(&mut *holder) };
}

/// Makes the coroutine of thread `tid`, runnable, with a stack of `size` and the budget its recursion
/// may take of it.
#[cfg(feature = "threads")]
fn make_coroutine(it: &mut Interp, tid: Tid, entry: Entry, size: usize, budget: usize) -> Result<(), Failure> {
    let stack = ThreadStack::new(size).map_err(|e| Failure::Unsupported(format!("a thread's stack of {} MiB cannot be made: {}", size >> 20, e)))?;
    let mut own = it.shell();
    // Its steps in slices, a signal seen between them, as the run's (`Interp::watch_signals`).
    own.watch_signals();
    own.exec.tid = tid;
    own.exec.stack_budget = budget;
    let holder = it.sched().holder;
    own.exec.holder = holder;
    let own = Box::into_raw(Box::new(own)) as *mut Interp<'static, 'static>;
    let co = Coroutine::with_stack(stack, move |yielder: &Yielder<Resume, Suspend>, first: Resume| body(own, yielder, first, entry));
    let s = it.sched();
    let slot = &mut s.slots[tid as usize];
    slot.co = Some(co);
    slot.interp = own;
    slot.state = State::Runnable;
    s.runnable.push_back(tid);
    Ok(())
}

/// A failure of the interpreter's own (a shape or a builtin it lacks) on a thread: the program stops
/// with `status` and no hook runs, since threads the failed one would have woken (a pool's task, a
/// fiber's continuation) may be what a hook waits for.
fn halt(it: &mut Interp, status: i32) {
    if let Some(s) = it.sched.as_deref_mut() {
        if s.shutdown.is_none() {
            s.shutdown = Some(status);
        }
        if s.hooks == Hooks::NotStarted {
            s.hooks = Hooks::Done;
        }
    }
}

/// The shutdown begins with `status`, unless it already has.
pub(super) fn begin_shutdown(it: &mut Interp, status: i32) {
    if let Some(s) = it.sched.as_deref_mut() {
        if s.shutdown.is_none() {
            s.shutdown = Some(status);
        }
    }
}

/// Whether the program's threads are coroutines: a signal then begins the shutdown, and the thread it
/// was seen on goes on.
pub(super) fn threaded(it: &Interp) -> bool {
    it.sched.as_deref().is_some_and(|s| s.threaded)
}

/// The end of a slice of the running thread's steps: it gives way when another thread can run, a timer
/// is due or the shutdown's hooks wait to start.
pub(super) fn quantum(it: &mut Interp) -> R<()> {
    let contended = it.sched.as_deref_mut().is_some_and(|s| s.threaded && s.contended());
    if contended {
        give_way(it)
    } else {
        Ok(())
    }
}

/// The running thread lets the others run, and runs on after them.
fn give_way(it: &mut Interp) -> R<()> {
    #[cfg(feature = "threads")]
    return it.switch(Suspend::Yield);
    #[cfg(not(feature = "threads"))]
    {
        let _ = it;
        Ok(())
    }
}

/// Blocks the running thread in `wait` until something wakes it or, with a deadline, its time is up:
/// what woke it. Without coroutines main waits alone, on the OS thread, for its deadline or a signal.
fn block(it: &mut Interp, wait: Wait, deadline: Option<Instant>) -> R<Wake> {
    it.flush();
    let s = it.sched();
    let tid = s.current;
    let slot = &mut s.slots[tid as usize];
    slot.gen += 1;
    slot.state = State::Blocked(wait);
    slot.timed = deadline.is_some();
    slot.wake = Wake::None;
    let gen = slot.gen;
    if let Some(at) = deadline {
        s.seq += 1;
        let seq = s.seq;
        s.timers.push(Reverse((at, seq, tid, gen)));
    }
    if s.threaded {
        #[cfg(feature = "threads")]
        it.switch(Suspend::Block)?;
    } else {
        loop {
            process::check_signal_blocking(it)?;
            let s = it.sched();
            if s.slots[tid as usize].state != State::Blocked(wait) {
                break;
            }
            let now = Instant::now();
            match deadline {
                Some(at) if at <= now => {
                    s.expire(now);
                    break;
                }
                Some(at) => std::thread::sleep((at - now).min(IDLE)),
                None => std::thread::sleep(IDLE),
            }
        }
    }
    Ok(it.sched().slots[tid as usize].wake)
}

/// The deadline `nanos` from now, none past what an `Instant` holds.
fn deadline_in(nanos: i64) -> Option<Instant> {
    Instant::now().checked_add(Duration::from_nanos(nanos.max(0) as u64))
}

/// The running thread's interrupt, cleared when `clear`.
fn interrupted(it: &mut Interp, clear: bool) -> bool {
    let s = it.sched();
    let slot = &mut s.slots[s.current as usize];
    let was = slot.interrupted;
    if clear {
        slot.interrupted = false;
    }
    was
}

/// `Thread.sleep`: the running thread waits for the time, an interrupt ending the wait with
/// `InterruptedException`, its flag cleared, as the JDK's `sleep0`.
fn sleep(it: &mut Interp, nanos: i64) -> R<()> {
    if it.pure {
        return it.impure("sleeping");
    }
    let end = deadline_in(nanos);
    loop {
        if interrupted(it, true) {
            return it.throw_named("InterruptedException", "sleep interrupted");
        }
        if end.is_some_and(|at| at <= Instant::now()) {
            return Ok(());
        }
        block(it, Wait::Sleep, end)?;
    }
}

/// Releases the monitor `k` the running thread holds whole: handed to the first thread waiting to
/// enter it, else freed (and forgotten unless a thread waits in it).
fn release(s: &mut Sched, k: &Identity) {
    let Some(m) = s.monitors.get_mut(k) else { return };
    match m.entrants.pop_front() {
        Some((next, count)) => {
            m.owner = next;
            m.count = count;
            s.wake(next, Wake::Signalled);
        }
        None => {
            m.owner = NOBODY;
            m.count = 0;
            if m.waiters.is_empty() {
                s.monitors.remove(k);
            }
        }
    }
}

/// How an acquisition of a lock may end: it waits for ever, gives up at once, or at a deadline; an
/// interrupt ends it or not.
#[derive(Clone, Copy)]
struct Acquire {
    deadline: Option<Instant>,
    only_try: bool,
    interruptible: bool,
}

const FOR_EVER: Acquire = Acquire { deadline: None, only_try: false, interruptible: false };

/// Acquires the lock of `v` with `count` holds (one for `synchronized` and `lock`, a waiter's own
/// count when it enters again): 0 once held, 1 when it gave up (a `tryLock`, its time up), 2 when an
/// interrupt ended it (the interrupt cleared). A thread waiting to enter is handed the lock by its
/// holder's release (`release`), which wakes it.
fn acquire(it: &mut Interp, v: &Value, k: Identity, count: u32, how: Acquire) -> R<i32> {
    let s = it.sched();
    let tid = s.current;
    match s.monitors.get_mut(&k) {
        None => {
            s.monitors.insert(k, Monitor { obj: v.clone(), owner: tid, count, entrants: VecDeque::new(), waiters: Vec::new() });
            return Ok(0);
        }
        Some(m) if m.owner == tid => {
            m.count += count;
            return Ok(0);
        }
        Some(m) if m.owner == NOBODY => {
            m.owner = tid;
            m.count = count;
            return Ok(0);
        }
        Some(_) if how.only_try => return Ok(1),
        Some(m) => m.entrants.push_back((tid, count)),
    }
    let wait = if how.interruptible || how.deadline.is_some() { Wait::Park } else { Wait::Enter };
    loop {
        let why = block(it, wait, how.deadline)?;
        let s = it.sched();
        // JDK 24's `AbstractQueuedSynchronizer.acquire`: the interrupt an interruptible acquisition finds
        // as its park returns cancels it before any acquiring, whatever woke it and whether or not the
        // holder released meanwhile. A lock the release handed this waiter passes on, as
        // `cancelAcquire`'s `signalNext` wakes the next.
        if how.interruptible && s.slots[tid as usize].interrupted {
            s.slots[tid as usize].interrupted = false;
            let m = s.monitors.get_mut(&k).expect("a lock with an entrant");
            if m.owner == tid {
                release(s, &k);
            } else {
                m.entrants.retain(|&(t, _)| t != tid);
            }
            return Ok(2);
        }
        let m = s.monitors.get_mut(&k).expect("a lock with an entrant");
        if m.owner == tid {
            return Ok(0);
        }
        if why == Wake::Timeout {
            m.entrants.retain(|&(t, _)| t != tid);
            return Ok(1);
        }
    }
}

/// Whether some thread runs an initialisation, which `wait_init` then looks up.
pub(super) fn initialising(it: &Interp) -> bool {
    it.sched.as_deref().is_some_and(|s| !s.inits.is_empty())
}

/// The running thread begins initialisation `key` of `obj`, which other threads reading it meanwhile
/// wait for (`wait_init`): as the JVM marks a class in progress by the current thread (JLS 12.4.2's
/// step 6) and `LazyVals.mkThreadSafeDef` sets `Evaluating`. Nothing outside a program's run.
pub(super) fn begin_init(it: &mut Interp, key: InitKey, obj: &Value) {
    let Some(s) = it.sched.as_deref_mut() else { return };
    let owner = s.current;
    s.inits.entry(key).and_modify(|i| i.depth += 1).or_insert_with(|| Init { owner, depth: 1, obj: obj.clone(), waiters: Vec::new() });
}

/// Its end, completed or failed: the threads waiting run again, to find it made or, failed, to run it
/// themselves, as `LazyVals` sets `Evaluating` back to null and counts the `Waiting` latch down.
pub(super) fn end_init(it: &mut Interp, key: &InitKey) {
    let Some(s) = it.sched.as_deref_mut() else { return };
    let Some(i) = s.inits.get_mut(key) else { return };
    i.depth -= 1;
    if i.depth > 0 {
        return;
    }
    if let Some(i) = s.inits.remove(key) {
        for w in i.waiters {
            s.wake(w, Wake::Signalled);
        }
    }
}

/// Waits while another thread runs initialisation `key`, until it ends (JLS 12.4.2's step 2, a
/// `Waiting` latch's `await`, the holder's monitor): whether it waited. The thread running it reads on
/// (step 3) and sees what is made so far. A module's wait is the JVM's, which an interrupt does not end;
/// a lazy val's ends at one with `InterruptedException`, as `CountDownLatch.await` does; a local lazy
/// val's is a monitor's entry, `BLOCKED` and not interrupted.
pub(super) fn wait_init(it: &mut Interp, key: &InitKey) -> R<bool> {
    let (interruptible, wait) = match key {
        InitKey::Module(_) => (false, Wait::Notify),
        InitKey::Lazy(..) => (true, Wait::Park),
        InitKey::Local(_) => (false, Wait::Enter),
    };
    let mut waited = false;
    loop {
        let Some(s) = it.sched.as_deref_mut() else { return Ok(waited) };
        let tid = s.current;
        match s.inits.get_mut(key) {
            Some(i) if i.owner != tid => {
                if !i.waiters.contains(&tid) {
                    i.waiters.push(tid);
                }
            }
            _ => return Ok(waited),
        }
        waited = true;
        // The latch's wait checks the interrupt before it parks and again as the park returns, before
        // it looks at the latch (JDK 24's `acquireSharedInterruptibly` and `AbstractQueuedSynchronizer
        // .acquire`): an interrupt there ends the wait though the initialisation ended meanwhile.
        if interruptible && interrupted(it, true) {
            return init_wait_interrupted(it, key, tid);
        }
        block(it, wait, None)?;
        if interruptible && interrupted(it, true) {
            return init_wait_interrupted(it, key, tid);
        }
    }
}

/// A lazy val's wait ended by an interrupt (cleared): the thread leaves the waiters, and
/// `InterruptedException` is thrown, as the latch's `await` throws it.
fn init_wait_interrupted(it: &mut Interp, key: &InitKey, tid: Tid) -> R<bool> {
    if let Some(i) = it.sched().inits.get_mut(key) {
        i.waiters.retain(|&w| w != tid);
    }
    it.throw_named("InterruptedException", "")
}

/// `synchronized`'s entry; `None` where the value has no monitor (a primitive), whose block runs as
/// it is.
pub(super) fn monitor_enter(it: &mut Interp, v: &Value) -> R<Option<Identity>> {
    if matches!(v, Value::Null) {
        return it.throw_named("NullPointerException", "Cannot enter synchronized block because the value is null");
    }
    if it.sched.is_none() {
        return Ok(None);
    }
    let Some(k) = identity(v) else { return Ok(None) };
    acquire(it, v, k.clone(), 1, FOR_EVER)?;
    Ok(Some(k))
}

/// `synchronized`'s exit: one hold of the monitor fewer, released at none.
pub(super) fn monitor_exit(it: &mut Interp, k: &Identity) {
    let Some(s) = it.sched.as_deref_mut() else { return };
    let tid = s.current;
    let Some(m) = s.monitors.get_mut(k) else { return };
    if m.owner != tid {
        return;
    }
    m.count -= 1;
    if m.count == 0 {
        release(s, k);
    }
}

/// The lock of `v` the running thread holds, or the JDK's `IllegalMonitorStateException` with
/// `message` (an object's monitor names its owner, a `ReentrantLock` nothing).
fn owned(it: &mut Interp, v: &Value, message: &str) -> R<Identity> {
    let k = identity(v);
    let held = match (&k, it.sched.as_deref()) {
        (Some(k), Some(s)) => s.monitors.get(k).is_some_and(|m| m.owner == s.current),
        _ => false,
    };
    match k {
        Some(k) if held => Ok(k),
        _ => it.throw_named("IllegalMonitorStateException", message),
    }
}

/// A wait in condition `cond` of the lock of `v` the running thread holds (`Object.wait`,
/// `Condition.await`): the lock released whole, the thread waiting for a signal, an interrupt (when
/// `interruptible`; else it waits on, its interrupt left set) or the time, then acquiring the lock
/// again with its count. 0 at a signal, 1 at the time, 2 at an interrupt (cleared), which the caller
/// throws.
fn await_condition(it: &mut Interp, v: &Value, k: Identity, cond: u32, nanos: i64, interruptible: bool) -> R<i32> {
    if interruptible && interrupted(it, true) {
        return Ok(2);
    }
    let s = it.sched();
    let tid = s.current;
    let m = s.monitors.get_mut(&k).expect("an owned lock");
    let count = m.count;
    // JDK 24's `ConditionObject.awaitNanos` with no time: `enableWait` releases the lock whole all the
    // same, the wait is cancelled at once, and `reacquire` takes the lock back behind the threads queued
    // for it, which run first.
    if nanos == 0 {
        release(s, &k);
        acquire(it, v, k, count, FOR_EVER)?;
        return Ok(1);
    }
    m.waiters.push((tid, cond, count));
    release(s, &k);
    let end = if nanos > 0 { deadline_in(nanos) } else { None };
    let mut why;
    loop {
        why = block(it, Wait::Notify, end)?;
        if transferred(it.sched(), &k, tid) || why != Wake::Interrupted || interruptible {
            break;
        }
    }
    // A waiter a signal moved to the lock's queue (`signal`) waits there until the lock is handed to it,
    // as `await` parks until `canReacquire`: an interrupt or its time no longer cancels it, `doSignal`
    // having taken the node, so the wait returns as signalled, the interrupt left set.
    if transferred(it.sched(), &k, tid) {
        while !it.sched().monitors.get(&k).is_some_and(|m| m.owner == tid) {
            block(it, Wait::Notify, None)?;
        }
        return Ok(0);
    }
    // Cancelled by its time or an interrupt: it leaves the condition and queues for the lock behind the
    // threads queued already.
    let s = it.sched();
    if let Some(m) = s.monitors.get_mut(&k) {
        m.waiters.retain(|&(w, _, _)| w != tid);
    }
    acquire(it, v, k, count, FOR_EVER)?;
    Ok(match why {
        Wake::Interrupted => {
            interrupted(it, true);
            2
        }
        Wake::Timeout => 1,
        _ => 0,
    })
}

/// A signal of condition `cond` of a lock the running thread holds (`Object.notify`,
/// `Condition.signal`): one waiter, or all, moved to the lock's queue with the count it holds the lock
/// with, as JDK 24's `ConditionObject.doSignal` enqueues the node, so that the lock's next release
/// hands it over in its turn and the waiter runs then. A waiter is in the condition's list until it
/// runs again: one an interrupt or its time made runnable, and which has not run to cancel its wait, is
/// moved all the same, its wait then returning as signalled with the interrupt left pending, while one
/// that cancelled is gone and the signal reaches the next (`doSignal` against `await`'s cancellation).
fn signal(it: &mut Interp, k: &Identity, cond: u32, all: bool) {
    let s = it.sched();
    let m = s.monitors.get_mut(k).expect("an owned lock");
    let mut moved = Vec::new();
    m.waiters.retain(|&(w, c, count)| {
        if c == cond && (all || moved.is_empty()) {
            moved.push((w, count));
            false
        } else {
            true
        }
    });
    m.entrants.extend(moved);
}

/// Whether a signal moved thread `tid`, waiting in a condition of the lock `k`, to the lock's queue,
/// or the lock was handed to it since.
fn transferred(s: &Sched, k: &Identity, tid: Tid) -> bool {
    s.monitors.get(k).is_some_and(|m| m.owner == tid || m.entrants.iter().any(|&(t, _)| t == tid))
}

fn int(a: A, i: usize) -> i32 {
    match a.get(i) {
        Some(Value::Int(n)) => *n,
        _ => -1,
    }
}

fn long(a: A, i: usize) -> i64 {
    match a.get(i) {
        Some(Value::Long(n)) => *n,
        Some(Value::Int(n)) => *n as i64,
        _ => 0,
    }
}

fn boolean(a: A, i: usize) -> bool {
    matches!(a.get(i), Some(Value::Bool(true)))
}

/// The slot of the thread numbered `tid` by the std's `Thread`, if it has started.
fn started(it: &mut Interp, tid: i32) -> Option<usize> {
    let s = it.sched.as_deref()?;
    (tid >= 0 && (tid as usize) < s.slots.len()).then_some(tid as usize)
}

/// The natives of `std/javalib/concurrent.scala`'s threads and `locks.scala`'s park, by the
/// qualified names of their `@js` defs.
pub(super) fn install(it: &mut Table) {
    // `Thread.start`: a new logical thread runs the object's `run`; its number for the object.
    reg!(it, "java.lang.threadStart", |it, a| {
        if it.pure {
            return it.impure("starting a thread");
        }
        let obj = a.first().cloned().unwrap_or(Value::Null);
        let (daemon, interrupt) = (boolean(a, 1), boolean(a, 2));
        let mut locals = FxMap::default();
        if let Some(Value::Array(pairs)) = a.get(3) {
            for pair in pairs.borrow().chunks(2) {
                if let ([local, value], Some(k)) = (pair, identity(&pair[0])) {
                    locals.insert(k, (local.clone(), value.clone()));
                }
            }
        }
        start(it, obj, daemon, interrupt, locals)
    });
    // The running thread's `InheritableThreadLocal` values for a thread it makes (`inherited`), each
    // local followed by its value; null outside a program's run or with none.
    reg!(it, "java.lang.threadInherited", |it, _a| {
        if it.sched.is_none() {
            return Ok(Value::Null);
        }
        let locals = inherited(it)?;
        if locals.is_empty() {
            return Ok(Value::Null);
        }
        Ok(Value::array(locals.into_values().flat_map(|(local, value)| [local, value]).collect()))
    });
    // The running thread's object, `null` for main's (the std's own) and the hooks' runner.
    reg!(it, "java.lang.threadCurrent", |it, _a| {
        Ok(match it.sched.as_deref() {
            Some(s) if s.current != NOBODY => s.slots[s.current as usize].obj.clone(),
            _ => Value::Null,
        })
    });
    // Whether the threads are the layer's: in a program's run, not in a compile-time one.
    reg!(it, "java.lang.threadsRun", |it, _a| Ok(Value::Bool(it.sched.is_some())));
    // The same for `java.util.concurrent`'s std, whose package every target's std has (`ConcurrentHashMap`).
    reg!(it, "java.util.concurrent.logicalThreads", |it, _a| Ok(Value::Bool(it.sched.is_some())));
    // A `ConcurrentHashMap` writer's wait in the map's monitor for a remapping's end, which an interrupt does not
    // end, as a writer blocks on the JDK's bin lock; and the remapping's end waking them.
    reg!(it, "java.util.concurrent.binWait", |it, a| {
        if it.pure {
            return it.impure("waiting");
        }
        let v = a.first().cloned().unwrap_or(Value::Null);
        let k = owned(it, &v, "current thread is not owner")?;
        await_condition(it, &v, k, 0, -1, false)?;
        Ok(Value::Unit)
    });
    reg!(it, "java.util.concurrent.binNotifyAll", |it, a| {
        let v = a.first().cloned().unwrap_or(Value::Null);
        let k = owned(it, &v, "current thread is not owner")?;
        signal(it, &k, 0, true);
        Ok(Value::Unit)
    });
    reg!(it, "java.lang.threadSleep", |it, a| {
        let nanos = long(a, 0).saturating_mul(1_000_000).saturating_add(long(a, 1));
        sleep(it, nanos)?;
        Ok(Value::Unit)
    });
    // Whether thread `tid` was interrupted, the flag cleared when asked: the running thread's when
    // `tid` is -2.
    reg!(it, "java.lang.threadInterrupted", |it, a| {
        let clear = boolean(a, 1);
        let tid = int(a, 0);
        if tid == -2 {
            return Ok(Value::Bool(it.sched.is_some() && interrupted(it, clear)));
        }
        let Some(i) = started(it, tid) else { return Ok(Value::Bool(false)) };
        let slot = &mut it.sched().slots[i];
        let was = slot.interrupted;
        if clear {
            slot.interrupted = false;
        }
        Ok(Value::Bool(was))
    });
    // `Thread.interrupt`: the flag set, a thread blocked in a sleep, a join, a wait or a park woken.
    reg!(it, "java.lang.threadInterrupt", |it, a| {
        if let Some(i) = started(it, int(a, 0)) {
            let s = it.sched();
            s.slots[i].interrupted = true;
            if let State::Blocked(w) = s.slots[i].state {
                if w != Wait::Enter {
                    s.wake(i as Tid, Wake::Interrupted);
                }
            }
        }
        Ok(Value::Unit)
    });
    // The state of thread `tid` as `Thread.State`'s ordinal: 1 runnable, 2 blocked, 3 waiting,
    // 4 timed waiting, 5 terminated.
    reg!(it, "java.lang.threadState", |it, a| {
        let Some(i) = started(it, int(a, 0)) else { return Ok(Value::Int(1)) };
        let slot = &it.sched().slots[i];
        Ok(Value::Int(match slot.state {
            State::Runnable => 1,
            State::Blocked(Wait::Enter) => 2,
            State::Blocked(_) if slot.timed => 4,
            State::Blocked(_) => 3,
            State::Done => 5,
        }))
    });
    // `Thread.join(millis)` of thread `tid`, 0 for ever: 0 once it ended, 1 at the time, 2 at an
    // interrupt of the joining thread (cleared), which the caller throws.
    reg!(it, "java.lang.threadJoin", |it, a| {
        if it.pure {
            return it.impure("joining a thread");
        }
        let Some(i) = started(it, int(a, 0)) else { return Ok(Value::Int(0)) };
        let nanos = long(a, 1);
        let end = if nanos > 0 { deadline_in(nanos) } else { None };
        // JDK 24's `join(long)`: the thread's end first (`isAlive`), then the time, and the interrupt only
        // where it would wait, as `wait` sees it: a thread that has ended is joined, the interrupt left set.
        loop {
            let s = it.sched();
            if s.slots[i].state == State::Done {
                return Ok(Value::Int(0));
            }
            if end.is_some_and(|at| at <= Instant::now()) {
                return Ok(Value::Int(1));
            }
            if interrupted(it, true) {
                return Ok(Value::Int(2));
            }
            let s = it.sched();
            let me = s.current;
            s.slots[i].joiners.push(me);
            block(it, Wait::Join, end)?;
            let s = it.sched();
            s.slots[i].joiners.retain(|&j| j != me);
        }
    });
    reg!(it, "java.lang.threadYield", |it, _a| {
        if it.sched.as_deref().is_some_and(|s| s.threaded && !s.runnable.is_empty()) {
            give_way(it)?;
        }
        Ok(Value::Unit)
    });
    // `LockSupport.park`, `parkNanos` and `parkUntil` (`nanos` from now, -1 for ever): the permit
    // taken, or the thread blocked until an unpark, an interrupt (left set) or the time.
    reg!(it, "java.util.concurrent.locks.parkNative", |it, a| {
        if it.pure {
            return it.impure("parking a thread");
        }
        let nanos = long(a, 0);
        let s = it.sched();
        let slot = &mut s.slots[s.current as usize];
        if std::mem::replace(&mut slot.permit, false) || slot.interrupted || nanos == 0 {
            return Ok(Value::Unit);
        }
        let end = if nanos > 0 { deadline_in(nanos) } else { None };
        block(it, Wait::Park, end)?;
        Ok(Value::Unit)
    });
    // `LockSupport.unpark` of thread `tid`: woken from its park, or its permit given.
    reg!(it, "java.util.concurrent.locks.unparkNative", |it, a| {
        if let Some(i) = started(it, int(a, 0)) {
            let s = it.sched();
            match s.slots[i].state {
                State::Blocked(Wait::Park) => s.wake(i as Tid, Wake::Signalled),
                State::Done => {}
                _ => s.slots[i].permit = true,
            }
        }
        Ok(Value::Unit)
    });
    // `Object.wait(millis, nanos)`, 0 and 0 for ever: `true` when an interrupt ended it.
    reg!(it, "java.lang.monitorWait", |it, a| {
        if it.pure {
            return it.impure("waiting");
        }
        let v = a.first().cloned().unwrap_or(Value::Null);
        let k = owned(it, &v, "current thread is not owner")?;
        let nanos = long(a, 1).saturating_mul(1_000_000).saturating_add(long(a, 2));
        Ok(Value::Bool(await_condition(it, &v, k, 0, if nanos == 0 { -1 } else { nanos }, true)? == 2))
    });
    reg!(it, "java.lang.monitorNotify", |it, a| {
        let v = a.first().cloned().unwrap_or(Value::Null);
        let k = owned(it, &v, "current thread is not owner")?;
        signal(it, &k, 0, boolean(a, 1));
        Ok(Value::Unit)
    });
    // A `ReentrantLock`'s acquisition, its lock keyed by the object `sync`: `nanos` -1 for ever, 0 a
    // `tryLock`, else its time; 0 once held, 1 given up, 2 interrupted (`acquire`).
    reg!(it, "java.util.concurrent.locks.lockAcquire", |it, a| {
        if it.pure {
            return it.impure("locking");
        }
        let v = a.first().cloned().unwrap_or(Value::Null);
        let Some(k) = identity(&v) else { return it.unsupported("a lock without an identity") };
        let nanos = long(a, 2);
        let how = Acquire { deadline: if nanos > 0 { deadline_in(nanos) } else { None }, only_try: nanos == 0, interruptible: boolean(a, 1) };
        Ok(Value::Int(acquire(it, &v, k, 1, how)?))
    });
    // `ReentrantLock.unlock`: one hold fewer, `IllegalMonitorStateException` from a thread not holding it.
    reg!(it, "java.util.concurrent.locks.lockRelease", |it, a| {
        let v = a.first().cloned().unwrap_or(Value::Null);
        let k = owned(it, &v, "")?;
        monitor_exit(it, &k);
        Ok(Value::Unit)
    });
    // The running thread's holds of the lock `sync`, 0 where it does not hold it.
    reg!(it, "java.util.concurrent.locks.lockHolds", |it, a| {
        let v = a.first().cloned().unwrap_or(Value::Null);
        let held = match (identity(&v), it.sched.as_deref()) {
            (Some(k), Some(s)) => s.monitors.get(&k).filter(|m| m.owner == s.current).map_or(0, |m| m.count),
            _ => 0,
        };
        Ok(Value::Int(held as i32))
    });
    // Whether any thread holds the lock `sync`, and how many wait to enter it.
    reg!(it, "java.util.concurrent.locks.lockState", |it, a| {
        let v = a.first().cloned().unwrap_or(Value::Null);
        let (locked, queued) = match (identity(&v), it.sched.as_deref()) {
            (Some(k), Some(s)) => s.monitors.get(&k).map_or((false, 0), |m| (m.owner != NOBODY, m.entrants.len())),
            _ => (false, 0),
        };
        Ok(Value::Int(if boolean(a, 1) { queued as i32 } else { locked as i32 }))
    });
    // `Condition.await` in condition `cond` of the lock `sync` (`await_condition`): `nanos` -1 for
    // ever; 0 at a signal, 1 at the time, 2 at an interrupt.
    reg!(it, "java.util.concurrent.locks.conditionAwait", |it, a| {
        if it.pure {
            return it.impure("waiting");
        }
        let v = a.first().cloned().unwrap_or(Value::Null);
        let k = owned(it, &v, "")?;
        Ok(Value::Int(await_condition(it, &v, k, int(a, 1) as u32, long(a, 2), boolean(a, 3))?))
    });
    reg!(it, "java.util.concurrent.locks.conditionSignal", |it, a| {
        let v = a.first().cloned().unwrap_or(Value::Null);
        let k = owned(it, &v, "")?;
        signal(it, &k, int(a, 1) as u32, boolean(a, 2));
        Ok(Value::Unit)
    });
    reg!(it, "java.lang.holdsLock", |it, a| {
        let v = a.first().cloned().unwrap_or(Value::Null);
        let held = match (identity(&v), it.sched.as_deref()) {
            (Some(k), Some(s)) => s.monitors.get(&k).is_some_and(|m| m.owner == s.current),
            _ => false,
        };
        Ok(Value::Bool(held))
    });
}

/// `Thread.start`: thread `obj` made and runnable, with the `InheritableThreadLocal` values it took at its
/// construction; its number.
fn start(it: &mut Interp, obj: Value, daemon: bool, interrupt: bool, locals: FxMap<Identity, (Value, Value)>) -> R {
    if it.sched.is_none() {
        return it.unsupported("a thread is started outside a program's run");
    }
    #[cfg(not(feature = "threads"))]
    {
        let _ = (obj, daemon, interrupt, locals);
        return it.unsupported(format!(
            "the program starts a thread, which this teq for {}-{} cannot run: it was built without the `threads` feature, which logical threads need",
            std::env::consts::ARCH,
            std::env::consts::OS
        ));
    }
    #[cfg(feature = "threads")]
    {
        if let Some(why) = it.sched().unavailable.clone() {
            let _ = (daemon, interrupt, locals);
            return it.unsupported(format!(
                "the program starts a thread, which this run of teq for {}-{} cannot run: the system refused the address space logical threads' stacks reserve ({}){}",
                std::env::consts::ARCH,
                std::env::consts::OS,
                why,
                address_space_limit().map_or(String::new(), |kib| format!(", under the process's limit of {} KiB of it (`ulimit -v`)", kib))
            ));
        }
        let s = it.sched();
        let tid = s.slots.len() as Tid;
        let mut slot = Slot::new(obj.clone(), !daemon);
        slot.interrupted = interrupt;
        slot.locals = locals;
        s.slots.push(slot);
        if make_coroutine(it, tid, Entry::Run(obj), THREAD_STACK, THREAD_BUDGET).is_err() {
            it.sched().slots[tid as usize].state = State::Done;
            return it.throw_named("OutOfMemoryError", "unable to create native thread: possibly out of memory or process/resource limits reached");
        }
        Ok(Value::Int(tid as i32))
    }
}

/// The process's limit of its address space in KiB (`RLIMIT_AS`), none where it has none.
fn address_space_limit() -> Option<u64> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        #[repr(C)]
        struct RLimit {
            cur: u64,
            max: u64,
        }
        extern "C" {
            fn getrlimit(resource: i32, rlim: *mut RLimit) -> i32;
        }
        #[cfg(target_os = "linux")]
        const RLIMIT_AS: i32 = 9;
        #[cfg(target_os = "macos")]
        const RLIMIT_AS: i32 = 5;
        let mut l = RLimit { cur: 0, max: 0 };
        // SAFETY: fills the struct it is given.
        let ok = unsafe { getrlimit(RLIMIT_AS, &mut l) } == 0;
        (ok && l.cur != u64::MAX).then_some(l.cur / 1024)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    None
}

/// The running thread's `InheritableThreadLocal` values as a thread it makes takes them: each through
/// the local's `childValue`, as the JDK's `ThreadLocal.createInheritedMap` makes them at the thread's
/// construction.
fn inherited(it: &mut Interp) -> R<FxMap<Identity, (Value, Value)>> {
    let s = it.sched();
    let mine: Vec<(Identity, (Value, Value))> = s.slots[s.current as usize].locals.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    let mut out = FxMap::default();
    for (k, (local, value)) in mine {
        if let Value::Bool(true) = it.call_by_name(local.clone(), "inheritable", Vec::new())? {
            let child = it.call_by_name(local.clone(), "childValue", vec![value])?;
            out.insert(k, (local, child));
        }
    }
    Ok(out)
}

/// `ThreadLocal.get` under `teq interp`: the running thread's value of `local`, its `initialValue`
/// stored first where it has none, as the JDK's `setInitialValue`; `None` in a compile-time run, whose
/// one thread keeps the value in the local (the Scala body).
pub(super) fn local_get(it: &mut Interp, local: &Value) -> R<Option<Value>> {
    let Some(s) = it.sched.as_deref() else { return Ok(None) };
    let Some(k) = identity(local) else { return Ok(None) };
    let tid = s.current as usize;
    if let Some((_, v)) = s.slots.get(tid).and_then(|slot| slot.locals.get(&k)) {
        return Ok(Some(v.clone()));
    }
    let v = it.call_by_name(local.clone(), "initialValue", Vec::new())?;
    if let Some(slot) = it.sched().slots.get_mut(tid) {
        slot.locals.insert(k, (local.clone(), v.clone()));
    }
    Ok(Some(v))
}

/// `ThreadLocal.set`, and `remove` with no value.
pub(super) fn local_set(it: &mut Interp, local: &Value, value: Option<Value>) -> Option<Value> {
    let s = it.sched.as_deref_mut()?;
    let k = identity(local)?;
    let tid = s.current as usize;
    let slot = s.slots.get_mut(tid)?;
    match value {
        Some(v) => slot.locals.insert(k, (local.clone(), v)),
        None => slot.locals.remove(&k),
    };
    Some(Value::Unit)
}
