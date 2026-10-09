//! The processes, the byte streams and the signals behind `std/javalib/process.scala` and
//! `std/javalib/io.scala`, as the JDK has them on the host: a child started with its redirects, its
//! status, its pipes, the process table that `ProcessHandle` reads, and the streams of files, pipes,
//! sockets and the interpreter's own stdin.
//!
//! The program runs on one thread; a child is waited for by a thread of its own (the JDK's process
//! reaper), which reaps it as soon as it ends, whether or not the program keeps its handle, and
//! records its status for the program's `waitFor`. A child is never ended because its handle was
//! dropped or the interpreter ends. Every blocking native (a wait, a sleep, a read of a pipe, an
//! accept) flushes the interpreter's output first and wakes at least every `SLICE` to see a signal:
//! SIGINT, SIGTERM or SIGHUP to `teq interp` stops the program as `System.exit(128 + n)` does, after
//! which the shutdown hooks run (`Interp::shut_down`); a second one ends the process at once.
//! Everything here is a program's under `teq interp`: a compile-time run has no effects.

use super::builtins::{reg, Table};
use super::value::*;
use super::*;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

type A<'a> = &'a [Value];

/// How long a blocking native waits before it looks for a signal again.
const SLICE: Duration = Duration::from_millis(50);

/// The size of the JDK's buffers over a process's pipes (`BufferedInputStream`,
/// `BufferedOutputStream`), which ours keep.
const BUFFER: usize = 8192;

// ---- signals ----

/// 0, a signal not yet seen by the program (its number), or one being handled (its number,
/// negated): the shutdown that follows it waits on no second one.
static SIGNAL: AtomicI32 = AtomicI32::new(0);

/// Has SIGINT, SIGTERM and SIGHUP (Ctrl-C, Ctrl-Break and the console's closing on Windows) stop
/// the program at its next step or blocking native, as the JVM's handlers start its shutdown. A
/// second signal ends the process with its status at once.
pub fn handle_signals() {
    os::handle_signals();
}

fn on_signal(sig: i32) {
    if SIGNAL.compare_exchange(0, sig, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        os::exit_now(128 + sig);
    }
}

/// The program's stop when a signal came: `Failure::Exit(128 + n)`, the JVM's status for it, once.
pub(super) fn check_signal(it: &mut Interp) -> R<()> {
    let sig = SIGNAL.load(Ordering::Relaxed);
    if sig > 0 && !it.pure && SIGNAL.compare_exchange(sig, -sig, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        return Err(Control::Fail(Failure::Exit(128 + sig)));
    }
    Ok(())
}

/// What a native that blocks does first: refuse a compile-time run, flush the program's output,
/// see a signal.
fn blocking(it: &mut Interp, what: &str) -> R<()> {
    if it.pure {
        return it.impure(what);
    }
    it.flush();
    check_signal(it)
}

/// `Thread.sleep`: the time in slices, a signal seen between them.
fn sleep(it: &mut Interp, millis: i64, nanos: i64) -> R<()> {
    blocking(it, "sleeping")?;
    let end = Instant::now() + Duration::from_millis(millis.max(0) as u64) + Duration::from_nanos(nanos.max(0) as u64);
    loop {
        let now = Instant::now();
        if now >= end {
            return Ok(());
        }
        std::thread::sleep((end - now).min(SLICE));
        check_signal(it)?;
    }
}

// ---- shutdown hooks ----

thread_local! {
    /// The hooks `Runtime.addShutdownHook` registered, in order; None once the shutdown began.
    static HOOKS: RefCell<Option<Vec<Value>>> = const { RefCell::new(Some(Vec::new())) };
}

fn same(a: &Value, b: &Value) -> bool {
    matches!((a, b), (Value::Obj(x), Value::Obj(y)) if Rc::ptr_eq(x, y))
}

/// The hooks to run at the end, none to be added after.
pub(super) fn take_hooks() -> Vec<Value> {
    HOOKS.with(|h| h.borrow_mut().take().unwrap_or_default())
}

// ---- children ----

/// The statuses of the children that ended, by their key, which their reaper threads write and
/// `ENDED` announces.
static EXITS: Mutex<Vec<(u64, i32)>> = Mutex::new(Vec::new());
static ENDED: Condvar = Condvar::new();
static NEXT_KEY: AtomicU64 = AtomicU64::new(1);

struct Child {
    key: u64,
    #[cfg(unix)]
    pid: u32,
    /// The status once the program has read it from `EXITS`.
    status: Option<i32>,
    /// The process's handle on Windows, open while its reaper holds the child: what
    /// `TerminateProcess` takes.
    #[cfg(windows)]
    handle: isize,
}

thread_local! {
    static CHILDREN: RefCell<Vec<Child>> = const { RefCell::new(Vec::new()) };
    static STREAMS: RefCell<Vec<Option<Stream>>> = const { RefCell::new(Vec::new()) };
}

fn exits() -> std::sync::MutexGuard<'static, Vec<(u64, i32)>> {
    EXITS.lock().unwrap_or_else(|e| e.into_inner())
}

/// The child's status if it ended, read from its reaper once and kept.
fn status_of(child: usize) -> Option<i32> {
    CHILDREN.with(|c| {
        let mut children = c.borrow_mut();
        let ch = &mut children[child];
        if ch.status.is_none() {
            let mut ended = exits();
            if let Some(at) = ended.iter().position(|&(k, _)| k == ch.key) {
                ch.status = Some(ended.swap_remove(at).1);
            }
        }
        ch.status
    })
}

/// Waits for the child to end, until `deadline` when one is given: its status, or None at the
/// deadline. A signal stops the wait.
fn wait_child(it: &mut Interp, child: usize, deadline: Option<Instant>) -> R<Option<i32>> {
    blocking(it, "waiting for a process")?;
    let key = CHILDREN.with(|c| c.borrow()[child].key);
    loop {
        if let Some(status) = status_of(child) {
            return Ok(Some(status));
        }
        let now = Instant::now();
        if deadline.is_some_and(|d| now >= d) {
            return Ok(None);
        }
        let slice = deadline.map_or(SLICE, |d| (d - now).min(SLICE));
        let guard = exits();
        if !guard.iter().any(|&(k, _)| k == key) {
            let _ = ENDED.wait_timeout(guard, slice);
        }
        check_signal(it)?;
    }
}

/// Ends a child that has not ended: SIGTERM, or SIGKILL when `force`, on Unix; `TerminateProcess`
/// with status 1 on Windows, as the JDK's. The lock its reaper takes to record the status is held,
/// so that the process is still the child's (a zombie at worst) when it is signalled.
fn end_child(child: usize, force: bool) {
    CHILDREN.with(|c| {
        let children = c.borrow();
        let ch = &children[child];
        if ch.status.is_some() {
            return;
        }
        let ended = exits();
        if ended.iter().any(|&(k, _)| k == ch.key) {
            return;
        }
        #[cfg(unix)]
        os::signal_pid(ch.pid, if force { os::SIGKILL } else { os::SIGTERM });
        #[cfg(windows)]
        {
            let _ = force;
            os::terminate(ch.handle);
        }
        drop(ended);
    });
}

/// Reaps the child when it ends and records its status: a wait that leaves it a zombie, then, under
/// the lock that `end_child` holds to signal it, the reaping wait. The status is the JDK's: the
/// exit code, or 128 plus the signal that ended it.
fn reap(mut child: std::process::Child, key: u64, pipes: Vec<(Shared, bool)>) {
    #[cfg(unix)]
    let (ended, status) = {
        use std::os::unix::process::ExitStatusExt;
        os::wait_exited(child.id());
        let ended = exits();
        let status = match child.wait() {
            Ok(s) => s.code().unwrap_or_else(|| 128 + s.signal().unwrap_or(0)),
            Err(_) => -1,
        };
        (ended, status)
    };
    // The handle stays open, the process its own, until the status is recorded.
    #[cfg(not(unix))]
    let (ended, status) = {
        let status = child.wait().ok().and_then(|s| s.code()).unwrap_or(-1);
        (exits(), status)
    };
    let mut ended = ended;
    ended.push((key, status));
    drop(ended);
    ENDED.notify_all();
    drop(child);
    // The JDK on Windows leaves a child's pipes to the program (`ProcessImpl`'s streams over the
    // handles, closed with them): what the child wrote stays in the pipe, read and peeked there.
    #[cfg(unix)]
    close_pipes(&pipes);
    #[cfg(not(unix))]
    drop(pipes);
}

/// What a redirect of `ProcessBuilder` says, as the std encodes it: `pipe`, `inherit`, `discard`,
/// or `read:`, `write:` or `append:` and the file.
enum Redirect {
    Pipe,
    Inherit,
    Discard,
    Read(String),
    Write(String),
    Append(String),
}

fn redirect(code: &str) -> Redirect {
    match code.split_once(':') {
        Some(("read", f)) => Redirect::Read(f.to_string()),
        Some(("write", f)) => Redirect::Write(f.to_string()),
        Some(("append", f)) => Redirect::Append(f.to_string()),
        _ => match code {
            "inherit" => Redirect::Inherit,
            "discard" => Redirect::Discard,
            _ => Redirect::Pipe,
        },
    }
}

/// A file a redirect opens, or the JDK's message for it (`<file> (No such file or directory)`).
fn redirect_file(r: &Redirect) -> Result<Option<std::fs::File>, String> {
    let (path, opened) = match r {
        Redirect::Read(p) => (p, std::fs::File::open(p)),
        Redirect::Write(p) => (p, std::fs::File::create(p)),
        Redirect::Append(p) => (p, std::fs::OpenOptions::new().append(true).create(true).open(p)),
        _ => return Ok(None),
    };
    opened.map(Some).map_err(|e| format!("{} ({})", path, reason(&e)))
}

/// The child's environment on Windows as the JDK's `ProcessEnvironment.toEnvironmentBlock` makes
/// it: the entries in the order of their names compared without case as Windows compares them
/// (`NameComparator`, each differing character upper-cased), names that differ in case alone kept
/// in the map's order, and `SystemRoot` the interpreter's where no entry names it. The JDK passes
/// such twins both, and Windows reads the first; `Command` keeps one entry per name, the last.
#[cfg_attr(not(windows), allow(dead_code))]
fn windows_block(mut vars: Vec<(String, String)>, system_root: Option<String>) -> Vec<(String, String)> {
    vars.sort_by(|a, b| windows_name_order(&a.0, &b.0));
    if let Some(root) = system_root {
        if !vars.iter().any(|(k, _)| windows_name_order(k, "SystemRoot").is_eq()) {
            vars.push(("SystemRoot".to_string(), root));
        }
    }
    vars
}

/// `ProcessEnvironment.NameComparator`: UTF-16 units compared, two that differ upper-cased as
/// `Character.toUpperCase` does, then the lengths.
fn windows_name_order(a: &str, b: &str) -> std::cmp::Ordering {
    let upper = |u: u16| match char::from_u32(u as u32).map(|c| {
        let mut up = c.to_uppercase();
        (up.next(), up.next())
    }) {
        Some((Some(c), None)) if (c as u32) <= 0xffff => c as u32 as u16,
        _ => u,
    };
    let (a16, b16): (Vec<u16>, Vec<u16>) = (a.encode_utf16().collect(), b.encode_utf16().collect());
    for (&x, &y) in a16.iter().zip(&b16) {
        if x != y {
            let (x, y) = (upper(x), upper(y));
            if x != y {
                return x.cmp(&y);
            }
        }
    }
    a16.len().cmp(&b16.len())
}

/// `ProcessBuilder.start`: the command run with its arguments as given (no shell), in `dir`, with
/// the environment `env` (names and values, alternating) or the interpreter's, and its three
/// redirects; stderr into stdout's pipe when `merge`. Answers the child's index, its pid and the
/// streams of its stdin, stdout and stderr (-1 where the redirect is no pipe), or the JDK's
/// `IOException`.
fn start(it: &mut Interp, command: Vec<String>, dir: Option<String>, env: Option<Vec<(String, String)>>, redirects: [Redirect; 3], merge: bool) -> R {
    if it.pure {
        return it.impure("starting a process");
    }
    let program = command[0].clone();
    let failed = |message: String| -> String {
        match &dir {
            Some(d) => format!("Cannot run program \"{}\" (in directory \"{}\"): {}", program, d, message),
            None => format!("Cannot run program \"{}\": {}", program, message),
        }
    };
    let mut cmd = std::process::Command::new(&program);
    // The program is looked up on the interpreter's PATH, as the JDK's, not on the one the child
    // is given; its argv[0] stays as written.
    #[cfg(unix)]
    if !program.contains('/') {
        if let Some(found) = os::on_path(&program) {
            use std::os::unix::process::CommandExt;
            cmd = std::process::Command::new(found);
            cmd.arg0(&program);
        }
    }
    cmd.args(&command[1..]);
    if let Some(d) = &dir {
        cmd.current_dir(d);
    }
    if let Some(vars) = env {
        cmd.env_clear();
        #[cfg(windows)]
        let vars = windows_block(vars, std::env::var("SystemRoot").ok());
        for (k, v) in &vars {
            cmd.env(k, v);
        }
    }
    let mut files: [Option<std::fs::File>; 3] = [None, None, None];
    for (i, r) in redirects.iter().enumerate() {
        match redirect_file(r) {
            Ok(f) => files[i] = f,
            Err(message) => return io_error(it, &format!("Cannot run program \"{}\": {}", program, message)),
        }
    }
    let null = || std::process::Stdio::null();
    let stdio = |r: &Redirect, file: Option<std::fs::File>| match r {
        Redirect::Pipe => std::process::Stdio::piped(),
        Redirect::Inherit => std::process::Stdio::inherit(),
        Redirect::Discard => null(),
        _ => file.map_or_else(null, std::process::Stdio::from),
    };
    let [fin, fout, ferr] = files;
    cmd.stdin(stdio(&redirects[0], fin));
    // stderr merged into stdout: one pipe, its write end given to both.
    let mut merged: Option<std::io::PipeReader> = None;
    if merge {
        match &redirects[1] {
            Redirect::Pipe => match std::io::pipe() {
                Ok((reader, writer)) => {
                    let copy = match writer.try_clone() {
                        Ok(c) => c,
                        Err(e) => return io_error(it, &failed(os_message(&e))),
                    };
                    cmd.stdout(writer);
                    cmd.stderr(copy);
                    merged = Some(reader);
                }
                Err(e) => return io_error(it, &failed(os_message(&e))),
            },
            Redirect::Inherit => {
                cmd.stdout(std::process::Stdio::inherit());
                cmd.stderr(std::process::Stdio::inherit());
            }
            Redirect::Discard => {
                cmd.stdout(null());
                cmd.stderr(null());
            }
            _ => match fout.as_ref().map(|f| f.try_clone()) {
                Some(Ok(copy)) => {
                    cmd.stdout(fout.unwrap());
                    cmd.stderr(copy);
                }
                _ => return io_error(it, &failed("the output file cannot be shared".to_string())),
            },
        }
    } else {
        cmd.stdout(stdio(&redirects[1], fout));
        cmd.stderr(stdio(&redirects[2], ferr));
    }
    // A child that writes to the interpreter's stdout comes after what the program printed.
    it.flush();
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            // The write end of a merged pipe goes with the command.
            drop(cmd);
            return io_error(it, &failed(os_message(&e)));
        }
    };
    drop(cmd);
    let pid = child.id();
    let key = NEXT_KEY.fetch_add(1, Ordering::Relaxed);
    #[cfg(windows)]
    let handle = {
        use std::os::windows::io::AsRawHandle;
        child.as_raw_handle() as isize
    };
    let index = CHILDREN.with(|c| {
        let mut children = c.borrow_mut();
        children.push(Child {
            key,
            #[cfg(unix)]
            pid,
            status: None,
            #[cfg(windows)]
            handle,
        });
        children.len() - 1
    });
    // The pipes, shared with the reaper, which closes them when the child ends; stdin buffered as
    // the JDK's `BufferedOutputStream` over it.
    let mut pipes: Vec<(Shared, bool)> = Vec::new();
    let mut piped = |file: std::fs::File, input: bool| {
        let shared: Shared = std::sync::Arc::new(Mutex::new(Pipe { file: Some(file), rest: Vec::new() }));
        pipes.push((shared.clone(), input));
        new_stream(Io::Pipe(shared), Some(index), !input, Skip::Buffered)
    };
    let stdin = child.stdin.take().map(|s| piped(owned_file(s), false));
    let stdout = match merged {
        Some(reader) => Some(piped(owned_file(reader), true)),
        None => child.stdout.take().map(|s| piped(owned_file(s), true)),
    };
    let stderr = child.stderr.take().map(|s| piped(owned_file(s), true));
    let spawned = std::thread::Builder::new().name("teq-reaper".to_string()).stack_size(64 * 1024).spawn(move || {
        crate::alloc::enter();
        reap(child, key, pipes)
    });
    if let Err(e) = spawned {
        return it.unsupported(format!("no thread to wait for the process: {}", e));
    }
    let id = |s: Option<i32>| Value::Long(s.map_or(-1, i64::from));
    Ok(Value::array(vec![Value::Long(index as i64), Value::Long(pid as i64), id(stdin), id(stdout), id(stderr)]))
}

#[cfg(unix)]
fn owned_file(s: impl Into<std::os::fd::OwnedFd>) -> std::fs::File {
    std::fs::File::from(s.into())
}

#[cfg(windows)]
fn owned_file(s: impl Into<std::os::windows::io::OwnedHandle>) -> std::fs::File {
    std::fs::File::from(s.into())
}

// ---- streams ----

/// A child's pipe, which on Unix its reaper drains and closes when the child ends, as the JDK's
/// reaper does there (`ProcessPipeInputStream.processExited`): what was there to read kept in
/// `rest`, the descriptor gone (`file` None), so that a program that never touches the pipe again
/// holds none. On Windows the JDK's streams keep the handle until they are closed, and so does the
/// pipe. The program's thread and the reaper take it in turn.
struct Pipe {
    file: Option<std::fs::File>,
    rest: Vec<u8>,
}

type Shared = std::sync::Arc<Mutex<Pipe>>;

fn locked(p: &Shared) -> std::sync::MutexGuard<'_, Pipe> {
    p.lock().unwrap_or_else(|e| e.into_inner())
}

/// The reaper's side of a child's end on Unix: each pipe it reads from drained into its rest and
/// closed, the one it writes to closed.
#[cfg(unix)]
fn close_pipes(pipes: &[(Shared, bool)]) {
    for (pipe, input) in pipes {
        let mut p = locked(pipe);
        let Some(file) = p.file.take() else { continue };
        if *input {
            let mut file = file;
            loop {
                let n = os::pending(&file);
                if n == 0 {
                    break;
                }
                let mut chunk = vec![0u8; n];
                match file.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(k) => p.rest.extend_from_slice(&chunk[..k]),
                }
            }
        }
    }
}

/// What a stream reads or writes; `Closed` holds no descriptor.
enum Io {
    File(std::fs::File),
    Pipe(Shared),
    Stdin,
    Socket(std::net::TcpStream),
    Closed,
}

/// How a stream skips, as the JDK class behind it does: `FileInputStream` over a regular file seeks
/// and may pass the end; `Files.newInputStream` over one moves its channel's position, the end at
/// most; a child's pipe and stdin skip what `BufferedInputStream` holds, else read past the rest;
/// another stream reads past `n` bytes or to the end.
#[derive(Clone, Copy, PartialEq)]
enum Skip {
    Seek,
    Channel,
    Buffered,
    Read,
}

/// What a closed stream's read or write throws, as the JDK class behind it: `FileInputStream`'s and
/// `FileOutputStream`'s `Stream Closed`, a file channel's `ClosedChannelException`, a pipe's
/// `Stream closed`, a socket's `Socket closed`.
#[derive(Clone, Copy, PartialEq)]
enum Refusal {
    Plain,
    Channel,
    Pipe,
    Socket,
}

/// A stream of the program's: an `InputStream` or an `OutputStream` over a file, a child's pipe, a
/// socket or stdin. Reads come in through a buffer of `BUFFER` bytes; writes go out at once but to
/// a child's stdin, which the JDK buffers (`buffered`) until a flush.
struct Stream {
    io: Io,
    refusal: Refusal,
    child: Option<usize>,
    read: Vec<u8>,
    at: usize,
    write: Vec<u8>,
    buffered: bool,
    skip: Skip,
    closed: bool,
    /// A socket's read timeout (`setSoTimeout`), none for no limit.
    timeout: Option<Duration>,
    /// A socket's input shut down (`shutdownInput`), `NioSocketImpl`'s `isInputClosed`: what the
    /// buffer held dropped, every read at the end and nothing available, whichever view reads.
    input_shut: bool,
}

fn new_stream(io: Io, child: Option<usize>, buffered: bool, skip: Skip) -> i32 {
    let refusal = match (&io, skip) {
        (Io::Socket(_), _) => Refusal::Socket,
        (Io::File(_), Skip::Channel) => Refusal::Channel,
        _ => Refusal::Pipe,
    };
    new_stream_refusing(io, child, buffered, skip, refusal)
}

fn new_stream_refusing(io: Io, child: Option<usize>, buffered: bool, skip: Skip, refusal: Refusal) -> i32 {
    STREAMS.with(|s| {
        let mut streams = s.borrow_mut();
        streams.push(Some(Stream { io, refusal, child, read: Vec::new(), at: 0, write: Vec::new(), buffered, skip, closed: false, timeout: None, input_shut: false }));
        (streams.len() - 1) as i32
    })
}

/// Runs `f` on the stream, which is taken out of the table meanwhile.
fn with_stream<T>(it: &mut Interp, id: i32, f: impl FnOnce(&mut Interp, &mut Stream) -> R<T>) -> R<T> {
    let taken = STREAMS.with(|s| s.borrow_mut().get_mut(id as usize).and_then(Option::take));
    let Some(mut stream) = taken else { return it.unsupported(format!("no stream {}", id)) };
    let r = f(it, &mut stream);
    STREAMS.with(|s| s.borrow_mut()[id as usize] = Some(stream));
    r
}

fn stream_closed<T>(it: &mut Interp) -> R<T> {
    io_error(it, "Stream closed")
}

/// A closed stream's refusal of a read or a write (`Refusal`).
fn refused<T>(it: &mut Interp, s: &Stream) -> R<T> {
    match s.refusal {
        Refusal::Plain => io_error(it, "Stream Closed"),
        Refusal::Channel => it.throw_new(&["java", "nio", "channels", "ClosedChannelException"], vec![]),
        Refusal::Pipe => stream_closed(it),
        Refusal::Socket => it.throw_new(&["java", "net", "SocketException"], vec![Value::str("Socket closed")]),
    }
}

impl Stream {
    /// The bytes waiting to be read without blocking, past the buffer.
    fn pending(&self) -> usize {
        match &self.io {
            Io::File(f) => os::pending(f),
            Io::Pipe(p) => {
                let p = locked(p);
                p.file.as_ref().map_or(p.rest.len(), os::pending)
            }
            Io::Stdin => os::stdin_pending(),
            Io::Socket(s) => os::socket_pending(s),
            Io::Closed => 0,
        }
    }

    /// Whether a read would not block: data, the end, or a file.
    fn readable(&self, timeout: Duration) -> bool {
        match &self.io {
            Io::File(f) => os::wait_readable(f, timeout),
            Io::Pipe(p) => locked(p).file.as_ref().is_none_or(|f| os::wait_readable(f, timeout)),
            Io::Stdin => os::wait_stdin(timeout),
            Io::Socket(_) | Io::Closed => true,
        }
    }

    fn read_some(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.io {
            Io::File(f) => f.read(out),
            Io::Pipe(p) => {
                let mut p = locked(p);
                match &mut p.file {
                    Some(f) => f.read(out),
                    None => {
                        let n = out.len().min(p.rest.len());
                        out[..n].copy_from_slice(&p.rest[..n]);
                        p.rest.drain(..n);
                        Ok(n)
                    }
                }
            }
            Io::Stdin => os::read_stdin(out),
            Io::Socket(s) => s.read(out),
            Io::Closed => Ok(0),
        }
    }
}

/// Has bytes in the stream's buffer, reading when it holds none: false at the end.
fn fill(it: &mut Interp, s: &mut Stream) -> R<bool> {
    if s.at < s.read.len() {
        return Ok(true);
    }
    s.read.clear();
    s.at = 0;
    read_more(it, s)
}

/// Appends what one read gives to the stream's buffer, blocking until there is something or the
/// end: false at the end. A signal stops the wait; a socket's timeout is its
/// `SocketTimeoutException`.
fn read_more(it: &mut Interp, s: &mut Stream) -> R<bool> {
    if s.input_shut {
        return Ok(false);
    }
    blocking(it, "reading a stream")?;
    let mut chunk = vec![0u8; BUFFER];
    if matches!(s.io, Io::Socket(_)) {
        let end = s.timeout.map(|t| Instant::now() + t);
        loop {
            let now = Instant::now();
            if end.is_some_and(|e| now >= e) {
                return it.throw_new(&["java", "net", "SocketTimeoutException"], vec![Value::str("Read timed out")]);
            }
            let slice = end.map_or(SLICE, |e| (e - now).min(SLICE)).max(Duration::from_millis(1));
            if let Io::Socket(sock) = &s.io {
                let _ = sock.set_read_timeout(Some(slice));
            }
            match s.read_some(&mut chunk) {
                Ok(n) => {
                    s.read.extend_from_slice(&chunk[..n]);
                    return Ok(n > 0);
                }
                Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted) => check_signal(it)?,
                Err(e) => return io_error(it, &reason(&e)),
            }
        }
    }
    while !s.readable(SLICE) {
        check_signal(it)?;
    }
    loop {
        match s.read_some(&mut chunk) {
            Ok(n) => {
                s.read.extend_from_slice(&chunk[..n]);
                return Ok(n > 0);
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => check_signal(it)?,
            // A pipe whose writer ended, on Windows.
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => return Ok(false),
            Err(e) => return io_error(it, &reason(&e)),
        }
    }
}

fn read_byte(it: &mut Interp, id: i32) -> R {
    with_stream(it, id, |it, s| {
        if s.closed {
            return refused(it, s);
        }
        if !fill(it, s)? {
            return Ok(Value::Int(-1));
        }
        s.at += 1;
        Ok(Value::Int(s.read[s.at - 1] as i32))
    })
}

/// `read(b, off, len)`: what the buffer holds, else one read's worth; -1 at the end.
fn read_into(it: &mut Interp, id: i32, b: &Rc<ArrayCell>, off: usize, len: usize) -> R {
    with_stream(it, id, |it, s| {
        if s.closed {
            return refused(it, s);
        }
        if len == 0 {
            return Ok(Value::Int(0));
        }
        if !fill(it, s)? {
            return Ok(Value::Int(-1));
        }
        let n = len.min(s.read.len() - s.at);
        let mut items = b.write();
        for (i, &byte) in s.read[s.at..s.at + n].iter().enumerate() {
            items[off + i] = Value::Byte(byte as i8);
        }
        s.at += n;
        Ok(Value::Int(n as i32))
    })
}

/// `readAllBytes` and `readNBytes(len)`: up to `max` bytes, to the end.
fn read_up_to(it: &mut Interp, id: i32, max: usize) -> R<Vec<u8>> {
    with_stream(it, id, |it, s| {
        if s.closed {
            return refused(it, s);
        }
        let mut out = Vec::new();
        while out.len() < max && fill(it, s)? {
            let n = (max - out.len()).min(s.read.len() - s.at);
            out.extend_from_slice(&s.read[s.at..s.at + n]);
            s.at += n;
        }
        Ok(out)
    })
}

fn bytes_value(bytes: &[u8]) -> Value {
    Value::array(bytes.iter().map(|&b| Value::Byte(b as i8)).collect())
}

/// Up to `max` bytes as text, UTF-8 decoded as the JDK's decoder decodes, a sequence cut by the
/// chunk's end kept for the next read; null at the end. A malformed sequence is replaced, as
/// `InputStreamReader` replaces it, or with `report` (`Files.newBufferedReader`'s decoder) is the
/// JDK's `MalformedInputException` with its length.
fn read_text(it: &mut Interp, id: i32, max: usize, report: bool) -> R {
    with_stream(it, id, |it, s| {
        if s.closed {
            return refused(it, s);
        }
        if !fill(it, s)? {
            return Ok(Value::Null);
        }
        // As `StreamDecoder` with the JDK's UTF-8 decoder: what is decided now is decoded now, a
        // malformed sequence too; a sequence that more bytes can still complete waits for them,
        // read while it is all there is, until it is whole, malformed or at the end.
        let limit = |s: &Stream| (s.at + max.max(4)).min(s.read.len());
        let mut end = limit(s);
        let mut wait = super::natives::utf8_underflow(&s.read[s.at..end]);
        while wait > 0 && end - wait == s.at && read_more(it, s)? {
            end = limit(s);
            wait = super::natives::utf8_underflow(&s.read[s.at..end]);
        }
        if end - wait > s.at {
            end -= wait;
        }
        if report {
            if let Some(length) = super::natives::jdk_utf8_malformed(&s.read[s.at..end]) {
                return it.throw_new(&["java", "nio", "charset", "MalformedInputException"], vec![Value::Int(length as i32)]);
            }
        }
        let text = super::natives::jdk_utf8_decode(&s.read[s.at..end]);
        s.at = end;
        Ok(Value::string(text))
    })
}

fn available(it: &mut Interp, id: i32) -> R {
    with_stream(it, id, |it, s| {
        if s.closed {
            return refused(it, s);
        }
        if s.input_shut {
            return Ok(Value::Int(0));
        }
        let buffered = s.read.len() - s.at;
        Ok(Value::Int((buffered + s.pending()).min(i32::MAX as usize) as i32))
    })
}

/// `skip(n)` as the stream's JDK class skips (`Skip`); nothing read into memory past the buffer.
fn skip(it: &mut Interp, id: i32, n: i64) -> R {
    with_stream(it, id, |it, s| {
        if s.closed {
            return refused(it, s);
        }
        let buffered = (s.read.len() - s.at) as i64;
        match (s.skip, &mut s.io) {
            (Skip::Seek | Skip::Channel, Io::File(f)) => {
                use std::io::{Seek, SeekFrom};
                // Where the program stands: the file's position less what the buffer holds.
                let at = match f.stream_position() {
                    Ok(p) => p as i64 - buffered,
                    Err(e) => return io_error(it, &reason(&e)),
                };
                let size = f.metadata().map_or(0, |m| m.len() as i64);
                let to = match s.skip {
                    Skip::Channel if n > 0 => match at.saturating_add(n) {
                        to if to > size => size,
                        to => to,
                    },
                    Skip::Channel => (at + n).max(0),
                    _ => at + n,
                };
                if to < 0 {
                    return io_error(it, "Invalid argument");
                }
                if let Err(e) = f.seek(SeekFrom::Start(to as u64)) {
                    return io_error(it, &reason(&e));
                }
                s.read.clear();
                s.at = 0;
                Ok(Value::Long(to - at))
            }
            _ if n <= 0 => Ok(Value::Long(0)),
            (Skip::Buffered, _) if buffered > 0 => {
                let k = n.min(buffered);
                s.at += k as usize;
                Ok(Value::Long(k))
            }
            _ => {
                // Read past, a buffer's worth at a time, to `n` or the end.
                let mut left = n;
                while left > 0 && fill(it, s)? {
                    let k = left.min((s.read.len() - s.at) as i64);
                    s.at += k as usize;
                    left -= k;
                }
                Ok(Value::Long(n - left))
            }
        }
    })
}

/// Writes the buffer out: to a child that ended, the JDK's `Stream closed`; to a socket, in parts,
/// each wait for the peer to take more bounded so that a signal is seen.
fn flush_stream(it: &mut Interp, s: &mut Stream) -> R<()> {
    if s.write.is_empty() {
        return Ok(());
    }
    let bytes = std::mem::take(&mut s.write);
    let r = match &mut s.io {
        Io::File(f) => os::write_all(f, &bytes, &mut || SIGNAL.load(Ordering::Relaxed) > 0),
        Io::Pipe(p) => match &mut locked(p).file {
            Some(f) => os::write_all(f, &bytes, &mut || SIGNAL.load(Ordering::Relaxed) > 0),
            None => return stream_closed(it),
        },
        Io::Socket(sock) => {
            it.flush();
            let _ = sock.set_write_timeout(Some(SLICE));
            let mut at = 0;
            let mut r = Ok(());
            while at < bytes.len() {
                match sock.write(&bytes[at..]) {
                    Ok(0) => {
                        r = Err(std::io::Error::from(std::io::ErrorKind::WriteZero));
                        break;
                    }
                    Ok(n) => at += n,
                    Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted) => check_signal(it)?,
                    Err(e) => {
                        r = Err(e);
                        break;
                    }
                }
            }
            r
        }
        Io::Stdin | Io::Closed => Ok(()),
    };
    check_signal(it)?;
    match r {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe && s.child.is_some() => stream_closed(it),
        Err(e) => io_error(it, &reason(&e)),
    }
}

fn write_bytes(it: &mut Interp, id: i32, bytes: &[u8]) -> R {
    with_stream(it, id, |it, s| {
        if s.closed {
            return refused(it, s);
        }
        if !s.buffered {
            s.write.extend_from_slice(bytes);
            return flush_stream(it, s).map(|()| Value::Unit);
        }
        if s.write.len() + bytes.len() > BUFFER {
            flush_stream(it, s)?;
        }
        s.write.extend_from_slice(bytes);
        if s.write.len() >= BUFFER {
            flush_stream(it, s)?;
        }
        Ok(Value::Unit)
    })
}

/// Closes the stream: what its buffer holds written, its descriptor (or its share of a child's
/// pipe) let go.
fn close_stream(it: &mut Interp, id: i32) -> R {
    with_stream(it, id, |it, s| {
        if s.closed {
            return Ok(Value::Unit);
        }
        let flushed = flush_stream(it, s);
        s.closed = true;
        s.read = Vec::new();
        s.write = Vec::new();
        if let Io::Pipe(p) = &s.io {
            locked(p).file = None;
        }
        s.io = Io::Closed;
        flushed.map(|()| Value::Unit)
    })
}

// ---- files as streams ----

/// `Files.newInputStream` and `FileInputStream`: the file's stream, or the JDK's exception (for
/// `FileInputStream`, `FileNotFoundException` with `<file> (<reason>)`).
fn open_read(it: &mut Interp, path: &str, plain_io: bool) -> R {
    match std::fs::File::open(if path.is_empty() { "." } else { path }) {
        Ok(f) if f.metadata().is_ok_and(|m| m.is_dir()) => {
            if plain_io {
                it.throw_new(&["java", "io", "FileNotFoundException"], vec![Value::string(format!("{} (Is a directory)", path))])
            } else {
                Ok(Value::Int(new_stream_refusing(Io::File(f), None, false, Skip::Read, Refusal::Channel)))
            }
        }
        Ok(f) => {
            let regular = f.metadata().is_ok_and(|m| m.is_file());
            let skip = match (regular, plain_io) {
                (true, true) => Skip::Seek,
                (true, false) => Skip::Channel,
                _ => Skip::Read,
            };
            Ok(Value::Int(new_stream_refusing(Io::File(f), None, false, skip, if plain_io { Refusal::Plain } else { Refusal::Channel })))
        }
        Err(e) if plain_io => it.throw_new(&["java", "io", "FileNotFoundException"], vec![Value::string(format!("{} ({})", path, reason(&e)))]),
        Err(e) => super::files::fail(it, &e, path),
    }
}

/// The open options of a write, as flags: `StandardOpenOption`'s CREATE, CREATE_NEW, APPEND,
/// TRUNCATE_EXISTING (`std/javalib/nio.scala` checks them as the JDK's
/// `Files.newOutputStream` does).
const OPEN_CREATE: i32 = 1;
const OPEN_CREATE_NEW: i32 = 2;
const OPEN_APPEND: i32 = 4;
const OPEN_TRUNCATE: i32 = 8;

/// `Files.newOutputStream` and `FileOutputStream`: the file opened for writing by its flags, as
/// `UnixChannelFactory` opens it: CREATE_NEW the file must not be there, else CREATE may make it,
/// APPEND writes at its end, TRUNCATE_EXISTING empties it.
fn open_write(it: &mut Interp, path: &str, flags: i32, plain_io: bool) -> R {
    let mut o = std::fs::OpenOptions::new();
    o.write(true);
    if flags & OPEN_CREATE_NEW != 0 {
        o.create_new(true);
    } else if flags & OPEN_CREATE != 0 {
        o.create(true);
    }
    if flags & OPEN_APPEND != 0 {
        o.append(true);
    }
    if flags & OPEN_TRUNCATE != 0 && flags & OPEN_CREATE_NEW == 0 {
        o.truncate(true);
    }
    match o.open(path) {
        Ok(f) => Ok(Value::Int(new_stream_refusing(Io::File(f), None, false, Skip::Read, if plain_io { Refusal::Plain } else { Refusal::Channel }))),
        Err(e) if plain_io => it.throw_new(&["java", "io", "FileNotFoundException"], vec![Value::string(format!("{} ({})", path, reason(&e)))]),
        Err(e) => super::files::fail(it, &e, path),
    }
}
// ---- the process table ----

/// The processes of the system: (pid, parent pid) for each.
fn table() -> Vec<(u32, u32)> {
    os::processes()
}

// ---- natives ----

fn int(it: &mut Interp, a: A, i: usize) -> R<i32> {
    match a.get(i).and_then(|v| v.as_i32()) {
        Some(v) => Ok(v),
        None => it.unsupported(format!("argument {} is no Int", i)),
    }
}

fn long(it: &mut Interp, a: A, i: usize) -> R<i64> {
    match a.get(i).and_then(|v| v.as_i64()) {
        Some(v) => Ok(v),
        None => it.unsupported(format!("argument {} is no Long", i)),
    }
}

fn strings(it: &mut Interp, a: A, i: usize) -> R<Option<Vec<String>>> {
    match a.get(i) {
        Some(Value::Array(items)) => {
            let mut out = Vec::new();
            for v in items.borrow().iter() {
                match v {
                    Value::Str(s) => out.push(s.to_string()),
                    _ => return it.throw_named("NullPointerException", ""),
                }
            }
            Ok(Some(out))
        }
        Some(Value::Null) | None => Ok(None),
        Some(_) => it.unsupported("an argument that is no array of strings"),
    }
}

fn bytes_arg(it: &mut Interp, a: A, i: usize) -> R<Rc<ArrayCell>> {
    match a.get(i) {
        Some(Value::Array(items)) => Ok(items.clone()),
        _ => it.throw_named("NullPointerException", ""),
    }
}

fn child_index(it: &mut Interp, a: A, i: usize) -> R<usize> {
    let n = long(it, a, i)?;
    let known = CHILDREN.with(|c| (n as usize) < c.borrow().len());
    if n < 0 || !known {
        return it.unsupported(format!("no child {}", n));
    }
    Ok(n as usize)
}

/// A system error's message as the JDK gives it: `error=<n>, <reason>` on Unix
/// (`ProcessImpl`'s), `CreateProcess error=<n>, <reason>` on Windows.
fn os_message(e: &std::io::Error) -> String {
    match e.raw_os_error() {
        Some(n) if cfg!(windows) => format!("CreateProcess error={}, {}", n, reason(e)),
        Some(n) => format!("error={}, {}", n, reason(e)),
        None => reason(e),
    }
}

/// The system's text for an error, without Rust's `(os error n)`.
fn reason(e: &std::io::Error) -> String {
    let text = e.to_string();
    match text.rfind(" (os error ") {
        Some(at) if text.ends_with(')') => text[..at].to_string(),
        _ => text,
    }
}

pub(super) fn io_error<T>(it: &mut Interp, message: &str) -> R<T> {
    it.throw_new(&["java", "io", "IOException"], vec![Value::str(message), Value::Null])
}

pub(super) fn install(it: &mut Table) {
    reg!(it, "java.lang.processStart", |it, a| {
        let Some(command) = strings(it, a, 0)? else { return it.throw_named("NullPointerException", "") };
        let dir = match a.get(1) {
            Some(Value::Str(d)) => Some(d.to_string()),
            _ => None,
        };
        let env = strings(it, a, 2)?.map(|flat| flat.chunks(2).map(|kv| (kv[0].clone(), kv.get(1).cloned().unwrap_or_default())).collect());
        let codes = strings(it, a, 3)?.unwrap_or_default();
        let code = |i: usize| redirect(codes.get(i).map_or("pipe", String::as_str));
        let merge = matches!(a.get(4), Some(Value::Bool(true)));
        start(it, command, dir, env, [code(0), code(1), code(2)], merge)
    });
    // The status once the child ended, `Long.MinValue` while it runs.
    reg!(it, "java.lang.processPoll", |it, a| {
        let child = child_index(it, a, 0)?;
        Ok(Value::Long(status_of(child).map_or(i64::MIN, i64::from)))
    });
    // Waits for the child, at most `millis` (none when negative): its status, or `Long.MinValue`.
    reg!(it, "java.lang.processWait", |it, a| {
        let child = child_index(it, a, 0)?;
        let nanos = long(it, a, 1)?;
        let deadline = (nanos >= 0).then(|| Instant::now() + Duration::from_nanos(nanos as u64));
        Ok(Value::Long(wait_child(it, child, deadline)?.map_or(i64::MIN, i64::from)))
    });
    reg!(it, "java.lang.processEnd", |it, a| {
        let child = child_index(it, a, 0)?;
        if it.pure {
            return it.impure("ending a process");
        }
        end_child(child, matches!(a.get(1), Some(Value::Bool(true))));
        Ok(Value::Unit)
    });
    reg!(it, "java.lang.processCurrent", |_it, _a| Ok(Value::Long(std::process::id() as i64)));
    // Whether the process is there, a zombie child of another counting as there, as the JDK's.
    reg!(it, "java.lang.processAlive", |it, a| {
        let pid = long(it, a, 0)?;
        if it.pure {
            return it.impure("the process table");
        }
        Ok(Value::Bool(pid > 0 && pid <= u32::MAX as i64 && os::alive(pid as u32)))
    });
    reg!(it, "java.lang.processSignal", |it, a| {
        let pid = long(it, a, 0)?;
        if it.pure {
            return it.impure("ending a process");
        }
        let force = matches!(a.get(1), Some(Value::Bool(true)));
        Ok(Value::Bool(pid > 0 && pid <= u32::MAX as i64 && os::end_pid(pid as u32, force)))
    });
    // The process table, a pid and its parent's alternating.
    reg!(it, "java.lang.processTable", |it, _a| {
        if it.pure {
            return it.impure("the process table");
        }
        Ok(Value::array(table().into_iter().flat_map(|(p, pp)| [Value::Long(p as i64), Value::Long(pp as i64)]).collect()))
    });
    // The executable and the arguments of a process as the system tells them: the executable's
    // path (null when it cannot be read), then the arguments past argv[0] (none when they cannot).
    reg!(it, "java.lang.processInfo", |it, a| {
        let pid = long(it, a, 0)?;
        if it.pure {
            return it.impure("the process table");
        }
        let (command, args) = os::info(pid as u32);
        let mut out = vec![command.map_or(Value::Null, Value::string)];
        match args {
            Some(args) => {
                out.push(Value::Bool(true));
                out.extend(args.into_iter().map(Value::string));
            }
            None => out.push(Value::Bool(false)),
        }
        Ok(Value::array(out))
    });
    reg!(it, "java.lang.hookAdd", |it, a| {
        if it.pure {
            return it.impure("a shutdown hook");
        }
        let hook = a.first().cloned().unwrap_or(Value::Null);
        let refused = HOOKS.with(|h| match h.borrow_mut().as_mut() {
            None => Some("IllegalStateException"),
            Some(hooks) if hooks.iter().any(|x| same(x, &hook)) => Some("IllegalArgumentException"),
            Some(hooks) => {
                hooks.push(hook.clone());
                None
            }
        });
        match refused {
            Some("IllegalStateException") => it.throw_named("IllegalStateException", "Shutdown in progress"),
            Some(_) => it.throw_named("IllegalArgumentException", "Hook previously registered"),
            None => Ok(Value::Unit),
        }
    });
    reg!(it, "java.lang.hookRemove", |it, a| {
        let hook = a.first().cloned().unwrap_or(Value::Null);
        let removed = HOOKS.with(|h| match h.borrow_mut().as_mut() {
            None => None,
            Some(hooks) => Some(hooks.iter().position(|x| same(x, &hook)).map(|at| hooks.remove(at)).is_some()),
        });
        match removed {
            None => it.throw_named("IllegalStateException", "Shutdown in progress"),
            Some(r) => Ok(Value::Bool(r)),
        }
    });
    reg!(it, "java.lang.threadSleep", |it, a| {
        let millis = long(it, a, 0)?;
        let nanos = long(it, a, 1)?;
        sleep(it, millis, nanos)?;
        Ok(Value::Unit)
    });
    // The machine's processors under `teq interp`, as the JVM counts them; one in a macro, whose
    // expansion may not depend on the machine, and none to a folded constant.
    reg!(it, "java.lang.Runtime.availableProcessors", |it, _a| {
        if it.pure && it.macro_ctx.is_none() {
            return it.impure("the machine's processors");
        }
        if it.macro_ctx.is_some() {
            return Ok(Value::Int(1));
        }
        Ok(Value::Int(std::thread::available_parallelism().map_or(1, |n| n.get()) as i32))
    });
    // `System.in`: the one stream over the interpreter's stdin (`java.io.Stdin`).
    reg!(it, "java.lang.System.in", |it, _a| {
        if it.pure {
            return it.impure("reading stdin");
        }
        let Some(c) = it.typer.class_at(&["java", "io", "Stdin"]) else { return it.unsupported("java.io.Stdin is not in the std") };
        let module = it.module(c)?;
        it.call_by_name(module, "stream", Vec::new())
    });
    reg!(it, "java.io.streamRead", |it, a| {
        let id = int(it, a, 0)?;
        read_byte(it, id)
    });
    reg!(it, "java.io.streamReadInto", |it, a| {
        let id = int(it, a, 0)?;
        let b = bytes_arg(it, a, 1)?;
        let (off, len) = (int(it, a, 2)?, int(it, a, 3)?);
        if off < 0 || len < 0 || off as usize + len as usize > b.borrow().len() {
            return it.throw_named("IndexOutOfBoundsException", &format!("Range [{}, {} + {}) out of bounds for length {}", off, off, len, b.borrow().len()));
        }
        read_into(it, id, &b, off as usize, len as usize)
    });
    reg!(it, "java.io.streamReadAll", |it, a| {
        let id = int(it, a, 0)?;
        let max = int(it, a, 1)?;
        if max < 0 {
            return it.throw_named("IllegalArgumentException", "len < 0");
        }
        let bytes = read_up_to(it, id, max as usize)?;
        Ok(bytes_value(&bytes))
    });
    reg!(it, "java.io.streamReadText", |it, a| {
        let id = int(it, a, 0)?;
        let max = int(it, a, 1)?;
        let report = matches!(a.get(2), Some(Value::Bool(true)));
        read_text(it, id, max.max(1) as usize, report)
    });
    reg!(it, "java.io.streamAvailable", |it, a| {
        let id = int(it, a, 0)?;
        available(it, id)
    });
    reg!(it, "java.io.streamSkip", |it, a| {
        let id = int(it, a, 0)?;
        let n = long(it, a, 1)?;
        skip(it, id, n)
    });
    reg!(it, "java.io.streamWrite", |it, a| {
        let id = int(it, a, 0)?;
        let b = int(it, a, 1)?;
        write_bytes(it, id, &[b as u8])
    });
    reg!(it, "java.io.streamWriteFrom", |it, a| {
        let id = int(it, a, 0)?;
        let b = bytes_arg(it, a, 1)?;
        let (off, len) = (int(it, a, 2)?, int(it, a, 3)?);
        let items = b.borrow();
        if off < 0 || len < 0 || off as usize + len as usize > items.len() {
            let n = items.len();
            drop(items);
            return it.throw_named("IndexOutOfBoundsException", &format!("Range [{}, {} + {}) out of bounds for length {}", off, off, len, n));
        }
        let bytes: Vec<u8> = items[off as usize..(off + len) as usize].iter().map(|v| v.as_i64().unwrap_or(0) as u8).collect();
        drop(items);
        write_bytes(it, id, &bytes)
    });
    reg!(it, "java.io.streamWriteText", |it, a| {
        let id = int(it, a, 0)?;
        let text = it.str_arg(a, 1)?;
        write_bytes(it, id, text.as_bytes())
    });
    reg!(it, "java.io.streamFlush", |it, a| {
        let id = int(it, a, 0)?;
        with_stream(it, id, |it, s| {
            if s.closed {
                return refused(it, s);
            }
            flush_stream(it, s).map(|()| Value::Unit)
        })
    });
    reg!(it, "java.io.streamClose", |it, a| {
        let id = int(it, a, 0)?;
        close_stream(it, id)
    });
    reg!(it, "java.io.streamStdin", |it, _a| {
        if it.pure {
            return it.impure("reading stdin");
        }
        thread_local! { static STDIN: std::cell::Cell<i32> = const { std::cell::Cell::new(-1) }; }
        let id = STDIN.with(|s| {
            if s.get() < 0 {
                s.set(new_stream(Io::Stdin, None, false, Skip::Buffered));
            }
            s.get()
        });
        Ok(Value::Int(id))
    });
    reg!(it, "java.io.streamOpenRead", |it, a| {
        if it.pure {
            return it.impure("reading a file");
        }
        let path = it.str_arg(a, 0)?;
        let plain = matches!(a.get(1), Some(Value::Bool(true)));
        open_read(it, &path, plain)
    });
    reg!(it, "java.io.streamOpenWrite", |it, a| {
        if it.pure {
            return it.impure("writing a file");
        }
        let path = it.str_arg(a, 0)?;
        let flags = int(it, a, 1)?;
        let plain = matches!(a.get(2), Some(Value::Bool(true)));
        open_write(it, &path, flags, plain)
    });
    super::net::install(it);
    super::archive::install(it);
}

/// The system's name as the JDK's `os.name` gives it: `Linux`, `Mac OS X`, `Windows 10` or
/// `Windows 11` by the build, else the kernel's name.
pub(super) fn os_name() -> String {
    os::name()
}

/// A socket's stream, for `java.net`'s natives (`net.rs`).
pub(super) fn socket_stream(sock: std::net::TcpStream) -> i32 {
    new_stream(Io::Socket(sock), None, false, Skip::Read)
}

/// Shuts a socket stream's connection for writes (`shutdownOutput`) or reads.
/// `shutdownInput` or `shutdownOutput` of a socket by its stream of that side: the system's
/// shutdown, and for the input the stream's state as `NioSocketImpl.shutdownInput` leaves it.
pub(super) fn shutdown_stream(id: i32, write: bool) {
    STREAMS.with(|s| {
        if let Some(Some(stream)) = s.borrow_mut().get_mut(id as usize) {
            if let Io::Socket(sock) = &stream.io {
                let _ = sock.shutdown(if write { std::net::Shutdown::Write } else { std::net::Shutdown::Read });
                if !write {
                    stream.input_shut = true;
                    stream.read.clear();
                    stream.at = 0;
                }
            }
        }
    });
}

/// Sets a socket stream's read timeout, none for zero.
pub(super) fn set_stream_timeout(id: i32, millis: i32) {
    STREAMS.with(|s| {
        if let Some(Some(stream)) = s.borrow_mut().get_mut(id as usize) {
            stream.timeout = (millis > 0).then(|| Duration::from_millis(millis as u64));
        }
    });
}

pub(super) fn signal_seen(it: &mut Interp) -> R<()> {
    check_signal(it)
}


#[cfg(unix)]
mod os {
    use std::os::fd::AsRawFd;
    use std::os::raw::{c_int, c_short, c_ulong, c_void};
    use std::time::Duration;

    pub const SIGTERM: c_int = 15;
    pub const SIGKILL: c_int = 9;
    const SIGHUP: c_int = 1;
    const SIGINT: c_int = 2;
    const P_PID: c_int = 1;
    const WEXITED: c_int = 4;
    #[cfg(target_os = "linux")]
    const WNOWAIT: c_int = 0x0100_0000;
    #[cfg(not(target_os = "linux"))]
    const WNOWAIT: c_int = 0x20;
    const EINTR: i32 = 4;
    const EPERM: i32 = 1;
    const POLLIN: c_short = 1;
    const POLLOUT: c_short = 4;
    #[cfg(target_os = "linux")]
    const FIONREAD: c_ulong = 0x541b;
    #[cfg(not(target_os = "linux"))]
    const FIONREAD: c_ulong = 0x4004_667f;
    #[cfg(target_os = "linux")]
    type NFds = c_ulong;
    #[cfg(not(target_os = "linux"))]
    type NFds = std::os::raw::c_uint;

    #[repr(C)]
    struct PollFd {
        fd: c_int,
        events: c_short,
        revents: c_short,
    }

    extern "C" {
        fn kill(pid: c_int, sig: c_int) -> c_int;
        fn signal(sig: c_int, handler: extern "C" fn(c_int)) -> usize;
        fn waitid(idtype: c_int, id: u32, infop: *mut c_void, options: c_int) -> c_int;
        fn poll(fds: *mut PollFd, nfds: NFds, timeout: c_int) -> c_int;
        fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
        fn _exit(code: c_int) -> !;
    }

    extern "C" fn on_signal(sig: c_int) {
        super::on_signal(sig);
    }

    pub fn handle_signals() {
        // SAFETY: installs a handler that stores the signal or ends the process.
        unsafe {
            signal(SIGINT, on_signal);
            signal(SIGTERM, on_signal);
            signal(SIGHUP, on_signal);
        }
    }

    pub fn exit_now(code: i32) -> ! {
        // SAFETY: ends the process; nothing runs after.
        unsafe { _exit(code) }
    }

    pub fn signal_pid(pid: u32, sig: c_int) {
        // SAFETY: a plain signal to a child of this process, not yet reaped.
        unsafe {
            kill(pid as c_int, sig);
        }
    }

    /// Waits until the child has ended, leaving it to be reaped.
    pub fn wait_exited(pid: u32) {
        let mut info = [0u64; 32];
        loop {
            // SAFETY: the buffer is larger than any platform's `siginfo_t`.
            let r = unsafe { waitid(P_PID, pid, info.as_mut_ptr() as *mut c_void, WEXITED | WNOWAIT) };
            if r == 0 || std::io::Error::last_os_error().raw_os_error() != Some(EINTR) {
                return;
            }
        }
    }

    pub fn alive(pid: u32) -> bool {
        // SAFETY: signal 0 tests the process alone.
        let r = unsafe { kill(pid as c_int, 0) };
        r == 0 || std::io::Error::last_os_error().raw_os_error() == Some(EPERM)
    }

    pub fn end_pid(pid: u32, force: bool) -> bool {
        // SAFETY: a plain signal, as `ProcessHandle.destroy` sends it.
        unsafe { kill(pid as c_int, if force { SIGKILL } else { SIGTERM }) == 0 }
    }

    pub fn on_path(program: &str) -> Option<std::path::PathBuf> {
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path).map(|d| d.join(program)).find(|p| {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
    }

    fn wait_fd(fd: c_int, events: c_short, timeout: Duration) -> bool {
        let mut p = PollFd { fd, events, revents: 0 };
        // SAFETY: one valid pollfd.
        let r = unsafe { poll(&mut p, 1, timeout.as_millis().min(i32::MAX as u128) as c_int) };
        r != 0
    }

    pub fn wait_readable(f: &std::fs::File, timeout: Duration) -> bool {
        wait_fd(f.as_raw_fd(), POLLIN, timeout)
    }

    pub fn wait_stdin(timeout: Duration) -> bool {
        wait_fd(0, POLLIN, timeout)
    }

    fn fd_pending(fd: c_int) -> usize {
        let mut n: c_int = 0;
        // SAFETY: FIONREAD writes one int.
        let r = unsafe { ioctl(fd, FIONREAD, &mut n as *mut c_int) };
        if r == 0 && n > 0 {
            n as usize
        } else {
            0
        }
    }

    /// What a read would give without blocking: a pipe's or a terminal's queue, a file's rest.
    pub fn pending(f: &std::fs::File) -> usize {
        match f.metadata() {
            Ok(m) if m.is_file() => {
                use std::io::Seek;
                let at = (&*f).stream_position().unwrap_or(m.len());
                m.len().saturating_sub(at) as usize
            }
            _ => fd_pending(f.as_raw_fd()),
        }
    }

    pub fn stdin_pending() -> usize {
        fd_pending(0)
    }

    /// A read of the process's stdin through its descriptor, below Rust's own buffer of it.
    pub fn read_stdin(out: &mut [u8]) -> std::io::Result<usize> {
        use std::io::Read;
        use std::os::fd::FromRawFd;
        // SAFETY: descriptor 0 stays open; the file is never dropped.
        let mut f = std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(0) });
        f.read(out)
    }

    pub fn socket_pending(s: &std::net::TcpStream) -> usize {
        fd_pending(s.as_raw_fd())
    }

    pub fn name() -> String {
        if cfg!(target_os = "macos") {
            "Mac OS X".to_string()
        } else if cfg!(target_os = "linux") {
            "Linux".to_string()
        } else {
            let mut s = std::env::consts::OS.to_string();
            if let Some(c) = s.get_mut(0..1) {
                c.make_ascii_uppercase();
            }
            s
        }
    }

    /// Writes all of `bytes`, waiting while the pipe is full and seeing a signal between waits.
    pub fn write_all(f: &mut std::fs::File, bytes: &[u8], signalled: &mut dyn FnMut() -> bool) -> std::io::Result<()> {
        use std::io::Write;
        let fd = f.as_raw_fd();
        let mut at = 0;
        while at < bytes.len() {
            if !wait_fd(fd, POLLOUT, super::SLICE) {
                if signalled() {
                    return Ok(());
                }
                continue;
            }
            // At most what a pipe takes at once when it says it is writable.
            let end = (at + 4096).min(bytes.len());
            match f.write(&bytes[at..end]) {
                Ok(n) => at += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Every process: from `/proc` on Linux, from the kernel's list elsewhere.
    #[cfg(target_os = "linux")]
    pub fn processes() -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir("/proc") else { return out };
        for e in entries.flatten() {
            let Some(pid) = e.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else { continue };
            let Ok(stat) = std::fs::read_to_string(format!("/proc/{}/stat", pid)) else { continue };
            // `pid (comm) state ppid ...`, the name possibly holding spaces and parentheses.
            let Some(close) = stat.rfind(')') else { continue };
            let mut fields = stat[close + 1..].split_whitespace();
            let (Some(_state), Some(ppid)) = (fields.next(), fields.next().and_then(|p| p.parse::<u32>().ok())) else { continue };
            out.push((pid, ppid));
        }
        out
    }

    #[cfg(target_os = "linux")]
    pub fn info(pid: u32) -> (Option<String>, Option<Vec<String>>) {
        let command = std::fs::read_link(format!("/proc/{}/exe", pid)).ok().map(|p| p.to_string_lossy().into_owned());
        let args = std::fs::read(format!("/proc/{}/cmdline", pid)).ok().filter(|b| !b.is_empty()).map(|b| {
            let mut parts: Vec<String> = b.split(|&c| c == 0).map(|p| String::from_utf8_lossy(p).into_owned()).collect();
            if parts.last().is_some_and(String::is_empty) {
                parts.pop();
            }
            parts.into_iter().skip(1).collect()
        });
        (command, args)
    }

    #[cfg(target_os = "macos")]
    extern "C" {
        fn proc_listallpids(buffer: *mut c_void, buffersize: c_int) -> c_int;
        fn proc_pidinfo(pid: c_int, flavor: c_int, arg: u64, buffer: *mut c_void, buffersize: c_int) -> c_int;
        fn proc_pidpath(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;
        fn sysctl(name: *mut c_int, namelen: u32, oldp: *mut c_void, oldlenp: *mut usize, newp: *mut c_void, newlen: usize) -> c_int;
    }

    #[cfg(target_os = "macos")]
    pub fn processes() -> Vec<(u32, u32)> {
        const PROC_PIDTBSDINFO: c_int = 3;
        // SAFETY: a null buffer asks for the count.
        let n = unsafe { proc_listallpids(std::ptr::null_mut(), 0) };
        if n <= 0 {
            return Vec::new();
        }
        let mut pids = vec![0 as c_int; n as usize + 64];
        // SAFETY: the buffer holds as many pids as its size in bytes says.
        let n = unsafe { proc_listallpids(pids.as_mut_ptr() as *mut c_void, (pids.len() * 4) as c_int) };
        let mut out = Vec::new();
        for &pid in pids.iter().take(n.max(0) as usize) {
            // `struct proc_bsdinfo`, 136 bytes, its parent's pid at 16.
            let mut info = [0u8; 136];
            // SAFETY: the buffer is the structure's size.
            let r = unsafe { proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, info.as_mut_ptr() as *mut c_void, info.len() as c_int) };
            if r as usize == info.len() {
                let ppid = u32::from_ne_bytes([info[16], info[17], info[18], info[19]]);
                out.push((pid as u32, ppid));
            }
        }
        out
    }

    #[cfg(target_os = "macos")]
    pub fn info(pid: u32) -> (Option<String>, Option<Vec<String>>) {
        let mut path = [0u8; 4096];
        // SAFETY: the buffer's size is given.
        let n = unsafe { proc_pidpath(pid as c_int, path.as_mut_ptr() as *mut c_void, path.len() as u32) };
        let command = (n > 0).then(|| String::from_utf8_lossy(&path[..n as usize]).into_owned());
        // KERN_PROCARGS2: argc, the executable's path, padding, then argv.
        let mut mib = [1 as c_int, 49, pid as c_int];
        let mut size: usize = 0;
        // SAFETY: a null buffer asks for the size.
        if unsafe { sysctl(mib.as_mut_ptr(), 3, std::ptr::null_mut(), &mut size, std::ptr::null_mut(), 0) } != 0 || size < 4 {
            return (command, None);
        }
        let mut buf = vec![0u8; size];
        // SAFETY: the buffer has the size the kernel gave.
        if unsafe { sysctl(mib.as_mut_ptr(), 3, buf.as_mut_ptr() as *mut c_void, &mut size, std::ptr::null_mut(), 0) } != 0 {
            return (command, None);
        }
        buf.truncate(size);
        let argc = i32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]).max(0) as usize;
        let mut at = 4;
        while at < buf.len() && buf[at] != 0 {
            at += 1;
        }
        while at < buf.len() && buf[at] == 0 {
            at += 1;
        }
        let mut argv = Vec::new();
        while argv.len() < argc && at < buf.len() {
            let end = buf[at..].iter().position(|&c| c == 0).map_or(buf.len(), |e| at + e);
            argv.push(String::from_utf8_lossy(&buf[at..end]).into_owned());
            at = end + 1;
        }
        (command, Some(argv.into_iter().skip(1).collect()))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    pub fn processes() -> Vec<(u32, u32)> {
        Vec::new()
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    pub fn info(_pid: u32) -> (Option<String>, Option<Vec<String>>) {
        (None, None)
    }
}

#[cfg(windows)]
mod os {
    use std::os::windows::io::AsRawHandle;
    use std::time::Duration;

    type Handle = *mut std::ffi::c_void;
    const PROCESS_TERMINATE: u32 = 0x0001;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const WAIT_TIMEOUT: u32 = 0x102;
    const TH32CS_SNAPPROCESS: u32 = 2;
    const STD_INPUT_HANDLE: u32 = -10i32 as u32;
    const FILE_TYPE_DISK: u32 = 1;

    #[repr(C)]
    struct ProcessEntry {
        size: u32,
        usage: u32,
        pid: u32,
        heap: usize,
        module: u32,
        threads: u32,
        parent: u32,
        priority: i32,
        flags: u32,
        exe: [u16; 260],
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn TerminateProcess(process: Handle, code: u32) -> i32;
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> Handle;
        fn CloseHandle(h: Handle) -> i32;
        fn WaitForSingleObject(h: Handle, millis: u32) -> u32;
        fn PeekNamedPipe(pipe: Handle, buffer: *mut std::ffi::c_void, size: u32, read: *mut u32, avail: *mut u32, left: *mut u32) -> i32;
        fn GetFileType(h: Handle) -> u32;
        fn SetConsoleCtrlHandler(handler: Option<unsafe extern "system" fn(u32) -> i32>, add: i32) -> i32;
        fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> Handle;
        fn Process32FirstW(snapshot: Handle, entry: *mut ProcessEntry) -> i32;
        fn Process32NextW(snapshot: Handle, entry: *mut ProcessEntry) -> i32;
        fn QueryFullProcessImageNameW(process: Handle, flags: u32, name: *mut u16, size: *mut u32) -> i32;
        fn GetStdHandle(which: u32) -> Handle;
        fn ExitProcess(code: u32) -> !;
    }

    unsafe extern "system" fn on_event(event: u32) -> i32 {
        // Ctrl-C, Ctrl-Break, the console closed, the session's end: the JVM's INT and TERM.
        match event {
            0 | 1 => super::on_signal(2),
            _ => super::on_signal(15),
        }
        1
    }

    pub fn handle_signals() {
        // SAFETY: registers a handler that stores the event or ends the process.
        unsafe {
            SetConsoleCtrlHandler(Some(on_event), 1);
        }
    }

    pub fn exit_now(code: i32) -> ! {
        // SAFETY: ends the process.
        unsafe { ExitProcess(code as u32) }
    }

    pub fn terminate(h: isize) {
        // SAFETY: the handle is the child's, open while its reaper holds it.
        unsafe {
            TerminateProcess(h as Handle, 1);
        }
    }

    pub fn alive(pid: u32) -> bool {
        // SAFETY: a handle opened and closed here.
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE, 0, pid);
            if h.is_null() {
                return false;
            }
            let r = WaitForSingleObject(h, 0) == WAIT_TIMEOUT;
            CloseHandle(h);
            r
        }
    }

    pub fn end_pid(pid: u32, _force: bool) -> bool {
        // SAFETY: a handle opened and closed here.
        unsafe {
            let h = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if h.is_null() {
                return false;
            }
            let r = TerminateProcess(h, 1) != 0;
            CloseHandle(h);
            r
        }
    }

    fn handle_pending(h: Handle) -> usize {
        let mut avail = 0u32;
        // SAFETY: a peek that copies nothing.
        let ok = unsafe { PeekNamedPipe(h, std::ptr::null_mut(), 0, std::ptr::null_mut(), &mut avail, std::ptr::null_mut()) };
        if ok != 0 {
            avail as usize
        } else {
            0
        }
    }

    /// By the handle's type, as the JDK's `handleAvailable` (`io_util_md.c`): a disk file's size past
    /// its position, a pipe's or a console's queue as `PeekNamedPipe` sees it (0 at a pipe's end).
    /// `File::metadata` cannot tell: Windows' `is_file` is any handle that is no directory, a pipe too.
    pub fn pending(f: &std::fs::File) -> usize {
        let h = f.as_raw_handle() as Handle;
        // SAFETY: the file's own handle.
        if unsafe { GetFileType(h) } != FILE_TYPE_DISK {
            return handle_pending(h);
        }
        let len = f.metadata().map_or(0, |m| m.len());
        use std::io::Seek;
        let at = (&*f).stream_position().unwrap_or(len);
        len.saturating_sub(at) as usize
    }

    pub fn stdin_pending() -> usize {
        // SAFETY: the process's own standard handle.
        handle_pending(unsafe { GetStdHandle(STD_INPUT_HANDLE) })
    }

    pub fn read_stdin(out: &mut [u8]) -> std::io::Result<usize> {
        use std::io::Read;
        use std::os::windows::io::FromRawHandle;
        // SAFETY: the process's own standard handle, never closed here.
        let mut f = std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_handle(GetStdHandle(STD_INPUT_HANDLE)) });
        f.read(out)
    }

    pub fn socket_pending(_s: &std::net::TcpStream) -> usize {
        1
    }

    /// A read of a pipe blocks on Windows: it is not waited for in slices.
    pub fn wait_readable(_f: &std::fs::File, _timeout: Duration) -> bool {
        true
    }

    pub fn wait_stdin(_timeout: Duration) -> bool {
        true
    }

    #[repr(C)]
    struct VersionInfo {
        size: u32,
        major: u32,
        minor: u32,
        build: u32,
        platform: u32,
        csd: [u16; 128],
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn RtlGetVersion(info: *mut VersionInfo) -> i32;
    }

    /// `Windows 11` from build 22000, `Windows 10` before, as the JDK names a workstation.
    pub fn name() -> String {
        // SAFETY: a structure of the documented layout, its size given.
        let build = unsafe {
            let mut v: VersionInfo = std::mem::zeroed();
            v.size = std::mem::size_of::<VersionInfo>() as u32;
            if RtlGetVersion(&mut v) == 0 { v.build } else { 0 }
        };
        if build >= 22000 { "Windows 11" } else { "Windows 10" }.to_string()
    }

    pub fn write_all(f: &mut std::fs::File, bytes: &[u8], _signalled: &mut dyn FnMut() -> bool) -> std::io::Result<()> {
        use std::io::Write;
        f.write_all(bytes)
    }

    pub fn processes() -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        // SAFETY: a snapshot read and closed here, its entries sized as the call asks.
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap as isize == -1 {
                return out;
            }
            let mut e: ProcessEntry = std::mem::zeroed();
            e.size = std::mem::size_of::<ProcessEntry>() as u32;
            let mut ok = Process32FirstW(snap, &mut e);
            while ok != 0 {
                out.push((e.pid, e.parent));
                ok = Process32NextW(snap, &mut e);
            }
            CloseHandle(snap);
        }
        out
    }

    /// The executable alone: the JDK reads no other process's arguments on Windows.
    pub fn info(pid: u32) -> (Option<String>, Option<Vec<String>>) {
        // SAFETY: a handle opened and closed here, the name's buffer sized as the call is told.
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return (None, None);
            }
            let mut name = [0u16; 1024];
            let mut size = name.len() as u32;
            let ok = QueryFullProcessImageNameW(h, 0, name.as_mut_ptr(), &mut size);
            CloseHandle(h);
            ((ok != 0).then(|| String::from_utf16_lossy(&name[..size as usize])), None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The JDK's block: names ordered as Windows compares them (`_` after `Z`, not before as by
    /// lower case), case twins kept in the map's order, `SystemRoot` added where no name is it.
    #[test]
    fn a_windows_block_is_the_jdks() {
        let vars = |names: &[&str]| names.iter().map(|n| (n.to_string(), "v".to_string())).collect::<Vec<_>>();
        let names = |block: Vec<(String, String)>| block.into_iter().map(|(k, _)| k).collect::<Vec<_>>();
        assert_eq!(names(windows_block(vars(&["_A", "path", "Za", "Path", "TEMP"]), None)), ["path", "Path", "TEMP", "Za", "_A"]);
        assert_eq!(names(windows_block(vars(&["Path"]), Some("C:\\Windows".to_string()))), ["Path", "SystemRoot"]);
        assert_eq!(names(windows_block(vars(&["SYSTEMROOT"]), Some("C:\\Windows".to_string()))), ["SYSTEMROOT"]);
        assert!(windows_name_order("Path", "PATH").is_eq() && windows_name_order("ab", "a").is_gt());
    }
}
