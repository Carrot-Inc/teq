//! `teq stage [project]`: the Docker stage sbt-native-packager's `Docker / stage` writes,
//! from the export's `stage` block, under its directory: `<layer>/<path in the image>` for each
//! mapping. An artifact or a file of the repository is copied, never linked, since Docker copies
//! what it finds; a product is the jar `packageBin` would make of it: the class files of the
//! source files under its configuration's source roots, as the resident's merged analysis
//! attributes them (the staged project's own jar also takes the closure's class files that no
//! source file stands behind, which teq writes for the program, such as `scala.Tuple23`), its
//! resource directories, and a manifest with the export's attributes, `Main-Class` among the
//! staged project's own where the build did not set `Compile / mainClass` to `None`, as
//! `packageBin` writes it; the script runs the main class over the jars in the classpath's
//! order, with `JAVA_OPTS`. The main class is the daemon's: the block's, which the build declared,
//! else the one the products imply as `run` implies it. What the stage held before and
//! the mappings no longer name is removed, as native-packager's own stage starts afresh. The
//! daemon holds the resident while the jars are made, so that no build rewrites the class files
//! they are read from.

use super::analysis::View;
use super::export::{Export, Key, Mapping, Staged};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// What `write` did: the files it staged, by layer.
pub struct Done {
    pub files: usize,
    pub layers: Vec<u32>,
    pub directory: String,
}

/// The manifest's attributes in the order `packageBin` writes them; others follow as given.
const MANIFEST_ORDER: [&str; 9] = [
    "Specification-Title",
    "Specification-Version",
    "Specification-Vendor",
    "Implementation-Title",
    "Implementation-Version",
    "Implementation-Vendor",
    "Implementation-Vendor-Id",
    "Implementation-URL",
    "Main-Class",
];

/// sbt's fixed entry time, 2010-01-01 00:00, as a zip entry's MS-DOS date and time.
const DOS_DATE: u16 = (30 << 9) | (1 << 5) | 1;
const DOS_TIME: u16 = 0;

/// Writes the project's stage from the class directory the resident wrote and its analysis, the
/// start script naming `main`, and the project's own jar's manifest too (`in_manifest`) where
/// the export's attributes do not.
pub fn write(export: &Export, name: &str, classes: &Path, view: &View, main: &str, in_manifest: bool) -> Result<Done, String> {
    let stage = export.projects[name].stage.as_ref().ok_or_else(|| format!("{} has no stage block", name))?;
    let dir = export.path(&stage.directory);
    let relative = export.relative(&dir);
    if dir == export.root || !Path::new(&relative).components().any(|c| c.as_os_str() == "target") {
        return Err(format!("the stage's directory {} is no directory under a target directory of the build, which a stage empties", stage.directory));
    }
    let unattributed = unattributed(classes, view)?;
    let mut written = BTreeSet::new();
    for (layer, mappings) in &stage.layers {
        for mapping in mappings {
            let target = dir.join(layer.to_string()).join(&mapping.to);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {}", parent.display(), e))?;
            }
            stage_one(export, name, mapping, &target, classes, view, &unattributed, main, in_manifest)?;
            written.insert(target);
        }
    }
    remove_others(&dir, &written)?;
    Ok(Done { files: written.len(), layers: stage.layers.keys().copied().collect(), directory: relative })
}

#[allow(clippy::too_many_arguments)]
fn stage_one(export: &Export, name: &str, mapping: &Mapping, target: &Path, classes: &Path, view: &View, unattributed: &[String], main_class: &str, in_manifest: bool) -> Result<(), String> {
    match &mapping.from {
        Staged::Artifact(module) => {
            let entry = export.artifact(name, module).ok_or_else(|| format!("{}'s runtime classpath holds no {}", name, module))?;
            copy(&export.resolve(entry)?, target)
        }
        Staged::File(path) => {
            let source = export.path(path);
            if !source.is_file() {
                return Err(format!("{} names {}, which is no file", super::export::FILE, path));
            }
            copy(&source, target)
        }
        Staged::Product { key, manifest } => {
            let own = key.project == name && key.configuration == "compile";
            let entries = product_entries(export, key, classes, view, if own { unattributed } else { &[] })?;
            // The staged project's own jar names the main class, as `packageBin` writes
            // `Compile / mainClass`: the export's attribute where the build declared it, else
            // the implied one, unless the build set the key to None.
            let mut attributes = manifest.clone();
            if own && in_manifest && !attributes.iter().any(|(k, _)| k == "Main-Class") {
                attributes.push(("Main-Class".to_string(), main_class.to_string()));
            }
            write_if_changed(target, &jar(&attributes, &entries)?)
        }
        Staged::Script { classpath } => {
            let classpath: Vec<&str> = classpath.iter().map(String::as_str).collect();
            write_if_changed(target, script(main_class, &classpath).as_bytes())?;
            executable(target)
        }
    }
}

/// The class files of the closure that no source file stands behind, which teq writes for the
/// program as a whole.
fn unattributed(classes: &Path, view: &View) -> Result<Vec<String>, String> {
    let named = view.class_files();
    let mut out = Vec::new();
    for file in files_under(classes)? {
        if file.ends_with(".class") && !named.contains(file.as_str()) {
            out.push(file);
        }
    }
    Ok(out)
}

/// The entries of a product's jar by their names: its class files, the TASTy beside them, and
/// the files of its configuration's resource directories, read from disk.
fn product_entries(export: &Export, key: &Key, classes: &Path, view: &View, extra: &[String]) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let conf = export.configuration(key).ok_or_else(|| format!("no configuration {}", key))?;
    let mut entries = BTreeMap::new();
    let read = |path: &Path| std::fs::read(path).map_err(|e| format!("cannot read {}: {}", path.display(), e));
    for file in view.class_files_under(&conf.sources).into_iter().chain(extra.iter().map(String::as_str)) {
        entries.insert(file.to_string(), read(&classes.join(file))?);
        if let Some(stem) = file.strip_suffix(".class") {
            let tasty = format!("{}.tasty", stem);
            if let Ok(bytes) = std::fs::read(classes.join(&tasty)) {
                entries.insert(tasty, bytes);
            }
        }
    }
    for root in &conf.resources {
        let dir = export.path(root);
        if !dir.is_dir() {
            continue;
        }
        for file in files_under(&dir)? {
            let bytes = read(&dir.join(&file))?;
            entries.insert(file, bytes);
        }
    }
    Ok(entries)
}

/// A jar as `packageBin` makes it: the manifest first, then every entry and every directory above
/// one, in order of name, stored, at sbt's fixed time, so that the same classes give the same
/// bytes and Docker's layer cache holds.
fn jar(manifest: &[(String, String)], files: &BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>, String> {
    let mut names: BTreeMap<String, &[u8]> = BTreeMap::new();
    for (name, bytes) in files {
        let mut at = 0;
        while let Some(slash) = name[at..].find('/') {
            at += slash + 1;
            names.insert(name[..at].to_string(), &[]);
        }
        names.insert(name.clone(), bytes);
    }
    names.remove("META-INF/MANIFEST.MF");
    let manifest = manifest_text(manifest);
    let entries: Vec<(&str, &[u8])> = std::iter::once(("META-INF/MANIFEST.MF", manifest.as_bytes())).chain(names.iter().map(|(n, b)| (n.as_str(), *b))).collect();
    zip(&entries)
}

/// The manifest as `java.util.jar.Manifest` writes it: `Manifest-Version` first, lines of at most
/// 72 bytes continued after a space, CRLF.
fn manifest_text(attributes: &[(String, String)]) -> String {
    let mut ordered: Vec<&(String, String)> = attributes.iter().filter(|(k, _)| k != "Manifest-Version").collect();
    ordered.sort_by_key(|(k, _)| MANIFEST_ORDER.iter().position(|o| o == k).unwrap_or(MANIFEST_ORDER.len()));
    let mut out = String::new();
    for (key, value) in std::iter::once(&("Manifest-Version".to_string(), "1.0".to_string())).chain(ordered) {
        let line = format!("{}: {}", key, value);
        let mut rest = line.as_str();
        let mut first = true;
        while !rest.is_empty() {
            let room = if first { 72 } else { 71 };
            let mut cut = rest.len().min(room);
            while !rest.is_char_boundary(cut) {
                cut -= 1;
            }
            if !first {
                out.push(' ');
            }
            out.push_str(&rest[..cut]);
            out.push_str("\r\n");
            rest = &rest[cut..];
            first = false;
        }
    }
    out.push_str("\r\n");
    out
}

/// A zip archive of stored entries without zip64: at most 65535 entries and 4 GiB.
fn zip(entries: &[(&str, &[u8])]) -> Result<Vec<u8>, String> {
    if entries.len() > u16::MAX as usize {
        return Err(format!("a jar of {} entries, more than a zip without zip64 holds", entries.len()));
    }
    let mut out: Vec<u8> = Vec::with_capacity(entries.iter().map(|(n, b)| b.len() + 2 * n.len() + 76).sum::<usize>() + 22);
    let mut central: Vec<u8> = Vec::new();
    for (name, data) in entries {
        let offset = u32::try_from(out.len()).map_err(|_| "a jar past 4 GiB, more than a zip without zip64 holds".to_string())?;
        let size = data.len() as u32;
        let crc = crate::zip::crc32(data);
        let header = |out: &mut Vec<u8>| {
            out.extend_from_slice(&10u16.to_le_bytes());
            out.extend_from_slice(&0x0800u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&DOS_TIME.to_le_bytes());
            out.extend_from_slice(&DOS_DATE.to_le_bytes());
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
        };
        out.extend_from_slice(&0x04034b50u32.to_le_bytes());
        header(&mut out);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        central.extend_from_slice(&0x02014b50u32.to_le_bytes());
        central.extend_from_slice(&10u16.to_le_bytes());
        header(&mut central);
        central.extend_from_slice(&[0; 6]);
        let directory = name.ends_with('/');
        central.extend_from_slice(&(if directory { 0x10u32 } else { 0 }).to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let central_offset = u32::try_from(out.len()).map_err(|_| "a jar past 4 GiB, more than a zip without zip64 holds".to_string())?;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x06054b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&central_offset.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    Ok(out)
}

/// The start script: a POSIX shell running the main class over the classpath, its entries
/// relative to the install location, the script's directory's parent, with `JAVA_OPTS` and the
/// script's arguments; `JAVA_HOME`'s java, else the one on `PATH`.
fn script(main_class: &str, classpath: &[&str]) -> String {
    let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
    let entries: Vec<String> = classpath.iter().map(|e| format!("\"$app_home\"/{}", quote(e))).collect();
    format!(
        "#!/bin/sh\napp_home=$(cd \"$(dirname \"$0\")/..\" && pwd -P)\nif [ -n \"$JAVA_HOME\" ]; then java=\"$JAVA_HOME/bin/java\"; else java=java; fi\nexec \"$java\" $JAVA_OPTS -cp {} {} \"$@\"\n",
        entries.join(":"),
        quote(main_class)
    )
}

/// A copy of the file, unless the target has its size and modification time already (rsync's
/// quick check): written beside the target and renamed into place, with the source's time.
fn copy(source: &Path, target: &Path) -> Result<(), String> {
    let meta = std::fs::metadata(source).map_err(|e| format!("cannot read {}: {}", source.display(), e))?;
    if let Ok(existing) = std::fs::symlink_metadata(target) {
        if existing.is_file() && existing.len() == meta.len() && existing.modified().ok() == meta.modified().ok() {
            return Ok(());
        }
    }
    let part = partial(target);
    std::fs::copy(source, &part).map_err(|e| format!("cannot copy {} to {}: {}", source.display(), part.display(), e))?;
    if let Ok(time) = meta.modified() {
        let _ = std::fs::File::options().write(true).open(&part).and_then(|f| f.set_modified(time));
    }
    replace(&part, target)
}

/// The bytes written to the target unless it holds them already.
fn write_if_changed(target: &Path, bytes: &[u8]) -> Result<(), String> {
    if std::fs::symlink_metadata(target).is_ok_and(|m| m.is_file() && m.len() == bytes.len() as u64) && std::fs::read(target).is_ok_and(|b| b == bytes) {
        return Ok(());
    }
    let part = partial(target);
    std::fs::write(&part, bytes).map_err(|e| format!("cannot write {}: {}", part.display(), e))?;
    replace(&part, target)
}

fn partial(target: &Path) -> PathBuf {
    let name = target.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    target.with_file_name(format!(".{}.{}.part", name, std::process::id()))
}

fn replace(part: &Path, target: &Path) -> Result<(), String> {
    if std::fs::symlink_metadata(target).is_ok_and(|m| m.is_dir()) {
        let _ = std::fs::remove_dir_all(target);
    }
    std::fs::rename(part, target).map_err(|e| {
        let _ = std::fs::remove_file(part);
        format!("cannot write {}: {}", target.display(), e)
    })
}

fn executable(target: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o755)).map_err(|e| format!("cannot make {} executable: {}", target.display(), e))?;
    }
    #[cfg(not(unix))]
    let _ = target;
    Ok(())
}

/// The files under a directory, by their paths relative to it with `/`, in order. Links are
/// followed, as sbt's `allSubpaths` follows them, a directory already on the way not twice.
fn files_under(dir: &Path) -> Result<Vec<String>, String> {
    fn walk(dir: &Path, prefix: &str, on_the_way: &mut Vec<PathBuf>, out: &mut Vec<String>) -> Result<(), String> {
        let real = crate::source::canonicalize(dir).map_err(|e| format!("cannot read {}: {}", dir.display(), e))?;
        if on_the_way.contains(&real) {
            return Ok(());
        }
        on_the_way.push(real);
        let entries = std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {}", dir.display(), e))?;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = format!("{}{}", prefix, name);
            let target = entry.path();
            if target.is_dir() {
                walk(&target, &format!("{}/", path), on_the_way, out)?;
            } else if target.is_file() {
                out.push(path);
            }
        }
        on_the_way.pop();
        Ok(())
    }
    let mut out = Vec::new();
    if dir.is_dir() {
        walk(dir, "", &mut Vec::new(), &mut out)?;
    }
    out.sort();
    Ok(out)
}

/// Removes what the stage holds besides the files just written, and the directories left empty.
fn remove_others(dir: &Path, written: &BTreeSet<PathBuf>) -> Result<(), String> {
    fn walk(dir: &Path, written: &BTreeSet<PathBuf>) -> Result<bool, String> {
        let mut empty = true;
        for entry in std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {}", dir.display(), e))?.flatten() {
            let path = entry.path();
            let directory = entry.file_type().is_ok_and(|k| k.is_dir());
            if directory && walk(&path, written)? {
                std::fs::remove_dir(&path).map_err(|e| format!("cannot remove {}: {}", path.display(), e))?;
            } else if !directory && !written.contains(&path) {
                std::fs::remove_file(&path).map_err(|e| format!("cannot remove {}: {}", path.display(), e))?;
            } else {
                empty = false;
            }
        }
        Ok(empty)
    }
    walk(dir, written).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_as_java_writes_it() {
        let long = "a.b".repeat(30);
        let text = manifest_text(&[("Main-Class".to_string(), long.clone()), ("Implementation-Title".to_string(), "api".to_string())]);
        let lines: Vec<&str> = text.split("\r\n").collect();
        assert_eq!(lines[0], "Manifest-Version: 1.0");
        assert_eq!(lines[1], "Implementation-Title: api");
        assert_eq!(lines[2].len(), 72);
        assert_eq!(format!("{}{}", lines[2], &lines[3][1..]), format!("Main-Class: {}", long));
        assert!(lines[3].starts_with(' '));
        assert_eq!(&lines[4..], ["", ""]);
    }

    #[test]
    fn a_jar_reads_back_with_its_directories() {
        let files: BTreeMap<String, Vec<u8>> = [("a/b/C.class", b"cafe".to_vec()), ("a/D.class", b"babe".to_vec()), ("r.txt", b"text".to_vec())].into_iter().map(|(n, b)| (n.to_string(), b)).collect();
        let bytes = jar(&[("Main-Class".to_string(), "a.D".to_string())], &files).unwrap();
        let dir = std::env::temp_dir().join(format!("teq-stage-jar-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("t.jar");
        std::fs::write(&file, &bytes).unwrap();
        let zip = crate::zip::Zip::open(&file.to_string_lossy()).unwrap();
        let names: Vec<String> = zip.entries.iter().map(|e| zip.name(e).to_string()).collect();
        assert_eq!(names, ["META-INF/MANIFEST.MF", "a/", "a/D.class", "a/b/", "a/b/C.class", "r.txt"]);
        let manifest = zip.read(0).unwrap();
        assert_eq!(String::from_utf8(manifest).unwrap(), "Manifest-Version: 1.0\r\nMain-Class: a.D\r\n\r\n");
        assert_eq!(zip.read(4).unwrap(), b"cafe");
        assert_eq!(jar(&[("Main-Class".to_string(), "a.D".to_string())], &files).unwrap(), bytes);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_script_quotes_what_it_names() {
        let text = script("a.Main$", &["lib/x.jar", "lib/it's.jar"]);
        assert!(text.starts_with("#!/bin/sh\n"));
        assert!(text.contains(r#"-cp "$app_home"/'lib/x.jar':"$app_home"/'lib/it'\''s.jar' 'a.Main$' "$@""#));
    }
}
