//! Property three: the order of the inputs does not matter (`src/order.rs`).

use hegel::{HealthCheck, TestCase};
use std::hash::{Hash, Hasher};
use std::io::Write;
use teq_proptests::confirm::Verdicts;
use teq_proptests::order;

static VERDICTS: Verdicts = Verdicts::new();

#[hegel::test(suppress_health_check = [HealthCheck::TooSlow])]
fn order_of_inputs_does_not_matter(tc: TestCase) {
    if teq_proptests::past_deadline() {
        return;
    }
    let programs = order::corpus();
    assert!(!programs.is_empty(), "no corpus");
    let arrangement = order::arrangement(&tc, &programs);
    let program = &programs[arrangement.program];
    let root = teq_proptests::work_root().join(format!("order/run-{}", teq_proptests::driver::invocation()));
    let build = root.join("build");
    let with_node = std::env::var_os("TEQ_PROP_NO_NODE").is_none();
    let described = order::describe(program, &arrangement, &build);
    let note = |outcome: &str| {
        let line = format!("{{\"outcome\":{outcome:?},\"program\":{:?},\"files\":{},\"of\":{}}}\n", program.name, arrangement.order.len(), program.files.len());
        let stats = teq_proptests::work_root().join("stats");
        let _ = std::fs::create_dir_all(&stats);
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(stats.join("order.jsonl")) {
            let _ = file.write_all(line.as_bytes());
        }
    };
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (&program.name, &arrangement.order, &arrangement.groups).hash(&mut hasher);
    let layout = order::layout_of(program, &arrangement);
    let answer = |inputs: Vec<String>| order::lay_out(&build, &layout, &program.others).and_then(|()| order::answer(&build, &inputs, with_node));
    // No cache serves this property; a failing arrangement is replayed all the same.
    let attempt = |_: bool| {
        let canonical = answer(order::files_by_name(&layout));
        let arranged = answer(order::directories(&layout));
        let (canonical, arranged) = match (canonical, arranged) {
            (Ok(canonical), Ok(arranged)) => (canonical, arranged),
            (Err(problem), _) | (_, Err(problem)) => {
                note("harness");
                panic!("{described}: {problem}");
            }
        };
        let found = order::compare(&canonical, &arranged);
        if found.is_empty() {
            let _ = std::fs::remove_dir_all(&root);
            return note("passed");
        }
        note("failed");
        panic!("{described}\n{}", found.join("\n"));
    };
    VERDICTS.check(hasher.finish(), &attempt);
}

/// The control of what the property's two builds differ in besides the order: every program of
/// the corpus, its files spread over the directories as an arrangement may spread them (the
/// n-th by name in `d<n mod 3>`), built at the same paths given its directories in ascending
/// order and given its files by name, must answer, write and print the same bytes.
#[test]
#[ignore]
fn files_named_answer_as_directories_listed() {
    let programs = order::corpus();
    let work = teq_proptests::work_root().join(format!("order/control-{}", teq_proptests::driver::invocation()));
    let with_node = std::env::var_os("TEQ_PROP_NO_NODE").is_none();
    let (mut built, mut differ, mut harness) = (0, Vec::new(), Vec::new());
    for program in &programs {
        let layout: Vec<(usize, String, String)> = program.files.iter().enumerate().map(|(n, (name, text))| (n % 3, name.clone(), text.clone())).collect();
        let answer = |inputs: Vec<String>| order::lay_out(&work, &layout, &program.others).and_then(|()| order::answer(&work, &inputs, with_node));
        match (answer(order::directories(&layout)), answer(order::files_by_name(&layout))) {
            (Ok(listed), Ok(named)) => {
                built += usize::from(listed.build_ok);
                let found = order::whole_differences(&listed, &named, "directories", "files");
                if !found.is_empty() {
                    differ.push(format!("{}:\n  {}", program.name, found.join("\n  ")));
                }
            }
            (Err(problem), _) | (_, Err(problem)) => harness.push(format!("{}: {problem}", program.name)),
        }
    }
    let _ = std::fs::remove_dir_all(&work);
    println!("{} programs, {built} of which build; {} answer otherwise by files, {} the harness's errors", programs.len(), differ.len(), harness.len());
    assert!(differ.is_empty() && harness.is_empty(), "{}\n{}", differ.join("\n"), harness.join("\n"));
}
