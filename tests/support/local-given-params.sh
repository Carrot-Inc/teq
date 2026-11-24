#!/bin/bash
# tests/support/local-given-params.sh scalac <cells dir> [<first> <last>] | teq <cells dir> <tag> <teq binary> [<first>
# <last>]: the runs of the table of tests/support/local_given_params.py (`local_given_params.py cells <dir>` writes the
# cells, `local_given_params.py judge <dir> <base> <head>` judges the runs), the cells of cells.txt's lines <first> to
# <last> (all by default). Not a suite of the gate: a table run by hand.
#   scalac: each cell's Main.scala compiled by scalac 3.8.4 (scala-cli) and run on the JVM, the leg `run`; appends to
#     <cells>/scalac.tsv `cell <TAB> run <TAB> ok:<output> | err:<line>:<col>: <message>`, the first error scalac
#     reports where it does not compile.
#   teq: the binary's legs `interp` (the interpreter), `jvm` (an -o jar run on the JVM), `js` (an -o build run by
#     node), each its output or its first error as scalac's is written; appends to <cells>/<tag>.tsv.
# Each command bounded (120 s; a scalac compilation 300 s, a run 60 s); several output lines joined by '|'.
set -u
mode=$1; C=$(cd "$2" && pwd)
cd "$(dirname "$0")/../.." || exit 1
. tests/support/jars.sh
SL=$(jar_of scala-library)
[ -f "$SL" ] || { echo "local-given-params: scala-library is not in the coursier cache"; exit 1; }
if [ "$mode" = scalac ]; then first=${3:-1}; last=${4:-100000}; else first=${5:-1}; last=${6:-100000}; fi
list=$(sed -n "${first},${last}p" "$C/cells.txt")
# scalac's first diagnostic as its reporter writes it (colours set aside): `-- [E172] Type Error: <path>/Main.scala:<line>:<col>`,
# its column counted from 0 (the caret's), written from 1 as teq's; the source line, the caret, and the message's first
# line after `|`.
scalac_error() {
  sed 's/\x1b\[[0-9;]*m//g' "$1" | awk '
    !pos && /^-- .*Main\.scala:[0-9]+:[0-9]+/ { n = split($0, f, ":"); pos = f[n-1] ":" (f[n] + 1); next }
    pos && /^ *\| *\^/ { caret = 1; next }
    pos && caret && /^ *\|/ { sub(/^ *\| */, ""); print pos ": " $0; exit }' | cut -c1-240 | tr '\t' ' '
}
# teq's: `Main.scala:<line>:<col>: error: <message>`; another failure's first line where none is.
teq_error() {
  local e
  e=$(grep -m1 -E '^Main\.scala:[0-9]+:[0-9]+: error: ' "$1" | sed -E 's/^Main\.scala:([0-9]+):([0-9]+): error: /\1:\2: /')
  [ -n "$e" ] || e=$(grep -m1 -iE 'error|exception' "$1")
  printf '%s' "$e" | cut -c1-240 | tr '\t' ' '
}
ok_line() { echo "ok:$(tr '\n' '|' < "$1" | sed 's/|$//')"; }
if [ "$mode" = scalac ]; then
  out=$C/scalac-out
  mkdir -p "$out"
  for c in $list; do
    o=$out/$c; mkdir -p "$o"
    if (cd "$C/$c" && timeout 300 scala-cli compile -S 3.8.4 --server=false -q --compilation-output "$o/classes" Main.scala) > "$o/compile.log" 2>&1; then
      if timeout 60 java -cp "$SL:$o/classes" run > "$o/run.out" 2> "$o/run.err"; then v=$(ok_line "$o/run.out"); else v="err:$(grep -m1 -iE 'error|exception' "$o/run.err" | cut -c1-240)"; fi
    else
      v="err:$(scalac_error "$o/compile.log")"
    fi
    printf '%s\trun\t%s\n' "$c" "$v" >> "$C/scalac.tsv"
  done
  echo "local-given-params scalac: cells $first-$last, $(wc -l < "$C/scalac.tsv") rows in all"
  exit 0
fi
tag=$3; B=$(cd "$(dirname "$4")" && pwd)/$(basename "$4")
out=$C/teq-$tag
mkdir -p "$out"
row() { printf '%s\t%s\t%s\n' "$1" "$2" "$3" >> "$C/$tag.tsv"; }
for c in $list; do
  o=$out/$c; mkdir -p "$o"
  cd "$C/$c" || exit 1
  if timeout 120 "$B" interp Main.scala > "$o/interp.out" 2> "$o/interp.err"; then row "$c" interp "$(ok_line "$o/interp.out")"; else row "$c" interp "err:$(teq_error "$o/interp.err")"; fi
  if timeout 120 "$B" compiler build --target jvm -o "$o/m.jar" --classpath "$SL" Main.scala > /dev/null 2> "$o/jvm.err"; then
    if timeout 60 java -cp "$o/m.jar:$SL" TeqMain > "$o/jvm.out" 2>> "$o/jvm.err"; then row "$c" jvm "$(ok_line "$o/jvm.out")"; else row "$c" jvm "err:$(teq_error "$o/jvm.err")"; fi
  else row "$c" jvm "err:$(teq_error "$o/jvm.err")"; fi
  if timeout 120 "$B" compiler build -o "$o/m.js" Main.scala > /dev/null 2> "$o/js.err"; then
    if timeout 60 node "$o/m.js" > "$o/js.out" 2>> "$o/js.err"; then row "$c" js "$(ok_line "$o/js.out")"; else row "$c" js "err:$(teq_error "$o/js.err")"; fi
  else row "$c" js "err:$(teq_error "$o/js.err")"; fi
done
echo "local-given-params teq $tag: cells $first-$last, $(wc -l < "$C/$tag.tsv") rows in all"
