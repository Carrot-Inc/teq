//! Property three: the order of the inputs does not matter. A program of a corpus is given to
//! teq as a subset of its files in a drawn order, split over one to three directories, and
//! what teq answers is held against the answer for the same files in their sorted order:
//!
//! - `teq compiler check` ends alike and reports the same diagnostics, complete with their continuation
//!   and caret lines, as a set: their order follows the inputs';
//! - `teq compiler build --split` ends alike, reports the same diagnostics (a failed build's errors, a
//!   successful build's warnings), writes modules of the same names, and the modules print
//!   the same under node.
//!
//! The bytes of the modules are not compared: definitions are laid out by file and position
//! (docs/TARGETS.md, "Module splitting"), and the order of the files is the inputs'. The
//! corpora are the directories of tests/split and of tests/cases with two files and more, and
//! those under `TEQ_PROP_CORPUS`; a program with a `// jars:` or a `// teq:` line is left out.

use crate::session::run;
use hegel::generators as gs;
use hegel::TestCase;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub struct Program {
    pub name: String,
    pub files: Vec<(String, String)>,
    /// The program's other files (a resource a macro reads), which every directory of an
    /// arrangement gets.
    pub others: Vec<(String, Vec<u8>)>,
}

fn dirs_of(root: &Path, min_files: usize, out: &mut Vec<Program>) {
    let Ok(entries) = std::fs::read_dir(root) else { return };
    let mut dirs: Vec<PathBuf> = entries.map(|entry| entry.unwrap().path()).filter(|path| path.is_dir()).collect();
    dirs.sort();
    for dir in dirs {
        if dir.ends_with("split/retype") && !crate::known::asked("macro-val-order") {
            continue;
        }
        let mut entries: Vec<String> = std::fs::read_dir(&dir).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
        entries.sort();
        let names: Vec<String> = entries.iter().filter(|name| name.ends_with(".scala")).cloned().collect();
        let others: Vec<(String, Vec<u8>)> = entries
            .iter()
            .filter(|name| !name.ends_with(".scala") && dir.join(name).is_file())
            .map(|name| (name.clone(), std::fs::read(dir.join(name)).unwrap()))
            .collect();
        if names.len() < min_files {
            continue;
        }
        let files: Vec<(String, String)> = names.iter().map(|name| (name.clone(), std::fs::read_to_string(dir.join(name)).unwrap())).collect();
        if files.iter().any(|(_, text)| text.lines().any(|line| line.starts_with("// jars:") || line.starts_with("// teq:"))) {
            continue;
        }
        let name = format!("{}/{}", root.file_name().unwrap().to_string_lossy(), dir.file_name().unwrap().to_string_lossy());
        out.push(Program { name, files, others });
    }
}

/// The programs of the corpora.
pub fn corpus() -> Vec<Program> {
    let mut programs = Vec::new();
    let repository = crate::repository();
    if let Some(extra) = std::env::var_os("TEQ_PROP_CORPUS") {
        dirs_of(Path::new(&extra), 1, &mut programs);
    }
    if std::env::var_os("TEQ_PROP_CORPUS_ONLY").is_none() {
        dirs_of(&repository.join("tests/split"), 2, &mut programs);
        dirs_of(&repository.join("tests/cases"), 2, &mut programs);
    }
    programs
}

/// One arrangement of a program's files: which are taken, in what order, and how they are
/// split over directories (`groups[i]` is the directory of `order[i]`, directories in the
/// order of their first file).
#[derive(Debug)]
pub struct Arrangement {
    pub program: usize,
    pub order: Vec<usize>,
    pub groups: Vec<usize>,
}

pub fn arrangement(tc: &TestCase, programs: &[Program]) -> Arrangement {
    let program = tc.draw(gs::integers::<usize>().min_value(0).max_value(programs.len() - 1));
    let count = programs[program].files.len();
    let mut taken: Vec<usize> = (0..count).filter(|_| tc.draw(gs::weighted_booleans(0.85))).collect();
    if taken.is_empty() {
        taken.push(tc.draw(gs::integers::<usize>().min_value(0).max_value(count - 1)));
    }
    let order: Vec<usize> = tc.draw(gs::permutations(taken));
    let directories = tc.draw(gs::integers::<usize>().min_value(1).max_value(3));
    let groups = order.iter().map(|_| tc.draw(gs::integers::<usize>().min_value(0).max_value(directories - 1))).collect();
    Arrangement { program, order, groups }
}

pub struct Answer {
    pub check_ok: bool,
    pub check: BTreeSet<String>,
    pub build_ok: bool,
    pub build: BTreeSet<String>,
    pub modules: BTreeSet<String>,
    pub printed: String,
}

/// The diagnostics of teq's text, each complete (its position line, the continuation lines
/// such as `inlined from`, the source line and the caret line) in a normal form without the
/// directories of the arrangement: a diagnostic starts at a line with a position and a
/// severity, or at a bare `error:`, and runs over the indented lines that follow.
pub fn diagnostics_of(text: &str, src: &Path) -> BTreeSet<String> {
    let root = format!("{}/", src.display());
    let normal = |line: &str| -> String {
        let mut line = line.to_string();
        while let Some(at) = line.find(&root) {
            line.replace_range(at..at + root.len(), "");
            let rest = &line[at..];
            if let Some(slash) = rest.find('/') {
                if rest.starts_with('d') && rest[1..slash].chars().all(|c| c.is_ascii_digit()) {
                    line.replace_range(at..at + slash + 1, "");
                }
            }
        }
        line
    };
    let starts = |line: &str| -> bool {
        line.starts_with("error:") || line.split_once(": ").is_some_and(|(head, rest)| head.matches(':').count() == 2 && (rest.starts_with("error: ") || rest.starts_with("warning: ")))
    };
    let mut found = BTreeSet::new();
    let mut current: Option<Vec<String>> = None;
    for line in text.lines() {
        let line = normal(line);
        if starts(&line) {
            if let Some(block) = current.take() {
                found.insert(block.join("\n"));
            }
            current = Some(vec![line]);
        } else if line.starts_with(' ') {
            if let Some(block) = current.as_mut() {
                block.push(line);
            }
        } else if let Some(block) = current.take() {
            found.insert(block.join("\n"));
        }
    }
    if let Some(block) = current {
        found.insert(block.join("\n"));
    }
    found
}

/// teq's answers for the files laid out as `layout` says, under `work`.
pub fn answer(work: &Path, layout: &[(usize, String, String)], others: &[(String, Vec<u8>)], with_node: bool) -> Result<Answer, String> {
    let teq = crate::teq();
    let _ = std::fs::remove_dir_all(work);
    let src = work.join("src");
    let mut inputs: Vec<PathBuf> = Vec::new();
    let mut seen = BTreeSet::new();
    for (group, name, text) in layout {
        let dir = src.join(format!("d{group}"));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(name), text).map_err(|e| e.to_string())?;
        if seen.insert(*group) {
            for (name, bytes) in others {
                std::fs::write(dir.join(name), bytes).map_err(|e| e.to_string())?;
            }
            inputs.push(dir);
        }
    }
    let mut check = Command::new(&teq);
    check.args(["compiler", "check"]).args(&inputs).current_dir(work);
    let checked = refused(run(check, b"", Duration::from_secs(60), work)?)?;
    let out = work.join("out");
    let mut build = Command::new(&teq);
    build.args(["compiler", "build"]).args(&inputs).arg("--split").arg(&out).current_dir(work);
    let built = refused(run(build, b"", Duration::from_secs(60), work)?)?;
    let mut answer = Answer {
        check_ok: checked.success,
        check: diagnostics_of(&format!("{}\n{}", checked.stdout, checked.stderr), &src),
        build_ok: built.success,
        build: diagnostics_of(&format!("{}\n{}", built.stdout, built.stderr), &src),
        modules: BTreeSet::new(),
        printed: String::new(),
    };
    if built.success {
        answer.modules = std::fs::read_dir(&out).map_err(|e| e.to_string())?.map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
        if with_node {
            let mut node = Command::new("node");
            node.arg(out.join("main.mjs")).current_dir(work);
            let ran = run(node, b"", Duration::from_secs(30), work)?;
            let stderr: Vec<&str> = ran.stderr.lines().filter(|line| !line.contains(&work.display().to_string())).collect();
            answer.printed = format!("{}\n{}\n{}", ran.status, ran.stdout, stderr.join("\n"));
        }
    }
    Ok(answer)
}

/// A command line teq refused (exit 2: the usage, a bad option) is the harness's error, never an
/// answer: without this, two arrangements refused alike would compare as equal and pass.
fn refused(output: crate::session::Output) -> Result<crate::session::Output, String> {
    if output.status == "exit status: 2" {
        return Err(format!("teq refused the command line ({}): {}", output.status, output.stderr.lines().next().unwrap_or_default()));
    }
    Ok(output)
}

/// What differs between the arrangement's answer and the canonical one.
pub fn compare(canonical: &Answer, arranged: &Answer) -> Vec<String> {
    let mut found = Vec::new();
    let ended = |what: &str, a: bool, b: bool| (a != b).then(|| format!("{what} {} in the sorted order and {} in the arranged one", if a { "succeeds" } else { "fails" }, if b { "succeeds" } else { "fails" }));
    found.extend(ended("check", canonical.check_ok, arranged.check_ok));
    found.extend(ended("build", canonical.build_ok, arranged.build_ok));
    let only = |name: &str, a: &BTreeSet<String>, b: &BTreeSet<String>, what: &str| -> Option<String> {
        let missing: Vec<&String> = a.difference(b).collect();
        (!missing.is_empty()).then(|| format!("{what} of the {name} order only:\n    {}", missing.iter().map(|s| s.replace('\n', "\n    ")).collect::<Vec<_>>().join("\n    ")))
    };
    found.extend(only("sorted", &canonical.check, &arranged.check, "diagnostics of check"));
    found.extend(only("arranged", &arranged.check, &canonical.check, "diagnostics of check"));
    found.extend(only("sorted", &canonical.build, &arranged.build, "diagnostics of build"));
    found.extend(only("arranged", &arranged.build, &canonical.build, "diagnostics of build"));
    found.extend(only("sorted", &canonical.modules, &arranged.modules, "modules"));
    found.extend(only("arranged", &arranged.modules, &canonical.modules, "modules"));
    if canonical.printed != arranged.printed {
        found.push(format!("the programs print differently:\n  sorted: {}\n  arranged: {}", canonical.printed.trim(), arranged.printed.trim()));
    }
    found
}

pub fn layout_of(program: &Program, arrangement: &Arrangement) -> Vec<(usize, String, String)> {
    arrangement.order.iter().zip(&arrangement.groups).map(|(file, group)| (*group, program.files[*file].0.clone(), program.files[*file].1.clone())).collect()
}

pub fn canonical_layout(program: &Program, arrangement: &Arrangement) -> Vec<(usize, String, String)> {
    let mut taken: Vec<usize> = arrangement.order.clone();
    taken.sort();
    taken.iter().map(|file| (0, program.files[*file].0.clone(), program.files[*file].1.clone())).collect()
}

pub fn describe(program: &Program, arrangement: &Arrangement) -> String {
    let mut dirs: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
    for (file, group) in arrangement.order.iter().zip(&arrangement.groups) {
        dirs.entry(*group).or_default().push(&program.files[*file].0);
    }
    let dirs: Vec<String> = dirs.iter().map(|(group, names)| format!("d{group}: {}", names.join(" "))).collect();
    let count = program.files.len();
    format!("{} ({} of its {count} files), given as {}", program.name, arrangement.order.len(), dirs.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_diagnostics_whole_without_the_directories() {
        let src = Path::new("/w/src");
        let text = "/w/src/d1/a.scala:3:17: error: type mismatch: found String, required Int\n  def d1(): Int = \"s\"\n                  ^^^\n/w/src/d0/b.scala:2:5: warning: A pure expression does nothing in statement position\n    1\n    ^\n1 error found\nerror: the program has no entry point\n/w/src/d0/b.scala:4:1: error: not supported yet in a macro: x (called by m)\n  inlined from /w/src/d1/m.scala:9\n    println(m())\n    ^^^\n";
        let found = diagnostics_of(text, src);
        let expected: BTreeSet<String> = [
            "a.scala:3:17: error: type mismatch: found String, required Int\n  def d1(): Int = \"s\"\n                  ^^^",
            "b.scala:2:5: warning: A pure expression does nothing in statement position\n    1\n    ^",
            "error: the program has no entry point",
            "b.scala:4:1: error: not supported yet in a macro: x (called by m)\n  inlined from m.scala:9\n    println(m())\n    ^^^",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        assert_eq!(found, expected);
    }
}
