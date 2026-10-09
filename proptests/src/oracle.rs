//! What a session's build is held against, stated per outcome.
//!
//! - The build succeeded: the answer's result and the output directory's bytes are a fresh
//!   build's of the same sources.
//! - The build failed: the answer's result is a fresh build's, and the output directory holds
//!   what it held after the last build that succeeded.
//!
//! An answer's result is `ok`, `errors`, `warnings`, `diagnostics` and, of a successful build,
//! `modules`. The rest of an answer says how the build was made and is not compared: `ms`,
//! `changed`, `incremental`, `fallback`, `retyped`.

use crate::json::Json;
use crate::session::Kind;
use crate::tree::{self, Tree};

pub const RESULT: [&str; 4] = ["ok", "errors", "warnings", "diagnostics"];

pub fn differences(kind: Kind, answer: &Json, fresh: &Json, out: &Tree, fresh_out: &Tree, last_good: &Tree) -> Vec<String> {
    let mut found = Vec::new();
    for field in RESULT {
        if answer.get(field) != fresh.get(field) {
            found.push(format!(
                "`{field}` of the session's answer is not the fresh build's:\n    session: {}\n    fresh:   {}",
                answer.get(field),
                fresh.get(field)
            ));
        }
    }
    if kind == Kind::Split {
        if answer.get("ok").is_true() {
            if answer.get("modules") != fresh.get("modules") {
                found.push(format!("the session counts {} modules, the fresh build {}", answer.get("modules"), fresh.get("modules")));
            }
            found.extend(tree::differences(out, fresh_out, "session", "fresh build"));
        } else {
            let kept = tree::differences(out, last_good, "session after the failed build", "session after its last good build");
            found.extend(kept.into_iter().map(|difference| format!("a failed build changed the output: {difference}")));
        }
    }
    found
}
