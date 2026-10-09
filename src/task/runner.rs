//! The test runner of `teq test` (docs/TARGETS.md, "The export and the project verbs"): the class files of
//! `runner/Runner.java`, compiled once by javac 17 under `--release 17` and committed beside it
//! (tests/runner-classes.sh holds them to the source), embedded here and written under
//! `<cache root>/runner/<sha1 of the classes>/` at first use; and the runner's JVM, which the
//! daemon keeps warm per test configuration, spoken to in the protocol `Runner.java` states.

use super::export::{join_paths, Framework};
use super::fetch;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

const PACKAGE: &str = "dev/teq/runner";
const MAIN: &str = "dev.teq.runner.Runner";
const CLASSES: [(&str, &[u8]); 4] = [
    ("Runner.class", include_bytes!("runner/dev/teq/runner/Runner.class")),
    ("Runner$1.class", include_bytes!("runner/dev/teq/runner/Runner$1.class")),
    ("Runner$Counts.class", include_bytes!("runner/dev/teq/runner/Runner$Counts.class")),
    ("Runner$Forwarded.class", include_bytes!("runner/dev/teq/runner/Runner$Forwarded.class")),
];
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
    let digest = sha1.hex();
    let parent = root.join("runner");
    let dir = parent.join(&digest);
    let complete = |dir: &Path| CLASSES.iter().all(|(name, bytes)| std::fs::metadata(dir.join(PACKAGE).join(name)).is_ok_and(|m| m.len() == bytes.len() as u64));
    if complete(&dir) {
        return Ok(dir);
    }
    let part = parent.join(format!("{}.{}", digest, std::process::id()));
    let _ = std::fs::remove_dir_all(&part);
    let write = || -> std::io::Result<()> {
        std::fs::create_dir_all(part.join(PACKAGE))?;
        for (name, bytes) in CLASSES {
            std::fs::write(part.join(PACKAGE).join(name), bytes)?;
        }
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
            Err(format!("cannot write the test runner's classes into {}: {}", dir.display(), e))
        }
    }
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

/// A jar of the runner's own class path with its size and modification time, which a build that
/// rewrites it in place changes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Jar {
    pub path: PathBuf,
    pub stamp: Option<(u64, SystemTime)>,
}

impl Jar {
    pub fn new(path: PathBuf) -> Jar {
        let stamp = std::fs::metadata(&path).ok().and_then(|m| Some((m.len(), m.modified().ok()?)));
        Jar { path, stamp }
    }
}

/// What a runner is started with and keeps for its life: another is another runner.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Spec {
    pub java: PathBuf,
    /// The least Java the export compiles for (`java.outputVersion`).
    pub least: Option<u32>,
    pub options: Vec<String>,
    pub directory: PathBuf,
    pub env: BTreeMap<String, String>,
    pub jars: Vec<Jar>,
    pub frameworks: Vec<Framework>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
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
}

pub struct Done {
    pub failed: u64,
    pub passed: u64,
    pub ms: u64,
}

pub struct Runner {
    pub spec: Spec,
    child: Arc<Mutex<Child>>,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: Arc<Mutex<String>>,
    token: String,
    pub fingerprints: Vec<Fingerprint>,
    /// The frameworks that did not load, with why.
    pub missing: Vec<(String, String)>,
}

/// A field of a command, its tabs, line breaks and backslashes escaped.
fn field(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n").replace('\r', "\\r")
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
    /// spec's variables, its class path in an argument file (a long one passes Windows' limit
    /// on a command line); `handshake` then waits for it.
    pub fn launch(spec: Spec, environment: &BTreeMap<String, String>, argfile: &Path) -> Result<Runner, String> {
        let mut classpath = vec![classes_dir()?];
        classpath.extend(spec.jars.iter().map(|j| j.path.clone()));
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
        Ok(Runner { spec, child: Arc::new(Mutex::new(child)), stdin, stdout, stderr, token, fingerprints: Vec::new(), missing: Vec::new() })
    }

    /// The JVM's version checked, refused when it is older than the export's classes; the
    /// frameworks loaded and their fingerprints read.
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
        let mut commands = String::new();
        for f in &self.spec.frameworks {
            commands.push_str("framework\t");
            commands.push_str(&field(&f.class));
            for a in &f.arguments {
                commands.push('\t');
                commands.push_str(&field(a));
            }
            commands.push('\n');
        }
        commands.push_str("start\n");
        self.send(&commands)?;
        loop {
            let Some((before, fields)) = self.next()? else { return Err(self.gone()) };
            self.stderr.lock().unwrap_or_else(|e| e.into_inner()).push_str(&before);
            let at = |i: usize| fields.get(i).map_or("", String::as_str);
            match at(0) {
                "ready" => return Ok(()),
                "fingerprint" => {
                    let kind = if at(3) == "annotated" { Kind::Annotated } else { Kind::Subclass };
                    self.fingerprints.push(Fingerprint { framework: at(1).parse().unwrap_or(0), index: at(2).parse().unwrap_or(0), kind, module: at(4) == "true", name: at(5).to_string() });
                }
                "missing" => {
                    let class = at(1).parse::<usize>().ok().and_then(|i| self.spec.frameworks.get(i)).map_or(String::new(), |f| f.class.clone());
                    self.missing.push((class, at(2).to_string()));
                }
                _ => {}
            }
        }
    }

    /// Runs the suites, each `(framework, fingerprint, name)`, through a class loader over the
    /// directories, each line the frameworks and the suites write passed to `out` as it comes.
    pub fn run(&mut self, directories: &[PathBuf], suites: &[(usize, usize, String)], verbose: bool, ansi: bool, out: &mut dyn FnMut(&str)) -> Result<Done, String> {
        let flag = |b: bool| if b { "1" } else { "0" };
        let mut commands = format!("run\t{}\t{}\n", flag(verbose), flag(ansi));
        for d in directories {
            commands.push_str(&format!("path\t{}\n", field(&d.to_string_lossy())));
        }
        for (framework, fingerprint, name) in suites {
            commands.push_str(&format!("suite\t{}\t{}\t{}\n", framework, fingerprint, field(name)));
        }
        commands.push_str("go\n");
        self.send(&commands)?;
        loop {
            let Some((before, fields)) = self.next()? else { return Err(self.gone()) };
            if !before.is_empty() {
                out(&before);
            }
            if fields.first().map(String::as_str) == Some("done") {
                let number = |i: usize| fields.get(i).and_then(|n| n.parse::<u64>().ok()).unwrap_or(0);
                return Ok(Done { failed: number(1), passed: number(2), ms: number(3) });
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
                let fields = line[at + self.token.len()..].trim_end_matches(['\n', '\r']).split('\t').skip(1).map(str::to_string).collect();
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
        std::fs::remove_file(written.join(PACKAGE).join("Runner$Counts.class")).unwrap();
        assert_eq!(classes_under(&dir).unwrap(), written);
        assert!(written.join(PACKAGE).join("Runner$Counts.class").is_file(), "an incomplete directory is written again");
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
