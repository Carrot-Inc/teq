#!/bin/bash
# Runs every suite in turn, or only the suites named as arguments (`tests/all.sh cases jvm`), and
# ends with one line per suite; exits non-zero when any failed or a name is unknown. The
# output of a failing suite is printed in full, a passing one is summed up by its last line. The
# two scala3 harnesses skip without a checkout in $SCALA3 and count as passed then, the
# classpath suite without its jars in the coursier cache, parts of the classfile suite
# without a JDK or javap, the app suite without the jars its corpus reads, and the budget and
# runtime suites on a machine other than the one that recorded bench/budgets.txt and
# bench/runtime-budgets.txt, the prop suite without the crates of proptests/ fetched, and the
# test runner's classes without a javac (a javac other than 17's only notes a difference).
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
only=" $* "
seen=()
suite() {
  local name=$1
  shift
  seen+=("$name")
  if [ "$only" != "  " ] && [[ "$only" != *" $name "* ]]; then
    return
  fi
  local out
  out=$("$@" 2>&1)
  local code=$?
  local last
  last=$(tail -1 <<< "$out")
  if [ $code -ne 0 ]; then
    status=1
    echo "$out"
    summary+=("FAIL  $name: $last")
  else
    summary+=("ok    $name: $last")
  fi
}
suite cases         ./tests/run.sh
suite errors        ./tests/run_errors.sh
suite parser        ./tests/parser.sh
suite interop       ./tests/run_interop.sh
suite dce           ./tests/dce.sh
suite size          ./teq interp tests/size.scala
suite split         ./tests/split.sh
suite split-watch   ./tests/split-watch.sh
suite split-watch   ./tests/check-watch.sh
suite split-watch   ./tests/jvm-watch.sh
suite split-watch   ./tests/lsp.sh
suite split-watch   ./tests/task.sh
suite split-watch   ./tests/runner-classes.sh
suite split-watch   ./tests/watch-memory.sh
suite split-watch   ./tests/split-determinism.sh
suite workers       ./tests/workers.sh
suite budget        ./bench/budget.sh
suite runtime       ./tests/runtime.sh
suite asymptotics   ./tests/asymptotics.sh
suite tasty         ./tests/tasty.sh
suite modules       ./tests/modules.sh
suite tasty-exec    ./teq interp tests/tasty-exec.scala
suite analysis-bytes ./tests/analysis-bytes.sh
suite classfile     ./teq interp tests/classfile.scala
suite classpath     ./tests/classpath.sh
suite stdlib        ./tests/run_stdlib.sh
suite app           ./tests/app.sh
suite jvm           ./tests/run_jvm.sh
suite jvm-21        ./tests/run_jvm.sh --java-output-version=21
suite interp        ./tests/run_interp.sh
suite fold          ./tests/fold.sh
suite planted       ./tests/planted.sh
suite scala3-run    tests/scala3/run.sh
suite scala3-interp tests/scala3/run.sh --target interp
suite scala3-macros tests/scala3/run.sh --suite run-macros
suite scala3-macros-interp tests/scala3/run.sh --suite run-macros --target interp
suite scala3-typing python3 tests/scala3-typing/harness.py
suite rust-warnings ./tests/rust-warnings.sh
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
