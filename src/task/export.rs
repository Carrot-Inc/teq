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
    /// The inputs the build declares besides its definition files (`inputs.declared`, Mill's), paths
    /// relative to the root, which every check of the inputs reads as well.
    pub declared: Vec<String>,
    /// The build tool whose plugin wrote the export (`exporter`, sbt when absent).
    pub exporter: Exporter,
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
    /// As a test context's `envPaths`.
    pub env_paths: BTreeMap<String, Vec<String>>,
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
    /// The resource generators the build tool runs, which teq cannot (each of kind `sbt` or `mill`).
    pub resource_generators: Vec<Value>,
    /// The main classes the build declares: `Compile / mainClass`, the aliases' targets and
    /// `teqMainClasses`, sorted.
    pub main_classes: Vec<String>,
    /// The test configuration's execution context.
    pub test: Option<TestContext>,
    /// Whether the configurations its products' own classpaths hold are seen by its sources too, as
    /// sbt's classpaths are (`transitive`, true when absent). False where the build tool keeps them
    /// out of sight (Mill's `compileModuleDeps`, not transitive): the classpath's products are then
    /// the whole of what its sources see, and the closure's others its products' prerequisites alone.
    pub transitive: bool,
    /// The classpath its suites run on where it is not `classpath` (`runtimeClasspath`, Mill's test
    /// module, compiled against its compile classpath and run on its run classpath).
    pub runtime_classpath: Option<Vec<Entry>>,
}

impl Configuration {
    /// The classpath the configuration's code runs on: `runtimeClasspath`, else `classpath`.
    pub fn run_classpath(&self) -> &[Entry] {
        self.runtime_classpath.as_deref().unwrap_or(&self.classpath)
    }
}

/// How the build tool runs a test configuration's suites: its JVM's directory, options and environment,
/// the frameworks with their arguments, and the suites a `Tests.Exclude` names; on Scala.js the link's
/// module kind and the JavaScript environment.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct TestContext {
    pub fork: bool,
    /// The threads a JVM configuration's suites run on (`parallelism`).
    pub parallelism: Parallelism,
    pub base_directory: String,
    pub env_vars: BTreeMap<String, String>,
    /// Variables whose values are paths under the build's root (`envPaths`): each the paths given,
    /// absolute, joined by `;`, as Mill sets `MILL_TEST_RESOURCE_DIR` and `MILL_WORKSPACE_ROOT`.
    pub env_paths: BTreeMap<String, Vec<String>>,
    pub java_options: Vec<String>,
    pub frameworks: Vec<Framework>,
    pub exclude: Vec<String>,
    /// Whether each run of the suites has a directory of its own, empty, as its working directory
    /// (`sandbox`, Mill's `testSandboxWorkingDir`): `sandbox_directory`, emptied before each run.
    pub sandbox: bool,
    /// A Scala.js test link's module kind (`moduleKind`: `NoModule`, `CommonJSModule`, `ESModule`).
    pub module_kind: Option<String>,
    /// The JavaScript environment a Scala.js configuration's suites run in (`jsEnv`).
    pub js_env: Option<JsEnv>,
}

/// How many threads a JVM test configuration's suites run on, a pool of sbt's forked test worker
/// (`ForkTestMain.executorService`): `parallelism` is `auto`, the processors the runner's JVM counts at each run,
/// as sbt 2 does by default (`testForkedParallel := true`, `testForkedParallelism := None`), or a count, 1 for
/// `testForkedParallel := false`; the field's absence is `auto`.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Parallelism {
    #[default]
    Auto,
    Threads(u32),
}

impl Parallelism {
    /// The field's value, which is `auto` or a positive Java `int`.
    fn read(value: Option<&Value>) -> Result<Parallelism, String> {
        let shown = match value {
            None => return Ok(Parallelism::Auto),
            Some(Value::Str(s)) if s == "auto" => return Ok(Parallelism::Auto),
            Some(Value::Int(n)) => match u32::try_from(*n) {
                Ok(n) if (1..=i32::MAX as u32).contains(&n) => return Ok(Parallelism::Threads(n)),
                _ => n.to_string(),
            },
            Some(Value::Str(s)) => {
                let mut quoted = String::new();
                lock::quote(s, &mut quoted);
                quoted
            }
            Some(Value::Bool(b)) => b.to_string(),
            Some(Value::List(_)) => "a list".to_string(),
            Some(Value::Map(_)) => "a mapping".to_string(),
        };
        Err(format!("is auto or a number of threads from 1 to {}, not {}", i32::MAX, shown))
    }

    /// The field as the runner's `run` command carries it.
    pub fn field(self) -> String {
        match self {
            Parallelism::Auto => "auto".to_string(),
            Parallelism::Threads(n) => n.to_string(),
        }
    }
}

/// A Scala.js test configuration's JavaScript environment: its kind (`node`, `jsdom`, or the build
/// tool's name of another), the executable, its arguments and environment variables; where sbt-teq
/// exports them, `NodeJSEnv`'s source-map policy (`sourceMap`: `disable`, `enable`,
/// `enableIfAvailable`) and, for jsdom, the package root it is required from and the lockfile that
/// installs it there (`packageRoot`, `lockfile`).
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct JsEnv {
    pub kind: String,
    pub executable: Option<String>,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub source_map: Option<String>,
    pub package_root: Option<String>,
    pub lockfile: Option<String>,
}

impl TestContext {
    /// The environment variables a run of the suites is given: `envVars`, then `envPaths` resolved
    /// under the build's root.
    pub fn env(&self, export: &Export) -> BTreeMap<String, String> {
        let mut env = self.env_vars.clone();
        env.extend(export.env_paths(&self.env_paths));
        env
    }

    /// How teq runs a Scala.js configuration's link, as sbt-scalajs does: its module kind and an
    /// environment of node or jsdom, jsdom installed by a lockfile at its package root; else why not.
    pub fn js_test(&self) -> Result<(ModuleKind, &JsEnv), String> {
        let (Some(kind), Some(env)) = (self.module_kind.as_deref(), self.js_env.as_ref()) else {
            return Err("the export does not say how the build tool runs its tests (moduleKind, jsEnv): export it again".to_string());
        };
        let kind = ModuleKind::parse(kind).ok_or_else(|| format!("its test link's module kind is {}, not NoModule, CommonJSModule or ESModule", kind))?;
        match env.kind.as_str() {
            "node" => {}
            "jsdom" if env.lockfile.as_deref().is_some_and(|l| !l.is_empty()) => {}
            "jsdom" => return Err("its suites run under jsdom, and the build's root holds no lockfile of npm, yarn or pnpm that installs jsdom".to_string()),
            other => return Err(format!("its suites run in the JavaScript environment {}: teq runs them under node and under jsdom alone", other)),
        }
        Ok((kind, env))
    }
}

/// `scalaJSLinkerConfig`'s `ModuleKind`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModuleKind {
    NoModule,
    CommonJs,
    EsModule,
}

impl ModuleKind {
    pub fn parse(name: &str) -> Option<ModuleKind> {
        match name {
            "NoModule" => Some(ModuleKind::NoModule),
            "CommonJSModule" => Some(ModuleKind::CommonJs),
            "ESModule" => Some(ModuleKind::EsModule),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ModuleKind::NoModule => "NoModule",
            ModuleKind::CommonJs => "CommonJSModule",
            ModuleKind::EsModule => "ESModule",
        }
    }
}

/// A Scala.js test configuration's link (`Export::test_link`): the root of its entry point and
/// the file it is written to.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TestLink {
    pub harness: PathBuf,
    pub out: PathBuf,
}

/// The entry point of a Scala.js test link: the start of Scala.js's test bridge, as sbt-scalajs's
/// module initializer of a test link calls it (`TestAdapterInitializer`), in a package of teq's own,
/// since a program with a file in a Scala.js package keeps the std's facades out.
pub const HARNESS_MAIN: &str = "dev.teq.test.TestMain";
const HARNESS_FILE: &str = "dev/teq/test/TestMain.scala";
const HARNESS_SOURCE: &str = "package dev.teq.test\n\n// The entry point of teq's link of a Scala.js test configuration (`teq test`).\nobject TestMain:\n  def main(args: Array[String]): Unit = org.scalajs.testing.bridge.Bridge.start()\n";

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
    /// `wunusedImports`, scalac's `-Wunused:imports` (or `all`): `--wunused imports`, where an
    /// export older than `wunused` gives it alone.
    pub wunused_imports: bool,
    /// `wunused`, the kinds of scalac's `-Wunused` options: `--wunused <kinds>`.
    pub wunused: Vec<String>,
    /// `deprecation`, `feature`, `wtostringInterpolated`, `wconf` and `language`: scalac's
    /// `-deprecation`, `-feature`, `-Wtostring-interpolated`, `-Wconf` and `-language`.
    pub deprecation: bool,
    pub feature: bool,
    pub wtostring_interpolated: bool,
    pub wconf: Vec<String>,
    pub language: Vec<String>,
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
        if !self.language.is_empty() {
            args.extend(["--language".to_string(), self.language.join(",")]);
        }
        if !self.wunused.is_empty() {
            args.extend(["--wunused".to_string(), self.wunused.join(",")]);
        } else if self.wunused_imports {
            args.extend(["--wunused".to_string(), "imports".to_string()]);
        }
        for (set, flag) in [(self.deprecation, "--deprecation"), (self.feature, "--feature"), (self.wtostring_interpolated, "--wtostring-interpolated")] {
            if set {
                args.push(flag.to_string());
            }
        }
        for rules in &self.wconf {
            args.extend(["--wconf".to_string(), rules.clone()]);
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

/// The build tool whose plugin wrote an export, the lock's `exporter` (sbt when absent, as in every lock
/// sbt-teq writes): which files are its build definition, the command that writes the export again, and
/// the tool that runs what teq cannot.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Exporter {
    #[default]
    Sbt,
    Mill,
}

/// The release of mill-teq this compiler names in the command that writes a Mill build's export again, its
/// own version line (`integrations/mill/plugin-version.txt`, which build.rs bakes in).
pub const MILL_PLUGIN_VERSION: &str = env!("TEQ_MILL_PLUGIN_VERSION");

/// The file at a Mill build's root where it declares what Mill cannot tell the export (docs/TOOLING.md, "The
/// export of a Mill build"): one of the build's inputs.
pub const MILL_OVERLAY: &str = "teq.mill.json";

impl Exporter {
    pub fn name(self) -> &'static str {
        match self {
            Exporter::Sbt => "sbt",
            Exporter::Mill => "mill",
        }
    }

    /// How a repository keeps the tool's build definition files LF in every checkout (`LF_REMEDY`,
    /// `MILL_LF_REMEDY`).
    pub fn lf_remedy(self) -> &'static str {
        match self {
            Exporter::Sbt => LF_REMEDY,
            Exporter::Mill => MILL_LF_REMEDY,
        }
    }

    /// The command that writes the export again, run at the build's root.
    pub fn command(self) -> String {
        match self {
            Exporter::Sbt => "sbt teqExportAll".to_string(),
            Exporter::Mill => format!("mill --import mvn:build.teq::mill-teq::{} dev.teq.mill.Teq/teqExportAll", MILL_PLUGIN_VERSION),
        }
    }
}

/// The build tool that runs a generator teq cannot, the generator's kind: `sbt` (sbt-teq's) or `mill`
/// (mill-teq's: a task of the build that produces sources or resources); none for one teq runs.
pub fn run_by(generator: &Value) -> Option<&str> {
    generator.get("kind").and_then(Value::str).filter(|k| matches!(*k, "sbt" | "mill"))
}

/// Whether a generator is one the build tool runs and teq cannot: of kind `sbt` or `mill`.
pub fn by_build_tool(generator: &Value) -> bool {
    run_by(generator).is_some()
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
        let declared = strings(value.at(&["inputs", "declared"]));
        let exporter = match value.get("exporter").map(Value::str) {
            None | Some(Some("sbt")) => Exporter::Sbt,
            Some(Some("mill")) => Exporter::Mill,
            Some(other) => return Err(fail(format!("exporter is sbt or mill, not {}: run the teq the lock names, {}", other.unwrap_or("a mapping or a list"), version))),
        };
        let mut projects = BTreeMap::new();
        for (name, p) in value.get("projects").map_or(&[][..], Value::entries) {
            projects.insert(name.clone(), project(p).map_err(|e| fail(e.message(text, name)))?);
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
            declared,
            exporter,
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
                for entry in conf.classpath.iter().chain(conf.runtime_classpath.iter().flatten()) {
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
    /// closure's sources are all teq's to generate (a test generator the build tool runs leaves the
    /// compile configuration to type); the compile configuration itself where that one is not
    /// transitive (a Mill module's), which the test's resident would show the test's class path.
    pub fn widest(&self, project: &str) -> Option<Key> {
        let p = self.projects.get(project)?;
        if p.configurations.get("compile").is_some_and(|c| !c.transitive) {
            return Some(Key::new(project, "compile"));
        }
        let present: Vec<Key> = ["test", "compile"].into_iter().filter(|c| p.configurations.contains_key(*c)).map(|c| Key::new(project, c)).collect();
        present.iter().find(|k| self.typable(k)).or(present.first()).cloned()
    }

    /// Whether a resident may type a configuration a verb asks for: always for a transitive one,
    /// which sees what its products build on as sbt's configurations do; for one that is not (a
    /// Mill module's), its own resident alone, since another's one program holds what its class path
    /// does not name.
    pub fn serves(&self, resident: &Key, wanted: &Key) -> bool {
        resident == wanted || self.configuration(wanted).is_none_or(|c| c.transitive)
    }

    /// Whether one resident may type a configuration for teq: no configuration of its
    /// closure has a source generator the build tool runs.
    pub fn typable(&self, key: &Key) -> bool {
        self.tool_generated(&self.closure(key)).is_empty()
    }

    /// The source generators of a closure's configurations that the build tool runs, by configuration.
    fn tool_generated(&self, closure: &[Key]) -> Vec<(Key, &Value)> {
        closure.iter().filter_map(|k| Some((k, self.configuration(k)?))).flat_map(|(k, c)| c.generators.iter().filter(|g| by_build_tool(g)).map(move |g| (k.clone(), g))).collect()
    }

    /// Why a verb cannot run a project, as the export records it: the generators the build tool runs of the
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
        for key in &typed {
            for k in self.closure(key) {
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
            for g in conf.generators.iter() {
                let Some(tool) = run_by(g) else { continue };
                let remedy = match (key.configuration.as_str(), tool) {
                    ("compile", "sbt") => format!("make it a TeqCommand of Compile / teqGenerators, or build {} with sbt", key.project),
                    ("compile", _) => format!("declare a command that writes its sources for {} in {}, or build {} with {}", key.project, MILL_OVERLAY, key.project, tool),
                    _ => format!("build {} with {}", key.project, tool),
                };
                let task = g.get("task").and_then(Value::str).unwrap_or("an unnamed task");
                match g.get("reason").and_then(Value::str) {
                    Some(reason) => add(format!("{} has the source generator {}, which {} runs and teq cannot ({}): {}", key, task, tool, reason, remedy)),
                    None => add(format!("{} has the source generator {}, which {} runs and teq cannot: {}", key, task, tool, remedy)),
                }
            }
            if resources {
                for g in conf.resource_generators.iter() {
                    let task = g.get("task").and_then(Value::str).unwrap_or("an unnamed task");
                    let tool = run_by(g).unwrap_or(self.exporter.name());
                    add(format!("{} has the resource generator {}, which {} runs and teq cannot: make its output a resource directory of the build, or {} {} with {}", key, task, tool, verb, key.project, tool));
                }
            }
        }
        let tool = self.exporter.name();
        // What a resident's one program shows a configuration typed in it that its build tool keeps out of
        // its sight (`transitive: false`): the resident's sources and class path are one union, so such a
        // configuration sees a prerequisite of one it names that its class path leaves out (Mill's
        // `compileModuleDeps` of an upstream), and it is typed without every jar
        // of its own class path whose module the resident's does not name at all (Mill's `compileMvnDeps`
        // of an upstream module, which its dependents' class paths leave out): scalac compiles the two
        // apart, each against its own. Refused until configurations have compilation scopes of their own.
        // What sbt's one program shows its configurations is not: a configuration of the closure none it
        // names builds on (a sibling), a jar of the resident's class path its own does not name (a
        // dependent's library, a test's framework) and a module the resident names at another version
        // (the dependent's resolution evicting the upstream's), which sbt's configurations, typed in
        // their dependents' residents, see as well.
        for resident in typed.iter().filter(|k| k.configuration != "runtime") {
            let Some(union) = self.configuration(resident) else { continue };
            let closure = self.closure(resident);
            let union_jars = jars_of(&union.classpath);
            let union_modules: Vec<Option<&str>> = union_jars.iter().map(|j| artifact_module(j)).filter(Option::is_some).collect();
            for key in closure.iter().filter(|k| self.configuration(k).is_some_and(|c| !c.transitive)) {
                let conf = self.configuration(key).expect("present");
                let named = products(&conf.classpath);
                let builds_on_it = |k: &Key| k != key && self.closure(k).contains(key);
                // A prerequisite of a configuration it names that its class path leaves out (an
                // upstream's `compileModuleDeps`); a configuration none of them builds on (a sibling
                // under the resident's project) it sees as an sbt configuration sees its siblings.
                for hidden in closure.iter().filter(|k| *k != key && !named.contains(k) && !builds_on_it(k)) {
                    let Some(through) = named.iter().find(|n| self.closure(n).contains(hidden)) else { continue };
                    add(format!("{} does not see {}, which {} builds on: teq types {} as one program with every configuration it builds on, where {} would be seen; {} {} with {}", key, hidden, through, resident, hidden, verb, resident.project, tool));
                }
                let own = jars_of(&conf.classpath);
                let lacking: Vec<&String> = own.iter().filter(|j| !union_jars.contains(*j) && !union_modules.contains(&artifact_module(j))).collect();
                if let Some(first) = lacking.first() {
                    let more = match lacking.len() {
                        1 => String::new(),
                        2 => format!(" and {}", lacking[1]),
                        n => format!(", {} and {} more", lacking[1], n - 2),
                    };
                    add(format!(
                        "{} is typed inside {}, whose class path lacks {}{}, which {}'s names: teq types a configuration with those that build on it as one program until configurations have compilation scopes of their own; {} {} with {}",
                        key, resident, first, more, key, verb, resident.project, tool
                    ));
                }
            }
        }
        // The products a run needs that no build of the verb's makes: those of the classpath the suites
        // or the program run on outside the closure the resident types (a runtime-only dependency).
        let runs: Option<(Key, Key)> = match verb {
            "test" => Some((Key::new(project, "test"), Key::new(project, "test"))),
            "run" | "stage" | "dev" => Some((Key::new(project, "runtime"), Key::new(project, "compile"))),
            _ => None,
        };
        if let Some((running, built)) = runs.filter(|(r, b)| self.configuration(r).is_some() && self.configuration(b).is_some()) {
            let closure = self.closure(&built);
            for needed in products(self.configuration(&running).expect("present").run_classpath()).into_iter().filter(|k| !closure.contains(k)) {
                add(format!("{} runs on {}'s products, which {}'s closure does not build (a dependency at run time alone): {} {} with {}", running, needed, built, verb, project, tool));
            }
        }
        for why in p.unsupported.get(verb).into_iter().flatten() {
            add(format!("{}: {}", project, why));
        }
        if verb == "test" && p.platform == Platform::Js {
            if let Some(test) = p.configurations.get("test").and_then(|c| c.test.as_ref()).filter(|t| !t.frameworks.is_empty()) {
                match test.js_test() {
                    Err(why) => add(format!("{}: {}", project, why)),
                    Ok((kind, env)) if env.kind == "jsdom" && kind != ModuleKind::NoModule => add(format!(
                        "{}: its suites run under JSDOMNodeJSEnv, which runs a script alone, and the test link is a {} ({} refuses the pair too)",
                        project,
                        kind.name(),
                        match self.exporter {
                            Exporter::Sbt => "sbt-scalajs",
                            Exporter::Mill => "Mill's Scala.js worker",
                        }
                    )),
                    Ok(_) => {}
                }
            }
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

    /// The build's inputs as they are now, with the digests of their contents: the definition files
    /// of the tool that wrote the export (`input_digests`) and the inputs it declares.
    pub fn current_inputs(&self) -> BTreeMap<String, String> {
        input_digests(&self.root, self.exporter, &self.declared)
    }

    /// What changed in the build definition since the export; nothing when it is current. A file added
    /// that git ignores (Metals' `metals.sbt`) is a tool's, not the build's, and does not count.
    pub fn stale_inputs(&self) -> Stale {
        let now = self.current_inputs();
        let mut stale = Stale { exporter: self.exporter, ..Stale::default() };
        let mut added = Vec::new();
        for (file, digest) in &now {
            match self.inputs.get(file) {
                Some(d) if d == digest => {}
                Some(d) => {
                    stale.changes.push(format!("{} changed", file));
                    if line_ends_alone(&self.path(file), d) {
                        stale.line_ends.push(file.clone());
                    }
                }
                None => added.push(file.as_str()),
            }
        }
        let ignored = git_ignored(&self.root, &added);
        stale.changes.extend(added.iter().filter(|f| !ignored.contains(**f)).map(|f| format!("{} added", f)));
        stale.changes.extend(self.inputs.keys().filter(|f| !now.contains_key(*f)).map(|f| format!("{} removed", f)));
        stale
    }

    /// Variables whose values are paths under the build's root, each the paths absolute and joined
    /// by `;` (`envPaths`).
    pub fn env_paths(&self, paths: &BTreeMap<String, Vec<String>>) -> BTreeMap<String, String> {
        paths.iter().map(|(k, ps)| (k.clone(), ps.iter().map(|p| self.path(p).to_string_lossy().into_owned()).collect::<Vec<_>>().join(";"))).collect()
    }

    /// The working directory of a test configuration's runs when it has one of its own (`sandbox`),
    /// beside its class directory.
    pub fn sandbox_directory(&self, key: &Key) -> PathBuf {
        self.path(&format!("target/teq/{}/{}/sandbox", key.project, key.configuration))
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
        } else if let Some(link) = self.test_link(key) {
            args.extend(["--analysis-version", "3", "--main", HARNESS_MAIN, "-o"].map(str::to_string));
            args.push(self.relative(&link.out));
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
        if let Some(link) = self.test_link(key) {
            inputs.push(link.harness);
        }
        inputs
    }

    /// The test link of a Scala.js project's test configuration with frameworks whose run the export
    /// describes (`moduleKind`, sbt-teq's and mill-teq's alike): the session of the configuration links the program whose entry
    /// point starts Scala.js's test bridge (`HARNESS_SOURCE`, under `harness`, one of the session's
    /// inputs) into one file, named for the module kind node loads it as (`.cjs` is CommonJS and
    /// `.mjs` an ES module whatever a `package.json` above says).
    pub fn test_link(&self, key: &Key) -> Option<TestLink> {
        let project = self.projects.get(&key.project)?;
        let test = project.configurations.get(&key.configuration)?.test.as_ref()?;
        let module_kind = ModuleKind::parse(test.module_kind.as_deref()?)?;
        if project.platform != Platform::Js || key.configuration != "test" || test.frameworks.is_empty() {
            return None;
        }
        let dir = self.path(&format!("target/teq/{}/{}", key.project, key.configuration));
        let name = match module_kind {
            ModuleKind::NoModule => "main.js",
            ModuleKind::CommonJs => "main.cjs",
            ModuleKind::EsModule => "main.mjs",
        };
        Some(TestLink { harness: dir.join("harness"), out: dir.join("link").join(name) })
    }

    /// The test link's entry point written under its harness root, unless it is there already.
    pub fn write_harness(&self, link: &TestLink) -> Result<(), String> {
        let file = link.harness.join(HARNESS_FILE);
        if std::fs::read_to_string(&file).is_ok_and(|t| t == HARNESS_SOURCE) {
            return Ok(());
        }
        let dir = file.parent().expect("a file under the root");
        std::fs::create_dir_all(dir).and_then(|()| std::fs::write(&file, HARNESS_SOURCE)).map_err(|e| format!("cannot write {}: {}", file.display(), e))
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

/// The jars and files a classpath names, each by its key or its path, in its order.
fn jars_of(classpath: &[Entry]) -> Vec<String> {
    classpath
        .iter()
        .filter_map(|e| match e {
            Entry::Artifact(key) => Some(key.clone()),
            Entry::File(path) => Some(path.clone()),
            Entry::Product(_) => None,
        })
        .collect()
}

/// The module of an artifact's coordinates (`group:artifact` of `group:artifact:version`), which a
/// file's path is not.
fn artifact_module(jar: &str) -> Option<&str> {
    let (module, _) = jar.rsplit_once(':')?;
    (module.matches(':').count() == 1 && !jar.contains(['/', '\\'])).then_some(module)
}

/// The configurations whose products a classpath names, in its order.
fn products(classpath: &[Entry]) -> Vec<Key> {
    classpath.iter().filter_map(|e| if let Entry::Product(k) = e { Some(k.clone()) } else { None }).collect()
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
    /// The tool that wrote the export, whose files the note's remedy names.
    pub exporter: Exporter,
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
        Some(format!("{} {} from {}'s record by line ends alone, LF against CRLF: {}", self.line_ends.join(", "), verb, FILE, self.exporter.lf_remedy()))
    }
}

/// How a repository keeps the build definition files LF in every checkout, so that a lock
/// exported from one checkout is current in another, and exports again a lock recorded from CRLF:
/// the remedy the note on a file that differs by line ends alone gives, and sbt-teq's export when
/// it reads CRLF in a Git work tree (`Export.LfRemedy`).
pub const LF_REMEDY: &str = ".gitattributes keeps the build's files LF in every checkout with *.sbt text eol=lf, project/**/*.scala text eol=lf and project/build.properties text eol=lf: add them, delete the files and check them out again, then export again";

/// `LF_REMEDY` for a Mill build, mill-teq's export's when it reads CRLF (`Inputs.LfRemedy`).
pub const MILL_LF_REMEDY: &str = ".gitattributes keeps the build's files LF in every checkout with *.mill text eol=lf, mill-build/** text eol=lf, .mill-version text eol=lf and teq.mill.json text eol=lf: add them, delete the files and check them out again, then export again";

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

/// The files among `files` (paths relative to `root`) that git ignores. None without git on the path or
/// outside a repository.
fn git_ignored(root: &Path, files: &[&str]) -> BTreeSet<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    if files.is_empty() {
        return BTreeSet::new();
    }
    let Ok(mut child) = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["check-ignore", "--stdin", "-z"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return BTreeSet::new();
    };
    if let Some(mut stdin) = child.stdin.take() {
        for file in files {
            if stdin.write_all(file.as_bytes()).and_then(|_| stdin.write_all(b"\0")).is_err() {
                break;
            }
        }
    }
    let Ok(out) = child.wait_with_output() else {
        return BTreeSet::new();
    };
    out.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()).map(|s| String::from_utf8_lossy(s).into_owned()).collect()
}

/// The digests of a build's inputs, by path relative to the root with `/`: its definition files as the
/// tool that exported it finds them (`sbt_definition_files`, `mill_definition_files`), and the files among
/// `declared` that exist.
pub fn input_digests(root: &Path, exporter: Exporter, declared: &[String]) -> BTreeMap<String, String> {
    let mut files = Vec::new();
    match exporter {
        Exporter::Sbt => sbt_definition_files(root, &mut files),
        Exporter::Mill => mill_definition_files(root, &mut files),
    }
    files.extend(declared.iter().map(|d| root.join(d)).filter(|f| f.is_file()));
    files
        .into_iter()
        .filter_map(|f| {
            let relative = f.strip_prefix(root).ok()?.to_string_lossy().replace('\\', "/");
            Some((relative, sha256::hex(&std::fs::read(&f).ok()?)))
        })
        .collect()
}

/// An sbt build's definition files: every `.sbt` at the root, `project/build.properties`, and every `.sbt`
/// and `.scala` under `project/` outside its `target` and hidden directories (sbt-teq's
/// `Export.definitionFiles`).
fn sbt_definition_files(root: &Path, out: &mut Vec<PathBuf>) {
    out.extend(std::fs::read_dir(root).map(|es| es.flatten().map(|e| e.path()).filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "sbt")).collect::<Vec<_>>()).unwrap_or_default());
    let properties = root.join("project/build.properties");
    if properties.is_file() {
        out.push(properties);
    }
    definition_files(&root.join("project"), out);
}

/// A Mill build's root build file: Mill 1.1's first (`build.mill.yaml`, then `build.mill`), else Mill 1.0's
/// (`build.mill.scala`, `build.sc`); none for a directory that is no Mill build.
pub fn mill_build_file(root: &Path) -> Option<PathBuf> {
    ["build.mill.yaml", "build.mill", "build.mill.scala", "build.sc"].into_iter().map(|f| root.join(f)).find(|f| f.is_file())
}

/// The extensions of a Mill build's scripts: `.mill` and `.mill.yaml` for a `build.mill` or `build.mill.yaml`
/// build (Mill 1.0 reads the first, Mill 1.1 both), `.mill.scala` or `.sc` for a build whose root file is
/// `build.mill.scala` or `build.sc` (Mill 1.0 alone); none for a directory that is no Mill build.
fn mill_extensions(root: &Path) -> &'static [&'static str] {
    if root.join("build.mill").is_file() || root.join("build.mill.yaml").is_file() {
        &["mill", "mill.yaml"]
    } else if root.join("build.mill.scala").is_file() {
        &["mill.scala"]
    } else if root.join("build.sc").is_file() {
        &["sc"]
    } else {
        &[]
    }
}

/// The files at a Mill build's root that choose its launcher's Mill and JVM and their options.
const MILL_LAUNCHER_FILES: [&str; 6] = [".mill-version", ".config/mill-version", ".mill-jvm-version", ".config/mill-jvm-version", ".mill-jvm-opts", ".mill-opts"];

/// A Mill build's definition files, as Mill 1.0's `FileImportGraph.walkBuildFiles` and Mill 1.1's
/// `BuildFileDiscovery.walkBuildFiles` find the build's scripts, the files of either (mill-teq's
/// `Inputs.definitionFiles` the same): every `package.<ext>` of a directory reached from the root through
/// directories that hold one, `out/` and `mill-build/` left out, and every file whose name ends in `.<ext>`
/// beside the root's build file or a `package.<ext>`, for each extension of the build's (`mill_extensions`).
/// Then every file of the meta-build under `mill-build/` outside its hidden directories, the launcher's files
/// (`MILL_LAUNCHER_FILES`) and the overlay, `teq.mill.json`. None without a root build file. (A build that allows
/// nested build files, Mill 1.1's `mill-allow-nested-build-mill`, is refused by the export.)
fn mill_definition_files(root: &Path, out: &mut Vec<PathBuf>) {
    let exts = mill_extensions(root);
    if exts.is_empty() {
        return;
    }
    let packages: Vec<String> = exts.iter().map(|e| format!("package.{}", e)).collect();
    let suffixes: Vec<String> = exts.iter().map(|e| format!(".{}", e)).collect();
    let mut dirs = vec![root.to_path_buf()];
    let mut i = 0;
    while i < dirs.len() {
        let mut below: Vec<PathBuf> = std::fs::read_dir(&dirs[i]).map(|es| es.flatten().map(|e| e.path()).collect()).unwrap_or_default();
        below.retain(|p| p.is_dir() && packages.iter().any(|n| p.join(n).is_file()) && !(i == 0 && (p.ends_with("out") || p.ends_with("mill-build"))));
        below.sort();
        dirs.extend(below);
        i += 1;
    }
    for dir in &dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        out.extend(entries.flatten().map(|e| e.path()).filter(|p| p.is_file() && p.file_name().is_some_and(|n| suffixes.iter().any(|s| n.to_string_lossy().ends_with(s.as_str())))));
    }
    meta_build_files(&root.join("mill-build"), out);
    out.extend(MILL_LAUNCHER_FILES.iter().chain([&MILL_OVERLAY]).map(|f| root.join(f)).filter(|f| f.is_file()));
}

/// Every file under a Mill meta-build's directory outside its hidden directories and its `out/`.
fn meta_build_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if entry.file_type().is_ok_and(|k| k.is_dir()) {
            if name != "out" && !name.starts_with('.') {
                meta_build_files(&entry.path(), out);
            }
        } else if entry.path().is_file() {
            out.push(entry.path());
        }
    }
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

/// Why a project's entry of the lock is refused, and the keys below the project's of the field it is about, whose
/// line the reader names (`lock::line_of`); none where the message names its place itself.
struct Refusal {
    keys: Vec<String>,
    why: String,
}

impl From<String> for Refusal {
    fn from(why: String) -> Refusal {
        Refusal { keys: Vec::new(), why }
    }
}

impl Refusal {
    /// A refusal of a field, its keys below the node being read.
    fn of(keys: &[&str], why: String) -> Refusal {
        Refusal { keys: keys.iter().map(|k| k.to_string()).collect(), why }
    }

    /// The refusal of a node read under `field` and `key`.
    fn within(mut self, field: &str, key: &str) -> Refusal {
        if self.keys.is_empty() {
            self.why = format!("{}.{}: {}", field, key, self.why);
        } else {
            self.keys.splice(0..0, [field.to_string(), key.to_string()]);
        }
        self
    }

    /// The message of a refusal of the project `name` of the lock's `text`.
    fn message(self, text: &str, name: &str) -> String {
        if self.keys.is_empty() {
            return format!("projects.{}: {}", name, self.why);
        }
        let path: Vec<&str> = ["projects", name].into_iter().chain(self.keys.iter().map(String::as_str)).collect();
        match lock::line_of(text, &path) {
            Some(line) => format!("line {}: {} {}", line, path.join("."), self.why),
            None => format!("{} {}", path.join("."), self.why),
        }
    }
}

fn project(value: &Value) -> Result<Project, Refusal> {
    let platform = match value.get("platform").and_then(Value::str) {
        Some("jvm") => Platform::Jvm,
        Some("js") => Platform::Js,
        other => return Err(format!("platform is jvm or js, not {}", other.unwrap_or("missing")).into()),
    };
    let mut configurations = BTreeMap::new();
    for (name, c) in value.get("configurations").map_or(&[][..], Value::entries) {
        configurations.insert(name.clone(), configuration(c).map_err(|e| e.within("configurations", name))?);
    }
    let dev = match value.get("dev") {
        None => None,
        Some(d) => {
            let text = |key: &str| d.get(key).and_then(Value::str).map(str::to_string).ok_or_else(|| format!("dev has no {}", key));
            let command = strings(d.get("command"));
            if command.is_empty() {
                return Err("dev has no command".to_string().into());
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
            env_paths: string_lists(r.get("envPaths")),
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

/// A map of lists of strings (`envPaths`).
fn string_lists(value: Option<&Value>) -> BTreeMap<String, Vec<String>> {
    value.map_or(&[][..], Value::entries).iter().map(|(k, v)| (k.clone(), strings(Some(v)))).collect()
}

/// A classpath's entries, each a jar's key, a product or a file.
fn classpath(value: Option<&Value>, field: &str) -> Result<Vec<Entry>, String> {
    let mut classpath = Vec::new();
    for (i, e) in value.map_or(&[][..], Value::items).iter().enumerate() {
        classpath.push(entry(e).ok_or_else(|| format!("{}[{}] is neither a jar's key, a product nor a file", field, i))?);
    }
    Ok(classpath)
}

fn configuration(value: &Value) -> Result<Configuration, Refusal> {
    let classpath = classpath(value.get("classpath"), "classpath")?;
    let runtime_classpath = match value.get("runtimeClasspath") {
        Some(v) => Some(self::classpath(Some(v), "runtimeClasspath")?),
        None => None,
    };
    let flags = value.get("flags").map_or(Flags::default(), |f| Flags {
        max_inlines: f.get("maxInlines").and_then(Value::uint),
        strict_equality: f.get("strictEquality").and_then(Value::bool) == Some(true),
        kind_projector: f.get("kindProjector").and_then(Value::bool) == Some(true),
        werror: f.get("werror").and_then(Value::bool) == Some(true),
        wunused_imports: f.get("wunusedImports").and_then(Value::bool) == Some(true),
        wunused: strings(f.get("wunused")),
        deprecation: f.get("deprecation").and_then(Value::bool) == Some(true),
        feature: f.get("feature").and_then(Value::bool) == Some(true),
        wtostring_interpolated: f.get("wtostringInterpolated").and_then(Value::bool) == Some(true),
        wconf: strings(f.get("wconf")),
        language: strings(f.get("language")),
        java_output_version: f.get("javaOutputVersion").and_then(Value::uint),
    });
    let parallelism = Parallelism::read(value.get("parallelism")).map_err(|why| Refusal::of(&["parallelism"], why))?;
    let test = value.get("frameworks").map(|_| TestContext {
        fork: value.get("fork").and_then(Value::bool) == Some(true),
        parallelism,
        base_directory: value.get("baseDirectory").and_then(Value::str).unwrap_or(".").to_string(),
        env_vars: string_map(value.get("envVars")),
        env_paths: string_lists(value.get("envPaths")),
        java_options: strings(value.get("javaOptions")),
        frameworks: value
            .get("frameworks")
            .map_or(&[][..], Value::items)
            .iter()
            .filter_map(|f| Some(Framework { class: f.get("class").and_then(Value::str)?.to_string(), arguments: strings(f.get("arguments")) }))
            .collect(),
        exclude: strings(value.get("exclude")),
        sandbox: value.get("sandbox").and_then(Value::bool) == Some(true),
        module_kind: value.get("moduleKind").and_then(Value::str).map(str::to_string),
        js_env: value.get("jsEnv").map(|e| JsEnv {
            kind: e.get("kind").and_then(Value::str).unwrap_or("node").to_string(),
            executable: e.get("executable").and_then(Value::str).map(str::to_string),
            args: strings(e.get("args")),
            env: string_map(e.get("env")),
            source_map: e.get("sourceMap").and_then(Value::str).map(str::to_string),
            package_root: e.get("packageRoot").and_then(Value::str).map(str::to_string),
            lockfile: e.get("lockfile").and_then(Value::str).map(str::to_string),
        }),
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
        transitive: value.get("transitive").and_then(Value::bool) != Some(false),
        runtime_classpath,
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
        assert_eq!(api.configurations["compile"].flags.args(true), ["--max-inlines", "80", "--wconf", "any:s"]);
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
        let inputs = input_digests(&dir, Exporter::Sbt, &[]);
        assert_eq!(inputs.keys().map(String::as_str).collect::<Vec<_>>(), ["build.sbt", "other.sbt", "project/D.scala", "project/build.properties", "project/project/P.scala"]);
        assert_eq!(inputs["build.sbt"], sha256::hex(b"a"));
        let mut recorded = inputs.clone();
        recorded.insert("project/Gone.scala".to_string(), sha256::hex(b"g"));
        recorded.insert("build.sbt".to_string(), sha256::hex(b"old"));
        recorded.remove("other.sbt");
        let files = recorded.iter().map(|(k, v)| format!("    {}: {}\n", k, v)).collect::<String>();
        std::fs::write(dir.join(FILE), lock(&format!("inputs:\n  files:\n{}", files))).unwrap();
        let export = Export::read(&dir.join(FILE)).unwrap();
        assert_eq!(export.stale_inputs(), Stale { changes: ["build.sbt changed", "other.sbt added", "project/Gone.scala removed"].map(String::from).to_vec(), line_ends: Vec::new(), exporter: Exporter::Sbt });
        assert_eq!(export.stale_inputs().line_end_note(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A Mill build's inputs as `FileImportGraph.walkBuildFiles` finds its scripts: the root's build file and
    /// the `.mill` files beside it, each `package.mill` reached through directories that hold one and the
    /// `.mill` files beside it (not `out/`'s, nor those of a directory without one), the meta-build's files,
    /// the launcher's files, the overlay, and the inputs the lock declares; every change to them read as one,
    /// the files added and removed among them.
    #[test]
    fn a_mill_builds_inputs_and_their_changes() {
        let dir = std::env::temp_dir().join(format!("teq-export-mill-inputs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for d in ["core/js", "core/plain", "docs/nested", "out/core", "mill-build/src", "mill-build/.bsp", ".config"] {
            std::fs::create_dir_all(dir.join(d)).unwrap();
        }
        for (f, t) in [
            ("build.mill", "//| mill-version: 1.0.6\nobject root"),
            ("helper.mill", "def shared = 1"),
            ("build.sbt", "not this build's"),
            ("core/package.mill", "package build.core"),
            ("core/Deps.mill", "object Deps"),
            ("core/js/package.mill", "package build.core.js"),
            ("core/plain/Unreached.mill", "x"),
            ("docs/nested/package.mill", "package build.docs.nested"),
            ("out/core/package.mill", "x"),
            ("mill-build/build.mill", "meta"),
            ("mill-build/src/Plugin.scala", "object Plugin"),
            ("mill-build/.bsp/x.json", "{}"),
            (".mill-version", "1.0.6\n"),
            (".config/mill-jvm-version", "17\n"),
            ("teq.mill.json", "{}"),
            (".tyrian-version", "0.14.0\n"),
            ("notes.txt", "n"),
        ] {
            std::fs::write(dir.join(f), t).unwrap();
        }
        let declared = [".tyrian-version".to_string(), "absent.txt".to_string()];
        let inputs = input_digests(&dir, Exporter::Mill, &declared);
        assert_eq!(
            inputs.keys().map(String::as_str).collect::<Vec<_>>(),
            [
                ".config/mill-jvm-version",
                ".mill-version",
                ".tyrian-version",
                "build.mill",
                "core/Deps.mill",
                "core/js/package.mill",
                "core/package.mill",
                "helper.mill",
                "mill-build/build.mill",
                "mill-build/src/Plugin.scala",
                "teq.mill.json"
            ]
        );
        // The same root read as sbt's: its .sbt alone.
        assert_eq!(input_digests(&dir, Exporter::Sbt, &[]).keys().map(String::as_str).collect::<Vec<_>>(), ["build.sbt"]);
        let write = |recorded: &BTreeMap<String, String>| {
            let files = recorded.iter().map(|(k, v)| format!("    {}: {}\n", k, v)).collect::<String>();
            std::fs::write(dir.join(FILE), lock(&format!("exporter: mill\ninputs:\n  declared:\n    - .tyrian-version\n    - absent.txt\n  files:\n{}", files))).unwrap();
        };
        write(&inputs);
        let export = Export::read(&dir.join(FILE)).unwrap();
        assert_eq!(export.exporter, Exporter::Mill);
        assert!(export.stale_inputs().is_empty(), "{:?}", export.stale_inputs());
        // An edit of a package, a declared input removed, a package added, a helper added, the meta-build edited.
        std::fs::write(dir.join("core/package.mill"), "package build.core // edited").unwrap();
        std::fs::remove_file(dir.join(".tyrian-version")).unwrap();
        std::fs::create_dir_all(dir.join("web")).unwrap();
        std::fs::write(dir.join("web/package.mill"), "package build.web").unwrap();
        std::fs::write(dir.join("core/More.mill"), "object More").unwrap();
        std::fs::write(dir.join("mill-build/src/Plugin.scala"), "object Plugin // edited").unwrap();
        std::fs::write(dir.join("notes.txt"), "not an input").unwrap();
        assert_eq!(
            export.stale_inputs().changes,
            ["core/package.mill changed", "mill-build/src/Plugin.scala changed", "core/More.mill added", "web/package.mill added", ".tyrian-version removed"]
        );
        // A package's directory removed takes its packages and helpers with it.
        std::fs::remove_dir_all(dir.join("core/js")).unwrap();
        assert!(export.stale_inputs().changes.contains(&"core/js/package.mill removed".to_string()));
        // Without its root build file a directory is no Mill build: every recorded file removed.
        std::fs::remove_file(dir.join("build.mill")).unwrap();
        assert!(input_digests(&dir, Exporter::Mill, &[]).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A Mill build's inputs under Mill 1.1's discovery as well as 1.0's (`BuildFileDiscovery.walkBuildFiles`):
    /// a `build.mill.yaml` root, `package.mill.yaml` packages reached as `package.mill` ones are, and `.mill.yaml`
    /// scripts beside them; a `build.mill.scala` build's `.mill.scala` scripts alone (Mill 1.0's). mill-teq's
    /// `InputsSuite` holds the same trees.
    #[test]
    fn a_mill_builds_yaml_and_scala_scripts() {
        let dir = std::env::temp_dir().join(format!("teq-export-mill-yaml-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for d in ["a/b", "c", "d"] {
            std::fs::create_dir_all(dir.join(d)).unwrap();
        }
        for f in ["build.mill.yaml", "helper.mill.yaml", "a/package.mill.yaml", "a/Deps.mill", "a/b/package.mill", "c/package.mill.scala", "d/stray.sc", "x.sc"] {
            std::fs::write(dir.join(f), "x").unwrap();
        }
        let files = |root: &Path| input_digests(root, Exporter::Mill, &[]).into_keys().collect::<Vec<_>>();
        assert_eq!(files(&dir), ["a/Deps.mill", "a/b/package.mill", "a/package.mill.yaml", "build.mill.yaml", "helper.mill.yaml"]);
        assert_eq!(mill_build_file(&dir), Some(dir.join("build.mill.yaml")));
        std::fs::remove_file(dir.join("build.mill.yaml")).unwrap();
        std::fs::write(dir.join("build.mill.scala"), "x").unwrap();
        assert_eq!(files(&dir), ["build.mill.scala", "c/package.mill.scala"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The exporter a lock names: sbt when absent, mill, and nothing else.
    #[test]
    fn the_exporter() {
        assert_eq!(parse(&lock("projects: {}\n")).unwrap().exporter, Exporter::Sbt);
        assert_eq!(parse(&lock("exporter: sbt\nprojects: {}\n")).unwrap().exporter, Exporter::Sbt);
        let mill = parse(&lock("exporter: mill\nprojects: {}\n")).unwrap();
        assert_eq!(mill.exporter, Exporter::Mill);
        assert!(mill.exporter.command().starts_with("mill --import mvn:build.teq::mill-teq::"), "{}", mill.exporter.command());
        let other = parse(&lock("exporter: gradle\nprojects: {}\n")).err().unwrap_or_default();
        assert!(other.contains("exporter is sbt or mill, not gradle"), "{other}");
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
        let one = Stale { changes: vec!["build.sbt changed".to_string()], line_ends: vec!["build.sbt".to_string()], exporter: Exporter::Sbt };
        assert!(one.line_end_note().unwrap().starts_with("build.sbt differs from teq.lock's record by line ends alone, LF against CRLF: .gitattributes keeps"));
        assert!(one.line_ends_only());
        assert!(!Stale { changes: vec!["build.sbt changed".to_string(), "other.sbt added".to_string()], line_ends: vec!["build.sbt".to_string()], exporter: Exporter::Sbt }.line_ends_only());
        // A Mill build's note names its own files' lines.
        let mill = Stale { changes: vec!["build.mill changed".to_string()], line_ends: vec!["build.mill".to_string()], exporter: Exporter::Mill };
        assert!(mill.line_end_note().unwrap().ends_with(MILL_LF_REMEDY) && MILL_LF_REMEDY.contains("*.mill text eol=lf"));
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

    /// A JVM test configuration's `parallelism`: `auto`, a positive Java `int`, or absent (`auto`), anything else
    /// refused before any run with the field's line, a value of a flow mapping by its key's.
    #[test]
    fn the_test_pool_s_size() {
        let read = |field: &str| {
            let text = lock(&format!("projects:\n  a:\n    configurations:\n      test:\n        frameworks: []\n{}    platform: jvm\n", field));
            parse(&text).map(|e| e.projects["a"].configurations["test"].test.as_ref().unwrap().parallelism)
        };
        assert_eq!(read(""), Ok(Parallelism::Auto));
        assert_eq!(read("        parallelism: auto\n"), Ok(Parallelism::Auto));
        assert_eq!(read("        parallelism: 1\n"), Ok(Parallelism::Threads(1)));
        assert_eq!(read("        parallelism: 2147483647\n"), Ok(Parallelism::Threads(i32::MAX as u32)));
        for (value, shown) in [("0", "0"), ("-1", "-1"), ("2147483648", "2147483648"), ("true", "true"), ("\"2\"", "\"2\""), ("Auto", "\"Auto\""), ("[]", "a list"), ("{}", "a mapping")] {
            assert_eq!(
                read(&format!("        parallelism: {}\n", value)),
                Err(format!("/b/teq.lock: line 9: projects.a.configurations.test.parallelism is auto or a number of threads from 1 to 2147483647, not {}", shown)),
                "{}",
                value
            );
        }
        let text = lock("projects:\n  a:\n    configurations:\n      test: {flags: {}, parallelism: 0}\n    platform: jvm\n");
        let refused = parse(&text).err().unwrap_or_default();
        assert!(refused.starts_with("/b/teq.lock: line 7: projects.a.configurations.test.parallelism is auto"), "{}", refused);
        assert_eq!(Parallelism::Threads(3).field(), "3");
        assert_eq!(Parallelism::Auto.field(), "auto");
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

    /// A Mill export's generators (kind `mill`, mill-teq's record of a task of the build that writes sources or
    /// resources): refused as sbt's are, each naming mill, the overlay where a command can stand in for one,
    /// and mill as the tool to build with.
    #[test]
    fn what_mill_alone_runs_is_refused_by_the_verbs_that_need_it() {
        let text = lock(
            "exporter: mill
projects:
  app:
    configurations:
      compile:
        classpath:
          - {configuration: compile, project: tags}
        sources: []
      test:
        classpath:
          - {configuration: compile, project: app}
          - {configuration: compile, project: tags}
        frameworks: []
        generators:
          - {kind: mill, task: app.test.generatedSources}
        resourceGenerators:
          - {kind: mill, task: app.test.resources}
    platform: jvm
  tags:
    configurations:
      compile:
        generators:
          - kind: mill
            reason: \"generatedSources is the build's own task\"
            task: tags.generatedSources
        sources:
          - out/tags/generatedSources.dest
    platform: jvm
",
        );
        let export = parse(&text).unwrap();
        let tags = "tags/compile has the source generator tags.generatedSources, which mill runs and teq cannot (generatedSources is the build's own task): declare a command that writes its sources for tags in teq.mill.json, or build tags with mill";
        assert_eq!(export.refusals("compile", "tags"), [tags]);
        assert_eq!(export.refusals("compile", "app"), [tags]);
        assert!(!export.typable(&Key::new("app", "compile")));
        let test = export.refusals("test", "app");
        assert_eq!(
            test,
            [
                tags,
                "app/test has the source generator app.test.generatedSources, which mill runs and teq cannot: build app with mill",
                "app/test has the resource generator app.test.resources, which mill runs and teq cannot: make its output a resource directory of the build, or test app with mill"
            ]
        );
        let refused = super::super::refused(&export, "compile", &["app"]).unwrap();
        assert!(refused.starts_with("cannot compile app, which needs what mill alone runs:\n  tags/compile has"), "{}", refused);
        assert!(super::super::left_out(&export, "test", &["app".to_string()]).starts_with("teq: test left out app, which needs what mill alone runs:\n"));
    }

    /// The contract of a Mill export's configurations: `transitive: false` (the classpath's products the
    /// whole of what the sources see, a prerequisite beyond them refusing the verbs that type it), a test
    /// configuration's `runtimeClasspath` (what its suites run on; a product its closure does not build
    /// refused), `sandbox` and `envPaths`, and a Scala.js test configuration's `moduleKind` and `jsEnv`.
    #[test]
    fn a_mill_exports_visibility_runs_and_scalajs_tests() {
        let text = lock(
            "exporter: mill
projects:
  a:
    configurations:
      compile:
        classpath:
          - {configuration: compile, project: b}
        transitive: false
    platform: jvm
  b:
    configurations:
      compile:
        transitive: false
    platform: jvm
  c:
    configurations:
      compile:
        classpath:
          - {configuration: compile, project: a}
        transitive: false
      runtime:
        classpath:
          - {configuration: compile, project: c}
          - {configuration: compile, project: a}
          - {configuration: compile, project: r}
      test:
        classpath:
          - {configuration: compile, project: a}
          - {configuration: compile, project: c}
        envPaths:
          MILL_TEST_RESOURCE_DIR:
            - c/test/resources
            - c/test/more
          MILL_WORKSPACE_ROOT:
            - .
        envVars:
          GIVEN: x
        frameworks: []
        runtimeClasspath:
          - {configuration: compile, project: c}
          - {file: lib/run.jar}
          - {configuration: compile, project: r}
        sandbox: true
        transitive: false
    platform: jvm
  r:
    configurations:
      compile: {}
    platform: jvm
  web:
    configurations:
      compile: {}
      test:
        frameworks:
          - class: munit.Framework
        jsEnv:
          args:
            - \"--experimental-vm-modules\"
          kind: jsdom
          lockfile: package-lock.json
        moduleKind: CommonJSModule
    platform: js
",
        );
        let export = parse(&text).unwrap();
        let hidden = "c/compile does not see b/compile, which a/compile builds on: teq types c/compile as one program with every configuration it builds on, where b/compile would be seen; compile c with mill";
        assert_eq!(export.refusals("compile", "c"), [hidden]);
        assert!(export.refusals("compile", "a").is_empty());
        assert_eq!(
            export.refusals("test", "c"),
            [
                hidden.replace("teq types c/compile", "teq types c/test").replace("; compile c", "; test c"),
                "c/test does not see b/compile, which a/compile builds on: teq types c/test as one program with every configuration it builds on, where b/compile would be seen; test c with mill".to_string(),
                "c/test runs on r/compile's products, which c/test's closure does not build (a dependency at run time alone): test c with mill".to_string()
            ]
        );
        assert_eq!(export.refusals("run", "c")[1], "c/runtime runs on r/compile's products, which c/compile's closure does not build (a dependency at run time alone): run c with mill");
        let conf = export.configuration(&Key::new("c", "test")).unwrap();
        assert_eq!(conf.run_classpath().len(), 3);
        assert_eq!(conf.classpath.len(), 2);
        let context = conf.test.as_ref().unwrap();
        assert!(context.sandbox);
        assert_eq!(export.sandbox_directory(&Key::new("c", "test")), Path::new("/b/target/teq/c/test/sandbox"));
        let env = context.env(&export);
        assert_eq!(env["MILL_TEST_RESOURCE_DIR"], "/b/c/test/resources;/b/c/test/more");
        assert_eq!((env["MILL_WORKSPACE_ROOT"].as_str(), env["GIVEN"].as_str()), ("/b", "x"));
        let web = export.configuration(&Key::new("web", "test")).unwrap().test.as_ref().unwrap();
        assert_eq!(web.js_env.as_ref().map(|e| (e.kind.as_str(), e.args.len())), Some(("jsdom", 1)));
        assert_eq!(
            export.refusals("test", "web"),
            ["web: its suites run under JSDOMNodeJSEnv, which runs a script alone, and the test link is a CommonJSModule (Mill's Scala.js worker refuses the pair too)"]
        );
        // Transitive configurations, as sbt's are: their products' prerequisites seen.
        let sbt = parse(&text.replace("exporter: mill\n", "").replace("transitive: false", "transitive: true")).unwrap();
        assert!(sbt.refusals("compile", "c").is_empty());
    }

    /// A configuration that is not transitive (a Mill module's) typed inside another configuration's resident
    /// sees the resident's jars as an sbt configuration typed there does: `leak`'s compile configuration,
    /// typed as `leak/test`, sees munit, which only its test's class path names, and its test runs, while its
    /// compile and watch get a resident of their own (`widest`, `serves`); `lib`'s, typed inside `app`, whose
    /// class path adds a jar, compiles with `app`; `sib1`'s, typed inside `top` beside `sib2`, which neither
    /// names, compiles with it; `same`'s test adds nothing its compile does not name. An sbt export's
    /// configurations, transitive, are typed as before.
    #[test]
    fn a_configuration_typed_inside_another_resident_sees_its_union() {
        let text = lock(
            "exporter: mill
jars:
  x:munit:1: maven-central 0000000000000000000000000000000000000001 1
  x:scala:1: maven-central 0000000000000000000000000000000000000002 1
  x:extra:1: maven-central 0000000000000000000000000000000000000003 1
repositories:
  - id: maven-central
    url: https://repo1.maven.org/maven2/
projects:
  leak:
    configurations:
      compile:
        classpath:
          - x:scala:1
        transitive: false
      test:
        classpath:
          - x:scala:1
          - x:munit:1
          - {configuration: compile, project: leak}
        frameworks: []
        transitive: false
    platform: jvm
  lib:
    configurations:
      compile:
        classpath:
          - x:scala:1
        transitive: false
    platform: jvm
  app:
    configurations:
      compile:
        classpath:
          - x:scala:1
          - x:extra:1
          - {configuration: compile, project: lib}
        transitive: false
    platform: jvm
  sib1:
    configurations:
      compile:
        classpath:
          - x:scala:1
        transitive: false
    platform: jvm
  sib2:
    configurations:
      compile:
        classpath:
          - x:scala:1
        transitive: false
    platform: jvm
  top:
    configurations:
      compile:
        classpath:
          - x:scala:1
          - {configuration: compile, project: sib1}
          - {configuration: compile, project: sib2}
        transitive: false
    platform: jvm
  same:
    configurations:
      compile:
        classpath:
          - x:scala:1
          - x:munit:1
        transitive: false
      test:
        classpath:
          - x:scala:1
          - x:munit:1
          - {configuration: compile, project: same}
        frameworks: []
        transitive: false
    platform: jvm
",
        );
        let export = parse(&text).unwrap();
        assert!(export.refusals("test", "leak").is_empty());
        assert!(export.refusals("compile", "leak").is_empty() && export.refusals("watch", "leak").is_empty());
        assert_eq!(export.widest("leak"), Some(Key::new("leak", "compile")));
        assert!(!export.serves(&Key::new("leak", "test"), &Key::new("leak", "compile")) && export.serves(&Key::new("leak", "compile"), &Key::new("leak", "compile")));
        assert!(export.refusals("compile", "app").is_empty());
        assert!(export.refusals("compile", "top").is_empty());
        assert!(export.refusals("test", "same").is_empty());
        let sbt = parse(&text.replace("exporter: mill\n", "").replace("transitive: false", "transitive: true")).unwrap();
        assert!(sbt.refusals("test", "leak").is_empty() && sbt.refusals("compile", "app").is_empty());
        assert_eq!(sbt.widest("leak"), Some(Key::new("leak", "test")));
        assert!(sbt.serves(&Key::new("leak", "test"), &Key::new("leak", "compile")));
    }

    /// An upstream configuration that is not transitive, typed inside a downstream resident whose class path
    /// lacks a jar of its own (Mill's `compileMvnDeps`, which its dependents' class paths leave out), would not
    /// find what it reads, where scalac compiles it apart against its own class path: the downstream's
    /// compile is refused, naming the two configurations and the jar, while the upstream's compiles alone. A
    /// downstream naming the module at another version (`evicting`, the dependent's resolution) compiles, as
    /// an sbt configuration typed in its dependent's resident does.
    #[test]
    fn a_configuration_typed_inside_a_resident_that_lacks_its_jars_is_refused() {
        let text = lock(
            "exporter: mill
jars:
  x:scala:1: maven-central 0000000000000000000000000000000000000002 1
  x:sourcecode:1: maven-central 0000000000000000000000000000000000000004 1
  x:sourcecode:2: maven-central 0000000000000000000000000000000000000005 1
repositories:
  - id: maven-central
    url: https://repo1.maven.org/maven2/
projects:
  upstream:
    configurations:
      compile:
        classpath:
          - x:scala:1
          - x:sourcecode:1
        transitive: false
    platform: jvm
  downstream:
    configurations:
      compile:
        classpath:
          - x:scala:1
          - {configuration: compile, project: upstream}
        transitive: false
    platform: jvm
  evicting:
    configurations:
      compile:
        classpath:
          - x:scala:1
          - x:sourcecode:2
          - {configuration: compile, project: upstream}
        transitive: false
    platform: jvm
",
        );
        let export = parse(&text).unwrap();
        assert_eq!(
            export.refusals("compile", "downstream"),
            ["upstream/compile is typed inside downstream/compile, whose class path lacks x:sourcecode:1, which upstream/compile's names: teq types a configuration with those that build on it as one program until configurations have compilation scopes of their own; compile downstream with mill"]
        );
        assert!(export.refusals("compile", "upstream").is_empty());
        assert!(export.refusals("compile", "evicting").is_empty());
        let sbt = parse(&text.replace("exporter: mill\n", "").replace("transitive: false", "transitive: true")).unwrap();
        assert!(sbt.refusals("compile", "downstream").is_empty());
    }

    #[test]
    fn scalajs_libraries_as_sbt_teq_takes_them() {
        assert!(is_scalajs_tasty_jar("cats-core_sjs1_3-2.13.0.jar"));
        assert!(!is_scalajs_tasty_jar("cats-core_3-2.13.0.jar"));
        assert!(!is_scalajs_tasty_jar("scalajs-library_2.13-1.22.0.jar"));
        assert!(!is_scalajs_tasty_jar("scala3-library_sjs1_3-3.8.4.jar"));
    }
}
