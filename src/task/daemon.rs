//! The session daemon of an export: started by the first `teq` of a build, it holds the
//! residents and answers the clients that connect to its loopback port, which it writes with a
//! token into `target/teq/daemon.json`; a request without the token is refused. Every request
//! then carries the handshake (docs/TARGETS.md, "The export and the project verbs"): a client of another protocol
//! or another binary, or one whose export pins another compiler, is told to start a new daemon,
//! and this one finishes what it is doing and exits; an export otherwise changed is read again
//! in place, the residents whose command lines it changed stopped. The daemon holds
//! `target/teq/task.lock` for its whole life, so that the next one writes `daemon.json` only once
//! this one is gone. It holds a warm test runner per test configuration too, started by the
//! configuration's first `test` and again when what it was started with changed or it died.

use super::analysis::View;
use super::export::{Entry, Export, Key, MainClass, Platform, TestContext};
use super::generators;
use super::resident::{Answer, Resident};
use super::runner::{self, Jar, Runner, Spec};
use super::stage;
use super::suites::{self, Patterns};
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

/// The class files a test configuration's resident wrote since the configuration's last run of
/// every suite that could be affected; all of them when the resident started anew since.
struct Changes {
    resident: Key,
    all: bool,
    files: BTreeSet<String>,
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
    let state = Arc::new(State { export: Mutex::new(Arc::new(export)), identity, exe, paths, token, port, residents: Mutex::new(BTreeMap::new()), runners: Mutex::new(BTreeMap::new()), runner_processes: Mutex::new(BTreeMap::new()), changes: Mutex::new(BTreeMap::new()), ending: Mutex::new(None), in_flight: Default::default() });
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

    /// Where the request's builds, on threads of their own, tell their progress (an artifact
    /// fetched), each line as it comes.
    fn sink(&self) -> Sink {
        Sink(Mutex::new(self.stream.try_clone().ok()))
    }
}

/// Lines of a request's progress, sent to its client as error output.
struct Sink(Mutex<Option<TcpStream>>);

impl Sink {
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
}

fn serve(state: &Arc<State>, stream: TcpStream) {
    let Ok(read) = stream.try_clone() else { return };
    let mut line = String::new();
    if BufReader::new(read).read_line(&mut line).unwrap_or(0) == 0 {
        return;
    }
    let mut reply = Reply { stream };
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
            reply.send(obj([("restart", why.as_str().into())]));
            return end(state, &why);
        }
    };
    let flight = Flight::enter(state);
    let strings = |key: &str| json.get(key).map_or(&[][..], Json::arr).iter().filter_map(Json::str).map(str::to_string).collect::<Vec<_>>();
    let variables = |key: &str| match json.get(key) {
        Some(Json::Obj(fields)) => fields.iter().filter_map(|(k, v)| Some((k.clone(), v.str()?.to_string()))).collect(),
        _ => BTreeMap::new(),
    };
    let request = Request {
        verb: json.get("verb").and_then(Json::str).unwrap_or("").to_string(),
        args: strings("args"),
        env: variables("env"),
        environment: variables("environment"),
        tty: json.get("tty").and_then(Json::bool) == Some(true),
        cwd: PathBuf::from(json.get("cwd").and_then(Json::str).unwrap_or("/")),
    };
    log(&format!("{} {}", request.verb, request.args.join(" ")));
    match request.verb.as_str() {
        "compile" => {
            let (code, _) = compile(state, &export, &request, &mut reply, Summary::Out);
            reply.exit(code);
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
            reply.exit(code);
        }
        "stage" => {
            let code = stage(state, &export, &request, &mut reply);
            reply.exit(code);
        }
        "test" => {
            let code = test(state, &export, &request, &mut reply);
            reply.exit(code);
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
    {
        let mut ending = state.ending.lock().unwrap_or_else(|e| e.into_inner());
        if ending.is_some() {
            return;
        }
        *ending = Some(why.to_string());
    }
    log(&format!("ending: {}", why));
    if Info::read(&state.paths.info).is_some_and(|i| i.pid == std::process::id()) {
        let _ = std::fs::remove_file(&state.paths.info);
    }
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
    let inputs = |k: &Key| export.session_inputs(k).into_iter().collect::<BTreeSet<PathBuf>>();
    let platform = |k: &Key| export.projects.get(&k.project).map(|p| p.platform);
    let needs: BTreeMap<Key, BTreeSet<PathBuf>> = wanted.iter().map(|k| (k.clone(), inputs(k))).collect();
    let types = |resident: &Key, has: &BTreeSet<PathBuf>, k: &Key| platform(resident) == platform(k) && needs[k].is_subset(has);
    let mut left: Vec<Key> = wanted.to_vec();
    let mut chosen: Vec<Key> = Vec::new();
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
                let k = export.widest(&first.project).unwrap_or_else(|| first.clone());
                let has = inputs(&k).union(&needs[&first]).cloned().collect();
                (k, has)
            }
        };
        left.retain(|k| !types(&key, &has, k));
        if !chosen.contains(&key) {
            chosen.push(key);
        }
    }
    chosen
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
    let keys = plan(export, &running, &wanted);
    let sink = reply.sink();
    let progress = |line: &str| sink.line(line);
    let progress = &progress;
    let results: Vec<(Key, Result<Answer, String>, Vec<String>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = keys.iter().map(|key| crate::alloc::spawn_in(scope, move || build(state, export, key, &request.env, progress))).collect();
        keys.iter()
            .cloned()
            .zip(handles)
            .map(|(k, h)| {
                let (answer, notes) = h.join().unwrap_or_else(|_| (Err("the build's thread panicked".to_string()), Vec::new()));
                (k, answer, notes)
            })
            .collect()
    });
    let mut code = 0;
    for (key, answer, notes) in results {
        for note in notes {
            reply.err(&format!("{}\n", note));
        }
        match answer {
            Ok(a) => {
                for n in &a.notes {
                    reply.err(&format!("teq: {}: {}\n", key, n));
                }
                for w in &a.warnings {
                    reply.err(&render(w, "warning", &export.root, &request.cwd));
                }
                for e in &a.errors {
                    reply.err(&render(e, "error", &export.root, &request.cwd));
                }
                let line = if a.ok {
                    let first = if a.fallback.as_deref() == Some("first build") { ", the first build" } else { "" };
                    let jvm = export.projects.get(&key.project).is_some_and(|p| p.platform == Platform::Jvm);
                    let what = if jvm { format!("{} classes, {} written", a.classes, a.changed.len()) } else { "checked".to_string() };
                    format!("{}: {} in {:.0} ms{}\n", key, what, a.ms, first)
                } else {
                    let n = a.errors.len().max(1);
                    code = 1;
                    format!("{}: {} error{}\n", key, n, if n == 1 { "" } else { "s" })
                };
                match summary {
                    Summary::Out => reply.out(&line),
                    Summary::Err => reply.err(&line),
                }
            }
            Err(e) => {
                reply.err(&format!("teq: {}: {}\n", key, e));
                code = 1;
            }
        }
    }
    if keys.is_empty() {
        reply.out("teq: nothing to compile\n");
    }
    let left = super::left_out(export, "compile", &left_out);
    match summary {
        Summary::Out => reply.out(&left),
        Summary::Err => reply.err(&left),
    }
    (super::with_left_out(code, &left_out), keys)
}

/// What `test` was asked: `[project] [pattern...] [--changed] [--list] [--verbose]`.
struct TestRequest {
    keys: Vec<Key>,
    patterns: Patterns,
    changed: bool,
    list: bool,
    verbose: bool,
}

/// `test`: each test configuration's closure compiled as `compile` does, then its suites run on
/// its warm runner; a configuration named by its project, or every JVM one with frameworks.
fn test(state: &Arc<State>, export: &Arc<Export>, request: &Request, reply: &mut Reply) -> i32 {
    let mut words: Vec<String> = Vec::new();
    let (mut changed, mut list, mut verbose) = (false, false, false);
    for a in &request.args {
        match a.as_str() {
            "--changed" => changed = true,
            "--list" => list = true,
            "--verbose" | "-v" => verbose = true,
            _ => words.push(a.clone()),
        }
    }
    let named = words.first().filter(|w| export.projects.contains_key(*w)).cloned();
    let mut left_out = Vec::new();
    let keys = match &named {
        Some(name) => {
            let project = &export.projects[name];
            if !project.configurations.get("test").is_some_and(|c| c.test.is_some()) {
                reply.err(&format!("teq: project {} has no test configuration\n", name));
                return 2;
            }
            if project.platform != Platform::Jvm {
                reply.err(&format!("teq: {} is a Scala.js project, whose suites teq does not run yet\n", name));
                return 2;
            }
            if let Some(why) = super::refused(export, "test", &[name]) {
                reply.err(&format!("teq: {}\n", why));
                return 2;
            }
            vec![Key::new(name, "test")]
        }
        // Every JVM project with frameworks, but those the export records what sbt alone runs of
        // for its test, named in the summary and failing the run.
        None => {
            let mut keys = Vec::new();
            for (n, _) in export.projects.iter().filter(|(_, p)| p.platform == Platform::Jvm && p.configurations.get("test").and_then(|c| c.test.as_ref()).is_some_and(|t| !t.frameworks.is_empty())) {
                if export.refusals("test", n).is_empty() { keys.push(Key::new(n, "test")) } else { left_out.push(n.clone()) }
            }
            keys
        }
    };
    let patterns = Patterns::new(&words[usize::from(named.is_some())..]);
    let asked = TestRequest { keys, patterns, changed, list, verbose };
    if asked.keys.is_empty() && left_out.is_empty() {
        reply.out("teq: no JVM project with test frameworks\n");
        return 0;
    }
    let code = asked.keys.iter().map(|key| test_configuration(state, export, request, &asked, key, reply)).max().unwrap_or(0);
    reply.out(&super::left_out(export, "test", &left_out));
    super::with_left_out(code, &left_out)
}

fn test_configuration(state: &Arc<State>, export: &Arc<Export>, request: &Request, asked: &TestRequest, key: &Key, reply: &mut Reply) -> i32 {
    let started = Instant::now();
    let running = running(state);
    let Some(owner) = plan(export, &running, std::slice::from_ref(key)).into_iter().next() else { return 2 };
    // Held through the run, so that no other build rewrites the class files the run reads.
    let slot = slot(state, &owner);
    let mut resident = slot.lock().unwrap_or_else(|e| e.into_inner());
    if !built(state, export, request, &owner, &mut resident, reply) {
        return 1;
    }
    let built_ms = started.elapsed().as_millis();
    let view = &resident.as_ref().expect("built").analysis;
    if let Some(why) = &view.unreadable {
        reply.err(&format!("teq: {}: the analysis does not read: {}\n", owner, why));
        return 1;
    }
    let conf = export.configuration(key).expect("a test configuration");
    let context = conf.test.clone().unwrap_or_default();
    if context.frameworks.is_empty() {
        reply.out(&format!("{}: no test framework\n", key));
        return 0;
    }
    let candidates = view.candidates(&conf.sources);
    let runner_slot = state.runners.lock().unwrap_or_else(|e| e.into_inner()).entry(key.clone()).or_default().clone();
    let mut runner = runner_slot.lock().unwrap_or_else(|e| e.into_inner());
    let spec = match runner_spec(export, key, &context, &request.environment) {
        Ok(s) => s,
        Err(e) => {
            reply.err(&format!("teq: {}: {}\n", key, e));
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
            if r.answers() {
                *runner = Some(r);
            } else {
                log(&format!("stopping the test runner of {}: it ended", key));
                r.stop();
            }
        }
    }
    if runner.is_none() {
        log(&format!("starting the test runner of {}: {}", key, spec.java.display()));
        let argfile = export.path(&format!("target/teq/{}/{}/runner.args", key.project, key.configuration));
        // Launched under the registry's lock, so that an ending daemon's kill pass either sees the
        // process or the launch sees the daemon ending.
        let launched = {
            let mut processes = state.runner_processes.lock().unwrap_or_else(|e| e.into_inner());
            match state.ending.lock().unwrap_or_else(|e| e.into_inner()).clone() {
                Some(why) => Err(format!("the daemon is ending ({})", why)),
                None => Runner::launch(spec, &request.environment, &argfile).inspect(|r| {
                    processes.insert(key.clone(), r.process());
                }),
            }
        };
        let started = launched.and_then(|mut r| {
            kill_when_gone(reply, &finished, r.process(), key);
            match r.handshake() {
                Ok(()) => Ok(r),
                Err(e) => {
                    r.stop();
                    Err(e)
                }
            }
        });
        match started {
            Ok(r) => *runner = Some(r),
            Err(e) => {
                reply.err(&format!("teq: {}: {}\n", key, e));
                return 2;
            }
        }
    }
    let r = runner.as_mut().expect("started");
    // A framework the export names that does not load leaves its suites unrun: never a pass.
    for (class, why) in &r.missing {
        reply.err(&format!("teq: {}: the test framework {} does not load: {}\n", key, class, why));
    }
    let incomplete = i32::from(!r.missing.is_empty());
    let defined = suites::defined(&candidates, &r.fingerprints);
    if asked.list {
        for d in &defined {
            reply.out(&format!("{}\n", d));
        }
        return incomplete;
    }
    let mut selected: Vec<suites::Suite> = suites::assign(&defined, &r.fingerprints).into_iter().filter(|s| !context.exclude.contains(&s.name) && asked.patterns.matches(&s.name)).collect();
    if asked.changed {
        let changes = state.changes.lock().unwrap_or_else(|e| e.into_inner());
        let why_every = match changes.get(key) {
            None => Some("no earlier run in this daemon's life".to_string()),
            Some(c) if c.resident != owner || c.all => Some("its resident started anew since the last run".to_string()),
            Some(_) => view.deps_missing.as_ref().map(|why| format!("the dependencies are unavailable: {}", why)),
        };
        match (why_every, changes.get(key)) {
            (Some(why), _) => reply.err(&format!("teq: {}: every suite, {}\n", key, why)),
            (None, Some(c)) => {
                let affected = view.affected(&c.files);
                selected.retain(|s| affected.contains(&s.name));
            }
            (None, None) => {}
        }
    }
    let every = asked.patterns.is_empty();
    if selected.is_empty() {
        let why = if asked.changed { "no suite depends on what changed" } else { "no suite to run" };
        reply.out(&format!("{}: {}\n", key, why));
        if every && incomplete == 0 {
            state.changes.lock().unwrap_or_else(|e| e.into_inner()).insert(key.clone(), Changes { resident: owner, all: false, files: BTreeSet::new() });
        }
        return incomplete;
    }
    let directories = child_directories(export, key, &owner);
    let names: Vec<(usize, usize, String)> = selected.iter().map(|s| (s.framework, s.fingerprint, s.name.clone())).collect();
    let result = r.run(&directories, &names, asked.verbose, request.tty, &mut |line| reply.out(line));
    drop(finished);
    let stderr = r.drain_stderr();
    if !stderr.is_empty() {
        reply.err(&stderr);
    }
    match result {
        Ok(done) => {
            if every && incomplete == 0 {
                state.changes.lock().unwrap_or_else(|e| e.into_inner()).insert(key.clone(), Changes { resident: owner, all: false, files: BTreeSet::new() });
            }
            let n = selected.len();
            reply.out(&format!(
                "{}: {} suite{}, {} passed, {} failed in {} ms (built in {} ms)\n",
                key,
                n,
                if n == 1 { "" } else { "s" },
                done.passed,
                done.failed,
                done.ms,
                built_ms
            ));
            i32::from(done.failed > 0).max(incomplete)
        }
        Err(e) => {
            if let Some(r) = runner.take() {
                r.stop();
            }
            reply.err(&format!("teq: {}: {}\n", key, e));
            1
        }
    }
}

/// The resident's build in its slot, held by the caller, its diagnostics and notes printed;
/// whether it passed.
fn built(state: &State, export: &Export, request: &Request, owner: &Key, resident: &mut Option<Resident>, reply: &mut Reply) -> bool {
    let sink = reply.sink();
    let (answer, notes) = build_in(state, export, owner, &request.env, resident, &|line| sink.line(line));
    for note in notes {
        reply.err(&format!("{}\n", note));
    }
    let answer = match answer {
        Ok(a) => a,
        Err(e) => {
            reply.err(&format!("teq: {}: {}\n", owner, e));
            return false;
        }
    };
    for n in &answer.notes {
        reply.err(&format!("teq: {}: {}\n", owner, n));
    }
    for w in &answer.warnings {
        reply.err(&render(w, "warning", &export.root, &request.cwd));
    }
    if !answer.ok {
        for e in &answer.errors {
            reply.err(&render(e, "error", &export.root, &request.cwd));
        }
        let n = answer.errors.len().max(1);
        reply.out(&format!("{}: {} error{}\n", owner, n, if n == 1 { "" } else { "s" }));
        return false;
    }
    true
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
    if !built(state, export, request, &owner, &mut resident, reply) {
        return 1;
    }
    let built_ms = started.elapsed().as_millis();
    let view = &resident.as_ref().expect("built").analysis;
    if let Some(why) = &view.unreadable {
        reply.err(&format!("teq: {}: the analysis does not read: {}\n", owner, why));
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
            reply.out(&format!("{}: staged {} files in layers {} under {} in {} ms (built in {} ms)\n", name, done.files, layers.join(", "), done.directory, started.elapsed().as_millis() - built_ms, built_ms));
            0
        }
        Err(e) => {
            reply.err(&format!("teq: {}: {}\n", name, e));
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
/// options, directory and variables, and the jars of its class path in its order.
fn runner_spec(export: &Export, key: &Key, context: &TestContext, environment: &BTreeMap<String, String>) -> Result<Spec, String> {
    let conf = export.configuration(key).ok_or_else(|| format!("no configuration {}", key))?;
    let mut jars = Vec::new();
    for entry in &conf.classpath {
        let path = match entry {
            Entry::Product(_) => continue,
            Entry::File(path) if export.path(path).is_dir() => continue,
            entry => export.resolve(entry)?,
        };
        jars.push(Jar::new(path));
    }
    Ok(Spec {
        java: runner::java(environment)?,
        least: export.java_output_version,
        options: context.java_options.clone(),
        directory: export.path(&context.base_directory),
        env: context.env_vars.clone(),
        jars,
        frameworks: context.frameworks.clone(),
    })
}

/// The directories a run reads through its own class loader: the resident's class files, the
/// configuration's resources, and those of the products and directories on its class path.
fn child_directories(export: &Export, key: &Key, owner: &Key) -> Vec<PathBuf> {
    let conf = export.configuration(key).expect("a test configuration");
    let mut out = vec![export.path(&export.classes(owner))];
    out.extend(conf.resources.iter().map(|r| export.path(r)));
    for entry in &conf.classpath {
        match entry {
            Entry::Product(k) => out.extend(export.configuration(k).map_or(&[][..], |c| &c.resources).iter().map(|r| export.path(r))),
            Entry::File(path) if export.path(path).is_dir() => out.push(export.path(path)),
            _ => {}
        }
    }
    let mut seen = BTreeSet::new();
    out.retain(|d| d.is_dir() && seen.insert(d.clone()));
    out
}

/// One resident's build: started when it is not running, its closure's generators run first.
fn build(state: &Arc<State>, export: &Arc<Export>, key: &Key, env: &BTreeMap<String, String>, progress: &(dyn Fn(&str) + Sync)) -> (Result<Answer, String>, Vec<String>) {
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
fn build_in(state: &State, export: &Export, key: &Key, env: &BTreeMap<String, String>, resident: &mut Option<Resident>, progress: &(dyn Fn(&str) + Sync)) -> (Result<Answer, String>, Vec<String>) {
    let closure = export.closure(key);
    let notes = match generators::run(export, &closure, resident_classes(export, key), env) {
        Ok(notes) => notes,
        Err(e) => return (Err(e), Vec::new()),
    };
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
            Err(e) => return (Err(e), notes),
        }
    }
    let answer = resident.as_mut().expect("started").build();
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
    let view = &resident.as_ref().ok_or_else(|| format!("{} has no resident", owner))?.analysis;
    if let Some(why) = &view.unreadable {
        return Err(format!("{}: the analysis does not read: {}", owner, why));
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
}
