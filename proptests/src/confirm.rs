//! How a property's verdicts are cached and confirmed. The shrinker asks for the same input,
//! and for inputs through the same states, many times over: an input that passed before in
//! this process is not run again while the engine shrinks. An input that failed before is
//! never skipped and runs with no cache at all, which is what the engine's confirmation run
//! of the minimal failing input gets.
//!
//! An input is replayed `TEQ_PROP_REPLAYS` (20) more times with no cache, the harness held
//! still, the first time its verdict changes in the process: it failed and then passed, it
//! passed and then failed (which a run with `TEQ_PROP_NO_CACHE` can see), or it failed with
//! another report than the first. Besides, the first input of a property and process that
//! fails a second time with the same report is replayed as a sample. Each replay's outcome
//! is kept, normalised to the report's finding (the lines before the history and the saved
//! files), and counted as the original failure reproduced, a failure with another report, or
//! a pass: a rate under all of them is the compiler's nondeterminism or the harness's, to be
//! told apart by hand. The counts go to stderr, which the failure's output shows, and to
//! `<work>/replays.txt`; the outcomes to `<work>/replays/<input>-<invocation>.txt`. The input
//! is named by its key, which the saved failure's `report.txt` carries: the sample is of that
//! input, one the shrinker reached, and says nothing of the minimal one the engine reports at
//! the end. `TEQ_PROP_NO_CACHE` turns the skipping off altogether.
//!
//! A report names its failure's kind on a line of its own (`the failure's kind: ..`): which
//! targets print alike on the lines that differ, which target refuses the program and with what
//! message, which fields of a session's answer differ, numbers and paths left out. The engine
//! tells failures apart by where the panic stands alone, so a shrinker may trade the failure it
//! was given for another one of the same program; with `TEQ_PROP_KEEP` set to a kind, a failure
//! of another kind counts as a pass, which keeps a shrink run on the failure it shrinks.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static REPLAYING: AtomicBool = AtomicBool::new(false);
static FAILED: AtomicBool = AtomicBool::new(false);

/// Whether an input failed in the process.
pub fn failed() -> bool {
    FAILED.load(Ordering::Relaxed)
}

/// How a report names its failure's kind.
pub const KIND: &str = "the failure's kind: ";

/// The kind a report names, if it names one.
pub fn kind_of(report: &str) -> Option<&str> {
    report.lines().find_map(|line| line.strip_prefix(KIND))
}

/// Whether a failure of this kind is the one the run keeps (`TEQ_PROP_KEEP`), which every kind
/// is where none is set.
pub fn kept(kind: &str) -> bool {
    std::env::var("TEQ_PROP_KEEP").map_or(true, |keep| keep.is_empty() || keep == kind)
}

/// A line of a report with what varies between failures of one kind left out: numbers, paths,
/// quoted text, and the names and types in the typer's messages that do.
pub fn normalize(text: &str) -> String {
    let mut text = text.to_string();
    for (start, middle, then) in [("value ", " is not a member of ", "value _ is not a member of _"), ("operator ", " cannot be applied to ", "operator _ cannot be applied to _"), ("found ", ", required ", "found _, required _")] {
        if let Some(at) = text.find(start) {
            if text[at..].contains(middle) {
                text = format!("{}{then}", &text[..at]);
            }
        }
    }
    let words: Vec<String> = text
        .split(' ')
        .map(|word| match word.contains('/') {
            true => "<path>".to_string(),
            false => word.to_string(),
        })
        .collect();
    let mut out = String::new();
    let mut quoted = None;
    for c in words.join(" ").chars() {
        match quoted {
            Some(end) if c == end => quoted = None,
            Some(_) => {}
            None if c == '"' || (c == '\'' && !out.ends_with(|before: char| before.is_alphanumeric())) => {
                quoted = Some(c);
                out.push('_');
            }
            None if c.is_ascii_digit() => {
                if !out.ends_with('#') {
                    out.push('#');
                }
            }
            None => out.push(c),
        }
    }
    out.chars().take(160).collect()
}

/// Whether the attempt running now is one of the replays, whose failures are not kept.
pub fn replaying() -> bool {
    REPLAYING.load(Ordering::Relaxed)
}

pub struct Verdicts {
    passed: Mutex<BTreeSet<u64>>,
    /// The first report of every input that failed, normalised.
    failed: Mutex<BTreeMap<u64, String>>,
    replayed: Mutex<BTreeSet<u64>>,
    sampled: AtomicBool,
}

pub enum Outcome {
    Skipped,
    Passed,
}

pub fn replays() -> usize {
    std::env::var("TEQ_PROP_REPLAYS").ok().and_then(|text| text.parse().ok()).unwrap_or(20)
}

fn skipping() -> bool {
    std::env::var_os("TEQ_PROP_NO_CACHE").is_none()
}

/// The finding of a failure's report: its lines before the history, the saved files and the
/// input's name, which a replay does not write.
pub fn normalised(report: &str) -> String {
    let trailer = |line: &&str| {
        ["the history, ", "the history's files: ", "the program: ", "input "].iter().any(|start| line.starts_with(start))
    };
    report.lines().take_while(|line| !trailer(line)).collect::<Vec<_>>().join("\n")
}

fn report_of(payload: &(dyn std::any::Any + Send)) -> String {
    match (payload.downcast_ref::<String>(), payload.downcast_ref::<&str>()) {
        (Some(text), _) => normalised(text),
        (_, Some(text)) => normalised(text),
        _ => "a panic without a message".to_string(),
    }
}

/// The line that names the input in a saved failure's report and in the replays' records.
pub fn named(key: u64) -> String {
    format!("input {key:016x}")
}

impl Verdicts {
    pub const fn new() -> Verdicts {
        Verdicts {
            passed: Mutex::new(BTreeSet::new()),
            failed: Mutex::new(BTreeMap::new()),
            replayed: Mutex::new(BTreeSet::new()),
            sampled: AtomicBool::new(false),
        }
    }

    /// Runs `attempt` for the input of the key, with `true` where the caches may serve it,
    /// and resumes the attempt's panic when it fails, after the replays where they are due.
    pub fn check(&self, key: u64, attempt: &dyn Fn(bool)) -> Outcome {
        let first_failure = self.failed.lock().unwrap().get(&key).cloned();
        let passed_before = self.passed.lock().unwrap().contains(&key);
        if skipping() && first_failure.is_none() && passed_before {
            return Outcome::Skipped;
        }
        let cached = skipping() && first_failure.is_none();
        match catch_unwind(AssertUnwindSafe(|| attempt(cached))) {
            Ok(()) => {
                self.passed.lock().unwrap().insert(key);
                if let Some(original) = first_failure {
                    self.replay(key, "failed, then passed", &original, attempt);
                }
                Outcome::Passed
            }
            Err(payload) => {
                let report = report_of(payload.as_ref());
                // A failure that names another kind than the one kept counts as a pass; one that
                // names none (the harness's own panic) stands.
                if kind_of(&report).is_some_and(|kind| !kept(kind)) {
                    return Outcome::Passed;
                }
                FAILED.store(true, Ordering::Relaxed);
                let original = self.failed.lock().unwrap().entry(key).or_insert_with(|| report.clone()).clone();
                if passed_before {
                    self.replay(key, "passed, then failed", &original, attempt);
                } else if first_failure.is_some() && original != report {
                    self.replay(key, "failed with another report", &original, attempt);
                } else if first_failure.is_some() && !self.replayed.lock().unwrap().contains(&key) && !self.sampled.swap(true, Ordering::Relaxed) {
                    self.replay(key, "the property's sample, failed twice", &original, attempt);
                }
                resume_unwind(payload)
            }
        }
    }

    fn replay(&self, key: u64, why: &str, original: &str, attempt: &dyn Fn(bool)) {
        let count = replays();
        if count == 0 || !self.replayed.lock().unwrap().insert(key) {
            return;
        }
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        REPLAYING.store(true, Ordering::Relaxed);
        let outcomes: Vec<Option<String>> =
            (0..count).map(|_| catch_unwind(AssertUnwindSafe(|| attempt(false))).err().map(|payload| report_of(payload.as_ref()))).collect();
        REPLAYING.store(false, Ordering::Relaxed);
        std::panic::set_hook(hook);
        let reproduced = outcomes.iter().filter(|outcome| outcome.as_deref() == Some(original)).count();
        let passed = outcomes.iter().filter(|outcome| outcome.is_none()).count();
        let otherwise: BTreeSet<&str> = outcomes.iter().flatten().map(String::as_str).filter(|report| *report != original).collect();
        let line = format!(
            "{} ({why}): replayed {count} times with no cache, the harness held still: {reproduced} reproduced the original failure, {} failed with another report ({} distinct), {passed} passed",
            named(key),
            count - reproduced - passed,
            otherwise.len(),
        );
        eprintln!("{line}");
        let root = crate::work_root();
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(root.join("replays.txt")) {
            let _ = writeln!(file, "{line}");
        }
        let mut kept = format!("{line}\n\nthe original failure:\n{original}\n");
        for (n, outcome) in outcomes.iter().enumerate() {
            kept.push_str(&format!("\nreplay {}: {}\n", n + 1, outcome.as_deref().unwrap_or("passed")));
        }
        let _ = std::fs::create_dir_all(root.join("replays"));
        let _ = std::fs::write(root.join(format!("replays/{key:016x}-{}.txt", crate::driver::invocation())), kept);
    }
}

impl Default for Verdicts {
    fn default() -> Verdicts {
        Verdicts::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// The replays' record of one input whose attempts give the outcomes in turn, run so often.
    fn replays_of(outcomes: &[Option<&'static str>], runs: usize) -> String {
        let work = std::env::temp_dir().join(format!("teq-proptests-confirm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&work);
        std::env::set_var("TEQ_PROP_WORK", &work);
        std::env::set_var("TEQ_PROP_REPLAYS", "4");
        let verdicts = Verdicts::new();
        let at = Cell::new(0);
        let attempt = |_: bool| {
            let outcome = outcomes[at.get() % outcomes.len()];
            at.set(at.get() + 1);
            if let Some(report) = outcome {
                panic!("{report}\nthe history's files: somewhere");
            }
        };
        for _ in 0..runs {
            let _ = catch_unwind(AssertUnwindSafe(|| verdicts.check(7, &attempt)));
        }
        let lines = std::fs::read_to_string(work.join("replays.txt")).unwrap_or_default();
        let _ = std::fs::remove_dir_all(&work);
        lines
    }

    #[test]
    fn names_a_kind_without_what_varies() {
        assert_eq!(normalize("error: value + is not a member of 3"), "error: value _ is not a member of _");
        assert_eq!(normalize("/tmp/a/cases.scala:12:4: error: operator >> cannot be applied to Short and 'z'"), "<path> error: operator _ cannot be applied to _");
        assert_eq!(normalize("`errors` of the session's answer is not the fresh build's:"), "`errors` of the session's answer is not the fresh build's:");
        assert_eq!(normalize("the session counts 12 modules, the fresh build 13"), "the session counts # modules, the fresh build #");
        assert_eq!(kind_of("2 of the program's lines differ\nthe failure's kind: v: interpreter | jvm\nthe program: x"), Some("v: interpreter | jvm"));
    }

    #[test]
    fn replays_an_input_whose_verdict_changes() {
        let lines = replays_of(&[Some("missing foo"), None], 3);
        assert!(lines.contains("(failed, then passed)"), "{lines}");
        assert!(lines.contains("2 reproduced the original failure, 0 failed with another report (0 distinct), 2 passed"), "{lines}");
        assert_eq!(lines.lines().count(), 1, "{lines}");
        let lines = replays_of(&[Some("missing foo"), Some("missing bar")], 2);
        assert!(lines.contains("(failed with another report)"), "{lines}");
        assert!(lines.contains("2 reproduced the original failure, 2 failed with another report (1 distinct), 0 passed"), "{lines}");
        let lines = replays_of(&[Some("missing foo")], 3);
        assert!(lines.contains("(the property's sample, failed twice): replayed 4 times with no cache, the harness held still: 4 reproduced"), "{lines}");
    }
}
