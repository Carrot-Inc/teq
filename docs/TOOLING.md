# Tooling

How teq fits a build and an editor. An sbt build takes teq through its plugin, which puts teq in the place of
sbt's compiler and exports the build into a lockfile; from the lockfile `teq`, the vite plugin and the
language server work with no sbt running, and a clone builds with nothing installed. The binary's own commands
and flags are in `docs/CLI.md`, the output and the targets in `docs/TARGETS.md`.

The notices of the code teq adapts from other projects, which a program teq compiles carries along with teq's
standard library, are in `NOTICE` at the repository's root, beside every release's binaries, in the vite package
and in the Zed bundle.

## The sbt plugin

sbt-teq is an sbt 2 plugin, on Maven Central. A build switches to teq by adding the plugin and setting
`teqCompiler`; the plugin fetches the compiler for the machine's platform from teq's GitHub release:

```scala
// project/plugins.sbt
addSbtPlugin("build.teq" % "sbt-teq" % "1.0.0")

// build.sbt
teqVersion := "0.1.7"
teqCompiler := true
```

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

sbt 2's `test` is `testQuick`, under either compiler: it runs a suite only if no run has passed it at its
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
with CRLF in its `.gitattributes`, and keeps the build's own files LF there too (below). Every tool looks
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
binary's URL, sha1 and size (`binaries: {}` while the release is not published); then `inputs`, every build definition file with its SHA-256; `repositories`, the
build's remote Maven ones and their credentials hosts; `jars`, every jar a classpath names, once, by its key
`organization:name:version[:classifier]` with its repository, sha1 and size; `java.outputVersion`, the class
files' version; and `projects`, each with its platform, Scala version and configurations (the roots, the full
ordered classpath, the flags, the generators, the declared main classes and the test context of `compile`,
`runtime` and `test`), the `run`, `dev` and `stage` blocks the verbs read, and the description the plugin's
own tasks build from.

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
| `Compile / teqGenerators` | none | `TeqCommand(run, inputs, cwd = ".", outputs)`, a program that writes sources |
| `teqMainClasses` | none | main classes `teq run` takes by name, besides `Compile / mainClass` |
| `teqRunAliases` | none | names for main classes, which `teq run` takes in their place |
| `teqDevCommand` | the `dev` script of the nearest `package.json` | a Scala.js project's dev command |
| `teqBuildTool` | false | the lock and the launchers at the build's root, committed |
| `teqExportSnapshots` | false under `teqBuildTool`, else true | lets the export name SNAPSHOT and dynamic versions |

A `TeqCommand` writes sources into the directory appended to `run` (sbt's `Compile / sourceManaged / "teq"`,
the exported build's `target/teq/<project>/compile/src_managed`, relative to the command's working directory);
sbt's compile runs it, and so does `teq`, again when the files its `inputs` globs match or the files its
arguments name change (its script among them), or when one of its `outputs` is missing: the files or
directories under the build's root the program writes besides that directory, a module of the application's
assets say, which `teq` checks as it checks the directory and whose unchanged files keep their times. The
first word `teq` is the build's own binary, sbt's `teqResolvedBinary` and under `teq` the one running, never
a `teq` of the `PATH`: a Scala script runs through `teq interp`, with no JVM and no other tool on any machine.

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
it refuse at use, naming each reason and its remedy, while the editor and vite read the rest. Recorded are a
source generator other than a `TeqCommand` or sbt-buildinfo's of a shape `teq` writes (the editor then
reads what sbt last generated), a generator of the Test or Runtime configuration, a resource generator, a
`Tests.Filter`, `Tests.Setup` or `Tests.Cleanup`, and a Docker stage `teq` would not write as
native-packager does (a layer function that reads a task, native-packager's classpath and launcher jars,
server archetype, ash script and jlink, a start script, classpath or mappings of the build's own); a Scala
Native project is left out. Without the driver the lock is the machine's own, so a SNAPSHOT or dynamic
version, a path outside the build's root and an artifact from none of the build's Maven repositories are
written as they are; under the driver they are refused, every reason named at once (SNAPSHOT and dynamic
versions where `teqExportSnapshots` allows them), as is in both modes a jar key naming two files.

## `teq`

`teq [--export file] [--strict] <verb> [args]` reads the export at or above the working directory, or the
one named; without a verb it lists the verbs and the projects. A project whose export records what sbt alone
runs for a verb is refused with the reasons; a verb over every project runs the others, names each one left out
and exits 2.

| Verb | |
|---|---|
| `compile [project...]` | types the projects' closures (every project by default) and writes class files |
| `test [project] [pattern...]` | compiles a project's test configuration and runs its suites |
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
options, variables and directory; Ctrl-C reaches it, and its exit code is the verb's.

**Tests.** The runner is a JVM over sbt's test interface, kept per test configuration from its first run; a
JVM older than `java.outputVersion`, or than 17, is refused. Suites are found as sbt's `definedTests` finds
them (`--list` prints them) and the patterns applied as `testOnly` applies them; `--changed` keeps the suites
whose classes depend, transitively, on a class rewritten since the configuration's last full run. Each failure
is reported as `[fail] <test>: <message>` with the frames in the suite's code, then a line per project with
its suites, the passed and failed counts and the times; the exit code is 1 when anything failed. Without a
project every JVM project's suites run; a Scala.js project's are not run yet, and naming one is refused.

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
(`"vite-plugin-teq": "file:<teq>/integrations/vite"` among its `devDependencies`). It depends on nothing but
`curl`, for fetching the binary; `vite` is a peer dependency. The other route to a browser goes through sbt:
with `TEQ_COMPILER=1` the stock Scala.js vite plugin serves what `fastLinkJS` names, teq's output unmodified,
`sbt ~frontend/fastLinkJS` keeps it current and `vite build` bundles the one file of `fullLinkJS`, with the
same swaps, since the refresh runtime and what a swap runs again are written by `--hot` into the output.

## The language server and Zed

`teq lsp` is a language server over stdin and stdout, used by Claude Code's LSP tool and by Zed. It answers
definitions, references, hovers, document and workspace symbols, implementations, the call hierarchy (the
callers and callees of a method, grouped by their enclosing definition), completion and signature help, and
publishes diagnostics after each edit (an unused import as a hint). A definition into a library goes to its
source, read from the sources jar beside the jar, or into teq's own standard library, shown in read-only
documents where navigation works as in a source, and completion and signature help too in the standard
library's. A member called on an object seen as one of its parents
(`(Obj: Base).f`) answers both declarations, the parent's and the object's that runs, as scalac's own lookup
does. Formatting, code actions and rename are not offered; hover shows a name's signature or type, not yet an
expression's.

Completion lists the members of a receiver (extension methods and conversions included), the names in scope,
types, the class after `new`, imports and keywords, filtered by the prefix case-insensitively and cut at
1,000. With a prefix of two characters or more, names out of scope follow, each with the import that brings
it, written after the file's last top-level import; merging into an existing braced import is a later part. A
client with snippet support gets a method's call with a placeholder per parameter (`inc(${1:x})$0`). Signature
help triggers on `(` and `,` and lists every overload with the active one as the build chose it; inside a type
argument list it is a later part.

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
written is a warning on `build.sbt` naming `sbt teqExportAll`. An sbt build without an export is exported by
the server itself, once: it runs `sbt teqExportAll` with the published sbt-teq of its own version loaded for
that run alone, which writes `teq.lock` under the build's `target/teq/` (at its root under the driver), and
again when that file goes, after an `sbt clean`; a run that fails is a diagnostic
on `build.sbt` and runs again once a build file changes. It needs sbt 2 on the `PATH`, fails under a
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
