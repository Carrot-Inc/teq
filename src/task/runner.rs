//! The test runner of `teq test` (docs/TARGETS.md, "The export and the project verbs"): the class files of
//! `runner/Runner.java`, compiled once by javac 17 under `--release 17` and committed beside it
//! (tests/runner-classes.sh holds them to the source), embedded here and written under
//! `<cache root>/runner/<sha1 of the classes>/` at first use; test-interface alone, its two packages taken
//! from the test class path, under `<cache root>/runner/interface/<sha1 of its entries>/`; and the runner's
//! JVM, one per invocation, which the daemon starts ahead of it, spoken to in the protocol `Runner.java`
//! states.

use super::events::{Event, Frame, Selector, Status, Tally, Thrown};
use super::export::{join_paths, Framework, Parallelism};
use super::fetch;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

const PACKAGE: &str = "dev/teq/runner";
const MAIN: &str = "dev.teq.runner.Runner";
const CLASSES: [(&str, &[u8]); 21] = [
    ("Runner.class", include_bytes!("runner/dev/teq/runner/Runner.class")),
    ("Runner$1.class", include_bytes!("runner/dev/teq/runner/Runner$1.class")),
    ("Runner$2.class", include_bytes!("runner/dev/teq/runner/Runner$2.class")),
    ("Runner$AnnotatedFingerprintWrapper.class", include_bytes!("runner/dev/teq/runner/Runner$AnnotatedFingerprintWrapper.class")),
    ("Runner$AnnotatedFingerscan.class", include_bytes!("runner/dev/teq/runner/Runner$AnnotatedFingerscan.class")),
    ("Runner$EventHandlerWrapper.class", include_bytes!("runner/dev/teq/runner/Runner$EventHandlerWrapper.class")),
    ("Runner$EventWrapper.class", include_bytes!("runner/dev/teq/runner/Runner$EventWrapper.class")),
    ("Runner$Events.class", include_bytes!("runner/dev/teq/runner/Runner$Events.class")),
    ("Runner$ForkError.class", include_bytes!("runner/dev/teq/runner/Runner$ForkError.class")),
    ("Runner$Forwarded.class", include_bytes!("runner/dev/teq/runner/Runner$Forwarded.class")),
    ("Runner$FrameworkWrapper.class", include_bytes!("runner/dev/teq/runner/Runner$FrameworkWrapper.class")),
    ("Runner$Read.class", include_bytes!("runner/dev/teq/runner/Runner$Read.class")),
    ("Runner$RunnerWrapper$1$1.class", include_bytes!("runner/dev/teq/runner/Runner$RunnerWrapper$1$1.class")),
    ("Runner$RunnerWrapper$1$2.class", include_bytes!("runner/dev/teq/runner/Runner$RunnerWrapper$1$2.class")),
    ("Runner$RunnerWrapper$1$3.class", include_bytes!("runner/dev/teq/runner/Runner$RunnerWrapper$1$3.class")),
    ("Runner$RunnerWrapper$1$4.class", include_bytes!("runner/dev/teq/runner/Runner$RunnerWrapper$1$4.class")),
    ("Runner$RunnerWrapper$1.class", include_bytes!("runner/dev/teq/runner/Runner$RunnerWrapper$1.class")),
    ("Runner$RunnerWrapper.class", include_bytes!("runner/dev/teq/runner/Runner$RunnerWrapper.class")),
    ("Runner$SubclassFingerprintWrapper.class", include_bytes!("runner/dev/teq/runner/Runner$SubclassFingerprintWrapper.class")),
    ("Runner$SubclassFingerscan.class", include_bytes!("runner/dev/teq/runner/Runner$SubclassFingerscan.class")),
    ("Runner$Unsendable.class", include_bytes!("runner/dev/teq/runner/Runner$Unsendable.class")),
];
/// test-interface's two packages, what of a container of it the runner's class path takes: the interface sbt's
/// worker runs on and its older one, which the worker's `FrameworkWrapper` adapts.
const INTERFACE_PACKAGES: [&str; 2] = ["sbt/testing/", "org/scalatools/testing/"];
/// The class that tells a container of test-interface, as sbt's worker finds its own (`classOf[Framework]`).
const INTERFACE_CLASS: &str = "sbt/testing/Framework.class";
/// The least Java the runner's class files load on.
pub const LEAST_JAVA: u32 = 17;
/// How long a runner asked to quit has before it is killed.
const QUIT_BOUND: Duration = Duration::from_secs(5);

/// The directory of the runner's classes, written whole by a rename the first time.
pub fn classes_dir() -> Result<PathBuf, String> {
    classes_under(&fetch::cache_root().ok_or("no cache directory for the test runner's classes: set TEQ_CACHE_DIR")?)
}

fn classes_under(root: &Path) -> Result<PathBuf, String> {
    let mut sha1 = fetch::Sha1::new();
    for (name, bytes) in CLASSES {
        sha1.update(name.as_bytes());
        sha1.update(&[0]);
        sha1.update(&(bytes.len() as u64).to_le_bytes());
        sha1.update(bytes);
    }
    let files: Vec<(String, &[u8])> = CLASSES.iter().map(|(name, bytes)| (format!("{}/{}", PACKAGE, name), *bytes)).collect();
    written(&root.join("runner"), &sha1.hex(), &files, "the test runner's classes")
}

/// `files`, each a path under the directory and its bytes, in `parent/<digest>/`, written whole by a rename the first
/// time and found again by their sizes.
fn written(parent: &Path, digest: &str, files: &[(String, &[u8])], what: &str) -> Result<PathBuf, String> {
    let dir = parent.join(digest);
    let complete = |dir: &Path| files.iter().all(|(name, bytes)| std::fs::metadata(dir.join(name)).is_ok_and(|m| m.len() == bytes.len() as u64));
    if complete(&dir) {
        return Ok(dir);
    }
    let part = parent.join(format!("{}.{}", digest, std::process::id()));
    let _ = std::fs::remove_dir_all(&part);
    let write = || -> std::io::Result<()> {
        for (name, bytes) in files {
            let file = part.join(name);
            if let Some(d) = file.parent() {
                std::fs::create_dir_all(d)?;
            }
            std::fs::write(file, bytes)?;
        }
        std::fs::create_dir_all(&part)?;
        if dir.exists() && !complete(&dir) {
            std::fs::remove_dir_all(&dir)?;
        }
        std::fs::rename(&part, &dir)
    };
    match write() {
        Ok(()) => Ok(dir),
        Err(_) if complete(&dir) => {
            let _ = std::fs::remove_dir_all(&part);
            Ok(dir)
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&part);
            Err(format!("cannot write {} into {}: {}", what, dir.display(), e))
        }
    }
}

/// What a jar of the test class path was found to hold of test-interface, by its path, as long as its size and
/// modification time are those it had: the directory of the interface it holds, or none.
type Seen = BTreeMap<PathBuf, (Option<(u64, SystemTime)>, Option<PathBuf>)>;
static INTERFACES: Mutex<Seen> = Mutex::new(BTreeMap::new());

/// test-interface alone, the runner's class path beside its own classes, as sbt's worker has its own copy there and
/// nothing of the project (`WorkerExchange.startWorker`): the entries of the interface's two packages
/// (`INTERFACE_PACKAGES`) taken from their container, the first entry of `paths`, the test class path in its order,
/// that holds `sbt/testing/Framework.class`, a jar or a directory, never the container itself (an assembly holding
/// other libraries beside the interface keeps them below, in the run's class loader); written under the cache root by
/// their digest, so that the same bytes are the same directory. None where no entry holds it.
pub fn interface_dir(paths: &[PathBuf]) -> Result<Option<PathBuf>, String> {
    interface_in(&fetch::cache_root().ok_or("no cache directory for the test runner's classes: set TEQ_CACHE_DIR")?, paths)
}

fn interface_in(root: &Path, paths: &[PathBuf]) -> Result<Option<PathBuf>, String> {
    for path in paths {
        if path.is_dir() {
            if path.join(INTERFACE_CLASS).is_file() {
                return interface_under(root, &directory_entries(path)?).map(Some);
            }
            continue;
        }
        let stamp = std::fs::metadata(path).ok().and_then(|m| Some((m.len(), m.modified().ok()?)));
        if let Some((seen, found)) = INTERFACES.lock().unwrap_or_else(|e| e.into_inner()).get(path) {
            if stamp.is_some() && *seen == stamp && found.as_ref().is_none_or(|d| d.is_dir()) {
                match found {
                    Some(dir) => return Ok(Some(dir.clone())),
                    None => continue,
                }
            }
        }
        // A file that is no zip archive holds no class, as the class loader reads it.
        let found = match crate::zip::Zip::open(&path.to_string_lossy()) {
            Ok(zip) if zip.find(INTERFACE_CLASS).is_some() => Some(interface_under(root, &jar_entries(&zip, path)?)?),
            _ => None,
        };
        INTERFACES.lock().unwrap_or_else(|e| e.into_inner()).insert(path.clone(), (stamp, found.clone()));
        if found.is_some() {
            return Ok(found);
        }
    }
    Ok(None)
}

fn in_interface(name: &str) -> bool {
    !name.ends_with('/') && INTERFACE_PACKAGES.iter().any(|p| name.starts_with(p))
}

/// The interface's entries of a jar.
fn jar_entries(zip: &crate::zip::Zip, path: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out = Vec::new();
    for (i, e) in zip.entries.iter().enumerate() {
        let name = zip.name(e);
        if in_interface(name) {
            let bytes = zip.read(i).map_err(|why| format!("cannot read {} in {}: {}", name, path.display(), why))?;
            out.push((name.to_string(), bytes));
        }
    }
    Ok(out)
}

/// The interface's files of a directory.
fn directory_entries(dir: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out = Vec::new();
    let mut pending: Vec<PathBuf> = INTERFACE_PACKAGES.iter().map(|p| dir.join(p)).filter(|d| d.is_dir()).collect();
    while let Some(d) = pending.pop() {
        for entry in std::fs::read_dir(&d).map_err(|e| format!("cannot read {}: {}", d.display(), e))?.flatten() {
            let path = entry.path();
            if entry.file_type().is_ok_and(|k| k.is_dir()) {
                pending.push(path);
            } else if let Ok(relative) = path.strip_prefix(dir) {
                let name = relative.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
                out.push((name, std::fs::read(&path).map_err(|e| format!("cannot read {}: {}", path.display(), e))?));
            }
        }
    }
    Ok(out)
}

/// The interface's entries written by their digest, their names sorted.
fn interface_under(root: &Path, entries: &[(String, Vec<u8>)]) -> Result<PathBuf, String> {
    let mut sorted: Vec<&(String, Vec<u8>)> = entries.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut sha1 = fetch::Sha1::new();
    for (name, bytes) in &sorted {
        sha1.update(name.as_bytes());
        sha1.update(&[0]);
        sha1.update(&(bytes.len() as u64).to_le_bytes());
        sha1.update(bytes);
    }
    let files: Vec<(String, &[u8])> = sorted.iter().map(|(name, bytes)| (name.clone(), bytes.as_slice())).collect();
    written(&root.join("runner").join("interface"), &sha1.hex(), &files, "test-interface")
}

/// A variable of the client's environment, by its name in any case on Windows.
fn var<'e>(environment: &'e BTreeMap<String, String>, name: &str) -> Option<&'e str> {
    let found = if cfg!(windows) { environment.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v) } else { environment.get(name) };
    found.map(String::as_str).filter(|v| !v.is_empty())
}

/// The `java` of the client's environment: `JAVA_HOME`'s, else the first on `PATH`.
pub fn java(environment: &BTreeMap<String, String>) -> Result<PathBuf, String> {
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    if let Some(home) = var(environment, "JAVA_HOME") {
        let java = Path::new(home).join("bin").join(exe);
        return if java.is_file() { Ok(java) } else { Err(format!("JAVA_HOME is {}, which has no bin/{}", home, exe)) };
    }
    var(environment, "PATH")
        .and_then(|path| std::env::split_paths(path).map(|d| d.join(exe)).find(|j| j.is_file()))
        .ok_or_else(|| format!("no JAVA_HOME and no {} on PATH: set JAVA_HOME to a JDK {} or later", exe, LEAST_JAVA))
}

/// What a runner is started with and keeps for its life: another is another runner. Nothing of the test class path
/// but the interface: the rest is the run's class loader's, made after the build (`Runner::load`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Spec {
    pub java: PathBuf,
    /// The least Java the export compiles for (`java.outputVersion`).
    pub least: Option<u32>,
    pub options: Vec<String>,
    pub directory: PathBuf,
    pub env: BTreeMap<String, String>,
    /// test-interface alone (`interface_dir`), a directory named by its digest.
    pub interface: PathBuf,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Kind {
    Subclass,
    Annotated,
}

/// A framework's fingerprint as its runner reported it: the framework's place in the spec's
/// list, the fingerprint's in the framework's.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Fingerprint {
    pub framework: usize,
    pub index: usize,
    pub kind: Kind,
    pub module: bool,
    pub name: String,
    /// A subclass fingerprint's `requireNoArgConstructor`, which Scala.js's bridge sends back
    /// with a task's definition (the JVM's runner keeps the fingerprint itself).
    pub no_args: bool,
}

pub struct Runner {
    pub spec: Spec,
    child: Arc<Mutex<Child>>,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: Arc<Mutex<String>>,
    token: String,
    /// The frameworks' fingerprints, which `load` reads.
    pub fingerprints: Vec<Fingerprint>,
    /// The frameworks whose class `load` did not find, with why.
    pub missing: Vec<(String, String)>,
    /// Whether `handshake` has read the JVM's version.
    handshaken: bool,
}

/// How a run ended: done, in its milliseconds; stopped by the worker's internal error (what reporting a
/// framework's failure threw), after which it answered as at its end, in its milliseconds, the internal
/// error's text written on the run's stream; or failed by a fork error (what a task threw, what a
/// framework's logger traced, a framework's failure to make its runner or its tasks, or its `done()`'s,
/// or what sbt's host threw), with what was thrown. The phases it ended are those `Report::Ended` named.
#[derive(Debug)]
pub enum Ran {
    Done(u64),
    Stopped(u64),
    Failed(Option<Thrown>),
}

/// What a run runs (`Runner.java`'s `run`): for sbt's host (`host`), the definitions its selection starts from,
/// `(framework, fingerprint, name)`, and its ordered filters, each the positions of the definitions it accepts; the
/// suites, each `(framework, fingerprint, phase, explicit, name)` (explicit for a suite named whole, its definition's
/// `explicitlySpecified`), in the selection's order; the number of phases.
pub struct Planned {
    pub host: bool,
    pub definitions: Vec<(usize, usize, String)>,
    pub filters: Vec<Vec<usize>>,
    pub suites: Vec<(usize, usize, usize, bool, String)>,
    pub phases: usize,
}

/// What a run reports as it comes: a line the suites, their frameworks or the JVM wrote, or the report of an event,
/// for the run's stream; a phase entered; a phase ended in its milliseconds, with its tally.
pub enum Report<'a> {
    Line(&'a str),
    Entered(usize),
    Ended(usize, u64, &'a Tally),
}

/// The runner's internal error (`Runner.java`'s `internalError`, the worker's: what reporting the run's error threw), as
/// the run's failure where no host of sbt's takes the run for completed (an export of Mill's): its text, no frame.
fn internal(text: &str) -> Thrown {
    Thrown { text: text.to_string(), frames: Vec::new(), cause: None }
}

/// What was thrown, its causes after it, as one throwable, the lines taken.
fn chain(thrown: &mut Vec<Thrown>) -> Option<Thrown> {
    thrown
        .drain(..)
        .rev()
        .fold(None, |cause: Option<Box<Thrown>>, mut t| {
            t.cause = cause;
            Some(Box::new(t))
        })
        .map(|t| *t)
}

/// A line of what was thrown (`throwable`, `cause`, `frame`) added to the throwables before the event or the crash
/// they belong to, a `throwable` starting them anew.
fn collect(thrown: &mut Vec<Thrown>, fields: &[String]) {
    let at = |i: usize| fields.get(i).map_or("", String::as_str);
    match at(0) {
        "throwable" | "cause" => {
            if at(0) == "throwable" {
                thrown.clear();
            }
            thrown.push(Thrown { text: at(1).to_string(), frames: Vec::new(), cause: None });
        }
        _ => {
            if let Some(t) = thrown.last_mut() {
                t.frames.push(Frame { class: at(1).to_string(), text: at(2).to_string() });
            }
        }
    }
}

/// A field of a command, its tabs, line breaks and backslashes escaped.
fn field(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n").replace('\r', "\\r")
}

/// A field of the runner's line, its escapes undone.
fn unescape(text: &str) -> String {
    if !text.contains('\\') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// A java argument file of the class path, `-cp` and the path. java's launcher reads the file in
/// the platform's encoding, on Windows the ANSI code page (`GetACP`), not UTF-8: a directory's
/// `è` written as UTF-8 reaches the JVM as `Ã¨`. A character the code page lacks reaches java
/// by no route, and is refused.
pub fn classpath_argfile(classpath: &[PathBuf]) -> Result<Vec<u8>, String> {
    let path = join_paths(classpath);
    argfile_of(&path, platform_encoded).map_err(|c| format!("the class path holds {:?}, which java on this machine cannot be given (the system's code page lacks it): {}", c, path))
}

/// `-cp` and `path` in java's argument-file syntax over the path's bytes in `encode`'s encoding:
/// quoted, and every backslash and quote byte escaped once encoded, since the launcher takes the
/// escapes byte by byte, where a character of a double-byte code page can end in either (CP932's
/// `ソ` is 0x83 0x5C).
fn argfile_of(path: &str, encode: impl Fn(&str) -> Result<Vec<u8>, char>) -> Result<Vec<u8>, char> {
    let mut out = b"-cp\n\"".to_vec();
    for b in encode(path)? {
        if b == b'\\' || b == b'"' {
            out.push(b'\\');
        }
        out.push(b);
    }
    out.extend_from_slice(b"\"\n");
    Ok(out)
}

#[cfg(not(windows))]
fn platform_encoded(text: &str) -> Result<Vec<u8>, char> {
    Ok(text.as_bytes().to_vec())
}

/// The text in the ANSI code page, or the first character it lacks (no best fit: a `ü` read as
/// `u` would name another file).
#[cfg(windows)]
fn platform_encoded(text: &str) -> Result<Vec<u8>, char> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetACP() -> u32;
        fn WideCharToMultiByte(page: u32, flags: u32, wide: *const u16, wide_len: i32, out: *mut u8, out_len: i32, default: *const u8, used_default: *mut i32) -> i32;
    }
    const UTF8: u32 = 65001;
    const NO_BEST_FIT_CHARS: u32 = 0x400;
    // SAFETY: no arguments.
    let page = unsafe { GetACP() };
    if page == UTF8 {
        return Ok(text.as_bytes().to_vec());
    }
    let encode = |s: &str| -> Option<Vec<u8>> {
        let wide: Vec<u16> = s.encode_utf16().collect();
        let mut used = 0;
        // SAFETY: the sizing call, no output buffer, the input's length given.
        let n = unsafe { WideCharToMultiByte(page, NO_BEST_FIT_CHARS, wide.as_ptr(), wide.len() as i32, std::ptr::null_mut(), 0, std::ptr::null(), &mut used) };
        if n <= 0 || used != 0 {
            return None;
        }
        let mut out = vec![0u8; n as usize];
        // SAFETY: a buffer of the length the sizing call gave.
        let n = unsafe { WideCharToMultiByte(page, NO_BEST_FIT_CHARS, wide.as_ptr(), wide.len() as i32, out.as_mut_ptr(), n, std::ptr::null(), &mut used) };
        (n > 0 && used == 0).then(|| {
            out.truncate(n as usize);
            out
        })
    };
    encode(text).ok_or_else(|| text.chars().find(|c| encode(c.encode_utf8(&mut [0; 4])).is_none()).unwrap_or(char::REPLACEMENT_CHARACTER))
}

impl Runner {
    /// Starts the runner's JVM in the spec's directory, with the client's environment and the
    /// spec's variables, its class path, its own classes and the interface, in an argument file
    /// (a long one passes Windows' limit on a command line); `handshake` then reads its version.
    pub fn launch(spec: Spec, environment: &BTreeMap<String, String>, argfile: &Path) -> Result<Runner, String> {
        let classpath = vec![classes_dir()?, spec.interface.clone()];
        if let Some(dir) = argfile.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {}", dir.display(), e))?;
        }
        std::fs::write(argfile, classpath_argfile(&classpath)?).map_err(|e| format!("cannot write {}: {}", argfile.display(), e))?;
        let token = super::token();
        let mut child = Command::new(&spec.java)
            .args(&spec.options)
            .arg(format!("@{}", argfile.display()))
            .arg(MAIN)
            .arg(&token)
            .current_dir(&spec.directory)
            .env_clear()
            .envs(environment)
            .envs(&spec.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("cannot start {}: {}", spec.java.display(), e))?;
        let stdin = child.stdin.take().expect("piped");
        let stdout = BufReader::new(child.stdout.take().expect("piped"));
        let mut err = child.stderr.take().expect("piped");
        let stderr = Arc::new(Mutex::new(String::new()));
        let tail = stderr.clone();
        crate::alloc::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = err.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let mut t = tail.lock().unwrap_or_else(|e| e.into_inner());
                t.push_str(&String::from_utf8_lossy(&buf[..n]));
                if t.len() > 65536 {
                    let cut = t.len() - 32768;
                    let cut = (cut..t.len()).find(|&i| t.is_char_boundary(i)).unwrap_or(t.len());
                    t.drain(..cut);
                }
            }
        });
        Ok(Runner { spec, child: Arc::new(Mutex::new(child)), stdin, stdout, stderr, token, fingerprints: Vec::new(), missing: Vec::new(), handshaken: false })
    }

    /// The JVM's version checked, refused when it is older than the export's classes.
    pub fn handshake(&mut self) -> Result<(), String> {
        let java = self.spec.java.display().to_string();
        let mut early = String::new();
        let version = loop {
            match self.next()? {
                Some((before, fields)) if fields.first().map(String::as_str) == Some("java") => {
                    early.push_str(&before);
                    break fields.get(1).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
                }
                Some((before, _)) => early.push_str(&before),
                None => {
                    let why = format!("{}{}", early, self.drain_stderr());
                    let old = if why.contains("UnsupportedClassVersionError") { format!(", which needs Java {} or later", LEAST_JAVA) } else { String::new() };
                    return Err(format!("{} did not start the test runner{}: {}", java, old, why.trim()));
                }
            }
        };
        self.stderr.lock().unwrap_or_else(|e| e.into_inner()).push_str(&early);
        let least = self.spec.least.unwrap_or(0).max(LEAST_JAVA);
        if version < least {
            return Err(format!("{} is Java {}, older than the {} the export's classes are for (java.outputVersion): set JAVA_HOME to a JDK {} or later", java, version, least, least));
        }
        self.handshaken = true;
        Ok(())
    }

    /// The run's class loader made over `paths`, the test class path in its order, and `frameworks` loaded through it,
    /// once the build is done (`Runner.java`'s `load`), as sbt's host loads them where `host` (an export of sbt's):
    /// their fingerprints read and those whose class is not found, the lines written meanwhile passed to `out`; a
    /// framework that failed to load otherwise is the run's fork error, as `run` gives one, its loading ended there.
    pub fn load(&mut self, paths: &[PathBuf], frameworks: &[Framework], host: bool, out: &mut dyn FnMut(&str)) -> Result<Result<(), Option<Thrown>>, String> {
        let mut commands = String::new();
        for p in paths {
            commands.push_str(&format!("path\t{}\n", field(&p.to_string_lossy())));
        }
        for f in frameworks {
            commands.push_str("framework\t");
            commands.push_str(&field(&f.class));
            for a in &f.arguments {
                commands.push('\t');
                commands.push_str(&field(a));
            }
            commands.push('\n');
        }
        commands.push_str(if host { "load\t1\n" } else { "load\t0\n" });
        self.send(&commands)?;
        self.fingerprints.clear();
        self.missing.clear();
        let mut thrown: Vec<Thrown> = Vec::new();
        loop {
            let Some((before, fields)) = self.next()? else { return Err(self.gone()) };
            if !before.is_empty() {
                out(&before);
            }
            let at = |i: usize| fields.get(i).map_or("", String::as_str);
            match at(0) {
                "loaded" => return Ok(Ok(())),
                "fingerprint" => {
                    let kind = if at(3) == "annotated" { Kind::Annotated } else { Kind::Subclass };
                    self.fingerprints.push(Fingerprint { framework: at(1).parse().unwrap_or(0), index: at(2).parse().unwrap_or(0), kind, module: at(4) == "true", name: at(5).to_string(), no_args: true });
                }
                "missing" => {
                    let class = at(1).parse::<usize>().ok().and_then(|i| frameworks.get(i)).map_or(String::new(), |f| f.class.clone());
                    self.missing.push((class, at(2).to_string()));
                }
                "throwable" | "cause" | "frame" => collect(&mut thrown, &fields),
                "forkError" => return Ok(Err(chain(&mut thrown))),
                // Of an export of Mill's, whose loading is the run's: what reporting its error threw.
                "internalError" => return Ok(Err(Some(internal(at(1))))),
                _ => {}
            }
        }
    }

    /// Runs the planned suites, the phases one after another, through the class loader `load` made on a pool of
    /// `parallelism`'s threads, sent with every run so that a lock changed since the runner started has its say, sbt's
    /// host played before and after its worker where `planned.host`: each event counted into its phase's tally in
    /// `tallies`, a suite's task's events and those of the tasks it returned its phase's, whatever their names; the
    /// lines the frameworks and the suites write, those that report an event, and the phases' boundaries passed to
    /// `report` as they come; how the run ended, its first fork error ending it, nothing more of it read, as sbt's
    /// `ForkTests` fails its harness at the worker's first `forkError`, and an internal error leaving it to end as the
    /// worker answers.
    pub fn run(&mut self, planned: &Planned, parallelism: Parallelism, verbose: bool, ansi: bool, tallies: &mut [Tally], report: &mut dyn FnMut(Report)) -> Result<Ran, String> {
        let flag = |b: bool| if b { "1" } else { "0" };
        let mut commands = format!("run\t{}\t{}\t{}\t{}\n", flag(verbose), flag(ansi), parallelism.field(), planned.phases);
        if planned.host {
            for (framework, fingerprint, name) in &planned.definitions {
                commands.push_str(&format!("def\t{}\t{}\t{}\n", framework, fingerprint, field(name)));
            }
            for accepted in &planned.filters {
                commands.push_str("filter");
                for k in accepted {
                    commands.push_str(&format!("\t{}", k));
                }
                commands.push('\n');
            }
        }
        for (framework, fingerprint, phase, explicit, name) in &planned.suites {
            commands.push_str(&format!("suite\t{}\t{}\t{}\t{}\t{}\n", framework, fingerprint, phase, flag(*explicit), field(name)));
        }
        commands.push_str("go\n");
        self.send(&commands)?;
        // The phase entered and not ended, where the events go.
        let mut current: Option<usize> = None;
        // What was thrown, its causes after it, until the event or the crash it belongs to.
        let mut thrown: Vec<Thrown> = Vec::new();
        // Whether the worker's internal error stopped the run.
        let mut stopped = false;
        loop {
            let Some((before, fields)) = self.next()? else { return Err(self.gone()) };
            if !before.is_empty() {
                report(Report::Line(&before));
            }
            let at = |i: usize| fields.get(i).map_or("", String::as_str);
            let phase = |i: usize| at(i).parse::<usize>().ok().filter(|p| *p < tallies.len());
            match at(0) {
                "throwable" | "cause" | "frame" => collect(&mut thrown, &fields),
                "forkError" => return Ok(Ran::Failed(chain(&mut thrown))),
                // Of an export of Mill's, whose runner has no host to take the run for completed, the run's failure.
                "internalError" if !planned.host => return Ok(Ran::Failed(Some(internal(at(1))))),
                // The worker writes it on the run's stream before it is sent (ForkTestMain.Run.internalError), then
                // answers as at its end, and sbt's host's outcome is its suites' (ForkTests.mainTestTask).
                "internalError" => {
                    stopped = true;
                    current = None;
                }
                "phase" => {
                    current = phase(1);
                    if let Some(p) = current {
                        report(Report::Entered(p));
                    }
                }
                "ended" => {
                    if let Some(p) = phase(1) {
                        current = None;
                        report(Report::Ended(p, at(2).parse().unwrap_or(0), &tallies[p]));
                    }
                }
                "event" => {
                    let thrown = chain(&mut thrown);
                    let selector = match at(4) {
                        "test" => Selector::Test(at(5).to_string()),
                        "nestedTest" => Selector::NestedTest(at(5).to_string(), at(6).to_string()),
                        "nestedSuite" => Selector::NestedSuite(at(5).to_string()),
                        "wildcard" => Selector::Wildcard(at(5).to_string()),
                        _ => Selector::Suite,
                    };
                    let status = match at(2).parse::<i64>() {
                        Ok(-1) => Status::Unset,
                        n => n.ok().and_then(Status::of_ordinal).unwrap_or(Status::Error),
                    };
                    let event = Event { suite: at(1).to_string(), name: at(3).to_string(), selector, status, thrown, ms: at(7).parse().unwrap_or(-1), crashed: false };
                    if let Some(p) = current {
                        let lines = tallies[p].add(event, verbose);
                        if !lines.is_empty() {
                            report(Report::Line(&lines));
                        }
                    }
                }
                "ran" => {
                    if let Some(p) = current {
                        tallies[p].ran(at(1));
                    }
                }
                "done" => {
                    let ms = at(1).parse::<u64>().unwrap_or(0);
                    return Ok(if stopped { Ran::Stopped(ms) } else { Ran::Done(ms) });
                }
                _ => {}
            }
        }
    }

    fn send(&mut self, commands: &str) -> Result<(), String> {
        self.stdin.write_all(commands.as_bytes()).and_then(|()| self.stdin.flush()).map_err(|_| self.gone())
    }

    /// The next line: what it holds before the protocol's token (all of it when it has none),
    /// and the protocol's fields after the token; nothing at the end of the output.
    fn next(&mut self) -> Result<Option<(String, Vec<String>)>, String> {
        let mut bytes = Vec::new();
        if self.stdout.read_until(b'\n', &mut bytes).map_err(|e| e.to_string())? == 0 {
            return Ok(None);
        }
        let line = String::from_utf8_lossy(&bytes);
        Ok(Some(match line.find(&self.token) {
            Some(at) => {
                let fields = line[at + self.token.len()..].trim_end_matches(['\n', '\r']).split('\t').skip(1).map(unescape).collect();
                (line[..at].to_string(), fields)
            }
            None => (line.into_owned(), Vec::new()),
        }))
    }

    /// What the JVM wrote on stderr (its own warnings, a crash) since this was last asked.
    pub fn drain_stderr(&self) -> String {
        std::mem::take(&mut *self.stderr.lock().unwrap_or_else(|e| e.into_inner()))
    }

    fn gone(&self) -> String {
        let _ = self.child.lock().unwrap_or_else(|e| e.into_inner()).try_wait();
        let tail = self.drain_stderr();
        let tail = tail.trim();
        if tail.is_empty() {
            "the test runner ended".to_string()
        } else {
            format!("the test runner ended: {}", tail)
        }
    }

    pub fn handshaken(&self) -> bool {
        self.handshaken
    }

    pub fn alive(&self) -> bool {
        matches!(self.child.lock().unwrap_or_else(|e| e.into_inner()).try_wait(), Ok(None))
    }

    /// Whether the runner answers, which a runner killed a moment ago may not have stopped
    /// looking like it does.
    pub fn answers(&mut self) -> bool {
        if !self.alive() || self.send("ping\n").is_err() {
            return false;
        }
        loop {
            match self.next() {
                Ok(Some((before, fields))) => {
                    self.stderr.lock().unwrap_or_else(|e| e.into_inner()).push_str(&before);
                    if fields.first().map(String::as_str) == Some("pong") {
                        return true;
                    }
                }
                _ => return false,
            }
        }
    }

    /// The process, for a watcher to kill when the client that asked for the run is gone.
    pub fn process(&self) -> Arc<Mutex<Child>> {
        self.child.clone()
    }

    /// A runner whose run failed at its fork error, retired as sbt's `ForkTests` leaves its worker once it has
    /// closed the worker's input (`WorkerProxy.close`): its tasks in flight finish and its frameworks' `done()`
    /// runs, what it writes drained unread, then it quits at the run's end; killed if it has not within `bound`.
    pub fn retire(self, bound: Duration) {
        let Runner { mut stdin, mut stdout, child, .. } = self;
        let drained = crate::alloc::spawn(move || {
            let mut line = Vec::new();
            while matches!(stdout.read_until(b'\n', &mut line), Ok(n) if n > 0) {
                line.clear();
            }
        });
        let _ = stdin.write_all(b"quit\n").and_then(|()| stdin.flush());
        let until = Instant::now() + bound;
        let ended = || matches!(child.lock().unwrap_or_else(|e| e.into_inner()).try_wait(), Ok(Some(_)) | Err(_));
        while !ended() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(20));
        }
        if !ended() {
            let mut child = child.lock().unwrap_or_else(|e| e.into_inner());
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = drained.join();
    }

    /// Asks the runner to quit and kills it if it has not within `QUIT_BOUND`.
    pub fn stop(mut self) {
        let _ = self.stdin.write_all(b"quit\n").and_then(|()| self.stdin.flush());
        let until = Instant::now() + QUIT_BOUND;
        while Instant::now() < until {
            if !self.alive() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let mut child = self.child.lock().unwrap_or_else(|e| e.into_inner());
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_classes_are_written_once_and_found_again() {
        let dir = std::env::temp_dir().join(format!("teq-runner-classes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let written = classes_under(&dir).unwrap();
        assert!(written.starts_with(dir.join("runner")));
        assert_eq!(std::fs::read(written.join(PACKAGE).join("Runner.class")).unwrap(), CLASSES[0].1);
        assert_eq!(classes_under(&dir).unwrap(), written);
        std::fs::remove_file(written.join(PACKAGE).join("Runner$Events.class")).unwrap();
        assert_eq!(classes_under(&dir).unwrap(), written);
        assert!(written.join(PACKAGE).join("Runner$Events.class").is_file(), "an incomplete directory is written again");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// test-interface's two packages alone, taken from the first entry that holds `sbt/testing/Framework.class`, the
    /// container's other classes left out, the same bytes the same directory; none where no entry holds it.
    #[test]
    fn the_interface_is_taken_alone_from_its_container() {
        let dir = std::env::temp_dir().join(format!("teq-runner-interface-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let write = |path: &str, text: &str| {
            let file = dir.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, text).unwrap();
        };
        write("other/sbt/testing/Task.class", "an earlier entry without the framework");
        write("assembly/sbt/testing/Framework.class", "framework");
        write("assembly/sbt/testing/Task.class", "task");
        write("assembly/org/scalatools/testing/Framework.class", "old");
        write("assembly/org/slf4j/LoggerFactory.class", "slf4j");
        write("later/sbt/testing/Framework.class", "another");
        let root = dir.join("cache");
        let paths = [dir.join("missing.jar"), dir.join("other"), dir.join("assembly"), dir.join("later")];
        let found = interface_in(&root, &paths).unwrap().unwrap();
        let mut files: Vec<String> = Vec::new();
        let mut pending = vec![found.clone()];
        while let Some(d) = pending.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                if e.path().is_dir() {
                    pending.push(e.path());
                } else {
                    files.push(e.path().strip_prefix(&found).unwrap().to_string_lossy().replace('\\', "/"));
                }
            }
        }
        files.sort();
        assert_eq!(files, ["org/scalatools/testing/Framework.class", "sbt/testing/Framework.class", "sbt/testing/Task.class"]);
        assert_eq!(std::fs::read_to_string(found.join("sbt/testing/Task.class")).unwrap(), "task");
        assert_eq!(interface_in(&root, &paths[2..]).unwrap().unwrap(), found, "the same bytes, the same directory");
        assert_ne!(interface_in(&root, &paths[3..]).unwrap().unwrap(), found);
        assert_eq!(interface_in(&root, &paths[..2]).unwrap(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn java_is_java_home_s_else_the_path_s() {
        let dir = std::env::temp_dir().join(format!("teq-runner-java-{}", std::process::id()));
        let exe = if cfg!(windows) { "java.exe" } else { "java" };
        std::fs::create_dir_all(dir.join("jdk/bin")).unwrap();
        std::fs::create_dir_all(dir.join("path")).unwrap();
        std::fs::write(dir.join("jdk/bin").join(exe), "").unwrap();
        std::fs::write(dir.join("path").join(exe), "").unwrap();
        let env = |pairs: &[(&str, &Path)]| pairs.iter().map(|(k, v)| (k.to_string(), v.to_string_lossy().into_owned())).collect::<BTreeMap<_, _>>();
        assert_eq!(java(&env(&[("JAVA_HOME", &dir.join("jdk")), ("PATH", &dir.join("path"))])).unwrap(), dir.join("jdk/bin").join(exe));
        assert_eq!(java(&env(&[("PATH", &dir.join("path"))])).unwrap(), dir.join("path").join(exe));
        assert!(java(&env(&[("JAVA_HOME", &dir.join("none")), ("PATH", &dir.join("path"))])).unwrap_err().contains("has no bin/"));
        assert!(java(&env(&[])).unwrap_err().contains("no JAVA_HOME"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fields_and_arguments_escaped() {
        assert_eq!(field("a\tb\\c\nd\re"), "a\\tb\\\\c\\nd\\re");
        assert_eq!(unescape(&field("a\tb\\c\nd\re\\n")), "a\tb\\c\nd\re\\n");
        let utf8 = |s: &str| Ok(s.as_bytes().to_vec());
        assert_eq!(argfile_of(r#"C:\a "b";d"#, utf8).unwrap(), br#"-cp
"C:\\a \"b\";d"
"#);
    }

    /// The bytes escaped once encoded: under CP932 `ソ` is 0x83 0x5C, whose trail byte is escaped
    /// as a backslash of its own, so java reads `C:\ソ\new` and not a newline; a character the
    /// page lacks is refused by name.
    #[test]
    fn argfile_escapes_the_encoded_bytes() {
        let cp932 = |s: &str| -> Result<Vec<u8>, char> {
            let mut out = Vec::new();
            for c in s.chars() {
                match c {
                    'ソ' => out.extend_from_slice(&[0x83, 0x5c]),
                    c if c.is_ascii() => out.push(c as u8),
                    c => return Err(c),
                }
            }
            Ok(out)
        };
        let bytes = argfile_of(r"C:\ソ\new\classes", cp932).unwrap();
        assert_eq!(bytes, b"-cp\n\"C:\\\\\x83\\\\\\\\new\\\\classes\"\n");
        // What java's launcher reads back, un-escaping byte by byte inside the quotes.
        let inner = &bytes[5..bytes.len() - 2];
        let mut read = Vec::new();
        let mut i = 0;
        while i < inner.len() {
            if inner[i] == b'\\' {
                i += 1;
            }
            read.push(inner[i]);
            i += 1;
        }
        assert_eq!(read, b"C:\\\x83\x5c\\new\\classes");
        assert_eq!(argfile_of("C:\\é", cp932), Err('é'));
    }
}
