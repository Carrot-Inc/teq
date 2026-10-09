//! The client side of `teq`: connects to the export's daemon over loopback TCP at the port
//! `target/teq/daemon.json` names, starting the daemon when none runs (serialised through
//! `target/teq/task.start.lock`, so that two clients never start two), sends the request with
//! the token and the handshake and prints what comes back. A daemon that answers `restart` is
//! ending; the client starts the next one, which waits for the old one's lock.

use super::export::Export;
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
    request_data(export, verb, args).0
}

/// A request whose answer carries data besides the output: the `data` lines, in order, with the
/// exit code.
pub fn request_data(export: &Export, verb: &str, args: &[String]) -> (i32, Vec<Json>) {
    let mut data = Vec::new();
    let code = exchange_all(export, verb, args, &mut data);
    (code, data)
}

fn exchange_all(export: &Export, verb: &str, args: &[String], data: &mut Vec<Json>) -> i32 {
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
            None => match start(export, &paths, &identity) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("teq: {}", e);
                    return 2;
                }
            },
        };
        data.clear();
        match exchange(stream, &message(&info.token), &paths.log, data) {
            Some(code) => return code,
            None if verb == "stop" => {
                println!("teq: the daemon was of another binary or export, and is ending");
                return 0;
            }
            None => continue,
        }
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
fn exchange(mut stream: TcpStream, message: &Json, log: &Path, data: &mut Vec<Json>) -> Option<i32> {
    let mut line = message.to_text();
    line.push('\n');
    if stream.write_all(line.as_bytes()).is_err() {
        return None;
    }
    let mut out = std::io::stdout();
    let mut err = std::io::stderr();
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else { break };
        let Ok(json) = Json::parse(&line) else { continue };
        if let Some(text) = json.get("out").and_then(Json::str) {
            let _ = out.write_all(text.as_bytes());
            let _ = out.flush();
        } else if let Some(text) = json.get("err").and_then(Json::str) {
            let _ = err.write_all(text.as_bytes());
        } else if let Some(code) = json.get("exit").and_then(Json::num) {
            return Some(code as i32);
        } else if let Some(d) = json.get("data") {
            data.push(d.clone());
        } else if json.get("restart").is_some() {
            return None;
        }
    }
    eprintln!("teq: the daemon ended without an answer (see {})", log.display());
    Some(2)
}

/// Starts the daemon unless another client started it meanwhile, and connects to it.
fn start(export: &Export, paths: &Paths, identity: &str) -> Result<(TcpStream, Info), String> {
    if let Some(dir) = paths.info.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {}", dir.display(), e))?;
    }
    let lock = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(&paths.start_lock).map_err(|e| format!("cannot open {}: {}", paths.start_lock.display(), e))?;
    let until = Instant::now() + START_BOUND;
    loop {
        match lock.try_lock() {
            Ok(()) => break,
            Err(std::fs::TryLockError::WouldBlock) if Instant::now() < until => std::thread::sleep(Duration::from_millis(10)),
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
            eprintln!("teq: waiting for the previous daemon to finish its work");
            told = true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Err(format!("the daemon did not listen within {} s (see {})", START_BOUND.as_secs(), paths.log.display()))
}

/// The daemon outlives the client: its own process group, no console of the client's.
fn detach(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
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
