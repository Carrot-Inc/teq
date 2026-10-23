# Developing teq

How the repository is laid out, how teq is built and tested, what a change passes before it merges (the
landing gate and its timing lines), the conventions of the property-based tests and the launchers, and how a
release is made: its changes, its GitHub release and the workflow that publishes it. teq
is a Scala 3 compiler in Rust whose one dependency is corosensei, the coroutines of the interpreter's threads
(the Cargo feature `threads`, on for every shipped target); the documents under `docs/` are the ones the README's
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
integrations/      sbt/ the sbt plugin (for sbt 2 and sbt 1), mill/ the Mill plugin, vite/ and vite-scalajs/, zed/ the Zed
                   extension, claude-code/
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
./tests/no-caps.sh               # numeric cutoffs added to the typing paths since the base, each naming dotty's constant
./tests/all.sh [suite...]        # every suite, or the named ones, one summary line each
```

Recording expectations: `tests/run.sh --regen` (the `.expected` files from scala-cli),
`--update` on `tests/run_jvm.sh`, `run_interp.sh`, `run_stdlib.sh`, `tests/scala3/run.sh` and
`tests/scala3-typing/harness.py` (their allow-lists), `SIZE_RECORD=1 ./teq interp tests/size.scala`,
`BUDGET_RECORD=1 bench/budget.sh`, `RUNTIME_RECORD=1 tests/runtime.sh`,
`WATCH_MEMORY_RECORD=1 tests/watch-memory.sh`, `./teq interp tests/source-version.scala -- --capture 3.8.4` and
`--capture 3.10.0` (each scalac's captures of the source version's runs, pinned apart; 3.10.0's over a coursier cache
of its own); `tests/lean-expected/<name>.expected` is written by hand, the
output of JavaScript and the interpreter for a case of a departure they keep (`.expected` stays scalac's, the
JVM's). A `// teq: <flags>` line in a test passes flags to teq; a
`// os: windows` or `// os: unix` line keeps a case of `run_interp.sh` and `run_jvm.sh` to that system, where
Git's bash is Windows (the `windows_*` cases are the interpreter's acceptance on Windows, their expectations
the Windows JDK's); a `// std: <modes>` line names the std modes a case's expectations hold under, which
`run_stdlib.sh` (scala-library's) and the lean std's suites (`run_interp.sh`, `split-watch.sh`) honour. The JVM's
expectations are JDK 24's; where another major words a JDK message otherwise, `tests/jvm-expected/<name>.jdk<major>.expected`
holds scalac's output on that major (`run_jvm.sh`'s header). `tests/all.sh` keeps each suite's whole output and time
under `out/all/`. `bench/budget.sh` and `tests/runtime.sh` skip on any machine but the one that recorded their
budgets.

`tests/semanticdb` is the SemanticDB oracle of `teq fix`'s forms: `harness.py --regen` records scalac 3.8.4's
`-Xsemanticdb` documents of each group of its corpus as golds, `--check` holds the manifest and the golds to each other,
`--self-test` proves the comparison refuses damaged documents, and `--teq <binary>` compares teq's documents
(`TEQ_FIX_SEMANTICDB`, written by `teq compiler fix` only for a program that types whole) with the golds field by
field; dotty's own corpus is read from the scala3 checkout at the tag 3.8.4 (the harness's header has the environment).

Evidence that a change leaves the output alone, or what it moves:

```
tests/support/identity.sh A B out/identity   # every case built by two binaries, compared byte for byte:
                                             # JavaScript, class files, diagnostics (REF_FLAGS, NEW_FLAGS)
python3 tests/support/classdiff.py A B       # two trees of class files as parsed structures
./bench/compare.sh A B                       # two binaries interleaved on every budget row
./bench/instr-compare.sh A B                 # instructions retired on the budget's programs, one worker
./bench/layout.sh plain|pgo                  # whether a no-op edit moves a row
TEQ_INLINE_COUNTS=<file> ./tests/run.sh      # the inline calls expanded by substitution and by retype,
                                             # those kept for the inlining phase and their bytes, the
                                             # phase's forked units, reflection's queries and its memo's,
                                             # the files suspended at typing and those typed again
```

`--time` prints the phases, the reach's and the emit's parts (`TEQ_PART_TIMES=0` turns the parts off,
the control for their cost) and the parallel typer's fork and merge parts. `--profile` attributes the
type phase to searches and expansions (docs/SPEED.md); run it at one worker (`--threads 1`): at several
it holds gigabytes. The other diagnostics are `TEQ_*` environment variables, each documented where it is
read (`git grep -n 'TEQ_[A-Z_]*'`).

The scala3 harnesses (`tests/scala3/`, `tests/scala3-typing/`) run the Scala 3 compiler's own tests
from a checkout in `$SCALA3` (by default `teq-ref/scala3` beside the main checkout, a linked worktree's
found through the git common directory) and fail without one, or when a suite they run finds no test; each directory has a README. The checkout is at scala3's `main`, not the 3.8.4 tag, so a test
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
never runs in the application's checkout. mill-teq, the Mill plugin (`integrations/mill`), is built by Mill,
compiling sbt-teq's sources that hold nothing of sbt (`Exported`, `Lock`, `Json`, `Launchers`, `Release`, `Served`,
`Sha1`, `ScalacOptions`, in the syntax both sbt lines' Scala reads) beside its own, and tested outside `tests/all.sh` too: `tests/mill-export.sh` builds it,
publishes it under a version of the test's own into a Maven directory of the test's, and exports
`tests/mill-export/fixture` under Mill 1.0.6 and 1.1.10 against `tests/mill-export/expected-<mill>.lock` (`--record`
writes them), then runs teq's verbs over the export (`TEQ`). A branch publishes it only under a SNAPSHOT of its own
(`TEQ_MILL_PLUGIN_VERSION`) into a directory it names (`publishM2Local --m2RepoPath`), never a release's into
`~/.m2`. The Zed extension (`integrations/zed`) is a crate of its own:
`cargo test` there; its dev build needs rustup's `wasm32-wasip2` target.

The JVM test runner (`src/task/runner/Runner.java`) is sbt 2.0.8's forked test worker, a JVM for each run, the
daemon starting the next once a run ends (docs/TOOLING.md, "Tests."): its suites run on a pool of the lock's
`parallelism`, which the sbt export writes from
`Test / testForkedParallel` and `Test / testForkedParallelism` and the Mill export as `1`. `tests/task.sh` holds it
to the pinned worker itself (`worker-2.0.8.jar` from the coursier cache, driven by `probe.Pinned` over the probe
framework of `tests/support/probe_framework`, the two transcripts made alike by `tests/support/runner_contract.py`)
and runs the pool through teq; a change of `Runner.java` compiles it again with javac 17 (`tests/runner-classes.sh`
says how) and lists each class javac writes in `src/task/runner.rs`'s `CLASSES`.

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
the scripts under `tests/` (its header has the lines, where each runs and their bounds). A contributor runs
`tests/all.sh`. The gate takes thirty minutes at most from its first command to its last, its builds, claims
and transfers included: its last line is that total, and a gate over it is flagged to the owner at once and fixed
at once, by running more at once or elsewhere, never by weakening a check (the owner's rule, 2026-10-14). Its
lines run at once. `--only <lines>` limits it to the lines named, `--with <lines>` adds named ones to its own (the
line it runs only when named, `app-scalac`), and `--without <lines>` leaves named ones out, whatever the others select;
a name of no line is refused. The correctness suites, every suite of `tests/all.sh` but those its `excluded:` lines name
(`tests/gate-coverage.sh` holds the list to `tests/all.sh`), are dealt to the free remote machines, which the
scripts `REMOTE_AGENT` names reach, by their recorded times (`tests/gate-times.txt`, which `--record` rewrites
from a run whose every line passed or whose only lines not passed are timing lines inconclusive, none over: their
durations are not their verdicts'), the head's binary built once by the first of them and shipped to the
others, each checking its digest and that it runs there; the timing lines measure after the suites, on a
machine of their own (below); the project lines, when named, on the machines that hold the projects' checkouts
("lila"), each project's chain dealt to one of them, and every project's records held to master's by the line
`records` ("The project records"); and the size, instruction, memory and application lines on the reference machine, where their budgets were recorded, at once
but the application API's type phase, the application's end-to-end suite, the memory line and the asymptotic probes, which
run alone after them (the type phase's pairs and the suite's specs are timed, the memory line's growth rows and the probes'
growth ratios depend on the machine's load; the probes run under the node `NODE` names there, node 22). The type phase,
`app-api-time`, is `bench/pairs.py api` over the lists the application lines read: the head's and master's
`--time --all-mains` checks of the API at eight workers in balanced pairs, judged as the budget line judges (8%, an
inconclusive row not a pass), every run complete with a positive type phase and the first run's diagnostics (their first
lines and count);
`bench/app/fixture-lists.scala` writes those lists for the public corpus, to run it on any machine.
The longest suites run as pieces, so that the machines' half shrinks with their number: `tests/lsp.sh`,
`tests/jvm-watch.sh`, `tests/modules.sh` and `tests/split.sh` take `--piece <k>/<n>`, which runs the k-th of n parts
of the units they list (a scenario, a program with its checks, a section of sessions; `--pieces <n>` prints the
parts), dealt by the weights of `tests/support/piece-weights.txt` and never by the recorded times, so that a unit
added lands in one part (`tests/support/pieces.sh`); each unit's body stamps itself as it starts, and a piece fails
unless every unit of its part ran exactly once. The gate deals each piece as a suite of its own (`lsp.1/4`, a
row of `tests/gate-times.txt` of its own) and merges their summaries into the suite's row, which fails when a piece
fails or is missing; `tests/all.sh` and a suite run by hand run it whole, or one piece when it is named as
`modules.1/2`, which the GitHub gate's two modules jobs do.
`tests/gate-scripts.sh` checks the gate's own decisions against stand-in machines (`tests/gate-agent/`).
Its application lines (`--app`) read lists that stay outside the repository with the application.
`bench/app/app-lists.sh <checkout> <project> <dir>` writes them from the application's sbt build, through
sbt-teq's `teqInputs` (the plugin added to that run alone; until a release has the task, a local SNAPSHOT
publish that `TEQ_PLUGIN_VERSION` names): the API's main modules, class path, teq flags and scalacOptions
(`APP_MODULES`, `APP_CLASSPATH`, `APP_FLAGS`, `APP_SCALAC_OPTIONS`) and its test configuration's
(`APP_TEST_*`), whose class path names each main module's products by a line `@<project>/<configuration>`
where sbt's has them; it refuses lists whose directories, expanded as teq expands them, are not exactly
sbt's sources, and records each list's configuration and source count. `app-api` checks the main lists
against master's diagnostics (a landing's intended difference, diagnostics it gives or stops giving where scalac
3.8.4 does, named by a file of patterns `APP_API_DELTA` names: `bench/app/delta-source-version-2.txt`); `app-api-test` the test sources the same way, over each binary's own
`--products` build of the main lists, the boundary sbt compiles the tests across (a macro body the
products withhold fails there, not in one program); `app-chain` builds each main module over its
upstreams' products and compares with the whole build, its permitted mismatches `CHAIN_KNOWN` (the format
of `bench/app/chain.sh`'s header; none when unset, every mismatch failing the line); `app-cypress` runs the
application's end-to-end suite through the script `APP_CYPRESS` names, over the binary's own build of the
application (its services, its API and its served bundle), a failing spec failing the line, since a build's
identity cannot see a change of run-time semantics (skipped without the script).

`app-scalac` runs only when `--only` or `--with` names it, at each release and nightly, not at a landing (scalac's
two compiles take minutes): scalac 3.8.4 as the oracle of teq's diagnostics on the main and test lists
(`bench/app/diagnostics.py`, whose docstring has the passes, the records and the known list's format;
`tests/scalac-oracle.sh` its own checks, which the line runs first). Both compilers run with warnings
enabled (the build's suppressions and promotions left out, the options teq ignores printed), their
diagnostics compared as multisets of structured records, and the exit codes under the build's own options
apart. A difference the known list `SCALAC_KNOWN` does not pin by both records' digests and its count fails
the line, and so does a line of that list whose difference is gone. Each new difference is classified
before it is listed: a defect of teq's (a task-list line), a recorded departure from scalac, or a warning of scalac's
teq has no rule for.

The GitHub gate, `.github/workflows/gate.yml`, runs the full suite set of `tests/all.sh` on GitHub's hosted
runners for every push to GitHub's `dev`, where every landing goes, for every pull request (which targets `dev`),
and by hand. GitHub's `master` is green only: it is fast-forwarded to a `dev` head whose run is green in every job,
and never pushed to otherwise. One job per group of suites, each within its bound, every suite in one of them but
`budget`, `runtime` and `watch-memory`, whose numbers are the recording machine's (the workflow's `excluded:` lines;
`tests/gate-coverage.sh` fails a suite neither run nor excluded, and a suite run as pieces must have each of its
pieces named once over the jobs); `tests/gate.sh` runs those on the maintainers'
machines. It needs nothing
of the maintainers' machines: one job builds the release binary, one prepares the jars once (the Maven ones from
the cache `tests/support/jars.sh` keys or from Maven Central, the built ones handed on as an artifact,
`bench/actions/jars.sh`), the scala3 checkout is fetched at the commit the harnesses' baselines name, and each
job keeps its suites' output and times (`out/all/`) as an artifact. Its runners hold Temurin 21 and node 22.

### The project records

Each reference project's gate lines are held to records in its directory under `bench/`, beside the oracle's own
(`tests.json`, the suites and cases the build's own test run passes under scalac 3.8.4, which stays the oracle's exact
inventory, statuses and multiplicity whole): `errors.txt` (the compile line's), `deviations.json` (the test line's)
and `run.json` (the run line's), each a floor a landing may only lower, never a licence (bench/reference/records.scala
has the rules). A record holds its provenance (`baseline`, the first, from master's binary; `record`, a run's
improvement; `migrate <reason>`, a new pin or new inputs; with the revision, the binary, the pin and the digest of the
inputs it holds for: `pin.json` and `generated.json`, and for the deviations `tests.json`), its scope (the units the
run covers, each exactly once: the compile's configurations, the test's projects, the run's named assertions) and its
debt, a sorted multiset:

- a compile's error is its whole block as teq prints it (the location, every line of the message, the source line and
  the caret), keyed by the configuration that reports it, the checkout's root and the machine's paths normalised away;
  a configuration whose build failed outright (a generator's run under `teq interp` among its causes) is one item of
  its whole failure as teq records it (`--diagnostics`, a generator's with its command's whole output, whatever the
  command printed); a generated input whose bytes are not the expected ones (`generated.json`'s, or the pin's for the
  checked-in files of the Scala.js projects) an item with the digest of the bytes it has, so that other bytes take a
  migration; each run's digests are its own;
- a test's departure is a case whose statuses are not the oracle's, with what it showed and what the oracle has, as
  many times as each side reports it (`<project>: <suite>: <case>: Failure (tests.json: Success)`, `...: did not run
  (...)`, `...: Success x2 (...)`), so that a known departure that shows otherwise is another item and one that
  disappears is gone only where the case ran as the oracle's; a project whose suites none ran while its own build
  failed is one item of its own (`<project>: blocked by its build`), its cases none; a suite or a case the oracle
  lacks is an item of its own;
- a run's violation is a named assertion it did not hold (bench/smoke.mjs's, lila's run verb's), by its name, a run
  whose build failed violating every one beside `blocked: <the build that failed>`.

Every test line and every run line is judged by one function of its evidence, the same for lila's, the Scala.js
projects' and Mill's drivers (`Deviations.judge` over a `TestEvidence`, `Assertions.judge` over a `RunEvidence`), each
driver only gathering its invocations, transcripts and events, so that a rule holds for every driver alike. A test
invocation's two channels are collected apart (`test-<n>.log` its stdout, `test-<n>.err` its stderr): `teq test`
writes its own lines on stderr (a build's diagnostics and count, each summary) and the run's stream on stdout (what
the suites print and its report of their events). The judgment reads a build's diagnostics from teq's records of them
(`--diagnostics`: each its whole message and the lines shown for it, the producer's extent, which no message's text
sets, a macro's info among them, beside each failure of a configuration's whole and what each passing generator
printed), the counts and
summaries from stderr past those lines (a one-shot build's `<n> errors found` claiming its errors, which may end its
output uncounted, every record shown whole either way; a run whose program started, `teq run`'s record of it, has the
program's output after teq's lines, which is never read for them), and the outcomes and the suites that ran from the events
(`--events`, a suite that ran no case its own line), never from a stream a run's text shares, nor from a count
(`Transcript`). Every
build that failed counts (a unit of the oracle's blocked by its own, a configuration of no unit an item of its own),
every executed case with its framework's status from teq's events (an invocation that wrote none, outcome lines after
its last summary, a transcript and events counting a unit's outcomes otherwise, a unit's tests completing twice are
problems), a unit blocked only where none of its cases ran, each invocation's exit explained, and a run blocked by its
build only on teq's exit of 1 on that build's own errors. A line's verdict is the whole run: every unit of the scope covered once (a compile's coverage owed by the export, every
configuration with sources of the projects asked for within those the run's summaries build on, never taken from a
transcript), the transcript complete (the verb's end marker `# exit <status>` after teq's output, an exit the
diagnostics give; a test's exit but 0 explained by a case that failed or a blocked build, a project's tests completed
once, no outcome after the last summary), and the observed debt within the record's,
an item outside it failing the line, printed in full (the first ten), an item gone passing and listed. A crash of
teq's (a build's thread that panicked, a resident or the daemon that ended, a panic's message), an error counted
without its diagnostic, a timeout, a cut transcript and a configuration covered twice are no outcome: no record holds
one (dotty's vulpix keeps a compiler crash apart from a test's expected errors, `ParallelTesting.scala`'s
`onComplete`). Each verb's `--record` steps its record from a run whose every run and every other check passed (the
most each run showed of each item), `--baseline` makes the first, from a run that is evidence, with the run's
normalised transcript as its evidence beside the record (`<record>.evidence.gz`), and `--migrate <reason>` makes one
over new inputs, as a baseline does; a recording publishes its record, its evidence and its generated digests as
one, staged beside them (`<file>.proposed`) and admitted against the record they replace first, and a failed one
changes nothing. The landing persists an improvement: the gate's `--record` installs the record steps its project
lines took in the worktree once every line passed (`bench/reference/ratchet.scala install`), for the landing to
commit with the change that made them.

Independently of any run, the gate's line `records` holds every record of the head against master's committed one
(`bench/reference/ratchet.scala check <revision>`, the revision `--master-rev` names, by default master's head, a head
that does not contain it refused until rebased), the records of either revision enumerated, so that a floor goes neither with its file nor with its
project's directory: for the same inputs the head's scope is master's and its debt within master's; a first record is
a baseline; new inputs take a migration with its reason and evidence; over unchanged inputs a migration is admitted when every item it adds pairs with one it loses by the message (the position and the printed lines apart) and the debt does not grow, an accepted error the compiler now places or prints elsewhere; a record removed fails.
`tests/gate-scripts.sh`'s `records` section proves the rules over stand-in transcripts through the ratchet's verbs,
and `tests/lila-test-verb.sh`, `tests/reference-test-verb.sh` and `tests/mill-test-verb.sh` the verbs' own use of them
over stand-in teq binaries.

### lila

lila, lichess's server ([lichess-org/lila](https://github.com/lichess-org/lila)), is teq's public reference
project: forty sbt modules of Scala 3.8.4 with Play routers, macros (reactivemongo's handlers, play-json's
formats, macwire's wiring) and a test suite, all of it public, so that what teq compiles, tests and runs of it
anyone can repeat. `bench/lila/` holds its pin (`pin.json`: the upstream commit, the JDK, sbt, the overlay's
digests), the oracle's inventory (`tests.json`: the suites and cases `testFull` passes under scalac 3.8.4), the
generated inputs' digests (`generated.json`), the overlay sbt's `-addPluginSbtFile` adds to lila's build without
changing its repository (`overlay/plugins.sbt` and `overlay/LilaTeq.scala`, which declares the generators teq runs
in the place of sbt's: liplay's routers through its routes compiler under `teq interp` (`routes.scala`) and the
serialised translations (`i18n.scala`, the JDK's bytes)), and the script `lila.scala` with its verbs:
`checkout` (the pin into a blob-filtered clone, `conf/version.conf` written from the pin as lila's CI writes
it), `oracle` (`clean;compile;testFull;stage` under sbt, `--runs n`, `--record`), `export`, `compile`, `test`
and `run` (each with `--teq <binary>`). The pin, the inventory, the digests and the overlay move together:
a later part moves the pin by running `checkout`, `oracle --record`, `export`, then `compile`, `test` and `run`
with `--migrate <reason>` at the new commit. The export publishes the head's sbt-teq (`integrations/sbt`, both sbt
lines by `^publish`) under a SNAPSHOT named by the digest of what the plugin is built from into a Maven repository of
its own beside the line's directory, once for each digest, which the overlay's `plugins.sbt` adds as a resolver, so
that an export takes the plugin under test and nothing reaches `~/.ivy2/local` (`TEQ_PLUGIN_VERSION` names another,
published by its caller, by hand; the gate's lines and the setup program run without it, whatever a machine's login
shell names, so that a holder needs nothing the setup did not make). An export that fails names its cause: a plugin
in no repository sbt reads by its coordinates and the sbt it is for, another failure by the build tool's first error.

The gate's lines `lila-export`, `lila-compile`, `lila-test` and `lila-run` take lila's checkout (`--lila`) and
the binary under test, and are mandatory once named: a missing checkout, export, service (`LILA_MONGOD`,
`LILA_REDIS`), built asset manifest or suite fails the line, never a skip, and each is held to its project's
records ("The project records"). `lila-test` holds the run to the inventory: without `tests.json`, or with an empty
one, it fails at the start, and it runs `teq test --verbose`, whose runner names every test with its outcome
(`[success]`, `[skipped]`, `[fail]`), so that each suite of the inventory has to have run in its project with each of
its cases as the inventory has it, or be a departure `deviations.json` holds (`tests/lila-test-verb.sh`), the lines of
suites that ran at once on the runner's pool taken by their suites' names. The project lines run in every gate, unnamed
(the owner's ruling, 2026-10-16: every reference project in, the gate never weakened to make it faster), `--without`
leaving named ones out; a gate past thirty minutes is answered with machines, never with fewer lines. A project's
lines are a chain (export, compile,
test, run, one after another) dealt to a free remote machine that holds its checkout: its pool line carries
`holds=<project>,...` (or `GATE_LILA_MACHINE` names it, for every project), the fewest such machines that hold the
projects wanted taken, and more of them while the projects would end after the shards by the recorded times, each
reserved, no shard or comparison taking it, and a machine holding a project not yet dealt left to it across the
polls; its login shell gives the checkouts at the pins (`GATE_<PROJECT>_CHECKOUT`), lila's services (`LILA_MONGOD`,
`LILA_REDIS`, its ports 27017, 6379 and 9663 its own), the pins' JDK (`JAVA_HOME`) and their jars. A line with no free
machine holding its project fails as not run after `GATE_CLAIM_WAIT` seconds, saying whether none of the pool holds
it or those that do were busy; `--lila`, `--airstream` or `--laminar` naming a checkout runs that project's lines on
the reference machine instead.

The checkouts are made by `bench/gate-projects/setup.scala <dir> [--projects <p>,...]` under `teq interp` on the
machine that is to hold them: the tools (git, node, npm, sbt, the pins' JDK, and for lila node 24 or later and pnpm
12 or later, which it checks and does not install), lila's services (the environment's or the PATH's, else the
releases `bench/gate-projects/services.json` pins, fetched, held to their digests and installed under `<dir>`, each
started on its port and stopped), each project's `checkout` and `export` verbs into `<dir>/<project>`, lila's ui built
(`ui/build`, every script its manifest names there) and Chrome's WebGL2 context; every stage runs again on each run,
so that a product deleted since is made again, and `<dir>/ready` and `<dir>/gate-projects.env` (the variables the
machine's login shell sources) are written only when every stage passed. It names no machine.

### Airstream and Laminar

Airstream and Laminar ([raquo/Airstream](https://github.com/raquo/Airstream),
[raquo/Laminar](https://github.com/raquo/Laminar)) are the reference projects of the JavaScript target: sbt 1
builds of Scala.js libraries (an FRP library and the UI library over it: observables, DOM construction, facades)
whose ScalaTest suites run under jsdom. Each is pinned at its release `v18.0.0-M5`, the last before both moved to
Scala 3.9 and to an `ew` whose TASTy scalac 3.8.4 refuses (`pin.json` says so), Laminar over Airstream's released
jar of that version. `bench/airstream/` and `bench/laminar/` hold the pin (the upstream tag and commit, the JDK,
sbt, the Scala axis the gate takes, the root project's name and the projects teq builds, the npm fixture's digest
and jsdom's version, the overlay's digests; Airstream's build names no root project, so sbt names it after the
checkout's directory, which every verb refuses where that is not `airstream`), the npm fixture
(`package-lock.json`, jsdom, which the releases do not commit), the oracle's inventory (`tests.json`: every suite
with its project and its cases, each under its status as the framework reported it, `cases` for a pass,
`ignored`, `pending` and the others by their names, a case as many times as it ran), the overlay
(`overlay/plugins.sbt` and the project's AutoPlugin, sbt 1's meta-build, Scala 2.12) and the generators teq runs in
the place of sbt's: Airstream's `Compile` and `Test` source generators, its build's own `project/Generate*.scala`
under `teq interp` (`generators.scala`, with the generators' base and sbt's `/` beside them), and Laminar's DOM
definitions, which its build writes in `onLoad` with `project/DomDefsGenerator.scala` over Scala DOM Types
(`domdefs.scala`, the library's Scala 3 jar retrieved by the overlay into `target/domtypes`). The overlay stages
each script with the file it reads into the checkout's `target/teq-scripts` as the build loads, the command's
inputs, so that teq's record of the generator hashes them and an edit to either runs it again. Both generators write
checked-in files, which `compile` deletes first and holds to the pin's bytes after. The verbs are `bench/reference/scalajs.scala`'s, each project's script naming them:
`checkout` (the pin, the npm fixture installed by `npm ci`), `oracle` (`++3.8.4! clean test` under sbt, sbt 1's
`test` being the full run, with `bench/reference/oracle`'s listener, which records every event of the run as `teq
test --events` records teq's; `--runs n`, `--record`, never from a run that executed nothing or failed), `export`
(`++3.8.4! teqExportAll` with the overlay, failing on an export that compiled the build or recorded a generator
only sbt runs), `compile` and `test` (`teq test --events`), and Airstream's `generators` (the staged script's arity
bound edited, the generators run again; restored, the pin's bytes again). The inventories are compared by project
and suite (a suite of one name in two projects is two), case and status, as many times as each side reports a
case: a lost case, a status changed (an ignored case pending), a case run twice, a failed case, a case of another
project's suite of the name and a run that did not end as it should fail (`tests/reference-test-verb.sh`, which also
holds the sampled peak memory to the node a verb starts: the processes the script starts and those naming the
binary, each with its descendants).

The gate's lines `airstream-export`, `airstream-compile`, `airstream-test`, `laminar-export`, `laminar-compile`
and `laminar-test` take the checkouts (`--airstream`, `--laminar`) and the binary under test, are mandatory once
named, run as lila's, and are held to their records. teq does not compile either project yet: their tests use
`scala.compiletime.testing.typeCheckErrors` (scalatest's `assertTypeError`), Airstream's macros a `Type` of a type
constructor (`AnyKind`), and Laminar overloads methods on `scala.Array` and `js.Array`, which teq's std makes one
type, and Laminar's DOM definitions do not generate under `teq interp` (its interpreter's std has no
`PrintStream(OutputStream)`); the export lines pass and the others hold to their records.

### Tyrian and Indigo

Tyrian ([PurpleKingdomGames/tyrian](https://github.com/PurpleKingdomGames/tyrian), an Elm-style Scala.js
framework) and Indigo ([PurpleKingdomGames/indigo](https://github.com/PurpleKingdomGames/indigo), a game engine
over WebGL) are the public reference projects built by Mill (1.0.6), as lila is for sbt: Scala.js modules with
cats-effect, ZIO and fs2, source generators, macros run in teq's interpreter (Indigo's shaders, ultraviolet's) and
munit suites, the JVM's modules beside Tyrian's. `bench/tyrian/` and `bench/indigo/` hold each one's pin
(`pin.json`: the commit, the JDK, the Mill the build names, the build's directory, the oracle's steps, the
overlay's digests, the run, and for Indigo the bootstrap of its own Mill plugin, which its engine build imports
from a local repository: built from the commit's tree written to one path on every machine, since its class files
and TASTy name their sources' absolute paths, and each artifact held to the digest of its entries), the oracle's inventory (`tests.json`: each module's scalac warnings and its suites and
cases under the statuses its framework reported, as many times as each ran, under scalac 3.8.4), the generated inputs' digests (`generated.json`), the overlay
(`build.mill.patch`, Scala 3.8.4 and no fatal warnings for the oracle's build and teq's alike; `teq.mill.json`,
the generators teq runs in the place of Mill's, with the scripts they run beside it) and `run/`, the page's
script the run serves. One script drives both, `bench/mill-reference.scala <tyrian|indigo> <verb> <checkout>
<out>`: `checkout` (the pin, the overlay applied), `oracle` (the bootstrap, then the steps under Mill, the
inventory read from Mill's `out/`, `--runs n`, `--record`), `export` (mill-teq published from
`integrations/mill` under a version of the run's own and imported by `mill --import`, no edit of the build),
`compile`, `test` (the export's modules, whatever the inventory holds: each with a test configuration asked for
its suites by `teq test --list` and run by `teq test --verbose` where it has one, its modules, suites and cases
held to `tests.json` identity for identity both ways, each case under its framework's status as many times as it ran,
its departures to `deviations.json`, `tests/mill-test-verb.sh` its scenarios) and `run`
(the module the pin's `run` names built by `teq build`, served with its page by `bench/smoke.mjs`, which opens it
in headless Chrome over the DevTools protocol on a pipe and drives it: Tyrian's sandbox counter clicked, Indigo's
sandbox frame drawn and its scene switched; an exception, a console error, a failed request or the time running
out fails it). The pin, the inventory, the digests and the overlay move together, as lila's do.

Indigo's generated sources come from its plugin's own files under `teq interp` (the overlay's `generate.scala`,
the module's options copied from its `package.mill`), over stand-ins of the part of os-lib and of `java.awt` they
name (`os.scala`, `awt.scala`) and a font generator of its own (`fontgen.scala`: the plugin's layouts and source,
the TrueType file read in Scala, the advances `hdmx` records for the size, the sheet drawn and written as a PNG into
the module's assets, where the plugin writes it, declared among the generator's outputs); the four modules' sources
are Mill's to the byte, the sheet is drawn by another rasterizer than AWT's, so the checkout's committed sheets are
the build's products the pin check leaves to the compile verb, which holds them to `generated.json` and deletes the
sheets and the generated directories apart to see each written again. Its compile
stops at the engine's shaders: ultraviolet's `toGLSL` macro matches the trees dotty's typer leaves (an inline
call's `Inlined(call, bindings, expansion)`, a case class's `apply`, a mirror bound to a proxy), which teq's are
not (`docs/COMPATIBILITY.md`, "Macros, inline and derivation"), so `indigo-compile`, `indigo-test` and `indigo-run` fail until
teq's trees carry them. The driver kills teq's processes past `MILL_REFERENCE_MEMORY_GB` (default 16), failing the
verb: Indigo's whole compile at eight workers reached the container's 22 GB, at one worker 2.9 GB, so the gate runs
Indigo's lines at one typer worker (`TEQ_THREADS=1`).

The gate's lines `tyrian-export`, `tyrian-compile`, `tyrian-test`, `tyrian-run` and Indigo's four take the
checkouts (`--tyrian`, `--indigo`) and the binary under test, and are mandatory once named: a missing checkout,
export, browser or suite fails the line, never a skip; a run line first checks that Chrome (`CHROME` names
another) makes a WebGL2 context (`node bench/smoke.mjs webgl`), which headless Chrome does through SwiftShader.
They run as lila's, held to their records; the run lines hold the smoke's named assertions to `run.json`.

## The timing lines on a machine

The gate's `budget`, `runtime` and `sentinel` lines compare the head's plain release build with
master's on remote machines of their own, over the budgets of `bench/budgets.txt`, `bench/runtime-budgets.txt`
and the sentinel's `bench/reference.txt`; `bench/pairs.py` is the estimator (its docstring has the
statistic, the schedule and the verdicts). They measure once the suites on the shards and the project lines
have ended: the machines share one host, its memory bandwidth and its scheduler, which a container's processors do
not isolate, and a pair's band widens with the suites' load (the first sharded gate, beside seven shards at a
host load of 59 to 62, had a budget row inconclusive and a sentinel row over). Their claims, syncs, builds and the
head's binary come beside the suites, on one machine a line, the first free ones of the pool, claimed before the
shards are dealt the others (one at least left to them); when the suites have ended, the lines go to as few of those
as end the gate within thirty minutes by the time it has taken and the lines' recorded times, one machine and its
lines one after another when that fits; each pair on a machine is one build configuration (`tests/support/build-manifest.sh`), the head's binary the builder's where its
manifest is master's build, else built there. The runtime line runs the node
and java `bench/runtime-machine.txt` records, by their paths, whatever a machine's PATH leads to; and
`tests/asymptotics.sh`, whose bounds are met on node 22 (the GitHub workflow's), runs the node `NODE` names, so
that a machine names its node 22 there and each suite that needs one node names it.

## Property-based tests

`proptests/` is a crate on Hegel, apart from the compiler (whose one dependency is corosensei); its README has
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
version: the one it replaces moves under `shipped/<n>/` (the current launcher counts as one above the highest
there), so that applications committing the old one get the new one at their next export; the files carry
no version of their own, the plugin tells an unedited one by its text.
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
file. What landed since the last release stands first as `## Unreleased`, which the release renames to its
version and date; `bench/changelog.sh` takes it there alone and reads no version's notes from it. The section is committed before the version bump, which `bench/release.sh` refuses without it
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
   coordinates and cached by `tests/support/jars.sh`; zig's cache is cached; the toolchain is the image's; teq's
   crates, corosensei and the few it builds on, are fetched from crates.io at each build, too few for a registry
   cache to hold. The ship's `no-threads` step checks the build without the `threads` feature.
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
