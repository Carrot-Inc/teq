#!/bin/bash
# The runtime budget: teq's own output at the quick size of bench/runtime.sh, teq-dev under node
# and teq-jvm under java with the flags `run_jvm` passes (tests/support/jars.sh), N runs per program (RUNTIME_RUNS,
# default 3), against the medians recorded in bench/runtime-budgets.txt for the wall time of the
# process and the program's steady-state time per iteration. A number whose fastest run takes
# more than RUNTIME_TOLERANCE percent (default 25) longer than its budget fails: a regression
# shows in every run, a busy machine in some. RUNTIME_RECORD=1 rewrites the budgets together
# with the identity of the machine; on another machine the suite skips, since the budgets say
# nothing about it. Like bench/budget.sh it is only meaningful on the machine that recorded the
# budgets, and only when that machine is quiet: the tolerance covers the noise of an idle
# machine, not of a build running next to it; the landing gate's runtime line compares the head's output
# with master's on a remote machine instead (bench/pairs.py), over the numbers this file's budgets list.
# Runs go to out/runtime-budget, apart from
# bench/runtime.sh's own results, and the expected outputs are those of tests/cases, so neither
# scala-cli nor a Scala.js build is needed.
cd "$(dirname "$0")/.."
budgets=bench/runtime-budgets.txt
runs=${RUNTIME_RUNS:-3}
tolerance=${RUNTIME_TOLERANCE:-25}
work=out/runtime-budget
machine="$(uname -m) $(sysctl -n machdep.cpu.brand_string 2> /dev/null || grep -m1 'model name' /proc/cpuinfo 2> /dev/null | cut -d: -f2- | sed 's/^ *//') $(sysctl -n hw.ncpu 2> /dev/null || nproc 2> /dev/null) cores"

if [ -z "$RUNTIME_RECORD" ]; then
  if [ ! -f "$budgets" ]; then
    echo "runtime: no $budgets; record one with RUNTIME_RECORD=1 $0"
    exit 0
  fi
  recorded=$(sed -n 's/^# machine: //p' "$budgets")
  if [ "$recorded" != "$machine" ]; then
    echo "runtime: skipped, the budgets were recorded on another machine ($recorded)"
    exit 0
  fi
fi

mkdir -p "$work"
rm -f "$work/quick/results.tsv"
if ! ./bench/runtime.sh --quick --runs "$runs" --only teq-dev --only teq-jvm --out "$work" > "$work/log.txt" 2>&1; then
  echo "FAIL: bench/runtime.sh failed (see $work/log.txt)"
  grep 'FAIL' "$work/log.txt" | head -10
  exit 1
fi
MACHINE="$machine" python3 bench/runtime.py budget "$work/quick/results.tsv" "$budgets" "$tolerance" ${RUNTIME_RECORD:+record}
