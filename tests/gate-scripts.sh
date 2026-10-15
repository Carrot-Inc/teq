#!/bin/bash
# Checks of the gate's harnesses' own decisions, each against a stand-in that fails as a loaded or broken machine
# fails (docs/DEVELOPING.md, "The landing gate"):
#   oracle   tests/modules.sh over its misuse case with scala-cli a stand-in that exits as a timeout (124), is
#            killed (137) or exits 1 before scalac reports (a dependency not resolved offline): each fails the
#            case's negative check with the status in its detail; scalac 3.8.4 itself refuses it (the control).
#   workers  tests/modules.sh over two cases at two at once, the first case's worker killed by its first build: the
#            case fails as one whose worker ended without its counts, the other's lines and counts still printed in
#            the cases' order, and the run exits 1.
#   parts    tests/tasty-exec.scala over six programs of its closure with a java whose first part of the
#            regenerations is killed at once: the suite fails, naming the part, its status and its stderr, and keeps
#            its work directory.
#   nodes    the runtime line's node and java are bench/runtime-machine.txt's by their recorded paths: with another
#            node first on the PATH, bench/pairs.py's preflight takes the pinned one; a pinned path with nothing
#            there refuses the line (exit 2), and --unpinned takes the PATH's; tests/asymptotics.sh runs the node
#            NODE names and says which.
#   skips    tests/gate.sh's want-of rule over each runner's wording: the by-design skips admitted (Scala.js's
#            expectations, another system's, another std's, a budget of one platform, the JVM's platform, a split
#            watch case of another std or of the JVM alone), the want-of ones refused (a jar, a tool, a note of a
#            check a tool's absence leaves undone).
#   driver   tests/gate.sh over stand-in machines (tests/gate-agent; cargo and rustc stand-ins whose build writes a
#            binary naming the revision asked for and running TEQ), on a copy of this tree in a repository of its
#            own: the first shard's machine builds the head and the others take its binary, one whose
#            copy is spoilt refuses it and builds; the machine whose pool line holds lila is left to the project
#            lines, which fail there without the checkout, and fail as not run without such a machine; --record
#            leaves tests/gate-times.txt as it was after a failing run and records the suites' times after a passing
#            one; the last line is the gate's total; and with five machines the comparison (bench/pairs.py a stand-in
#            in the landing) prepares three, each over the builder's binary, and measures its lines after the shards'
#            suites end on one of them, on three when the recorded times put one machine's end past thirty minutes; a
#            run whose only line not passed is a timing line inconclusive records the times.
# Every run is bounded; the script exits 1 on a failure. Needs the release binary (TEQ), scala-cli and the coursier
# cache the suites read.
cd "$(dirname "$0")/.." || exit 1
export TEQ=${TEQ:-$PWD/target/release/teq}
work=$(mktemp -d "${TMPDIR:-/tmp}/gate-scripts.XXXXXX") || exit 1
trap 'rm -rf "$work"' EXIT
pass=0
fail=0
# check <name> <log> <command...>: the command's success counts; a failure prints the log's last lines.
check() {
  local name=$1 log=$2
  shift 2
  if "$@"; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL $name"
    [ -f "$log" ] && tail -15 "$log" | sed 's/^/  /'
  fi
}
for tool in scala-cli java node python3; do
  command -v $tool > /dev/null || { echo "gate-scripts: no $tool (incomplete validation)"; echo "gate-scripts: 0 passed, 1 failed"; exit 1; }
done
[ -x "$TEQ" ] || { echo "gate-scripts: no binary at $TEQ"; echo "gate-scripts: 0 passed, 1 failed"; exit 1; }

# stand_in <dir> <body>: a scala-cli in <dir> that runs <body> instead of scala-cli.
stand_in() {
  mkdir -p "$1"
  printf '#!/bin/bash\n%s\n' "$2" > "$1/scala-cli"
  chmod +x "$1/scala-cli"
}

# The module oracle: a negative check passes on scalac's refusal alone.
stand_in "$work/timeout" 'echo "stand-in: the oracle ran out of time" >&2; exit 124'
stand_in "$work/killed" 'kill -9 $$'
stand_in "$work/resolution" 'echo "Error: Could not resolve org.scala-lang:scala3-compiler_3:3.8.4 offline" >&2; exit 1'
for kind in timeout:124 killed:137 resolution:1; do
  k=${kind%%:*}
  PATH=$work/$k:$PATH timeout 300 tests/modules.sh misuse > "$work/oracle-$k.log" 2>&1
  code=$?
  check "oracle: a stand-in scala-cli's $k (exit ${kind#*:}) refuses the misuse case" "$work/oracle-$k.log" \
    eval '[ $code = 1 ] && grep -q "^FAIL misuse scalac-b$" "$work/oracle-$k.log" && grep -q "^the oracle did not refuse b as scalac: scalac exit ${kind#*:}" "$work/oracle-$k.log" && grep -q "refused 0 misuses" "$work/oracle-$k.log"'
done
timeout 300 tests/modules.sh misuse > "$work/oracle-scalac.log" 2>&1
code=$?
check "oracle: scalac 3.8.4 refuses the misuse case" "$work/oracle-scalac.log" \
  eval '[ $code = 0 ] && grep -q "refused 1 misuses as expected" "$work/oracle-scalac.log"'

# A worker that dies is a failure of its case, never a case dropped.
mkdir -p "$work/killer"
printf '#!/bin/bash\ncase " $* " in *" tests/modules/enumorder/"*) kill -9 $PPID; exit 1 ;; esac\nexec "%s" "$@"\n' "$TEQ" > "$work/killer/teq"
chmod +x "$work/killer/teq"
JOBS=2 TEQ=$work/killer/teq timeout 300 tests/modules.sh enumorder misuse > "$work/workers.log" 2>&1
code=$?
check "workers: a killed worker fails its case and the run" "$work/workers.log" \
  eval '[ $code = 1 ] && [ "$(head -1 "$work/workers.log")" = "FAIL enumorder worker: the case'"'"'s worker ended without its counts (exit 137)" ] && grep -q "^modules: 2 passed, 1 failed, .* refused 1 misuses" "$work/workers.log"'

# A part of tasty-exec's regenerations that dies is reported with its status and stderr.
mkdir -p "$work/parts"
printf '#!/bin/bash\ncase " $* " in *" gate part.0 "*) echo "stand-in: part.0 killed" >&2; kill -9 $$ ;; esac\nexec "%s" "$@"\n' "$(command -v java)" > "$work/parts/java"
chmod +x "$work/parts/java"
PATH=$work/parts:$PATH timeout 600 ./teq interp tests/tasty-exec.scala -- $(grep -v '^#' tests/tasty/exec/closure.txt | head -6) > "$work/parts.log" 2>&1
code=$?
check "parts: a killed part of the regenerations fails the suite with its status and stderr" "$work/parts.log" eval '[ $code = 1 ] &&
  grep -q "^FAIL part 0 of [0-9]*: exit 137, killed by signal 9, after [0-9]* s, 0 of [0-9]* jobs answered; kept in " "$work/parts.log" &&
  grep -q "^  stderr: stand-in: part.0 killed$" "$work/parts.log" && grep -q "no answer from part 0" "$work/parts.log"'
kept=$(sed -n 's/^FAIL part 0 of [0-9]*: .*; kept in //p' "$work/parts.log")
case $kept in */tasty-exec.*) rm -rf "$kept" ;; esac

# The suites that need one node name it, whatever the PATH's order.
mkdir -p "$work/nodes/path" "$work/nodes/pinned"
printf '#!/bin/sh\necho v22.0.0-stand-in\n' > "$work/nodes/path/node"
printf '#!/bin/sh\necho v25.9.0-stand-in\n' > "$work/nodes/pinned/node"
chmod +x "$work/nodes/path/node" "$work/nodes/pinned/node"
PATH=$work/nodes/path:$PATH python3 - "$work/nodes" > "$work/nodes.log" 2>&1 <<'PY'
import os, subprocess, sys
sys.path.insert(0, "bench")
import pairs
d = sys.argv[1]
java = pairs.tool_identity("java")
pinned = os.path.join(d, "pinned", "node")
open(os.path.join(d, "pin"), "w").write(f"node {pinned} v25.9.0-stand-in\n{java}\n")
open(os.path.join(d, "gone"), "w").write(f"node {os.path.join(d, 'gone', 'node')} v25.9.0-stand-in\n{java}\n")
got = pairs.check_runtimes({"unpinned": False}, os.path.join(d, "pin"))
assert got == [f"node {pinned} v25.9.0-stand-in", java], got
got = pairs.check_runtimes({"unpinned": True}, os.path.join(d, "gone"))
assert got[0] == f"node {os.path.join(d, 'path', 'node')} v22.0.0-stand-in", got
try:
    pairs.check_runtimes({"unpinned": False}, os.path.join(d, "gone"))
    raise AssertionError("a pinned node with nothing at its path was taken")
except SystemExit as e:
    assert e.code == 2, e.code
print("pin: ok")
PY
check "nodes: the runtime line runs the pinned node and java by their paths" "$work/nodes.log" grep -q "^pin: ok$" "$work/nodes.log"
printf '#!/bin/sh\n[ "$1" = --version ] && { echo v0-stand-in; exit 0; }\ntouch "%s"\nexit 1\n' "$work/nodes/ran" > "$work/nodes/node0"
chmod +x "$work/nodes/node0"
NODE=$work/nodes/node0 ASYMPTOTICS_BASE=100 timeout 300 tests/asymptotics.sh > "$work/asymptotics.log" 2>&1
check "nodes: tests/asymptotics.sh runs the node NODE names" "$work/asymptotics.log" eval '[ -e "$work/nodes/ran" ] && tail -1 "$work/asymptotics.log" | grep -q "(node v0-stand-in)$"'

# The gate's want-of rule: what is skipped by design passes, what is skipped for want of something fails.
source <(sed -n '/^skips=/,/^lacking()/p' tests/gate.sh)
: > "$work/skips.log"
while IFS='|' read -r verdict line; do
  [ -n "$line" ] || continue
  printf '%s\n' "$line" > "$work/skip.txt"
  if [ -n "$(lacking "$work/skip.txt")" ]; then got=refused; else got=admitted; fi
  [ "$got" = "$verdict" ] || echo "$line: $got, not $verdict" >> "$work/skips.log"
done <<'WORDINGS'
admitted|skipped (expectations from Scala.js): alias_expected_shapes anon_js_object_undefined
admitted|skipped (another system's): windows_paths windows_process
admitted|skipped (another std's): anyval_extractor_view java_varargs_spread
admitted|skipped (a budget of one platform): a session
admitted|skip array_index_bounds: the JVM's platform
admitted|skip anyval_extractor_view: its std is scala-library
admitted|skip jvm_only_case: the JVM's alone
refused|skip x: not in the coursier cache: cats-core
refused|counted as passed, a jar of theirs not in the coursier cache: x
refused|stdlib: skipped, no scala-library 3.x jar in the coursier cache
refused|app: skipped, not in the coursier cache: cats-core
refused|analysis-bytes: skipped, no scala-library jar
refused|skip the jvm session: error: scala-library
refused|skip javap comparison: no javap or no ct.sym
refused|skip: scala-cli not found
refused|note: esbuild not found, the minified column is skipped
refused|task: note: no wine here, teq.cmd not run
refused|task: note: no shellcheck here, the launcher not linted
refused|task: note: no npm here, the jsdom run not checked
refused|runner-classes: note: javac 24.0.2's output differs from the committed classes, which javac 17 writes (not checked: run with JAVAC naming javac 17)
refused|runner-classes: note: no javac (javac), not checked: set JAVAC or JAVA_HOME
refused|skip the corpus: no /scala3/tests/warn
WORDINGS
check "skips: the want-of rule admits the by-design skips and refuses the others" "$work/skips.log" eval '[ ! -s "$work/skips.log" ]'

# The gate's driver over stand-in machines.
agent=$work/agent
mkdir -p "$agent/bin" "$agent/scala3/tests"
printf '%s\n' '#!/bin/bash' 'case $1 in' \
  '  -V) echo "cargo 1.0.0 (stand-in)" ;;' \
  '  build) dir=target; prev=; for a in "$@"; do [ "$prev" != --target-dir ] || dir=$a; prev=$a; done' \
  '    mkdir -p "$dir/release" && printf "#!/bin/bash\\n[ \"\$1\" != --version ] || { echo \"teq 0.1.8 %s\"; exit 0; }\\nexec \"%s\" \"\$@\"\\n" "$TEQ_REVISION" "$GATE_AGENT_TEQ" > "$dir/release/teq" && chmod +x "$dir/release/teq"' \
  '    echo "    Finished \`release\` profile [optimized] target(s) in 0.01s (stand-in)" ;;' \
  '  *) echo "cargo stand-in: no $1" >&2; exit 1 ;;' 'esac' > "$agent/bin/cargo"
printf '%s\n' '#!/bin/bash' 'printf "rustc 1.0.0 (stand-in)\nbinary: rustc\ncommit-hash: 0\nhost: x86_64-unknown-linux-gnu\nrelease: 1.0.0\nLLVM version: 0\n"' > "$agent/bin/rustc"
chmod +x "$agent/bin/cargo" "$agent/bin/rustc"
landing=$work/landing
scripts=$PWD/tests/gate-agent
# The landing: this tree's files in a repository of their own (the tree may be a mirror without one), the
# comparison's stand-in, and bench/reference.txt naming the landing's first commit, so that every revision the gate
# resolves is the landing's.
mkdir -p "$landing" && git -C "$landing" init -q &&
  { git ls-files -co --exclude-standard 2> /dev/null || find . -type f ! -path './out/*' ! -path './target/*' ! -path './.git/*'; } |
  tar -cf - -T - | tar -xf - -C "$landing" && cp tests/gate-agent/pairs.py "$landing/bench/pairs.py" &&
  git -C "$landing" add -A && git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -m "the tree" &&
  echo "reference $(git -C "$landing" rev-parse --short HEAD) 2026-10-14 ceiling 1.20 the landing's own" >> "$landing/bench/reference.txt" &&
  git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -a -m "the reference" ||
  { echo "FAIL driver: no landing in $landing"; fail=$((fail + 1)); }
# gate <log> <pool lines> <gate's arguments...>: the landing's gate over a fresh pool of stand-in machines.
gate() {
  local log=$1 pool=$2
  shift 2
  rm -rf "$agent/machines" "$agent/claims"
  printf '%s\n' "$pool" > "$agent/pool"
  for m in $(awk '{print $1}' "$agent/pool"); do mkdir -p "$agent/machines/$m"; echo "export SCALA3=$agent/scala3" > "$agent/machines/$m/env"; done
  # the gate's own settings and the machine contract's variables of this machine's shell are not the stand-ins'
  (cd "$landing" && for v in $(compgen -e | grep -E '^(GATE_|LILA_|APP_|CHAIN_KNOWN$|SCALAC_KNOWN$|NODE$)' | grep -v '^GATE_AGENT_'); do unset "$v"; done &&
    env -u JOBS REMOTE_AGENT="$scripts" GATE_AGENT_ROOT="$agent" GATE_AGENT_TEQ="$TEQ" GATE_POLL_SECONDS=2 \
    GATE_CLAIM_WAIT=6 GATE_CLAIM_SECONDS=6 GATE_HEAD_WAIT=120 timeout 300 tests/gate.sh "$landing" "$@") > "$log" 2>&1
}
row() { grep -E "^$1 +$2 " "$3"; }
times_before=$(cat "$landing/tests/gate-times.txt")
GATE_AGENT_CORRUPT=m3 gate "$work/driver-1.log" "$(printf 'm1 free\nm2 free\nm3 free\nh1 free holds=lila,airstream\nb1 busy')" --only parser,fold,planted,lila-export --record
code=$?
log=$work/driver-1.log
check "driver: the builder's head shipped, a spoilt copy refused, lila's line on its machine, nothing recorded" "$log" eval '[ $code = 1 ] &&
  row build m1 "$log" | grep -q "ok .*Finished" && row build m2 "$log" | grep -q "ok .*the head.s binary from m1 after .*s, its digest" &&
  row build m3 "$log" | grep -q "ok .*built here, the head.s binary from m1 refused: its digest is not the builder.s" && row build h1 "$log" | grep -q "ok .*the head.s binary from m1" &&
  row ship m1 "$log" | grep -q "ok .*from m1 to 2 shards" && row lila-export h1 "$log" | grep -q "FAIL .*needs GATE_LILA_CHECKOUT" &&
  row parser "m[123]" "$log" | grep -q " ok " && row fold "m[123]" "$log" | grep -q " ok " && row planted "m[123]" "$log" | grep -q " ok " &&
  ! grep -q "^claim  *h1 " "$log" && grep -q "^gate: no times recorded" "$log" && [ "$times_before" = "$(cat "$landing/tests/gate-times.txt")" ] &&
  grep -q "^gate: [0-9]*m[0-9]*s from its first command to its last, within thirty minutes" "$log"'
gate "$work/driver-2.log" "$(printf 'm1 free\nm2 free')" --only fold,lila-run
code=$?
log=$work/driver-2.log
check "driver: a project line without a machine holding its checkout fails as not run" "$log" eval '[ $code = 1 ] &&
  row lila-run - "$log" | grep -q "FAIL .*not run: no free machine holds lila" && row fold "m[12]" "$log" | grep -q " ok "'
gate "$work/driver-3.log" "$(printf 'm1 free\nm2 free')" --only parser,fold --record
code=$?
log=$work/driver-3.log
check "driver: a passing run records its suites' times" "$log" eval '[ $code = 0 ] && grep -q "^gate: the times recorded" "$log" &&
  grep -q -x "parser $(row parser "m[12]" "$log" | awk "{print \$3}")" "$landing/tests/gate-times.txt" &&
  grep -q -x "fold $(row fold "m[12]" "$log" | awk "{print \$3}")" "$landing/tests/gate-times.txt" &&
  [ "$(echo "$times_before" | grep -v -E "^(parser|fold) ")" = "$(grep -v -E "^(parser|fold) " "$landing/tests/gate-times.txt")" ]'
git -C "$landing" checkout -q tests/gate-times.txt
# The comparison: three machines prepared beside the suites; with the recorded times the lines measure on one of
# them, after the shards' suites; with times that one machine would end past thirty minutes, on as many as fit.
gate "$work/driver-4.log" "$(printf 'm1 free\nm2 free\nm3 free\nm4 free\nm5 free')" --only parser,fold,budget,runtime,sentinel --master-rev HEAD
code=$?
log=$work/driver-4.log
check "driver: the comparison prepared on three machines, measuring on one after the shards' suites, over the builder's binary" "$log" eval '[ $code = 0 ] &&
  [ "$(grep -E "^(budget|runtime|sentinel) " "$log" | grep -c " ok .*the stand-in of bench/pairs.py")" = 3 ] &&
  [ "$(grep -E "^(budget|runtime|sentinel) " "$log" | awk "{print \$2}" | sort -u | wc -l)" = 1 ] &&
  [ "$(grep -c -E "^compare .* ok +head: copied from " "$log")" = 3 ] && grep -q -E "^compare .* ok +measure: the lines on 1 of 3 machines ready" "$log" &&
  [ "$(grep -c -E "^compare .* ok +quiet: measured after the suites on the other machines.*here: *$" "$log")" = 2 ] &&
  [ "$(grep -n "shard done after" "$log" | tail -1 | cut -d: -f1)" -lt "$(grep -n -E "^[0-9:]+ m[0-9] +quiet " "$log" | head -1 | cut -d: -f1)" ]'
sed -i -E 's/^(budget|runtime|sentinel) [0-9]+$/\1 1500/' "$landing/tests/gate-times.txt"
echo budget > "$landing/bench/pairs-stand-in.txt"
git -C "$landing" add -A && git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -m "times past thirty minutes on one machine; budget inconclusive"
gate "$work/driver-5.log" "$(printf 'm1 free\nm2 free\nm3 free\nm4 free\nm5 free')" --only parser,fold,budget,runtime,sentinel --master-rev HEAD --record
code=$?
log=$work/driver-5.log
recorded=$(grep -E "^(parser|fold|runtime|sentinel|budget) " "$landing/tests/gate-times.txt" | sort | tr '\n' ' ')
git -C "$landing" reset -q --hard HEAD~1
check "driver: the lines on three machines when one would end the gate past thirty minutes; an inconclusive timing line records the times" "$log" eval '[ $code = 1 ] &&
  [ "$(grep -E "^(budget|runtime|sentinel) " "$log" | awk "{print \$2}" | sort -u | wc -l)" = 3 ] && grep -q -E "^compare .* ok +measure: the lines on 3 of 3 machines ready" "$log" &&
  grep -q -E "^budget .* INCONCLUSIVE " "$log" && grep -q "^gate: the times recorded" "$log" &&
  [ "$recorded" = "$({ echo "budget $(row budget "m[0-9]" "$log" | awk "{print \$3}")"; echo "fold $(row fold "m[0-9]" "$log" | awk "{print \$3}")"; echo "parser $(row parser "m[0-9]" "$log" | awk "{print \$3}")"; echo "runtime $(row runtime "m[0-9]" "$log" | awk "{print \$3}")"; echo "sentinel $(row sentinel "m[0-9]" "$log" | awk "{print \$3}")"; } | sort | tr "\n" " ")" ]'

echo "gate-scripts: $pass passed, $fail failed"
[ $fail = 0 ]
