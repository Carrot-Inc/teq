//! The client side of `teq`: connects to the export's daemon over loopback TCP at the port
//! `target/teq/daemon.json` names, starting the daemon when none runs (serialised through
//! `target/teq/task.start.lock`, so that two clients never start two), sends the request with
//! the token and the handshake and prints what comes back. A daemon that answers `restart` is
//! ending; the client starts the next one, which waits for the old one's lock, and never asks the
//! ending one again: its advertisement, while it stands, is no daemon's (`connect`).

use super::export::Export;
use super::progress::{counted, duration, duration_of_ms, errors_line, Progress};
use super::{daemon_alive, fetch, paths, Info, Paths, PROTOCOL};
use crate::lsp::json::{obj, Json};
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::path::{Path, PathBuf};
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
    let mut waiting = Waiting::new(Progress::stderr(), Box::new(std::io::stdout()), said, began, Box::new(Instant::now));
    let code = exchange_all(export, verb, args, &mut data, Some(&mut waiting));
    waiting.finish();
    (code, data)
}

/// How long a build may take before its client says that it waits, from the verb's start.
const QUIET: Duration = Duration::from_secs(1);
/// How often a waiting client looks at its clock.
const TICK: Duration = Duration::from_millis(100);
/// How long a daemon that closed a connection without a line is looked at before it counts as still there
/// (`ending`).
const GONE: Duration = Duration::from_millis(300);
/// What a client says once, where stderr is no terminal, of a daemon it started that waits for the one before it.
const PREVIOUS: &str = "teq: waiting for the previous teq daemon to stop";

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

/// What the client of a build tells (docs/TOOLING.md, "What compile and run print"): each build's line, composed
/// from the daemon's record of it and named by the projects asked for (`built`), and over several builds their
/// total (`done`). Of the wait, nothing for a build that answers within a second of the verb's start; of one that
/// has not answered by then: where stderr is no terminal one line, once, `compiling a, b, core (1 of 1)` (`, started
/// now` where the request started its compiler); on a terminal a status line, `compiling a (3 projects), 2 s`,
/// which a build leaves as it answers, `teq compile: starting, 2 s` while the daemon has sent no plan, and `teq
/// compile: waiting for the previous teq daemon to stop, 2 s` while the daemon this request started waits for an
/// ending one. The status is erased before anything else is written to either stream.
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
    /// Where compile's result lines go: stdout (`run`'s go to stderr with the status).
    out: Box<dyn Write>,
    /// How many builds the plan has: several end with a total, one with its own line.
    builds: usize,
    /// The projects the builds answered so far covered, and whether any of them typed a file or failed.
    covered: usize,
    acted: bool,
    /// Whether the daemon this client started waits for the one before it to stop, and whether that was said
    /// (where stderr is no terminal).
    previous: bool,
    previous_told: bool,
}

impl Waiting {
    pub fn new(progress: Progress, out: Box<dyn Write>, said: &str, began: Instant, now: Box<dyn Fn() -> Instant>) -> Waiting {
        Waiting { progress, said: said.to_string(), began, now, planned: None, building: Vec::new(), status: None, out, builds: 0, covered: 0, acted: false, previous: false, previous_told: false }
    }

    /// The daemon started for this request waits for the one before it, which is ending, or no longer does.
    fn previous(&mut self, waits: bool) {
        self.previous = waits;
        self.tick();
    }

    /// A `progress` record of the daemon's: the plan, a resident's answer, the end's counts.
    fn record(&mut self, record: &Json) {
        if let Some(plan) = record.get("plan") {
            let residents = plan.arr();
            self.planned = Some(residents.len());
            self.builds = residents.len();
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
        } else if let Some(built) = record.get("built") {
            self.built(built);
        } else if let Some(done) = record.get("done") {
            self.done(done.get("errors").and_then(Json::num).unwrap_or(0.0) as usize);
        }
    }

    /// A build's answer (the daemon's record, `daemon::compile`): its line, named by the projects asked for that
    /// it covered, with what it covered of them. One that typed nothing and wrote nothing found them up to date:
    /// said where it is the request's one build, left to the total among several. A build wider than the request
    /// (another project's resident served it) has no count of classes: the classes are the wider build's. `run`
    /// tells a build only where it wrote something or failed, before the program.
    fn built(&mut self, built: &Json) {
        let flag = |field: &str| built.get(field).and_then(Json::bool) == Some(true);
        let count = |field: &str| built.get(field).and_then(Json::num).unwrap_or(0.0);
        let names: Vec<&str> = built.get("projects").map_or(&[][..], Json::arr).iter().filter_map(Json::str).collect();
        self.covered += names.len();
        // A build that failed outright has the daemon's own line on stderr.
        if flag("failed") {
            self.acted = true;
            return;
        }
        let names = names.join(", ");
        let written = count("written") as usize;
        let line = if !flag("ok") {
            self.acted = true;
            let elsewhere: Vec<(String, usize)> = built
                .get("in")
                .map_or(&[][..], Json::arr)
                .iter()
                .filter_map(|share| Some((share.arr().first()?.str()?.to_string(), share.arr().get(1)?.num()? as usize)))
                .collect();
            Some(errors_line(&names, count("errors") as usize, &elsewhere))
        } else if !flag("typed") && written == 0 {
            (self.builds <= 1).then(|| format!("{}: up to date", names))
        } else {
            self.acted = true;
            let covers = if flag("test") { "main and test" } else { "main" };
            let took = duration_of_ms(count("ms"));
            Some(if !flag("jvm") {
                format!("{}: checked {} in {}", names, covers, took)
            } else if flag("wider") {
                format!("{}: compiled {} in {}", names, covers, took)
            } else if written == 0 {
                format!("{}: compiled {}, no class changed, in {}", names, covers, took)
            } else {
                format!("{}: compiled {}, {} written, in {}", names, covers, counted(written, "class", "classes"), took)
            })
        };
        if let Some(line) = line.filter(|_| self.said != "run" || !flag("ok") || written > 0) {
            self.result(&line);
        }
    }

    /// A result line: compile's on stdout, `run`'s on stderr, where the program's own output is not.
    fn result(&mut self, line: &str) {
        self.before_output();
        if self.said == "run" {
            self.progress.line(line);
        } else {
            let _ = writeln!(self.out, "{}", line);
            let _ = self.out.flush();
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
        if self.previous && !std::mem::replace(&mut self.previous_told, true) {
            self.progress.line(PREVIOUS);
        }
        let of = self.planned.unwrap_or(0);
        for b in self.building.iter_mut().filter(|b| !b.told) {
            b.told = true;
            self.progress.line(&format!("compiling {} ({} of {}){}", b.projects.join(", "), b.place, of, if b.started { ", started now" } else { "" }));
        }
    }

    /// The status line of what waits now, drawn where it is not the one showing; erased where nothing waits.
    fn draw(&mut self) {
        let text = match self.planned {
            None if self.previous => Some(format!("teq {}: waiting for the previous teq daemon to stop", self.said)),
            None => Some(format!("teq {}: starting", self.said)),
            Some(_) if self.building.is_empty() => None,
            Some(_) => {
                // Each build by its project: the configuration is the daemon's matter.
                let names: Vec<&str> = self.building.iter().map(|b| b.key.split('/').next().unwrap_or(&b.key)).collect();
                let projects: usize = self.building.iter().map(|b| b.projects.len()).sum();
                Some(format!("compiling {} ({})", names.join(", "), counted(projects, "project", "projects")))
            }
        };
        if text == self.status {
            return;
        }
        match &text {
            Some(text) => self.progress.status_since(text, self.began),
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

    /// The builds have ended. A request of several builds ends with its total, the command's own time: the
    /// projects covered and the errors, or that every one was up to date; one build's line is the whole answer.
    fn done(&mut self, errors: usize) {
        self.before_output();
        self.building.clear();
        self.planned = Some(0);
        if std::mem::take(&mut self.builds) > 1 {
            let projects = counted(self.covered, "project", "projects");
            let line = if self.acted {
                let took = duration((self.now)().saturating_duration_since(self.began));
                let errors = if errors == 0 { String::new() } else { format!(", {}", counted(errors, "error", "errors")) };
                format!("teq {}: {} in {}{}", self.said, projects, took, errors)
            } else {
                format!("teq {}: {}, up to date", self.said, projects)
            };
            self.result(&line);
        }
    }

    /// The exchange is over, however it ended: the status gives its row back.
    fn finish(&mut self) {
        self.before_output();
        self.building.clear();
    }
}

fn exchange_all(export: &Export, verb: &str, args: &[String], data: &mut Vec<Json>, waiting: Option<&mut Waiting>) -> i32 {
    let cwd = std::env::current_dir().unwrap_or_else(|_| export.root.clone());
    // The request's records (`--diagnostics`) go to a file of its own, moved over the file the command names when
    // the request ends, however it ends.
    let records = Records::of(args, &cwd);
    let code = match &records {
        Some((_, sent)) => exchange_tries(export, verb, sent, records.as_ref().map(|(r, _)| r), &cwd, data, waiting),
        None => exchange_tries(export, verb, args, None, &cwd, data, waiting),
    };
    if let Some((r, _)) = &records {
        r.end();
    }
    code
}

/// Where a request's records go (`--diagnostics <file>`). The file the command names holds one request's records,
/// whole: the request writes to a file of its own beside it (`own`), which the client moves over the named one when
/// the request ends. Of two requests that name one file the last to end is the one it holds, and a reader finds
/// neither two requests' records nor a part of one's. The named file is emptied as the request starts, so a request
/// that is killed leaves no earlier one's records; its own file the daemon removes (`daemon::ended`). A named file
/// that is no regular file (a pipe, a device), or beside which no file can be made, is written as it is.
struct Records {
    named: PathBuf,
    own: PathBuf,
}

impl Records {
    /// The request's records and the arguments as the daemon gets them, the file's name replaced by the request's
    /// own; none where the command names no file.
    fn of(args: &[String], cwd: &Path) -> Option<(Records, Vec<String>)> {
        let at = args.iter().position(|a| a == "--diagnostics")?;
        let named = cwd.join(args.get(at + 1)?);
        // Through a link to the file it stands for; a path that names no file yet is taken as it is written.
        let named = std::fs::canonicalize(&named).unwrap_or(named);
        let _ = std::fs::write(&named, "");
        let regular = std::fs::metadata(&named).map_or(true, |m| m.is_file());
        let own = match named.file_name().filter(|_| regular) {
            Some(name) => {
                let mut own_name = name.to_os_string();
                own_name.push(format!(".teq-{}", std::process::id()));
                Some(named.with_file_name(own_name)).filter(|own| std::fs::write(own, "").is_ok())
            }
            None => None,
        };
        let own = own.unwrap_or_else(|| named.clone());
        let mut sent = args.to_vec();
        sent[at + 1] = own.to_string_lossy().into_owned();
        Some((Records { named, own }, sent))
    }

    /// The request ended: its records moved over the named file, whole. A move the system refuses (a reader that
    /// holds the named file open, where that forbids it) is a copy.
    fn end(&self) {
        if self.own == self.named || !self.own.exists() {
            return;
        }
        if std::fs::rename(&self.own, &self.named).is_err() {
            match std::fs::copy(&self.own, &self.named) {
                Ok(_) => {
                    let _ = std::fs::remove_file(&self.own);
                }
                Err(e) => eprintln!("teq: cannot write {}: {}", self.named.display(), e),
            }
        }
    }
}

/// The request sent to the export's daemon, to the next one where the daemon asked is ending: up to three.
fn exchange_tries(export: &Export, verb: &str, args: &[String], records: Option<&Records>, cwd: &Path, data: &mut Vec<Json>, mut waiting: Option<&mut Waiting>) -> i32 {
    let paths = paths(&export.root);
    let identity = match identity() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("teq: {}", e);
            return 2;
        }
    };
    let env = Json::Obj(PASSED_ENV.iter().filter_map(|k| Some((k.to_string(), Json::Str(std::env::var(k).ok()?)))).collect());
    // A test runner starts with the environment of the client that started it.
    let environment = if verb == "test" { Json::Obj(std::env::vars_os().filter_map(|(k, v)| Some((k.into_string().ok()?, Json::Str(v.into_string().ok()?)))).collect()) } else { Json::Obj(Vec::new()) };
    let tty = std::io::IsTerminal::is_terminal(&std::io::stdout());
    let message = |token: &str, sha256: &str| {
        obj([
            ("token", token.into()),
            ("protocol", PROTOCOL.into()),
            ("export", export.file.to_string_lossy().as_ref().into()),
            ("sha256", sha256.into()),
            ("identity", identity.as_str().into()),
            ("verb", verb.into()),
            ("args", Json::Arr(args.iter().map(|a| a.as_str().into()).collect())),
            ("env", env.clone()),
            ("environment", environment.clone()),
            ("tty", tty.into()),
            ("cwd", cwd.to_string_lossy().as_ref().into()),
            ("own_records", records.is_some_and(|r| r.own != r.named).into()),
        ])
    };
    // The daemons that would not serve this client, by their pids: one that answered `restart` is ending, and is
    // not asked again while its advertisement stands; the next one is started beside it and waits for its lock.
    let mut refused: Vec<u32> = Vec::new();
    // Whether the last daemon asked ended before it sent a line.
    let mut silent = false;
    // The export's digest as the file has it now: read again before a new try, since a daemon that serves
    // another digest answers `restart` to this one.
    let mut sha256 = export.sha256.clone();
    for _ in 0..3 {
        let (stream, info) = match connect(&paths, &refused) {
            Some(c) => c,
            None if verb == "stop" => {
                println!("teq: no daemon runs for {}", export.file.display());
                return 0;
            }
            None => match start(export, &paths, &identity, &refused, waiting.as_deref_mut()) {
                Ok(c) => c,
                Err(e) => return unserved(records, &format!("teq: {}", e), "other", waiting.as_deref_mut()),
            },
        };
        data.clear();
        let answered = exchange(stream, &message(&info.token, &sha256), data, waiting.as_deref_mut());
        silent = matches!(answered, Answered::Nothing | Answered::Unsent);
        match answered {
            Answered::Exit(code) => return code,
            Answered::Restart if verb == "stop" => {
                println!("teq: the daemon was of another binary or export, and is ending");
                return 0;
            }
            // The daemon said it is ending, or that it serves no client of this kind.
            Answered::Restart => {}
            // No line of an answer. Only a daemon seen ending or gone (`ending`) is passed by for the next one: one
            // that is still there keeps its lock for as long as it lives, and the request fails at once. A request
            // that did not reach an ending daemon whole asked nothing of it and goes to the next whatever its verb;
            // one that did goes there only where the daemon builds and runs nothing of the user's and rewrites
            // nothing of the user's under it, so that nothing is done twice: a compile, and the build `run` asks
            // for before the client starts the program.
            Answered::Unsent if ending(&paths, info.pid) => {}
            Answered::Nothing if matches!(verb, "compile" | "classes") && ending(&paths, info.pid) => {}
            Answered::Unsent | Answered::Nothing | Answered::Ended => return unserved(records, &ended_line(&paths), "crash", waiting.as_deref_mut()),
        }
        refused.push(info.pid);
        if let Ok(now) = Export::read(&export.file) {
            sha256 = now.sha256;
        }
    }
    // Three daemons in a row would not serve the request: the last one's way of not serving it is the line's.
    if silent {
        return unserved(records, &ended_line(&paths), "crash", waiting.as_deref_mut());
    }
    unserved(records, "teq: the daemon asked for a restart three times", "other", waiting.as_deref_mut())
}

/// Whether the daemon `pid`, which closed a connection without a line of an answer, is ending or gone: its
/// advertisement withdrawn or another daemon's, or no live daemon holding the lock. A daemon retires before it
/// closes its connections and a dead one's lock is free, but a dying process may let go of its lock a moment after
/// its connections: looked at for `GONE` before the daemon counts as still there.
fn ending(paths: &Paths, pid: u32) -> bool {
    let until = Instant::now() + GONE;
    loop {
        if Info::read(&paths.info).is_none_or(|i| i.pid != pid) || !daemon_alive(&paths.lock) {
            return true;
        }
        if Instant::now() >= until {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn ended_line(paths: &Paths) -> String {
    format!("teq: the daemon ended without an answer (see {})", paths.log.display())
}

/// The request ends without a daemon having served it to its end: the line on stderr, and where the request names
/// a file for the records (`--diagnostics`), the failure's record after what this request's records hold: nothing
/// where no daemon began to serve it, the daemon's own records where one began. Exits 2.
fn unserved(records: Option<&Records>, line: &str, kind: &str, waiting: Option<&mut Waiting>) -> i32 {
    if let Some(w) = waiting {
        w.before_output();
    }
    eprintln!("{}", line);
    if let Some(r) = records {
        let record = obj([("severity", "failure".into()), ("kind", kind.into()), ("message", line.strip_prefix("teq: ").unwrap_or(line).into()), ("shown", format!("{}\n", line).as_str().into())]);
        let written = std::fs::OpenOptions::new().create(true).append(true).open(&r.own).and_then(|mut f| f.write_all(format!("{}\n", record.to_text()).as_bytes()));
        if let Err(e) = written {
            eprintln!("teq: cannot write {}: {}", r.named.display(), e);
        }
    }
    2
}

/// The running daemon: `daemon.json` read, the lock held by a live daemon, its port answering. One of `refused`,
/// which answered `restart` to this client and is ending, is none: it would answer the same again.
fn connect(paths: &Paths, refused: &[u32]) -> Option<(TcpStream, Info)> {
    let info = Info::read(&paths.info)?;
    if refused.contains(&info.pid) || !daemon_alive(&paths.lock) {
        return None;
    }
    let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, info.port)).ok()?;
    let _ = stream.set_nodelay(true);
    Some((stream, info))
}

/// How an exchange with a daemon ended: with the request's exit code; with the daemon asking for a new one
/// (`restart`: it is ending, or of another binary or export); with the request not written whole (`Unsent`); with
/// the stream closed before any line of an answer (`Nothing`); or closed after some (`Ended`).
enum Answered {
    Exit(i32),
    Restart,
    Unsent,
    Nothing,
    Ended,
}

/// Sends the request and prints the answer, keeping its data.
fn exchange(mut stream: TcpStream, message: &Json, data: &mut Vec<Json>, mut waiting: Option<&mut Waiting>) -> Answered {
    let mut line = message.to_text();
    line.push('\n');
    if stream.write_all(line.as_bytes()).is_err() {
        return Answered::Unsent;
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
    let mut answered = false;
    for line in lines {
        let Some(line) = line else {
            if let Some(w) = waiting.as_deref_mut() {
                w.tick();
            }
            continue;
        };
        answered = true;
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
            return Answered::Exit(code as i32);
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
            return Answered::Restart;
        }
        if let Some(w) = waiting.as_deref_mut() {
            w.after_line();
        }
    }
    if answered {
        Answered::Ended
    } else {
        Answered::Nothing
    }
}

/// Starts the daemon unless another client started it meanwhile, and connects to it. Where the wait is told
/// (`waiting`), the client looks at its clock while it waits for the start's lock and for the daemon to listen.
fn start(export: &Export, paths: &Paths, identity: &str, refused: &[u32], mut waiting: Option<&mut Waiting>) -> Result<(TcpStream, Info), String> {
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
    if let Some(c) = connect(paths, refused) {
        return Ok(c);
    }
    // A live daemon holds the lock and is not one to ask: the one started now waits for it to stop.
    let previous = daemon_alive(&paths.lock);
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
    // The wait for the previous daemon is told from the first second: through the reporter where the request has
    // one, on stderr once where it has none.
    let mut told = false;
    let started = Instant::now();
    if let (true, Some(w)) = (previous, waiting.as_deref_mut()) {
        w.previous(true);
    }
    while Instant::now() < until {
        if let Some(info) = Info::read(&paths.info).filter(|i| i.pid == daemon.id()) {
            if let Ok(stream) = TcpStream::connect((Ipv4Addr::LOCALHOST, info.port)) {
                let _ = stream.set_nodelay(true);
                if let Some(w) = waiting.as_deref_mut() {
                    w.previous(false);
                }
                return Ok((stream, info));
            }
        }
        if let Ok(Some(status)) = daemon.try_wait() {
            let tail = std::fs::read_to_string(&paths.log).unwrap_or_default();
            let tail: Vec<&str> = tail.lines().rev().take(5).collect();
            return Err(format!("the daemon ended ({}) before it listened: {}", status, tail.into_iter().rev().collect::<Vec<_>>().join(" | ")));
        }
        if previous && waiting.is_none() && !told && started.elapsed() > QUIET {
            eprintln!("{}", PREVIOUS);
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
        // Both streams in one kept writer: a test reads the result lines in their order among the status's.
        (Waiting::new(progress, Box::new(kept.clone()), said, began, Box::new(clock(&moved))), kept, moved)
    }

    fn plan(residents: &[(&str, Vec<&str>, bool)]) -> Json {
        let residents = residents
            .iter()
            .map(|(key, projects, started)| obj([("key", (*key).into()), ("projects", Json::Arr(projects.iter().map(|p| (*p).into()).collect())), ("started", (*started).into())]))
            .collect();
        obj([("plan", Json::Arr(residents))])
    }

    fn done(errors: usize) -> Json {
        obj([("done", obj([("errors", Json::Num(errors as f64))]))])
    }

    /// A build's record as the daemon sends it: a JVM build of the projects' main and test that wrote `written`
    /// classes in `ms`, with what `more` says otherwise.
    fn built(projects: &[&str], written: usize, ms: f64, more: &[(&str, Json)]) -> Json {
        let mut fields: Vec<(String, Json)> = vec![
            ("projects".to_string(), Json::Arr(projects.iter().map(|p| (*p).into()).collect())),
            ("test".to_string(), true.into()),
            ("wider".to_string(), false.into()),
            ("jvm".to_string(), true.into()),
            ("ok".to_string(), true.into()),
            ("written".to_string(), Json::Num(written as f64)),
            ("typed".to_string(), true.into()),
            ("ms".to_string(), Json::Num(ms)),
            ("errors".to_string(), Json::Num(0.0)),
        ];
        for (name, value) in more {
            fields.retain(|(n, _)| n != name);
            fields.push((name.to_string(), value.clone()));
        }
        obj([("built", Json::Obj(fields))])
    }

    #[test]
    fn one_build_s_line_is_the_whole_answer() {
        for terminal in [false, true] {
            let (mut w, kept, clock) = waiting(terminal, "compile");
            w.record(&plan(&[("a/test", vec!["a", "core"], true)]));
            *clock.lock().unwrap() = 900;
            w.tick();
            w.record(&obj([("answered", "a/test".into())]));
            w.record(&built(&["a"], 31, 107.4, &[]));
            *clock.lock().unwrap() = 950;
            w.record(&done(0));
            w.finish();
            assert_eq!(kept.taken(), "a: compiled main and test, 31 classes written, in 107 ms\n", "terminal: {}", terminal);
        }
    }

    #[test]
    fn a_line_says_what_the_build_covered_and_did() {
        let line = |record: Json| {
            let (mut w, kept, _) = waiting(false, "compile");
            w.record(&plan(&[("a/test", vec!["a"], false)]));
            w.record(&record);
            kept.taken()
        };
        assert_eq!(line(built(&["a"], 1, 16.0, &[])), "a: compiled main and test, 1 class written, in 16 ms\n");
        assert_eq!(line(built(&["a"], 0, 87.0, &[])), "a: compiled main and test, no class changed, in 87 ms\n");
        assert_eq!(line(built(&["a"], 0, 3.0, &[("typed", false.into())])), "a: up to date\n");
        assert_eq!(line(built(&["util"], 12, 60.0, &[("test", false.into())])), "util: compiled main, 12 classes written, in 60 ms\n");
        assert_eq!(line(built(&["web"], 0, 4_836.0, &[("jvm", false.into())])), "web: checked main and test in 4.8 s\n");
        assert_eq!(line(built(&["one"], 0, 5.0, &[("jvm", false.into()), ("test", false.into())])), "one: checked main in 5 ms\n");
        // Several projects asked for, one build: one line, named by them in the order asked.
        assert_eq!(line(built(&["a", "core"], 3, 212.0, &[])), "a, core: compiled main and test, 3 classes written, in 212 ms\n");
        // A build wider than the request has no count of classes.
        assert_eq!(line(built(&["core"], 2, 104.0, &[("wider", true.into()), ("test", false.into())])), "core: compiled main in 104 ms\n");
        assert_eq!(line(built(&["core"], 0, 4.0, &[("wider", true.into()), ("test", false.into()), ("typed", false.into())])), "core: up to date\n");
        // The errors, and the project whose sources they are in where it is none of those asked for.
        assert_eq!(line(built(&["a"], 0, 9.0, &[("ok", false.into()), ("errors", Json::Num(2.0))])), "a: 2 errors\n");
        let shares = |shares: &[(&str, f64)]| Json::Arr(shares.iter().map(|(p, n)| Json::Arr(vec![(*p).into(), Json::Num(*n)])).collect());
        assert_eq!(line(built(&["core"], 0, 9.0, &[("ok", false.into()), ("errors", Json::Num(1.0)), ("wider", true.into()), ("in", shares(&[("a", 1.0)]))])), "core: 1 error in a\n");
        // Some of the errors the project's own, or the others in several projects: the whole count, then the shares.
        assert_eq!(line(built(&["a"], 0, 9.0, &[("ok", false.into()), ("errors", Json::Num(5.0)), ("in", shares(&[("core", 2.0)]))])), "a: 5 errors, 2 in core\n");
        assert_eq!(line(built(&["a"], 0, 9.0, &[("ok", false.into()), ("errors", Json::Num(5.0)), ("in", shares(&[("core", 2.0), ("util", 1.0)]))])), "a: 5 errors, 2 in core, 1 in util\n");
        assert_eq!(line(built(&["a"], 0, 9.0, &[("ok", false.into()), ("errors", Json::Num(3.0)), ("in", shares(&[("core", 2.0), ("util", 1.0)]))])), "a: 3 errors, 2 in core, 1 in util\n");
    }

    #[test]
    fn several_builds_end_with_a_total_and_the_ones_up_to_date_have_no_line() {
        let (mut w, kept, clock) = waiting(false, "compile");
        w.record(&plan(&[("a/test", vec!["a", "b", "core"], true), ("d/compile", vec!["d"], false)]));
        *clock.lock().unwrap() = 999;
        w.tick();
        assert_eq!(kept.taken(), "");
        *clock.lock().unwrap() = 1_000;
        w.tick();
        assert_eq!(kept.taken(), "compiling a, b, core (1 of 2), started now\ncompiling d (2 of 2)\n");
        *clock.lock().unwrap() = 5_000;
        w.tick();
        w.record(&obj([("answered", "d/compile".into())]));
        w.record(&built(&["d"], 0, 2.0, &[("typed", false.into()), ("test", false.into())]));
        assert_eq!(kept.taken(), "");
        w.record(&obj([("answered", "a/test".into())]));
        w.record(&built(&["a", "b", "core"], 31, 13_900.0, &[]));
        assert_eq!(kept.taken(), "a, b, core: compiled main and test, 31 classes written, in 13.9 s\n");
        *clock.lock().unwrap() = 14_230;
        w.record(&done(0));
        assert_eq!(kept.taken(), "teq compile: 4 projects in 14.2 s\n");
        w.finish();
        assert_eq!(kept.taken(), "");
        // Every build up to date: the total alone says so.
        let (mut w, kept, _) = waiting(false, "compile");
        w.record(&plan(&[("a/test", vec!["a", "core"], false), ("d/compile", vec!["d"], false)]));
        w.record(&built(&["a", "core"], 0, 3.0, &[("typed", false.into())]));
        w.record(&built(&["d"], 0, 2.0, &[("typed", false.into()), ("test", false.into())]));
        w.record(&done(0));
        assert_eq!(kept.taken(), "teq compile: 3 projects, up to date\n");
        // An error among them: its line, and the total counts it.
        let (mut w, kept, clock) = waiting(false, "compile");
        w.record(&plan(&[("a/test", vec!["a", "core"], false), ("d/compile", vec!["d"], false)]));
        w.record(&built(&["a", "core"], 0, 9.0, &[("ok", false.into()), ("errors", Json::Num(1.0))]));
        w.record(&built(&["d"], 0, 2.0, &[("typed", false.into()), ("test", false.into())]));
        *clock.lock().unwrap() = 400;
        w.record(&done(1));
        assert_eq!(kept.taken(), "a, core: 1 error\nteq compile: 3 projects in 400 ms, 1 error\n");
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
        assert_eq!(kept.taken(), "compiling a (1 of 1), started now\n");
        w.after_line();
        assert_eq!(kept.taken(), "");
        w.record(&obj([("answered", "a/compile".into())]));
        *clock.lock().unwrap() = 1_400;
        w.after_line();
        w.record(&done(0));
        assert_eq!(kept.taken(), "");
        // No plan has come: a line read (a refusal's) is no look at the clock.
        let (mut w, kept, clock) = waiting(true, "compile");
        *clock.lock().unwrap() = 1_300;
        w.after_line();
        w.record(&done(0));
        assert_eq!(kept.taken(), "");
    }

    #[test]
    fn an_exchange_that_ends_without_an_answer_leaves_no_status_and_no_line() {
        // On a terminal, no plan come: the status, a refusal's line, the end.
        let erase = "\r\x1b[K";
        let (mut w, kept, clock) = waiting(true, "compile");
        *clock.lock().unwrap() = 1_400;
        w.tick();
        assert_eq!(kept.taken(), format!("{}teq compile: starting, 1 s", erase));
        *clock.lock().unwrap() = 1_450;
        w.finish();
        assert_eq!(kept.taken(), erase);
        w.finish();
        assert_eq!(kept.taken(), "");
        // Off a terminal, a header told and the daemon gone: nothing more.
        let (mut w, kept, clock) = waiting(false, "run");
        w.record(&plan(&[("a/compile", vec!["a"], false)]));
        *clock.lock().unwrap() = 1_200;
        w.tick();
        assert_eq!(kept.taken(), "compiling a (1 of 1)\n");
        *clock.lock().unwrap() = 2_000;
        w.finish();
        assert_eq!(kept.taken(), "");
    }

    #[test]
    fn a_request_s_records_go_to_a_file_of_its_own_and_are_moved_at_its_end() {
        let dir = std::env::temp_dir().join(format!("teq-records-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let args: Vec<String> = ["app", "--diagnostics", "records.jsonl"].map(str::to_string).to_vec();
        std::fs::write(dir.join("records.jsonl"), "an earlier run\n").unwrap();
        let (records, sent) = Records::of(&args, &dir).unwrap();
        // The named file is emptied as the request starts; the daemon is given the request's own, beside it.
        assert_eq!(std::fs::read_to_string(&records.named).unwrap(), "");
        assert_eq!(records.own.parent(), records.named.parent());
        assert_eq!(records.own.file_name().unwrap().to_string_lossy(), format!("records.jsonl.teq-{}", std::process::id()));
        assert_eq!(sent, ["app", "--diagnostics", records.own.to_str().unwrap()]);
        // Another request's records land in the named file while this one runs: this one's replace them whole.
        std::fs::write(&records.named, "another request\n").unwrap();
        std::fs::write(&records.own, "this request\n").unwrap();
        unserved(Some(&records), "teq: the daemon ended without an answer (see log)", "crash", None);
        records.end();
        let held = std::fs::read_to_string(&records.named).unwrap();
        assert_eq!(held.lines().count(), 2, "{held}");
        assert!(held.starts_with("this request\n{\"severity\":\"failure\",\"kind\":\"crash\","), "{held}");
        assert!(!records.own.exists());
        // A command that names no file has no records; one that names what is no regular file writes it as it is.
        assert!(Records::of(&["app".to_string()], &dir).is_none());
        let (direct, sent) = Records::of(&["--diagnostics".to_string(), ".".to_string()], &dir).unwrap();
        assert_eq!(direct.own, direct.named);
        assert_eq!(sent[1], direct.named.to_str().unwrap());
        direct.end();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_request_that_is_not_written_is_told_from_one_that_got_no_answer() {
        let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let request = obj([("verb", "compile".into())]);
        // The connection's writing side closed before the request: nothing of it was sent.
        let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        stream.shutdown(std::net::Shutdown::Write).unwrap();
        assert!(matches!(exchange(stream, &request, &mut Vec::new(), None), Answered::Unsent));
        // The request read and the connection closed without a line.
        let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        let served = crate::alloc::spawn(move || {
            // The first connection, which sent nothing; then this one, read to its request's end and closed.
            drop(listener.accept().unwrap());
            let (peer, _) = listener.accept().unwrap();
            let mut line = String::new();
            BufReader::new(peer).read_line(&mut line).unwrap();
            line
        });
        assert!(matches!(exchange(stream, &request, &mut Vec::new(), None), Answered::Nothing));
        assert_eq!(served.join().unwrap().trim_end(), request.to_text());
    }

    #[test]
    fn a_daemon_that_said_nothing_is_ending_only_where_it_is_seen_to() {
        let dir = std::env::temp_dir().join(format!("teq-ending-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = paths(&dir);
        std::fs::create_dir_all(paths.info.parent().unwrap()).unwrap();
        let info = |pid: u32| Info { port: 1, token: String::new(), pid, sha256: String::new() };
        // Its advertisement stands and its lock is held: still there, said after the look's bound.
        let lock = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(&paths.lock).unwrap();
        lock.lock().unwrap();
        info(7).write(&paths.info).unwrap();
        let began = Instant::now();
        assert!(!ending(&paths, 7));
        assert!(began.elapsed() >= GONE);
        // Another daemon's advertisement, or none: it retired.
        assert!(ending(&paths, 8));
        std::fs::remove_file(&paths.info).unwrap();
        assert!(ending(&paths, 7));
        // Its advertisement left behind and no daemon holding the lock: it is gone.
        info(7).write(&paths.info).unwrap();
        lock.unlock().unwrap();
        assert!(ending(&paths, 7));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_wait_for_the_previous_daemon_is_told_from_the_first_second() {
        // On a terminal the status says it, until the daemon started listens; then the start's own words.
        let erase = "\r\x1b[K";
        let (mut w, kept, clock) = waiting(true, "compile");
        w.previous(true);
        assert_eq!(kept.taken(), "");
        *clock.lock().unwrap() = 3_200;
        w.tick();
        assert_eq!(kept.taken(), format!("{}teq compile: waiting for the previous teq daemon to stop, 3 s", erase));
        *clock.lock().unwrap() = 4_100;
        w.previous(false);
        assert_eq!(kept.taken(), format!("{}teq compile: starting, 4 s", erase));
        // Off a terminal one line, once.
        let (mut w, kept, clock) = waiting(false, "compile");
        w.previous(true);
        *clock.lock().unwrap() = 1_000;
        w.tick();
        assert_eq!(kept.taken(), "teq: waiting for the previous teq daemon to stop\n");
        *clock.lock().unwrap() = 9_000;
        w.tick();
        w.previous(false);
        assert_eq!(kept.taken(), "");
    }

    #[test]
    fn a_build_that_failed_outright_has_no_line_of_the_client_s_and_counts_in_the_total() {
        let (mut w, kept, clock) = waiting(false, "compile");
        w.record(&plan(&[("a/test", vec!["a"], false), ("d/compile", vec!["d"], false)]));
        w.record(&obj([("built", obj([("projects", Json::Arr(vec!["a".into()])), ("failed", true.into())]))]));
        assert_eq!(kept.taken(), "");
        w.record(&built(&["d"], 0, 2.0, &[("typed", false.into()), ("test", false.into())]));
        *clock.lock().unwrap() = 400;
        w.record(&done(1));
        assert_eq!(kept.taken(), "teq compile: 2 projects in 400 ms, 1 error\n");
    }

    #[test]
    fn run_tells_a_build_only_where_it_wrote_or_failed() {
        let line = |record: Json| {
            let (mut w, kept, _) = waiting(false, "run");
            w.record(&plan(&[("a/test", vec!["a"], false)]));
            w.record(&record);
            w.record(&done(0));
            kept.taken()
        };
        assert_eq!(line(built(&["a"], 1, 16.0, &[])), "a: compiled main and test, 1 class written, in 16 ms\n");
        assert_eq!(line(built(&["a"], 0, 87.0, &[])), "");
        assert_eq!(line(built(&["a"], 0, 3.0, &[("typed", false.into())])), "");
        assert_eq!(line(built(&["a"], 0, 9.0, &[("ok", false.into()), ("errors", Json::Num(1.0))])), "a: 1 error\n");
    }

    #[test]
    fn on_a_terminal_the_status_names_what_waits_and_gives_its_row_to_every_line() {
        let erase = "\r\x1b[K";
        let (mut w, kept, clock) = waiting(true, "compile");
        *clock.lock().unwrap() = 1_200;
        w.tick();
        assert_eq!(kept.taken(), format!("{}teq compile: starting, 1 s", erase));
        *clock.lock().unwrap() = 2_100;
        w.record(&plan(&[("a/test", vec!["a", "b", "core"], true), ("d/compile", vec!["d", "e"], false)]));
        assert_eq!(kept.taken(), format!("{}compiling a, d (5 projects), 2 s", erase));
        *clock.lock().unwrap() = 3_050;
        w.tick();
        assert_eq!(kept.taken(), format!("{}compiling a, d (5 projects), 3 s", erase));
        w.record(&obj([("answered", "a/test".into())]));
        assert_eq!(kept.taken(), format!("{}compiling d (2 projects), 3 s", erase));
        // A result line, and a line of the daemon's for either stream: the status erased before it, drawn again at
        // the next look.
        w.record(&built(&["a", "b", "core"], 4, 2_900.0, &[]));
        assert_eq!(kept.taken(), format!("{}a, b, core: compiled main and test, 4 classes written, in 2.9 s\n", erase));
        w.tick();
        assert_eq!(kept.taken(), format!("{}compiling d (2 projects), 3 s", erase));
        w.before_output();
        assert_eq!(kept.taken(), erase);
        w.tick();
        assert_eq!(kept.taken(), format!("{}compiling d (2 projects), 3 s", erase));
        w.record(&obj([("answered", "d/compile".into())]));
        assert_eq!(kept.taken(), erase);
        w.record(&built(&["d", "e"], 0, 2.0, &[("typed", false.into()), ("test", false.into())]));
        *clock.lock().unwrap() = 4_000;
        w.record(&done(0));
        assert_eq!(kept.taken(), "teq compile: 5 projects in 4.0 s\n");
    }
}

