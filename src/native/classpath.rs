//! The class path of a link, after `linker.ClassPath` and `linker.ClassLoader` of Scala Native
//! 0.5.12: directories and jars of NIR files, one file a class named by its path, the first
//! entry that has a class giving it, and the service providers that `META-INF/services` names.
//! An entry is a directory, or a jar by its name, as `io.VirtualDirectory.real` takes it: any
//! other entry is refused before the link, the entry named.

use super::nir::read::{entry_points_flag, PRELUDE_LEN};
use super::nir::{Cx, Global};
use super::scala;
use crate::intern::FxMap;
use crate::zip::Zip;
use std::cell::OnceCell;
use std::path::{Path, PathBuf};

enum Source {
    Dir,
    Jar(Zip),
}

#[derive(Clone)]
enum At {
    File(PathBuf),
    Entry(usize),
}

/// A class's file in the first entry that has it, with its bytes once a link has read them: the
/// link's readers borrow them for as long as the class path stands.
struct ClassFile {
    source: u32,
    at: At,
    bytes: OnceCell<Vec<u8>>,
}

pub struct ClassPath {
    sources: Vec<(String, Source)>,
    files: FxMap<Global, ClassFile>,
    /// Every NIR file of every entry: a class shadowed by an earlier entry is still asked for its
    /// entry points.
    all: Vec<(Global, u32, At)>,
    services: FxMap<Global, Vec<Global>>,
}

/// The files under a directory, each with its name from there, in the names' order. A link to a
/// directory is followed, as Scala Native's `VirtualDirectory` follows it, and one that leads to a
/// directory it lies in is refused, as the JDK's `Files.walk` refuses it there.
fn walk(dir: &Path, prefix: &str, above: &mut Vec<PathBuf>, out: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    let cannot_read = |e: std::io::Error| format!("cannot read {}: {}", dir.display(), e);
    let real = dir.canonicalize().map_err(cannot_read)?;
    if above.contains(&real) {
        return Err(format!("{}: a link to a directory it lies in", dir.display()));
    }
    above.push(real);
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(cannot_read)? {
        entries.push(entry.map_err(cannot_read)?);
    }
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = format!("{}{}", prefix, entry.file_name().to_string_lossy());
        let path = entry.path();
        if path.is_dir() {
            walk(&path, &format!("{}/", name), above, out)?;
        } else {
            out.push((name, path));
        }
    }
    above.pop();
    Ok(())
}

/// What an entry of the class path is to a link.
pub enum Entry {
    Dir,
    Jar,
}

/// An entry of the class path as `io.VirtualDirectory.real` takes it: a directory, else a jar by
/// its name that is there. Any other is refused with why: one that is not there, a file whose name
/// does not end in `.jar` (Scala Native's "Neither a jar, nor a directory"), a jar that is not there
/// ("Jar doesn't exist").
pub fn entry(path: &str) -> Result<Entry, String> {
    let p = Path::new(path);
    if p.is_dir() {
        Ok(Entry::Dir)
    } else if !path.ends_with(".jar") {
        Err(format!("{}: neither a jar nor a directory", path))
    } else if !p.exists() {
        Err(format!("{}: no such jar", path))
    } else {
        Ok(Entry::Jar)
    }
}

/// The entries of a class path as `teq nir` takes it, directories and jars joined by the platform's
/// separator (`@<file>` is an argument file, read before the verb sees it: `argfile`), read as
/// teq's `--classpath` reads them.
pub use crate::classpath::entries;

/// `io.packageNameFromPath`: `java/lang/String.nir` is `java.lang.String`.
fn class_name(path: &str) -> String {
    path.strip_suffix(".nir").unwrap_or(path).trim_start_matches('/').replace('/', ".")
}

impl ClassPath {
    /// The class path of `paths` (`entries`). An entry that is neither a directory nor a jar that
    /// opens (`entry`) is refused, with its number among them.
    pub fn open(cx: &mut Cx, paths: &[String]) -> Result<ClassPath, String> {
        let mut cp = ClassPath { sources: Vec::new(), files: FxMap::default(), all: Vec::new(), services: FxMap::default() };
        for (n, path) in paths.iter().enumerate() {
            let refused = |why: String| format!("the class path, entry {}: {}", n + 1, why);
            let index = cp.sources.len() as u32;
            let mut found: Vec<(String, At)> = Vec::new();
            match entry(path).map_err(refused)? {
                Entry::Dir => {
                    let mut entries = Vec::new();
                    walk(Path::new(path), "", &mut Vec::new(), &mut entries).map_err(refused)?;
                    found.extend(entries.into_iter().map(|(name, file)| (name, At::File(file))));
                    cp.sources.push((path.clone(), Source::Dir));
                }
                Entry::Jar => {
                    let zip = Zip::open(path).map_err(refused)?;
                    found.extend(zip.entries.iter().enumerate().map(|(i, e)| (zip.name(e).to_string(), At::Entry(i))));
                    cp.sources.push((path.clone(), Source::Jar(zip)));
                }
            }
            for (name, at) in found {
                if name.ends_with(".nir") {
                    let class = cx.top(&class_name(&name));
                    cp.files.entry(class).or_insert_with(|| ClassFile { source: index, at: at.clone(), bytes: OnceCell::new() });
                    cp.all.push((class, index, at));
                } else if let Some(service) = name.trim_start_matches('/').strip_prefix("META-INF/services/") {
                    if service.is_empty() || service.contains('/') {
                        continue;
                    }
                    let text = cp.read_at(index, &at)?;
                    let providers = String::from_utf8_lossy(&text).lines().map(str::trim).filter(|l| !l.is_empty()).map(|l| cx.top(l)).collect();
                    // A later entry's list of a service replaces an earlier one's.
                    let service = cx.top(service);
                    cp.services.insert(service, providers);
                }
            }
        }
        Ok(cp)
    }

    fn read_at(&self, source: u32, at: &At) -> Result<Vec<u8>, String> {
        match (&self.sources[source as usize], at) {
            ((_, Source::Jar(zip)), At::Entry(i)) => zip.read(*i),
            (_, At::File(path)) => std::fs::read(path).map_err(|e| format!("cannot read {}: {}", path.display(), e)),
            ((name, _), _) => Err(format!("{}: no such entry", name)),
        }
    }

    /// The bytes of a class's NIR file, none for a class no entry has.
    pub fn read(&self, class: Global) -> Option<Result<&[u8], String>> {
        let file = self.files.get(&class)?;
        if file.bytes.get().is_none() {
            match self.read_at(file.source, &file.at) {
                Ok(bytes) => {
                    let _ = file.bytes.set(bytes);
                }
                Err(e) => return Some(Err(format!("{}: {}", self.sources[file.source as usize].0, e))),
            }
        }
        file.bytes.get().map(|bytes| Ok(bytes.as_slice()))
    }

    /// Where a class was found, for a message.
    pub fn origin(&self, class: Global) -> Option<&str> {
        self.files.get(&class).map(|file| self.sources[file.source as usize].0.as_str())
    }

    /// The classes whose file says it holds an entry point: a static initializer or an exported
    /// function. A file that is no NIR file is passed over, as Scala Native passes it over. They
    /// stand as Scala Native's loader gives them: each entry's in turn, of an entry in the order of
    /// the table its files are filtered into (`nirFiles.filter(...).keySet`), whose entries are added
    /// in the order of the table of its files (`nirFiles`, its files in the order they are listed).
    pub fn classes_with_entry_points(&self, cx: &Cx) -> Result<Vec<Global>, String> {
        let hash = |class: Global| scala::top_hash(cx.id(class));
        let mut classes = Vec::new();
        for source in 0..self.sources.len() as u32 {
            let mut files = scala::HashMap::default();
            for (class, _, at) in self.all.iter().filter(|(_, s, _)| *s == source) {
                files.update(*class, hash(*class), at.clone());
            }
            let mut found = scala::HashMap::default();
            for (class, at) in files.iter() {
                if self.has_entry_points(source, &at)? {
                    found.update(class, hash(class), ());
                }
            }
            classes.extend(found.keys());
        }
        Ok(classes)
    }

    /// Whether a NIR file's prelude says it holds an entry point.
    fn has_entry_points(&self, source: u32, at: &At) -> Result<bool, String> {
        let start = match (&self.sources[source as usize].1, at) {
            (Source::Jar(zip), At::Entry(i)) => zip.read_start(*i, PRELUDE_LEN)?,
            (_, At::File(path)) => {
                use std::io::Read;
                let cannot_read = |e: std::io::Error| format!("cannot read {}: {}", path.display(), e);
                let mut start = Vec::with_capacity(PRELUDE_LEN);
                std::fs::File::open(path).map_err(cannot_read)?.take(PRELUDE_LEN as u64).read_to_end(&mut start).map_err(cannot_read)?;
                start
            }
            _ => return Ok(false),
        };
        Ok(entry_points_flag(&start) == Some(true))
    }

    /// The hashes of the `NIRSource` Scala Native's reader gives the positions of a class's file:
    /// its entry's path (`sun.nio.fs.UnixPath`) and the file's, a jar's (`ZipPath`, from its root)
    /// or under a directory (relative to it).
    pub fn nir_source(&self, class: Global) -> Option<(i32, i32)> {
        let file = self.files.get(&class)?;
        let (entry, source) = &self.sources[file.source as usize];
        let path = match (source, &file.at) {
            (Source::Jar(zip), At::Entry(i)) => scala::zip_path_hash(format!("/{}", zip.name(&zip.entries[*i])).as_bytes()),
            (_, At::File(path)) => scala::unix_path_hash(path.strip_prefix(entry).unwrap_or(path).to_string_lossy().as_bytes()),
            _ => return None,
        };
        Some((scala::unix_path_hash(entry.as_bytes()), path))
    }

    pub fn service_providers(&self, service: Global) -> &[Global] {
        self.services.get(&service).map_or(&[], Vec::as_slice)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn follows_a_link_to_a_directory_and_refuses_one_to_a_directory_it_lies_in() {
        let root = std::env::temp_dir().join(format!("teq-native-classpath-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("classes/a")).unwrap();
        std::fs::create_dir_all(root.join("elsewhere")).unwrap();
        std::fs::write(root.join("elsewhere/B.nir"), b"").unwrap();
        std::os::unix::fs::symlink(root.join("elsewhere"), root.join("classes/a/b")).unwrap();
        let mut found = Vec::new();
        walk(&root.join("classes"), "", &mut Vec::new(), &mut found).unwrap();
        assert_eq!(found.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>(), ["a/b/B.nir"]);
        std::os::unix::fs::symlink(root.join("classes"), root.join("classes/a/up")).unwrap();
        let refused = walk(&root.join("classes"), "", &mut Vec::new(), &mut Vec::new());
        assert!(refused.is_err_and(|e| e.ends_with("a link to a directory it lies in")));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn refuses_each_entry_that_is_neither_a_directory_nor_a_jar_that_opens() {
        let root = std::env::temp_dir().join(format!("teq-native-entries-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("classes")).unwrap();
        std::fs::write(root.join("notes.txt"), b"").unwrap();
        std::fs::write(root.join("broken.jar"), b"not a zip").unwrap();
        let path = |name: &str| root.join(name).to_string_lossy().into_owned();
        let refusal = |entry: &str| {
            let paths = [path("classes"), entry.to_string()];
            ClassPath::open(&mut Cx::new(), &paths).err().unwrap_or_default()
        };
        assert_eq!(refusal(&path("missing.jar")), format!("the class path, entry 2: {}: no such jar", path("missing.jar")));
        assert_eq!(refusal(&path("missing")), format!("the class path, entry 2: {}: neither a jar nor a directory", path("missing")));
        assert_eq!(refusal(&path("notes.txt")), format!("the class path, entry 2: {}: neither a jar nor a directory", path("notes.txt")));
        assert!(refusal(&path("broken.jar")).starts_with(&format!("the class path, entry 2: {}: not a zip archive", path("broken.jar"))));
        assert!(ClassPath::open(&mut Cx::new(), &[path("classes")]).is_ok());
        let (classes, notes) = (path("classes"), path("notes.txt"));
        let joined = ["", classes.as_str(), "", notes.as_str(), ""].join(&crate::classpath::SEPARATOR.to_string());
        assert_eq!(entries(&joined), [classes, notes]);
        let refused = ClassPath::open(&mut Cx::new(), &entries(&joined)).err().unwrap_or_default();
        assert_eq!(refused, format!("the class path, entry 2: {}: neither a jar nor a directory", path("notes.txt")));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
