use std::fmt::Write;
use std::path::{Path, PathBuf};

/// `std::fs::canonicalize`, its answer in the ordinary form on Windows: canonicalisation there
/// gives the extended-length form (`\\?\C:\work\A.scala`, `\\?\UNC\server\share\A.scala`), which no
/// editor, diagnostic or class path spells, and which std adds itself where a file operation needs
/// it (past 260 characters). Every identity teq takes from a canonical path goes through here, so
/// that a file has one whether canonicalised, named by a URI (`lsp::uri`) or gone.
pub fn canonicalize(path: impl AsRef<Path>) -> std::io::Result<PathBuf> {
    std::fs::canonicalize(path).map(|p| ordinary_form(p, cfg!(windows)))
}

/// A path in the ordinary form where it is a Windows path in the extended-length one (`windows`).
pub fn ordinary_form(path: PathBuf, windows: bool) -> PathBuf {
    if !windows {
        return path;
    }
    let text = path.to_string_lossy();
    let plain = ordinary(&text);
    if plain.len() == text.len() {
        return path;
    }
    PathBuf::from(plain.as_ref())
}

/// A Windows path in its ordinary form: `\\?\C:\x` as `C:\x`, `\\?\UNC\server\share\x` as
/// `\\server\share\x`; any other path as it is.
pub fn ordinary(path: &str) -> std::borrow::Cow<'_, str> {
    if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{}", unc).into();
    }
    match path.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') && rest.as_bytes()[0].is_ascii_alphabetic() => rest.into(),
        _ => path.into(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    #[inline]
    pub fn new(start: u32, end: u32) -> Span {
        Span { start, end }
    }
    #[inline]
    pub fn to(self, other: Span) -> Span {
        Span { start: self.start, end: other.end.max(self.end) }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct FileId(pub u32);

pub struct SourceFile {
    pub path: String,
    pub text: String,
    pub is_std: bool,
    /// What names made from the file's positions know it by, the same whichever files come
    /// before it: a program file's path under the input it was found in, a std file's path, a
    /// jar's file name, a library body's jar entry.
    pub key: String,
}

impl SourceFile {
    pub fn copy(&self) -> SourceFile {
        SourceFile { path: self.path.clone(), text: self.text.clone(), is_std: self.is_std, key: self.key.clone() }
    }
}

/// The FNV-1a hash of a file's key, which fresh names carry in the place of its `FileId`.
pub fn file_tag(key: &str) -> u32 {
    key.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193))
}

/// The tags of the files with the given keys, in the files' order: each its key's `file_tag`
/// where no other key hashes alike, and among keys that hash alike the one whose text sorts
/// first; every other key takes the first tag past its own that no key's hash claims and no
/// displaced key took before it (the displaced keys placed in the order of their texts). A
/// function of the set of keys alone: whatever order the files come in, and a file's tag is its
/// hash unless its own text is the one displaced.
pub fn file_tags<'a>(keys: impl Iterator<Item = &'a str>) -> Vec<u32> {
    let keys: Vec<&str> = keys.collect();
    let mut tags: Vec<u32> = keys.iter().map(|k| file_tag(k)).collect();
    let mut claimed: crate::intern::FxMap<u32, ()> = tags.iter().map(|&t| (t, ())).collect();
    let mut by_text: Vec<usize> = (0..keys.len()).collect();
    by_text.sort_by(|&a, &b| keys[a].cmp(keys[b]).then(a.cmp(&b)));
    let mut kept: crate::intern::FxMap<u32, ()> = crate::intern::FxMap::default();
    let mut displaced: Vec<usize> = Vec::new();
    for &i in &by_text {
        if kept.contains_key(&tags[i]) {
            displaced.push(i);
        } else {
            kept.insert(tags[i], ());
        }
    }
    for i in displaced {
        let mut tag = tags[i].wrapping_add(1);
        while claimed.contains_key(&tag) {
            tag = tag.wrapping_add(1);
        }
        claimed.insert(tag, ());
        tags[i] = tag;
    }
    tags
}

/// The tag of a library body's pseudo file: a 64-bit hash of its key (FNV-1a), so that two
/// bodies with one tag are out of practical reach among the ten thousand a build converts.
pub fn body_tag(key: &str) -> u64 {
    key.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

/// A tag in base 36, the short form a name takes it in.
pub fn tag_text(tag: u64) -> String {
    let mut digits = Vec::new();
    let mut n = tag;
    loop {
        digits.push(b"0123456789abcdefghijklmnopqrstuvwxyz"[(n % 36) as usize]);
        n /= 36;
        if n == 0 {
            break;
        }
    }
    digits.reverse();
    String::from_utf8(digits).unwrap()
}

/// The source files, by `FileId`, with the text of an edited file replaceable between two typing
/// steps of a watch session as `ast::Asts` allows for the ASTs.
pub struct Sources {
    slots: Vec<std::cell::UnsafeCell<SourceFile>>,
}

// Shared between the compiler thread and the typing thread as `ast::Asts` is: the two never
// touch it at once, and a text is replaced between two typing steps.
unsafe impl Sync for Sources {}

impl Sources {
    /// The bytes of the texts.
    pub fn held(&self) -> usize {
        self.as_slice().iter().map(|f| f.text.capacity() + f.path.capacity() + f.key.capacity()).sum()
    }

    pub fn new(files: Vec<SourceFile>) -> Sources {
        Sources { slots: files.into_iter().map(std::cell::UnsafeCell::new).collect() }
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn as_slice(&self) -> &[SourceFile] {
        unsafe { std::slice::from_raw_parts(self.slots.as_ptr() as *const SourceFile, self.slots.len()) }
    }

    pub fn set_text(&self, i: usize, text: String) {
        unsafe { (*self.slots[i].get()).text = text }
    }

    pub fn into_vec(self) -> Vec<SourceFile> {
        self.slots.into_iter().map(std::cell::UnsafeCell::into_inner).collect()
    }
}

impl std::ops::Index<usize> for Sources {
    type Output = SourceFile;
    #[inline]
    fn index(&self, i: usize) -> &SourceFile {
        unsafe { &*self.slots[i].get() }
    }
}

/// The file of a diagnostic that belongs to no file: one of the build's options.
pub const NO_FILE: FileId = FileId(u32::MAX);

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    pub file: FileId,
    pub span: Span,
    pub msg: String,
    pub is_warning: bool,
    /// Whether typing the bodies of the file again reports it again (`Diagnostics::of_bodies`).
    /// One of a signature, a parent, an import or an export is reported once, when that is
    /// resolved, and stays with the file while its bodies are edited.
    pub of_body: bool,
    /// Where a diagnostic of a library body's pseudo file stands in the library's source, which
    /// it is rendered at in the place of the pseudo file's line.
    pub place: Option<Box<Place>>,
    /// An error that depends on what malformed syntax left unknown: it stays in the record,
    /// so that what a typing decides on what it
    /// reported is the same, and is never presented, counted or answered (`presented`).
    pub dependent: bool,
    /// A message about a type that holds the error type: not presented while the program has a
    /// syntax error (`Diagnostics::syntax_errors`), as scalac hides a message about an erroneous
    /// type once an error was reported, the syntax errors coming first. The program's state is
    /// read when the diagnostics are presented, so that a file typed under another state
    /// presents what a fresh build would.
    pub unknown: bool,
    /// An error of what scalac checks in a phase after its typer (the dispatch method of an
    /// inline override, made by its `Inlining`): presented only while the program has no other
    /// error, as scalac runs no phase after a typer that reported one (`Phase.isRunnable`).
    pub late: bool,
    /// For a diagnostic of an inline expansion, moved to the call, where the inlined code it
    /// was reported at stands: an error there hides a later one whose place overlaps it, as
    /// scalac hides a message at a position a message already took (`UniqueMessagePositions`,
    /// which reads the inlined position), the earlier by its call's position.
    pub inlined_from: Option<(FileId, Span)>,
    /// A macro's failure, which scalac reports at the expansion (an exception, an unsupported
    /// operation): each call's own, whatever place of the inlined code it was found at.
    pub at_expansion: bool,
    /// A hint (`is_warning` too): what a language server's session reports outside the flag
    /// that would make it a warning (an unused import), which no count reads: not `--werror`,
    /// not `ok`, not the errors or the warnings of an answer.
    pub hint: bool,
    /// Code that does nothing (an unused import): the language server tags it `Unnecessary`,
    /// which an editor greys out.
    pub unnecessary: bool,
}

/// A place in a library's source: the source (`<artifact>!<path>`), the line and column where
/// they are known, the line's text where the source is attached.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Place {
    pub source: String,
    pub line: Option<u32>,
    pub col: Option<u32>,
    pub line_text: Option<String>,
}

impl Place {
    /// `<source>:<line>:<column>`, or the source alone where they are not known.
    pub fn render(&self) -> String {
        match (self.line, self.col) {
            (Some(l), Some(c)) => format!("{}:{}:{}", self.source, l, c),
            _ => self.source.clone(),
        }
    }

    /// `<source>:<line>`, or the source alone.
    pub fn line_of(&self) -> String {
        match self.line {
            Some(l) => format!("{}:{}", self.source, l),
            None => self.source.clone(),
        }
    }
}

impl Diagnostic {
    /// A lexical or syntax error of a session's file (`Diagnostics::syntax`).
    pub fn syntax(file: FileId, span: Span, msg: String) -> Diagnostic {
        Diagnostic { file, span, msg, is_warning: false, of_body: false, place: None, dependent: false, unknown: false, late: false, inlined_from: None, at_expansion: false, hint: false, unnecessary: false }
    }
}

#[derive(Default)]
pub struct Diagnostics {
    pub items: Vec<Diagnostic>,
    /// Whether what is reported now comes of typing bodies, which a watch session's retype of
    /// the file does again: set by the typer around what it is doing.
    pub of_bodies: bool,
    /// Whether the program has a lexical or syntax error (`Diagnostic::unknown`): set by the
    /// driver for what it presents.
    pub syntax_errors: bool,
    /// The lexical and parse diagnostics of the program's texts, which a session sets before it
    /// answers: presented with the typer's, in file and position order.
    pub syntax: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn error(&mut self, file: FileId, span: Span, msg: impl Into<String>) {
        self.items.push(Diagnostic { file, span, msg: msg.into(), is_warning: false, of_body: self.of_bodies, place: None, dependent: false, unknown: false, late: false, inlined_from: None, at_expansion: false, hint: false, unnecessary: false });
    }

    /// An error that depends on malformed syntax (`Diagnostic::dependent`).
    pub fn dependent_error(&mut self, file: FileId, span: Span, msg: impl Into<String>) {
        self.items.push(Diagnostic { file, span, msg: msg.into(), is_warning: false, of_body: self.of_bodies, place: None, dependent: true, unknown: false, late: false, inlined_from: None, at_expansion: false, hint: false, unnecessary: false });
    }

    pub fn warn(&mut self, file: FileId, span: Span, msg: impl Into<String>) {
        self.items.push(Diagnostic { file, span, msg: msg.into(), is_warning: true, of_body: self.of_bodies, place: None, dependent: false, unknown: false, late: false, inlined_from: None, at_expansion: false, hint: false, unnecessary: false });
    }

    pub fn dependent_warn(&mut self, file: FileId, span: Span, msg: impl Into<String>) {
        self.items.push(Diagnostic { file, span, msg: msg.into(), is_warning: true, of_body: self.of_bodies, place: None, dependent: true, unknown: false, late: false, inlined_from: None, at_expansion: false, hint: false, unnecessary: false });
    }

    /// A selector no name resolved through (`typer::unused`): a warning, or a hint.
    pub fn unnecessary(&mut self, file: FileId, span: Span, msg: impl Into<String>, hint: bool) {
        self.items.push(Diagnostic { file, span, msg: msg.into(), is_warning: true, of_body: true, place: None, dependent: false, unknown: false, late: false, inlined_from: None, at_expansion: false, hint, unnecessary: true });
    }

    /// An error of a library body's pseudo file placed in the library's source.
    pub fn error_placed(&mut self, file: FileId, span: Span, msg: impl Into<String>, place: Option<Place>) {
        self.error(file, span, msg);
        if let Some(d) = self.items.last_mut() {
            d.place = place.map(Box::new);
        }
    }

    /// The diagnostics a build presents: the syntax diagnostics a session set, then the typer's
    /// but the dependent ones and, while the program has a syntax error, the unknown ones. One
    /// that says what an earlier one says at the same place is presented once, as scalac's
    /// reporter does, whether one worker reported it twice or two workers once each (`absorb`).
    pub fn presented(&self) -> impl Iterator<Item = &Diagnostic> {
        let syntax = self.syntax_errors;
        let earlier = !self.syntax.is_empty() || self.items.iter().any(|d| !d.is_warning && !d.dependent && !d.late && !(d.unknown && syntax));
        let hidden = self.hidden_inlined();
        let mut seen = std::collections::HashSet::new();
        self.syntax
            .iter()
            .chain(self.items.iter().enumerate().filter(move |(i, d)| !d.dependent && !(d.unknown && syntax) && !(d.late && earlier) && !hidden.contains(i)).map(|(_, d)| d))
            .filter(move |d| seen.insert((d.file, d.span, d.msg.as_str(), d.is_warning, d.place.as_deref())))
    }

    /// The errors of inline expansions whose inlined place overlaps that of an earlier one
    /// (`Diagnostic::inlined_from`), by index.
    fn hidden_inlined(&self) -> Vec<usize> {
        let mut inlined: Vec<(FileId, u32, usize, FileId, Span)> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, d)| !d.is_warning && !d.dependent)
            .filter_map(|(i, d)| d.inlined_from.map(|(f, s)| (d.file, d.span.start, i, f, s)))
            .collect();
        if inlined.len() < 2 {
            return Vec::new();
        }
        inlined.sort_by_key(|&(f, at, i, _, _)| (f.0, at, i));
        let mut kept: Vec<(FileId, Span)> = Vec::new();
        let mut hidden = Vec::new();
        for (_, _, i, f, s) in inlined {
            if kept.iter().any(|&(kf, ks)| kf == f && ks.start <= s.end && s.start <= ks.end) {
                hidden.push(i);
            } else {
                kept.push((f, s));
            }
        }
        hidden
    }

    /// A late error (`Diagnostic::late`).
    pub fn late_error(&mut self, file: FileId, span: Span, msg: impl Into<String>) {
        self.error(file, span, msg);
        if let Some(d) = self.items.last_mut() {
            d.late = true;
        }
    }

    /// Whether a build with these diagnostics fails: an error of the typer's or a syntax error,
    /// or under `--werror` any diagnostic.
    pub fn fails(&self, werror: bool) -> bool {
        self.has_errors() || !self.syntax.is_empty() || (werror && self.items.iter().any(|d| !d.hint))
    }

    /// The warnings `--werror` counts: every diagnostic but the hints.
    pub fn warning_count(&self) -> usize {
        self.items.iter().filter(|d| d.is_warning && !d.hint).count()
    }

    pub fn presented_error_count(&self) -> usize {
        self.presented().filter(|d| !d.is_warning).count()
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| !d.is_warning)
    }

    /// Adds another worker's diagnostics, leaving out what is reported already: two workers
    /// can report one problem, each from the body that met it.
    pub fn absorb(&mut self, other: Diagnostics) {
        let seen: std::collections::HashSet<&Diagnostic> = self.items.iter().collect();
        let fresh: Vec<Diagnostic> = other.items.into_iter().filter(|d| !seen.contains(d)).collect();
        self.items.extend(fresh);
    }

    pub fn error_count(&self) -> usize {
        self.items.iter().filter(|d| !d.is_warning).count()
    }

    pub fn render(&self, files: &[SourceFile]) -> String {
        self.render_ranked(files, &[])
    }

    /// The diagnostics in the order of `ranks` over the files (`Symbols::file_ranks`), a file
    /// without a rank by its id.
    pub fn render_ranked(&self, files: &[SourceFile], ranks: &[u32]) -> String {
        let mut out = String::new();
        let mut items: Vec<&Diagnostic> = self.presented().collect();
        items.sort_by_key(|d| (ranks.get(d.file.0 as usize).copied().unwrap_or(d.file.0), d.span.start));
        for d in items {
            let severity = if d.is_warning { "warning" } else { "error" };
            if d.file == NO_FILE {
                let _ = writeln!(out, "{}: {}", severity, d.msg);
                continue;
            }
            if let Some(p) = &d.place {
                let _ = writeln!(out, "{}: {}: {}", p.render(), severity, d.msg);
                if let (Some(text), Some(col)) = (&p.line_text, p.col) {
                    let _ = writeln!(out, "  {}", text);
                    let _ = writeln!(out, "  {}^", " ".repeat((col as usize).saturating_sub(1)));
                }
                continue;
            }
            let Some(f) = files.get(d.file.0 as usize) else {
                let _ = writeln!(out, "<library>: {}: {}", severity, d.msg);
                continue;
            };
            let (line, col, line_text) = locate(&f.text, d.span.start as usize);
            let _ = writeln!(out, "{}:{}:{}: {}: {}", f.path, line, col, severity, d.msg);
            let _ = writeln!(out, "  {}", line_text);
            let width = (d.span.end.saturating_sub(d.span.start) as usize)
                .clamp(1, line_text.len().saturating_sub(col - 1).max(1));
            let _ = writeln!(out, "  {}{}", " ".repeat(col - 1), "^".repeat(width));
        }
        out
    }
}

/// The 1-based line and column of `offset` and the text of its line.
pub fn locate(text: &str, offset: usize) -> (usize, usize, &str) {
    let offset = offset.min(text.len());
    let before = &text.as_bytes()[..offset];
    let line = before.iter().filter(|&&b| b == b'\n').count() + 1;
    let line_start = before.iter().rposition(|&b| b == b'\n').map_or(0, |p| p + 1);
    let line_end = text[line_start..].find('\n').map_or(text.len(), |p| line_start + p);
    (line, offset - line_start + 1, &text[line_start..line_end])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Three keys, two of which hash alike and the third with the next tag: the displaced key
    /// passes over the third's tag, which stays the third's.
    #[test]
    fn a_displaced_tag_passes_over_every_natural_tag() {
        let a = "Use29115316.scala";
        let b = "Use56507032.scala";
        let c = "ZZZ87kqi0.scala";
        assert_eq!(file_tag(a), file_tag(b));
        assert_eq!(file_tag(c), file_tag(a) + 1);
        let tags = file_tags([a, b, c].into_iter());
        assert_eq!(tags, vec![file_tag(a), file_tag(a) + 2, file_tag(c)]);
        let again = file_tags([c, b, a].into_iter());
        assert_eq!(again, vec![file_tag(c), file_tag(a) + 2, file_tag(a)]);
        assert_eq!(file_tags([c].into_iter()), vec![file_tag(c)]);
        assert_eq!(file_tags([a, b].into_iter()), vec![file_tag(a), file_tag(a) + 1]);
    }
}
