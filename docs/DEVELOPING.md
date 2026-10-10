# Developing teq

How the repository is laid out, how teq is built and tested, what a change passes before it merges (the
landing gate and its timing lines), the conventions of the property-based tests and the launchers, and how a
release is made: its changes, its GitHub release and the workflow that publishes it. teq
is a Scala 3 compiler in Rust with no dependencies; the documents under `docs/` are the ones the README's
table names. Every script under
`tests/` and `bench/` opens with a header that says what it checks or measures, its flags and its
environment variables: read it before running the script. This file names the entry points and the rules
the headers do not.

## Layout

```
src/main.rs        the command line: `teq compiler build|check|watch`, `teq interp`, `teq tasty`,
                   `teq classfile`, `teq lsp`, and the project verbs (compile, test, run, stage, watch,
                   build, dev, stop), which src/task/ serves
src/lexer.rs       tokens plus Scala 3 indentation (INDENT / OUTDENT / NEWLINE); token.rs the tokens,
                   intern.rs interned names, names.rs the well-known ones
src/parser/        recursive descent into the flat AST (src/ast.rs)
src/frontend.rs    parallel lex and parse, name remapping, the std files parsed with the program
build.rs           generates the std index (src/stdindex.rs); stamps the commit into `teq --version`
src/shape.rs       whether an edit changed only bodies
src/typer/         namer, lazy completion, exports, inference, givens, patterns, inline and macros → the
                   typed IR (src/tir.rs); stdlib.rs the std entered on demand, loader/ the classes and
                   bodies of jars, incremental.rs a watch session's retypes, thread.rs a session's typing
                   thread, profile.rs `--profile`, restrict.rs the dialect flags of src/dialect.rs;
                   merge/, merge.rs, bundle.rs, prep.rs the parallel typer
src/types.rs       the hash-consed type store; types/view.rs its worker-view checks
src/symbols.rs     the symbol records; arena.rs, shared.rs, crew.rs, shake.rs, measure.rs the arenas and
                   the parallel typer's shared structures, merge threads, schedule shaker, measurement
src/emit/          typed IR → JavaScript: reach.rs the dead-code walk, layout.rs, module.rs the ES module
                   framing, share.rs and outline.rs the shared inline expansions, runtime.rs the helpers
                   of rt.js the output uses, kept.rs a session's kept reach
src/jvm/           typed IR → class files and jars: classfile.rs (stack map frames), gen.rs, classes.rs,
                   pattern.rs, intrinsic.rs (@jvm templates), analysis.rs and api.rs the analysis for
                   build tools, kept.rs a session's kept class files
src/interp/        the interpreter over the typed IR: `teq interp` and macro expansion
src/tasty/         the TASTy reader and printer (`teq tasty`); write/ the TASTy writer
src/scala2/        Scala 2 pickles rewritten as TASTy; src/classfile/ Java class files
src/classpath.rs   `--classpath`; zip.rs jars, jarcache.rs the jar cache, products.rs `--products`
src/write.rs       the output file, or the changed modules of a split build
src/watch.rs       `teq compiler watch`; held.rs its `stats`, index.rs and complete.rs the resident check's
                   navigation and completion; src/lsp/ `teq lsp`
src/task/          the project verbs over the committed export and teq.lock;
                   runner/ the JVM test runner's committed classes
src/alloc.rs       the allocator (docs/SPEED.md, "A session's memory"); memory.rs the machine's
                   memory for the automatic worker count; report.rs `--time`; planted.rs TEQ_BENCH_SLOW
std/               the standard library in Scala; @js("...") and @jvm("...") mark intrinsics; javalib/ the
                   Java platform layer, js.scala the JS value API, jvm.scala the JVM runtime
runtime/rt.js      the JavaScript runtime
tests/             the suites (tests/all.sh lists them); tests/support/ what the runners share
bench/             generators, budgets and measurements; bench/app the application corpus (below)
proptests/         the property-based tests, a crate of its own (proptests/README.md)
integrations/      sbt/ the sbt plugin (for sbt 2 and sbt 1), vite/ and vite-scalajs/, zed/ the Zed extension, claude-code/
tools/             launcher/ (the launchers, below)
```

`bench/runtime/*.scala`, the programs whose output speed is measured (`bench/runtime.sh`,
`tests/runtime.sh`), are written in the subset teq, scalac 3.8.4 and Scala.js all take unchanged: no
`@js`, `@jvm` or `derives`, no printed `Double` (it prints differently on JavaScript), every traversal of
a `Map` or `Set` over sorted keys (teq keeps insertion order where scalac hashes). Each ends in one
checksum line of `Long`s, compared with scalac's before anything is timed, and takes its size from its
`val defaults` line, which the driver rewrites, since Scala.js passes `main` no arguments. They are
`tests/cases/bench_*.scala` too (symlinks), so the ordinary suites keep them correct.

### The application corpus

`bench/app` generates a synthetic application with the shape of a production Scala.js code base: a
frontend, an HTTP API and the model they share, written for a fictional observatory network (root
package `meridian`). `python3 bench/app/gen.py <out>` writes three trees, `shared/`, `frontend/` and
`api/`, deterministic from `--seed` (default 7), sized by `--scale`; its header lists the flags, each of
which switches in a construct teq lacks. scalac is the reference: `bench/app/expected` holds its output
of both sides' self-tests (`tests/app.sh --regen`), each ending in a checksum, and no `Double` reaches
the output.

- The shape: `bench/app/survey.py` counts a Scala tree without recording any of its content;
  `shape.json` is its survey of the application the corpus is modelled on, numbers only.
  `bench/app/compare.py <out>` prints a generated corpus against it module by module; the distributions
  the generator draws from are in `genlib/shared.py` (`CLASS_SETS`, `ENUM_SETS`, `ADT_SETS`) and its
  sibling modules.
- Provenance: nothing of the application's code, names, domain or text enters the corpus. Its tree is
  read by `survey.py` alone; every name comes from `genlib/vocab.py`, from the frameworks hand-written
  under `bench/app/src`, or from the standard library and the jars. A change to the generator keeps that.
- `bench/app/gaps/<name>`: one small program per construct teq lacks or treats otherwise, scalac's
  output and teq's message in its header, the jars it needs on a `// jars:` line.
- `bench/app.sh` measures the corpus; the budget's rows `realistic-frontend` and `realistic-api` are
  its two sides checked. The other scripts under `bench/app` (`export-identity.sh`, `api-check.sh`,
  `export-accept.sh`, `chain.sh`, `api-scalac-diff.sh` with `diagnostics.py` and
  `scalac-diagnostics.scala`) are the gate's application lines, which read a production application's
  checkout that the repository does not hold, through the lists `app-lists.sh` writes.

## Building and testing

```
cargo build --release            # thin LTO, target/release/teq: what the suites and the gate run
cargo build --profile fast       # for iterating: an edit rebuilds in seconds, the binary types ~8x slower
cargo build --profile checks     # release with debug assertions, target/checks/teq (below)
./bench/pgo.sh                   # profile-guided, target/pgo/use/release/teq: what bench/budget.sh measures
cargo test --release             # unit tests
./tests/rust-warnings.sh         # cargo check of every target, release and checks profiles, warnings denied
./tests/all.sh [suite...]        # every suite, or the named ones, one summary line each
```

Recording expectations: `tests/run.sh --regen` (the `.expected` files from scala-cli),
`--update` on `tests/run_jvm.sh`, `run_interp.sh`, `run_stdlib.sh`, `tests/scala3/run.sh` and
`tests/scala3-typing/harness.py` (their allow-lists), `SIZE_RECORD=1 ./teq interp tests/size.scala`,
`BUDGET_RECORD=1 bench/budget.sh`, `RUNTIME_RECORD=1 tests/runtime.sh`,
`WATCH_MEMORY_RECORD=1 tests/watch-memory.sh`. A `// teq: <flags>` line in a test passes flags to teq; a
`// os: windows` or `// os: unix` line keeps a case of `run_interp.sh` and `run_jvm.sh` to that system, where
Git's bash is Windows (the `windows_*` cases are the interpreter's acceptance on Windows, their expectations
the Windows JDK's). `bench/budget.sh` and `tests/runtime.sh` skip on any machine but the one that recorded their
budgets.

Evidence that a change leaves the output alone, or what it moves:

```
tests/support/identity.sh A B out/identity   # every case built by two binaries, compared byte for byte:
                                             # JavaScript, class files, diagnostics (REF_FLAGS, NEW_FLAGS)
python3 tests/support/classdiff.py A B       # two trees of class files as parsed structures
./bench/compare.sh A B                       # two binaries interleaved on every budget row
./bench/instr-compare.sh A B                 # instructions retired on the budget's programs, one worker
./bench/layout.sh plain|pgo                  # whether a no-op edit moves a row
TEQ_INLINE_COUNTS=<file> ./tests/run.sh      # the inline calls expanded by substitution and by retype
```

`--time` prints the phases, the reach's and the emit's parts (`TEQ_PART_TIMES=0` turns the parts off,
the control for their cost) and the parallel typer's fork and merge parts. `--profile` attributes the
type phase to searches and expansions (docs/SPEED.md); run it at one worker (`--threads 1`): at several
it holds gigabytes. The other diagnostics are `TEQ_*` environment variables, each documented where it is
read (`git grep -n 'TEQ_[A-Z_]*'`).

The scala3 harnesses (`tests/scala3/`, `tests/scala3-typing/`) run the Scala 3 compiler's own tests
from a checkout in `$SCALA3` (by default `teq-ref/scala3` beside the main checkout) and skip without
one; each directory has a README. The checkout is at scala3's `main`, not the 3.8.4 tag, so a test
`main` added is no difference from scalac 3.8.4. Running the output needs node (`Long` is a `BigInt`) and Java 21 or
later for the JVM target (the runtime calls `MatchResult.group(String)`).

The sbt plugin is a project of its own, built by sbt 2, outside `tests/all.sh`: one source tree, two modules,
sbt 2's (Scala 3) and sbt 1's (Scala 2.12, `^^1.13.0`; `integrations/sbt/README.md`, "The two sbt lines"):

```
(cd integrations/sbt && sbt testFull '^^1.13.0' testFull)
(cd integrations/sbt && sbt --batch 'set version := "0.1.1-<branch>-SNAPSHOT"; ^publishLocal')
TEQ_PLUGIN_VERSION=0.1.1-<branch>-SNAPSHOT TEQ=$PWD/target/release/teq integrations/sbt/example/check.sh
TEQ=$PWD/target/release/teq integrations/sbt/axes/check.sh
```

`example/check.sh` is sbt 2's; `axes/check.sh` publishes the checkout's plugin on both lines into a repository of
its own and checks the two over the fixtures under `axes/`.

A branch publishes the plugin only under a SNAPSHOT version of its own, never a release's:
`~/.ivy2/local` comes before every repository, so a local publish of a release's version would stand in
for that release in every sbt build on the machine. `check.sh`
never runs in the application's checkout. The Zed extension (`integrations/zed`) is a crate of its own:
`cargo test` there; its dev build needs rustup's `wasm32-wasip2` target.

### The assertion-enabled build

`cargo build --profile checks` gives `target/checks/teq`, whose `teq --version` ends in `assertions`:
the release build with debug assertions and line tables, so every `debug_assert` is compiled in, the
worker-view checks of the type store (`src/types/view.rs`) among them, which the release and fast
profiles compile out. A violation panics with its site; `TEQ_VIEW_CHECKS=report` lists the cascades
instead. In this build the merge of a forked build is swept for worker ids and overlay types left behind,
and checked against the serial walk's order under `TEQ_MERGE_TRACE=1` (`TEQ_MERGE_SERIAL=1`, in any
build, runs the serial walk, the check's reference); `TEQ_SESSION_INVENTORY=<file>` lists what a
session holds across builds. `cargo test --profile checks` runs the unit tests with the checks' refusals.
This build checks correctness only: the budgets and the instruction rule are the plain release's.
Not every assertion holds today: the emitter's `class Function1 is referred to but not numbered`
(`src/emit/mod.rs`) fires on most programs built with `--std=scala-library`, so a run is compared with
the same build's baseline.
`tests/fork-one.sh` runs the suites through the fork against one worker's output, under this build
with `TEQ=target/checks/teq` (its header has the modes).

## The landing gate

`tests/gate.sh` is what a change passes before it merges; it runs on the maintainers' machines with
the scripts under `tests/` (its header has the lines and their bounds). A contributor runs
`tests/all.sh`. Its machine lines reach the remote machines through the scripts `REMOTE_AGENT` names.
Its application lines (`--app`) read lists that stay outside the repository with the application.
`bench/app/app-lists.sh <checkout> <project> <dir>` writes them from the application's sbt build, through
sbt-teq's `teqInputs` (the plugin added to that run alone; until a release has the task, a local SNAPSHOT
publish that `TEQ_PLUGIN_VERSION` names): the API's main modules, class path, teq flags and scalacOptions
(`APP_MODULES`, `APP_CLASSPATH`, `APP_FLAGS`, `APP_SCALAC_OPTIONS`) and its test configuration's
(`APP_TEST_*`), whose class path names each main module's products by a line `@<project>/<configuration>`
where sbt's has them; it refuses lists whose directories, expanded as teq expands them, are not exactly
sbt's sources, and records each list's configuration and source count. `app-api` checks the main lists
against master's diagnostics; `app-api-test` the test sources the same way, over each binary's own
`--products` build of the main lists, the boundary sbt compiles the tests across (a macro body the
products withhold fails there, not in one program); `app-chain` builds each main module over its
upstreams' products and compares with the whole build, its permitted mismatches `CHAIN_KNOWN` (the format
of `bench/app/chain.sh`'s header; none when unset, every mismatch failing the line); `app-cypress` runs the
application's end-to-end suite through the script `APP_CYPRESS` names, over the binary's own build of the
application (its services, its API and its served bundle), a failing spec failing the line, since a build's
identity cannot see a change of run-time semantics (skipped without the script).

`app-scalac` runs only when `--only` names it, at each release and nightly, not at a landing (scalac's
two compiles take minutes): scalac 3.8.4 as the oracle of teq's diagnostics on the main and test lists
(`bench/app/diagnostics.py`, whose docstring has the passes, the records and the known list's format;
`tests/scalac-oracle.sh` its own checks, which the line runs first). Both compilers run with warnings
enabled (the build's suppressions and promotions left out, the options teq ignores printed), their
diagnostics compared as multisets of structured records, and the exit codes under the build's own options
apart. A difference the known list `SCALAC_KNOWN` does not pin by both records' digests and its count fails
the line, and so does a line of that list whose difference is gone. Each new difference is classified
before it is listed: a defect of teq's (a task-list line), a recorded departure from scalac, or a warning of scalac's
teq has no rule for.

## The timing lines on a machine

The gate's `budget`, `runtime` and `sentinel` lines compare the head's plain release build with
master's on one remote machine, over the budgets of `bench/budgets.txt`, `bench/runtime-budgets.txt`
and the sentinel's `bench/reference.txt`; `bench/pairs.py` is the estimator (its docstring has the
statistic, the schedule and the verdicts).

## Property-based tests

`proptests/` is a crate on Hegel, apart from the compiler (which keeps no dependencies); its README has
the properties, the settings and how a run is replayed. `./tests/prop.sh` runs them (100 cases, seed 1);
`tests/all.sh` runs it as the `prop` suite, outside the gate's groups. A failing run prints the shrunk
history and a `#[hegel::reproduce_failure("...")]` line; put under the test's attribute, it replays that
history alone. What is committed of a failure: a compiler defect goes into `proptests/src/known.rs` with the
narrow precondition that keeps its trigger out of the generators (`TEQ_PROP_KNOWN=<name>` puts it back,
which is how the defect is shown again and its repair checked; the entry goes with the repair), and its
reduced program under `tests/pending-cases/` when scalac accepts it and teq rejects it; a session's
defect is kept as its history with its report until the repair lands.

## The launchers

`tools/launcher/teq` (POSIX sh) and `tools/launcher/teq.cmd` (cmd) are what an application that takes teq
as its build tool (`teqBuildTool := true`) commits at its root beside `teq.lock`, so that nobody installs
teq: `./teq <args>` runs the pinned binary with the
arguments as given and returns its exit code. Each reads the lock's first line and its platform's
`binaries` line and nothing else. sbt-teq
carries them (its build copies them into the plugin's resources under `dev/teq/sbt/launcher/<version>/`,
with every version shipped before from `tools/launcher/shipped/<version>/`), and `teqExportAll` writes
them under `teqBuildTool` (without it the lock goes to `target/teq/teq.lock` and no launcher is written),
replacing one only while it is an unedited version it carries. A change to a launcher is a new
version: its marker line (`# teq launcher <n>:`, `rem teq launcher <n>:`) goes up and the version it
replaces moves under `shipped/` (launcher 1 has, for launcher 2, which refuses a release before 0.1.7 and
names `TEQ` and the release to pin for an empty table; launcher 2 has, for launcher 3, whose `teq.cmd` reads a
valid lock under wine's `cmd` as under Windows', and whose two launchers find coursier's copy of a URL where
coursier keeps it), so that applications committing the old one get the new one at their next export.
`tools/launcher/.gitattributes` keeps `teq` LF and `teq.cmd` CRLF; an application needs the same lines (`teq text
eol=lf`, `teq.cmd text eol=crlf`, `teq.lock text eol=lf`).
`tests/task.sh` runs `teq` under each shell present and shellcheck where installed, and `teq.cmd` under wine's
`cmd` where wine is installed, files named as `teq.cmd`'s variables in its working directory kept across each
resolution. Every implementation of the URL's place in coursier's cache (`src/task/fetch.rs`, vite-plugin-teq, the
export checker and both launchers) is tested on `tests/support/coursier-files.txt`, coursier's own answers:
`tests/support/coursier-files.scala` calls coursier-paths' `CachePath.localFile` (the coursier sbt 2.0.8 embeds) on
each URL under scala-cli and writes the file column, and `tests/task.sh` runs its `--check`. `teq.cmd` runs on
Windows at each release, in the release workflow's Windows job (`tests/windows.sh`: cmd.exe fetching the pinned
binary cold by a lock, running it warm, refusing another sha1); between releases its wine run is the check, short of what wine lacks (its
`certutil` prints nothing and it has no `curl.exe`, so the digests and the fetch are Windows' alone). Wine's `cmd`
(10.0) is not Windows': a substring past a value's end is its last character where Windows' is empty, and `echo(`
prints its parenthesis; `teq.cmd` takes every substring at an offset inside its value and echoes plainly.

## The repository's scripts

The repository's own scripts are Scala, run by teq's interpreter through the repository's launcher from its
root: `./teq interp tests/size.scala -- <args>` (`teq.cmd interp tests\size.scala -- <args>` on Windows), the
two entry points. The root holds what every project that takes teq as its build tool holds: `teq.lock`, the
example's first three sections (the release, the lock's format, the binaries; `bench/release.sh --pin` copies
them again at each release), and the two launchers, copies of `tools/launcher/`'s that
`integrations/sbt/example/check-export.sh` keeps identical. The launcher's rule stands: `TEQ` names the binary
when set, else the lock's release runs.

A script is one `.scala` file with a `@main def` taking the arguments after `--` as `String*`, which brings the
library of `tools/script/` with a directive whose path is relative to the script:

```scala
//> using file ../tools/script
// What the script checks and how it runs: `./teq interp tests/x.scala [-- --teq <binary>]`.
import java.nio.file.{Files, Paths}

@main def x(argv: String*): Unit = Script.run {
  val args = new Args(argv, "tests/x.scala [--teq <binary>]")
  val teq = args.teq("./target/release/teq")
  args.exactly(0)
  val r = Sh(teq, "compiler", "check", "tests/x").timeout(60).check(false).run()
  println(if r.ok then "x: passed" else s"FAIL x\n${r.errTail}")
  Script.exit(if r.ok then 0 else 1)
}
```

- `Sh`: children with the JDK's processes and no shell. `Sh(cmd*).run()` a finite command, its outputs spooled
  to files, its stdin closed unless given, killed with its descendants at its deadline (status 124, as
  `timeout` gives), a status other than 0 thrown unless `check(false)`; `start()` a resident child with its
  pipes for a protocol (`Frames`, JSON-RPC's frames); `Sh.pool(n, jobs)` at most n children at once, each job
  a chain of commands. The interpreter has no threads: what runs at once runs in children.
- `Script.run { }`, `Script.exit(status)` and `Script.atExit`: handlers run once however the body ends,
  SIGINT and SIGTERM included (a shutdown hook, which `teq interp` runs on its main thread).
- `Args` (flags, options, positionals, a usage line on a mistake, `--teq` built in), `Json` (a parser and
  a printer that writes what Python's `json.dumps` wrote), `Log` (a timestamped line, `die`).

The compiler a script tests is `--teq <binary>`, else `$TEQ`, else `target/release/teq`. `tests/all.sh` and
the gate set `TEQ` to the tree's build, so their scripts run under the binary they test; a harness that runs
under one binary and tests another says so: `TEQ=<a master binary> ./teq interp tests/size.scala -- --teq
target/fast/teq`.

The lock pins 0.1.8, the first release whose interpreter has the natives the library calls (processes, streams,
file writes, archives, digests, the clock; 0.1.7 has none), so that a fresh clone's `./teq interp tests/size.scala`
runs under the pinned release, as `integrations/zed/package.sh`'s check of the crates' notices does. A script that
calls a native added since runs with `TEQ` naming a binary built from the tree, which the gate and the workflows
build first. `site/stage.scala` uses no library, so that the site builds under the pinned release.

`teq compiler check` and the language server see a script alone; a check names the library:
`teq compiler check tools/script tests/size.scala`. scala-cli runs the same file (`scala-cli run tests/size.scala
-- <args>`), the directive being its own, which is how a script's behaviour is checked against the JDK.

A new script is Scala, never Python, with one exception: the JDK has no pseudo-terminal, so a test that presses
keys in a terminal (Ctrl-C, vite's `q`) keeps one Python helper that spawns the command under a pty and feeds it
the keys, which the Scala test invokes. Today that is the `pty.spawn` call in `check-task-dev.mjs`; it becomes
`tests/support/pty-run.py` when the terminal tests are converted.

## Releases

**The changes.** `CHANGELOG.md` has a section per release, the newest first, `## <version> (<yyyy-mm-dd>)`
over a bullet per change a user meets in the compiler, the build tool, the plugin, the editor or the
binaries' platforms, in public words: no application's name, no word of the work's process, no internal
file. The section is committed before the version bump, which `bench/release.sh` refuses without it
(`bench/changelog.sh` reads it), and it is the notes of the release's GitHub release.

**The GitHub release** is the publication of the binaries, `bench/github-release.sh` in three verbs, whose
header has the interface (the assets' names, the URLs, the binaries' manifest, the exit codes):
`<version>`, once the five binaries are built, staged and qualified, pushes the hub's master to GitHub by a
fast-forward and makes the draft `v<version>` with every asset, read back; `<version> publish`, after the
Central step, publishes the whole draft and reads every asset back from its URL; `<version> abandon`
deletes the draft when the ship gives the release up. Run again from the ship's checkout, a verb resumes
what it began; `<version> check` refuses what the draft would refuse, before a build; `--dry-run` takes the
draft through on the scratch version 0.0.0-dry and abandons it. The script reads the maintainer's token from
`~/.config/teq/github-token` (mode 600), and nothing prints it; in a GitHub Actions run of the repository's
workflow it takes `GH_TOKEN` when the file is absent and pushes no master, the run being on it. `tests/release-scripts.sh` checks the
scripts' decisions without the network, against a stand-in for gh and local repositories.

A release is built and published by a workflow of the repository on GitHub-hosted runners, which calls the
scripts below and writes their credentials from its secrets at each job's start (the Central Portal's token in a
file of the runner's that `TEQ_CENTRAL_CREDENTIALS` names in place of `~/.sbt/sonatype_central_credentials`, the
signing key into a fresh keyring that `GNUPGHOME` names). The order:

1. `bench/release.sh <version>`, the commit "Release <version>"; when the plugin changed, `bench/release.sh
   --plugin <version>`, the commit "Release sbt-teq <version>". After a change to the plugin's publication, one
   `integrations/sbt/publish.sh --rehearse` (each deployment may count against Central's monthly limit).
2. `bench/ship.sh --publish [--plugin <version>]`: the preflights (the GitHub release's, and the Portal's with
   `--plugin`), the five builds and their qualification, then `bench/ship-publish.sh`: the GitHub release as a draft
   with every asset, `SHA256SUMS` and the binary manifest; the plugin staged; the smoke against the staged set
   through a local mirror (`bench/release-smoke.sh --mirror`); with `--plugin`, the plugin's upload to the Portal, its
   validation, its promotion and its read-back; the draft published and read back; the public smoke.
3. `bench/release.sh --pin --commit`: the example, the repository's own lock and the documents moved to the
   release and committed as the workflow's identity, which pushes them; then the site built and deployed from them
   (`site/README.md`).

A job that stops leaves `out/ship/<version>/record.json` (and the plugin's `out/central/sbt-teq-<plugin>/`, the
staged set `out/github-release/`): the workflow keeps them as artifacts, and a rerun puts them back and runs
`bench/ship-publish.sh --resume <version> [--plugin <plugin>]`, which never does a recorded step again; after a
failure before the plugin's promotion, which deletes the draft, the rerun starts afresh, without `--resume`.

### The release workflow

A release is a run of the workflow `.github/workflows/release.yml` on GitHub's hosted runners; nothing of it runs
on a person's machine. What a person does: commit the release's section of `CHANGELOG.md`, land the version bump
that `bench/release.sh <version>` prepares (the commit "Release <version>"), and watch the run. The landing's push
to GitHub's master starts the run; a dispatch of the workflow with the version starts it too, which a push whose
head is a later commit needs (the run refuses such a push and says so). Each step is a script under
`bench/actions/` or `tests/`, whose header has its contract, and runs outside Actions as well; every action is
pinned by its commit. The order:

1. **The admission** (`bench/actions/admit.sh`): the canonical repository's master alone, the release's commit
   found by its subject, on master, the bump of the version its three files agree on; no tag `v<version>` (a run
   resuming another aside); no other run of the workflow queued or in progress (a second run is refused, not
   queued); the image named by `bench/actions/ship-image.txt` built from the commit's recipe and qualified by
   `bench/ship-qualified.txt`; the plugin the compiler selects (`integrations/sbt/plugin-version.txt`) served by
   Central already, and then the release publishes none, or published by this release, which a commit "Release
   sbt-teq <plugin>" on master before the release's states (`bench/release.sh --plugin`), never one up to 0.1.6
   (a dry run publishes no plugin); a resume takes that decision from the record of the run it resumes
   (`plugin.publish`), never deciding again, since Central serves the plugin that run promoted. Every later job
   checks out that commit, never a moving master.

   A history squashed before its first publication meets the same rules: master's first-parent history holds
   one commit "Release <version>", and that commit changes the version its parent's files name. So the squash
   stops before the bump, its files naming the release before, and the commit `bench/release.sh` makes goes on
   it:

   ```
   $ git log --first-parent --format='%h %s' master
   5d2c1e0 Release 0.1.8         # Cargo.toml, Cargo.lock: 0.1.8
   9a41f37 The tree at 0.1.7     # the squash: every file as it was before the bump
   ```

   An empty commit "Release 0.1.8" on a squash whose files name 0.1.8 already is refused (its parent names the
   version), and so is a second commit "Release 0.1.8" on the first-parent history (`tests/release-actions.sh`
   has the three cases).
2. **The builds**, `bench/ship.sh --step` in jobs of their own (`bench/actions/step.sh`), in the ship's images
   (below): the profiles, an earlier release's taken up by the job `profiles` or, when they will not serve, the
   aarch64 trainer and its training, natively on an arm64 runner in the arm64 image, and the x86-64 trainer and its
   training (below, "The profiles"); the guided Linux x86-64 build with its suite, then in a job of its own the identity
   reference of that binary (`tests/support/identity.sh --produce`); the Windows build with its wine smoke; then,
   from the profiles, the Darwin arm64, Darwin x86-64 and Linux aarch64 guided builds, all three on x86-64 (zig's
   cross links, whose bytes the stage and the qualification know); the Linux aarch64 binary's suite natively on an
   arm64 runner (`linux-arm-suite`), in the floor's container, Debian 10's arm64 image by its digest with the
   floor's glibc package installed and verified, so that its loader and teq's restart of itself are the system's
   own, its outputs against the x86-64 suite's, and its record given to the binary's manifest by the stage, which
   stages no Linux aarch64 binary without it; then the stage, the five by one invocation. A machine without an
   arm64 runner makes the same binaries by the emulated route, `bench/ship.sh` without `--step` on x86-64: the
   trainer and the Linux aarch64 suite under qemu-user; each manifest says which route made it
   (`bench/ship-manifest.sh`: the training's trainer and environment, the suite's line and environment). Each job's summary has its steps' times, its peak memory, its disk, its cores and
   its architecture; the products go from job to job as artifacts. Every long command of a job carries its bound,
   and a job's limit is their sum and twenty minutes for a cold setup (`tests/release-actions.sh` checks it; below). The jars of the corpus are fetched by their
   coordinates and cached by `tests/support/jars.sh`; zig's cache is cached; the toolchain is the image's; teq has
   no dependency for a registry cache to hold.
3. **The qualification**, each binary on its platform as soon as its build made it (`bench/actions/qualify.sh`
   on the build's own products; the five builds are jobs of their own, `build-linux-x64`, `build-linux-arm64`,
   `build-macos-arm64`, `build-macos-x64` and `build-windows`, each uploading its binary with its manifest):
   `qualify-macos-arm64` (its signature verified), `qualify-macos-x64`, `qualify-linux-arm64` and
   `qualify-linux-x64` run `tests/run.sh` with no case skipped and the POSIX launcher's cold and warm fetch and its
   refusal of another sha1, the first three `tests/support/identity.sh --compare` against the reference too, which
   they wait for; `qualify-windows` runs `tests/windows.sh`. The stage runs beside them, and the draft waits for the
   stage and the five, and checks that the staged five are the bytes the qualifications ran, compared with the
   staged Linux x86-64 binary's reference. A failure ends the run before anything is published.
4. **The publication**, `bench/ship-publish.sh` a few of its steps at a time (`--step`), each job running
   `bench/actions/release-step.sh publish <steps>` on the staged set: the first, `draft`, as a new publication
   (its preflights, the record made, the draft `v<version>` with its assets read back by `bench/github-release.sh
   <version>`, then compared with the staged set), the others with `--resume`. Without a plugin to publish, the
   job `smoke-local` runs `smoke`, the consumer's smoke against the staged set and the plugin Central serves
   through a local mirror (`bench/release-smoke.sh --mirror`, `bench/release-mirror.py`). With one, the job
   `central` runs `stage,smoke` (the plugin's signed staging made once, `integrations/sbt/publish.sh --stage`,
   and the smoke against it), `upload-begin` (the upload's intent) and `upload` (that staging uploaded,
   `integrations/sbt/central.py upload`), the records persisted between them; the job `central-promote` runs
   `promote` (the draft found whole again, the recorded deployment validated, promoted and published, central.py);
   the job `central-readback` runs `read-back` (`publish.sh`, which resumes the recorded deployment and reads it
   back from Central). Then the job `publish`, the draft published and read back from its public URLs
   (`bench/github-release.sh <version> publish`); `smoke-public`, the public smoke from a
   fresh runner; the pin (`release-step.sh pin`: `bench/release.sh --pin --commit` on the release's commit,
   committed as "Pin the example and the documents to <version>" by the Actions bot and pushed to master without
   force, onto a master that moved only when nothing the pin changes moved and no release came since); the site
   (`release-step.sh site`), when the environment holds its secrets. `bench/ship-publish.sh` refuses a step
   whose record lacks the steps before it.

**The record.** The run keeps `bench/ship-publish.sh`'s record (`out/ship/<version>/record.json`,
`bench/ship-record.py`: the version, the commit, the five binaries' digests, the plugin and whether this release
publishes it, `github.state`, `central.state` and the smokes, to which the workflow adds `workflow.*`, `pin.*`
and `site.*`) as its artifact `ship-record`, uploaded before each step that writes outside the run and after it,
with the plugin's `out/central/` (its signed staging and its deployment) as `central-record`, so that a runner
lost in the middle of a step loses neither. A run taken up after an upload or a promotion resumes the recorded
deployment without the fresh publication's preflight, which refuses a version the Portal holds; an upload whose
runner was lost before its deployment was recorded is refused by the Portal's own guard, the draft abandoned,
and reconciled on the Portal first. A resumed step publishes the binaries the record names, or none. After a failure
the job `conclude` reads it with GitHub's and the Portal's answers and says what the next run must do. Nothing
is deleted before both are asked: the Portal about every deployment the record holds, since a promotion whose job
was lost before its records were persisted leaves them at `uploaded`; an answer that does not tell, or none, keeps
the draft. Before the plugin's promotion nothing is irreversible, so the draft is abandoned and a new run publishes
again from scratch; after the promotion the draft and its assets are kept and the run's failed jobs are rerun,
or the workflow dispatched with `resume_run=<the run's id>`, which publishes the same bytes from the first run's
artifacts (kept 90 days) and never promotes again; after the GitHub publication the assets are immutable, the
read-back, smoke, pin or site is run again, and a defect needs a new version.

**The profiles.** A release trains no profile of its own by default: its builds are guided by the profiles of
the release before it, which each release publishes beside its binaries as the asset `teq-<version>-profiles.tar`
(`bench/ship-profiles.sh`: the x86-64 and the aarch64 profile, their trainings' records with the metadata of
teq's crate their names are made under, the ship that trained them, the trainer and its machine's toolchain). A
function whose control flow is unchanged finds its record by its hash; a changed one is built unguided. The job
`profiles` (`bench/actions/profiles.sh`) takes them up from the release the dispatch's `profiles` names, by
default `v<the version the release commit's parent names>`, and the release trains afresh instead (`train-arm`,
and `x86`'s training before its build) when the dispatch says `train`, when that release or its asset is not
there or the asset not whole, when the profiles were trained by another compiler (teq's crate is named by the
compiler's version, so none of their records would be found), or when they are stale for the tree: more than 10
of the x86-64 profile's 200 hottest functions changed, by its guided build's own report (`bench/pgo.sh ship
stale`; on record, one release changed 3 and 5 of them, two releases 7). The binaries' manifests say which
profiles guided them: `training release` the ship that trained them, `training source <tag> <sha256>` the release
and asset they were taken from, `profile` and `profile-original` their digests, `training trainer` and `training
tuple` the trainer's environment. The first release built here, 0.1.7, trains nothing and takes the profiles of
the 0.1.6 ship made on the reference machine, held by the asset `teq-0.1.6-profiles.tar` of a pre-release that
precedes the run (`bench/ship-profiles.sh write` of that ship's tree completes its records: the metadata from its
`target/cross/metadata`, the trainer's machine and toolchain from its Linux x86-64 binary's manifest); a dispatch
names that pre-release's tag in `profiles` unless it is `v0.1.6`.

**The runners.** Each job names its runner by a repository variable, GitHub's standard runner when the variable
is unset: `TEQ_RUNNER_LINUX_X64` (`ubuntu-24.04`: the admission, every build, the reference, the stage, the
publication and the Linux x86-64 qualification), `TEQ_RUNNER_LINUX_ARM64` (`ubuntu-24.04-arm`: the aarch64
training in the arm64 image, the Linux aarch64 suite in the floor's container, the Linux aarch64
qualification), `TEQ_RUNNER_MACOS_ARM64` (`macos-15`), `TEQ_RUNNER_MACOS_X64` (`macos-15-intel`) and
`TEQ_RUNNER_WINDOWS` (`windows-2025`), the qualifications on those platforms; the workflow `ship-image` takes
`TEQ_RUNNER_LINUX_X64`'s. A variable holds one runner label. A larger runner is made in the organization's
settings and given to the repository first (a Linux or Windows one's name is its label; macOS larger runners take
GitHub's labels, such as `macos-15-xlarge` for arm64 and `macos-15-large` for Intel), and is billed by the minute
even for a public repository; setting a variable makes no runner. A Linux variable names a Linux runner of its
architecture with Docker, since those jobs run in the image of that architecture (`TEQ_RUNNER_LINUX_ARM64`'s pull
the arm64 image and run the floor's container) and the caches are the container's (`/github/home/.cache`: zig's,
which `TEQ_ZIG_CACHE` names, keyed by the runner's architecture, and the corpus's jars, which are the same bytes on
either) or the runner's (`~/.cache/coursier/v1`). A larger runner changes the cores, the memory and the disk: the
builds are fat LTO in one codegen unit, mostly one thread in LLVM, so more cores shorten them less than in
proportion; the trainings and the suites run their programs one at a time; the stage and the publication wait on
the network. The limits stay the sums of the bounds whatever the runner, and the bounds are estimates until the
first native runs' summaries measure them: the native training's 60 seconds a run (on four x86-64 cores
the slowest run takes 7 seconds and the 65 checks 46 together; under qemu-user, 120), a fat-LTO build's
1200, twenty minutes for a cold setup (the image pulled, the checkout, the artifacts; the jars' and zig's caches
missed, whose fetches have bounds of their own), a few minutes once the caches hold. Depot's runners
(depot.dev) are GitHub Actions runners too, whose labels go into the variables unchanged: `depot-ubuntu-24.04-16`
for `TEQ_RUNNER_LINUX_X64`, `depot-ubuntu-24.04-arm-16` for `TEQ_RUNNER_LINUX_ARM64`, `depot-macos-15` and
`depot-windows-2025`, the size by the suffix (`-4`, `-8`, `-16`, ...); the defaults stay GitHub's. The landing
gate's workflow (`.github/workflows/gate.yml`, pull requests) stays on GitHub's hosted runners.

**The images.** The Linux jobs run in `ghcr.io/carrot-inc/teq-ship`, one image per architecture from one recipe,
`bench/actions/ship.Dockerfile` and `bench/actions/toolchain.sh` (Ubuntu 26.04 by the digest of its index, its
packages from a snapshot of the archive, the Rust toolchain, zig, node, sbt, scala-cli and gh at pinned versions,
each architecture's archives by their own digests; qemu-user, the sysroot and wine on linux/amd64 alone, where the
emulated work and the Windows smoke are), each pulled by its own digest; each job checks first that its image is
the commit's recipe (`toolchain.sh check`). A change of the recipe is followed by the workflow `ship-image` (run by
hand from master): with the repository variable `TEQ_DEPOT_PROJECT` set to a Depot project's ID (the variable
alone holds it), Depot's builders build both images, each on its own architecture, the run trusted by its OIDC
token (the project's trust of the repository, no secret); without it, buildx on the runner, the arm64 image under
emulation. Both push the same tag, and the run's summary gives the two lines for `bench/actions/ship-image.txt`
(`bench/actions/image.sh record`), committed; then by a dry
run, whose five binaries must pass their qualification, after which a commit gives `bench/ship-qualified.txt` the
lines the stage job's summary lists: the images (`image <reference>`, `image-aarch64 <reference>`), the floor of the
Linux aarch64 suite (`floor-aarch64`, `node-aarch64`) and the tuple the binaries record. Until then the workflow
refuses to publish; once the file names an image, a binary built elsewhere is not published.

**The dry run**: a dispatch with `dry` checked rehearses master's version: the builds, the qualification, the
scratch draft of `bench/github-release.sh --dry-run` (made, resumed and deleted) and the consumer's smoke against
the local mirror of the five new binaries; no Central, no publication, no pin, no site.

**Settings and secrets**, the maintainer's: the environment `release`, its deployment branches master
alone, with no required reviewer (the maintainer's choice: a release runs unattended; a reviewer added there gates
every publishing job), holds the secrets `CENTRAL_USER` and `CENTRAL_PASSWORD` (the Portal's user token),
`GPG_PRIVATE_KEY` (the armored signing key that `bench/ship-release.sh` names, without a passphrase) and,
optionally, `NETLIFY_AUTH_TOKEN` and `NETLIFY_SITE_ID` (the site's deploy); `bench/actions/credentials.sh` writes
the first three into files of the runner's for the jobs that ask the Portal or sign (the draft's preflight with a
plugin to publish, the three Central jobs, the conclusion) and removes them. The jobs ask for their token's
permissions one by one (contents write for the draft, the promotion's draft check, the publication, the pin and
the conclusion alone).
Master has no branch protection; a ruleset added on it must let the Actions bot push the pin directly.

**The hub and GitHub.** Master's source is the hub; every landing pushes master to GitHub as well. Before a
landing, `bench/actions/before-landing.sh` refuses while a release run is queued or in progress, and
fast-forwards the hub's master to GitHub's when the workflow pushed a pin there (refusing any other commit
GitHub holds). The workflow never rewrites master, never forces a push, and makes no tag but the release's,
which its publication makes.
`tests/release-actions.sh` checks the workflows' shape and the scripts' decisions without GitHub, against
stand-ins and local repositories.
