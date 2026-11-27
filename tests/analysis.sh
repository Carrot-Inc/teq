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
# stored relations beside them. Documented mappings are in tests/analysis/expected.txt and
# outstanding differences in tests/analysis/pending.txt. An unlisted or stale difference fails.
# Cases marked `exact` allow only pending records. A teq build without a dump requires
# `<project> (no-analysis) - pending`. Then tests/analysis/relations.txt: the hash of a name
# that must change, or stay, from one project to another, on both sides. Before sbt starts,
# tests/analysis/compare_test.py checks that compare.py fails recorded dumps with any part of one
# side taken away, changed, added or moved.
#
# The cases: every module case of tests/modules (but those with an expect-errors.txt, whose b
# does not compile by design), each module a project over the modules before it, and the cases
# of tests/analysis/cases. The oracle has to be there: without sbt, or when a scalac compile
# fails, the suite fails.
#
#   tests/analysis.sh [--update [--add-new]] [case...]      TEQ names the binary (target/release/teq by default)
#   --update removes stale pending records for the cases run; --add-new also records new ones.
#
# The dumps and comparisons go to target/analysis-results/. The exit code says whether every
# project passed.
cd "$(dirname "$0")/.."
root=$PWD
teq=${TEQ:-./target/release/teq}
case $teq in /*) ;; *) teq=$root/$teq ;; esac
pending=tests/analysis/pending.txt
update=0
add_new=0
selected=()
for arg in "$@"; do
  case $arg in
    --update) update=1 ;;
    --add-new) add_new=1 ;;
    --*) echo "unknown option: $arg" >&2; exit 2 ;;
    *) selected+=("$arg") ;;
  esac
done
if (( add_new && !update )); then
  echo "--add-new requires --update" >&2
  exit 2
fi
if ! command -v sbt > /dev/null 2>&1; then
  echo "FAIL sbt is not on the PATH: the oracle is unavailable"
  exit 1
fi
out=$root/target/analysis-results
rm -rf "${root:?}/target/analysis-results" || exit 1
mkdir -p "$out" || exit 1
export LC_ALL=C
set -o pipefail
old=$out/previous.pending
if [[ -f $pending || $update = 0 ]]; then
  awk 'NF && $1 !~ /^#/' "$pending" | sort -u > "$old" || exit 1
else
  : > "$old"
fi

pass=0
fail=0
if python3 tests/analysis/compare_test.py > "$out/compare_test.txt" 2>&1; then
  pass=$((pass + 1))
else
  fail=$((fail + 1))
  echo "FAIL compare_test"
  head -20 "$out/compare_test.txt"
  (( update )) && exit 1
fi
tail -1 "$out/compare_test.txt"

cases=()
if (( ${#selected[@]} )); then
  for c in "${selected[@]}"; do
    if [ -d "tests/modules/$c" ]; then cases+=("$root/tests/modules/$c"); else cases+=("$root/tests/analysis/cases/$c"); fi
  done
else
  for d in tests/modules/*/; do
    c=$(basename "$d")
    # A case whose downstream is refused by design (`expect-errors.txt`) has no analysis to
    # compare, and its failed compile would end sbt's batch.
    [ -f "$d/expect-errors.txt" ] && continue
    # a case tests/modules.sh expects to fail (tests/pending/modules.txt) has no analysis to compare
    grep -q "^$c\([[:space:]]\|\$\)" tests/pending/modules.txt 2> /dev/null && continue
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
  cd integrations/sbt/analysis || exit 1
  rm -rf target/teq-products target/scalac-products target/out/jvm/scala-3.8.4/*/analysis
  ANALYSIS_CASES=$list TEQ=$teq sbt --server --batch "${commands#; }"
) > "$out/sbt.log" 2>&1
sbt_status=$?

if (( sbt_status )); then
  tail -20 "$out/sbt.log"
  echo "FAIL sbt exited $sbt_status; $out/sbt.log"
  exit 1
fi

printf '%s\n' "${projects[@]}" > "$out/projects"
awk 'NR == FNR {project[$1]; next} !($1 in project)' "$out/projects" "$old" > "$out/unselected.pending"
next=$out/next.pending
: > "$next"
if (( ${#selected[@]} )); then
  cat "$out/unselected.pending" >> "$next"
elif (( !update )) && [[ -s $out/unselected.pending ]]; then
  sed 's/^/stale   /' "$out/unselected.pending"
  fail=$((fail + 1))
fi

for p in "${projects[@]}"; do
  dir=integrations/sbt/analysis/target/out/jvm/scala-3.8.4/$p/analysis
  before=$out/$p.previous
  actual=$out/$p.pending
  awk -v p="$p" '$1 == p' "$old" > "$before"
  missing="$p (no-analysis) - pending"
  status=0
  if [[ ! -f $dir/scalac.json || ! -f $dir/teq.json ]]; then
    if [[ ! -f $dir/scalac.json || ! -f $dir/teq-error.txt ]]; then
      echo "FAIL $p: missing oracle output; $out/sbt.log"
      exit 1
    fi
    echo "$missing" > "$actual"
    if (( !update )); then
      if ! grep -Fxq "$missing" "$before"; then
        echo "FAIL $p: no analysis (teq: $(head -c 300 "$dir/teq-error.txt"))"
        status=1
      fi
      if grep -Fvx "$missing" "$before" > "$out/$p.stale"; then
        sed 's/^/stale   /' "$out/$p.stale"
        status=1
      fi
    fi
  else
    cp "$dir/scalac.json" "$out/$p.scalac.json" || exit 1
    cp "$dir/teq.json" "$out/$p.teq.json" || exit 1
    [ -f "$dir/teq-costs.json" ] && cp "$dir/teq-costs.json" "$out/$p.costs.json"
    compare=(python3 tests/analysis/compare.py "$dir/scalac.json" "$dir/teq.json" --expect tests/analysis/expected.txt --case "$p")
    if (( update )); then
      "${compare[@]}" --emit-pending > "$actual" 2> "$out/$p.txt" || status=$?
    else
      "${compare[@]}" --pending "$before" > "$out/$p.txt" 2>&1 || status=$?
    fi
    if (( status )); then
      echo "FAIL $p"
      grep -v "^exact\|^mapped\|^shape" "$out/$p.txt" | head -20
      (( update )) && exit 1
    fi
    tail -1 "$out/$p.txt"
  fi
  if (( update )); then
    if (( add_new )); then
      cat "$actual" >> "$next"
    else
      comm -12 "$before" "$actual" >> "$next" || exit 1
      comm -13 "$before" "$actual" > "$out/$p.new" || exit 1
      if [[ -s $out/$p.new ]]; then
        echo "FAIL $p: --add-new required for"
        cat "$out/$p.new"
        status=1
      fi
    fi
  fi
  if (( status )); then fail=$((fail + 1)); else pass=$((pass + 1)); fi
done
if (( ! ${#selected[@]} )); then
  if python3 tests/analysis/relations.py tests/analysis/relations.txt "$out" > "$out/relations.txt" 2>&1; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    grep FAIL "$out/relations.txt"
  fi
  tail -1 "$out/relations.txt"
fi
echo "analysis: $pass passed, $fail failed (${#projects[@]} projects, compare_test and the relations)"
if (( update )); then
  sort -u -o "$next" "$next" || exit 1
  removed=$(comm -23 "$old" "$next" | wc -l) || exit 1
  added=$(comm -13 "$old" "$next" | wc -l) || exit 1
  {
    echo "# Generated by tests/analysis.sh --update. These lines are what teq's analysis DIFFERS FROM SCALAC"
    echo "# in and is NOT YET MENDED. None is a mapping. A line leaves only by tests/analysis.sh --update"
    echo "# once its difference is gone."
    cat "$next"
  } > "$out/generated.pending" && mv "$out/generated.pending" "$pending" || exit 1
  echo "analysis --update: records removed $removed, records added $added"
fi
[ $fail = 0 ]
