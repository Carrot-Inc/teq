//! The Scala release teq tracks, the TASTy versions its reader accepts, and by them the scala-library
//! jar a build takes from the coursier caches when `--classpath` names none (a JVM build,
//! `--std=scala-library`). Nothing is downloaded. This file depends on nothing outside the standard
//! library, so that every search of the jar is this one or held to it: proptests/src/targets.rs
//! includes it (`#[path]`), as it cannot depend on the compiler, and tests/support/jars.sh's
//! `scala_library_jar`, which starts `java` on a JVM build's output with the jar, reads its constants
//! and mirrors `choose`, the two held to the layouts of tests/classpath/library/layouts.txt, here and
//! by tests/classpath.sh.

use std::ffi::OsString;
use std::path::PathBuf;

/// The release whose scalac the expectations come from (docs/COMPATIBILITY.md), the jar taken where
/// the caches hold it.
pub const TRACKED: &str = "3.8.4";
/// TASTy's major version, and the newest minor the reader accepts (`tasty::MAJOR_VERSION` and
/// `tasty::MINOR_VERSION` are these): a file of the same major and an older minor reads the same
/// way, a newer minor may hold tags the reader does not know (dotty's
/// `TastyFormat.isVersionCompatible`). Every Scala 3 release writes TASTy `28.<its minor>`, so this
/// minor bounds the releases whose library the reader reads.
pub const TASTY_MAJOR: u32 = 28;
pub const TASTY_MINOR: u32 = 8;

/// The jar of the caches the environment names (`roots`), as `choose` takes it under the tracked
/// release and the reader's minor; without one, the build's refusal (`missing`).
pub fn find_scala_library() -> Result<String, String> {
    let roots = roots(cfg!(windows), &|name| std::env::var_os(name));
    choose(&roots, TRACKED, TASTY_MINOR).map(|jar| jar.to_string_lossy().into_owned()).map_err(|unread| missing(&roots, &unread))
}

/// The coursier caches searched, in order, from the environment `var` reads. On Windows the one teq
/// fetches into (`windows_cache`), with no `HOME` needed; elsewhere `COURSIER_CACHE` when it is set
/// (even empty), then `~/Library/Caches/Coursier/v1` and `~/.cache/coursier/v1`.
pub fn roots(windows: bool, var: &dyn Fn(&str) -> Option<OsString>) -> Vec<PathBuf> {
    if windows {
        return windows_cache(var).into_iter().collect();
    }
    let home = PathBuf::from(var("HOME").unwrap_or_default());
    var("COURSIER_CACHE").map(PathBuf::from).into_iter().chain([home.join("Library/Caches/Coursier/v1"), home.join(".cache/coursier/v1")]).collect()
}

/// Coursier's cache on Windows (`task::fetch::coursier_cache`): a non-empty `COURSIER_CACHE`, else
/// `%LOCALAPPDATA%\Coursier\cache\v1`.
pub fn windows_cache(var: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let set = |name: &str| var(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    set("COURSIER_CACHE").or_else(|| set("LOCALAPPDATA").map(|d| d.join("Coursier").join("cache").join("v1")))
}

/// The jar among the releases of scala-library in `roots` whose version reads as numbers (`3.8.4`,
/// not `3.8.0-RC1`) with 3 the first and whose jar is there. Of those whose minor is at most `minor`
/// (the ones the reader reads), the `tracked` release where there is one, else the newest, the first
/// root's of two equal ones: eligibility comes before the preference, so that a tracked release the
/// reader cannot read is never taken. Without one, the releases of a newer minor the roots hold,
/// sorted and each once.
pub fn choose(roots: &[PathBuf], tracked: &str, minor: u32) -> Result<PathBuf, Vec<String>> {
    let numbers = |version: &str| version.split('.').map(|p| p.parse().ok()).collect::<Option<Vec<u32>>>();
    let tracked = numbers(tracked);
    let mut best: Option<((bool, Vec<u32>), PathBuf)> = None;
    let mut unread: Vec<(Vec<u32>, String)> = Vec::new();
    for root in roots {
        let Ok(entries) = std::fs::read_dir(root.join("https/repo1.maven.org/maven2/org/scala-lang/scala-library")) else { continue };
        for entry in entries.flatten() {
            let version = entry.file_name().to_string_lossy().into_owned();
            let Some(parts) = numbers(&version) else { continue };
            let jar = entry.path().join(format!("scala-library-{version}.jar"));
            if parts.first() != Some(&3) || !jar.is_file() {
                continue;
            }
            if parts.get(1).is_some_and(|&m| m > minor) {
                unread.push((parts, version));
                continue;
            }
            let rank = (tracked.as_ref() == Some(&parts), parts);
            if best.as_ref().map_or(true, |(r, _)| rank > *r) {
                best = Some((rank, jar));
            }
        }
    }
    match best {
        Some((_, jar)) => Ok(jar),
        None => {
            unread.sort();
            unread.dedup();
            Err(unread.into_iter().map(|(_, version)| version).collect())
        }
    }
}

/// Why a build has no jar: the caches searched, and how to put one there; where they hold releases
/// the reader cannot read (`unread`), those and the TASTy it reads.
pub fn missing(roots: &[PathBuf], unread: &[String]) -> String {
    let roots = roots.iter().map(|c| c.to_string_lossy()).collect::<Vec<_>>().join(", ");
    if unread.is_empty() {
        return format!("no scala-library jar: a JVM build links against scala-library-<3.x>.jar, as does --std=scala-library; name it with --classpath, or put a 3.x release in the coursier cache ({roots}), which `cs fetch org.scala-lang:scala-library:{TRACKED}` or any scala-cli run under -S {TRACKED} does");
    }
    format!(
        "no readable scala-library jar: this reader accepts TASTy {TASTY_MAJOR}.0 to {TASTY_MAJOR}.{TASTY_MINOR} (Scala release minor at most 3.{TASTY_MINOR}); found {} in the coursier caches ({roots}); fetch the tracked release with `cs fetch org.scala-lang:scala-library:{TRACKED}`, or name a readable library with --classpath",
        unread.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::{choose, missing, roots, windows_cache, TASTY_MINOR, TRACKED};
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    /// The layouts tests/classpath.sh holds tests/support/jars.sh's `scala_library_jar` to.
    const LAYOUTS: &str = include_str!("../tests/classpath/library/layouts.txt");

    /// A layout's roots under `base`, each holding the releases its field names as directories of
    /// the cache's layout, with the jar (an empty file: nothing reads it) unless a `!` marks it absent.
    fn lay_out(base: &Path, field: &str) -> Vec<PathBuf> {
        field.split(';').enumerate().map(|(i, releases)| {
            let root = base.join(i.to_string());
            std::fs::create_dir_all(&root).unwrap();
            for release in releases.split_whitespace() {
                let (version, jar) = match release.strip_prefix('!') { Some(v) => (v, false), None => (release, true) };
                let dir = root.join("https/repo1.maven.org/maven2/org/scala-lang/scala-library").join(version);
                std::fs::create_dir_all(&dir).unwrap();
                if jar {
                    std::fs::write(dir.join(format!("scala-library-{version}.jar")), b"").unwrap();
                }
            }
            root
        }).collect()
    }

    /// The jar `choose` takes under the tracked release and the reader's minor, as the table writes
    /// it: `<release>@<root>` (the roots counted from 0), or `none` and the releases the reader
    /// cannot read.
    fn chosen(roots: &[PathBuf], tracked: &str, minor: u32) -> String {
        match choose(roots, tracked, minor) {
            Ok(jar) => {
                let release = jar.parent().unwrap();
                let root = roots.iter().position(|r| release.starts_with(r)).unwrap();
                format!("{}@{root}", release.file_name().unwrap().to_string_lossy())
            }
            Err(unread) => ["none".to_string()].into_iter().chain(unread).collect::<Vec<_>>().join(" "),
        }
    }

    /// Every layout of the table gives the jar its row expects.
    #[test]
    fn chooses_each_layouts_jar() {
        let base = std::env::temp_dir().join(format!("teq-scala-release-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let mut rows = 0;
        for line in LAYOUTS.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
            let fields: Vec<&str> = line.split('|').map(str::trim).collect();
            let [name, layout, expected] = fields[..] else { panic!("a row of three fields: {line}") };
            let roots = lay_out(&base.join(name), layout);
            assert_eq!(chosen(&roots, TRACKED, TASTY_MINOR), expected, "layout {name}");
            rows += 1;
        }
        assert!(rows >= 7, "the table's rows: {rows}");
        // A tracked release the reader cannot read is not taken; under a newer reader the tracked
        // release comes before a newer one it reads.
        let roots = lay_out(&base.join("stale"), "3.10.0 3.8.3 3.8.4");
        assert_eq!(chosen(&roots, "3.10.0", 8), "3.8.4@0");
        assert_eq!(chosen(&roots, "3.8.4", 10), "3.8.4@0");
        assert_eq!(chosen(&roots, "3.9.0", 10), "3.10.0@0");
        std::fs::remove_dir_all(&base).unwrap();
    }

    /// The refusals: of caches without a jar, and of caches whose releases the reader cannot read,
    /// those named in order once, the bound the reader's.
    #[test]
    fn refusals_name_the_caches() {
        let roots = [PathBuf::from("/c"), PathBuf::from("/h")];
        assert_eq!(missing(&roots, &[]), "no scala-library jar: a JVM build links against scala-library-<3.x>.jar, as does --std=scala-library; name it with --classpath, or put a 3.x release in the coursier cache (/c, /h), which `cs fetch org.scala-lang:scala-library:3.8.4` or any scala-cli run under -S 3.8.4 does");
        assert_eq!(missing(&roots, &["3.9.0".to_string(), "3.10.0".to_string()]), "no readable scala-library jar: this reader accepts TASTy 28.0 to 28.8 (Scala release minor at most 3.8); found 3.9.0, 3.10.0 in the coursier caches (/c, /h); fetch the tracked release with `cs fetch org.scala-lang:scala-library:3.8.4`, or name a readable library with --classpath");
    }

    /// The roots from the environment: on Unix `COURSIER_CACHE` first when it is set, even empty,
    /// then the two home caches, a `HOME` unset reading as empty; on Windows coursier's own cache
    /// alone, a non-empty `COURSIER_CACHE` before `LOCALAPPDATA`, no `HOME` read and none without
    /// either.
    #[test]
    fn roots_follow_the_environment() {
        let env = |pairs: &'static [(&'static str, &'static str)]| move |name: &str| pairs.iter().find(|(n, _)| *n == name).map(|(_, v)| OsString::from(v));
        let homes = |home: &str| [Path::new(home).join("Library/Caches/Coursier/v1"), Path::new(home).join(".cache/coursier/v1")];
        let [mac, linux] = homes("/h");
        let c = PathBuf::from("/c");
        assert_eq!(roots(false, &env(&[("HOME", "/h"), ("COURSIER_CACHE", "/c")])), [c.clone(), mac.clone(), linux.clone()]);
        assert_eq!(roots(false, &env(&[("HOME", "/h"), ("COURSIER_CACHE", "")])), [PathBuf::new(), mac, linux]);
        assert_eq!(roots(false, &env(&[("COURSIER_CACHE", "/c")])), [&[c.clone()][..], &homes("")[..]].concat());
        let local = Path::new("/l").join("Coursier").join("cache").join("v1");
        assert_eq!(roots(true, &env(&[("HOME", "/h"), ("COURSIER_CACHE", "/c"), ("LOCALAPPDATA", "/l")])), [c]);
        assert_eq!(roots(true, &env(&[("HOME", "/h"), ("COURSIER_CACHE", ""), ("LOCALAPPDATA", "/l")])), [local.clone()]);
        assert_eq!(windows_cache(&env(&[("LOCALAPPDATA", "/l")])), Some(local));
        assert_eq!(roots(true, &env(&[("HOME", "/h"), ("COURSIER_CACHE", ""), ("LOCALAPPDATA", "")])), Vec::<PathBuf>::new());
    }
}
