//! What a native build takes from outside the program: zig, which compiles the C code and links,
//! and the static archives of the C libraries a program links. zig is the one the build names
//! (`--zig=<path>`, else `TEQ_ZIG`), of the version `bench/zig.sh` pins; an archive is found in the
//! directories the build names, else teq compiles its own source of the library, else the build is
//! refused.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The zig a native build takes, the one `bench/zig.sh` pins (`zig_version`).
pub const ZIG_VERSION: &str = "0.16.0";

/// The zig of a build: the one `--zig` names, else `TEQ_ZIG`. It is run, in an empty environment as every zig of the
/// build is (build.rs, `run`), and refused unless it is zig `ZIG_VERSION`.
pub fn zig(flag: Option<&Path>) -> Result<PathBuf, String> {
    let named = match flag {
        Some(path) => Some(("--zig", path.to_path_buf())),
        None => std::env::var_os("TEQ_ZIG").filter(|path| !path.is_empty()).map(|path| ("TEQ_ZIG", PathBuf::from(path))),
    };
    let Some((by, zig)) = named else {
        return Err(format!("not supported on native: a native build compiles C and links with zig {}: --zig=<path> or TEQ_ZIG names it", ZIG_VERSION));
    };
    let zig = std::path::absolute(&zig).unwrap_or(zig);
    let said = Command::new(&zig).arg("version").env_clear().output().map_err(|e| format!("{} names {}, which does not run: {}", by, zig.display(), e))?;
    let version = String::from_utf8_lossy(&said.stdout).trim().to_string();
    if !said.status.success() || version != ZIG_VERSION {
        let found = if said.status.success() && !version.is_empty() { format!("zig {}", version) } else { "no zig that prints its version".to_string() };
        return Err(format!("{} names {}, which is {}: a native build takes zig {}", by, zig.display(), found, ZIG_VERSION));
    }
    Ok(zig)
}

/// Where a static link takes a C library from.
#[derive(Debug, PartialEq)]
pub enum Library {
    /// A static archive of a directory the build names, whose headers are `<dir>/include`
    /// (`includes`).
    Archive(PathBuf),
    /// teq's own source of the library, compiled with the program.
    Own(&'static Own),
}

/// A C library whose source teq carries: its files by name, the one compiled, and the flags it is
/// compiled with.
#[derive(Debug, PartialEq)]
pub struct Own {
    pub files: &'static [(&'static str, &'static str)],
    pub compiled: &'static str,
    pub flags: &'static [&'static str],
}

/// The IDNA library a program that links `idn2` gets: teq's own (`src/native/idn2/`), with GNU
/// libidn2's interface and answers.
static IDN2: Own = Own {
    files: &[("idna.c", include_str!("idn2/idna.c")), ("idn2.h", include_str!("idn2/idn2.h")), ("idna_tables.h", include_str!("idn2/idna_tables.h"))],
    compiled: "idna.c",
    flags: &["-std=c99", "-O2", "-g0", "-fPIC", "-ffunction-sections", "-fdata-sections", "-Wall", "-Wextra"],
};

fn own(name: &str) -> Option<&'static Own> {
    (name == "idn2").then_some(&IDN2)
}

/// The header directories of the directories the build names, `<dir>/include` where there is one,
/// in their order.
pub fn includes(lib_dirs: &[PathBuf]) -> Vec<PathBuf> {
    lib_dirs.iter().map(|dir| dir.join("include")).filter(|dir| dir.is_dir()).collect()
}

/// The C library `name` for `target`: `lib<name>.a` in the first of the directories the build names
/// that holds one, else teq's own source of it; a library found in neither is refused, naming the
/// option by which the build's caller names such a directory (`lib_dir_option`).
pub fn library(name: &str, lib_dirs: &[PathBuf], target: &str, lib_dir_option: &str) -> Result<Library, String> {
    let file = format!("lib{}.a", name);
    if let Some(found) = lib_dirs.iter().map(|dir| dir.join(&file)).find(|archive| archive.is_file()) {
        return Ok(Library::Archive(found));
    }
    own(name).map(Library::Own).ok_or_else(|| format!("not supported on native: the program links the C library {}, and teq has no static archive of it for {}; {} names a directory that holds {}", name, target, lib_dir_option, file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_the_zig_bench_zig_sh_pins() {
        let pinned = include_str!("../../bench/zig.sh").lines().find_map(|line| line.strip_prefix("zig_version="));
        assert_eq!(pinned, Some(ZIG_VERSION));
    }

    /// A directory of the test's own, removed when the test ends.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let dir = std::env::temp_dir().join(format!("teq-tools-{}-{}", name, std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn takes_a_library_from_the_directories_named_first() {
        let scratch = Scratch::new("named");
        let (empty, holding) = (scratch.0.join("empty"), scratch.0.join("holding"));
        std::fs::create_dir_all(&empty).unwrap();
        std::fs::create_dir_all(holding.join("include")).unwrap();
        std::fs::write(holding.join("libcrypto.a"), b"!<arch>\n").unwrap();
        let found = |name: &str, lib_dirs: &[PathBuf]| library(name, lib_dirs, "x86_64-linux-musl", "--lib-dir=<dir>");
        assert_eq!(found("crypto", &[empty.clone(), holding.clone()]), Ok(Library::Archive(holding.join("libcrypto.a"))));
        assert_eq!(includes(&[empty, holding.clone()]), [holding.join("include")]);
        // A directory of the build's own comes before teq's sources too.
        std::fs::write(holding.join("libidn2.a"), b"!<arch>\n").unwrap();
        assert_eq!(found("idn2", &[holding.clone()]), Ok(Library::Archive(holding.join("libidn2.a"))));
        assert_eq!(found("idn2", &[]), Ok(Library::Own(&IDN2)));
    }

    #[test]
    fn refuses_a_library_teq_has_no_archive_of() {
        let refused = library("nonesuch", &[], "x86_64-linux-musl", "--native-lib-dir <dir>");
        assert_eq!(refused, Err("not supported on native: the program links the C library nonesuch, and teq has no static archive of it for x86_64-linux-musl; --native-lib-dir <dir> names a directory that holds libnonesuch.a".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn takes_a_named_zig_of_the_pinned_version_alone() {
        use std::os::unix::fs::PermissionsExt;
        let scratch = Scratch::new("zig");
        let zig_of = |version: &str| {
            let file = scratch.0.join(format!("zig-{version}"));
            std::fs::write(&file, format!("#!/bin/sh\necho {version}\n")).unwrap();
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
            file
        };
        let pinned = zig_of(ZIG_VERSION);
        assert_eq!(zig(Some(&pinned)), Ok(pinned.clone()));
        let other = zig_of("0.15.1");
        assert_eq!(zig(Some(&other)), Err(format!("--zig names {}, which is zig 0.15.1: a native build takes zig {}", other.display(), ZIG_VERSION)));
        let missing = scratch.0.join("none");
        assert!(zig(Some(&missing)).err().unwrap().starts_with(&format!("--zig names {}, which does not run: ", missing.display())));
    }
}
