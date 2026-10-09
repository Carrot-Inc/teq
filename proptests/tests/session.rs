//! Property one: a resident session answers and writes what a fresh build of the same sources
//! does, whatever edits came before. `src/oracle.rs` states the invariant per outcome.

use hegel::{HealthCheck, TestCase};
use teq_proptests::edits::{Edits, PILOT, RULES};
use teq_proptests::session::Kind;

fn steps() -> i64 {
    std::env::var("TEQ_PROP_STEPS").ok().and_then(|text| text.parse().ok()).unwrap_or(30)
}

#[hegel::test(suppress_health_check = [HealthCheck::TooSlow])]
fn pilot_split(tc: TestCase) {
    if teq_proptests::past_deadline() {
        return;
    }
    Edits::small(Kind::Split, "pilot_split", &PILOT).run(tc, steps());
}

#[hegel::test(suppress_health_check = [HealthCheck::TooSlow])]
fn pilot_check(tc: TestCase) {
    if teq_proptests::past_deadline() {
        return;
    }
    Edits::small(Kind::Check, "pilot_check", &PILOT).run(tc, steps());
}

#[hegel::test(suppress_health_check = [HealthCheck::TooSlow])]
fn split_session_agrees_with_fresh(tc: TestCase) {
    if teq_proptests::past_deadline() {
        return;
    }
    Edits::drawn(&tc, Kind::Split, "split_session_agrees_with_fresh", &RULES).run(tc, steps());
}

#[hegel::test(suppress_health_check = [HealthCheck::TooSlow])]
fn check_session_agrees_with_fresh(tc: TestCase) {
    if teq_proptests::past_deadline() {
        return;
    }
    Edits::drawn(&tc, Kind::Check, "check_session_agrees_with_fresh", &RULES).run(tc, steps());
}
