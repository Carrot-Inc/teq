//! The controls: property one over the fixture of the retype's suites, tests/split/retype, run
//! against a binary that has a known defect and against one that has it repaired.
//!
//! - Bodies alone: e55983cf answers wrongly after a retype of the entry file, 68d62cff does not.
//! - Bodies and declared types: on 68d62cff a body edit takes a signature's error out of a
//!   check session's answer.

use hegel::{HealthCheck, TestCase};
use teq_proptests::fixture::{Fixture, BODIES, BODIES_AND_TYPES};
use teq_proptests::session::Kind;

fn steps() -> i64 {
    std::env::var("TEQ_PROP_STEPS").ok().and_then(|text| text.parse().ok()).unwrap_or(30)
}

#[hegel::test(suppress_health_check = [HealthCheck::TooSlow])]
fn fixture_bodies_split(tc: TestCase) {
    Fixture::read(Kind::Split, "fixture_bodies_split", &BODIES).run(tc, steps());
}

#[hegel::test(suppress_health_check = [HealthCheck::TooSlow])]
fn fixture_bodies_check(tc: TestCase) {
    Fixture::read(Kind::Check, "fixture_bodies_check", &BODIES).run(tc, steps());
}

#[hegel::test(suppress_health_check = [HealthCheck::TooSlow])]
fn fixture_types_split(tc: TestCase) {
    Fixture::read(Kind::Split, "fixture_types_split", &BODIES_AND_TYPES).run(tc, steps());
}

#[hegel::test(suppress_health_check = [HealthCheck::TooSlow])]
fn fixture_types_check(tc: TestCase) {
    Fixture::read(Kind::Check, "fixture_types_check", &BODIES_AND_TYPES).run(tc, steps());
}
