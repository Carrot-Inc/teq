//! `teq run <project> [<alias or main class>] [-- args]`: a JVM project's main class run
//! as sbt's forked `run` runs it. The word is an alias of the export's run block, a declared
//! main class or a qualified class of the build; without one the main class is implied as
//! sbt's `run` implies it: the one the build declares (the run block's `mainClass`), else the
//! single main class among the project's own products, which the daemon answers with the class
//! directory, else a refusal naming them (the choice is `implied` in `mod.rs`, the stage's too; the words are each verb's). The daemon compiles the project's closure as `compile` does and
//! answers the class directory of the resident that wrote it; the client then starts `java`
//! itself, so that the program is the developer's job: the `run` block's options, an argument
//! file with the runtime classpath in the export's order (the class directory where the products
//! stand, each product's resources after it, the artifacts and the repository's files), the main
//! class, the args; in the block's directory, with the client's environment and the block's
//! variables. The JVM is the test runner's, `JAVA_HOME`'s else the one on `PATH`, refused when it
//! is older than the export's classes. It leads its own process group and holds the terminal
//! (`job.rs`), and its exit code is the verb's.

use super::client;
use super::export::{Entry, Export, Key, Platform};
use super::{job, runner, Implied};
use crate::lsp::json::Json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const POLL: Duration = Duration::from_millis(20);

pub fn run(export: &Export, args: &[String]) -> i32 {
    let Some((name, rest)) = args.split_first() else {
        eprintln!("usage: teq run <project> [alias or main class] [-- args]");
        return 2;
    };
    // The word after the project is the alias or main class unless it is `--`, which starts the
    // program's arguments; anything else before `--` is refused.
    let (word, rest) = match rest.split_first() {
        Some((word, rest)) if word != "--" => (Some(word), rest),
        _ => (None, rest),
    };
    let extra = match rest.split_first() {
        None => &[][..],
        Some((dashes, extra)) if dashes == "--" => extra,
        Some((other, _)) => {
            eprintln!("teq run: {} is not --: arguments for the program follow --", other);
            return 2;
        }
    };
    let Some(project) = export.projects.get(name) else {
        eprintln!("teq: no project {} in {} (projects: {})", name, export.file.display(), export.projects.keys().cloned().collect::<Vec<_>>().join(", "));
        return 2;
    };
    let runnable: Vec<&str> = export.projects.iter().filter(|(_, p)| p.run.is_some() && p.platform == Platform::Jvm).map(|(n, _)| n.as_str()).collect();
    let with = || if runnable.is_empty() { "none has".to_string() } else { format!("{} {}", runnable.join(", "), if runnable.len() == 1 { "has" } else { "have" }) };
    if project.platform != Platform::Jvm {
        eprintln!("teq: {} is a Scala.js project, which teq dev runs; run takes a JVM project with a run block ({} one)", name, with());
        return 2;
    }
    let Some(context) = project.run.as_ref() else {
        eprintln!("teq: {} has no run block: its export predates the block every JVM project gets, export again ({} one)", name, with());
        return 2;
    };
    if let Some(why) = super::refused(export, "run", &[name]) {
        eprintln!("teq: {}", why);
        return 2;
    }
    let declared: Vec<&str> = export.configuration(&Key::new(name, "compile")).map_or(&[][..], |c| &c.main_classes).iter().map(String::as_str).collect();
    let names = || {
        let aliases: Vec<String> = context.aliases.iter().map(|(a, m)| format!("{} ({})", a, m)).collect();
        format!("aliases: {}; main classes: {}", if aliases.is_empty() { "none".to_string() } else { aliases.join(", ") }, if declared.is_empty() { "none the export names".to_string() } else { declared.join(", ") })
    };
    // An alias is a plain word and a main class a qualified name, so a word that is neither is
    // refused before the build. Without a word, the declared main class; without one the
    // products decide after the build.
    let main = match word {
        Some(word) => match context.aliases.get(word.as_str()) {
            Some(main) => Some(main.clone()),
            None if word.contains('.') || declared.contains(&word.as_str()) => Some(word.clone()),
            None => {
                eprintln!("teq: {} is neither an alias of {} nor a main class ({}); arguments for the program follow --", word, name, names());
                return 2;
            }
        },
        None => context.main_class.clone(),
    };
    let environment: BTreeMap<String, String> = std::env::vars_os().filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?))).collect();
    let java = match runner::java(&environment).and_then(|java| check_version(&java, export.java_output_version).map(|()| java)) {
        Ok(java) => java,
        Err(e) => {
            eprintln!("teq: {}", e);
            return 2;
        }
    };
    let (code, data) = client::request_data(export, "classes", std::slice::from_ref(name));
    if code != 0 {
        return code;
    }
    let Some(answer) = data.iter().find(|d| d.get("classes").is_some()) else {
        eprintln!("teq: the daemon named no class directory for {}", name);
        return 2;
    };
    let classes = export.path(answer.get("classes").and_then(Json::str).unwrap_or_default());
    let main = match main {
        Some(main) => main,
        None => {
            let Some(mains) = answer.get("mainClasses") else {
                let why = answer.get("mainClassesUnknown").and_then(Json::str).unwrap_or("the daemon named none");
                eprintln!("teq: the main classes of {}'s products are unknown: {}", name, why);
                return 2;
            };
            match super::implied(mains.arr().iter().filter_map(Json::str).map(str::to_string).collect()) {
                Implied::One(main) => main,
                Implied::Zero => {
                    eprintln!("teq: no main class in {}'s products", name);
                    return 2;
                }
                Implied::Several(several) => {
                    eprintln!("teq: {} declares no main class and its products hold {}, which sbt's run would ask to choose among: {}; name one", name, several.len(), several.join(", "));
                    return 2;
                }
            }
        }
    };
    if !declared.contains(&main.as_str()) && !context.aliases.values().any(|m| *m == main) && !classes.join(format!("{}.class", main.replace('.', "/"))).is_file() {
        eprintln!("teq: {} is no class of {}'s build ({})", main, name, names());
        return 2;
    }
    let argfile = export.path(&format!("target/teq/{}/runtime/run.args", name));
    let prepared = classpath(export, name, &classes).and_then(|cp| write_argfile(&argfile, &cp));
    if let Err(e) = prepared {
        eprintln!("teq: {}", e);
        return 2;
    }
    let mut command = Command::new(&java);
    command.args(&context.java_options).arg(format!("@{}", argfile.display())).arg(&main).args(extra).current_dir(export.path(&context.base_directory)).envs(&context.env_vars);
    job::take_signals();
    let mut child = match job::start(command) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("teq: cannot start {}: {}", java.display(), e);
            return 2;
        }
    };
    // A signal passed on gives the program 5 s from its arrival, whether or not it ends on it.
    loop {
        if let Some(sig) = job::received() {
            job::wait_for(&mut child);
            return 128 + sig;
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                job::stop(&mut child);
                return job::code_of(status);
            }
            Ok(None) => std::thread::sleep(POLL),
            Err(e) => {
                eprintln!("teq: cannot wait for {}: {}", java.display(), e);
                job::stop(&mut child);
                return 2;
            }
        }
    }
}


/// The runtime classpath in the export's order: the class directory where the first product
/// stands, each product's resource directories after it, the artifacts and the repository's
/// files resolved.
fn classpath(export: &Export, name: &str, classes: &Path) -> Result<Vec<PathBuf>, String> {
    let runtime = export.configuration(&Key::new(name, "runtime")).ok_or_else(|| format!("{} has no runtime configuration", name))?;
    let mut out = Vec::new();
    for entry in &runtime.classpath {
        match entry {
            Entry::Product(key) => {
                if !out.contains(&classes.to_path_buf()) {
                    out.push(classes.to_path_buf());
                }
                out.extend(export.configuration(key).map_or(&[][..], |c| &c.resources).iter().map(|r| export.path(r)).filter(|r| r.is_dir()));
            }
            entry => out.push(export.resolve(entry)?),
        }
    }
    if !out.contains(&classes.to_path_buf()) {
        out.insert(0, classes.to_path_buf());
    }
    Ok(out)
}

/// The classpath as a java argument file, which keeps a long one off the command line (past
/// Windows' limit), written whole by a rename.
fn write_argfile(file: &Path, classpath: &[PathBuf]) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {}", dir.display(), e))?;
    }
    let bytes = runner::classpath_argfile(classpath)?;
    let part = file.with_extension(format!("args.{}", std::process::id()));
    std::fs::write(&part, bytes).map_err(|e| format!("cannot write {}: {}", part.display(), e))?;
    std::fs::rename(&part, file).map_err(|e| format!("cannot write {}: {}", file.display(), e))
}

/// Refuses a JVM older than the class files: its version from the `release` file of the JDK it
/// belongs to, else from `java -version`.
fn check_version(java: &Path, least: Option<u32>) -> Result<(), String> {
    let Some(least) = least else { return Ok(()) };
    let version = release_version(java).or_else(|| reported_version(java)).ok_or_else(|| format!("cannot tell the version of {}", java.display()))?;
    if version < least {
        return Err(format!("{} is Java {}, older than the {} the export's classes are for (java.outputVersion): set JAVA_HOME to a JDK {} or later", java.display(), version, least, least));
    }
    Ok(())
}

fn release_version(java: &Path) -> Option<u32> {
    let home = std::fs::canonicalize(java).ok()?.parent()?.parent()?.to_path_buf();
    let release = std::fs::read_to_string(home.join("release")).ok()?;
    let line = release.lines().find_map(|l| l.strip_prefix("JAVA_VERSION="))?;
    major(line.trim_matches('"'))
}

fn reported_version(java: &Path) -> Option<u32> {
    let out = Command::new(java).arg("-version").output().ok()?;
    let text = String::from_utf8_lossy(&out.stderr);
    let quoted = text.split('"').nth(1)?;
    major(quoted)
}

/// The major version of `17.0.2`, `21`, `1.8.0_292`.
fn major(version: &str) -> Option<u32> {
    let mut parts = version.split(['.', '_', '-', '+']);
    match parts.next()?.parse().ok()? {
        1 => parts.next()?.parse().ok(),
        n => Some(n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_major_version_of_what_a_jdk_reports() {
        assert_eq!(major("17.0.2"), Some(17));
        assert_eq!(major("24"), Some(24));
        assert_eq!(major("1.8.0_292"), Some(8));
        assert_eq!(major("21-ea"), Some(21));
        assert_eq!(major("x"), None);
    }
}
