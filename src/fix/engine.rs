//! `teq fix`'s run (docs/FIX.md, "Patches, suppressions and rounds"): the rules compiled and configured on the rule workers, each with an
//! interpreter of its own; each job (a configuration's program, or the files `teq compiler fix` names) typed on a typing
//! thread of its own as `teq compiler check` types it, over compile's workers, a parallel attempt that gives way typed
//! again by one worker within its round; the round's typing and its rules never at once, every rule worker ready
//! before a typing begins; where a semantic rule is configured the typing thread then serves the rules' SemanticDB
//! questions until the round's last document is done (`serve`: a typing of its own by one worker, made at the first
//! question); each editable document's view handed to a worker, which runs every rule over it, the results gathered in the documents' order; the patches planned, all of them before anything
//! is written; at most two rounds, the second over the first's text, typed again, for the imports the first's edits
//! left unused. A run whose rules teq ships takes as many workers as `--threads` (the cores by default) and a round's
//! documents allow; one with a project rule (`file:`) keeps one worker, whose rule instances keep their state across
//! documents.

use super::config::Conf;
use super::host::{self, Asker, DocView, Host, Results, Severity, Utf16};
use super::nodes::{self, K};
use super::plan::{self, Plan, Printed};
use super::service::{Answer as SemAnswer, Ask, SemQuery, SemReply};
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
    Doc(Box<DocView>, Handout),
}

/// What comes with a document: whether its semantic rules alone run over it, who asks (the host's `asker`), the
/// milliseconds its answer waits before it is sent (`TEQ_FIX_SCHEDULE`'s shuffle, else none), and whether it is the
/// worker's first of the round, where `TEQ_FIX_BARRIER` holds it.
struct Handout {
    semantic_only: bool,
    asker: Asker,
    delay: u64,
    first: bool,
    /// The round's SemanticDB service, where a semantic rule is configured (`serve`).
    link: Option<mpsc::Sender<SemQuery>>,
}

enum FromRules {
    /// A worker's rules compiled and configured, or why not, and the microseconds that took.
    Ready(usize, Result<Vec<RuleInfo>, String>, u64),
    Done(Box<Answer>),
    /// A worker that panicked.
    Ended,
}

/// A document's results as its worker hands them back.
struct Answer {
    asker: Asker,
    results: Results,
    /// The document, handed back for the planner.
    view: DocView,
    /// The atomic groups the document's rules made, numbered from 1 there (`Native.newGroup`).
    groups: u32,
    /// When its rules began and ended (`now_us`), and the worker thread's CPU time in between, in microseconds: the
    /// work, whatever the machine's other load leaves the thread of the wall time.
    start: u64,
    finish: u64,
    cpu: u64,
    /// Whether the worker waited for `TEQ_FIX_BARRIER`'s workers in vain.
    barrier_timeout: bool,
    /// The SemanticDB questions its rules asked (`fix::service`).
    queries: u32,
}

#[derive(Clone, PartialEq)]
struct RuleInfo {
    name: String,
    semantic: bool,
    /// Whether teq ships the rule (`Driver.shipped`), not a `file:` entry's.
    shipped: bool,
}

/// What a worker needs to compile and configure the rules.
#[derive(Clone)]
struct Boot {
    conf: Conf,
    config_dir: PathBuf,
    requested: Vec<String>,
}

/// The rule workers: threads of the fix engine's own, so that none carries a compile worker's builtin table or caches,
/// each with the rules' program typed, its interpreter and its host. The lead, worker 0, starts with the run; the
/// others when a round's documents can use them, once its typing has ended. Dropping the pool closes their channels,
/// which ends them.
struct Pool<'scope, 'env> {
    scope: &'scope std::thread::Scope<'scope, 'env>,
    boot: Boot,
    to: Vec<mpsc::Sender<ToRules>>,
    back: mpsc::Sender<FromRules>,
    from: mpsc::Receiver<FromRules>,
    /// Whether each worker answered `Ready`.
    ready: Vec<bool>,
    /// The workers the run may use: one where a project rule is configured (`drive_rounds`).
    limit: usize,
    eligible: bool,
    gate: std::sync::Arc<Gate>,
}

impl<'scope, 'env> Pool<'scope, 'env> {
    fn new(scope: &'scope std::thread::Scope<'scope, 'env>, boot: Boot) -> Result<Self, String> {
        let (back, from) = mpsc::channel::<FromRules>();
        let k = std::env::var("TEQ_FIX_BARRIER").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        let gate = std::sync::Arc::new(Gate { k, begun: std::sync::Mutex::new(((u32::MAX, u32::MAX), 0)) });
        let mut pool = Pool { scope, boot, to: Vec::new(), back, from, ready: Vec::new(), limit: 1, eligible: false, gate };
        pool.add_worker().map(|()| pool)
    }

    /// One more worker, or why the system gave it no thread (its stack's address space): the pool then keeps the
    /// workers it has, which the documents' order makes no difference to.
    fn add_worker(&mut self) -> Result<(), String> {
        let w = self.to.len();
        let (tx, rx) = mpsc::channel::<ToRules>();
        let (back, boot, gate) = (self.back.clone(), self.boot.clone(), self.gate.clone());
        std::thread::Builder::new()
            .name(format!("teq-fix-rules-{}", w))
            .stack_size(1 << 30)
            .spawn_scoped(self.scope, move || {
                crate::alloc::enter();
                rule_thread(w, boot, rx, back, &gate)
            })
            .map_err(|e| format!("the system gave the rule worker {} no thread ({})", w, e))?;
        self.to.push(tx);
        self.ready.push(false);
        record(format_args!("worker-start w={} at_us={}", w, now_us()));
        Ok(())
    }
}

/// `TEQ_FIX_BARRIER=<k>`, the fix suite's: each worker waits at its first document of a round until `k` workers have
/// begun one of that round, so that the record shows several in rule evaluation before any finished, in every round.
struct Gate {
    k: usize,
    /// The round under way, its job and number, and the workers that began a document of it.
    begun: std::sync::Mutex<((u32, u32), usize)>,
}

impl Gate {
    /// At a worker's first document of `round`: waits until `k` workers have begun one, ten seconds at most; whether
    /// in vain.
    fn pass(&self, round: (u32, u32)) -> bool {
        if self.k <= 1 {
            return false;
        }
        {
            let mut b = self.begun.lock().unwrap_or_else(|e| e.into_inner());
            if b.0 != round {
                *b = (round, 0);
            }
            b.1 += 1;
        }
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let b = *self.begun.lock().unwrap_or_else(|e| e.into_inner());
            if b.0 != round || b.1 >= self.k {
                return false;
            }
            if std::time::Instant::now() > until {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}

/// The order a round's documents are handed out in (`TEQ_FIX_SCHEDULE`, the fix suite's perturbations): the documents'
/// own; `reverse`, the last first and the first held until a later one is done, so that the completion order is
/// another; `shuffle:<seed>`, a seeded permutation with each answer delayed a seeded 0 to 3 ms. Completion order never
/// decides an output's order.
#[derive(Clone, Copy, PartialEq)]
enum Schedule {
    Natural,
    Reverse,
    Shuffle(u64),
}

impl Schedule {
    fn of_env() -> Schedule {
        match std::env::var("TEQ_FIX_SCHEDULE").ok().as_deref() {
            Some("reverse") => Schedule::Reverse,
            Some(s) if s.starts_with("shuffle:") => Schedule::Shuffle(s["shuffle:".len()..].parse().unwrap_or(1)),
            _ => Schedule::Natural,
        }
    }

    fn order(self, n: usize) -> Vec<usize> {
        let mut order: Vec<usize> = (0..n).collect();
        match self {
            Schedule::Natural => {}
            Schedule::Reverse => order.reverse(),
            Schedule::Shuffle(seed) => {
                let mut x = seed;
                for i in (1..n).rev() {
                    x = mix(x);
                    order.swap(i, (x % (i as u64 + 1)) as usize);
                }
            }
        }
        order
    }

    fn delay(self, slot: usize) -> u64 {
        match self {
            Schedule::Shuffle(seed) => mix(seed ^ (slot as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)) % 4,
            _ => 0,
        }
    }
}

/// splitmix64's step.
fn mix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// `TEQ_FIX_PLANT`, the mutation controls of the fix suite's witness (tests/fix.scala, `Parallel`), a test-only switch
/// that nothing else reads: `serial` hands one document at a time to the lead alone while the workers are started,
/// which the record's check must refuse; `completion` consumes the documents in the order they are done, which the byte
/// comparison must refuse.
fn plant() -> Option<String> {
    std::env::var("TEQ_FIX_PLANT").ok()
}

/// The workers a run whose rules teq ships may take: the jobs' `--threads` (or `TEQ_THREADS`), else the machine's cores
/// (`TEQ_WORKERS` bounding them); `TEQ_FIX_RULE_THREADS=<n>`, the fix suite's, the rules' count apart from the typing's.
fn requested_workers(jobs: &[Job]) -> usize {
    if let Some(n) = std::env::var("TEQ_FIX_RULE_THREADS").ok().and_then(|v| v.parse::<usize>().ok()).filter(|&n| n >= 1) {
        return n;
    }
    let asked = jobs
        .iter()
        .filter_map(|j| {
            let mut args = vec!["compiler".to_string(), "check".to_string()];
            args.extend(j.args.iter().cloned());
            crate::frontend::requested_threads(crate::parse_options_from(args.into_iter()).threads)
        })
        .max();
    asked.unwrap_or_else(crate::workers).max(1)
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
    // `TEQ_FIX_INPUTS=<file>`: each job's inputs as the compiler resolves them (`program_paths`: the roots, the links
    // followed, the exclusions applied), a line `<job>\t<path>` each, written and nothing else done: the corpus a
    // measurement runs over, as bench/fix/run.sh takes its identity.
    if let Some(file) = std::env::var_os("TEQ_FIX_INPUTS") {
        let mut out = String::new();
        for (k, job) in jobs.iter().enumerate() {
            for p in program_paths(job) {
                out.push_str(&format!("{}\t{}\n", k, p));
            }
        }
        return match std::fs::write(&file, out) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("teq fix: cannot write the inputs {}: {}", Path::new(&file).display(), e);
                2
            }
        };
    }
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
    record_open();
    // The threads the typings' merges drop what they gave back on are waited for between typings (`round`).
    crate::crew::keep_given_back();
    let boot = Boot { conf: setup.conf.clone(), config_dir: setup.config_dir.clone(), requested };
    std::thread::scope(|scope| {
        let mut pool = match Pool::new(scope, boot) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("teq fix: {}", e);
                record_close();
                return 2;
            }
        };
        let code = drive(&setup, &lints, &jobs, &mut pool);
        drop(pool);
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

fn drive(setup: &Setup, lint_config: &LintConfig, jobs: &[Job], pool: &mut Pool<'_, '_>) -> i32 {
    let started = std::time::Instant::now();
    let code = drive_rounds(setup, lint_config, jobs, pool);
    record_close();
    if std::env::var_os("TEQ_FIX_TRACE").is_some() {
        TIMES.with(|t| {
            let (t, held) = (t.get(), crate::alloc::held());
            eprintln!("fix: {:.0} ms in all: the rules' program {:.0} ms, typing {:.0} ms, the rules {:.0} ms on {} worker{} (their documents {:.0} ms, {:.0} ms of CPU, the longest {:.0} ms), planning {:.0} ms, the SemanticDB answers {:.0} ms; the allocator reserved {} MB, its requests over 32 KB at most {} MB", started.elapsed().as_secs_f64() * 1e3, t[0] * 1e3, t[1] * 1e3, t[2] * 1e3, pool.to.len(), if pool.to.len() == 1 { "" } else { "s" }, t[4] * 1e3, t[6] * 1e3, t[5] * 1e3, t[3] * 1e3, t[7] * 1e3, held.reserved >> 20, held.large_peak >> 20);
        });
    }
    code
}

thread_local! {
    /// Seconds the run spent: making the rules' program ready, typing, in the rules (the rounds' wall time, from a
    /// round's first document handed out to its last one planned), planning (which overlaps the rules' wall time), the
    /// documents' rules summed over the workers, the longest document's, the documents' CPU time, and the SemanticDB
    /// answers (a round's service from its first question to its last answer, its typing among them) (`TEQ_FIX_TRACE`).
    static TIMES: std::cell::Cell<[f64; 8]> = const { std::cell::Cell::new([0.0; 8]) };
}

/// The run's epoch, which the coordinator's and the rule thread's clock readings of the execution record share.
static EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Microseconds since the run's epoch.
fn now_us() -> u64 {
    EPOCH.get_or_init(std::time::Instant::now).elapsed().as_micros() as u64
}

thread_local! {
    /// The execution record, `TEQ_FIX_RECORD=<file>` (docs/FIX.md, "Debugging a rule"): what the run dispatched to
    /// which worker and when each document's rules began and ended, written by the coordinator alone.
    static RECORD: std::cell::RefCell<Option<std::io::BufWriter<std::fs::File>>> = const { std::cell::RefCell::new(None) };
}

fn record_open() {
    now_us();
    if let Some(path) = std::env::var_os("TEQ_FIX_RECORD") {
        match std::fs::File::create(&path) {
            Ok(f) => RECORD.with(|r| *r.borrow_mut() = Some(std::io::BufWriter::new(f))),
            Err(e) => eprintln!("teq fix: cannot write the record {}: {}", std::path::Path::new(&path).display(), e),
        }
    }
}

fn record(line: std::fmt::Arguments<'_>) {
    use std::io::Write;
    RECORD.with(|r| {
        if let Some(w) = r.borrow_mut().as_mut() {
            let _ = writeln!(w, "{}", line);
        }
    });
}

fn record_close() {
    use std::io::Write;
    RECORD.with(|r| {
        if let Some(mut w) = r.borrow_mut().take() {
            let _ = w.flush();
        }
    });
}

fn timed(k: usize, since: std::time::Instant) {
    add_time(k, since.elapsed().as_secs_f64());
}

fn add_time(k: usize, d: f64) {
    TIMES.with(|t| {
        let mut v = t.get();
        if k == 5 {
            v[k] = v[k].max(d);
        } else {
            v[k] += d;
        }
        t.set(v);
    });
}

fn drive_rounds(setup: &Setup, lint_config: &LintConfig, jobs: &[Job], pool: &mut Pool<'_, '_>) -> i32 {
    let ready = std::time::Instant::now();
    let infos = match pool.from.recv() {
        Ok(FromRules::Ready(_, Ok(infos), boot)) => {
            record(format_args!("worker w=0 ready_us={} boot_us={}", now_us(), boot));
            infos
        }
        Ok(FromRules::Ready(_, Err(e), _)) => {
            eprintln!("{}", e.trim_end());
            return 2;
        }
        _ => {
            eprintln!("teq fix: the rule thread ended");
            return 2;
        }
    };
    timed(0, ready);
    pool.ready[0] = true;
    // The rules teq ships keep no state across documents (fix-speed's table, §1): their run takes several workers. A
    // project rule's instance may (Count's visits, Stale's kept token): its run keeps the lead alone.
    pool.eligible = infos.iter().all(|i| i.shipped);
    pool.limit = if pool.eligible { requested_workers(jobs) } else { 1 };
    let mut base = 0u32;
    let names: Vec<String> = infos.iter().map(|i| i.name.clone()).collect();
    let semantic_rules: Vec<bool> = infos.iter().map(|i| i.semantic).collect();
    // A semantic rule configured, or the SemanticDB dump: the typings read the unused imports (`-Wunused:imports`),
    // which the record says, so that a comparison with `teq compiler check` asks the same of it (bench/fix/parity.sh).
    let semantic = semantic_rules.contains(&true) || semanticdb_dump().is_some();
    record(format_args!("run eligible={} workers={} semantic={}", pool.eligible as u8, pool.limit, semantic as u8));
    let mut gen = 0u32;
    // Round one, over the files as read; the rules' results kept for round two.
    let mut first: Vec<(usize, Vec<Outcome>)> = Vec::new();
    let mut kept: HashMap<String, Results> = HashMap::new();
    for (k, job) in jobs.iter().enumerate() {
        let earlier = Earlier { results: &HashMap::new(), semantic: &semantic_rules };
        match round((k, 1), job, &HashMap::new(), semantic, &mut gen, &mut base, &names, &infos, &setup.display, pool, earlier, Some(&mut kept)) {
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
            match round((*k, 2), &jobs[*k], &changed, semantic, &mut gen, &mut base, &names, &infos, &setup.display, pool, earlier, None) {
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
        // In the paths' order, so that the file a refusal names is the same in every run.
        let mut second: Vec<(String, Outcome)> = second.into_iter().collect();
        second.sort_by(|a, b| a.0.cmp(&b.0));
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

/// A round of `job`: its program typed with `overrides` standing for files, its editable documents handed to the
/// workers, each document's results planned in the documents' order (`consume`); round one keeps each file's results
/// in `kept`. `base` numbers the atomic groups as one counter over the whole run would: each document's are numbered
/// from 1 on its worker, then offset by the groups of the documents planned before it.
#[allow(clippy::too_many_arguments)]
fn round((k, r): (usize, u32), job: &Job, overrides: &HashMap<String, String>, semantic: bool, gen: &mut u32, base: &mut u32, names: &[String], infos: &[RuleInfo], display: &Option<(PathBuf, PathBuf)>, pool: &mut Pool<'_, '_>, earlier: Earlier<'_>, mut kept: Option<&mut HashMap<String, Results>>) -> Result<Vec<Outcome>, String> {
    let first_gen = *gen;
    // Every rule worker started is ready before a typing begins: its bootstrap types the rules' program at one worker,
    // which sets the interpreter's process-wide watch (`interp::WATCHING`) a typing of several workers depends on.
    while let Some(w) = pool.ready.iter().position(|&ready| !ready) {
        match pool.from.recv() {
            Ok(FromRules::Ready(w, Ok(theirs), boot)) => {
                if theirs != infos {
                    return Err(format!("teq fix: the rule worker {} configured the rules otherwise than the first\n", w));
                }
                record(format_args!("worker w={} ready_us={} boot_us={}", w, now_us(), boot));
                pool.ready[w] = true;
            }
            Ok(FromRules::Ready(_, Err(e), _)) => return Err(format!("{}\n", e.trim_end())),
            Ok(FromRules::Done(_)) => return Err(format!("teq fix: the results of a document of another round, before worker {} was ready\n", w)),
            Ok(FromRules::Ended) | Err(_) => return Err("teq fix: the rule thread ended\n".to_string()),
        }
    }
    let typing = std::time::Instant::now();
    // The round's typing thread owns the round's typing (tests/semanticdb/TABLE.md, ruling 1): it types the round and
    // hands back its attempts and documents; where a semantic rule is configured it then serves the rules' SemanticDB
    // questions (`serve`) until the round drops its link after the last document, and the round joins it there; else
    // the round joins it at once.
    let (link, questions) = if infos.iter().any(|i| i.semantic) {
        let (link, questions) = mpsc::channel::<SemQuery>();
        (Some(link), Some(questions))
    } else {
        (None, None)
    };
    let (handed_back, typed_back) = mpsc::channel();
    let owned = (job.clone(), overrides.clone(), display.clone());
    let thread = std::thread::Builder::new().name("teq-fix-typer".to_string()).stack_size(1 << 30).spawn_scoped(pool.scope, move || {
        crate::alloc::enter();
        let (job, overrides, display) = owned;
        let (attempts, docs, texts) = typed(&job, &overrides, semantic, first_gen, &display);
        let served: Option<Vec<(u32, String)>> = docs.as_ref().ok().map(|d| d.iter().map(|v| (v.gen, v.file.clone())).collect());
        let _ = handed_back.send((attempts, docs));
        match (questions, served) {
            (Some(questions), Some(served)) => serve(&job, &overrides, &display, first_gen, served, questions, texts),
            _ => Served::default(),
        }
    });
    let (attempts, docs, mut owner) = match thread {
        Ok(t) => match typed_back.recv() {
            Ok((attempts, docs)) => (attempts, docs, Some(t)),
            Err(_) => {
                let _ = t.join();
                (Vec::new(), Err("teq fix: the typing thread panicked\n".to_string()), None)
            }
        },
        Err(e) => (Vec::new(), Err(format!("teq fix: the system gave the typing thread no thread ({})\n", e)), None),
    };
    // A round no rule asks about, or one that did not type: its typing thread ends now.
    if link.is_none() || docs.is_err() {
        if let Some(t) = owner.take() {
            t.join().map_err(|_| "teq fix: the typing thread panicked\n".to_string())?;
        }
    }
    // What the merges gave back to be dropped on threads of their own (`crew::give_back`), dropped before anything
    // follows, so that the next typing finds it.
    let waiting = std::time::Instant::now();
    crate::crew::given_back();
    // A resident process's mappings that sat at the centre through the whole typing go back to the system, as at a
    // session's idle point between builds.
    if crate::alloc::is_resident() {
        crate::alloc::age();
    }
    let waited = waiting.elapsed();
    timed(1, typing);
    record_attempts(k, r, &attempts, waited);
    let docs = docs?;
    let n = docs.len();
    *gen = first_gen + n as u32;
    record(format_args!("round job={} round={} docs={} first_gen={}", k, r, n, first_gen));
    // The workers this round takes, started now that its typing has ended: no worker's interpreter runs beside a typing
    // (`interp::WATCHING`, which a typer of several workers sets, is process-wide).
    let want = if pool.limit > 1 && n > 1 { pool.limit.min(n) } else { 1 };
    if want == 1 {
        let reason = if !pool.eligible { "project-rules" } else if pool.limit == 1 { "one-worker" } else { "one-document" };
        record(format_args!("serial reason={}", reason));
    }
    while pool.to.len() < want {
        if let Err(e) = pool.add_worker() {
            // The rounds after this one keep to the workers the system gave.
            record(format_args!("worker-refused w={} why={}", pool.to.len(), e));
            pool.limit = pool.to.len();
            break;
        }
    }
    let schedule = Schedule::of_env();
    let plant = plant();
    let (serial_plant, completion_plant) = (plant.as_deref() == Some("serial"), plant.as_deref() == Some("completion"));
    // Idle workers, the lowest last so that it is handed a document first.
    let mut idle: Vec<usize> = (0..pool.to.len()).rev().filter(|&w| pool.ready[w] && (!serial_plant || w == 0)).collect();
    let mut pending: std::collections::VecDeque<usize> = schedule.order(n).into();
    let mut docs: Vec<Option<DocView>> = docs.into_iter().map(Some).collect();
    let mut slots: Vec<Option<Box<Answer>>> = (0..n).map(|_| None).collect();
    // The workers handed a document of this round already.
    let mut handed = vec![false; pool.to.len()];
    let (mut next, mut consumed, mut in_flight, mut later_done) = (0usize, 0usize, 0usize, false);
    let (rules_start, round_start, mut service, mut cpu) = (std::time::Instant::now(), now_us(), 0u64, 0u64);
    let mut out = Vec::new();
    let ended = || "teq fix: the rule thread ended\n".to_string();
    while consumed < n {
        while !idle.is_empty() && !(serial_plant && in_flight > 0) {
            // Under `reverse` the first document waits for a later one's results.
            let Some(at) = pending.iter().position(|&s| !(schedule == Schedule::Reverse && s == 0 && n > 1 && !later_done)) else { break };
            let slot = pending.remove(at).expect("a pending document");
            let w = idle.pop().expect("an idle worker");
            let d = docs[slot].take().expect("a document handed out once");
            let semantic_only = !overrides.contains_key(&d.file) && earlier.results.contains_key(&d.file);
            let asker = Asker { job: k as u32, round: r, slot: slot as u32, gen: d.gen, worker: w as u32 };
            record(format_args!("dispatch slot={} gen={} worker={} at_us={}", slot, d.gen, w, now_us()));
            let first = !std::mem::replace(&mut handed[w], true);
            pool.to[w].send(ToRules::Doc(Box::new(d), Handout { semantic_only, asker, delay: schedule.delay(slot), first, link: link.clone() })).map_err(|_| ended())?;
            in_flight += 1;
        }
        match pool.from.recv() {
            Ok(FromRules::Done(a)) => {
                in_flight -= 1;
                let (w, slot) = (a.asker.worker as usize, a.asker.slot as usize);
                if a.asker.job != k as u32 || a.asker.round != r || slot >= n || slots[slot].is_some() || docs[slot].is_some() {
                    return Err(format!("teq fix: the results of a document of another round, or twice (slot {})\n", slot));
                }
                idle.push(w);
                later_done |= slot != 0;
                if a.barrier_timeout {
                    record(format_args!("barrier-timeout worker={}", w));
                }
                record(format_args!("done slot={} gen={} worker={} start_us={} finish_us={} cpu_us={} queries={}", slot, a.asker.gen, w, a.start, a.finish, a.cpu, a.queries));
                let took = a.finish.saturating_sub(a.start);
                (service, cpu) = (service + took, cpu + a.cpu);
                add_time(5, took as f64 / 1e6);
                if completion_plant {
                    out.push(consume(*a, overrides, &earlier, names, base, kept.as_deref_mut()));
                    consumed += 1;
                } else {
                    slots[slot] = Some(a);
                }
            }
            Ok(FromRules::Ready(w, Ok(theirs), boot)) => {
                if theirs != infos {
                    return Err(format!("teq fix: the rule worker {} configured the rules otherwise than the first\n", w));
                }
                record(format_args!("worker w={} ready_us={} boot_us={}", w, now_us(), boot));
                pool.ready[w] = true;
                if !serial_plant {
                    idle.push(w);
                }
            }
            Ok(FromRules::Ready(_, Err(e), _)) => return Err(format!("{}\n", e.trim_end())),
            Ok(FromRules::Ended) | Err(_) => return Err(ended()),
        }
        while next < n {
            let Some(a) = slots[next].take() else { break };
            out.push(consume(*a, overrides, &earlier, names, base, kept.as_deref_mut()));
            next += 1;
            consumed += 1;
        }
    }
    // The round's last document done, its service ends with the round's link, and the round joins its typing thread.
    drop(link);
    if let Some(owner) = owner {
        let served = owner.join().map_err(|_| "teq fix: the typing thread panicked\n".to_string())?;
        record(format_args!("serve job={} round={} queries={} typed={} start_us={} end_us={}", k, r, served.queries, served.typed as u8, served.start_us, served.end_us));
        add_time(7, served.end_us.saturating_sub(served.start_us) as f64 / 1e6);
    }
    timed(2, rules_start);
    add_time(4, service as f64 / 1e6);
    add_time(6, cpu as f64 / 1e6);
    record(format_args!("round-end wall_us={} service_us={} cpu_us={}", now_us() - round_start, service, cpu));
    Ok(out)
}

/// The round's typing attempts in the record and the trace (docs/FIX.md, "Debugging a rule"): each attempt's line, its
/// items under `TEQ_FIX_TYPING_WORK=1`; the trace's line of its parts, `run`'s phases, and the wait for the merges' drops.
fn record_attempts(k: usize, r: u32, attempts: &[Attempt], waited: std::time::Duration) {
    let trace = std::env::var_os("TEQ_FIX_TRACE").is_some();
    for a in attempts {
        let why = if a.why.is_empty() { String::new() } else { format!(" why={}", a.why.replace('\n', " ")) };
        record(format_args!("typing job={} round={} attempt={} asked={} effective={} mapping_refused={} worker_refused={} forked={} outcome={} start_us={} end_us={}{}", k, r, a.number, a.asked, a.effective, a.mapping_refused as u8, a.worker_refused as u8, a.forked as u8, a.outcome, a.start_us, a.end_us, why));
        for (worker, index, t0, t1, file) in &a.items {
            record(format_args!("typing-item job={} round={} attempt={} worker={} index={} t0_us={} t1_us={} file={}", k, r, a.number, worker, index, t0, t1, file));
        }
        if trace {
            let ms = |s: f64| s * 1e3;
            let phases: Vec<String> = a.phases.iter().map(|(n, d)| format!("{} {:.0}", n, d.as_secs_f64() * 1e3)).collect();
            eprintln!(
                "fix: typing job {} round {} attempt {}: {} worker{} asked, {} typed, {}{}: read and parse {:.0} ms, jars {:.0} ms, type {:.0} ms (run: {}), documents {:.0} ms, caches {:.0} ms, exit {:.0} ms; the allocator reserved {} MB at the typer's drop",
                k, r, a.number, a.asked, if a.asked == 1 { "" } else { "s" }, a.effective, a.outcome, if a.why.is_empty() { if a.mapping_refused { " (the overlays' mappings refused)".to_string() } else { String::new() } } else { format!(" ({})", a.why) },
                ms(a.parts[0]), ms(a.parts[1]), ms(a.parts[2]), phases.join(", "), ms(a.parts[3]), ms(a.parts[4]), ms(a.parts[5]), a.held >> 20
            );
        }
    }
    if trace {
        let held = crate::alloc::held();
        eprintln!(
            "fix: typing job {} round {}: the merges' drops waited for {:.0} ms; then the allocator reserved {} MB, {} MB of it free at the centre, {} MB of mappings kept",
            k, r, waited.as_secs_f64() * 1e3, held.reserved >> 20, held.centre_free >> 20, held.mapped_kept >> 20
        );
    }
}

/// A document's results planned: its atomic groups numbered after those planned before it, round one's results of a
/// file round two's semantic rules alone ran over joined with theirs, the trace's line, the plan.
fn consume(a: Answer, overrides: &HashMap<String, String>, earlier: &Earlier<'_>, names: &[String], base: &mut u32, kept: Option<&mut HashMap<String, Results>>) -> Outcome {
    let Answer { mut results, view, groups, .. } = a;
    let offset = |g: &mut u32| {
        if *g != 0 {
            *g += *base;
        }
    };
    for e in &mut results.edits {
        offset(&mut e.group);
    }
    for op in &mut results.imports {
        match op {
            host::ImportOp::Remove { group, .. } | host::ImportOp::Add { group, .. } => offset(group),
        }
    }
    for l in &mut results.lints {
        offset(&mut l.group);
    }
    *base += groups;
    let (file, path, text) = (view.file.clone(), view.path.clone(), view.text.clone());
    let before = if overrides.contains_key(&file) { None } else { earlier.results.get(&file) };
    let results = match before {
        Some(b) => earlier.joined(b, results),
        None => results,
    };
    let planning = std::time::Instant::now();
    if std::env::var_os("TEQ_FIX_TRACE").is_some() {
        eprintln!("fix: {} (generation {}): {} nodes, {} tokens, {} edits, {} import edits, {} lints, {} failures", view.path, view.gen, view.tree.nodes.len(), view.tokens.len(), results.edits.len(), results.imports.len(), results.lints.len(), results.failures.len());
        for (r, f) in &results.failures {
            eprintln!("fix:   rule {} failed: {}", r, f);
        }
    }
    let p = plan::plan(&view, &results, names);
    timed(3, planning);
    if let Some(k) = kept {
        k.insert(file.clone(), results);
    }
    Outcome { file, doc_path: path, text, plan: p }
}

/// The paths of the job's program, its editable files and the rest of what it reads.
pub(crate) fn program_paths(job: &Job) -> Vec<String> {
    let mut args = vec!["compiler".to_string(), "check".to_string()];
    args.extend(job.args.iter().cloned());
    crate::collect_program_paths_with_roots(&crate::parse_options_from(args.into_iter())).0
}

/// An attempt at a round's typing, as the record and the trace tell it (docs/FIX.md, "Debugging a rule").
struct Attempt {
    /// From 1; a second where the first gave way.
    number: u32,
    /// The workers handed to the typer, and those it typed with: one where the overlays' mappings were refused
    /// (`mapping_refused`), else the count within `typer::MAX_WORKERS`, which `Typer::run` clamps it to.
    asked: usize,
    effective: usize,
    /// Whether the system refused the overlays' mappings, so that `Typer::run` typed in place by one worker without the
    /// fork (`types::take_mapping_refused`).
    mapping_refused: bool,
    /// Whether the system refused a thread of the typing's after the fork (a body phase's or an inlining phase's worker,
    /// a merge thread), so that the attempt is given up and the round typed again by one worker
    /// (`types::take_worker_refused`).
    worker_refused: bool,
    /// Whether `Typer::run` took the fork: several workers, or `TEQ_FORK=1`, and the mappings given.
    forked: bool,
    /// `one` (no fork), `joined` (the workers merged), `retried` (it gave way), `refused` (the merge's check).
    outcome: &'static str,
    /// Why it gave way.
    why: String,
    /// When it began and ended, in microseconds since the run began.
    start_us: u64,
    end_us: u64,
    /// Seconds: reading and parsing, the jars, typing as compile's `type` phase counts it (the jars, the typer made and
    /// set up, `run`), the documents, the jar caches, the exit (the disposal and the drops).
    parts: [f64; 6],
    /// `run`'s phases (`measure::phase_parts`).
    phases: Vec<(&'static str, std::time::Duration)>,
    /// Under `TEQ_FIX_TYPING_WORK=1`, each item of the queue: its worker, its index in its file, when it began and
    /// ended (microseconds since the run began), its file.
    items: Vec<(u32, u32, u64, u64, String)>,
    /// The allocator's reserve at the typer's drop, in bytes.
    held: usize,
}

/// The job's program typed as `teq compiler check` types it, with the index and, for a semantic rule, the unused
/// imports, and the views of its editable files; the rendered errors where it does not type. A parallel attempt that
/// gives way is the round's: the round's texts typed again by one worker on this thread, nothing of the attempt
/// published, the rules configured and the earlier rounds' results as they were (docs/FIX.md, "Patches, suppressions and rounds"). The attempts
/// come back for the record.
fn typed(job: &Job, overrides: &HashMap<String, String>, semantic: bool, first_gen: u32, display: &Option<(PathBuf, PathBuf)>) -> (Vec<Attempt>, Result<Vec<DocView>, String>, Option<RoundTexts>) {
    let (mut attempts, mut texts) = (Vec::new(), None);
    #[cfg(debug_assertions)]
    let numbers = crate::typer::thread::run(crate::shared::numbers_taken_by_others);
    if let Some(docs) = attempt(job, overrides, semantic, first_gen, display, None, &mut attempts, &mut texts, None) {
        return (attempts, docs, texts);
    }
    // What watch's retry asserts of an attempt that gave way (`watch.rs`): nothing of it is held on the typing thread.
    #[cfg(debug_assertions)]
    {
        let (taken, depth, staged, overlay) = crate::typer::thread::run(|| (crate::shared::numbers_taken_by_others(), crate::shared::lock_depth(), crate::shared::staged_completions(), crate::types::in_overlay()));
        assert_eq!(taken, numbers, "the attempt's workers kept their threads' numbers");
        assert_eq!((depth, staged, overlay), (0, 0, false), "the attempt left the loader's lock held, completions staged or an overlay entered");
    }
    // One worker takes no fork, and only a forked build gives way (`Worker::need_serial_for_state`).
    let docs = attempt(job, overrides, semantic, first_gen, display, Some(1), &mut attempts, &mut texts, None).unwrap_or_else(|| Err("teq fix: the typing gave way at one worker\n".to_string()));
    (attempts, docs, texts)
}

/// The texts a round types (`attempt`): the program's paths, their roots and each file's text, `overrides` over the
/// disk as the round's first attempt read them.
struct RoundTexts {
    paths: Vec<String>,
    roots: crate::intern::FxMap<String, String>,
    texts: HashMap<String, String>,
}

/// The workers a round types with: compile's (`--threads`, `TEQ_THREADS`, else the automatic count over the round's
/// texts, `frontend::automatic_threads`), unless the fix suite's switches say otherwise: `TEQ_FIX_TYPING_THREADS=<n>`, the
/// matrix's typing count apart from the rules', and `TEQ_FIX_PLANT=serial-typing`, its control, one.
fn typing_threads(opts: &crate::Options, program_bytes: usize, largest_unit: usize) -> usize {
    if plant().as_deref() == Some("serial-typing") {
        return 1;
    }
    if let Some(n) = std::env::var("TEQ_FIX_TYPING_THREADS").ok().and_then(|v| v.parse::<usize>().ok()).filter(|&n| n >= 1) {
        return n;
    }
    crate::frontend::requested_threads(opts.threads).unwrap_or_else(|| crate::frontend::automatic_threads(program_bytes, largest_unit))
}

/// What the service's typing does with its typer before its exit (`serve`): the typer, the texts, the scripts' own texts.
type Serving<'s> = dyn FnMut(&mut crate::typer::Typer<'_>, &crate::source::Sources, &HashMap<String, String>) + 's;

/// What the round's SemanticDB service did, for the record: the questions it answered, whether it typed, and when it
/// began typing and ended (microseconds since the run began).
#[derive(Default)]
struct Served {
    queries: u32,
    typed: bool,
    start_us: u64,
    end_us: u64,
}

/// The round's SemanticDB service (`fix::service`; tests/semanticdb/TABLE.md, ruling 1), on the round's typing thread
/// after its typing: nothing typed before a rule asks; at the first question, the round's texts (`texts`, as the round's
/// typing read them) typed by one worker keeping the retained forms (`--semantic-types`), as `typed` types them but for
/// the worker count and the forms, so
/// that the answers are the same whatever the round's typing's count; each question answered on that thread until the
/// round drops its link after its last document. An answer's panic is caught and answers every later question where the build
/// unwinds (the release build aborts on any panic, Cargo.toml); the typing's errors answer every question.
fn serve(job: &Job, overrides: &HashMap<String, String>, display: &Option<(PathBuf, PathBuf)>, first_gen: u32, docs: Vec<(u32, String)>, questions: mpsc::Receiver<SemQuery>, mut texts: Option<RoundTexts>) -> Served {
    let Ok(first) = questions.recv() else { return Served::default() };
    let start_us = now_us();
    let (mut pending, mut queries, mut attempts) = (Some(first), 0u32, Vec::new());
    let mut answering = |typer: &mut crate::typer::Typer<'_>, sources: &crate::source::Sources, written: &HashMap<String, String>| {
        let mut broken: Option<String> = None;
        while let Some(q) = pending.take().or_else(|| questions.recv().ok()) {
            queries += 1;
            let answered = match &broken {
                Some(e) => Err(e.clone()),
                None => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| answer(typer, sources, written, &docs, &q))).unwrap_or_else(|p| {
                    let e = format!("the SemanticDB service failed: {}", panic_text(p.as_ref()));
                    broken = Some(e.clone());
                    Err(e)
                }),
            };
            let _ = q.reply.send(SemReply { job: q.job, round: q.round, gen: q.gen, answer: answered });
        }
    };
    let typed = attempt(job, overrides, true, first_gen, display, Some(1), &mut attempts, &mut texts, Some(&mut answering));
    // One worker never gives way (`Worker::need_serial_for_state`).
    if let Some(e) = match typed {
        Some(Ok(_)) => None,
        Some(Err(e)) => Some(e),
        None => Some("the typing gave way at one worker".to_string()),
    } {
        let e = format!("the round's semantic typing failed: {}", e.trim_end());
        while let Some(q) = pending.take().or_else(|| questions.recv().ok()) {
            queries += 1;
            let _ = q.reply.send(SemReply { job: q.job, round: q.round, gen: q.gen, answer: Err(e.clone()) });
        }
    }
    Served { queries, typed: true, start_us, end_us: now_us() }
}

/// The service's answer to `q` from the round's semantic typing: the document `q.gen` of the round's, its text the
/// typing's (a script's own, `written`). Before the port of ExtractSemanticDB (the order of work's step 7) the document
/// is its header alone, and the symbols and informations are refused, never answered empty. `TEQ_FIX_PLANT=query-failure`,
/// the fix suite's, fails every answer as an extraction's error would.
fn answer(_typer: &mut crate::typer::Typer<'_>, sources: &crate::source::Sources, written: &HashMap<String, String>, docs: &[(u32, String)], q: &SemQuery) -> Result<SemAnswer, String> {
    if plant().as_deref() == Some("query-failure") {
        return Err("the extraction failed (TEQ_FIX_PLANT=query-failure)".to_string());
    }
    let Some((_, file)) = docs.iter().find(|(g, _)| *g == q.gen) else { return Err(format!("generation {} is not this round's", q.gen)) };
    let Some(f) = sources.as_slice().iter().find(|f| &f.path == file) else { return Err(format!("{}: the round's semantic typing typed no such file", file)) };
    let text = written.get(file).map_or(f.text.as_str(), String::as_str);
    match &q.ask {
        Ask::Doc => {
            let root = std::env::current_dir().unwrap_or_default();
            let doc = crate::typer::semanticdb::document(Path::new(file), text, &root);
            Ok(SemAnswer::Doc(crate::typer::semanticdb::document::write_documents(&doc)))
        }
        Ask::Symbols { start, end } => Err(format!("the symbols at [{}, {}) of {}: {}", start, end, file, NOT_PORTED)),
        Ask::Info { symbol } => Err(format!("the information of {}: {}", symbol, NOT_PORTED)),
    }
}

/// Why the service answers no symbol yet.
const NOT_PORTED: &str = "SemanticDB's symbols are not extracted yet: the port of ExtractSemanticDB is the order of work's step 7 (tests/semanticdb/TABLE.md)";

/// A panic's message.
fn panic_text(p: &(dyn std::any::Any + Send)) -> String {
    p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_else(|| "a panic".to_string())
}

/// One attempt at the round's typing, at `forced` workers or the round's own count (`typing_threads`): its documents or
/// its rendered errors, or `None` where it gave way. Nothing is read of the typer before the give-way and the merge's
/// refusal are ruled out; the jar caches are written by an attempt that ends the round, never by one that gave way or
/// whose merge was refused, as compile writes them; every attempt ends with the interpreter's state of its program
/// disposed of on the thread that typed, and the typer, its trees and its texts dropped here.
#[allow(clippy::too_many_arguments)]
fn attempt(job: &Job, overrides: &HashMap<String, String>, semantic: bool, first_gen: u32, display: &Option<(PathBuf, PathBuf)>, forced: Option<usize>, attempts: &mut Vec<Attempt>, texts: &mut Option<RoundTexts>, serving: Option<&mut Serving<'_>>) -> Option<Result<Vec<DocView>, String>> {
    let (began, start_us) = (std::time::Instant::now(), now_us());
    let mut args = vec!["compiler".to_string(), "check".to_string()];
    args.extend(job.args.iter().cloned());
    let mut opts = crate::parse_options_from(args.into_iter());
    // The language server's index: the parser's name spans and the typer's records.
    opts.index = true;
    opts.syntax.index = true;
    if semantic {
        opts.wunused_imports = true;
    }
    // The typer keeps the forms the SemanticDB documents read (`TEQ_FIX_SEMANTICDB`'s, the service's), with one worker
    // (tests/semanticdb/TABLE.md, section 6), transparent to its typing (the identity run).
    if semanticdb_dump().is_some() || serving.is_some() {
        opts.semantic_types = true;
    }
    // The round's texts: its first attempt reads the program's files (their paths and texts, `overrides` over the disk)
    // and keeps them, and every later typing of the round (the retry at one worker, the SemanticDB service's) types them
    // as they were, never the disk again, whatever was written to it since.
    let (paths, roots) = match texts.as_ref() {
        Some(t) => (t.paths.clone(), t.roots.clone()),
        None => {
            let (paths, roots, _) = crate::collect_program_paths_with_roots(&opts);
            (paths, roots)
        }
    };
    let editable: Vec<String> = paths.iter().filter(|p| job.edits(p)).cloned().collect();
    opts.keep = editable.clone();
    // A file's text as round one left it where `overrides` has it; a script's as written, its document's text: the
    // wrapper `read_program` appends to a `.sc` (`script::wrap`) is for typing alone, past the script's own text.
    let mut written: HashMap<String, String> = HashMap::new();
    let read = match texts.as_ref() {
        Some(t) => crate::read_program_from(&opts, paths, &roots, |p| {
            let text = t.texts.get(p).cloned().ok_or_else(|| format!("teq fix: the round's texts hold no {}", p))?;
            if crate::script::is_script(p) {
                written.insert(p.to_string(), text.clone());
            }
            Ok(text)
        }),
        None => {
            let mut kept = RoundTexts { paths: paths.clone(), roots: roots.clone(), texts: HashMap::new() };
            let read = crate::read_program_with(&opts, paths, &roots, |p, disk| {
                let text = overrides.get(p).cloned().unwrap_or(disk);
                if crate::script::is_script(p) {
                    written.insert(p.to_string(), text.clone());
                }
                kept.texts.insert(p.to_string(), text.clone());
                text
            });
            if read.is_ok() {
                *texts = Some(kept);
            }
            read
        }
    };
    let program = match read {
        Ok(p) => p,
        Err(e) => return Some(Err(format!("{}\n", e))),
    };
    let program_bytes: usize = program.iter().map(|f| f.text.len()).sum();
    let mut interner = crate::intern::Interner::new();
    let mut diags = crate::source::Diagnostics::default();
    let opening = std::time::Instant::now();
    let cp = crate::open_jars(&opts);
    let jars = opening.elapsed();
    let (mut files, mut asts, blocks_of, std, selected, scalajs_unlocked) = crate::parse_build(&opts, cp.as_ref(), program, &mut interner, &mut diags);
    if diags.has_errors() {
        return Some(Err(diags.render(&files)));
    }
    // Compile's count, from the program's bytes and its largest unit before the jars' pseudo-files (`compile`).
    let largest_unit = crate::frontend::largest_unit_bytes(&files, &asts, &blocks_of);
    let (jar_files, jdk_file) = crate::jar_pseudo_files(&opts, &mut files, &mut asts);
    let asts = crate::ast::Asts::new(asts);
    let sources = crate::source::Sources::new(files);
    let mut parts = [0.0; 6];
    parts[0] = (began.elapsed() - jars).as_secs_f64();
    parts[1] = jars.as_secs_f64();
    let typing = std::time::Instant::now();
    // One worker keeps the retained forms (`configure_typer`).
    let asked = forced.unwrap_or_else(|| if opts.semantic_types { 1 } else { typing_threads(&opts, program_bytes, largest_unit) });
    let mut typer = crate::typer::Typer::new(&asts, &sources, &mut interner);
    typer.threads = asked;
    crate::install_std(&mut typer, std, &selected, &blocks_of, scalajs_unlocked);
    crate::open_classpath(&opts, &mut typer, cp, jar_files, jdk_file);
    crate::configure_typer(&mut typer, &opts, &asts, &sources);
    typer.choose_entry = false;
    // `TEQ_FIX_INDEX=off`, bench/fix's parity pair alone: the typer's records not made, as `teq compiler check`, which
    // indexes nothing, types; `hasExpansions` is then false everywhere.
    typer.index = (std::env::var_os("TEQ_FIX_INDEX").is_none_or(|v| v != "off")).then(Box::default);
    // The witness's items (`TEQ_FIX_TYPING_WORK=1`, the fix suite's): the counters `--time` turns on, for this attempt.
    let witness = std::env::var_os("TEQ_FIX_TYPING_WORK").is_some_and(|v| v == "1");
    crate::measure::enable(witness);
    // The flag of a refusal before this typing (a rule program's typer's under `TEQ_FORK=1`, every bootstrap ended
    // before a typing begins) is not this typing's.
    crate::types::take_mapping_refused();
    crate::types::take_worker_refused();
    typer.run();
    if witness {
        typer.on_thread(|_| crate::measure::flush(0));
        crate::measure::enable(false);
    }
    parts[2] = (typing.elapsed() + jars).as_secs_f64();
    // `Typer::run`'s own conditions: a refused mapping types in place by one worker, else the count clamped, forked at
    // several workers or under `TEQ_FORK=1`.
    let mapping_refused = crate::types::take_mapping_refused();
    // A thread of the typing's the system refused: the workers' starts give the build way themselves; a merge thread's
    // merge ran its share on the caller's thread and is not kept either (`crew::Crew::with`).
    let refusal = crate::types::take_worker_refused();
    let worker_refused = refusal.is_some();
    let effective = if mapping_refused { 1 } else { asked.clamp(1, crate::typer::MAX_WORKERS) };
    let forked = !mapping_refused && (effective > 1 || std::env::var_os("TEQ_FORK").is_some_and(|v| v == "1"));
    let epoch = *EPOCH.get_or_init(std::time::Instant::now);
    let at = |t: std::time::Instant| t.saturating_duration_since(epoch).as_micros() as u64;
    let items = if witness { crate::measure::take_items().into_iter().map(|i| (i.worker, i.index, at(i.start), at(i.end), sources.as_slice()[i.file as usize].path.clone())).collect() } else { Vec::new() };
    let mut a = Attempt { number: attempts.len() as u32 + 1, asked, effective, mapping_refused, worker_refused, forked, outcome: if forked { "joined" } else { "one" }, why: String::new(), start_us, end_us: 0, parts, phases: crate::measure::phase_parts(), items, held: 0 };
    let result = if typer.serial.is_needed() || worker_refused {
        a.outcome = "retried";
        a.why = typer.serial.reason().or(refusal).unwrap_or_default();
        crate::types::view::gave_way(&a.why);
        if std::env::var_os("TEQ_SERIAL_TRACE").is_some() {
            eprintln!("teq: typed again by one worker: {}", a.why);
        }
        None
    } else if typer.merge_failed {
        a.outcome = "refused";
        Some(Err(typer.report_diags().0))
    } else if let Some(serve) = serving {
        // The service's typing answers its questions here, before its exit; the round's typing wrote the jar caches.
        if typer.diags.items.iter().any(|d| !d.is_warning) || typer.diags.syntax_errors {
            Some(Err(typer.report_diags().0))
        } else {
            serve(&mut typer, &sources, &written);
            Some(Ok(Vec::new()))
        }
    } else {
        let caching = std::time::Instant::now();
        crate::save_jar_caches(&mut typer);
        a.parts[4] = caching.elapsed().as_secs_f64();
        if typer.diags.items.iter().any(|d| !d.is_warning) || typer.diags.syntax_errors {
            Some(Err(typer.report_diags().0))
        } else {
            let documents = std::time::Instant::now();
            // Round one's SemanticDB documents under `TEQ_FIX_SEMANTICDB` (`semanticdb_dump`), each file named by the
            // command line's argument for it.
            let dump = semanticdb_dump().filter(|_| overrides.is_empty());
            let docs = documents_of(&mut typer, &sources, &asts, &blocks_of, &editable, &written, semantic, first_gen, display, dump.as_deref().map(|d| (d, opts.inputs.as_slice())));
            a.parts[3] = documents.elapsed().as_secs_f64();
            Some(docs)
        }
    };
    // The attempt's exit, on the thread that typed (`thread::run`'s, `TEQ_BODY_THREAD`): the overlays' drop the merge
    // began, then the interpreter's caches and the cycles its macro runs left (`interp::dispose`, which `Typer::run`
    // does at its start for an earlier program on that thread); then the typer, its trees and its texts.
    let exiting = std::time::Instant::now();
    typer.on_thread(|w| {
        w.types.join_reclaiming();
        crate::interp::dispose();
    });
    a.held = crate::alloc::held().reserved;
    drop(typer);
    drop(asts);
    drop(sources);
    drop(interner);
    a.parts[5] = exiting.elapsed().as_secs_f64();
    a.end_us = now_us();
    attempts.push(a);
    result
}

/// The views of the job's editable files, from a typer that typed the program.
#[allow(clippy::too_many_arguments)]
fn documents_of(typer: &mut crate::typer::Typer<'_>, sources: &crate::source::Sources, asts: &crate::ast::Asts, blocks_of: &[Vec<usize>], editable: &[String], written: &HashMap<String, String>, semantic: bool, first_gen: u32, display: &Option<(PathBuf, PathBuf)>, dump: Option<(&Path, &[String])>) -> Result<Vec<DocView>, String> {
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
        if let Some((dir, inputs)) = dump {
            let root = std::env::var_os("TEQ_FIX_SEMANTICDB_ROOT").map(PathBuf::from).unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
            // The file's path as the command line gave it (`SourceFile`'s `jpath`), which the program's paths spell as
            // the file system does: its elements kept (`outside/./Other.scala`).
            let given = inputs.iter().find(|i| crate::file_identity(i) == crate::file_identity(&f.path)).cloned().unwrap_or_else(|| f.path.clone());
            let doc = crate::typer::semanticdb::document(Path::new(&given), text, &root);
            crate::typer::semanticdb::write_dump(dir, Path::new(&given), &root, &doc).map_err(|e| format!("teq fix: {}: the SemanticDB document is not written: {}\n", f.path, e))?;
            let ids: Vec<FileId> = entries.iter().map(|&e| FileId(e as u32)).collect();
            let view = typer.on_thread(|w| crate::typer::semanticdb::retained_view(w, &ids));
            crate::typer::semanticdb::write_view(dir, Path::new(&given), &root, &view).map_err(|e| format!("teq fix: {}: the retained forms' view is not written: {}\n", f.path, e))?;
        }
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

/// `TEQ_FIX_SEMANTICDB=<dir>`, a test's: round one writes each editable file's SemanticDB document where scalac's
/// `-semanticdb-target <dir>` writes it, its path relative to `TEQ_FIX_SEMANTICDB_ROOT` (scalac's `-sourceroot`, by
/// default the working directory), the run typed as for a semantic rule; tests/semanticdb compares it with scalac's.
fn semanticdb_dump() -> Option<PathBuf> {
    std::env::var_os("TEQ_FIX_SEMANTICDB").map(PathBuf::from)
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

/// Tells the coordinator of a worker that panicked, which would otherwise wait for its answer.
struct EndedGuard(mpsc::Sender<FromRules>);

impl Drop for EndedGuard {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let _ = self.0.send(FromRules::Ended);
        }
    }
}

/// Worker `worker`: the rules' program typed and the rules configured (`Ready`), then each document it is handed run through
/// every rule (`Done`), until its channel closes.
fn rule_thread(worker: usize, boot: Boot, rx: mpsc::Receiver<ToRules>, tx: mpsc::Sender<FromRules>, gate: &Gate) {
    crate::interp::fix::want();
    let _guard = EndedGuard(tx.clone());
    // `TEQ_FIX_BOOT_DELAY=<ms>`, the fix suite's: every worker but the lead holds its bootstrap that long, so that a
    // round's lead finishes its documents first and the next typing has to wait for the others (`round`).
    if worker > 0 {
        if let Some(ms) = std::env::var("TEQ_FIX_BOOT_DELAY").ok().and_then(|v| v.parse::<u64>().ok()) {
            std::thread::sleep(std::time::Duration::from_millis(ms));
        }
    }
    let booting = std::time::Instant::now();
    let since = |t: std::time::Instant| t.elapsed().as_micros() as u64;
    let Boot { conf, config_dir, requested } = boot;
    host::install(Host { doc: None, results: Results::default(), conf: conf.clone(), read: vec![false; conf.nodes.len()], groups: 0, asker: None, sem: None, queries: 0 });
    let sources: Vec<(String, PathBuf)> = requested.iter().filter_map(|r| r.strip_prefix("file:").map(|p| (r.clone(), config_dir.join(p)))).collect();
    let mut files: Vec<PathBuf> = Vec::new();
    for (_, p) in &sources {
        files.extend(scala_files(p));
    }
    // A path of the worker's own: another worker removes its directory while this one may still read its entry.
    let temp = std::env::temp_dir().join(format!("teq-fix-{}-{}", std::process::id(), worker));
    let entry = temp.join("entry.scala");
    if let Err(e) = std::fs::create_dir_all(&temp).and_then(|_| std::fs::write(&entry, ENTRY)) {
        let _ = tx.send(FromRules::Ready(worker, Err(format!("teq fix: cannot write {}: {}", entry.display(), e)), since(booting)));
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
            let _ = tx.send(FromRules::Ready(worker, Err(e), since(booting)));
            return;
        }
    };
    let _ = std::fs::remove_dir_all(&temp);
    let mut interner = crate::intern::Interner::new();
    let mut diags = crate::source::Diagnostics::default();
    let cp = crate::open_jars(&opts);
    let (mut sfiles, mut asts, blocks_of, std, selected, scalajs_unlocked) = crate::parse_build(&opts, cp.as_ref(), program, &mut interner, &mut diags);
    if diags.has_errors() {
        let _ = tx.send(FromRules::Ready(worker, Err(diags.render(&sfiles)), since(booting)));
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
        let _ = tx.send(FromRules::Ready(worker, Err(shown), since(booting)));
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
            let _ = tx.send(FromRules::Ready(worker, Err(e), since(booting)));
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
        let mut chosen: Vec<(String, crate::interp::Value, bool)> = Vec::new();
        let mut unknown: Vec<String> = Vec::new();
        for req in &requested {
            if let Some(p) = req.strip_prefix("file:") {
                let dir = config_dir.join(p);
                let of_file: Vec<&(String, Option<String>, crate::interp::Value)> = available.iter().filter(|(_, f, _)| f.as_ref().is_some_and(|f| Path::new(f).starts_with(&dir) || Path::new(f) == dir)).collect();
                if of_file.is_empty() {
                    unknown.push(req.clone());
                }
                for (n, _, v) in of_file {
                    chosen.push((n.clone(), v.clone(), false));
                }
            } else {
                match available.iter().find(|(n, _, _)| n == req) {
                    Some((n, f, v)) => chosen.push((n.clone(), v.clone(), f.is_none())),
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
        for (n, v, shipped) in &chosen {
            let c = interp.call_method(module.clone(), syms[3], vec![v.clone()]).map_err(|f| format!("teq fix: the configuration of {}: {}", n, failure(&mut interp, f)))?;
            let semantic = interp.call_method(module.clone(), syms[2], vec![c.clone()]).map_err(|f| failure(&mut interp, f))?;
            infos.push(RuleInfo { name: n.clone(), semantic: matches!(semantic, crate::interp::Value::Bool(true)), shipped: *shipped });
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
            let _ = tx.send(FromRules::Ready(worker, Err(e), since(booting)));
            return;
        }
    };
    let _ = tx.send(FromRules::Ready(worker, Ok(infos), since(booting)));
    let rules = crate::interp::fix::array_of(configured);
    while let Ok(ToRules::Doc(doc, Handout { semantic_only, asker, delay, first, link })) = rx.recv() {
        let gen = doc.gen;
        // A document's atomic groups are numbered from 1; the coordinator offsets them (`consume`).
        host::with_host(|h| {
            h.doc = Some(*doc);
            h.results = Results::default();
            h.groups = 0;
            h.asker = Some(asker);
            h.sem = link;
            h.queries = 0;
        });
        interp.set_limits(crate::interp::Limits::unlimited());
        let (start, cpu_start) = (now_us(), crate::measure::thread_cpu_ns());
        let barrier_timeout = first && gate.pass((asker.job, asker.round));
        let r = interp.call_method(module.clone(), syms[4], vec![rules.clone(), crate::interp::Value::Int(gen as i32), crate::interp::Value::Bool(semantic_only)]);
        let failure = match r {
            Ok(_) => None,
            Err(f) => Some(match f {
                crate::interp::Failure::Thrown(v) => interp.describe_value(&v),
                other => interp.describe(&other),
            }),
        };
        let (finish, cpu) = (now_us(), crate::measure::thread_cpu_ns().saturating_sub(cpu_start) / 1000);
        // The document's link to the service goes with it: the round's service ends with the round's last document.
        let (results, view, groups, asker, queries) = host::with_host(|h| {
            let mut r = std::mem::take(&mut h.results);
            if let Some(f) = failure {
                r.failures.push((0, f));
            }
            h.sem = None;
            (r, h.doc.take(), h.groups, h.asker.take(), h.queries)
        })
        .unwrap_or_default();
        let (Some(view), Some(asker)) = (view, asker) else { break };
        if delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }
        if tx.send(FromRules::Done(Box::new(Answer { asker, results, view, groups, start, finish, cpu, barrier_timeout, queries }))).is_err() {
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
