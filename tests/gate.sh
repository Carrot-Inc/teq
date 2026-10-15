#!/bin/bash
# The landing gate: tests/gate.sh <worktree> [--master <binary>] [--master-rev <revision>] [--app <build.json>]
#   [--export-manifest <file>] [--lila <checkout>] [--airstream <checkout>] [--laminar <checkout>] [--tyrian <checkout>]
#   [--indigo <checkout>] [--only <name,...>] [--with <name,...>] [--record]
#
# <worktree> holds the landing: a branch rebased onto master, committed (the gate refuses a tree with changes outside
# its head, since the machines get the tree and every binary names the head). The gate takes thirty minutes at most
# from its first command to its last, the builds, claims, transfers and collection included (the owner's rule): its
# lines run at once across the remote machines (REMOTE_AGENT, the directory of the remote machines' scripts) and the
# reference machine it runs on, and its last line is that total, flagged when over thirty minutes. What is timed runs
# on the remote machines, and nothing waits for the reference machine to be quiet.
#
# The head's binary on the machines is built once, by the machine of the first shard (the builder; build_teq of
# tests/support/build-manifest.sh: stamped with the head's revision, described in its manifest with the binary's
# digest and what it needs of a machine), and shipped from there to every other machine of the gate, mirrored as the
# project teq-head: each takes it once it has the builder's digest, the manifest's revision and source and runs there
# as a plain build of the head (`received`), and builds it itself where it does not, which its build row says. With
# no shard, the comparison's and the projects' machines build it themselves.
#
# The correctness suites (the list `machine`, every suite of tests/all.sh but those its `excluded:` lines name and
# the comparison's and the reference machine's lines; tests/gate-coverage.sh checks it): each other free machine is
# claimed for one shard, the tree synced there, the shard's suites run one after another as tests/all.sh runs them
# (tests/gate-shard.sh), the machine released as soon as its shard ends, and at the end or on a failure whatever is
# still held. The suites are dealt out to the machines by their times in tests/gate-times.txt, longest first, so the
# shards end together; --record rewrites those times from this run, and only from a run whose every line passed or
# whose only lines not passed are timing lines inconclusive, none over (their durations not their verdicts'). The
# gate polls the machines and retries its claims every GATE_POLL_SECONDS (default 30; the comparison's polls at half
# of it).
# split-determinism, when the tree has it, compares with master's binary (REF), built on its machine from master's
# sources, the revision --master names, unless --export-manifest is given: the landing then changes the output, as
# the export line attests through the manifest, and the suite's own runs are compared with each other alone.
#
# The timing lines: compare the head with master on a machine of their own (tests/gate-compare.sh, bench/pairs.py;
# docs/DEVELOPING.md, "The timing lines on a machine"): budget, the type phase of every program of the compile
# budget; runtime, the generated programs' wall and steady-state time; and sentinel, the rows of
# bench/reference.txt against its reference revision under their cumulative ceilings. They measure once the suites on
# the shards and the project lines have ended, since those machines share the comparison's host, its memory bandwidth
# and its scheduler, and a pair's band widens with their load; the comparison's claim, syncs, builds and the head's
# binary come beside the suites, on as many machines, up to one a line, as leave the shards' end where it is by the
# recorded times (`prepare`). When the suites have ended, the lines go to as few of those as end the gate within
# thirty minutes by the time it has taken and the lines' recorded times, one machine and its lines one after another
# when that fits (`deal_lines`); the shards take the other free machines but those the projects hold. Every side is a
# plain release build stamped with its revision: master's (the revision --master-rev names, by default the head's
# merge base with master) and the reference's are kept on each machine under ~/teq-bin by the hash of their sources
# and the toolchain, built there on first use; the head's is the builder's, or built there when no shard ran or its
# manifest is not master's build. With every machine busy the claim waits at most GATE_CLAIM_WAIT seconds (default
# 1200), after which the lines read "not run: claim timeout". A machine is released when its lines end, and on any
# failure.
#
# The project lines, over the public reference projects, run only when --only or --with names them (docs/DEVELOPING.md,
# "lila", "Airstream and Laminar"): four over lila, lila-export (sbt teqExportAll with bench/lila/overlay, JAVA_HOME's
# JDK the pin's), lila-compile (teq compile of every project and the generated inputs' digests against
# bench/lila/generated.json), lila-test (the suites and cases against bench/lila/tests.json) and lila-run (the server
# over MongoDB and Redis, LILA_MONGOD and LILA_REDIS, its home page and an asset); three each over Airstream and
# Laminar, airstream-export and laminar-export (sbt ++3.8.4! teqExportAll with the project's overlay, JAVA_HOME's JDK
# the pin's; an export that compiles the build or records a generator only sbt runs fails), airstream-compile and
# laminar-compile (teq compile of the pin's projects, the generators' outputs deleted first and written again with
# the pin's bytes) and airstream-test and laminar-test (teq test --verbose, the suites and cases by outcome against
# the project's tests.json); four each over Tyrian and Indigo, the public reference projects built by Mill,
# tyrian-export and indigo-export (mill-teq through `mill --import`, Indigo's plugin published first), tyrian-compile
# and indigo-compile (every module and the generated inputs' digests), tyrian-test and indigo-test (the cases against
# tests.json) and tyrian-run and indigo-run (the sandbox built by teq and checked in headless Chrome by
# bench/smoke.mjs, after the browser is checked to make a WebGL2 context, `node bench/smoke.mjs webgl`), each
# bench/mill-reference.scala's verb for the project; each bench/<project>/<project>.scala's verb under the head's binary otherwise, each checkout's
# lines one after another (export, compile, test, run: compile and test stop the project's daemon and delete its
# products). They run on the machine that holds the checkouts, its pool line carrying `holds=<project>,...`, or the
# one GATE_LILA_MACHINE names, which no shard and no comparison takes while a project line is selected: its login
# shell gives the checkouts at the pins (GATE_LILA_CHECKOUT, GATE_AIRSTREAM_CHECKOUT, GATE_LAMINAR_CHECKOUT,
# GATE_TYRIAN_CHECKOUT, GATE_INDIGO_CHECKOUT, their npm
# fixtures installed), lila's services (LILA_MONGOD, LILA_REDIS), the pins' JDK (JAVA_HOME) and their jars in the
# coursier cache, and its lines take lila's ports (27017, 6379, 9663). With --lila, --airstream, --laminar, --tyrian or --indigo naming a
# checkout here, that project's lines run on the reference machine instead. A line without its checkout, an export, a
# service or a suite fails, never skips, and so does a line with no machine to run on.
#
# Meanwhile on the reference machine, where the gate runs, over its release binary of the head (built by the gate when
# missing or not the head's, a row of its own): tests/size.scala (its minified column is measured with its esbuild,
# which the remote machines do not have) and the instruction comparison against master's binary (bench/instr-compare.sh,
# five runs on the five programs depth_1, gview_1, core-only, realistic-api and realistic-frontend, each within 0.5%,
# and on derive30 and schema30 recorded apart; the programs generated once before both, both binaries plain builds, the
# gate refuses a head's or master's whose `--version` ends in pgo or instrumented), whose counts of instructions need no
# quiet; the resident sessions' memory over edits against the budgets this machine recorded (tests/watch-memory.sh as
# the line memory; a crossed budget on another machine says nothing, the footprints being the allocator's and the
# kernel's); and with --app, which names the application's export build file, the export's identity against master's
# saved build (bench/app/export-identity.sh), the API check against master's diagnostics (bench/app/api-check.sh,
# app-api) and the check of the API's test configuration over each binary's own build of the main lists' products,
# against master's the same way (api-check.sh --test, app-api-test), all reading the application's checkout and the jars
# its sbt build resolved into the reference machine's coursier cache (the lists bench/app/app-lists.sh writes: the API's
# module list and class path the files APP_MODULES and APP_CLASSPATH name, its flags APP_FLAGS, the test configuration's
# APP_TEST_MODULES, APP_TEST_CLASSPATH and APP_TEST_FLAGS, all kept outside the repository); with --export-manifest as
# well, the export's differences against the manifest of permitted ones (bench/app/export-accept.sh), after the identity
# line, which still fails on any difference; and with --app, the application's modules built one against another's
# products and compared with the whole build, the Scala.js check chain and the JVM API chain (bench/app/chain.sh), which
# fails on a build's failure or a mismatch the list CHAIN_KNOWN names does not list (none when it is unset, which the
# line's result says); and with --app and APP_CYPRESS, the application's end-to-end suite run by that script over the
# binary's own build of the application (its services, its API, its served bundle, the ports 8080 and 9500), which fails
# on a failing spec (app-cypress; skipped without the script). One line runs only when --only or --with names it, at a
# release and nightly: app-scalac, scalac 3.8.4 as the oracle of teq's diagnostics on the API's main and test lists
# (tests/scalac-oracle.sh, then bench/app/api-scalac-diff.sh with the lists' scalacOptions APP_SCALAC_OPTIONS and
# APP_TEST_SCALAC_OPTIONS, both into the attempt's directory), which fails on a difference the list SCALAC_KNOWN does
# not list or a line of it that names no difference (no rows when it is unset, which the line's result says). These
# lines run at once, the export's acceptance after its identity, and then the end-to-end suite alone, the memory line
# alone, its growth rows depending on the machine's load (tests/watch-memory.sh), and the asymptotic probes alone
# (tests/asymptotics.sh as the line asymptotics, under the node NODE names there, node 22): their growth ratios at 10^6
# elements are the load's too, a shard's machine at a host load of 40 to 60 having taken the map's lookups past their
# bound on node 22, which a quiet machine passes. The paths of the lists and of CHAIN_KNOWN and SCALAC_KNOWN are read
# from the caller's directory. --only limits the gate to the named lines, and --with adds the named ones to the gate's
# own (app-scalac's or the project lines, which run only when named).
#
# Each run's logs go to a new attempt directory, out/gate/<head>/<attempt>/: a suite's <name>.log, each machine's
# build log, manifest, status and shard output, and under compare-<machine>/ and projects/ those machines' logs, raw
# runs and manifests. A line per suite is printed as it ends, and one table at the end: every line with where it
# ran, its seconds and its exit, the seconds each claim waited for its machine, the comparison's stages, the halves'
# times and the total. The gate exits non-zero when any line fails or is inconclusive. A suite the tree does not have
# (workers and split-determinism before the parallel typer lands) is a skip, not a failure.
caller=$PWD
gate_started=$(date +%s)
# The interval of the polls of the machines and of the claims' retries, the comparison's polls half of it.
poll=${GATE_POLL_SECONDS:-30}
cpoll=$(( ${GATE_POLL_SECONDS:-30} / 2 > 0 ? ${GATE_POLL_SECONDS:-30} / 2 : 1 ))
cd "$(dirname "$0")/.."
root=$PWD
ra=${REMOTE_AGENT:-}
usage() {
  echo "usage: tests/gate.sh <worktree> [--master <binary>] [--master-rev <revision>] [--app <build.json>] [--export-manifest <file>] [--lila <checkout>] [--airstream <checkout>] [--laminar <checkout>] [--tyrian <checkout>] [--indigo <checkout>] [--only <name,...>] [--with <name,...>] [--record]" >&2
  exit 2
}
absolute() { case $1 in /*) echo "$1" ;; *) echo "$caller/$1" ;; esac; }
wt=
master=
master_rev=
app=
lila=
airstream=
laminar=
tyrian=
indigo=
manifest=
only=
with=
record=0
while [ $# -gt 0 ]; do
  case $1 in
    --master) [ -n "$2" ] || usage; master=$(absolute "$2"); shift 2 ;;
    --master-rev) [ -n "$2" ] || usage; master_rev=$2; shift 2 ;;
    --pgo) echo "gate: --pgo is gone: the budget line compares plain builds of the head and master on a remote machine (docs/DEVELOPING.md, \"The timing lines on a machine\")" >&2; exit 2 ;;
    --app) [ -n "$2" ] || usage; app=$(absolute "$2"); shift 2 ;;
    --export-manifest) [ -n "$2" ] || usage; manifest=$(absolute "$2"); shift 2 ;;
    --lila) [ -n "$2" ] || usage; lila=$(absolute "$2"); shift 2 ;;
    --airstream) [ -n "$2" ] || usage; airstream=$(absolute "$2"); shift 2 ;;
    --laminar) [ -n "$2" ] || usage; laminar=$(absolute "$2"); shift 2 ;;
    --tyrian) [ -n "$2" ] || usage; tyrian=$(absolute "$2"); shift 2 ;;
    --indigo) [ -n "$2" ] || usage; indigo=$(absolute "$2"); shift 2 ;;
    --only) [ -n "$2" ] || usage; only=" ${2//,/ } "; shift 2 ;;
    --with) [ -n "$2" ] || usage; with=" ${2//,/ } "; shift 2 ;;
    --record) record=1; shift ;;
    -*) usage ;;
    *) [ -z "$wt" ] || usage; wt=$(absolute "$1"); shift ;;
  esac
done
[ -n "$wt" ] || usage
# The application's lists, kept outside the repository, named from the caller's directory.
for v in APP_MODULES APP_CLASSPATH APP_SCALAC_OPTIONS APP_TEST_MODULES APP_TEST_CLASSPATH APP_TEST_SCALAC_OPTIONS CHAIN_KNOWN SCALAC_KNOWN APP_CYPRESS; do
  [ -n "${!v}" ] && export "$v=$(absolute "${!v}")"
done
[ -z "$manifest" ] || [ -f "$manifest" ] || { echo "gate: no manifest $manifest" >&2; exit 2; }

# The machines' suites, as tests/all.sh names and runs them; a command's first path is the script the tree must have
# (the repository's launcher, ./teq, aside: a suite written in Scala is the path it interprets).
machine=(
  "cases ./tests/run.sh"
  "errors ./tests/run_errors.sh"
  "parser ./tests/parser.sh"
  "interop ./tests/run_interop.sh"
  "dce ./tests/dce.sh"
  "split ./tests/split.sh"
  "split-watch ./tests/split-watch.sh"
  "check-watch ./tests/check-watch.sh"
  "jvm-watch ./tests/jvm-watch.sh"
  "lsp ./tests/lsp.sh"
  "task ./tests/task.sh"
  "runner-classes ./tests/runner-classes.sh"
  "watch-memory ./tests/watch-memory.sh"
  "tasty ./tests/tasty.sh"
  "modules ./tests/modules.sh"
  "tasty-exec ./teq interp tests/tasty-exec.scala"
  "analysis-bytes ./tests/analysis-bytes.sh"
  "classfile ./teq interp tests/classfile.scala"
  "classpath ./tests/classpath.sh"
  "stdlib ./tests/run_stdlib.sh"
  "app ./tests/app.sh"
  "lila-test-verb ./tests/lila-test-verb.sh"
  "reference-test-verb ./tests/reference-test-verb.sh"
  "mill-test-verb ./tests/mill-test-verb.sh"
  "std-entered ./tests/std-entered.sh"
  "jvm ./tests/run_jvm.sh"
  "jvm-21 ./tests/run_jvm.sh --java-output-version=21"
  "interp ./tests/run_interp.sh"
  "fold ./tests/fold.sh"
  "planted ./tests/planted.sh"
  "workers ./tests/workers.sh"
  "split-determinism ./tests/split-determinism.sh"
  "scala3-run tests/scala3/run.sh \${JOBS:+--jobs \$JOBS}"
  "scala3-interp tests/scala3/run.sh --target interp \${JOBS:+--jobs \$JOBS}"
  "scala3-macros tests/scala3/run.sh --suite run-macros \${JOBS:+--jobs \$JOBS}"
  "scala3-macros-interp tests/scala3/run.sh --suite run-macros --target interp \${JOBS:+--jobs \$JOBS}"
  "scala3-typing python3 tests/scala3-typing/harness.py \${JOBS:+--jobs \$JOBS}"
  "cargo-test CARGO_TARGET_DIR=target/gate-test cargo test --release"
  "rust-warnings ./tests/rust-warnings.sh"
  "gate-coverage ./tests/gate-coverage.sh"
  "gate-scripts ./tests/gate-scripts.sh"
)
# excluded: prop  outside the gate, as tests/all.sh says (its crates are fetched apart); the GitHub gate runs it
compare="budget runtime sentinel"
local_lines="size instructions instructions-derive memory asymptotics app-export app-export-accept app-api app-api-test app-chain app-cypress app-scalac"
project_lines="lila-export lila-compile lila-test lila-run airstream-export airstream-compile airstream-test laminar-export laminar-compile laminar-test tyrian-export tyrian-compile tyrian-test tyrian-run indigo-export indigo-compile indigo-test indigo-run"
# The lines run only when --only or --with names them: the scalac oracle, at a release and nightly; the reference
# projects', until their time and memory are measured within the gate's budget (docs/DEVELOPING.md, "lila",
# "Airstream and Laminar", "Tyrian and Indigo").
optional="app-scalac lila-export lila-compile lila-test lila-run airstream-export airstream-compile airstream-test laminar-export laminar-compile laminar-test tyrian-export tyrian-compile tyrian-test tyrian-run indigo-export indigo-compile indigo-test indigo-run"
names=" $compare $local_lines $project_lines "
for entry in "${machine[@]}"; do names="$names${entry%% *} "; done
[ -z "$only" ] || [ -z "$with" ] || { echo "gate: --only and --with exclude each other" >&2; exit 2; }
for name in $only $with; do
  [[ "$names" == *" $name "* ]] || { echo "gate: no line named $name (the lines:$names)" >&2; exit 2; }
done
selected() { if [ -z "$only" ]; then [[ " $optional " != *" $1 "* || "$with" == *" $1 "* ]]; else [[ "$only" == *" $1 "* ]]; fi; }
# A suite that skips for want of something the machine or the reference machine should hold says so in one of these
# lines and exits 0; the gate fails it: a jar not in the coursier cache ("skip <x>: not in the coursier cache",
# "<suite>: skipped, ...", "counted as passed, a jar of theirs not in the coursier cache", "skip the jvm session: ..."
# and the like), a tool ("skip ...: no javap", "esbuild not found", "skip: scala-cli not found"), the scala3
# checkout, or a check a tool's absence leaves undone ("note: no wine here, teq.cmd not run", "note: no shellcheck
# here, ...", "note: no npm here, ...", runner-classes' "not checked" without a javac 17). What is skipped by design
# is admitted: the tests whose expectations come from Scala.js on the JVM and in the interpreter ("skipped
# (expectations from Scala.js)"), another system's programs and, in the interpreter, another std's (tests/run_jvm.sh,
# tests/run_interp.sh: "skipped (another system's)", "skipped (another std's)"), a memory session whose budget is
# another platform's alone (tests/watch-memory.sh: "skipped (a budget of one platform)"), the JVM platform's programs
# in the JavaScript run (tests/run.sh: "skip <case>: the JVM's platform"), and in the split watch a case of another
# std or of the JVM alone (tests/split-watch.sh: "skip <case>: its std is <modes>", "skip <case>: the JVM's alone").
skips='^skip|skipped, |counted as passed|not in the coursier cache|esbuild not found|note: no [^ ]+ here,|not checked'
allowed="^skipped \\((expectations from Scala\\.js|a budget of one platform|another system's|another std's)\\)|^skip [^ ]+: (the JVM's platform|its std is [^ ].*|the JVM's alone)\$"
# lacking <log>: the first line of the log that says it skipped for want of something, nothing when none does.
lacking() { grep -E "$skips" "$1" 2> /dev/null | grep -v -E "$allowed" | head -1; }
script_of() {
  local w
  for w in $1; do
    case $w in *=* | ./teq) ;; */*) echo "$w"; return ;; esac
  done
}
now() { date +%s; }
clock() { date +%H:%M:%S; }
# kind <version>: the word after the hash in a `teq --version`, pgo or instrumented, nothing for a plain build.
kind() { echo "$1" | awk '{print $4}'; }
# sources <revision>: the hash of what a binary is built from, the key of its artifact on a machine.
sources() {
  git -C "$wt" ls-tree "$1" -- src std runtime build.rs Cargo.toml Cargo.lock .cargo rust-toolchain.toml | git hash-object --stdin | cut -c1-12
}

# The landing.
wt=$(git -C "$wt" rev-parse --show-toplevel 2> /dev/null) || { echo "gate: $wt is not a git working tree" >&2; exit 2; }
head=$(git -C "$wt" rev-parse --short HEAD)
full=$(git -C "$wt" rev-parse HEAD)
dirty=$(git -C "$wt" status --porcelain --untracked-files=all | grep -v -E '^\?\? (\.metals|\.bsp|\.idea|\.vscode)/|\.DS_Store$')
if [ -n "$dirty" ]; then
  echo "gate: $wt has changes outside its head; commit or remove them:" >&2
  echo "$dirty" | head -10 >&2
  exit 2
fi
bin=$wt/target/release/teq
local_selected=0
for name in $local_lines; do selected $name && local_selected=1; done
remote_selected=0
for name in $compare; do selected $name && remote_selected=1; done
for entry in "${machine[@]}"; do selected "${entry%% *}" && remote_selected=1; done
# The projects whose lines run on the machine that holds their checkouts (projects), and those whose checkout
# --lila, --airstream, --laminar, --tyrian or --indigo names here, whose lines run on the reference machine (here_projects, here_lines).
projects=
here_projects=
here_lines=
for p in lila airstream laminar tyrian indigo; do
  wanted=0
  for name in $project_lines; do
    [[ $name == $p-* ]] && selected $name || continue
    wanted=1
    [ -z "${!p}" ] || here_lines="$here_lines $name"
  done
  [ $wanted = 1 ] || continue
  if [ -n "${!p}" ]; then here_projects="$here_projects $p"; else projects="$projects $p"; fi
done
[ -z "$projects" ] || remote_selected=1
[ -z "$here_projects" ] || local_selected=1
if [ $remote_selected = 1 ] && [ ! -x "$ra/run.sh" ]; then
  echo "gate: set REMOTE_AGENT to the directory of the remote machines' scripts (claim.sh, run.sh, sync.sh, get.sh, release.sh, pool.sh)" >&2
  exit 2
fi
# The reference machine's binary of the head: built by the gate's local half when it is missing or not the head's.
local_build=0
version=
if [ $local_selected = 1 ]; then
  version=$("$bin" --version 2> /dev/null)
  hash=$(echo "$version" | awk '{print $3}')
  [ -n "$hash" ] && [[ "$full" == "$hash"* ]] || local_build=1
fi
mfull=
if [ -n "$master" ]; then
  mversion=$("$master" --version 2> /dev/null) || { echo "gate: $master does not run" >&2; exit 2; }
  mhash=$(echo "$mversion" | awk '{print $3}')
  if selected instructions || selected instructions-derive; then
    [ -z "$(kind "$mversion")" ] || { echo "gate: $master is $mversion, not a plain build; the instruction comparison is of two plain builds (cargo build --release)" >&2; exit 2; }
  fi
  mfull=$(git -C "$wt" rev-parse --verify -q "$mhash^{commit}") || { echo "gate: $master names $mhash, which $wt does not know" >&2; exit 2; }
  git -C "$wt" merge-base --is-ancestor "$mfull" "$full" || echo "gate: warning: master's binary ($mhash) is not an ancestor of the head"
fi
compare_selected=0
for name in $compare; do selected $name && compare_selected=1; done
crev=
refrev=
if [ $compare_selected = 1 ]; then
  # The timing lines' master and reference, resolved once.
  if [ -n "$master_rev" ]; then
    crev=$(git -C "$wt" rev-parse --verify -q "$master_rev^{commit}") || { echo "gate: --master-rev $master_rev is no commit of $wt" >&2; exit 2; }
  else
    crev=$(git -C "$wt" merge-base HEAD master) || { echo "gate: $wt has no merge base with master; name the revision with --master-rev" >&2; exit 2; }
  fi
  crev=$(git -C "$wt" rev-parse --short "$crev")
  [ -z "$mfull" ] || [ "$(git -C "$wt" rev-parse --short "$mfull")" = "$crev" ] || echo "gate: warning: --master's binary is $mhash and the timing lines' master $crev"
  refrev=$(awk '$1 == "reference" || $1 == "advance" {r = $2} END {print r}' "$wt/bench/reference.txt" 2> /dev/null)
  refrev=$(git -C "$wt" rev-parse --short --verify -q "$refrev^{commit}") || { echo "gate: bench/reference.txt names no revision $wt knows" >&2; exit 2; }
fi
mkdir -p "$root/out/gate/$head"
for attempt in $(seq 1 1000); do mkdir "$root/out/gate/$head/$attempt" 2> /dev/null && break; done
out=$root/out/gate/$head/$attempt
[ -d "$out" ] || { echo "gate: no attempt directory left under out/gate/$head" >&2; exit 2; }
# The gate's own scratch, under the attempt's directory rather than the machine's /tmp, which the suites here fill
# (the memory line's sessions alone some 107,000 files) and which may be a tmpfs of few inodes; removed at the end.
tmp=$out/tmp
mkdir -p "$tmp"
results=$out/results
: > "$results"
# result <name> <where> <seconds> <exit or skip> <line>
result() { printf '%s\t%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$4" "$5" >> "${6:-$results}"; }
echo "gate: $head $(git -C "$wt" log -1 --format=%s "$full" | cut -c1-80)${master:+, master $mhash}${crev:+, timing lines against master $crev and reference $refrev}; logs in ${out#$root/}"

# A claim is recorded in $tmp/claim.<machine> (a shard's), $tmp/cclaim.<machine> (the comparison's) or
# $tmp/pclaim.<machine> (the projects') as soon as claim.sh returns its handle, and its release in
# $tmp/released.<handle>; the exit releases every recorded claim not yet released, and a release that fails, there or
# earlier, fails the gate with the handle named.
claimers=()
local_pid=
compare_pid=
projects_pid=
kill_tree() {
  local c
  for c in $(pgrep -P "$1"); do kill_tree "$c"; done
  kill "$1" 2> /dev/null
}
mark() { echo "${1//\//_}"; }
# release <handle>: the machine released, and the release recorded.
release() {
  "$ra/release.sh" "$1" > "$tmp/release.$(mark "$1")" 2>&1 && touch "$tmp/released.$(mark "$1")"
}
finish() {
  local code=$? f h p i
  [ -z "$local_pid" ] || ! kill -0 "$local_pid" 2> /dev/null || kill_tree "$local_pid"
  # No claim starts once $tmp/stop is there, and a claim in flight ends first (claim.sh bounds itself): one stopped
  # halfway could leave a machine held under a handle the gate never learns. The comparison and the projects
  # announce a claim before they look for the stop, so a claim either sees the stop or is seen here.
  touch "$tmp/stop"
  for p in "$compare_pid" "$projects_pid"; do
    [ -n "$p" ] && kill -0 "$p" 2> /dev/null || continue
    for i in $(seq 1 200); do [ -e "$tmp/compare.claiming" ] || [ -e "$tmp/projects.claiming" ] || break; sleep 3; done
    kill_tree "$p"
  done
  [ ${#claimers[@]} = 0 ] || echo "gate: waiting for ${#claimers[@]} claims in flight before releasing" >&2
  for p in "${claimers[@]}"; do wait "$p" 2> /dev/null; done
  for f in "$tmp"/claim.* "$tmp"/cclaim.* "$tmp"/pclaim.*; do
    [ -e "$f" ] || continue
    read -r h _ < "$f"
    [ -e "$tmp/released.$(mark "$h")" ] || release "$h" || { echo "gate: FAIL: could not release $h: $(tail -1 "$tmp/release.$(mark "$h")")" >&2; code=1; }
  done
  rm -rf "$tmp"
  exit $code
}
trap finish EXIT
trap 'exit 130' INT TERM

# The reference machine's half, in the background: its results go to their own file.
# local_line <name>: one of its lines, when selected.
local_line() {
  local file=$out/local.results s code side
  selected "$1" || return 0
  s=$(now)
  case $1 in
    size)
      # the minified column of the size budgets is esbuild's output, measured with this machine's esbuild
      (cd "$wt" && TEQ=$bin timeout 600 ./teq interp tests/size.scala) > "$out/size.log" 2>&1
      code=$?
      [ $code != 0 ] || [ -z "$(lacking "$out/size.log")" ] || { echo "gate: a skip: $(lacking "$out/size.log")" >> "$out/size.log"; code=1; }
      result size local $(($(now) - s)) $code "$(tail -1 "$out/size.log")" "$file"
      ;;
    instructions | instructions-derive)
      if [ -z "$master" ]; then
        result $1 local 0 skip "no --master" "$file"
      elif [ $1 = instructions ]; then
        INSTR_COUNTS=$out/instructions.counts timeout 1500 "$root/bench/instr-compare.sh" "$master" "$bin" 5 depth_1 gview_1 core-only realistic-api realistic-frontend > "$out/instructions.log" 2>&1
        code=$?
        result instructions local $(($(now) - s)) $code "$(tail -1 "$out/instructions.log")" "$file"
      else
        INSTR_TOLERANCE=none INSTR_COUNTS=$out/instructions-derive.counts timeout 900 "$root/bench/instr-compare.sh" "$master" "$bin" 5 derive30 schema30 > "$out/instructions-derive.log" 2>&1
        code=$?
        result instructions-derive local $(($(now) - s)) $code "$(tail -1 "$out/instructions-derive.log")" "$file"
      fi
      ;;
    asymptotics)
      (cd "$wt" && TEQ=$bin timeout 900 ./tests/asymptotics.sh) > "$out/asymptotics.log" 2>&1
      code=$?
      [ $code != 0 ] || [ -z "$(lacking "$out/asymptotics.log")" ] || { echo "gate: a skip: $(lacking "$out/asymptotics.log")" >> "$out/asymptotics.log"; code=1; }
      result asymptotics local $(($(now) - s)) $code "$(tail -1 "$out/asymptotics.log") (alone, after the memory line)" "$file"
      ;;
    memory)
      (cd "$wt" && TEQ=$bin timeout 1500 ./tests/watch-memory.sh) > "$out/memory.log" 2>&1
      code=$?
      [ $code != 0 ] || [ -z "$(lacking "$out/memory.log")" ] || { echo "gate: a skip: $(lacking "$out/memory.log")" >> "$out/memory.log"; code=1; }
      result memory local $(($(now) - s)) $code "$(tail -1 "$out/memory.log") (alone, after the other lines here)" "$file"
      ;;
    app-export)
      if [ -z "$app" ] || [ -z "$master" ]; then
        result app-export local 0 skip "needs --app and --master" "$file"
      else
        timeout 900 "$root/bench/app/export-identity.sh" "$app" "$bin" "$master" "$out/app-export" > "$out/app-export.log" 2>&1
        code=$?
        result app-export local $(($(now) - s)) $code "$(tail -1 "$out/app-export.log")" "$file"
      fi
      ;;
    app-export-accept)
      if [ -z "$manifest" ] && [ -n "$app" ] && [ -n "$master" ] && grep -q "identical to master's build" "$out/app-export.log" 2>/dev/null; then
        result app-export-accept local 0 0 "no manifest: the export is identical to master's" "$file"
      elif [ -z "$app" ] || [ -z "$master" ] || [ -z "$manifest" ]; then
        result app-export-accept local 0 skip "needs --app, --master and --export-manifest" "$file"
      else
        timeout 900 "$root/bench/app/export-accept.sh" "$manifest" "$app" "$bin" "$master" "$out/app-export-accept" "$wt" > "$out/app-export-accept.log" 2>&1
        code=$?
        result app-export-accept local $(($(now) - s)) $code "$(tail -1 "$out/app-export-accept.log")" "$file"
      fi
      ;;
    app-api | app-api-test)
      if [ -z "$app" ] || [ -z "$master" ]; then
        result $1 local 0 skip "needs --app and --master" "$file"
      else
        APP_ROOT=$(app_root) timeout 600 "$root/bench/app/api-check.sh" $([ $1 = app-api ] || echo --test) "$bin" "$master" > "$out/$1.log" 2>&1
        code=$?
        result $1 local $(($(now) - s)) $code "$(tail -1 "$out/$1.log")" "$file"
      fi
      ;;
    app-chain)
      if [ -z "$app" ]; then
        result app-chain local 0 skip "needs --app" "$file"
      else
        code=0
        for side in frontend api; do
          timeout 600 "$root/bench/app/chain.sh" "$app" "$bin" "$out/app-chain/$side" $side > "$out/app-chain-$side.log" 2>&1 || code=1
        done
        result app-chain local $(($(now) - s)) $code "$(chain_counts "$out/app-chain-frontend.log" "$out/app-chain-api.log")$([ -n "$CHAIN_KNOWN" ] || echo "; no CHAIN_KNOWN, no known rows")" "$file"
      fi
      ;;
    app-cypress)
      if [ -z "$app" ] || [ -z "${APP_CYPRESS:-}" ]; then
        result app-cypress local 0 skip "needs --app and APP_CYPRESS, the application's end-to-end suite script" "$file"
      else
        timeout 7200 "$APP_CYPRESS" "$bin" "$out/app-cypress" > "$out/app-cypress.log" 2>&1
        code=$?
        result app-cypress local $(($(now) - s)) $code "$(tail -1 "$out/app-cypress.log")" "$file"
      fi
      ;;
    app-scalac)
      if [ -z "$app" ]; then
        result app-scalac local 0 skip "needs --app" "$file"
      else
        { (cd "$wt" && TEQ=$bin timeout 900 ./tests/scalac-oracle.sh "$out/scalac-oracle") && \
          APP_ROOT=$(app_root) timeout 3600 "$root/bench/app/api-scalac-diff.sh" "$bin" "$out/app-scalac"; } > "$out/app-scalac.log" 2>&1
        code=$?
        [ $code != 0 ] || [ -z "$(lacking "$out/app-scalac.log")" ] || { echo "gate: a skip: $(lacking "$out/app-scalac.log")" >> "$out/app-scalac.log"; code=1; }
        result app-scalac local $(($(now) - s)) $code "$(tail -1 "$out/app-scalac.log")$([ -n "$SCALAC_KNOWN" ] || echo "; no SCALAC_KNOWN, no known rows")" "$file"
      fi
      ;;
  esac
}
# project_script <project>: the script whose verbs a project's lines run and the arguments before the verb:
# bench/<project>/<project>.scala, or bench/mill-reference.scala with the project's name for Tyrian and Indigo.
project_script() {
  case $1 in
    tyrian|indigo) echo "bench/mill-reference.scala -- $1" ;;
    *) echo "bench/$1/$1.scala --" ;;
  esac
}
# project_preflight <project> <verb>: what a verb needs before it runs: Tyrian's and Indigo's run a browser that
# makes a WebGL2 context, which headless Chrome does through SwiftShader.
project_preflight() {
  case $1-$2 in
    tyrian-run|indigo-run) echo "timeout 120 node bench/smoke.mjs webgl && " ;;
  esac
}
# project_env <project>: the assignments a project's verbs run under: Indigo's at one typer worker, whose whole
# compile stays under 3 GB where the automatic count's (eight) reached 22.2 GB, past a driver's 16 GB allowance (the
# parallel typer's growth on indigo.js, a found line of its own).
project_env() {
  case $1 in
    indigo) echo "TEQ_THREADS=1 " ;;
  esac
}
# local_project <project>: the project's lines over its checkout here (--lila, --airstream, --laminar, --tyrian, --indigo), one after
# another: bench/<project>/<project>.scala's verbs, its logs and caches under the gate's temporary directory and its
# logs copied into the attempt's afterwards.
local_project() {
  local p=$1 verb s code checkout=${!1}
  for verb in export compile test run; do
    selected "$p-$verb" || continue
    s=$(now)
    mkdir -p "$tmp/$p" "$out/$p"
    (cd "$wt" && eval "$(project_preflight $p $verb)true" && env $(project_env $p)timeout 3600 "$bin" interp $(project_script $p) "$verb" "$checkout" "$tmp/$p" --teq "$bin") > "$out/$p-$verb.log" 2>&1
    code=$?
    find "$tmp/$p" -maxdepth 2 -name '*.log' -exec cp {} "$out/$p/" \;
    result "$p-$verb" local $(($(now) - s)) $code "$(grep -E "^(FAIL )?$p: " "$out/$p-$verb.log" | tail -1)" "$out/local.results"
  done
}
# The lines here run at once, but the export's acceptance after its identity (it reads the identity's log, and with
# a manifest builds master's saved export as the identity does), and then the end-to-end suite alone (its specs wait
# on the application's services within the browser's timeouts), the memory line alone (its growth rows depend on
# the machine's load) and the asymptotic probes alone (their growth ratios likewise); the budget programs are
# generated once before the two instruction lines read them. The binary of the head is built first when it is
# missing or not the head's.
local_half() {
  local s code
  if [ $local_build = 1 ]; then
    s=$(now)
    (cd "$wt" && timeout 1500 cargo build --release) > "$out/build-local.log" 2>&1
    code=$?
    version=$("$bin" --version 2> /dev/null)
    [ $code != 0 ] || [[ "$full" == "$(echo "$version" | awk '{print $3}')"* ]] || { echo "the build is ${version:-missing}, not the head $head" >> "$out/build-local.log"; code=1; }
    result build local $(($(now) - s)) $code "the reference machine's binary of the head: $(tail -1 "$out/build-local.log")" "$out/local.results"
    if [ $code != 0 ]; then
      for name in $local_lines $project_lines; do
        [[ " $local_lines $here_lines " == *" $name "* ]] && selected $name && result $name local 0 1 "not run: no binary of the head here (build-local.log)" "$out/local.results"
      done
      now > "$out/local.ended"
      return 1
    fi
  fi
  if [ -n "$master" ] && { selected instructions || selected instructions-derive; } && [ -n "$(kind "$version")" ]; then
    for name in instructions instructions-derive; do
      selected $name && result $name local 0 1 "$bin is $version, not a plain build; the instruction comparison is of two plain builds" "$out/local.results"
    done
  elif selected instructions || selected instructions-derive; then
    (work=out/budget && . bench/programs.sh) > "$out/budget-programs.log" 2>&1
    local_line instructions &
    local_line instructions-derive &
  fi
  local_line size &
  { local_line app-export; local_line app-export-accept; } &
  local_line app-api &
  local_line app-api-test &
  local_line app-chain &
  local_line app-scalac &
  for p in $here_projects; do local_project $p & done
  wait
  local_line app-cypress
  local_line memory
  local_line asymptotics
  now > "$out/local.ended"
}
# app_root: the application's checkout, the --app build file's root.
app_root() {
  python3 -c 'import json, os, sys; f = sys.argv[1]; print(os.path.normpath(os.path.join(os.path.dirname(f), json.load(open(f)).get("root", "../../.."))))' "$app"
}
# The two chains' summary lines as one: per side its counts and the module it defers, or the build that failed.
chain_counts() {
  python3 - "$@" <<'PYEOF'
import re, sys
parts = []
for log in sys.argv[1:]:
    lines = [l.rstrip() for l in open(log) if l.startswith("chain ")] or ["chain ?: no summary"]
    m = re.match(r"chain (\w+): .*; (\d+ files identical, \d+ listed, \d+ differ)(?:; deferred: (\S+))?", lines[-1])
    parts.append(f"{m.group(1)} {m.group(2)}" + (f", {m.group(3)} deferred" if m.group(3) else "") if m else lines[-1][:200])
print("; ".join(parts))
PYEOF
}
local_started=$(now)
if [ $local_selected = 1 ]; then
  { local_half; rc=$?; echo "$rc" > "$out/local.exit"; exit "$rc"; } &
  local_pid=$!
fi

# The suites the machines are to run, known before either half claims.
want=()
for entry in "${machine[@]}"; do
  name=${entry%% *}
  selected "$name" || continue
  path=$(script_of "${entry#* }")
  if [ -n "$path" ] && [ ! -e "$wt/$path" ]; then
    result "$name" - 0 skip "no $path in this tree"
    continue
  fi
  want+=("$entry")
done
for entry in "${want[@]}"; do echo "$entry"; done > "$tmp/want"
[ ${#want[@]} -gt 0 ] || touch "$tmp/shards.none" "$tmp/shards.asked"

# sha256 <file>: its digest.
sha256() { { sha256sum "$1" 2> /dev/null || shasum -a 256 "$1"; } | cut -d' ' -f1; }
# free_hosts <pool> and holder_hosts <pool>: the free machines, but those holding the projects' checkouts while a
# project line is to run on one and none is claimed; and the free machines holding them: the one GATE_LILA_MACHINE
# names, else those whose pool line's holds=<project>,... names every project wanted.
pool_hosts() {
  echo "$2" | awk -v mode="$1" -v want="$projects" -v named="${GATE_LILA_MACHINE:-}" '
    function holder(   i, j, n, h, w, have) {
      if (named != "") return $1 == named
      for (i = 2; i <= NF; i++) if ($i ~ /^holds=/) { n = split(substr($i, 7), h, ","); for (j = 1; j <= n; j++) have[h[j]] = 1 }
      n = split(want, w, " ")
      for (j = 1; j <= n; j++) if (!(w[j] in have)) return 0
      return 1
    }
    $2 == "free" && (mode == "holders" ? want != "" && holder() : !(mode == "skip" && holder())) { print $1 }'
}
free_hosts() {
  if [ -n "$projects" ] && ! ls "$tmp"/pclaim.* > /dev/null 2>&1; then pool_hosts skip "$1"; else pool_hosts all "$1"; fi
}
holder_hosts() { pool_hosts holders "$1"; }
# ship <handle> <log>: the builder's binary ($tmp/teq-head) mirrored to that machine as teq-head and announced there
# (defined before the halves start, which ship it too).
ship() {
  "$ra/sync.sh" "$1" "$tmp/teq-head" > "$2" 2>&1 &&
    "$ra/run.sh" "$1" teq -t 60 "echo $(cat "$tmp/head.from") $(cat "$tmp/head.digest") > ../teq-head/ready" >> "$2" 2>&1
}
# prepare <free machines> <lines>: how many of the free machines the comparison prepares, by the recorded times
# (tests/gate-times.txt; a build 240 s, a suite or line without a time 120): up to one a line, leaving the shards one
# when there are suites, as many as keep the shards' end (the head's build, then the suites dealt to the other
# machines) within a minute of their end beside one comparison machine. Which of them measure is decided when the
# suites have ended (deal_lines).
prepare() {
  python3 - tests/gate-times.txt "$1" "$2" "$tmp/want" <<'PY'
import sys
times = {}
for l in open(sys.argv[1]):
    if l.strip() and not l.startswith("#"):
        name, secs = l.split()
        times[name] = int(secs)
free, lines = int(sys.argv[2]), sys.argv[3].split()
suites = [l.split()[0] for l in open(sys.argv[4]) if l.strip()]
build = times.get("build", 240)
def span(names, n):
    load = [0] * n
    for t in sorted((times.get(x, 120) for x in names), reverse=True):
        load[load.index(min(load))] += t
    return max(load)
def suites_end(k):
    shards = min(free - k, len(suites))
    return build + span(suites, shards) if shards > 0 else (float("inf") if suites else 0)
most = max(1, min(len(lines), free - (1 if suites else 0)))
print(max(k for k in range(1, most + 1) if k == 1 or suites_end(k) <= suites_end(1) + 60))
PY
}
# deal_lines <seconds so far> <machines ready> <lines>: the lines dealt to the fewest of the machines, in their order,
# with which the gate ends within thirty minutes (the time so far, the lines' recorded times dealt longest first to
# the machine with the least, and two minutes for the measure's variance and the collection), all of them when none
# fits; a line "<machine> <lines>" each, "none" for one that measures nothing, and a line "# <the decision>".
deal_lines() {
  python3 - tests/gate-times.txt "$1" "$2" "$3" <<'PY'
import sys
times = {}
for l in open(sys.argv[1]):
    if l.strip() and not l.startswith("#"):
        name, secs = l.split()
        times[name] = int(secs)
so_far, hosts, lines = int(sys.argv[2]), sys.argv[3].split(), sys.argv[4].split()
def deal(k):
    load, dealt = [0] * k, [[] for _ in range(k)]
    for name in sorted(lines, key=lambda n: -times.get(n, 120)):
        i = load.index(min(load))
        load[i] += times.get(name, 120)
        dealt[i].append(name)
    return max(load, default=0), dealt
k = len(hosts)
for n in range(1, len(hosts) + 1):
    if so_far + deal(n)[0] + 120 <= 1800:
        k = n
        break
end, dealt = deal(k) if hosts else (0, [])
for i, h in enumerate(hosts):
    print(h, " ".join(n for n in lines if i < k and n in dealt[i]) or "none")
if not hosts:
    print(f"# no comparison machine ready to measure{''.join(' ' + n for n in lines)}")
else:
    print(f"# measure: the lines on {k} of {len(hosts)} machines ready, {so_far} s into the gate, by the recorded "
          f"times {end} s more: " + "; ".join(f"{hosts[i]} {' '.join(n for n in lines if n in dealt[i])}" for i in range(k)))
PY
}

# The comparison's half, in the background beside the shards: its results go to their own file. The shards hand it
# the head's binary through $tmp: the directory $tmp/teq-head (mirrored to a machine as the project teq-head) and
# $tmp/head.ready once it holds the builder's build ($tmp/head.digest its digest), $tmp/head.failed when no shard
# built one; $tmp/shards.claimed when the shards hold machines, $tmp/shards.asked once their first claim is over,
# $tmp/shards.none when they will hold none.
compare_half() {
  local file=$out/compare.results s handle host pool= answer i left deadline lines= name free k=1 held j pending ready
  for name in $compare; do selected $name && lines="$lines $name"; done
  s=$(now)
  deadline=$((s + ${GATE_CLAIM_WAIT:-1200}))
  # At the first ask that finds free machines, how many the comparison prepares (prepare); then it claims them, and
  # asks again while it holds none.
  for i in $(seq 1 100000); do
    left=$((deadline - $(now)))
    answer=$(timeout $((left > 30 ? left : 30)) "$ra/pool.sh" 2>&1) && pool=$answer
    free=$(free_hosts "$pool")
    if [ -n "$free" ] && [ ! -e "$tmp/compare.k" ]; then
      k=$(prepare "$(echo "$free" | wc -l)" "$lines")
      echo "$k" > "$tmp/compare.k"
    fi
    for host in $free; do
      [ "$(ls "$tmp"/cclaim.* 2> /dev/null | wc -l)" -lt "$k" ] || break
      touch "$tmp/compare.claiming"
      [ ! -e "$tmp/stop" ] || { rm -f "$tmp/compare.claiming"; break 2; }
      handle=$("$ra/claim.sh" "gate-$head" "$host" 2> "$tmp/claim-err.compare") && echo "$handle $(($(now) - s))" > "$tmp/cclaim.$host"
      rm -f "$tmp/compare.claiming"
    done
    touch "$tmp/compare.asked"
    ls "$tmp"/cclaim.* > /dev/null 2>&1 && break
    left=$((deadline - $(now)))
    [ $left -gt 0 ] || break
    sleep $((left < poll ? left : poll))
  done
  if ! ls "$tmp"/cclaim.* > /dev/null 2>&1; then
    pool=$(echo "$pool" | awk '{printf "%s%s", sep, $0; sep = "; "}' | tr -s ' ')
    for name in $lines; do result $name - $(($(now) - s)) 1 "not run: claim timeout after $(($(now) - s))s (GATE_CLAIM_WAIT); the pool at the last ask: $pool" "$file"; done
    now > "$out/compare.ended"
    return
  fi
  # Every machine held is prepared for every line (its syncs, master's and the reference's builds, the head's binary),
  # beside the suites, all of them mirroring one clone of master's sources and one of the reference's (where their
  # sources differ), the gate's own.
  mkdir -p "$tmp/compare-trees"
  if ! { git clone -q --shared --no-checkout "$(git -C "$wt" rev-parse --path-format=absolute --git-common-dir)" "$tmp/compare-trees/teq-master" &&
    git -C "$tmp/compare-trees/teq-master" checkout -q -f --detach "$crev"; }; then
    echo "no clone of master at $crev" > "$tmp/compare-trees/failed"
  elif [ "$(sources "$refrev")" != "$(sources "$crev")" ] && ! { git clone -q --shared --no-checkout "$(git -C "$wt" rev-parse --path-format=absolute --git-common-dir)" "$tmp/compare-trees/teq-reference" &&
    git -C "$tmp/compare-trees/teq-reference" checkout -q -f --detach "$refrev"; }; then
    echo "no clone of the reference at $refrev" > "$tmp/compare-trees/failed"
  else
    touch "$tmp/compare-trees/ready"
  fi
  held=$(for f in "$tmp"/cclaim.*; do echo "${f#$tmp/cclaim.}"; done)
  echo "$(clock) comparison: $(echo $held) prepared for$lines"
  for host in $held; do
    read -r handle j < "$tmp/cclaim.$host"
    result compare "$host" "$j" 0 "claim: waited ${j}s for a comparison machine, $handle" "$file"
    compare_on "$handle" "$lines" &
  done
  # The measure: once the shards' and the projects' suites have ended and every machine held waits to measure or has
  # ended, the lines go to as few of the machines ready as end the gate within thirty minutes, by the time it has
  # taken so far and the lines' recorded times (deal_lines); the others measure nothing.
  while :; do
    if [ -e "$tmp/shards.ended" ] && { [ -z "$projects" ] || [ -e "$out/projects.ended" ]; }; then
      pending=0
      for host in $held; do
        grep -q '^measure-wanted ' "$tmp/compare-$host/status" 2> /dev/null || [ "$(tail -1 "$tmp/compare-$host/status" 2> /dev/null)" = done ] || pending=1
      done
      [ $pending = 1 ] || break
    fi
    [ $(now) -lt $((s + ${GATE_COMPARE_SECONDS:-3600})) ] || break
    sleep $cpoll
  done
  ready=$(for host in $held; do grep -q '^measure-wanted ' "$tmp/compare-$host/status" 2> /dev/null && echo $host; done)
  deal_lines $(($(now) - gate_started)) "$(echo $ready)" "$lines" > "$tmp/compare.assign.new" && mv "$tmp/compare.assign.new" "$tmp/compare.assign"
  result compare - 0 0 "$(sed -n 's/^# //p' "$tmp/compare.assign")" "$file"
  echo "$(clock) comparison: $(sed -n 's/^# //p' "$tmp/compare.assign")"
  wait
  now > "$out/compare.ended"
}
# compare_on <handle> <lines>: that machine prepared for the lines and measuring those the comparison deals it, its
# logs under $out/compare-<machine>/.
compare_on() {
  local handle=$1 lines=" $2" host=${1%%/*} file=$out/compare.results t=$tmp/compare-${1%%/*} from seen=0 answered=0
  local refdir= reftree=../teq-master tree i job envs deadline left name code secs last s=$(now) logs=$out/compare-${1%%/*} measured=0 here
  mkdir -p "$t"
  # master's sources and the reference's, the comparison's one clone of each (compare_half), mirrored as projects
  if [ ! -e "$tmp/compare-trees/ready" ]; then
    result compare "$host" 0 1 "sync: $(cat "$tmp/compare-trees/failed" 2> /dev/null || echo "no clone of master at $crev")" "$file"
    release "$handle"
    return
  fi
  if [ -d "$tmp/compare-trees/teq-reference" ]; then
    refdir=$tmp/compare-trees/teq-reference
    reftree=../teq-reference
  fi
  i=$(now)
  for tree in "$wt" "$tmp/compare-trees/teq-master" $refdir; do
    "$ra/sync.sh" "$handle" "$tree" >> "$t/sync" 2>&1 || { result compare "$host" $(($(now) - i)) 1 "sync: $(tail -1 "$t/sync")" "$file"; release "$handle"; return; }
  done
  result compare "$host" $(($(now) - i)) 0 "sync: the head, master ($crev) and the reference ($refrev) mirrored for$lines" "$file"
  # the head's origin is decided once the shards' first claim is over
  for i in $(seq 1 120); do [ -e "$tmp/shards.asked" ] && break; sleep 5; done
  from=
  [ -e "$tmp/shards.none" ] && from=here
  [ -e "$tmp/shards.asked" ] && [ ! -e "$tmp/shards.claimed" ] && from=here
  job=$(base64 < tests/gate-compare.sh | tr -d '\n')
  envs="GATE_HEAD=$head GATE_HEAD_SOURCE=$(sources "$full") GATE_MASTER=$crev GATE_MASTER_SOURCE=$(sources "$crev") GATE_MASTER_TREE=../teq-master"
  envs="$envs GATE_REFERENCE=$refrev GATE_REFERENCE_SOURCE=$(sources "$refrev") GATE_REFERENCE_TREE=$reftree GATE_LINES=$(printf %q "$lines")"
  envs="$envs GATE_MEASURE_AFTER=1"
  [ -z "$from" ] || envs="$envs GATE_HEAD_FROM=here"
  "$ra/run.sh" "$handle" teq -t 60 "rm -rf out/compare ../teq-head && mkdir -p out/compare && echo $job | base64 -d > out/compare/gate-compare.sh && { $envs nohup bash out/compare/gate-compare.sh > out/compare/job.out 2>&1 < /dev/null & }" > "$t/launch" 2>&1 ||
    { result compare "$host" 0 1 "the job did not start: $(tail -1 "$t/launch")" "$file"; release "$handle"; return; }
  # Polled every 15 s (half of GATE_POLL_SECONDS) until GATE_COMPARE_SECONDS (default 3600) have passed since the
  # job started.
  deadline=$(($(now) + ${GATE_COMPARE_SECONDS:-3600}))
  touch "$t/status"
  for i in $(seq 1 100000); do
    left=$((deadline - $(now)))
    [ $left -gt 0 ] || break
    if timeout $left "$ra/run.sh" "$handle" teq -t $((left < 60 ? left : 60)) 'cat out/compare/status 2> /dev/null' > "$t/status.new" 2> /dev/null; then
      tail -n +$((seen + 1)) "$t/status.new" | while read -r name code secs last; do
        [ "$name" = done ] || printf '%s %-8s %-18s [%s] %5ss  %s\n' "$(clock)" "$host" "$name" "$code" "$secs" "$(echo "$last" | cut -c1-100)"
      done
      seen=$(wc -l < "$t/status.new")
      mv "$t/status.new" "$t/status"
      [ "$(tail -1 "$t/status")" = done ] && break
      if [ $answered = 0 ] && grep -q '^head-wanted ' "$t/status"; then
        if [ -e "$tmp/head.ready" ]; then
          "$ra/sync.sh" "$handle" "$tmp/teq-head" > "$t/head" 2>&1 &&
            "$ra/run.sh" "$handle" teq -t 60 "echo $(cat "$tmp/head.from") $(cat "$tmp/head.digest") > out/compare/head-ready" >> "$t/head" 2>&1 && answered=1
        elif [ -e "$tmp/head.failed" ] || [ -e "$tmp/shards.none" ]; then
          "$ra/run.sh" "$handle" teq -t 60 "echo no shard built the head > out/compare/build-head" > "$t/head" 2>&1 && answered=1
        fi
      fi
      # the lines this machine measures, once the comparison has dealt them (compare_half), or none
      if [ $measured = 0 ] && grep -q '^measure-wanted ' "$t/status" && [ -e "$tmp/compare.assign" ]; then
        here=$(awk -v h="$host" '$1 == h {$1 = ""; sub(/^ /, ""); print}' "$tmp/compare.assign")
        "$ra/run.sh" "$handle" teq -t 60 "echo ${here:-none} > out/compare/measure" > "$t/measure" 2>&1 && measured=1
      fi
    fi
    left=$((deadline - $(now)))
    [ $left -gt 0 ] && sleep $((left < cpoll ? left : cpoll))
  done
  mkdir -p "$logs"
  "$ra/get.sh" "$handle" teq out/compare/logs "$logs" > "$t/get" 2>&1 || result compare "$host" 0 1 "collect: the comparison's logs did not come back: $(tail -1 "$t/get")" "$file"
  release "$handle" || result compare "$host" 0 1 "release: $handle not released: $(tail -1 "$tmp/release.$(mark "$handle")")" "$file"
  cp "$t/status" "$logs/status"
  while read -r name code secs last; do
    case $name in
      done | head-wanted | measure-wanted) ;;
      master | reference | head | quiet) result compare "$host" "$secs" "$code" "$name: $last" "$file" ;;
      *) result "$name" "$host" "$secs" "$code" "$last" "$file" ;;
    esac
  done < "$t/status"
  [ "$(tail -1 "$t/status")" = done ] || result compare "$host" $(($(now) - s)) 1 "the job did not end within GATE_COMPARE_SECONDS" "$file"
}
compare_started=$(now)
if [ $compare_selected = 1 ]; then
  { compare_half; rc=$?; echo "$rc" > "$out/compare.exit"; exit "$rc"; } &
  compare_pid=$!
fi

# The projects' half, in the background beside the shards: the machine that holds the checkouts claimed (it waits
# at most GATE_CLAIM_WAIT seconds for it), the tree synced there and its lines run by tests/gate-shard.sh as one
# list, each checkout's verbs one after another in their order, each bounded at 3600 s and without the want-of rule
# (a test run's summary counts its skipped cases), over the head's binary the shards hand it as they hand the
# comparison's; its results go to their own file.
projects_half() {
  local file=$out/projects.results s handle= host pool= answer i left deadline p verb list= entry envs seen=0 answered=0
  local name code secs last runner
  for p in $projects; do
    for verb in export compile test run; do
      selected "$p-$verb" || continue
      # The project's directory of logs and caches, outside the repository and fresh at its export.
      entry="d=\$(dirname \"\$PWD\")/gate-projects/$p; c=\${GATE_$(echo $p | tr a-z A-Z)_CHECKOUT:-}; [ -n \"\$c\" ] || { echo \"FAIL $p: needs GATE_$(echo $p | tr a-z A-Z)_CHECKOUT, the checkout at bench/$p/pin.json's commit, on \$(hostname)\"; exit 1; }"
      [ $verb != export ] || entry="$entry; rm -rf \"\$d\""
      entry="$entry; mkdir -p \"\$d\" out/gate/$p && $(project_preflight $p $verb)$(project_env $p)timeout 3600 \"\$TEQ\" interp $(project_script $p) $verb \"\$c\" \"\$d\" --teq \"\$TEQ\"; code=\$?; find \"\$d\" -maxdepth 2 -name '*.log' -exec cp {} out/gate/$p/ \\;; exit \$code"
      list="$list$p-$verb $entry"$'\n'
    done
  done
  s=$(now)
  deadline=$((s + ${GATE_CLAIM_WAIT:-1200}))
  for i in $(seq 1 100000); do
    left=$((deadline - $(now)))
    answer=$(timeout $((left > 30 ? left : 30)) "$ra/pool.sh" 2>&1) && pool=$answer
    for host in $(holder_hosts "$pool"); do
      touch "$tmp/projects.claiming"
      [ ! -e "$tmp/stop" ] || { rm -f "$tmp/projects.claiming"; break 2; }
      handle=$("$ra/claim.sh" "gate-$head" "$host" 2> "$tmp/claim-err.projects") && echo "$handle $(($(now) - s))" > "$tmp/pclaim.$host"
      rm -f "$tmp/projects.claiming"
      [ -z "$handle" ] || break
    done
    touch "$tmp/projects.asked"
    [ -z "$handle" ] || break
    left=$((deadline - $(now)))
    [ $left -gt 0 ] || break
    sleep $((left < poll ? left : poll))
  done
  if [ -z "$handle" ]; then
    pool=$(echo "$pool" | awk '{printf "%s%s", sep, $0; sep = "; "}' | tr -s ' ')
    while read -r name _; do
      [ -z "$name" ] || result "$name" - $(($(now) - s)) 1 "not run: no free machine holds$projects (a pool line's holds=, or GATE_LILA_MACHINE) after $(($(now) - s))s; the pool at the last ask: $pool" "$file"
    done <<< "$list"
    now > "$out/projects.ended"
    return
  fi
  host=${handle%%/*}
  result projects "$host" $(($(now) - s)) 0 "claim: waited $(($(now) - s))s for the machine holding$projects, $handle" "$file"
  i=$(now)
  "$ra/sync.sh" "$handle" "$wt" > "$tmp/projects.sync" 2>&1 || {
    while read -r name _; do [ -z "$name" ] || result "$name" "$host" 0 1 "not run: the tree did not reach $host: $(tail -1 "$tmp/projects.sync")" "$file"; done <<< "$list"
    release "$handle"; now > "$out/projects.ended"; return; }
  result projects "$host" $(($(now) - i)) 0 "sync: the head mirrored" "$file"
  # the head's origin is decided once the shards' first claim is over: theirs, or built here
  for i in $(seq 1 120); do [ -e "$tmp/shards.asked" ] && break; sleep 5; done
  envs="GATE_REVISION=$head GATE_SOURCE=$(sources "$full") GATE_SUITE_SECONDS=3700"
  if [ -e "$tmp/shards.claimed" ] && [ ! -e "$tmp/shards.none" ]; then envs="$envs GATE_HEAD_DIR=../teq-head"; else answered=1; fi
  runner=$(base64 < tests/gate-shard.sh | tr -d '\n')
  "$ra/run.sh" "$handle" teq -t 60 "rm -rf out/gate ../teq-head && mkdir -p out/gate && echo $runner | base64 -d > out/gate/shard.sh && echo $(printf '%s' "$list" | base64 | tr -d '\n') | base64 -d > out/gate/suites && { $envs nohup bash out/gate/shard.sh out/gate/suites > out/gate/shard.out 2>&1 < /dev/null & }" > "$tmp/projects.launch" 2>&1 ||
    { result projects "$host" 0 1 "the job did not start: $(tail -1 "$tmp/projects.launch")" "$file"; release "$handle"; now > "$out/projects.ended"; return; }
  echo "$(clock) $host: $(printf '%s' "$list" | awk '{printf "%s%s", sep, $1; sep = " "}')"
  # Polled every 30 s until GATE_MACHINE_SECONDS (default 2700) have passed since the job started.
  deadline=$(($(now) + ${GATE_MACHINE_SECONDS:-2700}))
  touch "$tmp/projects.status"
  for i in $(seq 1 100000); do
    left=$((deadline - $(now)))
    [ $left -gt 0 ] || break
    if timeout $left "$ra/run.sh" "$handle" teq -t $((left < 60 ? left : 60)) 'cat out/gate/status 2> /dev/null' > "$tmp/projects.status.new" 2> /dev/null; then
      tail -n +$((seen + 1)) "$tmp/projects.status.new" | while read -r name code secs last; do
        [ "$name" = done ] || printf '%s %-8s %-18s [%s] %5ss  %s\n' "$(clock)" "$host" "$name" "$code" "$secs" "$(echo "$last" | cut -c1-100)"
      done
      seen=$(wc -l < "$tmp/projects.status.new")
      mv "$tmp/projects.status.new" "$tmp/projects.status"
      [ "$(tail -1 "$tmp/projects.status")" = done ] && break
    fi
    # the head's binary, once the builder's build ends (the job writes no status before it has one)
    if [ $answered = 0 ]; then
      if [ -e "$tmp/head.ready" ]; then
        ship "$handle" "$tmp/projects.head" && answered=1
      elif [ -e "$tmp/head.failed" ]; then
        "$ra/run.sh" "$handle" teq -t 60 "mkdir -p ../teq-head && echo the builder did not build the head > ../teq-head/build-here" > "$tmp/projects.head" 2>&1 && answered=1
      fi
    fi
    left=$((deadline - $(now)))
    [ $left -gt 0 ] && sleep $((left < poll ? left : poll))
  done
  mkdir -p "$out/projects"
  "$ra/get.sh" "$handle" teq out/gate "$out/projects" > "$tmp/projects.get" 2>&1 || result projects "$host" 0 1 "collect: the logs did not come back: $(tail -1 "$tmp/projects.get")" "$file"
  release "$handle" || result projects "$host" 0 1 "release: $handle not released: $(tail -1 "$tmp/release.$(mark "$handle")")" "$file"
  while read -r name code secs last; do
    [ "$name" = done ] || result "$name" "$host" "$secs" "$code" "$last" "$file"
  done < "$tmp/projects.status"
  while read -r name _; do
    [ -z "$name" ] || grep -q "^$name " "$tmp/projects.status" || result "$name" "$host" 0 1 "not finished" "$file"
  done <<< "$list"
  now > "$out/projects.ended"
}
projects_started=$(now)
if [ -n "$projects" ]; then
  { projects_half; rc=$?; echo "$rc" > "$out/projects.exit"; exit "$rc"; } &
  projects_pid=$!
fi

# The machines' half.
# collect <i>: shard i's logs into $out, its machine released, its lines into the results.
collect() {
  local i=$1 f b name code secs last
  if "$ra/get.sh" "${handles[$i]}" teq out/gate "$tmp/fetch.$i" > "$tmp/get.$i" 2>&1; then
    for f in "$tmp/fetch.$i"/*; do
      b=$(basename "$f")
      case $b in
        shard.sh | suites) ;;
        build.log | ref.build.log | cargo-test.build.log | shard.out | status | build.manifest) cp "$f" "$out/${b%%.*}-${hosts[$i]}.${b#*.}" ;;
        *) cp -R "$f" "$out/$b" ;;
      esac
    done
    cp "$tmp/fetch.$i/status" "$tmp/status.$i" 2> /dev/null
  else
    # the status polled so far is still shown, but without the logs nothing of this shard counts as passed
    result collect "${hosts[$i]}" 0 1 "the logs did not come back: $(tail -1 "$tmp/get.$i")"
  fi
  release "${handles[$i]}" || result release "${hosts[$i]}" 0 1 "${handles[$i]} not released: $(tail -1 "$tmp/release.$(mark "${handles[$i]}")")"
  touch "$tmp/status.$i"
  while read -r name code secs last; do
    [ "$name" = done ] || result "$name" "${hosts[$i]}" "$secs" "$code" "$last"
  done < "$tmp/status.$i"
  while read -r name _; do
    grep -q "^$name " "$tmp/status.$i" || result "$name" "${hosts[$i]}" 0 1 "not finished"
  done < "$tmp/list.$i"
  result shard "${hosts[$i]}" "$(awk '$1 != "done" {s += $3} END {print s + 0}' "$tmp/status.$i")" 0 \
    "the build, then $(awk '{printf "%s%s", sep, $1; sep = " "}' "$tmp/list.$i"); ended $(($(now) - machines_started))s into the machines' half"
}
# take_head <i>: the builder's binary and manifest into $tmp/teq-head, once, its digest the manifest's: then
# $tmp/head.ready for the other machines, else $tmp/head.failed.
take_head() {
  local i=$1 digest
  rm -rf "$tmp/teq-head" && mkdir -p "$tmp/teq-head" && git -C "$tmp/teq-head" init -q &&
    "$ra/get.sh" "${handles[$i]}" teq target/release/teq "$tmp/teq-head/teq" > "$tmp/head.get" 2>&1 &&
    "$ra/get.sh" "${handles[$i]}" teq out/gate/build.manifest "$tmp/teq-head/manifest" >> "$tmp/head.get" 2>&1 &&
    digest=$(sed -n 's/^sha256 //p' "$tmp/teq-head/manifest") && [ -n "$digest" ] &&
    [ "$(sha256 "$tmp/teq-head/teq")" = "$digest" ] || { echo "the builder's binary is not its manifest's" >> "$tmp/head.get"; touch "$tmp/head.failed"; return 1; }
  chmod +x "$tmp/teq-head/teq"
  echo "$digest" > "$tmp/head.digest"
  echo "${hosts[$i]}" > "$tmp/head.from"
  touch "$tmp/head.ready"
}
machines_started=$(now)
hosts=()
handles=()
if [ ${#want[@]} -gt 0 ]; then
  # The comparison machines are claimed first: the shards ask once the comparison's first ask is over, and leave it
  # the first free machine of every ask until it holds one; neither takes the machine holding the projects'
  # checkouts while a project line is to run there.
  if [ $compare_selected = 1 ]; then
    for i in $(seq 1 200); do [ -e "$tmp/compare.asked" ] && break; kill -0 "$compare_pid" 2> /dev/null || break; sleep 2; done
  fi
  # The machines the pool shows free are claimed at once, as many as there are suites or GATE_MACHINES; with none
  # free, the pool is asked again every 30 s until GATE_CLAIM_SECONDS (default 1200) have passed since the first
  # ask. A claim's wait runs from the first ask to its handle. A claim once started runs to its end (claim.sh
  # bounds itself), since one stopped halfway could leave a machine held under a handle the gate never learns.
  pool=
  deadline=$((machines_started + ${GATE_CLAIM_SECONDS:-1200}))
  for i in $(seq 1 100000); do
    left=$((deadline - $(now)))
    answer=$(timeout $((left > 30 ? left : 30)) "$ra/pool.sh" 2>&1) && pool=$answer
    claimers=()
    free=$(free_hosts "$pool")
    # until the comparison holds its machine, the first free one is left to it
    if [ $compare_selected = 1 ] && ! ls "$tmp"/cclaim.* > /dev/null 2>&1 && kill -0 "$compare_pid" 2> /dev/null; then
      free=$(echo "$free" | tail -n +2)
    fi
    for h in $(echo "$free" | head -n "${GATE_MACHINES:-${#want[@]}}"); do
      (handle=$("$ra/claim.sh" "gate-$head" "$h" 2> "$tmp/claim-err.$h") && echo "$handle $(($(now) - machines_started))" > "$tmp/claim.$h") &
      claimers+=($!)
    done
    [ ${#claimers[@]} = 0 ] || wait "${claimers[@]}"
    claimers=()
    ls "$tmp"/claim.* > /dev/null 2>&1 && touch "$tmp/shards.claimed"
    touch "$tmp/shards.asked"
    [ -e "$tmp/shards.claimed" ] && break
    left=$((deadline - $(now)))
    [ $left -gt 0 ] || break
    sleep $((left < poll ? left : poll))
  done
  for f in "$tmp"/claim.*; do
    [ -e "$f" ] || continue
    read -r handle waited < "$f"
    hosts+=("${handle%%/*}")
    handles+=("$handle")
    result claim "${handle%%/*}" "$waited" 0 "waited ${waited}s for the machine: $handle"
  done
  busy=$(echo "$pool" | awk '$2 != "free" {printf "%s%s", sep, $0; sep = "; "}' | tr -s ' ')
  result pool - 0 0 "${#handles[@]} machines claimed${busy:+; not free at the last ask: $busy}"
  echo "$(clock) machines: ${hosts[*]:-none}${busy:+ (not free: $busy)}"
fi
n=${#handles[@]}
if [ ${#want[@]} -gt 0 ] && [ $n = 0 ]; then
  for entry in "${want[@]}"; do result "${entry%% *}" - 0 1 "not run: no machine was free"; done
fi
good=()
if [ $n -gt 0 ]; then
  # The tree to every machine at once, and a look at what the suites need there (a skip would read as a pass): a
  # machine whose sync fails or that lacks one of them is released and left out.
  ready='. tests/support/jars.sh; missing=; for name in $(sed -n "s/^ *\([A-Za-z0-9._-]*\)) echo \"\$M2\/.*/\1/p" tests/support/jars.sh); do [ -e "$(jar_of $name)" ] || missing="$missing $name"; done
    [ -z "$missing" ] || { echo "not in the coursier cache:$missing"; exit 1; }
    [ -d "$SCALA3/tests" ] || { echo "no scala3 checkout at $SCALA3"; exit 1; }
    for c in cargo java javap node scala-cli python3; do command -v $c > /dev/null || { echo "no $c"; exit 1; }; done'
  pids=()
  for i in "${!handles[@]}"; do
    ("$ra/sync.sh" "${handles[$i]}" "$wt" > "$tmp/sync.$i" 2>&1 && "$ra/run.sh" "${handles[$i]}" teq -t 60 "$ready" >> "$tmp/sync.$i" 2>&1
      echo $? > "$tmp/sync-exit.$i") &
    pids+=($!)
  done
  wait "${pids[@]}"
  for i in "${!handles[@]}"; do
    if [ "$(cat "$tmp/sync-exit.$i")" = 0 ]; then
      good+=("$i")
    else
      result sync "${hosts[$i]}" 0 1 "$(tail -1 "$tmp/sync.$i")"
      release "${handles[$i]}" || result release "${hosts[$i]}" 0 1 "${handles[$i]} not released: $(tail -1 "$tmp/release.$(mark "${handles[$i]}")")"
    fi
  done
  echo "$(clock) synced to ${#good[@]} of $n machines"
  n=${#good[@]}
  if [ $n = 0 ]; then
    for entry in "${want[@]}"; do result "${entry%% *}" - 0 1 "not run: no machine took the tree"; done
  fi
fi
[ $n -gt 0 ] || touch "$tmp/shards.none" "$tmp/shards.asked"
if [ $n -gt 0 ]; then
  # The suites dealt out by their recorded times, longest first, each to the shard with the least so far.
  python3 - "$tmp/want" tests/gate-times.txt "$n" > "$tmp/deal" <<'PY'
import sys
want = [l.rstrip("\n") for l in open(sys.argv[1]) if l.strip()]
times = {}
for l in open(sys.argv[2]):
    if l.strip() and not l.startswith("#"):
        name, secs = l.split()
        times[name] = int(secs)
n = int(sys.argv[3])
load = [0] * n
for entry in sorted(want, key=lambda e: -times.get(e.split()[0], 120)):
    shard = min(range(n), key=lambda s: load[s])
    load[shard] += times.get(entry.split()[0], 120)
    print(shard, entry)
PY
  ref=
  ref_shard=
  if [ -n "$mfull" ] && [ "$mfull" != "$full" ] && [ -z "$manifest" ] && grep -q ' split-determinism ' "$tmp/deal"; then
    # master's sources for REF: this gate's own clone at master's revision, mirrored as a project of its own
    refdir=$tmp/teq-master
    git clone -q --shared --no-checkout "$(git -C "$wt" rev-parse --path-format=absolute --git-common-dir)" "$refdir"
    ref_shard=$(awk '$2 == "split-determinism" {print $1}' "$tmp/deal")
    i=${good[$ref_shard]}
    if git -C "$refdir" checkout -q -f --detach "$mfull" && "$ra/sync.sh" "${handles[$i]}" "$refdir" > "$tmp/sync-ref" 2>&1; then
      ref=../teq-master
    else
      grep -v ' split-determinism ' "$tmp/deal" > "$tmp/deal.kept"
      mv "$tmp/deal.kept" "$tmp/deal"
      result split-determinism "${hosts[$i]}" 0 1 "not run: master's sources for REF did not reach the machine: $(tail -1 "$tmp/sync-ref")"
    fi
  fi
  # The first shard's machine builds the head (the builder); the others wait for its binary.
  builder=${good[0]}
  runner=$(base64 < tests/gate-shard.sh | tr -d '\n')
  for s in $(seq 0 $((n - 1))); do
    i=${good[$s]}
    awk -v s=$s '$1 == s {$1 = ""; sub(/^ /, ""); print}' "$tmp/deal" > "$tmp/list.$i"
    list=$(base64 < "$tmp/list.$i" | tr -d '\n')
    envs="GATE_SKIPS=$(printf %q "$skips") GATE_ALLOWED=$(printf %q "$allowed") GATE_REVISION=$head GATE_SOURCE=$(sources "$full")"
    if [ "$i" = "$builder" ]; then envs="$envs GATE_BUILD=1"; else envs="$envs GATE_HEAD_DIR=../teq-head"; fi
    # the settings of the suites and their bound, as the gate was given them
    [ -z "$JOBS" ] || envs="$envs JOBS=$(printf %q "$JOBS")"
    [ -z "$GATE_SUITE_SECONDS" ] || envs="$envs GATE_SUITE_SECONDS=$(printf %q "$GATE_SUITE_SECONDS")"
    [ -z "$ref" ] || [ "$s" != "$ref_shard" ] || envs="$envs GATE_REF=$ref"
    "$ra/run.sh" "${handles[$i]}" teq -t 60 "rm -rf out/gate ../teq-head && mkdir -p out/gate && echo $runner | base64 -d > out/gate/shard.sh && echo $list | base64 -d > out/gate/suites && { $envs nohup bash out/gate/shard.sh out/gate/suites > out/gate/shard.out 2>&1 < /dev/null & }" > "$tmp/launch.$i" 2>&1 ||
      echo "gate: the shard on ${hosts[$i]} did not start: $(tail -1 "$tmp/launch.$i")"
    echo "$(clock) ${hosts[$i]}: $([ "$i" = "$builder" ] && echo "the head's build, then ")$(awk '{printf "%s%s", sep, $1; sep = " "}' "$tmp/list.$i")"
  done
  # Polled every 30 s (GATE_POLL_SECONDS) until GATE_MACHINE_SECONDS (default 2700) have passed since the shards
  # started, every call and wait bounded by what is left; a shard's machine is released as soon as it ends, the rest
  # at the deadline.
  # The builder's build, once it ends, goes to every other shard (the comparison's and the projects' machines take
  # it from $tmp themselves), or its failure has them build the head themselves.
  running=" ${good[*]} "
  deadline=$(($(now) + ${GATE_MACHINE_SECONDS:-2700}))
  for poll in $(seq 1 100000); do
    for i in $running; do
      left=$((deadline - $(now)))
      [ $left -gt 0 ] || break 2
      timeout $left "$ra/run.sh" "${handles[$i]}" teq -t $((left < 60 ? left : 60)) 'cat out/gate/status 2> /dev/null' > "$tmp/status.$i.new" 2> /dev/null || continue
      touch "$tmp/status.$i"
      tail -n +$(($(wc -l < "$tmp/status.$i") + 1)) "$tmp/status.$i.new" | while read -r name code secs last; do
        [ "$name" = done ] || printf '%s %-8s %-18s [%s] %5ss  %s\n' "$(clock)" "${hosts[$i]}" "$name" "$code" "$secs" "$(echo "$last" | cut -c1-100)"
      done
      if [ "$i" = "$builder" ] && ! grep -q '^build ' "$tmp/status.$i" && grep -q '^build ' "$tmp/status.$i.new"; then
        s=$(now)
        pids=()
        if grep -q '^build 0 ' "$tmp/status.$i.new" && take_head "$i"; then
          for j in $running; do
            [ "$j" = "$i" ] && continue
            { ship "${handles[$j]}" "$tmp/ship.$j" || echo "gate: the head did not reach ${hosts[$j]}: $(tail -1 "$tmp/ship.$j")"; } &
            pids+=($!)
          done
          [ ${#pids[@]} = 0 ] || wait "${pids[@]}"
          result ship "${hosts[$i]}" $(($(now) - s)) 0 "the head's binary ($(cat "$tmp/head.digest" | cut -c1-12)) from ${hosts[$i]} to $((n - 1)) shards"
        else
          touch "$tmp/head.failed"
          for j in $running; do
            [ "$j" = "$i" ] && continue
            "$ra/run.sh" "${handles[$j]}" teq -t 60 "mkdir -p ../teq-head && echo the builder ${hosts[$i]} did not build the head > ../teq-head/build-here" > "$tmp/ship.$j" 2>&1 &
            pids+=($!)
          done
          [ ${#pids[@]} = 0 ] || wait "${pids[@]}"
          result ship "${hosts[$i]}" $(($(now) - s)) 1 "no head's binary from the builder: $(tail -1 "$tmp/head.get" 2> /dev/null)"
        fi
      fi
      mv "$tmp/status.$i.new" "$tmp/status.$i"
      if [ "$(tail -1 "$tmp/status.$i")" = done ]; then
        running=${running/ $i / }
        echo "$(clock) ${hosts[$i]}: shard done after $(($(now) - machines_started))s"
        collect "$i"
      fi
    done
    [ -n "${running// /}" ] || break
    left=$((deadline - $(now)))
    [ $left -gt 0 ] || break
    sleep $((left < poll ? left : poll))
  done
  for i in $running; do collect "$i"; done
fi
[ -e "$tmp/head.ready" ] || touch "$tmp/head.failed"
touch "$tmp/shards.ended"
machines_ended=$(now)
for half in local compare projects; do
  pid_var=${half}_pid
  [ -n "${!pid_var}" ] || continue
  # A half that ended long before this point may be gone from bash's table of finished children
  # (`wait` then answers 127): its own exit file is the status then.
  wait "${!pid_var}"
  code=$?
  [ $code != 127 ] || code=$(cat "$out/$half.exit" 2> /dev/null || echo 1)
  eval "$pid_var="
  cat "$out/$half.results" >> "$results" 2> /dev/null
  [ $code = 0 ] || result $half-half $half 0 1 "the $half half ended with exit $code"
  case $half in
    local) lines_of="$local_lines $here_lines" ;;
    compare) lines_of=$compare ;;
    projects) lines_of=$project_lines ;;
  esac
  for name in $lines_of; do
    selected "$name" || continue
    [ $half != projects ] || [[ " $projects " == *" ${name%%-*} "* ]] || continue
    cut -f1 "$results" | grep -q -x -- "$name" || result "$name" $half 0 1 "no result: the $half half ended before it"
  done
done
gate_ended=$(now)
local_ended=$(cat "$out/local.ended" 2> /dev/null || echo "$local_started")
compare_ended=$(cat "$out/compare.ended" 2> /dev/null || echo "$compare_started")
projects_ended=$(cat "$out/projects.ended" 2> /dev/null || echo "$projects_started")

# The table: the claims and builds, the suites in tests/all.sh's order, the comparison, the projects' lines, the
# reference machine's lines.
status=0
recordable=1
printf '\n%-18s %-9s %6s  %-5s %s\n' line where secs exit result
order="pool claim sync collect release shard build ship"
for entry in "${machine[@]}"; do order="$order ${entry%% *}"; done
order="$order compare-half compare $compare projects-half projects $project_lines local-half $local_lines"
for name in $order; do
  while IFS=$'\t' read -r row where secs code line; do
    [ "$row" = "$name" ] || continue
    case $code in
      0) mark=ok ;;
      skip) mark=skip ;;
      3) mark="INCONCLUSIVE"; status=1; [[ " $compare " == *" $name "* ]] || recordable=0 ;;
      *) mark="FAIL"; status=1; recordable=0 ;;
    esac
    printf '%-18s %-9s %6s  %-5s %s\n' "$name" "$where" "$secs" "$mark" "$(echo "$line" | cut -c1-150)"
  done < "$results"
done
# The times of the suites and the comparison's lines are recorded from a run whose every line passed, or whose only
# lines not passed are timing lines inconclusive (none over), whose durations do not depend on their verdicts; from
# no other run.
if [ $record = 1 ] && [ $recordable = 1 ]; then
  python3 - "$results" tests/gate-times.txt "$(for entry in "${machine[@]}"; do echo "${entry%% *}"; done) $compare" "$compare" <<'PY'
import sys
recorded, timing = set(sys.argv[3].split()), set(sys.argv[4].split())
measured = {}
for l in open(sys.argv[1]):
    name, where, secs, code, _ = l.rstrip("\n").split("\t", 4)
    if name in recorded and where not in ("-", "local") and (code == "0" or (code == "3" and name in timing)):
        measured[name] = secs
lines = open(sys.argv[2]).read().splitlines()
out = [l if l.startswith("#") or not l.strip() or l.split()[0] not in measured else f"{l.split()[0]} {measured.pop(l.split()[0])}" for l in lines]
out += [f"{name} {secs}" for name, secs in measured.items()]
open(sys.argv[2], "w").write("\n".join(out) + "\n")
PY
  echo "gate: the times recorded in tests/gate-times.txt"
elif [ $record = 1 ]; then
  echo "gate: no times recorded: a line failed, or one other than a timing line was inconclusive"
fi
printf "machines' half %dm%02ds on %d machines, the comparison %dm%02ds, the projects %dm%02ds, the reference machine's half %dm%02ds; logs in %s\n" \
  $(((machines_ended - machines_started) / 60)) $(((machines_ended - machines_started) % 60)) "$n" \
  $(((compare_ended - compare_started) / 60)) $(((compare_ended - compare_started) % 60)) \
  $(((projects_ended - projects_started) / 60)) $(((projects_ended - projects_started) % 60)) \
  $(((local_ended - local_started) / 60)) $(((local_ended - local_started) % 60)) "${out#$root/}"
total=$((gate_ended - gate_started))
if [ $total -gt 1800 ]; then
  printf "gate: %dm%02ds from its first command to its last: FLAG, over the thirty minutes the owner's rule allows\n" $((total / 60)) $((total % 60))
else
  printf "gate: %dm%02ds from its first command to its last, within thirty minutes\n" $((total / 60)) $((total % 60))
fi
exit $status
