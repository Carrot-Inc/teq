# Tooling

How teq fits a build and an editor. An sbt build takes teq through its plugin, which puts teq in the place of
sbt's compiler and exports the build into a lockfile, and a Mill build through mill-teq, which exports it into
the same lockfile; from the lockfile `teq`, the vite plugin and the language server work with neither tool
running, and a clone builds with nothing installed. The binary's own commands
and flags are in `docs/CLI.md`, the output and the targets in `docs/TARGETS.md`.

The notices of the code teq adapts from other projects, which a program teq compiles carries along with teq's
standard library, are in `NOTICE` at the repository's root, beside every release's binaries, in the vite package
and in the Zed bundle.

## The sbt plugin

sbt-teq is a plugin for sbt 2 and for sbt 1, on Maven Central. A build switches to teq by adding the plugin and
setting `teqCompiler`; the plugin fetches the compiler for the machine's platform from teq's GitHub release:

```scala
// project/plugins.sbt
addSbtPlugin("build.teq" % "sbt-teq" % "1.0.1")

// build.sbt
teqVersion := "0.1.9"
teqCompiler := true
```

The line is the same under both sbts. Each release of the plugin publishes a module per sbt line at its one
version, `sbt-teq_sbt2_3` for sbt 2 and `sbt-teq_2.12_1.0` for sbt 1 (built against sbt 1.13; its export checked
on 1.12 and 1.13), and sbt resolves the module of the sbt that loads the build, as it resolves every plugin. The two are
built from one source tree: the same tasks and settings, and the same `teqExportAll`, lockfile and launchers for
the same build. sbt 1's module comes with the plugin's first release after 1.0.0, which is sbt 2's alone.

`teqCompiler := true` puts teq in the place of zinc's incremental compiler. For a JVM project `compile`,
`test`, `testOnly`, `run` and `packageBin` then run through sbt's own tasks, with sbt's own reporting, over
teq's class files; for a Scala.js project `compile`, `test`, `testOnly` and the link tasks do, over teq's
JavaScript. zinc keeps its invalidation and hands teq the sources it invalidated, one `teq` process per batch
against the configuration's class directory and the upstream projects' products, and the answer gives zinc
each class's API and dependencies, so that an API change recompiles its dependents alone.

The default of `teqCompiler` reads `TEQ_COMPILER` from the environment as a boolean, so a CI job and a dev
shell can differ, and `set every teqCompiler := true` switches a session. The toggle is a setting of each
project and configuration, so one build can hold both compilers: a project with it off keeps sbt's stock tasks
and its class directory to scalac alone, one with it on reads an upstream project's products, scalac's or
teq's, through TASTy, and scalac reads teq's in the other direction. What stays with scalac is `doc`,
`console` and scalafix, and Java sources, which teq reads as class files only: a configuration with `.java`
sources is refused with an error naming them, and takes `teqCompiler := false` or a project of its own. JUnit
suites are found by their `@Test` methods but not run yet.

sbt 1's `test` runs every suite each time, as sbt 1 runs it (`testQuick` is a task of its own there), and sbt 1
caches no task. sbt 2's `test` is `testQuick`, under either compiler: it runs a suite only if no run has passed it at its
current digest, a hash of the suite's class files, those of the classes it uses, its libraries and its test
options. sbt records the successes in its disk cache, which every build and checkout on the machine shares
(`~/.cache/sbt/v2` on Linux, `~/Library/Caches/sbt/v2` on macOS, `%LOCALAPPDATA%\sbt\v2` on Windows, under
`<dir>` with `-Dsbt.global.localcache=<dir>`) and which `clean` leaves. So after a passing run, `clean` then
`test` compiles again and runs nothing (`No tests to run`), and `test` back on a toggle state whose suites
passed before, or in another checkout of the same code, skips the suites whose digests passed there;
`definedTests` still lists every suite. `testOnly` runs the suites it names, every suite with no name;
`cleanFull` empties the disk cache, every build's. A compile that fails right after its setup changed (a teq
binary or flag, a scalac option) leaves the class directory empty, and once the setup is put back sbt 2 may
answer the next compile from its cache with that empty directory, the classes then missing downstream;
`clean` before compiling again puts them back.

A Scala.js project's `fastLinkJS` and `fullLinkJS` are answered by teq's links into the directories the linker
would write, with the linker's report, so that a Scala.js vite plugin serves teq's output unmodified and
`sbt ~frontend/fastLinkJS` keeps it current. The project's compile is a check, the links are whole-program
from the sources, and sbt-scalajs's own test adapter runs the test configuration's link under node (zio-test,
munit and utest among the frameworks).

`teqBuild` runs the project's source generators and those of the projects it depends on, then one `teq compiler build`
into `target/teq/out`, for CI and production; `set teqRelease := true` adds `--release`, and teq's diagnostics
go to sbt's log and fail the task. A project without the Scala.js plugin builds for the JVM
(`teqTarget := "jvm"`, from sbt's `platform`): class files linked against every jar of
`Compile / externalDependencyClasspath`, which `java` runs beside them.

`teqLinkJS` and `teqFullLinkJS` are the links behind sbt's route to a browser: the dev build (`--split`,
`teqModulePerFile`, `--hot`) into `teqServedOutput`, kept by a resident `teq compiler watch` process (ended
`teqLinkIdle`, a minute, after its last build outside a watch) so that `~teqLinkJS` costs an incremental
build, with a stub `main.js` through which `scalajs:main.js` resolves as for a Scala.js build; and the
`--release` build into `teqFullServedOutput` as one file, `main.js`, for `vite build`. The scalac options teq
has a flag for are mapped, the others listed by the export. The settings, in short:

| Setting | Default | |
|---|---|---|
| `teqCompiler` | `$TEQ_COMPILER` | whether teq is the project's compiler |
| `teqVersion` | the plugin's version | the binary's version |
| `teqBinary` | `$TEQ` | a local binary instead of the resolved one; a bare name is looked up on the `PATH` |
| `teqTarget` | `jvm` on the JVM, else `js` | what teq compiles to |
| `teqMainClass` | none | the main class to run, for a JVM build with several |
| `teqModulePerFile` | none | `--module-per-file` packages |
| `teqHot`, `teqRelease` | false | `--hot`; `--release` in `teqBuild` |
| `teqSources`, `teqClasspath`, `teqScalacOptions` | from the build | the roots, the jars, the options |
| `teqDescriptionKeys` | none | string keys copied into the export for the tools that read it |

**Releases.** The compiler and the plugin have version lines of their own. The compiler is released at each ship
(0.1.7, 0.1.8, ...) as the GitHub release `v<version>` of `github.com/Carrot-Inc/teq`: one binary per platform,
`teq-<version>-<platform>` (`osx-aarch_64`, `osx-x86_64`, `linux-x86_64`, `linux-aarch_64`, `windows-x86_64`, which
alone ends in `.exe`), beside `SHA256SUMS` and `teq-<version>-binaries.txt`, which gives each binary's SHA-256, SHA-1 and size. The plugin
fetches the machine's binary from there into a cache of its own and runs it only once its bytes are the release's
(and the lockfile's, where the build's lock pins that compiler); a verified copy serves offline, nothing else does. The plugin, `build.teq:sbt-teq` on Maven Central, which sbt resolves from with
no resolver line, is released when the plugin changes (from 1.0.0), and its `teqVersion` defaults to the compiler
it was released with. A build names both, the plugin in `project/plugins.sbt` and the compiler by `teqVersion`, and
moves to a new release by naming it, nothing moving by itself: a release never changes, so coursier keeps its file
for good. The releases before 0.1.7 are not served: the plugin refuses a build naming one, and the launchers and
the Zed extension a lock pinning one ("releases before 0.1.7 are not served: pin 0.1.7 or later").

Which plugin serves which compiler is a matter of the protocols the two speak: the analysis graph a compile answers
with (`--analysis-version`; the plugin asks for 3 and reads 2 and 3), the products manifest a class directory keeps
(format 1), the lockfile the export writes (format 1), and the command line of `teq compiler build` and `check`. A
plugin serves the compiler it was released with and every later one that keeps them; a compiler release that changes
one names the first plugin that serves it here, and refuses an older plugin's request naming that plugin, as the
plugin refuses an analysis or a products manifest of a version it does not read before it touches the class
directory.

| sbt-teq | serves teq |
|---|---|
| 1.0.0 | 0.1.7 and later |

## The lockfile and the launchers

`sbt teqExportAll` writes `teq.lock`, the build's one description for every tool that works without sbt, to
`target/teq/teq.lock` under the build's root, adding nothing to the repository. With teq as the build tool,
`teqBuildTool := true`, it writes it at the build's root and beside it the launchers `teq` (POSIX sh)
and `teq.cmd`; the repository commits the three files, `teq` and `teq.lock` with LF line ends and `teq.cmd`
with CRLF in its `.gitattributes`, and keeps the build's own files LF there too (below; a Mill build's lines are in "The export of a Mill build"). Every tool looks
for `teq.lock` at a directory, then for `target/teq/teq.lock` under it. A clone of such a build
then needs no teq installed: `./teq test` reads the lock's first line and its platform's `binaries` line,
finds the pinned binary in teq's cache, else in coursier's, else fetches it with curl, its size and sha1
checked, and runs it with the arguments as given (`./teq lsp` too); `TEQ` in the environment names another
binary. A lock pinning a release before 0.1.7 is refused, and so is one without a binary for the platform (an
empty table, the lock of a release not published yet, which an export fills once it is), naming `TEQ` and the
release to pin. `teq.cmd` does the same on Windows, once a Windows binary is published. An edited launcher is kept by
later exports, which say so.

The lock is a subset of YAML 1.2 that any YAML reader reads, with every path relative to the build's root. Its
first line is the compiler, `teq: <version>`, then `format: 1` and `binaries:`, a line per platform with the
binary's URL, sha1 and size (`binaries: {}` while the release is not published); then `exporter`, the tool whose
plugin wrote it (`mill`; absent, sbt); `inputs`, every build definition file with its SHA-256 (and `declared`, the
inputs a Mill build names besides its scripts); `repositories`, the
build's remote Maven ones and their credentials hosts; `jars`, every jar a classpath names, once, by its key
`organization:name:version[:classifier]` with its repository, sha1 and size; `java.outputVersion`, the class
files' version; and `projects`, each with its platform, Scala version and configurations (the roots, the full
ordered classpath, the flags (the scalac options teq honours, `source` the configuration's `-source`, under which a
session of a closure types that configuration's roots, `--source-at`), the generators, the declared main classes and the test context of `compile`,
`runtime` and `test`, a JVM test context's `parallelism` among it, "Tests." below; and `compile`'s `evictions`,
absent where there is none: each dependency the build declares at a version its resolution did not choose, by its
key, with the key of the jar chosen in its place, which a script's `//> using dep` of the declared version resolves
to under `teq interp`), the `run`, `dev` and `stage`
blocks the verbs read, and the description the plugin's own tasks build from. A Mill build's configurations say
more (below, "The export of a Mill build"): `transitive: false`, the classpath's products the whole of what the
sources see; a test configuration's `runtimeClasspath`, what its suites run on where that is not its classpath,
`sandbox`, a directory of its own for each run, and `envPaths`, variables whose values are paths under the root,
absolute and joined by `;` (a run block's too).

The export compiles nothing and runs none of the build's code: the classpaths come from dependency resolution
alone, so it costs an sbt start. Under the driver CI checks it with

```bash
sbt teqExportAll && git diff --exit-code teq.lock teq teq.cmd
```

and `teq` warns at every start about build files changed since the export (`--strict` refuses). The lock
records the build files' bytes, so a checkout whose line ends differ from the export's (Git for Windows checks
text out with CRLF by default) finds it stale: the warning notes each file that differs by line ends alone, and
the export warns when it reads CRLF. These lines in `.gitattributes` keep the files LF in every checkout; once
they are added, delete the files, check them out again and export again:

```
*.sbt text eol=lf
project/**/*.scala text eol=lf
project/build.properties text eol=lf
```

What sbt would find by compiling or running the build's code is declared:

| Setting | Default | |
|---|---|---|
| `Compile / teqGenerators` | none | `TeqCommand(run, inputs, cwd = ".", outputs)`, a program that writes sources; `Test / teqGenerators` a test configuration's own |
| `teqReplacesSbtGenerators` | false | the configuration's commands stand for every source and resource generator sbt runs for it, which the export then records none of |
| `teqMainClasses` | none | main classes `teq run` takes by name, besides `Compile / mainClass` |
| `teqRunAliases` | none | names for main classes, which `teq run` takes in their place |
| `teqDevCommand` | the `dev` script of the nearest `package.json` | a Scala.js project's dev command |
| `teqBuildTool` | false | the lock and the launchers at the build's root, committed |
| `teqExportSnapshots` | false under `teqBuildTool`, else true | lets the export name SNAPSHOT and dynamic versions |

A `TeqCommand` writes sources into the directory appended to `run` (sbt's `Compile / sourceManaged / "teq"`,
the exported build's `target/teq/<project>/compile/src_managed`, relative to the command's working directory);
sbt's compile runs it, and so does `teq`, again when the files its `inputs` globs match or the files its
arguments name change (its script among them), or when one of its `outputs` the last run made is gone: the files
or directories under the build's root the program writes besides that directory, a module of the application's
assets say, which `teq` checks as it checks the directory and whose unchanged files keep their times; an output
the run leaves empty (that directory, for a command writing elsewhere) is not waited for. The
first word `teq` is the build's own binary, sbt's `teqResolvedBinary` and under `teq` the one running, never
a `teq` of the `PATH`: a Scala script runs through `teq interp`, with no JVM and no other tool on any machine. A
Scala CLI script (`.sc`) runs there as well, its `//> using dep` lines resolved through the build's lock rather
than fetched (docs/CLI.md, "teq interp").

```scala
Compile / teqGenerators += TeqCommand(Seq("teq", "interp", "scripts/images.scala", "--", "app"),
  inputs = Seq("app/assets/images/**/*.svg"), outputs = Seq("app/assets/images.js"))
Compile / teqGenerators += TeqCommand(Seq("node", "generate-labels.mjs", "labels.txt"), inputs = Seq("labels.txt"))
```

sbt-buildinfo's object is
exported as it is, with `gitSha` and a class directory as the dynamic values `teq` computes. A project
with sbt-native-packager's `JavaAppPackaging` gets a `stage` block, the files its `Docker / stage` writes, by
layer, so that `teq stage` writes them with no sbt and the Dockerfile copies its layers as before; the
start script runs `Compile / mainClass` where the build declares it (a build file's or a session's `set`), else
the main class `teq stage` finds (below).

The export refuses no build for what `teq` could not reproduce: it records it, and the verbs that need
it refuse at use, naming each reason and its remedy, while the editor and vite read the rest. A build
whose generators teq can run as commands declares them with `teqReplacesSbtGenerators` (for a
configuration and those extending it): the commands of the configuration then stand for sbt's, which are
not recorded, a configuration with no command of its own asserting that sbt's write nothing there
(bench/lila/overlay/LilaTeq.scala does this for lila's routers). Recorded are a
source generator other than a `TeqCommand` or sbt-buildinfo's of a shape `teq` writes (the editor then
reads what sbt last generated), a generator of the Test or Runtime configuration, a resource generator, a
`Tests.Filter`, `Tests.Setup` or `Tests.Cleanup`, and a Docker stage `teq` would not write as
native-packager does (a layer function that reads a task, native-packager's classpath and launcher jars,
server archetype, ash script and jlink, a start script, classpath or mappings of the build's own); a Scala
Native project is left out. Without the driver the lock is the machine's own, so a SNAPSHOT or dynamic
version, a path outside the build's root and an artifact from none of the build's Maven repositories are
written as they are; under the driver they are refused, every reason named at once (SNAPSHOT and dynamic
versions where `teqExportSnapshots` allows them), as is in both modes a jar key naming two files.

A Scala.js project's test configuration records, beside its frameworks, its link's module kind
(`moduleKind`: `NoModule`, `CommonJSModule` or `ESModule`, the `Test / scalaJSLinkerConfig`'s) and the
JavaScript environment its suites run in (`jsEnv`: its `kind`, `node`, `jsdom` or another environment's name,
and its `executable`, `args` and `env`), which `teq test` runs the suites with (below, under "Tests"). sbt-teq
adds what that run takes besides: `NodeJSEnv`'s source-map policy (`sourceMap`) and, for jsdom, the package root
it is required from and the lockfile there that installs it (`packageRoot`, `lockfile`); a Mill build's are
named when `teq test` refuses the project.

## The export of a Mill build

mill-teq exports a Mill 1.x build (1.0.6 and later) to the same `teq.lock`, from what Mill resolves of its
modules, compiling nothing. It is an external module on Maven Central under a version line of its own
(`build.teq:mill-teq_mill1_3`, `integrations/mill`), which a build runs without naming it:

```bash
mill --import mvn:build.teq::mill-teq::0.1.0 dev.teq.mill.Teq/teqExportAll
```

The lock lands in `target/teq/teq.lock`, or at the build's root with the launchers beside it when
`teq.mill.json` says `"buildTool": true`; the lock's root says `exporter: mill`, which tells the readers its
inputs and the command that exports it again. What sbt-teq's settings declare, `teq.mill.json` at the build's
root declares (`integrations/mill/README.md` has its fields): the driver's mode, SNAPSHOT versions, the compiler
the lock pins, the inputs the build reads besides its scripts (`inputs.declared`), and per module the commands that
write its sources, its main classes and aliases, its dev command and the module that is its test configuration.

The modules map onto teq's projects as sbt's projects do:

- a project per JavaModule the build resolves, its key the module's segments as Mill 1.0 renders them, every
  cross axis and nesting kept (`matrix[3.8.4]`, `cats.js`), its `base` the module's directory; a module's
  direct child `test`, a TestModule, is the project's test configuration, not a project of its own;
- left out, each named with its reason: a Scala Native module, a Scala 2 module, a Java module, a module whose
  sources hold Java files, a test module that is no module's `test` (unless the overlay names it), a module whose
  `compileClasspath`, `resolvedMvnDeps` or `resolvedRunMvnDeps` is the build's own task (told from its definition
  before anything of the module is read), or whose local class path (`compileResources`, `unmanagedClasspath`) the
  build computes, never read as empty, and every module that depends on one of those; a test module is held to the
  same before it is its owner's test configuration, the project keeping its compile and runtime configurations and
  its test refusing with the reason;
- `compile` holds the module's sources and resources and its compile class path in Mill's order (the resolved
  artifacts, then each module of `transitiveModuleCompileModuleDeps` by its local class path and its products,
  then its own local class path), `runtime` its run class path (`resolvedRunMvnDeps`, then
  `transitiveModuleRunModuleDeps`' and its own products); a jar is named by its coordinates, its repository and
  its path there, from Mill's resolution of the module with its own resolver settings (`Jvm.getArtifacts`),
  reconciled with the class path's files;
- `test` holds the test module's, compiled against its compile class path and run on its run class path
  (`runtimeClasspath` where they differ), in a directory of its own emptied before each run when Mill's run has
  one (`sandbox`, `testSandboxWorkingDir`), with `MILL_WORKSPACE_ROOT` and `MILL_TEST_RESOURCE_DIR` the paths
  Mill gives them (`envPaths`), its framework the module's `testFramework`, its suites on one thread
  (`parallelism: 1`), as Mill runs those of one JVM one after another: Mill's `testParallelism` and
  `testForkGrouping` are about JVM processes, which teq's one runner per configuration does not have (grouping is
  not supported), and the default grouping, over `discoveredTestClasses`, would make the export a compile;
- every configuration is `transitive: false`: Mill compiles each module against its own class path alone, its
  `compileModuleDeps` seen by their dependent alone, so the classpath's products and jars are the whole of what its
  sources see. teq types a configuration in a resident of its own (its compile never inside its test's), with every
  configuration it builds on as one program, whose sources and class path are one union: the verbs refuse where a
  configuration of that program would see a configuration its class path does not name (a prerequisite of one it
  names), or would lack a module of its own class path that the union does not name at any version (an upstream's
  `compileMvnDeps`), never flattening, until configurations have compilation scopes of their own. What an sbt
  configuration typed in its dependent's resident sees too, a Mill one sees: a configuration of the union none it
  names builds on (a sibling), the union's jars its own class path does not name (a dependent's libraries, a test's
  framework) and a module the union names at another version (the dependent's resolution evicting the upstream's);
- a `generatedSources` the build defines is never run: a command of the overlay stands for it, which teq runs
  as for an sbt build, else it is recorded as Mill's (a generator of kind `mill`), the task's directory under
  Mill's `out/` (`out/<module>/generatedSources.dest`, whatever Mill last wrote there) the editor's root, and the
  verbs that need it refuse. The export runs no computation of the build's or a plugin's: a `sources`,
  `resources` or local class path task is read only where its definition is static, told before anything runs
  (source tasks, and tasks of Mill's own modules); one the build computes is recorded as Mill's generator, never
  run, the source tasks it reads the editor's roots, and one returning a path under Mill's `out/` is Mill's
  generator too.

The lock's inputs are the build's scripts as Mill 1.0 and 1.1 find them (`FileImportGraph.walkBuildFiles`,
`BuildFileDiscovery.walkBuildFiles`: the root's `build.mill` or `build.mill.yaml`, Mill 1.0's `build.mill.scala`
or `build.sc`, every `package.mill` or `package.mill.yaml` reached through directories that hold one, and every
`.mill` or `.mill.yaml` file beside those), the meta-build's files under `mill-build/`, the launcher's files
(`.mill-version`, `.config/mill-version`, `.mill-jvm-version`, `.mill-jvm-opts`, `.mill-opts`), `teq.mill.json`
and the inputs it declares: an edit to one, a script added and one removed make the lock stale, as for an sbt
build, and the warning names the command above; the language server watches them and refreshes the warning, a
declared input outside the workspace's folders (`../external.txt`) watched from its own directory (a relative
pattern, where the client takes one). A build that allows nested build files (Mill 1.1's
`mill-allow-nested-build-mill`, its header read as YAML as Mill reads it) is refused. `*.mill text eol=lf`,
`mill-build/** text eol=lf`, `.mill-version text eol=lf` and `teq.mill.json text eol=lf` in `.gitattributes` keep
them LF in every checkout.

## `teq`

`teq [--export file] [--strict] <verb> [args]` reads the export at or above the working directory, or the
one named; without a verb it lists the verbs and the projects. A project whose export records what the build
tool alone runs for a verb (sbt's or Mill's: a generator it runs, a dependency teq's one program would show, a
product a run needs that no compile of teq's makes) is refused with the reasons; a verb over every project runs
the others, names each one left out and exits 2.

| Verb | |
|---|---|
| `compile [project...]` | types the projects' closures (every project by default) and writes class files |
| `test [project] [pattern...]` | compiles a project's test configuration and runs its suites, on the JVM or under node |
| `run <project> [alias or main class] [-- args]` | compiles and runs a JVM project's main class |
| `stage [project]` | compiles and writes a project's Docker stage |
| `watch <project> [options]` | becomes the project's resident `teq compiler watch`, for a tool that owns the process |
| `build <project> [options]` | one `teq compiler build` of a Scala.js project, the production build with `--release` |
| `dev <project> [-- args]` | a Scala.js project's dev loop |
| `stop` | ends the daemon, its residents and its test runners |

`compile`, `test`, `stage` and `stop` go through a daemon, which the first client of an export starts: it
listens on a loopback port behind a token only the user who started it can read, and owns the residents, one
`teq compiler watch` process per project configuration over its closure (class files on the JVM, a check on Scala.js).
One resident serves a project's compile, test and run, and an edit that breaks a test file keeps the earlier
class files. An export changed in place is read again, and the daemon ends when the export is gone, pins
another compiler or `target/teq` is cleaned, so that its residents never outlive what they served. `run` has
the daemon compile and runs the program itself, so that it holds the terminal; `watch`, `build` and `dev` are
the client's own. Jars come from coursier's cache when their size and sha1 are the pinned ones, else are
fetched with curl into teq's cache, each announced on stderr.

**Run.** `teq run api -- web` runs the main class the build declares (`Compile / run / mainClass`, else
`Compile / mainClass`), else the single main class among the project's own sources, as sbt's `run` does, and
refuses where sbt would prompt; `teq run api server` runs an alias of `teqRunAliases` or a main class by
its name. The program runs under `JAVA_HOME`'s `java`, else the first on the `PATH`, with the run block's
options, variables and directory; Ctrl-C reaches it, and its exit code is the verb's. With `--diagnostics <file>`
the build's records are written there (below) and, once the build passed, the program's start (severity `run`): the
program's output follows teq's own lines on the same channels, and a reader of the records never takes it for the
build's.

**Tests.** On the JVM the runner is a JVM over sbt's test interface for each run, as sbt forks a worker for each: the
daemon launches the next one as a run ends, before it answers, with the frameworks to load, so that its JVM starts
and loads them while the client reads the run, the spent one quitting meanwhile (`stop`, and a daemon that ends,
waiting for it within their bound and killing it past that), and a run inherits no thread, static or registration of
the one before (a library's executor whose threads took the earlier run's class loader as their context loader, a
library object's state, a shutdown hook); a JVM older than `java.outputVersion`, or than 17, is refused. A run is one
of sbt 2's forked test worker (sbt 2.0.8's `ForkTestMain`): the frameworks one after another on one pool of the test
configuration's `parallelism` threads, each given every suite whose fingerprint one of its own matches (a suite of
two frameworks of one fingerprint runs on both, and a framework that matches none still makes its runner and its
`done()`), a framework's tasks submitted together as a wave, their results awaited in its order before the tasks they
returned are submitted as the next wave, so that a returned task waits for the whole of its wave though a thread is
free, and the framework's `done()` once its last wave is drained, what `done()` returns shown nowhere. A suite's
logger and its output are written as they come, so that the lines of suites running at once interleave; its events
are reported together once its task returns, and a task that throws has `Uncaught exception when running <suite>:
<what it threw>` logged as an error and is one `Error` of its suite (a `SuiteSelector`, no duration), the events it
reported dropped; an event the worker's `ForkEvent` refuses (a selector that is not `Serializable`, a fingerprint of
neither kind) is thrown to the framework that reported it, so that its task throws, and one of a `Serializable`
selector of a kind other than test-interface's five is taken and fails, as the worker's gson does, when its task's
events are sent: `Failed to execute task <future>` is logged, the task's events and the tasks it returned are lost,
and the run goes on. What a task threw, a throwable a framework gives its logger's `trace`, and what a framework
throws making its runner or its tasks, or its `done()` throws (after `Uncaught exception when running tests: <what it
threw>`, no framework after it run), the worker also sends as a fork error, which ends the run as sbt's `ForkTests`
fails its harness at the first: nothing more of the run reaches the client, its runner retired as `ForkTests` leaves
its worker once it has closed the worker's input (its tasks in flight finish and its frameworks' `done()` runs, what
it writes drained unread, within the stop bound, past which it is killed), `[error]` lines with the throwable's first
line and first frame (its whole trace with `--verbose`) and `<project>/test: the run failed: <what was thrown>` stand
for the summary, `--events` holds the events reported before it, and the exit code is 1. The pool is made under the
run's class loader, whose classes its threads see, and shut down before that loader is closed; the count goes with
every run, so that a lock changed in place is the next run's. `parallelism` is `auto`, the processors the runner's
JVM counts at each run (`-XX:ActiveProcessorCount` among `javaOptions` sets it), or a count; the export writes it
from `Test / testForkedParallel` and `Test / testForkedParallelism` as sbt's worker sizes its pool (`1` where the
flag is false, the count where it is positive, else `auto`), so that sbt 2's defaults give `auto` and sbt 1's,
without the second key, `1`; a Mill build's is `1` ("The export of a Mill build"); a lock without the field is
`auto`, and any other value is refused on its line. `--verbose` names the pool's size (`[debug] Create a test
executor with a thread pool of 4 threads.`). Nothing on the command line sets it, as nothing on sbt's does: the
build's setting is the lock's.
On Scala.js the test configuration's
resident links the program whose entry point starts Scala.js's test bridge (the std's, `test-bridge` 1.22.0's
port) into one file, and each run starts a node of its own that loads it, speaking to the bridge as
sbt-scalajs's test adapter does: the frameworks found by their names, one controller runner per framework,
each task run through the continuation of `Task.execute` and the tasks it returns after it, then the runner's
`done`; a run passes when every call was answered and node exited with 0, so a node that ends early fails it.
The export says how sbt-scalajs runs the link (the test configuration's `moduleKind` and `jsEnv`: `Test /
scalaJSLinkerConfig`'s module kind and `Test / jsEnv`): a `NoModule` link is a script node runs in its global
scope, as `NodeJSEnv` runs it from its stdin (a global `require` included), a `CommonJSModule` link is
`require`d and an `ESModule` link imported, a link of a program that imports a JavaScript module being an ES
module, which the first two refuse; node takes the environment's executable, arguments and variables and runs
in the build's root. `JSDOMNodeJSEnv` runs a `NoModule` link inside jsdom's window at `http://localhost/` as
the environment does (the window is the global object, a jsdom error ends the run), jsdom required from the
build's root after the root's lockfile is installed there (`npm ci`, or yarn's and pnpm's frozen installs, unless
it is the lockfile installed last) and checked against the version an npm lockfile pins. Other environments
(Selenium, Playwright), a jsdom whose root holds no lockfile and jsdom under a module are refused by the
export's `jsEnv` and `moduleKind`, a module initializer or an input of the build's own in the test link by the
export's refusals. The test link's module split and ES features shape the linker's files and syntax alone:
teq's link is one file of ES2015 whatever they say. sbt-scalajs's `Test / jsEnvInput` is the project's own
`jsEnvInput` followed by the link, which teq supplies itself: the inputs a build adds at the project's level are
named in the refusal, and a definition of `Test / jsEnvInput` of the build's own by where it stands, since only
linking would give its inputs. A Mill build's Scala.js projects run the same way, the export's `moduleKind` and
`jsEnv` being Mill's (`moduleKind`, `jsEnvConfig`), which Mill's Scala.js worker runs through the same test
adapter.

`teq test` writes on stdout each run's stream: what the suites, their frameworks, the runner's JVM and node print, and
its report of each event (`[success] ...`, `[fail] ...` with what was thrown, a fork error), whose names and messages
are the suites'; and on stderr its own lines: a build's diagnostics and its count of errors, its notes and failures
(`teq: <configuration>: ...`), each configuration's summary or its end with nothing to run. A run's outcomes and the
suites that ran are its events (`--events`) and its builds' diagnostics their records (`--diagnostics`), so nothing a
suite names, throws or prints, and nothing a diagnostic's message holds, is taken for one of teq's lines; `--list`
prints the suites on stdout.

Suites are found as sbt's `definedTests` finds them (`--list` prints them), against the fingerprints the
frameworks give, and the patterns applied as `testOnly` applies them; `--changed` keeps the suites whose
classes depend, transitively, on a class rewritten since the configuration's last full run (on Scala.js, on a
source file edited since), or depended on a source file deleted since, by the dependencies before the deletion. A Scala.js configuration with frameworks whose sources define no suite fails: a run
of nothing is no pass. A framework the export names that the test link does not hold (one whose jar is Scala 2's IR
alone, as Scala.js's JUnit runtime that munit brings) is left out with a note, as sbt-scalajs leaves out the
frameworks its link does not answer for: its suites, if any, do not run. Each failure is reported as `[fail] <test>: <message>` with the frames in the suite's
code (none on Scala.js, where a throwable's stack is empty: docs/COMPATIBILITY.md), `--verbose` adds a line for
every other event (`[ignored] <test>`) and the whole trace of a failure, and a line per project ends the run
with its suites, the passed and failed counts, the ignored, skipped, pending and canceled ones that are not zero,
and the times; the exit code is 1 when anything failed. `--events <file>` writes every event of the run, a line
each: the project, the event's name, its selector, its status and the first line of what it threw,
tab-separated: on the JVM a suite whose task threw ends the run before its `Error` is reported, as a framework that
fails does (above); under node a suite whose task or framework threw is an `Error` of a `TestSelector` of its name,
as sbt reports one through sbt-scalajs. After the events, a line for each suite whose task ran, a suite that reported
no event too (an empty suite's): the project, the suite and `ran`, tab-separated. `--diagnostics <file>` (`compile`,
`test` and `run` take it, and `teq compiler` with `build` and `check`, which `teq build` passes on) writes each
diagnostic the builds show as well, a JSON line each in the order shown: the configuration (the daemon's verbs), the
severity, the file, line and column where it has them, the whole message, and the lines shown for it (`shown`), so
that a reader takes a diagnostic's extent from the build, never from the text, whose message may hold any line;
a macro's `report.info` is one of them (severity `info`, its whole message), as dotty reports it (`report.echo`, an
`Info` diagnostic). The daemon's verbs record beside them each failure of a configuration's as shown (`teq:
<configuration>: ...`, severity `failure`), a generator's with its command's whole output, and what each passing
generator printed, one record a generator (severity `note`), as `teq build` records its generators' before the
compiler's records (`--diagnostics-append`); their text is the command's and never one of teq's own lines. What is
shown does not change.
Without a project every project's suites run, on either platform, those the export refuses named and the exit
code 2.

**Stage.** `teq stage api` writes the files of the export's `stage` block under its directory,
`<layer>/<path in the image>`, so that a Dockerfile copying native-packager's layers
(`COPY <directory>/<n>/opt/docker /opt/docker`) builds the same image: each product's jar, the artifacts and a
POSIX start script, an unchanged jar left alone so that Docker's layer cache holds. The script runs the main
class the build declares, else the single main class among the project's own sources, as sbt picks it after a
compile, which the project's jar names as its `Main-Class` too unless the build sets `Compile / mainClass :=
None`. With several or none the stage is refused, naming them, where native-packager would write a script per
class or none: declare `Compile / mainClass := Some(...)` and export again. A stage directory under no
directory named `target` is refused, since the stage empties it.

**Dev.** `teq dev frontend` runs the project's generators, the package manager's install when the
lockfile's hash changed or `node_modules` is missing, then the `dev` command of its `package.json` (or
`teqDevCommand`) with `TEQ` naming this binary, and restarts the command when the export's blocks it depends
on, the lockfile or the binary change; an export pinning another compiler ends the loop.

## The vite plugin

vite-plugin-teq builds a Scala.js project of the export in the place of the Scala.js linker, with sbt out of
the edit loop: the export describes the build once, and vite runs teq itself from then on.

```js
// vite.config.js
import teq from "vite-plugin-teq"
export default defineConfig({ plugins: [teq({ project: "frontend" })] })

// main.js, the page's entry
import "scalajs:main.js"
if (import.meta.hot) import.meta.hot.accept()
```

In dev, `vite` keeps `teq watch frontend` writing the program as one ES module per package, and per file
for the project's `modulePerFile` packages, with `--hot`. A source change is rebuilt, the modules teq rewrote
are invalidated in vite's module graph and the entry that accepts its own updates is re-imported: the browser
re-executes the rewritten modules, the per-file modules importing them and `main.mjs`, and React Fast Refresh
re-renders the mounted components with their state. A rebuild that would re-execute more than `hotSwapUpTo`
modules (100, an entry of `teqDescriptionKeys`), one that rewrote a module that is not per file, or no
accepting entry reloads the page instead, and a compile error goes to vite's overlay while the last good build
stays served. On a small React page a saved edit reaches the DOM in 95 ms, teq's own incremental build under
10 ms of it. `vite build` runs `teq build frontend --release` into one file and bundles it.

| Option | Default | |
|---|---|---|
| `project` | none, required | the project of `teq.lock`, at or above vite's root, that the page builds |
| `prefix` | `"scalajs"` | the import prefix: `scalajs:main.js` the program, `scalajs:<module>` any module |
| `outDir` | `target/teq/<root>-teq-<port>-staged` | where teq writes, relative to vite's root; per port |
| `generate` | none | `{ run, watch, on }`: sources the application generates from files the export omits |

The export's own generators need no `generate`. The refresh runtime, `hot-refresh.mjs`, is teq's own file,
which `--hot` writes into the output and the plugin injects ahead of the page's other scripts; it is the part
of react-refresh's runtime a swap needs, so the application needs neither react-refresh nor vite's React
plugin. Components kept in a `lazy val`, a top-level `val` or created at render time are not refreshed: a swap
remounts them or leaves their old code. A `main()` that renders must keep its React root across the re-runs a
swap makes (on `window`, say) and render only the first time.

The package is not published yet: an application installs it from a checkout of the repository
(`"vite-plugin-teq": "file:<teq>/integrations/vite"` among its `devDependencies`; npm symlinks a `file:` dependency and
installs none of its own, so `npm install --legacy-peer-deps` once in `integrations/vite` too, where yarn needs nothing more). It depends
on the `yaml` package, for the lock, and on `curl`, for fetching the binary; `vite` is a peer dependency. The other
route to a browser goes through sbt:
with `TEQ_COMPILER=1` the stock Scala.js vite plugin serves what `fastLinkJS` names, teq's output unmodified,
`sbt ~frontend/fastLinkJS` keeps it current and `vite build` bundles the one file of `fullLinkJS`, with the
same swaps, since the refresh runtime and what a swap runs again are written by `--hot` into the output.

## The language server and Zed

`teq lsp` is a language server over stdin and stdout, used by Claude Code's LSP tool and by Zed. It answers
definitions, references, hovers, document and workspace symbols, implementations, the call hierarchy (the
callers and callees of a method, grouped by their enclosing definition), completion and signature help, and
publishes diagnostics after each edit (an unused import as a hint, with a quick fix removing it: scalac's own
edit, CheckUnused's, which deletes the statement, keeps its one used selector or drops the unused ones with
their commas). A definition into a library goes to its source, read from the jar's sources jar, or
into teq's own standard library, shown in read-only documents where navigation works as in a source, and
completion and signature help too in the standard library's. A jar's sources jar is found beside the jar, in
coursier's cache (where `sbt updateClassifiers` and Metals put it) or in teq's own cache: once a project's session
starts, the server fetches the sources of each of its libraries that has them nowhere from the repository the
`teq.lock` names for it (with the credentials sbt's credentials file holds for its host), in the background, each
told in the language server's log (`teq: fetching sources of <library>`), and the next navigation into the library
finds them, with no restart; a sources jar placed or replaced meanwhile is found the same way. A library that
publishes no sources artifact has no document: a definition into it answers nothing and its hover stays; the
server asks for its sources again after a quarter of an hour and at its next start, after a failed transfer
(offline, the repository down) after 1, 2, 4 and 8 minutes and then hourly, and after refused credentials once the
credentials file changes, each failure told once in the log. A member called on an object seen as one of its
parents (`(Obj: Base).f`) is the parent's, as scalac's tree names it; the object's that runs is among its
implementations. Formatting, other code actions and rename are not offered; hover shows a name's signature or
type, not yet an expression's.

Completion lists the members of a receiver (extension methods and conversions included), the names in scope
(the extension methods in scope among them), types, the class after `new`, imports and keywords, filtered by
the prefix case-insensitively and cut at 1,000. With a prefix of two characters or more, names out of scope
follow, each with the import that brings it, written after the file's last top-level import; merging into an
existing braced import is a later part. A client with snippet support gets a method's call with a placeholder
per parameter (`inc(${1:x})$0`), and after `new` a constructor's (an alias's class's, a Java class's single
one); where the program expects a function of the name (a function type, a trait of one method that takes
parameters, the element of a tuple whose type expects one) the name alone, as no call would type there.
Signature help triggers on `(` and `,` and lists every overload with the active one as the build applied it
(through a conversion of an argument too), at the types the call instantiated; inside a type argument list it
is a later part.

The server works from the folder's `teq.lock`, at or above the folder, else at the roots of the sbt builds up
to three levels below. Each project configuration is a session, one `teq compiler watch --check` process, started when
a file of its source roots is opened; at most four run at once (`maxSessions`) and one unused for half an hour
stops (`sessionIdleSeconds`), which the folder's `.zed/settings.json` can change:

```json
{ "lsp": { "teq": { "initialization_options": { "maxSessions": 2, "sessionIdleSeconds": 600 } } } }
```

During a build that takes longer than a moment the status bar shows `teq: checking <project>`; the check for
files changed on disk that a session makes every three seconds while idle is not shown. An export
changed after the server started is read again within seconds; a build definition changed since the export was
written is a warning on `build.sbt` naming `sbt teqExportAll` (a build file git ignores, such as Metals'
`project/metals.sbt`, is a tool's and does not count). An sbt build without an export is exported by
the server itself, once: it runs `sbt teqExportAll` with the published sbt-teq of its own version loaded for
that run alone, which writes `teq.lock` under the build's `target/teq/` (at its root under the driver), and
again when that file goes, after an `sbt clean`; a run that fails is a diagnostic
on `build.sbt` and runs again once a build file changes. It needs sbt on the `PATH` (its launcher runs the
build's sbt, 2 or 1 once the plugin's release carries sbt 1's module), fails under a
`--client` in `.sbtopts` (a server JVM without the plugin), and does not run on Windows. A folder that is no
sbt build is one session of every `.scala` file under it, on the lean standard library without a class path.

**Zed.** The extension brings the Scala language (the grammar and language files of metals-zed) with `teq lsp`
as its server. Zed runs one Scala language at a time, so Zed's own Scala extension is uninstalled first. It is
installed as a dev extension from a checkout, as a bundle, or on macOS through an unsigned installer package.
For a project folder it takes, in this order, the binary named in Zed's settings (`lsp.teq.binary.path`, with
`arguments` `["lsp"]`), which Zed starts itself; `TEQ` in the folder's shell environment; the binary the
folder's `teq.lock` (else `target/teq/teq.lock`) pins for the platform, fetched from the export's repository without credentials and kept
only when its sha1 and size are the pinned ones; `teq` on the `PATH`; and its own copy of the newest release
for the platform, teq's GitHub release (a lock pinning a release before 0.1.7 is refused, and no copy of one runs), checked against
the release's `SHA256SUMS` and fetched while the status bar says "Downloading teq", its copy checked again at each
start before it runs. What it found is asked again a day later, since GitHub limits an address's unauthenticated
requests; when GitHub refuses more, the copies present serve, each still the release it was fetched as. To keep the server out of a folder, in its `.zed/settings.json`:

```json
{ "languages": { "Scala": { "language_servers": ["!teq"] } } }
```
