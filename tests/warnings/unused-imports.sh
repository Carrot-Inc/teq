#!/bin/bash
# The unused-import warning against scalac 3.8.4: every file of dotty's tests/warn that names
# -Wunused:imports or -Wunused:all ($SCALA3/tests/warn) and every probe of tests/warnings/probes is
# checked with teq under --wunused imports, and teq's `unused import` lines (`line:col:width`, the
# selector's span) are compared with scalac's own, pinned in tests/warnings/expected by
# tests/warnings/capture-unused.sh. Like scalac's (scala-cli's) the build is a JVM one against
# scala-library 3.8.4 from the coursier cache, whose collections and JDK classes the files use. A file whose lines differ fails unless
# tests/warnings/unused-departures.txt names it with its reason; a named file that matches fails
# too, so that the list stays the departures. A probe may be a directory, one program of several
# files; each probe is checked at one, two and sixteen workers, each run as scalac. Run by
# tests/run_errors.sh; the last line gives the counts. Without the checkout's tests/warn the
# corpus is skipped and the probes run.
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
departures=tests/warnings/unused-departures.txt
records=out/unused-imports
rm -rf "$records"
mkdir -p "$records/probes"
fail=0
matched=0
known=0
total=0
compare() {
  local src=$1 expected=$2 name=$3 threads=${4:-}
  total=$((total + 1))
  local flags=""
  [ -f "$src" ] && flags=$(python3 tests/warnings/options.py teq < "$src")
  [ -n "$threads" ] && flags="$flags --threads $threads"
  local out
  out=$(timeout 60 "$TEQ" compiler check "$src" --wunused imports --target jvm --std scala-library --classpath "$library" $flags 2>&1)
  printf '%s\n' "$out" > "$records/$name.out"
  local got
  got=$(python3 tests/warnings/parse.py teq "$src" <<< "$out")
  local listed
  listed=$(grep -E "^${name%@*}: " "$departures" 2> /dev/null | head -1)
  if [ "$got" = "$(cat "$expected")" ]; then
    if [ -n "$listed" ]; then
      echo "FAIL $name: matches scalac but is listed as a departure"
      fail=1
    else
      matched=$((matched + 1))
    fi
  elif [ -n "$listed" ]; then
    known=$((known + 1))
  else
    echo "FAIL $name: teq's unused imports differ from scalac's ($src)"
    diff <(cat "$expected") <(printf '%s\n' "$got" | sed '/^$/d') | sed 's/^/  /'
    fail=1
  fi
}
corpus=0
if [ -d "$SCALA3/tests/warn" ]; then
  for expected in tests/warnings/expected/*.txt; do
    name=$(basename "$expected" .txt)
    compare "$SCALA3/tests/warn/$name.scala" "$expected" "$name"
    corpus=$((corpus + 1))
  done
else
  echo "skip the corpus: no $SCALA3/tests/warn"
fi
probes=0
for expected in tests/warnings/expected/probes/*.txt; do
  name=$(basename "$expected" .txt)
  probes=$((probes + 1))
  src=tests/warnings/probes/$name.scala
  [ -d "tests/warnings/probes/$name" ] && src=tests/warnings/probes/$name
  for threads in 1 2 16; do
    compare "$src" "$expected" "probes/$name@$threads" "$threads"
  done
done
summary="unused imports: $matched of $total checks as scalac ($corpus files of dotty's tests/warn, $probes probes at 1, 2 and 16 workers), $known departures listed"
if [ $fail = 0 ]; then echo "$summary"; else echo "$summary; failed"; fi
exit $fail
