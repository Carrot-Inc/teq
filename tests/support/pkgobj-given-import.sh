#!/bin/bash
# tests/support/pkgobj-given-import.sh scalac <cells dir> [<first> <last>] | teq <cells dir> <tag> <teq binary> [<first>
# <last>] | over <cells dir> <tag>: the runs of the table of tests/support/pkgobj_given_import.py (`pkgobj_given_import.py cells <dir>` writes the
# cells, `pkgobj_given_import.py judge <dir> <base> <head>` judges the runs), the cells of cells.txt's lines <first> to
# <last> (all by default). Not a suite of the gate: a table run by hand. The cells of tests/support/export_given_body.py
# and tests/support/singleton_cycles.py run by it too.
#   scalac: tests/support/scalac_cells.scala, scalac 3.8.4 over the cells in one JVM, each WHOLE and SPLIT, appending to
#     <cells>/scalac.tsv.
#   over: scalac 3.8.4 compiling each cell's user over teq's library, the products run <tag>'s split build left, and
#     running it beside them (scalac_cells.scala --over), appending to <cells>/scalac-over-<tag>.tsv.
#   teq: per cell, WHOLE (lib/ and use/ in one build) and SPLIT (lib/ built to products first, use/ against them), the
#     legs `interp` (the interpreter), `jvm` (an -o jar run on the JVM), `js` (an -o build run by node; the split's
#     library products a check's pickles), each its output or its first error as scalac's is written (`<file>:<line>:
#     <col>: <message>`, the file's name alone); appends to <cells>/<tag>.tsv.
# Each command bounded (a build 120 s, a run 60 s, or CELLS_BUILD_BOUND and CELLS_RUN_BOUND), a leg its bound stopped
# `hang:<bound>s`; several output lines joined by '|'.
set -u
mode=$1; C=$(cd "$2" && pwd)
cd "$(dirname "$0")/../.." || exit 1
. tests/support/jars.sh
SL=$(jar_of scala-library)
[ -f "$SL" ] || { echo "pkgobj-given-import: scala-library is not in the coursier cache"; exit 1; }
if [ "$mode" = scalac ]; then
  timeout 1200 scala-cli run tests/support/scalac_cells.scala --server=false -q -- "$C" "$C/scalac-out" "$SL" ${3:+"$3"} ${4:+"$4"}
  exit $?
fi
if [ "$mode" = over ]; then
  timeout 1200 scala-cli run tests/support/scalac_cells.scala --server=false -q -- "$C" "$C/scalac-out" "$SL" --over "$3"
  exit $?
fi
first=${5:-1}; last=${6:-100000}
# The bounds of a build and of a run, seconds (CELLS_BUILD_BOUND, CELLS_RUN_BOUND): a short one for a binary that hangs.
BUILD_BOUND=${CELLS_BUILD_BOUND:-120}; RUN_BOUND=${CELLS_RUN_BOUND:-60}
list=$(sed -n "${first},${last}p" "$C/cells.txt")
tag=$3; B=$(cd "$(dirname "$4")" && pwd)/$(basename "$4")
out=$C/teq-$tag
mkdir -p "$out"
# teq's first error, `<path>:<line>:<col>: error: <message>`, its path's file name alone; another failure's first line
# where none is.
teq_error() {
  local e
  e=$(grep -m1 -E '^[^ ]+\.scala:[0-9]+:[0-9]+: error: ' "$1" | sed -E 's/^([^ ]*\/)?([^/ ]+\.scala):([0-9]+):([0-9]+): error: /\2:\3:\4: /')
  [ -n "$e" ] || e="-:-:-: $(grep -m1 -iE 'error|exception' "$1")"
  printf '%s' "$e" | cut -c1-240 | tr '\t' ' '
}
# A failed command's verdict: `hang:<bound>s` where its bound stopped it, else its first error.
failed() { if [ "$1" = 124 ]; then printf 'hang:%ss' "${3:-$BUILD_BOUND}"; else printf 'err:%s' "$(teq_error "$2")"; fi; }
ok_line() { echo "ok:$(tr '\n' '|' < "$1" | sed 's/|$//')"; }
row() { printf '%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$4" >> "$C/$tag.tsv"; }
# leg <cell> <build> <leg> <dir> <run command...>: the build was made (or failed) before; runs the program.
ran() {
  if timeout "$RUN_BOUND" "${@:5}" > "$4/$3.out" 2>> "$4/$3.err"; then row "$1" "$2" "$3" "$(ok_line "$4/$3.out")"; else e=$?; row "$1" "$2" "$3" "$(failed $e "$4/$3.err" "$RUN_BOUND")"; fi
}
for c in $list; do
  cd "$C/$c" || exit 1
  # WHOLE
  o=$out/$c/whole; mkdir -p "$o"
  if timeout "$BUILD_BOUND" "$B" interp lib use > "$o/interp.out" 2> "$o/interp.err"; then row "$c" whole interp "$(ok_line "$o/interp.out")"; else e=$?; row "$c" whole interp "$(failed $e "$o/interp.err")"; fi
  if timeout "$BUILD_BOUND" "$B" compiler build --target jvm -o "$o/m.jar" --classpath "$SL" lib use > /dev/null 2> "$o/jvm.err"; then
    ran "$c" whole jvm "$o" java -cp "$o/m.jar:$SL" TeqMain
  else e=$?; row "$c" whole jvm "$(failed $e "$o/jvm.err")"; fi
  if timeout "$BUILD_BOUND" "$B" compiler build -o "$o/m.js" lib use > /dev/null 2> "$o/js.err"; then
    ran "$c" whole js "$o" node "$o/m.js"
  else e=$?; row "$c" whole js "$(failed $e "$o/js.err")"; fi
  # SPLIT: the library's products, a JVM build's for the interpreter and the JVM, a check's pickles for JavaScript.
  o=$out/$c/split; mkdir -p "$o"
  if timeout "$BUILD_BOUND" "$B" compiler build --target jvm --all-mains --products "$o/lib" --classpath "$SL" lib > /dev/null 2> "$o/lib.err"; then
    if timeout "$BUILD_BOUND" "$B" interp --classpath "$o/lib:$SL" use > "$o/interp.out" 2> "$o/interp.err"; then row "$c" split interp "$(ok_line "$o/interp.out")"; else e=$?; row "$c" split interp "$(failed $e "$o/interp.err")"; fi
    if timeout "$BUILD_BOUND" "$B" compiler build --target jvm -o "$o/m.jar" --classpath "$o/lib:$SL" use > /dev/null 2> "$o/jvm.err"; then
      ran "$c" split jvm "$o" java -cp "$o/m.jar:$o/lib:$SL" TeqMain
    else e=$?; row "$c" split jvm "$(failed $e "$o/jvm.err")"; fi
  else
    e=$(failed $? "$o/lib.err"); row "$c" split interp "$e"; row "$c" split jvm "$e"
  fi
  if timeout "$BUILD_BOUND" "$B" compiler check --products "$o/libjs" lib > /dev/null 2> "$o/libjs.err"; then
    if timeout "$BUILD_BOUND" "$B" compiler build -o "$o/m.js" --classpath "$o/libjs" use > /dev/null 2> "$o/js.err"; then
      ran "$c" split js "$o" node "$o/m.js"
    else e=$?; row "$c" split js "$(failed $e "$o/js.err")"; fi
  else e=$?; row "$c" split js "$(failed $e "$o/libjs.err")"; fi
done
echo "pkgobj-given-import teq $tag: cells $first-$last, $(wc -l < "$C/$tag.tsv") rows in all"
