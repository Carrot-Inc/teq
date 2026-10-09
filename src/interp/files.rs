//! The file system behind `java.nio.file` (`std/javalib/nio.scala`), as the JDK's default provider
//! answers on the host: the natives the std's `Files` and `Path` call, the rules of a path's text,
//! and the exceptions the JDK makes of a system error. A compile-time run (a macro) reads through a
//! cache kept for a build: a file's text and its lines are read once and held as long as its
//! modification time and length stay, so that a class list next to the sources costs one read per
//! build rather than one per expansion. A build starts without them (`forget`), as a macro under
//! scalac reads what is on disk when its run reads the file, and a write forgets the file's entry
//! (`forget_file`), so that a read after it sees what was written. A program under `teq interp`
//! reads afresh each time, and writes; a macro writes none, its expansions having no order under
//! the parallel typer, and a folded constant neither reads nor writes.

use super::builtins::{reg, Table};
use super::value::*;
use super::*;
use crate::intern::FxMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::SystemTime;

struct Entry {
    modified: Option<SystemTime>,
    len: u64,
    text: Rc<str>,
    lines: Rc<Vec<Rc<str>>>,
}

thread_local! {
    static FILES: RefCell<FxMap<String, Entry>> = RefCell::new(FxMap::default());
}

pub(super) fn count() -> usize {
    FILES.with(|files| files.borrow().len())
}

pub(super) fn forget() {
    FILES.with(|files| files.borrow_mut().clear());
}

/// Forgets what the cache holds of the file a path names, under that spelling or another of the
/// same absolute path.
fn forget_file(path: &str) {
    let target = std::path::absolute(path).ok();
    FILES.with(|files| files.borrow_mut().retain(|key, _| key != path && (target.is_none() || std::path::absolute(key).ok() != target)));
}

/// The natives of `std/javalib/nio.scala`, each over the path's text. Every one reads or changes
/// the host, which a folded constant may not; a macro reads as under scalac and changes nothing
/// (`written_path`). A system error is the exception the JDK's provider makes of it on the path
/// (`fail`).
pub(super) fn install(it: &mut Table) {
    reg!(it, "java.nio.file.fileExists", |it, a| {
        let path = host_path(it, a, 0)?;
        Ok(Value::Bool(std::path::Path::new(os(&path)).exists()))
    });
    reg!(it, "java.nio.file.fileIsDirectory", |it, a| {
        let path = host_path(it, a, 0)?;
        Ok(Value::Bool(std::path::Path::new(os(&path)).is_dir()))
    });
    reg!(it, "java.nio.file.fileText", |it, a| {
        let path = host_path(it, a, 0)?;
        match file_text(os(&path), it.pure) {
            Ok(text) => Ok(Value::Str(text)),
            Err(e) => read_failed(it, e, &path, false),
        }
    });
    reg!(it, "java.nio.file.fileLines", |it, a| {
        let path = host_path(it, a, 0)?;
        match file_lines(os(&path), it.pure) {
            Ok(lines) => Ok(Value::array(lines.iter().map(|l| Value::Str(l.clone())).collect())),
            Err(e) => read_failed(it, e, &path, true),
        }
    });
    reg!(it, "java.nio.file.fileBytes", |it, a| {
        let path = host_path(it, a, 0)?;
        use std::io::Read;
        let mut file = match std::fs::File::open(os(&path)) {
            Ok(file) => file,
            Err(e) => return fail(it, &e, &path),
        };
        let mut bytes = Vec::new();
        match file.read_to_end(&mut bytes) {
            Ok(_) => Ok(Value::array(bytes.into_iter().map(|b| Value::Byte(b as i8)).collect())),
            Err(e) => io_exception(it, &e),
        }
    });
    reg!(it, "java.nio.file.fileModified", |it, a| {
        let path = host_path(it, a, 0)?;
        match std::fs::metadata(os(&path)).and_then(|m| m.modified()) {
            Ok(time) => {
                let ms = match time.duration_since(std::time::UNIX_EPOCH) {
                    Ok(d) => d.as_millis() as i64,
                    Err(e) => -(e.duration().as_millis() as i64),
                };
                Ok(Value::Long(ms))
            }
            Err(e) => fail(it, &e, &path),
        }
    });
    // `Files.writeString` and `Files.write`: the text as UTF-8 or the bytes, the file created or
    // truncated; its parent is not.
    reg!(it, "java.nio.file.fileWrite", |it, a| {
        let path = written_path(it, a, 0)?;
        let text = it.str_arg(a, 1)?;
        match strict_utf8(&text) {
            Some(text) => write_file(it, &path, text.as_bytes()),
            None => it.throw_new(&["java", "nio", "charset", "UnmappableCharacterException"], vec![Value::Int(1)]),
        }
    });
    reg!(it, "java.nio.file.fileWriteBytes", |it, a| {
        let path = written_path(it, a, 0)?;
        let bytes: Vec<u8> = match a.get(1) {
            Some(Value::Array(items)) => items.borrow().iter().map(|v| v.as_i64().unwrap_or(0) as u8).collect(),
            _ => return it.throw_named("NullPointerException", "bytes"),
        };
        write_file(it, &path, &bytes)
    });
    reg!(it, "java.nio.file.fileCreateDirectory", |it, a| {
        let path = written_path(it, a, 0)?;
        match std::fs::create_dir(os(&path)) {
            Ok(()) => Ok(Value::Unit),
            Err(e) => fail(it, &e, &path),
        }
    });
    // `checkAccess` with no modes: whether the file is there, following a link; the error otherwise.
    reg!(it, "java.nio.file.fileAccess", |it, a| {
        let path = host_path(it, a, 0)?;
        match std::fs::metadata(os(&path)) {
            Ok(_) => Ok(Value::Unit),
            Err(e) => fail(it, &e, &path),
        }
    });
    // Whether the file is a directory, following a link or not; the error when it cannot be read.
    reg!(it, "java.nio.file.fileDirectory", |it, a| {
        let path = host_path(it, a, 0)?;
        let follow = matches!(a.get(1), Some(Value::Bool(true)));
        let meta = if follow { std::fs::metadata(os(&path)) } else { std::fs::symlink_metadata(os(&path)) };
        match meta {
            Ok(meta) => Ok(Value::Bool(meta.is_dir())),
            Err(e) => fail(it, &e, &path),
        }
    });
    // The names of a directory's entries, in the order the system lists them, as the JDK's
    // directory stream gives them (`.` and `..` left out); a file that is no directory is a
    // `NotDirectoryException`.
    reg!(it, "java.nio.file.fileEntries", |it, a| {
        let path = host_path(it, a, 0)?;
        let entries = match std::fs::read_dir(os(&path)) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotADirectory => return it.throw_new(&["java", "nio", "file", "NotDirectoryException"], vec![Value::Str(path)]),
            Err(e) => return fail(it, &e, &path),
        };
        let mut names = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry) => names.push(Value::string(entry.file_name().to_string_lossy().into_owned())),
                Err(e) => return fail(it, &e, &path),
            }
        }
        Ok(Value::array(names))
    });
    // `Files.deleteIfExists`: a link itself, an empty directory, a file; false when there is none.
    reg!(it, "java.nio.file.fileDelete", |it, a| {
        let path = written_path(it, a, 0)?;
        let directory = match std::fs::symlink_metadata(os(&path)) {
            Ok(meta) => meta.is_dir(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Value::Bool(false)),
            Err(e) => return fail(it, &e, &path),
        };
        let removed = if directory { std::fs::remove_dir(os(&path)) } else { std::fs::remove_file(os(&path)) };
        match removed {
            Ok(()) => Ok(Value::Bool(true)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Value::Bool(false)),
            Err(e) if directory && matches!(e.kind(), std::io::ErrorKind::DirectoryNotEmpty | std::io::ErrorKind::AlreadyExists) => {
                it.throw_new(&["java", "nio", "file", "DirectoryNotEmptyException"], vec![Value::Str(path)])
            }
            Err(e) => fail(it, &e, &path),
        }
    });
    reg!(it, "java.nio.file.pathAbsolute", |it, a| {
        let path = host_path(it, a, 0)?;
        let windows = cfg!(windows);
        if path::is_absolute(&path, windows) {
            return Ok(Value::Str(path));
        }
        match std::env::current_dir() {
            Ok(cwd) => Ok(Value::string(path::absolute(&path, &path::normalized(&cwd.to_string_lossy(), windows), windows, &path::drive_dir))),
            Err(e) => io_exception(it, &e),
        }
    });
    reg!(it, "java.nio.file.pathNormalize", |it, a| {
        let path = it.str_arg(a, 0)?;
        Ok(Value::string(path::normalize(&path, cfg!(windows))))
    });
    reg!(it, "java.nio.file.pathRelativize", |it, a| {
        let (base, other) = (it.str_arg(a, 0)?, it.str_arg(a, 1)?);
        match path::relativize(&base, &other, cfg!(windows)) {
            Ok(rel) => Ok(Value::string(rel)),
            Err(message) => it.throw_named("IllegalArgumentException", &message),
        }
    });
    reg!(it, "java.nio.file.pathStartsWith", |it, a| {
        let (p, other) = (it.str_arg(a, 0)?, it.str_arg(a, 1)?);
        Ok(Value::Bool(path::starts_with(&p, &other, cfg!(windows))))
    });
    reg!(it, "java.nio.file.absentPath", |_it, _a| Ok(Value::Null));
    reg!(it, "java.nio.file.windowsPaths", |_it, _a| Ok(Value::Bool(cfg!(windows))));
}

/// The text as the JDK's UTF-8 encoder without replacement takes it (`Files.writeString`): from
/// its UTF-16 units, a surrogate that pairs with none unmappable, which fails the write before the
/// file is opened. A stand-in of the interpreter's strings (`crate::text`) is the unit it stands for.
fn strict_utf8(text: &str) -> Option<std::borrow::Cow<'_, str>> {
    if !text.chars().any(|c| crate::text::surrogate_of(c).is_some()) {
        return Some(std::borrow::Cow::Borrowed(text));
    }
    char::decode_utf16(crate::text::utf16_units(text)).collect::<Result<String, _>>().ok().map(std::borrow::Cow::Owned)
}

fn write_file(it: &mut Interp, path: &str, bytes: &[u8]) -> R {
    use std::io::Write;
    let mut file = match std::fs::File::create(os(path)) {
        Ok(file) => file,
        Err(e) => return fail(it, &e, path),
    };
    match file.write_all(bytes) {
        Ok(()) => Ok(Value::Unit),
        Err(e) => io_exception(it, &e),
    }
}

/// The file a path names for the system: the empty path is the working directory, as the JDK
/// passes it (`UnixPath.getByteArrayForSysCalls`); an exception names the path as given.
fn os(path: &str) -> &str {
    if path.is_empty() {
        "."
    } else {
        path
    }
}

/// The path argument of a native that reads the host: refused to a folded constant, whose
/// value cannot depend on the disk it is folded on.
fn host_path(it: &mut Interp, a: &[Value], i: usize) -> R<Rc<str>> {
    if it.pure && it.macro_ctx.is_none() {
        return it.impure("the file system");
    }
    it.str_arg(a, i)
}

/// The path argument of a native that changes the host: a program's under `teq interp` alone. A
/// folded constant has no effect; a macro has none on disk, since its expansions run on the
/// parallel typer's workers and may be run again, so that its writes would have no order. The
/// build's cache forgets the file written.
fn written_path(it: &mut Interp, a: &[Value], i: usize) -> R<Rc<str>> {
    if it.pure && it.macro_ctx.is_some() {
        return it.unsupported("a macro writes no file: its expansions run in parallel and may be run again, so its effects on disk would have no order");
    }
    if it.pure {
        return it.impure("writing to the file system");
    }
    let path = it.str_arg(a, i)?;
    forget_file(os(&path));
    Ok(path)
}

/// The class under `java.nio.file` the JDK's provider makes of a system error on a file, and the
/// reason it gives (`UnixException.translateToIOException`, `WindowsException`'s): a file that is
/// not there, access denied, a file already there; a loop of links on Unix; any other error a
/// `FileSystemException` with the system's message.
pub(super) fn translate(e: &std::io::Error) -> (&'static str, Option<String>) {
    let reason = reason(e);
    #[cfg(unix)]
    {
        const ENOENT: i32 = 2;
        const EACCES: i32 = 13;
        const EEXIST: i32 = 17;
        #[cfg(target_os = "linux")]
        const ELOOP: i32 = 40;
        #[cfg(not(target_os = "linux"))]
        const ELOOP: i32 = 62;
        match e.raw_os_error() {
            Some(ENOENT) => ("NoSuchFileException", None),
            Some(EACCES) => ("AccessDeniedException", None),
            Some(EEXIST) => ("FileAlreadyExistsException", None),
            Some(ELOOP) => ("FileSystemException", Some(format!("{} or unable to access attributes of symbolic link", reason))),
            _ => ("FileSystemException", Some(reason)),
        }
    }
    #[cfg(not(unix))]
    {
        const ERROR_ACCESS_DENIED: i32 = 5;
        match e.kind() {
            std::io::ErrorKind::NotFound => ("NoSuchFileException", None),
            std::io::ErrorKind::AlreadyExists => ("FileAlreadyExistsException", None),
            _ if e.raw_os_error() == Some(ERROR_ACCESS_DENIED) => ("AccessDeniedException", None),
            _ => ("FileSystemException", Some(reason)),
        }
    }
}

/// The system's message of an error, as the JDK gives it (`strerror`, `FormatMessage`): Rust's
/// without its `(os error N)`.
fn reason(e: &std::io::Error) -> String {
    let text = e.to_string();
    match text.rfind(" (os error ") {
        Some(at) if text.ends_with(')') => text[..at].to_string(),
        _ => text,
    }
}

/// Throws the exception the JDK makes of the system error `e` on `file`.
fn fail<T>(it: &mut Interp, e: &std::io::Error, file: &str) -> R<T> {
    let (class, reason) = translate(e);
    it.throw_new(&["java", "nio", "file", class], vec![Value::str(file), Value::Null, reason.map_or(Value::Null, Value::string)])
}

/// A plain `IOException` with the system's message: what the JDK throws for an error past opening
/// a file (reading a directory on Unix, a full disk).
fn io_exception<T>(it: &mut Interp, e: &std::io::Error) -> R<T> {
    it.throw_new(&["java", "io", "IOException"], vec![Value::string(reason(e)), Value::Null])
}

fn read_failed<T>(it: &mut Interp, e: ReadError, file: &str, by_lines: bool) -> R<T> {
    match e {
        ReadError::Open(e) => fail(it, &e, file),
        ReadError::Read(e) => io_exception(it, &e),
        ReadError::Malformed { string, decoder } => {
            let length = if by_lines { decoder } else { string };
            it.throw_new(&["java", "nio", "charset", "MalformedInputException"], vec![Value::Int(length as i32)])
        }
    }
}

/// What reading a file as UTF-8 met, as the JDK's readers report it: the failure to open it (the
/// path's exception, `translate`), a failure to read it once open (a plain `IOException` with the
/// system's message: a directory on Unix, which opens), or bytes that are not UTF-8: the length of
/// the first malformed input as `String`'s decoder (`Files.readString`) and as a `CharsetDecoder`
/// (`Files.readAllLines`) state it in their `MalformedInputException`.
pub(super) enum ReadError {
    Open(std::io::Error),
    Read(std::io::Error),
    Malformed { string: usize, decoder: usize },
}

/// `Files.readString`: the text of the file, through the build's cache when `cached`.
pub(super) fn file_text(path: &str, cached: bool) -> Result<Rc<str>, ReadError> {
    if cached {
        return entry(path).map(|(text, _)| text);
    }
    read(path).map(Rc::from)
}

/// `Files.readAllLines`: the lines of the file, through the build's cache when `cached`.
pub(super) fn file_lines(path: &str, cached: bool) -> Result<Rc<Vec<Rc<str>>>, ReadError> {
    if cached {
        return entry(path).map(|(_, lines)| lines);
    }
    read(path).map(|text| Rc::new(split_lines(&text)))
}

/// The rules of `java.nio.file.Path` for a macro's paths, the natives' (`Interp::path_member`) as
/// `std/javalib/nio.scala`'s bodies state them: the JDK's `UnixPath` and, with `windows` (the
/// interpreter on Windows), its `WindowsPath` (`WindowsPathParser`, `WindowsPath.getParent`,
/// `getFileName`, `isAbsolute`, `resolve` in the JDK's
/// `src/java.base/windows/classes/sun/nio/fs/`): `\` and `/` both separate, and a path's root is a
/// drive's (`C:\`, absolute; `C:`, relative to the drive's directory), a share's
/// (`\\server\share\`, absolute) or `\` (relative to the current drive). A separator at the end
/// counts for nothing, as the JDK's parser drops it.
pub(super) mod path {
    pub fn is_separator(c: char, windows: bool) -> bool {
        c == '/' || (windows && c == '\\')
    }

    /// The length of the path's root, zero for a relative path.
    pub fn root_len(path: &str, windows: bool) -> usize {
        let b = path.as_bytes();
        let sep = |i: usize| b.get(i).is_some_and(|&c| is_separator(c as char, windows));
        if !windows {
            return sep(0) as usize;
        }
        if sep(0) && sep(1) {
            // The host and the share each past a run of separators, as the JDK's parser skips them.
            let separator = |from: usize| (from..b.len()).find(|&i| sep(i)).unwrap_or(b.len());
            let name = |from: usize| (from..b.len()).find(|&i| !sep(i)).unwrap_or(b.len());
            let share_end = separator(name(separator(name(2))));
            return if share_end < b.len() { share_end + 1 } else { b.len() };
        }
        if b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
            return if sep(2) { 3 } else { 2 };
        }
        sep(0) as usize
    }

    pub fn is_absolute(path: &str, windows: bool) -> bool {
        let root = root_len(path, windows);
        if !windows {
            return root == 1;
        }
        let b = path.as_bytes();
        (root == 3 && b[1] == b':') || (root >= 2 && is_separator(b[0] as char, true) && is_separator(b[1] as char, true))
    }

    /// The path without the separators at its end, its root kept whole.
    fn trimmed(path: &str, windows: bool) -> &str {
        let root = root_len(path, windows);
        let end = path[root..].trim_end_matches(|c| is_separator(c, windows)).len();
        &path[..root + end]
    }

    /// The last separator past the root.
    fn last_separator(path: &str, windows: bool) -> Option<usize> {
        let root = root_len(path, windows);
        path[root..].rfind(|c| is_separator(c, windows)).map(|at| root + at)
    }

    pub fn parent(path: &str, windows: bool) -> Option<&str> {
        let path = trimmed(path, windows);
        let root = root_len(path, windows);
        match last_separator(path, windows) {
            Some(cut) => Some(&path[..cut]),
            None if root > 0 && path.len() > root => Some(&path[..root]),
            None => None,
        }
    }

    pub fn file_name(path: &str, windows: bool) -> Option<&str> {
        let path = trimmed(path, windows);
        let root = root_len(path, windows);
        match last_separator(path, windows) {
            Some(cut) => Some(&path[cut + 1..]),
            None if root > 0 && root == path.len() => None,
            None => Some(&path[root..]),
        }
    }

    /// The path as the JDK's parsers write it: on Unix a run of `/` as one and none at the end
    /// past the root (`UnixPath.normalizeAndCheck`); on Windows (`windows`, `WindowsPathParser`)
    /// `/` as `\`, a share's root as `\\server\share\`, a run of separators past the root as one
    /// and none at the end.
    pub fn normalized(path: &str, windows: bool) -> String {
        if !windows {
            let mut out = String::with_capacity(path.len());
            for c in path.chars() {
                if !(c == '/' && out.ends_with('/')) {
                    out.push(c);
                }
            }
            if out.len() > 1 && out.ends_with('/') {
                out.pop();
            }
            return out;
        }
        let path = path.replace('/', "\\");
        let root = root_len(&path, true);
        let mut out = String::with_capacity(path.len() + 1);
        if path.starts_with("\\\\") {
            out.push_str("\\\\");
            out.push_str(&path[2..root].split('\\').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("\\"));
            out.push('\\');
        } else {
            out.push_str(&path[..root]);
        }
        out.push_str(&path[root..].split('\\').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("\\"));
        out
    }

    /// `other` against the path, as `UnixPath.resolve` and `WindowsPath.resolve` give it: an absolute
    /// `other` itself; a relative one appended (on Windows to a path that is its root alone with
    /// no separator, `C:` and `data.txt` give `C:data.txt`); on Windows one relative to the current
    /// drive (`\data.txt`) on the path's root, and one relative to a drive (`C:data.txt`) appended
    /// to a path on that drive's root, itself otherwise; both written as the parser writes them
    /// first (`normalized`), as a `Path` of the string is, so that `a` and `b//c/` give `a/b/c`.
    pub fn resolve(path: &str, other: &str, windows: bool) -> String {
        let ends = |s: &str| s.ends_with(|c| is_separator(c, windows));
        if !windows {
            let (path, other) = (normalized(path, false), normalized(other, false));
            return if other.is_empty() {
                path
            } else if root_len(&other, false) > 0 || path.is_empty() {
                other
            } else if ends(&path) {
                format!("{}{}", path, other)
            } else {
                format!("{}/{}", path, other)
            };
        }
        let (path, other) = (normalized(path, true), normalized(other, true));
        let (path, other) = (path.as_str(), other.as_str());
        if other.is_empty() {
            return path.to_string();
        }
        if is_absolute(other, true) {
            return other.to_string();
        }
        let root = &path[..root_len(path, true)];
        if root_len(other, true) == 0 {
            // Appended without a separator to an empty path and to a drive's `C:`; a share's root
            // ends with one, as the JDK's parser writes it.
            let bare = path.is_empty() || (path.len() == 2 && path.as_bytes()[1] == b':');
            return if ends(path) || bare { format!("{}{}", path, other) } else { format!("{}\\{}", path, other) };
        }
        if is_separator(other.as_bytes()[0] as char, true) {
            return if ends(root) { format!("{}{}", root, &other[1..]) } else { format!("{}{}", root, other) };
        }
        if !ends(root) || !root[..root.len() - 1].eq_ignore_ascii_case(&other[..2]) {
            return other.to_string();
        }
        if ends(path) {
            format!("{}{}", path, &other[2..])
        } else {
            format!("{}\\{}", path, &other[2..])
        }
    }

    /// The names of a path as the parser wrote it (`normalized`), past its root: the empty path
    /// has one, the empty name, and a root alone none, as `getNameCount` counts them.
    pub fn names(path: &str, windows: bool) -> Vec<&str> {
        if path.is_empty() {
            return vec![""];
        }
        let root = root_len(path, windows);
        path[root..].split(|c| is_separator(c, windows)).filter(|n| !n.is_empty()).collect()
    }

    /// Two names or paths alike: on Windows without case, a character at a time as
    /// `WindowsPath.compareTo` takes them (`Character.toUpperCase` of a UTF-16 unit, which maps
    /// one character to one and leaves `ß` and the supplementary characters as they are).
    fn same_name(a: &str, b: &str, windows: bool) -> bool {
        if !windows {
            return a == b;
        }
        let upper = |c: char| {
            if c as u32 > 0xFFFF {
                return c;
            }
            let mut u = c.to_uppercase();
            match (u.next(), u.next()) {
                (Some(one), None) => one,
                _ => c,
            }
        };
        a.chars().count() == b.chars().count() && a.chars().zip(b.chars()).all(|(x, y)| x == y || upper(x) == upper(y))
    }

    fn same(a: &str, b: &str, windows: bool) -> bool {
        same_name(a, b, windows)
    }

    /// What kind of path it is: Unix's absolute or relative, Windows' `WindowsPathType` (absolute
    /// `C:\`, a share's, relative, relative to the current drive's root `\`, relative to a drive's
    /// directory `C:`).
    fn kind(path: &str, windows: bool) -> u8 {
        let root = root_len(path, windows);
        if !windows || root == 0 {
            return (root > 0) as u8;
        }
        let b = path.as_bytes();
        if root >= 2 && is_separator(b[0] as char, true) && is_separator(b[1] as char, true) {
            2
        } else if b.len() >= 2 && b[1] == b':' {
            if root == 3 { 1 } else { 4 }
        } else {
            3
        }
    }

    fn join(root: &str, names: &[&str], windows: bool) -> String {
        let mut out = root.to_string();
        out.push_str(&names.join(if windows { "\\" } else { "/" }));
        out
    }

    fn dotted(path: &str, windows: bool) -> bool {
        names(path, windows).iter().any(|n| *n == "." || *n == "..")
    }

    /// The working directory of a Windows drive (`D:`): what `GetFullPathName` makes of the
    /// drive alone, which `std::path::absolute` calls there; none elsewhere.
    pub fn drive_dir(drive: &str) -> Option<String> {
        if !cfg!(windows) {
            return None;
        }
        std::path::absolute(drive).ok().map(|d| d.to_string_lossy().into_owned())
    }

    /// `Path.normalize`: the path without its `.` names and with each name followed by `..` taken
    /// out with it, pass after pass (`UnixPath.normalize`, `WindowsPath.normalize`); a `..` that
    /// has nothing before it stays, but under a root it cannot go above (an absolute path's, or
    /// Windows' `\`), where it goes. With every name gone it is its root, or the empty path.
    pub fn normalize(path: &str, windows: bool) -> String {
        let names = names(path, windows);
        let root = &path[..root_len(path, windows)];
        if path.is_empty() || names.is_empty() {
            return path.to_string();
        }
        let rooted = kind(path, windows) == 1 || kind(path, windows) == 2 || kind(path, windows) == 3;
        let mut ignore: Vec<bool> = names.iter().map(|n| *n == ".").collect();
        let mut remaining = ignore.iter().filter(|i| !**i).count();
        loop {
            let before = remaining;
            let mut previous: Option<usize> = None;
            for i in 0..names.len() {
                if ignore[i] {
                    continue;
                }
                if names[i] != ".." {
                    previous = Some(i);
                    continue;
                }
                if let Some(p) = previous {
                    ignore[p] = true;
                    ignore[i] = true;
                    remaining -= 2;
                    previous = None;
                } else if rooted && !(0..i).any(|j| !ignore[j]) {
                    ignore[i] = true;
                    remaining -= 1;
                }
            }
            if remaining == before {
                break;
            }
        }
        if remaining == names.len() {
            return path.to_string();
        }
        let kept: Vec<&str> = names.iter().zip(&ignore).filter(|(_, i)| !**i).map(|(n, _)| *n).collect();
        join(root, &kept, windows)
    }

    /// `Path.relativize`: the path that resolved against `base` gives `other`, as `UnixPath` and
    /// `WindowsPath` construct it: the empty path for the same path, `other` for an empty base,
    /// both normalized first when either has a `.` or `..` name, then a `..` for each name of the
    /// base past the common names and the rest of `other`. Paths of another kind (an absolute and a
    /// relative one), another root on Windows, and a base whose rest holds a `..` have none: the
    /// `IllegalArgumentException`'s message.
    pub fn relativize(base: &str, other: &str, windows: bool) -> Result<String, String> {
        if same(base, other, windows) {
            return Ok(String::new());
        }
        if kind(base, windows) != kind(other, windows) {
            return Err("'other' is different type of Path".to_string());
        }
        if windows && !same(&base[..root_len(base, true)], &other[..root_len(other, true)], true) {
            return Err("'other' has different root".to_string());
        }
        if base.is_empty() {
            return Ok(other.to_string());
        }
        let (b, c) = if dotted(base, windows) || dotted(other, windows) {
            (normalize(base, windows), normalize(other, windows))
        } else {
            (base.to_string(), other.to_string())
        };
        if b.is_empty() {
            return Ok(c);
        }
        let (bn, cn) = (names(&b, windows), names(&c, windows));
        let mut i = 0;
        while i < bn.len().min(cn.len()) && same_name(bn[i], cn[i], windows) {
            i += 1;
        }
        let rest: Vec<&str> = cn[i..].to_vec();
        let rest_empty = rest.is_empty() || rest == [""];
        if i == bn.len() {
            return Ok(if rest_empty { String::new() } else { join("", &rest, windows) });
        }
        if bn[i..].iter().any(|n| *n == "." || *n == "..") {
            return Err(format!("Unable to compute relative  path from {} to {}", base, other));
        }
        let mut up: Vec<&str> = vec![".."; bn.len() - i];
        if !rest_empty {
            up.extend(rest);
        }
        Ok(join("", &up, windows))
    }

    /// `Path.startsWith(other)`: the same root and `other`'s names the first of the path's, the
    /// empty path starting itself alone; names compared as the platform does (on Windows without
    /// case).
    pub fn starts_with(path: &str, other: &str, windows: bool) -> bool {
        if !same(&path[..root_len(path, windows)], &other[..root_len(other, windows)], windows) || kind(path, windows) != kind(other, windows) {
            return false;
        }
        if other.is_empty() {
            return path.is_empty();
        }
        let (pn, on) = (names(path, windows), names(other, windows));
        on.len() <= pn.len() && pn.iter().zip(&on).all(|(a, b)| same_name(a, b, windows))
    }

    /// `Path.toAbsolutePath` against the process's directory `cwd`: an absolute path itself; a
    /// relative one resolved against it (the empty path is the directory); on Windows one relative
    /// to the current drive's root (`\x`) on that root, and one relative to a drive's directory
    /// (`D:x`) on that drive's working directory (`drive_dir`, Windows' `GetFullPathName` of `D:`,
    /// as the JDK takes it), the process's for its own drive, the drive's root when there is none.
    pub fn absolute(path: &str, cwd: &str, windows: bool, drive_dir: &dyn Fn(&str) -> Option<String>) -> String {
        match kind(path, windows) {
            1 | 2 => path.to_string(),
            0 if path.is_empty() => cwd.to_string(),
            0 => resolve(cwd, path, windows),
            3 => resolve(&cwd[..root_len(cwd, windows)], &path[1..], windows),
            _ => {
                let rest = &path[2..];
                if kind(cwd, windows) == 1 && cwd[..2].eq_ignore_ascii_case(&path[..2]) {
                    resolve(cwd, rest, windows)
                } else {
                    let dir = drive_dir(&path[..2]).map(|d| normalized(&d, true)).filter(|d| kind(d, true) == 1 && d[..2].eq_ignore_ascii_case(&path[..2]));
                    resolve(&dir.unwrap_or_else(|| format!("{}\\", &path[..2])), rest, windows)
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn unix_paths() {
            assert_eq!(parent("/work/A.scala", false), Some("/work"));
            assert_eq!(parent("/work", false), Some("/"));
            assert_eq!(parent("/", false), None);
            assert_eq!(parent("A.scala", false), None);
            assert_eq!(parent("a/b/", false), Some("a"));
            assert_eq!(file_name("/work/A.scala", false), Some("A.scala"));
            assert_eq!(file_name("/", false), None);
            assert_eq!(file_name("a/b/", false), Some("b"));
            assert!(is_absolute("/work", false) && !is_absolute("work", false) && !is_absolute("C:\\work", false));
            assert_eq!(resolve("/work", "data.txt", false), "/work/data.txt");
            assert_eq!(resolve("/work/", "data.txt", false), "/work/data.txt");
            assert_eq!(resolve("/work", "/etc/x", false), "/etc/x");
            assert_eq!(resolve("", "x", false), "x");
            assert_eq!(resolve("/work", "", false), "/work");
            // The operand as the parser writes it, as the std's body takes it.
            assert_eq!(resolve("a", "b//c/", false), "a/b/c");
            assert_eq!(resolve("a//", "/x//y/", false), "/x/y");
            // A backslash is a name's character on Unix.
            assert_eq!(file_name("/work/a\\b", false), Some("a\\b"));
        }

        /// Scalac 3.8.4 on JDK 24 (`UnixPath`): the parser's form, and `normalize`, `relativize`,
        /// `startsWith` and `toAbsolutePath` of it.
        #[test]
        fn unix_algebra() {
            let n = |p: &str| normalized(p, false);
            assert_eq!(n("//a//b/"), "/a/b");
            assert_eq!(n("a/"), "a");
            assert_eq!(n("/"), "/");
            assert_eq!(n("//"), "/");
            assert_eq!(n(""), "");
            for (path, want) in [("a/./b/../c", "a/c"), ("a/..", ""), ("../a", "../a"), ("/..", "/"), ("/../a", "/a"), ("a/../..", ".."), (".", ""), ("", ""), ("./a", "a"), ("a/b/..", "a"), ("/a/./b/../../..", "/"), ("../../a/..", "../..")] {
                assert_eq!(normalize(&n(path), false), want, "normalize({})", path);
            }
            let rel = |a: &str, b: &str| relativize(&n(a), &n(b), false);
            for (a, b, want) in [
                ("a", "a/b", "b"), ("a/b", "a", ".."), ("a/b", "a/c", "../c"), ("a", "a", ""), ("", "a", "a"), ("a", "", ".."),
                ("/a", "/a/b/c", "b/c"), ("/a/b", "/c", "../../c"), ("/", "/a", "a"), ("a/./b", "a/c", "../c"), ("a/../b", "c", "../c"),
                ("a/b", "a/b/../c", "../c"), ("a", "..", "../.."), ("a/b/", "a/b/c/", "c"), (".", "a", "a"), ("a", ".", ".."), ("a/..", "b", "b"), ("x/..", "..", ".."),
            ] {
                assert_eq!(rel(a, b), Ok(want.to_string()), "relativize({}, {})", a, b);
            }
            assert_eq!(rel("a", "/a"), Err("'other' is different type of Path".to_string()));
            assert_eq!(rel("..", "a"), Err("Unable to compute relative  path from .. to a".to_string()));
            let sw = |a: &str, b: &str| starts_with(&n(a), &n(b), false);
            for (a, b, want) in [("abc", "a", false), ("a/b", "a", true), ("a/b", "a/b", true), ("a/b", "a/b/c", false), ("/a/b", "/a", true), ("/a/b", "a", false), ("a", "/", false), ("/a", "/", true), ("a", "", false), ("", "", true), ("a/b/", "a/b", true), ("a//b", "a/b", true)] {
                assert_eq!(sw(a, b), want, "startsWith({}, {})", a, b);
            }
            for (path, want) in [("rel/file", "/w/rel/file"), ("/x/y", "/x/y"), ("", "/w"), (".", "/w/."), ("a/../b", "/w/a/../b"), ("rel/", "/w/rel")] {
                assert_eq!(absolute(&n(path), "/w", false, &|_| None), want, "toAbsolutePath({})", path);
            }
            assert_eq!(absolute("a", "/", false, &|_| None), "/a");
            assert_eq!(names("", false), vec![""]);
            assert!(names("/", false).is_empty());
        }

        /// As `WindowsPath`'s source states them: names compared without case, a root that must
        /// match, a `..` under an absolute or `\` root dropped.
        #[test]
        fn windows_algebra() {
            let n = |p: &str| normalized(p, true);
            assert_eq!(normalize(&n("C:\\a\\.\\b\\..\\c"), true), "C:\\a\\c");
            assert_eq!(normalize(&n("C:\\.."), true), "C:\\");
            assert_eq!(normalize(&n("\\..\\a"), true), "\\a");
            assert_eq!(normalize(&n("C:..\\a"), true), "C:..\\a");
            assert_eq!(normalize(&n("a\\.."), true), "");
            assert_eq!(normalize(&n("\\\\server\\share\\..\\x"), true), "\\\\server\\share\\x");
            let rel = |a: &str, b: &str| relativize(&n(a), &n(b), true);
            assert_eq!(rel("C:\\work\\icons", "C:\\work\\icons\\alpha.svg"), Ok("alpha.svg".to_string()));
            assert_eq!(rel("app\\icons", "app/icons/sub/beta.svg"), Ok("sub\\beta.svg".to_string()));
            assert_eq!(rel("C:\\Work\\a", "c:\\work\\b"), Ok("..\\b".to_string()));
            assert_eq!(rel("C:\\work", "D:\\work"), Err("'other' has different root".to_string()));
            assert_eq!(rel("C:\\work", "work"), Err("'other' is different type of Path".to_string()));
            assert_eq!(rel("a", "a"), Ok(String::new()));
            assert!(starts_with(&n("C:\\Work\\a"), &n("c:/work"), true));
            assert!(!starts_with(&n("C:\\work"), &n("D:\\work"), true));
            assert!(!starts_with(&n("C:\\workshop"), &n("C:\\work"), true));
            assert_eq!(absolute("x", "C:\\w", true, &|_| None), "C:\\w\\x");
            assert_eq!(absolute("\\x", "C:\\w", true, &|_| None), "C:\\x");
            assert_eq!(absolute("C:x", "C:\\w", true, &|_| None), "C:\\w\\x");
            assert_eq!(absolute("D:x", "C:\\w", true, &|_| None), "D:\\x");
            assert_eq!(absolute("\\\\s\\h\\x", "C:\\w", true, &|_| None), "\\\\s\\h\\x");
            assert_eq!(absolute("x", "\\\\s\\h\\w", true, &|_| None), "\\\\s\\h\\w\\x");
            assert_eq!(absolute("", "C:\\w", true, &|_| None), "C:\\w");
            // A drive's own working directory, as Windows keeps one per drive.
            assert_eq!(absolute("D:icons", "C:\\work", true, &|d| (d == "D:").then(|| "D:\\assets".to_string())), "D:\\assets\\icons");
            assert_eq!(absolute("d:icons", "C:\\work", true, &|_| Some("D:\\assets".to_string())), "D:\\assets\\icons");
            // Names without case one character at a time: `ß` is no `SS`.
            assert!(!starts_with(&n("\u{df}"), &n("SS"), true));
            assert_eq!(rel("\u{df}", "SS"), Ok("..\\SS".to_string()));
            assert!(starts_with(&n("C:\\\u{c9}t\u{e9}\\x"), &n("c:\\\u{e9}T\u{c9}"), true));
        }

        /// As the JDK's `WindowsPath` answers.
        #[test]
        fn windows_paths() {
            assert_eq!(parent("C:\\work\\A.scala", true), Some("C:\\work"));
            assert_eq!(parent("C:\\work", true), Some("C:\\"));
            assert_eq!(parent("C:\\", true), None);
            assert_eq!(parent("C:/work/A.scala", true), Some("C:/work"));
            assert_eq!(parent("\\\\server\\share\\dir\\A.scala", true), Some("\\\\server\\share\\dir"));
            assert_eq!(parent("\\\\server\\share\\A.scala", true), Some("\\\\server\\share\\"));
            assert_eq!(parent("\\\\server\\share\\", true), None);
            assert_eq!(parent("C:work\\A.scala", true), Some("C:work"));
            assert_eq!(parent("A.scala", true), None);
            assert_eq!(file_name("C:\\work\\A.scala", true), Some("A.scala"));
            assert_eq!(file_name("C:\\", true), None);
            assert_eq!(file_name("C:", true), None);
            assert_eq!(file_name("C:A.scala", true), Some("A.scala"));
            assert_eq!(file_name("\\\\server\\share\\A.scala", true), Some("A.scala"));
            assert!(is_absolute("C:\\work", true) && is_absolute("C:/work", true) && is_absolute("\\\\server\\share\\x", true));
            assert!(!is_absolute("C:work", true) && !is_absolute("\\work", true) && !is_absolute("work", true));
            assert_eq!(resolve("C:\\work", "data.txt", true), "C:\\work\\data.txt");
            assert_eq!(resolve("C:\\", "data.txt", true), "C:\\data.txt");
            assert_eq!(resolve("C:\\work", "D:\\other", true), "D:\\other");
            assert_eq!(resolve(parent("C:\\work\\A.scala", true).unwrap(), "data.txt", true), "C:\\work\\data.txt");
            assert_eq!(resolve("C:", "data.txt", true), "C:data.txt");
            assert_eq!(resolve("C:\\work", "\\data.txt", true), "C:\\data.txt");
            assert_eq!(resolve("C:work", "\\data.txt", true), "C:\\data.txt");
            assert_eq!(resolve("work", "\\data.txt", true), "\\data.txt");
            assert_eq!(resolve("\\\\server\\share\\dir", "\\data.txt", true), "\\\\server\\share\\data.txt");
            assert_eq!(resolve("C:\\work", "C:data.txt", true), "C:\\work\\data.txt");
            assert_eq!(resolve("C:\\work", "c:data.txt", true), "C:\\work\\data.txt");
            assert_eq!(resolve("C:\\work", "D:data.txt", true), "D:data.txt");
            assert_eq!(resolve("C:work", "C:data.txt", true), "C:data.txt");
            assert_eq!(resolve("\\\\server\\share\\dir", "C:data.txt", true), "C:data.txt");
            assert_eq!(resolve("", "data.txt", true), "data.txt");
            assert_eq!(resolve("\\\\server\\share", "data.txt", true), "\\\\server\\share\\data.txt");
            assert_eq!(resolve("\\\\server\\share\\", "data.txt", true), "\\\\server\\share\\data.txt");
            assert_eq!(resolve("\\", "data.txt", true), "\\data.txt");
            // The operand as the parser writes it, as scalac's macro on a Windows JDK gives it.
            let base = normalized("C:/work", true);
            assert_eq!(base, "C:\\work");
            assert_eq!(resolve(&base, "sub/data.txt", true), "C:\\work\\sub\\data.txt");
            assert_eq!(resolve(&base, "D:/other/data.txt", true), "D:\\other\\data.txt");
            assert_eq!(resolve(&base, "//server/share/data.txt", true), "\\\\server\\share\\data.txt");
            let share = normalized("//server/share", true);
            assert_eq!(share, "\\\\server\\share\\");
            assert_eq!(resolve(&share, "/data.txt", true), "\\\\server\\share\\data.txt");
            assert_eq!(normalized("C:\\work\\\\sub//x\\", true), "C:\\work\\sub\\x");
            assert_eq!(normalized("C:", true), "C:");
            assert_eq!(normalized("a/b/", true), "a\\b");
            assert_eq!(normalized("", true), "");
            // UnixPath's parser too: a run of separators as one, none at the end.
            assert_eq!(normalized("a//b/", false), "a/b");
            // Separators repeated inside a share's root: the root is still the share's.
            let deep = normalized("////server/share/dir", true);
            assert_eq!(deep, "\\\\server\\share\\dir");
            assert_eq!(parent(&deep, true), Some("\\\\server\\share\\"));
            assert_eq!(file_name(parent(&deep, true).unwrap(), true), None);
            assert_eq!(normalized("//server//share", true), "\\\\server\\share\\");
            assert_eq!(root_len("\\\\\\server\\\\share\\x", true), "\\\\\\server\\\\share\\".len());
        }
    }
}

fn entry(path: &str) -> Result<(Rc<str>, Rc<Vec<Rc<str>>>), ReadError> {
    let meta = std::fs::metadata(path).map_err(ReadError::Open)?;
    let (modified, len) = (meta.modified().ok(), meta.len());
    let cached = FILES.with(|files| {
        let files = files.borrow();
        files.get(path).filter(|e| e.modified == modified && e.len == len).map(|e| (e.text.clone(), e.lines.clone()))
    });
    if let Some(found) = cached {
        return Ok(found);
    }
    let text: Rc<str> = Rc::from(read(path)?);
    let lines = Rc::new(split_lines(&text));
    FILES.with(|files| {
        files.borrow_mut().insert(path.to_string(), Entry { modified, len, text: text.clone(), lines: lines.clone() });
    });
    Ok((text, lines))
}

fn read(path: &str) -> Result<String, ReadError> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(ReadError::Open)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(ReadError::Read)?;
    String::from_utf8(bytes).map_err(|e| {
        let bytes = e.as_bytes();
        ReadError::Malformed { string: utf8::string_malformed(bytes).unwrap_or(1), decoder: utf8::decoder_malformed(bytes).unwrap_or(1) }
    })
}

/// `Files.readAllLines`: the lines as `BufferedReader.readLine` ends them, at `\n`, `\r` or `\r\n`; a
/// terminator at the end of the text starts no line, so `a\n\n` is `a` and an empty line.
fn split_lines(text: &str) -> Vec<Rc<str>> {
    let b = text.as_bytes();
    let mut lines = Vec::new();
    let (mut start, mut i) = (0, 0);
    while i < b.len() {
        match b[i] {
            b'\n' => {
                lines.push(Rc::from(&text[start..i]));
                i += 1;
                start = i;
            }
            b'\r' => {
                lines.push(Rc::from(&text[start..i]));
                i += 1;
                if b.get(i) == Some(&b'\n') {
                    i += 1;
                }
                start = i;
            }
            _ => i += 1,
        }
    }
    if start < b.len() {
        lines.push(Rc::from(&text[start..]));
    }
    lines
}

/// The lengths the JDK's two UTF-8 decoders give the first malformed input of a byte string: the
/// one behind `new String(bytes, UTF_8)` without replacement (`String.decodeUTF8_UTF16`, which
/// `Files.readString` reports) and the `CharsetDecoder` (`UTF_8.Decoder.decodeArrayLoop` and
/// `malformedN`, which `Files.readAllLines` reports through its reader, an input cut short at
/// the end counting whole). They reject the same inputs Rust's `from_utf8` does.
pub(super) mod utf8 {
    fn continuation(b: u8) -> bool {
        b & 0xc0 == 0x80
    }

    fn malformed3(b1: u8, b2: u8, b3: u8) -> bool {
        (b1 == 0xe0 && b2 & 0xe0 == 0x80) || !continuation(b2) || !continuation(b3)
    }

    fn malformed3_2(b1: u8, b2: u8) -> bool {
        (b1 == 0xe0 && b2 & 0xe0 == 0x80) || !continuation(b2)
    }

    fn malformed4_2(b1: u8, b2: u8) -> bool {
        (b1 == 0xf0 && !(0x90..=0xbf).contains(&b2)) || (b1 == 0xf4 && b2 & 0xf0 != 0x80) || !continuation(b2)
    }

    fn surrogate3(b1: u8, b2: u8, b3: u8) -> bool {
        let c = ((b1 as u32 & 0x0f) << 12) | ((b2 as u32 & 0x3f) << 6) | (b3 as u32 & 0x3f);
        (0xd800..=0xdfff).contains(&c)
    }

    fn malformed4(b: &[u8]) -> bool {
        let c = ((b[0] as u32 & 0x07) << 18) | ((b[1] as u32 & 0x3f) << 12) | ((b[2] as u32 & 0x3f) << 6) | (b[3] as u32 & 0x3f);
        !continuation(b[1]) || !continuation(b[2]) || !continuation(b[3]) || !(0x10000..0x110000).contains(&c)
    }

    pub fn string_malformed(b: &[u8]) -> Option<usize> {
        let n = b.len();
        let mut sp = 0;
        while sp < n {
            let b1 = b[sp];
            sp += 1;
            if b1 < 0x80 {
                continue;
            }
            if b1 >> 5 == 0b110 && b1 & 0x1e != 0 {
                if sp < n && continuation(b[sp]) {
                    sp += 1;
                    continue;
                }
                return Some(1);
            } else if b1 >> 4 == 0b1110 {
                if sp + 1 < n {
                    if malformed3(b1, b[sp], b[sp + 1]) || surrogate3(b1, b[sp], b[sp + 1]) {
                        return Some(3);
                    }
                    sp += 2;
                    continue;
                }
                return Some(if sp < n && malformed3_2(b1, b[sp]) { 2 } else { 1 });
            } else if b1 >> 3 == 0b11110 {
                if sp + 2 < n {
                    if malformed4(&b[sp - 1..sp + 3]) {
                        return Some(4);
                    }
                    sp += 3;
                    continue;
                }
                return Some(1);
            } else {
                return Some(1);
            }
        }
        None
    }

    pub fn decoder_malformed(b: &[u8]) -> Option<usize> {
        let n = b.len();
        let mut sp = 0;
        while sp < n {
            let b1 = b[sp];
            let rest = n - sp;
            if b1 < 0x80 {
                sp += 1;
            } else if b1 >> 5 == 0b110 && b1 & 0x1e != 0 {
                if rest < 2 {
                    return Some(rest);
                }
                if !continuation(b[sp + 1]) {
                    return Some(1);
                }
                sp += 2;
            } else if b1 >> 4 == 0b1110 {
                if rest < 3 {
                    return Some(if rest > 1 && malformed3_2(b1, b[sp + 1]) { 1 } else { rest });
                }
                let (b2, b3) = (b[sp + 1], b[sp + 2]);
                if malformed3(b1, b2, b3) {
                    return Some(if (b1 == 0xe0 && b2 & 0xe0 == 0x80) || !continuation(b2) { 1 } else { 2 });
                }
                if surrogate3(b1, b2, b3) {
                    return Some(3);
                }
                sp += 3;
            } else if b1 >> 3 == 0b11110 {
                if rest < 4 {
                    if b1 > 0xf4 || (rest > 1 && malformed4_2(b1, b[sp + 1])) {
                        return Some(1);
                    }
                    return Some(if rest > 2 && !continuation(b[sp + 2]) { 2 } else { rest });
                }
                if malformed4(&b[sp..sp + 4]) {
                    let (b2, b3) = (b[sp + 1], b[sp + 2]);
                    return Some(if b1 > 0xf4 || malformed4_2(b1, b2) {
                        1
                    } else if !continuation(b3) {
                        2
                    } else {
                        3
                    });
                }
                sp += 4;
            } else {
                return Some(1);
            }
        }
        None
    }

    /// Scalac 3.8.4 on JDK 24: the lengths `readString` and `readAllLines` reported for each input.
    #[cfg(test)]
    #[test]
    fn as_the_jdk_reports() {
        let cases: &[(&[u8], usize, usize)] = &[
            (&[0xc3, 0x28], 1, 1),
            (&[0x41, 0x80], 1, 1),
            (&[0x41, 0xe2, 0x82], 1, 2),
            (&[0xc3], 1, 1),
            (&[0xc0, 0xaf], 1, 1),
            (&[0xed, 0xa0, 0x80], 3, 3),
            (&[0xf4, 0x90, 0x80, 0x80], 4, 1),
            (&[0xf8, 0x88, 0x80, 0x80, 0x80], 1, 1),
            (&[0xe2, 0x28, 0xa1], 3, 1),
            (&[0xe2, 0x82, 0x28], 3, 2),
            (&[0xf0, 0x28, 0x8c, 0xbc], 4, 1),
            (&[0xf0, 0x9f, 0x28], 1, 2),
            (&[0xf0, 0x9f, 0x98, 0x28], 4, 3),
            (&[0x41, 0xf0, 0x9f, 0x98], 1, 3),
            (&[0xff], 1, 1),
            (&[0xe0, 0x80, 0x80], 3, 1),
        ];
        for &(bytes, string, decoder) in cases {
            assert!(std::str::from_utf8(bytes).is_err());
            assert_eq!(string_malformed(bytes), Some(string), "{:x?}", bytes);
            assert_eq!(decoder_malformed(bytes), Some(decoder), "{:x?}", bytes);
        }
        for valid in ["", "h\u{e9}llo", "\u{1f600}", "\u{ffff}\u{10000}\u{10ffff}"] {
            assert_eq!(string_malformed(valid.as_bytes()), None);
            assert_eq!(decoder_malformed(valid.as_bytes()), None);
        }
    }
}

#[cfg(test)]
#[test]
fn lines_end_as_read_line_ends_them() {
    let shown = |t: &str| split_lines(t).iter().map(|l| format!("<{}>", l)).collect::<Vec<_>>().join(",");
    assert_eq!(shown("a\rb\r"), "<a>,<b>");
    assert_eq!(shown("h\u{e9}llo\n\n"), "<h\u{e9}llo>,<>");
    assert_eq!(shown(""), "");
    assert_eq!(shown("\n"), "<>");
    assert_eq!(shown("a\r\n\r\nb"), "<a>,<>,<b>");
    assert_eq!(shown("x"), "<x>");
    assert_eq!(shown("\r"), "<>");
    assert_eq!(shown("\r\r\n"), "<>,<>");
    assert_eq!(shown("a\n\rb"), "<a>,<>,<b>");
}
