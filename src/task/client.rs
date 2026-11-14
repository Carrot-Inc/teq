//! The client side of `teq`: connects to the export's daemon over loopback TCP at the port
//! `target/teq/daemon.json` names, starting the daemon when none runs (serialised through
//! `target/teq/task.start.lock`, so that two clients never start two), sends the request with
//! the token and the handshake and prints what comes back. A daemon that answers `restart` is
//! ending; the client starts the next one, which waits for the old one's lock.

use super::export::Export;
use super::progress::{counted, duration, Progress};
use super::{daemon_alive, fetch, paths, Info, Paths, PROTOCOL};
use crate::lsp::json::{obj, Json};
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How long a client waits for the daemon it started to listen: the one before it may be
/// finishing a long build first.
const START_BOUND: Duration = Duration::from_secs(150);
/// The environment a request carries for the export's dynamic keys.
const PASSED_ENV: [&str; 1] = ["SOURCE_VERSION"];

pub fn request(export: &Export, verb: &str, args: &[String]) -> i32 {
    let mut data = Vec::new();
    exchange_all(export, verb, args, &mut data, None)
}

/// A request for a build (`compile`, or `classes` for `run`) whose wait is told (`Waiting`): `said` is the verb
/// the last line names, `began` when the process started. The answer's `data` lines, in order, with the exit code.
pub fn request_told(export: &Export, verb: &str, args: &[String], said: &str, began: Instant) -> (i32, Vec<Json>) {
    let mut data = Vec::new();
    let mut waiting = Waiting::new(Progress::stderr(), said, began, Box::new(Instant::now));
    let code = exchange_all(export, verb, args, &mut data, Some(&mut waiting));
    waiting.finish();
    (code, data)
}

/// How long a build may take before its client says that it waits, from the verb's start.
const QUIET: Duration = Duration::from_secs(1);
/// How often a waiting client looks at its clock.
const TICK: Duration = Duration::from_millis(100);

/// A resident of the request's plan that has not answered.
struct Building {
    key: String,
    projects: Vec<String>,
    started: bool,
    /// Its place among the plan's residents, from one.
    place: usize,
    /// Whether its header was written (where stderr is no terminal).
    told: bool,
}

/// What the client of a build tells while it waits (docs/TOOLING.md, "What compile and run print"). Nothing of a
/// build that answers within a second of the verb's start. Of a resident that has not answered by then: where
/// stderr is no terminal one line, once, `a/compile: compiling a, b, core (1 of 1)` (`, started now` where the
/// request started the resident); on a terminal a status line, `compiling a/compile (3 projects), 2 s`, which a
/// resident leaves as it answers, and `teq compile: starting, 2 s` while the daemon has sent no plan. The status
/// is erased before anything else is written to either stream. Where it told of the wait the run ends with a
/// line of the verb's wall time, `teq compile: 3 configurations in 14.2 s` (`, 1 error`).
pub struct Waiting {
    progress: Progress,
    said: String,
    began: Instant,
    now: Box<dyn Fn() -> Instant>,
    /// Whether the plan has come, and how many residents it has.
    planned: Option<usize>,
    building: Vec<Building>,
    /// The status line as drawn, none where none shows.
    status: Option<String>,
    /// Whether a header or a status was shown: the last line is then owed.
    told: bool,
}

impl Waiting {
    pub fn new(progress: Progress, said: &str, began: Instant, now: Box<dyn Fn() -> Instant>) -> Waiting {
        Waiting { progress, said: said.to_string(), began, now, planned: None, building: Vec::new(), status: None, told: false }
    }

    /// A `progress` record of the daemon's: the plan, a resident's answer, the end's counts.
    fn record(&mut self, record: &Json) {
        if let Some(plan) = record.get("plan") {
            let residents = plan.arr();
            self.planned = Some(residents.len());
            self.building = residents
                .iter()
                .enumerate()
                .map(|(i, r)| Building {
                    key: r.get("key").and_then(Json::str).unwrap_or("").to_string(),
                    projects: r.get("projects").map_or(&[][..], Json::arr).iter().filter_map(Json::str).map(str::to_string).collect(),
                    started: r.get("started").and_then(Json::bool) == Some(true),
                    place: i + 1,
                    told: false,
                })
                .collect();
            self.tick();
        } else if let Some(key) = record.get("answered").and_then(Json::str) {
            self.building.retain(|b| b.key != key);
            if self.status.is_some() {
                self.draw();
            }
        } else if let Some(done) = record.get("done") {
            let count = |field: &str| done.get(field).and_then(Json::num).unwrap_or(0.0) as usize;
            self.done(count("configurations"), count("errors"));
        }
    }

    /// The clock looked at: from a second after the verb's start, what waits is told.
    fn tick(&mut self) {
        if (self.now)().saturating_duration_since(self.began) < QUIET {
            return;
        }
        if self.progress.is_terminal() {
            self.draw();
            self.progress.tick();
            return;
        }
        let of = self.planned.unwrap_or(0);
        for b in self.building.iter_mut().filter(|b| !b.told) {
            b.told = true;
            self.told = true;
            self.progress.line(&format!("{}: compiling {} ({} of {}){}", b.key, b.projects.join(", "), b.place, of, if b.started { ", started now" } else { "" }));
        }
    }

    /// The status line of what waits now, drawn where it is not the one showing; erased where nothing waits.
    fn draw(&mut self) {
        let text = match self.planned {
            None => Some(format!("teq {}: starting", self.said)),
            Some(_) if self.building.is_empty() => None,
            Some(_) => {
                let keys: Vec<&str> = self.building.iter().map(|b| b.key.as_str()).collect();
                let projects: usize = self.building.iter().map(|b| b.projects.len()).sum();
                Some(format!("compiling {} ({})", keys.join(", "), counted(projects, "project", "projects")))
            }
        };
        if text == self.status {
            return;
        }
        match &text {
            Some(text) => {
                self.told = true;
                self.progress.status_since(text, self.began);
            }
            None => self.progress.clear(),
        }
        self.status = text;
    }

    /// A line of the reply was read while a resident builds (a line of its progress, an artifact fetched): a look
    /// at the clock too, so that lines that keep coming do not put the telling off. Nothing where none builds.
    fn after_line(&mut self) {
        if !self.building.is_empty() {
            self.tick();
        }
    }

    /// Before the client writes what the daemon sent to either stream: the status gives its row.
    fn before_output(&mut self) {
        if self.status.take().is_some() {
            self.progress.clear();
        }
    }

    /// The builds have ended: the last line, where the wait was told.
    fn done(&mut self, configurations: usize, errors: usize) {
        self.before_output();
        self.building.clear();
        self.planned = Some(0);
        if !self.told {
            return;
        }
        let took = duration((self.now)().saturating_duration_since(self.began));
        let what = counted(configurations, "configuration", "configurations");
        let what = if self.said == "run" { format!("{} compiled", what) } else { what };
        let errors = if errors == 0 { String::new() } else { format!(", {}", counted(errors, "error", "errors")) };
        self.progress.line(&format!("teq {}: {} in {}{}", self.said, what, took, errors));
        self.told = false;
    }

    /// The exchange is over, however it ended. Where the wait was told and the daemon sent no counts (it refused
    /// the request before any build, it ended without an answer, its start failed), the last line still ends the
    /// run, counting no configuration; where the counts came it is written already.
    fn finish(&mut self) {
        self.done(0, 0);
    }
}

fn exchange_all(export: &Export, verb: &str, args: &[String], data: &mut Vec<Json>, mut waiting: Option<&mut Waiting>) -> i32 {
    let paths = paths(&export.root);
    let identity = match identity() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("teq: {}", e);
            return 2;
        }
    };
    let cwd = std::env::current_dir().unwrap_or_else(|_| export.root.clone());
    let env = Json::Obj(PASSED_ENV.iter().filter_map(|k| Some((k.to_string(), Json::Str(std::env::var(k).ok()?)))).collect());
    // A test runner starts with the environment of the client that started it.
    let environment = if verb == "test" { Json::Obj(std::env::vars_os().filter_map(|(k, v)| Some((k.into_string().ok()?, Json::Str(v.into_string().ok()?)))).collect()) } else { Json::Obj(Vec::new()) };
    let tty = std::io::IsTerminal::is_terminal(&std::io::stdout());
    let message = |token: &str| {
        obj([
            ("token", token.into()),
            ("protocol", PROTOCOL.into()),
            ("export", export.file.to_string_lossy().as_ref().into()),
            ("sha256", export.sha256.as_str().into()),
            ("identity", identity.as_str().into()),
            ("verb", verb.into()),
            ("args", Json::Arr(args.iter().map(|a| a.as_str().into()).collect())),
            ("env", env.clone()),
            ("environment", environment.clone()),
            ("tty", tty.into()),
            ("cwd", cwd.to_string_lossy().as_ref().into()),
        ])
    };
    for _ in 0..3 {
        let (stream, info) = match connect(&paths) {
            Some(c) => c,
            None if verb == "stop" => {
                println!("teq: no daemon runs for {}", export.file.display());
                return 0;
            }
            None => match start(export, &paths, &identity, waiting.as_deref_mut()) {
                Ok(c) => c,
                Err(e) => {
                    if let Some(w) = waiting.as_deref_mut() {
                        w.before_output();
                    }
                    eprintln!("teq: {}", e);
                    return 2;
                }
            },
        };
        data.clear();
        match exchange(stream, &message(&info.token), &paths.log, data, waiting.as_deref_mut()) {
            Some(code) => return code,
            None if verb == "stop" => {
                println!("teq: the daemon was of another binary or export, and is ending");
                return 0;
            }
            None => continue,
        }
    }
    if let Some(w) = waiting.as_deref_mut() {
        w.before_output();
    }
    eprintln!("teq: the daemon asked for a restart three times");
    2
}

/// The running daemon: `daemon.json` read, the lock held by a live daemon, its port answering.
fn connect(paths: &Paths) -> Option<(TcpStream, Info)> {
    let info = Info::read(&paths.info)?;
    if !daemon_alive(&paths.lock) {
        return None;
    }
    let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, info.port)).ok()?;
    let _ = stream.set_nodelay(true);
    Some((stream, info))
}

/// Sends the request and prints the answer, keeping its data; the exit code, or nothing when the
/// daemon asked for a new one.
fn exchange(mut stream: TcpStream, message: &Json, log: &Path, data: &mut Vec<Json>, mut waiting: Option<&mut Waiting>) -> Option<i32> {
    let mut line = message.to_text();
    line.push('\n');
    if stream.write_all(line.as_bytes()).is_err() {
        return None;
    }
    let mut out = std::io::stdout();
    let mut err = std::io::stderr();
    // Where the wait is told the stream is read with a bound, so that the client looks at its clock between the
    // answer's lines: `Some(None)` is such a look, a line read in parts over several of them is kept whole.
    if waiting.is_some() {
        let _ = stream.set_read_timeout(Some(TICK));
    }
    let mut reader = BufReader::new(stream);
    let mut pending: Vec<u8> = Vec::new();
    let mut ended = false;
    let lines = std::iter::from_fn(move || {
        while !ended {
            match reader.read_until(b'\n', &mut pending) {
                Ok(0) => ended = true,
                Ok(_) if pending.last() == Some(&b'\n') => {
                    let line = String::from_utf8_lossy(&pending).trim_end_matches(['\n', '\r']).to_string();
                    pending.clear();
                    return Some(Some(line));
                }
                // The stream ended within a line: the next read says so.
                Ok(_) => {}
                Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => return Some(None),
                Err(_) => ended = true,
            }
        }
        (!pending.is_empty()).then(|| Some(String::from_utf8_lossy(&std::mem::take(&mut pending)).into_owned()))
    });
    for line in lines {
        let Some(line) = line else {
            if let Some(w) = waiting.as_deref_mut() {
                w.tick();
            }
            continue;
        };
        let Ok(json) = Json::parse(&line) else { continue };
        if let Some(text) = json.get("out").and_then(Json::str) {
            if let Some(w) = waiting.as_deref_mut() {
                w.before_output();
            }
            let _ = out.write_all(text.as_bytes());
            let _ = out.flush();
        } else if let Some(text) = json.get("err").and_then(Json::str) {
            if let Some(w) = waiting.as_deref_mut() {
                w.before_output();
            }
            let _ = err.write_all(text.as_bytes());
        } else if let Some(code) = json.get("exit").and_then(Json::num) {
            if let Some(w) = waiting.as_deref_mut() {
                w.before_output();
            }
            return Some(code as i32);
        } else if let Some(d) = json.get("data") {
            data.push(d.clone());
        } else if let Some(record) = json.get("progress") {
            if let Some(w) = waiting.as_deref_mut() {
                w.record(record);
            }
        } else if json.get("restart").is_some() {
            if let Some(w) = waiting.as_deref_mut() {
                w.before_output();
            }
            return None;
        }
        if let Some(w) = waiting.as_deref_mut() {
            w.after_line();
        }
    }
    if let Some(w) = waiting.as_deref_mut() {
        w.before_output();
    }
    eprintln!("teq: the daemon ended without an answer (see {})", log.display());
    Some(2)
}

/// Starts the daemon unless another client started it meanwhile, and connects to it. Where the wait is told
/// (`waiting`), the client looks at its clock while it waits for the start's lock and for the daemon to listen.
fn start(export: &Export, paths: &Paths, identity: &str, mut waiting: Option<&mut Waiting>) -> Result<(TcpStream, Info), String> {
    if let Some(dir) = paths.info.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {}", dir.display(), e))?;
    }
    let lock = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(&paths.start_lock).map_err(|e| format!("cannot open {}: {}", paths.start_lock.display(), e))?;
    let until = Instant::now() + START_BOUND;
    loop {
        match lock.try_lock() {
            Ok(()) => break,
            Err(std::fs::TryLockError::WouldBlock) if Instant::now() < until => {
                if let Some(w) = waiting.as_deref_mut() {
                    w.tick();
                }
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(e) => return Err(format!("cannot hold {}: {:?}", paths.start_lock.display(), e)),
        }
    }
    if let Some(c) = connect(paths) {
        return Ok(c);
    }
    let log = log_file(&paths.log)?;
    let exe = std::env::current_exe().map_err(|e| format!("cannot find this teq: {}", e))?;
    let mut command = Command::new(&exe);
    command
        .args(["--daemon", "--identity", identity, "--export"])
        .arg(&export.file)
        .current_dir(&export.root)
        .stdin(Stdio::null())
        .stdout(log.try_clone().map_err(|e| e.to_string())?)
        .stderr(log);
    detach(&mut command);
    let mut daemon = command.spawn().map_err(|e| format!("cannot start the daemon: {}", e))?;
    let mut told = false;
    let started = Instant::now();
    while Instant::now() < until {
        if let Some(info) = Info::read(&paths.info).filter(|i| i.pid == daemon.id()) {
            if let Ok(stream) = TcpStream::connect((Ipv4Addr::LOCALHOST, info.port)) {
                let _ = stream.set_nodelay(true);
                return Ok((stream, info));
            }
        }
        if let Ok(Some(status)) = daemon.try_wait() {
            let tail = std::fs::read_to_string(&paths.log).unwrap_or_default();
            let tail: Vec<&str> = tail.lines().rev().take(5).collect();
            return Err(format!("the daemon ended ({}) before it listened: {}", status, tail.into_iter().rev().collect::<Vec<_>>().join(" | ")));
        }
        if !told && started.elapsed() > Duration::from_secs(3) {
            if let Some(w) = waiting.as_deref_mut() {
                w.before_output();
            }
            eprintln!("teq: waiting for the previous daemon to finish its work");
            told = true;
        }
        if let Some(w) = waiting.as_deref_mut() {
            w.tick();
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Err(format!("the daemon did not listen within {} s (see {})", START_BOUND.as_secs(), paths.log.display()))
}

/// The daemon outlives the client: its own process group, no console of the client's. On Windows
/// it gets a hidden console of its own rather than none, since its residents, generators and test
/// runners inherit it, where each child of a process without a console opens a console window.
fn detach(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    }
}

/// The daemon's log, appended to, cut when it passes a megabyte.
fn log_file(path: &Path) -> Result<std::fs::File, String> {
    if std::fs::metadata(path).is_ok_and(|m| m.len() > 1 << 20) {
        let _ = std::fs::remove_file(path);
    }
    std::fs::OpenOptions::new().create(true).append(true).open(path).map_err(|e| format!("cannot open {}: {}", path.display(), e))
}

/// This binary as the handshake names it: its version and the sha1 of its file, computed once
/// per file (path, size, modification time, inode) and remembered in `<cache>/identities`.
pub fn identity() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("cannot find this teq: {}", e))?;
    let exe = std::fs::canonicalize(&exe).unwrap_or(exe);
    let meta = std::fs::metadata(&exe).map_err(|e| format!("cannot read {}: {}", exe.display(), e))?;
    let key = fetch::identity_key(&exe, &meta);
    let memo = fetch::cache_root().map(|r| r.join("identities"));
    let known = memo.as_ref().and_then(|m| fetch::read_memo(m).remove(&key));
    let sha1 = match known {
        Some(sha1) => sha1,
        None => {
            let sha1 = fetch::sha1_file(&exe).map_err(|e| format!("cannot read {}: {}", exe.display(), e))?;
            if let Some(memo) = &memo {
                fetch::append_memo(memo, &key, &sha1);
            }
            sha1
        }
    };
    Ok(format!("{} {}", super::version(), sha1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// What a reporter wrote, kept for a test to read.
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

    /// A waiting client over a kept writer and a clock the test moves, in milliseconds since the verb began.
    fn waiting(terminal: bool, said: &str) -> (Waiting, Kept, Arc<Mutex<u64>>) {
        let (kept, moved) = (Kept::default(), Arc::new(Mutex::new(0u64)));
        let began = Instant::now();
        let clock = |moved: &Arc<Mutex<u64>>| {
            let moved = moved.clone();
            move || began + Duration::from_millis(*moved.lock().unwrap())
        };
        let progress = Progress::over(Box::new(kept.clone()), terminal, Box::new(clock(&moved)), Box::new(|| 80));
        (Waiting::new(progress, said, began, Box::new(clock(&moved))), kept, moved)
    }

    fn plan(residents: &[(&str, Vec<&str>, bool)]) -> Json {
        let residents = residents
            .iter()
            .map(|(key, projects, started)| obj([("key", (*key).into()), ("projects", Json::Arr(projects.iter().map(|p| (*p).into()).collect())), ("started", (*started).into())]))
            .collect();
        obj([("plan", Json::Arr(residents))])
    }

    fn done(configurations: usize, errors: usize) -> Json {
        obj([("done", obj([("configurations", Json::Num(configurations as f64)), ("errors", Json::Num(errors as f64))]))])
    }

    #[test]
    fn a_build_that_answers_within_the_second_is_told_nothing_of() {
        for terminal in [false, true] {
            let (mut w, kept, clock) = waiting(terminal, "compile");
            w.record(&plan(&[("a/compile", vec!["a", "b", "core"], true)]));
            *clock.lock().unwrap() = 900;
            w.tick();
            w.record(&obj([("answered", "a/compile".into())]));
            w.before_output();
            *clock.lock().unwrap() = 950;
            w.record(&done(3, 0));
            assert_eq!(kept.taken(), "", "terminal: {}", terminal);
        }
    }

    #[test]
    fn off_a_terminal_a_build_over_a_second_gets_its_header_once_and_the_run_its_last_line() {
        let (mut w, kept, clock) = waiting(false, "compile");
        w.record(&plan(&[("a/compile", vec!["a", "b", "core"], true), ("d/compile", vec!["d"], false)]));
        *clock.lock().unwrap() = 999;
        w.tick();
        assert_eq!(kept.taken(), "");
        *clock.lock().unwrap() = 1_000;
        w.tick();
        assert_eq!(kept.taken(), "a/compile: compiling a, b, core (1 of 2), started now\nd/compile: compiling d (2 of 2)\n");
        *clock.lock().unwrap() = 5_000;
        w.tick();
        w.record(&obj([("answered", "d/compile".into())]));
        w.record(&obj([("answered", "a/compile".into())]));
        w.before_output();
        assert_eq!(kept.taken(), "");
        *clock.lock().unwrap() = 14_230;
        w.record(&done(4, 0));
        assert_eq!(kept.taken(), "teq compile: 4 configurations in 14.2 s\n");
    }

    #[test]
    fn a_line_read_while_a_resident_builds_is_a_look_at_the_clock() {
        let (mut w, kept, clock) = waiting(false, "compile");
        w.record(&plan(&[("a/compile", vec!["a"], true)]));
        *clock.lock().unwrap() = 600;
        w.after_line();
        assert_eq!(kept.taken(), "");
        *clock.lock().unwrap() = 1_300;
        w.after_line();
        assert_eq!(kept.taken(), "a/compile: compiling a (1 of 1), started now\n");
        w.after_line();
        assert_eq!(kept.taken(), "");
        w.record(&obj([("answered", "a/compile".into())]));
        *clock.lock().unwrap() = 1_400;
        w.after_line();
        w.record(&done(1, 0));
        assert_eq!(kept.taken(), "teq compile: 1 configuration in 1400 ms\n");
        // No plan has come: a line read (a refusal's) is no look at the clock, and no last line is owed.
        let (mut w, kept, clock) = waiting(true, "compile");
        *clock.lock().unwrap() = 1_300;
        w.after_line();
        w.record(&done(0, 0));
        assert_eq!(kept.taken(), "");
    }

    #[test]
    fn an_exchange_that_ends_without_counts_after_the_wait_was_told_has_its_last_line_once() {
        // On a terminal, no plan come: the status, a refusal's line, the end.
        let erase = "\r\x1b[K";
        let (mut w, kept, clock) = waiting(true, "compile");
        *clock.lock().unwrap() = 1_400;
        w.tick();
        assert_eq!(kept.taken(), format!("{}teq compile: starting, 1 s", erase));
        w.before_output();
        assert_eq!(kept.taken(), erase);
        *clock.lock().unwrap() = 1_450;
        w.finish();
        assert_eq!(kept.taken(), "teq compile: 0 configurations in 1450 ms\n");
        w.finish();
        assert_eq!(kept.taken(), "");
        // Off a terminal, a header told and the daemon gone.
        let (mut w, kept, clock) = waiting(false, "run");
        w.record(&plan(&[("a/compile", vec!["a"], false)]));
        *clock.lock().unwrap() = 1_200;
        w.tick();
        assert_eq!(kept.taken(), "a/compile: compiling a (1 of 1)\n");
        *clock.lock().unwrap() = 2_000;
        w.finish();
        assert_eq!(kept.taken(), "teq run: 0 configurations compiled in 2000 ms\n");
        // The counts came: the end adds nothing.
        let (mut w, kept, clock) = waiting(false, "compile");
        w.record(&plan(&[("a/compile", vec!["a"], false)]));
        *clock.lock().unwrap() = 1_200;
        w.tick();
        w.record(&obj([("answered", "a/compile".into())]));
        w.record(&done(1, 1));
        assert_eq!(kept.taken(), "a/compile: compiling a (1 of 1)\nteq compile: 1 configuration in 1200 ms, 1 error\n");
        w.finish();
        assert_eq!(kept.taken(), "");
        // Nothing was told: nothing is owed, on a terminal or off one.
        for terminal in [false, true] {
            let (mut w, kept, clock) = waiting(terminal, "compile");
            *clock.lock().unwrap() = 700;
            w.tick();
            w.before_output();
            w.finish();
            assert_eq!(kept.taken(), "", "terminal: {}", terminal);
        }
    }

    #[test]
    fn off_a_terminal_a_resident_that_answered_before_the_second_has_no_header() {
        let (mut w, kept, clock) = waiting(false, "run");
        w.record(&plan(&[("a/compile", vec!["a"], false), ("d/compile", vec!["d"], false)]));
        *clock.lock().unwrap() = 400;
        w.record(&obj([("answered", "a/compile".into())]));
        *clock.lock().unwrap() = 1_100;
        w.tick();
        assert_eq!(kept.taken(), "d/compile: compiling d (2 of 2)\n");
        *clock.lock().unwrap() = 2_300;
        w.record(&obj([("answered", "d/compile".into())]));
        w.record(&done(1, 1));
        assert_eq!(kept.taken(), "teq run: 1 configuration compiled in 2300 ms, 1 error\n");
    }

    #[test]
    fn on_a_terminal_the_status_names_what_waits_and_gives_its_row_to_every_line() {
        let erase = "\r\x1b[K";
        let (mut w, kept, clock) = waiting(true, "compile");
        *clock.lock().unwrap() = 1_200;
        w.tick();
        assert_eq!(kept.taken(), format!("{}teq compile: starting, 1 s", erase));
        *clock.lock().unwrap() = 2_100;
        w.record(&plan(&[("a/compile", vec!["a", "b", "core"], true), ("d/compile", vec!["d", "e"], false)]));
        assert_eq!(kept.taken(), format!("{}compiling a/compile, d/compile (5 projects), 2 s", erase));
        *clock.lock().unwrap() = 3_050;
        w.tick();
        assert_eq!(kept.taken(), format!("{}compiling a/compile, d/compile (5 projects), 3 s", erase));
        w.record(&obj([("answered", "a/compile".into())]));
        assert_eq!(kept.taken(), format!("{}compiling d/compile (2 projects), 3 s", erase));
        // A line of the daemon's for either stream: the status erased before it, drawn again at the next look.
        w.before_output();
        assert_eq!(kept.taken(), erase);
        w.tick();
        assert_eq!(kept.taken(), format!("{}compiling d/compile (2 projects), 3 s", erase));
        w.record(&obj([("answered", "d/compile".into())]));
        assert_eq!(kept.taken(), erase);
        *clock.lock().unwrap() = 4_000;
        w.record(&done(5, 2));
        assert_eq!(kept.taken(), "teq compile: 5 configurations in 4000 ms, 2 errors\n");
    }
}

