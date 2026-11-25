#!/bin/bash
# Every program of tests/cases that uses no JavaScript interop and does not carry `// std: lean`, built with `--std=scala-library`
# (scala-library from the coursier cache as the standard library, its bodies compiled from
# TASTy) and compared with its .expected file (scalac's, a lean departure's tests/lean-expected aside).
# The programs expected to fail under the mode are tests/pending/stdlib.txt (tests/support/pending.py): the run
# fails when a listed program passes and when an unlisted one fails; --update takes the programs that pass now out
# of the list, --add writes the unlisted failures in, and either writes the suite's row of the board. Without the jar in
# the cache it is not run: its skip line, which the gate fails, every program it would run a skip (`cannot run` on the
# board under --update, none of an earlier run's verdicts kept), and no pass. STDLIB_REASONS=1 prints the first error of each failure.
# A build's diagnostics go to out/stdlib-tests/<name>.diagnostics and are compared as tests/run.sh compares them
# (an expectation holds the warnings its case is about) but for the run's summary of the warnings its flags left
# unreported: scala-library's TASTy marks deprecated what the lean std's sources do not, so scalac's summary under
# this mode is no line of the lean std's expectation, and a summary the expectation holds for the lean std's
# build is left out of the comparison. The summaries of the programs that print scalac's output are
# checked against tests/stdlib-summaries.txt instead: scalac 3.8.4's (`scala-cli compile -S 3.8.4`) in teq's
# spelling, written by hand; a program whose summaries are not scalac's fails.
cd "$(dirname "$0")/.."
# The lists are sorted bytewise: `sort -u` and `comm` under a locale's collation would take
# `tail_calls` and `tailcalls` for one name. The character set stays the locale's, which java's
# output encoding follows.
export LC_COLLATE=C
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
SUMMARIES=tests/stdlib-summaries.txt
M2=${COURSIER_CACHE:-$HOME/Library/Caches/Coursier/v1}/https/repo1.maven.org/maven2
nojar=
if ! ls "$M2"/org/scala-lang/scala-library/3.*/scala-library-3.*.jar > /dev/null 2>&1; then
  echo "stdlib: skipped, no scala-library 3.x jar in the coursier cache"
  nojar=1
fi
out=out/stdlib-tests
mkdir -p "$out"
rm -f "$out"/*.result "$out"/*.diagnostics "$out/summaries.txt"
: > "$out/results.txt"
update=
add=
for arg in "$@"; do
  case $arg in
    --update) update=--update ;;
    --add) add=--add ;;
  esac
done
# TEQ_CASES names the programs of the run alone (tests/support/cases.sh): its line says so, no list is written
. tests/support/cases.sh
cases_filter_begin stdlib tests/cases TEQ_CASES "$@"
pass=0
fail=0
passing=""
for src in $(case_sources tests/cases TEQ_CASES); do
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
  if [ -n "$nojar" ]; then
    printf 'skip %s\tno scala-library 3.x jar in the coursier cache\n' "$name" >> "$out/results.txt"
    continue
  fi
  flags=$(grep -h -o '^// teq: .*' "$src" "$src"/*.scala 2>/dev/null | head -1 | sed 's|^// teq: ||')
  RUN_DIAGNOSTICS=$out/$name.diagnostics run_js 20 "$out/$name.js" "$src" $flags --std=scala-library > "$out/$name.run" 2>&1
  code=$?
  { grep -v -E "$SUMMARY" "$out/$name.diagnostics" 2> /dev/null; cat "$out/$name.run"; } > "$out/$name.actual"
  if [ $code = 0 ] && diff -q <(expectation_without_summary "$expected") "$out/$name.actual" > /dev/null; then
    pass=$((pass + 1))
    passing="$passing$name"$'\n'
    printf 'pass %s\n' "$name" >> "$out/results.txt"
    { grep -E "$SUMMARY" "$out/$name.diagnostics" | sed "s|^|$name: |"; } >> "$out/summaries.txt"
  else
    fail=$((fail + 1))
    reason=$(grep -m1 -o 'error: .*' "$out/$name.actual")
    [ -z "$reason" ] && reason=$(grep -m1 'Error\|error' "$out/$name.actual")
    [ -z "$reason" ] && reason="output differs"
    # A stack frame names the built file by its absolute path: the note keeps the file's name alone, the same on
    # every machine, and is cut to its length after that (cut first, a long checkout's path leaves a piece of itself).
    reason=${reason//"$PWD/$out/"/}
    reason=${reason//"$out/"/}
    reason=$(cut -c1-140 <<< "$reason")
    printf 'fail %s\t%s\n' "$name" "$(tr '\t' ' ' <<< "$reason")" >> "$out/results.txt"
    [ -n "$STDLIB_REASONS" ] && echo "$name: $reason"
  fi
done
printf '%s' "$passing" | sort > "$out/passing.txt"
touch "$out/summaries.txt"
sort -o "$out/summaries.txt" "$out/summaries.txt"
# A program that prints scalac's output passes when its summaries of hidden warnings are scalac's too
# (tests/stdlib-summaries.txt): the programs whose summaries differ, scalac's against this run's.
of_passing() { awk -F': ' 'NR == FNR { p[$0]; next } $1 in p' "$out/passing.txt" "$1" | sort; }
comm -3 <(of_passing "$SUMMARIES") <(of_passing "$out/summaries.txt") | sed -e $'s/^\t//' -e 's/: .*//' | sort -u > "$out/summaries-differ.txt"
awk 'FILENAME == ARGV[1] { d[$1]; next }
  $1 == "pass" && ($2 in d) { print "fail " $2 "\tits summaries of hidden warnings are not scalac'"'"'s (tests/stdlib-summaries.txt)"; next }
  { print }' "$out/summaries-differ.txt" "$out/results.txt" > "$out/results.judged"
echo "stdlib: $(grep -c '^pass ' "$out/results.judged") passed, $(grep -c '^fail ' "$out/results.judged") failed$([ -z "$nojar" ] || echo ", $(grep -c '^skip ' "$out/results.judged") not run")"
# The run held to the list of the programs expected to fail (tests/support/pending.py).
if [ -n "$CASES_FILTERED" ]; then
  held=$(python3 tests/support/pending.py stdlib "$out/results.judged" --partial)
  code=$?
  cases_judged "$out/results.judged" || code=1
  printf '%s\n' "$held" | sed '$d'
  cases_line "$(printf '%s\n' "$held" | tail -1)"
  exit $code
fi
python3 tests/support/pending.py stdlib "$out/results.judged" $update $add
