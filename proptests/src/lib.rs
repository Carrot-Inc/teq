//! What the properties share: the `teq` binary they drive, the directory they work in, and the
//! machines over a resident session.

pub mod confirm;
pub mod driver;
pub mod edits;
pub mod exprs;
pub mod fixture;
pub mod json;
pub mod known;
pub mod model;
pub mod oracle;
pub mod order;
pub mod regexes;
pub mod session;
pub mod stats;
pub mod targets;
pub mod tree;

use std::path::PathBuf;

/// The binary named by `TEQ`, or the release build of the repository the crate stands in.
pub fn teq() -> PathBuf {
    let path = match std::env::var_os("TEQ") {
        Some(path) => PathBuf::from(path),
        None => repository().join("target/release/teq"),
    };
    path.canonicalize()
        .unwrap_or_else(|e| panic!("no teq binary at {}: {e}", path.display()))
}

pub fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Where the properties write: `TEQ_PROP_WORK`, or `out/proptests` of the repository. What an
/// invocation writes while it runs stands under a name of its own (`driver::invocation`), so
/// that two invocations over one directory leave each other alone.
pub fn work_root() -> PathBuf {
    let root = match std::env::var_os("TEQ_PROP_WORK") {
        Some(path) => PathBuf::from(path),
        None => repository().join("out/proptests"),
    };
    std::fs::create_dir_all(&root)
        .unwrap_or_else(|e| panic!("cannot create {}: {e}", root.display()));
    root.canonicalize().unwrap()
}

/// Whether the run's time is up: `TEQ_PROP_UNTIL`, in seconds since the epoch, is past. A
/// property then returns before it draws anything, which the engine takes as the end of what
/// there is to generate, so a campaign's piece ends by itself. Never set for a shrink run: an
/// input replayed from the database would pass and be forgotten. Once an input failed in the
/// process the time is never up, so that the engine's replays of the failure, which confirm it
/// and keep it in the database, run in full.
pub fn past_deadline() -> bool {
    if crate::confirm::failed() {
        return false;
    }
    let until = std::env::var("TEQ_PROP_UNTIL").ok().and_then(|text| text.parse::<u64>().ok());
    until.is_some_and(|until| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()) >= until)
}
