//! The export, `teq.lock` (docs/TARGETS.md, "The export and the project verbs"): at the build's root, committed, under
//! the native driver, else under the root's `target/teq/`; read, checked against the format this
//! binary knows and against the build definition files it was made from, and turned into the
//! residents' command lines. Every path in it is relative to the build's root; what is
//! machine-specific (the jars' places) is resolved here at run time. What the export records as
//! sbt's alone (a generator of another kind, a test option, a stage it does not reproduce) is
//! refused by the verbs that need it (`refusals`).

use super::fetch;
use super::lock::{self, Value};
use super::sha256;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

pub const FILE: &str = lock::FILE;
/// Where the export lies without the native driver, relative to the build's root.
pub const UNDER_TARGET: &str = "target/teq/teq.lock";

pub struct Export {
    /// The file, canonical.
    pub file: PathBuf,
    /// The build's root: the file's directory, or the one whose `target/teq/` holds it.
    pub root: PathBuf,
    /// The digest of the file's bytes, which the daemon's handshake compares.
    pub sha256: String,
    pub projects: BTreeMap<String, Project>,
    pub repositories: BTreeMap<String, Repository>,
    /// The jar table: every jar a classpath names, by its key.
    pub table: BTreeMap<String, Jar>,
    /// The build definition files and the digests of their contents, as the export recorded them.
    pub inputs: BTreeMap<String, String>,
    /// The compiler the export pins, its header: the daemon exits when it changes.
    pub teq: Pinned,
    /// The class files' version the build compiles for: the least Java a test runner may run on.
    pub java_output_version: Option<u32>,
}

pub struct Repository {
    pub url: String,
    pub credentials: Option<String>,
}

/// The compiler a lock pins: its version and, per classifier, its binary's fields as written
/// (`<url> <sha1> <size>`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Pinned {
    pub version: String,
    pub binaries: Vec<(String, String)>,
}

/// A jar of the table: the repository it comes from, its path there (the Maven layout of its key
/// unless the export names another) and what the export pins of it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Jar {
    pub repository: String,
    pub path: String,
    pub sha1: String,
    pub size: u64,
}

pub struct Project {
    pub base: String,
    pub platform: Platform,
    pub configurations: BTreeMap<String, Configuration>,
    pub description: Description,
    pub dev: Option<Dev>,
    pub run: Option<RunContext>,
    pub stage: Option<Stage>,
    /// What teq cannot reproduce of the project but its generators, by the verb it keeps
    /// from running (`test`, `stage`), as the export says it.
    pub unsupported: BTreeMap<String, Vec<String>>,
}

/// How a JVM project's main classes run: the JVM's directory, options and environment, the
/// aliases of its entry points, and the main class the build declares for sbt's `run`.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct RunContext {
    pub base_directory: String,
    pub env_vars: BTreeMap<String, String>,
    pub java_options: Vec<String>,
    pub aliases: BTreeMap<String, String>,
    /// `Compile / run / mainClass`, else `Compile / mainClass`, when the build sets one; none in
    /// an export without `mainClass`, which an older plugin wrote, as when the build declares
    /// none: the products decide.
    pub main_class: Option<String>,
}

/// sbt-native-packager's `Docker / stage` of a project: the files of each layer under
/// `directory`, by their paths in the image's tree.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Stage {
    pub name: String,
    pub main_class: MainClass,
    pub exposed_ports: Vec<u32>,
    pub directory: String,
    pub layers: BTreeMap<u32, Vec<Mapping>>,
}

/// What the build says of `Compile / mainClass` for the stage: the start script's class and the
/// project's own jar's `Main-Class`, as sbt's `packageBin` writes it from that setting alone.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MainClass {
    /// The class the build declares (the block's `mainClass`): the script's and the manifest's.
    Declared(String),
    /// The build sets `Compile / mainClass := None` (the block's `mainClassNone`): the script runs
    /// the class the products imply, as native-packager's does, and the manifest keeps no
    /// `Main-Class`, as sbt leaves it.
    DeclaredNone,
    /// The build leaves the setting alone (neither field): the products imply the class for the
    /// script and the manifest, as sbt's `mainClass` picks the one a compile discovers.
    Unset,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Mapping {
    pub from: Staged,
    pub to: String,
}

/// What a file of the stage holds.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Staged {
    /// The jar `packageBin` makes of a configuration's products, with its manifest's attributes.
    Product { key: Key, manifest: Vec<(String, String)> },
    /// A jar of the project's runtime classpath, by its key.
    Artifact(String),
    /// A file of the repository's tree.
    File(String),
    /// The start script, its classpath relative to the install location.
    Script { classpath: Vec<String> },
}

/// A Scala.js project's dev loop: the package manager, the command, and the lockfile, whose
/// directory is the `package.json`'s, where both run.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Dev {
    pub package_manager: String,
    pub command: Vec<String>,
    pub lockfile: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Platform {
    Jvm,
    Js,
}

pub struct Configuration {
    pub sources: Vec<String>,
    pub resources: Vec<String>,
    pub classpath: Vec<Entry>,
    pub flags: Flags,
    pub generators: Vec<Value>,
    /// The resource generators sbt runs, which teq cannot (each of kind `sbt`).
    pub resource_generators: Vec<Value>,
    /// The main classes the build declares: `Compile / mainClass`, the aliases' targets and
    /// `teqMainClasses`, sorted.
    pub main_classes: Vec<String>,
    /// The test configuration's execution context.
    pub test: Option<TestContext>,
}

/// How sbt runs a test configuration's suites: its JVM's directory, options and environment, the
/// frameworks with their `Tests.Argument`s, and the suites a `Tests.Exclude` names.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct TestContext {
    pub fork: bool,
    pub base_directory: String,
    pub env_vars: BTreeMap<String, String>,
    pub java_options: Vec<String>,
    pub frameworks: Vec<Framework>,
    pub exclude: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Framework {
    pub class: String,
    pub arguments: Vec<String>,
}

/// An entry of a classpath: the products of a project's configuration, a jar of the table, or a
/// file of the repository's own tree.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Entry {
    Product(Key),
    /// The jar's key, `organization:name:version`, and `:classifier` when the artifact has one.
    Artifact(String),
    File(String),
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Flags {
    pub max_inlines: Option<u32>,
    pub strict_equality: bool,
    pub kind_projector: bool,
    pub werror: bool,
    /// `wunusedImports`, scalac's `-Wunused:imports` (or `all`): `--wunused imports`.
    pub wunused_imports: bool,
    pub java_output_version: Option<u32>,
}

impl Flags {
    /// The teq flags; the class files' version is a JVM build's alone.
    pub fn args(&self, jvm: bool) -> Vec<String> {
        let mut args = Vec::new();
        if let Some(n) = self.max_inlines {
            args.extend(["--max-inlines".to_string(), n.to_string()]);
        }
        for (set, flag) in [(self.strict_equality, "--strict-equality"), (self.kind_projector, "--kind-projector"), (self.werror, "--werror")] {
            if set {
                args.push(flag.to_string());
            }
        }
        if self.wunused_imports {
            args.extend(["--wunused".to_string(), "imports".to_string()]);
        }
        if let (true, Some(n)) = (jvm, self.java_output_version) {
            args.extend(["--java-output-version".to_string(), n.to_string()]);
        }
        args
    }
}

/// The project's description as sbt-teq's tasks make it, besides its configurations: the
/// template every session of the project takes its excludes, stand-ins and typer's settings from.
#[derive(Clone, Default, Debug)]
pub struct Description {
    pub lib: Option<String>,
    /// The compile sources of the project and the projects it depends on, `teqSources` and
    /// `teqExtraSources` as the build sets them; none when the export does not say.
    pub sources: Option<Vec<String>>,
    pub excludes: Vec<String>,
    pub out: Option<String>,
    pub module_per_file: Vec<String>,
    pub cacheable_state: Vec<String>,
    pub macro_state: Option<String>,
    pub threads: Option<u32>,
    pub hot: bool,
    pub release: bool,
    /// The sources added to the production build, and the paths it leaves out besides.
    pub production: Production,
    pub main_class: Option<String>,
    pub keys: BTreeMap<String, String>,
}

#[derive(Clone, Default, Debug)]
pub struct Production {
    pub sources: Vec<String>,
    pub excludes: Vec<String>,
}

impl Description {
    /// The typer's settings every session of the project takes: the cacheable state, the macro
    /// state, the workers, and the words of a `flags` key.
    pub fn session_args(&self) -> Vec<String> {
        let mut args: Vec<String> = self.cacheable_state.iter().flat_map(|n| ["--cacheable-state".to_string(), n.clone()]).collect();
        if self.macro_state.as_deref() == Some("per-worker") {
            args.extend(["--macro-state".to_string(), "per-worker".to_string()]);
        }
        if let Some(n) = self.threads {
            args.extend(["--threads".to_string(), n.to_string()]);
        }
        if let Some(words) = self.keys.get("flags") {
            args.extend(words.split_whitespace().map(str::to_string));
        }
        args
    }
}

/// A project's configuration.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Key {
    pub project: String,
    pub configuration: String,
}

impl Key {
    pub fn new(project: &str, configuration: &str) -> Key {
        Key { project: project.to_string(), configuration: configuration.to_string() }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.project, self.configuration)
    }
}

/// The export of the build whose root is a directory: its `teq.lock`, else its
/// `target/teq/teq.lock`.
pub fn at(dir: &Path) -> Option<PathBuf> {
    [FILE, UNDER_TARGET].iter().map(|f| dir.join(f)).find(|f| f.is_file())
}

/// The export at or above a directory, at each directory where `at` looks.
pub fn find(from: &Path) -> Option<PathBuf> {
    from.ancestors().find_map(at)
}

/// The build's root of an export: the directory whose `target/teq/` holds it, else its own.
pub fn root_of(file: &Path) -> PathBuf {
    let dir = file.parent().unwrap_or(Path::new("/"));
    match dir.parent().and_then(Path::parent) {
        Some(root) if dir.ends_with("target/teq") => root.to_path_buf(),
        _ => dir.to_path_buf(),
    }
}

/// An export's path as every reader keeps it: its build's root canonical, then its place under
/// the root as named, so that a `target` that is a link (to a directory elsewhere) still names
/// the build's root through `root_of`.
pub fn canonical(file: &Path) -> PathBuf {
    let root = root_of(file);
    let place = file.strip_prefix(&root).map_or_else(|_| PathBuf::from(FILE), Path::to_path_buf);
    let base = if root.as_os_str().is_empty() { Path::new(".") } else { root.as_path() };
    crate::source::canonicalize(base).unwrap_or_else(|_| base.to_path_buf()).join(place)
}

/// Whether a generator is one sbt runs and teq cannot: of kind `sbt`.
pub fn by_sbt(generator: &Value) -> bool {
    generator.get("kind").and_then(Value::str) == Some("sbt")
}

impl Export {
    pub fn read(file: &Path) -> Result<Export, String> {
        let bytes = std::fs::read(file).map_err(|e| format!("cannot read {}: {}", file.display(), e))?;
        Export::parse(&canonical(file), &bytes)
    }

    pub fn parse(file: &Path, bytes: &[u8]) -> Result<Export, String> {
        let shown = file.display();
        let fail = |what: String| format!("{}: {}", shown, what);
        let text = std::str::from_utf8(bytes).map_err(|_| fail("not UTF-8".to_string()))?;
        let (version, format) = lock::header(text).map_err(|e| fail(e.to_string()))?;
        if format != lock::FORMAT {
            return Err(fail(format!("format {}, which this teq does not read (it reads format {}): run the teq the lock names, {}", format, lock::FORMAT, version)));
        }
        let value = lock::parse(text).map_err(|e| fail(e.to_string()))?;
        let mut repositories = BTreeMap::new();
        for r in value.get("repositories").map_or(&[][..], Value::items) {
            let id = r.get("id").and_then(Value::str).ok_or_else(|| fail("a repository without an id".to_string()))?;
            let url = r.get("url").and_then(Value::str).ok_or_else(|| fail(format!("repository {} has no url", id)))?;
            repositories.insert(id.to_string(), Repository { url: url.to_string(), credentials: r.get("credentials").and_then(Value::str).map(str::to_string) });
        }
        let mut table = BTreeMap::new();
        for (key, record) in value.get("jars").map_or(&[][..], Value::entries) {
            table.insert(key.clone(), jar(key, record).map_err(|e| fail(format!("jars.{}: {}", key, e)))?);
        }
        let inputs = value.at(&["inputs", "files"]).map_or(&[][..], Value::entries).iter().filter_map(|(k, v)| Some((k.clone(), v.str()?.to_string()))).collect();
        let mut projects = BTreeMap::new();
        for (name, p) in value.get("projects").map_or(&[][..], Value::entries) {
            projects.insert(name.clone(), project(p).map_err(|e| fail(format!("projects.{}: {}", name, e)))?);
        }
        let mut binaries = Vec::new();
        for (classifier, fields) in value.get("binaries").map_or(&[][..], Value::entries) {
            let fields = fields.str().ok_or_else(|| fail(format!("binaries.{} is not `<url> <sha1> <size>`", classifier)))?;
            binaries.push((classifier.clone(), fields.to_string()));
        }
        let export = Export {
            file: file.to_path_buf(),
            root: root_of(file),
            sha256: sha256::hex(bytes),
            projects,
            repositories,
            table,
            inputs,
            teq: Pinned { version, binaries },
            java_output_version: value.at(&["java", "outputVersion"]).and_then(Value::uint),
        };
        export.check_references().map_err(fail)?;
        Ok(export)
    }

    /// Every product names a configuration of the export, every key a jar of the table, every jar
    /// a repository of it.
    fn check_references(&self) -> Result<(), String> {
        for (key, jar) in &self.table {
            if !self.repositories.contains_key(&jar.repository) {
                return Err(format!("the jar {} names repository {}, which the export does not list", key, jar.repository));
            }
        }
        for (name, p) in &self.projects {
            for (c, conf) in &p.configurations {
                for entry in &conf.classpath {
                    match entry {
                        Entry::Product(key) if self.configuration(key).is_none() => return Err(format!("{}/{} names {}, which the export does not hold", name, c, key)),
                        Entry::Artifact(key) if !self.table.contains_key(key) => return Err(format!("{}/{} names the jar {}, which the export's jars do not hold", name, c, key)),
                        _ => {}
                    }
                }
            }
            for mapping in p.stage.iter().flat_map(|s| s.layers.values().flatten()) {
                match &mapping.from {
                    Staged::Product { key, .. } if self.configuration(key).is_none() => return Err(format!("{}'s stage names {}, which the export does not hold", name, key)),
                    Staged::Artifact(module) if self.artifact(name, module).is_none() => return Err(format!("{}'s stage names {}, which its runtime classpath does not hold", name, module)),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// The jar of a project's runtime classpath that a stage names by its key.
    pub fn artifact(&self, project: &str, key: &str) -> Option<&Entry> {
        let conf = self.configuration(&Key::new(project, "runtime"))?;
        conf.classpath.iter().find(|e| matches!(e, Entry::Artifact(k) if k == key))
    }

    pub fn configuration(&self, key: &Key) -> Option<&Configuration> {
        self.projects.get(&key.project)?.configurations.get(&key.configuration)
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        normalize(&self.root.join(relative))
    }

    /// The configurations whose sources a configuration is typed with: those of the products on
    /// its classpath, each after the ones it depends on in turn, then the configuration itself.
    pub fn closure(&self, key: &Key) -> Vec<Key> {
        fn visit(export: &Export, key: &Key, seen: &mut BTreeSet<Key>, out: &mut Vec<Key>) {
            if !seen.insert(key.clone()) {
                return;
            }
            if let Some(conf) = export.configuration(key) {
                for entry in &conf.classpath {
                    if let Entry::Product(dep) = entry {
                        visit(export, dep, seen, out);
                    }
                }
            }
            out.push(key.clone());
        }
        let mut out = Vec::new();
        visit(self, key, &mut BTreeSet::new(), &mut out);
        out
    }

    /// The source roots of a closure in its order, each once.
    pub fn roots(&self, closure: &[Key]) -> Vec<String> {
        let mut roots: Vec<String> = Vec::new();
        for key in closure {
            for s in self.configuration(key).map_or(&[][..], |c| &c.sources) {
                if !roots.contains(s) {
                    roots.push(s.clone());
                }
            }
        }
        roots
    }

    /// The configuration of a project that one resident types for everything the project's
    /// verbs need: its test configuration, which holds the main sources, when it has one and its
    /// closure's sources are all teq's to generate (a test generator sbt runs leaves the
    /// compile configuration to type).
    pub fn widest(&self, project: &str) -> Option<Key> {
        let p = self.projects.get(project)?;
        let present: Vec<Key> = ["test", "compile"].into_iter().filter(|c| p.configurations.contains_key(*c)).map(|c| Key::new(project, c)).collect();
        present.iter().find(|k| self.typable(k)).or(present.first()).cloned()
    }

    /// Whether one resident may type a configuration for teq: no configuration of its
    /// closure has a source generator sbt runs.
    pub fn typable(&self, key: &Key) -> bool {
        self.sbt_generated(&self.closure(key)).is_empty()
    }

    /// The source generators of a closure's configurations that sbt runs, by configuration.
    fn sbt_generated(&self, closure: &[Key]) -> Vec<(Key, &Value)> {
        closure.iter().filter_map(|k| Some((k, self.configuration(k)?))).flat_map(|(k, c)| c.generators.iter().filter(|g| by_sbt(g)).map(move |g| (k.clone(), g))).collect()
    }

    /// Why a verb cannot run a project, as the export records it: the generators sbt runs of the
    /// configurations the verb types (their sources) or runs (their resources too), and what the
    /// project's export says the verb cannot reproduce; every reason of the projects involved,
    /// each once, none when it runs. `compile`, `watch` and `build` type the project's closure;
    /// `test` runs its test configuration's; `run`, `stage` and `dev` its runtime classpath's.
    pub fn refusals(&self, verb: &str, project: &str) -> Vec<String> {
        let Some(p) = self.projects.get(project) else { return Vec::new() };
        let mut keys: Vec<Key> = Vec::new();
        let (typed, resources): (Vec<Key>, bool) = match verb {
            "compile" => (vec![Key::new(project, "compile")], false),
            "watch" | "build" => (self.widest(project).into_iter().collect(), false),
            "test" => (vec![Key::new(project, "test")], true),
            _ => (vec![Key::new(project, "compile"), Key::new(project, "runtime")], true),
        };
        for key in typed {
            for k in self.closure(&key) {
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
        }
        let mut out: Vec<String> = Vec::new();
        let mut add = |line: String| {
            if !out.contains(&line) {
                out.push(line);
            }
        };
        for key in keys.iter().filter(|k| self.configuration(k).is_some()) {
            let conf = &self.configuration(key).expect("present");
            let remedy = if key.configuration == "compile" { format!("make it a TeqCommand of Compile / teqGenerators, or build {} with sbt", key.project) } else { format!("build {} with sbt", key.project) };
            for g in conf.generators.iter().filter(|g| by_sbt(g)) {
                let task = g.get("task").and_then(Value::str).unwrap_or("an unnamed task");
                match g.get("reason").and_then(Value::str) {
                    Some(reason) => add(format!("{} has the source generator {}, which sbt runs and teq cannot ({}): {}", key, task, reason, remedy)),
                    None => add(format!("{} has the source generator {}, which sbt runs and teq cannot: {}", key, task, remedy)),
                }
            }
            if resources {
                for g in conf.resource_generators.iter() {
                    let task = g.get("task").and_then(Value::str).unwrap_or("an unnamed task");
                    add(format!("{} has the resource generator {}, which sbt runs and teq cannot: make its output a resource directory of the build, or {} {} with sbt", key, task, verb, key.project));
                }
            }
        }
        for why in p.unsupported.get(verb).into_iter().flatten() {
            add(format!("{}: {}", project, why));
        }
        out
    }

    /// The configurations whose own source roots hold a file, a canonical path; a root only a
    /// project's description names (`teqExtraSources`) is its compile configuration's.
    pub fn owners(&self, file: &Path) -> Vec<Key> {
        let holds = |s: &String| file.starts_with(crate::lsp::uri::canonical(&self.path(s)));
        let declared: BTreeSet<&String> = self.projects.values().flat_map(|p| p.configurations.values().flat_map(|c| c.sources.iter())).collect();
        let mut out = Vec::new();
        for (name, p) in &self.projects {
            for (c, conf) in &p.configurations {
                if conf.sources.iter().any(holds) {
                    out.push(Key::new(name, c));
                }
            }
            let extra = p.description.sources.iter().flatten().any(|s| !declared.contains(s) && holds(s));
            if extra && p.configurations.contains_key("compile") && !out.iter().any(|k| k.project == *name) {
                out.push(Key::new(name, "compile"));
            }
        }
        out
    }

    /// The jars a session of the configuration reads, in the classpath's order: on the JVM every
    /// jar, on Scala.js the platform's Scala 3 libraries without the standard library's, as
    /// sbt-teq's `teqClasspath` takes them; the artifacts that neither cache holds fetched together
    /// (`fetch::resolve_all`), `progress` told of each transfer as it starts.
    pub fn jars_with(&self, key: &Key, progress: &(dyn Fn(&str) + Sync)) -> Result<Vec<PathBuf>, String> {
        let Some(project) = self.projects.get(&key.project) else { return Err(format!("no project {}", key.project)) };
        let Some(conf) = project.configurations.get(&key.configuration) else { return Err(format!("{} has no configuration {}", key.project, key.configuration)) };
        let mut entries = Vec::new();
        for entry in &conf.classpath {
            let name = match entry {
                Entry::Product(_) => continue,
                Entry::Artifact(key) => file_name(&self.table[key].path),
                Entry::File(path) => file_name(path),
            };
            // A directory of the repository's tree on the classpath is a module's products.
            let products = matches!(entry, Entry::File(path) if self.path(path).is_dir());
            if !products && (!name.ends_with(".jar") || (project.platform == Platform::Js && !is_scalajs_tasty_jar(name))) {
                continue;
            }
            entries.push(entry);
        }
        let artifacts: Vec<fetch::Wanted> = entries.iter().filter_map(|e| if let Entry::Artifact(k) = e { Some(self.wanted(k)) } else { None }).collect();
        let mut fetched = fetch::resolve_all(&artifacts, progress)?.into_iter();
        entries.into_iter().map(|e| if let Entry::Artifact(_) = e { Ok(fetched.next().expect("a file per artifact")) } else { self.resolve(e) }).collect()
    }

    /// A jar of the table as the fetch takes it.
    fn wanted<'a>(&'a self, key: &'a str) -> fetch::Wanted<'a> {
        let jar = &self.table[key];
        let repo = &self.repositories[&jar.repository];
        fetch::Wanted {
            source: fetch::Source { repository: &repo.url, path: &jar.path, credentials: repo.credentials.as_deref() },
            pin: fetch::Pin { sha1: &jar.sha1, size: jar.size },
            kind: "artifacts",
            name: file_name(&jar.path),
            key,
        }
    }

    /// The local file of a jar entry.
    pub fn resolve(&self, entry: &Entry) -> Result<PathBuf, String> {
        match entry {
            Entry::Product(key) => Err(format!("{} is a product, not a file", key)),
            Entry::File(path) => {
                let file = self.path(path);
                if file.exists() {
                    Ok(file)
                } else {
                    Err(format!("{} names {}, which does not exist", FILE, path))
                }
            }
            Entry::Artifact(key) => fetch::resolve_all(&[self.wanted(key)], &|_| {}).map(|mut files| files.remove(0)),
        }
    }

    /// The build definition files as they are now, with the digests of their contents: every
    /// `.sbt` at the root, `project/build.properties`, and every `.sbt` and `.scala` under
    /// `project/` outside its `target` and hidden directories.
    pub fn current_inputs(&self) -> BTreeMap<String, String> {
        input_digests(&self.root)
    }

    /// What changed in the build definition since the export; nothing when it is current.
    pub fn stale_inputs(&self) -> Stale {
        let now = self.current_inputs();
        let mut stale = Stale::default();
        for (file, digest) in &now {
            match self.inputs.get(file) {
                Some(d) if d == digest => {}
                Some(d) => {
                    stale.changes.push(format!("{} changed", file));
                    if line_ends_alone(&self.path(file), d) {
                        stale.line_ends.push(file.clone());
                    }
                }
                None => stale.changes.push(format!("{} added", file)),
            }
        }
        stale.changes.extend(self.inputs.keys().filter(|f| !now.contains_key(*f)).map(|f| format!("{} removed", f)));
        stale
    }

    /// The directory a resident of the configuration writes its class files into.
    pub fn classes(&self, key: &Key) -> String {
        format!("target/teq/{}/{}/classes", key.project, key.configuration)
    }

    /// The argument file of the daemon's resident of the configuration, beside its class
    /// directory: it starts as `teq @<file>`, since the class path on the command line runs past
    /// Windows' cap (docs/TARGETS.md, "Argument files").
    pub fn resident_args_file(&self, key: &Key) -> PathBuf {
        self.path(&format!("target/teq/{}/{}/resident.args", key.project, key.configuration))
    }

    /// The command line, after `teq`, of the resident that types and writes a configuration's
    /// closure, run from the build's root: on the JVM every class file of the closure, with the
    /// version 3 analysis; on Scala.js a check.
    pub fn resident_args(&self, key: &Key) -> Result<Vec<String>, String> {
        self.resident_args_with(key, &|_| {})
    }

    /// `resident_args`, `progress` told of each artifact fetched for it.
    pub fn resident_args_with(&self, key: &Key, progress: &(dyn Fn(&str) + Sync)) -> Result<Vec<String>, String> {
        let project = &self.projects[&key.project];
        let conf = &project.configurations[&key.configuration];
        let jvm = project.platform == Platform::Jvm;
        let mut args = vec!["compiler".to_string(), "watch".to_string()];
        args.extend(self.inputs_of(key).into_iter().map(|p| self.relative(&p)));
        for x in &project.description.excludes {
            args.extend(["--exclude".to_string(), x.clone()]);
        }
        let jars = self.jars_with(key, progress)?;
        if !jars.is_empty() {
            args.extend(["--classpath".to_string(), join_paths(&jars)]);
        }
        if jvm {
            args.extend(["--target", "jvm", "--std=scala-library", "--all-mains", "--analysis-version", "3", "-o"].map(str::to_string));
            args.push(self.classes(key));
        } else {
            args.push("--check".to_string());
        }
        args.extend(project.description.session_args());
        args.extend(conf.flags.args(jvm));
        Ok(args)
    }

    /// The inputs of a session of the configuration: the project's stand-ins, then the closure's
    /// source roots, the compile part of them as the description selects it (its overrides
    /// honoured) when it says, followed by the roots the closure adds to that (a test
    /// configuration's own and those of the configurations it adds).
    pub fn session_inputs(&self, key: &Key) -> Vec<PathBuf> {
        let project = &self.projects[&key.project];
        let mut inputs: Vec<PathBuf> = project.description.lib.iter().map(|l| self.path(l)).collect();
        let roots = self.roots(&self.closure(key));
        match &project.description.sources {
            Some(selected) => {
                let compile = self.roots(&self.closure(&Key::new(&key.project, "compile")));
                inputs.extend(selected.iter().map(|r| self.path(r)));
                inputs.extend(roots.iter().filter(|r| !compile.contains(r) && !selected.contains(r)).map(|r| self.path(r)));
            }
            None => inputs.extend(roots.iter().map(|r| self.path(r))),
        }
        inputs
    }

    /// The session's inputs that exist: generated sources not generated yet are left out.
    pub fn inputs_of(&self, key: &Key) -> Vec<PathBuf> {
        let mut inputs = self.session_inputs(key);
        inputs.retain(|p| p.exists());
        inputs
    }

    pub fn relative(&self, path: &Path) -> String {
        match path.strip_prefix(&self.root) {
            Ok(r) if r.as_os_str().is_empty() => ".".to_string(),
            Ok(r) => r.to_string_lossy().into_owned(),
            Err(_) => path.to_string_lossy().into_owned(),
        }
    }
}

/// A classpath in the platform's form: `;` between the entries on Windows, `:` elsewhere.
pub fn join_paths(paths: &[PathBuf]) -> String {
    let separator = if cfg!(windows) { ";" } else { ":" };
    paths.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>().join(separator)
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// The Maven layout of a jar's key, where a reader finds it in its repository unless the export
/// names another path: `org/with/slashes/name/version/name-version[-classifier].jar`; none for a
/// string that is no `organization:name:version[:classifier]`.
pub fn maven_path(key: &str) -> Option<String> {
    let parts: Vec<&str> = key.split(':').collect();
    if !(3..=4).contains(&parts.len()) || parts.iter().any(|p| p.is_empty()) {
        return None;
    }
    let classifier = parts.get(3).map_or(String::new(), |c| format!("-{}", c));
    Some(format!("{}/{}/{}/{}-{}{}.jar", parts[0].replace('.', "/"), parts[1], parts[2], parts[1], parts[2], classifier))
}

/// sbt-teq's `isTastyJar` for Scala.js: a `_sjs1_3-` library that is not the standard library's.
fn is_scalajs_tasty_jar(name: &str) -> bool {
    const STD: [&str; 4] = ["scala-library-", "scala3-library_", "scalajs-library_", "scalajs-scalalib_"];
    name.contains("_sjs1_3-") && !STD.iter().any(|s| name.starts_with(s))
}

/// The build definition files changed since the export.
#[derive(Default, PartialEq, Eq, Debug)]
pub struct Stale {
    /// Each file changed, added or removed, in order of path, the removed last: `<path> changed`.
    pub changes: Vec<String>,
    /// The changed files whose bytes are the recorded ones but for their line ends
    /// (`line_ends_alone`), a checkout's CRLF where the export read LF or the reverse. They are
    /// changed all the same: scalac keeps a `\r\n` in a multi-line string of a `.scala` file.
    pub line_ends: Vec<String>,
}

impl Stale {
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Whether every change is a file's line ends: an export here would record this checkout's,
    /// so the warning leaves the command to the note's remedy.
    pub fn line_ends_only(&self) -> bool {
        !self.changes.is_empty() && self.changes.len() == self.line_ends.len()
    }

    /// The note on the files that differ by line ends alone, with the remedy; none when none does.
    pub fn line_end_note(&self) -> Option<String> {
        let verb = match self.line_ends.len() {
            0 => return None,
            1 => "differs",
            _ => "differ",
        };
        Some(format!("{} {} from {}'s record by line ends alone, LF against CRLF: {}", self.line_ends.join(", "), verb, FILE, LF_REMEDY))
    }
}

/// How a repository keeps the build definition files LF in every checkout, so that a lock
/// exported from one checkout is current in another, and exports again a lock recorded from CRLF:
/// the remedy the note on a file that differs by line ends alone gives, and sbt-teq's export when
/// it reads CRLF in a Git work tree (`Export.LfRemedy`).
pub const LF_REMEDY: &str = ".gitattributes keeps the build's files LF in every checkout with *.sbt text eol=lf, project/**/*.scala text eol=lf and project/build.properties text eol=lf: add them, delete the files and check them out again, then export again";

/// Whether a file's bytes are those a digest records but for their line ends: with every `\r\n`
/// read as `\n`, or every `\n` alone written `\r\n` (Git's checkout of a text file with CRLF).
fn line_ends_alone(file: &Path, recorded: &str) -> bool {
    let Ok(bytes) = std::fs::read(file) else { return false };
    let mut lf = Vec::with_capacity(bytes.len());
    let mut crlf = Vec::with_capacity(bytes.len() + bytes.len() / 16);
    for (i, &b) in bytes.iter().enumerate() {
        if !(b == b'\r' && bytes.get(i + 1) == Some(&b'\n')) {
            lf.push(b);
        }
        if b == b'\n' && (i == 0 || bytes[i - 1] != b'\r') {
            crlf.push(b'\r');
        }
        crlf.push(b);
    }
    sha256::hex(&lf) == recorded || sha256::hex(&crlf) == recorded
}

/// The digests of a build's definition files, by path relative to the root.
pub fn input_digests(root: &Path) -> BTreeMap<String, String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(root).map(|es| es.flatten().map(|e| e.path()).filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "sbt")).collect()).unwrap_or_default();
    let properties = root.join("project/build.properties");
    if properties.is_file() {
        files.push(properties);
    }
    definition_files(&root.join("project"), &mut files);
    files
        .into_iter()
        .filter_map(|f| {
            let relative = f.strip_prefix(root).ok()?.to_string_lossy().replace('\\', "/");
            Some((relative, sha256::hex(&std::fs::read(&f).ok()?)))
        })
        .collect()
}

fn definition_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if entry.file_type().is_ok_and(|k| k.is_dir()) {
            if name != "target" && !name.starts_with('.') {
                definition_files(&path, out);
            }
        } else if path.extension().is_some_and(|x| x == "sbt" || x == "scala") {
            out.push(path);
        }
    }
}

fn project(value: &Value) -> Result<Project, String> {
    let platform = match value.get("platform").and_then(Value::str) {
        Some("jvm") => Platform::Jvm,
        Some("js") => Platform::Js,
        other => return Err(format!("platform is jvm or js, not {}", other.unwrap_or("missing"))),
    };
    let mut configurations = BTreeMap::new();
    for (name, c) in value.get("configurations").map_or(&[][..], Value::entries) {
        configurations.insert(name.clone(), configuration(c).map_err(|e| format!("configurations.{}: {}", name, e))?);
    }
    let dev = match value.get("dev") {
        None => None,
        Some(d) => {
            let text = |key: &str| d.get(key).and_then(Value::str).map(str::to_string).ok_or_else(|| format!("dev has no {}", key));
            let command = strings(d.get("command"));
            if command.is_empty() {
                return Err("dev has no command".to_string());
            }
            Some(Dev { package_manager: text("packageManager")?, command, lockfile: text("lockfile")? })
        }
    };
    Ok(Project {
        base: value.get("base").and_then(Value::str).unwrap_or(".").to_string(),
        platform,
        configurations,
        description: value.get("description").map(description).unwrap_or_default(),
        dev,
        run: value.get("run").map(|r| RunContext {
            base_directory: r.get("baseDirectory").and_then(Value::str).unwrap_or(".").to_string(),
            env_vars: string_map(r.get("envVars")),
            java_options: strings(r.get("javaOptions")),
            aliases: string_map(r.get("aliases")),
            main_class: r.get("mainClass").and_then(Value::str).map(str::to_string),
        }),
        stage: value.get("stage").map(stage).transpose()?,
        unsupported: value.get("unsupported").map_or(&[][..], Value::entries).iter().map(|(verb, whys)| (verb.clone(), strings(Some(whys)))).collect(),
    })
}

fn string_map(value: Option<&Value>) -> BTreeMap<String, String> {
    value.map_or(&[][..], Value::entries).iter().filter_map(|(k, v)| Some((k.clone(), v.str()?.to_string()))).collect()
}

fn stage(value: &Value) -> Result<Stage, String> {
    let text = |key: &str| value.get(key).and_then(Value::str).map(str::to_string).ok_or_else(|| format!("stage has no {}", key));
    match value.get("kind").and_then(Value::str) {
        Some("docker") => {}
        other => return Err(format!("stage is of kind docker, not {}", other.unwrap_or("missing"))),
    }
    let mut layers = BTreeMap::new();
    let Some(Value::Map(fields)) = value.get("layers") else { return Err("stage has no layers".to_string()) };
    for (layer, mappings) in fields {
        let n: u32 = layer.parse().map_err(|_| format!("stage layer {} is not a number", layer))?;
        let Value::List(mappings) = mappings else { return Err(format!("stage layer {} is not a list", layer)) };
        let mut out = Vec::new();
        for (i, m) in mappings.iter().enumerate() {
            out.push(mapping(m).ok_or_else(|| format!("stage.layers.{}[{}] is neither a product, an artifact, a file nor the script", layer, i))?);
        }
        layers.insert(n, out);
    }
    Ok(Stage {
        name: text("name")?,
        main_class: match (value.get("mainClass").and_then(Value::str), value.get("mainClassNone").and_then(Value::bool)) {
            (Some(main), _) => MainClass::Declared(main.to_string()),
            (None, Some(true)) => MainClass::DeclaredNone,
            (None, _) => MainClass::Unset,
        },
        exposed_ports: value.get("exposedPorts").map_or(&[][..], Value::items).iter().filter_map(Value::uint).collect(),
        directory: text("directory")?,
        layers,
    })
}

fn mapping(value: &Value) -> Option<Mapping> {
    if let Some(script) = value.get("script").and_then(Value::str) {
        return Some(Mapping { from: Staged::Script { classpath: strings(value.get("classpath")) }, to: script.to_string() });
    }
    let to = value.get("to").and_then(Value::str)?.to_string();
    let from = value.get("from")?;
    let from = if let Some(key) = from.str() {
        Staged::Artifact(key.to_string())
    } else if let Some(file) = from.get("file").and_then(Value::str) {
        Staged::File(file.to_string())
    } else {
        let key = Key::new(from.get("project").and_then(Value::str)?, from.get("configuration").and_then(Value::str)?);
        let manifest = value.get("manifest").map_or(&[][..], Value::entries).iter().filter_map(|(k, v)| Some((k.clone(), v.str()?.to_string()))).collect();
        Staged::Product { key, manifest }
    };
    Some(Mapping { from, to })
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value.map_or(&[][..], Value::items).iter().filter_map(Value::str).map(str::to_string).collect()
}

fn configuration(value: &Value) -> Result<Configuration, String> {
    let mut classpath = Vec::new();
    for (i, e) in value.get("classpath").map_or(&[][..], Value::items).iter().enumerate() {
        classpath.push(entry(e).ok_or_else(|| format!("classpath[{}] is neither a jar's key, a product nor a file", i))?);
    }
    let flags = value.get("flags").map_or(Flags::default(), |f| Flags {
        max_inlines: f.get("maxInlines").and_then(Value::uint),
        strict_equality: f.get("strictEquality").and_then(Value::bool) == Some(true),
        kind_projector: f.get("kindProjector").and_then(Value::bool) == Some(true),
        werror: f.get("werror").and_then(Value::bool) == Some(true),
        wunused_imports: f.get("wunusedImports").and_then(Value::bool) == Some(true),
        java_output_version: f.get("javaOutputVersion").and_then(Value::uint),
    });
    let test = value.get("frameworks").map(|_| TestContext {
        fork: value.get("fork").and_then(Value::bool) == Some(true),
        base_directory: value.get("baseDirectory").and_then(Value::str).unwrap_or(".").to_string(),
        env_vars: string_map(value.get("envVars")),
        java_options: strings(value.get("javaOptions")),
        frameworks: value
            .get("frameworks")
            .map_or(&[][..], Value::items)
            .iter()
            .filter_map(|f| Some(Framework { class: f.get("class").and_then(Value::str)?.to_string(), arguments: strings(f.get("arguments")) }))
            .collect(),
        exclude: strings(value.get("exclude")),
    });
    Ok(Configuration {
        sources: strings(value.get("sources")),
        resources: strings(value.get("resources")),
        classpath,
        flags,
        generators: value.get("generators").map_or(&[][..], Value::items).to_vec(),
        resource_generators: value.get("resourceGenerators").map_or(&[][..], Value::items).to_vec(),
        main_classes: strings(value.get("mainClasses")),
        test,
    })
}

fn entry(value: &Value) -> Option<Entry> {
    if let Some(key) = value.str() {
        return Some(Entry::Artifact(key.to_string()));
    }
    if let (Some(project), Some(configuration)) = (value.get("project").and_then(Value::str), value.get("configuration").and_then(Value::str)) {
        return Some(Entry::Product(Key::new(project, configuration)));
    }
    value.get("file").and_then(Value::str).map(|file| Entry::File(file.to_string()))
}

/// A record of the jar table, its fields: the repository, the sha1 and the size, then the path
/// where the jar is not at the Maven layout of its key.
fn jar(key: &str, value: &Value) -> Result<Jar, String> {
    let fields: Vec<&str> = value.str().ok_or("no fields `<repository> <sha1> <size> [<path>]`")?.split(' ').collect();
    let &[repository, sha1, size, ref rest @ ..] = fields.as_slice() else { return Err("fewer fields than `<repository> <sha1> <size>`".to_string()) };
    if !(sha1.len() == 40 && sha1.bytes().all(|b| b.is_ascii_hexdigit())) {
        return Err("no sha1 of 40 hexadecimal digits".to_string());
    }
    let size: u64 = size.parse().map_err(|_| "no size in bytes")?;
    let path = match rest {
        [] => maven_path(key).ok_or("no path, and its key is no organization:name:version[:classifier] to derive one from")?,
        [path] => (*path).to_string(),
        _ => return Err("more fields than `<repository> <sha1> <size> <path>`".to_string()),
    };
    Ok(Jar { repository: repository.to_string(), path, sha1: sha1.to_ascii_lowercase(), size })
}

fn description(value: &Value) -> Description {
    let number = |key: &str| value.get(key).and_then(|v| v.uint().or_else(|| v.str()?.parse().ok()));
    Description {
        lib: value.get("lib").and_then(Value::str).map(str::to_string),
        sources: value.get("sources").map(|s| strings(Some(s))),
        excludes: strings(value.get("excludes")),
        out: value.get("out").and_then(Value::str).map(str::to_string),
        module_per_file: strings(value.get("modulePerFile")),
        cacheable_state: strings(value.get("cacheableState")),
        macro_state: value.get("macroState").and_then(Value::str).map(str::to_string),
        threads: number("threads"),
        hot: value.get("hot").and_then(Value::bool) == Some(true),
        release: value.get("release").and_then(Value::bool) == Some(true),
        production: Production { sources: strings(value.at(&["production", "sources"])), excludes: strings(value.at(&["production", "excludes"])) },
        main_class: value.get("mainClass").and_then(Value::str).map(str::to_string),
        keys: string_map(value.get("keys")),
    }
}

/// `a/./b/../c` as `a/c`, without resolving symbolic links.
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::sha256::Sha256;

    /// The hash of the inputs: the SHA-256 of `<path>\0<digest>\n` for every file in order of path.
    fn inputs_hash(inputs: &BTreeMap<String, String>) -> String {
        let mut sha = Sha256::new();
        for (file, digest) in inputs {
            sha.update(file.as_bytes());
            sha.update(b"\0");
            sha.update(digest.as_bytes());
            sha.update(b"\n");
        }
        sha.hex()
    }

    fn fixture_file() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("integrations/sbt/example").join(FILE)
    }

    /// A lock of the header and the given body, as a test writes one.
    pub fn lock(body: &str) -> String {
        format!("teq: 0.1.2\nformat: 1\nbinaries: {{}}\n{}", body)
    }

    fn parse(text: &str) -> Result<Export, String> {
        Export::parse(Path::new("/b/teq.lock"), text.as_bytes())
    }

    /// The binaries table of a release not published yet in the shapes a lock may give it: empty, as the export writes
    /// it, and absent, each read as no binary of the compiler it names (the launchers refuse both); bare, which no
    /// writer gives, refused on its line.
    #[test]
    fn an_unpinned_table() {
        for table in ["binaries: {}\n", ""] {
            let export = parse(&format!("teq: 0.1.7\nformat: 1\n{}projects: {{}}\n", table)).unwrap();
            assert_eq!((export.teq.version.as_str(), export.teq.binaries.len()), ("0.1.7", 0), "{table:?}");
        }
        let bare = parse("teq: 0.1.7\nformat: 1\nbinaries:\nprojects: {}\n").err().unwrap_or_default();
        assert!(bare.contains("line 3: binaries with nothing below it"), "{bare}");
    }

    #[test]
    fn the_example_fixture_is_canonical() {
        let text = std::fs::read_to_string(fixture_file()).unwrap();
        assert_eq!(lock::write(&lock::parse(&text).unwrap()), text);
    }

    /// The conformance corpus (`tests/lock/`, which `tests/lock.sh` holds the YAML 1.2 core reader,
    /// vite-plugin-teq's and check-export.py's to): this reader reads it to its tree and refuses
    /// every document of `refused.txt` on its line, this writer writes the tree to its bytes.
    #[test]
    fn the_conformance_corpus() {
        use crate::lsp::json::Json;
        fn tree(json: &Json) -> Value {
            match json {
                Json::Obj(fields) => Value::Map(fields.iter().map(|(k, v)| (k.clone(), tree(v))).collect()),
                Json::Arr(items) => Value::List(items.iter().map(tree).collect()),
                Json::Str(s) => Value::Str(s.clone()),
                Json::Num(n) => Value::Int(*n as i64),
                Json::Bool(b) => Value::Bool(*b),
                Json::Null => panic!("the corpus holds no null"),
            }
        }
        fn sorted(v: &Value) -> Value {
            match v {
                Value::Map(entries) => {
                    let mut entries: Vec<(String, Value)> = entries.iter().map(|(k, v)| (k.clone(), sorted(v))).collect();
                    entries.sort_by(|a, b| a.0.cmp(&b.0));
                    Value::Map(entries)
                }
                Value::List(items) => Value::List(items.iter().map(sorted).collect()),
                other => other.clone(),
            }
        }
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/lock");
        let corpus = std::fs::read_to_string(dir.join("corpus.lock")).unwrap();
        let wanted = tree(&Json::parse(&std::fs::read_to_string(dir.join("corpus.json")).unwrap()).unwrap());
        assert_eq!(sorted(&lock::parse(&corpus).unwrap()), sorted(&wanted));
        assert_eq!(lock::write(&wanted), corpus);
        let cases = std::fs::read_to_string(dir.join("refused.txt")).unwrap();
        let cases: Vec<&str> = cases.split("\n=== ").skip(1).collect();
        assert!(cases.len() >= 20);
        for case in cases {
            let (head, text) = case.split_once('\n').unwrap();
            let line = head.split(' ').next().unwrap();
            let text = if text.ends_with('\n') { text.to_string() } else { format!("{}\n", text) };
            let message = lock::parse(&text).err().map(|e| e.to_string()).unwrap_or_default();
            assert!(message.starts_with(&format!("line {}: ", line)), "{}: {}", head, message);
        }
    }

    #[test]
    fn the_example_fixture_reads() {
        let export = Export::read(&fixture_file()).unwrap();
        assert_eq!(export.projects.len(), 9);
        let api = &export.projects["api"];
        assert_eq!(api.platform, Platform::Jvm);
        assert_eq!(export.closure(&Key::new("api", "test")), vec![Key::new("api", "compile"), Key::new("api", "test")]);
        assert_eq!(export.roots(&export.closure(&Key::new("api", "test"))), ["src/shared", "src/api", "target/teq/api/compile/src_managed", "api-test-src"]);
        assert_eq!(export.closure(&Key::new("jvmapp", "test")), vec![Key::new("jvmcore", "compile"), Key::new("jvmapp", "compile"), Key::new("jvmapp", "test")]);
        assert_eq!(export.closure(&Key::new("frontend", "compile")), vec![Key::new("shared", "compile"), Key::new("frontend", "compile")]);
        assert_eq!(export.widest("frontend"), Some(Key::new("frontend", "test")));
        assert_eq!(api.configurations["compile"].flags.args(true), ["--max-inlines", "80"]);
        assert_eq!(export.projects["frontend"].description.session_args(), ["--cacheable-state", "meridian.web.css.Catalog"]);
        assert_eq!(export.inputs_hash_recorded(), inputs_hash(&export.inputs));
        assert_eq!(export.java_output_version, Some(17));
        let test = &api.configurations["test"];
        assert_eq!(test.resources, ["api/src/test/resources"]);
        let context = test.test.as_ref().unwrap();
        let frameworks: Vec<&str> = context.frameworks.iter().map(|f| f.class.as_str()).collect();
        assert_eq!(frameworks, ["com.novocode.junit.JUnitFramework", "munit.Framework", "org.scalacheck.ScalaCheckFramework", "org.scalatest.tools.Framework", "zio.test.sbt.ZTestFramework"]);
        assert_eq!(context.exclude, ["meridian.apitest.JunitStyleSuite"]);
        assert_eq!((context.fork, context.base_directory.as_str()), (false, "."));
        assert!(api.configurations["compile"].test.is_none());
        assert_eq!(export.table["org.typelevel:cats-core_3:2.13.0"], Jar { repository: "maven-central".to_string(), path: "org/typelevel/cats-core_3/2.13.0/cats-core_3-2.13.0.jar".to_string(), sha1: "cafcdb23af26a0b2de13a1b362ae33c72fafcd6a".to_string(), size: 7123338 });
        let text = std::fs::read_to_string(fixture_file()).unwrap();
        assert_eq!(export.teq.version, lock::header(&text).unwrap().0);
    }

    impl Export {
        fn inputs_hash_recorded(&self) -> String {
            let text = std::fs::read_to_string(&self.file).unwrap();
            lock::parse(&text).unwrap().at(&["inputs", "sha256"]).and_then(Value::str).unwrap().to_string()
        }
    }

    #[test]
    fn the_format_is_checked_first() {
        let read = |text: &str| parse(text).err().unwrap_or_default();
        // Another format is refused before its body is read, whatever it holds.
        assert_eq!(read("teq: 0.2.0\nformat: 2\nwhatever { it holds\n"), "/b/teq.lock: format 2, which this teq does not read (it reads format 1): run the teq the lock names, 0.2.0");
        assert_eq!(read("{ not a lock\n"), "/b/teq.lock: line 1: `{ not a lock` where a key and its colon stand");
        assert_eq!(read("teq: 0.1.2\nformat: 1\nbinaries: {}\na: 1.0\n"), "/b/teq.lock: line 4: the bare `1.0`, which YAML reads as other than this text: quote it");
        assert!(read(&lock("projects:\n  a:\n    configurations:\n      compile:\n        classpath:\n          - {configuration: compile, project: b}\n    platform: jvm\n")).contains("does not hold"));
        assert!(read(&lock("projects:\n  a:\n    configurations:\n      compile:\n        classpath:\n          - x:y:1.0\n    platform: jvm\n")).contains("a/compile names the jar x:y:1.0, which the export's jars do not hold"));
        assert!(read(&lock("jars:\n  x:y:1.0: r da39a3ee5e6b4b0d3255bfef95601890afd80709 1\n")).contains("the jar x:y:1.0 names repository r, which the export does not list"));
        assert!(read(&lock("projects:\n  a:\n    configurations:\n      compile:\n        classpath:\n          - {path: x.jar}\n    platform: jvm\n")).contains("classpath[0] is neither a jar's key, a product nor a file"));
        assert!(read(&lock("binaries:\n  x: 1\n")).contains("the key binaries a second time"));
        let export = parse("teq: 0.1.2\nformat: 1\nbinaries:\n  linux-x86_64: https://r/teq.exe da39a3ee5e6b4b0d3255bfef95601890afd80709 1\n").unwrap();
        assert_eq!(export.teq, Pinned { version: "0.1.2".to_string(), binaries: vec![("linux-x86_64".to_string(), "https://r/teq.exe da39a3ee5e6b4b0d3255bfef95601890afd80709 1".to_string())] });
    }

    #[test]
    fn the_jar_table() {
        let sha1 = "da39a3ee5e6b4b0d3255bfef95601890afd80709";
        let text = lock(&format!(
            "jars:
  com.lihaoyi:sourcecode_3:0.4.2: central {upper} 0
  org.jline:jline:3.29.0:jdk8: central {sha1} 7
  x:snap:1.0-SNAPSHOT: central {sha1} 1 x/snap/1.0-SNAPSHOT/snap-1.0-20261005.101010-3.jar
projects:
  a:
    configurations:
      compile:
        classpath:
          - {{file: lib/u.jar}}
          - x:snap:1.0-SNAPSHOT
          - org.jline:jline:3.29.0:jdk8
          - com.lihaoyi:sourcecode_3:0.4.2
      runtime:
        classpath:
          - {{configuration: compile, project: a}}
          - com.lihaoyi:sourcecode_3:0.4.2
    platform: jvm
    stage:
      directory: target/docker/stage
      kind: docker
      layers:
        \"2\":
          - {{from: com.lihaoyi:sourcecode_3:0.4.2, to: opt/docker/lib/s.jar}}
      mainClass: a.Main
      name: a
repositories:
  - id: central
    url: https://repo/
",
            upper = sha1.to_ascii_uppercase()
        ));
        let export = parse(&text).unwrap();
        let path = |key: &str| export.table[key].path.clone();
        assert_eq!(path("org.jline:jline:3.29.0:jdk8"), "org/jline/jline/3.29.0/jline-3.29.0-jdk8.jar");
        assert_eq!(path("com.lihaoyi:sourcecode_3:0.4.2"), "com/lihaoyi/sourcecode_3/0.4.2/sourcecode_3-0.4.2.jar");
        assert_eq!(path("x:snap:1.0-SNAPSHOT"), "x/snap/1.0-SNAPSHOT/snap-1.0-20261005.101010-3.jar");
        assert_eq!(export.table["com.lihaoyi:sourcecode_3:0.4.2"].sha1, sha1);
        let compile = &export.projects["a"].configurations["compile"].classpath;
        assert_eq!(compile[..2], [Entry::File("lib/u.jar".to_string()), Entry::Artifact("x:snap:1.0-SNAPSHOT".to_string())]);
        assert_eq!(export.artifact("a", "com.lihaoyi:sourcecode_3:0.4.2"), Some(&Entry::Artifact("com.lihaoyi:sourcecode_3:0.4.2".to_string())));
        assert_eq!(export.projects["a"].stage.as_ref().unwrap().layers[&2][0].from, Staged::Artifact("com.lihaoyi:sourcecode_3:0.4.2".to_string()));
        assert_eq!(export.projects["a"].stage.as_ref().unwrap().main_class, MainClass::Declared("a.Main".to_string()));
        // A stage block without `mainClass`, the plugin's for a build that leaves the setting
        // alone, and one with `mainClassNone`, for a build that sets it to None.
        let unset = parse(&text.replace("      mainClass: a.Main\n", "")).unwrap();
        assert_eq!(unset.projects["a"].stage.as_ref().unwrap().main_class, MainClass::Unset);
        let none = parse(&text.replace("      mainClass: a.Main\n", "      mainClassNone: true\n")).unwrap();
        assert_eq!(none.projects["a"].stage.as_ref().unwrap().main_class, MainClass::DeclaredNone);
        let read = |record: &str| parse(&lock(&format!("jars:\n  {}\n", record))).err().unwrap_or_default();
        assert!(read(&format!("a:b:1: {{sha1: {sha1}}}")).contains("jars.a:b:1: no fields"));
        assert!(read(&format!("a:b:1: r {sha1}")).contains("fewer fields than"));
        assert!(read("a:b:1: r da39 1").contains("no sha1 of 40 hexadecimal digits"));
        assert!(read(&format!("a:b:1: r {sha1} x1")).contains("no size in bytes"));
        assert!(read(&format!("a:b:1: r {sha1} 1 p q")).contains("more fields than"));
        assert!(read(&format!("lib.jar: r {sha1} 1")).contains("its key is no organization:name:version[:classifier]"));
        // A stage naming a key its runtime classpath does not hold.
        let text = text.replace("{from: com.lihaoyi:sourcecode_3:0.4.2,", "{from: org.jline:jline:3.29.0:jdk8,");
        assert!(parse(&text).err().unwrap_or_default().contains("a's stage names org.jline:jline:3.29.0:jdk8, which its runtime classpath does not hold"));
    }

    #[test]
    fn a_key_with_an_at_is_found_where_coursier_keeps_it() {
        let text = lock(
            "jars:
  org.example:probe:1.0@build: local da39a3ee5e6b4b0d3255bfef95601890afd80709 0
projects:
  a:
    configurations:
      compile:
        classpath:
          - org.example:probe:1.0@build
    platform: jvm
repositories:
  - credentials: 127.0.0.1
    id: local
    url: http://127.0.0.1:42235/
",
        );
        let export = parse(&text).unwrap();
        let jar = &export.table["org.example:probe:1.0@build"];
        assert_eq!(jar.path, "org/example/probe/1.0@build/probe-1.0@build.jar");
        let source = fetch::Source { repository: &export.repositories[&jar.repository].url, path: &jar.path, credentials: None };
        assert_eq!(
            fetch::coursier_file(Path::new("/c/v1"), &source.url()),
            Some(PathBuf::from("/c/v1/http/127.0.0.1%3A42235/org/example/probe/1.0%40build/probe-1.0%40build.jar"))
        );
    }

    #[test]
    fn maven_paths() {
        assert_eq!(maven_path("org.scala-lang:scala3-library_3:3.8.4").as_deref(), Some("org/scala-lang/scala3-library_3/3.8.4/scala3-library_3-3.8.4.jar"));
        assert_eq!(maven_path("io.netty:netty-transport-native-epoll:4.1.0:linux-x86_64").as_deref(), Some("io/netty/netty-transport-native-epoll/4.1.0/netty-transport-native-epoll-4.1.0-linux-x86_64.jar"));
        for no in ["a:b", "a:b:c:d:e", "a::1", "a:b:1:", ""] {
            assert_eq!(maven_path(no), None, "{}", no);
        }
    }

    #[test]
    fn the_inputs_and_their_changes() {
        let dir = std::env::temp_dir().join(format!("teq-export-inputs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for d in ["project/project", "project/target/x", "project/.bsp"] {
            std::fs::create_dir_all(dir.join(d)).unwrap();
        }
        for (f, t) in [
            ("build.sbt", "a"),
            ("other.sbt", "b"),
            ("project/build.properties", "sbt.version=2.0.8\n"),
            ("project/D.scala", "d"),
            ("project/project/P.scala", "p"),
            ("project/target/x/T.scala", "t"),
            ("project/.bsp/B.scala", "b"),
            ("project/notes.txt", "n"),
        ] {
            std::fs::write(dir.join(f), t).unwrap();
        }
        let inputs = input_digests(&dir);
        assert_eq!(inputs.keys().map(String::as_str).collect::<Vec<_>>(), ["build.sbt", "other.sbt", "project/D.scala", "project/build.properties", "project/project/P.scala"]);
        assert_eq!(inputs["build.sbt"], sha256::hex(b"a"));
        let mut recorded = inputs.clone();
        recorded.insert("project/Gone.scala".to_string(), sha256::hex(b"g"));
        recorded.insert("build.sbt".to_string(), sha256::hex(b"old"));
        recorded.remove("other.sbt");
        let files = recorded.iter().map(|(k, v)| format!("    {}: {}\n", k, v)).collect::<String>();
        std::fs::write(dir.join(FILE), lock(&format!("inputs:\n  files:\n{}", files))).unwrap();
        let export = Export::read(&dir.join(FILE)).unwrap();
        assert_eq!(export.stale_inputs(), Stale { changes: ["build.sbt changed", "other.sbt added", "project/Gone.scala removed"].map(String::from).to_vec(), line_ends: Vec::new() });
        assert_eq!(export.stale_inputs().line_end_note(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_that_differs_by_line_ends_alone_is_changed_and_noted() {
        let dir = std::env::temp_dir().join(format!("teq-export-line-ends-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("project")).unwrap();
        // The export read LF; the checkout has CRLF (Git for Windows' default), or the reverse.
        let lf = "name := \"x\"\n\nversion := \"1\"\n";
        let crlf = lf.replace('\n', "\r\n");
        // scalac keeps the `\r\n` of a multi-line string, so this file means otherwise with CRLF:
        // changed all the same, and noted, since its bytes differ by line ends alone.
        let string = "object S { val s = \"\"\"a\nb\"\"\" }\n";
        let files = [
            ("build.sbt", lf.to_string(), crlf.clone()),
            ("other.sbt", crlf.clone(), lf.to_string()),
            ("project/S.scala", string.to_string(), string.replace('\n', "\r\n")),
            // A file mixing both line ends against its export with LF: noted too.
            ("project/build.properties", "a=1\nb=2\n".to_string(), "a=1\nb=2\r\n".to_string()),
            // A meaningful edit beside the line ends, and a lone `\r` for each `\n`, which scalac
            // does not read as a line's end: changed, not noted.
            ("edited.sbt", lf.to_string(), crlf.replace("\"1\"", "\"2\"")),
            ("lone.sbt", lf.to_string(), lf.replace('\n', "\r")),
            ("same.sbt", crlf.clone(), crlf.clone()),
        ];
        let recorded: String = files.iter().map(|(f, then, _)| format!("    {}: {}\n", f, sha256::hex(then.as_bytes()))).collect();
        for (f, _, now) in &files {
            std::fs::write(dir.join(f), now).unwrap();
        }
        std::fs::write(dir.join(FILE), lock(&format!("inputs:\n  files:\n{}", recorded))).unwrap();
        let stale = Export::read(&dir.join(FILE)).unwrap().stale_inputs();
        assert_eq!(stale.changes, ["build.sbt changed", "edited.sbt changed", "lone.sbt changed", "other.sbt changed", "project/S.scala changed", "project/build.properties changed"]);
        assert_eq!(stale.line_ends, ["build.sbt", "other.sbt", "project/S.scala", "project/build.properties"]);
        assert_eq!(
            stale.line_end_note().unwrap(),
            format!("build.sbt, other.sbt, project/S.scala, project/build.properties differ from teq.lock's record by line ends alone, LF against CRLF: {}", LF_REMEDY)
        );
        assert!(!stale.line_ends_only());
        let one = Stale { changes: vec!["build.sbt changed".to_string()], line_ends: vec!["build.sbt".to_string()] };
        assert!(one.line_end_note().unwrap().starts_with("build.sbt differs from teq.lock's record by line ends alone, LF against CRLF: .gitattributes keeps"));
        assert!(one.line_ends_only());
        assert!(!Stale { changes: vec!["build.sbt changed".to_string(), "other.sbt added".to_string()], line_ends: vec!["build.sbt".to_string()] }.line_ends_only());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_description_selects_the_compile_sources() {
        let dir = std::env::temp_dir().join(format!("teq-export-overrides-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let text = lock(
            "projects:
  app:
    configurations:
      compile:
        classpath:
          - {configuration: compile, project: core}
        sources:
          - app/src
          - app/dropped
      test:
        classpath:
          - {configuration: compile, project: app}
        sources:
          - app/test
    description:
      sources:
        - core/src
        - app/src
        - app/extra
    platform: jvm
  core:
    configurations:
      compile:
        sources:
          - core/src
    platform: jvm
",
        );
        std::fs::write(dir.join(FILE), text).unwrap();
        let export = Export::read(&dir.join(FILE)).unwrap();
        // A relative path in the platform's form, compared by its components (`core\src` on Windows).
        let rel = |key: &Key| export.session_inputs(key).iter().map(|p| export.relative(p).replace(std::path::MAIN_SEPARATOR, "/")).collect::<Vec<_>>();
        assert_eq!(rel(&Key::new("app", "compile")), ["core/src", "app/src", "app/extra"]);
        assert_eq!(rel(&Key::new("app", "test")), ["core/src", "app/src", "app/extra", "app/test"]);
        assert_eq!(rel(&Key::new("core", "compile")), ["core/src"]);
        let root = crate::lsp::uri::canonical(&export.root);
        assert_eq!(export.owners(&root.join("app/extra/X.scala")), [Key::new("app", "compile")]);
        assert_eq!(export.owners(&root.join("core/src/C.scala")), [Key::new("core", "compile")]);
        assert!(export.owners(&root.join("elsewhere/E.scala")).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_stage_without_its_layers_is_refused() {
        let read = |layers: &str| parse(&lock(&format!("projects:\n  a:\n    platform: jvm\n    stage:\n      directory: a/target/docker/stage\n      kind: docker\n{}      mainClass: a.Main\n      name: a\n", layers))).err().unwrap_or_default();
        assert!(read("").contains("stage has no layers"));
        assert!(read("      layers: []\n").contains("stage has no layers"));
        assert!(read("      layers:\n        \"4\": {}\n").contains("stage layer 4 is not a list"));
        assert!(read("      layers:\n        \"4\":\n          - classpath: []\n            script: opt/docker/bin/a\n").is_empty());
    }

    #[test]
    fn the_export_under_target_teq() {
        let dir = std::env::temp_dir().join(format!("teq-export-places-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("target/teq")).unwrap();
        std::fs::create_dir_all(dir.join("app/src")).unwrap();
        std::fs::write(dir.join(UNDER_TARGET), lock("projects:\n  app:\n    configurations:\n      compile:\n        sources:\n          - app/src\n    platform: jvm\n")).unwrap();
        // Found from below the root and from the root, the root the build's, not target/teq.
        assert_eq!(find(&dir.join("app/src")), Some(dir.join(UNDER_TARGET)));
        let export = Export::read(&find(&dir).unwrap()).unwrap();
        assert_eq!(export.root, crate::source::canonicalize(&dir).unwrap());
        assert_eq!(export.path("app/src"), export.root.join("app/src"));
        // A lock at the root comes first, the same root.
        std::fs::write(dir.join(FILE), lock("projects: {}\n")).unwrap();
        assert_eq!(find(&dir.join("app/src")), Some(dir.join(FILE)));
        assert_eq!(root_of(&dir.join(FILE)), dir);
        assert_eq!(root_of(&dir.join(UNDER_TARGET)), dir);
        assert_eq!(root_of(Path::new("teq.lock")), Path::new(""));
        // A target that is a link to a directory elsewhere keeps the build's root.
        #[cfg(unix)]
        {
            std::fs::remove_file(dir.join(FILE)).unwrap();
            std::fs::create_dir_all(dir.join("elsewhere/teq")).unwrap();
            std::fs::rename(dir.join(UNDER_TARGET), dir.join("elsewhere/teq").join(FILE)).unwrap();
            std::fs::remove_dir_all(dir.join("target")).unwrap();
            std::os::unix::fs::symlink(dir.join("elsewhere"), dir.join("target")).unwrap();
            let export = Export::read(&find(&dir.join("app/src")).unwrap()).unwrap();
            assert_eq!(export.root, crate::source::canonicalize(&dir).unwrap());
            assert_eq!(export.file, export.root.join(UNDER_TARGET));
            assert_eq!(Export::read(&export.file).unwrap().root, export.root);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn what_sbt_alone_runs_is_refused_by_the_verbs_that_need_it() {
        // gen has a generator sbt runs; app depends on it; tests has a test generator, a resource
        // generator and a test option sbt alone runs; packaged a stage it does not reproduce.
        let text = lock(
            "projects:
  app:
    configurations:
      compile:
        classpath:
          - {configuration: compile, project: gen}
        sources: []
      runtime:
        classpath:
          - {configuration: compile, project: app}
          - {configuration: compile, project: gen}
      test:
        classpath:
          - {configuration: compile, project: app}
          - {configuration: compile, project: gen}
        frameworks: []
    platform: jvm
  gen:
    configurations:
      compile:
        generators:
          - kind: sbt
            task: an unnamed task
          - kind: sbt
            reason: the BuildInfo key builtAt runs the action builtAt
            task: buildInfo
        sources: []
    platform: jvm
  packaged:
    configurations:
      compile:
        sources: []
    platform: jvm
    unsupported:
      stage:
        - the build sets scriptClasspath
  tests:
    configurations:
      compile:
        resourceGenerators:
          - kind: sbt
            task: resources
        sources: []
      test:
        classpath:
          - {configuration: compile, project: tests}
        frameworks: []
        generators:
          - kind: sbt
            task: testSources
    platform: jvm
    unsupported:
      test:
        - Test / testOptions holds a Tests.Setup or Tests.Cleanup
",
        );
        let export = parse(&text).unwrap();
        let unnamed = "gen/compile has the source generator an unnamed task, which sbt runs and teq cannot: make it a TeqCommand of Compile / teqGenerators, or build gen with sbt";
        let build_info = "gen/compile has the source generator buildInfo, which sbt runs and teq cannot (the BuildInfo key builtAt runs the action builtAt): make it a TeqCommand of Compile / teqGenerators, or build gen with sbt";
        assert_eq!(export.refusals("compile", "gen"), [unnamed, build_info]);
        // A project whose closure holds it, every verb that types it.
        for verb in ["compile", "test", "run", "stage", "dev", "watch"] {
            assert_eq!(export.refusals(verb, "app"), [unnamed, build_info], "{}", verb);
        }
        // A test generator: test refuses, compile and run do not, and the widest configuration
        // one resident types is compile.
        assert!(export.refusals("compile", "tests").is_empty());
        assert_eq!(export.widest("tests"), Some(Key::new("tests", "compile")));
        assert_eq!(export.widest("app"), Some(Key::new("app", "test")));
        let test = export.refusals("test", "tests");
        assert_eq!(test.len(), 3, "{:?}", test);
        assert!(test[0].starts_with("tests/compile has the resource generator resources, which sbt runs and teq cannot"));
        assert_eq!(test[1], "tests/test has the source generator testSources, which sbt runs and teq cannot: build tests with sbt");
        assert_eq!(test[2], "tests: Test / testOptions holds a Tests.Setup or Tests.Cleanup");
        // A resource generator: the verbs that run the classes, not compile.
        assert_eq!(export.refusals("run", "tests").len(), 1);
        assert_eq!(export.refusals("stage", "packaged"), ["packaged: the build sets scriptClasspath"]);
        assert!(export.refusals("run", "packaged").is_empty());
        let refused = super::super::refused(&export, "compile", &["app", "gen"]).unwrap();
        assert_eq!(refused.lines().count(), 3, "{}", refused);
        assert!(super::super::refused(&export, "compile", &["packaged"]).is_none());
    }

    #[test]
    fn scalajs_libraries_as_sbt_teq_takes_them() {
        assert!(is_scalajs_tasty_jar("cats-core_sjs1_3-2.13.0.jar"));
        assert!(!is_scalajs_tasty_jar("cats-core_3-2.13.0.jar"));
        assert!(!is_scalajs_tasty_jar("scalajs-library_2.13-1.22.0.jar"));
        assert!(!is_scalajs_tasty_jar("scala3-library_sjs1_3-3.8.4.jar"));
    }
}
