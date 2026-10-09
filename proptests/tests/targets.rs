//! Property two: generated expressions print the same on JavaScript, in the interpreter and on
//! the JVM, in the form the typer can fold and in the one it cannot, and scalac agrees with
//! them (`src/exprs.rs`, `src/targets.rs`).
//!
//! An input is `TEQ_PROP_PROGRAMS` programs (1): each runs on the targets in turn, and scalac
//! is asked about those on which the targets agree in one run for all of them
//! (`targets::scalac_many`). The batch is drawn within the one input, so the input the engine
//! keeps for a failure replays the program that failed; the failure is that program's, reported
//! and kept alone. With `TEQ_PROP_REDUCE` a failure is also reduced to its cases: the cases whose lines
//! differ, together, or the first case that is refused alone, kept as `reduced.scala` where it
//! fails in the same kind; a replay of the shrunk input asks for it, since it
//! costs a run of its own at every failure.

use hegel::generators as gs;
use hegel::{HealthCheck, TestCase};
use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use teq_proptests::confirm::{Outcome, Verdicts};
use teq_proptests::exprs::{self, Case};
use teq_proptests::targets::{self, Ran};

static VERDICTS: Verdicts = Verdicts::new();
static FAILURES: AtomicUsize = AtomicUsize::new(0);
/// The programs that passed in the process, by their text: the shrinker takes the programs
/// of an input out one at a time, and the others need not run again.
static PASSED: Mutex<BTreeSet<u64>> = Mutex::new(BTreeSet::new());

fn cases() -> usize {
    std::env::var("TEQ_PROP_CASES").ok().and_then(|text| text.parse().ok()).unwrap_or(40)
}

fn programs() -> usize {
    std::env::var("TEQ_PROP_PROGRAMS").ok().and_then(|text| text.parse().ok()).unwrap_or(1).max(1)
}

/// Every case is drawn and most are taken: the shrinker takes a case out by one choice, which
/// leaves the choices of the others where they are. Programs are drawn so too.
fn drawn_cases(tc: &TestCase) -> Vec<Case> {
    (0..cases()).map(|_| (tc.draw_silent(gs::weighted_booleans(0.9)), exprs::case(tc))).filter(|(taken, _)| *taken).map(|(_, case)| case).collect()
}

struct Program {
    cases: Vec<Case>,
    text: String,
    labels: Vec<String>,
    hash: u64,
}

impl Program {
    fn of(cases: Vec<Case>) -> Program {
        let (text, labels) = exprs::program(&cases);
        Program { hash: targets::text_hash(&text), cases, text, labels }
    }
}

enum Failure {
    Refused(String),
    Differs(Vec<(String, String)>),
}

impl Failure {
    fn kind(&self) -> String {
        match self {
            Failure::Refused(refusal) => targets::refusal_kind(refusal),
            Failure::Differs(found) => {
                let kinds: BTreeSet<String> = found.iter().map(|(label, what)| targets::difference_kind(label, what)).collect();
                kinds.into_iter().collect::<Vec<_>>().join("; ")
            }
        }
    }

    fn report(&self, program: &Program) -> String {
        match self {
            Failure::Refused(refusal) => format!("of {} cases:\n{refusal}", program.cases.len()),
            Failure::Differs(found) => {
                let mut report = vec![format!("{} of the program's lines differ, of {} cases", found.len(), program.cases.len())];
                for (id, what) in found.iter().take(6) {
                    report.push(format!("line {id}: {what}"));
                    if let Some(case) = case_of(id).and_then(|n| program.cases.get(n)) {
                        report.push(exprs::shown(case));
                    }
                }
                report.join("\n")
            }
        }
    }
}

/// The case a line's label names (`12.f.v`).
fn case_of(label: &str) -> Option<usize> {
    label.split('.').next().and_then(|n| n.parse().ok())
}

/// How the program fares on the targets and under scalac, asked whatever the sample says.
fn outcome(program: &Program, work: &Path, root: &Path) -> Result<(), Failure> {
    let mut ran = targets::targets(&program.text, &program.labels, work, root).map_err(Failure::Refused)?;
    if ran.scalac == "not asked" {
        let started = std::time::Instant::now();
        let answer = targets::scalac(&program.text, root).map_err(Failure::Refused)?;
        targets::answered(&mut ran, answer, started.elapsed().as_millis());
    }
    let found = targets::differences(&ran, &program.labels);
    let _ = std::fs::remove_dir_all(work);
    if found.is_empty() {
        Ok(())
    } else {
        Err(Failure::Differs(found))
    }
}

fn reducing() -> bool {
    std::env::var_os("TEQ_PROP_REDUCE").is_some()
}

/// The failing program's cases that fail by themselves, with a failure of the same kind: the
/// cases whose lines differ, together, or the first case that is refused alone.
fn reduced(program: &Program, failure: &Failure, work: &Path, root: &Path) -> Option<(Program, Failure)> {
    let kind = failure.kind();
    let fails = |cases: Vec<Case>| {
        let smaller = Program::of(cases);
        match outcome(&smaller, work, root) {
            Err(failure) if failure.kind() == kind => Some((smaller, failure)),
            _ => None,
        }
    };
    match failure {
        Failure::Differs(found) => {
            let mut cases: Vec<usize> = found.iter().filter_map(|(id, _)| case_of(id)).collect();
            cases.sort_unstable();
            cases.dedup();
            (!cases.is_empty() && cases.len() < program.cases.len()).then(|| fails(cases.iter().map(|&n| program.cases[n].clone()).collect())).flatten()
        }
        Failure::Refused(_) => (program.cases.len() > 1).then(|| program.cases.iter().find_map(|case| fails(vec![case.clone()]))).flatten(),
    }
}

fn note(root: &Path, outcome: &str, program: &Program, ran: Option<&Ran>) {
    let millis = ran.map(|ran| format!("{:?}", ran.millis)).unwrap_or_default();
    let scalac = ran.map_or("not asked", |ran| ran.scalac);
    let line = format!("{{\"outcome\":{outcome:?},\"cases\":{},\"scalac\":{scalac:?},\"millis\":{millis:?}}}\n", program.cases.len());
    let _ = std::fs::create_dir_all(root.join("stats"));
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(root.join("stats/targets_agree.jsonl")) {
        let _ = file.write_all(line.as_bytes());
    }
}

#[hegel::test(suppress_health_check = [HealthCheck::TooSlow, HealthCheck::TestCasesTooLarge, HealthCheck::LargeInitialTestCase])]
fn targets_agree(tc: TestCase) {
    if teq_proptests::past_deadline() {
        return;
    }
    let count = programs();
    let drawn: Vec<Vec<Case>> = match count {
        1 => vec![drawn_cases(&tc)],
        _ => (0..count).map(|_| (tc.draw_silent(gs::weighted_booleans(0.9)), drawn_cases(&tc))).filter(|(taken, _)| *taken).map(|(_, cases)| cases).collect(),
    };
    let programs: Vec<Program> = drawn
        .into_iter()
        .filter(|cases| !cases.is_empty())
        .map(Program::of)
        .collect();
    if programs.is_empty() {
        return;
    }
    let key = match &programs[..] {
        [one] => one.hash,
        many => targets::text_hash(&many.iter().map(|program| format!("{:016x}", program.hash)).collect::<String>()),
    };
    let root = teq_proptests::work_root();
    let run = root.join(format!("targets_agree/run-{}", teq_proptests::driver::invocation()));
    let fail = |program: &Program, failure: Failure| -> ! {
        let kind = failure.kind();
        let report = format!("{}\n{}{kind}", failure.report(program), teq_proptests::confirm::KIND);
        if teq_proptests::confirm::replaying() || !teq_proptests::confirm::kept(&kind) {
            panic!("{report}");
        }
        let n = FAILURES.fetch_add(1, Ordering::Relaxed);
        let kept = root.join(format!("targets_agree/failures/{}-{n:04}", teq_proptests::driver::invocation()));
        let _ = std::fs::create_dir_all(&kept);
        let _ = std::fs::write(kept.join("cases.scala"), &program.text);
        let mut report = format!("{report}\nthe program: {}", kept.join("cases.scala").display());
        if let Some((smaller, failure)) = reducing().then(|| reduced(program, &failure, &run.join("reduced"), &root)).flatten() {
            let _ = std::fs::write(kept.join("reduced.scala"), &smaller.text);
            let _ = std::fs::write(kept.join("reduced.txt"), format!("{}\n{}{}", failure.report(&smaller), teq_proptests::confirm::KIND, failure.kind()));
            report.push_str(&format!("\nthe program reduced to its failing cases: {}", kept.join("reduced.scala").display()));
        }
        let report = format!("{report}\n{}", teq_proptests::confirm::named(key));
        let _ = std::fs::write(kept.join("report.txt"), &report);
        let _ = std::fs::remove_dir_all(&run);
        panic!("{report}");
    };
    let judge = |program: &Program, ran: &Ran| {
        let found = targets::differences(ran, &program.labels);
        if found.is_empty() {
            PASSED.lock().unwrap().insert(program.hash);
            return note(&root, "passed", program, Some(ran));
        }
        note(&root, "failed", program, Some(ran));
        fail(program, Failure::Differs(found));
    };
    let attempt = |cached: bool| {
        let mut agreed: Vec<(&Program, Ran)> = Vec::new();
        for (n, program) in programs.iter().enumerate() {
            if cached && PASSED.lock().unwrap().contains(&program.hash) {
                note(&root, "passed before", program, None);
                continue;
            }
            let ran = match targets::targets(&program.text, &program.labels, &run.join(n.to_string()), &root) {
                Ok(ran) => ran,
                Err(refusal) => {
                    note(&root, "refused", program, None);
                    fail(program, Failure::Refused(refusal));
                }
            };
            if ran.scalac != "not asked" || !targets::asked_scalac(program.hash) {
                judge(program, &ran);
            } else {
                agreed.push((program, ran));
            }
        }
        let texts: Vec<&str> = agreed.iter().map(|(program, _)| program.text.as_str()).collect();
        let started = std::time::Instant::now();
        let answers = targets::scalac_many(&texts, &root);
        let millis = started.elapsed().as_millis();
        for ((program, mut ran), answer) in agreed.into_iter().zip(answers) {
            match answer {
                Ok(answer) => targets::answered(&mut ran, answer, millis),
                Err(refusal) => {
                    note(&root, "refused", program, None);
                    fail(program, Failure::Refused(refusal));
                }
            }
            judge(program, &ran);
        }
        let _ = std::fs::remove_dir_all(&run);
    };
    if let Outcome::Skipped = VERDICTS.check(key, &attempt) {
        for program in &programs {
            note(&root, "passed before", program, None);
        }
    }
}

/// Not a property: scalac asked about each of the drawn programs alone and about all of them
/// in one batch answers every one, the same both ways, and the stats say what each way costs
/// (`<work>/stats/scalac.jsonl`). By hand, with `scala-cli` on the path:
/// `TEQ_PROP_PROGRAMS=10 HEGEL_TEST_CASES=3 cargo test --test targets -- --ignored`.
#[hegel::test(suppress_health_check = [HealthCheck::TooSlow, HealthCheck::TestCasesTooLarge, HealthCheck::LargeInitialTestCase])]
#[ignore]
fn scalac_batched_answers_as_alone(tc: TestCase) {
    let mut programs: Vec<String> = (0..programs().max(2)).map(|_| drawn_cases(&tc)).filter(|cases| !cases.is_empty()).map(|cases| exprs::program(&cases).0).collect();
    programs.sort();
    programs.dedup();
    // Two programs at least, or the batch has nothing to answer.
    tc.assume(programs.len() > 1);
    let texts: Vec<&str> = programs.iter().map(String::as_str).collect();
    let root = teq_proptests::work_root();
    let fresh = |name: &str| {
        let dir = root.join(format!("scalac-{name}-{}", teq_proptests::driver::invocation()));
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("TEQ_PROP_SCALAC_CACHE", &dir);
    };
    fresh("alone");
    let alone: Vec<_> = texts.iter().map(|text| targets::scalac(text, &root)).collect();
    fresh("batched");
    let batched = targets::scalac_many(&texts, &root);
    std::env::remove_var("TEQ_PROP_SCALAC_CACHE");
    // Every program is answered, alone and in the batch: a refusal, a throw or a failure to run
    // scalac fails the check, which holds only where both answers stand.
    for (n, (alone, batched)) in alone.iter().zip(&batched).enumerate() {
        let (a, _) = alone.as_ref().unwrap_or_else(|problem| panic!("program {n}, asked alone: {problem}"));
        let (b, how) = batched.as_ref().unwrap_or_else(|problem| panic!("program {n}, in a batch: {problem}"));
        assert_eq!(*how, "batched", "program {n} was not answered by the batch");
        assert!(a == b, "program {n}: asked alone and in a batch, scalac answers differently");
    }
}
