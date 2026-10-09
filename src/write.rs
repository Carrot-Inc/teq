//! Writing the output: one file, or the directory of a split build, where a module is only
//! rewritten when its text differs from the file on disk and modules of an earlier build that no
//! longer exist are removed. The directory is teq's: every `.mjs` file in it is taken to be one.
//!
//! A build under watch is published in two steps: every file that changed is written to a
//! temporary name of the session's own, then, once the session has looked for the process it
//! writes for once more, the temporaries are renamed into place as one batch and what is stale
//! is removed, so that a reader never sees a partial file and a session whose owner is gone puts
//! nothing in place.

use crate::emit::{hot_build, Modules, HASH_LEN, HOT_BUILD, ID_AFTER_HASH};
use crate::intern::FxMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// The process that started a watch session, for which the session writes.
static OWNER: std::sync::OnceLock<u32> = std::sync::OnceLock::new();

#[cfg(unix)]
pub(crate) fn parent() -> u32 {
    std::os::unix::process::parent_id()
}

#[cfg(windows)]
pub(crate) use owner::parent;

/// A watch session writes for the process that started it, which may hold the directory for
/// it (sbt-teq's lock). That process ends a session in order by closing its input; when it
/// is killed in the middle of a build, the build would still be written, into a directory
/// that may be another writer's by then. So from this call on a build is published only while
/// the process that started the session is there, and the session ends without writing when
/// it is not.
pub fn for_parent() {
    let _ = OWNER.set(parent());
    #[cfg(windows)]
    owner::hold();
}

/// On Unix a process whose parent ends is given another, so a parent that differs from the
/// session's first is gone.
#[cfg(unix)]
fn orphaned() -> bool {
    OWNER.get().is_some_and(|&owner| owner != parent())
}

#[cfg(windows)]
fn orphaned() -> bool {
    owner::gone()
}

/// Windows keeps the pid of a process's parent as it was when the process started, whether the
/// parent still runs or the pid has gone to another process since: the session holds a handle
/// on its parent, which keeps the pid from being reused, and the parent is gone once the handle
/// is signalled.
#[cfg(windows)]
mod owner {
    use std::ffi::c_void;
    use std::sync::OnceLock;

    /// `PROCESSENTRY32W`.
    #[repr(C)]
    struct ProcessEntry {
        size: u32,
        usage: u32,
        pid: u32,
        heap: usize,
        module: u32,
        threads: u32,
        parent: u32,
        priority: i32,
        flags: u32,
        exe: [u16; 260],
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> *mut c_void;
        fn Process32FirstW(snapshot: *mut c_void, entry: *mut ProcessEntry) -> i32;
        fn Process32NextW(snapshot: *mut c_void, entry: *mut ProcessEntry) -> i32;
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn GetCurrentProcess() -> *mut c_void;
        fn GetProcessTimes(process: *mut c_void, creation: *mut u64, exit: *mut u64, kernel: *mut u64, user: *mut u64) -> i32;
        fn WaitForSingleObject(handle: *mut c_void, ms: u32) -> u32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    const TH32CS_SNAPPROCESS: u32 = 2;
    const INVALID_HANDLE_VALUE: *mut c_void = -1isize as *mut c_void;
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const ERROR_INVALID_PARAMETER: i32 = 87;
    const WAIT_OBJECT_0: u32 = 0;

    #[derive(Clone, Copy)]
    enum Parent {
        /// The parent's handle, as an address.
        Held(usize),
        /// Gone before the session began: no process has the pid, or the one that has it now
        /// started after this one.
        Gone,
        /// Not opened, the system refusing: its end goes unseen, as on Unix that of a parent
        /// gone before the session began.
        Unknown,
    }

    static PARENT: OnceLock<Parent> = OnceLock::new();

    /// The parent's pid as the system's list of processes records it; zero where the list
    /// cannot be read.
    pub fn parent() -> u32 {
        let own = std::process::id();
        // SAFETY: a snapshot taken, walked by entries of the size it is told, and closed here.
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return 0;
            }
            let mut entry: ProcessEntry = std::mem::zeroed();
            entry.size = std::mem::size_of::<ProcessEntry>() as u32;
            let mut more = Process32FirstW(snapshot, &mut entry) != 0;
            let mut parent = 0;
            while more {
                if entry.pid == own {
                    parent = entry.parent;
                    break;
                }
                more = Process32NextW(snapshot, &mut entry) != 0;
            }
            CloseHandle(snapshot);
            parent
        }
    }

    /// Takes the handle on the parent, for the rest of the process.
    pub fn hold() {
        PARENT.get_or_init(|| {
            let pid = parent();
            if pid == 0 {
                return Parent::Unknown;
            }
            // SAFETY: a handle opened here and kept for the process's life, the times written
            // by the queries.
            unsafe {
                let process = OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
                if process.is_null() {
                    let none = std::io::Error::last_os_error().raw_os_error() == Some(ERROR_INVALID_PARAMETER);
                    return if none { Parent::Gone } else { Parent::Unknown };
                }
                let (mut theirs, mut ours, mut unused) = (0u64, 0u64, [0u64; 3]);
                let [exit, kernel, user] = &mut unused;
                let timed = GetProcessTimes(process, &mut theirs, exit, kernel, user) != 0 && GetProcessTimes(GetCurrentProcess(), &mut ours, exit, kernel, user) != 0;
                if timed && theirs > ours {
                    CloseHandle(process);
                    return Parent::Gone;
                }
                Parent::Held(process as usize)
            }
        });
    }

    pub fn gone() -> bool {
        match PARENT.get() {
            // SAFETY: the handle `hold` keeps open; a wait of zero returns at once.
            Some(Parent::Held(process)) => unsafe { WaitForSingleObject(*process as *mut c_void, 0) == WAIT_OBJECT_0 },
            Some(Parent::Gone) => true,
            Some(Parent::Unknown) | None => false,
        }
    }
}

fn leave_if_orphaned() {
    if orphaned() {
        eprintln!("teq compiler watch: the process that started the session is gone: nothing is written");
        std::process::exit(0);
    }
}

/// `fs::rename`, which on Windows fails while another process holds the target or the source
/// open without sharing its deletion (a JVM reading a class file or a jar): tried again while the
/// system says the file is in use, as sbt's `IO.move` is (sbt io's `Retry`, ten tries 100 ms
/// apart), the error past them naming the file; elsewhere `fs::rename` itself.
pub(crate) fn rename(from: &Path, to: &Path) -> io::Result<()> {
    const ERROR_ACCESS_DENIED: i32 = 5;
    const ERROR_SHARING_VIOLATION: i32 = 32;
    const ERROR_LOCK_VIOLATION: i32 = 33;
    let mut tries = 1;
    loop {
        match fs::rename(from, to) {
            Err(e) if cfg!(windows) && matches!(e.raw_os_error(), Some(ERROR_ACCESS_DENIED | ERROR_SHARING_VIOLATION | ERROR_LOCK_VIOLATION)) => {
                if tries == 10 {
                    return Err(io::Error::new(e.kind(), format!("{} not replaced in ten tries 100 ms apart, another process may hold it open: {}", to.display(), e)));
                }
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            done => return done,
        }
    }
}

/// A file staged under a temporary name of this process's own, to be renamed into place.
struct Staged {
    name: String,
    tmp: PathBuf,
    path: PathBuf,
}

fn stage(dir: &Path, name: &str, path: PathBuf, bytes: &[u8]) -> io::Result<Staged> {
    let tmp = dir.join(format!("{}.{}.tmp", name, std::process::id()));
    if let Some(parent) = tmp.parent() {
        if !parent.is_dir() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(&tmp, bytes)?;
    Ok(Staged { name: name.to_string(), tmp, path })
}

/// Puts the staged files in place, in their order, after `remove` is gone; a session whose
/// owner has gone since the files were staged removes them instead and leaves. What an
/// orphan can still do is the batch of renames it had begun, `TEQ_PUBLISH_GATE` (a file a
/// test names, which the session waits for here) lets a test hold a session at this point.
fn publish(staged: Vec<Staged>, remove: &[PathBuf]) -> io::Result<Vec<String>> {
    if let Ok(gate) = std::env::var("TEQ_PUBLISH_GATE") {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !Path::new(&gate).exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    if orphaned() {
        for file in &staged {
            let _ = fs::remove_file(&file.tmp);
        }
        eprintln!("teq compiler watch: the process that started the session is gone: the build is not put in place");
        std::process::exit(0);
    }
    for path in remove {
        if let Err(e) = fs::remove_file(path) {
            if e.kind() != io::ErrorKind::NotFound {
                return Err(e);
            }
        }
    }
    let mut names = Vec::with_capacity(staged.len());
    for file in staged {
        rename(&file.tmp, &file.path)?;
        names.push(file.name);
    }
    Ok(names)
}

pub fn file(path: &str, text: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(path, text)
}

/// Writes the modules that changed and returns their names.
pub fn modules(dir: &str, modules: &mut Modules) -> io::Result<Vec<String>> {
    modules_after(dir, modules, &FxMap::default())
}

/// The same for a session that wrote `previous` (module name to text) last: a module whose text
/// is what was written then is checked to be on disk and not read back.
///
/// Under `--hot` a module's footer names the text before it by its hash and the build that
/// wrote the module by its id (`emit::Modules`), both filled in here: a text that is the last
/// build's but for the two keeps what that build gave it, and only a text that differs is
/// hashed and gets this build's id, which is the hash of every module's hash in the order of
/// the listing. `hot-build.mjs`, the listing of every module's hash and writer under the id,
/// joins the modules; whatever the build, the last file of the list is put in place when
/// every other is, so that the listing ends the publication.
pub fn modules_after(dir: &str, modules: &mut Modules, previous: &FxMap<String, String>) -> io::Result<Vec<String>> {
    let mut lap = crate::measure::Lap::resume(crate::measure::Pass::After);
    leave_if_orphaned();
    let dir = Path::new(dir);
    fs::create_dir_all(dir)?;
    let hot = !modules.footers.is_empty();
    // Modules of an earlier build that this one does not have, and temporaries a session left
    // behind: removed when the build is put in place.
    let mut stale: Vec<PathBuf> = Vec::new();
    {
        let files = &modules.files;
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let kept = files.iter().any(|(m, _)| *m == name) || (hot && name == HOT_BUILD);
            if name.ends_with(".tmp") || (name.ends_with(".mjs") && !kept) {
                stale.push(path);
            }
        }
    }
    lap.done("write: directory listed", stale.len());
    let staged: Mutex<Vec<(usize, Staged)>> = Mutex::new(Vec::new());
    let failure: Mutex<Option<io::Error>> = Mutex::new(None);
    // Stages the file unless it is on disk as it is; `same` when the text is the file's.
    let stage_text = |i: usize, name: &str, text: &str, same: bool| {
        let path = dir.join(name);
        let on_disk = if same { path.is_file() } else { fs::read(&path).map_or(false, |old| old == text.as_bytes()) };
        if on_disk {
            return;
        }
        match stage(dir, name, path, text.as_bytes()) {
            Ok(file) => staged.lock().unwrap().push((i, file)),
            Err(e) => *failure.lock().unwrap() = Some(e),
        }
    };
    let footers = &modules.footers;
    // Without a listing the last file waits for the others as well.
    let ahead = if hot { modules.files.len() } else { modules.files.len().saturating_sub(1) };
    let (first, last) = modules.files.split_at_mut(ahead);
    let workers = crate::workers().min(first.len()).max(1);
    let in_parallel = |work: &(dyn Fn() + Sync)| {
        if workers == 1 {
            work();
        } else {
            // Joined through their handles, so that no worker drops the last reference to its
            // own handle after its memory went back to the allocator (`alloc.rs`).
            std::thread::scope(|scope| {
                let handles: Vec<_> = (0..workers).map(|_| crate::alloc::spawn_in(scope, work)).collect();
                for handle in handles {
                    handle.join().expect("writer thread panicked");
                }
            });
        }
    };
    // Which texts are the last build's, hash and id apart (a build without a last one, over
    // an earlier build's directory, reads the file there): those keep both; the others get
    // their hash now and the build's id once every hash is known.
    let same: Vec<Mutex<bool>> = first.iter().map(|_| Mutex::new(false)).collect();
    let next = Mutex::new(first.iter_mut().enumerate());
    in_parallel(&|| loop {
        let Some((i, (name, text))) = next.lock().unwrap().next() else { break };
        let kept = match footers.get(i) {
            Some(&(body, at)) => {
                let on_disk;
                let old = match previous.get(name.as_str()) {
                    Some(old) => Some(old.as_str()),
                    None => {
                        on_disk = fs::read_to_string(dir.join(name.as_str())).ok();
                        on_disk.as_deref()
                    }
                };
                let after = at + ID_AFTER_HASH + HASH_LEN;
                let kept = old.map_or(false, |old| old.len() == text.len() && old[..at] == text[..at] && old[after..] == text[after..]);
                match old {
                    Some(old) if kept => text.replace_range(at..after, &old[at..after]),
                    _ => text.replace_range(at..at + HASH_LEN, &format!("{:016x}", hash(text[..body].as_bytes()))),
                }
                kept
            }
            None => previous.get(name.as_str()).map_or(false, |old| old == text),
        };
        *same[i].lock().unwrap() = kept;
    });
    drop(next);
    let same: Vec<bool> = same.into_iter().map(|s| s.into_inner().unwrap()).collect();
    lap.done("write: compared", same.iter().filter(|&&s| !s).count());
    let listing = if hot {
        let mut hashes: Vec<(&str, &str)> = first
            .iter()
            .zip(footers)
            .map(|((name, text), &(_, at))| (name.strip_suffix(".mjs").unwrap_or(name), &text[at..at + HASH_LEN]))
            .collect();
        hashes.sort_unstable();
        let mut listed = String::new();
        for (name, hash) in &hashes {
            listed.push_str(name);
            listed.push(' ');
            listed.push_str(hash);
            listed.push('\n');
        }
        let id = format!("{:016x}", hash(listed.as_bytes()));
        for ((_, text), (&(_, at), &kept)) in first.iter_mut().zip(footers.iter().zip(&same)) {
            if !kept {
                text.replace_range(at + ID_AFTER_HASH..at + ID_AFTER_HASH + HASH_LEN, &id);
            }
        }
        let mut written: Vec<(&str, &str, &str)> = first
            .iter()
            .zip(footers)
            .map(|((name, text), &(_, at))| (name.strip_suffix(".mjs").unwrap_or(name), &text[at..at + HASH_LEN], &text[at + ID_AFTER_HASH..at + ID_AFTER_HASH + HASH_LEN]))
            .collect();
        written.sort_unstable();
        Some(hot_build(&id, &written))
    } else {
        None
    };
    let next = Mutex::new(first.iter().enumerate());
    in_parallel(&|| loop {
        let Some((i, (name, text))) = next.lock().unwrap().next() else { break };
        if failure.lock().unwrap().is_some() {
            break;
        }
        stage_text(i, name.as_str(), text.as_str(), same[i]);
    });
    drop(next);
    if failure.lock().unwrap().is_none() {
        if let Some(listing) = listing {
            let same = previous.get(HOT_BUILD).map_or(false, |old| *old == listing);
            stage_text(usize::MAX, HOT_BUILD, &listing, same);
            modules.files.push((HOT_BUILD.to_string(), listing));
        } else if let Some((name, text)) = last.first() {
            let same = previous.get(name.as_str()).map_or(false, |old| old == text);
            stage_text(usize::MAX, name.as_str(), text.as_str(), same);
        }
    }
    let mut staged = staged.into_inner().unwrap();
    if let Some(e) = failure.into_inner().unwrap() {
        for (_, file) in &staged {
            let _ = fs::remove_file(&file.tmp);
        }
        return Err(e);
    }
    lap.done("write: staged", staged.len());
    staged.sort_by_key(|(i, _)| *i);
    let mut written = publish(staged.into_iter().map(|(_, file)| file).collect(), &stale)?;
    written.sort();
    lap.done("write: published", written.len());
    Ok(written)
}

/// The hash a module's footer and `hot-build.mjs` name its text by, and the id of a build is
/// made of, as a module's products' digests and identity are (`products.rs`): eight bytes at a
/// time, the same on every machine.
pub(crate) fn hash(bytes: &[u8]) -> u64 {
    const K: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut h: u64 = bytes.len() as u64;
    let mut words = bytes.chunks_exact(8);
    for word in &mut words {
        h = (h.rotate_left(27) ^ u64::from_le_bytes(word.try_into().unwrap())).wrapping_mul(K);
    }
    let mut tail = [0u8; 8];
    tail[..words.remainder().len()].copy_from_slice(words.remainder());
    h = (h.rotate_left(27) ^ u64::from_le_bytes(tail)).wrapping_mul(K);
    h ^ (h >> 32)
}

/// Puts back the modules of a session's last build (`written`, module name to text) that are no
/// longer in `dir`, and returns their names: another tool removed a file, or the directory,
/// since.
pub fn modules_missing(dir: &str, written: &FxMap<String, String>) -> io::Result<Vec<String>> {
    leave_if_orphaned();
    let dir = Path::new(dir);
    fs::create_dir_all(dir)?;
    let mut missing: Vec<&String> = written.keys().filter(|name| !dir.join(name).is_file()).collect();
    // The listing of a `--hot` build after the modules it lists, as a build publishes it.
    missing.sort_by_key(|name| (name.as_str() == HOT_BUILD, name.as_str()));
    let mut staged = Vec::with_capacity(missing.len());
    for name in &missing {
        match stage(dir, name, dir.join(name), written[*name].as_bytes()) {
            Ok(file) => staged.push(file),
            Err(e) => {
                for file in &staged {
                    let _ = fs::remove_file(&file.tmp);
                }
                return Err(e);
            }
        }
    }
    let mut names = publish(staged, &[])?;
    names.sort();
    Ok(names)
}

/// Writes the class files of a JVM build under watch into `dir` and returns the paths written
/// relative to `dir` (`p/Foo.class`). `previous` holds a hash of each
/// class written by the session so far, with its bytes: a class whose bytes are what was written
/// last is checked to be on disk and not read back, so a build that changed nothing still
/// restores what another tool deleted, and a class whose bytes are the very ones written last (a
/// class file the session kept, `jvm::kept`) is not hashed again. Nothing is ever deleted: the class files of a class that is gone are the
/// caller's to remove.
pub fn classes_after(dir: &str, classes: &[crate::jvm::Emitted], previous: &mut FxMap<String, (u64, std::sync::Arc<Vec<u8>>)>) -> io::Result<(Vec<String>, Vec<String>)> {
    let mut lap = crate::measure::Lap::resume(crate::measure::Pass::After);
    leave_if_orphaned();
    let dir = Path::new(dir);
    fs::create_dir_all(dir)?;
    let staged: Mutex<Vec<(Staged, u64, usize)>> = Mutex::new(Vec::new());
    let checked: Mutex<Vec<(String, u64, usize)>> = Mutex::new(Vec::new());
    let next = AtomicUsize::new(0);
    // The class files of this build the previous one wrote: when they are all it wrote, none goes.
    let listed = AtomicUsize::new(0);
    let failure: Mutex<Option<io::Error>> = Mutex::new(None);
    let stage_next = || loop {
        let i = next.fetch_add(1, Ordering::Relaxed);
        let Some(class) = classes.get(i) else { break };
        let name = format!("{}.class", class.name);
        let path = dir.join(&name);
        let last = previous.get(&name);
        if last.is_some() {
            listed.fetch_add(1, Ordering::Relaxed);
        }
        let h = match last {
            Some((h, bytes)) if std::sync::Arc::ptr_eq(bytes, &class.bytes) => *h,
            _ => hash(&class.bytes),
        };
        let known = last.map(|&(h, _)| h);
        let on_disk = match known {
            Some(old) if old == h => path.is_file(),
            _ => fs::read(&path).map_or(false, |old| old == *class.bytes),
        };
        if on_disk {
            if last.is_none_or(|(old, bytes)| *old != h || !std::sync::Arc::ptr_eq(bytes, &class.bytes)) {
                checked.lock().unwrap().push((name, h, i));
            }
            continue;
        }
        match stage(dir, &name, path, class.bytes.as_slice()) {
            Ok(file) => staged.lock().unwrap().push((file, h, i)),
            Err(e) => {
                *failure.lock().unwrap() = Some(e);
                break;
            }
        }
    };
    let workers = crate::workers().min(classes.len()).max(1);
    if workers == 1 {
        stage_next();
    } else {
        // Joined through their handles, so that no worker drops the last reference to its own
        // handle after its memory went back to the allocator (`alloc.rs`).
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers).map(|_| crate::alloc::spawn_in(scope, &stage_next)).collect();
            for handle in handles {
                handle.join().expect("writer thread panicked");
            }
        });
    }
    let staged = staged.into_inner().unwrap();
    if let Some(e) = failure.into_inner().unwrap() {
        for (file, _, _) in &staged {
            let _ = fs::remove_file(&file.tmp);
        }
        return Err(e);
    }
    lap.done("write: hashed, compared and staged", staged.len());
    lap.count("writer threads", workers);
    // A class file of the previous build that this one has no class for (a class deleted or
    // renamed, an anonymous class whose position moved) goes, as the build tool's class-file
    // manager would delete a product the compiler no longer writes.
    let mut deleted: Vec<String> = Vec::new();
    if listed.load(Ordering::Relaxed) < previous.len() {
        let current: std::collections::HashSet<String> = classes.iter().map(|c| format!("{}.class", c.name)).collect();
        deleted = previous.keys().filter(|n| !current.contains(*n)).cloned().collect();
    }
    deleted.sort();
    let hashes: Vec<(String, u64, usize)> = staged.iter().map(|(file, h, i)| (file.name.clone(), *h, *i)).collect();
    let remove: Vec<PathBuf> = deleted.iter().map(|name| dir.join(name)).collect();
    let mut written = publish(staged.into_iter().map(|(file, _, _)| file).collect(), &remove)?;
    for name in &deleted {
        previous.remove(name);
    }
    for (name, h, i) in checked.into_inner().unwrap().into_iter().chain(hashes) {
        previous.insert(name, (h, classes[i].bytes.clone()));
    }
    written.sort();
    lap.done("write: published", written.len() + deleted.len());
    Ok((written, deleted))
}
