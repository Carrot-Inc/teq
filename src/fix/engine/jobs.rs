//! The jobs of a fix run side by side (docs/FIX.md, "Jobs side by side"): each job's round is a child process of
//! the run, `teq compiler fix-job`, which types the job's program, runs its rules on rule workers of its own, plans
//! its documents and hands the run what `round` gives the loop of one job at a time; the run does once what it does
//! once (the merge by file, the refusals, round two's choice, the lints, the writes), in the jobs' order. This file
//! holds what crosses between the two, as JSON lines on the child's stdin and stdout, the child's side, and the
//! run's side.

use super::*;
use crate::lsp::json::{obj, Json};

fn num(n: impl Into<f64>) -> Json {
    Json::Num(n.into())
}

fn strings(items: &[String]) -> Json {
    Json::Arr(items.iter().map(|s| s.as_str().into()).collect())
}

fn strings_of(j: Option<&Json>) -> Vec<String> {
    j.map_or(&[][..], Json::arr).iter().filter_map(Json::str).map(str::to_string).collect()
}

fn u32_of(j: &Json, key: &str) -> Result<u32, String> {
    j.get(key).and_then(Json::uint).ok_or_else(|| format!("no {} in a message between a fix run and its job", key))
}

fn str_of<'a>(j: &'a Json, key: &str) -> Result<&'a str, String> {
    j.get(key).and_then(Json::str).ok_or_else(|| format!("no {} in a message between a fix run and its job", key))
}

fn span(s: Span) -> Json {
    Json::Arr(vec![num(s.start), num(s.end)])
}

fn span_of(j: Option<&Json>) -> Result<Span, String> {
    match j.map(Json::arr) {
        Some([a, b]) => Ok(Span { start: a.uint().ok_or("a span's start")?, end: b.uint().ok_or("a span's end")? }),
        _ => Err("a span in a message between a fix run and its job".to_string()),
    }
}

fn severity(s: Severity) -> Json {
    num(match s {
        Severity::Info => 0u32,
        Severity::Warning => 1,
        Severity::Error => 2,
    })
}

fn severity_of(j: Option<&Json>) -> Result<Severity, String> {
    match j.and_then(Json::uint) {
        Some(0) => Ok(Severity::Info),
        Some(1) => Ok(Severity::Warning),
        Some(2) => Ok(Severity::Error),
        _ => Err("a severity in a message between a fix run and its job".to_string()),
    }
}

/// A round of a job as its child is given it: the job, the configuration it runs under, and what the loop of one job
/// at a time hands `round`.
pub(super) struct Spec {
    /// The run's number of the job, and the round (1 or 2).
    pub job: usize,
    pub round: u32,
    pub program: Job,
    /// The part's configuration as the run read it, and where its relative `file:` entries start.
    pub conf: Conf,
    pub config_dir: PathBuf,
    /// `--rules`.
    pub rules: Option<Vec<String>>,
    pub display: Option<(PathBuf, PathBuf)>,
    /// The round's first generation, and the number its atomic groups go on from.
    pub first_gen: u32,
    pub base: u32,
    /// The run's clock where the child starts, in microseconds: the child's record goes by it.
    pub clock_us: u64,
    /// Round two: the files round one edits, with their texts; and the job's round-one results of its other files.
    pub changed: HashMap<String, String>,
    pub kept: Vec<(String, Results)>,
    /// The rule workers the part's run may take (`Pool::limit`), which decides the record's `serial` line as in a
    /// run of one job at a time, whatever the round is granted.
    pub limit: usize,
    /// The milliseconds the child waits before it hands its outcomes (`TEQ_FIX_JOBS_SCHEDULE`, the fix suite's).
    pub delay_ms: u64,
    /// Whether the run writes its record (`TEQ_FIX_RECORD`): the child then hands it its lines.
    pub record: bool,
}

impl Spec {
    pub fn to_json(&self) -> Json {
        let paths = |p: &Option<Vec<PathBuf>>| p.as_ref().map_or(Json::Null, |v| Json::Arr(v.iter().map(|p| p.to_string_lossy().as_ref().into()).collect()));
        obj([
            ("job", self.job.into()),
            ("round", self.round.into()),
            ("key", self.program.key.as_str().into()),
            ("args", strings(&self.program.args)),
            ("roots", self.program.roots.as_ref().map_or(Json::Null, |r| strings(r))),
            ("files", paths(&self.program.files)),
            ("generated", strings(&self.program.generated)),
            ("conf", conf(&self.conf)),
            ("config_dir", self.config_dir.to_string_lossy().as_ref().into()),
            ("rules", self.rules.as_ref().map_or(Json::Null, |r| strings(r))),
            ("display", self.display.as_ref().map_or(Json::Null, |(root, from)| Json::Arr(vec![root.to_string_lossy().as_ref().into(), from.to_string_lossy().as_ref().into()]))),
            ("first_gen", self.first_gen.into()),
            ("base", self.base.into()),
            ("clock_us", num(self.clock_us as f64)),
            ("changed", Json::Obj(self.changed.iter().map(|(k, v)| (k.clone(), v.as_str().into())).collect())),
            ("kept", Json::Arr(self.kept.iter().map(|(file, r)| obj([("file", file.as_str().into()), ("results", results(r))])).collect())),
            ("limit", self.limit.into()),
            ("delay_ms", num(self.delay_ms as f64)),
            ("record", self.record.into()),
        ])
    }

    pub fn of_json(j: &Json) -> Result<Spec, String> {
        let list = |key: &str| j.get(key).filter(|v| !v.is_null()).map(|v| strings_of(Some(v)));
        let display = match j.get("display").map(Json::arr) {
            Some([root, from]) => Some((PathBuf::from(root.str().unwrap_or("")), PathBuf::from(from.str().unwrap_or("")))),
            _ => None,
        };
        let changed = match j.get("changed") {
            Some(Json::Obj(fields)) => fields.iter().filter_map(|(k, v)| Some((k.clone(), v.str()?.to_string()))).collect(),
            _ => HashMap::new(),
        };
        let mut kept = Vec::new();
        for k in j.get("kept").map_or(&[][..], Json::arr) {
            kept.push((str_of(k, "file")?.to_string(), results_of(k.get("results").ok_or("a kept file's results")?)?));
        }
        Ok(Spec {
            job: u32_of(j, "job")? as usize,
            round: u32_of(j, "round")?,
            program: Job {
                key: str_of(j, "key")?.to_string(),
                args: strings_of(j.get("args")),
                roots: list("roots"),
                files: list("files").map(|v| v.into_iter().map(PathBuf::from).collect()),
                generated: strings_of(j.get("generated")),
            },
            conf: conf_of(j.get("conf").ok_or("no configuration in a job")?)?,
            config_dir: PathBuf::from(str_of(j, "config_dir")?),
            rules: list("rules"),
            display,
            first_gen: u32_of(j, "first_gen")?,
            base: u32_of(j, "base")?,
            clock_us: j.get("clock_us").and_then(Json::num).unwrap_or(0.0) as u64,
            changed,
            kept,
            limit: u32_of(j, "limit")? as usize,
            delay_ms: j.get("delay_ms").and_then(Json::num).unwrap_or(0.0) as u64,
            record: j.get("record").and_then(Json::bool) == Some(true),
        })
    }
}

/// A configuration as the run read it (`Conf`): its files and its nodes, each with its value and its place.
pub(super) fn conf(c: &Conf) -> Json {
    use crate::fix::config::Value;
    let nodes = c
        .nodes
        .iter()
        .map(|n| {
            let value = match &n.value {
                Value::Str(s) => Json::Arr(vec![0u32.into(), s.as_str().into()]),
                Value::Num(s) => Json::Arr(vec![1u32.into(), s.as_str().into()]),
                Value::Bool(b) => Json::Arr(vec![2u32.into(), (*b).into()]),
                Value::Null => Json::Arr(vec![3u32.into()]),
                Value::List(items) => Json::Arr(vec![4u32.into(), Json::Arr(items.iter().map(|&i| i.into()).collect())]),
                Value::Obj(fields) => Json::Arr(vec![5u32.into(), Json::Arr(fields.iter().map(|(k, v)| Json::Arr(vec![k.as_str().into(), (*v).into()])).collect())]),
            };
            Json::Arr(vec![value, n.file.into(), n.line.into(), n.col.into()])
        })
        .collect();
    obj([("file", c.file.as_str().into()), ("files", strings(&c.files)), ("nodes", Json::Arr(nodes))])
}

pub(super) fn conf_of(j: &Json) -> Result<Conf, String> {
    use crate::fix::config::{Node, Value};
    let bad = || "a configuration in a message between a fix run and its job".to_string();
    let mut nodes = Vec::new();
    for n in j.get("nodes").map_or(&[][..], Json::arr) {
        let [value, file, line, col] = n.arr() else { return Err(bad()) };
        let v = value.arr();
        let text = || v.get(1).and_then(Json::str).map(str::to_string).ok_or_else(bad);
        let value = match v.first().and_then(Json::uint) {
            Some(0) => Value::Str(text()?),
            Some(1) => Value::Num(text()?),
            Some(2) => Value::Bool(v.get(1).and_then(Json::bool).ok_or_else(bad)?),
            Some(3) => Value::Null,
            Some(4) => Value::List(v.get(1).map_or(&[][..], Json::arr).iter().map(|i| i.uint().ok_or_else(bad)).collect::<Result<Vec<u32>, String>>()?),
            Some(5) => {
                let mut fields = Vec::new();
                for f in v.get(1).map_or(&[][..], Json::arr) {
                    let [key, at] = f.arr() else { return Err(bad()) };
                    fields.push((key.str().ok_or_else(bad)?.to_string(), at.uint().ok_or_else(bad)?));
                }
                Value::Obj(fields)
            }
            _ => return Err(bad()),
        };
        nodes.push(Node { value, file: file.uint().ok_or_else(bad)?, line: line.uint().ok_or_else(bad)?, col: col.uint().ok_or_else(bad)? });
    }
    Ok(Conf { file: str_of(j, "file")?.to_string(), files: strings_of(j.get("files")), nodes })
}

/// The rules' descriptions as a worker gives them (`FromRules::Ready`): the run holds a child's to its own.
pub(super) fn infos(infos: &[RuleInfo]) -> Json {
    Json::Arr(
        infos
            .iter()
            .map(|i| {
                let state = i.state.as_ref().map_or(Json::Null, |s| obj([("path", s.path.as_str().into()), ("line", s.line.into()), ("col", s.col.into()), ("name", s.name.as_str().into())]));
                obj([("name", i.name.as_str().into()), ("semantic", i.semantic.into()), ("shipped", i.shipped.into()), ("state", state)])
            })
            .collect(),
    )
}

/// A document's results as its rules gave them (`host::Results`): what round two joins with its own.
pub(super) fn results(r: &Results) -> Json {
    let op = |o: host::Op| match o {
        host::Op::Left => 0u32,
        host::Op::Right => 1,
        host::Op::Remove => 2,
    };
    let edits = r.edits.iter().map(|e| Json::Arr(vec![e.rule.into(), e.group.into(), op(e.op).into(), e.token.into(), e.text.as_str().into()])).collect();
    let imports = r
        .imports
        .iter()
        .map(|i| match i {
            host::ImportOp::Remove { rule, group, importee } => Json::Arr(vec![0u32.into(), (*rule).into(), (*group).into(), (*importee).into()]),
            host::ImportOp::Add { rule, group, importer } => Json::Arr(vec![1u32.into(), (*rule).into(), (*group).into(), importer.as_str().into()]),
        })
        .collect();
    let lints = r.lints.iter().map(|l| Json::Arr(vec![l.rule.into(), l.category.as_str().into(), l.message.as_str().into(), severity(l.severity), span(l.span), l.group.into()])).collect();
    let failures = r.failures.iter().map(|(rule, why)| Json::Arr(vec![(*rule).into(), why.as_str().into()])).collect();
    obj([("edits", Json::Arr(edits)), ("imports", Json::Arr(imports)), ("lints", Json::Arr(lints)), ("failures", Json::Arr(failures))])
}

pub(super) fn results_of(j: &Json) -> Result<Results, String> {
    let bad = || "a document's results in a message between a fix run and its job".to_string();
    let items = |key: &str| j.get(key).map_or(&[][..], Json::arr);
    let n = |v: Option<&Json>| v.and_then(Json::uint).ok_or_else(bad);
    let s = |v: Option<&Json>| v.and_then(Json::str).map(str::to_string).ok_or_else(bad);
    let mut out = Results::default();
    for e in items("edits") {
        let e = e.arr();
        let op = match n(e.get(2))? {
            0 => host::Op::Left,
            1 => host::Op::Right,
            2 => host::Op::Remove,
            _ => return Err(bad()),
        };
        out.edits.push(host::Edit { rule: n(e.first())?, group: n(e.get(1))?, op, token: n(e.get(3))?, text: s(e.get(4))? });
    }
    for i in items("imports") {
        let i = i.arr();
        let (rule, group) = (n(i.get(1))?, n(i.get(2))?);
        out.imports.push(match n(i.first())? {
            0 => host::ImportOp::Remove { rule, group, importee: n(i.get(3))? },
            1 => host::ImportOp::Add { rule, group, importer: s(i.get(3))? },
            _ => return Err(bad()),
        });
    }
    for l in items("lints") {
        let l = l.arr();
        out.lints.push(host::Lint { rule: n(l.first())?, category: s(l.get(1))?, message: s(l.get(2))?, severity: severity_of(l.get(3))?, span: span_of(l.get(4))?, group: n(l.get(5))? });
    }
    for f in items("failures") {
        let f = f.arr();
        out.failures.push((n(f.first())?, s(f.get(1))?));
    }
    Ok(out)
}

/// A document's outcome (`Outcome`): its file, its text and its plan.
pub(super) fn outcome(o: &Outcome) -> Json {
    let edits = o.plan.edits.iter().map(|(at, text, rules)| Json::Arr(vec![span(*at), text.as_str().into(), Json::Arr(rules.iter().map(|&r| r.into()).collect())])).collect();
    let lints = o.plan.lints.iter().map(|l| Json::Arr(vec![l.id.as_str().into(), l.message.as_str().into(), severity(l.severity), span(l.span)])).collect();
    obj([("file", o.file.as_str().into()), ("path", o.doc_path.as_str().into()), ("text", o.text.as_str().into()), ("edits", Json::Arr(edits)), ("lints", Json::Arr(lints)), ("refused", strings(&o.plan.refused))])
}

pub(super) fn outcome_of(j: &Json) -> Result<Outcome, String> {
    let bad = || "a document's outcome in a message between a fix run and its job".to_string();
    let mut plan = Plan { edits: Vec::new(), lints: Vec::new(), refused: strings_of(j.get("refused")) };
    for e in j.get("edits").map_or(&[][..], Json::arr) {
        let e = e.arr();
        let rules = e.get(2).map_or(&[][..], Json::arr).iter().map(|r| r.uint().ok_or_else(bad)).collect::<Result<Vec<u32>, String>>()?;
        plan.edits.push((span_of(e.first())?, e.get(1).and_then(Json::str).ok_or_else(bad)?.to_string(), rules));
    }
    for l in j.get("lints").map_or(&[][..], Json::arr) {
        let l = l.arr();
        let text = |k: usize| l.get(k).and_then(Json::str).map(str::to_string).ok_or_else(bad);
        plan.lints.push(Printed { id: text(0)?, message: text(1)?, severity: severity_of(l.get(2))?, span: span_of(l.get(3))? });
    }
    Ok(Outcome { file: str_of(j, "file")?.to_string(), doc_path: str_of(j, "path")?.to_string(), text: str_of(j, "text")?.to_string(), plan })
}


// ---- the child ----

/// A job's child's link to its run: the job's number, whether the run takes its record's lines, and the grants the
/// run answers its asks with.
struct Link {
    job: usize,
    record: bool,
    grants: std::sync::Mutex<mpsc::Receiver<usize>>,
}

static LINK: std::sync::OnceLock<Link> = std::sync::OnceLock::new();
/// The run's clock where this process started, in microseconds; 0 in the run itself.
static CLOCK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// What a child adds to its own clock so that its record's times are the run's.
pub(super) fn clock() -> u64 {
    CLOCK.load(std::sync::atomic::Ordering::Relaxed)
}

/// Whether this process is a job's child.
pub(super) fn is_child() -> bool {
    LINK.get().is_some()
}

/// `job=<n> ` in a job's child, nothing in the run: what tells one child's rule worker from another's in the
/// record's `worker` lines.
pub(super) fn worker_tag() -> String {
    LINK.get().map_or(String::new(), |l| format!("job={} ", l.job))
}

/// What a message to the run begins with on the child's stdout (the record separator): the process's stdout is also
/// its program's, a macro's `println` while it types, and the run tells the two apart by it wherever a message
/// starts, at a line's start or after text that ended in no newline.
const MESSAGE: char = '\u{1e}';

/// One message to the run, a line of the child's stdout.
fn send(message: Json) {
    use std::io::Write;
    let mut line = String::from(MESSAGE);
    line.push_str(&message.to_text());
    line.push('\n');
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(line.as_bytes());
    let _ = out.flush();
}

/// In a job's child, a line of the record goes to the run where it writes one, which writes it at the job's place;
/// whether this process is a child, whose own record is none.
pub(super) fn record(line: &std::fmt::Arguments<'_>) -> bool {
    let Some(link) = LINK.get() else { return false };
    if link.record {
        send(obj([("record", format!("{}", line).as_str().into())]));
    }
    true
}

/// The child tells where its round is: its program typed, with the round's documents; its rules over `done` of them.
pub(super) fn tell_typed(docs: usize) {
    if LINK.get().is_some() {
        send(obj([("where", obj([("typed", docs.into())]))]));
    }
}

pub(super) fn tell_rules(done: usize, of: usize) {
    if LINK.get().is_some() {
        send(obj([("where", obj([("done", done.into()), ("of", of.into())]))]));
    }
}

/// The workers a phase of the child's round takes of the `asked`: the run's grant, in the run itself what it asked.
/// `whole` where a switch names the count (`TEQ_FIX_RULE_THREADS`, `TEQ_FIX_TYPING_THREADS`): granted as asked.
pub(super) fn granted(phase: &str, asked: usize, whole: bool) -> usize {
    let Some(link) = LINK.get() else { return asked };
    send(obj([("want", obj([("phase", phase.into()), ("workers", asked.into()), ("whole", whole.into())]))]));
    // The run answers at once; a run that is gone ends this process at its stdin's end (`child`).
    link.grants.lock().unwrap_or_else(|e| e.into_inner()).recv().map_or(1, |g| g.clamp(1, asked.max(1)))
}

/// The workers the run has raised this child's rules to since it last looked (`manage`: once another job's have come
/// free), if it has.
pub(super) fn raised() -> Option<usize> {
    let grants = LINK.get()?.grants.lock().unwrap_or_else(|e| e.into_inner());
    std::iter::from_fn(|| grants.try_recv().ok()).max()
}

/// The directories this process has made for the moment and removes itself when it is done with them (a rule
/// worker's entry, `rule_thread`): a job's process, which ends without taking its workers down, removes them first.
static HELD: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

/// `make` makes `dir`, which is this process's to remove (`released`).
pub(super) fn held<T>(dir: &Path, make: impl FnOnce() -> T) -> T {
    let mut held = HELD.lock().unwrap_or_else(|e| e.into_inner());
    held.push(dir.to_path_buf());
    make()
}

/// `dir` removed, as its maker does when it is done with it.
pub(super) fn released(dir: &Path) {
    let mut held = HELD.lock().unwrap_or_else(|e| e.into_inner());
    let _ = std::fs::remove_dir_all(dir);
    held.retain(|d| d != dir);
}

/// In a job's child, the front end's own exit (`crate::exit_failed`: an input that cannot be read, a jar that does
/// not open) is told to the run, which ends as a run of one job at a time ends there.
pub(super) fn leaves(code: i32) {
    if LINK.get().is_some() {
        send(obj([("exit", num(code))]));
        let held = HELD.lock().unwrap_or_else(|e| e.into_inner());
        for dir in held.iter() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// The process ends at once: no destructor runs and nothing waits for another thread; what it held on disk for the
/// moment is removed first, and no thread makes more of it meanwhile.
fn exit_now(code: i32) -> ! {
    let held = HELD.lock().unwrap_or_else(|e| e.into_inner());
    for dir in held.iter() {
        let _ = std::fs::remove_dir_all(dir);
    }
    #[cfg(unix)]
    {
        extern "C" {
            fn _exit(code: i32) -> !;
        }
        unsafe { _exit(code) }
    }
    #[cfg(windows)]
    {
        extern "system" {
            fn GetCurrentProcess() -> isize;
            fn TerminateProcess(process: isize, code: u32) -> i32;
        }
        unsafe {
            TerminateProcess(GetCurrentProcess(), code as u32);
        }
        std::process::abort()
    }
}

/// `TEQ_FIX_JOB_PLANT=<what>:<job>`, the fix suite's: this child, where it is the job named, dies before its
/// outcomes (`die`), after them and before its end (`die-late`), writes a megabyte to stderr before its first message
/// (`noise`), writes a line and then text without a newline to stdout before its first message (`prints`), or waits
/// two seconds before it types (`slow`).
fn planted(what: &str, job: usize) -> bool {
    std::env::var("TEQ_FIX_JOB_PLANT").is_ok_and(|v| v.split(',').any(|p| p == format!("{}:{}", what, job)))
}

/// `teq compiler fix-job`: one round of one job, for the fix run that started this process. The job comes as the
/// first line of stdin; the round's record, documents and end go out as lines of stdout; the end of stdin, which is
/// the run's end, ends the process at once.
pub fn child() -> ! {
    use std::io::{BufRead, Write};
    let mut line = String::new();
    let read = std::io::stdin().lock().read_line(&mut line);
    let spec = match read.map_err(|e| e.to_string()).and_then(|_| Json::parse(line.trim_end())).and_then(|j| Spec::of_json(&j)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("teq compiler fix-job: a fix run starts it with a job on its stdin ({})", e);
            exit_now(2)
        }
    };
    if planted("noise", spec.job) {
        // A megabyte: lines of 1,023 characters.
        eprint!("{}", ("x".repeat(1023) + "\n").repeat(1 << 10));
    }
    if planted("prints", spec.job) {
        print!("job {} line\njob {} part", spec.job, spec.job);
        let _ = std::io::stdout().lock().flush();
    }
    CLOCK.store(spec.clock_us, std::sync::atomic::Ordering::Relaxed);
    now_us();
    let (grant, grants) = mpsc::channel::<usize>();
    let _ = LINK.set(Link { job: spec.job, record: spec.record, grants: std::sync::Mutex::new(grants) });
    // The reader of the run's lines: a grant goes to the round; the end of stdin is the run's end, and this process's.
    crate::alloc::spawn(move || loop {
        let mut line = String::new();
        match std::io::stdin().lock().read_line(&mut line) {
            Ok(n) if n > 0 => {
                if let Some(g) = Json::parse(line.trim_end()).ok().and_then(|j| j.get("grant").and_then(Json::uint)) {
                    let _ = grant.send(g as usize);
                }
            }
            _ => exit_now(3),
        }
    });
    if let Err(e) = child_round(spec) {
        send(obj([("failed", obj([("text", format!("{}\n", e.trim_end()).as_str().into()), ("errors", 0u32.into())]))]));
    }
    let _ = std::io::stdout().lock().flush();
    let _ = std::io::stderr().lock().flush();
    exit_now(0)
}

fn child_round(mut spec: Spec) -> Result<(), String> {
    use std::io::Write;
    let (k, r) = (spec.job, spec.round);
    let conf = spec.conf.clone();
    let (requested, sources) = requested_rules(&conf, &spec.rules)?;
    let boot = Boot { conf, config_dir: spec.config_dir.clone(), requested, sources };
    ENTRIES.lock().unwrap_or_else(|e| e.into_inner()).clear();
    crate::crew::keep_given_back();
    let began = std::time::Instant::now();
    if planted("slow", k) {
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    std::thread::scope(|scope| {
        let mut pool = Pool::new(scope, boot)?;
        let infos = match pool.from.recv() {
            Ok(FromRules::Ready(_, Ok(infos), boot)) => {
                super::record(format_args!("worker {}w=0 ready_us={} boot_us={}", worker_tag(), now_us(), boot));
                infos
            }
            Ok(FromRules::Ready(_, Err(e), _)) => return Err(e),
            _ => return Err("teq fix: the rule thread ended".to_string()),
        };
        pool.ready[0] = true;
        send(obj([("ready", self::infos(&infos))]));
        pool.eligible = true;
        pool.limit = spec.limit.max(1);
        let names: Vec<String> = infos.iter().map(|i| i.name.clone()).collect();
        let semantic_rules: Vec<bool> = infos.iter().map(|i| i.semantic).collect();
        let semantic = semantic_rules.contains(&true) || semanticdb_dump().is_some();
        let given: HashMap<(usize, String), Results> = std::mem::take(&mut spec.kept).into_iter().map(|(file, results)| ((k, file), results)).collect();
        let mut kept: HashMap<(usize, String), Results> = HashMap::new();
        let (mut gen, mut base, mut twice) = (spec.first_gen, spec.base, false);
        let earlier = Earlier { job: k, results: &given, semantic: &semantic_rules };
        let round = round((k, r), &spec.program, &spec.changed, semantic, &mut gen, &mut base, &names, &infos, &spec.display, &mut pool, earlier, (r == 1).then_some(&mut kept), None, &mut twice);
        match round {
            Ok(outcomes) => {
                if planted("die", k) {
                    exit_now(9);
                }
                if spec.delay_ms > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(spec.delay_ms));
                }
                for o in &outcomes {
                    let mut fields = vec![("outcome".to_string(), outcome(o))];
                    if let Some(results) = kept.remove(&(k, o.file.clone())) {
                        fields.push(("kept".to_string(), self::results(&results)));
                    }
                    send(Json::Obj(fields));
                }
                if planted("die-late", k) {
                    exit_now(9);
                }
                let entries = ENTRIES.lock().unwrap_or_else(|e| e.into_inner());
                send(obj([("entries", Json::Arr(entries.iter().map(|((place, written), (resolved, why))| Json::Arr(vec![place.as_str().into(), written.as_str().into(), (*resolved).into(), why.as_str().into()])).collect()))]));
                let times = TIMES.with(|t| t.get());
                send(obj([("done", obj([("docs", outcomes.len().into()), ("twice", twice.into()), ("base", base.into()), ("times", Json::Arr(times.iter().map(|&t| num(t)).collect())), ("ms", num(began.elapsed().as_millis() as f64))]))]));
            }
            Err(stop) => send(obj([("failed", obj([("text", stop.text.as_str().into()), ("errors", stop.errors.into())]))])),
        }
        // The round is told: the process ends here, nothing of its workers and its typer taken down first.
        let _ = std::io::stdout().lock().flush();
        let _ = std::io::stderr().lock().flush();
        exit_now(0)
    })
}


// ---- the run ----

/// What a job's child gave of its round: what `round` returns and leaves behind in a run of one job at a time.
pub(super) struct RoundOut {
    pub outcomes: Vec<Outcome>,
    /// Round one's results by file, for the job's round two.
    pub kept: Vec<(String, Results)>,
    /// The entries its rules asked about: place, text, whether it resolved, the first reason it did not.
    pub entries: Vec<(String, String, bool, String)>,
    pub twice: bool,
    /// Where the job's atomic groups' numbers go on from.
    pub base: u32,
    pub times: [f64; 8],
    pub took: std::time::Duration,
}

/// A job's round as the run takes it: its end, the lines of its record, and what its process wrote by itself.
pub(super) struct Taken {
    pub result: Result<RoundOut, Stop>,
    pub records: Vec<String>,
    /// What the process wrote to its stderr, and to its stdout beside its messages.
    pub raw: String,
    pub out: Vec<u8>,
    /// The code the process's front end ended it with (`leaves`), where it did.
    pub leave: Option<i32>,
}

/// Where a job's round is, for the run's status.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Phase {
    Waiting,
    Typing,
    Rules,
    Done,
    Failed,
}

struct Table {
    taken: Vec<Option<Taken>>,
    phases: Vec<Phase>,
}

struct Shared {
    table: std::sync::Mutex<Table>,
    changed: std::sync::Condvar,
}

enum Event {
    Line(usize, String),
    /// Bytes of a child's stdout that are no message's.
    Out(usize, Vec<u8>),
    OutEnd(usize),
    /// Bytes of a child's stderr.
    Err(usize, Vec<u8>),
    ErrEnd(usize),
    Stop,
    /// Nothing arrived for a tenth of a second.
    Tick,
}

/// What a job's round is started with: its job, the rules' descriptions its part's own lead worker gave (the child's
/// are held to them), the documents its round has, and the most workers its typing takes (`typing_bound`).
pub(super) struct Start {
    pub spec: Spec,
    /// The job's project, as the run's lines name it.
    pub name: String,
    pub infos: String,
    pub docs: usize,
    pub bound: usize,
}

/// The children of one round of a run: started in the jobs' order under the run's budget, each read from its start.
pub(super) struct Children {
    shared: std::sync::Arc<Shared>,
    events: mpsc::Sender<Event>,
    manager: Option<std::thread::JoinHandle<()>>,
}

/// The jobs a run's rounds run at once (`--jobs n` lowers it): as many as the run has workers, each job holding one
/// at least; one where no two jobs' typings fit the budget together (`bounds`: the most workers each one's takes).
pub(super) fn jobs_at_once(budget: usize, asked: Option<usize>, bounds: &[usize]) -> usize {
    let mut least: Vec<usize> = bounds.to_vec();
    least.sort_unstable();
    if least.len() >= 2 && least[0] + least[1] > budget {
        return 1;
    }
    asked.unwrap_or(usize::MAX).min(budget).max(1)
}

/// The run's workers: its jobs' `--threads` (or `TEQ_THREADS`), else the machine's.
pub(super) fn budget<'a>(jobs: impl Iterator<Item = &'a Job>) -> usize {
    jobs.filter_map(|j| {
        let mut args = vec!["compiler".to_string(), "check".to_string()];
        args.extend(j.args.iter().cloned());
        crate::frontend::requested_threads(crate::parse_options_from(args.into_iter()).threads)
    })
    .max()
    .unwrap_or_else(crate::workers)
    .max(1)
}

/// A job's child under way, as the manager holds it.
struct Running {
    child: std::process::Child,
    stdin: Option<std::process::ChildStdin>,
    /// The workers of the budget it holds.
    held: usize,
    /// The workers its rules asked for where the run may give it fewer; 0 before its rules ask and where a switch
    /// names their count.
    rules: usize,
    records: Vec<String>,
    /// What it wrote to its stdout beside its messages.
    out: Vec<u8>,
    outcomes: Vec<Outcome>,
    kept: Vec<(String, Results)>,
    entries: Vec<(String, String, bool, String)>,
    done: Option<(usize, bool, u32, [f64; 8], u64)>,
    failed: Option<Stop>,
    /// The code its front end ended it with (`leaves`).
    leave: Option<i32>,
    /// When its stdin was closed for it to end: it is killed where it has not ended a little later.
    closed: Option<std::time::Instant>,
    /// What it wrote to its stderr, and whether each of its streams has ended.
    err: Vec<u8>,
    out_ended: bool,
    err_ended: bool,
    /// When the manager first saw that the process had ended.
    exited: Option<std::time::Instant>,
}

impl Children {
    /// Starts the round's children: `specs` by the run's number of their job (none for a job the round does not
    /// type, and for the jobs from `never` on, which the run ends before).
    pub fn start(specs: Vec<Option<Start>>, at_once: usize, budget: usize, never: Option<usize>) -> Children {
        let n = specs.len();
        // The order the jobs are started in: theirs; `TEQ_FIX_JOBS_SCHEDULE`, the fix suite's, another.
        let order: Vec<usize> = match std::env::var("TEQ_FIX_JOBS_SCHEDULE").ok().as_deref() {
            Some("reverse") => (0..n).rev().collect(),
            Some(v) if v.starts_with("shuffle:") => {
                let seed = v["shuffle:".len()..].parse::<u64>().unwrap_or(0);
                let mut order: Vec<usize> = (0..n).collect();
                order.sort_by_key(|&k| mix(seed ^ (k as u64 + 1)));
                order
            }
            _ => (0..n).collect(),
        };
        let order: Vec<usize> = order.into_iter().filter(|&k| specs[k].is_some()).collect();
        // The first job is started at once, whatever its typing takes: what the first status says.
        let mut phases: Vec<Phase> = specs.iter().enumerate().map(|(k, s)| if s.is_some() || never.is_some_and(|from| k >= from) { Phase::Waiting } else { Phase::Done }).collect();
        if let Some(&k) = order.first() {
            phases[k] = Phase::Typing;
        }
        let shared = std::sync::Arc::new(Shared { table: std::sync::Mutex::new(Table { taken: (0..n).map(|_| None).collect(), phases }), changed: std::sync::Condvar::new() });
        let (events, inbox) = mpsc::channel::<Event>();
        let (theirs, sender) = (shared.clone(), events.clone());
        let manager = crate::alloc::spawn(move || manage(specs, order, at_once, budget, theirs, sender, inbox));
        Children { shared, events, manager: Some(manager) }
    }

    /// The round of job `k`, once its child has ended; `tick` while it has not: at once, then each fifth of a second
    /// and whenever a job's round ends.
    pub fn take(&self, k: usize, tick: &mut dyn FnMut()) -> Taken {
        let mut table = self.shared.table.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if let Some(t) = table.taken[k].take() {
                return t;
            }
            drop(table);
            tick();
            table = self.shared.table.lock().unwrap_or_else(|e| e.into_inner());
            if table.taken[k].is_none() {
                table = self.shared.changed.wait_timeout(table, std::time::Duration::from_millis(200)).unwrap_or_else(|e| e.into_inner()).0;
            }
        }
    }

    /// Where each job's round is.
    pub fn phases(&self) -> Vec<Phase> {
        self.shared.table.lock().unwrap_or_else(|e| e.into_inner()).phases.clone()
    }
}

impl Drop for Children {
    /// The round is over, however the run leaves it: every child still there is ended and waited for.
    fn drop(&mut self) {
        let _ = self.events.send(Event::Stop);
        if let Some(m) = self.manager.take() {
            let _ = m.join();
        }
    }
}

/// `TEQ_FIX_GRANT=all`: every ask of a child is granted whole, the budget counting it and bounding nothing: what a
/// measurement compares the budget's grants with.
fn grant_all() -> bool {
    std::env::var("TEQ_FIX_GRANT").is_ok_and(|v| v == "all")
}

fn manage(mut specs: Vec<Option<Start>>, order: Vec<usize>, at_once: usize, budget: usize, shared: std::sync::Arc<Shared>, sender: mpsc::Sender<Event>, inbox: mpsc::Receiver<Event>) {
    use std::io::Write;
    let mut pending: std::collections::VecDeque<usize> = order.into();
    let mut running: HashMap<usize, Running> = HashMap::new();
    let mut expected: HashMap<usize, (String, usize, String)> = HashMap::new();
    let (mut used, mut cut, mut stopping) = (0usize, usize::MAX, false);
    let publish = |k: usize, taken: Taken, phase: Phase| {
        let mut table = shared.table.lock().unwrap_or_else(|e| e.into_inner());
        table.taken[k] = Some(taken);
        table.phases[k] = phase;
        shared.changed.notify_all();
    };
    let set_phase = |k: usize, phase: Phase| {
        shared.table.lock().unwrap_or_else(|e| e.into_inner()).phases[k] = phase;
    };
    // Whether a job, or a typing of several workers, has ended since the workers were last shared out.
    let mut freed = false;
    // When the children's processes were last looked at.
    let mut looked = std::time::Instant::now();
    loop {
        // The jobs the budget lets in, in their order; none after a job that failed. A job is started when the most
        // workers its typing takes are free, or when nothing runs: it holds them until its typing says its count.
        while !stopping && running.len() < at_once {
            let Some(&k) = pending.front() else { break };
            if k > cut {
                pending.pop_front();
                continue;
            }
            let bound = specs[k].as_ref().map_or(1, |s| s.bound.max(1));
            if !running.is_empty() && used + bound > budget {
                break;
            }
            pending.pop_front();
            let Start { spec, name, infos, docs, .. } = specs[k].take().expect("a job started once");
            expected.insert(k, (infos, docs, name.clone()));
            let started = std::env::current_exe().map_err(|e| e.to_string()).and_then(|exe| {
                // Its last argument is this process's id: what a process list shows of whose job it is.
                std::process::Command::new(exe)
                    .args(["compiler", "fix-job", &std::process::id().to_string()])
                    .env_remove("TEQ_FIX_RECORD")
                    .env_remove("TEQ_FIX_INPUTS")
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .map_err(|e| e.to_string())
            });
            let mut child = match started {
                Ok(c) => c,
                Err(e) => {
                    cut = cut.min(k);
                    publish(k, Taken { result: Err(format!("teq fix: the system gave no process to type {} ({})\n", name, e).into()), records: Vec::new(), raw: String::new(), out: Vec::new(), leave: None }, Phase::Failed);
                    continue;
                }
            };
            // Both of its streams are read from its start, each on a thread of its own: a child never waits on a
            // pipe the run does not read.
            if let Some(out) = child.stdout.take() {
                let tx = sender.clone();
                crate::alloc::spawn(move || {
                    use std::io::BufRead;
                    let mut reader = std::io::BufReader::new(out);
                    let mut bytes = Vec::new();
                    loop {
                        bytes.clear();
                        match reader.read_until(b'\n', &mut bytes) {
                            Ok(len) if len > 0 => {
                                // A message begins at its mark; what stands before it, and a line without one, is
                                // the process's own.
                                let mark = bytes.iter().position(|&b| b == MESSAGE as u8);
                                let own = &bytes[..mark.unwrap_or(bytes.len())];
                                if !own.is_empty() && tx.send(Event::Out(k, own.to_vec())).is_err() {
                                    break;
                                }
                                if let Some(at) = mark {
                                    if tx.send(Event::Line(k, String::from_utf8_lossy(&bytes[at + 1..]).trim_end_matches(['\n', '\r']).to_string())).is_err() {
                                        break;
                                    }
                                }
                            }
                            _ => break,
                        }
                    }
                    let _ = tx.send(Event::OutEnd(k));
                });
            }
            if let Some(mut err) = child.stderr.take() {
                let tx = sender.clone();
                crate::alloc::spawn(move || {
                    use std::io::Read;
                    let mut bytes = vec![0u8; 1 << 16];
                    loop {
                        match err.read(&mut bytes) {
                            Ok(len) if len > 0 => {
                                if tx.send(Event::Err(k, bytes[..len].to_vec())).is_err() {
                                    break;
                                }
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                            _ => break,
                        }
                    }
                    let _ = tx.send(Event::ErrEnd(k));
                });
            }
            let mut stdin = child.stdin.take();
            // The child's clock goes on from the run's at its start.
            let mut spec = spec;
            spec.clock_us = now_us();
            let mut line = spec.to_json().to_text();
            line.push('\n');
            if let Some(i) = stdin.as_mut() {
                let _ = i.write_all(line.as_bytes()).and_then(|()| i.flush());
            }
            used += bound;
            set_phase(k, Phase::Typing);
            running.insert(k, Running { child, stdin, held: bound, rules: 0, records: Vec::new(), out: Vec::new(), outcomes: Vec::new(), kept: Vec::new(), entries: Vec::new(), done: None, failed: None, leave: None, closed: None, err: Vec::new(), out_ended: false, err_ended: false, exited: None });
        }
        if running.is_empty() && (stopping || pending.is_empty()) {
            break;
        }
        if std::mem::take(&mut freed) && !stopping {
            // What an end left free goes to the jobs whose rules got fewer workers than they asked: one more each in
            // the jobs' order, up to each one's share, while any is free.
            let share = share(budget, running.len());
            let mut order: Vec<usize> = running.keys().copied().collect();
            order.sort_unstable();
            let before: Vec<usize> = order.iter().map(|k| running[k].held).collect();
            loop {
                let mut gave = false;
                for k in &order {
                    let r = running.get_mut(k).expect("a job under way");
                    if used < budget && r.held < r.rules.min(share) {
                        r.held += 1;
                        used += 1;
                        gave = true;
                    }
                }
                if !gave {
                    break;
                }
            }
            for (k, held) in order.iter().zip(before) {
                let r = running.get_mut(k).expect("a job under way");
                if r.held > held {
                    tell_grant(r);
                }
            }
        }
        // `quiet`: nothing was waiting to be read, so what every child had written until then has been taken in.
        let (event, quiet) = match inbox.try_recv() {
            Ok(event) => (event, false),
            Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => match inbox.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(event) => (event, true),
                Err(mpsc::RecvTimeoutError::Timeout) => (Event::Tick, true),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            },
        };
        match event {
            Event::Tick => {}
            Event::Stop => {
                stopping = true;
                pending.clear();
                for r in running.values_mut() {
                    // Its stdin's end ends a child at once; one that has not ended a little later is killed (below).
                    r.stdin = None;
                    r.closed.get_or_insert_with(std::time::Instant::now);
                }
            }
            Event::Line(k, line) => {
                let live = running.len();
                let Some(r) = running.get_mut(&k) else { continue };
                let message = Json::parse(&line).ok().filter(|j| matches!(j, Json::Obj(_)));
                let Some(j) = message else {
                    // Whatever it was, it is not answered: the child is ended.
                    r.failed = Some(format!("teq fix: {} sent a message that does not read\n", process_of(expected.get(&k).map_or("a project", |e| e.2.as_str()))).into());
                    r.stdin = None;
                    r.closed.get_or_insert_with(std::time::Instant::now);
                    continue;
                };
                if let Some(text) = j.get("record").and_then(Json::str) {
                    r.records.push(text.to_string());
                } else if let Some(o) = j.get("outcome") {
                    match outcome_of(o) {
                        Ok(o) => {
                            if let Some(kept) = j.get("kept").and_then(|k| results_of(k).ok()) {
                                r.kept.push((o.file.clone(), kept));
                            }
                            r.outcomes.push(o);
                        }
                        Err(e) => r.failed = Some(format!("teq fix: {}\n", e).into()),
                    }
                } else if let Some(w) = j.get("want") {
                    let asked = w.get("workers").and_then(Json::uint).unwrap_or(1).max(1) as usize;
                    let whole = w.get("whole").and_then(Json::bool) == Some(true) || grant_all();
                    let rules = w.get("phase").and_then(Json::str) == Some("rules");
                    // Answered at once. A typing says the count it types with, which is the job's from then on (what
                    // it was started with beyond that is free again). Rules get, of what is free beside what the job
                    // holds, its share at most, never less than it holds; a job that got fewer than it asked is given
                    // more as other jobs end (below).
                    let free = budget.saturating_sub(used);
                    let grant = if whole { asked } else { asked.min(share(budget, live)).min(r.held + free).max(r.held).max(1) };
                    if !rules && asked > r.held {
                        // A typing over more workers than the job was started with (`typing_bound`): the record says so.
                        r.records.push(format!("overdraw job={} bound={} asked={}", k, r.held, asked));
                    }
                    freed |= grant < r.held;
                    used = (used + grant).saturating_sub(r.held);
                    r.held = grant;
                    r.rules = if whole || !rules { 0 } else { asked };
                    tell_grant(r);
                    if rules {
                        set_phase(k, Phase::Rules);
                    }
                } else if let Some(w) = j.get("where") {
                    if w.get("typed").is_some() {
                        // Its typing's workers are the run's again: the job holds one until its rules ask.
                        used = used.saturating_sub(r.held.saturating_sub(1));
                        freed |= r.held > 1;
                        r.held = 1;
                    }
                    if w.get("typed").is_some() || w.get("done").is_some() {
                        set_phase(k, Phase::Rules);
                    }
                } else if let Some(i) = j.get("ready") {
                    if let Some((_, _, name)) = expected.get(&k).filter(|(infos, _, _)| *infos != i.to_text()) {
                        r.failed = Some(format!("teq fix: {} configured the rules otherwise than the run\n", process_of(name)).into());
                    }
                } else if let Some(e) = j.get("entries") {
                    for e in e.arr() {
                        if let [place, written, resolved, why] = e.arr() {
                            r.entries.push((place.str().unwrap_or("").to_string(), written.str().unwrap_or("").to_string(), resolved.bool() == Some(true), why.str().unwrap_or("").to_string()));
                        }
                    }
                } else if let Some(d) = j.get("done") {
                    let mut times = [0.0; 8];
                    for (t, v) in times.iter_mut().zip(d.get("times").map_or(&[][..], Json::arr)) {
                        *t = v.num().unwrap_or(0.0);
                    }
                    r.done = Some((d.get("docs").and_then(Json::uint).unwrap_or(0) as usize, d.get("twice").and_then(Json::bool) == Some(true), d.get("base").and_then(Json::uint).unwrap_or(0), times, d.get("ms").and_then(Json::num).unwrap_or(0.0) as u64));
                } else if let Some(code) = j.get("exit").and_then(Json::num) {
                    r.leave = Some(code as i32);
                } else if let Some(f) = j.get("failed") {
                    r.failed = Some(Stop { text: f.get("text").and_then(Json::str).unwrap_or("").to_string(), errors: f.get("errors").and_then(Json::uint).unwrap_or(0) as usize });
                }
            }
            Event::Out(k, bytes) => {
                if let Some(r) = running.get_mut(&k) {
                    r.out.extend(bytes);
                }
            }
            Event::OutEnd(k) => {
                if let Some(r) = running.get_mut(&k) {
                    r.out_ended = true;
                }
            }
            Event::Err(k, bytes) => {
                if let Some(r) = running.get_mut(&k) {
                    r.err.extend(bytes);
                }
            }
            Event::ErrEnd(k) => {
                if let Some(r) = running.get_mut(&k) {
                    r.err_ended = true;
                }
            }
        }
        if looked.elapsed() >= std::time::Duration::from_millis(100) {
            looked = std::time::Instant::now();
            for r in running.values_mut() {
                if r.exited.is_none() {
                    if matches!(r.child.try_wait(), Ok(Some(_))) {
                        r.exited = Some(looked);
                    } else if r.closed.is_some_and(|at| at.elapsed() >= LINGER) {
                        let _ = r.child.kill();
                    }
                }
            }
        }
        // A child whose two streams have ended has ended; and so has one whose process ended two seconds ago while
        // something it started still holds a stream of its open (a rule's own process, which the run does not wait
        // for, whatever it goes on writing there), once nothing was waiting to be read; a run that stops takes a
        // process's end alone. It is waited for, and its round is the run's to take.
        let mut over: Vec<usize> = running.iter().filter(|(_, r)| (r.out_ended && r.err_ended) || r.exited.is_some_and(|at| stopping || (quiet && at.elapsed() >= HELD_OPEN))).map(|(&k, _)| k).collect();
        over.sort_unstable();
        for k in over {
            let mut r = running.remove(&k).expect("the child that ended");
            r.stdin = None;
            let status = r.child.wait();
            used = used.saturating_sub(r.held);
            freed = true;
            if stopping {
                continue;
            }
            ended(k, r, status, &mut cut, &mut running, &expected, &publish);
        }
    }
    // Whatever is left (a run that stops) is waited for: no child outlives its round.
    for (_, mut r) in running {
        r.stdin = None;
        let _ = r.child.kill();
        let _ = r.child.wait();
    }
}

/// How long after a child's process ended the run goes on reading a stream of its that has not ended.
const HELD_OPEN: std::time::Duration = std::time::Duration::from_secs(2);

/// How long a child whose stdin was closed has to end by itself, removing what it holds on disk, before it is killed.
const LINGER: std::time::Duration = std::time::Duration::from_secs(2);

/// The round of the child `r` of job `k`, which has ended with `status`, made the run's to take; where it failed, the
/// jobs after it in the jobs' order are the run's no longer.
fn ended(k: usize, r: Running, status: std::io::Result<std::process::ExitStatus>, cut: &mut usize, running: &mut HashMap<usize, Running>, expected: &HashMap<usize, (String, usize, String)>, publish: &dyn Fn(usize, Taken, Phase)) {
    let raw = String::from_utf8_lossy(&r.err).into_owned();
    let result = match (r.failed, r.done) {
        (Some(stop), _) => Err(stop),
        (None, Some((docs, twice, base, times, ms))) => match expected.get(&k) {
            Some((_, counted, name)) if *counted != docs || docs != r.outcomes.len() => Err(format!("teq fix: {} gave {} documents of the {} it counted before it typed\n", process_of(name), r.outcomes.len(), counted).into()),
            _ => Ok(RoundOut { outcomes: r.outcomes, kept: r.kept, entries: r.entries, twice, base, times, took: std::time::Duration::from_millis(ms) }),
        },
        (None, None) => Err(format!("teq fix: {} ended early ({})\n", process_of(expected.get(&k).map_or("a project", |e| e.2.as_str())), status.map_or_else(|e| e.to_string(), |s| s.to_string())).into()),
    };
    let phase = if result.is_ok() { Phase::Done } else { Phase::Failed };
    if result.is_err() && k < *cut {
        // An immediate failure: the jobs after it are ended, each at its stdin's end.
        *cut = k;
        for (&later, r) in running.iter_mut() {
            if later > *cut {
                r.stdin = None;
                r.closed.get_or_insert_with(std::time::Instant::now);
            }
        }
    }
    publish(k, Taken { result, records: r.records, raw, out: r.out, leave: r.leave }, phase);
}

/// How a line names the process of a job of `project`.
fn process_of(project: &str) -> String {
    format!("the process that typed {} and ran its rules", project)
}

/// A job's share of the run's workers for its rules, among the `live` jobs under way: one at least.
fn share(budget: usize, live: usize) -> usize {
    (budget / live.max(1)).max(1)
}

/// The job's child told the workers it holds now.
fn tell_grant(r: &mut Running) {
    use std::io::Write;
    if let Some(i) = r.stdin.as_mut() {
        let _ = i.write_all(format!("{{\"grant\":{}}}\n", r.held).as_bytes()).and_then(|()| i.flush());
    }
}

/// The names as a line lists them: three at most by name, a longer list its first two and the count of the rest.
fn listed(names: &[&str]) -> String {
    match names {
        [a, b, rest @ ..] if rest.len() > 1 => format!("{}, {} and {} others", a, b, rest.len()),
        _ => names.join(", "),
    }
}

/// The run's status where its jobs run side by side: the projects done of the round's, and what runs: `teq fix: 2 of
/// 17 projects done, typing api, core, web`, `.. running the rules on api, web`, both where both run; round two's
/// typing is `again`; a job that failed while one before it still runs is named after them.
pub(super) fn status_text(done: usize, of: usize, typing: &[&str], rules: &[&str], again: bool, failed: Option<&str>) -> String {
    let mut text = format!("teq fix: {} of {} done", done, counted(of, "project", "projects"));
    if !typing.is_empty() {
        text.push_str(&format!(", typing {}{}", listed(typing), if again { " again" } else { "" }));
    }
    if !rules.is_empty() {
        text.push_str(&format!(", running the rules on {}", listed(rules)));
    }
    if let Some(project) = failed {
        text.push_str(&format!(" ({} does not type)", project));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_crosses_between_a_run_and_its_job_reads_back_as_it_was_written() {
        let r = Results {
            edits: vec![host::Edit { rule: 1, group: 3, op: host::Op::Right, token: 7, text: "x\n\"y\"".to_string() }],
            imports: vec![host::ImportOp::Remove { rule: 0, group: 0, importee: 12 }, host::ImportOp::Add { rule: 2, group: 4, importer: "a.b.c".to_string() }],
            lints: vec![host::Lint { rule: 1, category: "get".to_string(), message: "Don't".to_string(), severity: Severity::Warning, span: Span { start: 4, end: 9 }, group: 0 }],
            failures: vec![(2, "the rule threw".to_string())],
        };
        let back = results_of(&Json::parse(&results(&r).to_text()).unwrap()).unwrap();
        assert_eq!(results(&back).to_text(), results(&r).to_text());
        assert_eq!((back.edits.len(), back.imports.len(), back.lints[0].span.end, back.failures[0].1.as_str()), (1, 2, 9, "the rule threw"));
        let o = Outcome {
            file: "a/src/A.scala".to_string(),
            doc_path: "a/src/A.scala".to_string(),
            text: "object A\n".to_string(),
            plan: Plan {
                edits: vec![(Span { start: 0, end: 6 }, "class".to_string(), vec![0, 2])],
                lints: vec![Printed { id: "OptionGetWarn".to_string(), message: "m".to_string(), severity: Severity::Error, span: Span { start: 1, end: 2 } }],
                refused: vec!["a/src/A.scala:1:1: the edits of A and B overlap".to_string()],
            },
        };
        let back = outcome_of(&Json::parse(&outcome(&o).to_text()).unwrap()).unwrap();
        assert_eq!(outcome(&back).to_text(), outcome(&o).to_text());
        let spec = Spec {
            job: 2,
            round: 2,
            program: Job { key: "api/test".to_string(), args: vec!["api/src".to_string(), "--threads".to_string(), "4".to_string()], roots: Some(vec!["api/src".to_string()]), files: None, generated: vec!["api/target/gen".to_string()] },
            conf: Conf { file: ".scalafix.conf".to_string(), files: vec![".scalafix.conf".to_string()], nodes: vec![crate::fix::config::Node { value: crate::fix::config::Value::Obj(vec![("rules".to_string(), 1)]), file: 0, line: 1, col: 1 }, crate::fix::config::Node { value: crate::fix::config::Value::List(vec![2]), file: 0, line: 1, col: 9 }, crate::fix::config::Node { value: crate::fix::config::Value::Str("DisableSyntax".to_string()), file: 0, line: 1, col: 10 }] },
            config_dir: PathBuf::from("/build"),
            rules: Some(vec!["DisableSyntax".to_string()]),
            display: Some((PathBuf::from("/build"), PathBuf::from("/build/api"))),
            first_gen: 1322,
            base: 17,
            clock_us: 123_456,
            changed: HashMap::from([("api/src/A.scala".to_string(), "object A\n".to_string())]),
            kept: vec![("api/src/B.scala".to_string(), r)],
            limit: 14,
            delay_ms: 0,
            record: true,
        };
        let back = Spec::of_json(&Json::parse(&spec.to_json().to_text()).unwrap()).unwrap();
        assert_eq!(back.to_json().to_text(), spec.to_json().to_text());
        assert_eq!((back.job, back.round, back.first_gen, back.base, back.clock_us, back.program.roots.as_deref().map(<[String]>::len), back.program.files.is_none()), (2, 2, 1322, 17, 123_456, Some(1), true));
    }

    #[test]
    fn the_run_s_status_says_the_projects_done_and_what_runs() {
        assert_eq!(status_text(0, 17, &["api", "core", "web"], &[], false, None), "teq fix: 0 of 17 projects done, typing api, core, web");
        assert_eq!(status_text(2, 17, &["api", "core", "web", "a", "b", "c", "d", "e"], &[], false, None), "teq fix: 2 of 17 projects done, typing api, core and 6 others");
        assert_eq!(status_text(9, 17, &["admin"], &["api", "web"], false, None), "teq fix: 9 of 17 projects done, typing admin, running the rules on api, web");
        assert_eq!(status_text(9, 17, &[], &["api", "web"], false, None), "teq fix: 9 of 17 projects done, running the rules on api, web");
        assert_eq!(status_text(16, 17, &[], &["api"], false, None), "teq fix: 16 of 17 projects done, running the rules on api");
        assert_eq!(status_text(1, 3, &["api"], &["web"], true, None), "teq fix: 1 of 3 projects done, typing api again, running the rules on web");
        assert_eq!(status_text(3, 17, &["api"], &[], false, Some("core")), "teq fix: 3 of 17 projects done, typing api (core does not type)");
        assert_eq!(status_text(0, 1, &["api"], &[], false, None), "teq fix: 0 of 1 project done, typing api");
    }

    #[test]
    fn the_jobs_at_once_are_the_budget_s_unless_fewer_are_asked() {
        assert_eq!(jobs_at_once(14, None, &[1, 1, 1]), 14);
        assert_eq!(jobs_at_once(14, Some(4), &[1, 1, 1]), 4);
        assert_eq!(jobs_at_once(4, Some(8), &[1, 1]), 4);
        assert_eq!(jobs_at_once(0, None, &[]), 1);
        assert_eq!(jobs_at_once(14, Some(1), &[1, 1]), 1);
        // No two typings fit the budget together: one job at a time.
        assert_eq!(jobs_at_once(4, None, &[4, 4, 4]), 1);
        assert_eq!(jobs_at_once(8, None, &[8, 1, 8]), 1);
        assert_eq!(jobs_at_once(16, None, &[8, 8, 8]), 16);
        assert_eq!(jobs_at_once(8, None, &[8, 1, 1]), 8);
    }
}
