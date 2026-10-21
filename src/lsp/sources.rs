//! The sources of the libraries the sessions read (docs/TOOLING.md, "The language server and Zed"):
//! once a session starts, each artifact of its class path whose sources no candidate holds (beside
//! the jar, in coursier's cache, in teq's sources store) has its sources artifact fetched from the
//! lock's repository into the store, in the background, by the workers of the sources' own pool of
//! transfers (`fetch::fetch_sources`); the session's child finds it at its next navigation into the
//! jar. Binaries that share a sources artifact (a platform's classifiers of one library) share its
//! transfer and its state, the artifact remembered by its URL for the server's life, and each gets
//! its own copy in the store, from the one transfer or from another's copy. An attempt that fails
//! is made again: none published (404) after 15 minutes, and at the next start of a server; a transport
//! failure, another status or bytes refused after 1, 2, 4 and 8 minutes, then hourly; refused
//! credentials once sbt's credentials file is no longer the one the attempt read (looked at when the
//! attempt ends and when a navigation finds the sources nowhere), else as a transport failure. A
//! held artifact is read again from its store when a navigation finds its sources nowhere: every
//! request into a library document goes through the sessions' lookup (`places::SourcesLookup::find`),
//! which reports such a miss with the session's next message (`Fetcher::missed`). The client's log is told of a transfer's first start and of each state of
//! failure once, when entered, never by a diagnostic or the build's progress. The trace
//! (`hooks.rs`) has a transfer's start, a publication, the end of an attempt with its outcome, and
//! an artifact found held by a candidate, for the tests. An archive of the store that does not open
//! is a failed open: removed, and placed again.

use crate::task::export::LibrarySources;
use crate::task::fetch::{self, SourcesFetched};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime};

/// Why an attempt failed, which says when the next is made.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    /// No sources artifact published.
    Absent,
    /// No answer, an answer cut short, another status.
    Transport,
    /// The credentials refused or wanted.
    Refused,
    /// Bytes that are no archive or not those the repository's checksum names.
    Rejected,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Status {
    /// Waiting for a worker.
    Queued,
    Running,
    /// A candidate of every binary holds the sources: one was there, or an attempt placed them.
    Held,
    /// Made again at `at`, or for refused credentials once the credentials file's modification
    /// time is no longer `credentials`.
    Waiting { kind: Kind, at: Instant, credentials: Option<SystemTime> },
}

/// A binary that reads the sources artifact: its artifact's key, the jar the sessions read, the
/// files its sources are looked for in, and its place in the sources' store
/// (`Export::library_sources`).
#[derive(Clone, PartialEq, Debug)]
struct Consumer {
    key: String,
    jar: PathBuf,
    candidates: Vec<PathBuf>,
    store: PathBuf,
}

struct Artifact {
    url: String,
    /// The host whose credentials the transfer presents.
    credentials: Option<String>,
    consumers: Vec<Consumer>,
    status: Status,
    /// The failed attempts in a row, those of no sources published aside, whatever the status
    /// between them: the backoff's count.
    failures: u32,
    /// The attempts made, the transfers among them started.
    attempts: u32,
    /// Whether the client was told of a transfer, which it is once.
    announced: bool,
    /// The state of failure the client was told of last: each is told once when entered.
    told: Option<Kind>,
    /// A binary joined while an attempt ran, which that attempt may have placed nothing for.
    joined: bool,
    /// The lock changed the artifact's configuration (its credentials host) while an attempt ran
    /// with the one before.
    reconfigured: bool,
    /// Per binary, in `consumers`' order, the candidate the last attempt validated (or placed) with
    /// its stamp: what the artifact's being held stands on.
    holders: Vec<Option<(PathBuf, Stamp)>>,
    /// What of its candidates' content is rejected (`Rejections`).
    rejections: Rejections,
}

/// The rejection record of a sources artifact, one per artifact: its candidates' content found no
/// whole archive or unreadable, by the bytes' hash and by the file's stamp. Written when the
/// validation finds a file no whole archive and when a session's read rejects one
/// (`Fetcher::rejected`). Consulted by `acquire`, the one way an attempt takes a candidate, on each
/// of its paths:
/// - a download, refused when its bytes' hash is recorded (`fetch::SourcesWanted::unreadable`);
/// - a binary's own copy in the store;
/// - another binary's copy, shared: only a candidate `acquire` takes is a copy's source;
/// - a file reopened after a change, its new bytes hashed against the record;
/// - a rejected copy of teq's own whose removal failed, which stays recorded at its stamp.
///
/// Recovery alone clears the hashes, when other bytes are placed. A file still at a recorded stamp
/// stays rejected whatever happens (a sibling teq cannot remove), and the file's presence answers
/// nothing.
#[derive(Clone, Default, Debug)]
struct Rejections {
    /// The sha1s of the bytes a session could not read.
    hashes: BTreeSet<String>,
    /// The files found no whole archive or unreadable, at the stamp they had.
    files: BTreeMap<PathBuf, Stamp>,
    /// The candidates' sha1s by their stamp, read only while some hash is recorded.
    hashed: BTreeMap<PathBuf, (Stamp, String)>,
}

impl Rejections {
    /// Whether the candidate `path` at `stamp` is rejected content: recorded at that stamp, or bytes
    /// whose hash is recorded (the file hashed once per stamp, and only while some hash is). One
    /// found so by its hash is recorded at its stamp.
    fn rejects(&mut self, path: &Path, stamp: Stamp) -> bool {
        if self.files.get(path) == Some(&stamp) {
            return true;
        }
        if self.hashes.is_empty() {
            return false;
        }
        let hash = match self.hashed.get(path) {
            Some((at, hash)) if *at == stamp => Some(hash.clone()),
            _ => fetch::sha1_file(path).ok(),
        };
        let Some(hash) = hash else { return false };
        self.hashed.insert(path.to_path_buf(), (stamp, hash.clone()));
        let rejected = self.hashes.contains(&hash);
        if rejected {
            self.files.insert(path.to_path_buf(), stamp);
        }
        rejected
    }

    /// A file found no whole archive or unreadable at `stamp`, with its bytes' hash where a read
    /// rejected it.
    fn record(&mut self, path: &Path, stamp: Stamp, hash: Option<String>) {
        self.files.insert(path.to_path_buf(), stamp);
        self.hashes.extend(hash);
    }

    /// A file of teq's own removed or placed over: its entry forgotten, its bytes' hash kept.
    fn forget(&mut self, path: &Path) {
        self.files.remove(path);
        self.hashed.remove(path);
    }

    /// Other bytes placed, the recovery: the hashes cleared.
    fn recovered(&mut self) {
        self.hashes.clear();
    }

    /// What an attempt read (`after`, from the snapshot `before`) with what was recorded meanwhile.
    fn merged(&self, before: &Rejections, mut after: Rejections) -> Rejections {
        for (path, stamp) in &self.files {
            if before.files.get(path) != Some(stamp) {
                after.files.insert(path.clone(), *stamp);
            }
        }
        after.hashes.extend(self.hashes.iter().cloned());
        after
    }
}

/// A file as it was read: its size and modification time.
type Stamp = (u64, Option<SystemTime>);

/// A file's stamp now; none where no file is.
fn stamp_of(path: &Path) -> Option<Stamp> {
    std::fs::metadata(path).ok().filter(|m| m.is_file()).map(|m| (m.len(), m.modified().ok()))
}

impl Artifact {
    /// The key the client is told the artifact by: its first binary's.
    fn key(&self) -> &str {
        self.consumers.first().map_or(self.url.as_str(), |c| c.key.as_str())
    }
}

#[derive(Default)]
struct Table {
    artifacts: BTreeMap<String, Artifact>,
    queue: VecDeque<String>,
    running: usize,
    stopped: bool,
}

struct Shared {
    table: Mutex<Table>,
    /// Told when the table changes: an artifact queued, an attempt ended, the server stopping.
    changed: Condvar,
    cancel: AtomicBool,
}

/// The server's fetcher of sources, its workers started with the first artifact wanted.
pub(crate) struct Fetcher {
    shared: Arc<Shared>,
    started: bool,
}

/// The length of the backoff's minute: `TEQ_LSP_SOURCES_MINUTE` milliseconds for the tests, which
/// would wait minutes for a retry otherwise.
fn minute() -> Duration {
    static MINUTE: std::sync::OnceLock<Duration> = std::sync::OnceLock::new();
    *MINUTE.get_or_init(|| std::env::var("TEQ_LSP_SOURCES_MINUTE").ok().and_then(|v| v.parse().ok()).map_or(Duration::from_secs(60), Duration::from_millis))
}

/// How long after its `failures`-th failure in a row an attempt that failed so waits.
fn retry_after(kind: Kind, failures: u32, minute: Duration) -> Duration {
    match kind {
        Kind::Absent => minute * 15,
        Kind::Transport | Kind::Refused | Kind::Rejected => match failures {
            0 | 1 => minute,
            2 => minute * 2,
            3 => minute * 4,
            4 => minute * 8,
            _ => minute * 60,
        },
    }
}

/// A session's libraries entered in the table, as the lock gives them now (a session's start, a
/// reload of its lock): a sources artifact not yet known queued; a binary joining a known one added
/// to it, queued again when it was held (a binary new to it gets its copy, one whose copy is gone
/// gets it again) and marked when an attempt runs; one waiting keeps its time. The artifact's
/// configuration is the lock's of this call, whatever its first record said: credentials for its
/// host named or no longer named make one waiting due at once (its failure was the old one's), and
/// one running due once it ends. Whether a worker has something to do.
fn enter(table: &mut Table, sources: &[LibrarySources], now: Instant) -> bool {
    let mut changed = false;
    for s in sources {
        let Some(fetch) = &s.fetch else { continue };
        let consumer = Consumer { key: fetch.key.clone(), jar: s.jar.clone(), candidates: s.candidates.clone(), store: fetch.store.clone() };
        match table.artifacts.get_mut(&fetch.url) {
            None => {
                let artifact = Artifact {
                    url: fetch.url.clone(),
                    credentials: fetch.credentials.clone(),
                    consumers: vec![consumer],
                    status: Status::Queued,
                    failures: 0,
                    attempts: 0,
                    announced: false,
                    told: None,
                    joined: false,
                    reconfigured: false,
                    holders: vec![None],
                    rejections: Rejections::default(),
                };
                table.artifacts.insert(fetch.url.clone(), artifact);
            }
            Some(a) => {
                let new = !a.consumers.contains(&consumer);
                if new {
                    a.consumers.push(consumer);
                    a.holders.push(None);
                }
                let reconfigured = a.credentials != fetch.credentials;
                if reconfigured {
                    a.credentials = fetch.credentials.clone();
                }
                match &mut a.status {
                    Status::Held => a.status = Status::Queued,
                    Status::Running => {
                        a.joined |= new;
                        a.reconfigured |= reconfigured;
                        continue;
                    }
                    Status::Waiting { at, .. } if reconfigured => {
                        *at = now;
                        changed = true;
                        continue;
                    }
                    Status::Queued | Status::Waiting { .. } => continue,
                }
            }
        }
        table.queue.push_back(fetch.url.clone());
        changed = true;
    }
    changed
}

/// An artifact whose attempt ended, as the table changed meanwhile: a binary that joined gets the
/// held artifact queued again for its copy; a configuration the lock changed makes a failed attempt's
/// next one due at once. Whether it is queued again.
fn after_attempt(a: &mut Artifact, now: Instant) -> bool {
    if a.reconfigured {
        a.reconfigured = false;
        if let Status::Waiting { at, .. } = &mut a.status {
            *at = now;
        }
    }
    let again = a.joined && a.status == Status::Held;
    if again {
        a.status = Status::Queued;
    }
    again
}

/// A session's report that a navigation into the jar `jar` found its sources nowhere
/// (`places::SourcesLookup::find`, which every request into a library document goes through): the
/// artifact read again from what it stands on. A held one is queued for an attempt, which acquires
/// its candidates through the record again and fetches when none stands (its copy removed,
/// truncated, rejected); one whose credentials were refused is tried again at once when sbt's
/// credentials file (`credentials`, its modification time now) changed since; one waiting otherwise
/// keeps its time. Whether a worker has something to do.
fn note_miss(table: &mut Table, jar: &Path, credentials: Option<SystemTime>, now: Instant) -> bool {
    let mut changed = false;
    let mut queued = Vec::new();
    for a in table.artifacts.values_mut() {
        if !a.consumers.iter().any(|c| c.jar == jar) {
            continue;
        }
        match &mut a.status {
            Status::Held => {
                a.status = Status::Queued;
                queued.push(a.url.clone());
                changed = true;
            }
            Status::Waiting { kind: Kind::Refused, credentials: read, at } if *read != credentials => {
                *read = credentials;
                *at = now;
                changed = true;
            }
            Status::Queued | Status::Running | Status::Waiting { .. } => {}
        }
    }
    table.queue.extend(queued);
    changed
}

/// A session's report that the sources jar `path`, at `stamp`, does not read (an entry's CRC or size
/// not its directory's), which the fetcher's validation, reading no entry's data, cannot see: every
/// artifact with the file among its candidates records it, at that stamp and by its bytes' hash
/// (`Rejections`). teq's own copy in the store is removed, its publication a failed attempt
/// (`Rejected`, its backoff and its one line to the client), so a repaired repository is fetched,
/// and its bytes are never acquired again; another candidate held makes the artifact
/// queued, its next attempt passing over the file. A file changed since the read is the next
/// attempt's to read. What the client is told, and whether a worker has something to do.
fn reject_candidate(table: &mut Table, path: &Path, stamp: Stamp, sha1: Option<String>, now: Instant, minute: Duration) -> (Vec<(u32, String)>, bool) {
    let mut told = Vec::new();
    let mut changed = false;
    if stamp_of(path) != Some(stamp) {
        return (told, changed);
    }
    let mut queued = Vec::new();
    for a in table.artifacts.values_mut() {
        if !a.consumers.iter().any(|c| c.candidates.iter().any(|p| p == path)) {
            continue;
        }
        a.rejections.record(path, stamp, sha1.clone());
        changed = true;
        super::hooks::trace("server", || format!("sources-rejected {} {}", a.attempts, a.url));
        let ours = a.consumers.iter().any(|c| c.store == path);
        match a.status {
            Status::Running => a.joined = true,
            _ if ours => {
                // Still the archive read: a copy placed again meanwhile is not removed. One that
                // cannot be removed stays recorded, never taken; the next attempt meets its error.
                if stamp_of(path) == Some(stamp) && std::fs::remove_file(path).is_ok() {
                    a.rejections.forget(path);
                }
                let said = format!("{} does not read: an entry's CRC or size is not its directory's", path.display());
                told.extend(settle(a, &SourcesFetched::Rejected(said), now, minute, None, None, ""));
            }
            Status::Held => {
                a.status = Status::Queued;
                queued.push(a.url.clone());
            }
            Status::Queued | Status::Waiting { .. } => {}
        }
    }
    table.queue.extend(queued);
    (told, changed)
}

/// The artifacts whose wait is over, queued again.
fn queue_due(table: &mut Table, now: Instant) {
    let due: Vec<String> = table.artifacts.iter().filter(|(_, a)| matches!(a.status, Status::Waiting { at, .. } if at <= now)).map(|(url, _)| url.clone()).collect();
    for url in due {
        if let Some(a) = table.artifacts.get_mut(&url) {
            a.status = Status::Queued;
        }
        table.queue.push_back(url);
    }
}

/// What an attempt reads of its artifact.
struct Attempt {
    url: String,
    credentials: Option<String>,
    consumers: Vec<Consumer>,
    rejections: Rejections,
    number: u32,
    announce: bool,
}

/// A queued artifact taken by a worker: running, its attempt counted.
fn begin(a: &mut Artifact) -> Attempt {
    a.status = Status::Running;
    a.attempts += 1;
    a.joined = false;
    a.reconfigured = false;
    Attempt { url: a.url.clone(), credentials: a.credentials.clone(), consumers: a.consumers.clone(), rejections: a.rejections.clone(), number: a.attempts, announce: !a.announced }
}

/// Settles an artifact after an attempt: its next status, and what the client is told, its
/// message's type and text (an attempt that fails as the last did is told nothing). `read` is the
/// modification time of the credentials file the attempt read, `current` the file's now: refused
/// credentials repaired while the request was pending are tried again at once.
#[allow(clippy::too_many_arguments)]
fn settle(a: &mut Artifact, outcome: &SourcesFetched, now: Instant, minute: Duration, read: Option<SystemTime>, current: Option<SystemTime>, credentials_file: &str) -> Option<(u32, String)> {
    let key = a.key().to_string();
    let (kind, said) = match outcome {
        SourcesFetched::Published => {
            a.status = Status::Held;
            a.failures = 0;
            a.rejections.recovered();
            return a.told.take().map(|_| (3, format!("teq: fetched the sources of {}", key)));
        }
        SourcesFetched::Cancelled => {
            a.status = Status::Queued;
            return None;
        }
        SourcesFetched::Absent(said) => (Kind::Absent, said),
        SourcesFetched::Failed(said) => (Kind::Transport, said),
        SourcesFetched::Refused(said) => (Kind::Refused, said),
        SourcesFetched::Rejected(said) => (Kind::Rejected, said),
    };
    a.failures = if kind == Kind::Absent { 0 } else { a.failures + 1 };
    let repaired = kind == Kind::Refused && read != current;
    let at = if repaired { now } else { now + retry_after(kind, a.failures, minute) };
    a.status = Status::Waiting { kind, at, credentials: (kind == Kind::Refused).then_some(read).flatten() };
    if a.told == Some(kind) {
        return None;
    }
    a.told = Some(kind);
    let backoff = "after 1, 2, 4 and 8 minutes, then hourly";
    Some(match kind {
        Kind::Absent => (3, format!("teq: no sources published for {}: {}; tried again in 15 minutes and when the language server starts again", key, said)),
        Kind::Transport => (2, format!("teq: cannot fetch the sources of {}: {}; tried again {}", key, said, backoff)),
        Kind::Refused => (2, format!("teq: cannot fetch the sources of {}, the credentials refused: {}; tried again when {} changes, else {}", key, said, credentials_file, backoff)),
        Kind::Rejected => (2, format!("teq: the sources of {} are not placed: {}; tried again {}", key, said, backoff)),
    })
}

/// The outcome's word in the trace.
fn outcome_word(outcome: &SourcesFetched) -> &'static str {
    match outcome {
        SourcesFetched::Published => "published",
        SourcesFetched::Absent(_) => "absent",
        SourcesFetched::Refused(_) => "refused",
        SourcesFetched::Failed(_) => "transport",
        SourcesFetched::Rejected(_) => "rejected",
        SourcesFetched::Cancelled => "cancelled",
    }
}

fn lock(table: &Mutex<Table>) -> MutexGuard<'_, Table> {
    table.lock().unwrap_or_else(|e| e.into_inner())
}

fn tell(told: Option<(u32, String)>) {
    if let Some((kind, message)) = told {
        super::notify("window/logMessage", super::json::obj([("type", kind.into()), ("message", message.into())]));
    }
}

impl Fetcher {
    pub(crate) fn new() -> Fetcher {
        Fetcher { shared: Arc::new(Shared { table: Mutex::new(Table::default()), changed: Condvar::new(), cancel: AtomicBool::new(false) }), started: false }
    }

    /// A session's libraries (`enter`), the workers started with the first.
    pub(crate) fn want(&mut self, sources: &[LibrarySources]) {
        let mut table = lock(&self.shared.table);
        if table.stopped || !enter(&mut table, sources, Instant::now()) {
            return;
        }
        drop(table);
        if !self.started {
            self.started = true;
            for _ in 0..fetch::MAX_SOURCE_TRANSFERS {
                let shared = self.shared.clone();
                crate::alloc::spawn(move || work(&shared));
            }
        }
        self.shared.changed.notify_all();
    }

    /// A session's report that a navigation found the sources of `jar` nowhere (`note_miss`).
    pub(crate) fn missed(&self, jar: &Path) {
        let credentials = fetch::credentials_stamp();
        let mut table = lock(&self.shared.table);
        if table.stopped {
            return;
        }
        let changed = note_miss(&mut table, jar, credentials, Instant::now());
        drop(table);
        if changed {
            self.shared.changed.notify_all();
        }
    }

    /// A session's report of a sources jar it found damaged at a read (`reject_candidate`).
    pub(crate) fn rejected(&self, path: &Path, stamp: Stamp) {
        // The bytes read, off the table's lock, while the file is the archive the session read.
        let sha1 = (stamp_of(path) == Some(stamp)).then(|| fetch::sha1_file(path).ok()).flatten();
        let mut table = lock(&self.shared.table);
        if table.stopped {
            return;
        }
        let (told, changed) = reject_candidate(&mut table, path, stamp, sha1, Instant::now(), minute());
        drop(table);
        if changed {
            self.shared.changed.notify_all();
        }
        for t in told {
            tell(Some(t));
        }
    }

    /// The server's end: the transfers under way cancelled (their curl killed, their partial files
    /// removed) before it exits, a few seconds at most; none starts after.
    pub(crate) fn stop(&self) {
        let mut table = lock(&self.shared.table);
        table.stopped = true;
        self.shared.cancel.store(true, Ordering::Relaxed);
        self.shared.changed.notify_all();
        let until = Instant::now() + super::QUIT_BOUND;
        while table.running > 0 {
            let now = Instant::now();
            if now >= until {
                break;
            }
            table = self.shared.changed.wait_timeout(table, until - now).unwrap_or_else(|e| e.into_inner()).0;
        }
    }
}

/// A worker: the queued artifacts in order, those whose wait is over queued again, until the
/// server stops.
fn work(shared: &Shared) {
    let mut table = lock(&shared.table);
    loop {
        if table.stopped {
            return;
        }
        let now = Instant::now();
        queue_due(&mut table, now);
        let Some(url) = table.queue.pop_front() else {
            let next = table.artifacts.values().filter_map(|a| if let Status::Waiting { at, .. } = a.status { Some(at) } else { None }).min();
            table = match next {
                Some(at) => shared.changed.wait_timeout(table, at.saturating_duration_since(now)).unwrap_or_else(|e| e.into_inner()).0,
                None => shared.changed.wait(table).unwrap_or_else(|e| e.into_inner()),
            };
            continue;
        };
        let Some(a) = table.artifacts.get_mut(&url) else { continue };
        if a.status != Status::Queued {
            continue;
        }
        let attempt = begin(a);
        table.running += 1;
        drop(table);
        let (outcome, reading) = attempt_once(shared, &attempt);
        let current = fetch::credentials_stamp();
        let shown = fetch::credentials_file().map_or("~/.sbt/.credentials".to_string(), |f| f.display().to_string());
        table = lock(&shared.table);
        table.running -= 1;
        let mut again = false;
        let told = match table.artifacts.get_mut(&url) {
            Some(a) => {
                // What the attempt read of the files, a session's rejection meanwhile kept.
                a.rejections = a.rejections.merged(&attempt.rejections, reading.rejections);
                for (i, holder) in reading.holders.into_iter().enumerate() {
                    a.holders[i] = holder;
                }
                let told = match outcome {
                    Some(mut tried) => {
                        a.announced |= attempt.announce && tried.started;
                        if let Some(placed) = &reading.placed {
                            if let Some(withdrawn) = withdraw(a, placed, &attempt.url) {
                                tried.outcome = withdrawn;
                            }
                        }
                        if tried.outcome == SourcesFetched::Published {
                            super::hooks::trace("server", || format!("sources-published {} {}", attempt.number, attempt.url));
                        }
                        super::hooks::trace("server", || format!("sources-released {} {} {}", attempt.number, attempt.url, outcome_word(&tried.outcome)));
                        settle(a, &tried.outcome, Instant::now(), minute(), tried.credentials, current, &shown)
                    }
                    None => {
                        // The hold settled against the record as it is now: a holder rejected while
                        // the attempt ran leaves the artifact to another attempt.
                        a.status = if holds(a) { Status::Held } else { Status::Queued };
                        again = a.status == Status::Queued;
                        None
                    }
                };
                again |= after_attempt(a, Instant::now());
                told
            }
            None => None,
        };
        if again {
            table.queue.push_back(url);
        }
        shared.changed.notify_all();
        drop(table);
        tell(told);
        table = lock(&shared.table);
    }
}

/// What an attempt made: its outcome, whether a transfer started, and the modification time of the
/// credentials file as the transfer read it.
struct Tried {
    outcome: SourcesFetched,
    started: bool,
    credentials: Option<SystemTime>,
}

/// What an attempt read of the files: per binary the candidate it acquired or placed, with its
/// stamp, and the rejection record as it left it.
struct Reading {
    holders: Vec<Option<(PathBuf, Stamp)>>,
    rejections: Rejections,
    /// What a publication placed, settled against the record as it is at the settle (`withdraw`).
    placed: Option<Placed>,
}

/// The bytes a publication placed: their sha1, and the binaries' places in the store (by index in
/// `consumers`) that hold them now.
struct Placed {
    sha1: Option<String>,
    stores: Vec<(usize, PathBuf)>,
}

/// Whether every binary's holder stands against the record as it is at the settle (a session's
/// rejection made while the attempt ran included); one that does not is cleared.
fn holds(a: &mut Artifact) -> bool {
    let mut all = true;
    for holder in a.holders.iter_mut() {
        match holder {
            Some((path, stamp)) if !a.rejections.rejects(path, *stamp) => {}
            _ => {
                *holder = None;
                all = false;
            }
        }
    }
    all
}

/// A publication settled against the record as it is at the settle, a session's rejection made
/// while the attempt ran included: bytes the record rejects are not published and not held. Each
/// place that took them is removed (one that cannot be is recorded at its stamp, never taken), the
/// binary holds nothing, and the outcome becomes a refused archive, the failure history kept and
/// the reacquisition it calls for run at its time. None when the bytes stand.
fn withdraw(a: &mut Artifact, placed: &Placed, url: &str) -> Option<SourcesFetched> {
    let sha1 = placed.sha1.as_ref()?;
    if !a.rejections.hashes.contains(sha1) {
        return None;
    }
    for (i, store) in &placed.stores {
        let stamp = stamp_of(store);
        match std::fs::remove_file(store) {
            Ok(()) => a.rejections.forget(store),
            Err(_) => {
                if let Some(stamp) = stamp {
                    a.rejections.record(store, stamp, None);
                }
            }
        }
        a.holders[*i] = None;
    }
    Some(SourcesFetched::Rejected(format!("{} brought the bytes a session could not read while it was fetched (sha1 {})", url, sha1)))
}

/// The one way an attempt takes a candidate (`Rejections` lists its paths): the stamp of `path` when
/// it holds no rejected content and a whole archive (`fetch::valid_archive`), as read now; one found
/// no whole archive is recorded at its stamp.
fn acquire(path: &Path, rejections: &mut Rejections) -> Option<Stamp> {
    let stamp = stamp_of(path)?;
    if rejections.rejects(path, stamp) {
        return None;
    }
    if fetch::valid_archive(path) {
        rejections.files.remove(path);
        Some(stamp)
    } else {
        rejections.record(path, stamp, None);
        None
    }
}

/// One attempt at a sources artifact: none when every binary acquires a candidate already; else
/// what it made (`Tried`); and always what it read (`Reading`). A binary's copy in the store that is
/// not acquired (no whole archive, rejected content) is removed first, a failed open; one that
/// cannot be removed ends the attempt as a failure, its error told, the copy staying recorded and
/// never held. The binaries that acquire none get a copy of one another acquired, or of the one
/// transfer into the first's place (another of the process may place it meanwhile). The transfer's
/// start is told to the trace and, the first time, to the client.
fn attempt_once(shared: &Shared, attempt: &Attempt) -> (Option<Tried>, Reading) {
    let url = &attempt.url;
    let mut rejections = attempt.rejections.clone();
    let begun = std::cell::Cell::new(false);
    let read = std::cell::Cell::new(None);
    // The outcome's trace is the settle's, against the record as it is then (`withdraw`).
    let ended = |outcome: SourcesFetched, holders: Vec<Option<(PathBuf, Stamp)>>, rejections: Rejections, placed: Option<Placed>| {
        (Some(Tried { outcome, started: begun.get(), credentials: read.get() }), Reading { holders, rejections, placed })
    };
    for c in &attempt.consumers {
        if stamp_of(&c.store).is_some() && acquire(&c.store, &mut rejections).is_none() {
            match std::fs::remove_file(&c.store) {
                Ok(()) => rejections.forget(&c.store),
                Err(e) => {
                    let failed = SourcesFetched::Failed(format!("cannot remove {}, which is no readable archive: {}", c.store.display(), e));
                    return ended(failed, vec![None; attempt.consumers.len()], rejections, None);
                }
            }
        }
    }
    let mut holders: Vec<Option<(PathBuf, Stamp)>> = Vec::new();
    for c in &attempt.consumers {
        let found = c.candidates.iter().find_map(|p| acquire(p, &mut rejections).map(|stamp| (p.clone(), stamp)));
        holders.push(found);
    }
    let missing: Vec<usize> = (0..holders.len()).filter(|&i| holders[i].is_none()).collect();
    if missing.is_empty() {
        super::hooks::trace("server", || format!("sources-held 0 {}", url));
        return (None, Reading { holders, rejections, placed: None });
    }
    let source = match holders.iter().flatten().next() {
        Some((other, _)) => other.clone(),
        None => {
            let first = &attempt.consumers[missing[0]];
            let wanted = fetch::SourcesWanted { url, credentials: attempt.credentials.as_deref(), file: &first.store, unreadable: &rejections.hashes };
            // Told as the transfer starts, before it reads the credentials file.
            let started = || {
                begun.set(true);
                read.set(fetch::credentials_stamp());
                super::hooks::trace("server", || format!("sources-start {} {}", attempt.number, url));
                if attempt.announce {
                    tell(Some((4, format!("teq: fetching sources of {}", first.key))));
                }
            };
            let outcome = fetch::fetch_sources(&wanted, &shared.cancel, &started);
            if outcome != SourcesFetched::Published {
                return ended(outcome, holders, rejections, None);
            }
            first.store.clone()
        }
    };
    let mut stores = Vec::new();
    for i in missing {
        let c = &attempt.consumers[i];
        if c.store != source && stamp_of(&c.store).is_none() {
            let placed = fetch::publish_copy(&source, &c.store);
            if placed != SourcesFetched::Published {
                return ended(placed, holders, rejections, None);
            }
        }
        rejections.forget(&c.store);
        holders[i] = stamp_of(&c.store).map(|stamp| (c.store.clone(), stamp));
        stores.push((i, c.store.clone()));
    }
    // The bytes placed, for the settle to hold them to the record as it is then.
    let placed = Placed { sha1: fetch::sha1_file(&source).ok(), stores };
    ended(SourcesFetched::Published, holders, rejections, Some(placed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::export::SourcesFetch;

    fn sources(key: &str, url: &str, store: &str) -> LibrarySources {
        let fetch = SourcesFetch { key: key.into(), url: url.into(), credentials: None, store: PathBuf::from(store) };
        LibrarySources { jar: PathBuf::from(format!("/c/{key}.jar")), candidates: vec![PathBuf::from(store)], fetch: Some(fetch) }
    }

    const URL: &str = "https://repo.example/org/x_3/1.0/x_3-1.0-sources.jar";

    /// A table of one artifact, taken by a worker for an attempt as `work` takes it.
    fn running() -> Table {
        let mut table = Table::default();
        enter(&mut table, &[sources("org:x_3:1.0", URL, "/c/sources/s/x_3-1.0-sources.jar")], Instant::now());
        take(&mut table);
        table
    }

    /// The next queued artifact begun, as a worker begins it.
    fn take(table: &mut Table) {
        let url = table.queue.pop_front().expect("an artifact queued");
        begin(table.artifacts.get_mut(&url).unwrap());
    }

    fn waits(a: &Artifact, now: Instant) -> Option<(Kind, u32, Duration)> {
        match a.status {
            Status::Waiting { kind, at, .. } => Some((kind, a.failures, at - now)),
            _ => None,
        }
    }

    #[test]
    fn binaries_sharing_a_sources_artifact_share_its_transfer() {
        let mut table = Table::default();
        let linux = sources("org:x:1.0:linux-x86_64", URL, "/c/sources/a/x-1.0-sources.jar");
        let osx = sources("org:x:1.0:osx-aarch_64", URL, "/c/sources/b/x-1.0-sources.jar");
        assert!(enter(&mut table, &[linux.clone()], Instant::now()));
        // Joining while the attempt runs: marked, queued again once that attempt places its archive.
        take(&mut table);
        assert!(!enter(&mut table, &[osx.clone(), linux.clone()], Instant::now()));
        let a = table.artifacts.get_mut(URL).unwrap();
        assert_eq!((a.consumers.len(), a.joined), (2, true));
        settle(a, &SourcesFetched::Published, Instant::now(), Duration::from_secs(60), None, None, "c");
        assert_eq!(a.status, Status::Held);
        // A session started again over a held artifact looks at it again, with no new binary.
        assert!(enter(&mut table, &[osx], Instant::now()));
        assert_eq!((table.artifacts.len(), table.queue.len(), table.artifacts[URL].consumers.len()), (1, 1, 2));
    }

    #[test]
    fn none_published_waits_a_quarter_hour() {
        let (now, m) = (Instant::now(), Duration::from_secs(60));
        let mut table = running();
        let a = table.artifacts.get_mut(URL).unwrap();
        let told = settle(a, &SourcesFetched::Absent("GET answered 404".into()), now, m, None, None, "c");
        assert_eq!(waits(a, now), Some((Kind::Absent, 0, m * 15)));
        assert!(told.is_some_and(|(t, msg)| t == 3 && msg.starts_with("teq: no sources published for org:x_3:1.0: GET answered 404")));
        // Again none, its wait over and the attempt made: nothing told, the same wait.
        queue_due(&mut table, now + m * 15);
        take(&mut table);
        let a = table.artifacts.get_mut(URL).unwrap();
        assert_eq!(settle(a, &SourcesFetched::Absent("GET answered 404".into()), now, m, None, None, "c"), None);
        assert_eq!(waits(a, now), Some((Kind::Absent, 0, m * 15)));
        // Then a transport failure: another state, told, its backoff from its start.
        queue_due(&mut table, now + m * 15);
        take(&mut table);
        let a = table.artifacts.get_mut(URL).unwrap();
        assert!(settle(a, &SourcesFetched::Failed("cut".into()), now, m, None, None, "c").is_some());
        assert_eq!(waits(a, now), Some((Kind::Transport, 1, m)));
    }

    #[test]
    fn refused_credentials_wait_for_the_file_to_change() {
        let (now, m) = (Instant::now(), Duration::from_secs(60));
        let stamp = SystemTime::UNIX_EPOCH + Duration::from_secs(1000);
        let mut table = running();
        let a = table.artifacts.get_mut(URL).unwrap();
        let told = settle(a, &SourcesFetched::Refused("GET answered 401".into()), now, m, Some(stamp), Some(stamp), "/h/.sbt/.credentials");
        assert!(matches!(a.status, Status::Waiting { kind: Kind::Refused, credentials: Some(s), .. } if s == stamp));
        assert!(told.is_some_and(|(t, msg)| t == 2 && msg.contains("when /h/.sbt/.credentials changes")));
        // As a transport failure otherwise, and a rejected archive too, the count kept across the
        // queue and the attempt between them.
        queue_due(&mut table, now + m);
        take(&mut table);
        let a = table.artifacts.get_mut(URL).unwrap();
        settle(a, &SourcesFetched::Refused("GET answered 401".into()), now, m, Some(stamp), Some(stamp), "c");
        assert_eq!(waits(a, now), Some((Kind::Refused, 2, m * 2)));
        queue_due(&mut table, now + m * 2);
        take(&mut table);
        let a = table.artifacts.get_mut(URL).unwrap();
        settle(a, &SourcesFetched::Rejected("no zip".into()), now, m, None, None, "c");
        assert_eq!(waits(a, now), Some((Kind::Rejected, 3, m * 4)));
    }

    /// Credentials repaired while the refused request was pending: the file the attempt read is no
    /// longer the one there, so the next attempt is due at once, the failure counted.
    #[test]
    fn credentials_repaired_during_the_request_are_tried_at_once() {
        let (now, m) = (Instant::now(), Duration::from_secs(60));
        let (read, repaired) = (SystemTime::UNIX_EPOCH + Duration::from_secs(1000), SystemTime::UNIX_EPOCH + Duration::from_secs(2000));
        let mut table = running();
        let a = table.artifacts.get_mut(URL).unwrap();
        settle(a, &SourcesFetched::Refused("GET answered 401".into()), now, m, Some(read), Some(repaired), "c");
        assert_eq!(waits(a, now), Some((Kind::Refused, 1, Duration::ZERO)));
        assert!(matches!(a.status, Status::Waiting { credentials: Some(s), .. } if s == read));
        // The file created while the request was pending, where there was none.
        queue_due(&mut table, now);
        take(&mut table);
        let a = table.artifacts.get_mut(URL).unwrap();
        settle(a, &SourcesFetched::Refused("GET answered 401".into()), now, m, None, Some(repaired), "c");
        assert_eq!(waits(a, now), Some((Kind::Refused, 2, Duration::ZERO)));
    }

    /// The lock names the repository's host for credentials after an attempt was refused without
    /// them: the artifact takes the lock's configuration of the reload, and its next attempt is due at
    /// once; changed while an attempt runs, the attempt's failure is followed at once too.
    #[test]
    fn the_locks_reload_reconfigures_an_artifact() {
        let (now, m) = (Instant::now(), Duration::from_secs(60));
        let mut table = running();
        let a = table.artifacts.get_mut(URL).unwrap();
        settle(a, &SourcesFetched::Refused("GET answered 401".into()), now, m, None, None, "c");
        assert_eq!(waits(a, now), Some((Kind::Refused, 1, m)));
        let mut named = sources("org:x_3:1.0", URL, "/c/sources/s/x_3-1.0-sources.jar");
        named.fetch.as_mut().unwrap().credentials = Some("repo.example".into());
        let reload = now + Duration::from_secs(5);
        assert!(enter(&mut table, &[named.clone()], reload));
        let a = &table.artifacts[URL];
        assert_eq!(a.credentials.as_deref(), Some("repo.example"));
        assert!(matches!(a.status, Status::Waiting { at, .. } if at == reload));
        // The same lock again changes nothing: the wait stays.
        assert!(!enter(&mut table, &[named.clone()], reload + m));
        // Reconfigured while an attempt runs: its failure's next attempt due at once.
        queue_due(&mut table, reload);
        take(&mut table);
        let unnamed = sources("org:x_3:1.0", URL, "/c/sources/s/x_3-1.0-sources.jar");
        assert!(!enter(&mut table, &[unnamed], reload));
        let a = table.artifacts.get_mut(URL).unwrap();
        assert!(a.reconfigured && a.credentials.is_none());
        let ended = reload + Duration::from_secs(1);
        settle(a, &SourcesFetched::Refused("GET answered 401".into()), ended, m, None, None, "c");
        assert!(!after_attempt(a, ended));
        assert!(matches!(a.status, Status::Waiting { at, .. } if at == ended) && !a.reconfigured);
    }

    /// A navigation that finds a jar's sources nowhere: its held artifact queued for an attempt (which
    /// reads the store again), one refused for its credentials due at once when the file changed since,
    /// one waiting otherwise left to its time, another jar's untouched.
    #[test]
    fn a_navigation_finding_nothing_reads_the_store_again() {
        let (now, m) = (Instant::now(), Duration::from_secs(60));
        let jar = PathBuf::from("/c/org:x_3:1.0.jar");
        let mut table = running();
        table.artifacts.get_mut(URL).unwrap().status = Status::Held;
        assert!(!note_miss(&mut table, Path::new("/c/other.jar"), None, now));
        assert!(note_miss(&mut table, &jar, None, now));
        assert_eq!((table.artifacts[URL].status, table.queue.len()), (Status::Queued, 1));
        take(&mut table);
        let (read, repaired) = (SystemTime::UNIX_EPOCH + Duration::from_secs(1000), SystemTime::UNIX_EPOCH + Duration::from_secs(2000));
        let a = table.artifacts.get_mut(URL).unwrap();
        settle(a, &SourcesFetched::Refused("GET answered 401".into()), now, m, Some(read), Some(read), "c");
        let later = now + Duration::from_secs(5);
        assert!(!note_miss(&mut table, &jar, Some(read), later));
        assert!(note_miss(&mut table, &jar, Some(repaired), later));
        assert!(matches!(table.artifacts[URL].status, Status::Waiting { kind: Kind::Refused, at, credentials: Some(c) } if at == later && c == repaired));
        table.queue.clear();
        queue_due(&mut table, later);
        take(&mut table);
        let a = table.artifacts.get_mut(URL).unwrap();
        settle(a, &SourcesFetched::Absent("GET answered 404".into()), now, m, None, None, "c");
        assert!(!note_miss(&mut table, &jar, Some(repaired), later));
    }

    /// A session's rejection of an archive at a read: teq's own copy in the store removed and the
    /// artifact settled as a refused archive (its backoff, told once); a sibling held makes it queued;
    /// a file changed since the read is left to the next attempt.
    #[test]
    fn a_sessions_rejection_reaches_the_artifact() {
        let dir = std::env::temp_dir().join(format!("teq-rejected-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let (sibling, store) = (dir.join("sibling.jar"), dir.join("store.jar"));
        std::fs::write(&sibling, b"crc").unwrap();
        std::fs::write(&store, b"crc").unwrap();
        let (now, m) = (Instant::now(), Duration::from_secs(60));
        let source = |url: &str| {
            let fetch = SourcesFetch { key: "org:x_3:1.0".into(), url: url.into(), credentials: None, store: store.clone() };
            LibrarySources { jar: dir.join("x.jar"), candidates: vec![sibling.clone(), store.clone()], fetch: Some(fetch) }
        };
        let mut table = Table::default();
        enter(&mut table, &[source(URL)], now);
        take(&mut table);
        table.artifacts.get_mut(URL).unwrap().status = Status::Held;
        // The sibling rejected: queued, the file kept, recorded at its stamp.
        let (told, changed) = reject_candidate(&mut table, &sibling, stamp_of(&sibling).unwrap(), Some("a".repeat(40)), now, m);
        let a = &table.artifacts[URL];
        assert!(told.is_empty() && changed && a.status == Status::Queued && sibling.is_file() && a.rejections.files.contains_key(&sibling));
        // The store's copy rejected: removed, a refused archive's wait, told once.
        let stale = stamp_of(&store).unwrap();
        let (told, _) = reject_candidate(&mut table, &store, stale, Some("b".repeat(40)), now, m);
        let a = &table.artifacts[URL];
        assert!(!store.exists() && matches!(a.status, Status::Waiting { kind: Kind::Rejected, .. }) && a.failures == 1);
        assert_eq!(a.rejections.hashes, BTreeSet::from(["a".repeat(40), "b".repeat(40)]));
        assert!(told.len() == 1 && told[0].1.contains("does not read"), "{told:?}");
        // A file changed since the read: nothing taken from the report.
        std::fs::write(&store, b"placed again").unwrap();
        let (told, changed) = reject_candidate(&mut table, &store, stale, None, now, m);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(told.is_empty() && !changed);
    }

    /// The rejection record: a file rejected at its stamp, a copy of rejected bytes at another path
    /// found by its hash (and recorded there), a file changed to other bytes taken, recovery clearing
    /// the hashes while a file at a recorded stamp stays rejected, and a removed file forgotten.
    #[test]
    fn the_rejection_record_follows_the_content() {
        let dir = std::env::temp_dir().join(format!("teq-rejections-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let (one, copy, other) = (dir.join("one.jar"), dir.join("copy.jar"), dir.join("other.jar"));
        std::fs::write(&one, b"unreadable bytes").unwrap();
        std::fs::write(&copy, b"unreadable bytes").unwrap();
        std::fs::write(&other, b"readable bytes").unwrap();
        let mut r = Rejections::default();
        // Nothing recorded: nothing hashed.
        assert!(!r.rejects(&copy, stamp_of(&copy).unwrap()) && r.hashed.is_empty());
        r.record(&one, stamp_of(&one).unwrap(), fetch::sha1_file(&one).ok());
        let copied = r.rejects(&copy, stamp_of(&copy).unwrap());
        let readable = r.rejects(&other, stamp_of(&other).unwrap());
        // The copy changed to readable bytes: taken.
        std::fs::write(&copy, b"readable bytes, changed").unwrap();
        let changed = r.rejects(&copy, stamp_of(&copy).unwrap());
        // Recovery: the hashes cleared; the file still at its recorded stamp stays rejected.
        r.recovered();
        let still = r.rejects(&one, stamp_of(&one).unwrap());
        r.forget(&one);
        let forgotten = r.rejects(&one, stamp_of(&one).unwrap());
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!((copied, readable, changed, still, forgotten), (true, false, false, true, false));
    }

    /// A publication settled against the record as it is at the settle: bytes rejected while the
    /// attempt ran are withdrawn (their place removed, nothing held, a refused archive, the failure
    /// counted and the hashes kept); other bytes stand.
    #[test]
    fn a_publication_is_settled_against_the_record() {
        let dir = std::env::temp_dir().join(format!("teq-withdraw-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let store = dir.join("store.jar");
        std::fs::write(&store, b"unreadable bytes").unwrap();
        let sha1 = fetch::sha1_file(&store).unwrap();
        let (now, m) = (Instant::now(), Duration::from_secs(60));
        let mut table = running();
        let a = table.artifacts.get_mut(URL).unwrap();
        a.holders[0] = Some((store.clone(), stamp_of(&store).unwrap()));
        let placed = Placed { sha1: Some(sha1.clone()), stores: vec![(0, store.clone())] };
        let standing = withdraw(a, &placed, URL);
        a.rejections.record(&dir.join("sibling.jar"), (1, None), Some(sha1.clone()));
        let withdrawn = withdraw(a, &placed, URL).expect("withdrawn");
        settle(a, &withdrawn, now, m, None, None, "c");
        let removed = !store.exists();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(standing.is_none());
        assert!(matches!(withdrawn, SourcesFetched::Rejected(ref said) if said.contains(&sha1)));
        assert!(removed && a.holders[0].is_none() && a.failures == 1 && a.rejections.hashes.contains(&sha1));
        assert!(matches!(a.status, Status::Waiting { kind: Kind::Rejected, .. }));
    }

    #[test]
    fn a_cancelled_attempt_is_no_failure() {
        let (now, m) = (Instant::now(), Duration::from_secs(60));
        let mut table = running();
        let a = table.artifacts.get_mut(URL).unwrap();
        assert_eq!(settle(a, &SourcesFetched::Cancelled, now, m, None, None, "c"), None);
        assert_eq!((a.status, a.told, a.failures), (Status::Queued, None, 0));
    }
}
