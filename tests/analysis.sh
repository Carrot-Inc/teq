#!/bin/bash
# The analysis against scalac's (docs/TARGETS.md, "The analysis graph"): every case is compiled
# by scalac 3.8.4 through sbt 2.0.8's compiler bridge and built by teq with `--analysis-version`,
# whose answer the plugin's adapter hands to zinc, each side through zinc 2.0.4's own incremental
# compiler with its API storage on, into products of its own against the upstream modules'
# products, zinc finding an upstream class in that module's analysis on both sides
# (integrations/sbt/analysis, project/Oracle.scala). Each side's dump holds the graph of every
# class with zinc's hashes of it, the dependency callbacks the compile made as canonical tuples
# and the relations zinc stored. tests/analysis/compare.py then compares the two per project: each
# class by its name and kind, definition by definition, and zinc's hashes of it (`apiHash`,
# `extraHash`, the name hashes); the dependencies per referring class and part, exactly, and the
# stored relations beside them. A difference that tests/analysis/expected.txt does not name
# fails, as does a line there that names nothing that differs; the exact-match cases listed there
# under `exact` may name none. Then tests/analysis/relations.txt: the hash of a name that must
# change, or stay, from one project to another, on both sides. Before sbt starts,
# tests/analysis/compare_test.py checks that compare.py fails recorded dumps with any part of one
# side taken away, changed, added or moved.
#
# The cases: every module case of tests/modules (but misuse, whose b does not compile), each
# module a project over the modules before it, and the cases of tests/analysis/cases. The
# oracle has to be there: without sbt, or when a scalac compile fails, the suite fails.
#
#   tests/analysis.sh [case...]      TEQ names the binary (target/release/teq by default)
#
# sbt runs on a remote machine (it takes minutes: sbt's start, 96 scalac compiles); the results
# go to target/analysis-results/ (the dumps and the comparison), the exit code says whether
# every project passed.
cd "$(dirname "$0")/.."
root=$PWD
teq=${TEQ:-./target/release/teq}
case $teq in /*) ;; *) teq=$root/$teq ;; esac
if ! command -v sbt > /dev/null 2>&1; then
  echo "FAIL sbt is not on the PATH: the oracle is unavailable"
  exit 1
fi
out=$root/target/analysis-results
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

cases=()
if [ $# -gt 0 ]; then
  for c in "$@"; do
    if [ -d tests/modules/$c ]; then cases+=("$root/tests/modules/$c"); else cases+=("$root/tests/analysis/cases/$c"); fi
  done
else
  for d in tests/modules/*/; do
    c=$(basename "$d")
    [ "$c" = misuse ] && continue
    cases+=("$root/tests/modules/$c")
  done
  for d in tests/analysis/cases/*/; do
    [ -d "$d" ] && cases+=("$root/${d%/}")
  done
fi

# The projects of each case, in their order: a module case's modules, a plain case itself.
projects=()
for d in "${cases[@]}"; do
  c=$(basename "$d")
  found=0
  for m in a b c main; do
    if [ -d "$d/$m" ]; then
      projects+=("${c}_$m")
      found=1
    fi
  done
  [ $found = 0 ] && projects+=("$c")
done

commands=""
for p in "${projects[@]}"; do
  commands="$commands; $p/analysisScalac; $p/analysisTeq"
done
list=$(IFS=,; echo "${cases[*]}")
(
  cd integrations/sbt/analysis
  rm -rf target/teq-products target/scalac-products target/out/jvm/scala-3.8.4/*/analysis
  ANALYSIS_CASES=$list TEQ=$teq sbt --server --batch "${commands#; }"
) > "$out/sbt.log" 2>&1
sbt_status=$?

for p in "${projects[@]}"; do
  dir=integrations/sbt/analysis/target/out/jvm/scala-3.8.4/$p/analysis
  if [ ! -f "$dir/scalac.json" ] || [ ! -f "$dir/teq.json" ]; then
    why="sbt exited $sbt_status; $out/sbt.log"
    [ -f "$dir/scalac-error.txt" ] && why="scalac: $(head -c 300 "$dir/scalac-error.txt")"
    [ -f "$dir/teq-error.txt" ] && why="teq: $(head -c 300 "$dir/teq-error.txt")"
    echo "FAIL $p: no analysis ($why)"
    fail=$((fail + 1))
    continue
  fi
  cp "$dir/scalac.json" "$out/$p.scalac.json"
  cp "$dir/teq.json" "$out/$p.teq.json"
  [ -f "$dir/teq-costs.json" ] && cp "$dir/teq-costs.json" "$out/$p.costs.json"
  if python3 tests/analysis/compare.py "$dir/scalac.json" "$dir/teq.json" --expect tests/analysis/expected.txt --case "$p" > "$out/$p.txt" 2>&1; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL $p"
    grep -v "^exact\|^mapped\|^shape" "$out/$p.txt" | head -20
  fi
  tail -1 "$out/$p.txt"
done
if [ $# -eq 0 ]; then
  if python3 tests/analysis/relations.py tests/analysis/relations.txt "$out" > "$out/relations.txt" 2>&1; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    grep FAIL "$out/relations.txt"
  fi
  tail -1 "$out/relations.txt"
fi
echo "analysis: $pass passed, $fail failed (${#projects[@]} projects, compare_test and the relations)"
[ $fail = 0 ]
