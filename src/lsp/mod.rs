//! `teq lsp`: a language server over stdin and stdout for navigation and diagnostics, which
//! drives one resident check per project (`teq compiler watch --check --index`, `src/watch.rs`) as a
//! child process and speaks JSON-RPC to the client (docs/TARGETS.md, "The language server").
//!
//! One thread reads the client's messages, one per child reads its answers, and this loop
//! handles both in order. A child answers its commands in the order they were written: the loop
//! keeps per child the answers it waits for, in order (`Session::expect`). A request is live from
//! its registration to its one end (`end`), which sends its one response: its answer once each of
//! its parts (one command to each session asked) has answered, `RequestCancelled` for the client's
//! `$/cancelRequest`, `ContentModified` for an answer found in another text than the one its
//! position was given in (`index::digest`). A part waits in the server (`Session::held`) until its
//! session has nothing committed and the builds it waits for have answered (`admit`), so that it
//! reads the program of the edits that came before it and is never written behind a build, and a
//! cancel removes one never written; one committed when its request ends keeps its slot until its
//! answer comes, which is dropped. A child has one build under way at most, which carries an
//! identity its answer echoes; a build a newer edit of a document it carried overtakes is stale:
//! asked to stop (`cancel`), and its answer dropped (docs/TARGETS.md, "Documents and diagnostics").

mod export;
pub(crate) mod hooks;
pub mod json;
mod rpc;
mod sources;
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
/// A request the client cancelled (`$/cancelRequest`) while it was under way.
const REQUEST_CANCELLED: i64 = -32800;

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
    /// A part of a request (`Pending::parts`): the request and the part's index.
    Query(u64, usize),
    /// The answer nobody reads to a command of the request named, which ended while the command was
    /// committed (cancelled, or answered without it): its slot stays until the answer comes, so that
    /// the answers stay in step, and the session takes no other query meanwhile.
    Discard(u64),
}

/// One command of a request to one session, and where it stands: held in the server
/// (`Session::held`), committed once the child's outbox has it (its answer awaited in
/// `Session::expect`), done once answered or nulled (its session went, or it could not be sent).
struct Part {
    s: usize,
    command: String,
    /// The digest of the text its position is in (`index::digest`), which the session's answer must
    /// name: the text the request's position was turned into an offset in, or, for a follow-up, the
    /// declaration's file's text in the session that answered where the target is declared. `None`
    /// for a query without a position, or one the server has no text for (`position_at`).
    expect: Option<String>,
    /// The file its position is in, which a session that answers on another text of it, where the
    /// client has not opened it, reads again from the disk.
    file: Option<PathBuf>,
    state: PartState,
}

/// A part held in a session (`Session::held`), and the build it reads the program of: the build under
/// way when it was made, or the next where edits waited to be sent (`Session::wants_build`), so that
/// a query reads the program of the edits that came before it.
struct Held {
    q: u64,
    part: usize,
    after: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PartState {
    Held,
    Committed,
    Done,
}

/// The key a client's request id is matched by: its JSON text, so that an integer id and a string
/// id never match, and a cancel names a request by the id as the request spelled it.
fn id_key(id: &Json) -> String {
    id.to_text()
}

/// A build sent to a child, which carries its identity (`build #<id>`) and whose answer echoes
/// it: when it was sent, whether it reads every file and whether it is the idle poll alone, the
/// documents it carried and the stamp of every text the child held. It is stale once a document
/// it carried has a newer stamp: its answer is then dropped, and its cancel sent to the child
/// (`cancelled`), which stops it if it has not begun typing.
struct Sent {
    id: u64,
    /// Its place among the builds sent to the child (`Session::sent_builds`).
    seq: u64,
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
    /// A query's command, and the query it is for, which the trace names once it is written.
    Query(u64, String),
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
        Item::Line(l) | Item::Query(_, l) => l.len(),
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
                Item::Line(l) | Item::Query(_, l) => l.clone(),
                Item::Quit => "quit\n".to_string(),
            };
            if stdin.write_all(bytes.as_bytes()).and_then(|_| stdin.flush()).is_err() || matches!(item, Item::Quit) {
                return;
            }
            match &item {
                Item::Line(l) => {
                    if let Some(id) = l.strip_prefix("cancel ") {
                        hooks::trace("server", || format!("wrote-cancel {}", id.trim_end()));
                    }
                }
                Item::Query(q, _) => hooks::trace("server", || format!("wrote-query {}", q)),
                _ => {}
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
    /// The parts of requests held in the server, in order, until the child has nothing committed
    /// and the builds each waits for have answered (`admit`): a cancel removes one never written.
    held: VecDeque<Held>,
    /// The builds sent to the child, and the place of the last of them answered that typed
    /// (published or dropped; a cancelled one typed nothing): what a held part waits for.
    sent_builds: u64,
    typed_builds: u64,
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
            held: VecDeque::new(),
            sent_builds: 0,
            typed_builds: 0,
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
        let waited_on = self.wants_build() || self.generating.is_some() || !self.held.is_empty() || self.expect.iter().any(|e| matches!(e, Expect::Query(..)));
        self.expect.iter().find_map(|e| match e {
            Expect::Build(b) if !b.poll || waited_on => Some(b.sent),
            Expect::Build(_) => None,
            Expect::Query(..) | Expect::Discard(_) => None,
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
            Expect::Query(..) | Expect::Discard(_) => None,
        })
    }

    fn send(&mut self, text: &str) -> bool {
        self.proc.as_ref().map_or(false, |p| p.send(text))
    }

    /// Admits the query `q`'s command to the child's outbox, which writes it in its turn.
    fn send_query(&mut self, q: u64, command: &str) -> bool {
        let sent = self.proc.as_ref().map_or(false, |p| p.push(Item::Query(q, command.to_string())));
        if sent {
            hooks::trace("server", || format!("dispatched {} {}", q, self.label()));
        }
        sent
    }

    fn busy(&self) -> bool {
        !self.expect.is_empty() || !self.held.is_empty()
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
    /// The text's digest (`index::digest`), which a query on the document expects of its answer.
    digest: String,
}


/// A client request waiting for its children's answers: live from its registration (`register`)
/// to its one end (`end`), which sends its one response.
struct Pending {
    id: Json,
    method: String,
    /// Its commands, one per session asked: the sessions it was asked of first, then those its
    /// follow-ups ask (`follow`).
    parts: Vec<Part>,
    results: Vec<(usize, Json)>,
    /// The query of `workspace/symbol`, which orders the joined answers.
    query: String,
    /// The item of `completionItem/resolve`, which the answer completes.
    item: Option<Json>,
    /// The version of the document a completion, a resolve or a signature help was asked on
    /// (0 for a file not open), which a completion's items' `data` name for their resolve.
    version: Option<i64>,
    /// For a query about a target: the command's verb and what follows the offset, which the
    /// sessions that hold the target's declaration and were not asked are asked with there.
    follow: Option<(&'static str, &'static str)>,
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
    /// The live requests by their client id (`id_key`).
    ids: BTreeMap<String, u64>,
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
    /// Whether the client watches a glob relative to a directory of its own (LSP 3.17's `RelativePattern`).
    relative_patterns: bool,
    /// The watchers of the inputs the exports declare (a Mill build's `inputs.declared`) last registered, as
    /// their text.
    declared_watched: Vec<String>,
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
    /// Whether the client takes a workspace edit as document changes, each naming the version of
    /// the document it edits.
    document_changes: bool,
    /// Whether the client takes a signature's parameter label as a range of the signature's
    /// label (`labelOffsetSupport`); else as the label's text, as dotty's presentation compiler
    /// gives it (`SignatureHelpProvider.paramToParameterInformation`).
    label_offsets: bool,
    /// The fetches of the sessions' libraries' sources.
    sources: sources::Fetcher,
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
        ids: BTreeMap::new(),
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
        relative_patterns: false,
        declared_watched: Vec::new(),
        progress: false,
        next_request: 0,
        discovered_at: Instant::now(),
        exports: BTreeMap::new(),
        max_sessions: MAX_SESSIONS,
        session_idle: SESSION_IDLE,
        insert_replace: false,
        snippets: false,
        document_changes: false,
        label_offsets: false,
        sources: sources::Fetcher::new(),
    };
    server.serve(inbox)
}

/// Whether a `workspace/didChangeWatchedFiles` names an export.
fn names_an_export(params: &Json) -> bool {
    let changes = params.get("changes").map(Json::arr).unwrap_or(&[]);
    changes.iter().filter_map(|c| c.get("uri")?.str()).any(|uri| uri.ends_with(&format!("/{}", crate::task::export::FILE)))
}

/// The globs of the build definition files the server watches: sbt's and Mill's (the files
/// `task::export::input_digests` reads of either) and the exports.
const WATCHED: [&str; 16] = [
    "**/*.scala",
    "**/teq.lock",
    "**/*.sbt",
    "**/project/build.properties",
    "**/*.mill",
    "**/*.mill.yaml",
    "**/*.mill.scala",
    "**/*.sc",
    "**/mill-build/**",
    "**/.mill-version",
    "**/.config/mill-version",
    "**/.mill-jvm-version",
    "**/.config/mill-jvm-version",
    "**/.mill-jvm-opts",
    "**/.mill-opts",
    "**/teq.mill.json",
];

/// Whether a file a `workspace/didChangeWatchedFiles` or a `didSave` names is a build definition file: an
/// sbt build's `.sbt` files, `project/build.properties` and the sources under `project/`, none under a
/// `target`; a Mill build's scripts, meta-build, launcher files and overlay, none under its `out/`.
fn is_build_file(uri: &str) -> bool {
    let sbt = !uri.contains("/target/") && (uri.ends_with(".sbt") || uri.ends_with("/project/build.properties") || (uri.contains("/project/") && uri.ends_with(".scala")));
    let name = uri.rsplit('/').next().unwrap_or(uri);
    let mill = !uri.contains("/out/")
        && ([".mill", ".mill.yaml", ".mill.scala", ".sc"].iter().any(|ext| name.ends_with(ext))
            || uri.contains("/mill-build/")
            || uri.ends_with("/.config/mill-version")
            || uri.ends_with("/.config/mill-jvm-version")
            || [".mill-version", ".mill-jvm-version", ".mill-jvm-opts", ".mill-opts", crate::task::export::MILL_OVERLAY].contains(&name));
    sbt || mill
}

/// The files a `workspace/didChangeWatchedFiles` or a `didSave` names that are a build's inputs: its
/// definition files (`is_build_file`), or one of `declared`, the inputs the exports declare.
fn changed_build_files(params: &Json, declared: &BTreeSet<PathBuf>) -> Vec<PathBuf> {
    let mut uris: Vec<&str> = params.get("changes").map(Json::arr).unwrap_or(&[]).iter().filter_map(|c| c.get("uri")?.str()).collect();
    uris.extend(params.at(&["textDocument", "uri"]).and_then(Json::str));
    uris.into_iter()
        .filter_map(|uri| {
            let path = canonical(&uri::to_path(uri)?);
            (is_build_file(uri) || declared.contains(&path)).then_some(path)
        })
        .collect()
}

/// The watchers of the inputs the exports declare: a glob of the file's name (`**/<name>`) for one inside a
/// workspace folder, which the client resolves against its folders; one outside them (a Mill build's
/// `../external.txt`) watched from its own directory, as a relative pattern where the client takes one (LSP
/// 3.17's `relativePatternSupport`), else as its absolute path.
fn declared_watchers(declared: &BTreeSet<PathBuf>, folders: &[PathBuf], relative: bool) -> Vec<Json> {
    let mut out: Vec<Json> = Vec::new();
    for path in declared {
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else { continue };
        let watcher = match path.parent() {
            _ if folders.iter().any(|f| path.starts_with(f)) => obj([("globPattern", format!("**/{name}").into())]),
            Some(dir) if relative => obj([("globPattern", obj([("baseUri", uri::from_path(dir).into()), ("pattern", name.into())]))]),
            _ => obj([("globPattern", path.to_string_lossy().replace('\\', "/").into())]),
        };
        if !out.contains(&watcher) {
            out.push(watcher);
        }
    }
    out
}

/// Whether changed files touch an export: one under its build's root, or one of the inputs it declares,
/// wherever it is (a Mill build's `../external.txt`).
fn touches(root: &Path, declared: &[PathBuf], files: &[PathBuf]) -> bool {
    files.iter().any(|p| p.starts_with(root) || declared.contains(p))
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

/// Whether nothing at all is at `path`, a dangling link being something: a library document's
/// file removed with teq's cache, which the session makes whole again.
fn missing(path: &Path) -> bool {
    matches!(std::fs::symlink_metadata(path), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
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
                    let json = Json::parse(&line).unwrap_or(Json::Null);
                    // What a child reports of the libraries' sources is the fetcher's before anything is
                    // decided about the message that carried it: a stale child's, a discarded or dropped
                    // answer's.
                    self.sources_reported(&json);
                    if self.sessions.get(s).map_or(false, |x| x.generation == generation) {
                        self.answer(s, json);
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
        self.sources.stop();
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
        // A request is matched by its id while it is under way: one that reuses the id of another
        // under way is refused, so that each is answered once and a cancel names one.
        if self.ids.contains_key(&id_key(&id)) {
            return respond_error(&id, INVALID_REQUEST, "a request with the id of one under way");
        }
        match method {
            "shutdown" => {
                self.shutdown = true;
                for s in 0..self.sessions.len() {
                    if let Some(p) = self.sessions[s].proc.take() {
                        p.end();
                        self.ending.push(p);
                    }
                    self.drop_parts(s);
                    self.settle_progress(s);
                }
                self.kill_exports();
                self.sources.stop();
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
            "textDocument/codeAction" => self.code_actions(id, params),
            "textDocument/documentSymbol" => match document_path(params) {
                Some(path) if self.library_document_changed(&path) => respond(&id, Json::Null),
                Some(path) => {
                    let command = format!("symbols {}\n", path.display());
                    self.ask(id, method, Some(&path), &command, true, None)
                }
                None => respond(&id, Json::Null),
            },
            "workspace/symbol" => {
                let query = params.get("query").and_then(Json::str).unwrap_or("").replace('\n', " ");
                self.ask(id, method, None, &format!("workspace-symbols {}\n", query), false, None)
            }
            "callHierarchy/incomingCalls" | "callHierarchy/outgoingCalls" => {
                let item = params.get("item");
                let path = item.and_then(|i| i.get("uri")).and_then(Json::str).and_then(uri::to_path).map(|p| canonical(&p));
                let start = item.and_then(|i| Some((i.at(&["selectionRange", "start", "line"])?.uint()?, i.at(&["selectionRange", "start", "character"])?.uint()?)));
                // The digest of the text of the item's document its session answered on (`data`, which
                // the client keeps between the item's requests): the item's position is in that text.
                let given = item.and_then(|i| i.at(&["data", "text"])).and_then(Json::str).map(str::to_string);
                match (path, start) {
                    (Some(path), _) if self.library_document_changed(&path) => respond(&id, Json::Null),
                    (Some(path), Some((line, character))) => {
                        let verb = if method == "callHierarchy/incomingCalls" { "incoming" } else { "outgoing" };
                        let Some((at, text)) = self.position_at(&path, line, character) else { return respond(&id, Json::Null) };
                        // Turned into an offset in another text, the position would name another symbol.
                        if given.is_some() && text.is_some() && given != text {
                            return respond_error(&id, CONTENT_MODIFIED, "the document changed");
                        }
                        let command = format!("{} {} {}\n", verb, path.display(), at);
                        self.ask(id, method, Some(&path), &command, false, given.or(text))
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
        let Some((at, text)) = self.position_at(&path, line, character) else { return respond(&id, Json::Null) };
        let command = format!("{} {} {}{}\n", verb, path.display(), at, extra);
        self.ask(id, method, Some(&path), &command, false, text)
    }

    /// Whether a file no session owns (a library's source) is open with a text
    /// other than the one on disk, which the sessions' positions describe: its queries answer
    /// nothing. What is at the path and is not a regular file, or does not read, is another text.
    /// No file at all (teq's cache cleared) is none: the session's lookup reacquires the sources or
    /// reports their miss and makes the document whole again, and the answer's origin is checked
    /// here again when it goes out, on the file the session wrote.
    fn library_document_changed(&self, path: &Path) -> bool {
        match self.docs.get(path) {
            Some(_) if missing(path) => false,
            Some(doc) if !self.sessions.iter().any(|s| s.owns(path)) => read_text(path).map_or(true, |t| t != doc.text),
            _ => false,
        }
    }

    /// The position a navigation query sends: the byte offset in the text the client holds
    /// (`offset`), or `<line>:<character>` for a jar's document the client has not opened whose
    /// file is gone. There is no text here to turn that position with: the session resolves it in
    /// the document it makes whole through its lookup (`Query::parse_at`).
    /// With the digest of the text the offset is in (`offset`); none for a line and a character.
    fn position_at(&self, path: &Path, line: u32, character: u32) -> Option<(String, Option<String>)> {
        let attached = || crate::typer::loader::attach::is_document(path) && !crate::typer::loader::attach::is_std_document(path);
        if !self.docs.contains_key(path) && missing(path) && attached() {
            return Some((format!("{}:{}", line, character), None));
        }
        self.offset(path, line, character).map(|(o, text)| (o.to_string(), Some(text)))
    }

    /// The byte offset of a position in the file as the client has it, the open document's text or
    /// the file on disk, and that text's digest (`index::digest`), which the answer must name: a
    /// session that answers on another text of the file would answer for whatever stands at the
    /// offset there.
    fn offset(&self, path: &Path, line: u32, character: u32) -> Option<(u32, String)> {
        let positions = self.positions();
        match self.docs.get(path) {
            Some(doc) => Some((crate::index::offset_of(&doc.text, line, character, positions), doc.digest.clone())),
            None => {
                let text = read_text(path)?;
                Some((crate::index::offset_of(&text, line, character, positions), crate::index::digest(text.as_bytes())))
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
    fn ask(&mut self, id: Json, method: &str, file: Option<&Path>, command: &str, one: bool, expect: Option<String>) {
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
        let parts = targets.iter().map(|&s| Part { s, command: command.to_string(), expect: expect.clone(), file: origin.clone(), state: PartState::Held }).collect();
        let q = self.register(Pending { id, method: method.to_string(), parts, results: Vec::new(), query, item, version: None, follow, document, origin });
        for i in 0..targets.len() {
            self.offer(q, i);
        }
    }

    /// A request live from here: under its client id, with its parts made and not yet offered.
    fn register(&mut self, request: Pending) -> u64 {
        let q = self.next_query;
        self.next_query += 1;
        self.ids.insert(id_key(&request.id), q);
        self.pending.insert(q, request);
        q
    }

    /// A part made: committed at once where its session has nothing committed and nothing held,
    /// else held in the server (`admit`), where a cancel removes it before it is written.
    fn offer(&mut self, q: u64, i: usize) {
        let Some(s) = self.pending.get(&q).map(|p| p.parts[i].s) else { return };
        let session = &mut self.sessions[s];
        let after = session.sent_builds + u64::from(session.wants_build());
        session.held.push_back(Held { q, part: i, after });
        self.admit(s);
        if self.sessions[s].held.iter().any(|h| (h.q, h.part) == (q, i)) {
            hooks::trace("server", || format!("deferred {}", q));
        }
    }

    /// Commits the session's held parts in order, one at a time: each once the child has nothing
    /// committed (no build under way, no command's answer awaited, a cancelled one's included) and
    /// the builds it waits for have answered.
    fn admit(&mut self, s: usize) {
        loop {
            let session = &mut self.sessions[s];
            let Some(first) = session.held.front() else { return };
            if !session.expect.is_empty() || session.proc.is_none() || first.after > session.typed_builds {
                return;
            }
            let Held { q, part, .. } = session.held.pop_front().expect("first");
            self.commit(q, part);
        }
    }

    /// Commits a part of a live request: its command admitted to the child's outbox and its answer
    /// awaited; one that cannot be sent (the child is ended) is answered nothing there.
    fn commit(&mut self, q: u64, i: usize) {
        let Some(p) = self.pending.get_mut(&q) else { return };
        let (s, command) = (p.parts[i].s, p.parts[i].command.clone());
        if self.sessions[s].send_query(q, &command) {
            self.pending.get_mut(&q).expect("live").parts[i].state = PartState::Committed;
            self.sessions[s].expect.push_back(Expect::Query(q, i));
        } else {
            self.part_done(q, i, Json::Null);
        }
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
    /// build of the document's text as the request found it (a query held behind a build under
    /// way waits for the one that sends that text, `admit`); a file no session holds answers the
    /// empty list.
    fn complete(&mut self, id: Json, params: &Json) {
        let empty = || obj([("isIncomplete", false.into()), ("items", Json::Arr(Vec::new()))]);
        let (Some(path), Some((line, character))) = (document_path(params), position(params)) else { return respond(&id, empty()) };
        let Some((offset, text)) = self.offset(&path, line, character) else { return respond(&id, empty()) };
        let version = self.docs.get(&path).map_or(0, |d| d.version);
        self.flush();
        let Some(s) = self.one_for(&path) else { return respond(&id, empty()) };
        self.sessions[s].last_used = Instant::now();
        let flags = self.insert_replace as u32 | (self.snippets as u32) << 1;
        let command = format!("complete {} {} {}\n", path.display(), offset, flags);
        self.query_one(id, "textDocument/completion", s, &command, None, path, version, text);
    }

    /// `textDocument/signatureHelp`: asked as a completion is, of one session that owns the
    /// file (`one_for`) after the build of the document's text as the request found it, answered
    /// `ContentModified` once that text is never typed; null where no session holds the file.
    fn signature_help(&mut self, id: Json, params: &Json) {
        let (Some(path), Some((line, character))) = (document_path(params), position(params)) else { return respond(&id, Json::Null) };
        let Some((offset, text)) = self.offset(&path, line, character) else { return respond(&id, Json::Null) };
        let version = self.docs.get(&path).map_or(0, |d| d.version);
        self.flush();
        let Some(s) = self.one_for(&path) else { return respond(&id, Json::Null) };
        self.sessions[s].last_used = Instant::now();
        let command = format!("signature {} {}\n", path.display(), offset);
        self.query_one(id, "textDocument/signatureHelp", s, &command, None, path, version, text);
    }

    /// Sends one session a query whose answer answers the request.
    /// The document's version names the items of a completion (`name_version`), and the answer must be
    /// found in the text of digest `text`.
    #[allow(clippy::too_many_arguments)]
    fn query_one(&mut self, id: Json, method: &str, s: usize, command: &str, item: Option<Json>, path: PathBuf, version: i64, text: String) {
        let parts = vec![Part { s, command: command.to_string(), expect: Some(text), file: Some(path.clone()), state: PartState::Held }];
        let q = self.register(Pending { id, method: method.to_string(), parts, results: Vec::new(), query: String::new(), item, version: Some(version), follow: None, document: None, origin: Some(path) });
        self.offer(q, 0);
    }

    /// `$/cancelRequest`: the request of that id under way ends `RequestCancelled`; an id no
    /// request under way has (answered already, or never asked) is passed over.
    fn cancel(&mut self, id: &Json) {
        if let Some(&q) = self.ids.get(&id_key(id)) {
            hooks::trace("server", || format!("request-cancelled {}", q));
            self.end(q, Err((REQUEST_CANCELLED, "the request was cancelled")));
        }
    }

    /// The request's one end: its response sent, the request and its id forgotten, each part of it
    /// still held in a session dropped, and each committed to a child left to drain
    /// (`Expect::Discard`), so that the child's answers stay in step.
    fn end(&mut self, q: u64, outcome: Result<Json, (i64, &str)>) {
        let Some(p) = self.pending.remove(&q) else { return };
        self.ids.remove(&id_key(&p.id));
        for (i, part) in p.parts.iter().enumerate() {
            let session = &mut self.sessions[part.s];
            match part.state {
                PartState::Held => session.held.retain(|h| (h.q, h.part) != (q, i)),
                PartState::Committed => {
                    for e in session.expect.iter_mut() {
                        if matches!(e, Expect::Query(x, j) if (*x, *j) == (q, i)) {
                            *e = Expect::Discard(q);
                        }
                    }
                }
                PartState::Done => {}
            }
        }
        match outcome {
            Ok(result) => respond(&p.id, result),
            Err((code, message)) => respond_error(&p.id, code, message),
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
        // The item's offset is in the text of the version it names, the document's now.
        let text = self.docs.get(&path).map_or_else(String::new, |d| d.digest.clone());
        let command = format!("complete-resolve {} {} {} {}\n", path.display(), offset, generation, name);
        self.query_one(id, "completionItem/resolve", s, &command, Some(item), path, version, text);
    }

    fn notification(&mut self, method: &str, params: &Json) {
        if self.shutdown && method != "exit" {
            return;
        }
        match method {
            "initialized" => {
                self.register_watchers();
                self.register_declared_watchers();
                self.start_sessions()
            }
            "exit" => self.exit(),
            "$/cancelRequest" => {
                if let Some(id) = params.get("id") {
                    self.cancel(id);
                }
            }
            "textDocument/didOpen" => {
                let Some(path) = document_path(params) else { return };
                let text = params.at(&["textDocument", "text"]).and_then(Json::str).unwrap_or("").to_string();
                let version = params.at(&["textDocument", "version"]).and_then(Json::num).unwrap_or(0.0) as i64;
                let uri = params.at(&["textDocument", "uri"]).and_then(Json::str).unwrap_or("").to_string();
                let digest = crate::index::digest(text.as_bytes());
                self.docs.insert(path.clone(), Doc { text, version, uri, digest });
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
                self.docs.insert(path.clone(), Doc { text: text.to_string(), version, uri, digest: crate::index::digest(text.as_bytes()) });
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
                let declared = self.declared_inputs();
                self.build_files_changed(&changed_build_files(params, &declared));
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

    /// `textDocument/codeAction`: a quick fix for each diagnostic published on the document whose
    /// range meets the one asked on and that carries the compiler's change (an unused import's
    /// removal, `typer::unused_edits`). The edits were computed on the text the diagnostics were
    /// published for: where the document changed since, there are none until the next build's
    /// diagnostics are.
    fn code_actions(&mut self, id: Json, params: &Json) {
        let none = || Json::Arr(Vec::new());
        let only = params.at(&["context", "only"]).map(Json::arr);
        if only.is_some_and(|kinds| !kinds.iter().any(|k| matches!(k.str(), Some("quickfix" | "")))) {
            return respond(&id, none());
        }
        let Some(path) = document_path(params) else { return respond(&id, none()) };
        let current = self.docs.get(&path).map(|d| d.version);
        let Some((published, version)) = self.published.get(&path).filter(|(_, v)| *v == current) else { return respond(&id, none()) };
        let point = |p: Option<&Json>| -> Option<(u32, u32)> { Some((p?.get("line")?.uint()?, p?.get("character")?.uint()?)) };
        let (Some(from), Some(to)) = (point(params.at(&["range", "start"])), point(params.at(&["range", "end"]))) else { return respond(&id, none()) };
        let uri = self.client_uri(&path);
        let mut actions = Vec::new();
        for d in published {
            let Some(action) = d.at(&["data", "action"]) else { continue };
            let (Some(start), Some(end)) = (point(d.at(&["range", "start"])), point(d.at(&["range", "end"]))) else { continue };
            if end < from || to < start {
                continue;
            }
            let edits = action.get("edits").cloned().unwrap_or(Json::Arr(Vec::new()));
            let edit = match version {
                Some(v) if self.document_changes => {
                    let document = obj([("uri", uri.clone().into()), ("version", Json::Num(*v as f64))]);
                    obj([("documentChanges", Json::Arr(vec![obj([("textDocument", document), ("edits", edits)])]))])
                }
                _ => obj([("changes", Json::Obj(vec![(uri.clone(), edits)]))]),
            };
            let title = action.get("title").and_then(Json::str).unwrap_or("").to_string();
            actions.push(obj([("title", title.into()), ("kind", "quickfix".into()), ("diagnostics", Json::Arr(vec![d.clone()])), ("isPreferred", true.into()), ("edit", edit)]));
        }
        respond(&id, Json::Arr(actions));
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
            ("codeActionProvider", obj([("codeActionKinds", Json::Arr(vec!["quickfix".into()]))])),
        ]);
        self.insert_replace = params.at(&["capabilities", "textDocument", "completion", "completionItem", "insertReplaceSupport"]).and_then(Json::bool) == Some(true);
        self.snippets = params.at(&["capabilities", "textDocument", "completion", "completionItem", "snippetSupport"]).and_then(Json::bool) == Some(true);
        self.document_changes = params.at(&["capabilities", "workspace", "workspaceEdit", "documentChanges"]).and_then(Json::bool) == Some(true);
        self.label_offsets = params.at(&["capabilities", "textDocument", "signatureHelp", "signatureInformation", "parameterInformation", "labelOffsetSupport"]).and_then(Json::bool) == Some(true);
        self.watch_files = params.at(&["capabilities", "workspace", "didChangeWatchedFiles", "dynamicRegistration"]).and_then(Json::bool) == Some(true);
        self.relative_patterns = params.at(&["capabilities", "workspace", "didChangeWatchedFiles", "relativePatternSupport"]).and_then(Json::bool) == Some(true);
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
        let watchers = Json::Arr(WATCHED.iter().map(|g| obj([("globPattern", (*g).into())])).collect());
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
        let touched: Vec<PathBuf> = self
            .exports
            .iter()
            .filter(|(file, loaded)| {
                let declared: Vec<PathBuf> = loaded.export.as_ref().map(|e| e.declared.iter().map(|d| canonical(&e.path(d))).collect()).unwrap_or_default();
                touches(&crate::task::export::root_of(file), &declared, files)
            })
            .map(|(file, _)| file.clone())
            .collect();
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
    /// The inputs the loaded exports declare besides their definition files (a Mill build's
    /// `inputs.declared`), each canonical.
    fn declared_inputs(&self) -> BTreeSet<PathBuf> {
        self.exports.values().filter_map(|l| l.export.as_ref().ok()).flat_map(|e| e.declared.iter().map(|d| canonical(&e.path(d))).collect::<Vec<_>>()).collect()
    }

    /// Registers the watchers of the declared inputs anew when they changed (`declared_watchers`), a
    /// change of which `changed_build_files` keeps when it is one of them.
    fn register_declared_watchers(&mut self) {
        if !self.watch_files {
            return;
        }
        let folders: Vec<PathBuf> = self.folders.iter().map(|f| f.root.clone()).collect();
        let watchers = declared_watchers(&self.declared_inputs(), &folders, self.relative_patterns);
        let globs: Vec<String> = watchers.iter().map(Json::to_text).collect();
        if globs == self.declared_watched {
            return;
        }
        let id = "teq-declared-inputs";
        if !self.declared_watched.is_empty() {
            let unregistration = obj([("id", id.into()), ("method", "workspace/didChangeWatchedFiles".into())]);
            self.request_client("client/unregisterCapability", obj([("unregisterations", Json::Arr(vec![unregistration]))]));
        }
        if !watchers.is_empty() {
            let watchers = Json::Arr(watchers);
            let registration = obj([("id", id.into()), ("method", "workspace/didChangeWatchedFiles".into()), ("registerOptions", obj([("watchers", watchers)]))]);
            self.request_client("client/registerCapability", obj([("registrations", Json::Arr(vec![registration]))]));
        }
        self.declared_watched = globs;
    }

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
                // The fetcher's configuration is the reloaded lock's, whether the session runs or not.
                if let Ok(d) = &description {
                    self.sources.want(&d.sources);
                }
                if description == self.sessions[s].description {
                    continue;
                }
                self.replace_session(s, description);
            }
        }
        if read_any && self.initialized {
            self.register_declared_watchers();
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
        let anchor = if export::is_sbt_build(&root) { export::anchor(&root) } else { crate::task::export::mill_build_file(&root).unwrap_or_else(|| file.to_path_buf()) };
        let (on_file, stale) = match &loaded.export {
            Err(e) => (Some(diagnostic_at_start(e)), Default::default()),
            Ok(export) => (None, export.stale_inputs()),
        };
        match on_file {
            Some(d) => self.folder_diagnostics.insert(file.to_path_buf(), vec![d]),
            None => self.folder_diagnostics.remove(file),
        };
        if !stale.is_empty() {
            let command = match &loaded.export {
                Ok(export) if !stale.line_ends_only() => format!("; run {}", export.exporter.command()),
                _ => String::new(),
            };
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
        self.null_parts(std::mem::take(&mut old.expect), std::mem::take(&mut old.held));
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
        self.drop_parts(s);
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
        self.drop_parts(s);
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
        // The sources of its libraries that no candidate holds, fetched in the background.
        self.sources.want(&description.sources);
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
        session.sent_builds = 0;
        session.typed_builds = 0;
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
        self.drop_parts(s);
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
    /// every file where one was asked for, written behind whatever the child has committed.
    fn send_build(&mut self, s: usize) {
        let session = &mut self.sessions[s];
        if session.proc.is_none() || session.building().is_some() {
            return;
        }
        // The parts whose builds have answered go first, one at a time (`admit`): they read the
        // program of the edits that came before them, not of those this build carries.
        if session.held.front().is_some_and(|h| h.after <= session.typed_builds) {
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
        session.sent_builds += 1;
        session.expect.push_back(Expect::Build(Sent { id, seq: session.sent_builds, sent: Instant::now(), plain, poll, carried, typed, cancelled: false }));
        session.last_build = Instant::now();
    }

    /// The parts of requests a session's child will never answer, now that it is gone or going
    /// (shut down, parked, retired, exited): `null_parts`.
    fn drop_parts(&mut self, s: usize) {
        let expect = std::mem::take(&mut self.sessions[s].expect);
        let held = std::mem::take(&mut self.sessions[s].held);
        self.null_parts(expect, held);
    }

    /// The parts committed to a child that is gone (`expect`, whose builds' and drained answers'
    /// slots go with it) and those held for it: each answered nothing there, its request answered
    /// once its other parts are.
    fn null_parts(&mut self, expect: VecDeque<Expect>, held: VecDeque<Held>) {
        let committed = expect.into_iter().filter_map(|e| match e {
            Expect::Query(q, i) => Some((q, i)),
            Expect::Build(_) | Expect::Discard(_) => None,
        });
        let parts: Vec<(u64, usize)> = committed.chain(held.into_iter().map(|h| (h.q, h.part))).collect();
        for (q, i) in parts {
            self.part_done(q, i, Json::Null);
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

    fn answer(&mut self, s: usize, json: Json) {
        let Some(expect) = self.sessions[s].expect.pop_front() else { return };
        match expect {
            Expect::Build(build) => {
                // The idle period runs from here: a poll slower than `IDLE` is never followed by
                // the next at once.
                self.sessions[s].last_build = Instant::now();
                self.build_answered(s, &json, build);
                self.settle_progress(s);
            }
            Expect::Discard(q) => hooks::trace("server", || format!("drained {}", q)),
            Expect::Query(q, i) => self.part_replied(s, q, i, &json),
        }
        // The session's capacity is free once its answer is in: the next held part goes.
        self.admit(s);
    }

    /// A committed part's answer: checked against the text its position is in, its follow-ups asked
    /// where its target is declared, its result kept.
    fn part_replied(&mut self, s: usize, q: u64, i: usize, json: &Json) {
        hooks::trace("server", || format!("replied {}", q));
        // Live by construction: a request that ends turns its committed slots to `Discard`.
        if !self.pending.contains_key(&q) {
            return;
        }
        // A library document open with another text than its file's answers nothing, whatever its
        // parts found (`join`): no text of theirs is checked, no follow-up asked.
        let changed_document = self.pending[&q].origin.as_deref().is_some_and(|f| self.library_document_changed(f));
        // What the session found the answer in, for a part at a position: a text, by its digest, or
        // nothing, the query declined (a std or a library document whose file holds another text, no
        // program), its answer the declined one (null, the empty list). A text other than the one the
        // position is in names whatever stands at the offset there, which is not the position the
        // client gave; an answer that names neither breaks the session's contract, and may be such
        // an answer: either is refused. A session that typed another text of a file the client has
        // not opened (an edit on disk it has not read yet, or one since) reads the disk again with
        // its next build, which a request asked again finds.
        let part = &self.pending[&q].parts[i];
        let text = json.get("text").and_then(Json::str);
        let declined = json.get("declined").and_then(Json::bool) == Some(true);
        let (refused, other_text) = match (&part.expect, text) {
            _ if changed_document => (false, false),
            (Some(expect), Some(text)) => (expect != text, expect != text),
            (Some(_), None) => (!declined, false),
            (None, _) => (false, false),
        };
        if refused {
            let unopened = other_text && part.file.as_ref().is_some_and(|f| !self.docs.contains_key(f));
            hooks::trace("server", || format!("modified {}", q));
            self.end(q, Err((CONTENT_MODIFIED, "the document changed")));
            if unopened {
                self.plain_build(s);
            }
            return;
        }
        let result = json.get("result").cloned().unwrap_or(Json::Null);
        let declared = json.get("declared").and_then(|d| Some((PathBuf::from(d.get("path")?.str()?), d.get("offset")?.uint()?, d.get("text").and_then(Json::str).map(str::to_string))));
        // The follow-ups before the part is done, so that the request does not end first.
        if let Some((path, offset, text)) = declared.filter(|_| !changed_document) {
            self.follow(q, &path, offset, text);
        }
        self.tell_notes(s, json);
        self.part_done(q, i, result);
    }

    /// What a session reports of the libraries' sources since its last message, carried by every
    /// message it writes and read from every parsed line of every child, before the line meets what it
    /// answers: the sources jars it found damaged at a read (`rejected`, `places::take_rejections`),
    /// which the fetcher's validation, reading no entry's data, cannot see; and the jars whose sources a
    /// navigation found nowhere (`missed`, `places::take_misses`), which the fetcher reads its store
    /// again for.
    fn sources_reported(&mut self, answer: &Json) {
        // The rejections first: a miss that a rejection caused (the only candidate rejected) meets
        // the artifact as the rejection left it.
        for r in answer.get("rejected").map(Json::arr).unwrap_or(&[]) {
            let (Some(path), Some(size)) = (r.get("path").and_then(Json::str), r.get("size").and_then(Json::num)) else { continue };
            let modified = r.get("secs").and_then(Json::num).map(|secs| std::time::UNIX_EPOCH + Duration::new(secs as u64, r.get("nanos").and_then(Json::uint).unwrap_or(0)));
            self.sources.rejected(Path::new(path), (size as u64, modified));
        }
        for jar in answer.get("missed").map(Json::arr).unwrap_or(&[]).iter().filter_map(Json::str) {
            self.sources.missed(Path::new(jar));
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
    /// carried by the next build, whose program the parts held for this one read instead (no query
    /// is ever written behind a build, `admit`); else its diagnostics are the session's, with the
    /// stamps of the texts it typed, and the files whose lists changed are published.
    fn build_answered(&mut self, s: usize, answer: &Json, build: Sent) {
        let echoed = answer.get("build").and_then(Json::num) == Some(build.id as f64);
        if echoed && answer.get("cancelled").and_then(Json::bool) == Some(true) {
            hooks::trace("server", || format!("cancelled {}", build.id));
            let session = &mut self.sessions[s];
            session.carry.extend(build.carried);
            session.carry_plain |= build.plain;
            return;
        }
        // Whatever becomes of its answer, the child's program is the one this build typed, which the
        // parts held for it read (`admit`).
        self.sessions[s].typed_builds = build.seq;
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
            // The change the compiler proposes (an unused import's removal) goes in the `data`
            // the protocol keeps with the diagnostic, which `textDocument/codeAction` reads.
            if let (Some(action), Json::Obj(fields)) = (d.get("action"), &mut diagnostic) {
                fields.push(("data".to_string(), obj([("action", action.clone())])));
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
    /// The declaration's offset is in the text of its file the answering session typed (`text`): a
    /// session asked there must answer on that text.
    fn follow(&mut self, q: u64, path: &Path, offset: u32, text: Option<String>) {
        let Some((verb, extra)) = self.pending.get(&q).and_then(|p| p.follow) else { return };
        // Edits gathered for the sessions asked go now, as a request's do (`ask`).
        self.flush();
        let command = format!("{} {} {}{}\n", verb, path.display(), offset, extra);
        for s in 0..self.sessions.len() {
            let Some(p) = self.pending.get_mut(&q) else { return };
            if p.parts.iter().any(|x| x.s == s) || !(self.sessions[s].owns(path) || self.sessions[s].serves_document(path)) || self.sessions[s].proc.is_none() {
                continue;
            }
            p.parts.push(Part { s, command: command.clone(), expect: text.clone(), file: Some(path.to_path_buf()), state: PartState::Held });
            let i = p.parts.len() - 1;
            self.offer(q, i);
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

    /// A part's result: kept, and the request answered (`join`, `end`) once none of its parts is
    /// held or committed.
    fn part_done(&mut self, q: u64, i: usize, result: Json) {
        let Some(p) = self.pending.get_mut(&q) else { return };
        if p.parts[i].state == PartState::Done {
            return;
        }
        p.parts[i].state = PartState::Done;
        let s = p.parts[i].s;
        // A document asked of every library-reading session remembers its producer from the
        // winning answer alone (`join`); any other answer names the documents it produced.
        if p.document.is_none() {
            self.remember_documents(s, &result);
        }
        let p = self.pending.get_mut(&q).expect("live");
        p.results.push((s, result));
        if p.parts.iter().any(|x| x.state != PartState::Done) {
            return;
        }
        let outcome = self.join(q);
        self.end(q, outcome);
    }

    /// The answer of a request every part of which is done, from its parts' results.
    fn join(&mut self, q: u64) -> Result<Json, (i64, &'static str)> {
        let p = self.pending.get_mut(&q).expect("live");
        let mut results = std::mem::take(&mut p.results);
        // In the sessions' order, whatever order their answers came in: the joined answer is the
        // same for every schedule, `merge` keeping the first session's of two items of one callable.
        results.sort_by_key(|(s, _)| *s);
        let (document, item) = (p.document.take(), p.item.take());
        let (method, query, origin, version) = (p.method.clone(), std::mem::take(&mut p.query), p.origin.clone(), p.version);
        // A document asked of every library-reading session: the first that mapped it, in the
        // sessions' order, answers and is remembered for it and for the documents it named.
        if let Some(path) = document {
            let first = results.iter().find(|(_, r)| !r.is_null()).map(|(s, _)| *s);
            if let Some(s) = first {
                // The producer, unless a live one stands: a retired one is replaced, so that
                // the replacement is the session woken for the document from here on.
                match self.doc_sessions.get(&path) {
                    Some(&o) if !self.sessions[o].retired => {}
                    _ => {
                        self.doc_sessions.insert(path, s);
                    }
                }
                results.retain(|(o, _)| *o == s);
                let winner = results.first().map(|(_, r)| r.clone()).unwrap_or(Json::Null);
                self.remember_documents(s, &winner);
            }
        }
        if origin.as_deref().is_some_and(|f| self.library_document_changed(f)) {
            return Ok(match item {
                Some(item) => item,
                None if method == "textDocument/completion" => obj([("isIncomplete", false.into()), ("items", Json::Arr(Vec::new()))]),
                None => Json::Null,
            });
        }
        let results: Vec<Json> = results.into_iter().map(|(_, r)| r).collect();
        if let Some(item) = item {
            return resolved(item, results.into_iter().next().unwrap_or(Json::Null)).map_err(|why| (CONTENT_MODIFIED, why));
        }
        let mut answer = merge(&method, &query, results);
        if let Some(version) = version {
            name_version(&mut answer, version);
        }
        if method == "textDocument/signatureHelp" && !self.label_offsets {
            label_texts(&mut answer);
        }
        self.respell(&mut answer);
        Ok(answer)
    }
}

/// A signature help's parameter labels as the text each range of its signature's label holds,
/// for a client without `labelOffsetSupport`: the ranges are in UTF-16 units of the label.
fn label_texts(answer: &mut Json) {
    let Some(Json::Arr(signatures)) = answer.get_mut("signatures") else { return };
    for sig in signatures {
        let units: Vec<u16> = sig.get("label").and_then(Json::str).unwrap_or("").encode_utf16().collect();
        let Some(Json::Arr(params)) = sig.get_mut("parameters") else { continue };
        for p in params {
            let Some(Json::Arr(range)) = p.get_mut("label") else { continue };
            let bounds = match range.as_slice() {
                [a, b] => a.uint().zip(b.uint()).map(|(a, b)| (a as usize, b as usize)),
                _ => None,
            };
            let text = match bounds {
                Some((a, b)) if a <= b && b <= units.len() => String::from_utf16_lossy(&units[a..b]),
                _ => String::new(),
            };
            if let Some(label) = p.get_mut("label") {
                *label = Json::Str(text);
            }
        }
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

/// Whether two call hierarchy items (prepared, or the callers and callees of calls) are one, whatever
/// their `data`, compared in place: two sessions' items of one callable are one entry, the first
/// session's in the sessions' order (`join`), and a digest in it of a text since replaced refuses its
/// calls.
fn same_item(a: Option<&Json>, b: Option<&Json>) -> bool {
    match (a, b) {
        (Some(Json::Obj(a)), Some(Json::Obj(b))) => a.iter().filter(|(k, _)| k != "data").eq(b.iter().filter(|(k, _)| k != "data")),
        _ => a == b,
    }
}

/// The answers of several sessions as one: lists joined without repeats (prepared call items by
/// `same_item`), the call hierarchy's
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
    let prepared = method == "textDocument/prepareCallHierarchy";
    for r in results {
        for item in r.arr() {
            if let Some(key) = key_field {
                if let Some(existing) = out.iter_mut().find(|e| same_item(e.get(key), item.get(key))) {
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
            // A prepared item's `data` is its session's (its digest of the item's text), no part of its
            // identity: one entry per callable, the first session's. Other results carry no such data.
            let seen = if prepared { out.iter().any(|e| same_item(Some(e), Some(item))) } else { out.contains(item) };
            if !seen {
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

    /// Two sessions' items of one callable, found in different texts of its file: one entry, the
    /// first session's `data`, prepared or among a call's callers alike.
    #[test]
    fn items_of_one_callable_join_whatever_their_data() {
        let item = |text: &str| Json::parse(&format!(r#"{{"name":"alpha","uri":"file:///w/Api.scala","selectionRange":{{"start":{{"line":1,"character":6}}}},"data":{{"text":"{text}"}}}}"#)).unwrap();
        let joined = merge("textDocument/prepareCallHierarchy", "", vec![Json::Arr(vec![item("a")]), Json::Arr(vec![item("b")])]);
        assert_eq!(joined, Json::Arr(vec![item("a")]));
        let call = |text: &str| obj([("from", item(text)), ("fromRanges", Json::Arr(Vec::new()))]);
        let joined = merge("callHierarchy/incomingCalls", "", vec![Json::Arr(vec![call("a")]), Json::Arr(vec![call("b")])]);
        assert_eq!(joined, Json::Arr(vec![call("a")]));
    }

    /// A Mill build's definition files and the inputs an export declares are build files whose change
    /// refreshes the stale warning, as an sbt build's are: watched by their globs, kept by
    /// `changed_build_files`; a Mill build's `out/` and an sbt build's `target/` are not.
    #[test]
    fn a_mill_builds_files_and_declared_inputs_are_watched() {
        for glob in ["**/*.mill", "**/*.mill.yaml", "**/mill-build/**", "**/.mill-version", "**/teq.mill.json", "**/*.sbt"] {
            assert!(WATCHED.contains(&glob), "{glob}");
        }
        let change = |uri: &str| Json::parse(&format!("{{\"changes\":[{{\"uri\":\"{}\",\"type\":2}}]}}", uri)).unwrap();
        let declared: BTreeSet<PathBuf> = [canonical(Path::new("/w/.tyrian-version"))].into_iter().collect();
        for kept in ["file:///w/build.mill", "file:///w/core/package.mill", "file:///w/core/Deps.mill", "file:///w/build.mill.yaml", "file:///w/mill-build/src/P.scala", "file:///w/.mill-version", "file:///w/teq.mill.json", "file:///w/.tyrian-version", "file:///w/build.sbt", "file:///w/project/P.scala"] {
            assert_eq!(changed_build_files(&change(kept), &declared).len(), 1, "{kept}");
        }
        for dropped in ["file:///w/out/core/package.mill", "file:///w/target/x.sbt", "file:///w/core/src/A.scala", "file:///w/notes.txt", "file:///w/.indigo-version"] {
            assert!(changed_build_files(&change(dropped), &declared).is_empty(), "{dropped}");
        }
    }

    /// A declared input outside the workspace folders (a Mill build's `../external.txt`) is watched from its
    /// own directory, a relative pattern where the client takes one, else its absolute path, one inside them
    /// by its name; and its change touches the export that declares it, though it is outside the build's root.
    #[test]
    fn a_declared_input_outside_the_build_is_watched_from_its_directory_and_touches_its_export() {
        let folders = [PathBuf::from("/w/app")];
        let declared: BTreeSet<PathBuf> = [PathBuf::from("/w/app/.tyrian-version"), PathBuf::from("/w/external.txt")].into_iter().collect();
        let relative = declared_watchers(&declared, &folders, true);
        assert_eq!(relative.len(), 2);
        assert_eq!(relative[0].get("globPattern").and_then(Json::str), Some("**/.tyrian-version"));
        assert_eq!(relative[1].at(&["globPattern", "baseUri"]).and_then(Json::str), Some("file:///w"));
        assert_eq!(relative[1].at(&["globPattern", "pattern"]).and_then(Json::str), Some("external.txt"));
        assert_eq!(declared_watchers(&declared, &folders, false)[1].get("globPattern").and_then(Json::str), Some("/w/external.txt"));
        let declared: Vec<PathBuf> = declared.into_iter().collect();
        assert!(touches(Path::new("/w/app"), &declared, &[PathBuf::from("/w/external.txt")]));
        assert!(touches(Path::new("/w/app"), &[], &[PathBuf::from("/w/app/build.mill")]));
        assert!(!touches(Path::new("/w/app"), &declared, &[PathBuf::from("/w/other.txt")]));
        assert!(!touches(Path::new("/w/other"), &[], &[PathBuf::from("/w/external.txt")]));
    }

    fn session(key: Option<Key>, root: &str) -> Session {
        let description = Description { root: PathBuf::from(root), inputs: Vec::new(), excludes: Vec::new(), classpath: Vec::new(), sources: Vec::new(), jvm_std: None, flags: Vec::new() };
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
        let build = |id, sent, poll| Expect::Build(Sent { id, seq: id, sent, plain: true, poll, carried: Vec::new(), typed: BTreeMap::new(), cancelled: false });
        s.expect.push_back(Expect::Query(1, 0));
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
        s.expect.push_back(Expect::Query(2, 0));
        assert_eq!(s.oldest_build(), Some(first));
        s.expect.pop_back();
        s.generating = Some(0);
        assert_eq!(s.oldest_build(), Some(first));
        s.generating = None;
        s.dirty.insert(PathBuf::from("/w/bare/A.scala"));
        assert_eq!(s.oldest_build(), Some(first));
    }
}
