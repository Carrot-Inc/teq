#!/bin/bash
# Every program of tests/cases that uses no JavaScript interop and does not carry `// std: lean`, built with `--std=scala-library`
# (scala-library from the coursier cache as the standard library, its bodies compiled from
# TASTy) and compared with the .expected file its lean-std build is checked with.
# tests/stdlib-passing.txt lists the programs that pass under the mode; a listed one that fails
# is a regression, and --update rewrites the list from this run. Skips, and counts as passed,
# without the jar in the cache. STDLIB_REASONS=1 prints the first error of each failure.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
LIST=tests/stdlib-passing.txt
M2=${COURSIER_CACHE:-$HOME/Library/Caches/Coursier/v1}/https/repo1.maven.org/maven2
if ! ls "$M2"/org/scala-lang/scala-library/3.*/scala-library-3.*.jar > /dev/null 2>&1; then
  echo "stdlib: skipped, no scala-library 3.x jar in the coursier cache"
  exit 0
fi
out=out/stdlib-tests
mkdir -p "$out"
rm -f "$out"/*.result
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
  if run_js 20 "$out/$name.js" "$src" $flags --std=scala-library > "$out/$name.actual" 2>&1 \
    && diff -q "$expected" "$out/$name.actual" > /dev/null; then
    pass=$((pass + 1))
    passing="$passing$name"$'\n'
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
if [ "$1" = "--update" ]; then
  cp "$out/passing.txt" "$LIST"
  echo "wrote $pass names to $LIST"
fi
regressions=$(comm -23 <(sort "$LIST" 2>/dev/null) "$out/passing.txt" | tr '\n' ' ')
new=$(comm -13 <(sort "$LIST" 2>/dev/null) "$out/passing.txt" | tr '\n' ' ')
[ -n "$new" ] && echo "passing but not in $LIST: $new"
[ -n "$regressions" ] && echo "REGRESSIONS: $regressions"
echo "stdlib: $pass passed, $fail failed"
[ -z "$regressions" ]
