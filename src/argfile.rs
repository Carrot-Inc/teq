//! Argument files (docs/TARGETS.md, "Argument files"): an argument `@<path>` on teq's command
//! line stands for the file's lines, one argument per line. Windows refuses a command line over
//! 32,767 characters (error 206), which a class path of a few hundred jars runs past, so every
//! writer of a command that carries a class path puts the arguments after the program in a file
//! and starts `teq @<file>`, on every platform.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static ARGS: OnceLock<Vec<String>> = OnceLock::new();

/// Expands the process's arguments, once, before anything reads them; a file that cannot be read
/// ends the process with a refusal naming it.
pub fn init() {
    match expand(std::env::args_os().skip(1)) {
        Ok(args) => {
            let _ = ARGS.set(args);
        }
        Err(e) => {
            eprintln!("teq: {}", e);
            std::process::exit(2);
        }
    }
}

/// The process's arguments after the program, each `@<path>` replaced by the file's lines.
pub fn args() -> &'static [String] {
    ARGS.get().expect("argfile::init runs first")
}

/// The arguments with each `@<path>` replaced by its file's lines, in place, scanned left to
/// right up to the first `--`, on the command line or among a file's lines: what follows is the
/// program's (`teq interp`'s, a project's `teq run` and `teq dev`), an `@` there an argument as
/// under scalac. A file's lines are
/// taken as they are, so a `@` inside the file is a character of an argument. A file's path is
/// taken as the system gave it (a writer's file under a build whose path is no UTF-8); any other
/// argument that is no UTF-8 is refused.
pub fn expand(args: impl IntoIterator<Item = OsString>) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut program = false;
    for arg in args {
        let Some(path) = file_of(&arg).filter(|_| !program) else {
            let arg = arg.into_string().map_err(|a| format!("the argument {} is not UTF-8", a.to_string_lossy()))?;
            program |= arg == "--";
            out.push(arg);
            continue;
        };
        let bytes = std::fs::read(&path).map_err(|e| format!("cannot read the argument file {}: {}", path.display(), e))?;
        let text = String::from_utf8(bytes).map_err(|_| format!("the argument file {} is not UTF-8", path.display()))?;
        let lines = lines(&text);
        program |= lines.iter().any(|l| l == "--");
        out.extend(lines);
    }
    Ok(out)
}

/// The file an argument `@<path>` names.
fn file_of(arg: &OsStr) -> Option<PathBuf> {
    let bytes = arg.as_encoded_bytes();
    if bytes.first() != Some(&b'@') {
        return None;
    }
    // SAFETY: what follows an ASCII character at the start of an encoded `OsStr` is one too
    // (`OsStr::from_encoded_bytes_unchecked`: a split right after a non-empty UTF-8 substring).
    Some(PathBuf::from(unsafe { OsStr::from_encoded_bytes_unchecked(&bytes[1..]) }))
}

/// A file's arguments: a line ends at `\n`, one `\r` before it dropped; the final line end closes
/// the last argument, with no empty argument after it.
fn lines(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        match rest.split_once('\n') {
            Some((line, next)) => {
                out.push(line.strip_suffix('\r').unwrap_or(line).to_string());
                rest = next;
            }
            None => {
                out.push(rest.to_string());
                rest = "";
            }
        }
    }
    out
}

/// Writes the arguments for a process started as `teq @<path>`, one per line, the directory made;
/// the file is replaced whole (written beside it and renamed), and left as it is when it holds
/// them already. An argument with a line end in it is refused: the file would read it as two.
pub fn write(path: &Path, args: &[String]) -> Result<(), String> {
    if let Some(a) = args.iter().find(|a| a.contains(['\n', '\r'])) {
        return Err(format!("cannot pass {:?} in an argument file: it holds a line end", a));
    }
    let mut text = String::with_capacity(args.iter().map(|a| a.len() + 1).sum());
    for a in args {
        text.push_str(a);
        text.push('\n');
    }
    if std::fs::read(path).is_ok_and(|old| old == text.as_bytes()) {
        return Ok(());
    }
    let failed = |e: std::io::Error| format!("cannot write the argument file {}: {}", path.display(), e);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(failed)?;
    }
    let mut written = path.as_os_str().to_owned();
    written.push(format!(".{}.tmp", std::process::id()));
    std::fs::write(&written, &text).map_err(failed)?;
    std::fs::rename(&written, path).map_err(|e| {
        let _ = std::fs::remove_file(&written);
        failed(e)
    })
}

/// The argument that stands for the file.
pub fn at(path: &Path) -> OsString {
    let mut arg = OsString::from("@");
    arg.push(path);
    arg
}

/// The file under `dir` for a command whose arguments it holds, named by them: `<stem>-<digest>.args`.
/// Two commands with other arguments never share one, and none is rewritten under the process it
/// started, whose retry by one worker reads it again; the same arguments share the same file.
pub fn named_by(dir: &Path, stem: &str, args: &[String]) -> PathBuf {
    let digest = crate::task::sha256::hex(args.join("\n").as_bytes());
    dir.join(format!("{}-{}.args", stem, &digest[..16]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("teq-argfile-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn strings(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    fn os(a: &[&str]) -> Vec<OsString> {
        a.iter().map(OsString::from).collect()
    }

    #[test]
    fn a_file_stands_for_its_lines() {
        let d = dir("lines");
        let f = d.join("a.args");
        std::fs::write(&f, "compiler\r\nwatch\r\n/a dir/with spaces/ünï.jar\r\n@not-a-file\n\nlast").unwrap();
        let at = format!("@{}", f.display());
        let expected = strings(&["compiler", "watch", "/a dir/with spaces/ünï.jar", "@not-a-file", "", "last"]);
        assert_eq!(expand(os(&[&at])).unwrap(), expected);
        let mut after = strings(&["compiler", "watch"]);
        after.extend(expected.iter().cloned());
        after.push("--check".to_string());
        assert_eq!(expand(os(&["compiler", "watch", &at, "--check"])).unwrap(), after);
        // A `\r` closes no line; one with no `\n` after it is a character of the argument.
        std::fs::write(&f, "a\r\r\nb\rc\n").unwrap();
        assert_eq!(expand(os(&[&at])).unwrap(), strings(&["a\r", "b\rc"]));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// What follows the first `--` is the program's: an `@` there is an argument, the file
    /// existing or not, whether the `--` is on the command line or among a file's lines.
    #[test]
    fn nothing_after_a_double_dash_is_expanded() {
        let d = dir("dashes");
        let f = d.join("f.args");
        std::fs::write(&f, "interp\nHi.scala\n").unwrap();
        let (at_f, missing) = (format!("@{}", f.display()), format!("@{}", d.join("missing").display()));
        assert_eq!(expand(os(&[&at_f, "--", &at_f, &missing, "--"])).unwrap(), strings(&["interp", "Hi.scala", "--", &at_f, &missing, "--"]));
        let g = d.join("g.args");
        std::fs::write(&g, "interp\nHi.scala\n--\n@inside\n").unwrap();
        let at_g = format!("@{}", g.display());
        assert_eq!(expand(os(&[&at_g, &at_f, &missing])).unwrap(), strings(&["interp", "Hi.scala", "--", "@inside", &at_f, &missing]));
        // A `--` after the files leaves them expanded.
        assert_eq!(expand(os(&["compiler", "check", &at_f, "--"])).unwrap(), strings(&["compiler", "check", "interp", "Hi.scala", "--"]));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_empty_file_is_no_argument_and_a_line_end_an_empty_one() {
        let d = dir("empty");
        let f = d.join("e.args");
        std::fs::write(&f, "").unwrap();
        assert_eq!(expand(os(&["check", &format!("@{}", f.display())])).unwrap(), strings(&["check"]));
        std::fs::write(&f, "\n").unwrap();
        assert_eq!(expand(os(&[&format!("@{}", f.display())])).unwrap(), strings(&[""]));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_file_that_cannot_be_read_is_refused_naming_it() {
        let d = dir("missing");
        let f = d.join("missing.args");
        let e = expand(os(&["compiler", "watch", &format!("@{}", f.display())])).unwrap_err();
        assert!(e.starts_with(&format!("cannot read the argument file {}: ", f.display())), "{}", e);
        std::fs::write(&f, b"\xff\xfe").unwrap();
        assert_eq!(expand(os(&[&format!("@{}", f.display())])).unwrap_err(), format!("the argument file {} is not UTF-8", f.display()));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_long_file_is_read_whole_and_reads_back_what_was_written() {
        let d = dir("long");
        let f = d.join("sub/long.args");
        let args: Vec<String> = (0..2000).map(|i| format!("/home/someone/.cache/coursier/v1/https/repo1.maven.org/maven2/org/example/lib-{i}/1.0/lib-{i}-1.0.jar")).collect();
        write(&f, &args).unwrap();
        assert!(std::fs::metadata(&f).unwrap().len() > 100_000);
        assert_eq!(expand(vec![at(&f)]).unwrap(), args);
        // Written again with the same arguments, the file is left as it is.
        let before = std::fs::metadata(&f).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        write(&f, &args).unwrap();
        assert_eq!(std::fs::metadata(&f).unwrap().modified().unwrap(), before);
        assert_eq!(std::fs::read_dir(f.parent().unwrap()).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A build under a directory whose name is no UTF-8: the file's path is read as the system
    /// gave it, and an argument that is no UTF-8 is refused rather than a panic.
    #[cfg(unix)]
    #[test]
    fn a_file_under_a_path_that_is_no_utf8() {
        use std::os::unix::ffi::OsStrExt;
        let d = dir("bytes").join(OsStr::from_bytes(b"project-\xff"));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("target/teq/build.args");
        write(&f, &strings(&["build", "Main.scala"])).unwrap();
        assert_eq!(expand(vec![at(&f)]).unwrap(), strings(&["build", "Main.scala"]));
        let e = expand(vec![OsString::from("check"), d.join("Main.scala").into_os_string()]).unwrap_err();
        assert!(e.starts_with("the argument ") && e.ends_with(" is not UTF-8"), "{}", e);
        let _ = std::fs::remove_dir_all(d.parent().unwrap());
    }

    #[test]
    fn a_file_named_by_its_arguments() {
        let d = dir("named");
        let (a, b) = (strings(&["watch", "x"]), strings(&["watch", "y"]));
        assert_eq!(named_by(&d, "watch", &a), named_by(&d, "watch", &a));
        assert_ne!(named_by(&d, "watch", &a), named_by(&d, "watch", &b));
        assert!(named_by(&d, "build", &a).file_name().unwrap().to_str().unwrap().starts_with("build-"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_argument_with_a_line_end_is_refused() {
        let d = dir("refused");
        let f = d.join("r.args");
        for bad in ["two\nlines", "carriage\r"] {
            assert!(write(&f, &strings(&["watch", bad])).unwrap_err().contains("holds a line end"));
        }
        assert!(!f.exists());
        write(&f, &strings(&["", "@x", "a b"])).unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "\n@x\na b\n");
        assert_eq!(expand(vec![at(&f)]).unwrap(), strings(&["", "@x", "a b"]));
        let _ = std::fs::remove_dir_all(&d);
    }
}
