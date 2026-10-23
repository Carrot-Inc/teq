//! `teq fix`'s SemanticDB forms: a source file's document as dotty 3.8.4's `ExtractSemanticDB` makes it, computed
//! from the typer's answers (tests/semanticdb/TABLE.md), and the test-only dump of it that tests/semanticdb compares
//! with scalac's (`TEQ_FIX_SEMANTICDB`, docs/FIX.md).

pub mod document;

use std::path::{Component, Path, PathBuf};

use document::TextDocument;
use crate::types::{Retention, Type, TypeId};

/// The document of the file at `path` whose text is `text`, as ExtractSemanticDB.scala 168-178 heads it: the uri of
/// its path relative to `root` (the `-sourceroot`), the MD5 of its text.
pub fn document(path: &Path, text: &str, root: &Path) -> TextDocument {
    TextDocument { uri: uri_of(&parts(&relative_path(path, root))), md5: md5_hex(text), ..TextDocument::default() }
}

/// `SourceFile.relativePath` (SourceFile.scala 276-306): the file's absolute, normalized path past the root's, where
/// the root is a prefix of it; else the path as it was given (`jpath.toString`), as text, whose elements Java's path
/// keeps as written where Rust's `components` drops an interior `.`.
fn relative_path(path: &Path, root: &Path) -> String {
    match normalized(path).strip_prefix(normalized(root)) {
        Ok(rest) => rest.components().filter_map(|c| if let Component::Normal(p) = c { Some(p.to_string_lossy().into_owned()) } else { None }).collect::<Vec<_>>().join("/"),
        Err(_) => path.to_string_lossy().into_owned(),
    }
}

/// The names of a path's elements as Java's `Path.iterator` gives them: its root left out, an empty element (a
/// repeated or a trailing separator) dropped, `.` and `..` as written.
fn parts(path: &str) -> Vec<String> {
    path.split(|c| c == '/' || c == std::path::MAIN_SEPARATOR).filter(|e| !e.is_empty()).map(String::from).collect()
}

/// `toAbsolutePath.normalize`: against the working directory, `.` dropped and `..` taken back lexically.
fn normalized(p: &Path) -> PathBuf {
    let abs = if p.is_absolute() { p.to_path_buf() } else { std::env::current_dir().unwrap_or_default().join(p) };
    let mut out = PathBuf::new();
    for c in abs.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

/// `Tools.mkURIstring` (Tools.scala 11-18): each part through `new java.net.URI(null, null, "/" + part, null)`,
/// whose `quote` (URI.java) escapes, as the UTF-8 bytes of the character, an ASCII character that is no legal path
/// character and, above 0x7F, a space character (`Character.isSpaceChar`: the categories Zs, Zl, Zp) or an ISO
/// control (0x80-0x9F); every other character is kept as it is.
fn uri_of(parts: &[String]) -> String {
    const SAFE: &[u8] = b"-_.!~*'():@&=+$,;";
    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            out.push('/');
        }
        for ch in part.chars() {
            let keep = if ch.is_ascii() { ch.is_ascii_alphanumeric() || SAFE.contains(&(ch as u8)) } else { !is_space_char(ch) && !('\u{80}'..='\u{9f}').contains(&ch) };
            if keep {
                out.push(ch);
            } else {
                let mut buf = [0u8; 4];
                for b in ch.encode_utf8(&mut buf).bytes() {
                    out.push_str(&format!("%{:02X}", b));
                }
            }
        }
    }
    out
}

/// `Character.isSpaceChar` above 0x7F: the characters of the categories Zs, Zl and Zp.
fn is_space_char(ch: char) -> bool {
    matches!(ch, '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}')
}

/// `internal.MD5.compute` (MD5.scala): the digest of the text's UTF-8 bytes, in upper-case hexadecimal.
fn md5_hex(text: &str) -> String {
    crate::interp::archive::md5(text.as_bytes()).iter().map(|b| format!("{:02X}", b)).collect()
}

/// Writes `doc` where ExtractSemanticDB.scala 209-214 puts a file's document under `-semanticdb-target <dir>`:
/// `<dir>/META-INF/semanticdb` resolving the relative path (`relative_path`: an absolute one, of a file outside the
/// root given absolute, is that path, as `Path.resolve` has it), its sibling named after the file.
pub fn write_dump(dir: &Path, path: &Path, root: &Path, doc: &TextDocument) -> std::io::Result<()> {
    let target = dump_path(dir, path, root);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(target, document::write_documents(doc))
}

/// Writes `view` (`retained_view`) beside the document `write_dump` writes, its name's `.semanticdb` `.forms`.
pub fn write_view(dir: &Path, path: &Path, root: &Path, view: &str) -> std::io::Result<()> {
    std::fs::write(dump_path(dir, path, root).with_extension("forms"), view)
}

fn dump_path(dir: &Path, path: &Path, root: &Path) -> PathBuf {
    let mut target = dir.join("META-INF").join("semanticdb").join(relative_path(path, root));
    // `resolveSibling(name)`: the last element replaced, whatever the path's elements before it.
    let name = format!("{}.semanticdb", path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
    target.set_file_name(name);
    target
}

/// The types of the definitions of the files `files` (a file's own entry and its package blocks'), a line each in
/// source order, `owner.name: type`, with the retained forms written as dotty's typed tree prints them
/// (`Phantom[String]`, `Int @Ann`, `? <: Any @Ann`): a test's view of what the typer keeps beside its canonical types
/// (`TEQ_FIX_SEMANTICDB`), until the documents read the forms themselves.
pub(crate) fn retained_view(w: &mut super::Worker<'_>, files: &[crate::source::FileId]) -> String {
    use crate::symbols::{Owner, SymKind};
    let mut defs = Vec::new();
    for i in 0..w.syms.syms.len() as u32 {
        let s = crate::types::SymId(i);
        let info = w.syms.sym(s);
        if !files.contains(&info.file) || info.def.is_none() || !matches!(info.kind, SymKind::Val | SymKind::Var | SymKind::Def | SymKind::Given) {
            continue;
        }
        let owner = match info.owner {
            Owner::Class(c) => w.name_ref(w.syms.class(c).name).to_string(),
            _ => continue,
        };
        defs.push((info.span.start, s, owner));
    }
    defs.sort_by_key(|d| d.0);
    let mut out = String::new();
    for (_, s, owner) in defs {
        let Some(sig) = w.syms.sym(s).sig.clone() else { continue };
        let name = w.name_ref(w.syms.sym(s).name).to_string();
        out.push_str(&format!("{}.{}: {}\n", owner, name, show_retained(w, sig.ret)));
    }
    out
}

/// A type with its retained forms written out (`retained_view`).
fn show_retained(w: &mut super::Worker<'_>, t: TypeId) -> String {
    if let Some((under, r)) = w.types.form_of(t) {
        return match w.types.retention(r) {
            Retention::Alias(a, args) => {
                let name = w.name_ref(w.syms.aliases[a.idx()].name).to_string();
                let args: Vec<TypeId> = w.types.items(args).to_vec();
                if args.is_empty() { name } else { format!("{}[{}]", name, args.iter().map(|&x| show_retained(w, x)).collect::<Vec<_>>().join(", ")) }
            }
            Retention::Annot(c, _) => format!("{} @{}", show_retained(w, under), w.name_ref(w.syms.class(c).name)),
            Retention::Object(m) => format!("{}.type", w.name_ref(w.syms.sym(m).name)),
            Retention::Written(x) => show_retained(w, x),
        };
    }
    match w.types.get(t) {
        Type::Class(c, args) if args != crate::types::EMPTY_LIST => {
            let args: Vec<TypeId> = w.types.items(args).to_vec();
            let name = w.name_ref(w.syms.class(c).name).to_string();
            format!("{}[{}]", name, args.iter().map(|&x| show_retained(w, x)).collect::<Vec<_>>().join(", "))
        }
        Type::BoundedWild(lo, hi) => {
            // A bound dotty's printer leaves out where it is `Nothing` or `Any` (`isRef`, through an annotation).
            let lo = if w.types.canonical(lo) == crate::types::NOTHING { String::new() } else { format!(" >: {}", show_retained(w, lo)) };
            let hi = if hi == crate::types::ANY { String::new() } else { format!(" <: {}", show_retained(w, hi)) };
            format!("?{}{}", lo, hi)
        }
        _ => w.show(t),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris_as_java_net_uri_quotes_them() {
        let parts = |s: &str| s.split('/').map(String::from).collect::<Vec<_>>();
        assert_eq!(uri_of(&parts("a/filename with spaces.scala")), "a/filename%20with%20spaces.scala");
        assert_eq!(uri_of(&parts("A\u{200d}B.scala")), "A\u{200d}B.scala");
        assert_eq!(uri_of(&parts("x\u{a0}y#z.scala")), "x%C2%A0y%23z.scala");
        assert_eq!(uri_of(&parts("p:q/Ü.scala")), "p:q/Ü.scala");
    }

    #[test]
    fn a_file_outside_the_root_is_named_by_its_path_as_given() {
        let root = std::env::current_dir().unwrap().join("root");
        let inside = root.join("p").join("In.scala");
        assert_eq!(parts(&relative_path(&inside, &root)), ["p", "In.scala"]);
        // An interior `.` of a path outside the root is kept, as Java's path keeps it (the pass over 2f3b147c1).
        let dotted = Path::new("outside/./Other.scala");
        assert_eq!(uri_of(&parts(&relative_path(dotted, &root))), "outside/./Other.scala");
        assert_eq!(uri_of(&parts(&relative_path(&root.join(".").join("p").join("In.scala"), &root))), "p/In.scala");
        let outside = Path::new("outside/Other.scala");
        assert_eq!(uri_of(&parts(&relative_path(outside, &root))), "outside/Other.scala");
        assert_eq!(dump_path(Path::new("t"), outside, &root), Path::new("t/META-INF/semanticdb/outside/Other.scala.semanticdb"));
        assert_eq!(dump_path(Path::new("t"), &inside, &root), Path::new("t/META-INF/semanticdb/p/In.scala.semanticdb"));
    }

    #[test]
    fn md5_as_dotty_writes_it() {
        assert_eq!(md5_hex(""), "D41D8CD98F00B204E9800998ECF8427E");
    }
}
