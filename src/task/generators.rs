//! The export's source generators, run before a resident's build (docs/TARGETS.md, "The
//! export and the project verbs"): `buildInfo`, sbt-buildinfo's object written from the export's static values and the
//! dynamic ones computed here (`gitSha`, a configuration's `classDirectory`), and `command`, a
//! program of the repository run with the export's arguments in its working directory whenever
//! its fingerprint changed (the generator's block, the files its inputs' globs match and the
//! files its arguments name) or one of its outputs is missing: the generated directory, or a file
//! or directory the program writes besides it. A generated file whose text came out the same keeps
//! its modification time, so that the resident does not type it again. A command whose first word
//! is `teq` runs the build's own binary, the one running.

use super::export::{Export, Key};
use super::lock::Value;
use super::sha256::Sha256;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

/// Runs the generators of a closure's configurations; what they printed is returned to be shown.
/// A configuration with a generator sbt runs (kind `sbt`) has none run here: its sources name
/// sbt's managed directories, which hold what sbt last generated of them all, and the verbs that
/// would need them run refuse first (`Export::refusals`). The runs of the build are serialised through `target/teq/generators.lock`: two residents
/// whose closures share a configuration, or the daemon and the language server, run its
/// generators one after the other, never into one output at once.
pub fn run(export: &Export, closure: &[Key], classes: PathBuf, env: &BTreeMap<String, String>) -> Result<Vec<String>, String> {
    let mut notes = Vec::new();
    let ours = |k: &Key| export.configuration(k).filter(|c| !c.generators.iter().any(super::export::by_sbt));
    if closure.iter().all(|k| ours(k).is_none_or(|c| c.generators.is_empty())) {
        return Ok(notes);
    }
    let lock_file = export.path("target/teq/generators.lock");
    if let Some(dir) = lock_file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {}", dir.display(), e))?;
    }
    let lock = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(&lock_file).map_err(|e| format!("cannot open {}: {}", lock_file.display(), e))?;
    lock.lock().map_err(|e| format!("cannot hold {}: {}", lock_file.display(), e))?;
    let teq = std::env::current_exe().map_err(|e| e.to_string());
    for key in closure {
        let Some(conf) = ours(key) else { continue };
        for (i, generator) in conf.generators.iter().enumerate() {
            let what = format!("{} generator {} of {}", generator.get("kind").and_then(Value::str).unwrap_or("?"), i, key);
            match generator.get("kind").and_then(Value::str) {
                Some("buildInfo") => {
                    let class_directory = |dep: &Key| if closure.contains(dep) { classes.clone() } else { export.path(&export.classes(dep)) };
                    let (output, text) = build_info(export, generator, env, &class_directory).map_err(|e| format!("{}: {}", what, e))?;
                    write_if_changed(&output, &text).map_err(|e| format!("{}: {}", what, e))?;
                }
                Some("command") => notes.extend(command(export, key, i, generator, &teq).map_err(|e| format!("{}: {}", what, e))?),
                other => return Err(format!("{}: no generator of kind {}", what, other.unwrap_or("missing"))),
            }
        }
    }
    Ok(notes)
}

fn write_if_changed(file: &Path, text: &str) -> Result<(), String> {
    if std::fs::read_to_string(file).is_ok_and(|t| t == text) {
        return Ok(());
    }
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {}", dir.display(), e))?;
    }
    std::fs::write(file, text).map_err(|e| format!("cannot write {}: {}", file.display(), e))
}

/// The `BuildInfo` source and its file, as sbt-buildinfo 0.13.1's Scala 3 case object without
/// options writes it, byte for byte: the members in the order of the export's `members`
/// (sbt-buildinfo's, the order of its keys; of their names when the export does not say), its
/// literals and its platform's line separator. A string is a `String`, an integer a `scala.Int`
/// (a `scala.Long` past its range), a boolean a `scala.Boolean`, a list of strings a
/// `scala.collection.immutable.Seq[String]`, a class directory a `java.io.File`.
fn build_info(export: &Export, g: &Value, env: &BTreeMap<String, String>, class_directory: &dyn Fn(&Key) -> PathBuf) -> Result<(PathBuf, String), String> {
    let output = g.get("output").and_then(Value::str).ok_or("no output")?;
    let package = g.get("package").and_then(Value::str).unwrap_or("buildinfo");
    let object = g.get("object").and_then(Value::str).unwrap_or("BuildInfo");
    let mut members: BTreeMap<String, (String, String)> = BTreeMap::new();
    if let Some(Value::Map(fields)) = g.get("static") {
        for (name, value) in fields {
            members.insert(name.clone(), scala_value(value).ok_or_else(|| format!("static key {} has a value of no supported type", name))?);
        }
    }
    if let Some(Value::Map(fields)) = g.get("dynamic") {
        for (name, value) in fields {
            let member = match value {
                Value::Str(kind) if kind == "gitSha" => ("String".to_string(), scala_string(&git_sha(env, &export.root)?)),
                Value::Map(_) if value.get("classDirectory").is_some() => {
                    let of = value.get("classDirectory").expect("checked");
                    let key = Key::new(of.get("project").and_then(Value::str).unwrap_or(""), of.get("configuration").and_then(Value::str).unwrap_or("compile"));
                    if export.configuration(&key).is_none() {
                        return Err(format!("dynamic key {} names {}, which the export does not hold", name, key));
                    }
                    ("java.io.File".to_string(), format!("new java.io.File({})", scala_string(&class_directory(&key).to_string_lossy())))
                }
                _ => return Err(format!("dynamic key {} is gitSha or a classDirectory", name)),
            };
            members.insert(name.clone(), member);
        }
    }
    let order: Vec<String> = match g.get("members") {
        Some(listed) => {
            let names: Vec<String> = listed.items().iter().filter_map(Value::str).map(str::to_string).collect();
            let mut sorted = names.clone();
            sorted.sort();
            if !sorted.iter().eq(members.keys()) {
                return Err(format!("members names {:?}, not the static and dynamic keys {:?}", names, members.keys().collect::<Vec<_>>()));
            }
            names
        }
        None => members.keys().cloned().collect(),
    };
    let mut lines = vec![
        "// $COVERAGE-OFF$".to_string(),
        format!("package {}", package),
        String::new(),
        "/** This object was generated by sbt-buildinfo. */".to_string(),
        format!("case object {} {{", object),
    ];
    for name in &order {
        let (ty, value) = &members[name];
        lines.push(format!("  /** The value is {}. */", value));
        lines.push(format!("  val {}: {} = {}", name, ty, value));
    }
    let format = order.iter().map(|n| format!("{}: %s", n)).collect::<Vec<_>>().join(", ");
    lines.push("  override val toString: String = {".to_string());
    lines.push(format!("    \"{}\".format(", format));
    lines.push(format!("      {}", order.join(", ")));
    lines.push("    )".to_string());
    lines.push("  }".to_string());
    lines.push("}".to_string());
    lines.push("// $COVERAGE-ON$".to_string());
    // sbt's IO.writeLines ends every line with the platform's separator.
    let separator = if cfg!(windows) { "\r\n" } else { "\n" };
    Ok((export.path(output), lines.iter().map(|l| format!("{}{}", l, separator)).collect()))
}

fn scala_value(value: &Value) -> Option<(String, String)> {
    Some(match value {
        Value::Str(s) => ("String".to_string(), scala_string(s)),
        Value::Bool(b) => ("scala.Boolean".to_string(), b.to_string()),
        Value::Int(n) if i32::try_from(*n).is_ok() => ("scala.Int".to_string(), n.to_string()),
        Value::Int(n) => ("scala.Long".to_string(), format!("{}L", n)),
        Value::List(items) => {
            let strings: Option<Vec<String>> = items.iter().map(|i| i.str().map(scala_string)).collect();
            ("scala.collection.immutable.Seq[String]".to_string(), format!("scala.collection.immutable.Seq({})", strings?.join(", ")))
        }
        _ => return None,
    })
}

/// A string literal as sbt-buildinfo's `encodeStringLiteral` writes it.
fn scala_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\u{8}' => out.push_str("\\b"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\'' => out.push_str("\\'"),
            '\u{c}' => out.push_str("\\f"),
            '"' => out.push_str("\\\""),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `SOURCE_VERSION` from the client's environment when set, else the commit HEAD names, read
/// from the repository's files (a worktree's `.git` file and the common directory followed,
/// packed refs read).
pub fn git_sha(env: &BTreeMap<String, String>, root: &Path) -> Result<String, String> {
    if let Some(v) = env.get("SOURCE_VERSION").filter(|v| !v.is_empty()) {
        return Ok(v.clone());
    }
    let dot_git = root.ancestors().map(|d| d.join(".git")).find(|g| g.exists()).ok_or_else(|| format!("no git repository holds {} and SOURCE_VERSION is not set", root.display()))?;
    let git_dir = if dot_git.is_file() {
        let text = std::fs::read_to_string(&dot_git).map_err(|e| format!("cannot read {}: {}", dot_git.display(), e))?;
        let target = text.trim().strip_prefix("gitdir:").ok_or_else(|| format!("{} names no gitdir", dot_git.display()))?.trim();
        dot_git.parent().unwrap_or(root).join(target)
    } else {
        dot_git
    };
    let common = match std::fs::read_to_string(git_dir.join("commondir")) {
        Ok(text) => git_dir.join(text.trim()),
        Err(_) => git_dir.clone(),
    };
    let mut head = std::fs::read_to_string(git_dir.join("HEAD")).map_err(|e| format!("cannot read HEAD: {}", e))?.trim().to_string();
    for _ in 0..8 {
        let Some(name) = head.strip_prefix("ref:").map(str::trim) else { break };
        let name = name.to_string();
        let loose = [git_dir.join(&name), common.join(&name)].into_iter().find_map(|f| std::fs::read_to_string(f).ok());
        head = match loose {
            Some(text) => text.trim().to_string(),
            None => {
                let packed = std::fs::read_to_string(common.join("packed-refs")).unwrap_or_default();
                packed.lines().find_map(|l| l.strip_suffix(name.as_str()).map(|sha| sha.trim().to_string())).ok_or_else(|| format!("HEAD names {}, which has no commit", name))?
            }
        };
    }
    if head.len() == 40 && head.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(head)
    } else {
        Err(format!("HEAD names no commit: {}", head))
    }
}

/// A command generator: run when its fingerprint changed or an output its last run made is gone (a
/// file that is not there, a directory that is not there or empty), the fingerprint and the outputs
/// present after the run recorded once it succeeded; before any record, every output is expected.
/// An output the run leaves empty (the generated sources' directory of a command that writes
/// elsewhere) is not waited for.
/// The first word `teq` is `teq`, the binary running (`std::env::current_exe()`): the launchers
/// chose it, the lock's or `TEQ`'s, before the build tool ran, so it is never a `teq` of the
/// `PATH` nor a launcher of the repository.
fn command(export: &Export, key: &Key, index: usize, g: &Value, teq: &Result<PathBuf, String>) -> Result<Vec<String>, String> {
    let run: Vec<&str> = g.get("run").map_or(&[][..], Value::items).iter().filter_map(Value::str).collect();
    let Some((program, args)) = run.split_first() else { return Err("no command to run".to_string()) };
    let cwd = export.path(g.get("cwd").and_then(Value::str).unwrap_or("."));
    let listed: Vec<&str> = g.get("outputs").map_or(&[][..], Value::items).iter().filter_map(Value::str).collect();
    let outputs: Vec<PathBuf> = listed.iter().map(|o| export.path(o)).collect();
    let fingerprint = fingerprint(export, g, &cwd, &run);
    let record = export.path(&format!("target/teq/generators/{}-{}-{}", key.project, key.configuration, index));
    let present = |o: &Path| match std::fs::metadata(o) {
        Ok(meta) if meta.is_dir() => std::fs::read_dir(o).is_ok_and(|mut d| d.next().is_some()),
        Ok(_) => true,
        Err(_) => false,
    };
    let recorded = std::fs::read_to_string(&record).unwrap_or_default();
    let mut lines = recorded.lines();
    let same = lines.next().is_some_and(|f| f == fingerprint);
    let made: Option<Vec<PathBuf>> = lines.next().filter(|l| l.starts_with("made ")).map(|_| lines.map(|l| export.path(l)).collect());
    let gone = match &made {
        Some(made) => made.iter().any(|o| !present(o)),
        None => outputs.iter().any(|o| !present(o)),
    };
    if same && !gone {
        return Ok(Vec::new());
    }
    let before: Vec<(PathBuf, Vec<u8>, SystemTime)> = outputs
        .iter()
        .flat_map(|o| files_under(o))
        .filter_map(|f| {
            let meta = std::fs::metadata(&f).ok()?;
            Some((f.clone(), std::fs::read(&f).ok()?, meta.modified().ok()?))
        })
        .collect();
    let binary = match *program {
        "teq" => teq.clone().map_err(|e| format!("cannot find the running teq binary, which `teq` names: {}", e))?,
        other => resolved(other),
    };
    let output = Command::new(&binary).args(args).current_dir(&cwd).stdin(Stdio::null()).output().map_err(|e| format!("cannot run {}: {}", binary.display(), e))?;
    let printed = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        return Err(format!("`{}` failed ({}): {}", run.join(" "), output.status, printed.trim()));
    }
    for (file, bytes, modified) in before {
        if std::fs::read(&file).is_ok_and(|now| now == bytes) {
            if let Ok(f) = std::fs::File::options().write(true).open(&file) {
                let _ = f.set_modified(modified);
            }
        }
    }
    if let Some(dir) = record.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let made: Vec<&str> = listed.iter().copied().filter(|o| present(&export.path(o))).collect();
    let _ = std::fs::write(&record, format!("{}\nmade {}\n{}\n", fingerprint, made.len(), made.join("\n")));
    Ok(printed.lines().filter(|l| !l.trim().is_empty()).map(|l| format!("{}: {}", key, l)).collect())
}

/// The generator's block, then the path, size and modification time of every file its inputs'
/// globs match and every file of the repository its command line names.
fn fingerprint(export: &Export, g: &Value, cwd: &Path, run: &[&str]) -> String {
    let mut sha = Sha256::new();
    sha.update(super::lock::write(g).as_bytes());
    let mut files: Vec<PathBuf> = g.get("inputs").map_or(&[][..], Value::items).iter().filter_map(Value::str).flat_map(|p| glob(&export.root, p)).collect();
    files.extend(run.iter().map(|a| cwd.join(a)).filter(|f| f.is_file() && f.starts_with(&export.root)));
    files.sort();
    files.dedup();
    for f in files {
        let meta = std::fs::metadata(&f).ok();
        let modified = meta.as_ref().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos());
        sha.update(format!("{}\0{}\0{}\n", f.display(), meta.map_or(0, |m| m.len()), modified).as_bytes());
    }
    sha.hex()
}

/// The files of an output: the file itself, or those under the directory.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    if dir.is_file() {
        return vec![dir.to_path_buf()];
    }
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return out };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().is_ok_and(|k| k.is_dir()) {
            out.extend(files_under(&path));
        } else {
            out.push(path);
        }
    }
    out
}

/// The files under `root` a glob matches: `**` any run of directories (not `.git` or
/// `node_modules`), `*` any run of characters within a name, `?` one.
pub fn glob(root: &Path, pattern: &str) -> Vec<PathBuf> {
    fn walk(dir: &Path, parts: &[&str], out: &mut Vec<PathBuf>) {
        let Some((first, rest)) = parts.split_first() else { return };
        if *first == "**" {
            walk(dir, rest, out);
            let Ok(entries) = std::fs::read_dir(dir) else { return };
            for entry in entries.flatten() {
                let name = entry.file_name();
                if entry.file_type().is_ok_and(|k| k.is_dir()) && name != ".git" && name != "node_modules" {
                    walk(&entry.path(), parts, out);
                }
            }
            return;
        }
        if !first.contains(['*', '?']) {
            let next = dir.join(first);
            if rest.is_empty() {
                if next.is_file() {
                    out.push(next);
                }
            } else if next.is_dir() {
                walk(&next, rest, out);
            }
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let name = entry.file_name();
            if !matches(first.as_bytes(), name.to_string_lossy().as_bytes()) {
                continue;
            }
            let path = entry.path();
            if rest.is_empty() {
                if path.is_file() {
                    out.push(path);
                }
            } else if path.is_dir() {
                walk(&path, rest, out);
            }
        }
    }
    let parts: Vec<&str> = pattern.split('/').filter(|p| !p.is_empty() && *p != ".").collect();
    let mut out = Vec::new();
    walk(root, &parts, &mut out);
    out.sort();
    out.dedup();
    out
}

/// The program a command names. On Windows a bare name is the first file of `PATH` with one of
/// `PATHEXT`'s extensions, as the shell finds it: `npm` is `npm.cmd`, which std runs through
/// `cmd.exe` with that shell's quoting, where a bare name finds an `.exe` alone. Elsewhere, and
/// for a name with an extension or a directory, the name as given.
fn resolved(program: &str) -> PathBuf {
    let path = Path::new(program);
    if !cfg!(windows) || path.extension().is_some() || path.components().count() > 1 {
        return path.to_path_buf();
    }
    let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
    let dirs = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).unwrap_or_default();
    dirs.iter()
        .flat_map(|d| extensions.split(';').filter(|e| !e.is_empty()).map(move |e| d.join(format!("{}{}", program, e.to_ascii_lowercase()))))
        .find(|p| p.is_file())
        .unwrap_or_else(|| path.to_path_buf())
}

fn matches(pattern: &[u8], name: &[u8]) -> bool {
    match (pattern.first(), name.first()) {
        (None, None) => true,
        (Some(b'*'), _) => matches(&pattern[1..], name) || (!name.is_empty() && matches(pattern, &name[1..])),
        (Some(b'?'), Some(_)) => matches(&pattern[1..], &name[1..]),
        (Some(p), Some(n)) if p == n => matches(&pattern[1..], &name[1..]),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("teq-generators-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn globs() {
        let dir = scratch("glob");
        for f in ["a/images/x.svg", "a/images/deep/y.svg", "a/images/deep/z.png", "a/node_modules/n.svg", "labels.txt"] {
            std::fs::create_dir_all(dir.join(f).parent().unwrap()).unwrap();
            std::fs::write(dir.join(f), "").unwrap();
        }
        let names = |p: &str| glob(&dir, p).iter().map(|f| f.strip_prefix(&dir).unwrap().to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/")).collect::<Vec<_>>();
        assert_eq!(names("a/images/**/*.svg"), ["a/images/deep/y.svg", "a/images/x.svg"]);
        assert_eq!(names("**/*.svg"), ["a/images/deep/y.svg", "a/images/x.svg"]);
        assert_eq!(names("labels.txt"), ["labels.txt"]);
        assert_eq!(names("lab?ls.*"), ["labels.txt"]);
        assert!(names("missing/*.txt").is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn head_through_a_worktree_and_packed_refs() {
        let dir = scratch("git");
        let common = dir.join("main/.git");
        std::fs::create_dir_all(common.join("worktrees/wt")).unwrap();
        std::fs::create_dir_all(dir.join("wt/sub")).unwrap();
        std::fs::write(dir.join("wt/.git"), format!("gitdir: {}\n", common.join("worktrees/wt").display())).unwrap();
        std::fs::write(common.join("worktrees/wt/commondir"), "../..\n").unwrap();
        std::fs::write(common.join("worktrees/wt/HEAD"), "ref: refs/heads/feature\n").unwrap();
        let sha = "0123456789abcdef0123456789abcdef01234567";
        std::fs::write(common.join("packed-refs"), format!("# pack-refs with: peeled\n{} refs/heads/feature\n", sha)).unwrap();
        assert_eq!(git_sha(&BTreeMap::new(), &dir.join("wt/sub")).unwrap(), sha);
        let env = BTreeMap::from([("SOURCE_VERSION".to_string(), "from-ci".to_string())]);
        assert_eq!(git_sha(&env, &dir.join("wt/sub")).unwrap(), "from-ci");
        std::fs::create_dir_all(common.join("refs/heads")).unwrap();
        let loose = "fedcba9876543210fedcba9876543210fedcba98";
        std::fs::write(common.join("refs/heads/feature"), format!("{}\n", loose)).unwrap();
        assert_eq!(git_sha(&BTreeMap::new(), &dir.join("wt")).unwrap(), loose);
        let _ = std::fs::remove_dir_all(&dir);
    }

    const BLOCK: &str = r#"  dynamic:
    classes:
      classDirectory:
        configuration: compile
        project: api
    gitSha: gitSha
  kind: buildInfo
  members:
    - name
    - port
    - debug
    - tags
    - odd
    - gitSha
    - classes
  object: BuildInfo
  output: target/teq/api/compile/src_managed/sbt-buildinfo/BuildInfo.scala
  package: util
  static:
    debug: false
    name: api
    odd: "it's \\ a\ttab\r\u0001"
    port: 8080
    tags:
      - a
      - "b\"c"
"#;
    /// A generator's block: the value of `g` in a lock of these lines.
    fn block(lines: &str) -> Value {
        crate::task::lock::parse(&format!("teq: 0.1.2\nformat: 1\nbinaries: {{}}\ng:\n{}", lines)).unwrap().get("g").unwrap().clone()
    }

    fn lock_of(body: &str) -> String {
        format!("teq: 0.1.2\nformat: 1\nbinaries: {{}}\n{}", body)
    }

    const SBT: &str = "// $COVERAGE-OFF$\npackage util\n\n/** This object was generated by sbt-buildinfo. */\ncase object BuildInfo {\n  /** The value is \"api\". */\n  val name: String = \"api\"\n  /** The value is 8080. */\n  val port: scala.Int = 8080\n  /** The value is false. */\n  val debug: scala.Boolean = false\n  /** The value is scala.collection.immutable.Seq(\"a\", \"b\\\"c\"). */\n  val tags: scala.collection.immutable.Seq[String] = scala.collection.immutable.Seq(\"a\", \"b\\\"c\")\n  /** The value is \"it\\'s \\\\ a\\ttab\\r\u{1}\". */\n  val odd: String = \"it\\'s \\\\ a\\ttab\\r\u{1}\"\n  /** The value is \"abc\". */\n  val gitSha: String = \"abc\"\n  /** The value is new java.io.File(\"/out/classes\"). */\n  val classes: java.io.File = new java.io.File(\"/out/classes\")\n  override val toString: String = {\n    \"name: %s, port: %s, debug: %s, tags: %s, odd: %s, gitSha: %s, classes: %s\".format(\n      name, port, debug, tags, odd, gitSha, classes\n    )\n  }\n}\n// $COVERAGE-ON$\n";

    /// sbt-buildinfo 0.13.1's own output for these keys (a scratch build's `buildInfo` and its
    /// export's block), the class directory's path the test's.
    #[test]
    fn build_info_as_sbt_buildinfo_writes_it() {
        let dir = scratch("buildinfo");
        std::fs::write(dir.join("teq.lock"), lock_of("projects:\n  api:\n    configurations:\n      compile: {}\n    platform: jvm\n")).unwrap();
        let export = Export::read(&dir.join("teq.lock")).unwrap();
        let g = block(BLOCK);
        let env = BTreeMap::from([("SOURCE_VERSION".to_string(), "abc".to_string())]);
        let (file, text) = build_info(&export, &g, &env, &|_| PathBuf::from("/out/classes")).unwrap();
        assert_eq!(file, export.root.join("target/teq/api/compile/src_managed/sbt-buildinfo/BuildInfo.scala"));
        let separator = if cfg!(windows) { "\r\n" } else { "\n" };
        assert_eq!(text, SBT.replace('\n', separator));
        // Without the export's order, by name; a number past an Int's range a Long.
        let mut unordered = block(BLOCK);
        if let Value::Map(fields) = &mut unordered {
            fields.retain(|(k, _)| k != "members");
        }
        let (_, text) = build_info(&export, &unordered, &env, &|_| PathBuf::from("/out/classes")).unwrap();
        let vals: Vec<&str> = text.lines().filter_map(|l| l.strip_prefix("  val ")).map(|l| l.split(':').next().unwrap()).collect();
        assert_eq!(vals, ["classes", "debug", "gitSha", "name", "odd", "port", "tags"]);
        let numbers = block("  output: B.scala\n  static: {big: 9999999999, least: -2147483648}\n");
        let text = build_info(&export, &numbers, &env, &|_| PathBuf::new()).unwrap().1;
        assert!(text.contains("  val big: scala.Long = 9999999999L"));
        assert!(text.contains("  val least: scala.Int = -2147483648"));
        let wrong = block("  members:\n    - a\n  output: B.scala\n  static: {b: 1}\n");
        assert!(build_info(&export, &wrong, &env, &|_| PathBuf::new()).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The command is a POSIX shell's script.
    #[cfg(unix)]
    #[test]
    fn a_command_runs_when_its_fingerprint_changes_and_keeps_unchanged_outputs() {
        let dir = scratch("command");
        std::fs::write(dir.join("labels.txt"), "one\n").unwrap();
        std::fs::write(dir.join("gen.sh"), "mkdir -p \"$2\" && tr a-z A-Z < \"$1\" > \"$2/Out.scala\" && echo ran\n").unwrap();
        let generator = "          - cwd: .\n            inputs:\n              - labels.txt\n            kind: command\n            outputs:\n              - target/gen\n            run:\n              - sh\n              - gen.sh\n              - labels.txt\n              - target/gen\n";
        std::fs::write(dir.join("teq.lock"), lock_of(&format!("projects:\n  p:\n    configurations:\n      compile:\n        generators:\n{}    platform: jvm\n", generator))).unwrap();
        let export = Export::read(&dir.join("teq.lock")).unwrap();
        let key = Key::new("p", "compile");
        let g = &export.configuration(&key).unwrap().generators[0];
        let none = Err("no teq here".to_string());
        assert_eq!(command(&export, &key, 0, g, &none).unwrap(), ["p/compile: ran"]);
        let out = dir.join("target/gen/Out.scala");
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "ONE\n");
        assert!(command(&export, &key, 0, g, &none).unwrap().is_empty(), "an unchanged fingerprint runs nothing");
        let old = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
        std::fs::File::options().write(true).open(&out).unwrap().set_modified(old).unwrap();
        // The script changed, its output did not: the output keeps its modification time.
        std::fs::write(dir.join("gen.sh"), "mkdir -p \"$2\" && tr a-z A-Z < \"$1\" > \"$2/Out.scala\" && echo again\n").unwrap();
        assert_eq!(command(&export, &key, 0, g, &none).unwrap(), ["p/compile: again"]);
        assert_eq!(std::fs::metadata(&out).unwrap().modified().unwrap(), old);
        std::fs::remove_dir_all(dir.join("target/gen")).unwrap();
        assert_eq!(command(&export, &key, 0, g, &none).unwrap(), ["p/compile: again"], "a missing output runs it");
        std::fs::write(dir.join("labels.txt"), "three\n").unwrap();
        command(&export, &key, 0, g, &none).unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "THREE\n");
        // A second output directory the script leaves empty is not waited for: the next check runs nothing.
        let generator = generator.replace("              - target/gen\n", "              - target/gen\n              - target/managed\n");
        std::fs::write(dir.join("teq.lock"), lock_of(&format!("projects:\n  p:\n    configurations:\n      compile:\n        generators:\n{}    platform: jvm\n", generator))).unwrap();
        let export = Export::read(&dir.join("teq.lock")).unwrap();
        let g = &export.configuration(&key).unwrap().generators[0];
        assert_eq!(command(&export, &key, 0, g, &none).unwrap(), ["p/compile: again"], "the changed block runs it");
        std::fs::create_dir_all(dir.join("target/managed")).unwrap();
        assert!(command(&export, &key, 0, g, &none).unwrap().is_empty(), "an output left empty by the run does not run it again");
        std::fs::remove_dir_all(dir.join("target/gen")).unwrap();
        assert_eq!(command(&export, &key, 0, g, &none).unwrap(), ["p/compile: again"], "an output the run made, gone, runs it");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The word `teq` runs the binary given for it (in a build, the one running), never one of the
    /// `PATH`; a script the command names is part of its fingerprint, so its edit alone runs it
    /// again; a file it writes besides the generated directory, listed among its outputs, runs it
    /// again when deleted alone, and keeps its modification time when its text comes out the same.
    #[cfg(unix)]
    #[test]
    fn the_word_teq_runs_the_running_binary_and_a_listed_file_output_is_checked() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("teq-word");
        // The stand-in binary: `teq interp <script> -- <app> <out>` writes the object into <out> and the
        // module into <app>, both from the script's text, and says which binary it is.
        let binary = dir.join("bin/teq-binary");
        std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
        std::fs::write(&binary, "#!/bin/sh\nmkdir -p \"$5\" \"$4/assets\" && cp \"$2\" \"$5/Gen.scala\" && cp \"$2\" \"$4/assets/module.js\" && echo ran $1\n").unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::create_dir_all(dir.join("scripts")).unwrap();
        std::fs::write(dir.join("scripts/gen.scala"), "one\n").unwrap();
        let generator = "          - cwd: .\n            kind: command\n            outputs:\n              - target/gen\n              - app/assets/module.js\n            run:\n              - teq\n              - interp\n              - scripts/gen.scala\n              - \"--\"\n              - app\n              - target/gen\n";
        std::fs::write(dir.join("teq.lock"), lock_of(&format!("projects:\n  p:\n    configurations:\n      compile:\n        generators:\n{}    platform: jvm\n", generator))).unwrap();
        let export = Export::read(&dir.join("teq.lock")).unwrap();
        let key = Key::new("p", "compile");
        let g = &export.configuration(&key).unwrap().generators[0];
        let teq = Ok(binary.clone());
        let missing = command(&export, &key, 0, g, &Err("gone".to_string())).unwrap_err();
        assert!(missing.contains("cannot find the running teq binary") && missing.contains("gone"), "{}", missing);
        assert_eq!(command(&export, &key, 0, g, &teq).unwrap(), ["p/compile: ran interp"]);
        let module = dir.join("app/assets/module.js");
        assert_eq!(std::fs::read_to_string(&module).unwrap(), "one\n");
        assert!(command(&export, &key, 0, g, &teq).unwrap().is_empty(), "an unchanged fingerprint runs nothing");
        std::fs::remove_file(&module).unwrap();
        assert_eq!(command(&export, &key, 0, g, &teq).unwrap(), ["p/compile: ran interp"], "a listed file deleted alone runs it");
        assert!(module.is_file());
        let old = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
        std::fs::File::options().write(true).open(&module).unwrap().set_modified(old).unwrap();
        // The script's modification alone: run again, the module's text the same and its time kept.
        std::fs::File::options().write(true).open(dir.join("scripts/gen.scala")).unwrap().set_modified(old).unwrap();
        assert_eq!(command(&export, &key, 0, g, &teq).unwrap(), ["p/compile: ran interp"], "the script's edit runs it");
        assert_eq!(std::fs::metadata(&module).unwrap().modified().unwrap(), old);
        std::fs::write(dir.join("scripts/gen.scala"), "two\n").unwrap();
        assert_eq!(command(&export, &key, 0, g, &teq).unwrap(), ["p/compile: ran interp"]);
        assert_eq!(std::fs::read_to_string(&module).unwrap(), "two\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
