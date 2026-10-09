//! Where the JDK's class files are read from without a JVM: `lib/ct.sym`, the archive of
//! stripped class files that javac reads for `--release`. An entry is named
//! `<releases>/<module>/<package>/<Class>.sig`, where `<releases>` is one character per release
//! the file applies to, the release number written as a base-36 digit (`8`, `9`, `A` for 10,
//! `B` for 11, ... `O` for 24), which is how `JDKPlatformProvider` in javac reads it:
//! `GHIJK/java.base/java/lang/String.sig` is `String` as it stands in releases 16 to 20. A class
//! appears once per release under exactly one directory.

use crate::intern::FxMap;
use crate::zip::Zip;
use std::path::{Path, PathBuf};

/// The JDK's home: `JAVA_HOME`, the macOS `java_home` tool, then `java` on the path (`java.exe`
/// on Windows). Nothing is run except `java_home`, which prints a path and starts no JVM.
pub fn find_home() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        let p = PathBuf::from(home);
        if p.join("lib").join("ct.sym").is_file() {
            return Some(p);
        }
    }
    if cfg!(target_os = "macos") {
        if let Ok(out) = std::process::Command::new("/usr/libexec/java_home").output() {
            if out.status.success() {
                let p = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
                if p.join("lib").join("ct.sym").is_file() {
                    return Some(p);
                }
            }
        }
    }
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let java = dir.join(if cfg!(windows) { "java.exe" } else { "java" });
        if !java.is_file() {
            continue;
        }
        let resolved = std::fs::canonicalize(&java).unwrap_or(java);
        if let Some(home) = resolved.parent().and_then(Path::parent) {
            if home.join("lib").join("ct.sym").is_file() {
                return Some(home.to_path_buf());
            }
        }
    }
    None
}

pub fn ct_sym_path(home: &Path) -> PathBuf {
    home.join("lib").join("ct.sym")
}

pub fn release_char(release: u32) -> Option<char> {
    char::from_digit(release, 36).map(|c| c.to_ascii_uppercase())
}

pub fn release_of(c: char) -> Option<u32> {
    c.to_digit(36)
}

/// A class file inside `ct.sym`: its entry, module and binary class name, as slices of the
/// central directory.
pub struct SigEntry<'a> {
    pub entry: usize,
    pub module: &'a str,
    /// `java/lang/String`
    pub class: &'a str,
}

pub struct CtSym {
    pub zip: Zip,
    /// The releases the archive has directories for, ascending.
    pub releases: Vec<u32>,
}

impl CtSym {
    pub fn open(path: &str) -> Result<CtSym, String> {
        let zip = Zip::open(path)?;
        let mut releases: Vec<u32> = Vec::new();
        // A release counts where the archive holds class files for it: a JDK's own release may
        // have a directory with its `system-modules` alone (OpenJDK 21's `L/`), its classes being
        // the JDK's modules, which javac reads instead of `ct.sym` for that release.
        for e in &zip.entries {
            let name = zip.name(e);
            if !name.ends_with(".sig") {
                continue;
            }
            let Some(dir) = name.split('/').next() else { continue };
            if dir.contains('-') {
                continue;
            }
            for c in dir.chars() {
                if let Some(r) = release_of(c) {
                    if !releases.contains(&r) {
                        releases.push(r);
                    }
                }
            }
        }
        releases.sort_unstable();
        if releases.is_empty() {
            return Err(format!("{}: no release directories found", path));
        }
        Ok(CtSym { zip, releases })
    }

    pub fn latest_release(&self) -> u32 {
        *self.releases.last().unwrap()
    }

    /// The class files of one release, from the central directory alone; `prefix` narrows them
    /// to a module (`java.base/`) or a package (`java.base/java/lang/`).
    pub fn classes(&self, release: u32, prefix: &str) -> Vec<SigEntry<'_>> {
        self.select(release, |module, class| match prefix.strip_prefix(module) {
            Some(rest) if rest.is_empty() => true,
            Some(rest) => rest.strip_prefix('/').map_or(false, |r| class.starts_with(r)),
            None => module.starts_with(prefix),
        })
    }

    /// The classes of a package (`java/lang`) across the modules of the release, as a loader
    /// lists them: the module is not part of a Java package's name.
    pub fn package(&self, release: u32, package: &str) -> Vec<SigEntry<'_>> {
        self.select(release, |_, class| class.rfind('/').map_or("", |at| &class[..at]) == package)
    }

    /// The classes of a release by package, built once from the central directory, which is
    /// how a loader would hold it: `java/lang` to the entries of its classes over all modules.
    pub fn index(&self, release: u32) -> FxMap<&str, Vec<u32>> {
        let mut by_package: FxMap<&str, Vec<u32>> = FxMap::default();
        let Some(wanted) = release_char(release) else { return by_package };
        for (i, e) in self.zip.entries.iter().enumerate() {
            let name = self.zip.name(e);
            let Some(stem) = name.strip_suffix(".sig") else { continue };
            let Some((dir, rest)) = stem.split_once('/') else { continue };
            if dir.contains('-') || !dir.contains(wanted) {
                continue;
            }
            let Some((_, class)) = rest.split_once('/') else { continue };
            let package = class.rfind('/').map_or("", |at| &class[..at]);
            by_package.entry(package).or_default().push(i as u32);
        }
        by_package
    }

    fn select(&self, release: u32, keep: impl Fn(&str, &str) -> bool) -> Vec<SigEntry<'_>> {
        let Some(wanted) = release_char(release) else { return Vec::new() };
        let mut out = Vec::new();
        for (i, e) in self.zip.entries.iter().enumerate() {
            let name = self.zip.name(e);
            let Some(stem) = name.strip_suffix(".sig") else { continue };
            let Some((dir, rest)) = stem.split_once('/') else { continue };
            if dir.contains('-') || !dir.contains(wanted) {
                continue;
            }
            let Some((module, class)) = rest.split_once('/') else { continue };
            if keep(module, class) {
                out.push(SigEntry { entry: i, module, class });
            }
        }
        out.sort_by(|a, b| a.module.cmp(b.module).then(a.class.cmp(b.class)));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_digits() {
        assert_eq!(release_char(8), Some('8'));
        assert_eq!(release_char(9), Some('9'));
        assert_eq!(release_char(10), Some('A'));
        assert_eq!(release_char(24), Some('O'));
        assert_eq!(release_of('O'), Some(24));
        assert_eq!(release_of('8'), Some(8));
    }
}
