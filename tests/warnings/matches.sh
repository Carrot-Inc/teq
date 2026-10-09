#!/bin/bash
# The warnings about the cases of matches against scalac 3.8.4's: each probe of
# tests/warnings/matches is checked with teq on the JavaScript target (teq's std) and on the JVM
# against scala-library 3.8.4 from the coursier cache, whose collections the probes' overloads
# come from there, and teq's exhaustivity, unreachable-case and not-a-partial-function warnings
# (`line:col kind`, tests/warnings/parse_matches.py) are compared with scalac's, pinned in
# tests/warnings/expected/matches by tests/warnings/capture-matches.sh: a warning missing, one
# scalac does not give (its absence is what most probes assert), one at another place or one
# given more often fails; that the comparison counts a repeated warning is checked on a probe's
# own output with its first warning doubled. Run by tests/run_errors.sh; the last line gives the
# counts. Without the jar the JVM checks are skipped.
cd "$(dirname "$0")/../.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
library=$(scala_library_jar)
records=out/match-warnings
rm -rf "$records"
mkdir -p "$records"
fail=0
matched=0
total=0
for expected in tests/warnings/expected/matches/*.txt; do
  name=$(basename "$expected" .txt)
  src=tests/warnings/matches/$name.scala
  for target in js jvm; do
    flags="--target js"
    if [ $target = jvm ]; then
      [ -f "$library" ] || continue
      flags="--target jvm --std scala-library --classpath $library"
    fi
    total=$((total + 1))
    out=$(timeout 60 "$TEQ" compiler check "$src" $flags 2>&1)
    printf '%s\n' "$out" > "$records/$name.$target.out"
    got=$(python3 tests/warnings/parse_matches.py teq "$src" <<< "$out")
    if [ "$got" = "$(cat "$expected")" ]; then
      matched=$((matched + 1))
    else
      echo "FAIL $name ($target): teq's match warnings differ from scalac's ($src)"
      diff <(cat "$expected") <(printf '%s\n' "$got" | sed '/^$/d') | sed 's/^/  /'
      fail=1
    fi
  done
done
# The comparison counts a warning given twice: pf_unreachable's output with its first warning
# repeated (the message, its source line and its carets) differs from scalac's.
record=$records/pf_unreachable.js.out
doubled=$({ cat "$record"; grep -m1 -A2 ': warning: ' "$record"; } | python3 tests/warnings/parse_matches.py teq tests/warnings/matches/pf_unreachable.scala)
repeat="a repeated warning told apart"
if ! grep -q ': warning: ' "$record" || [ "$doubled" = "$(cat tests/warnings/expected/matches/pf_unreachable.txt)" ]; then
  echo "FAIL the comparison does not count a warning given twice ($record)"
  repeat="a repeated warning not told apart"
  fail=1
fi
summary="match warnings: $matched of $total checks as scalac, $repeat"
if [ $fail = 0 ]; then echo "$summary"; else echo "$summary; failed"; fi
exit $fail
