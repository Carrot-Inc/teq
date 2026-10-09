//! The module-product mode, `--products <dir>`: what a build writes for the modules that depend
//! on it to be compiled against, as scalac writes a project's class directory. Every definition
//! of the owned sources is kept (the API is complete, open world), nothing the class path
//! holds is written again, and beside the class files each top-level class has its `.tasty`,
//! the class files carrying the `TASTY` attribute (the file's UUID) and the `Scala` marker as
//! `GenBCode` writes them. `teq-products.json` lists each product with its owning source, its
//! class files, its `.tasty`, whether the `.tasty` holds bodies, its objects that initialise and
//! the std classes its class files name, a digest of its files, and the std classes the
//! directory holds, which a downstream build writes again for what it reaches rather than taking
//! an upstream's (docs/TARGETS.md, "Libraries from jars"); its first field, `identity`, is the
//! hash of the rest, which a session compares to see the directory change. A build publishes a
//! directory's products through a staging directory beside it, the manifest last
//! (`Publication`; docs/TARGETS.md, "The contract between the plugin and the compiler").

use crate::lsp::json::Json;
use std::path::{Path, PathBuf};

pub const MANIFEST: &str = "teq-products.json";

/// One product: a `.tasty` and the class files of its classes, or class files of a source
/// with no `.tasty` of their own (an entry point's launcher).
#[derive(Clone)]
pub struct Entry {
    pub source: String,
    /// The owning source's text as the build typed it, by `fingerprint`: what a reader of the
    /// products checks the source against before it locates a declaration there.
    pub fingerprint: Option<String>,
    pub tasty: Option<String>,
    pub classes: Vec<String>,
    pub bodies: bool,
    /// The module classes of the product's objects whose initialisation runs code (a field, a
    /// statement, a parent's constructor): a JVM downstream leaves out loading another object
    /// where it is only evaluated, as the whole program does.
    pub inits: Vec<String>,
    /// The std classes the product's class files name, which the directory holds while a
    /// product names them.
    pub std: Vec<String>,
    /// The hash of the product's files, their paths and bytes in the entry's order.
    pub digest: String,
}

impl Entry {
    /// The entry's files, its `.tasty` first, as the directory holds them.
    pub fn files(&self) -> impl Iterator<Item = &str> {
        self.tasty.iter().map(String::as_str).chain(self.classes.iter().map(String::as_str))
    }
}

/// A text's length and its 64-bit FNV-1a hash: `<length>:<hash in hex>`.
pub fn fingerprint(text: &str) -> String {
    let h = text.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3));
    format!("{}:{:016x}", text.len(), h)
}

pub struct Manifest {
    pub target: String,
    /// The producer's working directory, which a relative owning source is relative to; a reader
    /// in another reads a source under it under its own (`classpath::entry_source`).
    pub root: String,
    pub entries: Vec<Entry>,
    /// The std classes the directory holds: those its products name and those they name in
    /// turn, each written whole (`docs/TARGETS.md`, "A module's products").
    pub std: Vec<String>,
    /// A manifest before the entries held their `inits`, read alone.
    pub legacy_inits: Vec<String>,
}

/// Entries in a manifest's order: by source, then by `.tasty`, a source's launcher (no `.tasty`)
/// after its products.
pub fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|a, b| (&a.source, a.tasty.is_none(), &a.tasty, &a.classes).cmp(&(&b.source, b.tasty.is_none(), &b.tasty, &b.classes)));
}

/// A digest of files, `(path, bytes)` in their order: 16 hex digits.
pub fn digest<'b>(files: impl Iterator<Item = (&'b str, &'b [u8])>) -> String {
    let mut h = 0u64;
    for (path, bytes) in files {
        h = (h.rotate_left(13) ^ crate::write::hash(path.as_bytes())).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        h = (h.rotate_left(13) ^ crate::write::hash(bytes)).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }
    format!("{:016x}", h ^ (h >> 32))
}


impl Manifest {
    /// The module classes whose initialisation runs code, of every entry.
    pub fn inits(&self) -> impl Iterator<Item = &String> {
        self.entries.iter().flat_map(|e| e.inits.iter()).chain(self.legacy_inits.iter())
    }

    /// The manifest's text: the entries sorted by source, then by `.tasty` (a source's launcher,
    /// without one, last), so that a build of part of a module's sources and a build of all of
    /// them write the same text; `identity` first.
    pub fn to_json(&self) -> String {
        let entries: Vec<Json> = self
            .entries
            .iter()
            .map(|e| {
                Json::Obj(vec![
                    ("source".to_string(), Json::Str(e.source.clone())),
                    ("sourceFingerprint".to_string(), e.fingerprint.clone().map_or(Json::Null, Json::Str)),
                    ("tasty".to_string(), e.tasty.clone().map_or(Json::Null, Json::Str)),
                    ("classes".to_string(), Json::Arr(e.classes.iter().cloned().map(Json::Str).collect())),
                    ("bodies".to_string(), Json::Bool(e.bodies)),
                    ("inits".to_string(), Json::Arr(e.inits.iter().cloned().map(Json::Str).collect())),
                    ("std".to_string(), Json::Arr(e.std.iter().cloned().map(Json::Str).collect())),
                    ("digest".to_string(), Json::Str(e.digest.clone())),
                ])
            })
            .collect();
        let json = Json::Obj(vec![
            ("teq".to_string(), Json::Str(env!("CARGO_PKG_VERSION").to_string())),
            ("target".to_string(), Json::Str(self.target.clone())),
            ("root".to_string(), Json::Str(self.root.clone())),
            ("products".to_string(), Json::Arr(entries)),
            ("std".to_string(), Json::Arr(self.std.iter().cloned().map(Json::Str).collect())),
        ]);
        let rest = json.to_text();
        format!("{{\"identity\":\"{:016x}\",{}\n", crate::write::hash(rest.as_bytes()), &rest[1..])
    }

    /// Puts the entries in the manifest's order.
    pub fn sort(&mut self) {
        sort_entries(&mut self.entries);
    }

    /// The identity of the manifest of `dir`, from the first bytes of its text: what a session
    /// compares at every request to see the directory change.
    pub fn identity_in(dir: &Path) -> Option<String> {
        use std::io::Read as _;
        let mut head = [0u8; 32];
        let n = std::fs::File::open(dir.join(MANIFEST)).and_then(|mut f| f.read(&mut head)).ok()?;
        let rest = std::str::from_utf8(&head[..n]).ok()?.strip_prefix("{\"identity\":\"")?;
        rest.get(..16).map(str::to_string)
    }

    pub fn read(dir: &Path) -> Option<Result<Manifest, String>> {
        let path = dir.join(MANIFEST);
        let text = std::fs::read_to_string(&path).ok()?;
        Some(Manifest::parse(&text).map_err(|e| format!("{}: {}", path.display(), e)))
    }

    pub fn parse(text: &str) -> Result<Manifest, String> {
        let json = Json::parse(text)?;
        let strings = |j: &Json, key: &str| j.get(key).map_or(Vec::new(), |s| s.arr().iter().filter_map(Json::str).map(str::to_string).collect::<Vec<String>>());
        let target = json.get("target").and_then(Json::str).unwrap_or("jvm").to_string();
        let root = json.get("root").and_then(Json::str).unwrap_or("").to_string();
        let entries = json
            .get("products")
            .map(|p| p.arr().to_vec())
            .unwrap_or_default()
            .iter()
            .map(|e| Entry {
                source: e.get("source").and_then(Json::str).unwrap_or("").to_string(),
                fingerprint: e.get("sourceFingerprint").and_then(Json::str).map(str::to_string),
                tasty: e.get("tasty").and_then(Json::str).map(str::to_string),
                classes: strings(e, "classes"),
                bodies: e.get("bodies").and_then(Json::bool).unwrap_or(false),
                inits: strings(e, "inits"),
                std: strings(e, "std"),
                digest: e.get("digest").and_then(Json::str).unwrap_or("").to_string(),
            })
            .collect();
        Ok(Manifest { target, root, entries, std: strings(&json, "std"), legacy_inits: strings(&json, "inits") })
    }
}

/// The binary name of an object's or a class's class file, `pkg/Outer$Inner` with a trailing `$`
/// for an object, as the JVM backend names it; `None` for a local class.
pub fn binary_name(syms: &crate::symbols::Symbols, interner: &crate::intern::Interner, c: crate::types::ClassId) -> Option<String> {
    use crate::symbols::{ClassKind, Owner};
    let info = syms.class(c);
    let mut out = match info.owner {
        Owner::Package(p) => package_path(syms, interner, p),
        Owner::Class(o) => {
            let mut outer = binary_name(syms, interner, o)?;
            if !outer.ends_with('$') {
                outer.push('$');
            }
            outer
        }
        Owner::Local => return None,
    };
    out.push_str(&crate::jvm::names::encode(interner.get(info.name)));
    if info.kind == ClassKind::Object {
        out.push('$');
    }
    Some(out)
}

fn package_path(syms: &crate::symbols::Symbols, interner: &crate::intern::Interner, p: crate::types::PkgId) -> String {
    let info = syms.pkg(p);
    match info.parent {
        Some(parent) => format!("{}{}/", package_path(syms, interner, parent), crate::jvm::names::encode(interner.get(info.name))),
        None => String::new(),
    }
}

/// The manifest's `inits` over `objects`: the binary names, sorted, of those whose construction
/// runs code by the layout's rule (`Layout::has_body`: a field, a statement, a parent's call),
/// the rule a JVM build applies to the module classes it emits.
pub fn object_inits(syms: &crate::symbols::Symbols, interner: &crate::intern::Interner, layout: &crate::emit::layout::Layout, objects: impl Iterator<Item = crate::types::ClassId>) -> Vec<String> {
    let mut out: Vec<String> = objects
        .filter(|&c| syms.class(c).kind == crate::symbols::ClassKind::Object && layout.has_body(c))
        .filter_map(|c| binary_name(syms, interner, c))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The path of a product's `.tasty` file under its directory.
pub fn tasty_path(p: &crate::tasty::write::Product) -> String {
    if p.package.is_empty() {
        format!("{}.tasty", p.name)
    } else {
        format!("{}/{}.tasty", p.package, p.name)
    }
}

/// What a build puts in a directory of products: its files with their bytes, the files of the
/// entries it drops, and the manifest, published together or not at all.
pub struct Publication {
    pub files: Vec<(String, Vec<u8>)>,
    pub removed: Vec<String>,
    pub manifest: String,
}

impl Publication {
    /// Publishes into `dir`: each file whose bytes change written in place, a file it replaces
    /// or that goes moved first into a staging directory beside `dir` (`<dir>.teq-stage`), so
    /// that a failure puts back what was there; the manifest written there and renamed last,
    /// then the staging directory marked done and removed. Before anything changes, the staging
    /// directory holds the publication's journal (`recover`). An unchanged file keeps its time.
    pub fn publish(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let dir = crate::source::canonicalize(dir)?;
        let stage = staging_dir(&dir);
        if !recover(&dir)? {
            return Err(std::io::Error::other(format!("another build is publishing into {}", dir.display())));
        }
        let mut moves: Vec<Move> = Vec::new();
        let result = self.write_in_place(&dir, &stage, &mut moves);
        if result.is_err() {
            for m in moves.iter().rev() {
                m.undo();
            }
        }
        let _ = std::fs::remove_dir_all(&stage);
        result
    }

    /// The files of the products whose digest the manifest of `dir` holds already, each of them
    /// on disk: unchanged without reading them back, which for a build into a directory of the
    /// same products (a session's first, a one-shot build again) is the read of every class file
    /// for nothing, 1.9 s over the application's 26,000.
    fn unchanged_by_manifest(&self, dir: &Path) -> crate::intern::FxMap<String, ()> {
        let mut out = crate::intern::FxMap::default();
        let (Some(Ok(old)), Ok(new)) = (Manifest::read(dir), Manifest::parse(&self.manifest)) else { return out };
        // An entry is known by its first file, which one product alone holds.
        let digests: crate::intern::FxMap<&str, &str> =
            old.entries.iter().filter_map(|e| e.files().next().map(|f| (f, e.digest.as_str()))).collect();
        let kept: Vec<bool> = on_workers(&new.entries, |chunk| {
            chunk
                .iter()
                .map(|e| {
                    e.files().next().and_then(|f| digests.get(f)) == Some(&e.digest.as_str()) && e.files().all(|f| dir.join(f).exists())
                })
                .collect::<Vec<_>>()
        })
        .concat();
        // A product moved to another source keeps its old entry until the old source is built
        // again, so two entries may name one file: a file of an entry that changed is written.
        let changed: crate::intern::FxMap<&str, ()> =
            new.entries.iter().zip(&kept).filter(|(_, keep)| !**keep).flat_map(|(e, _)| e.files().map(|f| (f, ()))).collect();
        for (e, keep) in new.entries.iter().zip(kept) {
            if keep {
                out.extend(e.files().filter(|f| !changed.contains_key(f)).map(|f| (f.to_string(), ())));
            }
        }
        out
    }

    /// Each step over the files runs on every worker thread, as `jvm::write` does.
    fn write_in_place(&self, dir: &Path, stage: &Path, moves: &mut Vec<Move>) -> std::io::Result<()> {
        let aside = stage.join("old");
        let unchanged = self.unchanged_by_manifest(dir);
        // Per file, whether its bytes change and whether it is there.
        let states: Vec<Option<bool>> = on_workers(&self.files, |chunk| {
            chunk
                .iter()
                .map(|(rel, bytes)| {
                    if unchanged.contains_key(rel.as_str()) {
                        return None;
                    }
                    // A link whose target is gone is there: sbt 2's cache restores products as links
                    // into its store, and a write would
                    // go through it into the store.
                    match std::fs::read(dir.join(rel)) {
                        Ok(old) => (old != *bytes).then_some(true),
                        Err(e) => Some(e.kind() != std::io::ErrorKind::NotFound || std::fs::symlink_metadata(dir.join(rel)).is_ok()),
                    }
                })
                .collect::<Vec<_>>()
        })
        .concat();
        let changed: Vec<(&str, &[u8], bool)> = self.files.iter().zip(&states).filter_map(|((rel, bytes), state)| state.map(|there| (rel.as_str(), bytes.as_slice(), there))).collect();
        let manifest = dir.join(MANIFEST);
        let manifest_changes = std::fs::read(&manifest).map_or(true, |old| old != self.manifest.as_bytes());
        let mut journal = format!("pid {}\n", std::process::id());
        for (rel, _, there) in &changed {
            if !there {
                journal.push_str(rel);
                journal.push('\n');
            }
        }
        if manifest_changes && !manifest.exists() {
            journal.push_str(MANIFEST);
            journal.push('\n');
        }
        std::fs::create_dir_all(stage)?;
        std::fs::write(stage.join(JOURNAL), journal)?;
        for rel in &self.removed {
            let target = dir.join(rel);
            // A link whose target is gone is there too, and goes aside with the rest.
            if std::fs::symlink_metadata(&target).is_ok() {
                moves.push(Move::aside(&target, &aside.join(rel))?);
            }
        }
        make_parents(changed.iter().map(|(rel, _, _)| dir.join(rel)))?;
        let mut failed = None;
        for (done, result) in on_workers(&changed, |chunk| {
            let mut done = Vec::new();
            let result = chunk.iter().try_for_each(|&(rel, bytes, there)| {
                let target = dir.join(rel);
                if there {
                    done.push(Move::aside(&target, &aside.join(rel))?);
                }
                done.push(Move { from: target.clone(), to: None });
                std::fs::write(&target, bytes)
            });
            (done, result)
        }) {
            moves.extend(done);
            failed = failed.or(result.err());
        }
        if let Some(e) = failed {
            return Err(e);
        }
        // A process killed here, its files written and the manifest not (tests/modules.sh's
        // `own-cut`).
        if std::env::var_os("TEQ_CUT_PUBLICATION").is_some() {
            std::process::exit(137);
        }
        // The manifest last, once every file it names is in place, replaced whole.
        if manifest_changes {
            let fresh = stage.join(MANIFEST);
            std::fs::write(&fresh, &self.manifest)?;
            if manifest.exists() {
                moves.push(Move::aside(&manifest, &aside.join(MANIFEST))?);
            }
            crate::write::rename(&fresh, &manifest)?;
            moves.push(Move { from: manifest, to: None });
        }
        std::fs::write(stage.join(DONE), "")?;
        // The directories the removals emptied go with them, as a build into an empty directory
        // never makes them.
        for rel in &self.removed {
            let mut at = dir.join(rel);
            while let Some(parent) = at.parent().filter(|p| *p != dir) {
                if std::fs::remove_dir(parent).is_err() {
                    break;
                }
                at = parent.to_path_buf();
            }
        }
        Ok(())
    }
}

/// `f` over `items` in a chunk per worker thread, the chunks' results in order.
fn on_workers<T: Sync, R: Send>(items: &[T], f: impl Fn(&[T]) -> R + Sync) -> Vec<R> {
    let workers = crate::workers().min(items.len()).max(1);
    if workers == 1 {
        return vec![f(items)];
    }
    let f = &f;
    std::thread::scope(|scope| {
        let handles: Vec<_> = items.chunks(items.len().div_ceil(workers)).map(|chunk| crate::alloc::spawn_in(scope, move || f(chunk))).collect();
        handles.into_iter().map(|h| h.join().expect("publication thread panicked")).collect()
    })
}

/// The parent directories of the paths, each made once.
fn make_parents(paths: impl Iterator<Item = PathBuf>) -> std::io::Result<()> {
    let dirs: std::collections::BTreeSet<PathBuf> = paths.filter_map(|p| p.parent().map(Path::to_path_buf)).collect();
    dirs.iter().try_for_each(std::fs::create_dir_all)
}

/// The staging directory's journal of a publication: `pid <n>`, then every path it writes that
/// was not there, the manifest's last; written before anything of the directory changes. A
/// path that was there is moved aside before it is written.
const JOURNAL: &str = "journal";
/// The staging directory's mark of a publication whose manifest is in place.
const DONE: &str = "done";

/// Finishes what a publication into `dir` left in its staging directory, before a build reads
/// or publishes the directory: a staging directory marked done goes; one whose publisher was
/// killed before (no mark, its process gone) is undone, every journaled new path removed and
/// every file moved aside put back over what was written there, so the directory is the last
/// whole publication's again. Whether
/// the directory is free: false while another live process is publishing into it.
pub fn recover(dir: &Path) -> std::io::Result<bool> {
    let stage = staging_dir(dir);
    if !stage.exists() {
        return Ok(true);
    }
    if !stage.join(DONE).exists() {
        let journal = std::fs::read_to_string(stage.join(JOURNAL)).unwrap_or_default();
        let mut lines = journal.lines();
        let pid = lines.next().and_then(|l| l.strip_prefix("pid ")).and_then(|p| p.parse::<u32>().ok());
        if pid.is_some_and(|p| p != std::process::id() && process_alive(p)) {
            return Ok(false);
        }
        for rel in lines {
            let _ = std::fs::remove_file(dir.join(rel));
        }
        let aside = stage.join("old");
        let mut files = Vec::new();
        if aside.is_dir() {
            walk_files(&aside, &aside, &mut files)?;
        }
        for rel in files {
            let target = dir.join(&rel);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            crate::write::rename(&aside.join(&rel), &target)?;
        }
    }
    std::fs::remove_dir_all(&stage)?;
    Ok(true)
}

fn walk_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            walk_files(root, &path, out)?;
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_path_buf());
        }
    }
    Ok(())
}

#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    // Signal 0 tests the process without signalling it; ESRCH is 3 on Linux and macOS alike.
    let signalled = unsafe { kill(pid as i32, 0) } == 0;
    signalled || std::io::Error::last_os_error().raw_os_error() != Some(3)
}

/// A handle on the process, its exit code still `STILL_ACTIVE`. A process that cannot be opened
/// is gone where the system says the pid names none (`ERROR_INVALID_PARAMETER`), alive however
/// else the open failed (another user's, refused), as a process that ended with code 259 reads as
/// alive: an answer that is not sure leaves the publication to its process.
#[cfg(windows)]
fn process_alive(pid: u32) -> bool {
    use std::ffi::c_void;
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn GetExitCodeProcess(process: *mut c_void, code: *mut u32) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;
    const ERROR_INVALID_PARAMETER: i32 = 87;
    // SAFETY: a handle opened, queried and closed here; the code is written by the query.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return std::io::Error::last_os_error().raw_os_error() != Some(ERROR_INVALID_PARAMETER);
        }
        let mut code = 0;
        let queried = GetExitCodeProcess(process, &mut code) != 0;
        CloseHandle(process);
        !queried || code == STILL_ACTIVE
    }
}

/// The staging directory of a products directory, beside it.
pub fn staging_dir(dir: &Path) -> PathBuf {
    let name = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    dir.with_file_name(format!("{}.teq-stage", name))
}

/// A rename a failed publication undoes: a file moved aside (`to` its place in the staging
/// directory), or one put in place (`to` none, removed on undo).
struct Move {
    from: PathBuf,
    to: Option<PathBuf>,
}

impl Move {
    fn aside(from: &Path, to: &Path) -> std::io::Result<Move> {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::write::rename(from, to)?;
        Ok(Move { from: from.to_path_buf(), to: Some(to.to_path_buf()) })
    }

    fn undo(&self) {
        let _ = match &self.to {
            Some(to) => self.from.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| crate::write::rename(to, &self.from)),
            None => std::fs::remove_file(&self.from),
        };
    }
}

/// The text of every `Utf8` constant of a class file, the names and descriptors it mentions.
pub fn utf8_constants(bytes: &[u8]) -> Vec<&str> {
    let mut out = Vec::new();
    let u16_at = |at: usize| bytes.get(at..at + 2).map(|b| u16::from_be_bytes([b[0], b[1]]) as usize);
    let Some(count) = u16_at(8) else { return out };
    let (mut at, mut i) = (10, 1);
    while i < count {
        let Some(&tag) = bytes.get(at) else { break };
        at += 1;
        let len = match tag {
            1 => {
                let Some(n) = u16_at(at) else { break };
                if let Some(text) = bytes.get(at + 2..at + 2 + n).and_then(|b| std::str::from_utf8(b).ok()) {
                    out.push(text);
                }
                2 + n
            }
            3 | 4 | 9 | 10 | 11 | 12 | 17 | 18 => 4,
            5 | 6 => {
                i += 1;
                8
            }
            7 | 8 | 16 | 19 | 20 => 2,
            15 => 3,
            _ => break,
        };
        at += len;
        i += 1;
    }
    out
}

/// The classes of `names` (binary names, `scala/Tuple23`) that a class file mentions, as a
/// class or in a descriptor.
pub fn classes_named<'n>(bytes: &[u8], names: &crate::intern::FxMap<&'n str, ()>) -> Vec<&'n str> {
    let mut out: Vec<&'n str> = Vec::new();
    let mut add = |name: &str| {
        if let Some((&n, _)) = names.get_key_value(name) {
            if !out.contains(&n) {
                out.push(n);
            }
        }
    };
    for text in utf8_constants(bytes) {
        add(text);
        // A descriptor's or a signature's `Lp/C;`.
        for (at, _) in text.match_indices('L') {
            if let Some(end) = text[at + 1..].find(';') {
                add(&text[at + 1..at + 1 + end]);
            }
        }
    }
    out
}

/// Adds class attributes to a class file: their names go at the end of the constant pool, so
/// no index the class already uses moves, and the attributes after the class's own.
pub fn add_class_attributes(bytes: &[u8], attrs: &[(&str, &[u8])]) -> Result<Vec<u8>, String> {
    let bad = || "a class file the writer cannot read back".to_string();
    let u16_at = |at: usize| -> Result<usize, String> { bytes.get(at..at + 2).map(|b| u16::from_be_bytes([b[0], b[1]]) as usize).ok_or_else(bad) };
    let u32_at = |at: usize| -> Result<usize, String> { bytes.get(at..at + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize).ok_or_else(bad) };
    let count = u16_at(8)?;
    let mut at = 10;
    let mut i = 1;
    while i < count {
        let tag = *bytes.get(at).ok_or_else(bad)?;
        at += 1;
        at += match tag {
            1 => 2 + u16_at(at)?,
            3 | 4 | 9 | 10 | 11 | 12 | 17 | 18 => 4,
            5 | 6 => {
                i += 1;
                8
            }
            7 | 8 | 16 | 19 | 20 => 2,
            15 => 3,
            _ => return Err(bad()),
        };
        i += 1;
    }
    let pool_end = at;
    // access, this, super, interfaces
    at += 6;
    let interfaces = u16_at(at)?;
    at += 2 + 2 * interfaces;
    for _ in 0..2 {
        let members = u16_at(at)?;
        at += 2;
        for _ in 0..members {
            at += 6;
            let n = u16_at(at)?;
            at += 2;
            for _ in 0..n {
                at += 6 + u32_at(at + 2)?;
            }
        }
    }
    let attrs_at = at;
    let n = u16_at(attrs_at)?;
    let mut out = Vec::with_capacity(bytes.len() + 64);
    out.extend_from_slice(&bytes[..8]);
    out.extend_from_slice(&((count + attrs.len()) as u16).to_be_bytes());
    out.extend_from_slice(&bytes[10..pool_end]);
    for (name, _) in attrs {
        out.push(1);
        out.extend_from_slice(&(name.len() as u16).to_be_bytes());
        out.extend_from_slice(name.as_bytes());
    }
    out.extend_from_slice(&bytes[pool_end..attrs_at]);
    out.extend_from_slice(&((n + attrs.len()) as u16).to_be_bytes());
    out.extend_from_slice(&bytes[attrs_at + 2..]);
    for (i, (_, body)) in attrs.iter().enumerate() {
        out.extend_from_slice(&((count + i) as u16).to_be_bytes());
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(body);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("teq-publication-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn publication(files: &[(&str, &str)], removed: &[&str], manifest: &str) -> Publication {
        Publication {
            files: files.iter().map(|(p, b)| (p.to_string(), b.as_bytes().to_vec())).collect(),
            removed: removed.iter().map(|p| p.to_string()).collect(),
            manifest: manifest.to_string(),
        }
    }

    #[test]
    fn publishes_changed_files_and_removals() {
        let dir = scratch("changes");
        let many: Vec<(String, String)> = (0..200).map(|i| (format!("p{}/C{}.class", i % 7, i), format!("c{}", i))).collect();
        let mut files: Vec<(&str, &str)> = many.iter().map(|(p, b)| (p.as_str(), b.as_str())).collect();
        files.push(("q/Gone.class", "gone"));
        publication(&files, &[], "m1").publish(&dir).unwrap();
        let kept = std::fs::metadata(dir.join("p0/C0.class")).unwrap().modified().unwrap();
        files.pop();
        files[1].1 = "changed";
        std::thread::sleep(std::time::Duration::from_millis(20));
        publication(&files, &["q/Gone.class"], "m2").publish(&dir).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("p1/C1.class")).unwrap(), "changed");
        assert_eq!(std::fs::read_to_string(dir.join("p3/C199.class")).unwrap(), "c199");
        assert_eq!(std::fs::read_to_string(dir.join(MANIFEST)).unwrap(), "m2");
        assert_eq!(std::fs::metadata(dir.join("p0/C0.class")).unwrap().modified().unwrap(), kept);
        assert!(!dir.join("q").exists());
        assert!(!staging_dir(&dir.canonicalize().unwrap()).exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn failed_publication_puts_back_what_was_there() {
        let dir = scratch("undo");
        publication(&[("x", "file"), ("r.class", "r"), ("a/A.class", "a")], &[], "m1").publish(&dir).unwrap();
        let failed = publication(&[("a/A.class", "a2"), ("x/y.class", "y")], &["r.class"], "m2").publish(&dir);
        assert!(failed.is_err());
        assert_eq!(std::fs::read_to_string(dir.join("r.class")).unwrap(), "r");
        assert_eq!(std::fs::read_to_string(dir.join("a/A.class")).unwrap(), "a");
        assert_eq!(std::fs::read_to_string(dir.join("x")).unwrap(), "file");
        assert_eq!(std::fs::read_to_string(dir.join(MANIFEST)).unwrap(), "m1");
        assert!(!staging_dir(&dir.canonicalize().unwrap()).exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn a_link_whose_target_is_gone_is_moved_aside_not_written_through() {
        let dir = scratch("dangling");
        let store = scratch("dangling-store");
        std::fs::create_dir_all(&store).unwrap();
        publication(&[("x", "file"), ("a/A.class", "a"), ("b/B.class", "b")], &[], "m1").publish(&dir).unwrap();
        std::fs::remove_file(dir.join("a/A.class")).unwrap();
        std::os::unix::fs::symlink(store.join("blob"), dir.join("a/A.class")).unwrap();
        let is_link = |p: &Path| std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink());
        assert!(publication(&[("a/A.class", "a2"), ("x/y.class", "y")], &[], "m2").publish(&dir).is_err());
        assert!(is_link(&dir.join("a/A.class")) && !store.join("blob").exists());
        publication(&[("a/A.class", "a2"), ("b/B.class", "b")], &[], "m2").publish(&dir).unwrap();
        assert!(!is_link(&dir.join("a/A.class")) && !store.join("blob").exists());
        assert_eq!(std::fs::read_to_string(dir.join("a/A.class")).unwrap(), "a2");
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&store).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn a_removed_link_whose_target_is_gone_goes_aside_with_the_removal() {
        let dir = scratch("dangling-removed");
        let store = scratch("dangling-removed-store");
        std::fs::create_dir_all(&store).unwrap();
        publication(&[("x", "file"), ("a/A.class", "a"), ("r.class", "r")], &[], "m1").publish(&dir).unwrap();
        std::fs::remove_file(dir.join("r.class")).unwrap();
        std::os::unix::fs::symlink(store.join("blob"), dir.join("r.class")).unwrap();
        let is_link = |p: &Path| std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink());
        assert!(publication(&[("a/A.class", "a"), ("x/y.class", "y")], &["r.class"], "m2").publish(&dir).is_err());
        assert!(is_link(&dir.join("r.class")) && !store.join("blob").exists());
        publication(&[("a/A.class", "a")], &["r.class"], "m2").publish(&dir).unwrap();
        assert!(std::fs::symlink_metadata(dir.join("r.class")).is_err() && !store.join("blob").exists());
        assert_eq!(std::fs::read_to_string(dir.join(MANIFEST)).unwrap(), "m2");
        assert!(!staging_dir(&dir.canonicalize().unwrap()).exists());
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&store).unwrap();
    }

    /// A publication killed after its files and before its manifest, as `write_in_place` leaves
    /// one, its publisher gone.
    fn killed_publication(dir: &Path, pid: u32) {
        let dir = dir.canonicalize().unwrap();
        let stage = staging_dir(&dir);
        let old = stage.join("old");
        std::fs::create_dir_all(old.join("a")).unwrap();
        std::fs::write(stage.join(JOURNAL), format!("pid {}\nn/N.class\n", pid)).unwrap();
        std::fs::rename(dir.join("r.class"), old.join("r.class")).unwrap();
        std::fs::rename(dir.join("a/A.class"), old.join("a/A.class")).unwrap();
        std::fs::write(dir.join("a/A.class"), "a2").unwrap();
        std::fs::create_dir_all(dir.join("n")).unwrap();
        std::fs::write(dir.join("n/N.class"), "half").unwrap();
    }

    #[test]
    fn killed_publication_is_undone() {
        let dir = scratch("killed");
        publication(&[("a/A.class", "a"), ("r.class", "r")], &[], "m1").publish(&dir).unwrap();
        // A pid no process has: the largest a pid can be on Linux and macOS, past any in use.
        killed_publication(&dir, 4_194_303);
        assert!(recover(&dir.canonicalize().unwrap()).unwrap());
        assert_eq!(std::fs::read_to_string(dir.join("a/A.class")).unwrap(), "a");
        assert_eq!(std::fs::read_to_string(dir.join("r.class")).unwrap(), "r");
        assert!(!dir.join("n/N.class").exists());
        assert_eq!(std::fs::read_to_string(dir.join(MANIFEST)).unwrap(), "m1");
        assert!(!staging_dir(&dir.canonicalize().unwrap()).exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn live_publication_is_left_alone() {
        let dir = scratch("live");
        publication(&[("a/A.class", "a"), ("r.class", "r")], &[], "m1").publish(&dir).unwrap();
        killed_publication(&dir, std::os::unix::process::parent_id());
        assert!(!recover(&dir.canonicalize().unwrap()).unwrap());
        assert!(publication(&[("a/A.class", "a3")], &[], "m2").publish(&dir).is_err());
        assert_eq!(std::fs::read_to_string(dir.join("a/A.class")).unwrap(), "a2");
        std::fs::remove_dir_all(staging_dir(&dir.canonicalize().unwrap())).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The publisher of `held_then_killed_publication`, a process of its own: its publication left
    /// unfinished, as `killed_publication` leaves one, and the process held until the test kills
    /// it. Nothing without the test's directory.
    #[test]
    #[ignore]
    fn held_publisher() {
        let (Some(dir), Some(ready)) = (std::env::var_os("TEQ_HELD_PUBLICATION"), std::env::var_os("TEQ_HELD_READY")) else { return };
        killed_publication(Path::new(&dir), std::process::id());
        std::fs::write(ready, "").unwrap();
        std::thread::sleep(std::time::Duration::from_secs(120));
    }

    /// A publication under two processes: while its publisher lives, another process leaves it
    /// alone and cannot publish; once the publisher is killed, the publication is undone.
    #[test]
    fn held_then_killed_publication() {
        let dir = scratch("held");
        publication(&[("a/A.class", "a"), ("r.class", "r")], &[], "m1").publish(&dir).unwrap();
        let ready = dir.with_extension("ready");
        let _ = std::fs::remove_file(&ready);
        let mut publisher = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["products::tests::held_publisher", "--exact", "--ignored", "--test-threads=1"])
            .env("TEQ_HELD_PUBLICATION", &dir)
            .env("TEQ_HELD_READY", &ready)
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !ready.exists() {
            assert!(std::time::Instant::now() < until, "the publisher did not start its publication");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let canonical = dir.canonicalize().unwrap();
        assert!(!recover(&canonical).unwrap());
        assert!(publication(&[("a/A.class", "a3")], &[], "m2").publish(&dir).is_err());
        assert_eq!(std::fs::read_to_string(dir.join("a/A.class")).unwrap(), "a2");
        assert!(dir.join("n/N.class").exists());
        publisher.kill().unwrap();
        publisher.wait().unwrap();
        assert!(recover(&canonical).unwrap());
        assert_eq!(std::fs::read_to_string(dir.join("a/A.class")).unwrap(), "a");
        assert_eq!(std::fs::read_to_string(dir.join("r.class")).unwrap(), "r");
        assert!(!dir.join("n/N.class").exists());
        assert_eq!(std::fs::read_to_string(dir.join(MANIFEST)).unwrap(), "m1");
        assert!(!staging_dir(&canonical).exists());
        std::fs::remove_file(&ready).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
