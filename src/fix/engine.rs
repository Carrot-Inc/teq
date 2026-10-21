//! `teq fix`'s run (docs/FIX.md, "The run"): the rules compiled and configured once on the rule thread, the
//! interpreter's; each job (a configuration's program, or the files `teq compiler fix` names) typed on a typing thread
//! of its own, the two never at once; each editable document's view handed to the rules; the patches planned, all
//! of them before anything is written; at most two rounds, the second over the first's text, typed again, for the
//! imports the first's edits left unused.

use super::config::Conf;
use super::host::{self, DocView, Host, Results, Severity, Utf16};
use super::nodes::{self, K};
use super::plan::{self, Plan, Printed};
use crate::source::{FileId, SourceFile, Span};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// What a run asks: the configuration, the rules, whether it only checks.
pub struct Setup {
    pub conf: Conf,
    /// Where a relative `file:` entry of the configuration starts.
    pub config_dir: PathBuf,
    /// `--rules`, which replaces the configuration's `rules`.
    pub rules: Option<Vec<String>>,
    pub check: bool,
    /// Where the run works (the build's root) and the directory its paths print from, where they differ.
    pub display: Option<(PathBuf, PathBuf)>,
}

/// `path` (the program's, from the working directory) as printed: from the directory the paths print from.
pub fn shown_path(display: &Option<(PathBuf, PathBuf)>, path: &str) -> String {
    let Some((root, from)) = display else { return path.to_string() };
    let abs = root.join(path);
    relative(&abs, from).unwrap_or_else(|| abs.to_string_lossy().into_owned())
}

/// `to` from the directory `from`: `..` up to their common ancestor, then down.
fn relative(to: &Path, from: &Path) -> Option<String> {
    let t: Vec<_> = to.components().collect();
    let f: Vec<_> = from.components().collect();
    let common = t.iter().zip(&f).take_while(|(a, b)| a == b).count();
    if common == 0 {
        return None;
    }
    let mut parts: Vec<String> = std::iter::repeat_n("..".to_string(), f.len() - common).collect();
    parts.extend(t[common..].iter().map(|c| c.as_os_str().to_string_lossy().into_owned()));
    Some(parts.join("/"))
}

/// A program to type and the files of it the run reads and edits.
#[derive(Clone)]
pub struct Job {
    /// The compiler's arguments, `teq compiler check`'s: the inputs and the options.
    pub args: Vec<String>,
    /// The editable files: those under these roots (as the program's paths start), every file where none.
    pub roots: Option<Vec<String>>,
    /// The files a command line named, canonical: the editable ones narrowed to them.
    pub files: Option<Vec<PathBuf>>,
    /// What generators write (files or directories, as the program's paths start): typed, never edited.
    pub generated: Vec<String>,
}

impl Job {
    /// Whether the program's file `p` is the job's own: under its roots, written by no generator.
    pub fn owns(&self, p: &str) -> bool {
        self.roots.as_ref().is_none_or(|roots| roots.iter().any(|r| Path::new(p).starts_with(r))) && !self.generated.iter().any(|g| Path::new(p).starts_with(g))
    }

    /// Whether the job edits the program's file `p`: its own, among the files named where some are.
    pub fn edits(&self, p: &str) -> bool {
        self.owns(p) && self.files.as_ref().is_none_or(|files| crate::source::canonicalize(Path::new(p)).is_ok_and(|c| files.contains(&c)))
    }
}

enum ToRules {
    /// A document, and whether its semantic rules alone run over it.
    Doc(Box<DocView>, bool),
    End,
}

enum FromRules {
    Ready(Result<Vec<RuleInfo>, String>),
    /// The document's results, and the document handed back for the planner.
    Done(Box<Results>, Box<DocView>),
}

#[derive(Clone)]
struct RuleInfo {
    name: String,
    semantic: bool,
}

/// One file's outcome of a round.
struct Outcome {
    /// The path as the program has it, from where the run works.
    file: String,
    doc_path: String,
    text: String,
    plan: Plan,
}

/// Runs `jobs` under `setup`; the run's exit status (docs/CLI.md, "teq fix").
pub fn run(setup: Setup, jobs: Vec<Job>) -> i32 {
    let requested = match requested_rules(&setup) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("teq fix: {}", e);
            return 2;
        }
    };
    // The lint filters decoded before any rule runs: a setting that does not decode refuses the run.
    let lints = match LintConfig::of(&setup.conf) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("teq fix: {}", e);
            return 2;
        }
    };
    let (to_rules, rules_rx) = mpsc::channel::<ToRules>();
    let (from_rules_tx, from_rules) = mpsc::channel::<FromRules>();
    let conf = setup.conf.clone();
    let config_dir = setup.config_dir.clone();
    std::thread::scope(|scope| {
        let rule_thread = std::thread::Builder::new()
            .name("teq-fix-rules".to_string())
            .stack_size(1 << 30)
            .spawn_scoped(scope, move || {
                crate::alloc::enter();
                rule_thread(conf, config_dir, requested, rules_rx, from_rules_tx)
            })
            .expect("the rule thread");
        let code = drive(&setup, &lints, &jobs, &to_rules, &from_rules);
        let _ = to_rules.send(ToRules::End);
        let _ = rule_thread.join();
        code
    })
}

fn requested_rules(setup: &Setup) -> Result<Vec<String>, String> {
    if let Some(r) = &setup.rules {
        return Ok(r.clone());
    }
    let node = setup.conf.at(&["rules"]).or_else(|| setup.conf.at(&["rule"]));
    match node {
        Some(n) => setup.conf.strings(n),
        None => Err(format!("{}: no rules: the configuration names none (`rules = [...]`) and --rules is not given", setup.conf.file)),
    }
}

fn drive(setup: &Setup, lint_config: &LintConfig, jobs: &[Job], to_rules: &mpsc::Sender<ToRules>, from_rules: &mpsc::Receiver<FromRules>) -> i32 {
    let started = std::time::Instant::now();
    let code = drive_rounds(setup, lint_config, jobs, to_rules, from_rules);
    if std::env::var_os("TEQ_FIX_TRACE").is_some() {
        TIMES.with(|t| {
            let (t, held) = (t.get(), crate::alloc::held());
            eprintln!("fix: {:.0} ms in all: the rules' program {:.0} ms, typing {:.0} ms, the rules {:.0} ms, planning {:.0} ms; the allocator reserved {} MB, its requests over 32 KB at most {} MB", started.elapsed().as_secs_f64() * 1e3, t[0] * 1e3, t[1] * 1e3, t[2] * 1e3, t[3] * 1e3, held.reserved >> 20, held.large_peak >> 20);
        });
    }
    code
}

thread_local! {
    /// Seconds the run spent: making the rules' program ready, typing, in the rules, planning (`TEQ_FIX_TRACE`).
    static TIMES: std::cell::Cell<[f64; 4]> = const { std::cell::Cell::new([0.0; 4]) };
}

fn timed(k: usize, since: std::time::Instant) {
    let d = since.elapsed().as_secs_f64();
    TIMES.with(|t| {
        let mut v = t.get();
        v[k] += d;
        t.set(v);
    });
}

fn drive_rounds(setup: &Setup, lint_config: &LintConfig, jobs: &[Job], to_rules: &mpsc::Sender<ToRules>, from_rules: &mpsc::Receiver<FromRules>) -> i32 {
    let ready = std::time::Instant::now();
    let infos = match from_rules.recv() {
        Ok(FromRules::Ready(Ok(infos))) => infos,
        Ok(FromRules::Ready(Err(e))) => {
            eprintln!("{}", e.trim_end());
            return 2;
        }
        _ => {
            eprintln!("teq fix: the rule thread ended");
            return 2;
        }
    };
    timed(0, ready);
    let names: Vec<String> = infos.iter().map(|i| i.name.clone()).collect();
    let semantic_rules: Vec<bool> = infos.iter().map(|i| i.semantic).collect();
    let semantic = semantic_rules.contains(&true);
    let mut gen = 0u32;
    // Round one, over the files as read; the rules' results kept for round two.
    let mut first: Vec<(usize, Vec<Outcome>)> = Vec::new();
    let mut kept: HashMap<String, Results> = HashMap::new();
    for (k, job) in jobs.iter().enumerate() {
        let earlier = Earlier { results: &HashMap::new(), semantic: &semantic_rules };
        match round(job, &HashMap::new(), semantic, &mut gen, &names, &setup.display, to_rules, from_rules, earlier, Some(&mut kept)) {
            Ok(outcomes) => first.push((k, outcomes)),
            Err(e) => {
                eprint!("{}", e);
                return 2;
            }
        }
    }
    let mut by_path = match merge(first.iter().flat_map(|(_, o)| o.iter())) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}", e);
            return 2;
        }
    };
    if let Some(code) = refused(by_path.values().map(|o| &o.plan)) {
        return code;
    }
    // Round two, where round one edits: the files as round one leaves them, typed again, every job whose program reads
    // one of them (a dependent's too, whose own files are as they were); only the imports those edits left unused may
    // be removed there, and a job that no longer types refuses the whole plan before anything is written.
    let changed: HashMap<String, String> = by_path.values().filter(|o| !o.plan.edits.is_empty()).map(|o| (o.file.clone(), plan::apply(&o.text, &o.plan.edits))).collect();
    let mut finals: HashMap<String, (String, Plan)> = HashMap::new();
    if !changed.is_empty() {
        let mut second: Vec<Outcome> = Vec::new();
        for (k, _) in &first {
            if !program_paths(&jobs[*k]).iter().any(|p| changed.contains_key(p)) {
                continue;
            }
            let earlier = Earlier { results: &kept, semantic: &semantic_rules };
            match round(&jobs[*k], &changed, semantic, &mut gen, &names, &setup.display, to_rules, from_rules, earlier, None) {
                Ok(o) => second.extend(o),
                Err(e) => {
                    eprintln!("teq fix: the program does not type after the rules' edits ({}):", rules_of(&by_path, &changed));
                    eprint!("{}", e);
                    return 2;
                }
            }
        }
        let second = match merge(second.iter()) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{}", e);
                return 2;
            }
        };
        if let Some(code) = refused(second.values().map(|o| &o.plan)) {
            return code;
        }
        for (path, o) in second {
            if let Some(&(_, _, ref rules)) = o.plan.edits.iter().find(|(_, _, rules)| rules.iter().any(|&r| names[r as usize] != "NoUnusedImports")) {
                let rule = rules.iter().map(|&r| names[r as usize].as_str()).find(|n| *n != "NoUnusedImports").unwrap_or("");
                eprintln!("{}: the rule {} rewrites its own output: a second run would change the file again", path, rule);
                return 2;
            }
            let after = plan::apply(&o.text, &o.plan.edits);
            let lints = shifted(&o.plan.lints, &o.plan.edits);
            finals.insert(o.file.clone(), (after, Plan { edits: Vec::new(), lints, refused: Vec::new() }));
        }
        // A changed file round two did not see keeps round one's text.
        for (p, t) in &changed {
            finals.entry(p.clone()).or_insert_with(|| (t.clone(), Plan::default()));
        }
    }
    // The outcome: per file its final text where it changes, its lints.
    let mut shown: Vec<(String, String, Printed)> = Vec::new();
    let mut writes: Vec<(String, String)> = Vec::new();
    let mut paths: Vec<String> = by_path.keys().cloned().collect();
    paths.sort();
    for path in &paths {
        let o = by_path.remove(path).unwrap();
        let (text, lints) = match finals.remove(&o.file) {
            Some((text, p)) => {
                if text != o.text {
                    writes.push((o.file.clone(), text.clone()));
                }
                (text, p.lints)
            }
            None => (o.text.clone(), o.plan.lints),
        };
        for mut l in lints {
            match lint_config.severity(&l.id, l.severity) {
                Some(s) => l.severity = s,
                None => continue,
            }
            shown.push((path.clone(), text.clone(), l));
        }
    }
    print_lints(&shown);
    let errors = shown.iter().filter(|(_, _, l)| l.severity == Severity::Error).count();
    if setup.check {
        for (path, _) in &writes {
            eprintln!("{}: would be rewritten", shown_path(&setup.display, path));
        }
        return if writes.is_empty() && shown.is_empty() { 0 } else { 1 };
    }
    if let Err(e) = write_all(&writes) {
        eprintln!("teq fix: {}", e);
        return 2;
    }
    if errors > 0 {
        1
    } else {
        0
    }
}

/// The rules whose edits `changed` holds, for the message of a program they broke.
fn rules_of(by_path: &HashMap<String, Outcome>, changed: &HashMap<String, String>) -> String {
    let mut rules: Vec<u32> = by_path.iter().filter(|(p, _)| changed.contains_key(*p)).flat_map(|(_, o)| o.plan.edits.iter().flat_map(|e| e.2.iter().copied())).collect();
    rules.sort();
    rules.dedup();
    format!("{} rule edit{}", rules.len(), if rules.len() == 1 { "" } else { "s" })
}

/// One file's outcomes from several jobs (a file two configurations share): their plans must agree.
/// The outcomes by file, a file that several jobs read (configurations that share it) once: what refuses any of its
/// outcomes refuses it, and the lints of every outcome are kept, each once; its edits must be the same in each.
fn merge<'a>(outcomes: impl Iterator<Item = &'a Outcome>) -> Result<HashMap<String, Outcome>, String> {
    let mut out: HashMap<String, Outcome> = HashMap::new();
    for o in outcomes {
        match out.get_mut(&o.doc_path) {
            Some(prev) => {
                for r in &o.plan.refused {
                    if !prev.plan.refused.contains(r) {
                        prev.plan.refused.push(r.clone());
                    }
                }
                for l in &o.plan.lints {
                    if !prev.plan.lints.iter().any(|p| p.id == l.id && p.span == l.span && p.message == l.message && p.severity == l.severity) {
                        prev.plan.lints.push(l.clone());
                    }
                }
                prev.plan.lints.sort_by_key(|l| (l.span.start, l.span.end));
                let same = prev.plan.edits.iter().map(|(s, t, _)| (s, t)).eq(o.plan.edits.iter().map(|(s, t, _)| (s, t)));
                if !same && o.plan.refused.is_empty() && prev.plan.refused.is_empty() {
                    return Err(format!("{}: the configurations that share the file edit it differently", o.doc_path));
                }
            }
            None => {
                out.insert(o.doc_path.clone(), Outcome { file: o.file.clone(), doc_path: o.doc_path.clone(), text: o.text.clone(), plan: Plan { edits: o.plan.edits.clone(), lints: o.plan.lints.clone(), refused: o.plan.refused.clone() } });
            }
        }
    }
    Ok(out)
}

fn refused<'a>(plans: impl Iterator<Item = &'a Plan>) -> Option<i32> {
    let mut any = false;
    let mut msgs: Vec<&String> = plans.flat_map(|p| p.refused.iter()).collect();
    msgs.sort();
    msgs.dedup();
    for m in msgs {
        eprintln!("{}", m);
        any = true;
    }
    any.then_some(2)
}

/// Lints of a text after `edits`, which remove imports alone: each moved by what was removed before it, one inside
/// a removed range dropped.
fn shifted(lints: &[Printed], edits: &[(Span, String, Vec<u32>)]) -> Vec<Printed> {
    let mut out = Vec::new();
    'lints: for l in lints {
        let mut delta: i64 = 0;
        for (s, t, _) in edits {
            if l.span.start >= s.end {
                delta += t.len() as i64 - (s.end - s.start) as i64;
            } else if l.span.end > s.start {
                continue 'lints;
            }
        }
        let mut l = l.clone();
        l.span = Span::new((l.span.start as i64 + delta) as u32, (l.span.end as i64 + delta) as u32);
        out.push(l);
    }
    out
}

/// Round one's results per file, and which rules are semantic: in round two a file round one left as it was reads
/// alike, so that only its semantic rules may answer otherwise (the unused imports of a program another file's edits
/// changed); the other rules' results there are round one's.
struct Earlier<'a> {
    results: &'a HashMap<String, Results>,
    semantic: &'a [bool],
}

impl Earlier<'_> {
    /// Round one's results of the rules that are not semantic, with `later`'s of the semantic ones, each rule's in the
    /// order it made them and the rules in theirs, as one run of every rule over the document gives them.
    fn joined(&self, earlier: &Results, later: Results) -> Results {
        let syntactic = |r: u32| !self.semantic.get(r as usize).copied().unwrap_or(false);
        let import_rule = |op: &host::ImportOp| match op {
            host::ImportOp::Remove { rule, .. } | host::ImportOp::Add { rule, .. } => *rule,
        };
        let mut edits: Vec<host::Edit> = earlier.edits.iter().filter(|e| syntactic(e.rule)).cloned().chain(later.edits).collect();
        edits.sort_by_key(|e| e.rule);
        let mut imports: Vec<host::ImportOp> = earlier.imports.iter().filter(|op| syntactic(import_rule(op))).cloned().chain(later.imports).collect();
        imports.sort_by_key(import_rule);
        let mut lints: Vec<host::Lint> = earlier.lints.iter().filter(|l| syntactic(l.rule)).cloned().chain(later.lints).collect();
        lints.sort_by_key(|l| l.rule);
        let mut failures: Vec<(u32, String)> = earlier.failures.iter().filter(|(r, _)| syntactic(*r)).cloned().chain(later.failures).collect();
        failures.sort_by_key(|(r, _)| *r);
        Results { edits, imports, lints, failures }
    }
}

/// A round of `job`: its program typed with `overrides` standing for files, each editable document's rules run and
/// their patches planned; round one keeps each file's results in `kept`.
#[allow(clippy::too_many_arguments)]
fn round(job: &Job, overrides: &HashMap<String, String>, semantic: bool, gen: &mut u32, names: &[String], display: &Option<(PathBuf, PathBuf)>, to_rules: &mpsc::Sender<ToRules>, from_rules: &mpsc::Receiver<FromRules>, earlier: Earlier<'_>, mut kept: Option<&mut HashMap<String, Results>>) -> Result<Vec<Outcome>, String> {
    let first_gen = *gen;
    let typing = std::time::Instant::now();
    let docs = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("teq-fix-typer".to_string())
            .stack_size(1 << 30)
            .spawn_scoped(scope, || {
                crate::alloc::enter();
                typed(job, overrides, semantic, first_gen, display)
            })
            .expect("the typing thread")
            .join()
            .unwrap_or_else(|_| Err("teq fix: the typing thread panicked\n".to_string()))
    })?;
    timed(1, typing);
    *gen = first_gen + docs.len() as u32;
    let mut out = Vec::new();
    for d in docs {
        let (file, path, text) = (d.file.clone(), d.path.clone(), d.text.clone());
        // The planner reads the view too: the rules hand it back with their results.
        let in_rules = std::time::Instant::now();
        let before = if overrides.contains_key(&file) { None } else { earlier.results.get(&file) };
        let _ = to_rules.send(ToRules::Doc(Box::new(d), before.is_some()));
        let (view, results) = match from_rules.recv() {
            Ok(FromRules::Done(r, v)) => (v, r),
            _ => return Err("teq fix: the rule thread ended\n".to_string()),
        };
        let results = match before {
            Some(b) => earlier.joined(b, *results),
            None => *results,
        };
        timed(2, in_rules);
        let planning = std::time::Instant::now();
        if std::env::var_os("TEQ_FIX_TRACE").is_some() {
            eprintln!("fix: {} (generation {}): {} nodes, {} tokens, {} edits, {} import edits, {} lints, {} failures", view.path, view.gen, view.tree.nodes.len(), view.tokens.len(), results.edits.len(), results.imports.len(), results.lints.len(), results.failures.len());
            for (r, f) in &results.failures {
                eprintln!("fix:   rule {} failed: {}", r, f);
            }
        }
        let p = plan::plan(&view, &results, names);
        timed(3, planning);
        if let Some(k) = kept.as_deref_mut() {
            k.insert(file.clone(), results);
        }
        out.push(Outcome { file, doc_path: path, text, plan: p });
    }
    Ok(out)
}

/// The paths of the job's program, its editable files and the rest of what it reads.
pub(crate) fn program_paths(job: &Job) -> Vec<String> {
    let mut args = vec!["compiler".to_string(), "check".to_string()];
    args.extend(job.args.iter().cloned());
    crate::collect_program_paths_with_roots(&crate::parse_options_from(args.into_iter())).0
}

/// The job's program typed as `teq compiler check` types it, with the index and, for a semantic rule, the unused
/// imports, and the views of its editable files; the rendered errors where it does not type.
fn typed(job: &Job, overrides: &HashMap<String, String>, semantic: bool, first_gen: u32, display: &Option<(PathBuf, PathBuf)>) -> Result<Vec<DocView>, String> {
    let mut args = vec!["compiler".to_string(), "check".to_string()];
    args.extend(job.args.iter().cloned());
    let mut opts = crate::parse_options_from(args.into_iter());
    // The language server's index: the parser's name spans and the typer's records.
    opts.index = true;
    opts.syntax.index = true;
    if semantic {
        opts.wunused_imports = true;
    }
    let (paths, roots, _) = crate::collect_program_paths_with_roots(&opts);
    let editable: Vec<String> = paths.iter().filter(|p| job.edits(p)).cloned().collect();
    opts.keep = editable.clone();
    // A file's text as round one left it where `overrides` has it; a script's as written, its document's text: the
    // wrapper `read_program` appends to a `.sc` (`script::wrap`) is for typing alone, past the script's own text.
    let mut written: HashMap<String, String> = HashMap::new();
    let program = crate::read_program_with(&opts, paths, &roots, |p, disk| {
        let text = overrides.get(p).cloned().unwrap_or(disk);
        if crate::script::is_script(p) {
            written.insert(p.to_string(), text.clone());
        }
        text
    })
    .map_err(|e| format!("{}\n", e))?;
    let mut interner = crate::intern::Interner::new();
    let mut diags = crate::source::Diagnostics::default();
    let cp = crate::open_jars(&opts);
    let (mut files, mut asts, blocks_of, std, selected, scalajs_unlocked) = crate::parse_build(&opts, cp.as_ref(), program, &mut interner, &mut diags);
    if diags.has_errors() {
        return Err(diags.render(&files));
    }
    let (jar_files, jdk_file) = crate::jar_pseudo_files(&opts, &mut files, &mut asts);
    let asts = crate::ast::Asts::new(asts);
    let sources = crate::source::Sources::new(files);
    let mut typer = crate::typer::Typer::new(&asts, &sources, &mut interner);
    typer.threads = 1;
    crate::install_std(&mut typer, std, &selected, &blocks_of, scalajs_unlocked);
    crate::open_classpath(&opts, &mut typer, cp, jar_files, jdk_file);
    crate::configure_typer(&mut typer, &opts, &asts, &sources);
    typer.choose_entry = false;
    typer.index = Some(Box::default());
    typer.run();
    crate::save_jar_caches(&mut typer);
    if typer.diags.items.iter().any(|d| !d.is_warning) || typer.diags.syntax_errors {
        let (shown, _, _) = typer.report_diags();
        return Err(shown);
    }
    let files = sources.as_slice();
    let expanded: Vec<FileId> = typer.index.as_ref().map(|i| i.expanded.clone()).unwrap_or_default();
    let withheld = typer.unused_withheld();
    // A file's own AST stands at its index, its package blocks' after the files, each with a file entry of its own
    // (`frontend::lay_out`).
    let block_entries: Vec<usize> = blocks_of.iter().flatten().copied().collect();
    let mut out = Vec::new();
    for (i, f) in files.iter().enumerate() {
        if f.is_std || block_entries.contains(&i) || !editable.contains(&f.path) {
            continue;
        }
        // The file's entries: its own and its package blocks', which an expansion's site and a diagnostic name.
        let entries: Vec<usize> = std::iter::once(i).chain(blocks_of.get(i).into_iter().flatten().copied()).collect();
        let ours = |f: FileId| entries.contains(&(f.0 as usize));
        let file_asts: Vec<&crate::ast::Ast> = entries.iter().map(|&b| &asts[b]).collect();
        let first = file_asts[0];
        let Some(kept) = first.notes.source.as_deref() else { continue };
        // A script's document is its own text: its tokens and nodes inside it, those of the wrapper left out.
        let script = written.get(&f.path);
        let text: &str = script.map_or(&f.text, String::as_str);
        let own = script.map(|s| crate::ast::SourceKept {
            tokens: kept.tokens.iter().filter(|t| t.span.end as usize <= s.len()).copied().collect(),
            end_markers: kept.end_markers.iter().filter(|m| m.end as usize <= s.len()).copied().collect(),
        });
        let tokens = super::source::editable(text, own.as_ref().unwrap_or(kept)).map_err(|at| format!("teq fix: {}: the source the parser kept misses the byte at {}\n", f.path, at))?;
        let mut tree = nodes::build(&f.text, &file_asts, kept, typer.interner);
        if let Some(s) = script {
            tree = tree.within(s.len() as u32);
        }
        if std::env::var_os("TEQ_FIX_TREE").is_some() {
            eprint!("{}:\n{}", f.path, tree.show(text));
        }
        let (imports, importee_of) = imports_of(&file_asts, &tree);
        let unused = if !semantic {
            Err("no semantic rule is configured, so the run reads no unused imports".to_string())
        } else if withheld {
            Err("the analysis is withheld after an error".to_string())
        } else {
            let mut nodes_of = Vec::new();
            for d in typer.diags.items.iter().filter(|d| ours(d.file) && d.unnecessary && d.msg == "unused import") {
                match tree.nodes.iter().position(|n| n.span == d.span && is_importee(n.kind)) {
                    Some(n) => nodes_of.push(n as u32),
                    None => return Err(format!("teq fix: {}: an unused import at {:?} is no importee of the file's tree\n", f.path, d.span)),
                }
            }
            nodes_of.sort();
            nodes_of.dedup();
            Ok(nodes_of)
        };
        out.push(DocView {
            gen: first_gen + out.len() as u32,
            path: shown_path(display, &f.path),
            file: f.path.clone(),
            text: text.to_string(),
            utf16: Utf16::of(text),
            tokens,
            tree,
            unused,
            expanded: expanded.iter().any(|&e| ours(e)),
            imports,
            importee_of,
        });
    }
    Ok(out)
}

fn is_importee(k: K) -> bool {
    matches!(k, K::ImporteeName | K::ImporteeRename | K::ImporteeUnimport | K::ImporteeWildcard | K::ImporteeGiven | K::ImporteeGivenAll)
}

/// The file's import selectors in source order, each once (a package block's AST copies its file's), and each
/// importee node's selector.
fn imports_of(asts: &[&crate::ast::Ast], tree: &nodes::Tree) -> (Vec<crate::ast::Import>, Vec<(u32, usize)>) {
    let mut sels: Vec<crate::ast::Import> = Vec::new();
    for a in asts {
        for s in a.imports.iter().chain(&a.local_imports).chain(&a.language_imports) {
            if !sels.iter().any(|x| x.span == s.span && x.selector_span == s.selector_span) {
                sels.push(s.clone());
            }
        }
    }
    sels.sort_by_key(|s| (s.span.start, s.selector_span.start));
    let mut of = Vec::new();
    for (k, s) in sels.iter().enumerate() {
        if let Some(n) = tree.nodes.iter().position(|n| n.span == s.selector_span && is_importee(n.kind)) {
            of.push((n as u32, k));
        }
    }
    (sels, of)
}

/// The lints as the compiler prints its diagnostics (`source::render_shown`): `<file>:<line>:<col>: <severity>:
/// [<id>]: <message>`, the line and carets under it.
fn print_lints(shown: &[(String, String, Printed)]) {
    if shown.is_empty() {
        return;
    }
    let mut files: Vec<SourceFile> = Vec::new();
    let mut items: Vec<crate::source::Shown> = Vec::new();
    for (path, text, l) in shown {
        let file = match files.iter().position(|f| &f.path == path) {
            Some(i) => i,
            None => {
                files.push(SourceFile { path: path.clone(), text: text.clone(), is_std: false, key: path.clone() });
                files.len() - 1
            }
        };
        let msg = if l.message.is_empty() || l.message.starts_with('\n') { format!("[{}]:{}", l.id, l.message) } else { format!("[{}]: {}", l.id, l.message) };
        let level = match l.severity {
            Severity::Error => crate::source::Level::Error,
            Severity::Warning => crate::source::Level::Warning,
            Severity::Info => crate::source::Level::Info,
        };
        let d = crate::source::Diagnostic::plain(FileId(file as u32), l.span, msg, level != crate::source::Level::Error, false);
        items.push(crate::source::Shown { d: std::borrow::Cow::Owned(d), level, order: 0 });
    }
    eprint!("{}", crate::source::render_shown(&items, &files, &[]));
}

/// Each changed file to a temporary file beside it, then each over its original: a failure before the renames
/// leaves every original as it was.
fn write_all(writes: &[(String, String)]) -> Result<(), String> {
    let mut temps: Vec<(PathBuf, &str)> = Vec::new();
    for (path, text) in writes {
        let p = Path::new(path);
        let temp = p.with_file_name(format!(".{}.teq-fix", p.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned())));
        if let Err(e) = std::fs::write(&temp, text) {
            for (t, _) in &temps {
                let _ = std::fs::remove_file(t);
            }
            let _ = std::fs::remove_file(&temp);
            return Err(format!("cannot write {}: {}", path, e));
        }
        if let Ok(meta) = std::fs::metadata(p) {
            let _ = std::fs::set_permissions(&temp, meta.permissions());
        }
        temps.push((temp, path));
    }
    for (k, (temp, path)) in temps.iter().enumerate() {
        if let Err(e) = std::fs::rename(temp, path) {
            for (t, _) in &temps[k..] {
                let _ = std::fs::remove_file(t);
            }
            return Err(format!("cannot replace {}: {}", path, e));
        }
    }
    Ok(())
}

/// scalafix's `LintConfig`: the severity a lint's full id is given, `lint.error` before `lint.warning` before
/// `lint.info`, each a `FilterMatcher` of includes and excludes; `lint.ignore` drops a lint.
struct LintConfig {
    error: Option<Filter>,
    warning: Option<Filter>,
    info: Option<Filter>,
    ignore: Option<Filter>,
}

/// A `FilterMatcher`: its includes and its excludes, a regex each or none (`mkRegexp(Nil)`, which matches nothing).
struct Filter {
    includes: Option<crate::interp::regex::Regex>,
    excludes: Option<crate::interp::regex::Regex>,
}

impl Filter {
    /// `FilterMatcher.reader` (`scalafix/internal/config/FilterMatcher.scala` at `41cf798`): a string is the includes'
    /// regex; a list, its strings' alternation (`mkRegexp`); an object, its `include`/`includes` and
    /// `exclude`/`excludes` regexes, everything included (`matchEverything`'s `.*`) and nothing excluded where it names
    /// none. A regex that does not compile, or a value of another kind, is an error at its setting, as the reader's
    /// fallible decoding has it.
    fn of(c: &Conf, n: u32) -> Result<Filter, String> {
        use super::config::Value;
        let regex = |at: u32, pattern: &str| crate::interp::regex::Regex::compile(pattern).map(Some).map_err(|e| format!("{}: {}", c.place(at), e));
        let field = |names: [&str; 2]| -> Result<Option<Option<crate::interp::regex::Regex>>, String> {
            match names.iter().find_map(|k| c.field(n, k)) {
                None => Ok(None),
                Some(i) => match &c.nodes[i as usize].value {
                    Value::Str(s) => regex(i, s).map(Some),
                    _ => Err(format!("{}: expected a regex", c.place(i))),
                },
            }
        };
        match &c.nodes[n as usize].value {
            Value::Str(s) => Ok(Filter { includes: regex(n, s)?, excludes: None }),
            Value::List(_) => {
                let strings = c.strings(n)?;
                let includes = match &strings[..] {
                    [] => None,
                    [one] => regex(n, one)?,
                    _ => regex(n, &format!("({})", strings.join("|")))?,
                };
                Ok(Filter { includes, excludes: None })
            }
            Value::Obj(_) => {
                let includes = match field(["include", "includes"])? {
                    Some(r) => r,
                    None => regex(n, ".*")?,
                };
                let excludes = field(["exclude", "excludes"])?.flatten();
                Ok(Filter { includes, excludes })
            }
            _ => Err(format!("{}: expected a regex, a list of them or an object of includes and excludes", c.place(n))),
        }
    }

    /// `FilterMatcher.matches`: the includes found in the id and the excludes not.
    fn matches(&self, id: &str) -> bool {
        self.includes.as_ref().is_some_and(|r| super::escape::found(r, id)) && !self.excludes.as_ref().is_some_and(|r| super::escape::found(r, id))
    }
}

impl LintConfig {
    fn of(c: &Conf) -> Result<LintConfig, String> {
        let f = |k: &str| c.at(&["lint", k]).map(|n| Filter::of(c, n)).transpose();
        Ok(LintConfig { error: f("error")?, warning: f("warning")?, info: f("info")?, ignore: f("ignore")? })
    }

    /// The severity of a lint of `id`, its own unless configured; none for an ignored one.
    fn severity(&self, id: &str, own: Severity) -> Option<Severity> {
        if self.ignore.as_ref().is_some_and(|f| f.matches(id)) {
            return None;
        }
        if self.error.as_ref().is_some_and(|f| f.matches(id)) {
            Some(Severity::Error)
        } else if self.warning.as_ref().is_some_and(|f| f.matches(id)) {
            Some(Severity::Warning)
        } else if self.info.as_ref().is_some_and(|f| f.matches(id)) {
            Some(Severity::Info)
        } else {
            Some(own)
        }
    }
}

// ---- the rule thread ----

/// The program the rules run in: the std's `teq.fix` named, so that its files enter, and the `file:` sources.
const ENTRY: &str = "package teq.fix.program\n\nobject Entry:\n  val driver: teq.fix.Driver.type = teq.fix.Driver\n";

fn rule_thread(conf: Conf, config_dir: PathBuf, requested: Vec<String>, rx: mpsc::Receiver<ToRules>, tx: mpsc::Sender<FromRules>) {
    crate::interp::fix::want();
    host::install(Host { doc: None, results: Results::default(), conf: conf.clone(), read: vec![false; conf.nodes.len()], groups: 0 });
    let sources: Vec<(String, PathBuf)> = requested.iter().filter_map(|r| r.strip_prefix("file:").map(|p| (r.clone(), config_dir.join(p)))).collect();
    let mut files: Vec<PathBuf> = Vec::new();
    for (_, p) in &sources {
        files.extend(scala_files(p));
    }
    let temp = std::env::temp_dir().join(format!("teq-fix-{}", std::process::id()));
    let entry = temp.join("entry.scala");
    if let Err(e) = std::fs::create_dir_all(&temp).and_then(|_| std::fs::write(&entry, ENTRY)) {
        let _ = tx.send(FromRules::Ready(Err(format!("teq fix: cannot write {}: {}", entry.display(), e))));
        return;
    }
    let mut args = vec![entry.to_string_lossy().into_owned()];
    args.extend(files.iter().map(|f| f.to_string_lossy().into_owned()));
    let mut opts = crate::parse_file_options("interp", &args);
    opts.fix_std = true;
    let (paths, roots, _) = crate::collect_program_paths_with_roots(&opts);
    let program = match crate::read_program(&opts, paths, &roots) {
        Ok(p) => p,
        Err(e) => {
            let _ = tx.send(FromRules::Ready(Err(e)));
            return;
        }
    };
    let _ = std::fs::remove_dir_all(&temp);
    let mut interner = crate::intern::Interner::new();
    let mut diags = crate::source::Diagnostics::default();
    let cp = crate::open_jars(&opts);
    let (mut sfiles, mut asts, blocks_of, std, selected, scalajs_unlocked) = crate::parse_build(&opts, cp.as_ref(), program, &mut interner, &mut diags);
    if diags.has_errors() {
        let _ = tx.send(FromRules::Ready(Err(diags.render(&sfiles))));
        return;
    }
    let (jar_files, jdk_file) = crate::jar_pseudo_files(&opts, &mut sfiles, &mut asts);
    let asts = crate::ast::Asts::new(asts);
    let sources_all = crate::source::Sources::new(sfiles);
    let mut typer = crate::typer::Typer::new(&asts, &sources_all, &mut interner);
    typer.threads = 1;
    crate::install_std(&mut typer, std, &selected, &blocks_of, scalajs_unlocked);
    crate::open_classpath(&opts, &mut typer, cp, jar_files, jdk_file);
    crate::configure_typer(&mut typer, &opts, &asts, &sources_all);
    typer.choose_entry = false;
    typer.run();
    if typer.diags.items.iter().any(|d| !d.is_warning) {
        let (shown, _, _) = typer.report_diags();
        let _ = tx.send(FromRules::Ready(Err(shown)));
        return;
    }
    let rule_files: Vec<FileId> = sources_all.as_slice().iter().enumerate().filter(|(_, f)| files.iter().any(|p| p.to_string_lossy() == f.path)).map(|(i, _)| FileId(i as u32)).collect();
    let mut w = typer.w;
    let found = (|| -> Result<_, String> {
        let driver = w.class_at(&["teq", "fix", "Driver"]).ok_or("teq fix: the std has no teq.fix.Driver")?;
        let rule_class = w.class_at(&["teq", "fix", "Rule"]).ok_or("teq fix: the std has no teq.fix.Rule")?;
        let member = |w: &mut crate::typer::Worker, name: &str| -> Result<crate::types::SymId, String> {
            let n = w.interner.intern(name);
            w.syms.class(driver).members.get(&n).copied().ok_or_else(|| format!("teq fix: teq.fix.Driver has no {}", name))
        };
        let syms = [member(&mut w, "shipped")?, member(&mut w, "name")?, member(&mut w, "isSemantic")?, member(&mut w, "configure")?, member(&mut w, "run")?];
        // The rules of the `file:` sources: their concrete top-level classes that are rules.
        let mut project: Vec<(crate::types::ClassId, FileId)> = Vec::new();
        for c in 0..w.syms.classes.len() as u32 {
            let c = crate::types::ClassId(c);
            let info = w.syms.class(c);
            if !rule_files.contains(&info.file) || info.kind != crate::symbols::ClassKind::Class || info.mods & crate::ast::mods::ABSTRACT != 0 {
                continue;
            }
            if !matches!(info.owner, crate::symbols::Owner::Package(_)) {
                continue;
            }
            let file = info.file;
            if w.is_subclass_of(c, rule_class) {
                project.push((c, file));
            }
        }
        Ok((driver, syms, project))
    })();
    let (driver, syms, project) = match found {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(FromRules::Ready(Err(e)));
            return;
        }
    };
    let file_of: Vec<(crate::types::ClassId, String)> = project.iter().map(|&(c, f)| (c, sources_all.as_slice()[f.0 as usize].path.clone())).collect();
    let mut interp = crate::interp::Interp::new(&mut w, crate::interp::Limits::unlimited());
    interp.stream = true;
    interp.content_hashes = false;
    let ready = (|| -> Result<(crate::interp::Value, Vec<crate::interp::Value>, Vec<RuleInfo>), String> {
        let failure = |it: &mut crate::interp::Interp, f: crate::interp::Failure| match f {
            crate::interp::Failure::Thrown(v) => it.describe_value(&v),
            other => it.describe(&other),
        };
        let module = interp.module_value(driver).map_err(|f| failure(&mut interp, f))?;
        let shipped = interp.call_method(module.clone(), syms[0], Vec::new()).map_err(|f| failure(&mut interp, f))?;
        let mut available: Vec<(String, Option<String>, crate::interp::Value)> = Vec::new();
        for r in crate::interp::fix::array_items(&shipped) {
            let name = interp.call_method(module.clone(), syms[1], vec![r.clone()]).map_err(|f| failure(&mut interp, f))?;
            available.push((interp.describe_value(&name), None, r));
        }
        for (c, path) in &file_of {
            let r = interp.new_instance(*c).map_err(|f| format!("teq fix: {}: a rule needs a constructor of no arguments: {}", path, failure(&mut interp, f)))?;
            let name = interp.call_method(module.clone(), syms[1], vec![r.clone()]).map_err(|f| failure(&mut interp, f))?;
            available.push((interp.describe_value(&name), Some(path.clone()), r));
        }
        // The rules asked for, in the configuration's order: a name, or every rule of a `file:` source.
        let mut chosen: Vec<(String, crate::interp::Value)> = Vec::new();
        let mut unknown: Vec<String> = Vec::new();
        for req in &requested {
            if let Some(p) = req.strip_prefix("file:") {
                let dir = config_dir.join(p);
                let of_file: Vec<&(String, Option<String>, crate::interp::Value)> = available.iter().filter(|(_, f, _)| f.as_ref().is_some_and(|f| Path::new(f).starts_with(&dir) || Path::new(f) == dir)).collect();
                if of_file.is_empty() {
                    unknown.push(req.clone());
                }
                for (n, _, v) in of_file {
                    chosen.push((n.clone(), v.clone()));
                }
            } else {
                match available.iter().find(|(n, _, _)| n == req) {
                    Some((n, _, v)) => chosen.push((n.clone(), v.clone())),
                    None => unknown.push(req.clone()),
                }
            }
        }
        if !unknown.is_empty() {
            let known: Vec<&str> = available.iter().map(|(n, _, _)| n.as_str()).collect();
            return Err(format!("teq fix: unknown rule{} {} (the rules: {})", if unknown.len() == 1 { "" } else { "s" }, unknown.join(", "), known.join(", ")));
        }
        let mut configured = Vec::new();
        let mut infos = Vec::new();
        for (n, v) in &chosen {
            let c = interp.call_method(module.clone(), syms[3], vec![v.clone()]).map_err(|f| format!("teq fix: the configuration of {}: {}", n, failure(&mut interp, f)))?;
            let semantic = interp.call_method(module.clone(), syms[2], vec![c.clone()]).map_err(|f| failure(&mut interp, f))?;
            infos.push(RuleInfo { name: n.clone(), semantic: matches!(semantic, crate::interp::Value::Bool(true)) });
            configured.push(c);
        }
        // A setting under a configured rule's section that its decoding never read is unknown (metaconfig's
        // unknown field).
        let unread = host::with_host(|h| unread_settings(h, &infos)).unwrap_or_default();
        if !unread.is_empty() {
            return Err(unread.join("\n"));
        }
        Ok((module, configured, infos))
    })();
    let (module, configured, infos) = match ready {
        Ok(r) => r,
        Err(e) => {
            let _ = tx.send(FromRules::Ready(Err(e)));
            return;
        }
    };
    let _ = tx.send(FromRules::Ready(Ok(infos)));
    let rules = crate::interp::fix::array_of(configured);
    while let Ok(ToRules::Doc(doc, semantic_only)) = rx.recv() {
        let gen = doc.gen;
        host::with_host(|h| {
            h.doc = Some(*doc);
            h.results = Results::default();
        });
        interp.set_limits(crate::interp::Limits::unlimited());
        let r = interp.call_method(module.clone(), syms[4], vec![rules.clone(), crate::interp::Value::Int(gen as i32), crate::interp::Value::Bool(semantic_only)]);
        let failure = match r {
            Ok(_) => None,
            Err(f) => Some(match f {
                crate::interp::Failure::Thrown(v) => interp.describe_value(&v),
                other => interp.describe(&other),
            }),
        };
        let (results, view) = host::with_host(|h| {
            let mut r = std::mem::take(&mut h.results);
            if let Some(f) = failure {
                r.failures.push((0, f));
            }
            (r, h.doc.take())
        })
        .unwrap_or_default();
        let Some(view) = view else { break };
        if tx.send(FromRules::Done(Box::new(results), Box::new(view))).is_err() {
            break;
        }
    }
    drop(interp);
}

/// The `.scala` files of a `file:` entry: the file, or those under the directory.
fn scala_files(p: &Path) -> Vec<PathBuf> {
    if p.is_dir() {
        let mut out = Vec::new();
        if let Ok(entries) = std::fs::read_dir(p) {
            let mut entries: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            entries.sort();
            for e in entries {
                out.extend(scala_files(&e));
            }
        }
        out
    } else if p.extension().is_some_and(|x| x == "scala") || !p.exists() {
        vec![p.to_path_buf()]
    } else {
        Vec::new()
    }
}

/// The settings under the configured rules' sections no decoding read, each at its place.
fn unread_settings(h: &Host, infos: &[RuleInfo]) -> Vec<String> {
    use super::config::Value;
    let mut out = Vec::new();
    fn walk(h: &Host, n: u32, path: &str, out: &mut Vec<String>) {
        match &h.conf.nodes[n as usize].value {
            Value::Obj(fields) => {
                for (k, v) in fields {
                    let p = format!("{}.{}", path, k);
                    if !h.read.get(*v as usize).copied().unwrap_or(false) {
                        out.push(format!("{}: {}: no such setting", h.conf.place(*v), p));
                    } else {
                        walk(h, *v, &p, out);
                    }
                }
            }
            Value::List(items) => {
                for &i in items {
                    walk(h, i, path, out);
                }
            }
            _ => {}
        }
    }
    // The sections the configured rules read: each one's by its name, and any other a rule's decoding read (an alias,
    // `disableSyntax`); a section no configured rule reads is ignored, as scalafix ignores it.
    if let Value::Obj(fields) = &h.conf.nodes[0].value {
        for (i, (k, v)) in fields.iter().enumerate() {
            // A key's last occurrence is its value (`Conf::field`).
            if fields[i + 1..].iter().any(|(later, _)| later == k) {
                continue;
            }
            let configured = infos.iter().any(|r| r.name == *k);
            if configured || h.read.get(*v as usize).copied().unwrap_or(false) {
                walk(h, *v, k, &mut out);
            }
        }
    }
    out
}
