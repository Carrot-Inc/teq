//! `file://` URIs and the paths they stand for. On Windows a URI names a drive's path as
//! `file:///C:/work/A.scala` (an editor may spell the colon `%3A` and the drive in lower case) and a
//! share's as `file://server/share/A.scala`; the path is the native one, `C:\work\A.scala` and
//! `\\server\share\A.scala`. A path in the extended-length form canonicalisation gives
//! (`\\?\C:\...`, `\\?\UNC\server\share\...`) goes to the editor in the ordinary form, which is also
//! what `canonical` keeps: std adds the extended-length prefix itself where a path needs it for a
//! file operation (past 260 characters).

use std::path::{Path, PathBuf};

pub fn to_path(uri: &str) -> Option<PathBuf> {
    path_of(uri, cfg!(windows)).map(PathBuf::from)
}

/// The path a URI names, Windows' with `windows`.
fn path_of(uri: &str, windows: bool) -> Option<String> {
    let rest = uri.strip_prefix("file://")?;
    let (server, rest) = rest.split_at(rest.find('/')?);
    let path = decode(rest)?;
    // `file://localhost/p` names the same file as `file:///p`; any other authority is a Windows
    // share's server, and so is `localhost` on Windows unless a drive follows: VS Code reads
    // `file://localhost/c%24/x` as `\\localhost\c$\x`, which `uri_of` writes so.
    let drive = |p: &str| p.len() >= 3 && p.as_bytes()[1].is_ascii_alphabetic() && p.as_bytes()[2] == b':';
    let server = if server == "localhost" && (!windows || drive(&path)) { "" } else { server };
    if !server.is_empty() && !windows {
        return None;
    }
    if !windows {
        return Some(path);
    }
    let path = path.replace('/', "\\");
    if !server.is_empty() {
        return Some(format!("\\\\{}{}", decode(server)?, path));
    }
    // `\C:\work` is the drive's `C:\work`.
    let b = path.as_bytes();
    if b.len() >= 3 && b[1].is_ascii_alphabetic() && b[2] == b':' {
        return Some(path[1..].to_string());
    }
    Some(path)
}

fn decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(b) = text.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).ok()
}

pub fn from_path(path: &Path) -> String {
    uri_of(&path.to_string_lossy(), cfg!(windows))
}

/// The URI of a path, Windows' with `windows`.
fn uri_of(path: &str, windows: bool) -> String {
    let mut out = String::from("file://");
    let mut path = path.to_string();
    if windows {
        path = crate::source::ordinary(&path).replace('\\', "/");
        if let Some(share) = path.strip_prefix("//") {
            let (server, rest) = share.split_at(share.find('/').unwrap_or(share.len()));
            encode(&mut out, server, false);
            path = rest.to_string();
        } else if path.as_bytes().get(1) == Some(&b':') {
            path.insert(0, '/');
        }
    }
    encode(&mut out, &path, windows);
    out
}

/// Percent-encodes all but the unreserved characters and `/`, and with `drive` the colon of a
/// leading `/C:`.
fn encode(out: &mut String, text: &str, drive: bool) {
    let colon = (drive && text.len() >= 3 && text.as_bytes()[1].is_ascii_alphabetic() && text.as_bytes()[2] == b':').then_some(2);
    for (i, &b) in text.as_bytes().iter().enumerate() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => out.push(b as char),
            b':' if colon == Some(i) => out.push(':'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
}

/// The canonical form of a path (`source::canonicalize`, the ordinary form on Windows), or the
/// path itself where it does not resolve (a file gone).
pub fn canonical(path: &Path) -> PathBuf {
    crate::source::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let p = Path::new("/a b/c%d/é.scala");
        let uri = from_path(p);
        assert_eq!(uri, "file:///a%20b/c%25d/%C3%A9.scala");
        assert_eq!(to_path(&uri).as_deref(), Some(p));
        assert_eq!(to_path("file://localhost/x").as_deref(), Some(Path::new("/x")));
        assert_eq!(to_path("untitled:1"), None);
        assert_eq!(path_of("file://server/share/x", false), None);
        assert_eq!(path_of("file://localhost-dev/x", false), None);
    }

    /// Windows' forms, by the rules above whatever the platform the test runs on.
    #[test]
    fn windows_forms() {
        let both = |path: &str, uri: &str| {
            assert_eq!(uri_of(path, true), uri, "the URI of {path}");
            assert_eq!(path_of(uri, true).as_deref(), Some(path), "the path of {uri}");
        };
        both(r"C:\work\A.scala", "file:///C:/work/A.scala");
        both(r"C:\a b\é.scala", "file:///C:/a%20b/%C3%A9.scala");
        both(r"\\server\share\A.scala", "file://server/share/A.scala");
        both(r"\\localhost-dev\share\A.scala", "file://localhost-dev/share/A.scala");
        both(r"\\localhost\c$\work\A.scala", "file://localhost/c%24/work/A.scala");
        // The editor's spellings of a drive.
        assert_eq!(path_of("file:///c%3A/work/A.scala", true).as_deref(), Some(r"c:\work\A.scala"));
        assert_eq!(path_of("file://localhost/C:/work/A.scala", true).as_deref(), Some(r"C:\work\A.scala"));
        assert_eq!(path_of("file://localhost/c%3A/work/A.scala", true).as_deref(), Some(r"c:\work\A.scala"));
        // The extended-length forms go out in the ordinary one.
        assert_eq!(uri_of(r"\\?\C:\work\A.scala", true), "file:///C:/work/A.scala");
        assert_eq!(uri_of(r"\\?\UNC\server\share\A.scala", true), "file://server/share/A.scala");
        let long = format!(r"C:\{}\A.scala", "d".repeat(300));
        assert_eq!(path_of(&uri_of(&format!(r"\\?\{}", long), true), true), Some(long));
        // A colon past the drive is a name's, encoded.
        assert_eq!(uri_of(r"C:\a:b", true), "file:///C:/a%3Ab");
        // The identity a canonicalised file keeps: the ordinary form, the one its URI names.
        assert_eq!(crate::source::ordinary_form(PathBuf::from(r"\\?\C:\work\A.scala"), true), PathBuf::from(r"C:\work\A.scala"));
        assert_eq!(crate::source::ordinary_form(PathBuf::from(r"\\?\UNC\server\share\A.scala"), true), PathBuf::from(r"\\server\share\A.scala"));
        assert_eq!(crate::source::ordinary_form(PathBuf::from(r"C:\work\A.scala"), true), PathBuf::from(r"C:\work\A.scala"));
        assert_eq!(crate::source::ordinary_form(PathBuf::from(r"\\?\C:\x"), false), PathBuf::from(r"\\?\C:\x"));
    }
}
