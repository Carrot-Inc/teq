//! The projects of a workspace: the configurations of the export `teq.lock` at or above a
//! root (docs/TARGETS.md, "The workspace"), each a session opened when a file of its own source
//! roots is, or without an export one session over every `.scala` file of the root.

use crate::task::export::{Export, Key, Platform};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A session's program as `teq compiler watch --check` takes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Description {
    /// The directory the session runs in.
    pub root: PathBuf,
    /// Source directories and files, absolute.
    pub inputs: Vec<PathBuf>,
    pub excludes: Vec<PathBuf>,
    pub classpath: Vec<PathBuf>,
    /// `--target jvm --std=<std>`, for a JVM description.
    pub jvm_std: Option<String>,
    pub flags: Vec<String>,
}

impl Description {
    /// The command line of the session's child after `teq`.
    pub fn args(&self, positions: &str) -> Vec<String> {
        let mut args = vec!["compiler".to_string(), "watch".to_string(), "--check".to_string(), "--index".to_string(), "--wait-for-build".to_string(), "--positions".to_string(), positions.to_string()];
        args.extend(self.inputs.iter().map(|p| p.to_string_lossy().into_owned()));
        for x in &self.excludes {
            args.push("--exclude".to_string());
            args.push(x.to_string_lossy().into_owned());
        }
        if !self.classpath.is_empty() {
            args.push("--classpath".to_string());
            args.push(crate::task::export::join_paths(&self.classpath));
        }
        if let Some(std) = &self.jvm_std {
            args.extend(["--target".to_string(), "jvm".to_string(), format!("--std={}", std)]);
        }
        args.extend(self.flags.iter().cloned());
        args
    }
}

pub fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// The session of a configuration: the project's stand-ins and its closure's source roots, the
/// project's excludes, the configuration's libraries (resolved, fetched when missing), and the
/// typer's settings of its flags and its project's description.
pub fn description(export: &Export, key: &Key) -> Result<Description, String> {
    let project = export.projects.get(&key.project).ok_or_else(|| format!("no project {} in {}", key.project, export.file.display()))?;
    let conf = project.configurations.get(&key.configuration).ok_or_else(|| format!("{} has no configuration {}", key.project, key.configuration))?;
    let jvm = project.platform == Platform::Jvm;
    let mut flags = conf.flags.args(jvm);
    flags.extend(project.description.session_args());
    Ok(Description {
        root: export.root.clone(),
        inputs: export.session_inputs(key),
        excludes: project.description.excludes.iter().map(|x| export.path(x)).collect(),
        // The artifacts fetched for it, each told to the client's log as its transfer starts.
        classpath: export.jars_with(key, &|line| super::notify("window/logMessage", crate::lsp::json::obj([("type", 4u32.into()), ("message", line.into())])))?,
        jvm_std: jvm.then(|| "scala-library".to_string()),
        flags,
    })
}

/// One session over every `.scala` file of a root without an export, on the lean std.
pub fn bare(root: &Path) -> Description {
    Description { root: root.to_path_buf(), inputs: vec![root.to_path_buf()], excludes: Vec::new(), classpath: Vec::new(), jvm_std: None, flags: Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sessions_of_an_export() {
        let dir = std::env::temp_dir().join(format!("teq-workspace-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        std::fs::write(dir.join("lib/x_sjs1_3-1.0.jar"), "").unwrap();
        std::fs::write(dir.join("lib/x_3-1.0.jar"), "").unwrap();
        let text = "teq: 0.1.2
format: 1
binaries: {}
projects:
  app:
    configurations:
      compile:
        classpath:
          - {configuration: compile, project: core}
          - {file: lib/x_3-1.0.jar}
        sources:
          - app/src
      test:
        classpath:
          - {configuration: compile, project: app}
          - {file: lib/x_3-1.0.jar}
        sources:
          - app/test
    description:
      lib: app/stand-ins
    platform: jvm
  core:
    configurations:
      compile:
        classpath:
          - {file: lib/x_sjs1_3-1.0.jar}
          - {file: lib/x_3-1.0.jar}
        flags:
          kindProjector: true
          maxInlines: 32
        sources:
          - core/src
    description:
      cacheableState:
        - core.Cache
      excludes:
        - core/src/Skip.scala
    platform: js
";
        std::fs::write(dir.join("teq.lock"), text).unwrap();
        let export = Export::read(&dir.join("teq.lock")).unwrap();
        let core = description(&export, &Key::new("core", "compile")).unwrap();
        assert_eq!(core.inputs, [export.path("core/src")]);
        assert_eq!(core.excludes, [export.path("core/src/Skip.scala")]);
        assert_eq!(core.classpath, [export.path("lib/x_sjs1_3-1.0.jar")]);
        assert_eq!(core.jvm_std, None);
        assert_eq!(core.flags, ["--max-inlines", "32", "--kind-projector", "--cacheable-state", "core.Cache"]);
        let app = description(&export, &Key::new("app", "test")).unwrap();
        assert_eq!(app.inputs, [export.path("app/stand-ins"), export.path("core/src"), export.path("app/src"), export.path("app/test")]);
        assert_eq!(app.classpath, [export.path("lib/x_3-1.0.jar")]);
        assert!(app.args("utf-16").windows(3).any(|w| w == ["--target", "jvm", "--std=scala-library"]));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
