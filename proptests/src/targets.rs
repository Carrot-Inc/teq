//! Runs a program on the three targets and under scalac, and holds the lines they print
//! against each other.
//!
//! - On every target a case's folded form prints what its other form prints.
//! - The `v` lines are the same on JavaScript, in the interpreter and on the JVM; the `t` lines
//!   (how a `Double` or a `Float` prints) in the interpreter and on the JVM.
//! - scalac 3.8.4 is asked about every program on which the targets disagree, and about one
//!   program in `TEQ_PROP_SCALAC_EVERY` (10) chosen by the program's text whether or not they
//!   agree; its lines are the JVM's and the interpreter's, and its `v` lines JavaScript's. Its
//!   answers are kept by the program's text under `<work>/scalac`. `TEQ_PROP_SCALAC=always`
//!   asks it about every program, `never` about none.
//!
//! What a target prints is compared as it comes, line by line: the labels have to be the
//! program's in the program's order, and a label printed twice is refused.
//!
//! The JVM is started only when JavaScript and the interpreter agree, so that a disagreement
//! between those two is shrunk at their cost; scalac is asked about every disagreement.

use crate::session::run;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// The lines a target printed, in its order: a label and what followed it.
pub type Lines = Vec<(String, String)>;

pub struct Ran {
    pub lines: BTreeMap<&'static str, Lines>,
    pub millis: BTreeMap<&'static str, u128>,
    pub scalac: &'static str,
}

pub fn text_hash(text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn parse(target: &str, printed: &str) -> Result<Lines, String> {
    let mut lines = Lines::new();
    for line in printed.lines() {
        let (label, text) = line.split_once(':').ok_or_else(|| format!("{target}: a line that is no case's: {line}"))?;
        if lines.iter().any(|(seen, _)| seen == label) {
            return Err(format!("{target}: the label {label} is printed twice"));
        }
        lines.push((label.to_string(), text.to_string()));
    }
    Ok(lines)
}

/// The build of a target's output, which has to succeed before its runtime runs.
fn built(target: &str, command: Command, work: &Path, bound: Duration) -> Result<(), String> {
    let output = run(command, b"", bound, work).map_err(|problem| format!("{target}: {problem}"))?;
    if !output.success {
        let said: String = format!("{}\n{}", output.stderr.trim(), output.stdout.trim()).chars().take(1500).collect();
        return Err(format!("{target}: the build is refused or fails ({}):\n{said}", output.status));
    }
    Ok(())
}

/// The scala-library jar a JVM build links against when its class path names none: the newest 3.x of
/// the coursier caches, found as the compiler's `classpath::find_scala_library` finds it (and as
/// tests/support/jars.sh's `scala_library_jar` does). The three must agree: a change of one is a
/// change of all.
fn scala_library() -> Result<String, String> {
    let mut caches: Vec<PathBuf> = Vec::new();
    if cfg!(windows) {
        let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from);
        caches.extend(var("COURSIER_CACHE").or_else(|| var("LOCALAPPDATA").map(|d| d.join("Coursier").join("cache").join("v1"))));
    } else {
        let home = std::env::var("HOME").unwrap_or_default();
        if let Ok(c) = std::env::var("COURSIER_CACHE") {
            caches.push(PathBuf::from(c));
        }
        caches.push(Path::new(&home).join("Library/Caches/Coursier/v1"));
        caches.push(Path::new(&home).join(".cache/coursier/v1"));
    }
    let mut best: Option<(Vec<u32>, PathBuf)> = None;
    for cache in &caches {
        let Ok(entries) = std::fs::read_dir(cache.join("https/repo1.maven.org/maven2/org/scala-lang/scala-library")) else { continue };
        for entry in entries.flatten() {
            let version = entry.file_name().to_string_lossy().to_string();
            let Some(parts) = version.split('.').map(|p| p.parse().ok()).collect::<Option<Vec<u32>>>() else { continue };
            let jar = entry.path().join(format!("scala-library-{version}.jar"));
            if parts.first() == Some(&3) && jar.is_file() && best.as_ref().map_or(true, |(v, _)| parts > *v) {
                best = Some((parts, jar));
            }
        }
    }
    best.map(|(_, jar)| jar.to_string_lossy().into_owned()).ok_or_else(|| "no scala-library 3.x jar in the coursier cache for the JVM target".to_string())
}

fn lines_of(target: &str, command: Command, work: &Path, bound: Duration) -> Result<Lines, String> {
    let output = run(command, b"", bound, work).map_err(|problem| format!("{target}: {problem}"))?;
    if !output.success {
        let said: String = format!("{}\n{}", output.stderr.trim(), output.stdout.trim()).chars().take(1500).collect();
        return Err(format!("{target}: the program is refused or ends with an error ({}):\n{said}", output.status));
    }
    parse(target, &output.stdout)
}

/// Whether scalac is asked about a program on which the targets agree.
pub fn asked_scalac(hash: u64) -> bool {
    let every = std::env::var("TEQ_PROP_SCALAC_EVERY").ok().and_then(|text| text.parse().ok()).unwrap_or(10);
    match std::env::var("TEQ_PROP_SCALAC").as_deref() {
        Ok("always") => true,
        Ok("never") => false,
        _ => hash % every == 0,
    }
}

/// Where scalac's answers are kept: `TEQ_PROP_SCALAC_CACHE`, or `<work>/scalac`.
fn answers(root: &Path) -> PathBuf {
    std::env::var_os("TEQ_PROP_SCALAC_CACHE").map_or_else(|| root.join("scalac"), PathBuf::from)
}

fn answer_dir(text: &str, root: &Path) -> PathBuf {
    answers(root).join(format!("{:016x}", text_hash(text)))
}

fn kept(text: &str, root: &Path) -> Option<Result<Lines, String>> {
    std::fs::read_to_string(answer_dir(text, root).join("lines.txt")).ok().map(|printed| parse("scalac", &printed))
}

/// Keeps an answer beside its final name and renames it into place, so that a reader never
/// sees half of it.
fn publish(text: &str, root: &Path, printed: &str) -> Result<(), String> {
    let dir = answer_dir(text, root);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let beside = dir.join(format!("lines.txt.{}", crate::driver::invocation()));
    std::fs::write(&beside, printed).map_err(|e| e.to_string())?;
    std::fs::rename(&beside, dir.join("lines.txt")).map_err(|e| e.to_string())
}

/// `scala-cli` on the JVM of `TEQ_PROP_SCALAC_JVM`, the system's by default (`JAVA_HOME` or
/// the path), which the JVM target runs on too: without it `scala-cli` takes a JDK 17 of its
/// own where `JAVA_HOME` is unset, whose `Float` and `Double` text is not JDK 19's and later's
/// (`2147483648f` prints `2.14748365E9` there, `2.1474836E9` from JDK 19 on).
fn scala_cli(src: &Path, bound: Duration, work: &Path) -> Result<crate::session::Output, String> {
    let jvm = std::env::var("TEQ_PROP_SCALAC_JVM").unwrap_or_else(|_| "system".to_string());
    let mut command = Command::new("scala-cli");
    command.args(["--power", "run", "--server=false", "-S", "3.8.4", "--offline", "-q", "--jvm", &jvm]).arg(src);
    run(command, b"", bound, work).map_err(|problem| format!("scalac: {problem}"))
}

/// One line per `scala-cli` invocation in `<work>/stats/scalac.jsonl`: how many programs it
/// answered for and how long it took.
fn note_invocation(root: &Path, programs: usize, answered: usize, millis: u128) {
    let line = format!("{{\"programs\":{programs},\"answered\":{answered},\"millis\":{millis}}}\n");
    let _ = std::fs::create_dir_all(root.join("stats"));
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(root.join("stats/scalac.jsonl")) {
        let _ = file.write_all(line.as_bytes());
    }
}

/// scalac's lines for the program, from the answers kept or from `scala-cli`. Every
/// invocation compiles in a directory of its own and publishes its answer by a rename, so
/// that invocations sharing the answers neither remove each other's sources nor read a
/// half-written answer.
pub fn scalac(text: &str, root: &Path) -> Result<(Lines, &'static str), String> {
    if let Some(lines) = kept(text, root) {
        return Ok((lines?, "kept"));
    }
    let build = answer_dir(text, root).join(format!("build-{}", crate::driver::invocation()));
    let _ = std::fs::remove_dir_all(&build);
    std::fs::create_dir_all(build.join("src")).map_err(|e| e.to_string())?;
    std::fs::write(build.join("src/cases.scala"), text).map_err(|e| e.to_string())?;
    let started = Instant::now();
    let output = scala_cli(&build.join("src"), Duration::from_secs(120), &build);
    let _ = std::fs::remove_dir_all(&build);
    let output = output?;
    note_invocation(root, 1, usize::from(output.success), started.elapsed().as_millis());
    if !output.success {
        let said: String = output.stderr.chars().take(3000).collect();
        return Err(format!("scalac refuses the program, the generator's defect ({}):\n{said}", output.status));
    }
    publish(text, root, &output.stdout)?;
    Ok((parse("scalac", &output.stdout)?, "asked"))
}

const MAIN: &str = "@main def run(): Unit =";

/// A line the batch's main prints: it carries the run's nonce, which no program's output does,
/// so that a program cannot print another's markers.
fn marker(nonce: &str, program: usize, what: &str) -> String {
    format!("-- teq batch {nonce} {program} {what}")
}

/// The sources of one `scala-cli` run for several programs: each program in a package of its
/// own, its `@main` made a plain method, and a main that calls them in turn, marking where
/// each one's lines begin and end and where one throws. `None` for a text that is no
/// generated program.
pub fn batch_sources(texts: &[&str], nonce: &str) -> Option<Vec<(String, String)>> {
    let mut files = Vec::new();
    let mut main = "@main def teqBatch(): Unit =\n".to_string();
    for (n, text) in texts.iter().enumerate() {
        if text.matches(MAIN).count() != 1 {
            return None;
        }
        files.push((format!("batch{n}.scala"), format!("package batch{n}\n\n{}", text.replacen(MAIN, "def run(): Unit =", 1))));
        main.push_str(&format!(
            "  println({:?})\n  try\n    batch{n}.run()\n    println({:?})\n  catch case e: Throwable => println({:?} + e)\n",
            marker(nonce, n, "begins"),
            marker(nonce, n, "ends"),
            marker(nonce, n, "throws "),
        ));
    }
    files.push(("main.scala".to_string(), main));
    Some(files)
}

/// What each of `count` programs of a batch printed, cut from the batch's output by the
/// markers, which stand in the programs' order; `None` for a program whose lines do not end
/// with its marker (it threw, or the run stopped before it ended). A line outside the sections,
/// or one that carries the nonce but is not the marker due next, makes the framing ambiguous:
/// that program and every one after it are `None`, asked alone.
pub fn split_batch(printed: &str, count: usize, nonce: &str) -> Vec<Option<String>> {
    let mut sections: Vec<Option<String>> = vec![None; count];
    let mut next = 0;
    let mut open: Option<String> = None;
    for line in printed.lines() {
        if next == count {
            break;
        }
        match open.as_mut() {
            None if line == marker(nonce, next, "begins") => open = Some(String::new()),
            None => break,
            Some(lines) if line == marker(nonce, next, "ends") => {
                sections[next] = Some(std::mem::take(lines));
                open = None;
                next += 1;
            }
            Some(_) if line.starts_with(&marker(nonce, next, "throws ")) => {
                open = None;
                next += 1;
            }
            Some(_) if line.contains(nonce) => break,
            Some(lines) => {
                lines.push_str(line);
                lines.push('\n');
            }
        }
    }
    sections
}

/// scalac's lines for several programs: the answers kept, and one `scala-cli` run for the
/// others, whose output is cut into each program's lines (`batch_sources`, `split_batch`).
/// A program whose lines the batch does not give whole, because it threw, because the batch
/// was refused or because the run was cut, is asked alone, so that a refusal or a throw is
/// that program's own.
pub fn scalac_many(texts: &[&str], root: &Path) -> Vec<Result<(Lines, &'static str), String>> {
    let mut found: Vec<Option<Result<(Lines, &'static str), String>>> =
        texts.iter().map(|text| kept(text, root).map(|lines| lines.map(|lines| (lines, "kept")))).collect();
    let mut asked: Vec<usize> = Vec::new();
    for (n, text) in texts.iter().enumerate() {
        if found[n].is_none() && !asked.iter().any(|&m| texts[m] == *text) {
            asked.push(n);
        }
    }
    if asked.len() > 1 {
        let batch: Vec<&str> = asked.iter().map(|&n| texts[n]).collect();
        for (&n, printed) in asked.iter().zip(batched(&batch, root)) {
            if let Some(printed) = printed {
                found[n] = Some(publish(texts[n], root, &printed).and_then(|()| parse("scalac", &printed)).map(|lines| (lines, "batched")));
            }
        }
    }
    for n in 0..texts.len() {
        if found[n].is_none() {
            found[n] = Some(match (0..n).find(|&m| texts[m] == texts[n]) {
                Some(m) => found[m].clone().unwrap(),
                None => scalac(texts[n], root),
            });
        }
    }
    found.into_iter().map(Option::unwrap).collect()
}

/// One `scala-cli` run over the programs; `None` for every program whose lines it does not
/// give whole.
fn batched(texts: &[&str], root: &Path) -> Vec<Option<String>> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let run = format!("{}-{}", crate::driver::invocation(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let nonce = format!("{:016x}", text_hash(&format!("{run}-{nanos}")));
    let Some(files) = batch_sources(texts, &nonce) else {
        return vec![None; texts.len()];
    };
    let build = answers(root).join(format!("batch-{run}"));
    let _ = std::fs::remove_dir_all(&build);
    if std::fs::create_dir_all(build.join("src")).is_err() || files.iter().any(|(name, text)| std::fs::write(build.join("src").join(name), text).is_err()) {
        let _ = std::fs::remove_dir_all(&build);
        return vec![None; texts.len()];
    }
    let started = Instant::now();
    let output = scala_cli(&build.join("src"), Duration::from_secs(120 + 5 * texts.len() as u64), &build);
    let _ = std::fs::remove_dir_all(&build);
    let sections = output.map_or_else(|_| vec![None; texts.len()], |output| split_batch(&output.stdout, texts.len(), &nonce));
    note_invocation(root, texts.len(), sections.iter().flatten().count(), started.elapsed().as_millis());
    sections
}

/// The lines of every target that was run, or the first target's refusal. scalac is asked
/// here about a program on which the targets disagree; `asked_scalac` says whether the caller
/// asks it about one on which they agree.
pub fn targets(text: &str, labels: &[String], work: &Path, root: &Path) -> Result<Ran, String> {
    let teq = crate::teq();
    let src: PathBuf = work.join("src");
    let _ = std::fs::remove_dir_all(work);
    std::fs::create_dir_all(&src).map_err(|e| e.to_string())?;
    std::fs::write(src.join("cases.scala"), text).map_err(|e| e.to_string())?;
    let mut ran = Ran { lines: BTreeMap::new(), millis: BTreeMap::new(), scalac: "not asked" };
    // A target's run: `teq interp`, or a build followed by the runtime that runs its output, the two
    // under the one bound the raw `teq run` that did both had: the runtime gets what the build left.
    let target = |name: &'static str, build: Option<&[&str]>, runtime: Option<Command>, bound: u64, ran: &mut Ran| -> Result<(), String> {
        let bound = Duration::from_secs(bound);
        let mut command = Command::new(&teq);
        match build {
            Some(args) => command.args(["compiler", "build"]).arg(&src).args(args).current_dir(work),
            None => command.arg("interp").arg(&src).current_dir(work),
        };
        let started = Instant::now();
        let lines = match runtime {
            Some(mut runtime) => {
                built(name, command, work, bound)?;
                let left = bound.saturating_sub(started.elapsed());
                if left.is_zero() {
                    return Err(format!("{name}: the build took the whole bound of {} s it shares with the run", bound.as_secs()));
                }
                runtime.current_dir(work);
                lines_of(name, runtime, work, left)?
            }
            None => lines_of(name, command, work, bound)?,
        };
        ran.millis.insert(name, started.elapsed().as_millis());
        ran.lines.insert(name, lines);
        Ok(())
    };
    let mut node = Command::new("node");
    node.arg("out/main.js");
    target("javascript", Some(&["-o", "out/main.js"]), Some(node), 30, &mut ran)?;
    target("interpreter", None, None, 30, &mut ran)?;
    if differences(&ran, labels).is_empty() && std::env::var_os("TEQ_PROP_NO_JVM").is_none() {
        // As the raw `teq run --target jvm` started java: the std's stack, and the scala-library
        // jar the build itself linked against, from the coursier cache.
        let mut java = Command::new("java");
        java.args(["-Xss512m", "-Xshare:auto", "-cp"]).arg(format!("out/classes:{}", scala_library()?)).arg("TeqMain");
        target("jvm", Some(&["--target", "jvm", "-o", "out/classes"]), Some(java), 60, &mut ran)?;
    }
    if !differences(&ran, labels).is_empty() {
        let started = Instant::now();
        answered(&mut ran, scalac(text, root)?, started.elapsed().as_millis());
    }
    Ok(ran)
}

/// Adds scalac's answer to what the targets printed.
pub fn answered(ran: &mut Ran, (lines, how): (Lines, &'static str), millis: u128) {
    ran.millis.insert("scalac", millis);
    ran.lines.insert("scalac", lines);
    ran.scalac = how;
}

/// The kind of a difference (`confirm::KIND`): its leg and which targets print alike, or which
/// target's two forms differ, or which target prints other labels.
pub fn difference_kind(label: &str, what: &str) -> String {
    if let Some(target) = label.strip_prefix("labels of ") {
        return format!("labels differ on {target}");
    }
    let leg = label.rsplit('.').next().unwrap_or_default();
    if let Some(rest) = what.strip_prefix("on ") {
        return format!("{leg}: folded differs on {}", rest.split(' ').next().unwrap_or_default());
    }
    let mut groups: Vec<(&str, Vec<&str>)> = Vec::new();
    for line in what.lines().skip(1) {
        let Some((target, value)) = line.trim_start().split_once(": ") else { continue };
        match groups.iter_mut().find(|(printed, _)| *printed == value) {
            Some((_, targets)) => targets.push(target),
            None => groups.push((value, vec![target])),
        }
    }
    let partition: Vec<String> = groups.iter().map(|(_, targets)| targets.join(" ")).collect();
    format!("{leg}: {}", partition.join(" | "))
}

/// The kind of a refusal (`confirm::KIND`): the target and the first error's message, or a
/// target that did not end in its bound.
pub fn refusal_kind(refusal: &str) -> String {
    let first = refusal.lines().next().unwrap_or_default();
    let target = first.split(':').next().unwrap_or_default();
    if first.contains(" did not end in ") {
        return format!("timeout: {target}");
    }
    if first.starts_with("scalac refuses the program") {
        return "scalac refuses the program".to_string();
    }
    let message = refusal.lines().skip(1).find_map(|line| line.split_once("error: ").map(|(_, message)| message.to_string()).or_else(|| line.contains("Exception").then(|| line.to_string())));
    match message {
        Some(message) => format!("{target} refuses: {}", crate::confirm::normalize(&message)),
        None => format!("{target} refuses"),
    }
}

/// The labels of the lines on which the targets differ, each with what every target printed,
/// after the targets whose lines are not the program's in its order.
pub fn differences(ran: &Ran, labels: &[String]) -> Vec<(String, String)> {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for (target, lines) in &ran.lines {
        let printed: Vec<&str> = lines.iter().map(|(label, _)| label.as_str()).collect();
        if printed != labels.iter().map(|label| label.as_str()).collect::<Vec<_>>() {
            let at = printed.iter().zip(labels).position(|(a, b)| a != b).unwrap_or(printed.len().min(labels.len()));
            found.insert(
                format!("labels of {target}"),
                format!(
                    "{target} prints {} lines where {} are expected; at line {} it prints {} where {} is expected",
                    printed.len(),
                    labels.len(),
                    at + 1,
                    printed.get(at).map_or("nothing".to_string(), |label| format!("`{label}`")),
                    labels.get(at).map_or("nothing".to_string(), |label| format!("`{label}`"))
                ),
            );
        }
    }
    let by_label: BTreeMap<&str, BTreeMap<&str, &str>> =
        ran.lines.iter().map(|(target, lines)| (*target, lines.iter().map(|(label, text)| (label.as_str(), text.as_str())).collect())).collect();
    let printed = |target: &str, label: &str| by_label.get(target).and_then(|lines| lines.get(label).map(|text| text.to_string()));
    let all = |label: &str| -> String {
        let of: Vec<String> = ran.lines.keys().map(|target| format!("    {target}: {}", printed(target, label).unwrap_or("<no line>".to_string()))).collect();
        of.join("\n")
    };
    for label in labels {
        // The two forms on each target.
        if let Some(other) = label.strip_suffix(".f.v").map(|case| format!("{case}.r.v")).or(label.strip_suffix(".f.t").map(|case| format!("{case}.r.t"))) {
            for target in ran.lines.keys() {
                if let (Some(a), Some(b)) = (printed(target, label), printed(target, &other)) {
                    if a != b {
                        found.entry(label.clone()).or_insert_with(|| format!("on {target} the folded form prints {a:?} and the other {b:?}"));
                    }
                }
            }
        }
        // The targets among themselves, and scalac.
        let mut texts: Vec<Option<String>> = vec![printed("interpreter", label), printed("jvm", label), printed("scalac", label)];
        if !label.ends_with(".t") {
            texts.push(printed("javascript", label));
        }
        let texts: Vec<String> = texts.into_iter().flatten().collect();
        if texts.windows(2).any(|pair| pair[0] != pair[1]) {
            found.entry(label.clone()).or_insert_with(|| format!("the targets print\n{}", all(label)));
        }
    }
    found.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_a_label_printed_twice() {
        assert_eq!(parse("t", "0.f.v:1\n0.r.v:1\n").unwrap().len(), 2);
        assert!(parse("t", "0.f.v:999\n0.f.v:1\n0.r.v:1\n").unwrap_err().contains("printed twice"));
    }

    #[test]
    fn names_the_kind_of_a_failure() {
        let what = "the targets print\n    interpreter: 1.0E-45\n    jvm: 1.4E-45\n    scalac: 1.4E-45";
        assert_eq!(difference_kind("14.f.t", what), "t: interpreter | jvm scalac");
        assert_eq!(difference_kind("3.f.v", "on jvm the folded form prints \"3\" and the other \"3.0\""), "v: folded differs on jvm");
        let refusal = "javascript: the program is refused or ends with an error (exit status: 1):\n/x/cases.scala:3:9: error: value + is not a member of 3\n  show(..)";
        assert_eq!(refusal_kind(refusal), "javascript refuses: value _ is not a member of _");
        assert_eq!(refusal_kind("interpreter: \"/x/teq\" did not end in 30 s"), "timeout: interpreter");
    }

    #[test]
    fn makes_each_program_a_package() {
        let text = "def show(): Int = 1\n\n@main def run(): Unit =\n  println(show())\n";
        let files = batch_sources(&[text, text], "n0nce").unwrap();
        assert_eq!(files.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>(), ["batch0.scala", "batch1.scala", "main.scala"]);
        assert!(files[1].1.starts_with("package batch1\n\ndef show(): Int = 1\n\ndef run(): Unit =\n"), "{}", files[1].1);
        assert!(files[2].1.contains("    batch1.run()\n    println(\"-- teq batch n0nce 1 ends\")\n"), "{}", files[2].1);
        assert!(batch_sources(&[text, "@main def other(): Unit = ()"], "n0nce").is_none());
    }

    #[test]
    fn cuts_a_batch_by_its_markers() {
        let at = |program, what| marker("n0nce", program, what);
        let printed = [at(0, "begins"), "0.f.v:1".into(), at(0, "ends"), at(1, "begins"), "0.f.v:2".into(), format!("{}java.lang.StackOverflowError", at(1, "throws ")), at(2, "begins"), "0.f.v:3".into()].join("\n");
        assert_eq!(split_batch(&printed, 4, "n0nce"), [Some("0.f.v:1\n".to_string()), None, None, None]);
        assert_eq!(split_batch("error: not found: value x", 2, "n0nce"), [None, None]);
    }

    #[test]
    fn keeps_a_program_from_printing_another_ones_markers() {
        let at = |program, what| marker("n0nce", program, what);
        // Program 1 prints the markers of an earlier framing and of program 0 with a guessed nonce.
        let forged = ["-- teq batch 0 begins", "0.f.v:999", "-- teq batch 0 ends"].join("\n");
        let printed = [at(0, "begins"), "0.f.v:1".into(), at(0, "ends"), at(1, "begins"), forged.clone(), at(1, "ends")].join("\n");
        assert_eq!(split_batch(&printed, 2, "n0nce"), [Some("0.f.v:1\n".to_string()), Some(format!("{forged}\n"))]);
        let replayed = [at(0, "begins"), "0.f.v:1".into(), at(0, "ends"), at(1, "begins"), at(0, "begins"), "0.f.v:999".into(), at(0, "ends"), at(1, "ends")].join("\n");
        assert_eq!(split_batch(&replayed, 2, "n0nce"), [Some("0.f.v:1\n".to_string()), None]);
    }

    #[test]
    fn holds_the_lines_to_the_labels() {
        let labels = vec!["0.f.v".to_string(), "0.r.v".to_string()];
        let mut ran = Ran { lines: BTreeMap::new(), millis: BTreeMap::new(), scalac: "not asked" };
        ran.lines.insert("interpreter", parse("i", "0.f.v:1\n0.r.v:1\n").unwrap());
        ran.lines.insert("javascript", parse("j", "0.r.v:1\n0.f.v:1\n").unwrap());
        let found = differences(&ran, &labels);
        assert_eq!(found.len(), 1);
        assert!(found[0].1.contains("at line 1 it prints `0.r.v` where `0.f.v` is expected"), "{}", found[0].1);
        ran.lines.insert("javascript", parse("j", "0.f.v:2\n0.r.v:1\n").unwrap());
        let found = differences(&ran, &labels);
        assert!(found.iter().any(|(label, what)| label == "0.f.v" && what.contains("folded form prints \"2\"")), "{found:?}");
    }
}
