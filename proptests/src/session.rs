//! A resident `teq compiler watch` session and the fresh build it is compared with. Every wait is
//! bounded: a session that does not answer in time is killed and the case fails.

use crate::json::Json;
use crate::tree::{self, Tree};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Split,
    Check,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Split => "split",
            Kind::Check => "check",
        }
    }

    fn arguments(self, inputs: &[PathBuf], out: &Path) -> Vec<std::ffi::OsString> {
        let mut args: Vec<std::ffi::OsString> = vec!["compiler".into(), "watch".into()];
        if self == Kind::Check {
            args.push("--check".into());
        }
        args.extend(inputs.iter().map(|input| input.clone().into_os_string()));
        if self == Kind::Split {
            args.push("--split".into());
            args.push(out.into());
        }
        args
    }
}

/// The time spent waiting for teq's processes, for the share of a run that is the compiler's.
pub static WAITED_MICROS: AtomicU64 = AtomicU64::new(0);

pub fn answer_bound() -> Duration {
    seconds("TEQ_PROP_ANSWER_SECONDS", 20)
}

pub fn fresh_bound() -> Duration {
    seconds("TEQ_PROP_FRESH_SECONDS", 60)
}

fn seconds(variable: &str, default: u64) -> Duration {
    let value = std::env::var(variable).ok().and_then(|text| text.parse().ok());
    Duration::from_secs(value.unwrap_or(default))
}

pub struct Session {
    kind: Kind,
    child: Child,
    stdin: Option<ChildStdin>,
    answers: Receiver<String>,
    errors: PathBuf,
    pub out: PathBuf,
}

impl Session {
    /// Starts a session and reads the answer of its first build.
    pub fn start(teq: &Path, kind: Kind, inputs: &[PathBuf], work: &Path) -> Result<(Session, Json), String> {
        let out = work.join("out");
        let errors = work.join("session.err");
        let log = std::fs::File::create(&errors).map_err(|e| format!("cannot create {}: {e}", errors.display()))?;
        let mut child = Command::new(teq)
            .args(kind.arguments(inputs, &out))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(log)
            .spawn()
            .map_err(|e| format!("cannot start {}: {e}", teq.display()))?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (sender, answers) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) => {
                        if sender.send(line).is_err() {
                            return;
                        }
                    }
                    Err(_) => return,
                }
            }
        });
        let mut session = Session { kind, child, stdin, answers, errors, out };
        let first = session.answer()?;
        Ok((session, first))
    }

    /// `build` for the given files, or the plain `build` that looks at every file's time.
    pub fn build(&mut self, files: &[PathBuf]) -> Result<Json, String> {
        let mut command = String::from("build");
        if !files.is_empty() {
            command.push(' ');
            for file in files {
                command.push_str(&file.to_string_lossy());
                command.push('\n');
            }
        }
        command.push('\n');
        let stdin = self.stdin.as_mut().ok_or("the session has ended")?;
        let sent = stdin.write_all(command.as_bytes()).and_then(|_| stdin.flush());
        if let Err(e) = sent {
            return Err(format!("the session takes no command ({e}){}", self.said()));
        }
        self.answer()
    }

    fn answer(&mut self) -> Result<Json, String> {
        let bound = answer_bound();
        let started = Instant::now();
        let line = self.answers.recv_timeout(bound);
        WAITED_MICROS.fetch_add(started.elapsed().as_micros() as u64, Ordering::Relaxed);
        match line {
            Ok(line) => Json::parse(&line).map_err(|e| format!("an answer that is no JSON ({e}): {line}")),
            Err(RecvTimeoutError::Timeout) => {
                self.end();
                Err(format!("no answer from the {} session in {} s{}", self.kind.name(), bound.as_secs(), self.said()))
            }
            Err(RecvTimeoutError::Disconnected) => {
                let status = self.child.wait().map(|s| s.to_string()).unwrap_or_default();
                Err(format!("the {} session ended ({status}){}", self.kind.name(), self.said()))
            }
        }
    }

    fn said(&self) -> String {
        match std::fs::read_to_string(&self.errors) {
            Ok(text) if !text.trim().is_empty() => {
                let text: String = text.chars().take(600).collect();
                format!(", stderr: {}", text.trim())
            }
            _ => String::new(),
        }
    }

    fn end(&mut self) {
        self.stdin = None;
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(mut stdin) = self.stdin.take() {
            let _ = stdin.write_all(b"quit\n");
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// What a process of its own gives for the same inputs: the first answer of a new session,
/// and for a split session the directory it wrote.
pub fn fresh(teq: &Path, kind: Kind, inputs: &[PathBuf], work: &Path) -> Result<(Json, Tree), String> {
    let out = work.join("out");
    if out.exists() {
        std::fs::remove_dir_all(&out).map_err(|e| format!("cannot empty {}: {e}", out.display()))?;
    }
    std::fs::create_dir_all(work).map_err(|e| format!("cannot create {}: {e}", work.display()))?;
    let mut command = Command::new(teq);
    command.args(kind.arguments(inputs, &out));
    let output = run(command, b"quit\n", fresh_bound(), work)?;
    let first = output.stdout.lines().next().unwrap_or("").to_string();
    let answer = Json::parse(&first).map_err(|e| {
        format!("the fresh {} build gave no JSON ({e}): {first} {} ({})", kind.name(), output.stderr, output.status)
    })?;
    Ok((answer, tree::read(&out)))
}

pub struct Output {
    pub status: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Runs a process to its end within the bound; its output goes through files of `work`, so
/// that no pipe fills while nobody reads it.
pub fn run(mut command: Command, input: &[u8], bound: Duration, work: &Path) -> Result<Output, String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let out_path = work.join(format!("run-{}-{id}.out", std::process::id()));
    let err_path = work.join(format!("run-{}-{id}.err", std::process::id()));
    let create = |path: &Path| std::fs::File::create(path).map_err(|e| format!("cannot create {}: {e}", path.display()));
    let started = Instant::now();
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(create(&out_path)?)
        .stderr(create(&err_path)?)
        .spawn()
        .map_err(|e| format!("cannot start {:?}: {e}", command.get_program()))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input);
    }
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() > bound => {
                let _ = child.kill();
                let _ = child.wait();
                WAITED_MICROS.fetch_add(started.elapsed().as_micros() as u64, Ordering::Relaxed);
                return Err(format!("{:?} did not end in {} s", command.get_program(), bound.as_secs()));
            }
            Ok(None) => std::thread::sleep(Duration::from_micros(500)),
            Err(e) => return Err(format!("cannot wait for {:?}: {e}", command.get_program())),
        }
    };
    WAITED_MICROS.fetch_add(started.elapsed().as_micros() as u64, Ordering::Relaxed);
    let read = |path: &Path| String::from_utf8_lossy(&std::fs::read(path).unwrap_or_default()).into_owned();
    let output = Output {
        status: status.to_string(),
        success: status.success(),
        stdout: read(&out_path),
        stderr: read(&err_path),
    };
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&err_path);
    Ok(output)
}
