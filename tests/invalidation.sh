#!/bin/bash
# The invalidation oracle (docs/TARGETS.md, "The invalidation oracle"): what zinc 2.0.4's own
# incremental compiler recompiles after each scenario's edit, on scalac 3.8.4's analysis
# through sbt 2.0.8's compiler bridge and on teq's through the plugin's adapter. Per scenario of
# tests/analysis/invalidation/scenarios.txt and variant of its options, the driver
# (integrations/sbt/analysis, project/Invalidation.scala) builds the case's projects on both
# sides, applies the edit, runs zinc's `Incremental.apply` again over each project with the
# analysis it kept, the compile callback compiling exactly the batch zinc asks for, and builds
# the edited case afresh as the reference. tests/analysis/invalidation.py then compares the
# sides, the runs with the fresh builds, and the frozen batches and differences of
# tests/analysis/expected.txt; before sbt starts, tests/analysis/compare_test.py checks both
# comparators.
#
#   tests/invalidation.sh [scenario...]     TEQ names the binary (target/release/teq by default);
#                                           INVALIDATION_REPEAT=n runs the scenarios n times in
#                                           one sbt session, for the warmed runtime
#
# A mandatory landing check of the analysis contract, apart from tests/gate.sh, until it has a
# budget and a scheduled runner. sbt runs on a remote machine; the dumps, the comparison and
# the timings go to target/invalidation-results/. Without sbt, or without a scenario's dump,
# the suite fails.
cd "$(dirname "$0")/.."
root=$PWD
teq=${TEQ:-./target/release/teq}
case $teq in /*) ;; *) teq=$root/$teq ;; esac
if ! command -v sbt > /dev/null 2>&1; then
  echo "FAIL sbt is not on the PATH: the oracle is unavailable"
  exit 1
fi
if [ ! -x "$teq" ]; then
  echo "FAIL no teq binary at $teq"
  exit 1
fi
out=$root/target/invalidation-results
rm -rf "$out"
mkdir -p "$out"

pass=0
fail=0
if python3 tests/analysis/compare_test.py > "$out/compare_test.txt" 2>&1; then
  pass=$((pass + 1))
else
  fail=$((fail + 1))
  echo "FAIL compare_test"
  head -20 "$out/compare_test.txt"
fi
tail -1 "$out/compare_test.txt"

list=$(IFS=,; echo "$*")
repeat=${INVALIDATION_REPEAT:-1}
commands=invalidationOracle
for _ in $(seq 2 "$repeat"); do commands="$commands; invalidationOracle"; done
started=$(date +%s)
(
  cd integrations/sbt/analysis
  rm -rf target/invalidation
  INVALIDATION_SCENARIOS=$list TEQ=$teq sbt --server --batch "$commands"
) > "$out/sbt.log" 2>&1
sbt_status=$?
elapsed=$(( $(date +%s) - started ))
results=integrations/sbt/analysis/target/invalidation/results
cp "$results"/*.json "$out/" 2> /dev/null
grep -E '^\[info\] invalidation' "$out/sbt.log" | sed 's/^\[info\] //' > "$out/timings.txt"
only=()
[ -n "$list" ] && only=(--only "$list")
if [ $sbt_status = 0 ]; then
  pass=$((pass + 1))
else
  fail=$((fail + 1))
  echo "FAIL sbt exited $sbt_status ($out/sbt.log)"
  grep -E '^\[error\]' "$out/sbt.log" | head -10
fi
if python3 tests/analysis/invalidation.py "$out" --scenarios tests/analysis/invalidation/scenarios.txt --expect tests/analysis/expected.txt "${only[@]}" > "$out/compare.txt" 2>&1; then
  pass=$((pass + 1))
else
  fail=$((fail + 1))
  echo "FAIL invalidation ($out/compare.txt)"
  grep -E '^(FAIL|stale)' "$out/compare.txt" | head -40
fi
tail -1 "$out/compare.txt"
tail -1 "$out/timings.txt"
echo "invalidation: $pass passed, $fail failed (compare_test, sbt, the comparison; sbt and the scenarios ${elapsed} s)"
[ $fail = 0 ]
