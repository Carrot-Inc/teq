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
    /// A warning's identity for the reporting policy (`warnings.rs`): scalac's message id
    /// (`warnings::NO_ID` for none), its category, and its origin (the deprecated definition, the
    /// unused selector), which `-Wconf` and `@nowarn` filter by.
    pub id: u16,
    pub category: crate::warnings::Category,
    pub origin: Option<Box<str>>,
    /// The change of the source the diagnostic proposes (an unused import's removal), which a
    /// language server offers as a quick fix.
    pub action: Option<Box<Action>>,
    /// The phase of scalac's that reports it where its kind does not say (`producer_rank`): an
    /// inline expansion's late error is `Inlining`'s, an impossible type test's `Erasure`'s.
    pub phase: u8,
    /// scalac reports it without a position (a primitive cast no conversion makes): teq presents
    /// it where it stands, but no place hides it, nor does it take one (`isHidden`'s
    /// `pos.exists`).
    pub unplaced: bool,
}

/// A change of the source a diagnostic proposes, dotty's `CodeAction`: its title, what it does,
/// and its patches, each a span of the diagnostic's file and the text that replaces it.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Action {
    pub title: &'static str,
    pub description: &'static str,
    pub patches: Vec<(Span, String)>,
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
    /// A diagnostic of nothing but its place, message and severity.
    pub fn plain(file: FileId, span: Span, msg: String, is_warning: bool, of_body: bool) -> Diagnostic {
        Diagnostic { file, span, msg, is_warning, of_body, place: None, dependent: false, unknown: false, late: false, inlined_from: None, at_expansion: false, hint: false, unnecessary: false, id: crate::warnings::NO_ID, category: crate::warnings::Category::Plain, origin: None, action: None, phase: 0, unplaced: false }
    }

    /// A lexical or syntax error of a session's file (`Diagnostics::syntax`).
    pub fn syntax(file: FileId, span: Span, msg: String) -> Diagnostic {
        Diagnostic::plain(file, span, msg, false, false)
    }

    /// What the reporting policy's filters read of the diagnostic.
    fn subject<'a>(&'a self, path: Option<&'a str>) -> crate::warnings::Subject<'a> {
        crate::warnings::Subject { msg: &self.msg, id: self.id, category: self.category, origin: self.origin.as_deref(), path }
    }
}

/// A warning's identity (`Diagnostic::id`), the phase of scalac's that reports it where its
/// kind does not say (`Diagnostic::phase`), and whether scalac gives it no position
/// (`Diagnostic::unplaced`).
#[derive(Clone, Default)]
pub struct Warning {
    pub id: u16,
    pub category: crate::warnings::Category,
    pub origin: Option<Box<str>>,
    pub phase: u8,
    pub unplaced: bool,
}

impl Warning {
    pub fn id(id: u16) -> Warning {
        Warning { id, ..Warning::plain() }
    }

    pub fn plain() -> Warning {
        Warning { id: crate::warnings::NO_ID, category: crate::warnings::Category::Plain, origin: None, phase: 0, unplaced: false }
    }

    /// A plain warning of scalac's phase `phase` (`producer_rank`).
    pub fn of_phase(phase: u8) -> Warning {
        Warning { phase, ..Warning::plain() }
    }

    /// An unchecked type test (`UncheckedTypePattern`, an `UncheckedWarning`).
    pub fn unchecked() -> Warning {
        Warning { id: crate::warnings::id::UNCHECKED_TYPE_PATTERN, category: crate::warnings::Category::Unchecked, origin: None, phase: 0, unplaced: false }
    }
}

/// How a diagnostic is presented once the policy is applied.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Error,
    Warning,
    /// `-Wconf`'s `info`: shown, counted by nothing.
    Info,
    /// A language server's hint (`Diagnostic::hint`), counted by nothing.
    Hint,
}

impl Level {
    pub fn name(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warning => "warning",
            Level::Info => "info",
            Level::Hint => "hint",
        }
    }
}

/// A diagnostic as presented: itself or its copy with the policy's help added, its level, and
/// where it stands among the others (0 before the rest, 1 by its place, 2 after the rest).
pub struct Shown<'a> {
    pub d: std::borrow::Cow<'a, Diagnostic>,
    pub level: Level,
    pub order: u8,
}

/// The places the presented diagnostics took, scalac's `UniqueMessagePositions`: per file the
/// spans, each with the level of the diagnostic that took it. A warning is hidden where one of
/// a level at least its own took an offset of its span (`isHidden`, the span's both ends
/// included); one without a place is hidden only by the same one (`pos.exists`).
#[derive(Default)]
struct Occupied {
    taken: std::collections::HashMap<FileId, Vec<(u32, u32, u8)>>,
    placeless: std::collections::HashSet<(String, bool)>,
    errors: std::collections::HashSet<(FileId, Span, String, Option<Box<Place>>)>,
}

impl Occupied {
    /// scalac's levels: an info, a warning (a hint is one), an error.
    fn weight(level: Level) -> u8 {
        match level {
            Level::Info => 0,
            Level::Warning | Level::Hint => 1,
            Level::Error => 2,
        }
    }

    /// Whether `d` is presented at `level`, which takes its place where it is. An error is
    /// hidden by the same error alone: teq's errors do not stand where scalac's do (an object
    /// creation impossible spans the class, where scalac's stands at a point; an application
    /// teq rejects beside its argument scalac does not), so that one would hide another scalac
    /// shows; they take their places for the warnings.
    fn take(&mut self, d: &Diagnostic, level: Level) -> bool {
        if d.unplaced {
            return true;
        }
        if d.file == NO_FILE {
            return self.placeless.insert((d.msg.clone(), d.is_warning));
        }
        let w = Self::weight(level);
        let (start, end) = (d.span.start, d.span.end.max(d.span.start));
        if level == Level::Error && !self.errors.insert((d.file, d.span, d.msg.clone(), d.place.clone())) {
            return false;
        }
        let spans = self.taken.entry(d.file).or_default();
        if level != Level::Error && spans.iter().any(|&(s, e, l)| l >= w && s <= end && start <= e) {
            return false;
        }
        spans.push((start, end, w));
        true
    }

    /// `d`, presented by an earlier report, takes its place.
    fn mark(&mut self, d: &Diagnostic, level: Level) {
        self.take(d, level);
    }
}

/// The phase of scalac's that reports the diagnostic, in their order (`Compiler.phases`): which
/// of two at one place reports first. The typer's errors and warnings (`Typer`, `Checking`'s
/// feature warnings, `@nowarn`'s filters), an inline expansion's late error (`Inlining`), a
/// deprecation (`CrossVersionChecks`), a match's exhaustivity and reachability
/// (`PatternMatcher`), an unused definition or import (`CheckUnused.PostPatMat`), an
/// interpolation's lint (`StringInterpolatorOpt`), an unchecked type test (`Erasure`).
fn producer_rank(d: &Diagnostic) -> u8 {
    use crate::warnings::{id, Category};
    if d.phase != 0 {
        return d.phase;
    }
    if !d.is_warning {
        return PHASE_TYPER;
    }
    if d.category == Category::Deprecation {
        return 5;
    }
    match d.id {
        id::PATTERN_MATCH_EXHAUSTIVITY | id::MATCH_CASE_UNREACHABLE => 7,
        id::UNUSED_SYMBOL => 8,
        id::FORMAT_INTERPOLATION_ERROR => 9,
        id::UNCHECKED_TYPE_PATTERN => PHASE_ERASURE,
        _ => PHASE_TYPER,
    }
}

/// The phases `producer_rank` names.
pub const PHASE_TYPER: u8 = 1;
pub const PHASE_INLINING: u8 = 4;
pub const PHASE_ERASURE: u8 = 10;

/// What a build presents (`Diagnostics::report`): the diagnostics, and how many errors and
/// warnings are counted.
pub struct Report<'a> {
    pub shown: Vec<Shown<'a>>,
    pub errors: usize,
    pub warnings: usize,
}

impl Report<'_> {
    /// Whether the build fails: an error, or under `--werror` a warning.
    pub fn fails(&self, werror: bool) -> bool {
        self.errors > 0 || (werror && self.warnings > 0)
    }
}

#[derive(Default)]
pub struct Diagnostics {
    pub items: Vec<Diagnostic>,
    /// What the build asks of its warnings (`warnings::Policy`), set by the driver.
    pub policy: std::sync::Arc<crate::warnings::Policy>,
    /// The `@nowarn` annotations of the program's files (`warnings::Suppression`), and the
    /// warnings their filters gave, read of the files' trees once their typing is over
    /// (`Worker::register_suppressions`).
    pub suppressions: Vec<crate::warnings::Suppression>,
    pub suppression_warnings: Vec<Diagnostic>,
    /// How many of `items` a report printed (`Worker::report_diags`), whose later reports show
    /// the rest alone; `None` before the first.
    pub printed: Option<usize>,
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
        self.items.push(Diagnostic::plain(file, span, msg.into(), false, self.of_bodies));
    }

    /// An error that depends on malformed syntax (`Diagnostic::dependent`).
    pub fn dependent_error(&mut self, file: FileId, span: Span, msg: impl Into<String>) {
        let mut d = Diagnostic::plain(file, span, msg.into(), false, self.of_bodies);
        d.dependent = true;
        self.items.push(d);
    }

    pub fn warn(&mut self, file: FileId, span: Span, msg: impl Into<String>) {
        self.items.push(Diagnostic::plain(file, span, msg.into(), true, self.of_bodies));
    }

    /// A warning with its identity (`Diagnostic::id`), depending on malformed syntax where
    /// `dependent` says.
    pub fn warn_as(&mut self, file: FileId, span: Span, msg: impl Into<String>, w: Warning, dependent: bool) {
        let mut d = Diagnostic::plain(file, span, msg.into(), true, self.of_bodies);
        d.dependent = dependent;
        d.id = w.id;
        d.category = w.category;
        d.origin = w.origin;
        d.phase = w.phase;
        d.unplaced = w.unplaced;
        self.items.push(d);
    }

    /// A selector no name resolved through (`typer::unused`): a warning, or a hint.
    pub fn unnecessary(&mut self, file: FileId, span: Span, msg: impl Into<String>, hint: bool, origin: Option<Box<str>>) {
        let mut d = Diagnostic::plain(file, span, msg.into(), true, true);
        d.hint = hint;
        d.unnecessary = true;
        d.id = crate::warnings::id::UNUSED_SYMBOL;
        d.origin = origin;
        self.items.push(d);
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
    ///
    /// Of the typer's diagnostics from the index `from` on, those before left out, and the
    /// syntax diagnostics where the report is not `whole`.
    fn presented_from(&self, from: usize, whole: bool) -> impl Iterator<Item = &Diagnostic> {
        let syntax = self.syntax_errors;
        let earlier = !self.syntax.is_empty() || self.items.iter().any(|d| !d.is_warning && !d.dependent && !d.late && !(d.unknown && syntax));
        let hidden = self.hidden_inlined();
        self.syntax
            .iter()
            .filter(move |_| whole)
            .chain(self.items.iter().enumerate().skip(from).filter(move |(i, d)| !d.dependent && !(d.unknown && syntax) && !(d.late && earlier) && !hidden.contains(i)).map(|(_, d)| d))
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

    /// A late error (`Diagnostic::late`), of scalac's `Inlining`.
    pub fn late_error(&mut self, file: FileId, span: Span, msg: impl Into<String>) {
        self.late_error_of(file, span, msg, PHASE_INLINING);
    }

    /// A late error of the phase `phase` (`producer_rank`).
    pub fn late_error_of(&mut self, file: FileId, span: Span, msg: impl Into<String>, phase: u8) {
        self.error(file, span, msg);
        if let Some(d) = self.items.last_mut() {
            d.late = true;
            d.phase = phase;
        }
    }

    /// Whether a build with these diagnostics fails, as the policy presents them
    /// (`report`): an error, or under `--werror` a warning.
    pub fn fails(&self, files: &[SourceFile], werror: bool) -> bool {
        !self.syntax.is_empty() || self.has_errors() || self.report(files).fails(werror)
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

    /// The diagnostics as the reporting policy presents them (`warnings.rs`), every
    /// presentation's: those `presented` keeps, in file and position order, each warning filtered
    /// by the `@nowarn` annotations around it and then by `-Wconf`'s rules, a conditional one
    /// whose flag is off counted into its summary (`Reporter.issueIfNotSuppressed`); then under
    /// `--wunused nowarn` the annotations nothing used, while nothing is an error
    /// (`suppressions.runFinished`); then the summaries (`summarizeUnreportedWarnings`). A
    /// configuration of `-Wconf` that does not parse is reported before the first warning that
    /// consults it. `files` gives the sources' paths, which `src=` filters read.
    pub fn report<'a>(&'a self, files: &[SourceFile]) -> Report<'a> {
        self.report_from(files, None)
    }

    /// `report`, or with `after` the report of the typer's diagnostics from that index on:
    /// what a build that presented those before reports later (the errors the reach of a
    /// JavaScript build or an interpreted run met), each once, without the report's end.
    pub fn report_from<'a>(&'a self, files: &[SourceFile], after: Option<usize>) -> Report<'a> {
        use crate::warnings::{Action, Category};
        use std::borrow::Cow;
        let policy = &*self.policy;
        let whole = after.is_none();
        let from = after.unwrap_or(0);
        let extras = if whole { &self.suppression_warnings[..] } else { &[] };
        let candidates: Vec<&Diagnostic> = self.presented_from(from, whole).chain(extras.iter()).collect();
        // Presented in file and position order, the order they were made in at one place.
        let mut by_place: Vec<usize> = (0..candidates.len()).collect();
        by_place.sort_by_key(|&i| (candidates[i].file, candidates[i].span.start, i));
        let mut place_rank = vec![0usize; candidates.len()];
        for (r, &i) in by_place.iter().enumerate() {
            place_rank[i] = r;
        }
        // Weighed in the order scalac's phases report them, the order made within one: of two at
        // one place, the first reported takes it (`UniqueMessagePositions`).
        let mut by_producer: Vec<usize> = (0..candidates.len()).collect();
        by_producer.sort_by_key(|&i| (producer_rank(candidates[i]), i));
        let mut occupied = Occupied::default();
        for d in &self.items[..from.min(self.items.len())] {
            occupied.mark(d, if d.is_warning { Level::Warning } else { Level::Error });
        }
        // The first feature warning of each use site explains the feature, whatever becomes of
        // it (`report.featureWarning`).
        let mut sites: Vec<&str> = Vec::new();
        let mut explains = vec![false; candidates.len()];
        for &i in &by_place {
            let d = candidates[i];
            explains[i] = d.is_warning && d.category == Category::Feature && d.origin.as_deref().is_some_and(|site| !sites.contains(&site) && { sites.push(site); true });
        }
        let mut used = vec![0u8; self.suppressions.len()];
        let mut unreported: Vec<(Category, usize)> = Vec::new();
        let mut consulted = false;
        let mut shown_at: Vec<(usize, Shown)> = Vec::new();
        let paths: Vec<Option<String>> = if policy.wconf.is_some() || !self.suppressions.is_empty() { files.iter().map(|f| absolute_path(&f.path)).collect() } else { Vec::new() };
        let path_of = |f: FileId| paths.get(f.0 as usize).and_then(|p| p.as_deref());
        for &i in &by_producer {
            let d = candidates[i];
            if !d.is_warning {
                if occupied.take(d, Level::Error) {
                    shown_at.push((i, Shown { d: Cow::Borrowed(d), level: Level::Error, order: 1 }));
                }
                continue;
            }
            let subject = d.subject(path_of(d.file));
            // `@nowarn`: the first annotation around the warning whose filters match it.
            let mut verbose = false;
            let mut matching = self.suppressions.iter().enumerate().filter(|(_, s)| (s.covers(d.file, d.span) || d.inlined_from.is_some_and(|(f, at)| s.covers(f, at))) && s.matches(&subject));
            if let Some((first_match, sup)) = matching.next() {
                if !d.hint {
                    for (other, _) in matching {
                        if used[other] != 1 {
                            used[other] = 2;
                        }
                    }
                    used[first_match] = 1;
                }
                if !sup.verbose {
                    continue;
                }
                verbose = true;
            }
            if d.hint {
                if occupied.take(d, Level::Hint) {
                    shown_at.push((i, Shown { d: Cow::Borrowed(d), level: Level::Hint, order: 1 }));
                }
                continue;
            }
            consulted = true;
            let level = match policy.action(&subject) {
                Action::Silent => continue,
                Action::Error => Level::Error,
                Action::Info => Level::Info,
                Action::Verbose => {
                    verbose = true;
                    Level::Warning
                }
                Action::Warning => Level::Warning,
                // A warning summarized is counted whatever place it would take, as scalac counts
                // it before it hides any (`issueUnconfigured`).
                Action::Default if policy.summarizes(d.category) => {
                    match unreported.iter_mut().find(|(c, _)| *c == d.category) {
                        Some((_, n)) => *n += 1,
                        None => unreported.push((d.category, 1)),
                    }
                    continue;
                }
                Action::Default => Level::Warning,
            };
            if occupied.take(d, level) {
                let shown_d = if verbose || explains[i] {
                    let mut copy = d.clone();
                    if explains[i] {
                        copy.msg.push_str(&crate::typer::feature::explanation("implicitConversions"));
                    }
                    if verbose {
                        copy.msg.push_str(&crate::warnings::filter_help(d.id, d.category, d.origin.as_deref()));
                    }
                    Cow::Owned(copy)
                } else {
                    Cow::Borrowed(d)
                };
                shown_at.push((i, Shown { d: shown_d, level, order: 1 }));
            }
        }
        shown_at.sort_by_key(|&(i, _)| place_rank[i]);
        let mut shown: Vec<Shown> = shown_at.into_iter().map(|(_, s)| s).collect();
        // The warnings of the report's end go through `-Wconf` alone.
        let mut late: Vec<Diagnostic> = Vec::new();
        let errors = shown.iter().filter(|s| s.level == Level::Error).count();
        if whole && policy.unused.nowarn && errors == 0 && self.syntax.is_empty() {
            for (i, sup) in self.suppressions.iter().enumerate() {
                let duplicate = self.suppressions.iter().enumerate().any(|(j, other)| j != i && used[j] == 1 && other.file == sup.file && other.annot == sup.annot);
                if used[i] != 1 && !duplicate && !sup.invalid() {
                    let more = if used[i] == 2 { " but matches a diagnostic" } else { "" };
                    late.push(Diagnostic::plain(sup.file, sup.annot, format!("@nowarn annotation does not suppress any warnings{}", more), true, true));
                }
            }
        }
        for (category, n) in unreported.into_iter().filter(|_| whole) {
            if let Some(msg) = crate::warnings::summary(category, n) {
                late.push(Diagnostic::plain(NO_FILE, Span::default(), msg, true, true));
            }
        }
        for d in late {
            consulted = true;
            let level = match policy.action(&d.subject(path_of(d.file))) {
                Action::Silent => continue,
                Action::Error => Level::Error,
                Action::Info => Level::Info,
                _ => Level::Warning,
            };
            let order = if d.file == NO_FILE { 2 } else { 1 };
            shown.push(Shown { d: Cow::Owned(d), level, order });
        }
        if let (true, Some(msg)) = (consulted, policy.wconf_failure()) {
            let mut d = Diagnostic::plain(NO_FILE, Span::default(), msg, true, true);
            d.category = Category::Configuration;
            shown.insert(0, Shown { d: Cow::Owned(d), level: Level::Warning, order: 0 });
        }
        let errors = shown.iter().filter(|s| s.level == Level::Error).count();
        let warnings = shown.iter().filter(|s| s.level == Level::Warning).count();
        Report { shown, errors, warnings }
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
        render_shown(&self.report(files).shown, files, ranks)
    }

}

/// The shown diagnostics in the order of `ranks` over the files, a file without a rank by its id.
pub fn render_shown(shown: &[Shown], files: &[SourceFile], ranks: &[u32]) -> String {
    let mut out = String::new();
    let mut items: Vec<&Shown> = shown.iter().collect();
    items.sort_by_key(|s| (s.order, ranks.get(s.d.file.0 as usize).copied().unwrap_or(s.d.file.0), s.d.span.start));
    for s in items {
        let d = &*s.d;
        let severity = s.level.name();
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

/// The absolute form of a source's path, which `src=` filters read (scalac's
/// `toAbsolutePath.toUri.normalize().getRawPath`).
fn absolute_path(path: &str) -> Option<String> {
    let p = std::path::absolute(path).ok()?;
    let mut out = String::new();
    for c in p.components() {
        match c {
            std::path::Component::RootDir => {}
            std::path::Component::CurDir => continue,
            std::path::Component::ParentDir => {
                if let Some(i) = out.rfind('/') {
                    out.truncate(i);
                }
                continue;
            }
            std::path::Component::Prefix(_) => {}
            std::path::Component::Normal(n) => {
                out.push('/');
                out.push_str(&n.to_string_lossy());
            }
        }
    }
    if out.is_empty() {
        out.push('/');
    }
    Some(out)
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
