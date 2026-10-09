#!/bin/bash
# Every tests/errors/*.scala, and every directory under tests/errors (a program with the files
# next to it, such as a teq.toml), has to be rejected with all of its `// expect:` messages and
# none of its `// absent:` ones (a diagnostic scalac does not give, at its line and column). A
# `// teq: <flags>` line passes those flags to teq, a `// command: build` line builds the program
# in the place of checking it (for what only a build rejects), a `// jars: <names>` line puts those jars on
# the class path (tests/support/jars.sh); without one of them in the coursier cache the program
# is skipped. Each program's exit and complete diagnostics go to out/errors-tests/<name>.out, and
# the last line says whether every program was rejected as expected, with the counts of
# tests/warnings/unused-imports.sh and tests/warnings/matches.sh, which run after them.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
fail=0
records=out/errors-tests
rm -rf "$records"
mkdir -p "$records"
for src in tests/errors/*.scala tests/errors/*/; do
  src=${src%/}
  flags=$(grep -h -o '^// teq: .*' "$src" "$src"/*.scala 2>/dev/null | head -1 | sed 's|^// teq: ||')
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    echo "skip $src: not in the coursier cache:$JARS_MISSING"
    continue
  fi
  [ -n "$JARS_CP" ] && flags="$flags --classpath $JARS_CP"
  command=$(grep -h -o '^// command: .*' "$src" "$src"/*.scala 2>/dev/null | head -1 | sed 's|^// command: ||')
  [ "$command" = build ] && mkdir -p out && flags="$flags -o out/errors.js"
  out=$(timeout 20 "$TEQ" compiler ${command:-check} "$src" $flags 2>&1)
  code=$?
  printf 'exit %s\n%s\n' "$code" "$out" > "$records/$(basename "$src" .scala).out"
  if [ $code -ne 1 ]; then
    echo "FAIL $src: exit code $code"
    fail=1
  fi
  while IFS= read -r line; do
    expected=${line#// expect: }
    if ! grep -qF -- "$expected" <<< "$out"; then
      echo "FAIL $src: missing '$expected'"
      fail=1
    fi
  done < <(grep -h '^// expect: ' "$src" "$src"/*.scala 2>/dev/null)
  while IFS= read -r line; do
    absent=${line#// absent: }
    if grep -qF -- "$absent" <<< "$out"; then
      echo "FAIL $src: present '$absent'"
      fail=1
    fi
  done < <(grep -h '^// absent: ' "$src" "$src"/*.scala 2>/dev/null)
done
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
if [ $fail = 0 ]; then echo "error tests passed; $unused_summary; $matches_summary"; else echo "error tests failed; $unused_summary; $matches_summary"; fi
exit $fail
