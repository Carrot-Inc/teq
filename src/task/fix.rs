//! `teq fix [--check] [--rules A,B] [--config <file>] [<module>...] [<file>...]` (docs/CLI.md, "teq fix"): the
//! rules of `.scalafix.conf` over the lock's modules as `teq compile` reads them. Each project is typed under the keys
//! `teq compile` types it under (`Export::widest`, and a configuration of its own where that key's closure holds
//! none of its roots), each key's closure as one program with the resident's typing arguments
//! (`Export::typing_args_with`), its generators run first under compile's policy (`generators::run`); only the
//! selected projects' own source roots are edited, never a dependency's or a generated root.

use super::export::{Export, Key};
use crate::fix::engine::{self, Job, Setup};
use std::path::{Path, PathBuf};

pub fn run(export: &Export, args: &[String]) -> i32 {
    let (check, rules, config, rest) = match crate::fix::cli::flags(args) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("teq fix: {}", e);
            return 2;
        }
    };
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    // An argument that names a file, or has a path's shape, is a file; any other a module.
    let (mut files, mut modules): (Vec<PathBuf>, Vec<String>) = (Vec::new(), Vec::new());
    for a in &rest {
        if is_file_argument(a) {
            match crate::source::canonicalize(cwd.join(a)) {
                Ok(p) => files.push(p),
                Err(_) => {
                    eprintln!("teq fix: no file {}", a);
                    return 2;
                }
            }
        } else {
            modules.push(a.clone());
        }
    }
    // The modules: `project` or `project/configuration`.
    let mut selected: Vec<(String, Option<String>)> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();
    for m in &modules {
        let (project, configuration) = match m.split_once('/') {
            Some((p, c)) => (p.to_string(), Some(c.to_string())),
            None => (m.clone(), None),
        };
        match export.projects.get(&project) {
            None => unknown.push(format!("no project {}", project)),
            Some(p) => match &configuration {
                Some(c) if !p.configurations.contains_key(c) => unknown.push(format!("{} has no configuration {}", project, c)),
                _ => selected.push((project, configuration)),
            },
        }
    }
    if !unknown.is_empty() {
        let projects: Vec<&str> = export.projects.keys().map(String::as_str).collect();
        eprintln!("teq fix: {} (projects: {})", unknown.join(", "), projects.join(", "));
        return 2;
    }
    if selected.is_empty() {
        selected = export.projects.iter().filter(|(_, p)| p.configurations.values().any(|c| !c.sources.is_empty())).map(|(n, _)| (n.clone(), None)).collect();
    }
    let names: Vec<&str> = selected.iter().map(|(p, _)| p.as_str()).collect();
    if let Some(why) = super::refused(export, "compile", &names) {
        eprintln!("teq fix: {}", why);
        return 2;
    }
    // Each project's keys, and the roots of its own each key edits.
    let mut jobs: Vec<(Key, Vec<String>, Vec<PathBuf>)> = Vec::new();
    for (project, configuration) in &selected {
        let p = &export.projects[project];
        let wanted: Vec<&String> = match configuration {
            Some(c) => vec![c],
            None => p.configurations.iter().filter(|(_, c)| !c.sources.is_empty()).map(|(n, _)| n).collect(),
        };
        let mut keys: Vec<Key> = Vec::new();
        if configuration.is_none() {
            if let Some(w) = export.widest(project) {
                keys.push(w);
            }
        }
        for c in &wanted {
            let roots = own_roots(export, project, c);
            let covered = keys.iter().any(|k| {
                let typed = export.roots(&export.closure(k));
                roots.iter().all(|r| typed.contains(r))
            });
            if !covered {
                keys.push(Key::new(project, c));
            }
        }
        for k in keys {
            let typed = export.roots(&export.closure(&k));
            let own: Vec<String> = wanted.iter().flat_map(|c| own_roots(export, project, c)).filter(|r| typed.contains(r)).collect();
            jobs.push((k, own, generated(export, project)));
        }
    }
    let (conf, config_dir) = match crate::fix::cli::configuration(config.as_deref(), &export.root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("teq fix: {}", e);
            return 2;
        }
    };
    // The run works from the build's root, as a resident does; its paths print from where it was started.
    if let Err(e) = std::env::set_current_dir(&export.root) {
        eprintln!("teq fix: cannot work from {}: {}", export.root.display(), e);
        return 2;
    }
    let env = std::env::vars().filter(|(k, _)| k == "SOURCE_VERSION").collect();
    let mut engine_jobs = Vec::new();
    for (key, own, generated) in &jobs {
        let closure = export.closure(key);
        match super::generators::run(export, &closure, export.path(&export.classes(key)), &env) {
            Ok(notes) => {
                for n in notes {
                    eprintln!("{}", n);
                }
            }
            Err(e) => {
                eprintln!("teq fix: {}: {}", key, e);
                return 2;
            }
        }
        let typing = match export.typing_args_with(key, &|_| {}) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("teq fix: {}: {}", key, e);
                return 2;
            }
        };
        let mut args = typing.head;
        if typing.jvm {
            args.extend(["--target", "jvm", "--std=scala-library"].map(str::to_string));
        }
        args.extend(typing.tail);
        let roots: Vec<String> = own.iter().map(|r| export.relative(&export.path(r))).collect();
        let generated = generated.iter().map(|g| export.relative(g)).collect();
        engine_jobs.push(Job { args, roots: Some(roots), files: (!files.is_empty()).then(|| files.clone()), generated });
    }
    // A file argument names an input of the selection: a source its projects' own roots hold that the program reads
    // (their exclusions applied, what a generator writes not), not merely a path under a root.
    if !files.is_empty() {
        let inputs: Vec<PathBuf> = engine_jobs
            .iter()
            .flat_map(|j| engine::program_paths(j).into_iter().filter(|p| j.owns(p)))
            .filter_map(|p| crate::source::canonicalize(Path::new(&p)).ok())
            .collect();
        for f in &files {
            if !inputs.contains(f) {
                let at = crate::source::canonicalize(&cwd).ok().and_then(|c| f.strip_prefix(c).ok().map(Path::to_path_buf)).unwrap_or_else(|| f.clone());
                eprintln!("teq fix: {} is no source of the projects selected (not among the inputs they compile)", at.display());
                return 2;
            }
        }
    }
    let display = (export.root != cwd).then(|| (export.root.clone(), cwd.clone()));
    engine::run(Setup { conf, config_dir, rules, check, display }, engine_jobs)
}

/// The source roots of a project's configuration that are its own: its `sources` less what its generators write.
fn own_roots(export: &Export, project: &str, configuration: &str) -> Vec<String> {
    let Some(c) = export.projects.get(project).and_then(|p| p.configurations.get(configuration)) else { return Vec::new() };
    let generated = generated(export, project);
    c.sources.iter().filter(|s| !generated.iter().any(|g| export.path(s).starts_with(g))).cloned().collect()
}

/// What the generators of a project's configurations write, files or directories: never a root of its own, and,
/// beneath one, typed but no edit's and no named file's (docs/CLI.md).
fn generated(export: &Export, project: &str) -> Vec<PathBuf> {
    let Some(p) = export.projects.get(project) else { return Vec::new() };
    p.configurations
        .values()
        .flat_map(|c| c.generators.iter())
        .flat_map(|g| {
            let one = g.get("output").and_then(|v| v.str()).map(|s| vec![s.to_string()]).unwrap_or_default();
            let many: Vec<String> = g.get("outputs").map_or(&[][..], |v| v.items()).iter().filter_map(|v| v.str().map(str::to_string)).collect();
            one.into_iter().chain(many)
        })
        .map(|o| export.path(&o))
        .collect()
}

/// Whether a command-line argument names a file: one that exists, or has a path's shape (a separator, a drive
/// letter, a `.scala` or `.java` suffix), the rest being modules.
fn is_file_argument(a: &str) -> bool {
    let drive = a.len() >= 2 && a.as_bytes()[1] == b':' && a.as_bytes()[0].is_ascii_alphabetic();
    Path::new(a).is_file() || a.contains('\\') || drive || a.ends_with(".scala") || a.ends_with(".java") || (a.contains('/') && Path::new(a).exists())
}
