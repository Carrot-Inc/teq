//! The export of an sbt build the server runs by itself (docs/TARGETS.md, "The workspace"): an
//! sbt build without `teq.lock` gets `sbt teqExportAll` once, the file written where the build's
//! setting says: under its `target/teq/`, or at its root for the developer to commit with teq as
//! the build tool (`teqBuildTool`). A build that names sbt-teq itself runs its own plugin; any other
//! has sbt-teq added to that run alone through sbt's `-addPluginSbtFile`. A stale export is never
//! refreshed: that stays a warning naming the command.

use super::Event;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The sbt-teq the server adds to a build: the release of the plugin this compiler selects, its own
/// version line (`integrations/sbt/plugin-version.txt`, which build.rs bakes in; docs/TARGETS.md,
/// "Releases"), on Maven Central. Never the compiler's version: a compiler release that changes no
/// plugin publishes none.
const PLUGIN_VERSION: &str = env!("TEQ_PLUGIN_VERSION");
/// The compiler the export pins when the server adds the plugin: this binary's version, as the
/// default of the plugin's `teqVersion` (`TEQ_VERSION`), which a build's own setting overrides.
const COMPILER_VERSION: &str = env!("CARGO_PKG_VERSION");
/// How long an export may run before it is killed and told as a failure.
const BOUND: Duration = Duration::from_secs(15 * 60);
/// How much of sbt's output a failure's diagnostic carries, from its end.
const TAIL: usize = 4096;
/// How large sbt's output file may grow during the run before it is cut to its tail.
const OUTPUT_CAP: u64 = 4 << 20;

/// Whether a folder is an sbt build: `build.sbt` or `project/build.properties` at its root.
pub fn is_sbt_build(root: &Path) -> bool {
    root.join("build.sbt").is_file() || root.join("project/build.properties").is_file()
}

/// The sbt builds a workspace folder stands in: the folder itself, else the nearest ancestor
/// that is one within the folder's repository (up to the nearest directory holding `.git`; a
/// folder in no repository looks no higher than itself), else the builds found up to three
/// levels below it (not under `target`, `project`, `node_modules` or a hidden directory), in
/// order.
pub fn build_roots(folder: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for ancestor in folder.ancestors() {
        candidates.push(ancestor);
        if ancestor.join(".git").exists() {
            break;
        }
        if ancestor == folder && !folder.join(".git").exists() && !repository_above(folder) {
            break;
        }
    }
    if let Some(above) = candidates.iter().find(|a| is_sbt_build(a)) {
        return vec![above.to_path_buf()];
    }
    let mut found = Vec::new();
    builds_below(folder, 1, &mut found);
    found.sort();
    found
}

fn repository_above(folder: &Path) -> bool {
    folder.ancestors().skip(1).any(|a| a.join(".git").exists())
}

fn builds_below(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    if depth > 3 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !entry.file_type().map_or(false, |k| k.is_dir()) || name.starts_with('.') || name == "target" || name == "project" || name == "node_modules" {
            continue;
        }
        let path = entry.path();
        if is_sbt_build(&path) {
            out.push(path);
        } else {
            builds_below(&path, depth + 1, out);
        }
    }
}

/// The `sbt.version` of `project/build.properties`, when there is one.
pub fn sbt_version(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("project/build.properties")).ok()?;
    text.lines().find_map(|l| l.trim().strip_prefix("sbt.version").and_then(|r| r.trim().strip_prefix('=')).map(|v| v.trim().to_string()))
}

/// Whether the build names sbt-teq itself in `project/*.sbt`: its exports are the user's to run.
pub fn names_sbt_teq(root: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(root.join("project")) else { return false };
    entries.flatten().any(|e| {
        let path = e.path();
        path.extension().map_or(false, |x| x == "sbt") && std::fs::read_to_string(&path).map_or(false, |t| t.contains("sbt-teq"))
    })
}

/// Why a folder cannot be exported before anything runs, else nothing.
pub fn refusal(root: &Path) -> Option<String> {
    if !cfg!(unix) {
        return Some("the automatic export runs on macOS and Linux: export the build with sbt teqExportAll".to_string());
    }
    match sbt_version(root) {
        Some(v) if !v.starts_with("2.") => {
            return Some(format!("the automatic export needs sbt 2 (this build has sbt.version={}): add sbt-teq to the build and run sbt teqExportAll", v));
        }
        _ => {}
    }
    if sbt_on_path().is_none() {
        return Some("sbt is not on the PATH, which the automatic export runs: install it, or export the build with sbt teqExportAll".to_string());
    }
    None
}

fn sbt_on_path() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join("sbt")).find(|p| p.is_file())
}

/// The directory under the cache for the plugin file and the runs' output.
fn cache_dir() -> Result<PathBuf, String> {
    let dir = crate::jarcache::dir().ok_or("no cache directory for the sbt export")?.join("sbt");
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {}", dir.display(), e))?;
    Ok(dir)
}

/// The text of the plugin file for a release of sbt-teq: the plugin as Maven Central serves it, which
/// sbt resolves from with no resolver line. Which plugin the file names is this binary's version; the
/// build's own resolvers are the build's.
fn plugin_file_text(version: &str) -> String {
    format!("addSbtPlugin(\"build.teq\" % \"sbt-teq\" % \"{version}\")\n")
}

/// The plugin file sbt loads for the run, under the cache, written when missing or different
/// (whole, by a rename: another server may be reading it): `plugin_file_text` of this binary's
/// version.
pub fn plugin_file() -> Result<PathBuf, String> {
    let dir = cache_dir()?;
    let file = dir.join(format!("teq-{}.sbt", PLUGIN_VERSION));
    let text = plugin_file_text(PLUGIN_VERSION);
    if std::fs::read_to_string(&file).map_or(true, |t| t != text) {
        let partial = dir.join(format!("teq-{}.sbt.{}", PLUGIN_VERSION, std::process::id()));
        std::fs::write(&partial, text).and_then(|()| std::fs::rename(&partial, &file)).map_err(|e| format!("cannot write {}: {}", file.display(), e))?;
    }
    Ok(file)
}

#[cfg(unix)]
extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
    fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
    fn _exit(code: i32) -> !;
}
#[cfg(unix)]
const SIGKILL: i32 = 9;
#[cfg(unix)]
const SIGTERM: i32 = 15;

/// The process groups of the exports under way, for the signal handler to end: each sbt leads
/// its own group, so that the JVM the runner starts goes with it.
static GROUPS: [AtomicI32; 8] = [const { AtomicI32::new(0) }; 8];

fn end_group(pgid: i32) {
    #[cfg(unix)]
    if pgid > 0 {
        // SAFETY: a plain signal to a process group this server created.
        unsafe {
            kill(-pgid, SIGKILL);
        }
    }
    #[cfg(not(unix))]
    let _ = pgid;
}

#[cfg(unix)]
extern "C" fn on_termination(_sig: i32) {
    for slot in &GROUPS {
        end_group(slot.load(Ordering::Relaxed));
    }
    // SAFETY: the process is ending; nothing else runs after.
    unsafe { _exit(128 + SIGTERM) }
}

/// Has a termination signal end the exports under way before the server goes.
pub fn end_exports_on_termination() {
    // SAFETY: installs a handler that only signals process groups and exits.
    #[cfg(unix)]
    unsafe {
        signal(SIGTERM, on_termination);
    }
}

fn hold_group(pgid: i32) {
    for slot in &GROUPS {
        if slot.compare_exchange(0, pgid, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
            return;
        }
    }
}

fn release_group(pgid: i32) {
    for slot in &GROUPS {
        let _ = slot.compare_exchange(pgid, 0, Ordering::Relaxed, Ordering::Relaxed);
    }
}

/// An export under way: its sbt, which leads a process group, to end at exit, and the file its
/// output goes to.
pub struct Running {
    child: Arc<Mutex<Child>>,
    pgid: i32,
    log: PathBuf,
}

impl Running {
    /// Ends the sbt with every process it started and removes its output: the thread that
    /// would may not run again before the server exits.
    pub fn kill(&self) {
        let mut c = self.child.lock().unwrap_or_else(|e| e.into_inner());
        end_group(self.pgid);
        let _ = c.kill();
        let _ = c.wait();
        release_group(self.pgid);
        let _ = std::fs::remove_file(&self.log);
    }
}

/// Starts `sbt --server --batch [-addPluginSbtFile=<file>] teqExportAll` in `root`, the plugin
/// added unless the build names its own (and then `TEQ_VERSION` this compiler's version, which the
/// export pins unless the build sets `teqVersion`), with `TEQ` the server's executable, and tells the loop
/// how it ended: `Event::Exported(folder, build, ok, tail)`, the tail the end of what sbt wrote
/// on both streams. The output goes to a file under the cache rather than a pipe, so that a
/// process sbt leaves behind holding the stream keeps no thread of the server waiting, and the
/// file is cut to its tail whenever it passes the cap.
pub fn start(folder: usize, build: usize, root: &Path, exe: &Path, events: Sender<Event>) -> Result<Running, String> {
    let sbt = sbt_on_path().ok_or("sbt is not on the PATH")?;
    let log = cache_dir()?.join(format!("export-{}-{}-{}.log", std::process::id(), folder, build));
    let out = OpenOptions::new().create(true).write(true).append(true).truncate(false).open(&log).map_err(|e| format!("cannot write {}: {}", log.display(), e))?;
    let err = out.try_clone().map_err(|e| format!("cannot write {}: {}", log.display(), e))?;
    let mut command = Command::new(sbt);
    command.arg("--server").arg("--batch");
    if !names_sbt_teq(root) {
        command.arg(format!("-addPluginSbtFile={}", plugin_file()?.display())).env("TEQ_VERSION", COMPILER_VERSION);
    }
    command.arg("teqExportAll").current_dir(root).env("TEQ", exe).stdin(Stdio::null()).stdout(Stdio::from(out)).stderr(Stdio::from(err));
    #[cfg(unix)]
    command.process_group(0);
    let child = command.spawn().map_err(|e| format!("cannot start sbt: {}", e))?;
    let pgid = child.id() as i32;
    hold_group(pgid);
    let child = Arc::new(Mutex::new(child));
    let waited = child.clone();
    let running = Running { child, pgid, log: log.clone() };
    crate::alloc::spawn(move || {
        let until = Instant::now() + BOUND;
        let mut killed = false;
        let ok = loop {
            let mut c = waited.lock().unwrap_or_else(|e| e.into_inner());
            match c.try_wait() {
                Ok(Some(status)) => break status.success(),
                Ok(None) if Instant::now() >= until => {
                    end_group(pgid);
                    let _ = c.kill();
                    let _ = c.wait();
                    killed = true;
                    break false;
                }
                Ok(None) => {}
                Err(_) => break false,
            }
            drop(c);
            cap_output(&log);
            std::thread::sleep(Duration::from_millis(200));
        };
        release_group(pgid);
        let mut text = tail_of(&log);
        if killed {
            text.push_str("\nkilled after 15 minutes");
        }
        let _ = std::fs::remove_file(&log);
        let _ = events.send(Event::Exported(folder, build, ok, text.trim().to_string()));
    });
    Ok(running)
}

/// Cuts the output file to its tail once it passes the cap; sbt appends, so its next line
/// follows what is kept.
fn cap_output(log: &Path) {
    let Ok(len) = std::fs::metadata(log).map(|m| m.len()) else { return };
    if len <= OUTPUT_CAP {
        return;
    }
    let tail = tail_of(log);
    if let Ok(mut file) = OpenOptions::new().write(true).truncate(true).open(log) {
        let _ = file.write_all(tail.as_bytes());
    }
}

/// The end of a file's text, `TAIL` bytes at most, however much a process still writing to it
/// adds meanwhile.
fn tail_of(log: &Path) -> String {
    let Ok(mut file) = File::open(log) else { return String::new() };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    let from = len.saturating_sub(TAIL as u64);
    let _ = file.seek(SeekFrom::Start(from));
    let mut bytes = Vec::new();
    let _ = file.take(len - from).read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sbt_version_of_a_build() {
        let dir = std::env::temp_dir().join(format!("teq-export-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("project")).unwrap();
        std::fs::write(dir.join("project/build.properties"), "# the build\nsbt.version = 2.0.8\n").unwrap();
        assert_eq!(sbt_version(&dir).as_deref(), Some("2.0.8"));
        assert!(is_sbt_build(&dir));
        assert_eq!(anchor(&dir), dir.join("project/build.properties"));
        std::fs::write(dir.join("build.sbt"), "name := \"x\"\n").unwrap();
        assert_eq!(anchor(&dir), dir.join("build.sbt"));
        assert!(!names_sbt_teq(&dir));
        std::fs::write(dir.join("project/plugins.sbt"), "addSbtPlugin(\"build.teq\" % \"sbt-teq\" % \"0.1.6\")\n").unwrap();
        assert!(names_sbt_teq(&dir));
        std::fs::write(dir.join("project/build.properties"), "sbt.version=1.10.0\n").unwrap();
        // Windows refuses every build before its version, naming the export to run by hand.
        let refused = refusal(&dir).unwrap();
        assert!(if cfg!(unix) { refused.contains("sbt.version=1.10.0") } else { refused.contains("sbt teqExportAll") }, "{refused}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_plugin_file_of_a_release() {
        // Every version, an old release's among them, is build.teq's on Maven Central, with no resolver.
        for version in ["0.1.0-pre.2", "0.1.6", "1.0.0", "1.2.3", "0.1.1-branch-SNAPSHOT"] {
            assert_eq!(plugin_file_text(version), format!("addSbtPlugin(\"build.teq\" % \"sbt-teq\" % \"{version}\")\n"));
        }
        assert!(!plugin_file_text(PLUGIN_VERSION).contains("resolvers"));
    }

    #[test]
    fn the_builds_a_folder_stands_in() {
        let dir = std::env::temp_dir().join(format!("teq-roots-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for d in ["mono/.git", "mono/backend/project", "mono/frontend/src", "mono/tools/deep/er/build", "mono/target/x", "mono/.hidden/y", "loose/sub"] {
            std::fs::create_dir_all(dir.join(d)).unwrap();
        }
        std::fs::write(dir.join("mono/backend/build.sbt"), "").unwrap();
        std::fs::write(dir.join("mono/tools/deep/er/build/build.sbt"), "").unwrap();
        std::fs::write(dir.join("mono/target/x/build.sbt"), "").unwrap();
        std::fs::write(dir.join("mono/.hidden/y/build.sbt"), "").unwrap();
        std::fs::write(dir.join("loose/build.sbt"), "").unwrap();
        assert_eq!(build_roots(&dir.join("mono")), vec![dir.join("mono/backend")]);
        assert_eq!(build_roots(&dir.join("mono/backend/project")), vec![dir.join("mono/backend")]);
        assert_eq!(build_roots(&dir.join("mono/frontend")), Vec::<PathBuf>::new());
        // No repository above: the folder looks no higher than itself. The temporary directory may lie in a
        // repository the test does not own (a `.git` above it), where the folder looks up to that repository's root.
        let above = repository_above(&dir.join("loose/sub"));
        assert_eq!(build_roots(&dir.join("loose/sub")), if above { vec![dir.join("loose")] } else { Vec::new() });
        assert_eq!(build_roots(&dir.join("loose")), vec![dir.join("loose")]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// The file a failure's diagnostic goes on: `build.sbt`, else `project/build.properties`.
pub fn anchor(root: &Path) -> PathBuf {
    let build = root.join("build.sbt");
    if build.is_file() { build } else { root.join("project/build.properties") }
}
