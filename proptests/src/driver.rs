//! One history: the machines write down what every step leaves on disk and which build it
//! asks for, and once the history is complete it is run: the sources are written, the session
//! builds and every build is held against a fresh one (`oracle`).
//!
//! A history is run after its last rule because the shrinker asks for the same history, and
//! for histories through the same states, thousands of times: one that passed before in this
//! process is not run again, and a fresh build is made once per state of the sources, under
//! the rules of `confirm`: a history that failed before runs with neither and is replayed.

use crate::json::Json;
use crate::oracle;
use crate::session::{self, Kind, Session};
use crate::stats::Stats;
use crate::tree::{self, Tree};
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Clone, Debug, Hash, PartialEq)]
pub enum Due {
    /// `build` with the paths of the files whose text changed.
    Changed,
    /// `build` on its own, which looks at every file's time.
    Plain,
    /// `build` naming a file whose text did not change.
    Unchanged(String),
}

pub type Texts = BTreeMap<String, String>;

struct Step {
    rule: &'static str,
    due: Due,
    texts: Texts,
    /// The files that hold a macro's call.
    macro_files: BTreeSet<String>,
    /// Whether an injected fault stands in the program, so that a failed build is meant.
    faulty: bool,
}

/// What a history is made of, written down while the machine runs.
pub struct Plan {
    kind: Kind,
    test: String,
    first: Texts,
    steps: Vec<Step>,
    due: Option<(&'static str, Due)>,
    words: Vec<String>,
    /// Whether the history has ended before the machine's last rule: nothing is written
    /// down after it.
    ended: bool,
}

static FAILURES: AtomicUsize = AtomicUsize::new(0);

/// A name no other invocation writes under: the process's start time and id.
pub fn invocation() -> String {
    static STARTED: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    let started = STARTED.get_or_init(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()));
    format!("{started}-{}", std::process::id())
}
static FRESH: Mutex<BTreeMap<u64, (Json, Tree)>> = Mutex::new(BTreeMap::new());
static VERDICTS: crate::confirm::Verdicts = crate::confirm::Verdicts::new();

fn hash_of(value: impl Hash) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

impl Plan {
    pub fn new(kind: Kind, test: &str, first: Texts) -> Plan {
        Plan { kind, test: test.to_string(), first, steps: Vec::new(), due: None, words: Vec::new(), ended: false }
    }

    pub fn say(&mut self, what: String) {
        if !self.ended {
            self.words.push(format!("step {}: {what}", self.steps.len() + 1));
        }
    }

    /// A rule ran and asks for a build.
    pub fn rule(&mut self, name: &'static str, due: Due) {
        if !self.ended {
            self.due = Some((name, due));
        }
    }

    /// The state the rule left, if one ran since the last call.
    pub fn step(&mut self, texts: Texts, macro_files: BTreeSet<String>, faulty: bool) {
        if let Some((rule, due)) = self.due.take() {
            self.steps.push(Step { rule, due, texts, macro_files, faulty });
        }
    }

    /// Ends the history at its last step: the rules the machine still runs change the model
    /// alone.
    pub fn end(&mut self) {
        self.ended = true;
    }

    fn key(&self) -> u64 {
        let steps: Vec<(&Due, &Texts)> = self.steps.iter().map(|step| (&step.due, &step.texts)).collect();
        hash_of((self.kind.name(), &self.test, &self.first, steps))
    }

    /// Runs the history; panics with the report of the first build that differs.
    pub fn run(&self) {
        let attempt = |cached: bool| {
            let mut run = Run::start(self, cached);
            for step in &self.steps {
                run.step(step);
            }
        };
        if let crate::confirm::Outcome::Skipped = VERDICTS.check(self.key(), &attempt) {
            let mut stats = Stats::new();
            stats.outcome = "passed before";
            stats.planned = self.steps.len() as u32;
            stats.write(&crate::work_root(), &self.test);
        }
    }
}

struct Run<'a> {
    plan: &'a Plan,
    teq: PathBuf,
    work: PathBuf,
    /// This invocation's directory under `work`: the sources, the session's output, the fresh
    /// builds and the history's record.
    run: PathBuf,
    src: PathBuf,
    /// Whether a fresh build may come from the cache.
    cached: bool,
    session: Option<Session>,
    written: Texts,
    last_good: Tree,
    at: usize,
    error_run: u32,
    stats: Stats,
}

impl Run<'_> {
    fn start(plan: &Plan, cached: bool) -> Run<'_> {
        let work = crate::work_root().join(&plan.test);
        let dir = work.join(format!("run-{}", invocation()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        let src = dir.join("src");
        std::fs::create_dir_all(&src).unwrap();
        let mut run = Run {
            plan,
            teq: crate::teq(),
            work,
            run: dir,
            src,
            cached,
            session: None,
            written: Texts::new(),
            last_good: Tree::new(),
            at: 0,
            error_run: 0,
            stats: Stats::new(),
        };
        let changed = run.sync(&plan.first);
        run.record(&changed, "the first build");
        let started = Session::start(&run.teq, plan.kind, &[run.src.clone()], &run.run);
        let answer = match started {
            Ok((session, answer)) => {
                run.session = Some(session);
                answer
            }
            Err(problem) => run.fail(vec![problem]),
        };
        if !answer.get("ok").is_true() {
            run.stats.outcome = "generator";
            let refused = format!("the generator's program is refused, the generator's defect or a finding of its own: {}", answer.get("errors"));
            run.fail(vec![refused]);
        }
        run.compare(&answer, &BTreeSet::new(), false);
        run
    }

    fn step(&mut self, step: &Step) {
        self.at += 1;
        *self.stats.rules.entry(step.rule).or_insert(0) += 1;
        let changed = self.sync(&step.texts);
        let named: Vec<PathBuf> = match &step.due {
            Due::Plain => Vec::new(),
            Due::Unchanged(name) => vec![self.src.join(name)],
            Due::Changed => changed.iter().map(|name| self.src.join(name)).collect(),
        };
        let command = match named.is_empty() {
            true => "build".to_string(),
            false => format!("build {}", names(&named)),
        };
        self.record(&changed, &command);
        if changed.len() > 1 {
            self.stats.several_files += 1;
        }
        let answer = match self.session.as_mut().unwrap().build(&named) {
            Ok(answer) => answer,
            Err(problem) => self.fail(vec![problem]),
        };
        self.compare(&answer, &step.macro_files, step.faulty);
    }

    fn sync(&mut self, texts: &Texts) -> Vec<String> {
        let mut changed = Vec::new();
        for (name, text) in texts {
            if self.written.get(name) != Some(text) {
                std::fs::write(self.src.join(name), text).unwrap();
                changed.push(name.clone());
            }
        }
        for name in self.written.keys() {
            if !texts.contains_key(name) {
                std::fs::remove_file(self.src.join(name)).unwrap();
                changed.push(name.clone());
            }
        }
        self.written = texts.clone();
        changed
    }

    /// Keeps the step for a replay by hand: the files it wrote, the ones it removed and the
    /// command, under `run/history/<step>`.
    fn record(&self, changed: &[String], command: &str) {
        let dir = self.run.join(format!("history/{:03}", self.at));
        std::fs::create_dir_all(dir.join("files")).unwrap();
        let mut removed = String::new();
        for name in changed {
            match self.written.get(name) {
                Some(text) => std::fs::write(dir.join("files").join(name), text).unwrap(),
                None => removed.push_str(&format!("{name}\n")),
            }
        }
        std::fs::write(dir.join("removed"), removed).unwrap();
        std::fs::write(dir.join("command"), command).unwrap();
    }

    fn fresh(&mut self) -> (Json, Tree) {
        let key = hash_of((self.plan.kind.name(), &self.src, &self.written));
        if self.cached {
            if let Some(known) = FRESH.lock().unwrap().get(&key) {
                self.stats.fresh_cached += 1;
                return known.clone();
            }
        }
        let work = self.run.join("fresh");
        match session::fresh(&self.teq, self.plan.kind, &[self.src.clone()], &work) {
            Ok(fresh) => {
                FRESH.lock().unwrap().insert(key, fresh.clone());
                fresh
            }
            Err(problem) => self.fail(vec![problem]),
        }
    }

    fn compare(&mut self, answer: &Json, macro_files: &BTreeSet<String>, faulty: bool) {
        let stats = &mut self.stats;
        stats.builds += 1;
        let retyped = answer.get("retyped").items();
        if !answer.get("incremental").is_true() {
            stats.full += 1;
            *stats.fallbacks.entry(reason(answer.get("fallback").text().unwrap_or("none given"))).or_insert(0) += 1;
        } else if retyped.is_empty() {
            stats.unchanged += 1;
        } else {
            stats.incremental += 1;
        }
        stats.retyped_files += retyped.len() as u32;
        let of_macros = |path: &Json| path.text().is_some_and(|path| macro_files.iter().any(|name| path.ends_with(&format!("/{name}"))));
        if answer.get("incremental").is_true() && retyped.iter().any(of_macros) {
            stats.macro_retypes += 1;
        }
        if answer.get("ok").is_true() {
            self.error_run = 0;
        } else {
            stats.failed_builds += 1;
            if !faulty {
                stats.generator_errors += 1;
            }
            self.error_run += 1;
            stats.longest_error_run = stats.longest_error_run.max(self.error_run);
        }
        let (fresh, fresh_out) = self.fresh();
        let out = match self.plan.kind {
            Kind::Split => tree::read(&self.session.as_ref().unwrap().out),
            Kind::Check => Tree::new(),
        };
        let found = oracle::differences(self.plan.kind, answer, &fresh, &out, &fresh_out, &self.last_good);
        if answer.get("ok").is_true() {
            self.last_good = out;
        }
        if !found.is_empty() {
            let path = match answer.get("incremental").is_true() {
                true => format!("incremental, retyped {}", answer.get("retyped")),
                false => format!("full, {}", answer.get("fallback")),
            };
            let mut report = vec![format!("the build of step {} ({path}) differs from a fresh build", self.at)];
            report.extend(found);
            self.fail(report);
        }
    }

    fn fail(&mut self, mut report: Vec<String>) -> ! {
        if self.stats.outcome == "passed" {
            self.stats.outcome = "failed";
        }
        let kind = kind(&report);
        report.push(format!("{}{kind}", crate::confirm::KIND));
        if crate::confirm::replaying() || !crate::confirm::kept(&kind) {
            self.stats.outcome = "replayed and failed";
            panic!("{}", report.join("\n"));
        }
        let n = FAILURES.fetch_add(1, Ordering::Relaxed);
        let kept = self.work.join(format!("failures/{}-{n:04}", invocation()));
        std::fs::create_dir_all(&kept).unwrap();
        copy(&self.run.join("history"), &kept.join("history"));
        report.push(format!("the history, {} steps of which {} were run:", self.plan.steps.len(), self.at));
        report.extend(self.plan.words.iter().map(|line| format!("  {line}")));
        report.push(format!("the history's files: {}", kept.display()));
        report.push(crate::confirm::named(self.plan.key()));
        let report = report.join("\n");
        std::fs::write(kept.join("report.txt"), &report).unwrap();
        panic!("{report}");
    }
}

impl Drop for Run<'_> {
    fn drop(&mut self) {
        self.session = None;
        let _ = std::fs::remove_dir_all(&self.run);
        self.stats.planned = self.plan.steps.len() as u32;
        self.stats.write(&crate::work_root(), &self.plan.test);
    }
}

fn names(paths: &[PathBuf]) -> String {
    let names: Vec<String> = paths.iter().map(|path| path.file_name().unwrap().to_string_lossy().into_owned()).collect();
    names.join(" ")
}

/// A fallback's reason without the names it carries, so that the reasons can be counted.
fn reason(text: &str) -> String {
    let text = match text.rfind(".scala: ") {
        Some(at) => &text[at + 8..],
        None => text,
    };
    let words: Vec<&str> = text.split(' ').filter(|word| !word.chars().any(|c| c.is_ascii_digit()) && !word.ends_with(':')).collect();
    words.join(" ")
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

/// The kind of a session's failure (`confirm::KIND`): the kind of build and the first line of
/// each difference, or the first line of another failure, what varies left out.
fn kind(report: &[String]) -> String {
    let first = report.first().map(String::as_str).unwrap_or_default();
    match first.contains("differs from a fresh build") {
        true => {
            let build = if first.contains("(incremental") { "incremental" } else { "full" };
            let differences: Vec<String> = report[1..].iter().filter(|line| !line.starts_with(' ')).map(|line| crate::confirm::normalize(line)).collect();
            format!("{build} build: {}", differences.join("; "))
        }
        false => crate::confirm::normalize(first.split(": [").next().unwrap_or_default()),
    }
}
