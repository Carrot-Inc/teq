#!/bin/bash
# The unused warnings of every kind against scalac 3.8.4 (src/typer/unused_defs.rs and
# src/typer/unused.rs): every file of dotty's tests/warn that names -Wunused ($SCALA3/tests/warn)
# is checked with teq under the -Wunused kinds, -Wconf and -language options its `//> using
# options` lines give (tests/warnings/kinds.py), and teq's unused warnings (`line:col:width message`:
# the imports, privates, locals, parameters, pattern variables, the variables never assigned or
# never read, the unused `@nowarn`) are compared with scalac's own, pinned in
# tests/warnings/expected/kinds by tests/warnings/capture-kinds.sh. As tests/warnings/unused-imports.sh,
# the build is a JVM one against scala-library 3.8.4 from the coursier cache, and a file whose
# lines differ fails unless tests/warnings/unused-kinds-departures.txt names it with its reason (a
# named file that matches fails too). Each file is checked at one, two and sixteen workers, each run
# as scalac. Run by tests/run_errors.sh; the last line gives the counts.
# Without the checkout's tests/warn the suite is skipped.
cd "$(dirname "$0")/../.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
library=$(jar_of scala-library)
if [ ! -f "$library" ]; then
  echo "skip: scala-library 3.8.4 is not in the coursier cache ($library)"
  exit 0
fi
if [ -z "$SCALA3" ] && common=$(git rev-parse --path-format=absolute --git-common-dir 2> /dev/null); then
  SCALA3="$(dirname "$common")/../teq-ref/scala3"
fi
if [ ! -d "$SCALA3/tests/warn" ]; then
  echo "skip: no $SCALA3/tests/warn"
  exit 0
fi
departures=tests/warnings/unused-kinds-departures.txt
records=out/unused-kinds
rm -rf "$records"
mkdir -p "$records"
fail=0
matched=0
known=0
total=0
files=0
for expected in tests/warnings/expected/kinds/*.txt; do
  name=$(basename "$expected" .txt)
  src=$SCALA3/tests/warn/$name.scala
  flags=$(python3 tests/warnings/kinds.py options teq < "$src")
  listed=$(grep -E "^${name}: " "$departures" 2> /dev/null | head -1)
  files=$((files + 1))
  # The marks of the workers that typed the bodies, merged, give the warnings one does.
  for threads in 1 2 16; do
    total=$((total + 1))
    out=$(eval timeout 60 "$TEQ" compiler check "$src" --target jvm --std scala-library --classpath "$library" "$flags" --threads $threads 2>&1)
    printf '%s\n' "$out" > "$records/$name@$threads.out"
    got=$(python3 tests/warnings/kinds.py teq "$src" <<< "$out")
    if [ "$got" = "$(cat "$expected")" ]; then
      if [ -n "$listed" ]; then
        echo "FAIL $name@$threads: matches scalac but is listed as a departure"
        fail=1
      else
        matched=$((matched + 1))
      fi
    elif [ -n "$listed" ]; then
      known=$((known + 1))
    else
      echo "FAIL $name@$threads: teq's unused warnings differ from scalac's ($src)"
      diff <(cat "$expected") <(printf '%s\n' "$got" | sed '/^$/d') | sed 's/^/  /'
      fail=1
    fi
  done
done
summary="unused kinds: $matched of $total checks as scalac ($files files of dotty's tests/warn at 1, 2 and 16 workers), $known departures listed"
if [ $fail = 0 ]; then echo "$summary"; else echo "$summary; failed"; fi
exit $fail
