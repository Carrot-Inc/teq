//! An artifact of the export resolved to a local file: first in coursier's cache, whose layout
//! follows the URL, then in teq's shared cache, content-addressed by the artifact's sha1, then
//! fetched into that cache with curl, the host's credentials from sbt's credentials file given
//! to curl on its standard input, never on its command line. A file is taken only when its size
//! and sha1 are the export's; a fetched file that is not is refused and removed. The files are
//! the launchers' too: `<cache>/bin/<sha1>/<name>` for the compiler, `<cache>/artifacts/<sha1>/<file
//! name>` for a library, `<cache>` being teq's cache (`cache_root`).
//!
//! The artifacts a classpath lacks are fetched a few at a time (`resolve_all`): at most
//! `MAX_TRANSFERS` transfers in the process whatever is being resolved, one per sha1 (a second
//! resolution of the same bytes waits for the first's placement and takes it, copied under its own
//! name when it has another), a line per transfer to the caller as it starts, and the first failure
//! ending the others.

use std::collections::HashMap;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

/// The bound on a fetch, curl's `max-time`: a partial file older than that was left by a fetch
/// that is over.
const MAX_TIME: Duration = Duration::from_secs(1800);
const MAX_FILESIZE_EXCEEDED: i32 = 63;

/// Where an artifact comes from: the URL of its repository with its path under it, and the host
/// whose credentials the fetch presents, when the export names one.
pub struct Source<'a> {
    pub repository: &'a str,
    pub path: &'a str,
    pub credentials: Option<&'a str>,
}

impl Source<'_> {
    pub fn url(&self) -> String {
        format!("{}/{}", self.repository.trim_end_matches('/'), self.path.trim_start_matches('/'))
    }
}

/// What the export pins of an artifact.
pub struct Pin<'a> {
    pub sha1: &'a str,
    pub size: u64,
}

/// An artifact to resolve: where it comes from, what is pinned of it, its place in teq's cache
/// (`<kind>/<sha1>/<name>`), and the name a progress line gives it.
pub struct Wanted<'a> {
    pub source: Source<'a>,
    pub pin: Pin<'a>,
    pub kind: &'a str,
    pub name: &'a str,
    pub key: &'a str,
}

/// How many transfers the process runs at once.
pub const MAX_TRANSFERS: usize = 4;

/// The transfers under way in the process, and the artifacts they fetch (`<kind>/<sha1>`).
struct Transfers {
    running: usize,
    fetching: Vec<String>,
}

static TRANSFERS: Mutex<Transfers> = Mutex::new(Transfers { running: 0, fetching: Vec::new() });
/// Told whenever a transfer ends.
static ENDED: Condvar = Condvar::new();
/// How often a waiting transfer and a running curl look at the resolution's end.
const POLL: Duration = Duration::from_millis(20);
const CANCELLED: &str = "cancelled by an earlier failure";

/// teq's cache, the one `src/jarcache.rs` keeps its files in too: `$TEQ_CACHE_DIR`, else
/// `$XDG_CACHE_HOME/teq`, else `%LOCALAPPDATA%\teq` on Windows, `~/Library/Caches/teq` on macOS
/// and `~/.cache/teq` elsewhere.
pub fn cache_root() -> Option<PathBuf> {
    if let Some(d) = var("TEQ_CACHE_DIR") {
        return Some(d);
    }
    if let Some(d) = var("XDG_CACHE_HOME") {
        return Some(d.join("teq"));
    }
    if cfg!(windows) {
        return var("LOCALAPPDATA").map(|d| d.join("teq"));
    }
    let home = var("HOME")?;
    Some(if cfg!(target_os = "macos") { home.join("Library/Caches/teq") } else { home.join(".cache/teq") })
}

/// Coursier's cache: `$COURSIER_CACHE`, else `%LOCALAPPDATA%\Coursier\cache\v1` on Windows,
/// `~/Library/Caches/Coursier/v1` on macOS and `~/.cache/coursier/v1` elsewhere.
pub fn coursier_cache() -> Option<PathBuf> {
    if let Some(dir) = var("COURSIER_CACHE") {
        return Some(dir);
    }
    if cfg!(windows) {
        return var("LOCALAPPDATA").map(|d| d.join("Coursier").join("cache").join("v1"));
    }
    let home = var("HOME")?;
    Some(if cfg!(target_os = "macos") { home.join("Library/Caches/Coursier/v1") } else { home.join(".cache/coursier/v1") })
}

fn var(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// The user's home: `HOME`, else `USERPROFILE` on Windows.
fn home() -> Option<PathBuf> {
    var("HOME").or_else(|| var("USERPROFILE"))
}

/// The file coursier keeps for a URL, as its `CachePath.localFile` names it with no user: the scheme, a `/`,
/// then the URL after the scheme's `:` with its leading `/`s taken off (one, or three, then any left) and a
/// trailing `/` made `/.directory`, the whole escaped as `CachePath.escape` does (the authority's user, a port,
/// a version's `+` or `@`, the query and the fragment kept, escaped). None for a URL coursier refuses: no
/// scheme, no `/` after it, or a `.` or `..` segment (tests/support/coursier-files.txt has coursier's answers).
pub fn coursier_file(cache: &Path, url: &str) -> Option<PathBuf> {
    let (scheme, rest) = url.split_once(':')?;
    let rest = rest.strip_prefix("///").or_else(|| rest.strip_prefix('/'))?;
    let rest = if rest.ends_with('/') { format!("{}.directory", rest) } else { rest.to_string() };
    let place = coursier_escape(&format!("{}/{}", scheme, rest.trim_start_matches('/')));
    if place.split('/').any(|segment| segment == "." || segment == "..") {
        return None;
    }
    Some(cache.join(place))
}

/// coursier's `CachePath.escape`: every UTF-16 unit above 128 and each of ` %$&+,:;=?@<>#` as `%`
/// and two digits of its value in base 16, capitals for ten and above.
fn coursier_escape(text: &str) -> String {
    let digit = |n: u16| char::from_u32(if n < 10 { '0' as u32 + n as u32 } else { 'A' as u32 + n as u32 - 10 }).unwrap_or('?');
    let mut out = String::new();
    for unit in text.encode_utf16() {
        if unit > 128 || " %$&+,:;=?@<>#".encode_utf16().any(|u| u == unit) {
            out.push('%');
            out.push(digit(unit / 16));
            out.push(digit(unit % 16));
        } else {
            out.push(char::from_u32(unit as u32).unwrap_or('?'));
        }
    }
    out
}

/// The local files of artifacts, in their order: each coursier's copy when it is the pinned one,
/// else the shared cache's `<kind>/<sha1>/<name>` with the pinned size; the others fetched into teq's cache by `MAX_TRANSFERS` workers at most,
/// `progress` told `teq: fetching <key> (<size>)` as each transfer starts. The first failure is
/// the answer: the transfers under way end (their curl killed, their partial files removed), the
/// artifacts not started are not, and what was placed stays.
pub fn resolve_all(wanted: &[Wanted], progress: &(dyn Fn(&str) + Sync)) -> Result<Vec<PathBuf>, String> {
    let mut files: Vec<Option<PathBuf>> = wanted.iter().map(|w| local(w).or_else(|| cached(w))).collect();
    let missing: Vec<usize> = (0..wanted.len()).filter(|&i| files[i].is_none()).collect();
    if !missing.is_empty() {
        let next = AtomicUsize::new(0);
        let cancel = AtomicBool::new(false);
        let failure: Mutex<Option<String>> = Mutex::new(None);
        let placed: Mutex<Vec<(usize, PathBuf)>> = Mutex::new(Vec::new());
        std::thread::scope(|scope| {
            for _ in 0..missing.len().min(MAX_TRANSFERS) {
                crate::alloc::spawn_in(scope, || {
                    while let Some(&i) = missing.get(next.fetch_add(1, Ordering::Relaxed)) {
                        if cancel.load(Ordering::Relaxed) {
                            return;
                        }
                        match transfer(&wanted[i], &cancel, progress) {
                            Ok(file) => placed.lock().unwrap_or_else(|e| e.into_inner()).push((i, file)),
                            Err(e) => {
                                if !cancel.swap(true, Ordering::Relaxed) {
                                    *failure.lock().unwrap_or_else(|e| e.into_inner()) = Some(e);
                                }
                                return;
                            }
                        }
                    }
                });
            }
        });
        if let Some(e) = failure.into_inner().unwrap_or_else(|e| e.into_inner()) {
            return Err(e);
        }
        for (i, file) in placed.into_inner().unwrap_or_else(|e| e.into_inner()) {
            files[i] = Some(file);
        }
    }
    Ok(files.into_iter().map(|f| f.expect("every artifact resolved or the resolution failed")).collect())
}

/// Coursier's copy of the artifact, when it is the pinned one.
fn local(w: &Wanted) -> Option<PathBuf> {
    let file = coursier_cache().and_then(|cache| coursier_file(&cache, &w.source.url()))?;
    verified(&file, &w.pin).then_some(file)
}

/// The shared cache's copy, when it has the pinned size.
fn cached(w: &Wanted) -> Option<PathBuf> {
    let file = cache_root()?.join(w.kind).join(w.pin.sha1).join(w.name);
    fs::metadata(&file).is_ok_and(|m| m.len() == w.pin.size).then_some(file)
}

/// A file the shared cache holds of the same sha1 under another name, placed (and so verified)
/// by an earlier fetch: the same bytes, which a copy gives this name without a transfer.
fn sibling(w: &Wanted) -> Option<PathBuf> {
    let dir = cache_root()?.join(w.kind).join(w.pin.sha1);
    fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| {
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        !is_part(&name) && fs::metadata(p).is_ok_and(|m| m.is_file() && m.len() == w.pin.size)
    })
}

/// One artifact fetched into the shared cache once a transfer of the process's is free and no
/// other resolution is fetching it; when another was, its placement is taken.
fn transfer(w: &Wanted, cancel: &AtomicBool, progress: &(dyn Fn(&str) + Sync)) -> Result<PathBuf, String> {
    let id = format!("{}/{}", w.kind, w.pin.sha1);
    let mut transfers = TRANSFERS.lock().unwrap_or_else(|e| e.into_inner());
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(CANCELLED.to_string());
        }
        if !transfers.fetching.contains(&id) {
            if let Some(file) = cached(w) {
                return Ok(file);
            }
            if let Some(placed) = sibling(w) {
                drop(transfers);
                return copied(w, &placed).map_err(|e| format!("{}: {}", w.key, e));
            }
            if transfers.running < MAX_TRANSFERS {
                break;
            }
        }
        transfers = ENDED.wait_timeout(transfers, POLL).unwrap_or_else(|e| e.into_inner()).0;
    }
    transfers.running += 1;
    transfers.fetching.push(id.clone());
    drop(transfers);
    progress(&format!("teq: fetching {} ({})", w.key, shown_size(w.pin.size)));
    let placed = fetch(w, cancel).map_err(|e| if e == CANCELLED { e } else { format!("{}: {}", w.key, e) });
    let mut transfers = TRANSFERS.lock().unwrap_or_else(|e| e.into_inner());
    transfers.running -= 1;
    transfers.fetching.retain(|f| *f != id);
    drop(transfers);
    ENDED.notify_all();
    placed
}

/// A size as a person reads it: bytes, kB or MB.
fn shown_size(size: u64) -> String {
    match size {
        s if s < 1000 => format!("{} B", s),
        s if s < 1_000_000 => format!("{:.0} kB", s as f64 / 1000.0),
        s => format!("{:.1} MB", s as f64 / 1_000_000.0),
    }
}

/// The artifact placed under its name from a sibling of the same sha1, through a partial file.
fn copied(w: &Wanted, placed: &Path) -> Result<PathBuf, String> {
    let file = placed.with_file_name(w.name);
    let part = part_beside(&file)?;
    let done = fs::copy(placed, &part)
        .map_err(|e| format!("cannot copy {} to {}: {}", placed.display(), part.display(), e))
        .and_then(|_| set_executable(&part, w.kind == "bin"))
        .and_then(|()| fs::rename(&part, &file).map_err(|e| format!("cannot move {} to {}: {}", part.display(), file.display(), e)));
    let _ = fs::remove_file(&part);
    done.map(|()| file)
}

/// The artifact fetched into the shared cache's `<kind>/<sha1>/<name>`, through a partial file of
/// its own, verified, made executable for a binary and renamed into place.
fn fetch(w: &Wanted, cancel: &AtomicBool) -> Result<PathBuf, String> {
    let url = w.source.url();
    let root = cache_root().ok_or("no cache directory: neither TEQ_CACHE_DIR nor a home is set")?;
    let dir = root.join(w.kind).join(w.pin.sha1);
    let file = dir.join(w.name);
    let credentials_file = credentials_file();
    let credentials = match (w.source.credentials, &credentials_file) {
        (Some(host), Some(f)) => Credentials::read(f, host)?,
        _ => None,
    };
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {}", dir.display(), e))?;
    remove_abandoned_parts(&dir);
    let part = part_beside(&file)?;
    let placed = download(&url, w.pin.size, w.source.credentials, credentials.as_ref(), credentials_file.as_deref(), &part, cancel)
        .and_then(|()| verify(&url, &w.pin, &part))
        .and_then(|()| {
            set_executable(&part, w.kind == "bin")?;
            // Another fetch may have placed it first, and Windows refuses to replace a running binary.
            fs::rename(&part, &file).or_else(|e| match fs::metadata(&file) {
                Ok(placed) if placed.is_file() && placed.len() == w.pin.size => Ok(()),
                _ => Err(format!("cannot move {} to {}: {}", part.display(), file.display(), e)),
            })
        });
    let _ = fs::remove_file(&part);
    placed.map(|()| file)
}

/// A partial file beside `path` that this fetch alone writes, created afresh: neither a process
/// id nor a counter is such a name where containers share the cache, each with its own ids.
fn part_beside(path: &Path) -> Result<PathBuf, String> {
    let name = path.file_name().expect("a cache path has a file name").to_string_lossy();
    let mut attempts = 0u32;
    loop {
        let random = std::hash::BuildHasher::hash_one(&std::collections::hash_map::RandomState::new(), attempts);
        let part = path.with_file_name(format!(".{name}.{random:016x}.part"));
        match fs::OpenOptions::new().write(true).create_new(true).open(&part) {
            Ok(_) => return Ok(part),
            Err(e) if e.kind() == ErrorKind::AlreadyExists && attempts < 8 => attempts += 1,
            Err(e) => return Err(format!("cannot create {}: {}", part.display(), e)),
        }
    }
}

/// Whether a file name is a partial file's, `.<name>.<random>.part` (`part_beside`).
fn is_part(name: &str) -> bool {
    name.starts_with('.') && name.ends_with(".part")
}

/// The partial files of fetches interrupted before they could remove them.
fn remove_abandoned_parts(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let age = entry.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok());
        if is_part(&name) && age.is_some_and(|age| age > MAX_TIME) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// The downloaded file when its size and sha1 are the pinned ones, its bytes on the disk before
/// it is renamed into place.
fn verify(url: &str, pin: &Pin, part: &Path) -> Result<(), String> {
    let unreadable = |e: std::io::Error| format!("cannot read {}: {}", part.display(), e);
    let size = fs::metadata(part).map_err(unreadable)?.len();
    // Windows flushes a file through a handle that may write it alone.
    let mut file = fs::File::options().read(true).write(true).open(part).map_err(unreadable)?;
    let mut sha1 = Sha1::new();
    let mut buffer = vec![0; 1 << 16];
    loop {
        let n = file.read(&mut buffer).map_err(unreadable)?;
        if n == 0 {
            break;
        }
        sha1.update(&buffer[..n]);
    }
    let sha1 = sha1.hex();
    if size == pin.size && sha1 == pin.sha1 {
        return file.sync_all().map_err(|e| format!("cannot write {}: {}", part.display(), e));
    }
    Err(format!("{} has sha1 {} and {} bytes where the export pins {} and {} bytes: refused", url, sha1, size, pin.sha1, pin.size))
}

/// Whether a file is the pinned artifact: its size, then its sha1, which is computed once per
/// file (path, size, modification time and inode) and remembered in `<cache>/verified`.
fn verified(file: &Path, pin: &Pin) -> bool {
    let Ok(meta) = std::fs::metadata(file) else { return false };
    if meta.len() != pin.size {
        return false;
    }
    let key = identity_key(file, &meta);
    let memo = cache_root().map(|r| r.join("verified"));
    if let Some(known) = memo.as_ref().and_then(|m| read_memo(m).remove(&key)) {
        return known == pin.sha1;
    }
    let Ok(sha1) = sha1_file(file) else { return false };
    if let Some(memo) = memo {
        append_memo(&memo, &key, &sha1);
    }
    sha1 == pin.sha1
}

/// A file as the memo knows it: path, size, modification time in nanoseconds and, where the
/// platform has one, inode.
pub fn identity_key(file: &Path, meta: &std::fs::Metadata) -> String {
    let modified = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos());
    #[cfg(unix)]
    let inode = std::os::unix::fs::MetadataExt::ino(meta);
    #[cfg(not(unix))]
    let inode = 0;
    format!("{}\t{}\t{}\t{}", file.display(), meta.len(), modified, inode)
}

/// The memo's lines, `<sha1>\t<key>`; a line torn by a concurrent writer is passed over.
pub fn read_memo(memo: &Path) -> HashMap<String, String> {
    let text = std::fs::read_to_string(memo).unwrap_or_default();
    text.lines().filter_map(|l| l.split_once('\t')).filter(|(sha1, _)| sha1.len() == 40).map(|(sha1, key)| (key.to_string(), sha1.to_string())).collect()
}

/// Appends a line to the memo in one write, which concurrent appenders cannot interleave.
pub fn append_memo(memo: &Path, key: &str, sha1: &str) {
    if let Some(dir) = memo.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let line = format!("{}\t{}\n", sha1, key);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(memo) {
        let _ = f.write_all(line.as_bytes());
    }
}

#[cfg(unix)]
fn set_executable(file: &Path, executable: bool) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mode = if executable { 0o755 } else { 0o644 };
    std::fs::set_permissions(file, std::fs::Permissions::from_mode(mode)).map_err(|e| format!("cannot set the mode of {}: {}", file.display(), e))
}

#[cfg(not(unix))]
fn set_executable(_file: &Path, _executable: bool) -> Result<(), String> {
    Ok(())
}

/// The response's body written to `part` by curl, its configuration on stdin and none of the
/// user's (a `.curlrc` could add headers to the file or log the credentials): redirects followed
/// to https alone (the credentials sent to the first host alone, as curl does), a body larger
/// than the pinned size refused, a transfer slower than a kilobyte a second for a minute or
/// longer than `MAX_TIME` given up. Proxies and certificates come from the environment, as for
/// any curl.
fn download(url: &str, size: u64, host: Option<&str>, credentials: Option<&Credentials>, credentials_file: Option<&Path>, part: &Path, cancel: &AtomicBool) -> Result<(), String> {
    refuse_cleartext(url)?;
    let output = part.to_str().ok_or_else(|| format!("{} is no UTF-8 path for curl to write", part.display()))?;
    let mut config = format!(
        "url = {}\noutput = {}\nwrite-out = \"%{{http_code}}\"\nsilent\nshow-error\nlocation\nproto-redir = \"=https\"\n\
         max-filesize = {}\nconnect-timeout = 15\nspeed-limit = 1024\nspeed-time = 60\nmax-time = {}\n",
        quoted(url),
        quoted(output),
        size,
        MAX_TIME.as_secs()
    );
    if let Some(c) = credentials {
        config.push_str(&format!("user = {}\n", quoted(&format!("{}:{}", c.user, c.password))));
    }
    let mut curl = Command::new("curl")
        .args(["--disable", "--config", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run curl to fetch {}: {}", url, e))?;
    let written = curl.stdin.take().expect("curl's stdin is piped").write_all(config.as_bytes());
    // Its output is a status code and a line of error at most, which no pipe fills.
    let exit = loop {
        match curl.try_wait() {
            Ok(Some(exit)) => break exit,
            Ok(None) if cancel.load(Ordering::Relaxed) => {
                let _ = curl.kill();
                let _ = curl.wait();
                return Err(CANCELLED.to_string());
            }
            Ok(None) => std::thread::sleep(POLL),
            Err(e) => return Err(format!("curl fetching {}: {}", url, e)),
        }
    };
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let _ = curl.stdout.take().map(|mut o| o.read_to_end(&mut stdout));
    let _ = curl.stderr.take().map(|mut e| e.read_to_end(&mut stderr));
    let done = std::process::Output { status: exit, stdout, stderr };
    if done.status.code() == Some(MAX_FILESIZE_EXCEEDED) {
        return Err(format!("{} has more than the {} bytes the export pins: refused", url, size));
    }
    if !done.status.success() || written.is_err() {
        let said = String::from_utf8_lossy(&done.stderr);
        let said = said.lines().map(str::trim).rfind(|l| !l.is_empty()).unwrap_or("no message");
        return Err(format!("GET {} failed: {}", url, said));
    }
    let status = String::from_utf8_lossy(&done.stdout).trim().to_string();
    let shown = credentials_file.map_or("~/.sbt/.credentials".to_string(), |f| f.display().to_string());
    if !status.is_empty() && status != "000" && status != "200" {
        let whose = match (host, credentials) {
            (Some(host), Some(_)) => format!(" (sent with the credentials for {} from {})", host, shown),
            (Some(host), None) => format!(" ({} holds no credentials for {})", shown, host),
            (None, _) => String::new(),
        };
        return Err(format!("GET {} answered {}{}", url, status, whose));
    }
    Ok(())
}

/// Credentials go over https alone; plain http reaches this machine and no other.
fn refuse_cleartext(url: &str) -> Result<(), String> {
    if url.starts_with("https://") {
        return Ok(());
    }
    let authority = url.strip_prefix("http://").map(|rest| rest.split('/').next().unwrap_or_default());
    let host = authority.map(|a| match a.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or_default(),
        None => a.split(':').next().unwrap_or_default(),
    });
    match host {
        Some(h) if h == "localhost" || h.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback()) => Ok(()),
        _ => Err(format!("{} is not https, which teq fetches over from every host but this one", url)),
    }
}

/// A string as curl's configuration file takes it: in double quotes, with `\` and `"` escaped.
fn quoted(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' | '"' => {
                out.push('\\');
                out.push(c);
            }
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// sbt's credentials file: `$SBT_CREDENTIALS`, else `~/.sbt/.credentials`.
fn credentials_file() -> Option<PathBuf> {
    var("SBT_CREDENTIALS").or_else(|| Some(home()?.join(".sbt").join(".credentials")))
}

struct Credentials {
    user: String,
    password: String,
}

impl Credentials {
    /// The user and password the file holds for `host`, read as sbt reads it: a Java properties
    /// file in ISO-8859-1 holding one credential, its keys sbt's (`host` or `hostname`, `user`,
    /// `user.name` or `username`, `password`, `pwd`, `pass` or `passwd`), its values trimmed as
    /// Java's `trim` does.
    fn read(file: &Path, host: &str) -> Result<Option<Credentials>, String> {
        let text: String = match fs::read(file) {
            Ok(bytes) => bytes.into_iter().map(char::from).collect(),
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("cannot read {}: {}", file.display(), e)),
        };
        let pairs = properties(&text);
        let get = |keys: &[&str]| {
            keys.iter().find_map(|key| pairs.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v.trim_matches(|c| c <= ' ').to_string()))
        };
        Ok(match (get(&["host", "hostname"]), get(&["user", "user.name", "username"]), get(&["password", "pwd", "pass", "passwd"])) {
            (Some(h), Some(user), Some(password)) if h.eq_ignore_ascii_case(host) => Some(Credentials { user, password }),
            _ => None,
        })
    }
}

/// The key-value pairs of a Java properties file, in order: lines ended by `\n`, `\r` or both,
/// comment lines, `=`, `:` or blanks between key and value, backslash escapes (a surrogate pair
/// of `\u` escapes one character) and continued lines as `java.util.Properties` takes them.
fn properties(text: &str) -> Vec<(String, String)> {
    const BLANK: [char; 3] = [' ', '\t', '\x0c'];
    let mut pairs = Vec::new();
    let mut lines = text.split("\r\n").flat_map(|l| l.split(['\r', '\n']));
    while let Some(first) = lines.next() {
        let mut line = first.trim_start_matches(BLANK).to_string();
        if line.is_empty() || line.starts_with(['#', '!']) {
            continue;
        }
        while line.chars().rev().take_while(|&c| c == '\\').count() % 2 == 1 {
            line.pop();
            line.push_str(lines.next().unwrap_or_default().trim_start_matches(BLANK));
        }
        let (key, rest) = split_key(&line);
        let rest = rest.trim_start_matches(BLANK);
        let value = rest.strip_prefix(['=', ':']).unwrap_or(rest).trim_start_matches(BLANK);
        pairs.push((unescape(key), unescape(value)));
    }
    pairs
}

/// The key up to its first unescaped `=`, `:` or blank, and what follows.
fn split_key(line: &str) -> (&str, &str) {
    let mut escaped = false;
    for (i, c) in line.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '=' | ':' | ' ' | '\t' | '\x0c' => return line.split_at(i),
            _ => {}
        }
    }
    (line, "")
}

fn unescape(s: &str) -> String {
    let mut units = Vec::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        let c = match c {
            '\\' => match chars.next() {
                Some('t') => '\t',
                Some('n') => '\n',
                Some('r') => '\r',
                Some('f') => '\x0c',
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    units.push(u16::from_str_radix(&hex, 16).unwrap_or(0xfffd));
                    continue;
                }
                Some(other) => other,
                None => break,
            },
            c => c,
        };
        units.extend_from_slice(c.encode_utf16(&mut [0; 2]));
    }
    String::from_utf16_lossy(&units)
}

/// The sha1 of a file's contents, in lowercase hexadecimal.
pub fn sha1_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut sha1 = Sha1::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        sha1.update(&buf[..n]);
    }
    Ok(sha1.hex())
}

/// SHA-1 (FIPS 180-4), for the checksums Maven repositories publish.
pub struct Sha1 {
    state: [u32; 5],
    block: [u8; 64],
    filled: usize,
    length: u64,
}

impl Sha1 {
    pub fn new() -> Sha1 {
        Sha1 { state: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0], block: [0; 64], filled: 0, length: 0 }
    }

    pub fn update(&mut self, mut bytes: &[u8]) {
        self.length += bytes.len() as u64;
        while !bytes.is_empty() {
            let take = (64 - self.filled).min(bytes.len());
            self.block[self.filled..self.filled + take].copy_from_slice(&bytes[..take]);
            self.filled += take;
            bytes = &bytes[take..];
            if self.filled == 64 {
                let block = self.block;
                self.compress(&block);
                self.filled = 0;
            }
        }
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 80];
        for (i, word) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = self.state;
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        for (s, v) in self.state.iter_mut().zip([a, b, c, d, e]) {
            *s = s.wrapping_add(v);
        }
    }

    pub fn hex(mut self) -> String {
        let bits = self.length.wrapping_mul(8);
        self.update(&[0x80]);
        while self.filled != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_be_bytes());
        self.state.iter().map(|w| format!("{:08x}", w)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha1(bytes: &[u8]) -> String {
        let mut s = Sha1::new();
        s.update(bytes);
        s.hex()
    }

    #[test]
    fn sha1_of_the_standard_vectors() {
        assert_eq!(sha1(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(sha1(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(sha1(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"), "84983e441c3bd26ebaae4aa1f95129e5e54670f1");
        let mut s = Sha1::new();
        for _ in 0..1000 {
            s.update(&[b'a'; 1000]);
        }
        assert_eq!(s.hex(), "34aa973cd4c4daa4f61eeb2bdbad27316534016f");
    }

    #[test]
    fn coursier_keeps_a_url_under_its_scheme_and_host() {
        let cache = Path::new("/c/v1");
        // coursier's own answers (CachePath.localFile), which the launchers, vite-plugin-teq and the export
        // checker are tested on: a repository's port, a version's `+`, an `@`, a user, a percent sign, the query
        // and the fragment, a trailing `/`, and a `.` segment refused (`-`).
        let cases = include_str!("../../tests/support/coursier-files.txt");
        let cases: Vec<(&str, &str)> = cases.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).filter_map(|l| l.split_once(' ')).collect();
        assert!(cases.len() > 10);
        for (url, file) in cases {
            assert_eq!(coursier_file(cache, url), (file != "-").then(|| cache.join(file)), "{url}");
        }
        assert_eq!(coursier_file(cache, "no-scheme"), None);
        assert_eq!(coursier_file(cache, "https://repo.invalid/\u{e9}/x.jar"), Some(PathBuf::from("/c/v1/https/repo.invalid/%E9/x.jar")));
        let source = Source { repository: "https://repo1.maven.org/maven2/", path: "a/b.jar", credentials: None };
        assert_eq!(source.url(), "https://repo1.maven.org/maven2/a/b.jar");
    }

    #[test]
    fn reads_properties_as_java_does() {
        let text = "# a comment\n! another\n  realm=Sonatype Nexus Repository Manager\nhost : artifacts.example\nuser me\\\n   self\npassword=p\\=ss\\\\w\\u00f6rd  \n";
        let pairs = properties(text);
        let value = |key: &str| pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());
        assert_eq!(value("realm"), Some("Sonatype Nexus Repository Manager"));
        assert_eq!(value("host"), Some("artifacts.example"));
        assert_eq!(value("user"), Some("meself"));
        assert_eq!(value("password"), Some("p=ss\\wörd  "));
    }

    #[test]
    fn reads_lines_and_escapes_as_java_does() {
        let pairs = properties("a=1\rb=2\r\nc=\\ud83d\\ude00\n\u{b}d=4");
        let value = |key: &str| pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());
        assert_eq!((value("a"), value("b"), value("c")), (Some("1"), Some("2"), Some("😀")));
        assert_eq!(value("\u{b}d"), Some("4"));
    }

    #[test]
    fn reads_credentials_in_iso_8859_1() {
        let file = std::env::temp_dir().join(format!("teq-credentials-latin1-{}", std::process::id()));
        fs::write(&file, b"# caf\xe9\nhost=h.example\nuser=u\npassword=p\xe9\\u00e9\n").unwrap();
        let found = Credentials::read(&file, "h.example").unwrap().unwrap();
        fs::remove_file(&file).unwrap();
        assert_eq!(found.password, "péé");
    }

    #[test]
    fn takes_credentials_for_their_host_alone() {
        let file = std::env::temp_dir().join(format!("teq-credentials-{}", std::process::id()));
        fs::write(&file, "realm=R\nhost=Artifacts.Example\nusername=u\npasswd= p \n").unwrap();
        let found = Credentials::read(&file, "artifacts.example").unwrap().unwrap();
        assert_eq!((found.user.as_str(), found.password.as_str()), ("u", "p"));
        assert!(Credentials::read(&file, "elsewhere.example").unwrap().is_none());
        fs::remove_file(&file).unwrap();
        assert!(Credentials::read(&file, "artifacts.example").unwrap().is_none());
    }

    #[test]
    fn makes_each_partial_file_afresh() {
        let dir = std::env::temp_dir().join(format!("teq-parts-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("lib.jar");
        let (a, b) = (part_beside(&path).unwrap(), part_beside(&path).unwrap());
        let made = (a != b, a.is_file(), b.is_file());
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(made, (true, true, true));
    }

    #[test]
    fn a_partial_file_by_its_name() {
        let dir = std::env::temp_dir().join(format!("teq-part-name-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let part = part_beside(&dir.join(".one.jar")).unwrap();
        let named = is_part(&part.file_name().unwrap().to_string_lossy());
        fs::remove_dir_all(&dir).unwrap();
        assert!(named, "{}", part.display());
        assert!(is_part(".lib-1.0.jar.0123456789abcdef.part"));
        // An artifact whose own name starts with a dot is placed, a sibling to take.
        assert!(!is_part(".one.jar") && !is_part("lib-1.0.jar"));
    }

    #[test]
    fn quotes_for_curl() {
        assert_eq!(quoted("a\"b\\c\nd"), r#""a\"b\\c\nd""#);
    }

    #[test]
    fn fetches_in_clear_from_this_machine_alone() {
        for url in ["https://a.example/x", "http://127.0.0.1:8080/x", "http://localhost/x", "http://[::1]:9/x"] {
            assert!(refuse_cleartext(url).is_ok(), "{url}");
        }
        for url in ["http://a.example/x", "http://10.0.0.1/x", "ftp://a.example/x", "file:///x"] {
            assert!(refuse_cleartext(url).is_err(), "{url}");
        }
    }
}
