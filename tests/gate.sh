#!/bin/bash
# The landing gate: tests/gate.sh <worktree> [--master <binary>] [--master-rev <revision>] [--app <build.json>]
#   [--export-manifest <file>] [--only <name,...>] [--record]
#
# <worktree> holds the landing: a branch rebased onto master, committed (the gate refuses a tree with changes outside
# its head, since the machines get the tree and the reference machine's binary names the head), its release binary
# built (`cargo build --release`) when a reference machine's line is selected, whose `teq --version` has to name the
# head. What is timed runs on the remote machines (REMOTE_AGENT, the directory of the remote machines' scripts),
# and nothing waits for the reference machine to be quiet.
#
# The timing lines: one machine, claimed before the others, compares the head with master on itself
# (tests/gate-compare.sh, bench/pairs.py; docs/DEVELOPING.md, "The timing lines on a machine"): budget, the type
# phase of every program of the compile budget; runtime, the generated programs' wall and steady-state time; and
# sentinel, the rows of bench/reference.txt against its reference revision under their cumulative ceilings. Every
# side is a plain release build stamped with its revision: master's (the revision --master-rev names, by default
# the head's merge base with master) and the reference's are kept on each machine under ~/teq-bin by the hash of
# their sources and the toolchain, built there on first use; the head's is a shard's build copied over, or built
# there when no shard ran or its manifest is not master's build. With every machine busy the claim waits at most
# GATE_CLAIM_WAIT seconds (default 1200), after which the lines read "not run: claim timeout". The machine is
# released when its lines end, and on any failure.
#
# The correctness suites: each other free machine is claimed for one shard: the tree synced there, the release
# binary built there, the shard's suites run one after another as tests/all.sh runs them, the machine released as
# soon as its shard ends, and at the end or on a failure whatever is still held. The suites are dealt out to the
# machines by their times in tests/gate-times.txt, longest first, so the shards end together; --record rewrites
# those times from this run. split-determinism, when the tree has it, compares with master's binary (REF), built on
# its machine from master's sources, the revision --master names.
#
# Meanwhile on the reference machine, where the gate runs: tests/size.scala (its minified column is measured with its
# esbuild, which the remote machines do not have) and the instruction comparison against master's binary
# (bench/instr-compare.sh, five runs
# on the five programs depth_1, gview_1, core-only, realistic-api and realistic-frontend, each within 0.5%, and on
# derive30 and schema30 recorded apart; both binaries plain builds, the gate refuses a head's or master's whose
# `--version` ends in pgo or instrumented), whose counts of instructions need no quiet; and with --app, which names
# the application's export build file, the export's identity against master's saved build
# (bench/app/export-identity.sh), the API check against master's diagnostics (bench/app/api-check.sh, app-api) and the
# check of the API's test configuration over each binary's own build of the main lists' products, against master's
# the same way (api-check.sh --test, app-api-test), all reading the application's checkout and the jars its sbt build
# resolved into the reference machine's coursier cache (the lists bench/app/app-lists.sh writes: the API's module list and class
# path the files APP_MODULES and APP_CLASSPATH name, its flags APP_FLAGS, the test configuration's APP_TEST_MODULES,
# APP_TEST_CLASSPATH and APP_TEST_FLAGS, all kept outside the repository); with
# --export-manifest as well, the export's differences against the manifest of permitted ones
# (bench/app/export-accept.sh), beside the identity line, which still fails on any difference; and with --app, the
# application's modules built one against another's products and compared with the whole build, the Scala.js
# check chain and the JVM API chain (bench/app/chain.sh), which fails on a build's failure or a mismatch
# the list CHAIN_KNOWN names does not list (none when it is unset, which the line's result says); and with --app
# and APP_CYPRESS, the application's end-to-end suite run by that script over the binary's own build of the
# application (its services, its API, its served bundle), which fails on a failing spec (app-cypress; skipped
# without the script). One line runs only
# when --only names it, at a release and nightly: app-scalac, scalac 3.8.4 as the oracle of teq's diagnostics on the
# API's main and test lists (tests/scalac-oracle.sh, then bench/app/api-scalac-diff.sh with the lists' scalacOptions
# APP_SCALAC_OPTIONS and APP_TEST_SCALAC_OPTIONS, both into the attempt's directory), which fails on a difference
# the list SCALAC_KNOWN does not list or a line of it that names no difference (no rows when it is unset, which the
# line's result says). The paths of the lists and of CHAIN_KNOWN and SCALAC_KNOWN are read from the caller's
# directory. --only limits the gate to the named lines.
#
# Each run's logs go to a new attempt directory, out/gate/<head>/<attempt>/: a suite's <name>.log, each machine's
# build log, manifest, status and shard output, and under compare/ the comparison's logs, raw runs and manifests. A
# line per suite is printed as it ends, and one table at the end: every line with where it ran, its seconds and its
# exit, the seconds each claim waited for its machine, the comparison's stages, and the halves' times. The gate
# exits non-zero when any line fails or is inconclusive. A suite the tree does not have (workers and
# split-determinism before the parallel typer lands) is a skip, not a failure.
caller=$PWD
cd "$(dirname "$0")/.."
root=$PWD
ra=${REMOTE_AGENT:-}
usage() {
  echo "usage: tests/gate.sh <worktree> [--master <binary>] [--master-rev <revision>] [--app <build.json>] [--export-manifest <file>] [--only <name,...>] [--record]" >&2
  exit 2
}
absolute() { case $1 in /*) echo "$1" ;; *) echo "$caller/$1" ;; esac; }
wt=
master=
master_rev=
app=
manifest=
only=
record=0
while [ $# -gt 0 ]; do
  case $1 in
    --master) [ -n "$2" ] || usage; master=$(absolute "$2"); shift 2 ;;
    --master-rev) [ -n "$2" ] || usage; master_rev=$2; shift 2 ;;
    --pgo) echo "gate: --pgo is gone: the budget line compares plain builds of the head and master on a remote machine (docs/DEVELOPING.md, \"The timing lines on a machine\")" >&2; exit 2 ;;
    --app) [ -n "$2" ] || usage; app=$(absolute "$2"); shift 2 ;;
    --export-manifest) [ -n "$2" ] || usage; manifest=$(absolute "$2"); shift 2 ;;
    --only) [ -n "$2" ] || usage; only=" ${2//,/ } "; shift 2 ;;
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
  "watch-memory ./tests/watch-memory.sh"
  "tasty ./tests/tasty.sh"
  "modules ./tests/modules.sh"
  "tasty-exec ./teq interp tests/tasty-exec.scala"
  "analysis-bytes ./tests/analysis-bytes.sh"
  "classfile ./teq interp tests/classfile.scala"
  "classpath ./tests/classpath.sh"
  "stdlib ./tests/run_stdlib.sh"
  "app ./tests/app.sh"
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
)
compare="budget runtime sentinel"
local_lines="size instructions instructions-derive app-export app-export-accept app-api app-api-test app-chain app-cypress app-scalac"
# The lines run only when --only names them: the scalac oracle, at a release and nightly.
optional="app-scalac"
names=" $compare $local_lines "
for entry in "${machine[@]}"; do names="$names${entry%% *} "; done
for name in $only; do
  [[ "$names" == *" $name "* ]] || { echo "gate: no line named $name (the lines:$names)" >&2; exit 2; }
done
selected() { if [ -z "$only" ]; then [[ " $optional " != *" $1 "* ]]; else [[ "$only" == *" $1 "* ]]; fi; }
# A suite that skips for want of something the machine or the reference machine should hold (a jar, node, javap, the scala3
# checkout, esbuild) says so in one of these lines and exits 0; the gate fails it. The tests whose expectations come
# from Scala.js are skipped by design on the JVM and in the interpreter, and so is a memory session whose budget is
# another platform's alone (tests/watch-memory.sh).
skips='^skip|skipped, |counted as passed|not in the coursier cache|esbuild not found'
allowed='^skipped \((expectations from Scala\.js|a budget of one platform)\)'
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
if [ $remote_selected = 1 ] && [ ! -x "$ra/run.sh" ]; then
  echo "gate: set REMOTE_AGENT to the directory of the remote machines' scripts (claim.sh, run.sh, sync.sh, get.sh, release.sh)" >&2
  exit 2
fi
if [ $local_selected = 1 ]; then
  version=$("$bin" --version 2> /dev/null)
  hash=$(echo "$version" | awk '{print $3}')
  if [ -z "$hash" ] || [[ "$full" != "$hash"* ]]; then
    echo "gate: $bin is ${version:-missing}, not the head $head: cargo build --release in $wt" >&2
    exit 2
  fi
fi
mfull=
if [ -n "$master" ]; then
  mversion=$("$master" --version 2> /dev/null) || { echo "gate: $master does not run" >&2; exit 2; }
  mhash=$(echo "$mversion" | awk '{print $3}')
  if selected instructions || selected instructions-derive; then
    for checked in "$bin|$version" "$master|$mversion"; do
      [ -z "$(kind "${checked#*|}")" ] || { echo "gate: ${checked%%|*} is ${checked#*|}, not a plain build; the instruction comparison is of two plain builds (cargo build --release)" >&2; exit 2; }
    done
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
tmp=$(mktemp -d)
results=$out/results
: > "$results"
# result <name> <where> <seconds> <exit or skip> <line>
result() { printf '%s\t%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$4" "$5" >> "${6:-$results}"; }
echo "gate: $head $(git -C "$wt" log -1 --format=%s "$full" | cut -c1-80)${master:+, master $mhash}${crev:+, timing lines against master $crev and reference $refrev}; logs in ${out#$root/}"

# A claim is recorded in $tmp/claim.<machine> (a shard's) or $tmp/cclaim.<machine> (the comparison's) as soon as
# claim.sh returns its handle, and its release in $tmp/released.<handle>; the exit releases every recorded claim
# not yet released, and a release that fails, there or earlier, fails the gate with the handle named.
claimers=()
local_pid=
compare_pid=
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
  if [ -n "$compare_pid" ] && kill -0 "$compare_pid" 2> /dev/null; then
    # No claim starts once $tmp/stop is there, and a claim in flight ends first (claim.sh bounds itself): one
    # stopped halfway could leave a machine held under a handle the gate never learns. The comparison announces a
    # claim before it looks for the stop, so a claim either sees the stop or is seen here.
    touch "$tmp/stop"
    for i in $(seq 1 200); do [ -e "$tmp/compare.claiming" ] || break; sleep 3; done
    kill_tree "$compare_pid"
  fi
  [ ${#claimers[@]} = 0 ] || echo "gate: waiting for ${#claimers[@]} claims in flight before releasing" >&2
  for p in "${claimers[@]}"; do wait "$p" 2> /dev/null; done
  for f in "$tmp"/claim.* "$tmp"/cclaim.*; do
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
local_half() {
  local file=$out/local.results s code
  # the minified column of the size budgets is esbuild's output, measured with this machine's esbuild
  if selected size; then
    s=$(now)
    (cd "$wt" && TEQ=$bin timeout 600 ./teq interp tests/size.scala) > "$out/size.log" 2>&1
    code=$?
    [ $code != 0 ] || [ -z "$(lacking "$out/size.log")" ] || { echo "gate: a skip: $(lacking "$out/size.log")" >> "$out/size.log"; code=1; }
    result size local $(($(now) - s)) $code "$(tail -1 "$out/size.log")" "$file"
  fi
  if selected instructions; then
    if [ -z "$master" ]; then
      result instructions local 0 skip "no --master" "$file"
    else
      s=$(now)
      timeout 1500 "$root/bench/instr-compare.sh" "$master" "$bin" 5 depth_1 gview_1 core-only realistic-api realistic-frontend > "$out/instructions.log" 2>&1
      code=$?
      result instructions local $(($(now) - s)) $code "$(tail -1 "$out/instructions.log")" "$file"
    fi
  fi
  if selected instructions-derive; then
    if [ -z "$master" ]; then
      result instructions-derive local 0 skip "no --master" "$file"
    else
      s=$(now)
      INSTR_TOLERANCE=none timeout 900 "$root/bench/instr-compare.sh" "$master" "$bin" 5 derive30 schema30 > "$out/instructions-derive.log" 2>&1
      code=$?
      result instructions-derive local $(($(now) - s)) $code "$(tail -1 "$out/instructions-derive.log")" "$file"
    fi
  fi
  if selected app-export; then
    if [ -z "$app" ] || [ -z "$master" ]; then
      result app-export local 0 skip "needs --app and --master" "$file"
    else
      s=$(now)
      timeout 900 "$root/bench/app/export-identity.sh" "$app" "$bin" "$master" "$out/app-export" > "$out/app-export.log" 2>&1
      code=$?
      result app-export local $(($(now) - s)) $code "$(tail -1 "$out/app-export.log")" "$file"
    fi
  fi
  if selected app-export-accept; then
    if [ -z "$manifest" ] && [ -n "$app" ] && [ -n "$master" ] && grep -q "identical to master's build" "$out/app-export.log" 2>/dev/null; then
      result app-export-accept local 0 0 "no manifest: the export is identical to master's" "$file"
    elif [ -z "$app" ] || [ -z "$master" ] || [ -z "$manifest" ]; then
      result app-export-accept local 0 skip "needs --app, --master and --export-manifest" "$file"
    else
      s=$(now)
      timeout 900 "$root/bench/app/export-accept.sh" "$manifest" "$app" "$bin" "$master" "$out/app-export-accept" > "$out/app-export-accept.log" 2>&1
      code=$?
      result app-export-accept local $(($(now) - s)) $code "$(tail -1 "$out/app-export-accept.log")" "$file"
    fi
  fi
  if selected app-api; then
    if [ -z "$app" ] || [ -z "$master" ]; then
      result app-api local 0 skip "needs --app and --master" "$file"
    else
      s=$(now)
      APP_ROOT=$(app_root) timeout 600 "$root/bench/app/api-check.sh" "$bin" "$master" > "$out/app-api.log" 2>&1
      code=$?
      result app-api local $(($(now) - s)) $code "$(tail -1 "$out/app-api.log")" "$file"
    fi
  fi
  if selected app-api-test; then
    if [ -z "$app" ] || [ -z "$master" ]; then
      result app-api-test local 0 skip "needs --app and --master" "$file"
    else
      s=$(now)
      APP_ROOT=$(app_root) timeout 600 "$root/bench/app/api-check.sh" --test "$bin" "$master" > "$out/app-api-test.log" 2>&1
      code=$?
      result app-api-test local $(($(now) - s)) $code "$(tail -1 "$out/app-api-test.log")" "$file"
    fi
  fi
  if selected app-chain; then
    if [ -z "$app" ]; then
      result app-chain local 0 skip "needs --app" "$file"
    else
      s=$(now)
      code=0
      for side in frontend api; do
        timeout 600 "$root/bench/app/chain.sh" "$app" "$bin" "$out/app-chain/$side" $side > "$out/app-chain-$side.log" 2>&1 || code=1
      done
      result app-chain local $(($(now) - s)) $code "$(chain_counts "$out/app-chain-frontend.log" "$out/app-chain-api.log")$([ -n "$CHAIN_KNOWN" ] || echo "; no CHAIN_KNOWN, no known rows")" "$file"
    fi
  fi
  if selected app-cypress; then
    if [ -z "$app" ] || [ -z "${APP_CYPRESS:-}" ]; then
      result app-cypress local 0 skip "needs --app and APP_CYPRESS, the application's end-to-end suite script" "$file"
    else
      s=$(now)
      timeout 7200 "$APP_CYPRESS" "$bin" "$out/app-cypress" > "$out/app-cypress.log" 2>&1
      code=$?
      result app-cypress local $(($(now) - s)) $code "$(tail -1 "$out/app-cypress.log")" "$file"
    fi
  fi
  if selected app-scalac; then
    if [ -z "$app" ]; then
      result app-scalac local 0 skip "needs --app" "$file"
    else
      s=$(now)
      { (cd "$wt" && TEQ=$bin timeout 900 ./tests/scalac-oracle.sh "$out/scalac-oracle") && \
        APP_ROOT=$(app_root) timeout 3600 "$root/bench/app/api-scalac-diff.sh" "$bin" "$out/app-scalac"; } > "$out/app-scalac.log" 2>&1
      code=$?
      [ $code != 0 ] || [ -z "$(lacking "$out/app-scalac.log")" ] || { echo "gate: a skip: $(lacking "$out/app-scalac.log")" >> "$out/app-scalac.log"; code=1; }
      result app-scalac local $(($(now) - s)) $code "$(tail -1 "$out/app-scalac.log")$([ -n "$SCALAC_KNOWN" ] || echo "; no SCALAC_KNOWN, no known rows")" "$file"
    fi
  fi
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
[ ${#want[@]} -gt 0 ] || touch "$tmp/shards.none" "$tmp/shards.asked"

# The comparison machine's half, in the background beside the shards: its results go to their own file. The shards
# hand it the head's binary through $tmp: the directory $tmp/teq-head (mirrored to the machine as the project
# teq-head) and $tmp/head.ready once it holds a shard's build, $tmp/head.failed when no shard built one;
# $tmp/shards.claimed when the shards hold machines, $tmp/shards.asked once their first claim is over,
# $tmp/shards.none when they will hold none.
compare_half() {
  local file=$out/compare.results s handle= host pool= answer waited i left deadline lines= name envs job refdir= reftree tree
  local from seen=0 answered=0 code secs last
  for name in $compare; do selected $name && lines="$lines $name"; done
  s=$(now)
  deadline=$((s + ${GATE_CLAIM_WAIT:-1200}))
  for i in $(seq 1 100000); do
    left=$((deadline - $(now)))
    answer=$(timeout $((left > 30 ? left : 30)) "$ra/pool.sh" 2>&1) && pool=$answer
    for host in $(echo "$pool" | awk '$2 == "free" {print $1}'); do
      touch "$tmp/compare.claiming"
      [ ! -e "$tmp/stop" ] || { rm -f "$tmp/compare.claiming"; break 2; }
      handle=$("$ra/claim.sh" "gate-$head" "$host" 2> "$tmp/claim-err.compare") && echo "$handle $(($(now) - s))" > "$tmp/cclaim.$host"
      rm -f "$tmp/compare.claiming"
      [ -z "$handle" ] || break
    done
    touch "$tmp/compare.asked"
    [ -z "$handle" ] || break
    left=$((deadline - $(now)))
    [ $left -gt 0 ] || break
    sleep $((left < 30 ? left : 30))
  done
  waited=$(($(now) - s))
  if [ -z "$handle" ]; then
    pool=$(echo "$pool" | awk '{printf "%s%s", sep, $0; sep = "; "}' | tr -s ' ')
    for name in $lines; do result $name - $waited 1 "not run: claim timeout after ${waited}s (GATE_CLAIM_WAIT); the pool at the last ask: $pool" "$file"; done
    now > "$out/compare.ended"
    return
  fi
  host=${handle%%/*}
  result compare "$host" $waited 0 "claim: waited ${waited}s for the comparison machine, $handle" "$file"
  echo "$(clock) comparison: $host for$lines"
  # master's sources and the reference's, this gate's own clones mirrored as projects of their own
  git clone -q --shared --no-checkout "$(git -C "$wt" rev-parse --path-format=absolute --git-common-dir)" "$tmp/compare/teq-master" &&
    git -C "$tmp/compare/teq-master" checkout -q -f --detach "$crev" || { result compare "$host" 0 1 "sync: no clone of master at $crev" "$file"; release "$handle"; return; }
  reftree=../teq-master
  if [ "$(sources "$refrev")" != "$(sources "$crev")" ]; then
    refdir=$tmp/compare/teq-reference
    git clone -q --shared --no-checkout "$(git -C "$wt" rev-parse --path-format=absolute --git-common-dir)" "$refdir" &&
      git -C "$refdir" checkout -q -f --detach "$refrev" || { result compare "$host" 0 1 "sync: no clone of the reference at $refrev" "$file"; release "$handle"; return; }
    reftree=../teq-reference
  fi
  i=$(now)
  for tree in "$wt" "$tmp/compare/teq-master" $refdir; do
    "$ra/sync.sh" "$handle" "$tree" >> "$tmp/compare.sync" 2>&1 || { result compare "$host" $(($(now) - i)) 1 "sync: $(tail -1 "$tmp/compare.sync")" "$file"; release "$handle"; return; }
  done
  result compare "$host" $(($(now) - i)) 0 "sync: the head, master ($crev) and the reference ($refrev) mirrored" "$file"
  # the head's origin is decided once the shards' first claim is over
  for i in $(seq 1 120); do [ -e "$tmp/shards.asked" ] && break; sleep 5; done
  from=
  [ -e "$tmp/shards.none" ] && from=here
  [ -e "$tmp/shards.asked" ] && [ ! -e "$tmp/shards.claimed" ] && from=here
  job=$(base64 < tests/gate-compare.sh | tr -d '\n')
  envs="GATE_HEAD=$head GATE_HEAD_SOURCE=$(sources "$full") GATE_MASTER=$crev GATE_MASTER_SOURCE=$(sources "$crev") GATE_MASTER_TREE=../teq-master"
  envs="$envs GATE_REFERENCE=$refrev GATE_REFERENCE_SOURCE=$(sources "$refrev") GATE_REFERENCE_TREE=$reftree GATE_LINES=$(printf %q "$lines")"
  [ -z "$from" ] || envs="$envs GATE_HEAD_FROM=here"
  "$ra/run.sh" "$handle" teq -t 60 "rm -rf out/compare && mkdir -p out/compare && echo $job | base64 -d > out/compare/gate-compare.sh && { $envs nohup bash out/compare/gate-compare.sh > out/compare/job.out 2>&1 < /dev/null & }" > "$tmp/compare.launch" 2>&1 ||
    { result compare "$host" 0 1 "the job did not start: $(tail -1 "$tmp/compare.launch")" "$file"; release "$handle"; return; }
  # Polled every 15 s until GATE_COMPARE_SECONDS (default 3600) have passed since the job started.
  deadline=$(($(now) + ${GATE_COMPARE_SECONDS:-3600}))
  touch "$tmp/compare.status"
  for i in $(seq 1 100000); do
    left=$((deadline - $(now)))
    [ $left -gt 0 ] || break
    if timeout $left "$ra/run.sh" "$handle" teq -t $((left < 60 ? left : 60)) 'cat out/compare/status 2> /dev/null' > "$tmp/compare.status.new" 2> /dev/null; then
      tail -n +$((seen + 1)) "$tmp/compare.status.new" | while read -r name code secs last; do
        [ "$name" = done ] || printf '%s %-8s %-18s [%s] %5ss  %s\n' "$(clock)" "$host" "$name" "$code" "$secs" "$(echo "$last" | cut -c1-100)"
      done
      seen=$(wc -l < "$tmp/compare.status.new")
      mv "$tmp/compare.status.new" "$tmp/compare.status"
      [ "$(tail -1 "$tmp/compare.status")" = done ] && break
      if [ $answered = 0 ] && grep -q '^head-wanted ' "$tmp/compare.status"; then
        if [ -e "$tmp/head.ready" ]; then
          "$ra/sync.sh" "$handle" "$tmp/teq-head" > "$tmp/compare.head" 2>&1 &&
            "$ra/run.sh" "$handle" teq -t 60 "echo $(cat "$tmp/head.from") > out/compare/head-ready" >> "$tmp/compare.head" 2>&1 && answered=1
        elif [ -e "$tmp/head.failed" ] || [ -e "$tmp/shards.none" ]; then
          "$ra/run.sh" "$handle" teq -t 60 "echo no shard built the head > out/compare/build-head" > "$tmp/compare.head" 2>&1 && answered=1
        fi
      fi
    fi
    left=$((deadline - $(now)))
    [ $left -gt 0 ] && sleep $((left < 15 ? left : 15))
  done
  mkdir -p "$out/compare"
  "$ra/get.sh" "$handle" teq out/compare/logs "$out/compare" > "$tmp/compare.get" 2>&1 || result compare "$host" 0 1 "collect: the comparison's logs did not come back: $(tail -1 "$tmp/compare.get")" "$file"
  release "$handle" || result compare "$host" 0 1 "release: $handle not released: $(tail -1 "$tmp/release.$(mark "$handle")")" "$file"
  cp "$tmp/compare.status" "$out/compare/status"
  while read -r name code secs last; do
    case $name in
      done | head-wanted) ;;
      master | reference | head) result compare "$host" "$secs" "$code" "$name: $last" "$file" ;;
      *) result "$name" "$host" "$secs" "$code" "$last" "$file" ;;
    esac
  done < "$tmp/compare.status"
  [ "$(tail -1 "$tmp/compare.status")" = done ] || result compare "$host" $(($(now) - s)) 1 "the job did not end within GATE_COMPARE_SECONDS" "$file"
  now > "$out/compare.ended"
}
compare_started=$(now)
if [ $compare_selected = 1 ]; then
  { compare_half; rc=$?; echo "$rc" > "$out/compare.exit"; exit "$rc"; } &
  compare_pid=$!
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
        *) cp "$f" "$out/$b" ;;
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
# take_head <i>: shard i's build into $tmp/teq-head for the comparison machine, once.
take_head() {
  local i=$1
  [ $compare_selected = 1 ] && [ ! -e "$tmp/head.ready" ] || return 0
  rm -rf "$tmp/teq-head" && mkdir -p "$tmp/teq-head" && git -C "$tmp/teq-head" init -q &&
    "$ra/get.sh" "${handles[$i]}" teq target/release/teq "$tmp/teq-head/teq" > "$tmp/head.get" 2>&1 &&
    "$ra/get.sh" "${handles[$i]}" teq out/gate/build.manifest "$tmp/teq-head/manifest" >> "$tmp/head.get" 2>&1 &&
    echo "${hosts[$i]}" > "$tmp/head.from" && touch "$tmp/head.ready"
}
machines_started=$(now)
hosts=()
handles=()
if [ ${#want[@]} -gt 0 ]; then
  # The comparison machine is claimed first: the shards ask once its first ask is over, and leave it the first free
  # machine of every ask until it holds one.
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
    # until the comparison holds its machine, the first free one is left to it
    free=$(echo "$pool" | awk '$2 == "free" {print $1}')
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
    sleep $((left < 30 ? left : 30))
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
  for entry in "${want[@]}"; do echo "$entry"; done > "$tmp/want"
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
  if [ -n "$mfull" ] && [ "$mfull" != "$full" ] && grep -q ' split-determinism ' "$tmp/deal"; then
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
  runner=$(base64 < tests/gate-shard.sh | tr -d '\n')
  for s in $(seq 0 $((n - 1))); do
    i=${good[$s]}
    awk -v s=$s '$1 == s {$1 = ""; sub(/^ /, ""); print}' "$tmp/deal" > "$tmp/list.$i"
    list=$(base64 < "$tmp/list.$i" | tr -d '\n')
    envs="GATE_SKIPS=$(printf %q "$skips") GATE_ALLOWED=$(printf %q "$allowed") GATE_REVISION=$head GATE_SOURCE=$(sources "$full")"
    # the settings of the suites and their bound, as the gate was given them
    [ -z "$JOBS" ] || envs="$envs JOBS=$(printf %q "$JOBS")"
    [ -z "$GATE_SUITE_SECONDS" ] || envs="$envs GATE_SUITE_SECONDS=$(printf %q "$GATE_SUITE_SECONDS")"
    [ -z "$ref" ] || [ "$s" != "$ref_shard" ] || envs="$envs GATE_REF=$ref"
    "$ra/run.sh" "${handles[$i]}" teq -t 60 "rm -rf out/gate && mkdir -p out/gate && echo $runner | base64 -d > out/gate/shard.sh && echo $list | base64 -d > out/gate/suites && { $envs nohup bash out/gate/shard.sh out/gate/suites > out/gate/shard.out 2>&1 < /dev/null & }" > "$tmp/launch.$i" 2>&1 ||
      echo "gate: the shard on ${hosts[$i]} did not start: $(tail -1 "$tmp/launch.$i")"
    echo "$(clock) ${hosts[$i]}: $(awk '{printf "%s%s", sep, $1; sep = " "}' "$tmp/list.$i")"
  done
  # Polled every 30 s until GATE_MACHINE_SECONDS (default 2700) have passed since the shards started, every call
  # and wait bounded by what is left; a shard's machine is released as soon as it ends, the rest at the deadline.
  # The first shard's build that succeeds is taken for the comparison machine.
  running=" ${good[*]} "
  failed_builds=0
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
      if ! grep -q '^build ' "$tmp/status.$i" && grep -q '^build ' "$tmp/status.$i.new"; then
        if grep -q '^build 0 ' "$tmp/status.$i.new"; then
          take_head "$i"
        else
          failed_builds=$((failed_builds + 1))
          [ $failed_builds -lt $n ] || touch "$tmp/head.failed"
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
    sleep $((left < 30 ? left : 30))
  done
  for i in $running; do collect "$i"; done
fi
[ -e "$tmp/head.ready" ] || touch "$tmp/head.failed"
machines_ended=$(now)
for half in local compare; do
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
  lines_of=$local_lines
  [ $half = local ] || lines_of=$compare
  for name in $lines_of; do
    selected "$name" || continue
    cut -f1 "$results" | grep -q -x -- "$name" || result "$name" $half 0 1 "no result: the $half half ended before it"
  done
done
local_ended=$(cat "$out/local.ended" 2> /dev/null || echo "$local_started")
compare_ended=$(cat "$out/compare.ended" 2> /dev/null || echo "$compare_started")

if [ $record = 1 ]; then
  python3 - "$results" tests/gate-times.txt "$names" "$compare" <<'PY'
import sys
suites = set(sys.argv[3].split()) - set(sys.argv[4].split())
measured = {}
for l in open(sys.argv[1]):
    name, where, secs, code, _ = l.rstrip("\n").split("\t", 4)
    if name in suites and where not in ("-", "local") and code == "0":
        measured[name] = secs
lines = open(sys.argv[2]).read().splitlines()
out = [l if l.startswith("#") or not l.strip() or l.split()[0] not in measured else f"{l.split()[0]} {measured.pop(l.split()[0])}" for l in lines]
out += [f"{name} {secs}" for name, secs in measured.items()]
open(sys.argv[2], "w").write("\n".join(out) + "\n")
PY
  echo "gate: the times recorded in tests/gate-times.txt"
fi

# The table: the claims and builds, the suites in tests/all.sh's order, the comparison, the reference machine's lines.
status=0
printf '\n%-18s %-9s %6s  %-5s %s\n' line where secs exit result
order="pool claim sync collect release shard build"
for entry in "${machine[@]}"; do order="$order ${entry%% *}"; done
order="$order compare-half compare $compare local-half $local_lines"
for name in $order; do
  while IFS=$'\t' read -r row where secs code line; do
    [ "$row" = "$name" ] || continue
    case $code in
      0) mark=ok ;;
      skip) mark=skip ;;
      3) mark="INCONCLUSIVE"; status=1 ;;
      *) mark="FAIL"; status=1 ;;
    esac
    printf '%-18s %-9s %6s  %-5s %s\n' "$name" "$where" "$secs" "$mark" "$(echo "$line" | cut -c1-150)"
  done < "$results"
done
printf "machines' half %dm%02ds on %d machines, the comparison %dm%02ds, the reference machine's half %dm%02ds; logs in %s\n" \
  $(((machines_ended - machines_started) / 60)) $(((machines_ended - machines_started) % 60)) "$n" \
  $(((compare_ended - compare_started) / 60)) $(((compare_ended - compare_started) % 60)) \
  $(((local_ended - local_started) / 60)) $(((local_ended - local_started) % 60)) "${out#$root/}"
exit $status
