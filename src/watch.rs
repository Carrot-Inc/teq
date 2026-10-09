//! `teq compiler watch`: builds once, then stays resident and rebuilds on command, keeping the parsed
//! files and the typed program in memory between builds.
//!
//! Commands come on stdin, one per line: `build` checks every file's modification time, `build
//! <path>` followed by further `<path>` lines and an empty line rebuilds after those files
//! changed, `stats` answers what the session holds (`print_stats`), `quit` ends the session.
//! The language server's builds carry an identity, `build #<n>` before the first path, which the
//! answer echoes as `"build":<n>`; `cancel <n>` stops that build at its commit point, before
//! anything the session keeps has changed, and it answers `{"cancelled":true,"build":<n>}`, its
//! files carried to the next build; past that point the build runs to its answer. Under
//! `--wait-for-build` the first build waits for the first `build` command and types the texts
//! handed in before it.
//!
//! A retype leaves the records it replaces dead where they are, so a session of body edits
//! grows. When the session's memory by the allocator's account (`account`) has grown by
//! `GROWN` of what it was when the last full build was answered, the next build that is asked
//! for takes the full path, whether or not anything changed, which frees them, and answers as
//! a full build does, with `"fallback":"the session's memory"`. `TEQ_COMPACT_EVERY=<n>` makes
//! every n-th request for a build take it for that reason, whatever the request brings, for the
//! tests of what such a build has to keep.
//! Every build answers with one JSON line on stdout:
//! `{"ok":true,"changed":[<modules written>],"modules":<count>,"ms":{...},"incremental":<bool>,
//! "fallback":"<why the build was full>","retyped":[<paths>],"warnings":[...]}` or
//! `{"ok":false,"errors":[{"file","line","col","message","source","caret"}],...}`. A build that
//! fails leaves the output directory as it was.
//!
//! `text <path> <bytes>` followed by exactly that many bytes hands the session the current text
//! of a file, which stands in for the file on disk (an editor's unsaved document) until `text
//! <path> 0` withdraws it (`empty <path>` hands in an empty text); the command answers nothing,
//! the next `build` uses the text. A text with a syntax error is applied as any other: the
//! parser's recovered tree is typed, the lexical and
//! parse diagnostics are kept with the file's text and tree until the file is parsed again or
//! removed, every answer carries them, and a build with one writes nothing.
//!
//! `teq compiler watch --check <inputs>` keeps the files and the typed program the same way but stops
//! after typing: nothing is written, no entry point is needed or chosen (several `@main`
//! methods are no error), and a program for the JVM (`--target jvm --std=scala-library`) is
//! accepted. Its answers carry the fields above and
//! `"diagnostics":[{"file","line","col","endLine","endCol","severity","message","source",
//! "caret"}]`, every error and warning of the whole program, the syntax errors among them, in
//! file and position order (an unused import with `"tags":[1]`, and in a session with `--index`
//! outside `--wunused` a `"hint"`, which no count reads); lines and columns are 1-based, the end
//! exclusive, and the columns of a check session count UTF-16 units, as scalac's and an
//! editor's do. `ok` is false when there is an error (or a warning under `--werror`); a
//! diagnostic without a position has no `line`. The typed program survives a build with errors,
//! so that the edit fixing them is typed incrementally.
//!
//! An edit that changed only bodies (`shape`) has its file's bodies typed again against the
//! symbol table of the last build (`typer::incremental`); anything else, a file added or removed
//! and a body whose inferred type came out different take the full path, where the unchanged
//! files keep their ASTs and everything from the symbol table onwards is built anew. An
//! inferred signature that came out different is decided by its own definition. Where
//! inferring it reported an error, the type may be nonsense: the old signature is put back (the
//! rest of the program was typed against it) and the file stays pending, typed again from its
//! current text with every following build until its inference reports none, which settles it
//! through the comparison and, when the type did change, the full path; the other files of the
//! same build, typed against the signature before it was put back, are pending with it. Where
//! inferring it reported no error, the type is what the rest of the program has to be typed
//! against, and the build takes the full path at once, whatever errors stand elsewhere, in the
//! same file or another: they may be the ones the new type causes or repairs, the errors a
//! retype keeps for a file among them (those of what it does not resolve again: a signature
//! written out, an import), and the full path, which has its own diagnostics, is what tells.
//!
//! `--index` (with `--check`) keeps the language server's navigation index and answers its
//! queries, one JSON line each (`crate::index`, `typer::index`, docs/TARGETS.md "The language
//! server"); its diagnostics carry `uri` and `range` counted as `--positions` says.
//!
//! Known limits: an entry's canonical path is resolved when the file is first read, so a file
//! replaced by a symbolic link during the session is still known by its old path; in a plain
//! `watch`, a build that changed nothing after a build with type errors answers `ok` with the
//! modules of the last successful build (a `--check` session answers the current diagnostics);
//! and a file of the inputs that vanished is kept with its last text on the full path (the read
//! there is best effort), until a build names it and finds it gone.

use crate::ast::{Ast, Asts};
use crate::frontend;
use crate::intern::{FxMap, Interner};
use crate::source::{locate, Diagnostic, Diagnostics, FileId, SourceFile, Sources};
use crate::typer::incremental::Change;
use crate::typer::{Typer, Worker};
use crate::{emit, jvm, shape, write, Options};
use std::io::{BufRead, Write as _};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

enum Command {
    /// The files named (none: every file whose modification time moved) and the identity the
    /// language server gave the build (`build #<n>`), which its answer carries.
    Build(Option<Vec<String>>, Option<u64>),
    /// `text <path> <bytes>`: the text, or `None` for `text <path> 0`.
    Text(String, Option<String>),
    /// A query of the language server's (`--index`), answered with one JSON line.
    Query(crate::index::Query),
    /// `stats`: what the session holds, answered with one JSON line.
    Stats,
    /// `cancel <n>`: the build of that identity stops if it has not begun typing; answered by
    /// that build alone, and ignored when no such build is under way.
    Cancel(u64),
    /// A line the session does not know, answered in its turn.
    Unknown(String),
    /// An empty line, which answers nothing.
    Blank,
    Quit,
}

/// The session's commands: stdin through a buffer of the session's own, larger than stdin's,
/// which a read of its size bypasses, so that a request read ahead stands in it alone
/// (`request_waiting`); the commands a build's commit point read ahead of their turn, looking for
/// its cancel (`cancelled`), which are taken first, in order; and the bytes it read after them,
/// the start of a command not all there yet, which are read before stdin's (`Input` reads as one
/// stream, `pending` then stdin).
struct Input {
    stdin: std::io::BufReader<std::io::Stdin>,
    ahead: std::collections::VecDeque<Command>,
    pending: Vec<u8>,
}

impl std::io::Read for Input {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let n = {
            let available = std::io::BufRead::fill_buf(self)?;
            let n = available.len().min(out.len());
            out[..n].copy_from_slice(&available[..n]);
            n
        };
        std::io::BufRead::consume(self, n);
        Ok(n)
    }
}

impl BufRead for Input {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if self.pending.is_empty() {
            self.stdin.fill_buf()
        } else {
            Ok(&self.pending)
        }
    }

    fn consume(&mut self, n: usize) {
        if self.pending.is_empty() {
            self.stdin.consume(n)
        } else {
            self.pending.drain(..n);
        }
    }
}

/// The bytes a build's commit point has read, as a command reader that ends where they do and
/// says whether a command reached that end (`hit_end`): such a command is not all there yet.
struct Arrived<'a> {
    bytes: &'a [u8],
    at: usize,
    hit_end: bool,
}

impl std::io::Read for Arrived<'_> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let n = {
            let available = self.fill_buf()?;
            let n = available.len().min(out.len());
            out[..n].copy_from_slice(&available[..n]);
            n
        };
        self.consume(n);
        Ok(n)
    }
}

impl BufRead for Arrived<'_> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if self.at == self.bytes.len() {
            self.hit_end = true;
        }
        Ok(&self.bytes[self.at..])
    }

    fn consume(&mut self, n: usize) {
        self.at += n;
    }
}

/// How the positions of a `--index` session's answers count columns: in UTF-16 units or bytes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Positions {
    Utf16,
    Utf8,
}

/// The texts handed in with `text`, by canonical path: read in the place of the files on disk.
#[derive(Default)]
struct Overlays {
    texts: FxMap<PathBuf, String>,
    /// The canonical path of each path a text came for, so that the text is found again by the
    /// path as given once the file is gone and no longer resolves.
    keys: FxMap<PathBuf, PathBuf>,
}

impl Overlays {
    fn set(&mut self, path: &str, text: String) {
        let given = PathBuf::from(path);
        // A path that no longer resolves keeps the key its earlier text had.
        let key = crate::source::canonicalize(path).unwrap_or_else(|_| self.keys.get(&given).cloned().unwrap_or_else(|| given.clone()));
        // A path resolving anew (the file deleted and recreated elsewhere) leaves no text
        // under its earlier key.
        if let Some(previous) = self.keys.insert(given, key.clone()) {
            if previous != key {
                self.texts.remove(&previous);
            }
        }
        self.texts.insert(key, text);
    }

    fn remove(&mut self, path: &str) {
        let key = self.keys.remove(&PathBuf::from(path)).unwrap_or_else(|| canonical(path));
        self.texts.remove(&key);
    }

    fn get(&self, canonical: &PathBuf) -> Option<&String> {
        if self.texts.is_empty() {
            return None;
        }
        self.texts.get(canonical)
    }
}

fn canonical(path: &str) -> PathBuf {
    crate::source::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path))
}

/// A program file between builds: its text and, when parsed for that text, its ASTs. While a
/// build's stores hold the text, `text` is empty; `reparse` marks a text that was replaced there
/// without its AST.
struct Entry {
    path: String,
    /// The canonical path, by which the texts of `text` are found; the path itself when the
    /// file cannot be resolved.
    canonical: PathBuf,
    text: String,
    asts: Option<Vec<Ast>>,
    mtime: Option<SystemTime>,
    reparse: bool,
}

#[derive(Default)]
struct Timings {
    /// A parallel attempt that gave way before the build was typed again by one worker.
    attempt: Option<Duration>,
    read: Duration,
    parse: Duration,
    type_check: Duration,
    reach: Duration,
    emit: Duration,
    write: Duration,
    /// Under `--analysis-version`: the API graph's pickling and rendering.
    api: Duration,
    /// Under `--analysis-version 3`: the part of `api` the dependencies take.
    deps: Duration,
    total: Duration,
}

#[derive(Default)]
struct Report {
    ok: bool,
    changed: Vec<String>,
    /// The class files of a JVM build that the previous build wrote and this one has no class for.
    deleted: Vec<String>,
    modules: usize,
    ms: Timings,
    incremental: bool,
    fallback: Option<String>,
    retyped: Vec<String>,
    errors: Vec<String>,
    warnings: Vec<String>,
    /// Every diagnostic of a `--check` session or a JVM build, with its severity.
    diagnostics: Option<Vec<String>>,
    /// The analysis of a JVM build (`jvm::analysis`): the files typed again by this build, or
    /// every owned file after a full build.
    analysis: Option<Vec<String>>,
    /// Under `--analysis-version`, the API graph of the same files (`jvm::api`), when the
    /// build passed, or why it could not be stated.
    api: Option<String>,
    api_errors: Vec<String>,
    /// The program files a full build found gone.
    removed: Vec<String>,
    /// Why a retype's reach was walked from the roots and not kept (`emit::kept`).
    walked: Option<String>,
    /// Under `TEQ_SESSION_PARTS=1`, the reach's and the emit's parts (`measure::parts_json`) and
    /// the time of the reach's own function.
    parts: Option<String>,
    /// The identity the language server gave the build (`build #<n>`).
    build: Option<u64>,
}

/// What a session builds after typing.
enum Target {
    /// The split JavaScript build into the directory.
    Js(String),
    /// The class files of a JVM build into the directory, the files under `--own` only.
    Jvm(String),
    /// Nothing: the diagnostics alone.
    Check,
}

/// The class files a JVM session wrote, kept so that a build that changed nothing still puts
/// back what another tool deleted, and a hash of each as written.
#[derive(Default)]
struct JvmState {
    written: FxMap<String, (u64, std::sync::Arc<Vec<u8>>)>,
    last: Vec<jvm::Emitted>,
    /// The class files per unit, which a retype's build takes again where nothing they were made
    /// from changed (`jvm::kept`); their bytes are `last`'s.
    kept: jvm::kept::Kept,
}

/// The growth of a session's memory since its last full build at which the next build takes
/// the full path: a half, and sixteen megabytes where that is more, since a small program's
/// full build frees little.
const GROWN: (usize, usize) = (2, 16 << 20);

/// What the session holds by the allocator's account: the blocks in use and the requests
/// over 32 KB. It counts the program's stores with their capacities and what their records
/// own, and whatever else the builds since left behind. The batches the compiler thread and
/// the typing thread keep on their lists count as in use: at most 32 KB per size class each,
/// a bounded constant that walking the lists after every build would not be worth.
fn account() -> usize {
    let held = crate::alloc::held();
    held.reserved.saturating_sub(held.centre_free) + held.large
}

/// `TEQ_COMPACT_EVERY`, or zero.
fn compact_every() -> usize {
    static EVERY: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *EVERY.get_or_init(|| std::env::var("TEQ_COMPACT_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(0))
}

/// The sizes of what a session keeps, for `stats`.
struct Kept {
    program: usize,
    records: usize,
    symbols: usize,
    types: usize,
    names: usize,
    interner: usize,
    sources: usize,
    trees: usize,
    emit_cache: usize,
    written: usize,
    classes: usize,
    /// The units of a JVM session's kept class files (`jvm::kept`), whose bytes `classes` counts.
    kept_units: usize,
    index: usize,
    /// Under `--analysis-version 3`, the typer's records for the dependencies (`typer::deps`).
    deps: usize,
    /// The indexes of the stored inline bodies the session's worker keeps, one per record.
    inline_indexes: usize,
    baseline: usize,
    builds: usize,
}

/// With `--index`: how the positions of the answers count columns.
static INDEXED: std::sync::OnceLock<Positions> = std::sync::OnceLock::new();

/// With `--analysis-version`: the version every answer names.
static ANALYSIS_VERSION: std::sync::OnceLock<u32> = std::sync::OnceLock::new();

pub fn run(opts: &Options) -> ! {
    if opts.index {
        let _ = INDEXED.set(opts.positions);
    }
    if let Some(v) = opts.analysis_version {
        let _ = ANALYSIS_VERSION.set(v);
    }
    let target = if opts.check {
        Target::Check
    } else if opts.jvm {
        match opts.output.clone() {
            Some(dir) if !dir.ends_with(".jar") => Target::Jvm(dir),
            _ => {
                eprintln!("teq compiler watch --target jvm needs -o dir, a directory of class files");
                std::process::exit(2);
            }
        }
    } else {
        match opts.split.clone() {
            Some(dir) => Target::Js(dir),
            None => {
                eprintln!("teq compiler watch needs --split dir, -o dir with --target jvm, or --check");
                std::process::exit(2);
            }
        }
    };
    write::for_parent();
    #[cfg(windows)]
    relay_unpeekable_stdin();
    crate::measure::enable_parts(opts.timings || std::env::var_os("TEQ_SESSION_PARTS").is_some_and(|v| v == "1"));
    let mut jvm_state = JvmState::default();
    let mut input = Input { stdin: std::io::BufReader::with_capacity(1 << 16, std::io::stdin()), ahead: Default::default(), pending: Vec::new() };
    let mut overlays = Overlays::default();
    let mut interner = Interner::new();
    // The ASTs of the std files parsed so far, by slot: a build enters them from the start.
    let mut std_asts: Vec<Option<Vec<Ast>>> = Vec::new();
    let mut entries: Vec<Entry> = Vec::new();
    let mut written: FxMap<String, String> = FxMap::default();
    let mut emit_cache;
    // The last build's type store counts: a full build's store is sized from them, so that it
    // grows through no doubling and retires no buffer (`TypeStore::reserve`).
    let mut store_hint: Option<(usize, usize)> = None;
    let mut reason = "first build".to_string();
    let mut full_builds = 0usize;
    // A full build whose parallel attempt gave way: typed again at once by one worker, as the
    // session's next full build would be, before it is answered.
    let mut retry: Option<Retry> = None;
    // The full builds typed again in a row because a directory of products changed while they
    // read it, bounded so that an upstream published without end still gets an answer.
    let mut product_retries = 0usize;
    // Whether an attempt at the rule's count gave way: the session's later full builds are one
    // worker's, the state that gave it away being the same until the session starts again.
    let mut gave_way = false;
    // Per file (canonical path), the lexical and parse diagnostics of the program's text of it,
    // kept with its tree across builds, the full path's included: replaced where the file is
    // parsed again, and gone with the file.
    let mut syntax: Syntax = FxMap::default();
    // The identity of the build under way, which its answer carries (`build #<n>`).
    let mut build_id: Option<u64> = None;
    // The files of a build cancelled before it typed (none: every file), which the next build
    // types with its own.
    let mut carried: Option<Option<Vec<String>>> = None;
    // The language server's session builds first when told: the texts of the open documents come
    // before, so that its first build types them.
    if opts.wait_for_build {
        loop {
            match next_command(&mut input, &mut overlays, opts) {
                Some(Command::Build(requested, id)) => {
                    build_id = id;
                    crate::lsp::hooks::began(build_id, requested.is_some());
                    break;
                }
                Some(Command::Query(_)) => print_answer(&crate::lsp::json::Json::Null),
                Some(Command::Stats) => print_stats(None),
                _ => end_session(full_builds),
            }
        }
    }
    loop {
        let t_read = Instant::now();
        let t0 = retry.as_ref().map_or(t_read, |r| r.requested);
        let mut report = Report { fallback: Some(std::mem::take(&mut reason)), build: build_id, ..Report::default() };
        report.ms.attempt = retry.as_ref().map(|r| r.attempt);
        let mut diags = Diagnostics::default();
        let paths = crate::collect_program_paths(opts);
        let mut next: Vec<Entry> = Vec::with_capacity(paths.len());
        let mut previous: FxMap<String, Entry> = entries.drain(..).map(|e| (e.path.clone(), e)).collect();
        let mut unreadable = None;
        for path in &paths {
            match previous.remove(path) {
                Some(mut e) => {
                    // A text handed in since the entry was read stands in for it here too, and
                    // an entry read from a text since withdrawn or named by the request that
                    // took this path (no modification time), or whose file was modified since,
                    // is read from disk again.
                    let fresh = match overlays.get(&e.canonical) {
                        Some(text) => Some((text.clone(), None)),
                        None if e.mtime.is_none() || modified(&e.path) != e.mtime => read_file(&e.canonical, &e.path, &overlays).ok(),
                        None => None,
                    };
                    if let Some((text, mtime)) = fresh {
                        if text != e.text {
                            e.text = text;
                            e.asts = None;
                        }
                        e.mtime = mtime;
                    }
                    next.push(e)
                }
                None => {
                    let canonical = canonical(path);
                    match read_file(&canonical, path, &overlays) {
                        Ok((text, mtime)) => next.push(Entry { path: path.clone(), canonical, text, asts: None, mtime, reparse: false }),
                        Err(e) => unreadable = Some(format!("cannot read {}: {}", path, e)),
                    }
                }
            }
        }
        let mut removed: Vec<String> = previous.into_keys().collect();
        // The files the attempt found gone are the answer's, with what went since.
        if let Some(r) = &mut retry {
            removed.append(&mut r.removed);
        }
        removed.sort();
        removed.dedup();
        report.removed = removed;
        entries = next;
        let canon = canonical_index(&entries);
        report.ms.read = t_read.elapsed();
        let t_open = Instant::now();
        // A jar that cannot be opened fails the build as an unreadable input does: the session
        // answers and waits for the next build, which opens the jars again.
        let (cp, unreadable) = match unreadable {
            Some(msg) => (None, Some(msg)),
            None => match crate::try_open_jars(opts) {
                Ok(cp) => (cp, None),
                Err(msg) => (None, Some(msg)),
            },
        };
        let opening = t_open.elapsed();
        if let Some(msg) = unreadable {
            report.errors.push(json_object(&[("message", JsonValue::Str(&msg))]));
            if opts.check || opts.jvm {
                report.diagnostics = Some(vec![json_object(&[("severity", JsonValue::Str("error")), ("message", JsonValue::Str(&msg))])]);
            }
            report.ms.total = t0.elapsed();
            print_report(&report);
            loop {
                match next_command(&mut input, &mut overlays, opts) {
                    Some(Command::Build(requested, id)) => {
                        build_id = id;
                        crate::lsp::hooks::began(build_id, requested.is_some());
                        reread(&mut entries, requested, &overlays);
                        reason = "previous build failed".to_string();
                        break;
                    }
                    Some(Command::Query(_)) => print_answer(&crate::lsp::json::Json::Null),
                    Some(Command::Stats) => print_stats(None),
                    _ => end_session(full_builds),
                }
            }
            continue;
        }

        let t_parse = Instant::now();
        let std = crate::std_layout(opts, cp.as_ref());
        let n_std = std.len();
        if std_asts.len() != n_std {
            std_asts = (0..n_std).map(|_| None).collect();
        }
        let mut files: Vec<SourceFile> = std.iter().map(|s| SourceFile { path: s.index.path.to_string(), text: String::new(), is_std: true, key: s.index.path.to_string() }).collect();
        let mut slots: Vec<Option<Vec<Ast>>> = std_asts.iter_mut().map(Option::take).collect();
        let keys = crate::program_keys(opts, entries.iter().map(|e| e.path.as_str()));
        for (e, key) in entries.iter_mut().zip(keys) {
            files.push(SourceFile { path: e.path.clone(), text: std::mem::take(&mut e.text), is_std: false, key });
            slots.push(e.asts.take());
        }
        let parallel = frontend::parallel(&files);
        let program_bytes: usize = files[n_std..].iter().map(|f| f.text.len()).sum();
        let mut used = frontend::UsedNames::default();
        // The files parsed now have their syntax diagnostics replaced; a file whose tree was
        // kept keeps its own, and a file gone takes its own with it.
        let parsed_now: Vec<usize> = (n_std..files.len()).filter(|&f| slots[f].is_none()).collect();
        frontend::parse_files(&files, n_std..files.len(), &mut slots, &mut interner, &mut diags, Some(&mut used), parallel, usize::MAX, opts.syntax);
        for &f in &parsed_now {
            syntax.remove(&entries[f - n_std].canonical);
        }
        for d in diags.items.drain(..) {
            syntax.entry(entries[d.file.0 as usize - n_std].canonical.clone()).or_default().push((d.span, d.msg));
        }
        syntax.retain(|c, _| canon.contains_key(c));
        // A file parsed for an earlier build of the session stays parsed: its names are known.
        for (i, e) in entries.iter().enumerate() {
            if e.asts.is_none() && slots[n_std + i].is_some() {
                for ast in slots[n_std + i].as_ref().unwrap() {
                    frontend::note_used_names(ast, &mut used, &interner);
                }
            }
        }
        let scalajs_unlocked = opts.classpath.iter().any(|p| p.contains("_sjs1_")) || frontend::names_scalajs(&used, &interner);
        let mut selected = frontend::select_std(&std, &used, &interner, scalajs_unlocked);
        for i in 0..n_std {
            selected[i] |= slots[i].is_some();
        }
        for i in (0..n_std).filter(|&i| selected[i]) {
            files[i].text = std[i].text.to_string();
        }
        // The std's trees keep the names' spans where the index records them (`Syntax::index`).
        let std_syntax = crate::parser::Syntax { index: opts.index, ..Default::default() };
        let parsed_before: Vec<bool> = slots[..n_std].iter().map(Option::is_some).collect();
        frontend::parse_files(&files, (0..n_std).filter(|&i| selected[i]), &mut slots, &mut interner, &mut diags, None, parallel, frontend::STD_WORKERS, std_syntax);
        if opts.index {
            for (slot, _) in slots[..n_std].iter_mut().zip(&parsed_before).filter(|(_, &before)| !before) {
                slot.iter_mut().flatten().for_each(Ast::shrink_index_spans);
            }
        }
        let (mut flat, blocks_of) = frontend::lay_out(&mut files, slots, &std);
        let largest_unit = frontend::largest_unit_bytes(&files, &flat, &blocks_of);
        let (jar_files, jdk_file) = crate::jar_pseudo_files(opts, &mut files, &mut flat);
        let asts = Asts::new(flat);
        let sources = Sources::new(files);
        report.ms.parse = t_parse.elapsed();
        // A std file's path is its document's (`attach::std_document_path`), where a language
        // server's session locates the std's declarations and records.
        let std_document = |slot: usize| if opts.index { crate::typer::loader::attach::std_document_path(&sources[slot].path) } else { None };
        let query_files = crate::typer::index::Files {
            paths: (0..sources.len())
                .map(|f| {
                    let entry = if f < n_std { None } else { entry_of_file(f, n_std, entries.len(), &blocks_of) };
                    match entry {
                        Some(i) => Some(entries[i].canonical.clone()),
                        None => (0..n_std).find(|&i| i == f || blocks_of[i].contains(&f)).and_then(std_document),
                    }
                })
                .collect(),
            positions: opts.positions,
        };
        let scalajs_unlocked = scalajs_unlocked && !used.defines_scalajs;

        let mut typer: Option<Typer> = None;
        // The last build's reach, while the retypes since leave it as it was (`emit::kept`).
        let mut kept_reach: Option<emit::kept::Kept> = None;
        emit_cache = emit::Cache::default();
        let mut std_parsed: Vec<bool> = selected.clone();
        // The file each unit's classes are reported under: a `package p:` block's is its file's.
        let mut unit_file: Vec<FileId> = (0..sources.len()).map(|i| FileId(i as u32)).collect();
        for (file, blocks) in blocks_of.iter().enumerate() {
            for &b in blocks {
                unit_file[b] = FileId(file as u32);
            }
        }
        {
            let t_type = Instant::now();
            session_inventory(full_builds, "before the typer", &interner, &[]);
            let mut t = Typer::new(&asts, &sources, &mut interner);
            // A session's texts change: what the parser recovered may come and go with any edit.
            t.recovered = true;
            if let Some((types, lists)) = store_hint {
                t.types.reserve(types + types / 4, lists + lists / 4);
            }
            let (threads, automatic) = match &retry {
                Some(r) if !r.products => (1, false),
                _ => match full_build_threads(opts, full_builds, program_bytes, largest_unit) {
                    (_, true) if gave_way => (1, false),
                    counted => counted,
                },
            };
            t.threads = threads;
            #[cfg(debug_assertions)]
            if let Some(r) = &retry {
                assert_attempt_left_nothing(r);
            }
            crate::install_std(&mut t, std, &selected, &blocks_of, scalajs_unlocked);
            crate::open_classpath(opts, &mut t, cp, jar_files, jdk_file);
            if let Some(main) = &opts.main {
                t.set_main_name(main);
            }
            t.cacheable_state = opts.cacheable_state.clone();
            t.known_caches = !opts.no_known_caches;
            t.macro_state_per_worker = opts.macro_state_per_worker;
            t.dialect = opts.dialect;
            // The unused imports: warnings under the flag, else a language server's hints.
            t.unused.mode = match (opts.wunused_imports, opts.index) {
                (true, _) => crate::typer::unused::Mode::Warn,
                (false, true) => crate::typer::unused::Mode::Hint,
                (false, false) => crate::typer::unused::Mode::Off,
            };
            t.inline.max_depth = opts.max_inlines;
            t.inline.production = opts.release;
            // A JVM session writes every entry point's class, as `--all-mains` does.
            let js = matches!(target, Target::Js(_));
            t.inline.es_modules = js;
            t.inline.outlines = js && !opts.no_outline;
            t.choose_entry = js;
            if opts.jvm {
                t.set_jvm();
            }
            if !opts.classpath.is_empty() || crate::frontend::uses_quotes(&asts, sources.as_slice()) {
                t.prog.record_types = true;
            }
            if crate::typer::capture::forced() || opts.analysis_version.is_some() {
                t.enable_capture(None);
            }
            if opts.analysis_version == Some(jvm::api::WITH_DEPS) {
                t.deps = Some(Box::default());
            }
            t.profile.set_on(opts.profile);
            if opts.index {
                t.index = Some(Box::default());
                // Completion reads the type of a receiver's node (`typer::complete`); a session
                // that keeps no types otherwise keeps no sources of expressions for it.
                if !t.prog.record_types {
                    t.prog.record_types = true;
                    t.prog.types_only = true;
                }
            }
            #[cfg(debug_assertions)]
            let numbers_taken = crate::typer::thread::run(crate::shared::numbers_taken_by_others);
            t.run();
            if crate::measure::inventory_on() {
                let (bodies, body_bytes) = asts.bodies();
                let std_parsed = t.std.slots.iter().filter(|s| s.parsed()).count();
                let items = [
                    ("threads", t.threads),
                    ("gave way", t.serial.is_needed() as usize),
                    ("program bytes", t.prog.held()),
                    ("symbols bytes", t.syms.held()),
                    ("types bytes", t.types.held()),
                    ("index bytes", t.index.as_ref().map_or(0, |index| index.held())),
                    ("trees bytes", asts.held()),
                    ("library bodies", bodies),
                    ("library bodies bytes", body_bytes),
                    ("std selected", selected.iter().filter(|&&s| s).count()),
                    ("std parsed", std_parsed),
                ];
                session_inventory(full_builds, "after the run", t.interner, &items);
            }
            if t.serial.is_needed() {
                let why = t.serial.reason().unwrap_or_default();
                crate::types::view::gave_way(&why);
                if automatic && !t.serial.of_diagnostics() {
                    let module = t.serial.undeclared_module().filter(|(name, c)| t.on_thread(|w| w.declared_by(name, *c))).map(|(name, _)| name);
                    let until = module.as_ref().map(|m| format!(", until the session restarts with {m} declared")).unwrap_or_default();
                    eprintln!("{}; later full builds of this session type with one worker{until}", crate::give_way_note(&why, module.as_deref()));
                    gave_way = true;
                }
                retry = Some(Retry {
                    requested: t0,
                    attempt: t0.elapsed(),
                    threads: t.threads,
                    removed: std::mem::take(&mut report.removed),
                    products: false,
                    #[cfg(debug_assertions)]
                    numbers_taken,
                });
            } else if let Some(dir) = (product_retries < 3).then(|| t.loaded.as_ref().and_then(|l| l.cp.products_changed()).map(str::to_string)).flatten() {
                // A directory of products another build published into while this one read it:
                // typed again at once from what it holds now, before it is answered.
                product_retries += 1;
                report.fallback = Some(format!("the products of {} changed", dir));
                retry = Some(Retry {
                    requested: t0,
                    attempt: t0.elapsed(),
                    threads: t.threads,
                    removed: std::mem::take(&mut report.removed),
                    products: true,
                    #[cfg(debug_assertions)]
                    numbers_taken,
                });
            } else {
                product_retries = 0;
                full_builds += 1;
                log_full_build(&t, full_builds - 1, retry.take().filter(|r| !r.products).map(|r| r.threads));
            }
            if retry.is_some() {
                // Not answered: the attempt's typer, trees and texts go as a compaction's do.
            } else if t.merge_failed {
                // The merge's check refused the merged program: its
                // error is the build's answer, nothing reads the program, and the next build is a
                // full one.
                report.ms.type_check = t_type.elapsed() + opening;
                check_report(&t, &sources, opts, &mut report);
            } else {
                t.on_thread(|w| w.finish_capture());
                t.flush_infos();
                store_hint = Some(t.types.len());
                report.ms.type_check = t_type.elapsed() + opening;
                if opts.profile {
                    crate::typer::thread::run(|| crate::print_profile(&mut t, report.ms.type_check, opts.profile_json.as_deref()));
                }
                present_syntax(&mut t, &syntax, &canon, n_std);
                let names = jvm_target_names(&mut t, &target);
                if finish(&mut t, &sources, &asts, &target, opts, &mut report, &mut written, &mut emit_cache, &mut jvm_state, None, &unit_file, &mut kept_reach, false, names) {
                    typer = Some(t);
                }
            }
        }
        if retry.is_none() {
            // A full build's answer waits for its reach to settle: it takes seconds, and the
            // retypes after it, which a settling walk would otherwise make wait, can keep the reach.
            if let Some(t) = typer.as_mut() {
                let settling = t.on_thread(|w| emit::kept::Kept::settle(&mut kept_reach, w, &|| false));
                note_settling(&mut report, &mut kept_reach, settling);
            }
            report.ms.total = t0.elapsed();
            print_built(&report);
            // What the interpreter made for a full build is not the retypes': at any count they
            // start from none, so that a declared cacheable object never holds a full build's state.
            crate::typer::thread::run(crate::interp::dispose);
        }
        let mut modules_written = report.modules;
        // What the session holds with nothing dead in it, and what queries demanded of the std
        // since (`Worker::demand_std_document`), which is no dead record either.
        let mut baseline = account();
        let mut builds = 0usize;

        // The entry each AST belongs to, for the files `retype` reports.
        let mut entry_of: FxMap<usize, usize> = FxMap::default();
        for i in 0..entries.len() {
            let file = n_std + i;
            entry_of.insert(file, i);
            for &b in &blocks_of[file] {
                entry_of.insert(b, i);
            }
        }
        // The entries whose inferred signatures a build with errors put back: typed again with
        // every build until one without errors settles them.
        let mut pending: Vec<usize> = Vec::new();

        // Incremental builds until one needs the full path.
        let fallback = loop {
            if retry.is_some() {
                // The full path again, at once, with the build's reason.
                break report.fallback.take();
            }
            // After a retype the settling gives way to a request, which is answered first.
            if let Some(t) = typer.as_mut() {
                let wait = std::env::var_os("TEQ_KEPT_SETTLE").is_some_and(|v| v == "wait");
                let settling = t.on_thread(|w| emit::kept::Kept::settle(&mut kept_reach, w, &|| !wait && request_waiting(&input)));
                if let emit::kept::Settling::Dropped(walks) = settling {
                    log_dropped(walks);
                }
            }
            let Some(cmd) = next_command(&mut input, &mut overlays, opts) else { break None };
            let requested = match cmd {
                Command::Quit => break None,
                Command::Build(paths, id) => {
                    build_id = id;
                    crate::lsp::hooks::began(build_id, paths.is_some());
                    with_carried(carried.take(), paths)
                }
                // next_command keeps the texts, cancels and unknown lines to itself
                Command::Text(..) | Command::Cancel(_) | Command::Unknown(_) | Command::Blank => continue,
                Command::Stats => {
                    let kept = typer.as_ref().map(|t| Kept {
                        program: t.prog.held(),
                        records: t.prog.records(),
                        symbols: t.syms.held(),
                        types: t.types.held(),
                        names: t.interner.len(),
                        interner: t.interner.held(),
                        sources: sources.held(),
                        trees: asts.held(),
                        emit_cache: emit_cache.held(),
                        written: written.iter().map(|(name, text)| name.capacity() + text.capacity()).sum(),
                        classes: jvm_state.last.iter().map(|c| c.name.capacity() + c.bytes.capacity()).sum(),
                        kept_units: jvm_state.kept.units(),
                        index: t.index.as_ref().map_or(0, |index| index.held()),
                        deps: t.deps.as_ref().map_or(0, |deps| deps.held()),
                        inline_indexes: t.stored_index_count(),
                        baseline,
                        builds,
                    });
                    print_stats(kept.as_ref());
                    continue;
                }
                Command::Query(q) => {
                    let demanded = |typer: &Option<Typer>| typer.as_ref().and_then(|t| t.index.as_ref()).map_or(0, |ix| ix.demanded());
                    let (before, held) = (demanded(&typer), account());
                    let (answer, declared) = match typer.as_mut() {
                        Some(t) => t.on_thread(|w| w.index_query(&q, &query_files)),
                        None => (crate::lsp::json::Json::Null, None),
                    };
                    if demanded(&typer) > before {
                        baseline += account().saturating_sub(held);
                    }
                    print_answer_declared(&answer, declared.as_ref());
                    continue;
                }
            };
            let Some(t) = typer.as_mut() else {
                let reason = match changed_files(&mut entries, &canon, requested, opts, &sources, n_std, &overlays) {
                    Ok(changed) => {
                        for (i, text, mtime) in changed {
                            sources.set_text(n_std + i, text);
                            entries[i].mtime = mtime;
                            entries[i].reparse = true;
                        }
                        "previous build failed".to_string()
                    }
                    Err(reason) => reason,
                };
                break Some(reason);
            };
            // Decided before anything is read, so that a request that changes nothing or brings
            // only a text that fails to parse takes the full path as well once it is due.
            builds += 1;
            let compact = account() > baseline + (baseline / GROWN.0).max(GROWN.1) || (compact_every() > 0 && builds % compact_every() == 0);
            let t0 = Instant::now();
            let mut report = Report { incremental: true, build: build_id, ..Report::default() };
            // What a cancel carries to the next build: the files this one was asked for.
            let asked = build_id.map(|_| requested.clone());
            // A directory of products on the class path that another build published into
            // since: the whole typing state goes, as with a full build's (docs/TARGETS.md).
            let mut changed = match changed_files(&mut entries, &canon, requested, opts, &sources, n_std, &overlays) {
                Ok(changed) => changed,
                Err(reason) => break Some(reason),
            };
            if let Some(dir) = t.loaded.as_ref().and_then(|l| l.cp.products_changed()) {
                for (i, text, mtime) in changed {
                    sources.set_text(n_std + i, text);
                    entries[i].mtime = mtime;
                    entries[i].reparse = true;
                }
                break Some(format!("the products of {} changed", dir));
            }
            for &i in &pending {
                if !changed.iter().any(|(j, _, _)| *j == i) {
                    // From the text as it stands now (a newer or malformed text is what counts).
                    let (text, mtime) = read_file(&entries[i].canonical, &entries[i].path, &overlays).unwrap_or_else(|_| (sources[n_std + i].text.clone(), entries[i].mtime));
                    changed.push((i, text, mtime));
                }
            }
            // A file whose text was handed in and since withdrawn (no modification time, no
            // text) is read from disk again with whatever build comes next.
            for i in 0..entries.len() {
                if entries[i].mtime.is_some() || overlays.get(&entries[i].canonical).is_some() || changed.iter().any(|(j, _, _)| *j == i) {
                    continue;
                }
                if let Ok((text, mtime)) = read_file(&entries[i].canonical, &entries[i].path, &overlays) {
                    if text != sources[n_std + i].text {
                        changed.push((i, text, mtime));
                    } else {
                        entries[i].mtime = mtime;
                    }
                }
            }
            report.ms.read = t0.elapsed();
            if changed.is_empty() && !compact {
                if opts.check {
                    check_report(t, &sources, opts, &mut report);
                } else if t.diags.fails(opts.werror) {
                    // The last build failed and nothing changed: the answer is its errors again.
                    report.errors = render_all(&t.diags, &diag_files(t, &sources), opts.werror);
                    if opts.jvm {
                        report.diagnostics = Some(render_check(&t.diags, &diag_files(t, &sources)));
                    }
                } else if matches!(target, Target::Js(_)) && written.is_empty() {
                    // The last build could not be written: this one writes it.
                    if finish(t, &sources, &asts, &target, opts, &mut report, &mut written, &mut emit_cache, &mut jvm_state, None, &unit_file, &mut kept_reach, true, None) {
                        modules_written = report.modules;
                    }
                } else {
                    report.ok = true;
                    report.modules = modules_written;
                    if let Target::Js(dir) = &target {
                        // What another tool deleted since the last build is put back.
                        let t_write = Instant::now();
                        match write::modules_missing(dir, &written) {
                            Ok(restored) => report.changed = restored,
                            Err(e) => {
                                report.ok = false;
                                report.errors.push(json_object(&[("message", JsonValue::Str(&format!("cannot write {}: {}", dir, e)))]));
                            }
                        }
                        report.ms.write = t_write.elapsed();
                    }
                    if let Target::Jvm(dir) = &target {
                        // What another tool deleted since the last build is put back.
                        let t_write = Instant::now();
                        match write::classes_after(dir, &jvm_state.last, &mut jvm_state.written) {
                            Ok((restored, _)) => report.changed = restored,
                            Err(e) => {
                                report.ok = false;
                                report.errors.push(json_object(&[("message", JsonValue::Str(&format!("cannot write {}: {}", dir, e)))]));
                            }
                        }
                        report.ms.write = t_write.elapsed();
                        report.warnings = render_warnings(&t.diags, &diag_files(t, &sources));
                        report.diagnostics = Some(render_check(&t.diags, &diag_files(t, &sources)));
                    }
                }
                report.ms.total = t0.elapsed();
                print_report(&report);
                continue;
            }
            let t_parse = Instant::now();
            let mut parsed: Vec<(usize, Vec<Ast>, String, Option<SystemTime>)> = Vec::new();
            // The parse diagnostics of the texts parsed, which replace the files' at the commit point.
            let mut parse_errors = Vec::new();
            for (i, text, mtime) in changed {
                let (new_asts, errors) = frontend::parse_one(&text, t.interner, opts.syntax);
                parse_errors.push((entries[i].canonical.clone(), errors));
                parsed.push((i, new_asts, text, mtime));
            }
            report.ms.parse = t_parse.elapsed();
            // The files that expanded an inline body of a changed file, and theirs in turn:
            // their bodies hold the old expansion, so they are typed again with the change,
            // on their unchanged ASTs.
            let mut expanding: Vec<usize> = Vec::new();
            {
                let mut seen: Vec<usize> = Vec::new();
                let mut queue: Vec<usize> = parsed.iter().flat_map(|(i, _, _, _)| std::iter::once(n_std + i).chain(blocks_of[n_std + i].iter().copied())).collect();
                while let Some(unit) = queue.pop() {
                    let Some(sites) = t.inline_deps.get(&FileId(unit as u32)) else { continue };
                    for site in sites.keys().map(|f| f.0 as usize) {
                        if seen.contains(&site) {
                            continue;
                        }
                        seen.push(site);
                        queue.push(site);
                        if let Some(&e) = entry_of.get(&site) {
                            if !parsed.iter().any(|(i, _, _, _)| *i == e) && !expanding.contains(&e) {
                                expanding.push(e);
                            }
                        }
                    }
                }
                expanding.sort_unstable();
            }
            let mut remaps = Vec::new();
            let mut structural = None;
            // The index moves its records by the names the comparison pairs, and so does the
            // unused-import check its marks of what is resolved once (`typer::unused`).
            // A file with a diagnostic of what is resolved once has the positions outside its
            // bodies paired as well, which the diagnostic moves by.
            let kept = |t: &Typer, file: usize| t.diags.items.iter().any(|d| !d.of_body && unit_file.get(d.file.0 as usize) == Some(&FileId(file as u32)));
            let compare = |t: &Typer, file: usize| if opts.index || t.unused.on() || kept(t, file) { shape::compare_moving } else { shape::compare };
            // A file whose definitions a macro ran, in any of its package blocks, is no file to
            // type again on its own, whether it was edited or expanded what was: the expansions
            // elsewhere hold what the macro made of the old definitions.
            let macro_ran = |t: &Typer, file: usize| std::iter::once(file).chain(blocks_of[file].iter().copied()).any(|o| t.macro_files.contains_key(&FileId(o as u32)));
            for &i in &expanding {
                let file = n_std + i;
                if macro_ran(t, file) {
                    structural = Some(format!("{}: a macro ran its definitions", sources[file].path));
                }
                for o in std::iter::once(file).chain(blocks_of[file].iter().copied()) {
                    match compare(t, file)(&asts[o], &asts[o], t.interner) {
                        Ok(remap) => remaps.push((o, remap)),
                        Err(what) => structural = Some(format!("{}: {}", sources[file].path, what)),
                    }
                }
                report.retyped.push(sources[file].path.clone());
            }
            for (i, new_asts, _, _) in &parsed {
                let file = n_std + i;
                let old: Vec<usize> = std::iter::once(file).chain(blocks_of[file].iter().copied()).collect();
                if old.len() != new_asts.len() {
                    structural = Some(format!("{}: package blocks added or removed", sources[file].path));
                    break;
                }
                if macro_ran(t, file) {
                    structural = Some(format!("{}: a macro ran its definitions", sources[file].path));
                    break;
                }
                for (&o, n) in old.iter().zip(new_asts) {
                    match compare(t, file)(&asts[o], n, t.interner) {
                        Ok(remap) => remaps.push((o, remap)),
                        Err(what) => {
                            structural = Some(format!("{}: {}", sources[file].path, what));
                            break;
                        }
                    }
                }
                if structural.is_some() {
                    break;
                }
            }
            // The commit point: nothing the session keeps has changed yet (the names the parse
            // interned aside). A cancel of this build that came since it began stops it here,
            // answered as cancelled, its files carried to the next build, which types them with
            // its own; past this point the build runs to its answer, since a retype that stopped
            // halfway would leave no program to type the next one against.
            crate::lsp::hooks::hold("commit");
            if cancelled(&mut input, build_id, opts) {
                carried = asked;
                print_cancelled(build_id);
                continue;
            }
            for (canonical, errors) in parse_errors {
                if errors.is_empty() {
                    syntax.remove(&canonical);
                } else {
                    syntax.insert(canonical, errors);
                }
            }
            // The units' walks before the retype read their definitions in the old trees.
            let t_compare = Instant::now();
            if let (Some(k), None, false) = (kept_reach.as_mut(), &structural, compact) {
                let units: Vec<FileId> = remaps.iter().map(|(o, _)| FileId(*o as u32)).collect();
                t.on_thread(|w| k.retyping(w, &units));
            }
            let mut compared = t_compare.elapsed();
            // The new ASTs go in either way: a full build starts from them. A file whose package
            // blocks were added or removed has units that no longer pair with its new trees: the
            // full path parses its text again.
            for (i, new_asts, text, mtime) in parsed {
                let file = n_std + i;
                let ids: Vec<usize> = std::iter::once(file).chain(blocks_of[file].iter().copied()).collect();
                if ids.len() == new_asts.len() {
                    for (id, ast) in ids.iter().zip(new_asts) {
                        asts.replace(*id, ast);
                    }
                } else {
                    entries[i].reparse = true;
                }
                for &id in &ids {
                    sources.set_text(id, text.clone());
                }
                entries[i].mtime = mtime;
                report.retyped.push(sources[file].path.clone());
            }
            if let Some(reason) = structural.or_else(|| compact.then(|| "the session's memory".to_string())) {
                break Some(reason);
            }
            let changes: Vec<Change> = remaps.iter().map(|(o, remap)| Change { file: FileId(*o as u32), remap }).collect();
            let t_type = Instant::now();
            match t.retype(&changes) {
                Ok(restored) => {
                    store_hint = Some(t.types.len());
                    let mut restored: Vec<usize> = restored.iter().filter_map(|f| entry_of.get(&(f.0 as usize)).copied()).collect();
                    restored.sort_unstable();
                    restored.dedup();
                    // The other files of the build were typed against the signatures put back,
                    // so they are typed again with the pending ones.
                    pending = restored;
                    if !pending.is_empty() {
                        pending.extend(changes.iter().filter_map(|c| entry_of.get(&(c.file.0 as usize)).copied()));
                    }
                    pending.sort_unstable();
                    pending.dedup();
                }
                Err(reason) => break Some(reason),
            }
            // A directory of products another build published into while the retype read it.
            if let Some(dir) = t.loaded.as_ref().and_then(|l| l.cp.products_changed()) {
                break Some(format!("the products of {} changed", dir));
            }
            report.ms.type_check = t_type.elapsed();
            if opts.profile {
                crate::typer::thread::run(|| crate::print_profile(t, report.ms.type_check, opts.profile_json.as_deref()));
            }
            present_syntax(t, &syntax, &canon, n_std);
            // Before the kept reach is compared and covered, as before a walk.
            let names = jvm_target_names(t, &target);
            let t_compare = Instant::now();
            let reuse = match kept_reach.as_mut() {
                Some(k) => match t.on_thread(|w| k.retyped(w)) {
                    Ok(()) => true,
                    Err(why) => {
                        report.walked = Some(why);
                        false
                    }
                },
                None => false,
            };
            compared += t_compare.elapsed();
            let mut scope: Vec<FileId> = changes.iter().map(|c| unit_file[c.file.0 as usize]).collect();
            scope.sort_unstable();
            scope.dedup();
            let reached = finish(t, &sources, &asts, &target, opts, &mut report, &mut written, &mut emit_cache, &mut jvm_state, Some(&scope), &unit_file, &mut kept_reach, reuse, names);
            if reached {
                modules_written = report.modules;
            }
            if let (Some(path), true, false) = (std::env::var_os("TEQ_KEPT_LOG"), reached, opts.check) {
                append_session_line(&path, &if reuse { "kept".to_string() } else { format!("walked {}", report.walked.as_deref().unwrap_or("with no reach kept")) });
            }
            report.ms.reach += compared;
            let settling = kept_reach.as_mut().and_then(|k| k.settling.take());
            note_kept(&mut report, reuse, compared, settling);
            report.ms.total = t0.elapsed();
            print_built(&report);
        };
        if let Some(t) = &typer {
            for (i, slot) in t.std.slots.iter().enumerate() {
                std_parsed[i] = slot.parsed();
            }
        }
        // A session that ends leaves its program as it is.
        let Some(next_reason) = fallback else { end_session(full_builds) };
        drop(typer);
        // The program's blocks are the typing thread's to take for the next.
        crate::alloc::settle();
        reason = next_reason;

        // Take the ASTs and texts back for the next full build; a file that failed to parse or
        // whose text moved on without its AST is parsed again. A std file parsed for this build,
        // with the program or while typing, keeps its AST.
        let mut flat: Vec<Option<Ast>> = asts.into_vec().into_iter().map(Some).collect();
        let mut files = sources.into_vec();
        let group = |flat: &mut Vec<Option<Ast>>, i: usize| -> Vec<Ast> {
            std::iter::once(i).chain(blocks_of[i].iter().copied()).map(|id| flat[id].take().expect("laid out once")).collect()
        };
        std_asts = (0..n_std).map(|i| std_parsed[i].then(|| group(&mut flat, i))).collect();
        for (i, e) in entries.iter_mut().enumerate() {
            let file = n_std + i;
            let asts = group(&mut flat, file);
            e.text = std::mem::take(&mut files[file].text);
            e.asts = (!e.reparse).then_some(asts);
            e.reparse = false;
        }
    }
}

/// What follows the type check: the split build into its directory, the JVM build into its
/// (with the analysis of `scope`, or of every owned file), or under `--check` the report of
/// the diagnostics, after which the typed program is kept whether or not it has errors.
#[allow(clippy::too_many_arguments)]
fn finish(
    t: &mut Typer,
    sources: &Sources,
    asts: &Asts,
    target: &Target,
    opts: &Options,
    report: &mut Report,
    written: &mut FxMap<String, String>,
    cache: &mut emit::Cache,
    jvm_state: &mut JvmState,
    scope: Option<&[FileId]>,
    unit_file: &[FileId],
    kept: &mut Option<emit::kept::Kept>,
    reuse: bool,
    target_names: Option<FxMap<crate::types::SymId, crate::intern::Name>>,
) -> bool {
    let reuse = reuse && emit::kept::wanted() && kept.as_ref().is_some_and(|k| k.current());
    let done = match target {
        Target::Js(dir) => finish_build(t, sources, dir, opts, report, written, cache, kept, reuse),
        Target::Jvm(dir) => finish_jvm(t, sources, asts, dir, opts, report, jvm_state, scope, unit_file, kept, reuse, target_names),
        Target::Check => {
            check_report(t, sources, opts, report);
            // Under `--own`, the classes of the owned files typed anew, for a build tool's
            // discovery, as a JVM session answers them; under `--analysis-version` without it,
            // those of every file of the program.
            let all_sources = t.all_sources();
            let owned = crate::owned_files(opts, &all_sources);
            if owned.is_some() || opts.analysis_version.is_some() {
                let owned_file = |f: &FileId| !sources[f.0 as usize].is_std && owned.as_ref().is_none_or(|o| o[f.0 as usize]);
                let files: Vec<FileId> = match scope {
                    Some(files) => files.iter().copied().filter(owned_file).collect(),
                    None => (0..sources.len()).map(|i| FileId(i as u32)).filter(owned_file).collect(),
                };
                let unit_file = unit_file.to_vec();
                let mark = t.diags.items.len();
                report.analysis = Some(crate::typer::thread::run(|| jvm::analysis::render_typed(t, &files, &unit_file)));
                add_api(t, opts, report, &files, &unit_file);
                t.diags.items.truncate(mark);
            }
            true
        }
    };
    if !emit::kept::wanted() {
        *kept = None;
    }
    done
}

/// Under `--analysis-version`, the API graph of the files the answer's analysis covers, once the
/// build has passed; a definition the pickler cannot write leaves the graph out, with why.
fn add_api(t: &mut Worker, opts: &Options, report: &mut Report, files: &[FileId], unit_file: &[FileId]) {
    if opts.analysis_version.is_none() || !report.ok {
        return;
    }
    let root = crate::sourceroot(opts);
    let start = Instant::now();
    let version = opts.analysis_version.unwrap_or(2);
    match crate::typer::thread::run(|| jvm::api::render(t, files, unit_file, None, &root, version)) {
        Ok(api) => report.api = Some(api),
        Err(errors) => report.api_errors = errors,
    }
    report.ms.api = start.elapsed();
    if version >= 3 {
        report.ms.deps = Duration::from_nanos(crate::typer::deps::RENDER_NANOS.load(std::sync::atomic::Ordering::Relaxed));
    }
}

/// Reach, the class files and their writing after a successful type check of a JVM session;
/// false when errors stop the build, the output then left as it was. The class files whose
/// bytes changed are rewritten and the ones of classes gone deleted (`write::classes_after`).
#[allow(clippy::too_many_arguments)]
fn finish_jvm(
    t: &mut Worker,
    sources: &Sources,
    asts: &Asts,
    dir: &str,
    opts: &Options,
    report: &mut Report,
    state: &mut JvmState,
    scope: Option<&[FileId]>,
    unit_file: &[FileId],
    kept: &mut Option<emit::kept::Kept>,
    reuse: bool,
    target_names: Option<FxMap<crate::types::SymId, crate::intern::Name>>,
) -> bool {
    // The files a retype typed again are emitted again with the next build that emits, whatever
    // stops this one; a full build keeps nothing.
    match scope {
        Some(files) if jvm::kept::wanted() => state.kept.retyped((0..unit_file.len()).filter(|&i| files.contains(&unit_file[i])).map(|i| FileId(i as u32))),
        _ => state.kept.clear(),
    }
    let failed = t.diags.fails(opts.werror);
    if failed {
        report.errors = render_all(&t.diags, &diag_files(t, sources), opts.werror);
        report.diagnostics = Some(render_check(&t.diags, &diag_files(t, sources)));
        report.warnings = Vec::new();
        return false;
    }
    // Resolved before the reach (`jvm_target_names`); the reach's tables cover what it entered.
    let source_target_names = target_names.unwrap_or_else(|| crate::typer::thread::run(|| t.source_target_names()));
    let t_reach = Instant::now();
    let array_seq = crate::typer::thread::run(|| t.array_seq_class());
    // A kept reach finds the jar classes the retype entered and that the JVM erases incomplete,
    // as the walk completes them at its end.
    if reuse {
        crate::typer::thread::run(|| t.complete_erased_jar_classes());
    }
    let reach = session_reach(t, array_seq, false, kept, reuse, report);
    report.ms.reach = t_reach.elapsed();
    if t.diags.has_errors() {
        report.errors = render_all(&t.diags, &diag_files(t, sources), false);
        report.diagnostics = Some(render_check(&t.diags, &diag_files(t, sources)));
        return false;
    }
    let t_emit = Instant::now();
    let all_sources = t.all_sources();
    let owned = crate::owned_files(opts, &all_sources);
    let stackable = crate::stackable_overrides(t, reach);
    let product_inits = t.loaded.as_ref().map(|l| l.get_mut().cp.product_inits.clone()).unwrap_or_default();
    let before_emit = t_emit.elapsed();
    let output = crate::typer::thread::run(|| {
        let Worker { prog, syms, types, tvars, interner, file_pkgs, loaded, entry_points, jvm_volatile, .. } = &mut *t;
        // A JVM build always reads the class path: scala-library is on it.
        let crate::typer::loader::Loaded { cp, classes, java, target_names, product_target_names, volatile, .. } = loaded.as_ref().expect("a JVM build reads its class path").get_mut();
        let input = jvm::Input {
            prog,
            syms,
            types,
            insts: &*tvars,
            interner: &**interner,
            reach,
            array_seq,
            sources: &all_sources,
            file_pkgs: file_pkgs.own(),
            asts,
            java: Some(&*java),
            target_names: Some(&*target_names),
            source_target_names: Some(&source_target_names),
            product_target_names: Some(&*product_target_names),
            volatile: Some(&*volatile),
            source_volatile: Some(&*jvm_volatile),
            link: jvm::Link { jar_classes: &*classes },
            owned: owned.as_deref(),
            all_mains: Some(entry_points.as_slice()),
            output_version: opts.java_output_version,
            open_world: false,
            mixin_supers: None,
            stackable: Some(&stackable),
            product_inits: Some(&product_inits),
        };
        jvm::emit(input, cp, jvm::kept::wanted().then_some(&mut state.kept))
    });
    report.ms.emit = t_emit.elapsed();
    crate::measure::part(crate::measure::Pass::Emit, "sources, overrides and inits", 0, before_emit);
    if let (Some(path), Some(_)) = (std::env::var_os("TEQ_KEPT_LOG"), scope) {
        let u = &output.kept;
        append_session_line(&path, &match u.why {
            Some(why) => format!("emit {}", why),
            None => format!("emit kept {} of {}", u.kept, u.kept + u.emitted),
        });
    }
    note_parts(report, Duration::ZERO, true);
    if !output.errors.is_empty() {
        report.errors = output.errors.iter().map(|e| json_object(&[("message", JsonValue::Str(&format!("jvm backend: {}", e)))])).collect();
        report.diagnostics = Some(output.errors.iter().map(|e| json_object(&[("severity", JsonValue::Str("error")), ("message", JsonValue::Str(&format!("jvm backend: {}", e)))])).collect());
        return false;
    }
    let t_write = Instant::now();
    crate::measure::Lap::begin(crate::measure::Pass::After);
    match write::classes_after(dir, &output.classes, &mut state.written) {
        Ok((changed, deleted)) => {
            report.changed = changed;
            report.deleted = deleted;
            report.modules = output.classes.len();
            report.ms.write = t_write.elapsed();
            report.ok = true;
            report.warnings = render_warnings(&t.diags, &diag_files(t, sources));
            report.diagnostics = Some(render_check(&t.diags, &diag_files(t, sources)));
            let owned_file = |f: &FileId| !sources[f.0 as usize].is_std && owned.as_ref().map_or(true, |o| o[f.0 as usize]);
            let files: Vec<FileId> = match scope {
                Some(files) => files.iter().copied().filter(owned_file).collect(),
                None => (0..sources.len()).map(|i| FileId(i as u32)).filter(owned_file).collect(),
            };
            let unit_file = unit_file.to_vec();
            let mut lap = crate::measure::Lap::resume(crate::measure::Pass::After);
            // The renderings resolve names on trial; what they report is not the build's.
            let mark = t.diags.items.len();
            report.analysis = Some(crate::typer::thread::run(|| jvm::analysis::render(t, &output, &files, &unit_file)));
            lap.done("analysis", files.len());
            add_api(t, opts, report, &files, &unit_file);
            t.diags.items.truncate(mark);
            lap.done("api", 0);
            state.last = output.classes;
            lap.done("output kept", 0);
            note_after(report);
            true
        }
        Err(e) => {
            state.written.clear();
            let msg = format!("cannot write {}: {}", dir, e);
            report.errors.push(json_object(&[("message", JsonValue::Str(&msg))]));
            false
        }
    }
}

/// The workers of a session's full build (the first, a rebuild, a compaction, a failed retype's
/// fallback), from the program's texts as they are at that build: a count asked for (`--threads`,
/// `TEQ_THREADS`), else the suites' switches, `TEQ_SESSION_WORKERS=<n>` for every full build
/// (tests/watch-memory.sh's rows) and `TEQ_SESSION_THREADS=<n>` for the first (tests/lsp.sh: the
/// navigation index of several workers' records), else the one-shot builds' rule
/// (`frontend::automatic_threads`). A retype stays one worker's.
/// Whether the rule chose the count is the second value: a build whose attempt gives way then
/// says so (`give_way_note`), once, the session's later full builds at one worker.
fn full_build_threads(opts: &Options, full_builds: usize, program_bytes: usize, largest_unit: usize) -> (usize, bool) {
    if let Some(n) = frontend::requested_threads(opts.threads) {
        return (n, false);
    }
    let asked = |var: &str| std::env::var(var).ok().and_then(|v| v.parse::<usize>().ok()).filter(|&n| n >= 1);
    match (asked("TEQ_SESSION_WORKERS"), asked("TEQ_SESSION_THREADS")) {
        (Some(n), _) => (n, false),
        (None, Some(n)) if full_builds == 0 => (n, false),
        _ => (frontend::automatic_threads(program_bytes, largest_unit), true),
    }
}

/// A full build whose parallel attempt gave way: the request it answers, and the attempt's time
/// and workers.
struct Retry {
    requested: Instant,
    attempt: Duration,
    threads: usize,
    /// The program files the attempt's build found gone, which the answer names.
    removed: Vec<String>,
    /// Whether a directory of products changed under the attempt, which then keeps its workers.
    products: bool,
    /// The threads' numbers other threads than the typing thread took before the attempt, which
    /// its workers give back as they end.
    #[cfg(debug_assertions)]
    numbers_taken: usize,
}

/// What a retry's argument rests on: the attempt's workers have
/// ended and given their numbers back, and the typing thread holds no lock, no completion staged
/// for a store that is gone and no overlay of one.
#[cfg(debug_assertions)]
fn assert_attempt_left_nothing(r: &Retry) {
    let (numbers, depth, staged, overlay) = crate::typer::thread::run(|| (crate::shared::numbers_taken_by_others(), crate::shared::lock_depth(), crate::shared::staged_completions(), crate::types::in_overlay()));
    assert_eq!(numbers, r.numbers_taken, "the attempt's workers kept their threads' numbers");
    assert_eq!((depth, staged, overlay), (0, 0, false), "the attempt left the loader's lock held, completions staged or an overlay entered");
}

/// `TEQ_SESSION_WORKERS_LOG=<file>`: a line per full build the session answers, appended, `<pid>
/// <parent's pid> <k> <how>` for its k-th (the first is 0), saying how it was typed: `joined <n>`
/// where `n` workers typed it through the fork and the merge took their work (one under
/// `TEQ_FORK=1`), `retried <n>` where the attempt of `n` gave way and one worker typed it again,
/// `refused <n>` (the merge's check), `one` at one worker without the fork; and as the session
/// ends, `<pid> <parent's pid> end <full builds>`, its own count of the full builds it answered,
/// which says that no build is missing. A child the language server starts inherits the setting.
fn log_full_build(t: &Typer, k: usize, retried: Option<usize>) {
    let Some(path) = std::env::var_os("TEQ_SESSION_WORKERS_LOG") else { return };
    let forked = t.threads > 1 || std::env::var_os("TEQ_FORK").is_some_and(|v| v == "1");
    let how = if let Some(n) = retried {
        format!("retried {}", n)
    } else if !forked {
        "one".to_string()
    } else if t.merge_failed {
        format!("refused {}", t.threads)
    } else {
        format!("joined {}", t.threads)
    };
    append_session_line(&path, &format!("{} {}", k, how));
}

/// A line of `TEQ_SESSION_INVENTORY` for the session's k-th full build: what the allocator holds,
/// the names, and what the typing thread keeps (the interpreter's state, the threads' numbers,
/// the cell directories, the overlay), with `items` of the caller's.
fn session_inventory(k: usize, what: &str, interner: &Interner, items: &[(&str, usize)]) {
    if !crate::measure::inventory_on() {
        return;
    }
    crate::alloc::settle();
    let mut line: Vec<(&str, usize)> = vec![("account", account()), ("footprint", crate::alloc::footprint()), ("names", interner.len()), ("names bytes", interner.held())];
    line.extend_from_slice(items);
    line.extend(crate::typer::thread::run(|| {
        let mut kept = crate::interp::inventory();
        kept.extend(crate::shared::inventory());
        kept.extend(crate::arena::inventory());
        kept.extend(crate::types::inventory());
        kept.push(("typing thread free", crate::alloc::thread_free()));
        kept.push(("typing thread stack", crate::memory::stack_resident()));
        kept
    }));
    crate::measure::inventory_line(&format!("full build {} {}", k, what), &line);
}

/// A full build's settling in its answer's parts: the walks and their time.
fn note_settling(report: &mut Report, kept: &mut Option<emit::kept::Kept>, settling: emit::kept::Settling) {
    if let emit::kept::Settling::Dropped(walks) = settling {
        log_dropped(walks);
    }
    let Some((walks, time)) = kept.as_mut().and_then(|k| k.settling.take()) else { return };
    if crate::measure::parts_on() {
        let part = format!("\"settled\":[{},{:.3}]", walks, time.as_secs_f64() * 1000.0);
        report.parts = Some(match report.parts.take() {
            Some(parts) => format!("{},{}", parts, part),
            None => part,
        });
    }
}

fn log_dropped(walks: usize) {
    if let Some(path) = std::env::var_os("TEQ_KEPT_LOG") {
        append_session_line(&path, &format!("dropped: not settled after {} walks", walks));
    }
}

/// Whether a request waits, read ahead into `input` or not: what a settling walk gives way to.
fn request_waiting(input: &Input) -> bool {
    !input.ahead.is_empty() || !input.pending.is_empty() || stdin_waiting(&input.stdin)
}

/// Whether a command waits on stdin, read into the buffer or not.
fn stdin_waiting(commands: &std::io::BufReader<std::io::Stdin>) -> bool {
    !commands.buffer().is_empty() || stdin_readable()
}

#[cfg(unix)]
fn stdin_readable() -> bool {
    use std::os::raw::{c_int, c_short};
    #[repr(C)]
    struct PollFd {
        fd: c_int,
        events: c_short,
        revents: c_short,
    }
    #[cfg(target_os = "linux")]
    type Nfds = std::os::raw::c_ulong;
    #[cfg(not(target_os = "linux"))]
    type Nfds = std::os::raw::c_uint;
    extern "C" {
        fn poll(fds: *mut PollFd, nfds: Nfds, timeout: c_int) -> c_int;
    }
    const POLLIN: c_short = 1;
    let mut stdin = PollFd { fd: 0, events: POLLIN, revents: 0 };
    // SAFETY: one descriptor, valid for the call; a timeout of zero returns at once.
    unsafe { poll(&mut stdin, 1, 0) > 0 }
}

/// Without a poll of the handle: a pipe says what it holds (`PeekNamedPipe`), and one whose
/// writer is gone says so by failing with `ERROR_BROKEN_PIPE`, readable as `poll` finds its end;
/// a pipe that cannot be peeked holds nothing as far as the session can tell, so that no read
/// waits on it (the session's stdin is never one: `relay_unpeekable_stdin`); a console has a line
/// once an Enter is among its input events, as a terminal's `poll` in canonical mode has one
/// once the line is whole, so that the read it leads to does not wait; a file, `NUL` and a
/// handle of no kind read at once, at worst their end.
#[cfg(windows)]
fn stdin_readable() -> bool {
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;
    /// `INPUT_RECORD` with its `KEY_EVENT_RECORD`.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct InputRecord {
        kind: u16,
        key_down: i32,
        repeat: u16,
        key: u16,
        scan: u16,
        char: u16,
        control: u32,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetFileType(file: *mut c_void) -> u32;
        fn PeekNamedPipe(pipe: *mut c_void, buf: *mut c_void, size: u32, read: *mut u32, available: *mut u32, left: *mut u32) -> i32;
        fn GetNumberOfConsoleInputEvents(console: *mut c_void, events: *mut u32) -> i32;
        fn PeekConsoleInputW(console: *mut c_void, records: *mut InputRecord, len: u32, read: *mut u32) -> i32;
    }
    const FILE_TYPE_CHAR: u32 = 2;
    const FILE_TYPE_PIPE: u32 = 3;
    const ERROR_BROKEN_PIPE: i32 = 109;
    const KEY_EVENT: u16 = 1;
    const VK_RETURN: u16 = 0x0d;
    let handle = std::io::stdin().as_raw_handle();
    let mut n = 0u32;
    // SAFETY: queries of the process's stdin handle, which outlives the calls; each writes `n`
    // and at most `n` records of the buffer it is given.
    unsafe {
        match GetFileType(handle) {
            FILE_TYPE_PIPE => match PeekNamedPipe(handle, std::ptr::null_mut(), 0, std::ptr::null_mut(), &mut n, std::ptr::null_mut()) {
                0 => std::io::Error::last_os_error().raw_os_error() == Some(ERROR_BROKEN_PIPE),
                _ => n > 0,
            },
            FILE_TYPE_CHAR => {
                if GetNumberOfConsoleInputEvents(handle, &mut n) == 0 {
                    return true;
                }
                let mut records = vec![InputRecord { kind: 0, key_down: 0, repeat: 0, key: 0, scan: 0, char: 0, control: 0 }; n as usize];
                if n == 0 || PeekConsoleInputW(handle, records.as_mut_ptr(), n, &mut n) == 0 {
                    return false;
                }
                records[..n as usize].iter().any(|r| r.kind == KEY_EVENT && r.key_down != 0 && r.key == VK_RETURN)
            }
            _ => true,
        }
    }
}

/// A stdin pipe that cannot be peeked (wine's over a Unix pipe, `ERROR_NOT_SUPPORTED`; a socket
/// given as stdin) is read by a thread of its own into a pipe of this process's, which takes its
/// place as the standard input (`SetStdHandle`: std reads stdin through the standard handle at
/// every read) and which `stdin_readable` peeks, so that a cancel at the commit point is seen
/// whatever the caller's pipe; the end of the first is the end of the second. Before anything
/// reads stdin.
#[cfg(windows)]
fn relay_unpeekable_stdin() {
    use std::ffi::c_void;
    use std::ptr::null_mut;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(id: u32) -> *mut c_void;
        fn SetStdHandle(id: u32, handle: *mut c_void) -> i32;
        fn GetFileType(file: *mut c_void) -> u32;
        fn PeekNamedPipe(pipe: *mut c_void, buf: *mut c_void, size: u32, read: *mut u32, available: *mut u32, left: *mut u32) -> i32;
        fn CreatePipe(read: *mut *mut c_void, write: *mut *mut c_void, attributes: *mut c_void, size: u32) -> i32;
        fn ReadFile(file: *mut c_void, buf: *mut u8, len: u32, read: *mut u32, overlapped: *mut c_void) -> i32;
        fn WriteFile(file: *mut c_void, buf: *const u8, len: u32, written: *mut u32, overlapped: *mut c_void) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    const STD_INPUT_HANDLE: u32 = -10i32 as u32;
    const FILE_TYPE_PIPE: u32 = 3;
    const ERROR_BROKEN_PIPE: i32 = 109;
    // SAFETY: the process's stdin handle queried; a pipe made, its read end the standard input from
    // here, its write end and the first handle the relay's alone, each buffer the call's length.
    unsafe {
        let original = GetStdHandle(STD_INPUT_HANDLE);
        let mut n = 0u32;
        if GetFileType(original) != FILE_TYPE_PIPE || PeekNamedPipe(original, null_mut(), 0, null_mut(), &mut n, null_mut()) != 0 || std::io::Error::last_os_error().raw_os_error() == Some(ERROR_BROKEN_PIPE) {
            return;
        }
        let (mut read, mut write) = (null_mut(), null_mut());
        if CreatePipe(&mut read, &mut write, null_mut(), 1 << 16) == 0 {
            return;
        }
        let (from, to) = (original as usize, write as usize);
        crate::alloc::spawn(move || {
            let mut buf = vec![0u8; 1 << 16];
            'relay: loop {
                let mut got = 0u32;
                if ReadFile(from as *mut c_void, buf.as_mut_ptr(), buf.len() as u32, &mut got, null_mut()) == 0 || got == 0 {
                    break;
                }
                let mut at = 0;
                while at < got {
                    let mut put = 0u32;
                    if WriteFile(to as *mut c_void, buf[at as usize..].as_ptr(), got - at, &mut put, null_mut()) == 0 {
                        break 'relay;
                    }
                    at += put;
                }
            }
            CloseHandle(to as *mut c_void);
        });
        SetStdHandle(STD_INPUT_HANDLE, read);
    }
}

fn append_session_line(path: &std::ffi::OsStr, what: &str) {
    let line = format!("{} {} {}\n", std::process::id(), crate::write::parent(), what);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// The session ends, its count of full builds logged (`log_full_build`).
fn end_session(full_builds: usize) -> ! {
    if let Some(path) = std::env::var_os("TEQ_SESSION_WORKERS_LOG") {
        append_session_line(&path, &format!("end {}", full_builds));
    }
    std::process::exit(0)
}

/// The answer of a `--check` session: every diagnostic with its severity under `diagnostics`,
/// the errors and warnings as the build answers carry them, and `ok` when nothing is an error.
fn check_report(t: &Typer, sources: &Sources, opts: &Options, report: &mut Report) {
    let files = diag_files(t, sources);
    let failed = t.diags.fails(opts.werror);
    report.ok = !failed;
    report.errors = if failed { render_all(&t.diags, &files, opts.werror) } else { Vec::new() };
    report.warnings = if opts.werror { Vec::new() } else { render_warnings(&t.diags, &files) };
    report.diagnostics = Some(render_check(&t.diags, &files));
}

/// The reach's parts once it has walked (`compute` its function's time), or the emit's.
fn note_parts(report: &mut Report, compute: Duration, emit: bool) {
    if !crate::measure::parts_on() {
        return;
    }
    let part = match emit {
        false => format!("\"reach\":{},\"compute\":{:.3}", crate::measure::parts_json(crate::measure::Pass::Reach), compute.as_secs_f64() * 1000.0),
        true => format!("\"emit\":{}", crate::measure::parts_json(crate::measure::Pass::Emit)),
    };
    report.parts = Some(match report.parts.take() {
        Some(parts) => format!("{},{}", parts, part),
        None => part,
    });
}

/// The parts of the write and of what follows it.
fn note_after(report: &mut Report) {
    if !crate::measure::parts_on() {
        return;
    }
    let part = format!("\"after\":{}", crate::measure::parts_json(crate::measure::Pass::After));
    report.parts = Some(match report.parts.take() {
        Some(parts) => format!("{},{}", parts, part),
        None => part,
    });
}

/// Whether a retype's reach was kept, with the time of the comparison of its units' walks, or
/// why it was walked from the roots.
fn note_kept(report: &mut Report, kept: bool, compared: Duration, settling: Option<(usize, Duration)>) {
    if !crate::measure::parts_on() {
        return;
    }
    let mut part = format!("\"kept\":{},\"compared\":{:.3}", kept, compared.as_secs_f64() * 1000.0);
    if let Some((walks, time)) = settling {
        part.push_str(&format!(",\"settled\":[{},{:.3}]", walks, time.as_secs_f64() * 1000.0));
    }
    if let Some(why) = &report.walked {
        part.push_str(",\"walked\":");
        json_string(why, &mut part);
    }
    report.parts = Some(match report.parts.take() {
        Some(parts) => format!("{},{}", parts, part),
        None => part,
    });
}

/// The JVM's method names by `@targetName` (`Worker::source_target_names`), resolved after a
/// type check and before the reach is walked or a kept one compared: resolving an annotation may
/// enter its class, which the reach's tables must cover.
fn jvm_target_names(t: &mut Typer, target: &Target) -> Option<FxMap<crate::types::SymId, crate::intern::Name>> {
    matches!(target, Target::Jvm(_)).then(|| t.on_thread(|w| w.source_target_names()))
}

/// The build's reach: the kept one when the retypes since left it as it was (`reuse`), checked
/// in the assertion-enabled build against the walk from the roots; else that walk, kept for the
/// next build.
fn session_reach<'k>(t: &mut Worker, array_seq: Option<crate::types::ClassId>, js_visible: bool, kept: &'k mut Option<emit::kept::Kept>, reuse: bool, report: &mut Report) -> &'k emit::reach::Reach {
    if reuse && kept.is_some() {
        #[cfg(debug_assertions)]
        if std::env::var_os("TEQ_KEPT_REACH_CHECK").map_or(true, |v| v != "off") {
            let k = kept.as_ref().expect("a kept reach");
            let w: &mut Worker = t;
            let differences = crate::typer::thread::run(move || k.check(w));
            assert!(differences.is_empty(), "the kept reach differs from the walk from the roots: {}", differences.join("; "));
        }
    } else {
        *kept = None;
        let t_compute = Instant::now();
        let before = emit::kept::Holdings::of(t);
        let (reach, setup) = crate::typer::thread::run(|| emit::reach::compute_with_setup(t, array_seq, js_visible));
        note_parts(report, t_compute.elapsed(), false);
        *kept = Some(emit::kept::Kept::new(t, reach, setup, &before));
    }
    &kept.as_ref().expect("a reach kept or walked").reach
}

/// Reach, emit and write after a successful type check; false when errors stop the build. The
/// modules written last are kept in `written`, so that the writer reads back only what changed.
fn finish_build(
    t: &mut Worker,
    sources: &Sources,
    dir: &str,
    opts: &Options,
    report: &mut Report,
    written: &mut FxMap<String, String>,
    cache: &mut emit::Cache,
    kept: &mut Option<emit::kept::Kept>,
    reuse: bool,
) -> bool {
    let failed = t.diags.fails(opts.werror);
    if failed {
        report.errors = render_all(&t.diags, &diag_files(t, sources), opts.werror);
        report.warnings = Vec::new();
        return false;
    }
    let t_reach = Instant::now();
    let array_seq = crate::typer::thread::run(|| t.array_seq_class());
    let reach = session_reach(t, array_seq, opts.release, kept, reuse, report);
    report.ms.reach = t_reach.elapsed();
    if t.diags.has_errors() {
        report.errors = render_all(&t.diags, &diag_files(t, sources), false);
        return false;
    }
    let t_emit = Instant::now();
    let array_seq = crate::typer::thread::run(|| t.array_seq_class());
    t.rank_files();
    let split = emit::Split { files: sources.as_slice(), file_pkgs: t.file_pkgs.own(), module_per_file: &opts.module_per_file, hot: opts.hot };
    let options = emit::Options { release: opts.release, size_report: None, outline: !opts.no_outline };
    let (output, _) = emit::emit(&t.prog, &t.syms, t.interner, array_seq, reach, Some(split), options, Some(cache));
    report.ms.emit = t_emit.elapsed();
    note_parts(report, Duration::ZERO, true);
    let emit::Output::Modules(mut modules) = output else { unreachable!() };
    let t_write = Instant::now();
    crate::measure::Lap::begin(crate::measure::Pass::After);
    match write::modules_after(dir, &mut modules, written) {
        Ok(changed) => {
            report.changed = changed;
            report.modules = modules.files.len();
            report.ms.write = t_write.elapsed();
            report.ok = true;
            report.warnings = render_warnings(&t.diags, &diag_files(t, sources));
            let mut lap = crate::measure::Lap::resume(crate::measure::Pass::After);
            *written = modules.files.into_iter().collect();
            lap.done("output kept", 0);
            note_after(report);
            true
        }
        Err(e) => {
            written.clear();
            let msg = format!("cannot write {}: {}", dir, e);
            report.errors.push(json_object(&[("message", JsonValue::Str(&msg))]));
            false
        }
    }
}

/// The lexical and parse diagnostics of the program's files, by canonical path.
type Syntax = FxMap<PathBuf, Vec<(crate::source::Span, String)>>;

/// Hands the typer's diagnostics the program's syntax diagnostics, which its answers then present
/// with its own and count as errors (`Diagnostics::fails`), and the program's syntax state, by
/// which a message about an erroneous type is presented or not (`Diagnostic::unknown`).
fn present_syntax(t: &mut Worker, syntax: &Syntax, canon: &FxMap<PathBuf, usize>, n_std: usize) {
    let mut all = Vec::new();
    for (c, errors) in syntax {
        let Some(&i) = canon.get(c) else { continue };
        let file = FileId((n_std + i) as u32);
        all.extend(errors.iter().map(|(span, msg)| Diagnostic::syntax(file, *span, msg.clone())));
    }
    all.sort_by_key(|d| (d.file.0, d.span.start));
    t.diags.syntax_errors = !all.is_empty();
    t.diags.syntax = all;
}

/// The files diagnostics point into: with a classpath, the library bodies' pseudo files too.
fn diag_files(t: &Worker, sources: &Sources) -> Vec<SourceFile> {
    if t.loaded.is_some() {
        t.all_sources()
    } else {
        sources.as_slice().iter().map(SourceFile::copy).collect()
    }
}

/// The text of a program file: the one handed in with `text`, without a modification time so
/// that a plain `build` reads it again, or the file on disk.
fn read_file(canonical: &PathBuf, path: &str, overlays: &Overlays) -> std::io::Result<(String, Option<SystemTime>)> {
    if let Some(text) = overlays.get(canonical) {
        return Ok((text.clone(), None));
    }
    let text = std::fs::read_to_string(path)?;
    Ok((text, std::fs::metadata(path).and_then(|m| m.modified()).ok()))
}

/// The program files that changed, as (index, new text, mtime): the ones asked for, or with no
/// list every file whose modification time moved. `Err` names why the full path is needed.
fn changed_files(
    entries: &mut [Entry],
    known: &FxMap<PathBuf, usize>,
    requested: Option<Vec<String>>,
    opts: &Options,
    sources: &Sources,
    n_std: usize,
    overlays: &Overlays,
) -> Result<Vec<(usize, String, Option<SystemTime>)>, String> {
    let current: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
    let indices: Vec<usize> = match requested {
        Some(paths) => {
            let mut out = Vec::new();
            let mut unknown = false;
            for p in paths {
                match crate::source::canonicalize(p).ok().and_then(|c| known.get(&c).copied()) {
                    Some(i) => out.push(i),
                    None => unknown = true,
                }
            }
            if unknown && crate::collect_program_paths(opts) != current {
                // The full path reads the files the request names again.
                for &i in &out {
                    entries[i].mtime = None;
                }
                return Err("files added or removed".to_string());
            }
            out.sort_unstable();
            out.dedup();
            out
        }
        None => {
            if crate::collect_program_paths(opts) != current {
                return Err("files added or removed".to_string());
            }
            // A file with a text handed in is read again either way: the text may have changed.
            (0..entries.len())
                .filter(|&i| overlays.get(&entries[i].canonical).is_some() || modified(&entries[i].path) != entries[i].mtime)
                .collect()
        }
    };
    let mut changed = Vec::new();
    for i in indices {
        let (text, mtime) = read_file(&entries[i].canonical, &entries[i].path, overlays).map_err(|_| "files added or removed".to_string())?;
        if text != sources[n_std + i].text {
            changed.push((i, text, mtime));
        } else {
            entries[i].mtime = mtime;
        }
    }
    Ok(changed)
}

/// After a build that could not read its inputs, where the entries hold their texts themselves:
/// takes the texts of the files named, or of every file, afresh.
fn reread(entries: &mut [Entry], requested: Option<Vec<String>>, overlays: &Overlays) {
    let known = canonical_index(entries);
    let indices: Vec<usize> = match requested {
        Some(paths) => paths.iter().filter_map(|p| crate::source::canonicalize(p).ok().and_then(|c| known.get(&c).copied())).collect(),
        None => (0..entries.len()).collect(),
    };
    for i in indices {
        if let Ok((text, mtime)) = read_file(&entries[i].canonical, &entries[i].path, overlays) {
            if text != entries[i].text {
                entries[i].text = text;
                entries[i].asts = None;
            }
            entries[i].mtime = mtime;
        }
    }
}

/// The modification time of the file at `path`, if it can be read.
fn modified(path: &str) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn canonical_index(entries: &[Entry]) -> FxMap<PathBuf, usize> {
    entries.iter().enumerate().map(|(i, e)| (e.canonical.clone(), i)).collect()
}

/// The next `build`, query or `quit`, those read ahead first; the `text` commands on the way go
/// into `overlays`, a line the session does not know is answered, and a `cancel` read here names
/// no build under way and answers nothing.
fn next_command(input: &mut Input, overlays: &mut Overlays, opts: &Options) -> Option<Command> {
    // The session goes idle: what it freed on this thread is for every thread to take.
    crate::alloc::settle();
    loop {
        let command = match input.ahead.pop_front() {
            Some(command) => command,
            None => read_command(input, opts)?,
        };
        match command {
            Command::Text(path, Some(text)) => overlays.set(&path, text),
            Command::Text(path, None) => overlays.remove(&path),
            Command::Cancel(build) => crate::lsp::hooks::trace("session", || format!("ignored-cancel {}", build)),
            Command::Blank => {}
            // Every line of a language server's session has its one answer.
            Command::Unknown(_) if opts.index => print_answer(&crate::lsp::json::Json::Null),
            Command::Unknown(line) => {
                let msg = format!("unknown command: {}", line);
                let error = json_object(&[("message", JsonValue::Str(&msg))]);
                let diagnostic = json_object(&[("severity", JsonValue::Str("error")), ("message", JsonValue::Str(&msg))]);
                let report = Report { errors: vec![error], diagnostics: opts.check.then(|| vec![diagnostic]), ..Report::default() };
                print_report(&report);
            }
            other => return Some(other),
        }
    }
}

/// Whether a `cancel` of the build `build` came since it began: the bytes on stdin are read as far
/// as they have come, never waiting for more, and the commands they hold whole are kept for their
/// turn (a cancel of another build dropped); a command not all there yet stays as bytes, read with
/// the rest of it in its turn.
fn cancelled(input: &mut Input, build: Option<u64>, opts: &Options) -> bool {
    let Some(build) = build else { return false };
    while stdin_waiting(&input.stdin) {
        let Ok(arrived) = input.stdin.fill_buf() else { break };
        let n = arrived.len();
        if n == 0 {
            break;
        }
        input.pending.extend_from_slice(arrived);
        input.stdin.consume(n);
    }
    let mut found = false;
    let mut whole = 0;
    loop {
        let mut arrived = Arrived { bytes: &input.pending[whole..], at: 0, hit_end: false };
        let command = read_command(&mut arrived, opts);
        if arrived.hit_end {
            break;
        }
        whole += arrived.at;
        match command {
            Some(Command::Cancel(n)) if n == build => found = true,
            Some(Command::Cancel(n)) => crate::lsp::hooks::trace("session", || format!("ignored-cancel {}", n)),
            Some(Command::Blank) => {}
            Some(command) => input.ahead.push_back(command),
            None => break,
        }
    }
    input.pending.drain(..whole);
    found
}

/// The identity `#<n>` the language server puts at the head of a build's line, and what follows it.
fn build_identity(rest: &str) -> (Option<u64>, &str) {
    if let Some(after) = rest.strip_prefix('#') {
        let (digits, more) = after.split_once(' ').unwrap_or((after, ""));
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            if let Ok(n) = digits.parse() {
                return (Some(n), more);
            }
        }
    }
    (None, rest)
}

fn read_command(input: &mut impl BufRead, opts: &Options) -> Option<Command> {
    let mut line = String::new();
    if input.read_line(&mut line).ok()? == 0 {
        return None;
    }
    let line = line.trim_end_matches(['\n', '\r']);
    if line == "quit" {
        return Some(Command::Quit);
    }
    if line == "stats" {
        return Some(Command::Stats);
    }
    if opts.index {
        if let Some(q) = crate::index::Query::parse(line) {
            return Some(Command::Query(q));
        }
    }
    if let Some(rest) = line.strip_prefix("text ") {
        if let Some((path, count)) = rest.rsplit_once(' ') {
            if let Ok(count) = count.trim().parse::<usize>() {
                if count == 0 {
                    return Some(Command::Text(path.to_string(), None));
                }
                let mut bytes = vec![0u8; count];
                input.read_exact(&mut bytes).ok()?;
                return Some(Command::Text(path.to_string(), Some(String::from_utf8_lossy(&bytes).into_owned())));
            }
        }
    }
    // `empty <path>`: an empty text, which `text <path> 0` cannot say.
    if let Some(path) = line.strip_prefix("empty ") {
        return Some(Command::Text(path.to_string(), Some(String::new())));
    }
    if let Some(n) = line.strip_prefix("cancel ").and_then(|n| n.trim().parse().ok()) {
        return Some(Command::Cancel(n));
    }
    if line == "build" {
        return Some(Command::Build(None, None));
    }
    if let Some(rest) = line.strip_prefix("build ") {
        let (build, first) = build_identity(rest);
        if build.is_some() && first.trim().is_empty() {
            return Some(Command::Build(None, build));
        }
        let mut paths = vec![first.trim().to_string()];
        loop {
            let mut more = String::new();
            if input.read_line(&mut more).ok()? == 0 {
                break;
            }
            let more = more.trim();
            if more.is_empty() {
                break;
            }
            paths.push(more.to_string());
        }
        return Some(Command::Build(Some(paths), build));
    }
    Some(if line.trim().is_empty() { Command::Blank } else { Command::Unknown(line.to_string()) })
}

enum JsonValue<'a> {
    Str(&'a str),
    Num(f64),
    Int(usize),
    Bool(bool),
    Raw(&'a str),
}

pub(crate) fn json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn json_object(fields: &[(&str, JsonValue)]) -> String {
    let mut out = String::from("{");
    for (i, (key, value)) in fields.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        json_string(key, &mut out);
        out.push(':');
        match value {
            JsonValue::Str(s) => json_string(s, &mut out),
            JsonValue::Num(n) => out.push_str(&format!("{:.2}", n)),
            JsonValue::Int(n) => out.push_str(&n.to_string()),
            JsonValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            JsonValue::Raw(r) => out.push_str(r),
        }
    }
    out.push('}');
    out
}

fn json_array(items: &[String], quoted: bool) -> String {
    let mut out = String::from("[");
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        if quoted {
            json_string(item, &mut out);
        } else {
            out.push_str(item);
        }
    }
    out.push(']');
    out
}

/// One diagnostic as JSON, with the line it points at and the caret line under it, as the text
/// form prints them.
fn render_at(path: &str, line: usize, col: usize, line_text: &str, start: u32, end: u32, msg: &str) -> String {
    let width = (end.saturating_sub(start) as usize).clamp(1, line_text.len().saturating_sub(col - 1).max(1));
    let caret = format!("{}{}", " ".repeat(col - 1), "^".repeat(width));
    json_object(&[
        ("file", JsonValue::Str(path)),
        ("line", JsonValue::Int(line)),
        ("col", JsonValue::Int(col)),
        ("message", JsonValue::Str(msg)),
        ("source", JsonValue::Str(line_text)),
        ("caret", JsonValue::Str(&caret)),
    ])
}

/// The column, 1-based, in UTF-16 units of the byte column `byte_col` of `line_text`.
fn utf16_col(line_text: &str, byte_col: usize) -> usize {
    let mut b = byte_col.saturating_sub(1).min(line_text.len());
    while !line_text.is_char_boundary(b) {
        b -= 1;
    }
    line_text[..b].encode_utf16().count() + 1
}

/// One diagnostic of a `--check` session: the span's start and exclusive end as lines and
/// UTF-16 columns, its severity, and the source line and caret as the build answers have them.
fn render_check_at(path: &str, text: &str, start: u32, end: u32, severity: &str, msg: &str, unnecessary: bool) -> String {
    let end = end.max(start);
    let (line, col, line_text) = locate(text, start as usize);
    let (end_line, end_col, end_line_text) = locate(text, end as usize);
    let width = (end.saturating_sub(start) as usize).clamp(1, line_text.len().saturating_sub(col - 1).max(1));
    let caret = format!("{}{}", " ".repeat(col - 1), "^".repeat(width));
    // A language server's session gives the file's URI and the range as the protocol counts it.
    let (uri, range) = match INDEXED.get() {
        Some(&positions) => {
            let uri = crate::lsp::uri::from_path(&canonical(path));
            let range = crate::index::Lines::new(text).range(crate::source::Span::new(start, end), positions).to_text();
            (Some(uri), Some(range))
        }
        None => (None, None),
    };
    let mut fields = vec![
        ("file", JsonValue::Str(path)),
        ("line", JsonValue::Int(line)),
        ("col", JsonValue::Int(utf16_col(line_text, col))),
        ("endLine", JsonValue::Int(end_line)),
        ("endCol", JsonValue::Int(utf16_col(end_line_text, end_col))),
        ("severity", JsonValue::Str(severity)),
        ("message", JsonValue::Str(msg)),
        ("source", JsonValue::Str(line_text)),
        ("caret", JsonValue::Str(&caret)),
    ];
    if let (Some(uri), Some(range)) = (&uri, &range) {
        fields.push(("uri", JsonValue::Str(uri)));
        fields.push(("range", JsonValue::Raw(range)));
    }
    // Code that does nothing (an unused import): the protocol's `Unnecessary` tag.
    if unnecessary {
        fields.push(("tags", JsonValue::Raw("[1]")));
    }
    json_object(&fields)
}

fn render_check_one(d: &Diagnostic, files: &[SourceFile]) -> String {
    let severity = if d.hint { "hint" } else if d.is_warning { "warning" } else { "error" };
    if d.file == crate::source::NO_FILE {
        return json_object(&[("severity", JsonValue::Str(severity)), ("message", JsonValue::Str(&d.msg))]);
    }
    if let Some(p) = &d.place {
        return render_placed(p, Some(severity), &d.msg);
    }
    let Some(f) = files.get(d.file.0 as usize) else {
        return json_object(&[("file", JsonValue::Str("<library>")), ("severity", JsonValue::Str(severity)), ("message", JsonValue::Str(&d.msg))]);
    };
    render_check_at(&f.path, &f.text, d.span.start, d.span.end, severity, &d.msg, d.unnecessary)
}

/// Every diagnostic in file and position order, for a `--check` answer.
pub(crate) fn render_check(diags: &Diagnostics, files: &[SourceFile]) -> Vec<String> {
    let mut items: Vec<&Diagnostic> = diags.presented().collect();
    items.sort_by_key(|d| (d.file.0, d.span.start));
    items.into_iter().map(|d| render_check_one(d, files)).collect()
}

/// The warnings of a build in file and position order; a hint is none.
fn render_warnings(diags: &Diagnostics, files: &[SourceFile]) -> Vec<String> {
    let mut items: Vec<&Diagnostic> = diags.presented().filter(|d| d.is_warning && !d.hint).collect();
    items.sort_by_key(|d| (d.file.0, d.span.start));
    items.into_iter().map(|d| render_one(d, files)).collect()
}

fn render_one(d: &Diagnostic, files: &[SourceFile]) -> String {
    if d.file == crate::source::NO_FILE {
        return json_object(&[("message", JsonValue::Str(&d.msg))]);
    }
    if let Some(p) = &d.place {
        return render_placed(p, None, &d.msg);
    }
    let Some(f) = files.get(d.file.0 as usize) else {
        return json_object(&[("file", JsonValue::Str("<library>")), ("message", JsonValue::Str(&d.msg))]);
    };
    let (line, col, line_text) = locate(&f.text, d.span.start as usize);
    render_at(&f.path, line, col, line_text, d.span.start, d.span.end, &d.msg)
}

/// A diagnostic placed in a library's source: the place as the file, its line and column where
/// known.
fn render_placed(p: &crate::source::Place, severity: Option<&str>, msg: &str) -> String {
    let mut fields = vec![("file", JsonValue::Str(&p.source))];
    if let (Some(line), Some(col)) = (p.line, p.col) {
        fields.push(("line", JsonValue::Int(line as usize)));
        fields.push(("col", JsonValue::Int(col as usize)));
    }
    if let Some(s) = severity {
        fields.push(("severity", JsonValue::Str(s)));
    }
    fields.push(("message", JsonValue::Str(msg)));
    if let Some(t) = &p.line_text {
        fields.push(("source", JsonValue::Str(t)));
    }
    json_object(&fields)
}

/// The errors of a build in file and position order, or every diagnostic but the hints under
/// `--werror`.
fn render_all(diags: &Diagnostics, files: &[SourceFile], all: bool) -> Vec<String> {
    let mut items: Vec<&Diagnostic> = diags.presented().filter(|d| (all && !d.hint) || !d.is_warning).collect();
    items.sort_by_key(|d| (d.file.0, d.span.start));
    items.into_iter().map(|d| render_one(d, files)).collect()
}

/// The answer to a query: one line `{"result":...}`.
fn print_answer(result: &crate::lsp::json::Json) {
    print_answer_declared(result, None);
}

/// With where the query's target is declared: `{"result":...,"declared":{"path":..,"offset":..}}`,
/// and `"notes"`, the document paths the query found something else at
/// (`attach::take_refusals`).
fn print_answer_declared(result: &crate::lsp::json::Json, declared: Option<&(PathBuf, u32)>) {
    let mut line = String::from("{\"result\":");
    result.write(&mut line);
    if let Some((path, offset)) = declared {
        line.push_str(",\"declared\":");
        crate::lsp::json::obj([("path", path.to_string_lossy().into_owned().into()), ("offset", (*offset).into())]).write(&mut line);
    }
    let notes = crate::typer::loader::attach::take_refusals();
    if !notes.is_empty() {
        line.push_str(",\"notes\":");
        crate::lsp::json::Json::Arr(notes.into_iter().map(Into::into).collect()).write(&mut line);
    }
    line.push('}');
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", line);
    let _ = out.flush();
}

/// The program entry a file belongs to: its own, or that of the file whose `package p:` block
/// it is.
fn entry_of_file(f: usize, n_std: usize, n_entries: usize, blocks_of: &[Vec<usize>]) -> Option<usize> {
    if f >= n_std && f < n_std + n_entries {
        return Some(f - n_std);
    }
    (0..n_entries).find(|&i| blocks_of.get(n_std + i).map_or(false, |b| b.contains(&f)))
}

/// The answer to `stats`, one JSON line, in bytes but for the counts. `allocator`: what the
/// allocator holds. `reserved` is what it took from the system for blocks up to 32 KB and
/// never gives back, `free` what of that is on no thread's hands (at the centre, on the lists
/// of the compiler thread and of the typing thread), `large` the larger requests, `large_peak`
/// the most of them since the last `stats`, `mapped` the ones of 1 MB and more among them,
/// which the session maps itself, and `mapped_kept` the address space of the freed mappings it
/// keeps for the next build. `session`, once a build has typed: what its stores hold by their
/// own shapes (`held.rs`), `account` and `baseline` the allocator's account of the session
/// now and when its last full build was answered, `builds` the builds since. `interp`: the
/// nodes the interpreter registered as able to close a cycle (`interp/registry.rs`), by kind,
/// and what its last sweep emptied and freed, in how many microseconds.
fn print_stats(kept: Option<&Kept>) {
    let held = crate::alloc::held();
    let resident = crate::alloc::resident_pages();
    crate::alloc::forget_peak();
    let registry = crate::typer::thread::run(crate::interp::registry::stats);
    let interp = json_object(&[
        ("registered", JsonValue::Int(registry.registered)),
        ("frames", JsonValue::Int(registry.frames)),
        ("objects", JsonValue::Int(registry.objects)),
        ("arrays", JsonValue::Int(registry.arrays)),
        ("maps", JsonValue::Int(registry.maps)),
        ("swept", JsonValue::Int(registry.swept)),
        ("freed", JsonValue::Int(registry.freed)),
        ("sweep_us", JsonValue::Int(registry.sweep_us as usize)),
    ]);
    let compiler = crate::alloc::thread_free();
    let typer = if crate::typer::thread::separate() { crate::typer::thread::run(crate::alloc::thread_free) } else { 0 };
    let free = held.centre_free + compiler + typer;
    let allocator = json_object(&[
        ("reserved", JsonValue::Int(held.reserved)),
        ("chunks", JsonValue::Int(held.chunks)),
        ("free", JsonValue::Int(free)),
        ("free_centre", JsonValue::Int(held.centre_free)),
        ("free_compiler", JsonValue::Int(compiler)),
        ("free_typer", JsonValue::Int(typer)),
        ("live", JsonValue::Int(held.reserved.saturating_sub(free))),
        ("large", JsonValue::Int(held.large)),
        ("large_peak", JsonValue::Int(held.large_peak)),
        ("mapped", JsonValue::Int(held.mapped)),
        ("mapped_kept", JsonValue::Int(held.mapped_kept)),
        ("mappings_made", JsonValue::Int(held.mappings_made)),
        ("chains_taken", JsonValue::Int(held.chains_taken)),
        ("runs_carved", JsonValue::Int(held.runs_carved)),
        ("rss", JsonValue::Int(resident.rss)),
        ("chunks_resident", JsonValue::Int(resident.chunks)),
        ("kept_resident", JsonValue::Int(resident.kept)),
        ("mapped_resident", JsonValue::Int(resident.mapped)),
    ]);
    let mut parts = vec![("allocator", allocator), ("interp", interp)];
    if let Some(k) = kept {
        parts.push((
            "session",
            json_object(&[
                ("account", JsonValue::Int(account())),
                ("baseline", JsonValue::Int(k.baseline)),
                ("builds", JsonValue::Int(k.builds)),
                ("program", JsonValue::Int(k.program)),
                ("records", JsonValue::Int(k.records)),
                ("symbols", JsonValue::Int(k.symbols)),
                ("types", JsonValue::Int(k.types)),
                ("names", JsonValue::Int(k.names)),
                ("interner", JsonValue::Int(k.interner)),
                ("sources", JsonValue::Int(k.sources)),
                ("trees", JsonValue::Int(k.trees)),
                ("emit_cache", JsonValue::Int(k.emit_cache)),
                ("written", JsonValue::Int(k.written)),
                ("classes", JsonValue::Int(k.classes)),
                ("kept_units", JsonValue::Int(k.kept_units)),
                ("index", JsonValue::Int(k.index)),
                ("deps", JsonValue::Int(k.deps)),
                ("inline_indexes", JsonValue::Int(k.inline_indexes)),
            ]),
        ));
    }
    let parts: Vec<(&str, JsonValue)> = parts.iter().map(|(name, part)| (*name, JsonValue::Raw(part))).collect();
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", json_object(&[("stats", JsonValue::Raw(&json_object(&parts)))]));
    let _ = out.flush();
}

/// The answer of a build that parsed changed input, whether it went on to type or failed
/// there. The mappings the build did not take from the ones kept go back to the system:
/// once a build, since a query or a build that found nothing changed frees no array for the
/// next to take.
/// The files a build asks for with those a cancelled one carried (none: every file).
fn with_carried(carried: Option<Option<Vec<String>>>, requested: Option<Vec<String>>) -> Option<Vec<String>> {
    match (carried, requested) {
        (None, requested) => requested,
        (Some(None), _) | (_, None) => None,
        (Some(Some(mut paths)), Some(more)) => {
            paths.extend(more);
            Some(paths)
        }
    }
}

/// The answer of a build cancelled at its commit point.
fn print_cancelled(build: Option<u64>) {
    let id = build.unwrap_or_default();
    crate::lsp::hooks::trace("session", || format!("cancelled {}", id));
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", json_object(&[("cancelled", JsonValue::Bool(true)), ("build", JsonValue::Int(id as usize))]));
    let _ = out.flush();
}

fn print_built(r: &Report) {
    print_report(r);
    crate::alloc::age();
}

fn print_report(r: &Report) {
    let ms = |d: Duration| JsonValue::Num(d.as_secs_f64() * 1000.0);
    let mut phases: Vec<(&str, JsonValue)> = r.ms.attempt.map(|a| ("attempt", ms(a))).into_iter().collect();
    phases.extend([
        ("read", ms(r.ms.read)),
        ("parse", ms(r.ms.parse)),
        ("type", ms(r.ms.type_check)),
        ("reach", ms(r.ms.reach)),
        ("emit", ms(r.ms.emit)),
        ("write", ms(r.ms.write)),
    ]);
    if let Some(&v) = ANALYSIS_VERSION.get() {
        phases.push(("api", ms(r.ms.api)));
        if v >= 3 {
            phases.push(("deps", ms(r.ms.deps)));
        }
    }
    phases.push(("total", ms(r.ms.total)));
    let timings = json_object(&phases);
    let changed = json_array(&r.changed, true);
    let retyped = json_array(&r.retyped, true);
    let removed = json_array(&r.removed, true);
    let deleted = json_array(&r.deleted, true);
    let analysis = r.analysis.as_ref().map(|a| json_array(a, false));
    let errors = json_array(&r.errors, false);
    let warnings = json_array(&r.warnings, false);
    let diagnostics = r.diagnostics.as_ref().map(|d| json_array(d, false));
    let notes = crate::types::take_mapping_refused().then(|| json_array(&[crate::types::MAPPING_REFUSED.to_string()], true));
    let mut fields: Vec<(&str, JsonValue)> = vec![("ok", JsonValue::Bool(r.ok))];
    if r.ok {
        fields.push(("changed", JsonValue::Raw(&changed)));
        fields.push(("modules", JsonValue::Int(r.modules)));
    } else {
        fields.push(("errors", JsonValue::Raw(&errors)));
    }
    fields.push(("warnings", JsonValue::Raw(&warnings)));
    if let Some(notes) = &notes {
        fields.push(("notes", JsonValue::Raw(notes)));
    }
    if let Some(diagnostics) = &diagnostics {
        fields.push(("diagnostics", JsonValue::Raw(diagnostics)));
    }
    fields.push(("ms", JsonValue::Raw(&timings)));
    let parts = r.parts.as_ref().map(|p| format!("{{{}}}", p));
    if let Some(parts) = &parts {
        fields.push(("parts", JsonValue::Raw(parts)));
    }
    fields.push(("incremental", JsonValue::Bool(r.incremental)));
    if let Some(f) = &r.fallback {
        fields.push(("fallback", JsonValue::Str(f)));
    }
    if r.incremental {
        fields.push(("retyped", JsonValue::Raw(&retyped)));
    }
    if !r.deleted.is_empty() {
        fields.push(("deleted", JsonValue::Raw(&deleted)));
    }
    if !r.removed.is_empty() {
        fields.push(("removed", JsonValue::Raw(&removed)));
    }
    if let Some(analysis) = &analysis {
        fields.push(("analysis", JsonValue::Raw(analysis)));
    }
    let api_errors = json_array(&r.api_errors, true);
    if let Some(&v) = ANALYSIS_VERSION.get() {
        fields.push(("analysisVersion", JsonValue::Int(v as usize)));
        if let Some(api) = &r.api {
            fields.push(("api", JsonValue::Raw(api)));
        }
        if !r.api_errors.is_empty() {
            fields.push(("apiErrors", JsonValue::Raw(&api_errors)));
        }
    }
    if let Some(id) = r.build {
        fields.push(("build", JsonValue::Int(id as usize)));
        crate::lsp::hooks::hold("answer");
        crate::lsp::hooks::trace("session", || match &r.fallback {
            Some(why) if !r.incremental => format!("answered {} full {}", id, why),
            _ => format!("answered {} retyped {}", id, r.retyped.len()),
        });
    }
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", json_object(&fields));
    let _ = out.flush();
}
