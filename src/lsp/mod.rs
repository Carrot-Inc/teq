//! `teq lsp`: a language server over stdin and stdout for navigation and diagnostics, which
//! drives one resident check per project (`teq compiler watch --check --index`, `src/watch.rs`) as a
//! child process and speaks JSON-RPC to the client (docs/TARGETS.md, "The language server").
//!
//! One thread reads the client's messages, one per child reads its answers, and this loop
//! handles both in order. A child answers its commands in the order they were written, so a
//! query written after a build is answered by the program that build made: the loop keeps per
//! child the answers it waits for, in order, and a request that asks several children is
//! answered once the last of them has. A child has one build under way at most, which carries an
//! identity its answer echoes; a build a newer edit of a document it carried overtakes is stale:
//! asked to stop (`cancel`), and its answer dropped (docs/TARGETS.md, "Documents and diagnostics").

mod export;
pub(crate) mod hooks;
pub mod json;
mod rpc;
pub mod uri;
mod workspace;

use crate::task::export::{Export, Key};
use json::{obj, Json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use uri::canonical;
use workspace::Description;

/// How long a burst of edits is gathered before the build it causes is sent.
const COALESCE: Duration = Duration::from_millis(100);
/// How long a stream of edits can push the build back before it is typed anyway.
const COALESCE_BOUND: Duration = Duration::from_millis(400);
/// How long a mapping the server freed is kept for the next request of its size, and half of
/// how long at most (`alloc::age`): the texts of a large document, once it is closed.
const AGE: Duration = Duration::from_secs(1);
/// How long a session stays free after its last build before a plain build, the idle poll,
/// reconciles it with the disk.
const IDLE: Duration = Duration::from_secs(3);
/// How often the folders are searched again for exports while idle.
const REDISCOVER: Duration = Duration::from_secs(10);
/// How long a build is pending before the client is shown it as work in progress; a child's
/// first build is shown at once.
const PROGRESS_AFTER: Duration = Duration::from_millis(300);
/// How long after the last change to a build's files a failed export runs again.
const EXPORT_QUIET: Duration = Duration::from_secs(2);
/// How many sessions run at once and how long one stays unused before it is stopped, unless the
/// client's initialization options say otherwise (`maxSessions`, `sessionIdleSeconds`): each
/// holds its closure's typed program.
const MAX_SESSIONS: usize = 4;
const SESSION_IDLE: Duration = Duration::from_secs(30 * 60);

/// How many symbols `workspace/symbol` answers at most.
const WORKSPACE_SYMBOLS: usize = 200;

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const SERVER_NOT_INITIALIZED: i64 = -32002;
/// A request whose document changed before it could be answered: the client asks again.
const CONTENT_MODIFIED: i64 = -32801;

pub(crate) enum Event {
    Client(Result<String, String>),
    ClientGone,
    /// A line a child printed: its session and the generation of the child that printed it.
    Line(usize, u64, String),
    Exited(usize, u64),
    /// A build's sbt export ended: its folder and build, whether it succeeded, and the end of
    /// sbt's output.
    Exported(usize, usize, bool, String),
    /// A session's generators ran on a worker: the session, the generation it ran for, and why
    /// they failed.
    Generated(usize, u64, Result<(), String>),
}

/// What a child's next answer is for.
enum Expect {
    Build(Sent),
    /// A query and the command that asked it.
    Query(u64, String),
    /// The answer of a query written behind a build that was cancelled, which that build's
    /// program never existed to answer: dropped, the query asked again after the next build.
    Discard,
}

/// A build sent to a child, which carries its identity (`build #<id>`) and whose answer echoes
/// it: when it was sent, whether it reads every file and whether it is the idle poll alone, the
/// documents it carried and the stamp of every text the child held. It is stale once a document
/// it carried has a newer stamp: its answer is then dropped, and its cancel sent to the child
/// (`cancelled`), which stops it if it has not begun typing.
struct Sent {
    id: u64,
    sent: Instant,
    plain: bool,
    poll: bool,
    carried: Vec<PathBuf>,
    typed: BTreeMap<PathBuf, Stamp>,
    cancelled: bool,
}

/// The server's mark of a document's open, change or close, counted across the documents, and
/// the client's version of the text it marks (none once closed): two texts of a path differ by
/// their stamps, whatever versions a client reuses after a close.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Stamp {
    n: u64,
    version: Option<i64>,
}

/// A child and the thread that writes its commands, so that a child that does not read blocks
/// that thread alone and never the loop. What waits to be written is bounded: a document's text
/// that the child has not taken yet is replaced by the next one, and past `OUTBOX_BOUND` bytes
/// the child is ended.
struct Proc {
    child: Arc<Mutex<Child>>,
    outbox: Arc<(Mutex<Outbox>, std::sync::Condvar)>,
    /// The argument file the child was started with, removed once it is gone.
    args_file: Option<PathBuf>,
}

enum Item {
    /// The text of a document (`text`, `empty`), `None` to withdraw it.
    Text(PathBuf, Option<String>),
    Line(String),
    Quit,
}

#[derive(Default)]
struct Outbox {
    items: VecDeque<Item>,
    bytes: usize,
    closed: bool,
}

/// How long a child asked to quit has before it is killed.
const QUIT_BOUND: Duration = Duration::from_secs(3);
/// How many bytes of commands may wait for a child that does not read.
const OUTBOX_BOUND: usize = 64 << 20;

fn item_bytes(item: &Item) -> usize {
    match item {
        Item::Text(_, Some(t)) => t.len(),
        Item::Line(l) => l.len(),
        _ => 0,
    }
}

impl Proc {
    fn start(child: Child, mut stdin: ChildStdin, args_file: Option<PathBuf>) -> Proc {
        let outbox: Arc<(Mutex<Outbox>, std::sync::Condvar)> = Arc::default();
        let queue = outbox.clone();
        crate::alloc::spawn(move || loop {
            let item = {
                let (lock, ready) = &*queue;
                let mut o = lock.lock().unwrap_or_else(|e| e.into_inner());
                loop {
                    if let Some(item) = o.items.pop_front() {
                        o.bytes -= item_bytes(&item);
                        break item;
                    }
                    if o.closed {
                        return;
                    }
                    // What was written is freed here and allocated by the loop.
                    crate::alloc::settle();
                    o = ready.wait(o).unwrap_or_else(|e| e.into_inner());
                }
            };
            let bytes = match &item {
                Item::Text(path, Some(t)) if t.is_empty() => format!("empty {}\n", path.display()),
                Item::Text(path, Some(t)) => format!("text {} {}\n{}", path.display(), t.len(), t),
                Item::Text(path, None) => format!("text {} 0\n", path.display()),
                Item::Line(l) => l.clone(),
                Item::Quit => "quit\n".to_string(),
            };
            if stdin.write_all(bytes.as_bytes()).and_then(|_| stdin.flush()).is_err() || matches!(item, Item::Quit) {
                return;
            }
            if let Item::Line(l) = &item {
                if let Some(id) = l.strip_prefix("cancel ") {
                    hooks::trace("server", || format!("wrote-cancel {}", id.trim_end()));
                }
            }
        });
        Proc { child: Arc::new(Mutex::new(child)), outbox, args_file }
    }

    /// Queues an item; false when the child is ended for what waits for it.
    fn push(&self, item: Item) -> bool {
        let (lock, ready) = &*self.outbox;
        let mut o = lock.lock().unwrap_or_else(|e| e.into_inner());
        if o.closed {
            return false;
        }
        if let Item::Text(path, text) = &item {
            let unsent = o.items.iter().position(|i| matches!(i, Item::Text(p, _) if p == path));
            if let Some(i) = unsent {
                let old = std::mem::replace(&mut o.items[i], Item::Text(path.clone(), text.clone()));
                o.bytes = o.bytes - item_bytes(&old) + text.as_ref().map_or(0, String::len);
                ready.notify_one();
                return true;
            }
        }
        o.bytes += item_bytes(&item);
        o.items.push_back(item);
        if o.bytes > OUTBOX_BOUND {
            o.items.clear();
            o.closed = true;
            drop(o);
            self.kill_now();
            return false;
        }
        ready.notify_one();
        true
    }

    fn send(&self, line: &str) -> bool {
        self.push(Item::Line(line.to_string()))
    }

    /// Asks the child to quit and, on a thread of its own, kills it if it has not within
    /// `QUIT_BOUND`; the server keeps the child to kill it at `exit`.
    fn end(&self) {
        self.push(Item::Quit);
        let child = self.child.clone();
        crate::alloc::spawn(move || {
            let until = Instant::now() + QUIT_BOUND;
            loop {
                let mut c = child.lock().unwrap_or_else(|e| e.into_inner());
                if !matches!(c.try_wait(), Ok(None)) {
                    return;
                }
                if Instant::now() >= until {
                    let _ = c.kill();
                    let _ = c.wait();
                    return;
                }
                drop(c);
                std::thread::sleep(Duration::from_millis(50));
            }
        });
    }

    fn kill_now(&self) {
        {
            let (lock, ready) = &*self.outbox;
            let mut o = lock.lock().unwrap_or_else(|e| e.into_inner());
            o.closed = true;
            ready.notify_all();
        }
        let mut c = self.child.lock().unwrap_or_else(|e| e.into_inner());
        let _ = c.kill();
        let _ = c.wait();
    }

    fn kill(self) {
        self.kill_now();
        if let Some(file) = &self.args_file {
            let _ = std::fs::remove_file(file);
        }
    }
}

struct Session {
    /// The workspace folder whose bare project this is, or the export's root.
    folder: PathBuf,
    /// Its export or configuration is gone: it owns nothing and starts nothing; its index stays,
    /// since the children's events name a session by it.
    retired: bool,
    /// The export the session's configuration is of; none for a bare root.
    export: Option<PathBuf>,
    key: Option<Key>,
    description: Result<Description, String>,
    /// Stopped for being unused or past the cap: started again by the next edit or request that
    /// concerns it.
    parked: bool,
    last_used: Instant,
    /// The inputs that existed when the child started: a declared root created since is not
    /// among them, so a session whose set changed is started anew.
    started_inputs: Vec<PathBuf>,
    /// The generation a worker runs the closure's generators for, before a plain build.
    generating: Option<u64>,
    /// The canonical inputs and excludes, which decide what files the session owns.
    roots: Vec<PathBuf>,
    excluded: Vec<PathBuf>,
    proc: Option<Proc>,
    generation: u64,
    /// What the child's answers are for, in order: one build under way at most.
    expect: VecDeque<Expect>,
    /// Documents whose text changed since it was last sent.
    dirty: BTreeSet<PathBuf>,
    /// A build of every file asked for while one was under way.
    plain_wanted: bool,
    /// Whether the build of every file asked for is the idle poll's alone (`reconcile`).
    poll: bool,
    /// What a cancelled build carried, which the next carries (the child types it again).
    carry: Vec<PathBuf>,
    carry_plain: bool,
    /// The stamp of each document's text the child was last sent.
    sent: BTreeMap<PathBuf, Stamp>,
    /// Queries about a document whose latest text the child has not been sent while a build is
    /// under way, asked after the build that sends it, so that their offsets are in that text.
    deferred: Vec<(u64, String)>,
    /// The diagnostics of its last build answered for the texts it typed, by file, and the stamps
    /// of those texts: its part of a document's diagnostics is stale once the stamp moved on.
    diagnostics: BTreeMap<PathBuf, Vec<Json>>,
    typed: BTreeMap<PathBuf, Stamp>,
    /// When its last build was sent or answered: free that long, the session polls (`IDLE`).
    last_build: Instant,
    /// The client's progress token for its pending builds, while one is begun.
    progress: Option<String>,
    stderr: Arc<Mutex<String>>,
    /// Why the child is gone, until it is started again.
    failure: Option<String>,
    /// The notes of its answers told to the client (`window/logMessage`), each once.
    told: BTreeSet<String>,
}

impl Session {
    fn new(folder: PathBuf, export: Option<PathBuf>, key: Option<Key>, description: Result<Description, String>) -> Session {
        let (roots, excluded) = match &description {
            Ok(d) => (d.inputs.iter().map(|p| canonical(p)).collect(), d.excludes.iter().map(|p| canonical(p)).collect()),
            Err(_) => (Vec::new(), Vec::new()),
        };
        Session {
            folder,
            retired: false,
            export,
            key,
            description,
            parked: false,
            last_used: Instant::now(),
            started_inputs: Vec::new(),
            generating: None,
            roots,
            excluded,
            proc: None,
            generation: 0,
            expect: VecDeque::new(),
            dirty: BTreeSet::new(),
            plain_wanted: false,
            poll: false,
            carry: Vec::new(),
            carry_plain: false,
            sent: BTreeMap::new(),
            deferred: Vec::new(),
            diagnostics: BTreeMap::new(),
            typed: BTreeMap::new(),
            last_build: Instant::now(),
            progress: None,
            stderr: Arc::new(Mutex::new(String::new())),
            failure: None,
            told: BTreeSet::new(),
        }
    }

    /// Whether the session's program reads libraries, whose sources' documents it answers on:
    /// a class path, or scala-library as the std.
    fn reads_libraries(&self) -> bool {
        self.description.as_ref().map_or(false, |d| !d.classpath.is_empty() || d.jvm_std.as_deref() == Some("scala-library"))
    }

    /// Whether the session answers on the document `path` of a file no session owns: a jar's or a
    /// product's where it reads libraries, a std file's where its std is the embedded one, each
    /// session typing its own std (a document of another binary's std answers nothing there).
    fn serves_document(&self, path: &Path) -> bool {
        match crate::typer::loader::attach::is_std_document(path) {
            true => self.embedded_std(),
            false => self.reads_libraries(),
        }
    }

    /// Whether the session's std is the embedded one, whose files are documents of their own: not
    /// a JVM session's, nor one given `--std=scala-library`, which scala-library's jar stands for.
    fn embedded_std(&self) -> bool {
        self.description.as_ref().map_or(true, |d| {
            let flags = &d.flags;
            let std = flags.iter().enumerate().filter_map(|(i, f)| f.strip_prefix("--std=").or_else(|| (f == "--std").then(|| flags.get(i + 1).map(String::as_str)).flatten())).last();
            d.jvm_std.is_none() && std.map_or(true, |m| m == "lean")
        })
    }

    fn owns(&self, file: &Path) -> bool {
        let under = |root: &PathBuf| file == root || file.starts_with(root);
        file.extension().map_or(false, |e| e == "scala") && self.roots.iter().any(under) && !self.excluded.iter().any(under)
    }

    /// The file diagnostics without a position are put on: the export, or the root.
    fn anchor(&self) -> PathBuf {
        match (&self.export, &self.description) {
            (Some(e), _) => e.clone(),
            (None, Ok(d)) => d.root.clone(),
            (None, Err(_)) => PathBuf::from("/"),
        }
    }

    /// The project as the client's status bar names it: the configuration, else the bare root's
    /// directory, else the anchor.
    fn label(&self) -> String {
        if let Some(key) = &self.key {
            return key.to_string();
        }
        match self.description.as_ref().ok().and_then(|d| d.root.file_name()) {
            Some(name) => name.to_string_lossy().into_owned(),
            None => self.anchor().display().to_string(),
        }
    }

    /// When the oldest of its pending builds was sent; the idle poll's only while an edit, a
    /// request or a notified change waits behind it: alone it is never shown as work in
    /// progress, since where reading every file's time takes longer than `PROGRESS_AFTER`
    /// (Windows) every poll would be. Generators running while a poll is under way are a
    /// notified change's (a poll's own run before it is sent, the session free).
    fn oldest_build(&self) -> Option<Instant> {
        let waited_on = self.wants_build() || self.generating.is_some() || !self.deferred.is_empty() || self.expect.iter().any(|e| matches!(e, Expect::Query(..)));
        self.expect.iter().find_map(|e| match e {
            Expect::Build(b) if !b.poll || waited_on => Some(b.sent),
            Expect::Build(_) => None,
            Expect::Query(..) | Expect::Discard => None,
        })
    }

    /// Whether a build waits to be sent: edits kept, what a cancelled build carried, or a build
    /// of every file asked for.
    fn wants_build(&self) -> bool {
        !self.dirty.is_empty() || self.plain_wanted || self.carry_plain || !self.carry.is_empty()
    }

    /// The build under way.
    fn building(&mut self) -> Option<&mut Sent> {
        self.expect.iter_mut().find_map(|e| match e {
            Expect::Build(b) => Some(b),
            Expect::Query(..) | Expect::Discard => None,
        })
    }

    fn send(&mut self, text: &str) -> bool {
        self.proc.as_ref().map_or(false, |p| p.send(text))
    }

    fn busy(&self) -> bool {
        !self.expect.is_empty()
    }

    /// When its idle poll is due: `IDLE` after its last build, while its child runs with nothing
    /// under way, pending or generating.
    fn poll_due(&self) -> Option<Instant> {
        (self.proc.is_some() && !self.busy() && self.dirty.is_empty() && self.generating.is_none()).then(|| self.last_build + IDLE)
    }
}

struct Doc {
    text: String,
    version: i64,
    /// The URI the client opened it under, which answers name it by.
    uri: String,
}

/// What `Pending::revision` holds.
struct Revision {
    path: PathBuf,
    version: i64,
    stamp: Option<Stamp>,
}

/// A client request waiting for its children's answers.
struct Pending {
    id: Json,
    method: String,
    waiting: usize,
    results: Vec<(usize, Json)>,
    /// The query of `workspace/symbol`, which orders the joined answers.
    query: String,
    /// The item of `completionItem/resolve`, which the answer completes.
    item: Option<Json>,
    /// The document a completion, a resolve or a signature help was asked on, its version and
    /// its stamp then (none for a file not open): its items' `data` name the version, and a
    /// query sent (or sent again, behind a cancelled build) once the document moved on, changed
    /// or closed, is answered `ContentModified`, since its revision is never typed.
    revision: Option<Revision>,
    /// For a query about a target: the command's verb and what follows the offset, which the
    /// sessions that hold the target's declaration and were not asked are asked with there.
    follow: Option<(&'static str, &'static str)>,
    asked: Vec<usize>,
    /// A query on a library document no session is remembered for, asked of every running
    /// session that reads libraries: the first that maps the document answers, in the sessions'
    /// order, and is remembered for the document.
    document: Option<PathBuf>,
    /// The file the request was asked on, checked again when the answer goes out: a library
    /// document open with another text than its file by then (another query made a damaged one
    /// whole while this one was pending) answers nothing (`library_document_changed`).
    origin: Option<PathBuf>,
}

/// A workspace folder, canonical and as the client spelled it, the exports that serve it, and the
/// sbt builds it stands in.
struct Folder {
    root: PathBuf,
    given: PathBuf,
    /// The export at or above the folder, else those at the roots of the builds below it; none
    /// for a bare root.
    exports: Vec<PathBuf>,
    builds: Vec<Build>,
}

impl Folder {
    fn find_exports(&self) -> Vec<PathBuf> {
        if let Some(file) = crate::task::export::find(&self.root) {
            return vec![crate::task::export::canonical(&file)];
        }
        self.builds.iter().filter_map(|b| crate::task::export::at(&b.root)).map(|f| crate::task::export::canonical(&f)).collect()
    }
}

/// An sbt build the folder stands in, and the export the server runs for it while it has none.
struct Build {
    root: PathBuf,
    running: Option<export::Running>,
    /// The client's progress token of the run under way.
    progress: Option<String>,
    /// A run asked for while this build's was under way, or while another build's was.
    again: bool,
    /// Why the last run failed, until one succeeds.
    failure: Option<String>,
    /// When a build file last changed; a failed export runs again once the changes rest.
    changed_at: Option<Instant>,
}

impl Build {
    fn new(root: PathBuf) -> Build {
        Build { root, running: None, progress: None, again: false, failure: None, changed_at: None }
    }

    /// Whether the build has an export, at its root or under its `target/teq/`, where its
    /// setting (`teqBuildTool`) has the export write it.
    fn exported(&self) -> bool {
        crate::task::export::at(&self.root).is_some()
    }

    /// Whether a run asked for may start now: one was asked for, and the changes that asked
    /// for it have rested.
    fn due(&self) -> bool {
        self.again && self.changed_at.map_or(true, |t| t.elapsed() >= EXPORT_QUIET)
    }
}

/// An export as the server read it: what it describes, or why it does not read, and its
/// modification time before the read.
struct Loaded {
    export: Result<std::sync::Arc<Export>, String>,
    mtime: Option<SystemTime>,
}

struct Server {
    events: Sender<Event>,
    /// `utf-8` or `utf-16`, as negotiated.
    positions: &'static str,
    initialized: bool,
    shutdown: bool,
    /// The children asked to quit at `shutdown`, which `exit` kills if they are still there.
    ending: Vec<Proc>,
    sessions: Vec<Session>,
    docs: BTreeMap<PathBuf, Doc>,
    /// Per library document, the session whose navigation produced it: the
    /// one its queries go to, woken when parked.
    doc_sessions: BTreeMap<PathBuf, usize>,
    pending: BTreeMap<u64, Pending>,
    next_query: u64,
    /// What was last published per file, and the version it was published with.
    published: BTreeMap<PathBuf, (Vec<Json>, Option<i64>)>,
    /// The latest stamp of every document opened since the start (`Stamp`), and the counters of
    /// the stamps and of the builds' identities.
    stamps: BTreeMap<PathBuf, Stamp>,
    /// The files `publish_changed` passed over while every running session that holds them had a
    /// stale part, published again with the next build answered (a withdrawal among them).
    held_back: BTreeSet<PathBuf>,
    next_stamp: u64,
    next_build: u64,
    coalesce_until: Option<Instant>,
    /// When the pending edits began, which bounds how long they push the build back.
    coalesce_start: Option<Instant>,
    exe: PathBuf,
    folders: Vec<Folder>,
    /// The diagnostics of the folders' exports (a failed run, on the build file), by file.
    folder_diagnostics: BTreeMap<PathBuf, Vec<Json>>,
    /// Whether the client takes a registration of the files to watch.
    watch_files: bool,
    /// Whether the client shows work-done progress.
    progress: bool,
    /// Numbers the requests to the client and the progress tokens.
    next_request: u64,
    /// When the folders were last searched for exports.
    discovered_at: Instant,
    exports: BTreeMap<PathBuf, Loaded>,
    max_sessions: usize,
    session_idle: Duration,
    /// Whether the client takes a completion item's edit as an insert and a replace range.
    insert_replace: bool,
    /// Whether the client takes a completion item's text as a snippet.
    snippets: bool,
}


pub fn run(args: &[String]) -> ! {
    if !args.is_empty() && args.iter().any(|a| a != "--stdio") {
        eprintln!("usage: teq lsp [--stdio]: a language server over stdin and stdout");
        std::process::exit(2);
    }
    let (events, inbox) = channel();
    let client = events.clone();
    crate::alloc::spawn(move || {
        let mut input = BufReader::new(std::io::stdin());
        while let Some(message) = rpc::read_message(&mut input) {
            if client.send(Event::Client(message)).is_err() {
                return;
            }
        }
        let _ = client.send(Event::ClientGone);
    });
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("teq"));
    export::end_exports_on_termination();
    let mut server = Server {
        events,
        positions: "utf-16",
        initialized: false,
        shutdown: false,
        ending: Vec::new(),
        sessions: Vec::new(),
        docs: BTreeMap::new(),
        doc_sessions: BTreeMap::new(),
        pending: BTreeMap::new(),
        next_query: 0,
        published: BTreeMap::new(),
        stamps: BTreeMap::new(),
        held_back: BTreeSet::new(),
        next_stamp: 0,
        next_build: 0,
        coalesce_until: None,
        coalesce_start: None,
        exe,
        folders: Vec::new(),
        folder_diagnostics: BTreeMap::new(),
        watch_files: false,
        progress: false,
        next_request: 0,
        discovered_at: Instant::now(),
        exports: BTreeMap::new(),
        max_sessions: MAX_SESSIONS,
        session_idle: SESSION_IDLE,
        insert_replace: false,
        snippets: false,
    };
    server.serve(inbox)
}

/// Whether a `workspace/didChangeWatchedFiles` names an export.
fn names_an_export(params: &Json) -> bool {
    let changes = params.get("changes").map(Json::arr).unwrap_or(&[]);
    changes.iter().filter_map(|c| c.get("uri")?.str()).any(|uri| uri.ends_with(&format!("/{}", crate::task::export::FILE)))
}

/// The files of an sbt build a `workspace/didChangeWatchedFiles` or a `didSave` names: `.sbt`
/// files, `project/build.properties` and the sources under `project/`, none under a `target`.
fn changed_build_files(params: &Json) -> Vec<PathBuf> {
    let mut uris: Vec<&str> = params.get("changes").map(Json::arr).unwrap_or(&[]).iter().filter_map(|c| c.get("uri")?.str()).collect();
    uris.extend(params.at(&["textDocument", "uri"]).and_then(Json::str));
    uris.into_iter()
        .filter(|uri| !uri.contains("/target/"))
        .filter(|uri| uri.ends_with(".sbt") || uri.ends_with("/project/build.properties") || (uri.contains("/project/") && uri.ends_with(".scala")))
        .filter_map(uri::to_path)
        .map(|p| canonical(&p))
        .collect()
}

fn send_to_client(message: &Json) {
    let mut out = std::io::stdout().lock();
    rpc::write_message(&mut out, &message.to_text());
}

fn respond(id: &Json, result: Json) {
    send_to_client(&obj([("jsonrpc", "2.0".into()), ("id", id.clone()), ("result", result)]));
}

fn respond_error(id: &Json, code: i64, message: &str) {
    let error = obj([("code", Json::Num(code as f64)), ("message", message.into())]);
    send_to_client(&obj([("jsonrpc", "2.0".into()), ("id", id.clone()), ("error", error)]));
}

fn notify(method: &str, params: Json) {
    send_to_client(&obj([("jsonrpc", "2.0".into()), ("method", method.into()), ("params", params)]));
}

fn end_progress(token: &str) {
    notify("$/progress", obj([("token", token.into()), ("value", obj([("kind", "end".into())]))]));
}

/// How long the loop waits for a message: until the gathered edits are sent, a pending build is
/// old enough to show or a session's idle poll is due, at most 500 ms.
fn wake_in(now: Instant, deadlines: [Option<Instant>; 3]) -> Duration {
    let soonest = deadlines.into_iter().flatten().min();
    soonest.map_or(Duration::from_millis(500), |t| t.saturating_duration_since(now).min(Duration::from_millis(500)))
}

/// The text of a file on disk, a regular file's alone: a pipe in a document's place is never
/// opened, which would block the server.
fn read_text(path: &Path) -> Option<String> {
    std::fs::metadata(path).ok().filter(|m| m.is_file())?;
    std::fs::read_to_string(path).ok()
}

/// The file a request's `textDocument.uri` names, canonical.
fn document_path(params: &Json) -> Option<PathBuf> {
    let uri = params.at(&["textDocument", "uri"])?.str()?;
    Some(canonical(&uri::to_path(uri)?))
}

fn position(params: &Json) -> Option<(u32, u32)> {
    Some((params.at(&["position", "line"])?.uint()?, params.at(&["position", "character"])?.uint()?))
}

impl Server {
    fn serve(&mut self, inbox: Receiver<Event>) -> ! {
        let mut age_at = Instant::now() + AGE;
        loop {
            let timeout = wake_in(Instant::now(), [self.coalesce_until, self.progress_deadline(), self.poll_deadline()]);
            // The messages are allocated by the threads that read them and freed here.
            crate::alloc::settle();
            match inbox.recv_timeout(timeout) {
                Ok(Event::Client(Ok(body))) => self.message(&body),
                Ok(Event::Client(Err(msg))) => respond_error(&Json::Null, PARSE_ERROR, &msg),
                Ok(Event::ClientGone) => self.exit(),
                Ok(Event::Line(s, generation, line)) => {
                    if self.sessions.get(s).map_or(false, |x| x.generation == generation) {
                        self.answer(s, &line);
                    }
                }
                Ok(Event::Exited(s, generation)) => {
                    if self.sessions.get(s).map_or(false, |x| x.generation == generation) {
                        self.child_gone(s);
                    }
                }
                Ok(Event::Exported(f, b, ok, tail)) => self.exported(f, b, ok, &tail),
                Ok(Event::Generated(s, generation, result)) => self.generated(s, generation, result),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => self.exit(),
            }
            // By the clock and not by a pause: messages that keep coming put it off no longer.
            if Instant::now() >= age_at {
                crate::alloc::age();
                age_at = Instant::now() + AGE;
            }
            if self.coalesce_until.map_or(false, |t| Instant::now() >= t) {
                self.flush();
            }
            self.reconcile();
        }
    }

    fn exit(&mut self) -> ! {
        for s in &mut self.sessions {
            if let Some(p) = s.proc.take() {
                p.kill();
            }
        }
        for p in self.ending.drain(..) {
            p.kill();
        }
        self.kill_exports();
        std::process::exit(if self.shutdown { 0 } else { 1 });
    }

    /// Ends the exports under way, their tokens with them.
    fn kill_exports(&mut self) {
        for build in self.folders.iter_mut().flat_map(|f| f.builds.iter_mut()) {
            if let Some(running) = build.running.take() {
                running.kill();
            }
            if let Some(token) = build.progress.take() {
                end_progress(&token);
            }
        }
    }

    fn message(&mut self, body: &str) {
        let msg = match Json::parse(body) {
            Ok(m) => m,
            Err(e) => return respond_error(&Json::Null, PARSE_ERROR, &e),
        };
        let method = msg.get("method").and_then(Json::str).map(str::to_string);
        let id = msg.get("id").cloned();
        let params = msg.get("params").cloned().unwrap_or(Json::Null);
        match (method, id) {
            (Some(method), Some(id)) => self.request(id, &method, &params),
            (Some(method), None) => self.notification(&method, &params),
            // An answer to a request of ours (a registration, a progress token's creation): nothing to do.
            (None, Some(_)) if msg.get("result").is_some() || msg.get("error").is_some() => {}
            (None, id) => respond_error(&id.unwrap_or(Json::Null), INVALID_REQUEST, "a message without a method"),
        }
    }

    fn request(&mut self, id: Json, method: &str, params: &Json) {
        if self.shutdown {
            return respond_error(&id, INVALID_REQUEST, "the server is shut down");
        }
        if method == "initialize" {
            return self.initialize(&id, params);
        }
        if !self.initialized && method != "shutdown" {
            return respond_error(&id, SERVER_NOT_INITIALIZED, "initialize first");
        }
        match method {
            "shutdown" => {
                self.shutdown = true;
                for s in 0..self.sessions.len() {
                    if let Some(p) = self.sessions[s].proc.take() {
                        p.end();
                        self.ending.push(p);
                    }
                    let expected: Vec<Expect> = self.sessions[s].expect.drain(..).collect();
                    for e in expected {
                        if let Expect::Query(q, _) = e {
                            self.query_answered(q, s, Json::Null);
                        }
                    }
                    self.answer_deferred(s);
                    self.settle_progress(s);
                }
                self.kill_exports();
                self.coalesce_until = None;
                respond(&id, Json::Null);
            }
            "textDocument/definition" => self.at_position(id, method, params, "definition", ""),
            "textDocument/references" => {
                let declaration = params.at(&["context", "includeDeclaration"]).and_then(Json::bool).unwrap_or(false);
                self.at_position(id, method, params, "references", if declaration { " 1" } else { " 0" })
            }
            "textDocument/hover" => self.at_position(id, method, params, "hover", ""),
            "textDocument/signatureHelp" => self.signature_help(id, params),
            "textDocument/implementation" => self.at_position(id, method, params, "implementation", ""),
            "textDocument/prepareCallHierarchy" => self.at_position(id, method, params, "prepare-call", ""),
            "textDocument/completion" => self.complete(id, params),
            "completionItem/resolve" => self.resolve_item(id, params),
            "textDocument/documentSymbol" => match document_path(params) {
                Some(path) if self.library_document_changed(&path) => respond(&id, Json::Null),
                Some(path) => {
                    let command = format!("symbols {}\n", path.display());
                    self.ask(id, method, Some(&path), &command, true)
                }
                None => respond(&id, Json::Null),
            },
            "workspace/symbol" => {
                let query = params.get("query").and_then(Json::str).unwrap_or("").replace('\n', " ");
                self.ask(id, method, None, &format!("workspace-symbols {}\n", query), false)
            }
            "callHierarchy/incomingCalls" | "callHierarchy/outgoingCalls" => {
                let item = params.get("item");
                let path = item.and_then(|i| i.get("uri")).and_then(Json::str).and_then(uri::to_path).map(|p| canonical(&p));
                let start = item.and_then(|i| Some((i.at(&["selectionRange", "start", "line"])?.uint()?, i.at(&["selectionRange", "start", "character"])?.uint()?)));
                match (path, start) {
                    (Some(path), _) if self.library_document_changed(&path) => respond(&id, Json::Null),
                    (Some(path), Some((line, character))) => {
                        let verb = if method == "callHierarchy/incomingCalls" { "incoming" } else { "outgoing" };
                        let Some(offset) = self.offset(&path, line, character) else { return respond(&id, Json::Null) };
                        let command = format!("{} {} {}\n", verb, path.display(), offset);
                        self.ask(id, method, Some(&path), &command, false)
                    }
                    _ => respond(&id, Json::Null),
                }
            }
            _ => respond_error(&id, METHOD_NOT_FOUND, &format!("{} is not supported", method)),
        }
    }

    fn at_position(&mut self, id: Json, method: &str, params: &Json, verb: &str, extra: &str) {
        let (Some(path), Some((line, character))) = (document_path(params), position(params)) else {
            return respond(&id, Json::Null);
        };
        if self.library_document_changed(&path) {
            return respond(&id, Json::Null);
        }
        let Some(offset) = self.offset(&path, line, character) else { return respond(&id, Json::Null) };
        let command = format!("{} {} {}{}\n", verb, path.display(), offset, extra);
        self.ask(id, method, Some(&path), &command, false)
    }

    /// Whether a file no session owns (a library's source) is open with a text
    /// other than the one on disk, which the sessions' positions describe: its queries answer
    /// nothing.
    fn library_document_changed(&self, path: &Path) -> bool {
        match self.docs.get(path) {
            Some(doc) if !self.sessions.iter().any(|s| s.owns(path)) => read_text(path).map_or(true, |t| t != doc.text),
            _ => false,
        }
    }

    /// The byte offset of a position in the file as the client has it: the open document's
    /// text, or the file on disk.
    fn offset(&self, path: &Path, line: u32, character: u32) -> Option<u32> {
        let positions = self.positions();
        match self.docs.get(path) {
            Some(doc) => Some(crate::index::offset_of(&doc.text, line, character, positions)),
            None => {
                let text = read_text(path)?;
                Some(crate::index::offset_of(&text, line, character, positions))
            }
        }
    }

    fn positions(&self) -> crate::watch::Positions {
        if self.positions == "utf-8" {
            crate::watch::Positions::Utf8
        } else {
            crate::watch::Positions::Utf16
        }
    }

    /// Sends a query to the sessions that serve `file`, started when none runs (without a file,
    /// to every session not stopped; for a file no configuration holds, a library's document,
    /// to the session remembered as its producer, woken when parked, else to the running
    /// sessions that serve it, the first that maps it answering: those that read libraries for
    /// a jar's or a product's, every one for a std file's) and answers the request
    /// once they have. A file whose current text a session holds back answers nothing there;
    /// `one` asks a single session, for what every owner answers alike.
    fn ask(&mut self, id: Json, method: &str, file: Option<&Path>, command: &str, one: bool) {
        self.flush();
        let mut targets: Vec<usize> = match file {
            Some(f) => self.open_for(f),
            None => (0..self.sessions.len()).filter(|&s| !self.sessions[s].parked && !self.sessions[s].retired).collect(),
        };
        let mut document = None;
        if let (Some(f), true) = (file, targets.is_empty()) {
            match self.doc_sessions.get(f).copied().filter(|&s| !self.sessions[s].retired) {
                Some(s) => {
                    self.wake(s);
                    self.enforce_cap(&[s]);
                    targets = vec![s];
                }
                None => {
                    targets = (0..self.sessions.len()).filter(|&s| self.sessions[s].serves_document(f) && self.sessions[s].proc.is_some()).collect();
                    document = Some(f.to_path_buf());
                }
            }
        }
        for &s in &targets {
            self.ensure_running(s);
            self.sessions[s].last_used = Instant::now();
        }
        targets.retain(|&s| self.sessions[s].proc.is_some());
        if one {
            targets.truncate(1);
        }
        if targets.is_empty() {
            return respond(&id, Json::Null);
        }
        let q = self.next_query;
        self.next_query += 1;
        let mut waiting = 0;
        for &s in &targets {
            if self.query_waits(s) {
                hooks::trace("server", || format!("deferred {}", q));
                self.sessions[s].deferred.push((q, command.to_string()));
                waiting += 1;
            } else if self.sessions[s].send(command) {
                self.sessions[s].expect.push_back(Expect::Query(q, command.to_string()));
                waiting += 1;
            }
        }
        if waiting == 0 {
            return respond(&id, Json::Null);
        }
        let query = match method {
            "workspace/symbol" => command.trim_end().strip_prefix("workspace-symbols ").unwrap_or("").to_lowercase(),
            _ => String::new(),
        };
        let item = None;
        let follow = match command.split_once(' ').map(|(verb, _)| verb) {
            Some("references") => Some(("references", if command.trim_end().ends_with(" 1") { " 1" } else { " 0" })),
            Some("implementation") => Some(("implementation", "")),
            Some("incoming") => Some(("incoming", "")),
            _ => None,
        };
        let origin = file.map(Path::to_path_buf);
        self.pending.insert(q, Pending { id, method: method.to_string(), waiting, results: Vec::new(), query, item, revision: None, follow, asked: targets, document, origin });
    }

    /// The one session a completion or a signature help on `path` is asked of: a running one that
    /// owns the file, else for a library's document (a std file's, which a session answers as a
    /// program file of its own) the session navigation asks (`ask`): its producer, woken where
    /// parked, else the first running session that serves it. `None` for a library document
    /// open with another text (`library_document_changed`).
    fn one_for(&mut self, path: &Path) -> Option<usize> {
        let owners = self.open_for(path);
        if !owners.is_empty() {
            return owners.into_iter().find(|&s| self.sessions[s].proc.is_some());
        }
        if self.library_document_changed(path) {
            return None;
        }
        match self.doc_sessions.get(path).copied().filter(|&s| !self.sessions[s].retired) {
            Some(s) => {
                self.wake(s);
                self.enforce_cap(&[s]);
                Some(s).filter(|&s| self.sessions[s].proc.is_some())
            }
            None => (0..self.sessions.len()).find(|&s| self.sessions[s].serves_document(path) && self.sessions[s].proc.is_some()),
        }
    }

    /// `textDocument/completion`: asked of one session that owns the file (`one_for`), after the
    /// build of the document's text as the request found it (a query waits behind a build under
    /// way for the one that sends that text, `query_waits`); a file no session holds answers the
    /// empty list.
    fn complete(&mut self, id: Json, params: &Json) {
        let empty = || obj([("isIncomplete", false.into()), ("items", Json::Arr(Vec::new()))]);
        let (Some(path), Some((line, character))) = (document_path(params), position(params)) else { return respond(&id, empty()) };
        let Some(offset) = self.offset(&path, line, character) else { return respond(&id, empty()) };
        let version = self.docs.get(&path).map_or(0, |d| d.version);
        self.flush();
        let Some(s) = self.one_for(&path) else { return respond(&id, empty()) };
        self.sessions[s].last_used = Instant::now();
        let flags = self.insert_replace as u32 | (self.snippets as u32) << 1;
        let command = format!("complete {} {} {}\n", path.display(), offset, flags);
        self.query_one(id, "textDocument/completion", s, &command, None, Some((path, version)));
    }

    /// `textDocument/signatureHelp`: asked as a completion is, of one session that owns the
    /// file (`one_for`) after the build of the document's text as the request found it, answered
    /// `ContentModified` once that text is never typed; null where no session holds the file.
    fn signature_help(&mut self, id: Json, params: &Json) {
        let (Some(path), Some((line, character))) = (document_path(params), position(params)) else { return respond(&id, Json::Null) };
        let Some(offset) = self.offset(&path, line, character) else { return respond(&id, Json::Null) };
        let version = self.docs.get(&path).map_or(0, |d| d.version);
        self.flush();
        let Some(s) = self.one_for(&path) else { return respond(&id, Json::Null) };
        self.sessions[s].last_used = Instant::now();
        let command = format!("signature {} {}\n", path.display(), offset);
        self.query_one(id, "textDocument/signatureHelp", s, &command, None, Some((path, version)));
    }

    /// Sends one session a query whose answer answers the request.
    fn query_one(&mut self, id: Json, method: &str, s: usize, command: &str, item: Option<Json>, revision: Option<(PathBuf, i64)>) {
        let revision = revision.map(|(path, version)| Revision { stamp: self.stamps.get(&path).copied(), path, version });
        let q = self.next_query;
        if self.query_waits(s) {
            hooks::trace("server", || format!("deferred {}", q));
            self.sessions[s].deferred.push((q, command.to_string()));
        } else if self.sessions[s].send(command) {
            self.sessions[s].expect.push_back(Expect::Query(q, command.to_string()));
        } else {
            return respond(&id, Json::Null);
        }
        self.next_query += 1;
        let origin = revision.as_ref().map(|r| r.path.clone());
        self.pending.insert(q, Pending { id, method: method.to_string(), waiting: 1, results: Vec::new(), query: String::new(), item, revision, follow: None, asked: vec![s], document: None, origin });
    }

    /// Whether the query `q` was asked on a revision of a document that moved on since: a
    /// completion's, a resolve's or a signature help's, whose offset and answer are that
    /// revision's.
    fn obsolete(&self, q: u64) -> bool {
        let Some(r) = self.pending.get(&q).and_then(|p| p.revision.as_ref()) else { return false };
        // Every open, change and close of the document stamps it anew.
        self.stamps.get(&r.path) != r.stamp.as_ref()
    }

    /// Answers the query `q` `ContentModified`: its revision is never typed.
    fn modified(&mut self, q: u64) {
        if let Some(p) = self.pending.remove(&q) {
            respond_error(&p.id, CONTENT_MODIFIED, "the document changed");
        }
    }

    /// `completionItem/resolve`: an item of a name in scope is complete as it is; one that
    /// brings an import (`data`) is asked of the session that offered it, which answers the
    /// import's edit, unless the document or the program changed since.
    fn resolve_item(&mut self, id: Json, params: &Json) {
        let item = params.clone();
        let Some(data) = item.get("data").filter(|d| d.get("import").is_some()) else { return respond(&id, item) };
        let path = data.get("path").and_then(Json::str).map(PathBuf::from);
        let version = data.get("version").and_then(Json::num).map(|v| v as i64);
        let (Some(path), Some(version)) = (path, version) else { return respond(&id, item) };
        if self.docs.get(&path).map_or(true, |d| d.version != version) {
            return respond_error(&id, CONTENT_MODIFIED, "the document changed");
        }
        let offset = data.get("offset").and_then(Json::uint).unwrap_or(0);
        let generation = data.get("generation").and_then(Json::str).unwrap_or("").to_string();
        let name = data.get("import").and_then(Json::str).unwrap_or("").to_string();
        self.flush();
        let Some(s) = self.one_for(&path) else { return respond(&id, item) };
        if self.sessions[s].dirty.contains(&path) {
            return respond_error(&id, CONTENT_MODIFIED, "the document changed");
        }
        let command = format!("complete-resolve {} {} {} {}\n", path.display(), offset, generation, name);
        self.query_one(id, "completionItem/resolve", s, &command, Some(item), Some((path, version)));
    }

    fn notification(&mut self, method: &str, params: &Json) {
        if self.shutdown && method != "exit" {
            return;
        }
        match method {
            "initialized" => {
                self.register_watchers();
                self.start_sessions()
            }
            "exit" => self.exit(),
            "textDocument/didOpen" => {
                let Some(path) = document_path(params) else { return };
                let text = params.at(&["textDocument", "text"]).and_then(Json::str).unwrap_or("").to_string();
                let version = params.at(&["textDocument", "version"]).and_then(Json::num).unwrap_or(0.0) as i64;
                let uri = params.at(&["textDocument", "uri"]).and_then(Json::str).unwrap_or("").to_string();
                self.docs.insert(path.clone(), Doc { text, version, uri });
                self.stamp(&path);
                self.open_for(&path);
                self.touch(&path);
            }
            "textDocument/didChange" => {
                let Some(path) = document_path(params) else { return };
                // Full synchronisation: the last change holds the whole text.
                let Some(text) = params.get("contentChanges").map(Json::arr).and_then(|c| c.last()).and_then(|c| c.get("text")).and_then(Json::str) else { return };
                let version = params.at(&["textDocument", "version"]).and_then(Json::num).unwrap_or(0.0) as i64;
                let uri = match self.docs.get(&path) {
                    Some(doc) => doc.uri.clone(),
                    None => params.at(&["textDocument", "uri"]).and_then(Json::str).unwrap_or("").to_string(),
                };
                self.docs.insert(path.clone(), Doc { text: text.to_string(), version, uri });
                self.stamp(&path);
                self.open_for(&path);
                self.touch(&path);
            }
            "textDocument/didClose" => {
                let Some(path) = document_path(params) else { return };
                // A document closed after its file was deleted: its path no longer canonicalises, and the
                // client's URI may spell it otherwise than the canonical form it was opened under (a link,
                // a Windows short name), so the document is found by the URI it was opened with.
                let key = if self.docs.contains_key(&path) {
                    Some(path)
                } else {
                    let uri = params.at(&["textDocument", "uri"]).and_then(|u| u.str()).unwrap_or("");
                    self.docs.iter().find(|(_, d)| d.uri == uri).map(|(p, _)| p.clone())
                };
                if let Some(path) = key {
                    if self.docs.remove(&path).is_some() {
                        self.stamp(&path);
                        self.touch(&path);
                    }
                }
            }
            "textDocument/didSave" | "workspace/didChangeWatchedFiles" => {
                self.flush();
                if names_an_export(params) {
                    self.rediscover();
                }
                self.build_files_changed(&changed_build_files(params));
                self.reload_exports();
                self.restart_moved_inputs();
                for s in 0..self.sessions.len() {
                    self.sessions[s].poll = false;
                    self.build_all(s);
                }
            }
            _ => {}
        }
    }

    fn initialize(&mut self, id: &Json, params: &Json) {
        let offered = params.at(&["capabilities", "general", "positionEncodings"]).map(Json::arr).unwrap_or(&[]);
        self.positions = if offered.iter().any(|e| e.str() == Some("utf-8")) { "utf-8" } else { "utf-16" };
        let mut roots: Vec<PathBuf> = params.get("workspaceFolders").map(Json::arr).unwrap_or(&[]).iter().filter_map(|f| f.get("uri")?.str()).filter_map(uri::to_path).collect();
        if roots.is_empty() {
            let root = params.get("rootUri").and_then(Json::str).and_then(uri::to_path).or_else(|| params.get("rootPath").and_then(Json::str).map(PathBuf::from));
            roots.extend(root);
        }
        let options = params.get("initializationOptions");
        if let Some(n) = options.and_then(|o| o.get("maxSessions")).and_then(Json::uint) {
            self.max_sessions = (n as usize).max(1);
        }
        if let Some(n) = options.and_then(|o| o.get("sessionIdleSeconds")).and_then(Json::num).filter(|n| *n > 0.0) {
            self.session_idle = Duration::from_secs_f64(n);
        }
        for given in roots {
            let root = canonical(&given);
            let builds = export::build_roots(&root).into_iter().map(Build::new).collect();
            self.folders.push(Folder { root, given, exports: Vec::new(), builds });
        }
        self.rediscover();
        self.discovered_at = Instant::now();
        self.initialized = true;
        let capabilities = obj([
            ("positionEncoding", self.positions.into()),
            ("textDocumentSync", obj([("openClose", true.into()), ("change", 1u32.into()), ("save", true.into())])),
            ("definitionProvider", true.into()),
            ("referencesProvider", true.into()),
            ("hoverProvider", true.into()),
            ("signatureHelpProvider", obj([("triggerCharacters", Json::Arr(vec!["(".into(), ",".into()])), ("retriggerCharacters", Json::Arr(vec![",".into()]))])),
            ("documentSymbolProvider", true.into()),
            ("workspaceSymbolProvider", true.into()),
            ("implementationProvider", true.into()),
            ("callHierarchyProvider", true.into()),
            ("completionProvider", obj([("triggerCharacters", Json::Arr(vec![".".into()])), ("resolveProvider", true.into())])),
        ]);
        self.insert_replace = params.at(&["capabilities", "textDocument", "completion", "completionItem", "insertReplaceSupport"]).and_then(Json::bool) == Some(true);
        self.snippets = params.at(&["capabilities", "textDocument", "completion", "completionItem", "snippetSupport"]).and_then(Json::bool) == Some(true);
        self.watch_files = params.at(&["capabilities", "workspace", "didChangeWatchedFiles", "dynamicRegistration"]).and_then(Json::bool) == Some(true);
        self.progress = params.at(&["capabilities", "window", "workDoneProgress"]).and_then(Json::bool) == Some(true);
        respond(id, obj([("capabilities", capabilities), ("serverInfo", obj([("name", "teq".into()), ("version", env!("CARGO_PKG_VERSION").into())]))]));
        // What the exports said while the folders were read is published after the answer.
        let files: Vec<PathBuf> = self.folder_diagnostics.keys().cloned().collect();
        self.publish_changed(&files);
    }

    /// A request to the client, whose answer the loop ignores.
    fn request_client(&mut self, method: &str, params: Json) {
        self.next_request += 1;
        let id = format!("teq-request-{}", self.next_request);
        send_to_client(&obj([("jsonrpc", "2.0".into()), ("id", id.into()), ("method", method.into()), ("params", params)]));
    }

    fn register_watchers(&mut self) {
        if !self.watch_files {
            return;
        }
        let globs = ["**/*.scala", "**/teq.lock", "**/*.sbt", "**/project/build.properties"];
        let watchers = Json::Arr(globs.iter().map(|g| obj([("globPattern", (*g).into())])).collect());
        let registration = obj([
            ("id", "teq-watched-files".into()),
            ("method", "workspace/didChangeWatchedFiles".into()),
            ("registerOptions", obj([("watchers", watchers)])),
        ]);
        self.request_client("client/registerCapability", obj([("registrations", Json::Arr(vec![registration]))]));
    }

    /// Tells the client the session's builds are under way: a token begun with the project's name.
    fn begin_progress(&mut self, s: usize) {
        if self.sessions[s].progress.is_some() || self.sessions[s].proc.is_none() {
            return;
        }
        let message = format!("checking {}", self.sessions[s].label());
        self.sessions[s].progress = self.begin_token(&message);
    }

    /// A progress token created and begun with `message`, titled `teq`; none for a client that
    /// did not offer `window.workDoneProgress`.
    fn begin_token(&mut self, message: &str) -> Option<String> {
        if !self.progress {
            return None;
        }
        self.next_request += 1;
        let token = format!("teq-progress-{}", self.next_request);
        self.request_client("window/workDoneProgress/create", obj([("token", token.as_str().into())]));
        let value = obj([("kind", "begin".into()), ("title", "teq".into()), ("message", message.into()), ("cancellable", false.into())]);
        notify("$/progress", obj([("token", token.as_str().into()), ("value", value)]));
        Some(token)
    }

    /// Ends the session's token once no build is pending or waits to be sent, or the child is gone.
    fn settle_progress(&mut self, s: usize) {
        let session = &mut self.sessions[s];
        if session.proc.is_some() && (session.oldest_build().is_some() || session.wants_build()) {
            return;
        }
        if let Some(token) = session.progress.take() {
            end_progress(&token);
        }
    }

    /// When the first session's idle poll is due (`reconcile`).
    fn poll_deadline(&self) -> Option<Instant> {
        if !self.initialized || self.shutdown {
            return None;
        }
        self.sessions.iter().filter_map(Session::poll_due).min()
    }

    /// When the oldest pending build of a session without a token turns `PROGRESS_AFTER` old.
    fn progress_deadline(&self) -> Option<Instant> {
        if !self.progress {
            return None;
        }
        self.sessions.iter().filter(|x| x.progress.is_none() && x.proc.is_some()).filter_map(|x| x.oldest_build()).map(|t| t + PROGRESS_AFTER).min()
    }

    /// The bare roots' children, and the export of each build that has none. The sessions of an
    /// export start when a file of theirs is opened or asked about.
    fn start_sessions(&mut self) {
        for s in 0..self.sessions.len() {
            if !self.sessions[s].parked {
                self.ensure_running(s);
            }
        }
        for f in 0..self.folders.len() {
            for b in 0..self.folders[f].builds.len() {
                if !self.folders[f].builds[b].exported() {
                    self.start_export(f, b);
                }
            }
        }
    }

    /// Runs the build's export: `sbt teqExportAll`, this binary as its teq, shown as work in
    /// progress until it ends; whether a run started. One export runs at a time: a run asked
    /// for during one follows it.
    fn start_export(&mut self, f: usize, b: usize) -> bool {
        if self.shutdown {
            return false;
        }
        if self.folders.iter().flat_map(|x| x.builds.iter()).any(|x| x.running.is_some()) {
            self.folders[f].builds[b].again = true;
            return false;
        }
        let root = self.folders[f].builds[b].root.clone();
        if let Some(why) = export::refusal(&root) {
            self.export_failed(f, b, &why);
            return false;
        }
        match export::start(f, b, &root, &self.exe, self.events.clone()) {
            Ok(running) => {
                let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| root.display().to_string());
                let token = self.begin_token(&format!("exporting the sbt build of {}", name));
                let build = &mut self.folders[f].builds[b];
                build.running = Some(running);
                build.progress = token;
                true
            }
            Err(why) => {
                self.export_failed(f, b, &why);
                false
            }
        }
    }

    /// The runs waiting, in order, until one starts, while none is under way: those whose
    /// changes have rested (`Build::due`) and whose build has no export yet; a refused one (sbt
    /// 1, no sbt) ends at once and must not keep the others waiting.
    fn start_due_export(&mut self) {
        if self.folders.iter().flat_map(|x| x.builds.iter()).any(|x| x.running.is_some()) {
            return;
        }
        loop {
            let due = self.folders.iter().enumerate().flat_map(|(f, x)| x.builds.iter().enumerate().map(move |(b, y)| (f, b, y.due()))).find(|(_, _, due)| *due);
            let Some((f, b, _)) = due else { return };
            let build = &mut self.folders[f].builds[b];
            build.again = false;
            build.changed_at = None;
            if !build.exported() && self.start_export(f, b) {
                return;
            }
        }
    }

    /// The build's export ended: on success the export is read and its sessions open on demand;
    /// a run that wrote no `teq.lock` (a client of a running server, which knows no
    /// `teqExportAll`) is a failure like one that ended badly, told on the build file. Then the
    /// next run waiting.
    fn exported(&mut self, f: usize, b: usize, ok: bool, tail: &str) {
        if self.shutdown {
            return;
        }
        let Some(build) = self.folders.get_mut(f).and_then(|x| x.builds.get_mut(b)) else { return };
        build.running = None;
        if let Some(token) = build.progress.take() {
            end_progress(&token);
        }
        let root = build.root.clone();
        if ok && build.exported() {
            build.failure = None;
            let anchor = export::anchor(&root);
            self.folder_diagnostics.remove(&anchor);
            self.publish_changed(&[anchor]);
            self.rediscover();
        } else {
            let why = if !ok {
                if tail.is_empty() { "the sbt export failed".to_string() } else { format!("the sbt export failed: {}", tail) }
            } else if tail.contains("entering thin client") {
                "sbt ran as a client of a running server, which has no sbt-teq: the automatic export needs sbt in the foreground (no --client in .sbtopts); or export the build with sbt teqExportAll".to_string()
            } else {
                format!("sbt ran but wrote no {} at the build's root or under its target/teq/; export it with sbt teqExportAll to see why", crate::task::export::FILE)
            };
            self.export_failed(f, b, &why);
        }
        self.start_due_export();
    }

    /// A failed run: the reason on the build file.
    fn export_failed(&mut self, f: usize, b: usize, why: &str) {
        let build = &mut self.folders[f].builds[b];
        build.failure = Some(why.to_string());
        let anchor = export::anchor(&build.root);
        self.folder_diagnostics.insert(anchor.clone(), vec![diagnostic_at_start(why)]);
        self.publish_changed(&[anchor]);
    }

    /// Build files changed: a build without an export runs it again once the changes rest
    /// (`reconcile`, `start_due_export`), and an export of the build is checked against them.
    fn build_files_changed(&mut self, files: &[PathBuf]) {
        if files.is_empty() {
            return;
        }
        for build in self.folders.iter_mut().flat_map(|f| f.builds.iter_mut()) {
            if !build.exported() && files.iter().any(|p| p.starts_with(&build.root)) {
                build.changed_at = Some(Instant::now());
                build.again = true;
            }
        }
        let touched: Vec<PathBuf> = self.exports.keys().filter(|file| files.iter().any(|p| p.starts_with(crate::task::export::root_of(file)))).cloned().collect();
        for file in touched {
            self.export_diagnostics(&file);
        }
    }

    /// The folders searched again for exports written or removed since: the sessions of an
    /// export no folder finds any more retired, a bare root giving way to its first export and
    /// returning when it is gone (never while the folder stands in an sbt build, whose export
    /// the server runs); the exports read again where they changed; then every session not
    /// running, not failed and not stopped (one that had nothing to type yet) tried again.
    fn rediscover(&mut self) {
        self.discovered_at = Instant::now();
        for f in 0..self.folders.len() {
            let found = self.folders[f].find_exports();
            let root = self.folders[f].root.clone();
            let bare = found.is_empty() && self.folders[f].builds.is_empty();
            self.folders[f].exports = found;
            for s in 0..self.sessions.len() {
                let session = &self.sessions[s];
                if !session.retired && session.export.is_none() && session.folder == root && !bare {
                    self.retire(s);
                }
            }
            if bare && !self.sessions.iter().any(|x| !x.retired && x.export.is_none() && x.folder == root) {
                self.sessions.push(Session::new(root.clone(), None, None, Ok(workspace::bare(&root))));
            }
        }
        // An export written by other hands than the fallback's (the developer ran sbt
        // teqExportAll) ends the failure the fallback published.
        let mut cleared = Vec::new();
        for build in self.folders.iter_mut().flat_map(|f| f.builds.iter_mut()) {
            if build.failure.is_some() && build.running.is_none() && build.exported() {
                build.failure = None;
                cleared.push(export::anchor(&build.root));
            }
        }
        for anchor in cleared {
            self.folder_diagnostics.remove(&anchor);
            self.publish_changed(&[anchor]);
        }
        let found: BTreeSet<PathBuf> = self.folders.iter().flat_map(|f| f.exports.iter().cloned()).collect();
        for s in 0..self.sessions.len() {
            if self.sessions[s].export.as_ref().is_some_and(|e| !found.contains(e)) && !self.sessions[s].retired {
                self.retire(s);
            }
        }
        for gone in self.exports.keys().filter(|e| !found.contains(*e)).cloned().collect::<Vec<_>>() {
            // A build whose export is gone (`sbt clean` under its target/teq/) gets one export
            // again once the change has rested, as a build without one does at the start; a
            // failed run waits for a change of the build's files, as at the start.
            let root = crate::task::export::root_of(&gone);
            for build in self.folders.iter_mut().flat_map(|f| f.builds.iter_mut()) {
                if build.root == root && build.running.is_none() && !build.exported() {
                    build.again = true;
                    build.changed_at = Some(Instant::now());
                }
            }
            self.exports.remove(&gone);
            if self.folder_diagnostics.remove(&gone).is_some() {
                self.publish_changed(&[gone]);
            }
        }
        self.reload_exports();
        if !self.initialized {
            return;
        }
        self.restart_moved_inputs();
        for s in 0..self.sessions.len() {
            let session = &self.sessions[s];
            if session.proc.is_none() && session.failure.is_none() && !session.parked && !session.retired {
                self.ensure_running(s);
            }
        }
    }

    /// Reads each export the folders find again when it changed since it was read: the sessions
    /// whose configuration it removed retire, those whose description it changed start anew (a
    /// running one at once), and why it does not read goes on it, the sessions left as they were.
    fn reload_exports(&mut self) {
        let files: BTreeSet<PathBuf> = self.folders.iter().flat_map(|f| f.exports.iter().cloned()).collect();
        let mut read_any = false;
        for file in files {
            let now = workspace::mtime(&file);
            if self.exports.get(&file).is_some_and(|l| l.mtime == now) {
                continue;
            }
            let read = Export::read(&file).map(std::sync::Arc::new);
            read_any = true;
            self.exports.insert(file.clone(), Loaded { export: read.clone(), mtime: now });
            self.export_diagnostics(&file);
            let Ok(export) = read else { continue };
            for s in 0..self.sessions.len() {
                if self.sessions[s].retired || self.sessions[s].export.as_ref() != Some(&file) {
                    continue;
                }
                let key = self.sessions[s].key.clone().expect("a session of an export has a configuration");
                if export.configuration(&key).is_none() {
                    self.retire(s);
                    continue;
                }
                let description = workspace::description(&export, &key);
                if description == self.sessions[s].description {
                    continue;
                }
                self.replace_session(s, description);
            }
        }
        // An open document a configuration the export gained holds is served by it.
        if read_any && self.initialized {
            let open: Vec<PathBuf> = self.docs.keys().cloned().collect();
            for doc in open {
                if !self.sessions.iter().any(|x| !x.retired && x.owns(&doc)) {
                    self.open_for(&doc);
                    self.touch(&doc);
                }
            }
        }
    }

    /// The export's own diagnostics: why it does not read, on it; the build definition files
    /// changed since it was written, a warning on the build's file, with the note on those that
    /// differ by line ends alone.
    fn export_diagnostics(&mut self, file: &Path) {
        let Some(loaded) = self.exports.get(file) else { return };
        let root = crate::task::export::root_of(file);
        let anchor = if export::is_sbt_build(&root) { export::anchor(&root) } else { file.to_path_buf() };
        let (on_file, stale) = match &loaded.export {
            Err(e) => (Some(diagnostic_at_start(e)), Default::default()),
            Ok(export) => (None, export.stale_inputs()),
        };
        match on_file {
            Some(d) => self.folder_diagnostics.insert(file.to_path_buf(), vec![d]),
            None => self.folder_diagnostics.remove(file),
        };
        if !stale.is_empty() {
            let command = if stale.line_ends_only() { "" } else { "; run sbt teqExportAll" };
            let mut message = format!("{} was written before {}{}", crate::task::export::FILE, stale.changes.join(", "), command);
            if let Some(note) = stale.line_end_note() {
                message = format!("{}\n{}", message, note);
            }
            let warning = obj([("range", zero_range()), ("severity", 2u32.into()), ("source", "teq".into()), ("message", message.into())]);
            self.folder_diagnostics.insert(anchor.clone(), vec![warning]);
        } else if self.folder_diagnostics.get(&anchor).is_some_and(|ds| ds.iter().all(|d| d.get("severity").and_then(Json::uint) == Some(2))) {
            self.folder_diagnostics.remove(&anchor);
        }
        self.publish_changed(&[file.to_path_buf(), anchor]);
    }

    /// The running sessions whose declared inputs that exist are no longer those they started
    /// with, started anew.
    fn restart_moved_inputs(&mut self) {
        for s in 0..self.sessions.len() {
            let session = &self.sessions[s];
            let Ok(description) = &session.description else { continue };
            if session.proc.is_none() || session.retired {
                continue;
            }
            let existing: Vec<PathBuf> = description.inputs.iter().filter(|p| p.exists()).cloned().collect();
            if existing != session.started_inputs {
                let description = Ok(description.clone());
                self.replace_session(s, description);
            }
        }
    }

    /// A session's description changed: a new session in its place, its diagnostics kept until
    /// the new one's first build, started at once unless the old one was stopped.
    fn replace_session(&mut self, s: usize, description: Result<Description, String>) {
        let folder = self.sessions[s].folder.clone();
        let (export, key) = (self.sessions[s].export.clone(), self.sessions[s].key.clone());
        let mut old = std::mem::replace(&mut self.sessions[s], Session::new(folder, export, key, description));
        if let Some(token) = old.progress.take() {
            end_progress(&token);
        }
        let session = &mut self.sessions[s];
        // The old child's exit and lines then arrive under a generation that is not the session's.
        session.generation = old.generation + 1;
        session.diagnostics = std::mem::take(&mut old.diagnostics);
        session.parked = old.parked;
        if let Some(p) = old.proc.take() {
            p.kill();
        }
        session.typed = std::mem::take(&mut old.typed);
        for e in old.expect.drain(..) {
            if let Expect::Query(q, _) = e {
                self.query_answered(q, s, Json::Null);
            }
        }
        for (q, _) in old.deferred.drain(..) {
            self.query_answered(q, s, Json::Null);
        }
        self.ensure_running(s);
        // With nothing to type, no build would replace the last one's diagnostics.
        if self.sessions[s].proc.is_none() && self.sessions[s].failure.is_none() {
            let files: Vec<PathBuf> = std::mem::take(&mut self.sessions[s].diagnostics).into_keys().collect();
            self.publish_changed(&files);
        }
    }

    /// The sessions that serve a file: the running ones whose closure holds it, else the stopped
    /// one of them used last, started again, else one started for each configuration of an export
    /// whose own source roots hold it (no more than the cap); a session of the same project whose
    /// inputs a started one's hold is ended (its test configuration takes over from its compile
    /// configuration), and past the cap the sessions used longest ago are stopped.
    fn open_for(&mut self, file: &Path) -> Vec<usize> {
        let owning: Vec<usize> = (0..self.sessions.len()).filter(|&s| !self.sessions[s].retired && self.sessions[s].owns(file)).collect();
        let running: Vec<usize> = owning.iter().copied().filter(|&s| self.sessions[s].proc.is_some()).collect();
        if !running.is_empty() {
            for &s in &running {
                self.sessions[s].last_used = Instant::now();
            }
            return running;
        }
        // Of the owners that do not run, the one used last starts again.
        if let Some(&s) = owning.iter().max_by_key(|&&s| self.sessions[s].last_used) {
            self.wake(s);
            self.enforce_cap(&[s]);
            return vec![s];
        }
        let mut started = Vec::new();
        let exports: Vec<(PathBuf, std::sync::Arc<Export>)> = self.exports.iter().filter_map(|(f, l)| Some((f.clone(), l.export.as_ref().ok()?.clone()))).collect();
        'exports: for (file_of, export) in exports {
            for key in export.owners(file) {
                if started.len() >= self.max_sessions {
                    break 'exports;
                }
                let existing = (0..self.sessions.len()).find(|&s| !self.sessions[s].retired && self.sessions[s].export.as_ref() == Some(&file_of) && self.sessions[s].key.as_ref() == Some(&key));
                let s = match existing {
                    Some(s) => s,
                    None => {
                        let description = workspace::description(&export, &key);
                        self.sessions.push(Session::new(export.root.clone(), Some(file_of.clone()), Some(key.clone()), description));
                        self.sessions.len() - 1
                    }
                };
                let holds: BTreeSet<PathBuf> = self.sessions[s].description.as_ref().map(|d| d.inputs.iter().cloned().collect()).unwrap_or_default();
                for other in 0..self.sessions.len() {
                    let o = &self.sessions[other];
                    let inside = other != s
                        && o.export.as_ref() == Some(&file_of)
                        && o.key.as_ref().is_some_and(|k| k.project == key.project)
                        && o.description.as_ref().is_ok_and(|d| !d.inputs.is_empty() && d.inputs.iter().all(|i| holds.contains(i)));
                    if inside && !o.retired {
                        self.retire(other);
                    }
                }
                started.push(s);
            }
        }
        started.retain(|&s| !self.sessions[s].retired);
        for &s in &started {
            self.wake(s);
        }
        self.enforce_cap(&started);
        started
    }

    /// A session wanted again: started unless it runs.
    fn wake(&mut self, s: usize) {
        self.sessions[s].parked = false;
        self.sessions[s].last_used = Instant::now();
        if self.initialized {
            self.ensure_running(s);
        }
    }

    /// Stops the sessions used longest ago, idle ones first, until no more than the cap run;
    /// those of `keep` stay.
    fn enforce_cap(&mut self, keep: &[usize]) {
        loop {
            let running: Vec<usize> = (0..self.sessions.len()).filter(|&s| self.sessions[s].proc.is_some()).collect();
            if running.len() <= self.max_sessions {
                return;
            }
            let victim = running.into_iter().filter(|s| !keep.contains(s)).min_by_key(|&s| (self.sessions[s].busy(), self.sessions[s].last_used));
            match victim {
                Some(s) => self.park(s),
                None => return,
            }
        }
    }

    /// A session stopped for being unused or past the cap: its child ended and its queries
    /// answered; its diagnostics stay, those of its last build, until it runs again.
    fn park(&mut self, s: usize) {
        let session = &mut self.sessions[s];
        session.parked = true;
        session.generation += 1;
        if let Some(p) = session.proc.take() {
            p.kill();
        }
        let expected: Vec<Expect> = session.expect.drain(..).collect();
        for e in expected {
            if let Expect::Query(q, _) = e {
                self.query_answered(q, s, Json::Null);
            }
        }
        self.answer_deferred(s);
        self.settle_progress(s);
    }

    /// An export gone: the child ended, its queries answered, its diagnostics withdrawn.
    fn retire(&mut self, s: usize) {
        let session = &mut self.sessions[s];
        session.retired = true;
        session.roots.clear();
        session.dirty.clear();
        // The child's exit then arrives under a generation that is not the session's.
        session.generation += 1;
        if let Some(p) = session.proc.take() {
            p.kill();
        }
        let expected: Vec<Expect> = session.expect.drain(..).collect();
        for e in expected {
            if let Expect::Query(q, _) = e {
                self.query_answered(q, s, Json::Null);
            }
        }
        self.answer_deferred(s);
        let files: Vec<PathBuf> = std::mem::take(&mut self.sessions[s].diagnostics).into_keys().collect();
        self.publish_changed(&files);
        self.settle_progress(s);
    }

    /// Starts the session's child when it is not running: its first build, of every file, waits
    /// for the texts of its open documents (`--wait-for-build`), so that it types them.
    fn ensure_running(&mut self, s: usize) {
        if self.shutdown || self.sessions[s].proc.is_some() || self.sessions[s].retired || self.sessions[s].parked {
            return;
        }
        let mut description = match &self.sessions[s].description {
            Ok(d) => d.clone(),
            Err(msg) => {
                let msg = msg.clone();
                self.fail(s, &msg);
                return;
            }
        };
        if let Err(msg) = self.run_generators(s) {
            self.fail(s, &msg);
            return;
        }
        // A description that declares nothing (sbt's aggregate project) has nothing to type. An
        // input that does not exist (generated sources not yet generated) is left out rather
        // than failing every build, and a project with none left waits for them.
        if description.inputs.is_empty() {
            return;
        }
        description.inputs.retain(|p| p.exists());
        if description.inputs.is_empty() {
            return;
        }
        // A build's session starts as `teq @<file>`, its class path past Windows' cap on the
        // command line: a file of this server's under the build's `target/teq`, which no other
        // server's session shares. A bare root's has no class path and keeps the command line.
        let args = description.args(self.positions);
        let args_file = self.sessions[s].key.as_ref().map(|k| description.root.join(format!("target/teq/{}/{}/lsp-{}.args", k.project, k.configuration, std::process::id())));
        if let Some(file) = &args_file {
            if let Err(msg) = crate::argfile::write(file, &args) {
                self.fail(s, &msg);
                return;
            }
        }
        let session = &mut self.sessions[s];
        session.started_inputs = description.inputs.clone();
        session.generation += 1;
        let generation = session.generation;
        let mut command = Command::new(&self.exe);
        match &args_file {
            Some(file) => command.arg(crate::argfile::at(file)),
            None => command.args(&args),
        };
        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        // Windows starts no process in a directory past 258 characters (os error 267), where the
        // session, whose paths are all absolute, runs in the server's.
        if !cfg!(windows) || description.root.to_string_lossy().encode_utf16().count() <= 258 {
            command.current_dir(&description.root);
        }
        let spawned = command.spawn();
        let mut child = match spawned {
            Ok(c) => c,
            Err(e) => {
                if let Some(file) = &args_file {
                    let _ = std::fs::remove_file(file);
                }
                let msg = format!("cannot start teq: {}", e);
                self.fail(s, &msg);
                return;
            }
        };
        let stdout = child.stdout.take().expect("piped");
        let stderr = child.stderr.take().expect("piped");
        let stdin = child.stdin.take().expect("piped");
        let events = self.events.clone();
        // The exit is told once the child's stderr is read to its end, so that a session that
        // stops at once (a refusal of its arguments) ends with its message; bounded, since a
        // child's child could hold the pipe open.
        let (stderr_read, stderr_done) = std::sync::mpsc::channel::<()>();
        crate::alloc::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if events.send(Event::Line(s, generation, line)).is_err() {
                    return;
                }
            }
            let _ = stderr_done.recv_timeout(Duration::from_secs(5));
            let _ = events.send(Event::Exited(s, generation));
        });
        let tail = session.stderr.clone();
        tail.lock().unwrap_or_else(|e| e.into_inner()).clear();
        crate::alloc::spawn(move || {
            let _done = stderr_read;
            let mut stderr = stderr;
            let mut buf = [0u8; 4096];
            while let Ok(n) = stderr.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let mut t = tail.lock().unwrap_or_else(|e| e.into_inner());
                t.push_str(&String::from_utf8_lossy(&buf[..n]));
                if t.len() > 8192 {
                    let cut = t.len() - 4096;
                    let cut = (cut..t.len()).find(|&i| t.is_char_boundary(i)).unwrap_or(t.len());
                    t.drain(..cut);
                }
            }
        });
        session.proc = Some(Proc::start(child, stdin, args_file));
        session.expect.clear();
        session.failure = None;
        session.last_build = Instant::now();
        session.sent.clear();
        session.carry.clear();
        session.carry_plain = false;
        let overlays: Vec<PathBuf> = self.docs.keys().filter(|p| self.sessions[s].owns(p)).cloned().collect();
        let session = &mut self.sessions[s];
        session.dirty.extend(overlays);
        session.plain_wanted = true;
        session.poll = false;
        self.send_build(s);
        self.begin_progress(s);
    }

    /// The generators of a session's closure, as `teq` runs them before a build: what
    /// they generate is among the session's sources.
    fn run_generators(&self, s: usize) -> Result<(), String> {
        let session = &self.sessions[s];
        let (Some(file), Some(key)) = (&session.export, &session.key) else { return Ok(()) };
        let Some(Ok(export)) = self.exports.get(file).map(|l| &l.export) else { return Ok(()) };
        let env = std::env::vars().filter(|(k, _)| k == "SOURCE_VERSION").collect();
        crate::task::generators::run(export, &export.closure(key), export.path(&export.classes(key)), &env).map(|_| ())
    }

    /// A session that cannot run: the reason on its anchor, what its last build published
    /// withdrawn.
    fn fail(&mut self, s: usize, msg: &str) {
        let session = &mut self.sessions[s];
        session.failure = Some(msg.to_string());
        let anchor = session.anchor();
        let diagnostic = diagnostic_at_start(msg);
        let old = std::mem::replace(&mut session.diagnostics, BTreeMap::from([(anchor.clone(), vec![diagnostic])]));
        let mut files: Vec<PathBuf> = old.into_keys().collect();
        files.push(anchor);
        files.sort();
        files.dedup();
        self.publish_changed(&files);
    }

    fn child_gone(&mut self, s: usize) {
        let session = &mut self.sessions[s];
        if let Some(p) = session.proc.take() {
            p.kill();
        }
        let expected: Vec<Expect> = session.expect.drain(..).collect();
        for e in expected {
            if let Expect::Query(q, _) = e {
                self.query_answered(q, s, Json::Null);
            }
        }
        self.answer_deferred(s);
        self.settle_progress(s);
        if self.shutdown {
            return;
        }
        let tail = self.sessions[s].stderr.lock().unwrap_or_else(|e| e.into_inner()).trim().to_string();
        let msg = if tail.is_empty() { "the teq session ended".to_string() } else { format!("the teq session ended: {}", tail) };
        self.fail(s, &msg);
    }

    /// A document opened, changed or closed: a new stamp of its text, and the builds under way
    /// that carried it stale, their cancel sent.
    fn stamp(&mut self, path: &Path) {
        self.next_stamp += 1;
        let stamp = Stamp { n: self.next_stamp, version: self.docs.get(path).map(|d| d.version) };
        self.stamps.insert(path.to_path_buf(), stamp);
        for session in &mut self.sessions {
            let Some(build) = session.building() else { continue };
            if build.cancelled || !build.carried.iter().any(|p| p == path) {
                continue;
            }
            build.cancelled = true;
            let id = build.id;
            hooks::trace("server", || format!("cancel {}", id));
            session.send(&format!("cancel {}\n", id));
        }
    }

    /// A document opened, changed or closed: the sessions that own it build it after the
    /// burst of edits it is part of, but one already sent its latest text (with the first build
    /// of a child started for it).
    fn touch(&mut self, path: &Path) {
        let mut any = false;
        let latest = self.stamps.get(path);
        for s in 0..self.sessions.len() {
            if self.sessions[s].owns(path) && (latest.is_none() || self.sessions[s].sent.get(path) != latest) {
                self.sessions[s].dirty.insert(path.to_path_buf());
                any = true;
            }
        }
        if any {
            // Pushed back by every edit, within the bound, so that a burst of edits is typed once.
            let now = Instant::now();
            let start = *self.coalesce_start.get_or_insert(now);
            self.coalesce_until = Some((now + COALESCE).min(start + COALESCE_BOUND));
        }
    }

    /// Sends every gathered edit.
    fn flush(&mut self) {
        self.coalesce_until = None;
        self.coalesce_start = None;
        for s in 0..self.sessions.len() {
            if !self.sessions[s].dirty.is_empty() {
                self.ensure_running(s);
                // A session with a build under way keeps its edits for the build that follows
                // its answer, which then types the latest text once.
                self.send_build(s);
            }
        }
    }

    /// The session's next build, unless one is under way: the texts of its changed documents (a
    /// closed one withdrawn), then a build of them with what a cancelled build carried, or of
    /// every file where one was asked for; then the queries that waited for it.
    fn send_build(&mut self, s: usize) {
        let session = &mut self.sessions[s];
        if session.proc.is_none() || session.building().is_some() {
            return;
        }
        let polled = std::mem::take(&mut session.poll);
        let carried_plain = std::mem::take(&mut session.carry_plain);
        let plain = std::mem::take(&mut session.plain_wanted) | carried_plain;
        let dirty: Vec<PathBuf> = std::mem::take(&mut session.dirty).into_iter().collect();
        let mut carried = std::mem::take(&mut session.carry);
        carried.extend(dirty.iter().cloned());
        carried.sort();
        carried.dedup();
        if !plain && carried.is_empty() {
            return;
        }
        // A poll that also carries texts or a cancelled build's files is theirs as well.
        let poll = polled && !carried_plain && carried.is_empty();
        self.next_build += 1;
        let id = self.next_build;
        let command = match carried.split_first() {
            Some((first, rest)) if !plain => {
                let mut command = format!("build #{} {}\n", id, first.display());
                for path in rest {
                    command.push_str(&format!("{}\n", path.display()));
                }
                command.push('\n');
                command
            }
            _ => format!("build #{}\n", id),
        };
        let texts: Vec<(PathBuf, Option<String>)> = dirty.iter().map(|p| (p.clone(), self.docs.get(p).map(|d| d.text.clone()))).collect();
        let session = &mut self.sessions[s];
        let Some(proc) = session.proc.as_ref() else { return };
        for (path, text) in texts {
            if !proc.push(Item::Text(path.clone(), text)) {
                return;
            }
            if let Some(stamp) = self.stamps.get(&path) {
                session.sent.insert(path, *stamp);
            }
        }
        if !proc.send(&command) {
            return;
        }
        hooks::trace("server", || format!("build {} {} {}{}", id, session.label(), if plain { "plain" } else { "named" }, if poll { " poll" } else { "" }));
        let typed = session.sent.clone();
        session.expect.push_back(Expect::Build(Sent { id, sent: Instant::now(), plain, poll, carried, typed, cancelled: false }));
        session.last_build = Instant::now();
        let mut unsent = Vec::new();
        let waited = std::mem::take(&mut session.deferred);
        for (q, command) in waited {
            // A completion asked on a revision this build does not type (the edit after it came
            // first, or the build that typed it was cancelled) is never answered from it.
            if self.obsolete(q) {
                hooks::trace("server", || format!("modified {}", q));
                self.modified(q);
                continue;
            }
            let session = &mut self.sessions[s];
            if session.send(&command) {
                session.expect.push_back(Expect::Query(q, command));
            } else {
                unsent.push(q);
            }
        }
        for q in unsent {
            self.query_answered(q, s, Json::Null);
        }
    }

    /// Whether a query waits for the session's next build: the session has a build under way and
    /// another waits behind it (edits kept, a build of every file asked for, what a cancelled build
    /// carried), which the query is to read, whatever file it is about or none: an edit came
    /// before it, so it reads the program that edit makes (the offset of a file not open read from
    /// the disk that build reads again, that of an edited document in the text that build sends).
    fn query_waits(&mut self, s: usize) -> bool {
        let session = &mut self.sessions[s];
        session.wants_build() && session.building().is_some()
    }

    /// Answers the queries that waited for the session's next build, which will not come.
    fn answer_deferred(&mut self, s: usize) {
        for (q, _) in std::mem::take(&mut self.sessions[s].deferred) {
            self.query_answered(q, s, Json::Null);
        }
    }

    /// A plain build, which reads every file whose modification time moved, after the closure's
    /// generators have run on a worker (they may wait for `teq`'s or run a slow script), so
    /// that what changed their fingerprint is generated again.
    fn build_all(&mut self, s: usize) {
        let session = &self.sessions[s];
        if session.proc.is_none() || session.generating.is_some() {
            return;
        }
        let (Some(file), Some(key)) = (&session.export, &session.key) else { return self.plain_build(s) };
        let Some(Ok(export)) = self.exports.get(file).map(|l| l.export.clone()) else { return self.plain_build(s) };
        let closure = export.closure(key);
        if closure.iter().all(|k| export.configuration(k).is_none_or(|c| c.generators.is_empty())) {
            return self.plain_build(s);
        }
        let generation = session.generation;
        let classes = export.path(&export.classes(key));
        let env = std::env::vars().filter(|(k, _)| k == "SOURCE_VERSION").collect();
        let events = self.events.clone();
        self.sessions[s].generating = Some(generation);
        crate::alloc::spawn(move || {
            let result = crate::task::generators::run(&export, &closure, classes, &env).map(|_| ());
            let _ = events.send(Event::Generated(s, generation, result));
        });
    }

    /// The generators of a session's plain build ran: the build follows, or the failure is told.
    fn generated(&mut self, s: usize, generation: u64, result: Result<(), String>) {
        let Some(session) = self.sessions.get_mut(s) else { return };
        if session.generating != Some(generation) {
            return;
        }
        session.generating = None;
        if session.generation != generation || self.shutdown {
            return;
        }
        match result {
            Ok(()) => {
                self.sessions[s].failure = None;
                self.plain_build(s);
            }
            Err(msg) => self.fail(s, &msg),
        }
    }

    /// A build of every file, sent now unless one is under way, else when the session is free.
    fn plain_build(&mut self, s: usize) {
        if self.sessions[s].proc.is_none() {
            return;
        }
        self.sessions[s].plain_wanted = true;
        self.send_build(s);
    }

    /// While idle: an export that changed is read again, a session unused for the session idle
    /// time stops, a session free for `IDLE` since its last build polls (a plain build, which
    /// takes up what changed on disk without a notification), and a build pending for
    /// `PROGRESS_AFTER` is shown.
    fn reconcile(&mut self) {
        if !self.initialized || self.shutdown {
            return;
        }
        if self.discovered_at.elapsed() >= REDISCOVER {
            self.rediscover();
        }
        self.reload_exports();
        // Edits kept while a build was under way go once the session is free and no edit is
        // pending, and so does a build of every file asked for meanwhile.
        if self.coalesce_until.is_none() {
            for s in 0..self.sessions.len() {
                let session = &self.sessions[s];
                if session.proc.is_some() && session.wants_build() && session.generating.is_none() {
                    self.send_build(s);
                }
            }
        }
        for s in 0..self.sessions.len() {
            let session = &self.sessions[s];
            if session.proc.is_some() && !session.busy() && session.dirty.is_empty() && session.last_used.elapsed() >= self.session_idle {
                self.park(s);
            }
        }
        for s in 0..self.sessions.len() {
            if self.sessions[s].poll_due().is_some_and(|t| Instant::now() >= t) {
                let session = &mut self.sessions[s];
                session.poll = true;
                session.last_build = Instant::now();
                self.build_all(s);
            }
        }
        for s in 0..self.sessions.len() {
            if self.sessions[s].oldest_build().map_or(false, |t| t.elapsed() >= PROGRESS_AFTER) {
                self.begin_progress(s);
            }
        }
        self.start_due_export();
    }

    fn answer(&mut self, s: usize, line: &str) {
        let Some(expect) = self.sessions[s].expect.pop_front() else { return };
        let json = Json::parse(line).unwrap_or(Json::Null);
        match expect {
            Expect::Build(build) => {
                // The idle period runs from here: a poll slower than `IDLE` is never followed by
                // the next at once.
                self.sessions[s].last_build = Instant::now();
                self.build_answered(s, &json, build);
                self.settle_progress(s);
            }
            Expect::Discard => {}
            Expect::Query(q, _) => {
                let result = json.get("result").cloned().unwrap_or(Json::Null);
                let declared = json.get("declared").and_then(|d| Some((PathBuf::from(d.get("path")?.str()?), d.get("offset")?.uint()?)));
                if let Some((path, offset)) = declared {
                    self.follow(q, &path, offset);
                }
                self.tell_notes(s, &json);
                self.query_answered(q, s, result);
            }
        }
    }

    /// The notes of an answer told to the client (`window/logMessage`), each once per session: a
    /// build's, and a query's refusal to write a document where something else stands.
    fn tell_notes(&mut self, s: usize, answer: &Json) {
        for note in answer.get("notes").map(Json::arr).unwrap_or(&[]).iter().filter_map(Json::str) {
            let session = &mut self.sessions[s];
            if session.told.insert(note.to_string()) {
                let who = session.key.as_ref().map_or_else(|| session.folder.display().to_string(), |k| k.to_string());
                notify("window/logMessage", obj([("type", 2u32.into()), ("message", format!("teq: {}: {}", who, note).into())]));
            }
        }
    }

    /// A build's answer: one that does not echo the build's identity, a stale one (a document it
    /// carried changed since) and a cancelled one are dropped, the files of a cancelled one
    /// carried by the next build and the queries written behind it asked again after that build,
    /// since the program they were to read is never made; else its diagnostics are the session's,
    /// with the stamps of the texts it typed, and the files whose lists changed are published.
    fn build_answered(&mut self, s: usize, answer: &Json, build: Sent) {
        let echoed = answer.get("build").and_then(Json::num) == Some(build.id as f64);
        if echoed && answer.get("cancelled").and_then(Json::bool) == Some(true) {
            hooks::trace("server", || format!("cancelled {}", build.id));
            let session = &mut self.sessions[s];
            session.carry.extend(build.carried);
            session.carry_plain |= build.plain;
            // The build under way was the first the child had to answer: what follows it in the
            // queue was written behind it.
            for e in session.expect.iter_mut() {
                if let Expect::Query(q, command) = std::mem::replace(e, Expect::Discard) {
                    hooks::trace("server", || format!("deferred {}", q));
                    session.deferred.push((q, command));
                }
            }
            return;
        }
        let stale = build.carried.iter().any(|p| self.stamps.get(p) != build.typed.get(p));
        if !echoed || stale {
            hooks::trace("server", || format!("dropped {}", build.id));
            return;
        }
        hooks::trace("server", || format!("published {}", build.id));
        self.tell_notes(s, answer);
        let anchor = self.sessions[s].anchor();
        let mut by_file: BTreeMap<PathBuf, Vec<Json>> = BTreeMap::new();
        for d in answer.get("diagnostics").map(Json::arr).unwrap_or(&[]) {
            // A hint (an unused import outside the flag that makes it a warning) is the
            // protocol's severity 4, and `tags` (`Unnecessary`) go through: an editor greys it.
            let severity = match d.get("severity").and_then(Json::str) {
                Some("warning") => 2u32,
                Some("hint") => 4u32,
                _ => 1u32,
            };
            let message = d.get("message").and_then(Json::str).unwrap_or("").to_string();
            let file = d.get("uri").and_then(Json::str).and_then(uri::to_path);
            let (file, range) = match (file, d.get("range")) {
                (Some(f), Some(r)) => (f, r.clone()),
                _ => (anchor.clone(), zero_range()),
            };
            let mut diagnostic = obj([("range", range), ("severity", severity.into()), ("source", "teq".into()), ("message", message.into())]);
            if let (Some(tags), Json::Obj(fields)) = (d.get("tags"), &mut diagnostic) {
                fields.push(("tags".to_string(), tags.clone()));
            }
            let list = by_file.entry(file).or_default();
            if !list.contains(&diagnostic) {
                list.push(diagnostic);
            }
        }
        // The files whose publication waited for a session that typed their latest text.
        let mut touched: Vec<PathBuf> = self.held_back.iter().cloned().collect();
        let session = &mut self.sessions[s];
        touched.extend(session.diagnostics.keys().cloned());
        touched.extend(by_file.keys().cloned());
        // A document typed anew whose list did not change is published again with its version,
        // and one the session holds no text of any more (a child started since its close) is
        // looked at again.
        touched.extend(build.typed.iter().filter(|(p, t)| session.typed.get(*p) != Some(*t)).map(|(p, _)| p.clone()));
        touched.extend(session.typed.keys().filter(|p| !build.typed.contains_key(*p)).cloned());
        session.diagnostics = by_file;
        session.typed = build.typed;
        touched.sort();
        touched.dedup();
        self.publish_changed(&touched);
    }

    /// Publishes, for each file, the diagnostics of every session where they differ from what
    /// was last published (an empty list for a file that became clean), or where an open
    /// document's version did. Of a document, a session that owns it gives its part only where it
    /// typed the latest text (the stamps are the staleness check): a stale part is left out, and
    /// while every running owner's is stale nothing is published, so that what is published under
    /// the document's current version was typed from that very text.
    fn publish_changed(&mut self, files: &[PathBuf]) {
        // Nothing goes to the client before the answer to `initialize`.
        if !self.initialized {
            return;
        }
        for file in files {
            let latest = self.stamps.get(file);
            // Of a closed document the latest text is the disk's, which a session that holds no
            // text of it (a child started since the close) typed.
            let closed = latest.is_some_and(|l| l.version.is_none());
            let current = |x: &Session| latest.is_none() || !x.owns(file) || x.typed.get(file) == latest || (closed && x.typed.get(file).is_none());
            let mut owners = self.sessions.iter().filter(|x| latest.is_some() && x.proc.is_some() && x.owns(file)).peekable();
            if owners.peek().is_some() && !owners.any(current) {
                self.held_back.insert(file.clone());
                continue;
            }
            self.held_back.remove(file);
            let mut all: Vec<Json> = Vec::new();
            let of_folders = self.folder_diagnostics.get(file).map(Vec::as_slice).unwrap_or(&[]);
            for d in self.sessions.iter().filter(|x| current(x)).flat_map(|s| s.diagnostics.get(file).map(Vec::as_slice).unwrap_or(&[])).chain(of_folders) {
                if !all.contains(d) {
                    all.push(d.clone());
                }
            }
            let version = self.docs.get(file).map(|d| d.version);
            let before = self.published.get(file);
            if before.map_or(all.is_empty(), |(b, v)| *b == all && (all.is_empty() || *v == version)) {
                continue;
            }
            let mut params = vec![("uri".to_string(), Json::from(self.client_uri(file))), ("diagnostics".to_string(), Json::Arr(all.clone()))];
            if let Some(version) = version {
                params.insert(1, ("version".to_string(), Json::Num(version as f64)));
            }
            notify("textDocument/publishDiagnostics", Json::Obj(params));
            if all.is_empty() {
                self.published.remove(file);
            } else {
                self.published.insert(file.clone(), (all, version));
            }
        }
    }

    /// Asks the query again, at the declaration of its target, of the sessions that hold the
    /// declaration's file or serve its document (whose library view holds it as a product's or a
    /// jar's source, or whose std holds it) and were not asked yet.
    fn follow(&mut self, q: u64, path: &Path, offset: u32) {
        let Some((verb, extra)) = self.pending.get(&q).and_then(|p| p.follow) else { return };
        let asked = self.pending[&q].asked.clone();
        let command = format!("{} {} {}{}\n", verb, path.display(), offset, extra);
        for s in 0..self.sessions.len() {
            if asked.contains(&s) || !(self.sessions[s].owns(path) || self.sessions[s].serves_document(path)) || self.sessions[s].proc.is_none() {
                continue;
            }
            let asked = if self.query_waits(s) {
                hooks::trace("server", || format!("deferred {}", q));
                self.sessions[s].deferred.push((q, command.clone()));
                true
            } else if self.sessions[s].send(&command) {
                self.sessions[s].expect.push_back(Expect::Query(q, command.clone()));
                true
            } else {
                false
            };
            if asked {
                let p = self.pending.get_mut(&q).expect("pending");
                p.waiting += 1;
                p.asked.push(s);
            }
        }
    }

    /// The URI of a file (a canonical path) as the client spells it: an open document's own, a
    /// file under a workspace folder the folder's URI with the path below it (the innermost
    /// folder, the first listed among equals), another file its canonical path.
    fn client_uri(&self, path: &Path) -> String {
        if let Some(doc) = self.docs.get(path).filter(|d| !d.uri.is_empty()) {
            return doc.uri.clone();
        }
        let mut best: Option<&Folder> = None;
        for folder in &self.folders {
            if path.starts_with(&folder.root) && best.map_or(true, |b| folder.root.components().count() > b.root.components().count()) {
                best = Some(folder);
            }
        }
        match best {
            Some(folder) => uri::from_path(&folder.given.join(path.strip_prefix(&folder.root).unwrap_or(path))),
            None => uri::from_path(path),
        }
    }

    /// Every file URI of an answer in the client's spelling; the sessions answer with
    /// canonical paths.
    fn respell(&self, json: &mut Json) {
        match json {
            Json::Arr(items) => items.iter_mut().for_each(|i| self.respell(i)),
            Json::Obj(fields) => {
                for (k, v) in fields.iter_mut() {
                    match v {
                        Json::Str(u) if k == "uri" => {
                            if let Some(path) = uri::to_path(u) {
                                *u = self.client_uri(&path);
                            }
                        }
                        other => self.respell(other),
                    }
                }
            }
            _ => {}
        }
    }

    fn query_answered(&mut self, q: u64, s: usize, result: Json) {
        let Some(p) = self.pending.get_mut(&q) else { return };
        // A document asked of every library-reading session remembers its producer from the
        // winning answer alone, below; any other answer names the documents it produced.
        if p.document.is_none() {
            self.remember_documents(s, &result);
        }
        let Some(p) = self.pending.get_mut(&q) else { return };
        p.results.push((s, result));
        p.waiting -= 1;
        if p.waiting > 0 {
            return;
        }
        let mut p = self.pending.remove(&q).expect("present");
        // A document asked of every library-reading session: the first that mapped it, in the
        // sessions' order, answers and is remembered for it and for the documents it named.
        if let Some(path) = p.document.take() {
            p.results.sort_by_key(|(s, _)| *s);
            let first = p.results.iter().find(|(_, r)| !r.is_null()).map(|(s, _)| *s);
            if let Some(s) = first {
                // The producer, unless a live one stands: a retired one is replaced, so that
                // the replacement is the session woken for the document from here on.
                match self.doc_sessions.get(&path) {
                    Some(&o) if !self.sessions[o].retired => {}
                    _ => {
                        self.doc_sessions.insert(path, s);
                    }
                }
                p.results.retain(|(o, _)| *o == s);
                let winner = p.results.first().map(|(_, r)| r.clone()).unwrap_or(Json::Null);
                self.remember_documents(s, &winner);
            }
        }
        if p.origin.as_deref().is_some_and(|f| self.library_document_changed(f)) {
            return match p.item {
                Some(item) => respond(&p.id, item),
                None if p.method == "textDocument/completion" => respond(&p.id, obj([("isIncomplete", false.into()), ("items", Json::Arr(Vec::new()))])),
                None => respond(&p.id, Json::Null),
            };
        }
        let results: Vec<Json> = p.results.into_iter().map(|(_, r)| r).collect();
        if let Some(item) = p.item {
            return match resolved(item, results.into_iter().next().unwrap_or(Json::Null)) {
                Ok(item) => respond(&p.id, item),
                Err(why) => respond_error(&p.id, CONTENT_MODIFIED, why),
            };
        }
        let mut answer = merge(&p.method, &p.query, results);
        if let Some(r) = &p.revision {
            name_version(&mut answer, r.version);
        }
        self.respell(&mut answer);
        respond(&p.id, answer);
    }
}

/// Every `uri` a result names, at any depth.
fn uris_in(json: &Json, out: &mut Vec<String>) {
    match json {
        Json::Obj(fields) => {
            for (k, v) in fields {
                match v {
                    Json::Str(uri) if k == "uri" => out.push(uri.clone()),
                    other => uris_in(other, out),
                }
            }
        }
        Json::Arr(items) => items.iter().for_each(|i| uris_in(i, out)),
        _ => {}
    }
}

impl Server {
    /// The library documents a session's answer names are its: the session that produced a
    /// document is the one its queries go to, the first producer kept while
    /// it is not retired.
    fn remember_documents(&mut self, s: usize, result: &Json) {
        let mut uris = Vec::new();
        uris_in(result, &mut uris);
        for uri in uris {
            let Some(path) = uri::to_path(&uri) else { continue };
            let path = canonical(&path);
            if !crate::typer::loader::attach::is_document(&path) {
                continue;
            }
            match self.doc_sessions.get(&path) {
                Some(&o) if !self.sessions[o].retired => {}
                _ => {
                    self.doc_sessions.insert(path, s);
                }
            }
        }
    }
}

/// The item of `completionItem/resolve` with what the session answered: the edits that import
/// its name, or that the program changed since the item was offered.
fn resolved(item: Json, answer: Json) -> Result<Json, &'static str> {
    if answer.get("modified").and_then(Json::bool) == Some(true) {
        return Err("the program changed");
    }
    if let Some(warning) = answer.get("warning").and_then(Json::str) {
        eprintln!("teq lsp: {}", warning);
    }
    let Some(edits) = answer.get("edits").filter(|e| !e.arr().is_empty()) else { return Ok(item) };
    let Json::Obj(mut fields) = item else { return Ok(item) };
    fields.retain(|(k, _)| k != "additionalTextEdits");
    fields.push(("additionalTextEdits".to_string(), edits.clone()));
    Ok(Json::Obj(fields))
}

/// Names the document's version in the `data` of a completion list's items, which their
/// resolve checks against the document's version then.
fn name_version(list: &mut Json, version: i64) {
    let Json::Obj(fields) = list else { return };
    let Some((_, Json::Arr(items))) = fields.iter_mut().find(|(k, _)| k == "items") else { return };
    for item in items {
        let Json::Obj(fields) = item else { continue };
        if let Some((_, Json::Obj(data))) = fields.iter_mut().find(|(k, _)| k == "data") {
            data.push(("version".to_string(), Json::Num(version as f64)));
        }
    }
}

fn zero_range() -> Json {
    let at = || obj([("line", 0u32.into()), ("character", 0u32.into())]);
    obj([("start", at()), ("end", at())])
}

fn diagnostic_at_start(msg: &str) -> Json {
    obj([("range", zero_range()), ("severity", 1u32.into()), ("source", "teq".into()), ("message", msg.into())])
}

/// The answers of several sessions as one: lists joined without repeats, the call hierarchy's
/// calls of one caller or callee joined into one entry, `workspace/symbol` ordered again and
/// cut to its limit; for a single value the first that is not null.
fn merge(method: &str, query: &str, results: Vec<Json>) -> Json {
    if results.iter().all(Json::is_null) {
        return Json::Null;
    }
    if !results.iter().any(|r| matches!(r, Json::Arr(_))) {
        return results.into_iter().find(|r| !r.is_null()).unwrap_or(Json::Null);
    }
    let mut out: Vec<Json> = Vec::new();
    let key_field = match method {
        "callHierarchy/incomingCalls" => Some("from"),
        "callHierarchy/outgoingCalls" => Some("to"),
        _ => None,
    };
    for r in results {
        for item in r.arr() {
            if let Some(key) = key_field {
                if let Some(existing) = out.iter_mut().find(|e| e.get(key) == item.get(key)) {
                    if let (Json::Obj(fields), Some(ranges)) = (existing, item.get("fromRanges")) {
                        if let Some((_, Json::Arr(have))) = fields.iter_mut().find(|(k, _)| k == "fromRanges") {
                            for range in ranges.arr() {
                                if !have.contains(range) {
                                    have.push(range.clone());
                                }
                            }
                        }
                    }
                    continue;
                }
            }
            if !out.contains(item) {
                out.push(item.clone());
            }
        }
    }
    if method == "workspace/symbol" {
        out.sort_by_cached_key(|s| {
            let name = s.get("name").and_then(Json::str).unwrap_or("").to_lowercase();
            (!name.starts_with(query), name)
        });
        out.truncate(WORKSPACE_SYMBOLS);
    }
    Json::Arr(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(key: Option<Key>, root: &str) -> Session {
        let description = Description { root: PathBuf::from(root), inputs: Vec::new(), excludes: Vec::new(), classpath: Vec::new(), jvm_std: None, flags: Vec::new() };
        let export = key.as_ref().map(|_| PathBuf::from("/w/teq.lock"));
        Session::new(PathBuf::from("/w"), export, key, Ok(description))
    }

    #[test]
    fn labels() {
        assert_eq!(session(Some(Key::new("api", "test")), "/w").label(), "api/test");
        assert_eq!(session(None, "/w/bare").label(), "bare");
    }

    #[test]
    fn wake() {
        let now = Instant::now();
        let ms = Duration::from_millis;
        assert_eq!(wake_in(now, [None, None, None]), ms(500));
        assert_eq!(wake_in(now, [None, Some(now + ms(200)), None]), ms(200));
        assert_eq!(wake_in(now, [Some(now + ms(50)), Some(now + ms(200)), None]), ms(50));
        assert_eq!(wake_in(now, [None, Some(now - ms(1)), None]), Duration::ZERO);
        assert_eq!(wake_in(now, [None, Some(now + Duration::from_secs(9)), None]), ms(500));
        assert_eq!(wake_in(now, [None, Some(now + ms(300)), Some(now + ms(120))]), ms(120));
    }

    #[test]
    fn oldest_build_is_the_first_pending() {
        let mut s = session(None, "/w/bare");
        assert_eq!(s.oldest_build(), None);
        let first = Instant::now();
        let build = |id, sent, poll| Expect::Build(Sent { id, sent, plain: true, poll, carried: Vec::new(), typed: BTreeMap::new(), cancelled: false });
        s.expect.push_back(Expect::Query(1, String::new()));
        s.expect.push_back(build(1, first, false));
        s.expect.push_back(build(2, first + Duration::from_millis(250), false));
        assert_eq!(s.oldest_build(), Some(first));
        s.expect.pop_front();
        s.expect.pop_front();
        assert_eq!(s.oldest_build(), Some(first + Duration::from_millis(250)));
        // The idle poll is shown only while something waits behind it.
        s.expect.clear();
        s.expect.push_back(build(3, first, true));
        assert_eq!(s.oldest_build(), None);
        s.expect.push_back(Expect::Query(2, String::new()));
        assert_eq!(s.oldest_build(), Some(first));
        s.expect.pop_back();
        s.generating = Some(0);
        assert_eq!(s.oldest_build(), Some(first));
        s.generating = None;
        s.dirty.insert(PathBuf::from("/w/bare/A.scala"));
        assert_eq!(s.oldest_build(), Some(first));
    }
}
