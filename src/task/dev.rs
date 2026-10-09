//! `teq dev <project> [-- args]`: a Scala.js project's dev loop, which the client runs
//! itself, with no daemon: the project's generators, the package manager's install when the
//! lockfile is not the one installed last (or `node_modules` is missing), then the export's dev
//! command with the args added, in the directory of the `package.json`, its output the
//! terminal's. The dev command is vite with vite-plugin-teq, which runs `teq watch
//! <project>` itself; `TEQ` names this binary to it, so that the loop runs one binary. Each
//! second the generators run where their fingerprint changed, and the dev command is restarted
//! when the export changes what the project's build reads, when the lockfile is not the one the
//! command started with (installed first unless another loop did) or when this binary's file
//! changes. A change of the export's `teq` block ends the loop with code 75, as it ends the
//! daemon, so that the binary the export names now is the next one run. The dev command leads a
//! process group of its own, which a restart ends whole; given the terminal, it takes Ctrl-C and
//! the keys itself, and its end is the loop's. A signal the loop receives is passed on to the
//! group, which then has 5 s to end before it is killed (`job.rs`).

use super::export::{self, Entry, Export, Key, Platform};
use super::generators;
use super::job;
use super::lock;
use super::sha256;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime};

/// The exit code when the export pins another compiler: the loop is to be run again.
const REPINNED: i32 = 75;
const POLL: Duration = Duration::from_millis(100);
const CHECK_EVERY: Duration = Duration::from_secs(1);

pub fn run(mut export: Export, args: &[String]) -> i32 {
    let Some((name, rest)) = args.split_first() else {
        eprintln!("usage: teq dev <project> [-- args]");
        return 2;
    };
    let extra: Vec<String> = match rest.split_first() {
        None => Vec::new(),
        Some((dashes, extra)) if dashes == "--" => extra.to_vec(),
        Some((other, _)) => {
            eprintln!("teq dev: {} is not --: arguments for the dev command follow --", other);
            return 2;
        }
    };
    let exe = match std::env::current_exe().and_then(std::fs::canonicalize) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("teq dev: cannot find this teq: {}", e);
            return 2;
        }
    };
    job::take_signals();
    let pinned = export.teq.clone();
    let mut binary_seen = file_stamp(&exe);
    loop {
        let mut file_seen = file_stamp(&export.file);
        let reads = match snapshot(&export.file, name, &pinned) {
            Ok(Snapshot::Reads(fresh, reads)) => {
                export = fresh;
                reads
            }
            Ok(Snapshot::Repinned) => return repinned(),
            Ok(Snapshot::Gone(e)) | Err(e) => {
                eprintln!("teq dev: {}", e);
                return 2;
            }
        };
        let dev = export.projects[name].dev.clone().expect("checked by the snapshot");
        let lockfile = export.path(&dev.lockfile);
        let dir = lockfile.parent().map_or_else(|| export.root.clone(), Path::to_path_buf);
        if let Err(e) = install(&export, &dev, &lockfile, &dir) {
            eprintln!("teq dev: {}", e);
            return 2;
        }
        let lock_installed = digest(&lockfile);
        let mut lock_seen = file_stamp(&lockfile);
        let closure = export.closure(&Key::new(name, "compile"));
        let classes = export.path(&export.classes(&Key::new(name, "compile")));
        let env: BTreeMap<String, String> = std::env::vars().filter(|(k, _)| k == "SOURCE_VERSION").collect();
        match generators::run(&export, &closure, classes.clone(), &env) {
            Ok(notes) => notes.iter().for_each(|n| eprintln!("{}", n)),
            Err(e) => {
                eprintln!("teq dev: {}", e);
                return 2;
            }
        }
        let mut command = shell_command(&dev.command, &extra);
        command.current_dir(&dir).env("TEQ", &exe);
        let mut child = match job::start(command) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("teq dev: cannot run {}: {}", dev.command.join(" "), e);
                return 2;
            }
        };
        let mut generator_error = String::new();
        let mut last_check = Instant::now();
        let why = loop {
            match child.try_wait() {
                Ok(Some(status)) => match job::received() {
                    Some(sig) => {
                        job::wait_for(&mut child);
                        return 128 + sig;
                    }
                    None => {
                        job::stop(&mut child);
                        return job::code_of(status);
                    }
                },
                Ok(None) => {}
                Err(e) => {
                    eprintln!("teq dev: cannot wait for the dev command: {}", e);
                    job::stop(&mut child);
                    return 2;
                }
            }
            if let Some(sig) = job::received() {
                job::wait_for(&mut child);
                return 128 + sig;
            }
            std::thread::sleep(POLL);
            if last_check.elapsed() < CHECK_EVERY {
                continue;
            }
            last_check = Instant::now();
            match generators::run(&export, &closure, classes.clone(), &env) {
                Ok(notes) => {
                    generator_error.clear();
                    notes.iter().for_each(|n| eprintln!("{}", n));
                }
                Err(e) if e != generator_error => {
                    eprintln!("teq dev: {}", e);
                    generator_error = e;
                }
                Err(_) => {}
            }
            let stamp = file_stamp(&export.file);
            if stamp != file_seen {
                file_seen = stamp;
                match snapshot(&export.file, name, &pinned) {
                    Ok(Snapshot::Reads(_, now)) if now != reads => break format!("{} changed what {} reads", export::FILE, name),
                    Ok(Snapshot::Reads(..)) => {}
                    Ok(Snapshot::Repinned) => {
                        job::stop(&mut child);
                        return repinned();
                    }
                    Ok(Snapshot::Gone(e)) => {
                        job::stop(&mut child);
                        eprintln!("teq dev: {}", e);
                        return 2;
                    }
                    // A file being written: read again when it changes next.
                    Err(e) => eprintln!("teq dev: {}", e),
                }
            }
            let stamp = file_stamp(&lockfile);
            if stamp != lock_seen {
                lock_seen = stamp;
                if digest(&lockfile) != lock_installed {
                    break format!("{} changed", dev.lockfile);
                }
            }
            let stamp = file_stamp(&exe);
            if stamp != binary_seen {
                std::thread::sleep(CHECK_EVERY);
                binary_seen = file_stamp(&exe);
                break format!("{} changed", exe.display());
            }
        };
        eprintln!("teq dev: {}: restarting {}", why, dev.command.join(" "));
        job::stop(&mut child);
    }
}

fn repinned() -> i32 {
    eprintln!("teq dev: {} pins another compiler now: run the command again for it", export::FILE);
    REPINNED
}

/// The export as the file holds it now: another compiler pinned, the project gone (or its dev
/// loop), or the export with what the dev build of the project reads of it, from the same bytes:
/// the project's block, the blocks of the projects its compile closure takes sources from, the
/// repositories and the records of the jars those blocks name, in their canonical form.
enum Snapshot {
    Repinned,
    Gone(String),
    Reads(Export, String),
}

fn snapshot(file: &Path, name: &str, pinned: &export::Pinned) -> Result<Snapshot, String> {
    let bytes = std::fs::read(file).map_err(|e| format!("cannot read {}: {}", file.display(), e))?;
    let export = Export::parse(file, &bytes)?;
    if export.teq != *pinned {
        return Ok(Snapshot::Repinned);
    }
    let Some(project) = export.projects.get(name) else {
        return Ok(Snapshot::Gone(format!("no project {} in {} (projects: {})", name, file.display(), export.projects.keys().cloned().collect::<Vec<_>>().join(", "))));
    };
    if project.platform != Platform::Js || project.dev.is_none() {
        return Ok(Snapshot::Gone(format!("{} has no dev loop: dev is a Scala.js project's whose package.json has a dev script or that sets teqDevCommand", name)));
    }
    if let Some(why) = super::refused(&export, "dev", &[name]) {
        return Ok(Snapshot::Gone(why));
    }
    let value = lock::parse(&String::from_utf8_lossy(&bytes)).map_err(|e| format!("{}: {}", file.display(), e))?;
    let block = |v: Option<&lock::Value>| v.map_or(String::new(), lock::write);
    let mut read = block(value.get("repositories"));
    let mut projects: Vec<String> = export.closure(&Key::new(name, "compile")).into_iter().map(|k| k.project).collect();
    projects.push(name.to_string());
    projects.dedup();
    let mut jars = BTreeSet::new();
    for p in &projects {
        read.push_str(&block(value.at(&["projects", p])));
        for conf in export.projects.get(p).into_iter().flat_map(|p| p.configurations.values()) {
            jars.extend(conf.classpath.iter().filter_map(|e| if let Entry::Artifact(key) = e { Some(key.as_str()) } else { None }));
        }
    }
    // The records of the jar table the blocks name, which a jar re-pinned under its key changes.
    for key in jars {
        read.push_str(&block(value.at(&["jars", key])));
    }
    Ok(Snapshot::Reads(export, read))
}

fn digest(file: &Path) -> Option<String> {
    std::fs::read(file).ok().map(|bytes| sha256::hex(&bytes))
}

/// The record of the lockfile last installed, under the build's `target/teq/dev/`.
fn record_of(export: &Export, dev: &export::Dev) -> PathBuf {
    export.path(&format!("target/teq/dev/{}.sha256", dev.lockfile))
}

fn installed(export: &Export, dev: &export::Dev, lockfile: &Path, dir: &Path) -> bool {
    let Ok(bytes) = std::fs::read(lockfile) else { return false };
    dir.join("node_modules").is_dir() && std::fs::read_to_string(record_of(export, dev)).is_ok_and(|r| r.trim() == sha256::hex(&bytes))
}

/// The package manager's install, unless the lockfile is the one installed last; serialised
/// through `target/teq/dev/install.lock`, since two projects' loops may share a `package.json`.
fn install(export: &Export, dev: &export::Dev, lockfile: &Path, dir: &Path) -> Result<(), String> {
    if installed(export, dev, lockfile, dir) {
        return Ok(());
    }
    let record = record_of(export, dev);
    let lock_file = export.path("target/teq/dev/install.lock");
    if let Some(parent) = record.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {}", parent.display(), e))?;
    }
    let lock = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(&lock_file).map_err(|e| format!("cannot open {}: {}", lock_file.display(), e))?;
    lock.lock().map_err(|e| format!("cannot hold {}: {}", lock_file.display(), e))?;
    if installed(export, dev, lockfile, dir) {
        return Ok(());
    }
    let install = [dev.package_manager.clone(), "install".to_string()];
    eprintln!("teq dev: {} in {}", install.join(" "), dir.display());
    let status = shell_command(&install, &[]).current_dir(dir).status().map_err(|e| format!("cannot run {}: {}", install.join(" "), e))?;
    if !status.success() {
        return Err(format!("{} failed ({})", install.join(" "), status));
    }
    let bytes = std::fs::read(lockfile).map_err(|e| format!("{} wrote no {}: {}", install.join(" "), lockfile.display(), e))?;
    std::fs::write(&record, format!("{}\n", sha256::hex(&bytes))).map_err(|e| format!("cannot write {}: {}", record.display(), e))
}

/// A command of the package's, which on Windows is often a `.cmd` that only the shell runs.
fn shell_command(command: &[String], extra: &[String]) -> Command {
    let (program, args) = command.split_first().expect("a dev command is not empty");
    #[cfg(windows)]
    {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(program).args(args).args(extra);
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = Command::new(program);
        c.args(args).args(extra);
        c
    }
}

/// A file's size, modification time and inode, which change when it is written or replaced.
fn file_stamp(file: &Path) -> Option<(u64, SystemTime, u64)> {
    let meta = std::fs::metadata(file).ok()?;
    #[cfg(unix)]
    let inode = std::os::unix::fs::MetadataExt::ino(&meta);
    #[cfg(not(unix))]
    let inode = 0;
    Some((meta.len(), meta.modified().ok()?, inode))
}

