#!/bin/bash
# Runs every suite in turn, or only the suites named as arguments (`tests/all.sh cases jvm`; a pieced suite's
# one piece as `modules.1/2`), and
# ends with one line per suite; exits non-zero when any failed or a name is unknown. The
# output of a failing suite is printed in full, a passing one is summed up by its last line. The
# scala3 harnesses fail without a checkout in $SCALA3. These leave checks not run then, which pass nothing: the
# classpath suite without its jars in the coursier cache, parts of the classfile suite without a JDK
# or javap, the nir suite's jars and the native-link suite without them in the coursier cache, the app suite
# without the jars its corpus reads, and the budget and
# runtime suites on a machine other than the one that recorded bench/budgets.txt and
# bench/runtime-budgets.txt, the prop suite without the crates of proptests/ fetched, the idna suite without zig
# 0.16.0, the native-build suite without it or its jars, both on a host other than x86_64 Linux, and the
# test runner's classes without a javac (a javac other than 17's only notes a difference). Such a suite exits 0, and
# its line here is `part` with the first line that says what did not run (tests/gate.sh's want-of rule, which fails
# it at the gate), not `ok`. Each suite's whole
# output goes to out/all/<suite>.log as it runs, and its time and verdict to a line of out/all/times.txt
# (`<suite> <seconds> ok|part|FAIL`), which a run on another machine keeps (the GitHub workflow's artifacts).
cd "$(dirname "$0")/.."
# The compiler the suites test, target/release/teq unless TEQ names another; the repository's launcher runs
# the suites written in Scala (./teq interp tests/x.scala) under it too, so that they run under the binary they
# test, not the release teq.lock pins (docs/DEVELOPING.md, "The repository's scripts").
export TEQ=${TEQ:-$PWD/target/release/teq}
# The default checkout sits next to the main working tree, which a linked worktree is not in.
if [ -z "$SCALA3" ] && common=$(git rev-parse --path-format=absolute --git-common-dir 2> /dev/null); then
  export SCALA3="$(dirname "$common")/../teq-ref/scala3"
fi
status=0
summary=()
# lacking <log>: the first line of the log that says a check did not run for want of something (tests/gate.sh's rule)
source <(sed -n '/^skips=/,/^lacking()/p' tests/gate.sh)
logs=out/all
mkdir -p "$logs"
rm -f "$logs/times.txt"
# A suite named `<suite>.<k>/<n>` runs the k-th of its n pieces (`--piece k/n`, the suites the gate deals as
# pieces: tests/support/pieces.sh) under that name in the summary, the log and the times; one piece of a suite a run.
pieced=" lsp jvm-watch modules split gate-scripts "
declare -A piece
only="  "
for name in "$@"; do
  if [[ $name =~ ^([a-z0-9-]+)\.([1-9][0-9]*)/([1-9][0-9]*)$ ]]; then
    if [[ "$pieced" != *" ${BASH_REMATCH[1]} "* ]]; then
      status=1
      summary+=("FAIL  $name: ${BASH_REMATCH[1]} runs whole, in no pieces")
      continue
    fi
    piece[${BASH_REMATCH[1]}]=${BASH_REMATCH[2]}/${BASH_REMATCH[3]}
    name=${BASH_REMATCH[1]}
  fi
  only+="$name "
done
seen=()
suite() {
  local name=$1
  shift
  seen+=("$name")
  if [ "$only" != "  " ] && [[ "$only" != *" $name "* ]]; then
    return
  fi
  local label=$name
  if [ -n "${piece[$name]:-}" ]; then
    set -- "$@" --piece "${piece[$name]}"
    label="$name.${piece[$name]}"
  fi
  local log=$logs/${label//\//-}.log
  local out start=$SECONDS
  # Written as the suite runs, so that a run cut short keeps what it printed.
  "$@" > "$log" 2>&1
  local code=$?
  out=$(cat "$log")
  local last
  last=$(tail -1 <<< "$out")
  if [ $code -ne 0 ]; then
    status=1
    echo "$out"
    summary+=("FAIL  $label: $last")
    echo "$label $((SECONDS - start)) FAIL" >> "$logs/times.txt"
  elif lack=$(lacking "$log") && [ -n "$lack" ]; then
    summary+=("part  $label: $last; not run: $lack")
    echo "$label $((SECONDS - start)) part" >> "$logs/times.txt"
  else
    summary+=("ok    $label: $last")
    echo "$label $((SECONDS - start)) ok" >> "$logs/times.txt"
  fi
}
suite cases         ./tests/run.sh
suite errors        ./tests/run_errors.sh
suite parser        ./tests/parser.sh
suite interop       ./tests/run_interop.sh
suite dce           ./tests/dce.sh
suite size          ./teq interp tests/size.scala
suite fix           ./teq interp tests/fix.scala
suite split         ./tests/split.sh
suite split-watch   ./tests/split-watch.sh
suite check-watch   ./tests/check-watch.sh
suite jvm-watch     ./tests/jvm-watch.sh
suite lsp           ./tests/lsp.sh
suite task          ./tests/task.sh
suite runner-classes ./tests/runner-classes.sh
suite watch-memory  ./tests/watch-memory.sh
suite split-determinism ./tests/split-determinism.sh
suite workers       ./tests/workers.sh
suite budget        ./bench/budget.sh
suite runtime       ./tests/runtime.sh
suite asymptotics   ./tests/asymptotics.sh
suite tasty         ./tests/tasty.sh
suite init-view     ./tests/init-view.sh
suite modules       ./tests/modules.sh
suite tasty-exec    ./teq interp tests/tasty-exec.scala
suite analysis-bytes ./tests/analysis-bytes.sh
suite classfile     ./teq interp tests/classfile.scala
suite nir           ./teq interp tests/nir.scala
suite native-link   ./teq interp tests/native-link.scala
suite idna          ./teq interp tests/idna.scala
suite native-build  ./teq interp tests/native-build.scala
suite classpath     ./tests/classpath.sh
suite java-sources  ./tests/java-sources.sh
suite java-reader   ./tests/java-reader.sh
suite stdlib        ./tests/run_stdlib.sh
suite app           ./tests/app.sh
suite lila-test-verb ./tests/lila-test-verb.sh
suite reference-test-verb ./tests/reference-test-verb.sh
suite mill-test-verb ./tests/mill-test-verb.sh
suite std-entered   ./tests/std-entered.sh
suite std-kinds     ./tests/std-kinds.sh
suite java-lang-names ./tests/java-lang-names.sh
suite jvm           ./tests/run_jvm.sh
suite jvm-21        ./tests/run_jvm.sh --java-output-version=21
suite pairs-twin    ./tests/pairs-twin.sh
suite interp        ./tests/run_interp.sh
suite interp-script ./tests/interp-script.sh
suite fold          ./tests/fold.sh
suite planted       ./tests/planted.sh
suite scala3-run    tests/scala3/run.sh
suite scala3-interp tests/scala3/run.sh --target interp
suite scala3-macros tests/scala3/run.sh --suite run-macros
suite scala3-macros-interp tests/scala3/run.sh --suite run-macros --target interp
suite scala3-typing python3 tests/scala3-typing/harness.py
# The conversion search's matrix (tests/conversion-matrix/run.py): every tenth program of each set on the lean std,
# scalac 3.8.4's answers its oracle and known.tsv the departures; a whole run on the four legs writes its board rows.
suite conversion-matrix python3 tests/conversion-matrix/run.py --tenth
# The cut of the kept body's clause question (tests/kept-body-cut/run.py): the programs generated from the std's
# definitions, over lean products on two legs, held to their recorded observables.
suite kept-body-cut python3 tests/kept-body-cut/run.py
# A kept body's lists of user-defined callees (tests/kept-body-lists/run.py): the generated programs over lean products
# on two legs, held to scalac 3.8.4's answers and tests/pending/kept-body-lists.txt (mutants/controls.sh: its controls).
suite kept-body-lists python3 tests/kept-body-lists/run.py
# The conformance board's rows (tests/board/dotty.py: dotty's own suites at the tag 3.8.4, each held to its list of
# the tests expected to fail and to its committed list's verdicts), as four lines: the three longest rows one each,
# every other row in the rest. They need dotty at the tag: SCALA3_TAG, else scala3-3.8.4 beside $SCALA3.
suite board-run     python3 tests/board/dotty.py --row run
suite board-run-scalajs python3 tests/board/dotty.py --row run-scalajs
suite board-pos     python3 tests/board/dotty.py --row pos
suite board-rest    python3 tests/board/dotty.py --except run,run-scalajs,pos
suite rust-warnings ./tests/rust-warnings.sh
suite no-caps ./tests/no-caps.sh
suite gate-coverage ./tests/gate-coverage.sh
suite gate-scripts  ./tests/gate-scripts.sh
suite tree-paths    ./tests/tree-paths.sh
# Outside the groups the gate runs: it needs the crates of proptests/ fetched once, and skips
# without them.
suite prop          ./tests/prop.sh
for name in $only; do
  if [[ " ${seen[*]} " != *" $name "* ]]; then
    status=1
    summary+=("FAIL  $name: no such suite")
  fi
done
printf '%s\n' "${summary[@]}"
exit $status
