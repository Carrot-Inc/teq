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

/// The completion whose typing reported a diagnostic (`Diagnostic::owner`): by its file and
/// where it stands there, a member's name or a top-level definition's start, so that it is the
/// same on every worker. A member's signature and its body are two completions at one place, the
/// signature's marked (`DiagOwner::SIG`), as dotty's completer types a declared result apart from
/// the body `typedDefDef` types. Equality and hashing ignore it: the reporter hides a diagnostic
/// equal to an earlier one at its place whichever completion reported each.
#[derive(Clone, Copy, Debug)]
pub struct DiagOwner {
    pub file: u32,
    pub place: u32,
}

impl DiagOwner {
    /// No definition's typing: a pass over the whole program, the driver's.
    pub const NONE: DiagOwner = DiagOwner { file: u32::MAX, place: 0 };

    /// The mark of a signature's completion on its definition's place.
    pub const SIG: u32 = 1 << 31;

    pub fn new(file: FileId, place: u32) -> DiagOwner {
        DiagOwner { file: file.0, place }
    }

    /// The completion of the signature of the definition at `place` (`Namer.Completer.complete`).
    pub fn sig(file: FileId, place: u32) -> DiagOwner {
        DiagOwner { file: file.0, place: place | DiagOwner::SIG }
    }

    /// Where the owner stands in its file, a signature's at its definition's place.
    pub fn at(self) -> u32 {
        self.place & !DiagOwner::SIG
    }

    pub fn is_none(self) -> bool {
        self.file == u32::MAX
    }

    /// The owner as a key of the typer's tables, which do compare it.
    pub fn key(self) -> (u32, u32) {
        (self.file, self.place)
    }
}

impl Default for DiagOwner {
    fn default() -> DiagOwner {
        DiagOwner::NONE
    }
}

impl PartialEq for DiagOwner {
    fn eq(&self, _: &DiagOwner) -> bool {
        true
    }
}

impl Eq for DiagOwner {}

impl std::hash::Hash for DiagOwner {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

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
    /// The definition whose typing reported it, the innermost one in progress on the worker
    /// that did (`Diagnostics::owner`): what a unit's suspension at typing keeps of it is decided
    /// by its owner (`Worker::take_typing_suspended`).
    pub owner: DiagOwner,
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
    /// Made by the inlining phase (`Diagnostics::in_phase`), an error or a warning, or an error
    /// the typing of a unit the phase suspended reported in a later run
    /// (`Diagnostics::later_typing`): scalac's `Inlining` runs only where its typer reported no
    /// error, and a session keeps the record with its file through later checks, which may
    /// report one. Presented only while the records hold no error of the first run's typer and
    /// the program no syntax error (`Diagnostics::typer_failed`).
    pub by_phase: bool,
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
    /// The name the report says is missing (a name not found, the member a selection lacks),
    /// which a language server reads to offer the imports that bring it
    /// (`typer::complete::import_fix`).
    pub missing: MissingName,
    /// The phase of scalac's that reports it where its kind does not say (`producer_rank`): an
    /// inline expansion's late error is `Inlining`'s, an impossible type test's `Erasure`'s.
    pub phase: u8,
    /// scalac reports it without a position (a primitive cast no conversion makes): teq presents
    /// it where it stands, but no place hides it, nor does it take one (`isHidden`'s
    /// `pos.exists`).
    pub unplaced: bool,
    /// The occurrence's identity (`Ordinal`), which equality and hashing ignore.
    pub ordinal: Ordinal,
    /// Where a fresh check of the reference makes the record (`Position`), which equality and
    /// hashing ignore.
    pub position: Position,
    /// Made by the code a unit's suspension leaves (`Unwound`), which equality and hashing ignore.
    pub unwound: Unwound,
}

/// The phase group of the reference's that makes a record, in the order of a run's phases (dotty
/// 3.8.4's `Compiler.phases`, the phases it fuses into one group being one here): a group runs
/// where no group before it reported an error (`Phase.isRunnable`), the typer whatever the parser
/// reported.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Group {
    /// No producer named the record's group yet: the dump says so.
    #[default]
    Unstamped,
    /// teq's own: the reference makes no such record.
    Own,
    Parser,
    Typer,
    PostTyper,
    PrepJSInterop,
    Inlining,
    /// `CrossVersionChecks`, `InlineVals`, `ExpandSAMs`, `RefChecks`, `DropForMap` and the phases
    /// fused with them.
    RefChecks,
    /// `ForwardDepChecks`, `PatternMatcher` and the phases fused with them.
    PatternMatcher,
    /// `CheckUnused.PostPatMat`.
    CheckUnused,
    /// `StringInterpolatorOpt` and the phases fused with `ElimOpaque`.
    ElimOpaque,
    /// `ResolveSuper`, `TupleOptimizations` and the phases fused with `PruneErasedDefs`.
    PruneErasedDefs,
    Erasure,
    /// `ElimErasedValueType`, `TailRec`, `Mixin`, `NonLocalReturns` and the phases fused with them.
    TailRec,
    Backend,
    /// The run's final report (`Run.runFinished`, `Reporter.finalizeReporting`).
    RunEnd,
}

/// What of the expansion under way a record's group turned on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Context {
    #[default]
    None,
    /// Inside an expansion the typer makes (a transparent method's, a forced one).
    Typing,
    /// Inside an expansion the inlining phase makes.
    Phase,
}

/// Where a fresh check of the reference makes a record: the run of its unit (1 for the first, 0
/// while no producer said), the phase group that reports it, what of the expansion under way the
/// group turned on, and the place in teq's source that made the record. Given where the record is
/// made (`Diagnostics::push`); two facts the producer cannot know then are given where they are
/// known: a record an expansion ahead of the inlining phase held is the phase's
/// (`ahead_of_phase`), and a record of a unit that leaves its run, or would where the phase is
/// not run, is of the unit's next run where its group is after the one the unit left in
/// (`Diagnostics::suspend_unit`). Nothing reads it
/// but the dump (`TEQ_POSITIONS=1`, `Diagnostics::trace_positions`): equality and hashing ignore
/// it, as `Ordinal`'s.
#[derive(Clone, Copy, Debug, Default)]
pub struct Position {
    pub run: u8,
    pub group: Group,
    /// Whether the record's producer named the group (`Diagnostics::error_by`, `Warning::by`, a
    /// late error's phase); the typer's reporters name none (`Diagnostics::push`).
    pub named: bool,
    pub context: Context,
    pub site: Option<&'static std::panic::Location<'static>>,
}

impl Position {
    /// The position of a record the group `group` makes, named by its producer.
    pub fn of(group: Group) -> Position {
        Position { group, named: true, ..Position::default() }
    }

    /// The record is one an expansion made ahead of the inlining phase, for it (a plain inline
    /// method's call expanded where its unit's typing flushes it): the phase's record, in the
    /// phase's expansion, unless its producer named another group.
    pub fn ahead_of_phase(&mut self) {
        if !self.named && self.group == Group::Typer {
            self.group = Group::Inlining;
        }
        self.context = Context::Phase;
    }
}

impl Group {
    /// The group of the phase a producer names by its rank (`producer_rank`'s constants).
    pub fn of_phase(phase: u8) -> Group {
        match phase {
            PHASE_TYPER => Group::Typer,
            PHASE_INLINING => Group::Inlining,
            PHASE_FIRST_TRANSFORM => Group::RefChecks,
            PHASE_FORWARD_DEP_CHECKS | 7 => Group::PatternMatcher,
            8 => Group::CheckUnused,
            9 => Group::ElimOpaque,
            PHASE_TUPLE_OPTIMIZATIONS => Group::PruneErasedDefs,
            PHASE_ERASURE => Group::Erasure,
            PHASE_TAILREC | PHASE_NONLOCAL_RETURNS => Group::TailRec,
            _ => Group::Unstamped,
        }
    }
}

impl PartialEq for Position {
    fn eq(&self, _: &Position) -> bool {
        true
    }
}

impl Eq for Position {}

impl std::hash::Hash for Position {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

/// A diagnostic's identity in its build: a number no other record of the build has, given where it
/// is made (`Diagnostics::push`) and kept where it goes (a unit's suspension reports, an expansion's
/// held reports, another worker's merge), so that what a cut or a round's suspension had reported
/// is told from an equal record made after (`TypingCut::reported`); 0 for a record not made yet.
/// Equality and hashing ignore it, as `DiagOwner`'s do.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ordinal(pub u64);

impl PartialEq for Ordinal {
    fn eq(&self, _: &Ordinal) -> bool {
        true
    }
}

impl Eq for Ordinal {}

impl std::hash::Hash for Ordinal {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

/// Whether a record was made by the code a unit's suspension at typing leaves
/// (`Diagnostics::unwinding`): the rest of the item whose typing unwinds after its unit's cut,
/// which dotty's `SuspendException` never types and whose records the cut drops
/// (`keep_typing_reports`), so that what the history reads (`typer::merge::HistoryQuery`, dotty's
/// `errorsReported`) never counts it. Equality and hashing ignore it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Unwound(pub bool);

impl PartialEq for Unwound {
    fn eq(&self, _: &Unwound) -> bool {
        true
    }
}

impl Eq for Unwound {}

impl std::hash::Hash for Unwound {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

/// A change of the source a diagnostic proposes, dotty's `CodeAction`: its title, what it does,
/// and its patches, each a span of the diagnostic's file and the text that replaces it.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Action {
    pub title: &'static str,
    pub description: &'static str,
    pub patches: Vec<(Span, String)>,
}

/// What a report says is missing, for the quick fix that imports it: the name as the source
/// spells it and what was wanted of it.
#[derive(Clone, Debug)]
pub struct Missing {
    pub name: Box<str>,
    pub kind: MissingKind,
}

/// A name not found where a term or where a type is wanted, or the member a selection `e.N`
/// lacks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MissingKind {
    Term,
    Type,
    Member,
}

/// A report's missing name as a language server hands it back (`import-actions`,
/// `typer::complete::import_fix`), with the report's span: every such report of a document,
/// those the imports are asked for (`asked`, the ones of the request's range) and the others,
/// whose places the answer needs to know as well.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportWanted {
    pub name: Box<str>,
    pub kind: MissingKind,
    pub span: Span,
    pub asked: bool,
}

/// A report's missing name, where it says one is (`Diagnostic::missing`). Equality and hashing
/// ignore it: a report is the same record with it and without.
#[derive(Clone, Debug, Default)]
pub struct MissingName(pub Option<Box<Missing>>);

impl PartialEq for MissingName {
    fn eq(&self, _: &MissingName) -> bool {
        true
    }
}

impl Eq for MissingName {}

impl std::hash::Hash for MissingName {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
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
    /// `plain`, of the first run, as the phase group `group` of the reference's makes it
    /// (`Position`): for a record made outside the store's reporters.
    pub fn plain_by(group: Group, file: FileId, span: Span, msg: String, is_warning: bool, of_body: bool) -> Diagnostic {
        let mut d = Diagnostic::plain(file, span, msg, is_warning, of_body);
        d.position = Position { run: 1, ..Position::of(group) };
        d
    }

    /// A diagnostic of nothing but its place, message and severity.
    pub fn plain(file: FileId, span: Span, msg: String, is_warning: bool, of_body: bool) -> Diagnostic {
        Diagnostic { file, span, msg, is_warning, of_body, owner: DiagOwner::NONE, place: None, dependent: false, unknown: false, late: false, by_phase: false, inlined_from: None, at_expansion: false, hint: false, unnecessary: false, id: crate::warnings::NO_ID, category: crate::warnings::Category::Plain, origin: None, action: None, missing: MissingName(None), phase: 0, unplaced: false, ordinal: Ordinal(0), position: Position::default(), unwound: Unwound(false) }
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
    /// The group of the reference's that makes it, where its producer names one (`by`).
    pub group: Group,
}

impl Warning {
    pub fn id(id: u16) -> Warning {
        Warning { id, ..Warning::plain() }
    }

    pub fn plain() -> Warning {
        Warning { id: crate::warnings::NO_ID, category: crate::warnings::Category::Plain, origin: None, phase: 0, unplaced: false, group: Group::Unstamped }
    }

    /// A plain warning of scalac's phase `phase` (`producer_rank`).
    pub fn of_phase(phase: u8) -> Warning {
        Warning { phase, ..Warning::plain() }
    }

    /// The warning as the group `group` of the reference makes it (`Position::group`).
    pub fn by(self, group: Group) -> Warning {
        Warning { group, ..self }
    }

    /// An unchecked type test (`UncheckedTypePattern`, an `UncheckedWarning`), which the
    /// reference's erasure reports (`TypeTestsCasts.interceptTypeApply`).
    pub fn unchecked() -> Warning {
        Warning { id: crate::warnings::id::UNCHECKED_TYPE_PATTERN, category: crate::warnings::Category::Unchecked, origin: None, phase: 0, unplaced: false, group: Group::Erasure }
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
/// interpolation's lint (`StringInterpolatorOpt`), an unchecked type test (`Erasure`), a non-local
/// return (`NonLocalReturns`).
fn producer_rank(d: &Diagnostic) -> u8 {
    use crate::warnings::{id, Category};
    if d.phase != 0 {
        return d.phase;
    }
    if !d.is_warning {
        return PHASE_TYPER;
    }
    if d.category == Category::Deprecation {
        return PHASE_FIRST_TRANSFORM;
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
/// The group of `FirstTransform`, after `Inlining`: `CrossVersionChecks`, `InlineVals`, `RefChecks`.
pub const PHASE_FIRST_TRANSFORM: u8 = 5;
/// `RefChecks`, of the group `CrossVersionChecks` begins.
pub const PHASE_REFCHECKS: u8 = 5;
/// `ForwardDepChecks`, of `PatternMatcher`'s group and before it in that group's order: after
/// `RefChecks`, before a match's warnings, neither hiding the other.
pub const PHASE_FORWARD_DEP_CHECKS: u8 = 6;
/// `TupleOptimizations`, before `Erasure`.
pub const PHASE_TUPLE_OPTIMIZATIONS: u8 = 10;
pub const PHASE_ERASURE: u8 = 11;
/// `TailRec`, after `Erasure`.
pub const PHASE_TAILREC: u8 = 12;
/// `NonLocalReturns`, of `TailRec`'s group, after it.
pub const PHASE_NONLOCAL_RETURNS: u8 = 13;

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
    /// What the build asks of its warnings (`warnings::Policy`): set once by the driver before the
    /// typer runs and never changed while the build lasts, so that every worker attached for the build
    /// shares it (`Worker::attach`). A watch session's next full build makes a new typer with a new
    /// policy, and a project whose flags change gets a new watch child from the language server.
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
    /// The definition being typed, whose the diagnostics reported now are (`Diagnostic::owner`):
    /// set by the typer around an item and each member's body it types.
    pub owner: DiagOwner,
    /// Whether the program has a lexical or syntax error (`Diagnostic::unknown`): set by the
    /// driver for what it presents.
    pub syntax_errors: bool,
    /// Whether the inlining phase is running: what is reported now is the phase's
    /// (`Diagnostic::by_phase`). Set by the typer with `Inlining::running`, on each worker.
    pub in_phase: bool,
    /// Whether a later run of the inlining phase is typing again the units it suspended
    /// (`Worker::retype_suspended`): an error reported now is the phase's doing
    /// (`Diagnostic::by_phase`), since a later run exists only where the first run's typer
    /// reported no error, so no check whose typer failed reports it. A warning may be one the
    /// first run's typing made too, and stays the typer's.
    pub later_typing: bool,
    /// The lexical and parse diagnostics of the program's texts, which a session sets before it
    /// answers: presented with the typer's, in file and position order.
    pub syntax: Vec<Diagnostic>,
    /// The build's count of the records made, every worker's (`Ordinal`), shared by the workers
    /// attached for the build.
    pub made: std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// The file of the unit whose suspension at typing the item being typed unwinds from
    /// (`Inlining::unwinding`): a record a definition of it makes is `Unwound`.
    pub unwinding: Option<u32>,
    /// The group the record being made is of, where its producer names one (`error_by`,
    /// `Worker::error_by`); `Unstamped` for the typer's reporters.
    pub producer: Group,
    /// The run of each unit that a later run of the reference compiles again (`Position::run`),
    /// by its file: a unit a macro of the build suspended; 1 for every other.
    pub unit_runs: crate::intern::FxMap<u32, u8>,
    /// The units that left their run, each with the group it left in (the typer where a
    /// transparent macro suspended it, the inlining phase for a plain one): the reference's later
    /// groups do not run on such a unit before its next run, so a record of a later group, made
    /// before the unit left or after, is of that run (`suspend_unit`).
    pub suspended_after: crate::intern::FxMap<u32, Group>,
    /// What of the expansion under way the code being typed stands in (`Position::context`): set by
    /// the typer where an expansion begins and ends and where a definition's own typing sets the
    /// expansions aside (`Worker::expansion_context`).
    pub context: Context,
}

impl Diagnostics {
    /// A diagnostic of the definition being typed (`owner`), made by the caller's caller
    /// (`Position::site`: the reporting helpers pass their own callers on).
    #[track_caller]
    fn made(&self, file: FileId, span: Span, msg: String, is_warning: bool, of_body: bool) -> Diagnostic {
        let mut d = Diagnostic::plain(file, span, msg, is_warning, of_body);
        d.owner = self.owner;
        d.position.site = Some(std::panic::Location::caller());
        if self.producer != Group::Unstamped {
            d.position.group = self.producer;
            d.position.named = true;
        }
        d
    }

    /// `error` of a record the group `group` of the reference makes (`Position`).
    #[track_caller]
    pub fn error_by(&mut self, group: Group, file: FileId, span: Span, msg: impl Into<String>) {
        let outer = std::mem::replace(&mut self.producer, group);
        self.error(file, span, msg);
        self.producer = outer;
    }

    #[track_caller]
    pub fn error(&mut self, file: FileId, span: Span, msg: impl Into<String>) {
        let d = self.made(file, span, msg.into(), false, self.of_bodies);
        self.push(d);
    }

    /// The record `d`, made now where it has no identity yet (`Ordinal`), as the last of the
    /// diagnostics: every record goes in by here or `extend`.
    pub fn push(&mut self, mut d: Diagnostic) {
        // What enters the records while the inlining phase runs is the phase's report: a record
        // an expansion ahead of the phase made and held for it among them (`Worker::release_held`),
        // and an error of a later run's typing (`later_typing`).
        if self.in_phase || (self.later_typing && !d.is_warning) {
            d.by_phase = true;
        }
        if d.ordinal.0 == 0 {
            // The record's position, given once: its producer's group, or the typer's for a
            // record of the typer's reporters (the inlining phase's while it runs); the run of
            // its unit.
            if d.position.group == Group::Unstamped {
                d.position.group = if self.in_phase { Group::Inlining } else { Group::Typer };
            }
            if d.position.context == Context::None {
                d.position.context = self.context;
            }
            if d.position.run == 0 {
                let unit = if d.owner.is_none() { d.file.0 } else { d.owner.file };
                d.position.run = self.unit_runs.get(&unit).copied().unwrap_or(1);
                if self.suspended_after.get(&unit).is_some_and(|&left| d.position.group > left) {
                    d.position.run = d.position.run.saturating_add(1);
                }
            }
            d.ordinal = Ordinal(self.made.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1);
            if self.unwinding.is_some_and(|f| !d.owner.is_none() && d.owner.file == f) {
                d.unwound = Unwound(true);
            }
        }
        self.items.push(d);
    }

    /// The unit `file` leaves its run in the group `left`, or would in the reference where teq's
    /// inlining phase is not run (`Worker::inlining_phase`): what a later group reported of it so
    /// far is of its next run, and so is what one reports of it until that run starts
    /// (`suspended_after`). Once a unit, until its next run starts (`resume_unit`).
    pub fn suspend_unit(&mut self, file: u32, left: Group) {
        if self.suspended_after.contains_key(&file) {
            return;
        }
        self.suspended_after.insert(file, left);
        for d in &mut self.items {
            let unit = if d.owner.is_none() { d.file.0 } else { d.owner.file };
            if unit == file && d.position.group > left && d.position.group != Group::RunEnd {
                d.position.run = d.position.run.saturating_add(1);
            }
        }
    }

    /// The unit `file` is in a run again: one run more than its last (`Position::run`).
    pub fn resume_unit(&mut self, file: u32) {
        let run = self.unit_runs.get(&file).copied().unwrap_or(1).saturating_add(1);
        self.unit_runs.insert(file, run);
        self.suspended_after.remove(&file);
    }

    /// `push` of each of `ds`.
    pub fn extend(&mut self, ds: impl IntoIterator<Item = Diagnostic>) {
        for d in ds {
            self.push(d);
        }
    }

    /// An error that depends on malformed syntax (`Diagnostic::dependent`).
    #[track_caller]
    pub fn dependent_error(&mut self, file: FileId, span: Span, msg: impl Into<String>) {
        let mut d = self.made(file, span, msg.into(), false, self.of_bodies);
        d.dependent = true;
        self.push(d);
    }

    /// `dependent_error`, as the phase group `group` of the reference's reports it.
    #[track_caller]
    pub fn dependent_error_by(&mut self, group: Group, file: FileId, span: Span, msg: impl Into<String>) {
        let outer = std::mem::replace(&mut self.producer, group);
        self.dependent_error(file, span, msg);
        self.producer = outer;
    }

    /// A warning of the syntax, a migration warning where `migration` says (`Category::Migration`), with
    /// dotty's message's identity `id`.
    #[track_caller]
    pub fn syntax_warning(&mut self, file: FileId, span: Span, msg: impl Into<String>, migration: bool, id: u16) {
        let mut d = self.made(file, span, msg.into(), true, false);
        d.id = id;
        if migration {
            d.category = crate::warnings::Category::Migration;
        }
        // The parser's.
        d.position.group = Group::Parser;
        d.position.named = true;
        self.push(d);
    }

    #[track_caller]
    pub fn warn(&mut self, file: FileId, span: Span, msg: impl Into<String>) {
        let d = self.made(file, span, msg.into(), true, self.of_bodies);
        self.push(d);
    }

    /// A warning the phase group `group` of the reference's reports (`Position::group`).
    #[track_caller]
    pub fn warn_by(&mut self, group: Group, file: FileId, span: Span, msg: impl Into<String>) {
        let outer = std::mem::replace(&mut self.producer, group);
        self.warn(file, span, msg);
        self.producer = outer;
    }

    /// A warning with its identity (`Diagnostic::id`), depending on malformed syntax where
    /// `dependent` says.
    #[track_caller]
    pub fn warn_as(&mut self, file: FileId, span: Span, msg: impl Into<String>, w: Warning, dependent: bool) {
        let mut d = self.made(file, span, msg.into(), true, self.of_bodies);
        d.dependent = dependent;
        d.id = w.id;
        d.category = w.category;
        d.origin = w.origin;
        d.phase = w.phase;
        d.unplaced = w.unplaced;
        // The group its producer named (`Warning::by`), or that of the phase it named by its rank.
        let group = if w.group != Group::Unstamped { w.group } else { Group::of_phase(w.phase) };
        if group != Group::Unstamped {
            d.position.group = group;
            d.position.named = true;
        }
        self.push(d);
    }

    /// A selector no name resolved through (`typer::unused`): a warning, or a hint.
    #[track_caller]
    pub fn unnecessary(&mut self, file: FileId, span: Span, msg: impl Into<String>, hint: bool, origin: Option<Box<str>>) {
        let mut d = self.made(file, span, msg.into(), true, true);
        d.hint = hint;
        d.unnecessary = true;
        d.id = crate::warnings::id::UNUSED_SYMBOL;
        d.origin = origin;
        // The warning is `CheckUnused.PostPatMat`'s; the hint, which stands where no option asks
        // for the warning, is teq's own.
        d.position.group = if hint { Group::Own } else { Group::CheckUnused };
        d.position.named = true;
        self.push(d);
    }

    /// An error of a library body's pseudo file placed in the library's source.
    #[track_caller]
    pub fn error_placed(&mut self, file: FileId, span: Span, msg: impl Into<String>, place: Option<Place>) {
        self.error(file, span, msg);
        if let Some(d) = self.items.last_mut() {
            d.place = place.map(Box::new);
        }
    }

    /// The diagnostics a build presents: the syntax diagnostics a session set, then the typer's
    /// but the dependent ones and, while the program has a syntax error, the unknown ones. One
    /// that says what an earlier one says at the same place is presented once, as scalac's
    /// reporter does (`report_from`'s `Occupied`).
    ///
    /// Of the typer's diagnostics from the index `from` on, those before left out, and the
    /// syntax diagnostics where the report is not `whole`.
    fn presented_from(&self, from: usize, whole: bool) -> impl Iterator<Item = &Diagnostic> {
        let syntax = self.syntax_errors;
        let earlier = self.syntax.iter().any(|d| !d.is_warning) || self.items.iter().any(|d| !d.is_warning && !d.dependent && !d.late && !(d.unknown && syntax));
        let failed = self.typer_failed();
        let hidden = self.hidden_inlined(earlier, failed);
        self.syntax
            .iter()
            .filter(move |_| whole)
            .chain(self.items.iter().enumerate().skip(from).filter(move |(i, d)| !d.dependent && !(d.unknown && syntax) && !(d.late && earlier) && !(d.by_phase && failed) && !hidden.contains(i)).map(|(_, d)| d))
    }

    /// Whether the typer failed, after which scalac runs no phase: the program has a syntax error
    /// (the driver's flag, set before the typing; a session's records of the syntax are the last
    /// check's until it answers), or the records hold an error that is the first run's typer's
    /// own (not a later phase's, `late`, nor the inlining phase's or a later run's typing's,
    /// `by_phase`, which a session keeps from the checks before). One question, asked by the
    /// typer before it runs the inlining phase and by a report of what a phase made: dotty asks
    /// whether its reporter has errors run by run (`Run.compileSuspendedUnits`), so what the
    /// phase of one run reported stays reported where a later run's typer reports an error.
    pub(crate) fn typer_failed(&self) -> bool {
        self.syntax_errors || self.items.iter().any(|d| !d.is_warning && !d.dependent && !d.late && !d.by_phase)
    }

    /// `TEQ_UNPRESENTED=1`: the typer's records `presented_from` leaves out, each with why, in their
    /// order, for a trace of what a history read (`typer::merge::HistoryQuery`) counts and the
    /// report does not show.
    #[cold]
    fn trace_unpresented(&self) {
        let syntax = self.syntax_errors;
        let earlier = self.syntax.iter().any(|d| !d.is_warning) || self.items.iter().any(|d| !d.is_warning && !d.dependent && !d.late && !(d.unknown && syntax));
        let failed = self.typer_failed();
        let hidden = self.hidden_inlined(earlier, failed);
        for (i, d) in self.items.iter().enumerate() {
            let why = if d.dependent {
                "dependent"
            } else if d.unknown && syntax {
                "unknown after a syntax error"
            } else if d.late && earlier {
                "late, an earlier error presented"
            } else if d.by_phase && failed {
                "the inlining phase's, the typer failed"
            } else if hidden.contains(&i) {
                "an inlined error at an earlier one's place"
            } else {
                continue;
            };
            eprintln!("teq: not presented ({}): {} at file {} {}..{}: {}", why, if d.is_warning { "warning" } else { "error" }, d.file.0, d.span.start, d.span.end, d.msg.lines().next().unwrap_or(""));
        }
    }

    /// `TEQ_POSITIONS=1`: every record of the build, presented or not, with its position
    /// (`Position`), one line a record in file and place order, then how many records each group
    /// has: for a script that applies the reference's gates to them, and for a census of the
    /// records no producer named a group for.
    #[cold]
    fn trace_positions(&self, files: &[SourceFile]) {
        use crate::warnings::Action;
        // What the policy makes of a warning, which the reference decides where it issues one:
        // an `@nowarn` around it, `-Wconf`'s action, a category its option leaves to the summary.
        let paths: Vec<Option<String>> = files.iter().map(|f| absolute_path(&f.path)).collect();
        let policy = &*self.policy;
        let action = |d: &Diagnostic| -> &'static str {
            if !d.is_warning || d.hint {
                return "";
            }
            let subject = d.subject(paths.get(d.file.0 as usize).and_then(|p| p.as_deref()));
            let covered = self.suppressions.iter().any(|s| (s.covers(d.file, d.span) || d.inlined_from.is_some_and(|(f, at)| s.covers(f, at))) && s.matches(&subject));
            match policy.action(&subject) {
                _ if covered => " nowarn",
                Action::Silent => " as_silent",
                Action::Error => " as_error",
                Action::Info => " as_info",
                Action::Default if policy.summarizes(d.category) => " summarized",
                Action::Verbose | Action::Warning | Action::Default => "",
            }
        };
        let mut lines: Vec<(u32, u32, u32, String)> = Vec::new();
        let mut census: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        let records = self.syntax.iter().map(|d| ("syntax", d)).chain(self.items.iter().map(|d| ("item", d))).chain(self.suppression_warnings.iter().map(|d| ("suppression", d)));
        for (kind, d) in records {
            let p = d.position;
            *census.entry(format!("{:?}{}", p.group, if p.named { "" } else { " by default" })).or_default() += 1;
            lines.push((d.file.0, d.span.start, d.ordinal.0 as u32, position_line(files, kind, d, action(d))));
        }
        lines.sort();
        for (_, _, _, l) in &lines {
            eprintln!("{}", l);
        }
        let census: Vec<String> = census.iter().map(|(g, n)| format!("{} {}", g, n)).collect();
        eprintln!("teq: positions: {} records: {}", lines.len(), census.join(", "));
    }

    /// The errors of inline expansions whose inlined place overlaps that of an earlier one
    /// (`Diagnostic::inlined_from`), by index. An error takes a place only where it is presented
    /// itself: not one left out for what malformed syntax left unknown, nor a late one while an
    /// error that is not late exists (`earlier`), nor the inlining phase's where the typer
    /// failed (`failed`), as scalac hides a message at a position a message it reported took
    /// (`UniqueMessagePositions`) and reports none of a phase it does not run: an `inline if` an
    /// expansion cannot reduce is presented where an `inline val` of the same body, whose check
    /// is a later phase's, reported first at the same place.
    fn hidden_inlined(&self, earlier: bool, failed: bool) -> Vec<usize> {
        let syntax = self.syntax_errors;
        let mut inlined: Vec<(FileId, u32, usize, FileId, Span)> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, d)| !d.is_warning && !d.dependent && !(d.unknown && syntax) && !(d.late && earlier) && !(d.by_phase && failed))
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
    #[track_caller]
    pub fn late_error(&mut self, file: FileId, span: Span, msg: impl Into<String>) {
        self.late_error_of(file, span, msg, PHASE_INLINING);
    }

    /// `late_error`, the record's position that of the phase group `group` of the reference's.
    #[track_caller]
    pub fn late_error_by(&mut self, group: Group, file: FileId, span: Span, msg: impl Into<String>) {
        let outer = std::mem::replace(&mut self.producer, group);
        self.late_error(file, span, msg);
        self.producer = outer;
    }

    /// A late error of the phase `phase` (`producer_rank`).
    #[track_caller]
    pub fn late_error_of(&mut self, file: FileId, span: Span, msg: impl Into<String>, phase: u8) {
        // The phase names the record's group, unless its producer named another (`error_by`).
        let outer = self.producer;
        if outer == Group::Unstamped {
            self.producer = Group::of_phase(phase);
        }
        self.error(file, span, msg);
        self.producer = outer;
        if let Some(d) = self.items.last_mut() {
            d.late = true;
            d.phase = phase;
        }
    }

    /// Whether a build with these diagnostics fails, as the policy presents them
    /// (`report`): an error, or under `--werror` a warning.
    pub fn fails(&self, files: &[SourceFile], werror: bool) -> bool {
        self.syntax.iter().any(|d| !d.is_warning) || self.has_errors() || self.report(files).fails(werror)
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| !d.is_warning)
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
        if whole && std::env::var_os("TEQ_UNPRESENTED").is_some_and(|v| v == "1") {
            self.trace_unpresented();
        }
        if whole && std::env::var_os("TEQ_POSITIONS").is_some_and(|v| v == "1") {
            self.trace_positions(files);
        }
        let candidates: Vec<&Diagnostic> = self.presented_from(from, whole).chain(extras.iter()).collect();
        // Presented in file and position order, the order they were made in at one place.
        let mut by_place: Vec<usize> = (0..candidates.len()).collect();
        by_place.sort_by_key(|&i| (candidates[i].file, candidates[i].span.start, i));
        let mut place_rank = vec![0usize; candidates.len()];
        for (r, &i) in by_place.iter().enumerate() {
            place_rank[i] = r;
        }
        // Weighed in the order scalac's phases report them, the order made within one: of two at
        // one place, the first reported takes it (`UniqueMessagePositions`). A unit's warnings of
        // its parser and typer are issued once its typer is done (`Reporter.issueIfNotSuppressed`
        // suspends them until `Run.Suppressions.reportSuspendedMessages`), after the errors of both.
        let mut by_producer: Vec<usize> = (0..candidates.len()).collect();
        by_producer.sort_by_key(|&i| {
            let rank = producer_rank(candidates[i]);
            (rank, rank == PHASE_TYPER && candidates[i].is_warning, i)
        });
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
        // Each with the position of the first warning it counts: the reference prints the summary
        // at the run's end whatever was reported (`Run.printSummary`), where a warning it counts
        // was made at all.
        let mut unreported: Vec<(Category, usize, Position)> = Vec::new();
        let mut consulted = false;
        // The position of the first warning the policy was asked about: where the reference parses
        // its `-Wconf` (`WConf.parsed`, at the first warning it issues) and reports a failure.
        let mut first_consulted = Position::of(Group::RunEnd);
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
            if !consulted {
                first_consulted = d.position;
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
                    match unreported.iter_mut().find(|(c, ..)| *c == d.category) {
                        Some((_, n, _)) => *n += 1,
                        None => unreported.push((d.category, 1, d.position)),
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
        if whole && policy.unused.nowarn && errors == 0 && !self.syntax.iter().any(|d| !d.is_warning) {
            for (i, sup) in self.suppressions.iter().enumerate() {
                let duplicate = self.suppressions.iter().enumerate().any(|(j, other)| j != i && used[j] == 1 && other.file == sup.file && other.annot == sup.annot);
                if used[i] != 1 && !duplicate && !sup.invalid() {
                    let more = if used[i] == 2 { " but matches a diagnostic" } else { "" };
                    late.push(Diagnostic::plain_by(Group::RunEnd, sup.file, sup.annot, format!("@nowarn annotation does not suppress any warnings{}", more), true, true));
                }
            }
        }
        for (category, n, first) in unreported.into_iter().filter(|_| whole) {
            if let Some(msg) = crate::warnings::summary(category, n) {
                let mut d = Diagnostic::plain(NO_FILE, Span::default(), msg, true, true);
                d.position = Position { named: true, site: None, ..first };
                late.push(d);
            }
        }
        for d in late {
            if !consulted {
                first_consulted = d.position;
            }
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
            d.position = Position { named: true, site: None, ..first_consulted };
            shown.insert(0, Shown { d: Cow::Owned(d), level: Level::Warning, order: 0 });
        }
        if whole && std::env::var_os("TEQ_POSITIONS").is_some_and(|v| v == "1") {
            // The records made here, which no store holds (never numbered: `Ordinal`).
            for s in shown.iter().filter(|s| s.d.ordinal.0 == 0 && !self.suppression_warnings.iter().any(|w| std::ptr::eq(w, &*s.d))) {
                eprintln!("{}", position_line(files, "report", &s.d, ""));
            }
        }
        let errors = shown.iter().filter(|s| s.level == Level::Error).count();
        let warnings = shown.iter().filter(|s| s.level == Level::Warning).count();
        // A build whose records hold an error presents one: a rule that leaves an error's place to
        // another record leaves it to one that is shown. A dependent error and, under a syntax
        // error, an unknown one or the inlining phase's are left out for an error reported before
        // them, and are not asked about.
        debug_assert!(
            !whole || errors > 0 || !self.items.iter().any(|d| !d.is_warning && !d.dependent && !((d.unknown || d.by_phase) && self.syntax_errors)),
            "the records hold an error and the report presents none"
        );
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

/// Where a build run with `--diagnostics <file>` writes each diagnostic it shows, a JSON line a
/// diagnostic in the order shown: its severity, its file, line and column where it has them, its
/// whole message, and the lines shown for it (`shown`), so that a reader takes a diagnostic's
/// extent from the build, never from the shown text, whose message may hold any line (dotty's
/// `TestReporter` collects the `Diagnostic`s themselves). A macro's `report.info` is one of them
/// (`info`), as dotty's `QuotesImpl` reports it through `report.echo`, an `Info` diagnostic.
static RECORDS: std::sync::OnceLock<std::sync::Mutex<std::fs::File>> = std::sync::OnceLock::new();

/// Opens `path` for the diagnostics' records (`RECORDS`): afresh, or after the records a verb
/// wrote there first (`append`, `--diagnostics-append`: `teq build`'s generators').
pub fn record_diagnostics_to(path: &str, append: bool) -> Result<(), String> {
    let opened = if append { std::fs::OpenOptions::new().create(true).append(true).open(path) } else { std::fs::File::create(path) };
    let file = opened.map_err(|e| format!("cannot write {}: {}", path, e))?;
    let _ = RECORDS.set(std::sync::Mutex::new(file));
    Ok(())
}

/// One shown diagnostic's record, where a file is open for them.
pub(crate) fn record_shown(severity: &str, at: Option<(&str, Option<(usize, usize)>)>, message: &str, shown: &str) {
    let Some(file) = RECORDS.get() else { return };
    let mut line = String::from("{\"severity\":");
    crate::watch::json_string(severity, &mut line);
    if let Some((path, position)) = at {
        line.push_str(",\"file\":");
        crate::watch::json_string(path, &mut line);
        if let Some((l, c)) = position {
            let _ = write!(line, ",\"line\":{},\"col\":{}", l, c);
        }
    }
    line.push_str(",\"message\":");
    crate::watch::json_string(message, &mut line);
    line.push_str(",\"shown\":");
    crate::watch::json_string(shown, &mut line);
    line.push_str("}\n");
    let mut f = file.lock().unwrap_or_else(|e| e.into_inner());
    let _ = std::io::Write::write_all(&mut *f, line.as_bytes());
}

/// The shown diagnostics in the order of `ranks` over the files, a file without a rank by its id,
/// each recorded as it is shown (`record_shown`).
pub fn render_shown(shown: &[Shown], files: &[SourceFile], ranks: &[u32]) -> String {
    let mut out = String::new();
    let mut items: Vec<&Shown> = shown.iter().collect();
    items.sort_by_key(|s| (s.order, ranks.get(s.d.file.0 as usize).copied().unwrap_or(s.d.file.0), s.d.span.start));
    for s in items {
        let d = &*s.d;
        let severity = s.level.name();
        // scalac's `Migration Warning` (`Diagnostic.MigrationWarning`) apart from its other warnings in the
        // text; its record keeps the level, a migration warning's being a warning's.
        let header = if s.level == Level::Warning && d.category == crate::warnings::Category::Migration { "migration warning" } else { severity };
        let start = out.len();
        if d.file == NO_FILE {
            let _ = writeln!(out, "{}: {}", header, d.msg);
            record_shown(severity, None, &d.msg, &out[start..]);
            continue;
        }
        if let Some(p) = &d.place {
            let place = p.render();
            let _ = writeln!(out, "{}: {}: {}", place, header, d.msg);
            if let (Some(text), Some(col)) = (&p.line_text, p.col) {
                let _ = writeln!(out, "  {}", text);
                let _ = writeln!(out, "  {}^", " ".repeat((col as usize).saturating_sub(1)));
            }
            record_shown(severity, Some((&place, None)), &d.msg, &out[start..]);
            continue;
        }
        let Some(f) = files.get(d.file.0 as usize) else {
            let _ = writeln!(out, "<library>: {}: {}", header, d.msg);
            record_shown(severity, Some(("<library>", None)), &d.msg, &out[start..]);
            continue;
        };
        let (line, col, line_text) = locate(&f.text, d.span.start as usize);
        let _ = writeln!(out, "{}:{}:{}: {}: {}", f.path, line, col, header, d.msg);
        let _ = writeln!(out, "  {}", line_text);
        let width = (d.span.end.saturating_sub(d.span.start) as usize)
            .clamp(1, line_text.len().saturating_sub(col - 1).max(1));
        let _ = writeln!(out, "  {}{}", " ".repeat(col - 1), "^".repeat(width));
        record_shown(severity, Some((&f.path, Some((line, col)))), &d.msg, &out[start..]);
    }
    out
}

/// A record's line of the positions dump (`Diagnostics::trace_positions`): its kind, place and
/// level, then its position (run, group, context, the site in teq's source that made it), its
/// flags (`default`: no producer named the group), what the policy makes of it, its first words.
fn position_line(files: &[SourceFile], kind: &str, d: &Diagnostic, action: &str) -> String {
    let place = match files.get(d.file.0 as usize) {
        Some(f) => {
            let (line, col, _) = locate(&f.text, d.span.start as usize);
            format!("{}:{}:{}", f.path.rsplit('/').next().unwrap_or(&f.path), line, col)
        }
        None => format!("file {}:{}", d.file.0, d.span.start),
    };
    let p = d.position;
    let site = p.site.map_or("?".to_string(), |l| format!("{}:{}", l.file(), l.line()));
    // `default`: no producer named the record's group (the typer's reporters).
    let flags: String = [(!p.named, "default"), (d.dependent, "dependent"), (d.unknown, "unknown"), (d.late, "late"), (d.by_phase, "by_phase"), (d.hint, "hint")].iter().filter(|(on, _)| *on).map(|(_, n)| format!(" {}", n)).collect();
    let msg = d.msg.lines().next().unwrap_or("");
    format!("teq: position: {} {} {}: run {} {:?} {:?} [{}]{}{} | {}", kind, place, if d.is_warning { "warning" } else { "error" }, p.run, p.group, p.context, site, flags, action, msg)
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

    /// FNV-1a's test vectors, 32 bits for a file's tag and 64 for a body's: both reach names in
    /// the output.
    #[test]
    fn tags_are_fnv_1a() {
        assert_eq!([file_tag(""), file_tag("a"), file_tag("foobar")], [0x811c_9dc5, 0xe40c_292c, 0xbf9c_f968]);
        assert_eq!([body_tag(""), body_tag("a"), body_tag("foobar")], [0xcbf2_9ce4_8422_2325, 0xaf63_dc4c_8601_ec8c, 0x8594_4171_f739_67e8]);
    }

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

    /// An error of an inline expansion, moved to the call at `at`, of the inlined code at `from`.
    fn inlined(at: u32, from: u32, msg: &str) -> Diagnostic {
        let mut d = Diagnostic::plain(FileId(0), Span { start: at, end: at + 4 }, msg.to_string(), false, true);
        d.inlined_from = Some((FileId(0), Span { start: from, end: from + 4 }));
        d
    }

    /// The errors a whole report of `items` presents, by their messages.
    fn presented(items: Vec<Diagnostic>, syntax_errors: bool) -> Vec<String> {
        let diags = Diagnostics { items, syntax_errors, ..Default::default() };
        diags.report(&[]).shown.iter().map(|s| s.d.msg.clone()).collect()
    }

    /// An inlined error hides a later one at its place only where it is presented itself: a late
    /// one gives way to an error that is not late, and then takes no place.
    #[test]
    fn a_late_error_left_out_hides_no_error_at_its_place() {
        let late = |at, from, msg| Diagnostic { late: true, phase: PHASE_FIRST_TRANSFORM, ..inlined(at, from, msg) };
        // the late one first, the typer's at the same inlined place: the typer's is the one shown
        assert_eq!(presented(vec![late(10, 40, "of a later phase"), inlined(10, 40, "of the typer")], false), ["of the typer"]);
        // alone, the late one is shown; of two late ones at a place, the first
        assert_eq!(presented(vec![late(10, 40, "of a later phase")], false), ["of a later phase"]);
        assert_eq!(presented(vec![late(10, 40, "the first"), late(10, 40, "the second")], false), ["the first"]);
        // two of the typer's at one inlined place: the first
        assert_eq!(presented(vec![inlined(10, 40, "the first"), inlined(10, 40, "the second")], false), ["the first"]);
        // the typer's error elsewhere leaves the late one out, which hides nothing at its own place
        assert_eq!(presented(vec![late(10, 40, "of a later phase"), inlined(10, 40, "at its place"), inlined(90, 70, "elsewhere")], false), ["at its place", "elsewhere"]);
        // an error over the late one's place takes the place, and hides the next one over its own
        assert_eq!(presented(vec![late(10, 40, "of a later phase"), inlined(20, 42, "over it"), inlined(30, 45, "over that one alone")], false), ["over it"]);
    }

    /// An error the inlining phase made is presented only while the records hold no error of the
    /// typer and the program no syntax error, as scalac runs no phase after its typer failed; a
    /// later phase's error at its place gives way to it.
    #[test]
    fn the_inlining_phases_error_gives_way_to_the_typers() {
        let phase = |at, from, msg| Diagnostic { by_phase: true, ..inlined(at, from, msg) };
        let late = |at, from, msg| Diagnostic { late: true, by_phase: true, phase: PHASE_FIRST_TRANSFORM, ..inlined(at, from, msg) };
        let typers = |at, msg: &str| Diagnostic::plain(FileId(1), Span { start: at, end: at + 4 }, msg.to_string(), false, true);
        // alone, and over a later phase's error made first at the same inlined place
        assert_eq!(presented(vec![phase(10, 40, "of the phase")], false), ["of the phase"]);
        assert_eq!(presented(vec![late(10, 40, "of a later phase"), phase(10, 40, "of the phase")], false), ["of the phase"]);
        // beside an error of the typer in another file: the typer's alone
        assert_eq!(presented(vec![late(10, 40, "of a later phase"), phase(10, 40, "of the phase"), typers(5, "of the typer")], false), ["of the typer"]);
        // under a syntax error, which the driver presents: none of the records
        assert_eq!(presented(vec![late(10, 40, "of a later phase"), phase(10, 40, "of the phase")], true), [] as [&str; 0]);
        // left out, it takes no place: an inlined error of the typer over its place is shown
        assert_eq!(presented(vec![phase(10, 40, "of the phase"), inlined(10, 40, "of the typer, inlined")], false), ["of the typer, inlined"]);
        // a warning of the phase is the phase's too: shown alone, left out beside the typer's error
        let warning = |at: u32, msg: &str| Diagnostic { by_phase: true, ..Diagnostic::plain(FileId(0), Span { start: at, end: at + 4 }, msg.to_string(), true, true) };
        assert_eq!(presented(vec![warning(60, "a warning of the phase")], false), ["a warning of the phase"]);
        assert_eq!(presented(vec![warning(60, "a warning of the phase"), typers(5, "of the typer")], false), ["of the typer"]);
    }

    /// An error the typing of a suspended unit reports in a later run is the phase's doing: it
    /// leaves what the phase reported before it presented, and goes with it where the first
    /// run's typer failed; a warning of that typing stays the typer's.
    #[test]
    fn a_later_runs_typer_error_leaves_the_phases_report_presented() {
        let warning = |at: u32, msg: &str| Diagnostic { by_phase: true, ..Diagnostic::plain(FileId(0), Span { start: at, end: at + 4 }, msg.to_string(), true, true) };
        let made = |later_typing: bool, is_warning: bool, msg: &str| {
            let mut diags = Diagnostics { later_typing, ..Default::default() };
            diags.push(Diagnostic::plain(FileId(1), Span { start: 5, end: 9 }, msg.to_string(), is_warning, true));
            diags.items.pop().unwrap()
        };
        assert!(made(true, false, "of a later run's typer").by_phase);
        assert!(!made(true, true, "a warning of a later run's typing").by_phase);
        assert!(!made(false, false, "of the first run's typer").by_phase);
        // the phase's warning of the run before, then the later run's typer's error: both
        assert_eq!(presented(vec![warning(60, "a warning of the phase"), made(true, false, "of a later run's typer")], false), ["a warning of the phase", "of a later run's typer"]);
        // beside an error of the first run's typer, and under a syntax error: neither
        assert_eq!(presented(vec![warning(60, "a warning of the phase"), made(true, false, "of a later run's typer"), made(false, false, "of the first run's typer")], false), ["of the first run's typer"]);
        assert_eq!(presented(vec![warning(60, "a warning of the phase"), made(true, false, "of a later run's typer")], true), [] as [&str; 0]);
    }

    /// An error about an erroneous type is left out while the program has a syntax error, and
    /// then hides no error at its inlined place.
    #[test]
    fn an_unknown_error_left_out_hides_no_error_at_its_place() {
        let unknown = |at, from, msg| Diagnostic { unknown: true, ..inlined(at, from, msg) };
        assert_eq!(presented(vec![unknown(10, 40, "of an erroneous type"), inlined(10, 40, "a known one")], true), ["a known one"]);
        // without a syntax error the unknown one is shown and takes the place
        assert_eq!(presented(vec![unknown(10, 40, "of an erroneous type"), inlined(10, 40, "a known one")], false), ["of an erroneous type"]);
    }

    /// A record's position is given where it is made: the group its producer names (by itself, by
    /// a warning's `by`, by a phase's rank), or the typer's for a reporter that names none, the
    /// inlining phase's while the phase runs; the run of its unit; the expansion it stands in.
    #[test]
    fn a_record_takes_its_position_where_it_is_made() {
        let at = Span { start: 3, end: 7 };
        let mut diags = Diagnostics::default();
        diags.error(FileId(0), at, "of the typer");
        diags.error_by(Group::RefChecks, FileId(0), at, "of the overriding checks");
        diags.warn_as(FileId(0), at, "of the pattern matcher", Warning::plain().by(Group::PatternMatcher), false);
        diags.warn_as(FileId(0), at, "of the erasure by its rank", Warning::of_phase(PHASE_ERASURE), false);
        diags.late_error_of(FileId(0), at, "of the tail calls' phase", PHASE_TAILREC);
        diags.late_error_by(Group::Typer, FileId(0), at, "late, and the typer's");
        diags.error_by(Group::Own, FileId(0), at, "a refusal of teq's own");
        diags.unnecessary(FileId(0), at, "unused import", false, None);
        diags.unnecessary(FileId(0), at, "unused import", true, None);
        let positions: Vec<(Group, bool)> = diags.items.iter().map(|d| (d.position.group, d.position.named)).collect();
        assert_eq!(
            positions,
            [
                (Group::Typer, false),
                (Group::RefChecks, true),
                (Group::PatternMatcher, true),
                (Group::Erasure, true),
                (Group::TailRec, true),
                (Group::Typer, true),
                (Group::Own, true),
                (Group::CheckUnused, true),
                (Group::Own, true),
            ]
        );
        assert!(diags.items.iter().all(|d| d.position.run == 1 && d.position.context == Context::None && d.position.site.is_some()));
        // a producer's group lasts for its one record
        assert_eq!(diags.producer, Group::Unstamped);
        // while the inlining phase runs, in its expansion, on a unit of a later run
        let mut phase = Diagnostics { in_phase: true, context: Context::Phase, ..Default::default() };
        phase.unit_runs.insert(4, 2);
        phase.error(FileId(4), at, "of the phase's expansion");
        phase.error_by(Group::Erasure, FileId(4), at, "of a check at the phase's end");
        phase.error(FileId(5), at, "of a unit of the first run");
        let made: Vec<(u8, Group, Context)> = phase.items.iter().map(|d| (d.position.run, d.position.group, d.position.context)).collect();
        assert_eq!(made, [(2, Group::Inlining, Context::Phase), (2, Group::Erasure, Context::Phase), (1, Group::Inlining, Context::Phase)]);
        // in an expansion the typer makes
        let mut typing = Diagnostics { context: Context::Typing, ..Default::default() };
        typing.error(FileId(0), at, "of a transparent method's expansion");
        assert_eq!((typing.items[0].position.group, typing.items[0].position.context), (Group::Typer, Context::Typing));
    }

    /// What an expansion ahead of the inlining phase reported is the phase's, unless its producer
    /// named another group; a record pushed again keeps the position it was made with.
    #[test]
    fn a_held_record_is_the_inlining_phases() {
        let at = Span { start: 3, end: 7 };
        let mut diags = Diagnostics { context: Context::Typing, ..Default::default() };
        diags.error(FileId(0), at, "of the expansion");
        diags.error_by(Group::Erasure, FileId(0), at, "of a later group");
        let mut held = std::mem::take(&mut diags.items);
        for d in &mut held {
            d.position.ahead_of_phase();
        }
        diags.in_phase = true;
        diags.context = Context::None;
        diags.extend(held);
        let made: Vec<(Group, bool, Context)> = diags.items.iter().map(|d| (d.position.group, d.position.named, d.position.context)).collect();
        assert_eq!(made, [(Group::Inlining, false, Context::Phase), (Group::Erasure, true, Context::Phase)]);
    }

    /// A unit that leaves its run (a macro of the run suspends it) is taken up by the reference's
    /// later groups only in its next run: what a group after the one it left in reported of it,
    /// before it left or after, is of that run; what the groups up to it reported stays.
    #[test]
    fn a_suspended_units_later_groups_are_its_next_runs() {
        let at = Span { start: 3, end: 7 };
        let mut diags = Diagnostics::default();
        diags.error(FileId(2), at, "of the typer, before the unit left");
        diags.error_by(Group::PostTyper, FileId(2), at, "after the typer, before the phase");
        diags.error_by(Group::RefChecks, FileId(2), at, "of the overriding checks, before the unit left");
        diags.error_by(Group::RefChecks, FileId(3), at, "of another unit");
        diags.suspend_unit(2, Group::Inlining);
        diags.suspend_unit(2, Group::Typer);
        diags.error_by(Group::TailRec, FileId(2), at, "of a later group, after it left");
        diags.error(FileId(2), at, "of the typer, on another unit's demand");
        let runs = |d: &Diagnostics| d.items.iter().map(|x| x.position.run).collect::<Vec<u8>>();
        assert_eq!(runs(&diags), [1, 1, 2, 1, 2, 1]);
        // its next run starts: every record of it made from now on is of that run
        diags.resume_unit(2);
        diags.error_by(Group::RefChecks, FileId(2), at, "of the next run");
        diags.error(FileId(2), at, "of the next run's typer");
        assert_eq!(runs(&diags), [1, 1, 2, 1, 2, 1, 2, 2]);
        // one a transparent macro suspended left in the typer: PostTyper's too is the next run's
        let mut typing = Diagnostics::default();
        typing.error_by(Group::PostTyper, FileId(2), at, "after the typer");
        typing.suspend_unit(2, Group::Typer);
        assert_eq!(runs(&typing), [2]);
    }

    /// Nothing but the dump reads a position: two records that differ in it alone are equal and
    /// hash alike, so no presentation and no comparison of two builds' records can turn on it.
    #[test]
    fn a_position_is_invisible_to_equality() {
        use std::hash::{Hash, Hasher};
        let at = Span { start: 3, end: 7 };
        let mut diags = Diagnostics::default();
        diags.error(FileId(0), at, "the same words");
        diags.error_by(Group::TailRec, FileId(0), at, "the same words");
        let (a, b) = (&diags.items[0], &diags.items[1]);
        assert_ne!(a.position.group, b.position.group);
        assert!(a == b);
        let hash = |d: &Diagnostic| {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            d.hash(&mut h);
            h.finish()
        };
        assert_eq!(hash(a), hash(b));
        assert_eq!(Group::of_phase(PHASE_FIRST_TRANSFORM), Group::RefChecks);
        assert_eq!(Group::of_phase(0), Group::Unstamped);
    }
}
