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
    let with_node = std::env::var_os("TEQ_PROP_NO_NODE").is_none();
    let described = order::describe(program, &arrangement);
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
    // No cache serves this property; a failing arrangement is replayed all the same.
    let attempt = |_: bool| {
        let canonical = order::answer(&root.join("sorted"), &order::canonical_layout(program, &arrangement), &program.others, with_node);
        let arranged = order::answer(&root.join("arranged"), &order::layout_of(program, &arrangement), &program.others, with_node);
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
