//! A script's `//> using dep` under `teq interp`, resolved through the lock of the project the script
//! is in: teq has no resolver, the build's export is the resolution (docs/CLI.md, "teq interp").
//!
//! The script's sources (its own and those its `//> using file` lines include) belong to the build's
//! JVM configurations whose source roots hold them; the configuration that owns one of them and whose
//! closure owns them all is the script's, unique or named with `--module`. Its classpath's jars, in the
//! export's order, are the script's class path, fetched as the build's verbs fetch them; the directives
//! name no jar of their own but are checked against that classpath, each coordinate (`org::name:version`,
//! `::` the Scala binary suffix, `:::` the full version) found there at the version asked, or at the one
//! the export records the build's resolution chose in its place (`evictions`). A coordinate the lock does
//! not hold, a version it does not, an owner it cannot tell and a script outside any build are refused,
//! naming what to do. With `--classpath`, inside a project (a configuration owning the sources, or
//! `--module`) the lock validates every coordinate as without it, and the classpath supplies the jars alone;
//! outside one each coordinate is checked against the given jars by artifact and version (`jar_is`), one no
//! jar is refused alike. A lock's coordinates are the JVM's, whatever platform `//> using platform` names:
//! the interpreter runs a build's JVM artifacts. The given jars' are the run's platform's, Scala.js's
//! where the classpath holds Scala.js artifacts (`_sjs1_3`, as Scala CLI names them for Scala.js), the
//! JVM's otherwise.

use crate::task::export::{Entry, Export, Key, Platform};
use std::path::{Path, PathBuf};

/// A `//> using dep` of a program file, or another key of the dependencies a run's classpath holds: where it
/// stands, its key as written and the coordinate.
struct Directive {
    file: String,
    line: usize,
    key: String,
    coordinate: String,
}

/// A coordinate as the lock keys jars: `organization:name:version[:classifier]`, the JVM's artifact.
fn lock_key(coordinate: &str, scala: &str) -> Result<(String, String, String), String> {
    platform_key(coordinate, scala, "")
}

/// A coordinate's key for the artifact of a platform, `platform` the suffix before the Scala version that
/// Scala CLI's dependency names give a Scala artifact on it (`sjs1_` for Scala.js, nothing on the JVM).
fn platform_key(coordinate: &str, scala: &str, platform: &str) -> Result<(String, String, String), String> {
    // The coordinate's parameters, as Scala CLI's dependency parser keeps them: its classifier, which the
    // lock's key holds; any other the lock cannot tell, and so cannot validate.
    let (main, params) = coordinate.split_once(',').unwrap_or((coordinate, ""));
    let mut classifier = None;
    for p in params.split(',').filter(|_| !params.is_empty()) {
        match p.strip_prefix("classifier=") {
            Some(c) if !c.is_empty() => classifier = Some(c),
            _ => return Err(format!("the parameter `{}` is none the lock can check, as it records a coordinate's classifier alone (`,classifier=<name>`): leave it out", p)),
        }
    }
    let (org, rest, suffix) = if let Some((org, rest)) = main.split_once(":::") {
        (org, rest, format!("_{}{}", platform, scala))
    } else if let Some((org, rest)) = main.split_once("::") {
        (org, rest, format!("_{}{}", platform, scala.split('.').next().unwrap_or("3")))
    } else if let Some((org, rest)) = main.split_once(':') {
        (org, rest, String::new())
    } else {
        return Err(format!("`{}` is no coordinate (organization::name:version)", coordinate));
    };
    // `name::version` names a platform's artifact, the Scala suffix's platform part.
    let (name, version) = match rest.split_once("::").or_else(|| rest.split_once(':')) {
        Some((n, v)) if !n.is_empty() && !v.is_empty() && !v.contains(':') => (n, v),
        _ => return Err(format!("`{}` is no coordinate (organization::name:version)", coordinate)),
    };
    let name = format!("{}{}", name, suffix);
    let key = match classifier {
        Some(c) => format!("{}:{}:{}:{}", org, name, version, c),
        None => format!("{}:{}:{}", org, name, version),
    };
    Ok((format!("{}:{}", org, name), version.to_string(), key))
}

/// The jar keys of a configuration's classpath.
fn artifact_keys(export: &Export, key: &Key) -> Vec<String> {
    export.configuration(key).map_or(Vec::new(), |c| c.classpath.iter().filter_map(|e| if let Entry::Artifact(k) = e { Some(k.clone()) } else { None }).collect())
}

/// The organization and name of a jar key, without its version (and with its classifier).
fn module_of(key: &str) -> String {
    let parts: Vec<&str> = key.split(':').collect();
    match parts.as_slice() {
        [org, name, _version, classifier] => format!("{}:{}::{}", org, name, classifier),
        [org, name, ..] => format!("{}:{}", org, name),
        _ => key.to_string(),
    }
}

/// The configuration the script's sources belong to: `--module` (`project` or `project/configuration`),
/// else the one JVM configuration that owns one of the sources directly and whose closure owns them all.
fn owner(export: &Export, sources: &[PathBuf], module: Option<&str>, script: &str) -> Result<Key, String> {
    if let Some(m) = module {
        let key = match m.split_once('/') {
            Some((p, c)) => Key::new(p, c),
            None => Key::new(m, "compile"),
        };
        return match export.configuration(&key) {
            Some(_) => Ok(key),
            None => Err(format!("--module {}: {} has no configuration {}", m, export.file.display(), key)),
        };
    }
    let owned: Vec<(PathBuf, Vec<Key>)> = sources.iter().map(|f| (f.clone(), export.owners(f))).filter(|(_, o)| !o.is_empty()).collect();
    if owned.is_empty() {
        return Err(format!(
            "{}: its `//> using dep` lines resolve through the configuration of {} whose sources the script includes, and it includes none: name one with --module <project>",
            script,
            export.file.display()
        ));
    }
    let jvm = |k: &Key| export.projects.get(&k.project).is_some_and(|p| p.platform == Platform::Jvm);
    let mut candidates: Vec<Key> = owned.iter().flat_map(|(_, o)| o.iter().cloned()).filter(|k| jvm(k) && k.configuration == "compile").collect();
    candidates.sort();
    candidates.dedup();
    candidates.retain(|k| {
        let closure = export.closure(k);
        owned.iter().all(|(_, o)| o.iter().any(|ok| closure.contains(ok)))
    });
    match candidates.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(format!(
            "{}: no JVM configuration of {} holds every source the script includes ({}): name one with --module <project>",
            script,
            export.file.display(),
            owned.iter().map(|(f, _)| f.display().to_string()).collect::<Vec<_>>().join(", ")
        )),
        many => Err(format!(
            "{}: the sources the script includes belong to {} of {}: name one with --module <project>",
            script,
            many.iter().map(|k| k.to_string()).collect::<Vec<_>>().join(" and "),
            export.file.display()
        )),
    }
}

/// Whether the jar at `path` is the artifact `name` at `version` (with `classifier`): its file named
/// `<name>-<version>[-<classifier>].jar`, as Maven's and coursier's caches and ivy's cache name it, or
/// under `<name>/<version>/` with its file named after the artifact, as ivy's local repository lays it
/// out (`<name>/<version>/jars/<name>.jar`).
fn jar_is(path: &str, name: &str, version: &str, classifier: Option<&str>) -> bool {
    let segments: Vec<&str> = path.split(['/', '\\']).filter(|s| !s.is_empty()).collect();
    let Some(file) = segments.last() else { return false };
    let suffix = classifier.map_or(String::new(), |c| format!("-{}", c));
    if *file == format!("{}-{}{}.jar", name, version, suffix) {
        return true;
    }
    *file == format!("{}{}.jar", name, suffix) && segments.windows(2).any(|w| w[0] == name && w[1] == version)
}

/// Outside a project: each coordinate one of the jars `--classpath` names, by its artifact for the run's
/// platform and its version: Scala.js's where the jars hold Scala.js artifacts (`crate::scalajs_jar`, which
/// gives the run the Scala.js layer), the JVM's otherwise. `instead` is what else would resolve it, after the
/// refusal's "add its jar to --classpath".
fn check_given(directives: &[Directive], jars: &[String], instead: &str) -> Result<(), String> {
    let platform = if crate::scalajs_jar(jars) { "sjs1_" } else { "" };
    for d in directives {
        let at = format!("{}:{}: using {} {}", d.file, d.line, d.key, d.coordinate);
        let (_, version, key) = platform_key(&d.coordinate, "3.8.4", platform).map_err(|e| format!("{}: {}", at, e))?;
        let parts: Vec<&str> = key.split(':').collect();
        let (name, classifier) = (parts[1], parts.get(3).copied());
        if !jars.iter().any(|j| jar_is(j, name, &version, classifier)) {
            let suffix = classifier.map_or(String::new(), |c| format!("-{}", c));
            return Err(format!(
                "{}: no jar --classpath names is {} (a {}-{}{}.jar, or a {}{}.jar under {}/{}/): add its jar to --classpath{}",
                at, key, name, version, suffix, name, suffix, name, version, instead
            ));
        }
    }
    Ok(())
}

/// What a script's `//> using dep` lines add to its run: the jars of its configuration's classpath, the
/// source files of the configurations whose products that classpath holds (`product_sources`), and the source
/// version of each of those configurations' roots (`product_source_versions`).
#[derive(Debug, Default, PartialEq)]
pub struct Resolved {
    pub jars: Vec<String>,
    pub sources: Vec<String>,
    pub source_at: Vec<(PathBuf, crate::source_version::SourceVersion)>,
}

/// What the script's `//> using dep` lines add: nothing without one; with them, the classpath of the
/// configuration of the build's lock that owns the script's sources, every coordinate checked against
/// it, and its product closure's inputs; with `given` (`--classpath`'s jars) no jar more, the lock checking
/// every coordinate inside the project, its products' inputs added all the same, and those jars outside it.
/// `program` is the run's files, the script first; `module` is `--module`.
pub fn classpath(program: &[(String, String)], module: Option<&str>, given: Option<&[String]>, excludes: &[String], left_out: &crate::Excluded) -> Result<Resolved, String> {
    let mut directives = Vec::new();
    for (path, text) in program {
        for d in crate::directives::using_deps(text) {
            for c in d.coordinates {
                directives.push(Directive { file: path.clone(), line: d.line, key: d.key.clone(), coordinate: c });
            }
        }
    }
    if directives.is_empty() {
        return Ok(Resolved::default());
    }
    let script = &program[0].0;
    let from = Path::new(script).parent().map(Path::to_path_buf).unwrap_or_default();
    let from = crate::source::canonicalize(if from.as_os_str().is_empty() { Path::new(".") } else { &from }).unwrap_or(from);
    let Some(file) = crate::task::export::find(&from) else {
        // Outside every build the jars `--classpath` names validate the coordinates.
        if let Some(jars) = given {
            check_given(&directives, jars, ", or put the script in a build and export it (sbt teqExportAll) for its lock to resolve it")?;
            return Ok(Resolved::default());
        }
        let d = &directives[0];
        return Err(format!(
            "{}:{}: using {} {}: teq resolves a script's dependencies through the lock of the build it is in (teq.lock, or target/teq/teq.lock, at or above {}), and there is none: export the build (sbt teqExportAll) or give the jars with --classpath",
            d.file,
            d.line,
            d.key,
            d.coordinate,
            from.display()
        ));
    };
    let export = Export::read(&file)?;
    let sources: Vec<PathBuf> = program.iter().filter(|(p, _)| !crate::script::is_script(p)).filter_map(|(p, _)| crate::source::canonicalize(Path::new(p)).ok()).collect();
    // No configuration of the build owns a source of the program and no `--module` names one: the script is
    // outside the project, and the jars `--classpath` names validate the coordinates. Sources the build
    // owns put it inside, where an owner it cannot tell is refused with a classpath as without one.
    if let Some(jars) = given.filter(|_| module.is_none() && sources.iter().all(|f| export.owners(f).is_empty())) {
        check_given(&directives, jars, ", or name the configuration with --module <project> for the lock to resolve it")?;
        return Ok(Resolved::default());
    }
    let key = owner(&export, &sources, module, script)?;
    // The product closure's sources run with the script, so their `//> using dep` lines are the run's too and
    // are validated with the script's, the owner chosen from the script's own sources alone.
    let sources = product_sources(&export, &key, script, program, excludes, left_out)?;
    let source_at = product_source_versions(&export, &key)?;
    for path in &sources {
        let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {}", path, e))?;
        // Their directives are read as the program's are, a lexer's error refusing the run.
        for p in crate::directives::problems(&text) {
            if p.error {
                return Err(p.render(path).trim_end().to_string());
            }
            eprint!("{}", p.render(path));
        }
        for d in crate::directives::using_deps(&text) {
            for c in d.coordinates {
                directives.push(Directive { file: path.clone(), line: d.line, key: d.key.clone(), coordinate: c });
            }
        }
    }
    let held = artifact_keys(&export, &key);
    let evictions = export.configuration(&key).map(|c| c.evictions.clone()).unwrap_or_default();
    for d in &directives {
        let at = format!("{}:{}: using {} {}", d.file, d.line, d.key, d.coordinate);
        let (module, version, wanted) = lock_key(&d.coordinate, "3.8.4").map_err(|e| format!("{}: {}", at, e))?;
        if held.contains(&wanted) || evictions.get(&wanted).is_some_and(|chosen| held.contains(chosen)) {
            continue;
        }
        let module_key = module_of(&wanted);
        match held.iter().find(|k| module_of(k) == module_key) {
            Some(resolved) => {
                return Err(format!(
                    "{}: the script asks {} at {}, and {}'s {} resolved {}, which the lock records no eviction of: ask {} or export the build again ({}) for the lock to record it",
                    at,
                    module,
                    version,
                    export.file.display(),
                    key,
                    resolved,
                    resolved,
                    export.exporter.command()
                ))
            }
            None => {
                return Err(format!(
                    "{}: {} is not on the classpath of {}'s {}: add it to the build and export it again ({})",
                    at,
                    wanted,
                    export.file.display(),
                    key,
                    export.exporter.command()
                ))
            }
        }
    }
    // Inside the project the lock validated every coordinate; `--classpath` supplies the jars alone.
    if given.is_some() {
        return Ok(Resolved { jars: Vec::new(), sources, source_at });
    }
    let jars = export.jars_keyed(&key, &|name| eprintln!("teq: fetching {}", name))?;
    let jars: Vec<(String, Option<String>)> = jars.into_iter().map(|(p, k)| (p.to_string_lossy().into_owned(), k)).collect();
    let _ = LOCKED.set(jars.iter().filter_map(|(p, k)| k.clone().map(|k| (p.clone(), k))).collect());
    Ok(Resolved { jars: jars.into_iter().map(|(p, _)| p).collect(), sources, source_at })
}

/// The source version of each root of the configurations whose products the classpath of `key` holds, as the
/// build compiled them and its verbs type them (`Export::root_versions`, which `source_args` gives a session
/// as `--source-at`): each configuration's `-source`, the default where it names none. The roots `key` names
/// itself are left out: the script includes those (`//> using file`) and types them at its own version, as
/// Scala CLI compiles a script's sources with its options.
fn product_source_versions(export: &Export, key: &Key) -> Result<Vec<(PathBuf, crate::source_version::SourceVersion)>, String> {
    let own = export.configuration(key).map_or(&[][..], |c| &c.sources[..]);
    let upstream: Vec<Key> = export.closure(key).into_iter().filter(|k| k != key).collect();
    let mut out = Vec::new();
    for (root, version) in export.root_versions(&upstream).into_iter().filter(|(r, _)| !own.contains(r)) {
        let v = crate::source_version::SourceVersion::setting(&version, crate::source_version::TAG)
            .map_err(|e| format!("{}: the source version of {}: {}", export.file.display(), root, e))?;
        let path = export.path(root);
        out.push((crate::source::canonicalize(&path).unwrap_or(path), v));
    }
    Ok(out)
}

/// The `.scala` files of the product closure's inputs (`product_inputs`), in the order of their paths,
/// hidden directories left out as a directory input's are (`main.rs`'s `collect_files`), each not among
/// the program's files already (a script's `//> using file` names some) nor one `--exclude` leaves out:
/// an excluded file is neither read for its directives nor typed, as Scala CLI's `CrossSources.excludeSources`
/// leaves it out of the inputs on disk before anything reads them. A file the program's inputs reached and the
/// exclusion left out is the same file here, by its identity (`left_out`, `main.rs`'s `file_identity`), however
/// the command spelled its path (`./`, a link). One no input reached is matched by `excludes` itself, its path the
/// lock's made whole, as it stands and as relative to the working directory, bare or `./`-spelled, the spellings a
/// command line gives a path there (`main.rs`'s `excluded`).
fn product_sources(export: &Export, key: &Key, script: &str, program: &[(String, String)], excludes: &[String], left_out: &crate::Excluded) -> Result<Vec<String>, String> {
    fn walk(path: &Path, out: &mut Vec<PathBuf>) {
        if path.is_dir() {
            let Ok(entries) = std::fs::read_dir(path) else { return };
            let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
            entries.sort();
            for e in entries {
                let hidden = e.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'));
                if e.extension().is_some_and(|x| x == "scala") || (!hidden && e.is_dir()) {
                    walk(&e, out);
                }
            }
        } else if path.extension().is_some_and(|x| x == "scala") {
            out.push(path.to_path_buf());
        }
    }
    let mut files = Vec::new();
    for input in product_inputs(export, key, script)? {
        walk(Path::new(&input), &mut files);
    }
    if !excludes.is_empty() {
        let here = std::env::current_dir().ok();
        files.retain(|f| {
            let path = f.to_string_lossy();
            let relative = here.as_deref().and_then(|d| f.strip_prefix(d).ok()).map(|r| r.to_string_lossy().into_owned());
            let named = |r: &String| crate::excluded(excludes, r) || crate::excluded(excludes, &format!("./{}", r));
            !crate::excluded(excludes, &path) && !relative.as_ref().is_some_and(named) && !left_out.contains(&crate::file_identity(&path))
        });
    }
    let identity = |p: &Path| crate::source::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let mut seen: std::collections::HashSet<PathBuf> = program.iter().map(|(p, _)| identity(Path::new(p))).collect();
    Ok(files.into_iter().filter(|f| seen.insert(identity(f))).map(|f| f.to_string_lossy().into_owned()).collect())
}

/// The inputs of the configurations whose products the classpath of `key` holds, its product closure
/// (ruling 4), as the build's verbs type them (`Export::session_inputs`: their source roots and stand-ins
/// on disk), since teq interp builds no product and runs their sources with the script's. The closure is
/// one program, as a verb's resident is: refused where its build keeps a configuration from another's
/// sight (`transitive: false`, `Export::one_program_refusals`), and where a product's sources wait for a
/// generator the build tool runs.
fn product_inputs(export: &Export, key: &Key, script: &str) -> Result<Vec<String>, String> {
    let upstream: Vec<Key> = export.closure(key).into_iter().filter(|k| k != key).collect();
    if upstream.is_empty() {
        return Ok(Vec::new());
    }
    let tool = export.exporter.name();
    if let Some(line) = export.one_program_refusals(key, &format!("run the script with {}", tool)).into_iter().next() {
        return Err(format!("{}: {}", script, line));
    }
    for k in &upstream {
        let Some(conf) = export.configuration(k) else { continue };
        if let Some((g, by)) = conf.generators.iter().find_map(|g| crate::task::export::run_by(g).map(|by| (g, by))) {
            let task = g.get("task").and_then(crate::task::lock::Value::str).unwrap_or("an unnamed task");
            return Err(format!(
                "{}: {} builds on {}, whose sources the generator {} completes, which {} runs and teq interp cannot: run the script with {}",
                script, key, k, task, by, by
            ));
        }
    }
    let mut out: Vec<String> = Vec::new();
    for k in &upstream {
        for input in export.session_inputs(k).into_iter().filter(|p| p.exists()) {
            let input = input.to_string_lossy().into_owned();
            if !out.contains(&input) {
                out.push(input);
            }
        }
    }
    Ok(out)
}

/// The lock's coordinates of the jars a script's `using dep` lines resolved to, by their paths.
static LOCKED: std::sync::OnceLock<crate::intern::FxMap<String, String>> = std::sync::OnceLock::new();

/// The coordinate (`organization:name:version`) the lock gives the jar at `path`, where the run's jars
/// are a lock's.
pub fn locked_coordinate(path: &str) -> Option<&'static str> {
    LOCKED.get()?.get(path).map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_take_the_scala_suffix_their_colons_ask() {
        assert_eq!(lock_key("dev.zio::zio:2.1.26", "3.8.4").unwrap().2, "dev.zio:zio_3:2.1.26");
        assert_eq!(lock_key("org.typelevel:::kind-projector:0.13.3", "3.8.4").unwrap().2, "org.typelevel:kind-projector_3.8.4:0.13.3");
        assert_eq!(lock_key("software.amazon.awssdk:s3:2.49.5", "3.8.4").unwrap().2, "software.amazon.awssdk:s3:2.49.5");
        assert_eq!(lock_key("org.scala-js::scalajs-dom::2.8.1", "3.8.4").unwrap().2, "org.scala-js:scalajs-dom_3:2.8.1");
        assert_eq!(lock_key("a:b:1,classifier=natives", "3.8.4").unwrap().2, "a:b:1:natives");
        assert!(lock_key("nonsense", "3.8.4").is_err());
        assert_eq!(module_of("co.fs2:fs2-io_3:3.13.0"), "co.fs2:fs2-io_3");
    }

    #[test]
    fn outside_a_build_the_classpath_jars_validate() {
        let program = |dep: &str| vec![("/teq-no-build/x.scala".to_string(), format!("//> using platform js\n//> using dep {}\n", dep))];
        let given = ["/c/io/github/cquiroz/scala-java-locales_3/1.5.4/scala-java-locales_3-1.5.4.jar".to_string(), "/ivy/local/org.a/b_3/2.0/jars/b_3.jar".to_string()];
        assert_eq!(classpath(&program("io.github.cquiroz::scala-java-locales::1.5.4"), None, Some(given.as_slice()), &[], &Default::default()), Ok(Resolved::default()));
        assert_eq!(classpath(&program("org.a::b:2.0"), None, Some(given.as_slice()), &[], &Default::default()), Ok(Resolved::default()));
        assert!(classpath(&program("io.github.cquiroz::scala-java-locales::1.5.3"), None, Some(given.as_slice()), &[], &Default::default()).is_err());
        assert!(classpath(&program("not.present::missing:99"), None, Some(&[] as &[String]), &[], &Default::default()).is_err());
        // On Scala.js artifacts the run is Scala.js's, and a coordinate names its Scala.js artifact.
        let sjs = ["/c/org/typelevel/cats-core_sjs1_3/2.13.0/cats-core_sjs1_3-2.13.0.jar".to_string(), "/c/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar".to_string()];
        assert_eq!(classpath(&program("org.typelevel::cats-core:2.13.0"), None, Some(sjs.as_slice()), &[], &Default::default()), Ok(Resolved::default()));
        assert_eq!(classpath(&program("org.scala-lang:scala-library:3.8.4"), None, Some(sjs.as_slice()), &[], &Default::default()), Ok(Resolved::default()));
        assert!(classpath(&program("org.typelevel::cats-core:2.12.0"), None, Some(sjs.as_slice()), &[], &Default::default()).is_err());
        assert!(classpath(&program("org.typelevel::cats-kernel:2.13.0"), None, Some(sjs.as_slice()), &[], &Default::default()).is_err());
        assert_eq!(platform_key("org.typelevel:::kind-projector:0.13.3", "3.8.4", "sjs1_").unwrap().2, "org.typelevel:kind-projector_sjs1_3.8.4:0.13.3");
        assert!(jar_is("/m/a_3-1.0-natives.jar", "a_3", "1.0", Some("natives")));
        assert!(!jar_is("/m/a_3-1.0.jar", "a_3", "1.0", Some("natives")));
    }
}
