//! The jars and class directories of `--classpath`: opened once, their TASTy and class file
//! entries grouped by package from the central directories (or a directory walk) alone. A file
//! is inflated and parsed when the typer asks for it, and kept for the rest of the run.

use crate::classfile::ClassFile;
use crate::intern::FxMap;
use crate::jarcache::JarCache;
use crate::tasty::TastyFile;
use crate::zip::Zip;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// What separates the entries of a class path given as one string (`--classpath`): the platform's
/// path list separator, `;` on Windows (`File.pathSeparator`, which sbt-teq passes) and `:` elsewhere.
pub const SEPARATOR: char = if cfg!(windows) { ';' } else { ':' };

/// An entry of one source of the classpath.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CpFile {
    pub jar: u16,
    pub entry: u32,
}

pub struct CpPackage {
    /// `scala/collection`, the empty string for the root package.
    pub path: String,
    /// The TASTy files.
    pub files: Vec<CpFile>,
    /// The class files, Scala's included until a package is indexed (`Classpath::is_scala`).
    pub classes: Vec<CpFile>,
    /// `package.class` of a Scala 2 jar, which holds the pickle of the package object.
    pub scala2_objects: Vec<CpFile>,
}

#[derive(Default)]
pub struct CpStats {
    pub jars_opened: usize,
    pub open_time: Duration,
    pub files_inflated: usize,
    pub bytes_inflated: usize,
    pub inflate_time: Duration,
    pub class_files_parsed: usize,
    pub class_bytes: usize,
    pub class_parse_time: Duration,
    /// Of the files inflated, those read from the jar cache instead.
    pub files_cached: usize,
    pub caches_written: usize,
    pub cache_bytes_written: usize,
    pub cache_write_time: Duration,
    /// The TASTy files searched for reflectively instantiatable classes, and in how long.
    pub files_scanned: usize,
    pub scan_time: Duration,
}

/// A jar, or a directory of class files walked once when it is opened.
enum Source {
    Jar(Zip),
    Dir { root: PathBuf, files: Vec<String> },
}

const MAX_DIR_DEPTH: u32 = 32;

/// A module's own directory of products on its class path (`--products` there too): read
/// through its manifest alone, without the entries of the sources the build compiles or
/// removes (docs/TARGETS.md, "The contract between the plugin and the compiler").
pub struct OwnProducts {
    /// The directory, canonical.
    pub dir: PathBuf,
    /// The sources the build compiles or removes, canonical as far as they exist.
    pub dropped: Vec<PathBuf>,
}

/// The own directory as `Classpath::open` read it: its manifest, if it has one, and which of
/// its entries the build keeps.
pub struct OwnRead {
    /// Its place on the class path.
    pub entry: usize,
    pub manifest: Option<crate::products::Manifest>,
    pub retained: Vec<bool>,
}

/// A path made absolute and canonical as far as it exists: a removed source's directory is
/// resolved, its name kept, and a `..` after a part that does not exist taken from the path, as
/// Java's `getCanonicalFile` takes it (nothing there is a link).
pub fn canonical_lenient(path: &Path) -> PathBuf {
    let path = if path.is_absolute() { path.to_path_buf() } else { std::env::current_dir().unwrap_or_default().join(path) };
    if let Ok(c) = crate::source::canonicalize(&path) {
        return c;
    }
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => canonical_lenient(parent).join(name),
        (Some(parent), None) if path.ends_with("..") => {
            let mut up = canonical_lenient(parent);
            up.pop();
            up
        }
        _ => path,
    }
}

/// A manifest entry's source as a path: under the manifest's root where it is relative. A manifest
/// written under another root came with its directory from another checkout of the build, as sbt
/// 2's cache answers a compile with what another checkout's compile wrote: a source under that root is this
/// checkout's, under the working directory.
pub fn entry_source(manifest: &crate::products::Manifest, source: &str) -> PathBuf {
    static HERE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    entry_source_in(manifest, source, HERE.get_or_init(|| std::env::current_dir().unwrap_or_default()))
}

/// `entry_source` with the working directory `here`. The source is first the file the filesystem
/// names, links resolved before a `..` after them (`link/../B.scala`, the link to another
/// directory, names that directory's parent's `B.scala`), as the plugin's `getCanonicalFile`
/// does; under the manifest's root, also resolved, it is then this checkout's.
fn entry_source_in(manifest: &crate::products::Manifest, source: &str, here: &Path) -> PathBuf {
    let root = Path::new(&manifest.root);
    let p = Path::new(source);
    let real = canonical_lenient(&if p.is_absolute() { p.to_path_buf() } else { root.join(p) });
    if manifest.root.is_empty() || root == here {
        return real;
    }
    match real.strip_prefix(canonical_lenient(root)) {
        Ok(rel) => canonical_lenient(&here.join(rel)),
        Err(_) => real,
    }
}

impl Source {
    fn len(&self) -> usize {
        match self {
            Source::Jar(zip) => zip.entries.len(),
            Source::Dir { files, .. } => files.len(),
        }
    }

    fn name(&self, i: usize) -> &str {
        match self {
            Source::Jar(zip) => zip.name(&zip.entries[i]),
            Source::Dir { files, .. } => &files[i],
        }
    }

    fn crc(&self, i: usize) -> u32 {
        match self {
            Source::Jar(zip) => zip.entries[i].crc32,
            Source::Dir { .. } => 0,
        }
    }

    fn read(&self, i: usize) -> Result<Vec<u8>, String> {
        match self {
            Source::Jar(zip) => zip.read(i),
            Source::Dir { root, files } => {
                let path = root.join(&files[i]);
                std::fs::read(&path).map_err(|e| format!("cannot read {}: {}", path.display(), e))
            }
        }
    }
}

fn walk_dir(root: &Path, dir: &Path, depth: u32, out: &mut Vec<String>) -> Result<(), String> {
    if depth > MAX_DIR_DEPTH {
        return Ok(());
    }
    let entries = std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {}", dir.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read {}: {}", dir.display(), e))?;
        let path = entry.path();
        let is_dir = match entry.file_type() {
            Ok(t) if !t.is_symlink() => t.is_dir(),
            _ => path.is_dir(),
        };
        if is_dir {
            walk_dir(root, &path, depth + 1, out)?;
        } else if path.extension().map_or(false, |e| e == "class" || e == "tasty") {
            let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
            out.push(rel);
        }
    }
    Ok(())
}

pub struct Classpath {
    sources: Vec<Source>,
    /// Each source's path as the class path names it, which a binary dependency names
    /// (`typer::deps`).
    pub paths: Vec<String>,
    pub packages: Vec<CpPackage>,
    by_path: FxMap<String, u32>,
    parsed: FxMap<CpFile, Arc<TastyFile>>,
    parsed_classes: FxMap<CpFile, Arc<ClassFile>>,
    /// Per source, the jar's cache (`jarcache.rs`), and whether this run inflated one of its
    /// entries, which the cache is then written with.
    caches: Vec<Option<JarCache>>,
    inflated: Vec<bool>,
    /// The bytes of the TASTy files a search read that nothing parsed, kept for the searches that
    /// follow until the caches are written, with them.
    scanned: FxMap<CpFile, Vec<u8>>,
    /// The entries a search could not read, with why; each is reported once and never cached.
    pub scan_failures: Vec<(CpFile, String)>,
    scan_failed: FxMap<CpFile, ()>,
    /// Per source, whether it holds Scala.js IR (`.sjsir`): only such a jar was compiled against
    /// the Scala.js library.
    scalajs: Vec<bool>,
    /// Per source, whether it looks written by scalac 2: no TASTy file and a module class
    /// (`Foo$.class`). Its top-level class files are read for their pickles on the JVM, where
    /// its class files run as they are.
    scala2: Vec<bool>,
    any_scala2: bool,
    /// Every package path of the classpath with its ancestors, built on the first pickle read.
    package_dirs: Option<FxMap<String, ()>>,
    /// Per source, whether it is a directory of teq's products (`teq-products.json` in it):
    /// its classes are another module's, called and never compiled, and the class files of
    /// the standard library its build wrote are left out of it (`products.rs`).
    products: Vec<bool>,
    /// The module classes of the product directories' objects whose initialisation runs code.
    pub product_inits: FxMap<String, ()>,
    /// The module's own directory of products, when the build names it (`OwnProducts`).
    pub own: Option<OwnRead>,
    /// Per source, a directory of products' identity as its manifest stated it when the
    /// directory was read (`products_changed`).
    identities: Vec<Option<String>>,
    /// The TASTy files of a directory's package read together on the first read of one of
    /// them, over threads, each until it is parsed.
    prefetched: FxMap<CpFile, Vec<u8>>,
    prefetched_packages: FxMap<(u16, u32), ()>,
    /// Per package, its class files and its TASTy files by stem, each stem's entry the one the
    /// class path names first (`first_by_stem`), made the first time the package is asked.
    class_stems: Vec<std::sync::OnceLock<FxMap<Box<str>, CpFile>>>,
    tasty_stems: Vec<std::sync::OnceLock<FxMap<Box<str>, CpFile>>>,
    pub stats: CpStats,
}

impl Classpath {
    /// Whether some jar or directory holds TASTy files in the package `path` (`org/scalajs/dom`)
    /// or in a package inside it.
    pub fn holds_tasty_in(&self, path: &str) -> bool {
        self.packages.iter().any(|p| {
            !p.files.is_empty() && p.path.strip_prefix(path).map_or(false, |rest| rest.is_empty() || rest.starts_with('/'))
        })
    }

    /// Opens the jars and directories of `paths`, each jar with its cache when `cache` names
    /// the cache directory, and the module's own directory of products through its manifest.
    pub fn open(paths: &[String], cache: Option<&Path>, own: Option<&OwnProducts>) -> Result<Classpath, String> {
        let start = Instant::now();
        let mut cp = Classpath {
            sources: Vec::with_capacity(paths.len()),
            paths: paths.to_vec(),
            packages: Vec::new(),
            by_path: FxMap::default(),
            parsed: FxMap::default(),
            parsed_classes: FxMap::default(),
            caches: Vec::with_capacity(paths.len()),
            inflated: vec![false; paths.len()],
            scanned: FxMap::default(),
            scan_failures: Vec::new(),
            scan_failed: FxMap::default(),
            scalajs: Vec::with_capacity(paths.len()),
            scala2: vec![false; paths.len()],
            any_scala2: false,
            package_dirs: None,
            products: vec![false; paths.len()],
            product_inits: FxMap::default(),
            own: None,
            identities: vec![None; paths.len()],
            prefetched: FxMap::default(),
            prefetched_packages: FxMap::default(),
            class_stems: Vec::new(),
            tasty_stems: Vec::new(),
            stats: CpStats::default(),
        };
        for (j, path) in paths.iter().enumerate() {
            // The own directory before the module's first build is an empty one.
            let first_build = own.is_some_and(|o| !Path::new(path).exists() && canonical_lenient(Path::new(path)) == o.dir);
            let source = if Path::new(path).is_dir() || first_build {
                let root = PathBuf::from(path);
                let mut files = Vec::new();
                if !first_build {
                    // What a killed build's publication left is undone first (`products::recover`).
                    crate::products::recover(&canonical_lenient(&root)).map_err(|e| format!("{}: {}", path, e))?;
                    walk_dir(&root, &root, 0, &mut files)?;
                }
                files.sort();
                let manifest = crate::products::Manifest::read(&root).transpose()?;
                cp.identities[j] = crate::products::Manifest::identity_in(&root);
                match own.filter(|o| cp.own.is_none() && canonical_lenient(&root) == o.dir) {
                    // The own directory: its retained entries' files alone, nothing without a
                    // manifest.
                    Some(o) => {
                        let retained: Vec<bool> = manifest.as_ref().map_or(Vec::new(), |m| m.entries.iter().map(|e| !o.dropped.contains(&entry_source(m, &e.source))).collect());
                        let kept: FxMap<&str, ()> = manifest.iter().flat_map(|m| m.entries.iter().zip(&retained).filter(|(_, &r)| r).flat_map(|(e, _)| e.files())).map(|f| (f, ())).collect();
                        files.retain(|f| kept.contains_key(f.as_str()));
                        if let Some(m) = &manifest {
                            cp.products[j] = true;
                            cp.product_inits.extend(m.entries.iter().zip(&retained).filter(|(_, &r)| r).flat_map(|(e, _)| e.inits.iter()).map(|n| (n.clone(), ())));
                        }
                        cp.own = Some(OwnRead { entry: j, manifest, retained });
                    }
                    None => {
                        if let Some(manifest) = manifest {
                            cp.products[j] = true;
                            cp.product_inits.extend(manifest.inits().map(|n| (n.clone(), ())));
                            let std: FxMap<&str, ()> = manifest.std.iter().map(|s| (s.as_str(), ())).collect();
                            files.retain(|f| !std.contains_key(f.as_str()));
                        }
                    }
                }
                Source::Dir { root, files }
            } else {
                Source::Jar(Zip::open(path)?)
            };
            let jar_cache = match (&source, cache) {
                (Source::Jar(_), Some(dir)) => JarCache::open(path, dir),
                _ => None,
            };
            cp.caches.push(jar_cache);
            let mut scalajs = false;
            let (mut tasty_seen, mut module_seen) = (false, false);
            for i in 0..source.len() {
                let name = source.name(i);
                scalajs |= name.ends_with(".sjsir");
                let (stem, tasty) = match name.strip_suffix(".tasty") {
                    Some(stem) => (stem, true),
                    None => match name.strip_suffix(".class") {
                        Some(stem) => (stem, false),
                        None => continue,
                    },
                };
                let dir = stem.rfind('/').map_or("", |at| &stem[..at]);
                let p = match cp.by_path.get(dir) {
                    Some(&p) => p as usize,
                    None => {
                        cp.by_path.insert(dir.to_string(), cp.packages.len() as u32);
                        cp.packages.push(CpPackage { path: dir.to_string(), files: Vec::new(), classes: Vec::new(), scala2_objects: Vec::new() });
                        cp.packages.len() - 1
                    }
                };
                let f = CpFile { jar: j as u16, entry: i as u32 };
                tasty_seen |= tasty;
                module_seen |= !tasty && stem.ends_with('$');
                if tasty {
                    cp.packages[p].files.push(f);
                } else {
                    cp.packages[p].classes.push(f);
                }
            }
            // Scala 2's own library is what scala-library 3.8 replaces.
            if module_seen && !tasty_seen && !is_scala_library_jar(path) {
                cp.scala2[j] = true;
                cp.any_scala2 = true;
                for i in 0..source.len() {
                    let name = source.name(i);
                    let Some(dir) = name.strip_suffix("package.class").filter(|d| d.is_empty() || d.ends_with('/')) else { continue };
                    if let Some(&p) = cp.by_path.get(dir.trim_end_matches('/')) {
                        cp.packages[p as usize].scala2_objects.push(CpFile { jar: j as u16, entry: i as u32 });
                    }
                }
            }
            cp.sources.push(source);
            cp.scalajs.push(scalajs);
        }
        cp.class_stems = cp.packages.iter().map(|_| std::sync::OnceLock::new()).collect();
        cp.tasty_stems = cp.packages.iter().map(|_| std::sync::OnceLock::new()).collect();
        cp.stats.jars_opened = paths.len();
        cp.stats.open_time = start.elapsed();
        Ok(cp)
    }

    /// The entry name without its directory and extension: `Option` for `scala/Option.tasty`,
    /// `Map$Entry` for `java/util/Map$Entry.class`.
    pub fn stem(&self, f: CpFile) -> &str {
        let name = self.entry_name(f);
        let stem = name.strip_suffix(".tasty").or_else(|| name.strip_suffix(".class")).unwrap_or(name);
        stem.rfind('/').map_or(stem, |at| &stem[at + 1..])
    }

    pub fn entry_name(&self, f: CpFile) -> &str {
        self.sources[f.jar as usize].name(f.entry as usize)
    }

    /// Whether a class file of package `p` was written by a Scala compiler: its stem, or a
    /// `$`-prefix of it (`Foo$`, `Foo$Inner`, `$colon$colon$`), has a TASTy sibling. The
    /// stems of the package's TASTy files are in `tasty_stems`.
    pub fn is_scala(&self, tasty_stems: &FxMap<Box<str>, CpFile>, class: CpFile) -> bool {
        let mut stem = self.stem(class);
        loop {
            if tasty_stems.contains_key(stem) {
                return true;
            }
            match stem.rfind('$') {
                Some(at) if at > 0 => stem = &stem[..at],
                _ => return false,
            }
        }
    }

    /// Whether the entry is in a directory of teq's products.
    pub fn is_products(&self, f: CpFile) -> bool {
        self.products.get(f.jar as usize).copied().unwrap_or(false)
    }

    /// A directory of products whose manifest's identity is not the one it had when the class
    /// path was opened, or beside which a staging directory stands: another build published, or
    /// publishes, into it since (docs/TARGETS.md, "Watch mode"). A read of the manifest's first
    /// bytes per directory, no walk; a directory without a manifest then is checked by the next
    /// full build's walk alone.
    pub fn products_changed(&self) -> Option<&str> {
        self.identities.iter().enumerate().find_map(|(j, read)| {
            let read = read.as_deref()?;
            let Source::Dir { root, .. } = &self.sources[j] else { return None };
            // A staging directory is a publication under way or one a killed build left, whose
            // files the manifest's identity does not tell from the old.
            let publishing = crate::products::staging_dir(&canonical_lenient(root)).exists();
            (publishing || crate::products::Manifest::identity_in(root).as_deref() != Some(read)).then(|| self.paths[j].as_str())
        })
    }

    /// Whether the class path's `j`th entry is a directory of teq's products.
    pub fn is_own_entry(&self, j: usize) -> bool {
        self.own.as_ref().is_some_and(|o| o.entry == j)
    }

    pub fn is_products_entry(&self, j: usize) -> bool {
        self.products.get(j).copied().unwrap_or(false)
    }

    /// Whether a directory of teq's products is on the class path.
    pub fn holds_products(&self) -> bool {
        self.products.iter().any(|&p| p)
    }

    pub fn only_products(&self) -> bool {
        self.products.iter().all(|&p| p)
    }

    pub fn is_scala2(&self, f: CpFile) -> bool {
        self.scala2[f.jar as usize]
    }

    pub fn holds_scala2(&self) -> bool {
        self.any_scala2
    }

    /// The parsed file, inflated on the first request; a class file of a Scala 2 jar is read
    /// as the TASTy its pickle is rewritten into.
    /// The bytes of a TASTy entry read from its jar or directory alone, neither cached nor
    /// counted: for a scan that keeps nothing of the file (the source map of a jar's documents,
    /// `typer::loader::attach`).
    pub fn raw_bytes(&self, f: CpFile) -> Result<Vec<u8>, String> {
        self.sources[f.jar as usize].read(f.entry as usize)
    }

    /// The TASTy entries of one source of the class path, in the order of its packages.
    pub fn files_of(&self, jar: u16) -> Vec<CpFile> {
        self.packages.iter().flat_map(|p| p.files.iter().copied()).filter(|f| f.jar == jar).collect()
    }

    pub fn file(&mut self, f: CpFile) -> Result<Arc<TastyFile>, String> {
        if let Some(t) = self.parsed.get(&f) {
            return Ok(t.clone());
        }
        let start = Instant::now();
        let (j, source) = (f.jar as usize, &self.sources[f.jar as usize]);
        let bytes = match self.caches[j].as_mut().and_then(|c| c.read(f.entry, source.crc(f.entry as usize))) {
            Some(bytes) => {
                self.stats.files_cached += 1;
                bytes
            }
            None if self.scala2[j] => {
                self.inflated[j] |= self.caches[j].is_some();
                self.transcode(f)?
            }
            None => {
                self.inflated[j] |= self.caches[j].is_some();
                if matches!(source, Source::Dir { .. }) {
                    self.prefetch_package(f);
                }
                match self.prefetched.remove(&f) {
                    Some(bytes) => bytes,
                    None => self.sources[j].read(f.entry as usize)?,
                }
            }
        };
        self.stats.files_inflated += 1;
        self.stats.bytes_inflated += bytes.len();
        let parsed = TastyFile::parse(bytes).map_err(|e| format!("{}: {}", self.entry_name(f), e))?;
        self.stats.inflate_time += start.elapsed();
        let rc = Arc::new(parsed);
        self.parsed.insert(f, rc.clone());
        Ok(rc)
    }

    /// Reads the TASTy files of `f`'s directory and package that nothing parsed, over threads
    /// when they are many, the first time one of them is asked for.
    fn prefetch_package(&mut self, f: CpFile) {
        const PER_THREAD: usize = 16;
        // Past four threads the opens contend (macOS, 2,020 files: 31 ms on one thread, 16 on
        // four, 22 on eight).
        const MAX_THREADS: usize = 4;
        let source = &self.sources[f.jar as usize];
        let dir = source.name(f.entry as usize).rsplit_once('/').map_or("", |(d, _)| d);
        let Some(&p) = self.by_path.get(dir) else { return };
        if self.prefetched_packages.insert((f.jar, p), ()).is_some() {
            return;
        }
        let wanted: Vec<CpFile> = self.packages[p as usize].files.iter().copied().filter(|g| g.jar == f.jar && !self.parsed.contains_key(g)).collect();
        let threads = (wanted.len() / PER_THREAD).clamp(1, crate::workers().min(MAX_THREADS));
        if threads == 1 {
            return;
        }
        let read: Vec<Vec<(CpFile, Vec<u8>)>> = std::thread::scope(|scope| {
            let handles: Vec<_> = wanted
                .chunks(wanted.len().div_ceil(threads))
                .map(|chunk| crate::alloc::spawn_in(scope, move || chunk.iter().filter_map(|&g| Some((g, source.read(g.entry as usize).ok()?))).collect()))
                .collect();
            handles.into_iter().map(|h| h.join().unwrap_or_default()).collect()
        });
        self.prefetched.extend(read.into_iter().flatten());
    }

    /// The TASTy files of the Scala.js jars, in class path order, whose name tables hold one of
    /// the simple names `names`. A file nothing parsed is read once, from the jar cache when it
    /// holds it, and goes into the cache with the files parsed.
    pub fn scalajs_tasty_files_naming(&mut self, names: &[&str]) -> Vec<CpFile> {
        let start = Instant::now();
        let wanted: Vec<&[u8]> = names.iter().map(|n| n.as_bytes()).collect();
        let mut files: Vec<CpFile> = self.packages.iter().flat_map(|p| p.files.iter().copied()).filter(|f| self.scalajs[f.jar as usize]).collect();
        files.sort_unstable_by_key(|f| (f.jar, f.entry));
        let mut out = Vec::new();
        for f in files {
            if self.scan_failed.contains_key(&f) {
                continue;
            }
            if !self.parsed.contains_key(&f) && !self.scanned.contains_key(&f) {
                let (j, source) = (f.jar as usize, &self.sources[f.jar as usize]);
                let read = match self.caches[j].as_mut().and_then(|c| c.read(f.entry, source.crc(f.entry as usize))) {
                    Some(bytes) => Ok(bytes),
                    None => {
                        self.inflated[j] |= self.caches[j].is_some();
                        source.read(f.entry as usize)
                    }
                };
                self.stats.files_scanned += 1;
                match read {
                    Ok(bytes) => {
                        self.scanned.insert(f, bytes);
                    }
                    Err(e) => {
                        let why = e;
                        self.scan_failed.insert(f, ());
                        self.scan_failures.push((f, why));
                        continue;
                    }
                }
            }
            let bytes = match self.parsed.get(&f) {
                Some(t) => t.bytes.as_slice(),
                None => self.scanned[&f].as_slice(),
            };
            if crate::tasty::names_one_of(bytes, &wanted) {
                out.push(f);
            }
        }
        self.stats.scan_time += start.elapsed();
        out
    }

    fn transcode(&mut self, f: CpFile) -> Result<Vec<u8>, String> {
        let cf = self.class_file(f)?;
        let raw = cf.scala_sig.clone().ok_or_else(|| format!("{}: no Scala 2 signature", self.entry_name(f)))?;
        let entry = self.entry_name(f).to_string();
        let package = entry.rfind('/').map_or("", |at| &entry[..at]).to_string();
        if self.package_dirs.is_none() {
            let mut dirs = FxMap::default();
            for p in &self.packages {
                let mut path = p.path.as_str();
                loop {
                    dirs.insert(path.to_string(), ());
                    match path.rfind('/') {
                        Some(at) => path = &path[..at],
                        None => break,
                    }
                }
            }
            self.package_dirs = Some(dirs);
        }
        let dirs = self.package_dirs.as_ref().unwrap();
        let is_package = |path: &str| {
            dirs.contains_key(path)
                || (["java", "javax", "jdk", "sun"].contains(&path.split('/').next().unwrap_or(""))
                    && path.split('/').all(|seg| seg.starts_with(|c: char| c.is_ascii_lowercase())))
        };
        let bytes = crate::scala2::to_tasty(raw, &package, &is_package).map_err(|e| format!("{}: {}", entry, e))?;
        if let Some(dir) = std::env::var_os("TEQ_SCALA2_DUMP") {
            let path = Path::new(&dir).join(entry.replace(".class", ".tasty"));
            let _ = std::fs::create_dir_all(path.parent().unwrap());
            let _ = std::fs::write(&path, &bytes);
            if let Ok(p) = crate::scala2::pickle::Pickle::parse(crate::scala2::pickle::decode_signature(cf.scala_sig.clone().unwrap())) {
                let _ = std::fs::write(path.with_extension("pickle"), p.dump());
            }
        }
        Ok(bytes)
    }

    /// Writes the cache of every jar this run inflated an entry of, with the TASTy files read
    /// from it; a cache that cannot be written is left out, silently, as a missing one is.
    pub fn save_caches(&mut self) {
        let start = Instant::now();
        for j in 0..self.sources.len() {
            if !self.inflated[j] {
                continue;
            }
            self.inflated[j] = false;
            let Some(cache) = self.caches[j].as_mut() else { continue };
            let source = &self.sources[j];
            let mut read: Vec<(u32, u32, &[u8])> =
                self.parsed.iter().filter(|(f, _)| f.jar as usize == j).map(|(f, t)| (f.entry, source.crc(f.entry as usize), t.bytes.as_slice())).collect();
            read.extend(self.scanned.iter().filter(|(f, _)| f.jar as usize == j && !self.parsed.contains_key(f)).map(|(f, b)| (f.entry, source.crc(f.entry as usize), b.as_slice())));
            read.sort_by_key(|&(i, _, _)| i);
            if let Ok(n) = cache.save(&read) {
                self.stats.caches_written += 1;
                self.stats.cache_bytes_written += n;
            }
        }
        self.scanned = FxMap::default();
        self.stats.cache_write_time += start.elapsed();
    }

    /// The bytes of a resource (`org/scalactic/ScalacticBundle.properties`), the first on the
    /// class path as a class loader finds it.
    pub fn resource(&self, path: &str) -> Option<Vec<u8>> {
        self.sources.iter().find_map(|s| match s {
            Source::Jar(zip) => zip.find(path).and_then(|i| zip.read(i).ok()),
            Source::Dir { root, .. } => std::fs::read(root.join(path)).ok(),
        })
    }

    /// The class file of a binary name (`scala/Tuple2`, `scala/Predef$`), the first on the class
    /// path as `java -cp` finds it.
    pub fn class_named(&self, binary: &str) -> Option<CpFile> {
        let (dir, stem) = binary.rsplit_once('/').unwrap_or(("", binary));
        let p = *self.by_path.get(dir)? as usize;
        self.class_stems[p].get_or_init(|| self.first_by_stem(&self.packages[p].classes)).get(stem).copied()
    }

    /// The TASTy file of a top-level class by its path without the extension (`scala/Option`), the
    /// first on the class path.
    pub fn tasty_named(&self, path: &str) -> Option<CpFile> {
        let (dir, stem) = path.rsplit_once('/').unwrap_or(("", path));
        let p = *self.by_path.get(dir)? as usize;
        self.tasty_stems[p].get_or_init(|| self.first_by_stem(&self.packages[p].files)).get(stem).copied()
    }

    /// The entries of a package by stem, each stem's first on the class path: the one of the
    /// earliest jar, and of that jar the earliest entry.
    fn first_by_stem(&self, files: &[CpFile]) -> FxMap<Box<str>, CpFile> {
        let mut out: FxMap<Box<str>, CpFile> = FxMap::default();
        for &f in files {
            match out.get_mut(self.stem(f)) {
                Some(g) => {
                    if f.jar < g.jar {
                        *g = f;
                    }
                }
                None => {
                    out.insert(self.stem(f).into(), f);
                }
            }
        }
        out
    }

    /// Whether the entry is a directory's file, not a jar's.
    pub fn in_directory(&self, f: CpFile) -> bool {
        matches!(self.sources[f.jar as usize], Source::Dir { .. })
    }

    /// The parsed class file, read on the first request.
    pub fn class_file(&mut self, f: CpFile) -> Result<Arc<ClassFile>, String> {
        if let Some(c) = self.parsed_classes.get(&f) {
            return Ok(c.clone());
        }
        let start = Instant::now();
        let bytes = self.sources[f.jar as usize].read(f.entry as usize)?;
        self.stats.class_files_parsed += 1;
        self.stats.class_bytes += bytes.len();
        let parsed = ClassFile::parse(&bytes).map_err(|e| format!("{}: {}", self.entry_name(f), e))?;
        self.stats.class_parse_time += start.elapsed();
        let rc = Arc::new(parsed);
        self.parsed_classes.insert(f, rc.clone());
        Ok(rc)
    }
}

/// The name of a class as its class file spells it: operator characters are written out
/// (`$colon$colon` for `::`), as `scala.reflect.NameTransformer` does.
pub fn encode_name(name: &str) -> std::borrow::Cow<'_, str> {
    if name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$' || b >= 0x80) {
        return std::borrow::Cow::Borrowed(name);
    }
    let mut out = String::with_capacity(name.len() * 4);
    for c in name.chars() {
        let code = match c {
            '~' => "$tilde",
            '=' => "$eq",
            '<' => "$less",
            '>' => "$greater",
            '!' => "$bang",
            '#' => "$hash",
            '%' => "$percent",
            '^' => "$up",
            '&' => "$amp",
            '|' => "$bar",
            '*' => "$times",
            '/' => "$div",
            '+' => "$plus",
            '-' => "$minus",
            ':' => "$colon",
            '\\' => "$bslash",
            '?' => "$qmark",
            '@' => "$at",
            other => {
                out.push(other);
                continue;
            }
        };
        out.push_str(code);
    }
    std::borrow::Cow::Owned(out)
}

/// Whether a classpath entry is an artifact of the standard library, which scalac's macro class
/// loader finds in the compiler's own loader: scala-library or scala3-library, of the JVM or of
/// Scala.js, by the jar's file name (`<artifact>-<version>.jar`), and of the organisation
/// `org.scala-lang` where the jar lies in a repository's layout
/// (`org/scala-lang/<artifact>/<version>/`). The names of the directories above say nothing else.
/// The path's components are the platform's (`std::path::is_separator`: `\` as well as `/` on
/// Windows).
pub fn is_compiler_library(path: &str) -> bool {
    let mut dirs = path.rsplit(std::path::is_separator);
    let name = dirs.next().unwrap_or(path);
    let Some(stem) = name.strip_suffix(".jar") else { return false };
    let versioned = |artifact: &str| stem.strip_prefix(artifact).and_then(|rest| rest.strip_prefix('-')).filter(|v| v.starts_with(|c: char| c.is_ascii_digit()));
    let Some((artifact, version)) = ["scala-library", "scala3-library_3", "scala3-library_sjs1_3"].into_iter().find_map(|a| versioned(a).map(|v| (a, v))) else { return false };
    match (dirs.next(), dirs.next()) {
        (Some(v), Some(a)) if v == version && a == artifact => dirs.next() == Some("scala-lang") && dirs.next() == Some("org"),
        _ => true,
    }
}

/// Whether a classpath entry is the scala-library jar.
pub fn is_scala_library_jar(path: &str) -> bool {
    let name = path.rsplit(std::path::is_separator).next().unwrap_or(path);
    name.starts_with("scala-library-") && name.ends_with(".jar")
}

/// The newest scala-library 3.x jar of the coursier cache, for a JVM build or `--std=scala-library`
/// without a `--classpath` that names one. Nothing is downloaded. On Windows the cache is the one
/// `teq` fetches into (`task::fetch::coursier_cache`: `COURSIER_CACHE`, else
/// `%LOCALAPPDATA%\Coursier\cache\v1`), with no `HOME` needed. The suites start `java` on a JVM
/// build's output with the jar this finds: tests/support/jars.sh's `scala_library_jar` and
/// proptests/src/targets.rs's `scala_library` search the same caches and choose the same jar, and
/// a change here is a change of both.
pub fn find_scala_library() -> Result<String, String> {
    let mut caches: Vec<PathBuf> = Vec::new();
    if cfg!(windows) {
        caches.extend(crate::task::fetch::coursier_cache());
    } else {
        let home = std::env::var("HOME").unwrap_or_default();
        if let Ok(c) = std::env::var("COURSIER_CACHE") {
            caches.push(PathBuf::from(c));
        }
        caches.push(Path::new(&home).join("Library/Caches/Coursier/v1"));
        caches.push(Path::new(&home).join(".cache/coursier/v1"));
    }
    let mut best: Option<(Vec<u32>, PathBuf)> = None;
    for cache in &caches {
        let dir = cache.join("https/repo1.maven.org/maven2/org/scala-lang/scala-library");
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let version = entry.file_name().to_string_lossy().to_string();
            let parts: Option<Vec<u32>> = version.split('.').map(|p| p.parse().ok()).collect();
            let Some(parts) = parts else { continue };
            if parts.first() != Some(&3) {
                continue;
            }
            let jar = entry.path().join(format!("scala-library-{}.jar", version));
            if jar.is_file() && best.as_ref().map_or(true, |(v, _)| parts > *v) {
                best = Some((parts, jar));
            }
        }
    }
    match best {
        Some((_, jar)) => Ok(jar.to_string_lossy().to_string()),
        None => Err(format!(
            "no scala-library jar: a JVM build links against scala-library-<3.x>.jar, as does --std=scala-library; name it with --classpath, or put a 3.x release in the coursier cache ({}), which `cs fetch org.scala-lang:scala-library:3.8.4` or any scala-cli run under -S 3.8.4 does",
            caches.iter().map(|c| c.to_string_lossy().to_string()).collect::<Vec<_>>().join(", ")
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{canonical_lenient, is_compiler_library, Classpath, CpFile};

    /// A manifest sbt's cache brought from another checkout names this checkout's sources: those
    /// under its root are read under the working directory, the others as they are; a source is
    /// the file the filesystem names, a link resolved before a `..` after it (`link/../B.scala`,
    /// `link` to `external/sub`, is `external/B.scala`, outside the root), in the manifest's own
    /// checkout and in another.
    #[test]
    #[cfg(unix)]
    fn reads_another_checkouts_sources_under_this_one() {
        let base = std::env::temp_dir().join(format!("teq-entry-source-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        for d in ["old/src", "new/src", "external/sub"] {
            std::fs::create_dir_all(base.join(d)).unwrap();
        }
        let base = canonical_lenient(&base);
        for checkout in ["old", "new"] {
            std::os::unix::fs::symlink("../external/sub", base.join(checkout).join("link")).unwrap();
        }
        std::fs::write(base.join("external/B.scala"), "class B\n").unwrap();
        let (old, new) = (base.join("old"), base.join("new"));
        let manifest = |root: &std::path::Path, source: &str| {
            crate::products::Manifest::parse(&format!(r#"{{"root":"{}","products":[{{"source":"{}"}}]}}"#, root.display(), source)).unwrap()
        };
        let read = |root: &std::path::Path, source: &str, here: &std::path::Path| {
            let m = manifest(root, source);
            super::entry_source_in(&m, &m.entries[0].source, here)
        };
        // Moved: in-root sources, absolute and relative, follow the checkout; the others stay.
        assert_eq!(read(&old, &old.join("src/A.scala").display().to_string(), &new), new.join("src/A.scala"));
        assert_eq!(read(&old, "src/A.scala", &new), new.join("src/A.scala"));
        assert_eq!(read(&old, "../shared/B.scala", &new), base.join("shared/B.scala"));
        assert_eq!(read(&old, "link/../B.scala", &new), base.join("external/B.scala"));
        assert_eq!(read(&old, &old.join("link/../B.scala").display().to_string(), &new), base.join("external/B.scala"));
        // A root no longer there: its sources, all in it, are this checkout's.
        let gone = base.join("gone");
        assert_eq!(read(&gone, "src/A.scala", &new), new.join("src/A.scala"));
        assert_eq!(read(&gone, "../shared/B.scala", &new), base.join("shared/B.scala"));
        // The manifest's own checkout: the file itself, as the batch's sources and `--removed` read.
        assert_eq!(read(&old, "link/../B.scala", &old), base.join("external/B.scala"));
        assert_eq!(read(&old, "link/../B.scala", &old), canonical_lenient(&old.join("link/../B.scala")));
        assert_eq!(read(&old, "src/A.scala", &old), old.join("src/A.scala"));
        // A `..` after a part that does not exist, the link before it resolved.
        assert_eq!(canonical_lenient(&old.join("link/gone/../B.scala")), base.join("external/sub/B.scala"));
        assert_eq!(canonical_lenient(&old.join("link/../Missing.scala")), base.join("external/Missing.scala"));
        std::fs::remove_dir_all(&base).unwrap();
    }

    /// A class or TASTy file of one stem in several entries of the class path is the first jar's,
    /// as the scan of the package's entries found it before the index.
    #[test]
    fn names_a_stem_by_its_first_jar() {
        let root = std::env::temp_dir().join(format!("teq-cp-stems-{}", std::process::id()));
        let dirs: Vec<std::path::PathBuf> = (0..3).map(|i| root.join(format!("d{}", i))).collect();
        let files: [&[&str]; 3] = [&["p/B.class", "p/B.tasty"], &["p/A.class", "p/A.tasty", "p/B.class", "A.class"], &["p/A.class", "p/A.tasty", "A.class", "p/C.class"]];
        for (dir, names) in dirs.iter().zip(files) {
            for name in names {
                let path = dir.join(name);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, b"").unwrap();
            }
        }
        let paths: Vec<String> = dirs.iter().map(|d| d.to_string_lossy().to_string()).collect();
        let cp = Classpath::open(&paths, None, None).unwrap();
        let scan = |files: &[CpFile], stem: &str| {
            let mut found: Option<CpFile> = None;
            for &f in files {
                if cp.stem(f) == stem && found.map_or(true, |g| f.jar < g.jar) {
                    found = Some(f);
                }
            }
            found
        };
        let jar = |f: Option<CpFile>| f.map(|f| f.jar);
        assert_eq!(jar(cp.class_named("p/A")), Some(1));
        assert_eq!(jar(cp.class_named("p/B")), Some(0));
        assert_eq!(jar(cp.class_named("p/C")), Some(2));
        assert_eq!(jar(cp.class_named("A")), Some(1));
        assert_eq!(jar(cp.tasty_named("p/A")), Some(1));
        assert_eq!(cp.class_named("p/D"), None);
        assert_eq!(cp.class_named("q/A"), None);
        for p in &cp.packages {
            for &f in p.classes.iter().chain(&p.files) {
                let binary = if p.path.is_empty() { cp.stem(f).to_string() } else { format!("{}/{}", p.path, cp.stem(f)) };
                assert_eq!(cp.class_named(&binary), scan(&p.classes, cp.stem(f)));
                assert_eq!(cp.tasty_named(&binary), scan(&p.files, cp.stem(f)));
            }
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn knows_the_standard_library_by_its_artifact() {
        let maven = "/cache/https/repo1.maven.org/maven2";
        assert!(is_compiler_library(&format!("{maven}/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar")));
        assert!(is_compiler_library(&format!("{maven}/org/scala-lang/scala3-library_3/3.8.4/scala3-library_3-3.8.4.jar")));
        assert!(is_compiler_library(&format!("{maven}/org/scala-lang/scala3-library_sjs1_3/3.8.4/scala3-library_sjs1_3-3.8.4.jar")));
        assert!(is_compiler_library("/opt/scala/lib/scala-library-2.13.16.jar"));
        assert!(!is_compiler_library(&format!("{maven}/org/other/scala-library/2.13.16/scala-library-2.13.16.jar")));
        assert!(!is_compiler_library(&format!("{maven}/org/scala-lang/modules/scala-collection-compat_3/2.14.0/scala-collection-compat_3-2.14.0.jar")));
        assert!(!is_compiler_library("/work/scala-library-tools/scala-library-tools-1.0.jar"));
        assert!(!is_compiler_library("/work/scala-library/lib/counter.jar"));
    }
}
