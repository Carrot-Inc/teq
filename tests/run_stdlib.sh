#!/bin/bash
# Every program of tests/cases that uses no JavaScript interop and does not carry `// std: lean`, built with `--std=scala-library`
# (scala-library from the coursier cache as the standard library, its bodies compiled from
# TASTy) and compared with its .expected file (scalac's, a lean departure's tests/lean-expected aside).
# tests/stdlib-passing.txt lists the programs that pass under the mode; a listed one that fails
# is a regression, and --update rewrites the list from this run. Skips, and counts as passed,
# without the jar in the cache. STDLIB_REASONS=1 prints the first error of each failure.
# A build's diagnostics go to out/stdlib-tests/<name>.diagnostics and are compared as tests/run.sh compares them
# (an expectation holds the warnings its case is about) but for the run's summary of the warnings its flags left
# unreported: scala-library's TASTy marks deprecated what the lean std's sources do not, so scalac's summary under
# this mode is no line of the lean std's expectation. The summaries of the listed programs are checked against
# tests/stdlib-summaries.txt instead: scalac 3.8.4's (`scala-cli compile -S 3.8.4`) in teq's spelling, written by
# hand, so that a program the list gains is checked against scalac's summary, not this run's.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
LIST=tests/stdlib-passing.txt
SUMMARIES=tests/stdlib-summaries.txt
SUMMARY='^warning: there (was|were) [0-9]+ [a-z]+ warnings?; re-run with --[a-z]+ for details$'
M2=${COURSIER_CACHE:-$HOME/Library/Caches/Coursier/v1}/https/repo1.maven.org/maven2
if ! ls "$M2"/org/scala-lang/scala-library/3.*/scala-library-3.*.jar > /dev/null 2>&1; then
  echo "stdlib: skipped, no scala-library 3.x jar in the coursier cache"
  exit 0
fi
out=out/stdlib-tests
mkdir -p "$out"
rm -f "$out"/*.result "$out"/*.diagnostics "$out/summaries.txt"
pass=0
fail=0
passing=""
for src in tests/cases/*.scala tests/cases/*/; do
  src=${src%/}
  name=$(basename "$src" .scala)
  expected="tests/cases/$name.expected"
  [ -f "$expected" ] || continue
  if grep -q -h 'scala\.scalajs\|@js\b\|@js(\|jsImport\|jsExport\|js\.Dynamic\|js\.Array\|js\.Object\|import js\.' "$src" "$src"/*.scala 2>/dev/null; then
    continue
  fi
  # A program whose expectations are the lean std's own contract (`// std: lean`, as the
  # classpath suite's directive names the modes a program runs under) has no oracle here.
  if grep -q -h '^// std: lean$' "$src" "$src"/*.scala 2>/dev/null; then
    continue
  fi
  # sourcecode comes from its jar, which this runner does not put on the class path.
  if grep -q -h '^import sourcecode\|sourcecode\.' "$src" "$src"/*.scala 2>/dev/null; then
    continue
  fi
  flags=$(grep -h -o '^// teq: .*' "$src" "$src"/*.scala 2>/dev/null | head -1 | sed 's|^// teq: ||')
  RUN_DIAGNOSTICS=$out/$name.diagnostics run_js 20 "$out/$name.js" "$src" $flags --std=scala-library > "$out/$name.run" 2>&1
  code=$?
  { grep -v -E "$SUMMARY" "$out/$name.diagnostics" 2> /dev/null; cat "$out/$name.run"; } > "$out/$name.actual"
  if [ $code = 0 ] && diff -q "$expected" "$out/$name.actual" > /dev/null; then
    pass=$((pass + 1))
    passing="$passing$name"$'\n'
    { grep -E "$SUMMARY" "$out/$name.diagnostics" | sed "s|^|$name: |"; } >> "$out/summaries.txt"
  else
    fail=$((fail + 1))
    if [ -n "$STDLIB_REASONS" ]; then
      reason=$(grep -m1 -o 'error: .*' "$out/$name.actual" | cut -c1-140)
      [ -z "$reason" ] && reason=$(grep -m1 'Error\|error' "$out/$name.actual" | cut -c1-140)
      [ -z "$reason" ] && reason="output differs"
      echo "$name: $reason"
    fi
  fi
done
printf '%s' "$passing" | sort > "$out/passing.txt"
touch "$out/summaries.txt"
sort -o "$out/summaries.txt" "$out/summaries.txt"
if [ "$1" = "--update" ]; then
  cp "$out/passing.txt" "$LIST"
  echo "wrote $pass names to $LIST"
fi
# The summaries of the listed programs that passed, scalac's against this run's.
checked=$(sort "$LIST" | comm -12 - "$out/passing.txt")
of_checked() { awk -F': ' 'NR == FNR { p[$0]; next } $1 in p' <(printf '%s\n' "$checked") "$1" | sort; }
summaries=$(diff <(of_checked "$SUMMARIES") <(of_checked "$out/summaries.txt"))
regressions=$(comm -23 <(sort "$LIST" 2>/dev/null) "$out/passing.txt" | tr '\n' ' ')
new=$(comm -13 <(sort "$LIST" 2>/dev/null) "$out/passing.txt" | tr '\n' ' ')
[ -n "$new" ] && echo "passing but not in $LIST: $new"
[ -n "$regressions" ] && echo "REGRESSIONS: $regressions"
[ -n "$summaries" ] && printf 'SUMMARIES (< %s, > this run):\n%s\n' "$SUMMARIES" "$summaries"
echo "stdlib: $pass passed, $fail failed"
[ -z "$regressions" ] && [ -z "$summaries" ]
