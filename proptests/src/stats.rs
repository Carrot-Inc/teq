//! What a history did, written as one line per history for the coverage figures of a campaign.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

pub struct Stats {
    pub started: Instant,
    pub waited_at_start: u64,
    pub rules: BTreeMap<&'static str, u32>,
    pub builds: u32,
    /// Builds that retyped a file, against the full ones and those that changed nothing.
    pub incremental: u32,
    pub full: u32,
    pub unchanged: u32,
    pub fallbacks: BTreeMap<String, u32>,
    pub failed_builds: u32,
    /// Failed builds with no injected fault standing: the generator's errors, counted apart.
    pub generator_errors: u32,
    pub several_files: u32,
    pub macro_retypes: u32,
    pub retyped_files: u32,
    pub errors_standing: u32,
    pub longest_error_run: u32,
    pub fresh_cached: u32,
    pub planned: u32,
    pub outcome: &'static str,
}

impl Stats {
    pub fn new() -> Stats {
        Stats {
            started: Instant::now(),
            waited_at_start: crate::session::WAITED_MICROS.load(std::sync::atomic::Ordering::Relaxed),
            rules: BTreeMap::new(),
            builds: 0,
            incremental: 0,
            full: 0,
            unchanged: 0,
            fallbacks: BTreeMap::new(),
            failed_builds: 0,
            generator_errors: 0,
            several_files: 0,
            macro_retypes: 0,
            retyped_files: 0,
            errors_standing: 0,
            longest_error_run: 0,
            fresh_cached: 0,
            planned: 0,
            outcome: "passed",
        }
    }

    /// Appends the history's line to `<work>/stats/<test>.jsonl`.
    pub fn write(&self, work_root: &Path, test: &str) {
        let waited = crate::session::WAITED_MICROS.load(std::sync::atomic::Ordering::Relaxed) - self.waited_at_start;
        let map = |entries: Vec<(String, u32)>| {
            let fields: Vec<String> = entries.iter().map(|(key, count)| format!("{key:?}:{count}")).collect();
            format!("{{{}}}", fields.join(","))
        };
        let line = format!(
            "{{\"outcome\":{:?},\"micros\":{},\"waited_micros\":{},\"builds\":{},\"incremental\":{},\"full\":{},\"unchanged\":{},\"failed_builds\":{},\"generator_errors\":{},\"several_files\":{},\"macro_retypes\":{},\"retyped_files\":{},\"longest_error_run\":{},\"fresh_cached\":{},\"planned\":{},\"rules\":{},\"fallbacks\":{}}}\n",
            self.outcome,
            self.started.elapsed().as_micros(),
            waited,
            self.builds,
            self.incremental,
            self.full,
            self.unchanged,
            self.failed_builds,
            self.generator_errors,
            self.several_files,
            self.macro_retypes,
            self.retyped_files,
            self.longest_error_run,
            self.fresh_cached,
            self.planned,
            map(self.rules.iter().map(|(rule, count)| (rule.to_string(), *count)).collect()),
            map(self.fallbacks.iter().map(|(reason, count)| (reason.clone(), *count)).collect()),
        );
        let dir = work_root.join("stats");
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join(format!("{test}.jsonl"))) {
            let _ = file.write_all(line.as_bytes());
        }
    }
}

impl Default for Stats {
    fn default() -> Stats {
        Stats::new()
    }
}
