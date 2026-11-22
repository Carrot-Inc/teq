#!/bin/bash
# tests/init-view/quotes.sh <teq>: the table of the `Quotes` a macro's pickled trees select the reflection API on
# (tests/init-view/quotes.py). Each cell's macro file alone through tests/init-view-corpus.sh (scalac 3.8.4's own pickle
# and its view, teq's products and their view), then each cell run by each compiler: the macro, then its client
# against it, then `java Test` over the classes; then the table. Every step is bounded: the corpus runner's own, and
# here scalac 300 s and teq 120 s a compilation, the program 60 s, a pickle's signatures 60 s; a step past its bound is
# NOT JUDGED and named. Each step's exit beside its output (`<output>.exit`, the last step's), which the judge tells a
# refusal from a missing, cut or bounded record by, and the output's sum after each step that writes to it
# (`<output>.sum`, tests/init-view/records.sh), which the judge holds the output to. Exits 1 when the table has a cell
# that differs or is not judged, or when the judge's self-test (`quotes.py --self-test`, run first) fails. Writes under
# out/quotes and out/init-view-corpus/quotes. Env: COURSIER, SCALAC_LOCK (a lock file for flock around scalac).
cd "$(dirname "$0")/../.."
. tests/init-view/records.sh
# The judge on planted records first: a judge that passes a bad record judges nothing.
python3 tests/init-view/quotes.py --self-test || exit 1
abs() { echo "$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"; }
teq=$(abs "${1:?the teq}")
out=out/quotes
library=$("${COURSIER:-cs}" fetch --classpath --intransitive org.scala-lang:scala-library:3.8.4 2> /dev/null | tail -1)
compiler=$("${COURSIER:-cs}" fetch --classpath org.scala-lang:scala3-compiler_3:3.8.4 2> /dev/null | tail -1)
[ -n "$library" ] && [ -n "$compiler" ] || { echo "quotes: scala 3.8.4 not fetched"; exit 1; }
python3 tests/init-view/quotes.py generate "$out/cells" || exit 1
# The view: the macro file alone, the client left out (it expands the macro).
rm -rf "${out:?}/macros" "${out:?}/run"
mkdir -p "$out/macros"
for f in "$out"/cells/*_1.scala; do cp "$f" "$out/macros/$(basename "$f" _1.scala).scala"; done
TEQ=$teq bash tests/init-view-corpus.sh "$out/macros" quotes > "$out/corpus.log" 2>&1
tail -3 "$out/corpus.log"
# bounded <seconds> <log> <command...>: the command's output appended to the log, its exit in `<log without its
# extension>.exit` and the log's sum in `<log without its extension>.sum`; a step past its bound names the cell.
bounded() {
  local secs=$1 log=$2
  shift 2
  timeout "$secs" "$@" >> "$log" 2>&1
  local rc=$?
  echo $rc > "${log%.*}.exit"
  record_seal "$(dirname "$log")" "$log"
  [ $rc = 124 ] && echo "$(basename "$log") step over $secs s: $*" >> "$(dirname "$log")/not-judged"
  return $rc
}
lock=()
[ -n "$SCALAC_LOCK" ] && lock=(flock "$SCALAC_LOCK")
for f in "$out"/cells/*_1.scala; do
  stem=$(basename "$f" _1.scala)
  d=$out/run/$stem
  mkdir -p "$d/scalac" "$d/teq"
  macro=$(abs "$f")
  client=$(abs "$out/cells/${stem}_2.scala")
  # The parameters' types as each pickle of the macro file has them: the view prints no type tree's insides.
  w=out/init-view-corpus/quotes/$stem
  [ -f "$w/oracle/classes/Macro.tasty" ] && bounded 60 "$d/scalac-signatures.txt" "$teq" tasty --full "$w/oracle/classes/Macro.tasty"
  [ -f "$w/p/Macro.tasty" ] && bounded 60 "$d/teq-signatures.txt" "$teq" tasty --full "$w/p/Macro.tasty"
  # Each compiler's build of the macro file is a step of its own: its exit and log on their own record
  # (`<side>-macro.exit`, `<side>-macro.out`), which alone tell a refusal of a cell scalac refuses (a later step's
  # failure is no refusal of the macro), and the run's log from its start (`<side>.out`, the last step's exit).
  # scalac: each file a compilation of its own, the client against the macro's classes, as dotty's run tests compile
  # `_1` and `_2`.
  "${lock[@]}" timeout 300 java -Xss2m -cp "$compiler" dotty.tools.dotc.Main -classpath "$library" -color:never -d "$d/scalac" "$macro" >> "$d/scalac-macro.out" 2>&1
  rc=$?
  echo $rc > "$d/scalac-macro.exit"
  record_seal "$d" "$d/scalac-macro.out"
  cat "$d/scalac-macro.out" >> "$d/scalac.out"
  [ $rc = 0 ] && "${lock[@]}" timeout 300 java -Xss2m -cp "$compiler" dotty.tools.dotc.Main -classpath "$d/scalac:$library" -color:never -d "$d/scalac" "$client" >> "$d/scalac.out" 2>&1
  [ $rc = 0 ] && rc=$?
  echo $rc > "$d/scalac.exit"
  record_seal "$d" "$d/scalac.out"
  [ $rc = 124 ] && echo "a compilation of scalac's over 300 s" >> "$d/not-judged"
  [ $rc = 0 ] && bounded 60 "$d/scalac.out" java -cp "$d/scalac:$library" Test
  bounded 120 "$d/teq-macro.out" "$teq" compiler build "$macro" --target jvm --all-mains --products "$d/teq" --classpath "$library"
  rc=$?
  cat "$d/teq-macro.out" >> "$d/teq.out"
  echo $rc > "$d/teq.exit"
  record_seal "$d" "$d/teq.out"
  [ $rc = 0 ] && bounded 120 "$d/teq.out" "$teq" compiler build "$client" --target jvm --all-mains --products "$d/teq" --classpath "$d/teq:$library" &&
    bounded 60 "$d/teq.out" java -cp "$d/teq:$library" Test
done
timeout 600 python3 tests/init-view/quotes.py judge out/init-view-corpus/quotes "$out/run" > "$out/table.md"
rc=$?
grep -E '^quotes:|^MANIFEST' "$out/table.md"
exit $rc
