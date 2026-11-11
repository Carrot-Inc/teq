#!/bin/bash
# Every tests/errors/*.scala, and every directory under tests/errors (a program with the files
# next to it, such as a teq.toml), has to be rejected with all of its `// expect:` messages and
# none of its `// absent:` ones (a diagnostic scalac does not give, at its line and column), and
# its whole output equal to a `<name>.check` next to it where there is one. A
# `// teq: <flags>` line passes those flags to teq, a `// command: build` line builds the program
# in the place of checking it (for what only a build rejects), a `// jars: <names>` line puts those jars on
# the class path (tests/support/jars.sh); without one of them in the coursier cache the program
# is skipped. A `// jdk: none` line runs teq with no JDK to find (no JAVA_HOME, an empty PATH). Each program's exit and complete diagnostics go to out/errors-tests/<name>.out, and
# the last line says whether every program was rejected as expected, with the counts of
# tests/warnings/unused-imports.sh, tests/warnings/unused-kinds.sh, tests/warnings/matches.sh,
# tests/warnings/policy.sh, tests/warnings/workers.sh and tests/source-version.scala, which run after them.
# The programs expected to fail are tests/pending/errors.txt (tests/support/pending.py): the suite fails when a
# listed program is rejected as expected and when an unlisted one is not; --update takes the programs that pass
# now out of the list, --add writes the unlisted failures in (for the change that brings the programs), and
# either writes the suite's row of the board, tests/board/suites/errors.tsv. A listed program's failure is not
# printed. A program's `.check` is listed apart from the program, as `<name>.check`.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
fail=0
update=
add=
for arg in "$@"; do
  case $arg in
    --update) update=--update ;;
    --add) add=--add ;;
  esac
done
records=out/errors-tests
rm -rf "$records"
mkdir -p "$records"
: > "$records/results.txt"
declare -A listed
while IFS=$'\t' read -r listed_name _; do [ -n "$listed_name" ] && listed[$listed_name]=1; done < <(cat tests/pending/errors.txt 2> /dev/null)
# said <line>: a failure's line, unless the program is on the list
said() { [ -n "${listed[$name]:-}" ] || echo "$1"; }
for src in tests/errors/*.scala tests/errors/*/; do
  src=${src%/}
  name=$(basename "$src" .scala)
  why=
  flags=$(grep -h -o '^// teq: .*' "$src" "$src"/*.scala 2>/dev/null | head -1 | sed 's|^// teq: ||')
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    echo "skip $src: not in the coursier cache:$JARS_MISSING"
    printf 'skip %s\tnot in the coursier cache:%s\n' "$name" "$JARS_MISSING" >> "$records/results.txt"
    continue
  fi
  [ -n "$JARS_CP" ] && flags="$flags --classpath $JARS_CP"
  command=$(grep -h -o '^// command: .*' "$src" "$src"/*.scala 2>/dev/null | head -1 | sed 's|^// command: ||')
  [ "$command" = build ] && mkdir -p out && flags="$flags -o out/errors.js"
  nojdk=()
  grep -q '^// jdk: none' "$src" "$src"/*.scala 2>/dev/null && nojdk=(env -u JAVA_HOME PATH=/nonexistent)
  out=$(timeout 20 "${nojdk[@]}" "$TEQ" compiler ${command:-check} "$src" $flags 2>&1)
  code=$?
  printf 'exit %s\n%s\n' "$code" "$out" > "$records/$(basename "$src" .scala).out"
  if [ $code -ne 1 ]; then
    said "FAIL $src: exit code $code"
    why=${why:-"exit code $code"}
  fi
  while IFS= read -r line; do
    expected=${line#// expect: }
    if ! grep -qF -- "$expected" <<< "$out"; then
      said "FAIL $src: missing '$expected'"
      why=${why:-"missing '$expected'"}
    fi
  done < <(grep -h '^// expect: ' "$src" "$src"/*.scala 2>/dev/null)
  while IFS= read -r line; do
    absent=${line#// absent: }
    if grep -qF -- "$absent" <<< "$out"; then
      said "FAIL $src: present '$absent'"
      why=${why:-"present '$absent'"}
    fi
  done < <(grep -h '^// absent: ' "$src" "$src"/*.scala 2>/dev/null)
  # A `<name>.check` next to a program is its complete diagnostics, compared whole: a message's
  # every line, its blank ones included, which a line's `// expect:` cannot place.
  check=${src%.scala}.check
  # The whole output against the program's .check is a test of its own, `<name>.check`: a check that does not
  # hold leaves the program's exit and messages held.
  if [ -f "$check" ]; then
    if diff -q "$check" - <<< "$out" > /dev/null; then
      printf 'pass %s.check\n' "$name" >> "$records/results.txt"
    else
      if [ -z "${listed[$name.check]:-}" ]; then
        echo "FAIL $src: the diagnostics are not $check"
        diff "$check" - <<< "$out" | head -20
      fi
      printf 'fail %s.check\t%s\n' "$name" "$(diff "$check" - <<< "$out" | grep -m1 '^[<>]' | tr '\t' ' ' | cut -c1-200)" >> "$records/results.txt"
    fi
  fi
  if [ -n "$why" ]; then
    printf 'fail %s\t%s\n' "$name" "$(tr '\t\n' '  ' <<< "$why" | cut -c1-200)" >> "$records/results.txt"
  else
    printf 'pass %s\n' "$name" >> "$records/results.txt"
  fi
done
# The programs held to the list of those expected to fail.
python3 tests/support/pending.py errors "$records/results.txt" $update $add || fail=1
# The unused-import warning against scalac's on dotty's corpus and the probes (its own script).
unused=$(./tests/warnings/unused-imports.sh 2>&1)
[ $? -ne 0 ] && fail=1
grep -v '^unused imports: ' <<< "$unused"
unused_summary=$(tail -1 <<< "$unused")
# The warnings about the cases of matches against scalac's on their probes (its own script).
matches=$(./tests/warnings/matches.sh 2>&1)
[ $? -ne 0 ] && fail=1
grep -v '^match warnings: ' <<< "$matches"
matches_summary=$(tail -1 <<< "$matches")
# The reporting policy of warnings against scalac's: flags, -Wconf, @nowarn, exits (its own script).
policy=$(./tests/warnings/policy.sh 2>&1)
[ $? -ne 0 ] && fail=1
grep -v '^warning policy: ' <<< "$policy"
policy_summary=$(tail -1 <<< "$policy")
# The unused warnings of every kind against scalac's on dotty's corpus (its own script).
kinds=$(./tests/warnings/unused-kinds.sh 2>&1)
[ $? -ne 0 ] && fail=1
grep -v '^unused kinds: ' <<< "$kinds"
kinds_summary=$(tail -1 <<< "$kinds")
# The diagnostics of attached workers against one worker's (its own script).
workers=$(./tests/warnings/workers.sh 2>&1)
[ $? -ne 0 ] && fail=1
grep -v "^workers' diagnostics: " <<< "$workers"
workers_summary=$(tail -1 <<< "$workers")
# The source version against both scalacs' captures: settings, imports, directives, the migration table's rows
# (its own script, under the binary it tests).
source=$(TEQ=$TEQ ./teq interp tests/source-version.scala -- --teq "$TEQ" 2>&1)
[ $? -ne 0 ] && fail=1
grep -v '^source version: ' <<< "$source"
source_summary=$(tail -1 <<< "$source")
if [ $fail = 0 ]; then echo "error tests passed; $unused_summary; $kinds_summary; $matches_summary; $policy_summary; $workers_summary; $source_summary"; else echo "error tests failed; $unused_summary; $kinds_summary; $matches_summary; $policy_summary; $workers_summary; $source_summary"; fi
exit $fail
