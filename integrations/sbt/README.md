# sbt-teq

An sbt plugin, for sbt 2 and sbt 1, that builds a Scala.js project, or a JVM project, with [teq](../../README.md)
from what the sbt build already knows: the source roots, the resolved jars and the scalac options. One source tree
builds both modules (below, "The two sbt lines").

## Adopting teq

### teq as the compiler

A few lines of sbt config; the plugin is on Maven Central, which sbt resolves from with no resolver
line:

```scala
// project/plugins.sbt
addSbtPlugin("build.teq" % "sbt-teq" % "1.0.0")

// build.sbt
teqVersion := "0.1.8"
teqCompiler := true
```

sbt then compiles, tests, runs and packages the build with teq in the place of zinc's Scala
compiler (below, "teq as the compiler"), running the build's own source and resource generators
whatever they are; no file is added to the repository. Without the setting, `TEQ_COMPILER=1` in
the environment does the same for one shell or CI job. The compiler and the plugin have version
lines of their own (`docs/TOOLING.md`, "The sbt plugin": which plugin serves which compiler): the
compiler is released at each ship, the plugin when it changes, from 1.0.0, its `teqVersion`
defaulting to the compiler it was released with ($TEQ_VERSION another). A build moves to a new release
by naming it here, and nothing moves by itself. The releases before 0.1.7 are not served: a build
naming one is refused (`teqVersion is 0.1.6, and releases before 0.1.7 are not served: pin 0.1.7 or
later`), as is a lock pinning one, and moves to 0.1.7 with the plugin 1.0.0.

### The editor

Nothing to add. teq's language server (`teq lsp`, which the Zed extension runs) reads the build's
export, and on an sbt build without one runs `sbt teqExportAll` by itself, adding this plugin to
that run alone when the build does not name it: the export lands in `target/teq/teq.lock`, under
`target`, so that the repository gets no file, and runs once again when that file goes (an `sbt
clean`). The export refuses no build for what `teq`
could not run: it records it (below, "The export"), the editor reads the rest, and a source
generator sbt alone runs is read from what sbt last generated.

### teq as the build tool (opt-in)

`teqBuildTool := true` in `build.sbt` takes teq as the build tool: `teq`, the native driver,
compiles, tests, runs and stages the build with no sbt. `teqExportAll` then writes `teq.lock` at
the build's root and the launchers `teq` and `teq.cmd` beside it, which the repository commits
with the `.gitattributes` lines below, and CI checks them with `sbt teqExportAll && git diff
--exit-code teq.lock teq teq.cmd`. What `teq` needs of the build: its source generators `TeqCommand`s
(`Compile / teqGenerators`) or sbt-buildinfo's of a shape it writes, and what sbt would find by
compiling declared (`Compile / mainClass`, `teqMainClasses`; below, "The export"). A project it
cannot run is named when a verb asks for it: `./teq compile api` refuses api with
every reason the export recorded and its remedy, and a verb over every project runs the others
and names in its summary each one it left out. Under the driver a SNAPSHOT or dynamic version is
refused unless `teqExportSnapshots` allows it, since a committed lock would drift with a later
resolution.

## The tasks

- `teqExportAll` writes `teq.lock`, the description of every project that `teq`, the
  language server and vite-plugin-teq ([`../vite`](../vite)) read without sbt: under the build's
  `target/teq/`, or at its root with the launchers `teq` and `teq.cmd` beside it under
  `teqBuildTool` (below, "The export").
- `teqBuild` runs the project's source generators and those of the projects it depends on, then
  `teq compiler build` once into `target/teq/out`, for CI and production
  (`set teqRelease := true` for `--release`); teq's diagnostics go to sbt's log and fail the task.
  Its workers are the count teq selects by the program's size, the cores and the memory
  (`docs/TARGETS.md`, "The typer's workers"), `teqThreads := Some(n)` sets them for every full
  build, its, `teqFullLinkJS`'s, the compiler's and the links' below (in `build.sbt` for every
  project, a project's own setting before it); a link resident's retypes
  are one worker's, and a full build whose parallel attempt gives way is typed again by one
  worker before it answers, the note saying so on its stderr.
- `teqLinkJS` and `teqFullLinkJS` are the links behind sbt's route to a browser: sbt itself
  drives the build, the way `Compile / fastLinkJS` and `fullLinkJS` do for a Scala.js project,
  so a vite plugin only has to print where the output landed; under `teqCompiler` (below) the
  two Scala.js tasks of a project with the Scala.js plugin are answered by the same links, into
  the directories the linker would write, and `@scala-js/vite-plugin-scalajs` or an
  application's own plugin serves teq's output unmodified. `teqLinkJS` runs the dev build
  (`--split`, `teqModulePerFile`, `teqCacheableState`, `--hot`) into `teqServedOutput`, keeping
  one resident `teq compiler watch` process per directory (a `~teqLinkJS` trigger costs sbt's own
  file-change detection plus an incremental build, not a fresh process), and writes the stub
  `main.js` beside teq's own `main.mjs`: `import "./hot-refresh.mjs"; export * from
  "./main.mjs"; if (import.meta.hot) import.meta.hot.accept((updated) => { if (updated ===
  undefined) globalThis.__teqHot?.failed_("main", import.meta.url); });`, so that
  `scalajs:main.js` resolves as it would for a Scala.js build and the application's entry
  carries nothing of teq's (the refresh runtime ahead of the program, the acceptance of the
  entry's own hot updates, a failed one told to the runtime). `teqFullLinkJS` runs the
  `--release` build into `teqFullServedOutput` as one file, `main.js`, for `vite build`. Both
  tasks return the directory, so `sbt --batch 'print frontend/teqLinkJS'` builds and names it in
  one command. `docs/TOOLING.md` ("The vite plugin") has the route.
- A link's resident ends `teqLinkIdle` (a minute) after its last build, unless a watch that
  asked for a build is still under way then, whichever task it is a watch of. A watch of the
  link task itself ends it at once (Enter, or the watch's client gone: within 5 s, measured 0.2
  to 0.4 s), through a `watchOnTermination` of the link task that calls the build's own hook
  after it; the sbt server stays. One sbt process writes a link's directory at a time
  (`<directory>.lock` beside it; another process waits `teqLinkWait`, 30 s, and fails naming
  the holder), one link inside it (a link waits for a link of teq's, and fails where the
  Scala.js linker has the directory in the same command, which it has until its link has
  ended or the command is cancelled), and `<directory>.backend` names the writer, so that a link by the other compiler,
  or of the other kind, starts from an empty directory.

A project without the Scala.js plugin builds for the JVM (`teqTarget := "jvm"`, from sbt's
`platform`): its description names every jar of `Compile / externalDependencyClasspath`, and
`teqBuild` writes class files linked against them (`--target jvm --std=scala-library`) into
`target/teq/out`, which `java -cp target/teq/out:<the jars> TeqMain` runs, or the main class
itself (`teqMainClass := Some("app.Main")` names it for a program with several entry points).

## The export

`sbt teqExportAll` writes `teq.lock`: at the build's root under `teqBuildTool`, which the repository
commits, else under its `target/teq/`, saying where in one line and writing nothing at the root (a
`teq.lock` there from before is left alone, with a warning that every reader takes it first). Each
reader looks at a directory's `teq.lock`, then at its `target/teq/teq.lock`, the build's root
being that directory either way, and the file is the same bytes wherever it lies. It holds every
project with the plugin (but an aggregate with no source of its own that no project depends on,
such as the build's root), its configurations' source and resource roots, their classpaths (each
entry the products of a project's configuration, a jar of the repository's tree, or a resolved
jar by its key, `organization:name:version[:classifier]`), the jar table (one line per key: its
repository, sha1 and size, and its path only when it is not the Maven layout of the key), the
flags, the generators, the test and run contexts, the Docker stage of a project packaged by
sbt-native-packager, and the compiler for every platform whose binary the build's repositories
serve, by its URL, sha1 and size, read from the repository with no binary downloaded. A key that would
name two files (two repositories, paths, digests or sizes) is refused, both named. A version bump
changes the table's line and the classpath lines that name the key. Its first line is the compiler,
`teq: <version>`. Under the driver CI checks it with `sbt teqExportAll && git diff --exit-code
teq.lock teq teq.cmd`; `docs/TOOLING.md` ("The lockfile and the launchers") has the format.

Under the driver the export writes beside it the launchers `teq` (POSIX sh) and `teq.cmd`, which
the repository commits too, with these lines in its `.gitattributes`:

```
teq text eol=lf
teq.cmd text eol=crlf
teq.lock text eol=lf
*.sbt text eol=lf
project/**/*.scala text eol=lf
project/build.properties text eol=lf
```

The last three keep the build's own files LF in every checkout: the lock records their bytes, so a
checkout with CRLF (Git for Windows' default) of a lock exported from LF, or the reverse, finds it
stale. `teq` then warns as of any change, `--strict` refuses, and a note names each file that
differs by line ends alone with this remedy; the export warns when it reads CRLF. Once the lines
are added, delete the files, check them out again and export again.

A clone then needs no teq installed: `./teq test` (`teq.cmd test` on Windows) reads the
lock's first line and its platform's `binaries` line, finds the pinned binary in teq's cache, else
in coursier's, else fetches it (curl, its size and sha1 checked), and runs it with the arguments as
given; `TEQ` names another binary. `./teq run <project> [-- args]` runs the main class the
build declares (`Compile / run / mainClass`, else `Compile / mainClass`), else the single main class
its build finds, as sbt's `run` does, and `./teq run <project> <alias or main class>` the one
named. A launcher is replaced by a later plugin's only while it is
unedited; an edited one is kept and the export says so. The export compiles nothing and runs none of the build's code: the classpaths come from
dependency resolution alone, so it costs an sbt start in CI's check and in the language server's
fallback, not a build. What sbt would find by compiling or by running the build's code is
declared instead:

| Setting | Default | |
|---|---|---|
| `teqMainClasses` | none | main classes `teq run` takes by name before a compile, besides `Compile / mainClass` (when the build or a session's `set` or `set every` sets it) and the aliases' targets; `teq run <project>` runs the run block's class (`Compile / run / mainClass`, else `Compile / mainClass`), and without one the project's own products decide, as they do for `teq stage` when the build declares no `Compile / mainClass` |
| `teqRunAliases` | none | names for main classes, which `teq run` takes in their place |
| `Compile / teqGenerators` | none | `TeqCommand(run, inputs, cwd = ".", outputs)`: a program of the repository that writes sources into the directory appended to `run`, run by sbt's compile into `sourceManaged / "teq"` and exported for `teq` with `target/teq/<project>/compile/src_managed`; `inputs` are globs under the build's root which, with the files its arguments name, rerun it under a watch; `outputs` are the files or directories under the build's root it writes besides, which `teq` runs it again to restore; the first word `teq` is the build's own binary (`teqResolvedBinary`, the one running under `teq`), never a `teq` of the `PATH` |
| `teqDevCommand` | the `dev` script of a `package.json` in the project's base | a Scala.js project's dev command, run from the directory of the nearest `package.json` |
| `teqBuildTool` | false | `teq.lock` and the launchers at the build's root, to commit; the build takes teq as its build tool when any project's setting says so, as a bare `teqBuildTool := true` of `build.sbt` does for every project |
| `teqExportSnapshots` | true without `teqBuildTool`, false under it | lets the export name SNAPSHOT and dynamic versions |

```scala
Compile / teqGenerators += TeqCommand(Seq("node", "generate-labels.mjs", "browserdemo-labels.txt"), inputs = Seq("browserdemo-labels.txt"))
// A Scala script run by the build's own binary: the object into the directory appended to `run`, a
// module into the application's assets, which `outputs` names.
Compile / teqGenerators += TeqCommand(Seq("teq", "interp", "scripts/images.scala", "--", "app"),
  inputs = Seq("app/assets/images/**/*.svg"), outputs = Seq("app/assets/images.js"))
```

sbt-buildinfo's object is exported as it is, in its keys' order: their values as static ones
(strings, `Int`s, `Boolean`s, lists of strings), and as dynamic ones, which `teq` computes,
an action named `gitSha` (its own code is not run) and a key whose value is a class directory of
the build; `teq` writes the file as sbt-buildinfo does, byte for byte but for the class
directories. The repositories are the remote Maven ones among
the build's resolvers, Maven Central as `maven-central`, the others by their names; each but
Maven Central names its host, whose entry in `~/.sbt/.credentials` a reader presents when the
file has one.

A project with sbt-native-packager's `JavaAppPackaging` (which brings its `DockerPlugin`) gets a
`stage` block, the files native-packager's `Docker / stage` writes, so that `teq stage`
writes them with no sbt and the Dockerfile copies its layers as before: its own jar and those of
the runtime classpath under `lib/` by native-packager's names (`<organization>.<name>-<version>.jar`),
the start script under `bin/`, each in the layer `dockerGroupLayers` gives its path, the stage's
directory (`Docker / stagingDirectory`), the exposed ports and the main class. The start script
runs `Compile / mainClass` where the build declares it (a build file's or a session `set`'s); a build leaving the setting alone gets the
block and the jar's manifest without the class, and `teq stage` implies it as `teq run`
does: the single main class among the project's own products, as sbt's `mainClass` picks it after
a compile, else a refusal naming them with the declaration as the remedy (teq's restriction:
native-packager writes a start script per class there, or none); a build setting it to `None` gets
`mainClassNone` in the block, the script's class implied and the manifest without one, as sbt
leaves it. native-packager's own layer grouping is
rebuilt in the export, since its task reads the packaged jars; a `dockerGroupLayers` of the
build's own is evaluated when it reads settings alone, its paths being all it needs.

What `teq` could not reproduce is recorded in the lock, on the project and the block it
concerns, and the verbs that need it refuse at use, naming every reason of the projects involved
and its remedy in one failure; a verb over every project (`teq test`, `compile` or `stage`
without one) runs the others, names in its summary each project it left out with its reasons and
then exits 2, so that a CI run never passes with work left out. The editor and vite read
everything else of the project. A reason is the same bytes on every machine: a file in it is
named relative to the build's root (one outside it is not named), an exception by its class.
What is recorded:

- a source generator of Compile other than a `TeqCommand` or sbt-buildinfo's of a shape `teq`
  writes (sbt-buildinfo's options, an action other than `gitSha`, a task among its keys or a value
  of another kind): a `generators` entry of kind `sbt` naming its task (or `an unnamed task`), with
  the reason for sbt-buildinfo's, and the configuration's sources naming sbt's managed source
  directories in the place of `teq`'s, so that the editor reads what sbt last generated (its
  `TeqCommand`s' and sbt-buildinfo's outputs among them, which sbt writes there too; one directory
  outside the build's root is named absolute without the driver, left out with a warning under it).
  `compile`, `test`, `run`, `stage`, `dev`, `watch` and `build` refuse the project and every
  project whose closure holds it: a generator written in Scala becomes a script that both run, a
  `TeqCommand`, or the project builds with sbt;
- a source generator of Test or Runtime, and a resource generator: entries of kind `sbt` in that
  configuration's `generators` or `resourceGenerators`. The verbs that type or run the
  configuration refuse (`test` for the test configuration's, `run`, `stage`, `dev` and `test` for a
  resource generator of Compile); `compile` does not, typing the compile configuration alone where
  the test one holds a generator sbt runs;
- a `Tests.Filter`, `Tests.Setup` or `Tests.Cleanup` (a `Tests.Exclude` names the suites to leave
  out): the project's `unsupported.test`, which `test` refuses with;
- in a packaged project's stage, a `dockerGroupLayers` of the build's that reads a task (the
  default grouping among them, by `dockerGroupLayers.value`) or fails on a path, a path it puts in
  no layer, native-packager's `ClasspathJarPlugin`, `LauncherJarPlugin`, `JavaServerAppPackaging`,
  `AshScriptPlugin` and `JlinkPlugin`, a `scriptClasspath`, `scriptClasspathOrdering`,
  `bashScriptExtraDefines`, `bashScriptDefines`, `bashScriptConfigLocation`,
  `bashScriptTemplateLocation`, `bundledJvmLocation`, `dockerPackageMappings`, `Universal /
  mappings`, `Docker / mappings` or `Universal / javaOptions` the build sets, `packageOptions` set
  on a project whose jar the stage holds, two files mapped to one path, and files in
  `src/universal`, `src/docker` or a `src/templates/bash-template`: the stage block is left out and
  the project's `unsupported.stage` says why, which `stage` refuses with;
- a project of a platform neither the JVM nor Scala.js (Scala Native) is left out of `projects`,
  the export's log saying so, so that a cross build exports.

Without the driver the lock is this machine's description, and what only a committed lock could
not hold is written: a SNAPSHOT or dynamic version as resolved; a path outside the build's root
absolute, with `/`; an artifact from none of the build's Maven repositories (a file repository,
sbt's own boot jar the build's resolvers do not give) as a file entry by its path (absolute, but
relative where it lies under the build's root, as every path of the lock); and a binaries
table that cannot be filled (a binary served without its `.sha1`, a classifier whose `.sha1` or
size its repository answers otherwise than with them or a 404, no binary of `teqVersion` served
at all) empty, each reason a warning of the export. With no repository serving the version and
one out of reach the table is empty with a warning in both modes. Under the driver all
of these are refused (a later resolution can make a SNAPSHOT another file, and CI's check then
fails with no change to the build; a reader elsewhere has none of the machine's files), but
SNAPSHOT and dynamic versions where `teqExportSnapshots` allows them. Refused in both modes,
every reason in one failure: a key naming two files and a repository id naming two URLs.
[`example/check-export.sh`](example/check-export.sh) checks the example's
export against its committed fixture.

## teq as the compiler

`teqCompiler := true` (by default `TEQ_COMPILER` read from the environment as a boolean, so a
CI job and a dev shell can differ; `set every teqCompiler := true` switches a session) puts teq
in the place of zinc's incremental compiler: for a JVM project `compile`, `test`, `testOnly`,
`run` and `packageBin` run through sbt's own tasks, with sbt's own reporting (a failing test by
its framework, a type error with its position), over teq's class files; for a Scala.js project
`compile`, `test`, `testOnly` and the links do, over teq's JavaScript (below). False leaves the
build stock scalac.

```
TEQ_COMPILER=1 sbt api/compile api/test        # or: sbt; set every teqCompiler := true
```

- What runs: zinc's own incremental compile (`docs/TOOLING.md`, "The sbt plugin"), with teq
  as the configuration's Scala compiler, put into `compileInputs`' compilers for `Compile` and
  `Test` (`inConfig(IntegrationTest)(TeqPlugin.compilerSettings)` adds a configuration of the
  project's own). zinc keeps its invalidation, its analysis and its class-file manager, and
  hands teq the sources it invalidated: each batch is one `teq compiler build --target jvm --products
  <class directory>` (a Scala.js project's `teq compiler check --products`) over them, its arguments
  in an argument file ("The teq binary" below), against the
  configuration's own class directory first on the class path, then the upstream configurations'
  class directories or jars and the library jars. The other sources' products stay in the
  directory and are read as an upstream's are, through the products' manifest
  (`teq-products.json`), which the compiler rewrites only once a batch has passed. Each source
  is typed by its own project's compile, once; nothing is kept between compiles but what is on
  disk, and no process outlives its batch. The log line per batch carries its times:
  `teq: 1 source of classes in 806 ms (process start and publication 41, class path 7, type 260,
  emission 61, graph 85, callback 9)`, the publication counted with the process start since the
  compiler answers before it publishes.
- The analysis: the answer's analysis graph (below) reaches zinc's callback as scalac's bridge
  hands it a compile's: every class's API with every member's signature, an inline method's
  fingerprint among them, the dependencies of the classes' code on classes of the build and of
  the class path, the names they use, the products of every source, its entry points and its
  problems. zinc then compiles the dependents of an API change alone, by the names they use, in
  the configuration and in the ones over it, the callers of an inline body that changed, and a
  class with a macro after a change of what its implementation calls; `definedTests` finds
  zio-test, munit and scalatest suites (through a project's own base class too) and a JUnit
  suite by its `@Test` methods (found, not yet run: teq writes no annotation attributes into
  class files, so JUnit itself sees no `@Test` there); `discoveredMainClasses` finds the entry
  points.
- A Scala.js project's compile is a check, which writes TASTy and no IR; its classes' products for
  zinc are a stamp per class, `p/Foo.teq` under each binary name the JVM would give the class
  (`p/Foo$.teq` and the mirror's `p/Foo.teq` for a top-level object), holding a hash of the
  class's source, which sbt's test cache reads as it reads class files and the linker never
  does. `fastLinkJS` and `fullLinkJS` are answered by the links of `teqLinkJS` and
  `teqFullLinkJS`, whole-program from the sources, into the task's
  `scalaJSLinkerOutputDirectory`, with the linker's report (one ES module `main` in `main.js`)
  attributed with that directory; `Test / fastLinkJS` (and `fullLinkJS`) link the test
  configuration with a generated entry point, `dev.teq.sbt.TestMain`, that starts Scala.js's test
  bridge. sbt-scalajs's own test adapter runs what it links under node: `loadedTestFrameworks`,
  `definedTests` (which links, since the adapter asks the bridge for the frameworks), `test`,
  `testOnly` and `testQuick`, with the frameworks' own reports and positions (the example runs
  zio-test, munit and utest suites). The bridge, sbt's test interface and the JUnit classes munit
  extends are Scala 2.13 artifacts with no TASTy, so teq's std carries ports of them
  (`std/scalajs/test_bridge.scala`, `testing.scala`, `junit.scala`); the JVM side, the adapter and
  the node environment are the stock ones.
- The test cache: sbt 1's `test` runs every suite each time, and sbt 1 caches no task. sbt 2's `test` is
  `testQuick` under either compiler, which runs the suites
  whose digest has no recorded success, and the digest walks zinc's dependencies of a suite's
  class and the products' hashes (a Scala.js project's stamps among them), so a change re-runs
  the suites it reaches. The successes are in sbt's disk cache, the machine's, which `clean`
  leaves: after a passing run `clean; test` runs nothing, nor does `test` back on a toggle state
  whose suites passed, while `definedTests` lists them all; `testOnly` runs what it names
  (`example/check.sh` uses it, `example/check-tests.sh` checks the cache's rules). sbt 2.0.8
  caches `definedTestDigests` itself, by its inputs and across plugin versions
  (`IncrementalTest.scala`): digests another sbt-teq computed answer for the same inputs, which a
  comparison of two plugin versions over one build has to keep in mind.
- A failure, and the rollback: a batch that fails publishes nothing, and its problems reach sbt
  with their positions. A run is several batches; when a later one, zinc's callback or a
  cancellation ends it, the plugin's journal (zinc's external class-file manager) removes every
  file the run added to the class directory, puts back what a publication cut short moved aside,
  the products' manifest and the files zinc does not own (the runtime's classes, which it keeps
  aside for the run in `<directory>.teq-run`), and zinc's own manager restores what it moved
  aside, so the directory is what it was before the run. zinc calls no compiler for a run whose
  only change is a removal; after it, the plugin drops the sources the configuration no longer
  has from the manifest with the manifest's own operation (`--removed` without sources).
- What changes the setup: the setup's `extra` and the compile cache's key carry teq's identity,
  the binary's SHA-256, whether it builds or checks and the flags that change the products
  (`teqCacheableState`, `teqMacroState`, the mapped scalac options; `teqThreads` does not), so a
  change of any compiles the configuration again, as zinc does when its compiler's options
  change.
- What stays with scalac: `doc`, `console` and scalafix, whose `compilers` are untouched. With
  the toggle off the Scala.js linker links into the directories teq linked into, after what teq
  wrote there is gone; neither compiler's files stay for the other to serve. A marker beside the
  class directory (`classes.backend`, `teq` for teq's) names the compiler that wrote it; a compile
  under the other toggle state empties the directory and starts afresh, since zinc deletes only
  the products its analysis registered and a products' manifest left in a directory scalac
  rewrote would be read by a teq downstream as teq's.
- A known limit: Java sources, which the toggle refuses with an error naming the files (teq
  reads Java class files only; set `teqCompiler := false` for such a project or move them to a
  project of their own).
- The toggle per project: a configuration with the toggle off keeps sbt's stock tasks and its
  class directory to scalac alone; one with it on reads an upstream project's products, scalac's
  or teq's, through TASTy, and scalac reads teq's in the other direction.
- The cost: an sbt `compile` costs a process start and the class path's opening per batch,
  where the resident's compile kept both; the edit loop keeps its resident behind `teqLinkJS`, the
  vite plugin and the editor.

`docs/TOOLING.md` ("The sbt plugin") lists the settings, and
[`example/`](example) is the application corpus of `bench/app` as an sbt build, a Scala.js frontend and a
JVM API side with a test configuration of the three frameworks, which `example/check.sh` builds
and runs against scalac's output, through `teqBuild` and through the compiler toggle, alongside a
small hand-written page (`example/browserdemo-src/`) that exercises `fastLinkJS`/`fullLinkJS` over
teq's output under headless Chrome, with the stock Scala.js vite plugin and in a build shaped like
the reference application's, and whose Scala.js tests (`example/browserdemo-test-src/`, with
suites that fail on purpose under `demoFailing`) run through the compiler toggle.

The plugin is released with the compiler (`docs/TOOLING.md`, "The sbt plugin"; "Publishing the
binary" below). A branch publishes it locally under a SNAPSHOT version of its own, which a build names
with the branch's binary in `TEQ`; the build refuses a local publish of a release's version,
which would stand in for that release in every build of the machine:

```
sbt --batch 'set version := "0.1.1-mybranch-SNAPSHOT"; publishLocal'   # ~/.ivy2/local
```

### The analysis graph (internal)

The first stages of the module model's analysis: with
`--analysis-version 2`, teq answers the API of every class it compiled as a graph of zinc's
`xsbti.api` objects, and `sbt.internal.teq.ApiGraph` builds those objects from it, mechanically,
for `sbt.internal.teq.TeqAnalysis` to hand to zinc's own `AnalysisCallback`, which computes the
name hashes; with `--analysis-version 3`, the answer carries per file what its classes depend
on besides, which the adapter hands to the callback as scalac's `ExtractDependencies` does
(`usedName`, `classDependency`, `binaryDependency`, after every file's `api`), and zinc stores
the relations from. An answer of a failed build, one with `apiErrors`, or one whose graph or
dependencies are missing or malformed is refused before anything reaches zinc. The compile under
`teqCompiler` feeds zinc through it. `sbt testFull` here runs the adapter's tests over recorded answers
(`src/test`), zinc's `Incremental.apply` among them, reading back the relations it stored, and `sbt '^^1.13.0'
testFull` runs them on the sbt 1 module (this build is sbt 2's, whose `test` is testQuick).
[`analysis/`](analysis) is the oracle of `tests/analysis.sh`: an sbt build whose projects are the
cases, each compiled by scalac and built by teq through the adapter, both through zinc's
incremental compiler with its API storage on, whose two analyses and dependency callbacks the
suite compares; it compiles the adapter from these sources, so it needs no published plugin.
Its `analysisCost` times the JVM's side of an answer (`bench/analysis-costs.sh`), and its
`invalidationOracle` is the oracle of `tests/invalidation.sh`: each scenario of
`tests/analysis/invalidation` built on both sides, edited and compiled again by zinc's own
`Incremental.apply` with sbt's lookup, teq through a test-only incremental adapter that builds the
batch zinc asks for as the plugin's compile does (`project/TeqBatch.scala`), then built afresh as
the reference.

## The two sbt lines

The plugin is published for sbt 2 as `sbt-teq_sbt2_3` (Scala 3) and for sbt 1 as `sbt-teq_2.12_1.0` (Scala
2.12), both at the version of `plugin-version.txt`; a build's `addSbtPlugin` line names neither, and sbt resolves
the one of the sbt that loads it. This build, an sbt 2 one, makes both: `crossSbtVersions` names sbt 2.0.8 and
sbt 1.13.0, `^^1.13.0` selects the sbt 1 module (Scala 2.12.21, sbt 1's API jars) and `^` runs a command for each
(`publish.sh` stages both with `^publishSigned`). The sources under `src/main/scala` are written in the syntax
Scala 2.12 (with `-Xsource:3`) and Scala 3 both read; what each sbt's API has of its own is behind
`dev.teq.sbt.Compat`, `BuildInfoCompat` and `sbt.internal.teq.InterDependencies`, one file of each per line,
`src/main/scala-sbt-2` and `src/main/scala-sbt-1.0`, with the same members:

| What | sbt 2 | sbt 1 |
|---|---|---|
| A classpath entry, an artifact's path | a virtual file, through `fileConverter` | a file |
| A resolved module, its artifact and configuration on an entry | JSON strings (`moduleIDStr`, `artifactStr`, `configurationStr`) | typed attributes (`moduleID.key`, `artifact.key`, `configuration.key`) |
| The build's settings | `Def.Settings`, by scoped key | `Settings[Scope]`, by scope and key |
| The key a task was defined under | the task's attribute | its `info`'s attribute |
| The project's platform | `platform` | the plugins enabled (`sjs1` with Scala.js's, else `jvm`) |
| Credentials | `librarymanagement.Credentials`, a file read by `IvyCredentials` | `librarymanagement.ivy.Credentials` |
| A task over a task's value | `flatMapTask` | `Def.taskDyn` |
| A linker report's directory | the path's string under the key's label | the file under `scalaJSLinkerOutputDirectory` |
| A task whose work sbt cannot see (a generator, zinc's compile, a link) | `Def.uncached` | a task (sbt 1 caches none) |
| zinc's second inputs | teq's identity added under `teqCompiler` | none: `setup.extra` carries it on both |
| sbt-buildinfo's keys | its public `Entry` | its package's own cases, read by `sbtbuildinfo.TeqBuildInfoAccess` |
| A configuration's dependencies (`ClasspathImpl.interSort`) | over `Def.Settings` | over `Settings[Scope]` |

The two give one build the same lockfile but where the sbts resolve the build differently, and then each export
is its own sbt's: a Test class path of a project that depends on another holds the two in the order of each sbt's
`ClasspathImpl.interSort` (sbt 1 visits the project's configurations before its dependencies, sbt 2 the
dependencies of each configuration as it goes), the Docker stage's default directory is under each sbt's target,
and BuildInfo's `sbtVersion` is the sbt that runs. [`axes/check.sh`](axes/check.sh) checks both modules over two
fixtures under sbt 1.13.0 and sbt 2.0.8 (its header lists the cases): `equal/`, whose exports are its committed
lock's bytes on both but for the digest of `project/build.properties` and the hash over the build's files, and
`order/`, whose exports are each sbt's committed lock, every class path in its sbt's own order.

## The teq binary

From 0.1.7 the compiler is the GitHub release `v<version>` of `github.com/Carrot-Inc/teq`, one
file per platform, `teq-<version>-<classifier>` (`.exe` for Windows alone), its classifier in the
convention of protoc's artifacts (`osx-aarch_64`, `osx-x86_64`, `linux-x86_64`, `linux-aarch_64`,
`windows-x86_64`), beside
`SHA256SUMS` and the binary manifest `teq-<version>-binaries.txt` (a line `teq <version> <commit>`,
then `<classifier> <asset> <sha256> <sha1> <size>` a line). `teqResolvedBinary` reads the manifest
and `SHA256SUMS` from the release's own directory (`teqReleases`), downloads the machine's asset
through its redirect into a file of its own in the plugin's cache, `<cache>/releases/<base key>/<version>/<classifier>/`,
and renames it into place beside a receipt (the digests accepted and the manifest's SHA-256) only
once its SHA-256, SHA-1 and size are the manifest's and its SHA-256 is `SHA256SUMS`'s, and its SHA-1
and size the build's lock's where that lock pins the same compiler; nothing of it runs before. A
copy is reused while its bytes give its receipt's digest, which is also what serves offline; a copy
without its receipt, a part left by an interrupted download, and a copy whose bytes changed are
fetched and checked again. The plugin then copies the binary to
`target/teq/bin/teq-<version>-<classifier>` of the project with the executable bit set, checks
that `--version` prints the version asked for, and runs that copy. A release without the platform's
binary is refused naming the release and the classifier.

A release before 0.1.7 is refused before any request. A SNAPSHOT of the compiler published locally is
the Maven artifact `build.teq:teq:<version>`, one `exe` per classifier, resolved through the build's
own resolvers.

| Setting | Default | |
|---|---|---|
| `teqVersion` | the compiler the plugin was released with, or `$TEQ_VERSION` | the binary's version |
| `teqReleases` | `https://github.com/Carrot-Inc/teq/releases/download` | where the releases are, `<base>/v<version>/` each |
| `teqArtifact` | `teq` | the name of a SNAPSHOT's Maven artifact, and of the copies |
| `teqClassifier` | from `os.name` and `os.arch` | its platform |
| `teqBinary` | `$TEQ` | a local binary instead: resolution is skipped when it is set, a bare name is looked up on the PATH |

`teqResolvedBinary` names the binary the other tasks run (`teqBinary` when that is set). It copies
again whenever the copy's bytes are not the verified binary's. The binary also goes into teq's own
cache, `<cache>/bin/<sha1>/teq-<version>-<classifier>`, where the launchers, vite-plugin-teq and the
language server find it: `$TEQ_CACHE_DIR`, else `$XDG_CACHE_HOME/teq`, else `%LOCALAPPDATA%\teq` on
Windows, `~/Library/Caches/teq` on macOS and `~/.cache/teq` elsewhere.

The export pins the binary of every platform without downloading one: from 0.1.7, the URL of each
asset the release's manifest lists (its canonical URL, not where its redirect leads) with the
manifest's SHA-1 and size, the manifest checked against `SHA256SUMS`; a release that is not there is
refused (a warning for a SNAPSHOT or a dynamic version), and one out of reach is warned about, the
lock then naming no binary, so that an export offline still writes the lock; a release before 0.1.7 is
refused. For a SNAPSHOT published locally, of the build's remote Maven repositories, in its order, the first that serves the version's
pom (`teq-<version>.pom`, by which coursier resolves the module) gives each classifier's record from
its `.sha1` and a HEAD of the binary for its size; a repository that refuses a HEAD (403, 405 or 501, as
a redirect to a URL signed for the GET alone answers) is asked a GET of the file's first byte instead, its
size the total of the answer's `Content-Range` (or its `Content-Length` where the range is ignored), the
lock keeping the repository's URL, never a signed one; a classifier whose `.sha1` and binary both answer
404 is not published, and a binary without its `.sha1` is refused. The requests carry the credentials
sbt holds for the host, its `credentials` inline or from a file (`credentials +=
Credentials(Path.userHome / ".sbt" / ".credentials")`, `$SBT_CREDENTIALS`), not coursier's own. Only
`teqResolvedBinary` downloads, this machine's binary alone, when a task runs it.

A release never changes, so a build runs the binary it named until it names another release. A
SNAPSHOT of the binary published locally (in `~/.ivy2/local`) comes before every repository.

Every command the plugin runs with a class path starts as `teq @<file>`, its arguments one per
line, in UTF-8, in the project's `target/teq/<configuration>-<kind>-<digest>.args`, named by the
SHA-256 of its contents, so that two commands with other arguments never share a file: `batch` for
a batch of `teqCompiler` and the products' manifest's own operation, `build` for `teqBuild`'s `teq
compiler build`, `link-<directory>-<path digest>` for a link's resident `teq compiler watch` and
`full-<directory>-<path digest>` for a full link's `teq compiler build`. Windows refuses a command
line over 32,767 characters, and a class path of a few hundred jars runs past it; the same file
serves on every platform (`docs/TARGETS.md`, "Argument files"). The debug log's `teq: starting`
line of a link's resident prints the arguments themselves. A write removes its kind's files older
than an hour, and `clean` removes them all, keeping the rest of `target/teq` (the lock
`teq.lock` where the export writes it there, the binary's copy in `bin/`, the links' directories
and their locks).

## Publishing the binary

The binaries go to the GitHub release alone, `teq-<version>-<classifier>` each (`bench/github-release.sh`,
which `bench/ship-publish.sh` runs), from what [`binary/`](binary) stages. The ship stages five, all built on a Linux x86-64 machine by
`bench/ship.sh` from one commit and linked by the pinned zig (docs/SPEED.md, "The ship from
Linux"): `osx-aarch_64`, built for macOS by `bench/cross-ship.sh` and guided by an aarch64 trainer
(natively on an arm64 Linux machine, or under qemu-user); `osx-x86_64`, guided by the Linux training;
`linux-x86_64`, `bench/pgo.sh ship`'s, against glibc 2.28, which passed `tests/run.sh` there;
`linux-aarch_64`, guided by the aarch64 trainer, whose `tests/run.sh` ran over the floor's glibc,
natively on an arm64 machine or under qemu-user; and `windows-x86_64`, plain, smoked under wine.
Which of them a ship publishes is `bench/ship-qualified.txt`'s say, by route; the others are held
back, and a release may lack them. An Apple-silicon Mac running an x86-64 JVM (under Rosetta) has
`os.arch` x86_64 and resolves the `osx-x86_64` binary, which runs under Rosetta. `stage.sh <binary>
<classifier>` copies each into `binaries/` beside its manifest, which holds the binary's digest,
the version its `--version` prints (a Mac binary cannot be run where it is built), the commit, the
training and the toolchain, and refuses a binary its manifest does not vouch for (the digest, and
the version the binary itself carries, before the commit is believed); `stage.sh`
without a classifier copies this machine's `target/ship/teq` without a manifest, which nothing
publishes.

`binary/check.sh [<classifier>...]` checks the set `bench/ship.sh` staged as the release's draft takes
it, before anything is published: the release's inputs as the head commits them, each binary its
manifest's, built from the head and printing the release's version, one staging invocation, the
toolchain of `bench/ship-qualified.txt`, and a version the GitHub release serves nothing of. The
binaries never go to Maven Central, whose monthly limits per organisation a release's five binaries
exceed.

`publish.sh [--check | --preflight | --rehearse [<version>]]`, beside this README, publishes the plugin
alone to Maven Central: sbt-pgp's `publishSigned` stages it into `localStaging` (`project/Central.scala`:
the pom Central requires, the sources jar, a javadoc jar holding a README), into the release's record
`out/central/sbt-teq-<version>/` at the checkout's root; `central.py` checks the staging against the
plugin's files and uploads its bundle to the Central Portal as a USER_MANAGED deployment, promotes that
deployment once the Portal has validated it, and reads every file back from `repo1.maven.org`. The
build neither uploads nor holds the Portal's token, and lacks sbt's `sonaUpload` and `sonaRelease`. A
run that stops after the upload resumes the recorded deployment when run again. It needs the Portal's
user token in `~/.sbt/sonatype_central_credentials` (`host=central.sonatype.com`, `user`, `password`;
the file alone, mode 600), the signing key of `bench/ship-release.sh` in gpg's keyring without a
passphrase, its public key on keyserver.ubuntu.com, `gpg`, `python3` and `sbt` on the `PATH`;
`--preflight` checks them all and that the Portal holds nothing of the version. `--rehearse` stages,
uploads and validates `<version>-rehearsal` as a release and then drops it; each deployment may count
against Central's monthly limit of releases.

The plugin's version is its own line's, `plugin-version.txt`'s beside this README (`bench/release.sh
--plugin <version>` moves it), apart from the compiler's, which `Cargo.toml` and `Cargo.lock` agree on
(`bench/release.sh <version>`); the compiler names the plugin it selects, and the plugin's `BuildInfo`
the compiler it was released with, its default `teqVersion`. Before it publishes, `publish.sh` refuses
plugin inputs the head does not commit, a version that is not a release's, a version Central serves
anything of or of which a deployment is recorded, and what `--preflight` checks; `--check` stops
there. The binaries are no part of it: they go to the GitHub release (`bench/ship-publish.sh`), which
refuses a binary of the five missing, one built from another commit than the head or printing another
version than the release's, and a version of which anything is served, after `bench/ship.sh` has
qualified them (`stage.sh`'s manifest, a Linux binary's suite on those bytes, the toolchain of
`bench/ship-qualified.txt`).

```
bench/release.sh 0.1.7                                                              # the compiler's bump, a commit on master
bench/release.sh --plugin 1.0.0                                                     # the plugin's, when it changed
integrations/sbt/publish.sh --rehearse                                              # after a change to the plugin's publication
bench/ship.sh --publish --plugin 1.0.0                                              # the ship and its publication (bench/ship-publish.sh), on a Linux runner
bench/ship-publish.sh --resume 0.1.7 --plugin 1.0.0                                 # a publication taken up from its record
bench/release.sh --pin --commit                                                     # the example's and the documents' pin, committed
```
