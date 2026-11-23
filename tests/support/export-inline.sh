#!/bin/bash
# tests/support/export-inline.sh scalac <cells dir> | teq <cells dir> <tag> <teq binary>: the runs of the table of
# tests/support/export_inline.py (`export_inline.py cells <dir>` writes the cells, `export_inline.py judge <dir>
# <tag>...` judges the runs). Not a suite of the gate: a table run by hand, about 15 minutes for scalac's side.
#   scalac: each cell's three modules compiled apart by scalac 3.8.4 (scala-cli: m1, m2 over m1's classes, m3 over
#     both) and the three whole, each run on the JVM; writes <cells>/scalac.tsv, `cell <TAB> leg <TAB> ok:<output> |
#     err:<first error line>`, the legs `sep` and `whole`.
#   teq: the binary's legs: `whole` (the interpreter over the three files), `whole-jvm` (one -o jar run on the
#     JVM), and over products, each module apart: `lean` (check --products, the interpreter over m1's and m2's),
#     `js` (-o over the same products, run by node), `jvm` (--target jvm --products, m3's -o jar run on the JVM over
#     m1's and m2's class directories); writes <cells>/<tag>.tsv the same way.
# Each command bounded (120 s; a scalac compilation and a run 300 s and 60 s); several output lines joined by '|'.
set -u
mode=$1; C=$(cd "$2" && pwd)
cd "$(dirname "$0")/../.." || exit 1
. tests/support/jars.sh
SL=$(jar_of scala-library)
[ -f "$SL" ] || { echo "export-inline: scala-library is not in the coursier cache"; exit 1; }
first_error() { grep -m1 -iE 'error|exception' "$1" | cut -c1-240 | tr '\t' ' '; }
verdict() { # <exit> <out file> <err file>
  if [ "$1" = 0 ]; then echo "ok:$(tr '\n' '|' < "$2" | sed 's/|$//')"; else echo "err:$(first_error "$3")"; fi
}
if [ "$mode" = scalac ]; then
  out=$C/scalac-out
  rm -rf "${out:?}" && mkdir -p "$out"
  : > "$C/scalac.tsv"
  compile() { # <out dir> <class path> <files...>
    local d=$1 cp=$2; shift 2
    (cd "$C/$c" && timeout 300 scala-cli compile -S 3.8.4 --server=false -q --classpath "$cp" --compilation-output "$d" "$@") > "$d.log" 2>&1
  }
  for c in $(cat "$C/cells.txt"); do
    o=$out/$c
    if compile "$o-1" "$SL" m1/A.scala && compile "$o-2" "$SL:$o-1" m2/U.scala && compile "$o-3" "$SL:$o-1:$o-2" m3/B.scala; then
      timeout 60 java -cp "$SL:$o-1:$o-2:$o-3" app.run > "$o.sep.out" 2> "$o.sep.err"
      printf '%s\tsep\t%s\n' "$c" "$(verdict $? "$o.sep.out" "$o.sep.err")" >> "$C/scalac.tsv"
    else
      printf '%s\tsep\terr:%s\n' "$c" "$(cat "$o"-*.log | first_error /dev/stdin)" >> "$C/scalac.tsv"
    fi
    if compile "$o-w" "$SL" m1/A.scala m2/U.scala m3/B.scala; then
      timeout 60 java -cp "$SL:$o-w" app.run > "$o.whole.out" 2> "$o.whole.err"
      printf '%s\twhole\t%s\n' "$c" "$(verdict $? "$o.whole.out" "$o.whole.err")" >> "$C/scalac.tsv"
    else
      printf '%s\twhole\terr:%s\n' "$c" "$(first_error "$o-w.log")" >> "$C/scalac.tsv"
    fi
  done
  echo "export-inline scalac: $(wc -l < "$C/scalac.tsv") rows"
  exit 0
fi
tag=$3; B=$(cd "$(dirname "$4")" && pwd)/$(basename "$4")
out=$C/teq-$tag
rm -rf "${out:?}" && mkdir -p "$out"
: > "$C/$tag.tsv"
row() { printf '%s\t%s\t%s\n' "$1" "$2" "$3" >> "$C/$tag.tsv"; }
for c in $(cat "$C/cells.txt"); do
  o=$out/$c
  mkdir -p "$o"
  cd "$C/$c" || exit 1
  timeout 120 "$B" interp m1/A.scala m2/U.scala m3/B.scala > "$o/whole.out" 2> "$o/whole.err"
  row "$c" whole "$(verdict $? "$o/whole.out" "$o/whole.err")"
  if timeout 120 "$B" compiler build --target jvm -o "$o/whole.jar" --classpath "$SL" m1/A.scala m2/U.scala m3/B.scala > /dev/null 2> "$o/whole-jvm.err"; then
    timeout 60 java -cp "$o/whole.jar:$SL" TeqMain > "$o/whole-jvm.out" 2>> "$o/whole-jvm.err"
    row "$c" whole-jvm "$(verdict $? "$o/whole-jvm.out" "$o/whole-jvm.err")"
  else row "$c" whole-jvm "err:$(first_error "$o/whole-jvm.err")"; fi
  # Over the lean products (TASTy): the interpreter and JavaScript.
  if timeout 120 "$B" compiler check --products "$o/x1" m1/A.scala > /dev/null 2> "$o/x1.err" \
     && timeout 120 "$B" compiler check --products "$o/x2" --classpath "$o/x1" m2/U.scala > /dev/null 2> "$o/x2.err"; then
    timeout 120 "$B" interp --classpath "$o/x1:$o/x2" m3/B.scala > "$o/lean.out" 2> "$o/lean.err"
    row "$c" lean "$(verdict $? "$o/lean.out" "$o/lean.err")"
    if timeout 120 "$B" compiler build -o "$o/m3.js" --classpath "$o/x1:$o/x2" m3/B.scala > /dev/null 2> "$o/js.err"; then
      timeout 60 node "$o/m3.js" > "$o/js.out" 2>> "$o/js.err"
      row "$c" js "$(verdict $? "$o/js.out" "$o/js.err")"
    else row "$c" js "err:$(first_error "$o/js.err")"; fi
  else
    e=$(cat "$o/x1.err" "$o/x2.err" 2> /dev/null | first_error /dev/stdin)
    row "$c" lean "err:upstream: $e"; row "$c" js "err:upstream: $e"
  fi
  # Over the JVM's products: class directories.
  if timeout 120 "$B" compiler build --target jvm --products "$o/y1" --classpath "$SL" m1/A.scala > /dev/null 2> "$o/y1.err" \
     && timeout 120 "$B" compiler build --target jvm --products "$o/y2" --classpath "$o/y1:$SL" m2/U.scala > /dev/null 2> "$o/y2.err"; then
    if timeout 120 "$B" compiler build --target jvm -o "$o/m3.jar" --classpath "$o/y1:$o/y2:$SL" m3/B.scala > /dev/null 2> "$o/jvm.err"; then
      timeout 60 java -cp "$o/m3.jar:$o/y1:$o/y2:$SL" TeqMain > "$o/jvm.out" 2>> "$o/jvm.err"
      row "$c" jvm "$(verdict $? "$o/jvm.out" "$o/jvm.err")"
    else row "$c" jvm "err:$(first_error "$o/jvm.err")"; fi
  else row "$c" jvm "err:upstream: $(cat "$o/y1.err" "$o/y2.err" 2> /dev/null | first_error /dev/stdin)"; fi
done
echo "export-inline teq $tag: $(wc -l < "$C/$tag.tsv") rows"
