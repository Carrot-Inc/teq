//! `teq`: the driver over the export, `teq.lock` at the build's root (committed, under the
//! native driver) or under its `target/teq/` (docs/TARGETS.md, "The export and the project verbs"). It reads the export,
//! refuses a project whose export records what sbt alone runs for the verb (`refused`), warns
//! when the build definition changed since it was written, and answers its verbs through the
//! export's session daemon, which keeps one resident per closure and one test runner per test
//! configuration, and writes the Docker stage (`stage.rs`); `run` has the daemon compile and runs
//! the program itself (`run.rs`), `watch` runs a project's resident on stdin and stdout, `build`
//! one build, and `dev` a Scala.js project's dev loop (`dev.rs`), none of them through the daemon.

mod analysis;
mod client;
mod daemon;
mod dev;
pub mod export;
pub mod fetch;
pub mod generators;
mod job;
pub mod lock;
mod resident;
mod run;
mod runner;
pub mod sha256;
mod stage;
mod suites;

use export::{Export, Platform};
use std::path::{Path, PathBuf};

/// The version of the daemon's line protocol, which the handshake compares.
pub const PROTOCOL: u32 = 1;

/// The daemon's files under the build's `target/teq/`: where it listens, the lock it holds for
/// its whole life, the lock that serialises its start, its log.
pub struct Paths {
    pub info: PathBuf,
    pub lock: PathBuf,
    pub start_lock: PathBuf,
    pub log: PathBuf,
}

pub fn paths(root: &Path) -> Paths {
    let dir = root.join("target/teq");
    Paths { info: dir.join("daemon.json"), lock: dir.join("task.lock"), start_lock: dir.join("task.start.lock"), log: dir.join("task.log") }
}

/// What `target/teq/daemon.json` says of the daemon: the loopback port it listens on, the token
/// every request opens with (so that another user of the machine cannot talk to it), its
/// process and the digest of the export it serves.
pub struct Info {
    pub port: u16,
    pub token: String,
    pub pid: u32,
    pub sha256: String,
}

impl Info {
    pub fn read(file: &Path) -> Option<Info> {
        let json = crate::lsp::json::Json::parse(&std::fs::read_to_string(file).ok()?).ok()?;
        let text = |k: &str| json.get(k).and_then(crate::lsp::json::Json::str).map(str::to_string);
        Some(Info {
            port: json.get("port").and_then(crate::lsp::json::Json::uint).and_then(|p| u16::try_from(p).ok())?,
            token: text("token")?,
            pid: json.get("pid").and_then(crate::lsp::json::Json::uint)?,
            sha256: text("sha256").unwrap_or_default(),
        })
    }

    /// Written whole by a rename, readable by its owner alone.
    pub fn write(&self, file: &Path) -> std::io::Result<()> {
        use crate::lsp::json::obj;
        let text = obj([("port", u32::from(self.port).into()), ("token", self.token.as_str().into()), ("pid", self.pid.into()), ("sha256", self.sha256.as_str().into())]).to_text();
        let partial = file.with_extension(format!("json.{}", std::process::id()));
        std::fs::write(&partial, text)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&partial, std::fs::Permissions::from_mode(0o600))?;
        }
        std::fs::rename(&partial, file)
    }
}

/// 32 bytes of the system's randomness, in hexadecimal: the keys `RandomState` draws from it,
/// through SipHash.
pub fn token() -> String {
    use std::hash::{BuildHasher, Hasher};
    (0..4u64)
        .map(|i| {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u64(i);
            h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
            h.write_u32(std::process::id());
            format!("{:016x}", h.finish())
        })
        .collect()
}

/// Whether a daemon holds the export's lock, which it does for its whole life.
pub fn daemon_alive(lock: &Path) -> bool {
    let Ok(file) = std::fs::OpenOptions::new().write(true).open(lock) else { return false };
    match file.try_lock() {
        Ok(()) => {
            let _ = file.unlock();
            false
        }
        Err(_) => true,
    }
}

/// The version as `teq --version` prints it.
pub fn version() -> String {
    let checks = if cfg!(debug_assertions) { " assertions" } else { "" };
    match env!("TEQ_GIT_HASH") {
        "" => format!("teq {}{}", env!("CARGO_PKG_VERSION"), checks),
        hash => format!("teq {} {}{}", env!("CARGO_PKG_VERSION"), hash, checks),
    }
}

const VERBS: &str = "  compile [project...]       type the projects' closures and write their class files
  test [project] [pattern...] [--changed] [--list] [--verbose]
                             compile, then run the project's suites (every JVM project's without
                             one) on its warm runner: those the patterns name, as sbt's testOnly
                             takes them, or with --changed those whose classes depend on the
                             classes written since the last run of every suite; --list prints
                             the suites as sbt's definedTests, --verbose the frameworks' debug
  run <project> [alias or main class] [-- args]
                             compile, then run the main class (an alias of the project's run
                             block names one; without a word the declared one, else the single
                             main class among the products, as sbt's run) on a JVM of its own
                             with the runtime classpath
  stage [project]            compile, then write the project's Docker stage (every project's
                             with a stage block without one) as sbt-native-packager's does
  watch <project> [options]  the project's resident on stdin and stdout, options added to its
                             command line (a Scala.js project's dev build by default)
  build <project> [options]  one build of a Scala.js project, options added to its command line
                             (the production build with --release)
  dev <project> [-- args]    a Scala.js project's dev loop: its generators, the package manager's
                             install, the export's dev command with the args added, restarted
                             when the export or the binary changes
  stop                       end the daemon, its residents and its test runners";

/// The verbs after the project verbs, a line each.
const WORDS: &str = "  interp <files or directories...> [options] [-- args]
                             run a program in the compiler's own interpreter, no node or JVM
                             needed; teq interp --help lists its options
  compiler <build|check|watch> <files or directories...> [options]
                             compile files to JavaScript or JVM class files, type check them,
                             or keep a build resident; teq compiler --help lists its options
  lsp [--stdio]              a language server over stdin and stdout for the projects of the
                             workspace";

/// The prefix options of the project verbs (the daemon's own stay out: internal).
const OPTIONS: &str = "  --export file              the export a project verb reads, by default the teq.lock at or
                             above the working directory
  --strict                   refuse an export written before its inputs changed, where by
                             default the verb warns and goes on";

/// The top-level usage, ending with the export's projects when one was read.
fn usage_text(export: Option<&Export>) -> String {
    let mut text = format!("usage: teq [--export file] [--strict] <verb> [args]\n\n{}\n\n{}\n\n{}", VERBS, WORDS, OPTIONS);
    if let Some(export) = export {
        let projects: Vec<String> = export.projects.iter().map(|(n, p)| format!("{} ({})", n, if p.platform == Platform::Jvm { "jvm" } else { "js" })).collect();
        text.push_str(&format!("\n\nprojects of {}: {}", export.file.display(), projects.join(", ")));
    }
    text
}

fn usage(export: Option<&Export>) -> ! {
    eprintln!("{}", usage_text(export));
    std::process::exit(2);
}

/// `teq --help`: the usage on stdout, exit 0.
pub fn help() -> ! {
    println!("{}", usage_text(quiet_export().as_ref()));
    std::process::exit(0);
}

/// A first word that is no verb: the usage, exit 2.
pub fn unknown(word: &str) -> ! {
    eprintln!("teq: no verb {}", word);
    usage(quiet_export().as_ref())
}

/// The export at or above the working directory when it reads, for a usage that lists its
/// projects; nothing is said when there is none or it does not read.
fn quiet_export() -> Option<Export> {
    let cwd = std::env::current_dir().ok()?;
    Export::read(&export::find(&cwd)?).ok()
}

/// The eight project verbs, the top level's own.
pub const PROJECT_VERBS: [&str; 8] = ["compile", "test", "run", "stage", "watch", "build", "dev", "stop"];

/// The options `run` reads before the verb: the project verbs' own, and the daemon's.
const PREFIX_OPTIONS: [&str; 4] = ["--export", "--strict", "--daemon", "--identity"];

/// Whether a command line's first word is `run`'s: a project verb or a prefix option.
pub fn takes(word: &str) -> bool {
    PROJECT_VERBS.contains(&word) || PREFIX_OPTIONS.contains(&word)
}

/// `teq [--export file] [--strict] <verb> [args]`, the whole command line after the binary;
/// the daemon's own `--daemon --identity <id> --export <file>` comes without a verb. A prefix
/// option before any word but a project verb is refused.
pub fn run(args: &[String]) -> ! {
    let mut export_file: Option<PathBuf> = None;
    let mut identity: Option<String> = None;
    let (mut strict, mut daemon) = (false, false);
    let mut prefix: Option<String> = None;
    let mut rest = args.iter();
    let mut verb = None;
    while let Some(a) = rest.next() {
        match a.as_str() {
            "--export" => export_file = rest.next().map(PathBuf::from),
            "--strict" => strict = true,
            "--daemon" => daemon = true,
            "--identity" => identity = rest.next().cloned(),
            "--help" | "-h" => help(),
            _ => {
                verb = Some(a.clone());
                break;
            }
        }
        prefix.get_or_insert_with(|| a.clone());
    }
    let rest: Vec<String> = rest.cloned().collect();
    if daemon {
        let (Some(file), Some(identity)) = (export_file, identity) else { usage(None) };
        daemon::run(&file, identity);
    }
    if let Some(word) = verb.as_deref().filter(|v| !PROJECT_VERBS.contains(v)) {
        if let Some(option) = prefix.filter(|_| matches!(word, "compiler" | "interp" | "lsp" | "tasty" | "classfile" | "--version")) {
            eprintln!("teq: {} applies to the project verbs", option);
            std::process::exit(2);
        }
        unknown(word);
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let Some(file) = export_file.or_else(|| export::find(&cwd)) else {
        if verb.is_none() {
            usage(None);
        }
        eprintln!("teq: no {} at or above {}, at a directory or under its {}: export the build with sbt teqExportAll; for files, teq compiler build and teq interp", export::FILE, cwd.display(), export::UNDER_TARGET.trim_end_matches(export::FILE));
        std::process::exit(2);
    };
    let export = match Export::read(&file) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("teq: {}", e);
            std::process::exit(2);
        }
    };
    let stale = export.stale_inputs();
    if !stale.is_empty() {
        let what = if strict { "refusing a stale export" } else { "warning" };
        let commit = if export.file.parent() == Some(export.root.as_path()) { " and commit the file" } else { "" };
        let command = if stale.line_ends_only() { String::new() } else { format!("; run sbt teqExportAll{}", commit) };
        eprintln!("teq: {}: {} was written before {}{}", what, export::FILE, stale.changes.join(", "), command);
        if let Some(note) = stale.line_end_note() {
            eprintln!("teq: note: {}", note);
        }
        if strict {
            std::process::exit(2);
        }
    }
    let Some(verb) = verb else { usage(Some(&export)) };
    let code = match verb.as_str() {
        "compile" | "test" | "stage" | "stop" => client::request(&export, &verb, &rest),
        "run" => run::run(&export, &rest),
        "watch" | "build" => one_project(&export, &verb, &rest),
        "dev" => dev::run(export, &rest),
        _ => unreachable!("a project verb"),
    };
    std::process::exit(code)
}

/// A verb's refusal of the projects it was given, every reason the export records of the
/// projects involved in one failure; none when the verb runs them.
pub fn refused(export: &Export, verb: &str, projects: &[&str]) -> Option<String> {
    let mut reasons: Vec<String> = Vec::new();
    for why in projects.iter().flat_map(|p| export.refusals(verb, p)) {
        if !reasons.contains(&why) {
            reasons.push(why);
        }
    }
    (!reasons.is_empty()).then(|| format!("cannot {} {}, which need{} what sbt alone runs:\n{}", verb, projects.join(" "), if projects.len() == 1 { "s" } else { "" }, reasons.iter().map(|r| format!("  {}", r)).collect::<Vec<_>>().join("\n")))
}

/// The exit code of a verb over every project: 2 when it left a project out, so that a run that
/// omitted work never passes, else the run's own.
pub fn with_left_out(code: i32, left_out: &[String]) -> i32 {
    if left_out.is_empty() { code } else { 2 }
}

/// The summary's lines of a verb over every project for those it left out, each with its
/// reasons; empty when it left out none.
pub fn left_out(export: &Export, verb: &str, projects: &[String]) -> String {
    projects
        .iter()
        .map(|p| format!("teq: {} left out {}, which needs what sbt alone runs:\n{}\n", verb, p, export.refusals(verb, p).iter().map(|r| format!("  {}", r)).collect::<Vec<_>>().join("\n")))
        .collect()
}

/// `watch <project> [options]` and `build <project> [options]`: the project's generators run,
/// its artifacts resolved, then this process becomes the resident, or the build. A Scala.js
/// project's is the description's (`js_args`); a JVM project's resident is the daemon's of its
/// widest configuration, writing the daemon's class directory unless the options give another
/// `-o`, and a JVM project has no one-shot build here (`compile` is the daemon's). The arguments,
/// the options last, go in an argument file of this command's own, named by them beside the
/// configuration's class directory (`argfile::named_by`): no other command rewrites it under the
/// process, whose retry by one worker reads it again, and the options, expanded already, are not
/// read as argument files once more.
fn one_project(export: &Export, verb: &str, args: &[String]) -> i32 {
    let Some((name, options)) = args.split_first() else { usage(Some(export)) };
    let Some(project) = export.projects.get(name) else {
        eprintln!("teq: no project {} in {}", name, export.file.display());
        return 2;
    };
    if let Some(why) = refused(export, verb, &[name]) {
        eprintln!("teq: {}", why);
        return 2;
    }
    let Some(key) = export.widest(name) else {
        eprintln!("teq: project {} has neither a compile nor a test configuration", name);
        return 2;
    };
    if verb == "build" && project.platform == Platform::Jvm {
        eprintln!("teq: build is a Scala.js project's; {} is a JVM project, which compile builds", name);
        return 2;
    }
    let closure = export.closure(&key);
    let env = std::env::vars().filter(|(k, _)| k == "SOURCE_VERSION").collect();
    let prepared = generators::run(export, &closure, export.path(&export.classes(&key)), &env).and_then(|notes| {
        for note in notes {
            eprintln!("{}", note);
        }
        let (mut args, configuration) = match project.platform {
            Platform::Jvm => (export.resident_args_with(&key, &|line| eprintln!("{}", line))?, key.configuration.as_str()),
            Platform::Js => (js_args(export, name, verb, options)?, "compile"),
        };
        args.extend(options.iter().cloned());
        let file = crate::argfile::named_by(&export.path(&format!("target/teq/{}/{}", name, configuration)), verb, &args);
        crate::argfile::write(&file, &args)?;
        Ok(file)
    });
    let file = match prepared {
        Ok(f) => f,
        Err(e) => {
            eprintln!("teq: {}", e);
            return 2;
        }
    };
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("teq"));
    let mut command = std::process::Command::new(exe);
    command.arg(crate::argfile::at(&file)).current_dir(&export.root);
    become_resident(command)
}

/// Replaces this process with the resident, its stdin and stdout the caller's.
#[cfg(unix)]
fn become_resident(mut command: std::process::Command) -> i32 {
    use std::os::unix::process::CommandExt;
    let error = command.exec();
    eprintln!("teq: cannot run the resident: {}", error);
    2
}

/// Runs the resident on this process's stdin and stdout, its exit code this one's.
#[cfg(not(unix))]
fn become_resident(mut command: std::process::Command) -> i32 {
    match command.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("teq: cannot run the resident: {}", error);
            2
        }
    }
}

/// A Scala.js project's `watch` or `build` of its description: its stand-ins and sources,
/// excludes, libraries and typer's settings, then the description's output unless the options
/// name one (split, with its per-file packages and `--hot` as the description sets them). A build
/// with `--release`, or of a description that sets `release`, is the production build, which adds
/// the description's production sources and excludes, and `--release`.
fn js_args(export: &Export, name: &str, verb: &str, options: &[String]) -> Result<Vec<String>, String> {
    let project = &export.projects[name];
    let d = &project.description;
    let names_output = options.iter().any(|o| matches!(o.as_str(), "--split" | "-o" | "--check"));
    let asks_release = options.iter().any(|o| o == "--release");
    let production = verb == "build" && (asks_release || d.release);
    let mut args = vec!["compiler".to_string(), verb.to_string()];
    let compile = export::Key::new(name, "compile");
    args.extend(export.session_inputs(&compile).into_iter().filter(|p| p.exists()).map(|p| export.relative(&p)));
    let production_part = |list: &[String]| if production { list.to_vec() } else { Vec::new() };
    args.extend(production_part(&d.production.sources));
    for x in d.excludes.iter().chain(&production_part(&d.production.excludes)) {
        args.extend(["--exclude".to_string(), x.clone()]);
    }
    let jars = export.jars_with(&compile, &|line| eprintln!("{}", line))?;
    if !jars.is_empty() {
        args.extend(["--classpath".to_string(), export::join_paths(&jars)]);
    }
    args.extend(d.session_args());
    if let Some(conf) = export.configuration(&compile) {
        args.extend(conf.flags.args(false));
    }
    if !names_output {
        let out = d.out.clone().unwrap_or_else(|| format!("{}/target/teq/out", project.base));
        args.extend(["--split".to_string(), out]);
        if !d.module_per_file.is_empty() {
            args.extend(["--module-per-file".to_string(), d.module_per_file.join(",")]);
        }
        if d.hot {
            args.push("--hot".to_string());
        }
    }
    if production && !asks_release {
        args.push("--release".to_string());
    }
    if let Some(main) = &d.main_class {
        args.extend(["--main".to_string(), main.clone()]);
    }
    Ok(args)
}

/// What a project's products imply for its main class when the build declares none, as sbt's
/// `mainClass` picks it after a compile: the single main class among them, else the several in
/// their sorted order, or none, which each verb refuses in its own words. `run` and `stage` share
/// the choice.
pub enum Implied {
    One(String),
    Several(Vec<String>),
    Zero,
}

/// `mains`: the main classes among the products of the project's own compile sources, as the
/// daemon's view answers them, sorted.
pub fn implied(mut mains: Vec<String>) -> Implied {
    match mains.len() {
        1 => Implied::One(mains.remove(0)),
        0 => Implied::Zero,
        _ => Implied::Several(mains),
    }
}
