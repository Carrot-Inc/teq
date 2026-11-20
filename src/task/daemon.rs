//! The session daemon of an export: started by the first `teq` of a build, it holds the
//! residents and answers the clients that connect to its loopback port, which it writes with a
//! token into `target/teq/daemon.json`; a request without the token is refused. Every request
//! then carries the handshake (docs/TARGETS.md, "The export and the project verbs"): a client of another protocol
//! or another binary, or one whose export pins another compiler, is told to start a new daemon,
//! and this one finishes what it is doing and exits; an export otherwise changed is read again
//! in place, the residents whose command lines it changed stopped. The daemon holds
//! `target/teq/task.lock` for its whole life, so that the next one writes `daemon.json` only once
//! this one is gone. It holds a test runner per test configuration too, a JVM for each run as sbt
//! forks a worker for each: started by the configuration's first `test`, launched again as a run
//! ends so that the next run finds it up, and when what it was started with changed or it died.

use super::analysis::View;
use super::events::{self, Tally, Thrown};
use super::export::ModuleKind;
use super::{jsdom, jsrunner};
use super::export::{Entry, Export, Exporter, Group, Key, MainClass, Platform, TestContext};
use super::generators;
use super::resident::{Answer, Resident};
use super::runner::{self, Planned, Ran, Report, Runner, Spec};
use super::stage;
use super::suites::{self, Patterns};
use super::progress::duration_of_ms;
use super::{paths, Implied, Info, Paths, PROTOCOL};
use crate::lsp::json::{obj, Json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long a new daemon waits for the one before it to finish and let go of the lock.
const LOCK_BOUND: Duration = Duration::from_secs(120);
/// How long an ending daemon waits for the requests under way to be answered.
const FINISH_BOUND: Duration = Duration::from_secs(100);
/// How often the daemon looks at `daemon.json` and the export.
const WATCH_EVERY: Duration = Duration::from_secs(2);
/// How long an ending daemon waits for a resident or a runner that a request still holds.
const STOP_BOUND: Duration = Duration::from_secs(10);

type Slot = Arc<Mutex<Option<Resident>>>;
type RunnerSlot = Arc<Mutex<Option<Runner>>>;

/// A runner whose run is done, quitting: its process and whether its stop has completed.
struct Retiring {
    process: Arc<Mutex<std::process::Child>>,
    stopped: Arc<AtomicBool>,
}

/// The class files a test configuration's resident wrote since the configuration's last run of
/// every suite that could be affected; all of them when the resident started anew since.
struct Changes {
    resident: Key,
    all: bool,
    files: BTreeSet<String>,
    /// Of a Scala.js test configuration, whose link names no class file: the digest of each source
    /// file of the last full run's view.
    sources: BTreeMap<String, String>,
    /// The classes of the source files removed since, and of those that depended on them then,
    /// which no class file written and no digest compared tells (`Answer::removed_reach`).
    removed: BTreeSet<String>,
}

struct State {
    export: Mutex<Arc<Export>>,
    identity: String,
    exe: PathBuf,
    paths: Paths,
    token: String,
    port: u16,
    residents: Mutex<BTreeMap<Key, Slot>>,
    runners: Mutex<BTreeMap<Key, RunnerSlot>>,
    /// The runners' processes, which an ending daemon kills without waiting for a run that holds
    /// its runner.
    runner_processes: Mutex<BTreeMap<Key, Arc<Mutex<std::process::Child>>>>,
    /// The runners whose run is done, quitting on threads of their own, each with whether its stop has completed:
    /// an ending daemon waits for them within the stop bound and kills what is left, so that a spent runner never
    /// outlives it.
    retiring: Mutex<Vec<Retiring>>,
    /// By test configuration, from its first run in this daemon's life.
    changes: Mutex<BTreeMap<Key, Changes>>,
    /// Set once the daemon is to end: requests that arrive meanwhile are told to start anew.
    ending: Mutex<Option<String>>,
    /// The requests past their handshake and not yet answered, which an ending daemon waits for.
    in_flight: (Mutex<usize>, std::sync::Condvar),
}

/// A request under way, counted while it lives.
struct Flight<'a>(&'a State);

impl<'a> Flight<'a> {
    fn enter(state: &'a State) -> Flight<'a> {
        *state.in_flight.0.lock().unwrap_or_else(|e| e.into_inner()) += 1;
        Flight(state)
    }
}

impl Drop for Flight<'_> {
    fn drop(&mut self) {
        let (count, changed) = &self.0.in_flight;
        *count.lock().unwrap_or_else(|e| e.into_inner()) -= 1;
        changed.notify_all();
    }
}

pub fn run(export_file: &Path, identity: String) -> ! {
    let export = match Export::read(export_file) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("teq: {}", e);
            std::process::exit(2);
        }
    };
    let paths = paths(&export.root);
    let lock = match hold_lock(&paths.lock) {
        Ok(lock) => lock,
        Err(e) => {
            eprintln!("teq: {}", e);
            std::process::exit(2);
        }
    };
    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("teq: cannot listen on the loopback interface: {}", e);
            std::process::exit(2);
        }
    };
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
    let token = super::token();
    if let Err(e) = (Info { port, token: token.clone(), pid: std::process::id(), sha256: export.sha256.clone() }).write(&paths.info) {
        eprintln!("teq: cannot write {}: {}", paths.info.display(), e);
        std::process::exit(2);
    }
    log(&format!("daemon {} for {} on port {} ({})", std::process::id(), export.file.display(), port, identity));
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("teq"));
    let state = Arc::new(State { export: Mutex::new(Arc::new(export)), identity, exe, paths, token, port, residents: Mutex::new(BTreeMap::new()), runners: Mutex::new(BTreeMap::new()), runner_processes: Mutex::new(BTreeMap::new()), retiring: Mutex::new(Vec::new()), changes: Mutex::new(BTreeMap::new()), ending: Mutex::new(None), in_flight: Default::default() });
    let watched = state.clone();
    crate::alloc::spawn(move || watch(&watched));
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let _ = stream.set_nodelay(true);
        let state = state.clone();
        crate::alloc::spawn(move || serve(&state, stream));
    }
    drop(lock);
    std::process::exit(0)
}

/// Ends the daemon when `daemon.json` no longer names it (`target/teq` cleaned, another daemon
/// started over it) or when the export is gone or pins another compiler (a branch switch), so
/// that its residents do not outlive what they served.
fn watch(state: &Arc<State>) {
    let file = state.export.lock().unwrap_or_else(|e| e.into_inner()).file.clone();
    let mut seen = std::fs::metadata(&file).and_then(|m| m.modified()).ok();
    loop {
        std::thread::sleep(WATCH_EVERY);
        if Info::read(&state.paths.info).is_none_or(|i| i.pid != std::process::id()) {
            return end(state, "daemon.json no longer names this daemon");
        }
        let now = std::fs::metadata(&file).and_then(|m| m.modified()).ok();
        if now == seen {
            continue;
        }
        seen = now;
        let current = state.export.lock().unwrap_or_else(|e| e.into_inner()).teq.clone();
        match Export::read(&file) {
            Err(_) if now.is_none() => return end(state, "the export is gone"),
            Ok(fresh) if fresh.teq != current => return end(state, "the export pins another compiler"),
            _ => {}
        }
    }
}

fn log(line: &str) {
    eprintln!("{}", line);
}

/// The daemon's lock, waited for while the daemon before it ends.
fn hold_lock(file: &Path) -> Result<std::fs::File, String> {
    let lock = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(file).map_err(|e| format!("cannot open {}: {}", file.display(), e))?;
    let until = Instant::now() + LOCK_BOUND;
    loop {
        match lock.try_lock() {
            Ok(()) => return Ok(lock),
            Err(std::fs::TryLockError::WouldBlock) if Instant::now() < until => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(format!("cannot hold {}: {:?}", file.display(), e)),
        }
    }
}

/// The answer of a request: lines of output, then the exit code.
pub struct Reply {
    stream: TcpStream,
}

impl Reply {
    fn send(&mut self, message: Json) {
        let mut line = message.to_text();
        line.push('\n');
        let _ = self.stream.write_all(line.as_bytes());
    }

    pub fn out(&mut self, text: &str) {
        self.send(obj([("out", text.into())]));
    }

    pub fn err(&mut self, text: &str) {
        self.send(obj([("err", text.into())]));
    }

    fn exit(&mut self, code: i32) {
        self.send(obj([("exit", Json::Num(code as f64))]));
    }

    /// Whether the client closed the connection. A client only reads until its request ends, so a read that ends
    /// at once, or that the system refuses because the connection is no more, says it is gone; any other outcome
    /// of the look (no byte within its bound, a read that was interrupted) leaves the client there.
    fn client_gone(&self) -> bool {
        use std::io::ErrorKind::{BrokenPipe, ConnectionAborted, ConnectionReset, NotConnected};
        let _ = self.stream.set_read_timeout(Some(Duration::from_millis(1)));
        let mut byte = [0u8; 1];
        let gone = match self.stream.peek(&mut byte) {
            Ok(n) => n == 0,
            Err(e) => matches!(e.kind(), BrokenPipe | ConnectionAborted | ConnectionReset | NotConnected),
        };
        let _ = self.stream.set_read_timeout(None);
        gone
    }

    /// The request's output as a JavaScript test run writes it, from the runner and from the
    /// threads reading node's output, each text sent whole under one lock.
    fn shared_out(&self) -> jsrunner::Output {
        let stream = Mutex::new(self.stream.try_clone().ok());
        Arc::new(move |text: &str| {
            if let Some(s) = stream.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                let mut line = obj([("out", text.into())]).to_text();
                line.push('\n');
                let _ = s.write_all(line.as_bytes());
            }
        })
    }

    /// Where the request's builds, on threads of their own, tell their progress (an artifact
    /// fetched), each line as it comes.
    fn sink(&self) -> Sink {
        Sink(Mutex::new(self.stream.try_clone().ok()))
    }
}

/// Lines of a request's progress, sent to its client as error output.
struct Sink(Mutex<Option<TcpStream>>);

impl Sink {
    /// A record of the request's own (`progress`), which its client reads and does not print.
    fn record(&self, record: Json) {
        if let Some(stream) = self.0.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            let mut line = record.to_text();
            line.push('\n');
            let _ = stream.write_all(line.as_bytes());
        }
    }

    fn line(&self, text: &str) {
        log(text);
        if let Some(stream) = self.0.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            let mut line = obj([("err", format!("{}\n", text).as_str().into())]).to_text();
            line.push('\n');
            let _ = stream.write_all(line.as_bytes());
        }
    }
}

struct Request {
    verb: String,
    args: Vec<String>,
    /// The variables of the client's environment the export's dynamic keys read.
    env: BTreeMap<String, String>,
    /// The client's whole environment, which a test runner starts with.
    environment: BTreeMap<String, String>,
    /// Whether the client writes to a terminal.
    tty: bool,
    cwd: PathBuf,
    /// `--diagnostics <file>`, any verb's: each diagnostic a build of the request shows, each failure
    /// of a configuration's (`teq: <key>: <why>`, a generator's with its command's output) and what
    /// each passing generator printed recorded there, a JSON line each (`record`).
    diagnostics: Option<PathBuf>,
    /// Whether that file is the request's own, which its client moves over the file the command names when the
    /// request ends (`client::Records`).
    own_records: bool,
}

fn serve(state: &Arc<State>, stream: TcpStream) {
    // A request that does not read is refused with a line, as one that is no request is: a client that is left
    // without a line takes the daemon for one that ended. No return below leaves a connection without one.
    let mut line = String::new();
    let read = stream.try_clone().and_then(|read| BufReader::new(read).read_line(&mut line));
    let mut reply = Reply { stream };
    match read {
        Ok(n) if n > 0 => {}
        Ok(_) => {
            reply.err("teq: an empty request\n");
            return reply.exit(2);
        }
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
            reply.err("teq: a request that is not UTF-8\n");
            return reply.exit(2);
        }
        Err(e) => {
            reply.err(&format!("teq: a request that does not read: {}\n", e));
            return reply.exit(2);
        }
    }
    let Ok(json) = Json::parse(line.trim_end()) else {
        reply.err("teq: a request that is not JSON\n");
        return reply.exit(2);
    };
    if json.get("token").and_then(Json::str) != Some(state.token.as_str()) {
        reply.err("teq: refused: the request does not carry the daemon's token\n");
        return reply.exit(2);
    }
    let export = match handshake(state, &json) {
        Ok(export) => export,
        Err(why) => {
            // The daemon is ending and advertises itself no longer before the client reads the answer: the daemon
            // that client starts next, and any other client, find nothing of this one's to connect to.
            let first = retire(state, &why);
            reply.send(obj([("restart", why.as_str().into())]));
            if first {
                finish(state);
            }
            return;
        }
    };
    let flight = Flight::enter(state);
    let strings = |key: &str| json.get(key).map_or(&[][..], Json::arr).iter().filter_map(Json::str).map(str::to_string).collect::<Vec<_>>();
    let variables = |key: &str| match json.get(key) {
        Some(Json::Obj(fields)) => fields.iter().filter_map(|(k, v)| Some((k.clone(), v.str()?.to_string()))).collect(),
        _ => BTreeMap::new(),
    };
    let cwd = PathBuf::from(json.get("cwd").and_then(Json::str).unwrap_or("/"));
    // `--diagnostics <file>` taken out of any verb's arguments, the file made afresh.
    let (mut args, mut diagnostics) = (Vec::new(), None);
    let mut given = strings("args").into_iter();
    while let Some(a) = given.next() {
        if a != "--diagnostics" {
            args.push(a);
            continue;
        }
        let Some(file) = given.next() else {
            reply.err("teq: --diagnostics needs a file\n");
            return reply.exit(2);
        };
        let file = cwd.join(file);
        if let Err(e) = std::fs::write(&file, "") {
            reply.err(&format!("teq: cannot write {}: {}\n", file.display(), e));
            return reply.exit(2);
        }
        diagnostics = Some(file);
    }
    let request = Request {
        verb: json.get("verb").and_then(Json::str).unwrap_or("").to_string(),
        args,
        env: variables("env"),
        environment: variables("environment"),
        tty: json.get("tty").and_then(Json::bool) == Some(true),
        cwd,
        own_records: diagnostics.is_some() && json.get("own_records").and_then(Json::bool) == Some(true),
        diagnostics,
    };
    log(&format!("{} {}", request.verb, request.args.join(" ")));
    match request.verb.as_str() {
        "compile" => {
            let (code, _) = compile(state, &export, &request, &mut reply, Summary::Out);
            ended(&request, &mut reply, code);
        }
        // `run`'s compile: the summary on stderr, so that the program's output is alone on
        // stdout, and as data the class directory of the resident that wrote the closure with
        // the main classes among the project's products.
        "classes" => {
            let (code, keys) = compile(state, &export, &request, &mut reply, Summary::Err);
            if let (0, [key], [name]) = (code, keys.as_slice(), request.args.as_slice()) {
                let mains = match main_classes(state, &export, key, name) {
                    Ok(mains) => ("mainClasses", Json::Arr(mains.into_iter().map(Json::Str).collect())),
                    Err(why) => ("mainClassesUnknown", why.as_str().into()),
                };
                reply.send(obj([("data", obj([("classes", export.classes(key).as_str().into()), mains]))]));
            }
            ended(&request, &mut reply, code);
        }
        "stage" => {
            let code = stage(state, &export, &request, &mut reply);
            ended(&request, &mut reply, code);
        }
        "test" => {
            let code = test(state, &export, &request, &mut reply);
            ended(&request, &mut reply, code);
        }
        "stop" => {
            reply.out("teq: stopped the daemon, its residents and its test runners\n");
            reply.exit(0);
            drop(flight);
            end(state, "stop");
        }
        other => {
            reply.err(&format!("teq: the daemon knows no verb {}\n", other));
            reply.exit(2);
        }
    }
}

/// A request's end: its exit code sent. Records in a file of the request's own wait for its client to move them
/// over the file the command names; where the client is gone nobody will, and the file is removed.
fn ended(request: &Request, reply: &mut Reply, code: i32) {
    if let (true, Some(file)) = (request.own_records && reply.client_gone(), &request.diagnostics) {
        let _ = std::fs::remove_file(file);
    }
    reply.exit(code);
}

/// The export a request is served from, or why the client is to start a new daemon.
fn handshake(state: &Arc<State>, json: &Json) -> Result<Arc<Export>, String> {
    if let Some(why) = state.ending.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        return Err(format!("the daemon is ending ({})", why));
    }
    if json.get("protocol").and_then(Json::uint) != Some(PROTOCOL) {
        return Err("the client speaks another protocol".to_string());
    }
    if json.get("identity").and_then(Json::str) != Some(state.identity.as_str()) {
        return Err("the client is another teq binary".to_string());
    }
    // Held while the export is read again, so that two clients of a changed export swap it once.
    let mut residents = state.residents.lock().unwrap_or_else(|e| e.into_inner());
    let current = state.export.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if json.get("export").and_then(Json::str) != Some(&*current.file.to_string_lossy()) {
        return Err("the client reads another export".to_string());
    }
    let sha256 = json.get("sha256").and_then(Json::str).unwrap_or("");
    if sha256 == current.sha256 {
        return Ok(current);
    }
    let fresh = Export::read(&current.file).map_err(|e| format!("the export changed and does not read: {}", e))?;
    if fresh.sha256 != sha256 {
        return Err("the export changed while the client read it".to_string());
    }
    if fresh.teq != current.teq {
        return Err("the export pins another compiler".to_string());
    }
    // What the old export started keeps running where its command line stands.
    let fresh = Arc::new(fresh);
    let stale: Vec<Key> = residents
        .iter()
        .filter(|(key, slot)| {
            let args = slot.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|r| r.args.clone());
            fresh.configuration(key).is_none() || !fresh.typable(key) || fresh.resident_args(key).ok() != args
        })
        .map(|(k, _)| k.clone())
        .collect();
    for key in stale {
        if let Some(slot) = residents.remove(&key) {
            if let Some(r) = slot.lock().unwrap_or_else(|e| e.into_inner()).take() {
                log(&format!("stopping {}: the export changed its command line or its closure's generators", key));
                r.stop();
            }
        }
    }
    let mut runners = state.runners.lock().unwrap_or_else(|e| e.into_inner());
    let gone: Vec<Key> = runners.keys().filter(|k| fresh.configuration(k).is_none_or(|c| c.test.is_none())).cloned().collect();
    for key in gone {
        if let Some(r) = runners.remove(&key).and_then(|slot| slot.lock().unwrap_or_else(|e| e.into_inner()).take()) {
            log(&format!("stopping the test runner of {}: the export no longer has the configuration", key));
            r.stop();
        }
    }
    drop(runners);
    *state.export.lock().unwrap_or_else(|e| e.into_inner()) = fresh.clone();
    let _ = (Info { port: state.port, token: state.token.clone(), pid: std::process::id(), sha256: fresh.sha256.clone() }).write(&state.paths.info);
    Ok(fresh)
}

/// Ends the daemon once the requests under way are answered: `daemon.json` removed at once, so
/// that a new client starts the next daemon, which waits for this one's lock.
fn end(state: &Arc<State>, why: &str) {
    if retire(state, why) {
        finish(state);
    }
}

/// The daemon is ending from now on, for `why`: it serves no new request and its `daemon.json` is removed. Whether
/// this call is the one that ended it, which then finishes it (`finish`).
fn retire(state: &Arc<State>, why: &str) -> bool {
    {
        let mut ending = state.ending.lock().unwrap_or_else(|e| e.into_inner());
        if ending.is_some() {
            return false;
        }
        *ending = Some(why.to_string());
    }
    // The advertisement goes before anything that can wait (the log's line: a log that is not read holds its
    // writer), so that no client finds this daemon once it is ending.
    if Info::read(&state.paths.info).is_some_and(|i| i.pid == std::process::id()) {
        let _ = std::fs::remove_file(&state.paths.info);
    }
    log(&format!("ending: {}", why));
    true
}

/// The end of a daemon that is ending (`retire`): the requests under way answered, its residents and runners
/// stopped, the process left.
fn finish(state: &Arc<State>) {
    let (count, changed) = &state.in_flight;
    let until = Instant::now() + FINISH_BOUND;
    let mut n = count.lock().unwrap_or_else(|e| e.into_inner());
    while *n > 0 && Instant::now() < until {
        n = changed.wait_timeout(n, Duration::from_millis(200)).unwrap_or_else(|e| e.into_inner()).0;
    }
    if *n > 0 {
        // A suite that hangs holds its run's slots: its runner killed, the run ends.
        log(&format!("killing the test runners: {} requests still under way", *n));
        for process in state.runner_processes.lock().unwrap_or_else(|e| e.into_inner()).values() {
            let _ = process.lock().unwrap_or_else(|e| e.into_inner()).kill();
        }
    }
    drop(n);
    let slots: Vec<Slot> = state.residents.lock().unwrap_or_else(|e| e.into_inner()).values().cloned().collect();
    for slot in slots {
        if let Some(r) = lock_within(&slot, STOP_BOUND).and_then(|mut s| s.take()) {
            r.stop();
        }
    }
    let runners: Vec<RunnerSlot> = state.runners.lock().unwrap_or_else(|e| e.into_inner()).values().cloned().collect();
    for slot in runners {
        if let Some(r) = lock_within(&slot, STOP_BOUND).and_then(|mut s| s.take()) {
            r.stop();
        }
    }
    // The spent runners still quitting, waited for within the bound, what is left killed.
    let until = Instant::now() + STOP_BOUND;
    loop {
        let left: Vec<Arc<Mutex<std::process::Child>>> = state.retiring.lock().unwrap_or_else(|e| e.into_inner()).iter().filter(|r| !r.stopped.load(Ordering::SeqCst)).map(|r| r.process.clone()).collect();
        if left.is_empty() {
            break;
        }
        if Instant::now() >= until {
            log(&format!("killing {} test runners still quitting", left.len()));
            for process in left {
                let mut child = process.lock().unwrap_or_else(|e| e.into_inner());
                let _ = child.kill();
                let _ = child.wait();
            }
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    std::process::exit(0);
}

/// The lock, unless another holds it past the bound.
fn lock_within<T>(mutex: &Mutex<T>, bound: Duration) -> Option<std::sync::MutexGuard<'_, T>> {
    let until = Instant::now() + bound;
    loop {
        match mutex.try_lock() {
            Ok(guard) => return Some(guard),
            Err(std::sync::TryLockError::Poisoned(e)) => return Some(e.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) if Instant::now() < until => std::thread::sleep(Duration::from_millis(20)),
            Err(std::sync::TryLockError::WouldBlock) => return None,
        }
    }
}

/// The residents that serve the configurations: while any is left, a running resident that types
/// the most of them, else the widest configuration of a project that does, a resident typing a
/// configuration of its platform when its inputs hold the configuration's (which a
/// description's source overrides decide, not the dependency graph: a JVM resident over the
/// sources a Scala.js project shares writes class files a Scala.js check does not); a
/// configuration nothing else types gets its project's widest resident.
fn plan(export: &Export, running: &[Key], wanted: &[Key]) -> Vec<Key> {
    plan_served(export, running, wanted).into_iter().map(|(key, _)| key).collect()
}

/// `plan`, each resident with the wanted configurations it types, in their order.
fn plan_served(export: &Export, running: &[Key], wanted: &[Key]) -> Vec<(Key, Vec<Key>)> {
    let inputs = |k: &Key| export.session_inputs(k).into_iter().collect::<BTreeSet<PathBuf>>();
    let platform = |k: &Key| export.projects.get(&k.project).map(|p| p.platform);
    let needs: BTreeMap<Key, BTreeSet<PathBuf>> = wanted.iter().map(|k| (k.clone(), inputs(k))).collect();
    let types = |resident: &Key, has: &BTreeSet<PathBuf>, k: &Key| platform(resident) == platform(k) && needs[k].is_subset(has) && export.serves(resident, k);
    let mut left: Vec<Key> = wanted.to_vec();
    let mut chosen: Vec<(Key, Vec<Key>)> = Vec::new();
    while let Some(first) = left.first().cloned() {
        let typed = |resident: &Key, has: &BTreeSet<PathBuf>, left: &[Key]| left.iter().filter(|k| types(resident, has, k)).count();
        // A running resident whose closure has come to hold a generator sbt runs is passed over,
        // as `widest` passes over its configuration.
        let candidates: Vec<(Key, BTreeSet<PathBuf>)> = match running.iter().filter(|k| export.typable(k)).map(|k| (k.clone(), inputs(k))).filter(|(k, has)| typed(k, has, &left) > 0).collect::<Vec<_>>() {
            found if !found.is_empty() => found,
            _ => left.iter().filter_map(|k| export.widest(&k.project)).map(|k| { let has = inputs(&k); (k, has) }).collect(),
        };
        let best = candidates.into_iter().map(|(k, has)| (typed(&k, &has, &left), k, has)).max_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
        let (key, has) = match best {
            Some((n, k, has)) if n > 0 => (k, has),
            _ => {
                // The project's widest resident, unless it may not type the configuration (one that is not
                // transitive, which a resident of its own types): then that one.
                let k = export.widest(&first.project).filter(|w| export.serves(w, &first)).unwrap_or_else(|| first.clone());
                let has = inputs(&k).union(&needs[&first]).cloned().collect();
                (k, has)
            }
        };
        let served: Vec<Key> = left.iter().filter(|k| types(&key, &has, k)).cloned().collect();
        left.retain(|k| !types(&key, &has, k));
        match chosen.iter_mut().find(|(k, _)| *k == key) {
            Some((_, taken)) => taken.extend(served),
            None => chosen.push((key, served)),
        }
    }
    chosen
}

/// The projects whose sources hold `file`, a diagnostic's, the one with the innermost source root that holds it
/// first: several where projects share a source root (a cross-built project's shared sources).
fn projects_of(export: &Export, file: &str) -> Vec<String> {
    let file = export.path(file);
    let mut holders: Vec<(usize, &String)> = export
        .projects
        .iter()
        .flat_map(|(name, p)| p.configurations.values().flat_map(move |c| c.sources.iter().map(move |s| (name, export.path(s)))))
        .filter(|(_, root)| file.starts_with(root))
        .map(|(name, root)| (root.components().count(), name))
        .collect();
    holders.sort_by(|a, b| b.0.cmp(&a.0));
    let mut names: Vec<String> = Vec::new();
    for (_, name) in holders {
        if !names.contains(name) {
            names.push(name.clone());
        }
    }
    names
}

/// The errors of a failed build of `key` that are in no project named on its line, counted by the project whose
/// sources hold each, in the order met: the build failed, in that share, for what the request did not ask for. An
/// error in a file that one of the named projects holds, with others or alone, is theirs. Of several projects that
/// share a file's source root (a cross-built project's shared sources) the place is the one the build itself types;
/// where it types several of them, or none, the line names no place: it never says more than is true.
fn elsewhere(export: &Export, key: &Key, answer: &Answer, named: &[String]) -> Vec<(String, usize)> {
    let mut shares: Vec<(String, usize)> = Vec::new();
    if answer.ok {
        return shares;
    }
    let typed: Vec<String> = export.closure(key).into_iter().map(|k| k.project).collect();
    for file in answer.errors.iter().filter_map(|e| e.get("file").and_then(Json::str)) {
        let holders = projects_of(export, file);
        if holders.iter().any(|p| named.contains(p)) {
            continue;
        }
        let mut of_the_build = holders.iter().filter(|p| typed.contains(p));
        let (Some(first), None) = (of_the_build.next(), of_the_build.next()) else { continue };
        match shares.iter_mut().find(|(p, _)| p == first) {
            Some((_, n)) => *n += 1,
            None => shares.push((first.clone(), 1)),
        }
    }
    shares
}

/// The projects whose compile configuration the resident `key` types, in the export's order: what a build of it
/// types, whichever of them the request asked for (`served`, which are among them).
fn typed_projects(export: &Export, key: &Key, served: &[Key]) -> Vec<String> {
    let has: BTreeSet<PathBuf> = export.session_inputs(key).into_iter().collect();
    let platform = |k: &Key| export.projects.get(&k.project).map(|p| p.platform);
    let mut names: BTreeSet<String> = served.iter().map(|k| k.project.clone()).collect();
    for (name, _) in export.projects.iter().filter(|(_, p)| p.configurations.contains_key("compile")) {
        let k = Key::new(name, "compile");
        if platform(&k) == platform(key) && export.serves(key, &k) && export.session_inputs(&k).into_iter().all(|i| has.contains(&i)) {
            names.insert(name.clone());
        }
    }
    names.into_iter().collect()
}

/// Where a build's summary line goes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Summary {
    Out,
    Err,
}

/// The projects' compile configurations built; the exit code and the residents that built them.
fn compile(state: &Arc<State>, export: &Arc<Export>, request: &Request, reply: &mut Reply, summary: Summary) -> (i32, Vec<Key>) {
    let mut wanted = Vec::new();
    for name in &request.args {
        match export.projects.get(name) {
            Some(p) if p.configurations.contains_key("compile") => wanted.push(Key::new(name, "compile")),
            Some(_) => {}
            None => {
                reply.err(&format!("teq: no project {} in {} (projects: {})\n", name, export.file.display(), export.projects.keys().cloned().collect::<Vec<_>>().join(", ")));
                return (2, Vec::new());
            }
        }
    }
    let named: Vec<&str> = request.args.iter().map(String::as_str).collect();
    if let Some(why) = super::refused(export, "compile", &named) {
        reply.err(&format!("teq: {}\n", why));
        return (2, Vec::new());
    }
    // Without a project every project's, but those whose generators sbt alone runs, named after
    // and failing the run.
    let mut left_out = Vec::new();
    if request.args.is_empty() {
        for (n, _) in export.projects.iter().filter(|(_, p)| p.configurations.contains_key("compile")) {
            if export.refusals("compile", n).is_empty() { wanted.push(Key::new(n, "compile")) } else { left_out.push(n.clone()) }
        }
    }
    let running = running(state);
    let planned = plan_served(export, &running, &wanted);
    let keys: Vec<Key> = planned.iter().map(|(key, _)| key.clone()).collect();
    let sink = reply.sink();
    // What the client tells of a build that has not answered within a second (`client::Waiting`): the plan
    // before the builds start, each resident with the projects it types and whether this request starts it,
    // and a record as each answers.
    let residents: Vec<Json> = planned
        .iter()
        .map(|(key, served)| {
            let projects: Vec<Json> = typed_projects(export, key, served).iter().map(|n| Json::from(n.as_str())).collect();
            obj([("key", key.to_string().as_str().into()), ("projects", Json::Arr(projects)), ("started", (!running.contains(key)).into())])
        })
        .collect();
    sink.record(obj([("progress", obj([("plan", Json::Arr(residents))]))]));
    let sink = &sink;
    let progress = |line: &str| sink.line(line);
    let progress = &progress;
    let results: Vec<(Key, Result<Answer, Failure>, Vec<String>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = keys
            .iter()
            .map(|key| {
                crate::alloc::spawn_in(scope, move || {
                    let built = build(state, export, key, &request.env, progress);
                    sink.record(obj([("progress", obj([("answered", key.to_string().as_str().into())]))]));
                    built
                })
            })
            .collect();
        keys.iter()
            .cloned()
            .zip(handles)
            .map(|(k, h)| {
                let (answer, notes) = h.join().unwrap_or_else(|_| (Err(Failure::crash("the build's thread panicked")), Vec::new()));
                (k, answer, notes)
            })
            .collect()
    });
    let mut code = 0;
    // The errors the result lines tell, a build that failed otherwise counted as one.
    let mut errors = 0;
    for (key, answer, notes) in results {
        // The projects asked for that this build covered, in the order asked: the name of its lines.
        let mut names: Vec<String> = Vec::new();
        for served in planned.iter().filter(|(k, _)| *k == key).flat_map(|(_, served)| served) {
            if !names.contains(&served.project) {
                names.push(served.project.clone());
            }
        }
        if names.is_empty() {
            names.push(key.project.clone());
        }
        for note in notes {
            noted(request, &key, &note, reply);
        }
        // The build's record, which the client makes its line of (`client::Waiting`): the projects asked for
        // that this build covered and what of them; whether the build is wider than the request (another
        // project's resident served it); where it failed, its errors that are in none of them, by project
        // (`elsewhere`); a build that failed outright has its own line (`failed_as`).
        let mut built = vec![
            ("projects", Json::Arr(names.iter().map(|n| Json::from(n.as_str())).collect())),
            ("test", (key.configuration == "test" && names.contains(&key.project)).into()),
            ("wider", (!names.contains(&key.project)).into()),
            ("jvm", export.projects.get(&key.project).is_some_and(|p| p.platform == Platform::Jvm).into()),
        ];
        match &answer {
            Ok(a) => {
                for n in &a.notes {
                    told(request, &key, &names, n, reply);
                }
                for w in &a.warnings {
                    shown(request, &key, w, "warning", &export.root, reply);
                }
                for e in &a.errors {
                    shown(request, &key, e, "error", &export.root, reply);
                }
                let n = if a.ok { 0 } else { a.errors.len().max(1) };
                if !a.ok {
                    code = 1;
                    errors += n;
                }
                built.extend([
                    ("ok", a.ok.into()),
                    ("written", Json::Num(a.changed.len() as f64)),
                    ("typed", a.typed.into()),
                    ("ms", Json::Num(a.ms)),
                    ("errors", Json::Num(n as f64)),
                    ("in", Json::Arr(elsewhere(export, &key, a, &names).into_iter().map(|(p, n)| Json::Arr(vec![p.as_str().into(), Json::Num(n as f64)])).collect())),
                ]);
            }
            Err(e) => {
                failed_as(request, &key, &names, e, reply);
                code = 1;
                errors += 1;
                built.push(("failed", true.into()));
            }
        }
        outcome(request, &key, &names, answer.as_ref(), reply);
        sink.record(obj([("progress", obj([("built", Json::Obj(built.into_iter().map(|(k, v)| (k.to_string(), v)).collect()))]))]));
    }
    if keys.is_empty() {
        match summary {
            Summary::Out => reply.out("teq: nothing to compile\n"),
            Summary::Err => reply.err("teq: nothing to compile\n"),
        }
    }
    // Each project left out: its lines, and its record (`--diagnostics`, severity `left-out`), which says of the
    // request that it compiled less than it was asked.
    for project in &left_out {
        let left = super::left_out(export, "compile", std::slice::from_ref(project));
        match summary {
            Summary::Out => reply.out(&left),
            Summary::Err => reply.err(&left),
        }
        record(request, vec![("severity", "left-out".into()), ("project", project.as_str().into()), ("shown", left.as_str().into())], reply);
    }
    // What the client's last line counts, where it told of the wait: the configurations asked for and the errors.
    sink.record(obj([("progress", obj([("done", obj([("configurations", Json::Num(wanted.len() as f64)), ("errors", Json::Num(errors as f64))]))]))]));
    (super::with_left_out(code, &left_out), keys)
}

/// What `test` was asked: `[project] [pattern...] [--changed] [--list] [--verbose]`.
struct TestRequest {
    keys: Vec<Key>,
    patterns: Patterns,
    changed: bool,
    list: bool,
    verbose: bool,
    /// `--events <file>`: where every event of the run is written, a line each (`Event::line`).
    events: Option<PathBuf>,
}

/// `test`: each test configuration's closure compiled as `compile` does, then its suites run on
/// its runner; a configuration named by its project, or every JVM one with frameworks.
fn test(state: &Arc<State>, export: &Arc<Export>, request: &Request, reply: &mut Reply) -> i32 {
    let mut words: Vec<String> = Vec::new();
    let (mut changed, mut list, mut verbose) = (false, false, false);
    let mut events = None;
    let mut args = request.args.iter();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--changed" => changed = true,
            "--list" => list = true,
            "--verbose" | "-v" => verbose = true,
            "--events" => match args.next() {
                Some(file) => events = Some(request.cwd.join(file)),
                None => {
                    reply.err("teq: --events needs a file\n");
                    return 2;
                }
            },
            _ => words.push(a.clone()),
        }
    }
    if let Some(file) = &events {
        if let Err(e) = std::fs::write(file, "") {
            reply.err(&format!("teq: cannot write {}: {}\n", file.display(), e));
            return 2;
        }
    }
    let named = words.first().filter(|w| export.projects.contains_key(*w)).cloned();
    let mut left_out = Vec::new();
    let keys = match &named {
        Some(name) => {
            let project = &export.projects[name];
            if !project.configurations.get("test").is_some_and(|c| c.test.is_some()) {
                // A test configuration the export left out (a Mill test module it could not describe) is
                // refused with the export's reasons.
                match super::refused(export, "test", &[name]) {
                    Some(why) => reply.err(&format!("teq: {}\n", why)),
                    None => reply.err(&format!("teq: project {} has no test configuration\n", name)),
                }
                return 2;
            }
            if let Some(why) = super::refused(export, "test", &[name]) {
                reply.err(&format!("teq: {}\n", why));
                return 2;
            }
            vec![Key::new(name, "test")]
        }
        // Every project with frameworks, on either platform, but those the export records what the
        // build tool alone runs of for its test: named in the summary and failing the run, never
        // skipped unsaid.
        None => {
            let mut keys = Vec::new();
            // A test configuration the export left out with its reasons is left out here, named.
            left_out.extend(export.projects.iter().filter(|(_, p)| !p.configurations.contains_key("test") && p.unsupported.get("test").is_some_and(|w| !w.is_empty())).map(|(n, _)| n.clone()));
            for (n, _) in export.projects.iter().filter(|(_, p)| p.configurations.get("test").and_then(|c| c.test.as_ref()).is_some_and(|t| !t.frameworks.is_empty())) {
                if export.refusals("test", n).is_empty() {
                    keys.push(Key::new(n, "test"))
                } else {
                    left_out.push(n.clone())
                }
            }
            keys
        }
    };
    let patterns = Patterns::new(&words[usize::from(named.is_some())..]);
    let asked = TestRequest { keys, patterns, changed, list, verbose, events };
    if asked.keys.is_empty() && left_out.is_empty() {
        reply.err("teq: no project with test frameworks\n");
        return 0;
    }
    // A run over several configurations (every project's, lila's 86) holds one resident and one
    // runner at a time: each that the request started is stopped after the last configuration it
    // serves, so that the overlapping closures of the projects' residents are never all held at
    // once; those running before the request stay, as a single configuration's run keeps its own.
    let before: BTreeSet<Key> = running(state).into_iter().collect();
    let runners_before: BTreeSet<Key> = state.runners.lock().unwrap_or_else(|e| e.into_inner()).iter().filter(|(_, slot)| slot.try_lock().map_or(true, |r| r.is_some())).map(|(k, _)| k.clone()).collect();
    let last_use = if asked.keys.len() > 1 { last_uses(export, &before, &asked.keys) } else { BTreeMap::new() };
    let mut code = 0;
    for (i, key) in asked.keys.iter().enumerate() {
        code = code.max(test_configuration(state, export, request, &asked, key, reply));
        if !last_use.is_empty() && !runners_before.contains(key) {
            if let Some(r) = state.runners.lock().unwrap_or_else(|e| e.into_inner()).get(key).cloned().and_then(|slot| slot.lock().unwrap_or_else(|e| e.into_inner()).take()) {
                log(&format!("stopping the test runner of {}: its run of several configurations is done with it", key));
                r.stop();
            }
        }
        for (owner, _) in last_use.iter().filter(|&(o, &at)| at == i && !before.contains(o)) {
            if let Some(r) = slot(state, owner).lock().unwrap_or_else(|e| e.into_inner()).take() {
                log(&format!("stopping {}: its run of several configurations is done with it", owner));
                r.stop();
            }
        }
    }
    reply.err(&super::left_out(export, "test", &left_out));
    super::with_left_out(code, &left_out)
}

/// The residents a run over `keys` builds them with, as `plan` chooses for each in turn over the
/// residents running and those chosen before it, with the index of the last configuration each
/// serves.
fn last_uses(export: &Export, running: &BTreeSet<Key>, keys: &[Key]) -> BTreeMap<Key, usize> {
    let mut up: Vec<Key> = running.iter().cloned().collect();
    let mut last = BTreeMap::new();
    for (i, key) in keys.iter().enumerate() {
        if let Some(owner) = plan(export, &up, std::slice::from_ref(key)).into_iter().next() {
            if !up.contains(&owner) {
                up.push(owner.clone());
            }
            last.insert(owner, i);
        }
    }
    last
}

fn test_configuration(state: &Arc<State>, export: &Arc<Export>, request: &Request, asked: &TestRequest, key: &Key, reply: &mut Reply) -> i32 {
    if export.projects.get(&key.project).is_some_and(|p| p.platform == Platform::Js) {
        return test_js_configuration(state, export, request, asked, key, reply);
    }
    let started = Instant::now();
    let running = running(state);
    let Some(owner) = plan(export, &running, std::slice::from_ref(key)).into_iter().next() else { return 2 };
    // Held through the run, so that no other build rewrites the class files the run reads.
    let slot = slot(state, &owner);
    let mut resident = slot.lock().unwrap_or_else(|e| e.into_inner());
    if !built(state, export, request, &owner, &key.project, &mut resident, reply, Summary::Err) {
        return 1;
    }
    let built_ms = started.elapsed().as_millis();
    let view = &resident.as_ref().expect("built").analysis;
    if let Some(why) = &view.unreadable {
        failed(request, key, format!("the analysis does not read: {}", why), reply);
        return 1;
    }
    let conf = export.configuration(key).expect("a test configuration");
    let context = conf.test.clone().unwrap_or_default();
    if context.frameworks.is_empty() {
        idle(request, key, "no test framework", reply);
        return 0;
    }
    let candidates = view.candidates(&conf.sources);
    // Nothing under the configuration's source roots can be a suite: no runner is started to say
    // so (lila's 86 test configurations, most without a test).
    if candidates.is_empty() {
        if asked.list {
            reply.out(&suites::listing(&vec![Vec::new(); context.groups.len() + 1], &context.groups));
        } else {
            idle(request, key, "no suite to run", reply);
        }
        if asked.patterns.is_empty() && !asked.list {
            state.changes.lock().unwrap_or_else(|e| e.into_inner()).insert(key.clone(), Changes { resident: owner, all: false, files: BTreeSet::new(), sources: BTreeMap::new(), removed: BTreeSet::new() });
        }
        return 0;
    }
    let runner_slot = state.runners.lock().unwrap_or_else(|e| e.into_inner()).entry(key.clone()).or_default().clone();
    let mut runner = runner_slot.lock().unwrap_or_else(|e| e.into_inner());
    // The test class path in its order, which the run's class loader is made over once the runner is up.
    let paths = match run_paths(export, key, &owner) {
        Ok(p) => p,
        Err(e) => {
            failed(request, key, e, reply);
            return 2;
        }
    };
    let spec = match runner_spec(export, key, &context, &request.environment, &paths) {
        Ok(s) => s,
        Err(e) => {
            failed(request, key, e, reply);
            return 2;
        }
    };
    // From here on, a client that leaves has the runner it waits for killed (one that hangs at
    // its start, or a suite that hangs), so that the next `test` finds the runner free.
    let finished = Finished(Arc::new(AtomicBool::new(false)));
    if let Some(mut r) = runner.take() {
        if r.spec != spec {
            log(&format!("stopping the test runner of {}: what it was started with changed", key));
            r.stop();
        } else {
            kill_when_gone(reply, &finished, r.process(), key);
            // One launched once the last invocation ended has started meanwhile: its version is read, then a ping
            // tells whether it is still there.
            let ready = (r.handshaken() || (r.alive() && r.handshake().is_ok())) && r.answers();
            if ready {
                *runner = Some(r);
            } else {
                log(&format!("stopping the test runner of {}: it ended", key));
                r.stop();
            }
        }
    }
    let argfile = export.path(&format!("target/teq/{}/{}/runner.args", key.project, key.configuration));
    if runner.is_none() {
        log(&format!("starting the test runner of {}: {}", key, spec.java.display()));
        if context.sandbox {
            if let Err(e) = std::fs::create_dir_all(&spec.directory) {
                failed(request, key, format!("cannot create {}: {}", spec.directory.display(), e), reply);
                return 2;
            }
        }
        match start_runner(state, key, spec, &argfile, Some((reply, &finished))) {
            Ok(r) => *runner = Some(r),
            Err(e) => {
                failed(request, key, e, reply);
                return 2;
            }
        }
    }
    // A runner sent `load` holds this build's classes and whatever its frameworks made loading: it serves this
    // invocation alone, as sbt's worker serves one request, and every way out from here spends it.
    let spend = |runner: &mut Option<Runner>, failed: bool| {
        if let Some(spent) = runner.take() {
            *runner = renew(state, spent, key, &argfile, failed);
        }
    };
    let r = runner.as_mut().expect("started");
    // An export of sbt's has its host played (sbt's `Tests.discover`, `processOptions`, `filteredFrameworks`,
    // `createTestRunners`, `ForkTests`' request and the runners' summaries), one of Mill's, whose runner has none, not.
    let host = export.exporter == Exporter::Sbt;
    // The run's class loader, made once the build is done, and the frameworks loaded through it.
    match r.load(&paths, &context.frameworks, host, &mut |line| reply.out(line)) {
        Ok(Ok(())) => {}
        Ok(Err(thrown)) => {
            // A framework that failed to load ends the run as its run's error does (ForkTestMain.Run.run), before any
            // suite was discovered.
            spend(&mut runner, true);
            if let Some(t) = &thrown {
                reply.out(&events::fork_error(t, asked.verbose));
            }
            let why = thrown.as_ref().and_then(|t| t.text.lines().next()).unwrap_or("a fork error");
            run_failed(request, key, &context.groups, None, why, built_ms, reply);
            not_started(request, key, &context.groups, reply);
            return 1;
        }
        Err(e) => {
            if let Some(r) = runner.take() {
                r.stop();
            }
            failed_in_group(request, key, &context.groups, None, e, reply);
            not_started(request, key, &context.groups, reply);
            return 1;
        }
    }
    let r = runner.as_mut().expect("started");
    // A framework the export names that does not load leaves its suites unrun: never a pass.
    for (class, why) in &r.missing {
        failed(request, key, format!("the test framework {} does not load: {}", class, why), reply);
    }
    let incomplete = i32::from(!r.missing.is_empty());
    let defined = suites::defined(&candidates, &r.fingerprints);
    // Under `--changed`, the suites whose classes depend on a class rewritten since the last full run, or those of a
    // source deleted since; none when every suite runs, which is said.
    let affected = if asked.changed {
        let changes = state.changes.lock().unwrap_or_else(|e| e.into_inner());
        let why_every = match changes.get(key) {
            None => Some("no earlier run in this daemon's life".to_string()),
            Some(c) if c.resident != owner || c.all => Some("its resident started anew since the last run".to_string()),
            Some(_) => view.deps_missing.as_ref().map(|why| format!("the dependencies are unavailable: {}", why)),
        };
        match (why_every, changes.get(key)) {
            (Some(why), _) => {
                told(request, key, std::slice::from_ref(&key.project), &format!("every suite, {}", why), reply);
                None
            }
            (None, Some(c)) => {
                let mut affected = view.affected(&c.files);
                affected.extend(c.removed.iter().cloned());
                Some(affected)
            }
            (None, None) => None,
        }
    } else {
        None
    };
    let plan = selection(defined, &context, asked, affected.as_ref(), export.exporter);
    // A listing is the plan a run with the same arguments would run, the change history left as it is.
    if asked.list {
        reply.out(&suites::listing(&plan.phases, &context.groups));
        spend(&mut runner, false);
        return incomplete;
    }
    // The suites in the selection's order, each with its phase, a phase per group and the suites of none last, and
    // what sbt's host's selection reads.
    let planned = Planned {
        host,
        definitions: plan.definitions.iter().map(|d| (d.framework, d.index, d.name.clone())).collect(),
        filters: plan.filters.clone(),
        suites: plan.selected.iter().map(|(p, d)| (d.framework, d.index, *p, asked.patterns.names(&d.name), d.name.clone())).collect(),
        phases: plan.phases.len(),
    };
    let count = planned.suites.len();
    let every = asked.patterns.is_empty();
    if count == 0 {
        // sbt's host still makes its selection's calls of the frameworks (`processOptions`' and `filteredFrameworks`'),
        // and starts no worker (`ForkTests.apply`); what they throw fails the run.
        if host {
            let r = runner.as_mut().expect("started");
            let mut none: Vec<Tally> = plan.phases.iter().map(|_| Tally::default()).collect();
            match r.run(&planned, context.parallelism, asked.verbose, request.tty, &mut none, &mut |report| {
                if let Report::Line(line) = report {
                    reply.out(line)
                }
            }) {
                Ok(Ran::Failed(thrown)) => {
                    spend(&mut runner, true);
                    if let Some(t) = &thrown {
                        reply.out(&events::fork_error(t, asked.verbose));
                    }
                    let why = thrown.as_ref().and_then(|t| t.text.lines().next()).unwrap_or("a fork error");
                    run_failed(request, key, &context.groups, None, why, built_ms, reply);
                    return 1;
                }
                Ok(_) => {}
                Err(e) => {
                    if let Some(r) = runner.take() {
                        r.stop();
                    }
                    failed_in_group(request, key, &context.groups, None, e, reply);
                    return 1;
                }
            }
        }
        spend(&mut runner, false);
        let why = if asked.changed { "no suite depends on what changed" } else { "no suite to run" };
        idle(request, key, why, reply);
        if every && incomplete == 0 {
            state.changes.lock().unwrap_or_else(|e| e.into_inner()).insert(key.clone(), Changes { resident: owner, all: false, files: BTreeSet::new(), sources: BTreeMap::new(), removed: BTreeSet::new() });
        }
        return incomplete;
    }
    // A run of its own directory starts in it empty, as Mill's sandbox (`TestModuleUtil`) does.
    if context.sandbox {
        if let Err(e) = empty_directory(&export.sandbox_directory(key)) {
            spend(&mut runner, false);
            failed(request, key, e, reply);
            return 2;
        }
    }
    let r = runner.as_mut().expect("started");
    let sizes: Vec<usize> = plan.phases.iter().map(Vec::len).collect();
    let mut tallies: Vec<Tally> = plan.phases.iter().map(|_| Tally::default()).collect();
    let mut ended = vec![false; plan.phases.len()];
    // The phase entered and not ended, where a failure happens in a group.
    let mut current: Option<usize> = None;
    let groups = &context.groups;
    // The run's stream on stdout: what the suites, their frameworks and the runner's JVM print, and teq's report of
    // each event, whose names and messages are the suites' (the run's outcomes are its events, `--events`); teq's own
    // lines on stderr, a build's diagnostics and count, the groups' headers and summaries, the run's summary or end, so
    // that nothing of a run's reaches them.
    let result = r.run(&planned, context.parallelism, asked.verbose, request.tty, &mut tallies, &mut |report| match report {
        Report::Line(line) => reply.out(line),
        Report::Entered(p) => {
            current = Some(p);
            entered(request, key, groups, p, sizes[p], reply);
        }
        Report::Ended(p, ms, tally) => {
            current = None;
            ended[p] = true;
            group_summary(request, key, groups, p, sizes[p], tally, ms, reply);
        }
    });
    drop(finished);
    let stderr = r.drain_stderr();
    if !stderr.is_empty() {
        reply.out(&stderr);
    }
    match result {
        Ok(Ran::Failed(thrown)) => {
            // As sbt's ForkTests fails its harness at the worker's first forkError: nothing more of the run reaches the
            // client, no summary, the events reported before it written; the runner, still at its suites, retired as
            // ForkTests leaves its worker once it has closed its input, its tasks in flight and done() let finish, no
            // later group started.
            spend(&mut runner, true);
            if let Some(t) = &thrown {
                reply.out(&events::fork_error(t, asked.verbose));
            }
            let why = thrown.as_ref().and_then(|t| t.text.lines().next()).unwrap_or("a fork error");
            run_failed(request, key, groups, current, why, built_ms, reply);
            not_completed(request, key, groups, &sizes, &ended, reply);
            write_events(asked, key, groups, &tallies, reply);
            1
        }
        Ok(Ran::Done(ms)) => {
            spend(&mut runner, false);
            if every && incomplete == 0 {
                state.changes.lock().unwrap_or_else(|e| e.into_inner()).insert(key.clone(), Changes { resident: owner, all: false, files: BTreeSet::new(), sources: BTreeMap::new(), removed: BTreeSet::new() });
            }
            let total = Tally::sum(&tallies);
            summary(request, key, count, &total, ms, built_ms, reply);
            write_events(asked, key, groups, &tallies, reply);
            i32::from(total.failed() > 0).max(incomplete)
        }
        Ok(Ran::Stopped(ms)) => {
            // The worker's internal error, its line on the run's stream, ended the run's phases, and the worker
            // answered as at its end: sbt's host's outcome is then that of the suites reported (ForkTests.mainTestTask,
            // `overall` of what React counted), its exit too; the groups not completed named, the change history kept.
            spend(&mut runner, false);
            not_completed(request, key, groups, &sizes, &ended, reply);
            let total = Tally::sum(&tallies);
            summary(request, key, count, &total, ms, built_ms, reply);
            write_events(asked, key, groups, &tallies, reply);
            i32::from(total.failed() > 0).max(incomplete)
        }
        Err(e) => {
            if let Some(r) = runner.take() {
                r.stop();
            }
            // A runner that ended (a suite's System.exit, a JVM lost) ends the run where it is, the events of what ran
            // before written with their groups, as every other end writes them.
            failed_in_group(request, key, groups, current, e, reply);
            not_completed(request, key, groups, &sizes, &ended, reply);
            write_events(asked, key, groups, &tallies, reply);
            1
        }
    }
}

/// How the groups' lines name a phase: a group by its name, `[<name>]`, the suites of none `(ungrouped)`, which no
/// group's is.
fn group_label(groups: &[Group], phase: usize) -> String {
    groups.get(phase).map_or("(ungrouped)".to_string(), |g| format!("[{}]", g.name))
}

/// A group's line on stderr, recorded (`--diagnostics`) with its severity and the group, null for the suites of none.
fn group_line(request: &Request, key: &Key, groups: &[Group], phase: usize, severity: &str, text: String, reply: &mut Reply) {
    reply.err(&text);
    let group = groups.get(phase).map_or(Json::Null, |g| g.name.as_str().into());
    record(request, vec![("key", key.to_string().as_str().into()), ("severity", severity.into()), ("group", group), ("message", text.trim_end().into()), ("shown", text.as_str().into())], reply);
}

/// A phase entered, where the configuration has groups: the group's header, `<key> [<name>]: <n> suites`, or for an
/// empty group `no suite to run`; nothing for the suites of none when there are none.
fn entered(request: &Request, key: &Key, groups: &[Group], phase: usize, size: usize, reply: &mut Reply) {
    if groups.is_empty() || (size == 0 && phase == groups.len()) {
        return;
    }
    let text = match size {
        0 => format!("{} {}: no suite to run\n", key.project, group_label(groups, phase)),
        n => format!("{} {}: {} suite{}\n", key.project, group_label(groups, phase), n, if n == 1 { "" } else { "s" }),
    };
    group_line(request, key, groups, phase, "group", text, reply);
}

/// A phase ended, where the configuration has groups and the phase had suites: the group's summary, its suites, the
/// events of its suites' tasks and of the tasks they returned by status, and its time.
#[allow(clippy::too_many_arguments)]
fn group_summary(request: &Request, key: &Key, groups: &[Group], phase: usize, size: usize, tally: &Tally, ms: u64, reply: &mut Reply) {
    if groups.is_empty() || size == 0 {
        return;
    }
    let text = format!("{} {}: {} suite{}, {} passed, {} failed{} in {}\n", key.project, group_label(groups, phase), size, if size == 1 { "" } else { "s" }, tally.passed(), tally.failed(), tally.others(), duration_of_ms(ms as f64));
    group_line(request, key, groups, phase, "summary", text, reply);
}

/// The end of a run that failed, `<key>: the run failed: <why> (built in <ms> ms)`, on stderr; where the configuration has
/// groups, recorded (`--diagnostics`, severity `failure`) with the group the failure happened in, null where it happened
/// in no group (loading the frameworks, making the runners, their `done()` once every group has ended) or among the
/// suites of none.
#[allow(clippy::too_many_arguments)]
fn run_failed(request: &Request, key: &Key, groups: &[Group], phase: Option<usize>, why: &str, built_ms: u128, reply: &mut Reply) {
    let text = format!("{}: the run failed: {} (built in {})\n", key.project, why, duration_of_ms(built_ms as f64));
    tests_ended(request, key, vec![("run_failed", why.into())], reply);
    if groups.is_empty() {
        reply.err(&text);
        return;
    }
    // The suites of none's phase is no group: null, as outside every phase.
    group_line(request, key, groups, phase.unwrap_or(groups.len()), "failure", text, reply);
}

/// A failure of `key`'s runner or node, which ended (`teq: <project>: <why>`), as `failed` shows and records it; where the
/// configuration has groups, its record carries the group it happened in, as `run_failed`'s does.
fn failed_in_group(request: &Request, key: &Key, groups: &[Group], phase: Option<usize>, why: impl Into<Failure>, reply: &mut Reply) {
    if groups.is_empty() {
        failed(request, key, why, reply);
        return;
    }
    let group = phase.and_then(|p| groups.get(p)).map_or(Json::Null, |g| g.name.as_str().into());
    failure_line(request, key, std::slice::from_ref(&key.project), &why.into(), Some(group), reply);
}

/// The groups a run whose frameworks failed to load, before any suite was discovered, did not start: every group and
/// the suites of none, named on stderr and recorded as `not_completed` names them; nothing without groups.
fn not_started(request: &Request, key: &Key, groups: &[Group], reply: &mut Reply) {
    if groups.is_empty() {
        return;
    }
    let names: Vec<String> = (0..=groups.len()).map(|p| group_label(groups, p)).collect();
    let text = format!("{}: not completed: {}, no suite discovered\n", key.project, names.join(", "));
    group_line(request, key, groups, 0, "failure", text, reply);
}

/// The groups a run that failed did not complete, those with suites that it did not end, named on stderr and recorded
/// (`--diagnostics`, severity `failure`, the first of them its group); nothing without groups, or when every group
/// ended and the frameworks' `done()` failed.
fn not_completed(request: &Request, key: &Key, groups: &[Group], sizes: &[usize], ended: &[bool], reply: &mut Reply) {
    let unfinished: Vec<usize> = (0..sizes.len()).filter(|&p| !ended[p] && sizes[p] > 0).collect();
    if groups.is_empty() || unfinished.is_empty() {
        return;
    }
    let names: Vec<String> = unfinished.iter().map(|&p| group_label(groups, p)).collect();
    let text = format!("{}: not completed: {}\n", key.project, names.join(", "));
    group_line(request, key, groups, unfinished[0], "failure", text, reply);
}

/// The run's events appended to the `--events` file, if one was asked for: each phase's, in their order, with its
/// group where the configuration has groups.
fn write_events(asked: &TestRequest, key: &Key, groups: &[Group], tallies: &[Tally], reply: &mut Reply) {
    let Some(file) = &asked.events else { return };
    // Each case's line, then each suite's that ran (`Tally::ran_line`), a suite that sent no case among them.
    let mut text = String::new();
    for (p, tally) in tallies.iter().enumerate() {
        let group = (!groups.is_empty()).then(|| groups.get(p).map_or("", |g| g.name.as_str()));
        text.extend(tally.events.iter().map(|e| e.line(&key.project, group)).chain(tally.ran.iter().map(|s| Tally::ran_line(&key.project, s, group))));
    }
    let written = std::fs::OpenOptions::new().append(true).open(file).and_then(|mut f| f.write_all(text.as_bytes()));
    if let Err(e) = written {
        reply.err(&format!("teq: cannot write {}: {}\n", file.display(), e));
    }
}

/// One invocation's plan, which a run runs and `--list` prints, on either platform: the suites discovered, as sbt's
/// `Tests.processOptions` selects them (`suites::plan`): less those a `Tests.Exclude` names, then those `testOnly`'s
/// ordered filters accept, in their order, each also one of those `affected` names under `--changed` (all where it is
/// none), as `testQuick`'s filters each also ask its own question, one definition per name; each in the first of the
/// configuration's groups whose patterns match it, then those of none (`suites::placed`). An export of Mill's, whose
/// runner has no host, has its own (`suites::mill_plan`): the patterns one filter, the suites in discovery's order.
fn selection<'a>(defined: Vec<suites::Defined>, context: &TestContext, asked: &'a TestRequest, affected: Option<&'a BTreeSet<String>>, exporter: Exporter) -> suites::Plan {
    let excluded = |name: &str| context.exclude.iter().any(|e| e == name);
    if exporter == Exporter::Mill {
        return suites::mill_plan(defined, excluded, |name| asked.patterns.matches(name) && affected.is_none_or(|a| a.contains(name)), &context.groups);
    }
    let filters: Vec<Box<dyn Fn(&str) -> bool + 'a>> = asked.patterns.ordered();
    let filters: Vec<Box<dyn Fn(&str) -> bool + 'a>> = match affected {
        None => filters,
        Some(a) => filters.into_iter().map(|f| Box::new(move |name: &str| f(name) && a.contains(name)) as Box<dyn Fn(&str) -> bool + 'a>).collect(),
    };
    suites::plan(defined, excluded, &filters, &context.groups)
}

/// The digest of each source file of the view, read from the build's root.
fn source_digests(export: &Export, view: &View) -> BTreeMap<String, String> {
    view.sources().filter_map(|f| Some((f.to_string(), super::sha256::hex(&std::fs::read(export.root.join(f)).ok()?)))).collect()
}

/// `test` of a Scala.js test configuration: its resident links the program whose entry point
/// starts Scala.js's test bridge (`Export::test_link`), whose analysis finds the suites; a fresh
/// node per run loads it as the export's module kind says in the export's environment
/// (`jsrunner`), and the frameworks' fingerprints come from the bridge. An empty discovered set
/// fails, before any listing or selection: a configuration with frameworks and no suite has
/// run nothing (an empty `--changed` selection does not fail).
fn test_js_configuration(state: &Arc<State>, export: &Arc<Export>, request: &Request, asked: &TestRequest, key: &Key, reply: &mut Reply) -> i32 {
    let started = Instant::now();
    let conf = export.configuration(key).expect("a test configuration");
    let context = conf.test.clone().unwrap_or_default();
    if context.frameworks.is_empty() {
        idle(request, key, "no test framework", reply);
        return 0;
    }
    let (module_kind, js_env) = match context.js_test() {
        Ok((kind, env)) => (kind, env.clone()),
        Err(why) => {
            failed(request, key, why, reply);
            return 2;
        }
    };
    let Some(link) = export.test_link(key) else {
        failed(request, key, "the export does not say how the build tool runs its tests: export it again", reply);
        return 2;
    };
    // The configuration's own resident links its program: held through the run, so that no other
    // build rewrites the link node reads.
    let slot = slot(state, key);
    let mut resident = slot.lock().unwrap_or_else(|e| e.into_inner());
    if !built(state, export, request, key, &key.project, &mut resident, reply, Summary::Err) {
        return 1;
    }
    let built_ms = started.elapsed().as_millis();
    let r = resident.as_ref().expect("built");
    let view = &r.analysis;
    if let Some(why) = &view.unreadable {
        failed(request, key, format!("the analysis does not read: {}", why), reply);
        return 1;
    }
    let input = match (module_kind, r.es_module == Some(true)) {
        (ModuleKind::EsModule, _) => jsrunner::Input::EsModule(link.out.clone()),
        (ModuleKind::NoModule, false) => jsrunner::Input::Script(link.out.clone()),
        (ModuleKind::CommonJs, false) => jsrunner::Input::CommonJs(link.out.clone()),
        (kind, true) => {
            failed(
                request,
                key,
                format!(
                    "the test link imports or exports JavaScript modules, and the export's module kind is {}: teq writes an ES module for such a program, which a {} run cannot load (set the Test configuration's scalaJSLinkerConfig to ESModule)",
                    kind.name(),
                    kind.name()
                ),
                reply,
            );
            return 2;
        }
    };
    let jsdom = js_env.kind == "jsdom";
    let env = jsrunner::Env {
        executable: js_env.executable.clone().unwrap_or_else(|| "node".to_string()),
        args: js_env.args.clone(),
        env: js_env.env.clone(),
        cwd: export.path("."),
        source_map: match js_env.source_map.as_deref() {
            Some("disable") => jsrunner::SourceMap::Disable,
            Some("enable") => jsrunner::SourceMap::Enable,
            _ => jsrunner::SourceMap::EnableIfAvailable,
        },
        jsdom: jsdom.then(|| export.path(js_env.package_root.as_deref().unwrap_or("."))),
    };
    if jsdom {
        if let Err(e) = jsdom::ready(export, &js_env, &request.environment, &|line| reply.sink().line(line)) {
            failed(request, key, e, reply);
            return 2;
        }
    }
    let candidates = view.candidates(&conf.sources);
    // The run's stream on stdout: what the suites, their frameworks and node print, and teq's report of each event;
    // teq's own lines on stderr.
    let out = reply.shared_out();
    let finished = Finished(Arc::new(AtomicBool::new(false)));
    let mut run = match jsrunner::JsRun::start(&env, &input, &request.environment, asked.verbose, out.clone()) {
        Ok(run) => run,
        Err(e) => {
            failed(request, key, e, reply);
            return 1;
        }
    };
    kill_when_gone(reply, &finished, run.process(), key);
    let classes: Vec<String> = context.frameworks.iter().map(|f| f.class.clone()).collect();
    let found = match run.detect(&classes) {
        Ok(found) => found,
        Err(e) => {
            failed(request, key, e, reply);
            not_started(request, key, &context.groups, reply);
            return 1;
        }
    };
    let mut prints: Vec<runner::Fingerprint> = Vec::new();
    for (i, f) in found.iter().enumerate() {
        match f {
            Some(info) => {
                for (j, (kind, module, name, no_args)) in info.fingerprints.iter().enumerate() {
                    prints.push(runner::Fingerprint { framework: i, index: j, kind: *kind, module: *module, name: name.clone(), no_args: *no_args });
                }
            }
            // A framework the export names that the link does not hold is left out, as sbt-scalajs's
            // `loadedTestFrameworks` keeps only those its link answers for; teq links TASTy alone,
            // so one whose jar is Scala 2's IR alone (Scala.js's JUnit runtime, which munit brings)
            // is never in it, and the note says its suites, if any, do not run.
            None => {
                reply.err(&format!("teq: note: {}: the test framework {} is not in the test link (its jar holds no TASTy): its suites, if any, do not run\n", key.project, classes[i]));
            }
        }
    }
    let defined = suites::defined(&candidates, &prints);
    if defined.is_empty() {
        let _ = run.finish();
        failed(request, key, format!("no suite of the test frameworks ({}) among the test sources: a run of nothing is no pass", classes.join(", ")), reply);
        return 1;
    }
    let digests = source_digests(export, view);
    // Under `--changed`, the suites whose sources depend on a source edited since the last full run, or those of a
    // source deleted since; none when every suite runs, which is said.
    let affected = if asked.changed {
        let changes = state.changes.lock().unwrap_or_else(|e| e.into_inner());
        let why_every = match changes.get(key) {
            None => Some("no earlier run in this daemon's life".to_string()),
            Some(c) if c.resident != *key || c.all => Some("its resident started anew since the last run".to_string()),
            Some(_) => view.deps_missing.as_ref().map(|why| format!("the dependencies are unavailable: {}", why)),
        };
        match (why_every, changes.get(key)) {
            (Some(why), _) => {
                told(request, key, std::slice::from_ref(&key.project), &format!("every suite, {}", why), reply);
                None
            }
            (None, Some(c)) => {
                let edited: BTreeSet<String> = digests.iter().filter(|(f, d)| c.sources.get(*f) != Some(*d)).map(|(f, _)| f.clone()).collect();
                let mut affected = view.affected_by_sources(&edited);
                affected.extend(c.removed.iter().cloned());
                Some(affected)
            }
            (None, None) => None,
        }
    } else {
        None
    };
    let plan = selection(defined, &context, asked, affected.as_ref(), export.exporter);
    // A listing is the plan a run with the same arguments would run, the change history left as it is.
    if asked.list {
        out(&suites::listing(&plan.phases, &context.groups));
        return match run.finish() {
            Ok(()) => 0,
            Err(e) => {
                failed(request, key, e, reply);
                1
            }
        };
    }
    let selected: Vec<Vec<suites::Suite>> = plan.phases.iter().map(|p| suites::assign(p, &prints)).collect();
    let count: usize = selected.iter().map(Vec::len).sum();
    let every = asked.patterns.is_empty();
    let record = |state: &State| {
        state.changes.lock().unwrap_or_else(|e| e.into_inner()).insert(key.clone(), Changes { resident: key.clone(), all: false, files: BTreeSet::new(), sources: digests.clone(), removed: BTreeSet::new() });
    };
    if count == 0 {
        let _ = run.finish();
        let why = if asked.changed { "no suite depends on what changed" } else { "no suite to run" };
        idle(request, key, why, reply);
        if every {
            record(state);
        }
        return 0;
    }
    let groups = &context.groups;
    let sizes: Vec<usize> = selected.iter().map(Vec::len).collect();
    let mut tallies: Vec<Tally> = selected.iter().map(|_| Tally::default()).collect();
    let mut ended = vec![false; selected.len()];
    let run_started = Instant::now();
    // The phases one after another, as sbt runs its groups in process (`Defaults.allTestGroupsTask`): one controller
    // runner for each framework one of whose fingerprints matches a selected suite's, in any phase
    // (`filteredFrameworks`), all made before the first phase (`createTestRunners`), each phase's tasks asked of them,
    // then each runner's `done` once every phase has run. A failure of the bridge to make
    // a runner, its tasks or its `done` is the run's, as sbt lets those throw through; a channel that fails is the
    // error.
    let mut made = vec![false; context.frameworks.len()];
    let mut outcome: Result<Option<Thrown>, String> = Ok(None);
    // The phase a failure of the bridge happened in, none outside every phase.
    let mut failed_in: Option<usize> = None;
    for (i, framework) in context.frameworks.iter().enumerate() {
        let Some(info) = &found[i] else { continue };
        if !prints.iter().any(|p| p.framework == i && plan.selected.iter().any(|(_, d)| d.kind == p.kind && d.module == p.module && d.print == p.name)) {
            continue;
        }
        match run.create(i as i32, &info.implementation, &framework.arguments, &mut tallies[0]) {
            Ok(Ok(())) => made[i] = true,
            Ok(Err(thrown)) => {
                outcome = Ok(Some(thrown));
                break;
            }
            Err(e) => {
                outcome = Err(e);
                break;
            }
        }
    }
    'phases: for (p, suites) in selected.iter().enumerate() {
        if !matches!(outcome, Ok(None)) {
            break;
        }
        entered(request, key, groups, p, sizes[p], reply);
        let phase_started = Instant::now();
        for i in (0..made.len()).filter(|&i| made[i]) {
            let defs: Vec<jsrunner::TaskDef> = suites.iter().filter(|s| s.framework == i).map(|s| jsrunner::TaskDef { name: s.name.clone(), fingerprint: prints.iter().find(|p| p.framework == i && p.index == s.fingerprint).expect("assigned").clone(), explicit: asked.patterns.names(&s.name) }).collect();
            if defs.is_empty() {
                continue;
            }
            match run.run_tasks(i as i32, &defs, request.tty, &mut tallies[p]) {
                Ok(Ok(())) => {}
                Ok(Err(thrown)) => {
                    outcome = Ok(Some(thrown));
                    failed_in = Some(p);
                    break 'phases;
                }
                Err(e) => {
                    outcome = Err(e);
                    failed_in = Some(p);
                    break 'phases;
                }
            }
        }
        ended[p] = true;
        group_summary(request, key, groups, p, sizes[p], &tallies[p], phase_started.elapsed().as_millis() as u64, reply);
    }
    if matches!(outcome, Ok(None)) {
        let last = tallies.len() - 1;
        for i in (0..made.len()).filter(|&i| made[i]) {
            match run.done(i as i32, &mut tallies[last]) {
                Ok(Ok(())) => {}
                Ok(Err(thrown)) => {
                    outcome = Ok(Some(thrown));
                    break;
                }
                Err(e) => {
                    outcome = Err(e);
                    break;
                }
            }
        }
    }
    let outcome = match outcome {
        Ok(None) => run.finish().map(|()| None),
        Ok(Some(thrown)) => {
            let _ = run.finish();
            Ok(Some(thrown))
        }
        Err(e) => Err(e),
    };
    drop(finished);
    let ms = run_started.elapsed().as_millis() as u64;
    match outcome {
        Ok(None) => {
            if every {
                record(state);
            }
            let total = Tally::sum(&tallies);
            summary(request, key, count, &total, ms, built_ms, reply);
            write_events(asked, key, groups, &tallies, reply);
            i32::from(total.failed() > 0)
        }
        Ok(Some(thrown)) => {
            reply.out(&events::fork_error(&thrown, asked.verbose));
            run_failed(request, key, groups, failed_in, thrown.text.lines().next().unwrap_or("a framework's failure"), built_ms, reply);
            not_completed(request, key, groups, &sizes, &ended, reply);
            write_events(asked, key, groups, &tallies, reply);
            1
        }
        Err(e) => {
            // A node that ended (a suite's process.exit, a channel lost) ends the run where it is, the events of what
            // ran before written with their groups.
            failed_in_group(request, key, groups, failed_in, e, reply);
            not_completed(request, key, groups, &sizes, &ended, reply);
            write_events(asked, key, groups, &tallies, reply);
            1
        }
    }
}

/// The line that ends a configuration's run, on stderr: its suites, the events by status, the run's time and the
/// build's; and its record (`tests_ended`), the counts a reader takes.
fn summary(request: &Request, key: &Key, suites: usize, tally: &Tally, ms: u64, built_ms: u128, reply: &mut Reply) {
    reply.err(&summary_line(key, suites, tally, ms, built_ms));
    let mut counts = vec![("suites", Json::Num(suites as f64))];
    counts.extend(tally.by_status().into_iter().filter(|(_, n)| *n > 0).map(|(status, n)| (status, Json::Num(n as f64))));
    counts.extend([("ms", Json::Num(ms as f64)), ("built_ms", Json::Num(built_ms as f64))]);
    tests_ended(request, key, counts, reply);
}

fn summary_line(key: &Key, suites: usize, tally: &Tally, ms: u64, built_ms: u128) -> String {
    format!(
        "{}: {} suite{}, {} passed, {} failed{} in {} (built in {})\n",
        key.project,
        suites,
        if suites == 1 { "" } else { "s" },
        tally.passed(),
        tally.failed(),
        tally.others(),
        duration_of_ms(ms as f64),
        duration_of_ms(built_ms as f64)
    )
}

/// A configuration's tests ended with nothing run (`<project>: <why>`), on stderr, and recorded (`tests_ended`).
fn idle(request: &Request, key: &Key, why: &str, reply: &mut Reply) {
    reply.err(&format!("{}: {}\n", key.project, why));
    tests_ended(request, key, vec![("idle", why.into())], reply);
}

/// The resident's build in its slot, held by the caller, for the project `name` asked for: its diagnostics
/// and notes printed, its outcome recorded, and a failed build's count where `summary` says (`test`'s on stderr
/// beside its diagnostics, apart from the run's stream on stdout); whether it passed.
#[allow(clippy::too_many_arguments)]
fn built(state: &State, export: &Export, request: &Request, owner: &Key, name: &str, resident: &mut Option<Resident>, reply: &mut Reply, summary: Summary) -> bool {
    let named = [name.to_string()];
    let sink = reply.sink();
    let (answer, notes) = build_in(state, export, owner, &request.env, resident, &|line| sink.line(line));
    for note in notes {
        noted(request, owner, &note, reply);
    }
    let passed = match &answer {
        Err(e) => {
            failed_as(request, owner, &named, e, reply);
            false
        }
        Ok(a) => {
            for n in &a.notes {
                told(request, owner, &named, n, reply);
            }
            for w in &a.warnings {
                shown(request, owner, w, "warning", &export.root, reply);
            }
            if !a.ok {
                for e in &a.errors {
                    shown(request, owner, e, "error", &export.root, reply);
                }
                let n = a.errors.len().max(1);
                let line = format!("{}\n", super::progress::errors_line(name, n, &elsewhere(export, owner, a, &named)));
                match summary {
                    Summary::Out => reply.out(&line),
                    Summary::Err => reply.err(&line),
                }
            }
            a.ok
        }
    };
    outcome(request, owner, &named, answer.as_ref(), reply);
    passed
}

/// `stage [project]`: the project's closure built as `compile` builds it, then its Docker stage
/// written from the export's mappings (`stage.rs`) while its resident is held, so that no other
/// build rewrites the class files the jars are made of; without a project, every project's with
/// a stage block.
fn stage(state: &Arc<State>, export: &Arc<Export>, request: &Request, reply: &mut Reply) -> i32 {
    let staged = || export.projects.iter().filter(|(_, p)| p.stage.is_some()).map(|(n, _)| n.clone()).collect::<Vec<_>>();
    // A project whose stage the export does not reproduce has no stage block and says why.
    let stages = |n: &String| export.projects[n].stage.is_some() || export.projects[n].unsupported.contains_key("stage");
    let mut left_out = Vec::new();
    let names = match request.args.as_slice() {
        [] => {
            let (ok, refused): (Vec<String>, Vec<String>) = export.projects.keys().filter(|n| stages(n)).cloned().partition(|n| export.refusals("stage", n).is_empty());
            left_out = refused;
            ok
        }
        [name] if export.projects.contains_key(name) && stages(name) && !export.refusals("stage", name).is_empty() => {
            reply.err(&format!("teq: {}\n", super::refused(export, "stage", &[name]).unwrap_or_default()));
            return 2;
        }
        [name] => match export.projects.get(name) {
            Some(p) if p.stage.is_some() => vec![name.clone()],
            Some(_) => {
                let with = staged();
                let with = if with.is_empty() { "none has".to_string() } else { format!("{} {}", with.join(", "), if with.len() == 1 { "has" } else { "have" }) };
                reply.err(&format!("teq: {} has no stage block, which the export writes for a project with sbt-native-packager's JavaAppPackaging and DockerPlugin ({} one)\n", name, with));
                return 2;
            }
            None => {
                reply.err(&format!("teq: no project {} in {} (projects: {})\n", name, export.file.display(), export.projects.keys().cloned().collect::<Vec<_>>().join(", ")));
                return 2;
            }
        },
        _ => {
            reply.err("usage: teq stage [project]\n");
            return 2;
        }
    };
    if names.is_empty() && left_out.is_empty() {
        reply.out("teq: no project with a stage block\n");
        return 0;
    }
    let code = names.iter().map(|name| stage_project(state, export, request, name, reply)).max().unwrap_or(0);
    reply.out(&super::left_out(export, "stage", &left_out));
    super::with_left_out(code, &left_out)
}

fn stage_project(state: &Arc<State>, export: &Arc<Export>, request: &Request, name: &str, reply: &mut Reply) -> i32 {
    let started = Instant::now();
    let key = Key::new(name, "compile");
    let running = running(state);
    let Some(owner) = plan(export, &running, std::slice::from_ref(&key)).into_iter().next() else { return 2 };
    let slot = slot(state, &owner);
    let mut resident = slot.lock().unwrap_or_else(|e| e.into_inner());
    if !built(state, export, request, &owner, name, &mut resident, reply, Summary::Out) {
        return 1;
    }
    let built_ms = started.elapsed().as_millis();
    let view = &resident.as_ref().expect("built").analysis;
    if let Some(why) = &view.unreadable {
        failed_as(request, &owner, &[name.to_string()], &format!("the analysis does not read: {}", why).into(), reply);
        return 1;
    }
    // The class the start script runs: the block's, which the build declared, else the one the
    // products imply as `run` implies it (the same choice, the stage's own words: it takes no
    // class, a declaration is the way), refused before anything is written, where
    // native-packager would write a start script per class, or none. The project's own jar's
    // manifest names it too, unless the build set `Compile / mainClass := None`, where sbt's
    // `packageBin` writes no `Main-Class`.
    let setting = &export.projects[name].stage.as_ref().expect("a stage block").main_class;
    let main = match setting {
        MainClass::Declared(main) => main.clone(),
        MainClass::DeclaredNone | MainClass::Unset => match super::implied(product_mains(export, view, name)) {
            Implied::One(main) => main,
            Implied::Zero => {
                reply.err(&format!("teq: no main class in {}'s products, which native-packager would stage without a start script\n", name));
                return 2;
            }
            Implied::Several(several) => {
                reply.err(&format!("teq: {} declares no main class and its products hold {}, for which native-packager would write a start script each: {}; set Compile / mainClass := Some(one of them) and export again\n", name, several.len(), several.join(", ")));
                return 2;
            }
        },
    };
    let in_manifest = *setting != MainClass::DeclaredNone;
    match stage::write(export, name, &export.path(&export.classes(&owner)), view, &main, in_manifest) {
        Ok(done) => {
            let layers: Vec<String> = done.layers.iter().map(u32::to_string).collect();
            reply.out(&format!(
                "{}: staged {} files in layers {} under {} in {} (built in {})\n",
                name,
                done.files,
                layers.join(", "),
                done.directory,
                duration_of_ms((started.elapsed().as_millis() - built_ms) as f64),
                duration_of_ms(built_ms as f64)
            ));
            0
        }
        Err(e) => {
            failed_as(request, &owner, &[name.to_string()], &e.into(), reply);
            1
        }
    }
}

/// Set when a request is done with its runner, which a client leaving after that leaves alone.
struct Finished(Arc<AtomicBool>);

impl Drop for Finished {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// Kills the runner's process when the request's client leaves before the request is done with
/// it.
/// A runner launched, registered for an ending daemon's kill pass, and its frameworks loaded; `watched`, the client
/// waiting for it, has it killed when it leaves.
fn start_runner(state: &State, key: &Key, spec: Spec, argfile: &Path, watched: Option<(&Reply, &Finished)>) -> Result<Runner, Failure> {
    launch_runner(state, key, spec, argfile).and_then(|mut r| {
        if let Some((reply, finished)) = watched {
            kill_when_gone(reply, finished, r.process(), key);
        }
        match r.handshake() {
            Ok(()) => Ok(r),
            Err(e) => {
                r.stop();
                Err(e.into())
            }
        }
    })
}

/// A runner launched and registered for an ending daemon's kill pass.
fn launch_runner(state: &State, key: &Key, spec: Spec, argfile: &Path) -> Result<Runner, Failure> {
    // Launched under the registry's lock, so that an ending daemon's kill pass either sees the
    // process or the launch sees the daemon ending.
    let mut processes = state.runner_processes.lock().unwrap_or_else(|e| e.into_inner());
    match state.ending.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        Some(why) => Err(Failure::crash(format!("the daemon is ending ({})", why))),
        None => Runner::launch(spec, argfile).map_err(Failure::from).inspect(|r| {
            processes.insert(key.clone(), r.process());
        }),
    }
}

/// A runner serves one invocation, as sbt's forked worker serves one request (`ForkTests` starts a worker for each
/// run): a run inherits no thread, static or registration of the one before, which a JVM kept across runs cannot shed
/// (a library's executor keeps the class loader of the run that made its threads as their context loader, its objects
/// keep their state), and a runner that loaded the frameworks holds that build's classes. Once its invocation ends (a
/// run, a listing, a selection of nothing), the spent runner quits, on a thread of its own and kept among the retiring runners until it has
/// (an ending daemon waits for them within the stop bound); one whose run `failed` at its fork error is let finish its
/// run first, its output drained, within the stop bound (`Runner::retire`). The next is launched with its spec before the
/// client is answered: its JVM starts while the client reads the run, nothing of the test class path loaded until the
/// next `test` sends it `load` after its build, and the next `test` reads its version. None when it cannot be
/// launched, the next `test` starting its own.
fn renew(state: &State, spent: Runner, key: &Key, argfile: &Path, failed: bool) -> Option<Runner> {
    let spec = spent.spec.clone();
    if failed {
        log(&format!("stopping the test runner of {}: its run failed, its tasks in flight and done() given {} s", key, STOP_BOUND.as_secs()));
    } else {
        log(&format!("stopping the test runner of {}: its run is done", key));
    }
    // Kept until its stop has completed, which an ending daemon waits for.
    let stopped = Arc::new(AtomicBool::new(false));
    {
        let mut retiring = state.retiring.lock().unwrap_or_else(|e| e.into_inner());
        retiring.retain(|r| !r.stopped.load(Ordering::SeqCst));
        retiring.push(Retiring { process: spent.process(), stopped: stopped.clone() });
    }
    crate::alloc::spawn(move || {
        if failed {
            spent.retire(STOP_BOUND);
        } else {
            spent.stop();
        }
        stopped.store(true, Ordering::SeqCst);
    });
    log(&format!("starting the test runner of {} for the next run: {}", key, spec.java.display()));
    launch_runner(state, key, spec, argfile).inspect_err(|e| log(&format!("the test runner of {} for the next run did not start: {}", key, e.message()))).ok()
}

fn kill_when_gone(reply: &Reply, finished: &Finished, process: Arc<Mutex<std::process::Child>>, key: &Key) {
    let Ok(mut client) = reply.stream.try_clone() else { return };
    let (finished, key) = (finished.0.clone(), key.clone());
    crate::alloc::spawn(move || {
        let mut buf = [0u8; 256];
        while matches!(client.read(&mut buf), Ok(n) if n > 0) {}
        if !finished.load(Ordering::SeqCst) {
            log(&format!("killing the test runner of {}: its client left during the run", key));
            let _ = process.lock().unwrap_or_else(|e| e.into_inner()).kill();
        }
    });
}

/// The runner a test configuration's suites run on: the client's `java`, the configuration's
/// options, directory and variables, and test-interface alone, taken from `paths`, the test class path
/// (`runner::interface_dir`).
fn runner_spec(export: &Export, key: &Key, context: &TestContext, environment: &BTreeMap<String, String>, paths: &[PathBuf]) -> Result<Spec, String> {
    let interface = runner::interface_dir(paths)?.ok_or("no entry of the test class path holds sbt's test interface (sbt/testing/Framework.class), which the test runner runs on")?;
    Ok(Spec {
        java: runner::java(environment)?,
        least: export.java_output_version,
        options: context.java_options.clone(),
        directory: if context.sandbox { export.sandbox_directory(key) } else { export.path(&context.base_directory) },
        env: context.env(export),
        environment: environment.clone(),
        interface,
    })
}

/// A directory made empty, created when missing: what it holds removed, the directory itself kept,
/// since a test runner started in it keeps it as its working directory.
fn empty_directory(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {}", dir.display(), e))?;
    for entry in std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {}", dir.display(), e))?.flatten() {
        let path = entry.path();
        let removed = if entry.file_type().is_ok_and(|k| k.is_dir()) { std::fs::remove_dir_all(&path) } else { std::fs::remove_file(&path) };
        removed.map_err(|e| format!("cannot empty {}: {}", path.display(), e))?;
    }
    Ok(())
}

/// The test class path a run reads through its own class loader, in its order, as sbt's worker is sent it
/// (`ForkTests.mainTestTask`): the resident's class files, the configuration's and every project's it depends on, the
/// configuration's resources, then each entry of its run class path, a product's resources, an artifact's jar, a
/// file of the tree; those that exist, each once.
fn run_paths(export: &Export, key: &Key, owner: &Key) -> Result<Vec<PathBuf>, String> {
    let conf = export.configuration(key).expect("a test configuration");
    let mut out = vec![export.path(&export.classes(owner))];
    out.extend(conf.resources.iter().map(|r| export.path(r)));
    for entry in conf.run_classpath() {
        match entry {
            Entry::Product(k) => out.extend(export.configuration(k).map_or(&[][..], |c| &c.resources).iter().map(|r| export.path(r))),
            entry => out.push(export.resolve(entry)?),
        }
    }
    let mut seen = BTreeSet::new();
    out.retain(|p| p.exists() && seen.insert(p.clone()));
    Ok(out)
}

/// One resident's build: started when it is not running, its closure's generators run first.
fn build(state: &Arc<State>, export: &Arc<Export>, key: &Key, env: &BTreeMap<String, String>, progress: &(dyn Fn(&str) + Sync)) -> (Result<Answer, Failure>, Vec<String>) {
    let slot = slot(state, key);
    let mut resident = slot.lock().unwrap_or_else(|e| e.into_inner());
    build_in(state, export, key, env, &mut resident, progress)
}

/// The residents started: those whose slot holds one, and those whose slot a build or a run holds,
/// which is not waited for.
fn running(state: &State) -> Vec<Key> {
    let residents = state.residents.lock().unwrap_or_else(|e| e.into_inner());
    let started = |slot: &Slot| match slot.try_lock() {
        Ok(resident) => resident.is_some(),
        Err(std::sync::TryLockError::Poisoned(e)) => e.into_inner().is_some(),
        Err(std::sync::TryLockError::WouldBlock) => true,
    };
    residents.iter().filter(|(_, slot)| started(slot)).map(|(k, _)| k.clone()).collect()
}

fn slot(state: &State, key: &Key) -> Slot {
    state.residents.lock().unwrap_or_else(|e| e.into_inner()).entry(key.clone()).or_default().clone()
}

/// A build of the resident in its slot, held by the caller; what it wrote is recorded for the
/// test configurations it serves. A resident started fetches the artifacts its classpath lacks,
/// telling `progress` of each.
fn build_in(state: &State, export: &Export, key: &Key, env: &BTreeMap<String, String>, resident: &mut Option<Resident>, progress: &(dyn Fn(&str) + Sync)) -> (Result<Answer, Failure>, Vec<String>) {
    if resident.is_none() {
        if let Err(e) = settle_classes_abi(export, key) {
            return (Err(e.into()), Vec::new());
        }
    }
    let closure = export.closure(key);
    let notes = match generators::run(export, &closure, resident_classes(export, key), env) {
        Ok(notes) => notes,
        Err(e) => return (Err(e.into()), Vec::new()),
    };
    if let Some(link) = export.test_link(key) {
        if let Err(e) = export.write_harness(&link) {
            return (Err(e.into()), notes);
        }
    }
    let inputs = export.inputs_of(key);
    if resident.as_ref().is_some_and(|r| r.inputs != inputs) {
        log(&format!("restarting {}: its existing source roots changed", key));
        resident.take().expect("checked").stop();
    }
    if resident.is_none() {
        // What its first passing build writes is only what differs from the class files on disk.
        for c in state.changes.lock().unwrap_or_else(|e| e.into_inner()).values_mut().filter(|c| c.resident == *key) {
            c.all = true;
        }
        let started = export.resident_args_with(key, progress).and_then(|args| {
            let file = export.resident_args_file(key);
            log(&format!("starting {}: {} @{}, which holds {}", key, state.exe.display(), file.display(), args.join(" ")));
            Resident::start(&state.exe, &export.root, key.clone(), args, &file, inputs)
        });
        match started {
            Ok(r) => *resident = Some(r),
            Err(e) => return (Err(e.into()), notes),
        }
    }
    let answer = resident.as_mut().expect("started").build().map_err(Failure::from);
    // What a removal reaches counts whether or not the build passed: the view has dropped it.
    if let Ok(a) = &answer {
        for c in state.changes.lock().unwrap_or_else(|e| e.into_inner()).values_mut().filter(|c| c.resident == *key) {
            c.removed.extend(a.removed_reach.iter().cloned());
        }
    }
    match &answer {
        Ok(a) if a.ok => {
            if a.first {
                log(&memory(key, resident.as_ref().expect("started")));
            }
            for c in state.changes.lock().unwrap_or_else(|e| e.into_inner()).values_mut().filter(|c| c.resident == *key) {
                c.files.extend(a.changed.iter().cloned());
            }
        }
        Ok(_) => {}
        // A resident that ended is started again by the next request.
        Err(_) => {
            if let Some(r) = resident.take() {
                r.stop();
            }
        }
    }
    (answer, notes)
}

/// The main classes among the products of a project's compile configuration, as sbt's
/// `discoveredMainClasses` finds them: the classes under its source roots that the resident's
/// merged analysis marks `main`, which the emitter recorded as it wrote them (no class file is
/// read; the mark is a flag per class of the view). The configuration exists: `compile` planned
/// the resident for it.
fn product_mains(export: &Export, view: &View, project: &str) -> Vec<String> {
    let roots = export.configuration(&Key::new(project, "compile")).map_or(&[][..], |c| c.sources.as_slice());
    view.main_classes_under(roots).into_iter().map(str::to_string).collect()
}

/// `product_mains` from the resident of the configuration that owns the project's closure, for
/// the `classes` request.
fn main_classes(state: &State, export: &Export, owner: &Key, project: &str) -> Result<Vec<String>, String> {
    let slot = slot(state, owner);
    let resident = slot.lock().unwrap_or_else(|e| e.into_inner());
    let view = &resident.as_ref().ok_or_else(|| format!("{} has no resident", project))?.analysis;
    if let Some(why) = &view.unreadable {
        return Err(format!("{}: the analysis does not read: {}", project, why));
    }
    Ok(product_mains(export, view, project))
}

/// What the resident's merged analysis holds, and the daemon's memory.
fn memory(key: &Key, resident: &Resident) -> String {
    let view = &resident.analysis;
    let footprint = match crate::alloc::footprint() {
        0 => String::new(),
        bytes => format!("; the daemon's footprint {:.1} MB", bytes as f64 / 1e6),
    };
    format!("{}: the merged analysis holds {} files, {} classes in {:.2} MB{}", key, view.file_count(), view.class_count(), view.heap_bytes() as f64 / 1e6, footprint)
}

/// The class directory a resident writes, which a `classDirectory` key of a generator names.
fn resident_classes(export: &Export, key: &Key) -> PathBuf {
    export.path(&export.classes(key))
}

/// The marker beside a JVM resident's class directory, `classes.abi`: the number of the products
/// whose class files it holds (`products::ABI`), beside rather than inside it so that no class path
/// holds it.
fn classes_abi_marker(classes: &Path) -> PathBuf {
    classes.with_file_name(format!("{}.abi", classes.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()))
}

/// Before a JVM resident starts: a class directory whose marker names another number of products,
/// or none (a teq up to 0.1.11 wrote it), is emptied, so that no class file of another number is
/// left beside the ones the resident writes, which deletes only those of its own earlier builds;
/// then the marker names this one's.
fn settle_classes_abi(export: &Export, key: &Key) -> Result<(), String> {
    if export.projects.get(&key.project).is_none_or(|p| p.platform != Platform::Jvm) {
        return Ok(());
    }
    let classes = resident_classes(export, key);
    let marker = classes_abi_marker(&classes);
    let ours = crate::products::ABI.to_string();
    let read = std::fs::read_to_string(&marker).ok();
    if read.as_deref().map(str::trim) == Some(ours.as_str()) {
        return Ok(());
    }
    if classes.exists() {
        log(&format!("emptying {}: its class files are of products ABI {}, this teq's are of {}", classes.display(), read.as_deref().map_or("0 (no marker)", str::trim), ours));
        std::fs::remove_dir_all(&classes).map_err(|e| format!("cannot empty {}: {}", classes.display(), e))?;
    }
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {}", parent.display(), e))?;
    }
    std::fs::write(&marker, format!("{}\n", ours)).map_err(|e| format!("cannot write {}: {}", marker.display(), e))
}

/// A diagnostic of `key`'s build shown on stderr (`render`), and recorded where the request names
/// a file for them (`--diagnostics`): a JSON line, the configuration, the severity, the file (as the
/// export has it), line and column, the whole message and the lines shown for it (`shown`), so that a
/// reader takes the diagnostic's extent from the build, never from the shown text, whose message
/// may hold any line.
fn shown(request: &Request, key: &Key, d: &Json, severity: &str, root: &Path, reply: &mut Reply) {
    let text = render(d, severity, root, &request.cwd);
    reply.err(&text);
    let mut fields = vec![("key", key.to_string().as_str().into()), ("severity", severity.into())];
    for name in ["file", "line", "col"] {
        if let Some(v) = d.get(name) {
            fields.push((name, v.clone()));
        }
    }
    fields.push(("message", d.get("message").and_then(Json::str).unwrap_or("").into()));
    fields.push(("shown", text.as_str().into()));
    record(request, fields, reply);
}

/// Why a build or a run ended without its answer: what its record says of it (its `kind`: `crash` for teq's own, a
/// build's thread that panicked, a compiler that ended, the daemon ending; `generator` for a generator that
/// failed; `other` for the rest; and its message, which names a configuration by its key) and what its line tells
/// a person, who asked for projects (`told`).
struct Failure {
    kind: &'static str,
    why: Why,
}

enum Why {
    Text(String),
    Generator(generators::Failed),
}

impl Failure {
    fn crash(why: impl Into<String>) -> Failure {
        Failure { kind: "crash", why: Why::Text(why.into()) }
    }

    fn message(&self) -> String {
        match &self.why {
            Why::Text(t) => t.clone(),
            Why::Generator(g) => g.to_string(),
        }
    }

    /// The failure in a line that names the projects `named`.
    fn told(&self, named: &[String]) -> String {
        match &self.why {
            Why::Text(t) => t.clone(),
            Why::Generator(g) => g.told(named),
        }
    }
}

impl From<String> for Failure {
    fn from(why: String) -> Failure {
        Failure { kind: "other", why: Why::Text(why) }
    }
}

impl From<&str> for Failure {
    fn from(why: &str) -> Failure {
        why.to_string().into()
    }
}

impl From<&String> for Failure {
    fn from(why: &String) -> Failure {
        why.clone().into()
    }
}

impl From<generators::Failed> for Failure {
    fn from(why: generators::Failed) -> Failure {
        Failure { kind: if why.of.is_some() { "generator" } else { "other" }, why: Why::Generator(why) }
    }
}

impl From<super::resident::NoAnswer> for Failure {
    fn from(why: super::resident::NoAnswer) -> Failure {
        match why {
            super::resident::NoAnswer::Gone(why) => Failure::crash(why),
            super::resident::NoAnswer::Unread(why) => why.into(),
        }
    }
}

/// A failure of `key`'s, a build's or a run's (`teq: <project>: <why>`), shown on stderr and recorded
/// whole where the request names a file for the records (`--diagnostics`, severity `failure`, with
/// its kind): a generator's carries its command's whole output, so that a reader takes the failure's
/// extent and what it is from teq, never from the text, whose lines are the command's.
fn failed(request: &Request, key: &Key, why: impl Into<Failure>, reply: &mut Reply) {
    failed_as(request, key, std::slice::from_ref(&key.project), &why.into(), reply);
}

/// `failed` in a line that names the projects asked for that the build of `key` covered.
fn failed_as(request: &Request, key: &Key, named: &[String], why: &Failure, reply: &mut Reply) {
    failure_line(request, key, named, why, None, reply);
}

/// THE ONE WRITER of a failure's line in a project's name (`teq: <projects>: <why>`), on stderr, with its record
/// (`--diagnostics`, severity `failure`, its kind, its message; its `group` where the failure is a run's of a
/// configuration with groups): a reader of the records sees every such line, since none is written elsewhere
/// (`tests::a_line_in_a_project_s_name_has_its_record`).
fn failure_line(request: &Request, key: &Key, named: &[String], why: &Failure, group: Option<Json>, reply: &mut Reply) {
    let text = format!("teq: {}: {}\n", named.join(", "), why.told(named));
    reply.err(&text);
    let mut fields = vec![("key", key.to_string().as_str().into()), ("severity", "failure".into()), ("kind", why.kind.into())];
    fields.extend(group.map(|g| ("group", g)));
    fields.extend([("message", why.message().as_str().into()), ("shown", text.as_str().into())]);
    record(request, fields, reply);
}

/// What teq says of `key`'s build or run that is no failure (`teq: <project>: <note>`: why a run takes every
/// suite, a build's own note), shown on stderr and recorded (`--diagnostics`, severity `note`).
fn told(request: &Request, key: &Key, named: &[String], note: &str, reply: &mut Reply) {
    let text = format!("teq: {}: {}\n", named.join(", "), note);
    reply.err(&text);
    record(request, vec![("key", key.to_string().as_str().into()), ("severity", "note".into()), ("message", note.into()), ("shown", text.as_str().into())], reply);
}

/// A build's outcome recorded (`--diagnostics`, severity `outcome`) after its diagnostics: the configuration built,
/// the projects asked for that it covered, its errors as its line counts them, whether it ended well, for one that
/// failed outright the failure's kind and message, and its time. A reader pairs a build with its diagnostics by
/// these records, never by a line a person reads.
fn outcome(request: &Request, key: &Key, named: &[String], result: Result<&Answer, &Failure>, reply: &mut Reply) {
    let key = key.to_string();
    let line = match result {
        Ok(a) => crate::source::outcome_record(Some(&key), named, a.ok, if a.ok { 0 } else { a.errors.len().max(1) }, None, Some(a.ms)),
        Err(why) => crate::source::outcome_record(Some(&key), named, false, 0, Some((why.kind, &why.message())), None),
    };
    append(request, &line, reply);
}

/// The end of a configuration's tests recorded (`--diagnostics`, severity `tests`): `ended` is its summary's counts
/// (`suites`, each status that has events, `ms`, `built_ms`), `idle` with why nothing ran, or `run_failed` with why
/// the run failed.
fn tests_ended(request: &Request, key: &Key, ended: Vec<(&str, Json)>, reply: &mut Reply) {
    let mut fields = vec![("key", key.to_string().as_str().into()), ("severity", "tests".into())];
    fields.extend(ended);
    record(request, fields, reply);
}

/// What a generator of `key`'s build printed as it passed (`generators::run`'s notes, one a
/// generator), shown on stderr and recorded whole (`--diagnostics`, severity `note`): its text is
/// the command's, never one of teq's own lines.
fn noted(request: &Request, key: &Key, note: &str, reply: &mut Reply) {
    let text = format!("{}\n", note);
    reply.err(&text);
    record(request, vec![("key", key.to_string().as_str().into()), ("severity", "note".into()), ("message", note.into()), ("shown", text.as_str().into())], reply);
}

/// One record appended to the request's `--diagnostics` file, a JSON line, where it names one.
fn record(request: &Request, fields: Vec<(&str, Json)>, reply: &mut Reply) {
    let mut line = Json::Obj(fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect()).to_text();
    line.push('\n');
    append(request, &line, reply);
}

/// A record's line appended to the request's `--diagnostics` file, where it names one.
fn append(request: &Request, line: &str, reply: &mut Reply) {
    let Some(file) = &request.diagnostics else { return };
    let written = std::fs::OpenOptions::new().append(true).open(file).and_then(|mut f| f.write_all(line.as_bytes()));
    if let Err(e) = written {
        reply.err(&format!("teq: cannot write {}: {}\n", file.display(), e));
    }
}

/// A diagnostic as a terminal shows it: the file relative to the client's directory when it lies
/// under it, the line and column, the message, the source line and the caret.
fn render(d: &Json, severity: &str, root: &Path, cwd: &Path) -> String {
    let message = d.get("message").and_then(Json::str).unwrap_or("");
    let mut out = match d.get("file").and_then(Json::str).filter(|f| !f.is_empty()) {
        Some(file) => {
            let path = root.join(file);
            let shown = path.strip_prefix(cwd).map(Path::to_path_buf).unwrap_or(path);
            let at = [d.get("line"), d.get("col")].iter().filter_map(|v| v.and_then(Json::num)).map(|n| format!(":{}", n as u64)).collect::<String>();
            format!("{}{}: {}: {}\n", shown.display(), at, severity, message)
        }
        None => format!("teq: {}: {}\n", severity, message),
    };
    for key in ["source", "caret"] {
        if let Some(text) = d.get(key).and_then(Json::str).filter(|t| !t.is_empty()) {
            out.push_str(text);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_residents_a_compile_needs() {
        let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("integrations/sbt/example/teq.lock");
        let export = Export::read(&file).unwrap();
        let compile = |names: &[&str]| names.iter().map(|n| Key::new(n, "compile")).collect::<Vec<_>>();
        let none: Vec<Key> = Vec::new();
        assert_eq!(plan(&export, &none, &compile(&["api"])), [Key::new("api", "test")]);
        assert_eq!(plan(&export, &none, &compile(&["jvmcore", "jvmapp"])), [Key::new("jvmapp", "test")]);
        let all: Vec<&str> = export.projects.keys().map(String::as_str).collect();
        let mut planned = plan(&export, &none, &compile(&all));
        planned.sort();
        assert_eq!(planned, ["api", "browserdemo", "frontend", "jvmapp", "mirrored", "sjsapp"].map(|p| Key::new(p, "test")));
        assert_eq!(plan(&export, &[Key::new("jvmcore", "compile")], &compile(&["jvmcore"])), [Key::new("jvmcore", "compile")]);
        assert_eq!(plan(&export, &[Key::new("jvmapp", "test")], &compile(&["jvmcore"])), [Key::new("jvmapp", "test")]);
        // sjsapp's Scala.js check reads jvmcore's sources but writes no class file of them.
        assert_eq!(plan(&export, &[Key::new("sjsapp", "test")], &compile(&["jvmcore"])), [Key::new("jvmcore", "test")]);
        assert_eq!(plan(&export, &[Key::new("jvmapp", "test")], &compile(&["sjscore"])), [Key::new("sjscore", "test")]);
    }

    #[test]
    fn a_resident_types_what_its_inputs_hold() {
        let dir = std::env::temp_dir().join(format!("teq-daemon-plan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let text = "teq: 0.1.2
format: 1
binaries: {}
projects:
  app:
    configurations:
      compile:
        classpath:
          - {configuration: compile, project: core}
        sources:
          - app/src
    description:
      sources:
        - app/src
    platform: jvm
  core:
    configurations:
      compile:
        sources:
          - core/src
    platform: jvm
";
        std::fs::write(dir.join("teq.lock"), text).unwrap();
        let export = Export::read(&dir.join("teq.lock")).unwrap();
        let compile = |names: &[&str]| names.iter().map(|n| Key::new(n, "compile")).collect::<Vec<_>>();
        // app's description leaves core's sources out: its resident does not type core.
        assert_eq!(plan(&export, &[], &compile(&["app", "core"])), [Key::new("app", "compile"), Key::new("core", "compile")]);
        assert_eq!(plan(&export, &[Key::new("app", "compile")], &compile(&["core"])), [Key::new("core", "compile")]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A Mill module's configurations, not transitive, each a resident of its own: a compile is never served by
    /// the test's resident (whose class path is wider) nor by a dependent's, and a test's plan ends with the test's
    /// own resident (it once looped, the fallback widest no resident of the test).
    #[test]
    fn a_configuration_that_is_not_transitive_gets_a_resident_of_its_own() {
        let dir = std::env::temp_dir().join(format!("teq-plan-mill-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let text = "teq: 0.1.2
format: 1
binaries: {}
exporter: mill
projects:
  app:
    configurations:
      compile:
        classpath:
          - {configuration: compile, project: core}
        sources:
          - app/src
        transitive: false
      test:
        classpath:
          - {configuration: compile, project: app}
        frameworks: []
        sources:
          - app/test
        transitive: false
    platform: jvm
  core:
    configurations:
      compile:
        sources:
          - core/src
        transitive: false
    platform: jvm
";
        std::fs::write(dir.join("teq.lock"), text).unwrap();
        let export = Export::read(&dir.join("teq.lock")).unwrap();
        let (app, app_test, core) = (Key::new("app", "compile"), Key::new("app", "test"), Key::new("core", "compile"));
        assert_eq!(plan(&export, &[], std::slice::from_ref(&app_test)), [app_test.clone()]);
        assert_eq!(plan(&export, &[], std::slice::from_ref(&app)), [app.clone()]);
        assert_eq!(plan(&export, std::slice::from_ref(&app_test), std::slice::from_ref(&app)), [app.clone()]);
        assert_eq!(plan(&export, std::slice::from_ref(&app), std::slice::from_ref(&core)), [core.clone()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_test_generator_sbt_runs_leaves_the_compile_configuration_to_type() {
        let text = "teq: 0.1.2
format: 1
binaries: {}
projects:
  app:
    configurations:
      compile:
        sources:
          - app/src
      test:
        classpath:
          - {configuration: compile, project: app}
        generators:
          - kind: sbt
            task: an unnamed task
        sources:
          - app/test
    platform: jvm
";
        let export = Export::parse(Path::new("/b/teq.lock"), text.as_bytes()).unwrap();
        let compile = [Key::new("app", "compile")];
        assert_eq!(plan(&export, &[], &compile), compile);
        // A resident of the test configuration from before the generator was recorded is passed over.
        assert_eq!(plan(&export, &[Key::new("app", "test")], &compile), compile);
    }

    #[test]
    fn diagnostics_as_a_terminal_shows_them() {
        let d = Json::parse(r#"{"file":"src/A.scala","line":3,"col":7,"message":"type mismatch","source":"  val x: Int = \"\"","caret":"              ^^"}"#).unwrap();
        assert_eq!(render(&d, "error", Path::new("/b"), Path::new("/b")), "src/A.scala:3:7: error: type mismatch\n  val x: Int = \"\"\n              ^^\n");
        let outside = format!("{}:3:7: error: type mismatch", Path::new("/b").join("src/A.scala").display());
        assert_eq!(render(&d, "error", Path::new("/b"), Path::new("/elsewhere")).lines().next(), Some(outside.as_str()));
        let bare = Json::parse(r#"{"message":"cannot open x.jar"}"#).unwrap();
        assert_eq!(render(&bare, "error", Path::new("/b"), Path::new("/b")), "teq: error: cannot open x.jar\n");
    }

    /// A line in a project's name (`teq: <project>: ..`) is written by the two functions that record it: a failure
    /// by `failure_line`, what is no failure by `told`. One written anywhere else in src/task's daemon would have
    /// no record, and the gate's parser, which reads the records alone, would never see it. (`render`'s is a
    /// diagnostic without a file, `teq: <severity>: ..`, recorded by `shown`.)
    #[test]
    fn a_line_in_a_project_s_name_has_its_record() {
        let source = include_str!("daemon.rs");
        let needle = ["\"teq: ", "{}: "].concat();
        let writers: Vec<&str> = source
            .match_indices(&needle)
            .map(|(at, _)| {
                let before = &source[..at];
                let start = before.rfind("\nfn ").map_or(0, |i| i + 4);
                before[start..].split('(').next().unwrap_or("").trim()
            })
            .collect();
        assert_eq!(writers, ["failure_line", "told", "render"]);
    }
}
