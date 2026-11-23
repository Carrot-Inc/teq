#!/bin/bash
# tests/support/export-inline.sh scalac <cells dir> | teq <cells dir> <tag> <teq binary>: the runs of the table of
# tests/support/export_inline.py (`export_inline.py cells <dir>` writes the cells, `export_inline.py judge <dir>
# <tag>...` judges the runs). Not a suite of the gate: a table run by hand, about 15 minutes for scalac's side.
#   scalac: each cell's three modules compiled apart by scalac 3.8.4 (scala-cli: m1, m2 over m1's classes, m3 over
#     both) and the three whole, each run on the JVM; writes <cells>/scalac.tsv, `cell <TAB> leg <TAB> ok:<output> |
#     err:<first error line>`, the legs `sep` and `whole`.
#   teq: the binary's legs: `whole` (the interpreter over the three files), `whole-jvm` (one -o jar run on the
#     JVM), `whole-js` (one -o JavaScript build run by node), and over products, each module apart: `lean` (check
#     --products, the interpreter over m1's and m2's), `js` (-o over the same products, run by node), `jvm` (--target
#     jvm --products, m3's -o jar run on the JVM over m1's and m2's class directories), `m23-lean`, `m23-js`,
#     `m23-jvm` (m2 and m3 built together over m1's products, as the three before), `readtasty` (scalac 3.8.4
#     reading m1's and m2's pickles of those class directories, tests/tasty/fromtasty/Gate.scala: every right-hand
#     side forced and the tree checker run, `ok:read` where both are read); writes <cells>/<tag>.tsv the same way.
# Each command bounded (120 s; a scalac compilation and a run 300 s and 60 s); several output lines joined by '|'.
set -u
mode=$1; C=$(cd "$2" && pwd)
cd "$(dirname "$0")/../.." || exit 1
root=$PWD
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
  if timeout 120 "$B" compiler build -o "$o/whole.js" m1/A.scala m2/U.scala m3/B.scala > /dev/null 2> "$o/whole-js.err"; then
    timeout 60 node "$o/whole.js" > "$o/whole-js.out" 2>> "$o/whole-js.err"
    row "$c" whole-js "$(verdict $? "$o/whole-js.out" "$o/whole-js.err")"
  else row "$c" whole-js "err:$(first_error "$o/whole-js.err")"; fi
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
  # m2 and m3 together over m1's lean products: the forwarder and its call in one build, the member loaded.
  if [ -d "$o/x1" ]; then
    timeout 120 "$B" interp --classpath "$o/x1" m2/U.scala m3/B.scala > "$o/m23-lean.out" 2> "$o/m23-lean.err"
    row "$c" m23-lean "$(verdict $? "$o/m23-lean.out" "$o/m23-lean.err")"
    if timeout 120 "$B" compiler build -o "$o/m23.js" --classpath "$o/x1" m2/U.scala m3/B.scala > /dev/null 2> "$o/m23-js.err"; then
      timeout 60 node "$o/m23.js" > "$o/m23-js.out" 2>> "$o/m23-js.err"
      row "$c" m23-js "$(verdict $? "$o/m23-js.out" "$o/m23-js.err")"
    else row "$c" m23-js "err:$(first_error "$o/m23-js.err")"; fi
  else
    e=$(first_error "$o/x1.err" 2> /dev/null)
    row "$c" m23-lean "err:upstream: $e"; row "$c" m23-js "err:upstream: $e"
  fi
  # Over the JVM's products: class directories.
  if timeout 120 "$B" compiler build --target jvm --products "$o/y1" --classpath "$SL" m1/A.scala > /dev/null 2> "$o/y1.err"; then
    if timeout 120 "$B" compiler build --target jvm -o "$o/m23.jar" --classpath "$o/y1:$SL" m2/U.scala m3/B.scala > /dev/null 2> "$o/m23-jvm.err"; then
      timeout 60 java -cp "$o/m23.jar:$o/y1:$SL" TeqMain > "$o/m23-jvm.out" 2>> "$o/m23-jvm.err"
      row "$c" m23-jvm "$(verdict $? "$o/m23-jvm.out" "$o/m23-jvm.err")"
    else row "$c" m23-jvm "err:$(first_error "$o/m23-jvm.err")"; fi
  else row "$c" m23-jvm "err:upstream: $(first_error "$o/y1.err")"; fi
  if [ -d "$o/y1" ] \
     && timeout 120 "$B" compiler build --target jvm --products "$o/y2" --classpath "$o/y1:$SL" m2/U.scala > /dev/null 2> "$o/y2.err"; then
    if timeout 120 "$B" compiler build --target jvm -o "$o/m3.jar" --classpath "$o/y1:$o/y2:$SL" m3/B.scala > /dev/null 2> "$o/jvm.err"; then
      timeout 60 java -cp "$o/m3.jar:$o/y1:$o/y2:$SL" TeqMain > "$o/jvm.out" 2>> "$o/jvm.err"
      row "$c" jvm "$(verdict $? "$o/jvm.out" "$o/jvm.err")"
    else row "$c" jvm "err:$(first_error "$o/jvm.err")"; fi
    # scalac's reader over both modules' pickles, every cell's jobs in one JVM after the loop.
    for m in 1 2; do
      cp=$o/y1; [ $m = 2 ] && cp=$o/y1:$o/y2
      printf 'read\t%s\t%s\t%s\n' "$c m$m" "$cp" "$(find "$o/y$m" -name '*.tasty' | sort | tr '\n' '\t' | sed 's/\t$//')" >> "$out/readtasty.jobs"
    done
  else
    e=$(cat "$o/y1.err" "$o/y2.err" 2> /dev/null | first_error /dev/stdin)
    row "$c" jvm "err:upstream: $e"; row "$c" readtasty "err:upstream: $e"
  fi
done
if [ -s "$out/readtasty.jobs" ]; then
  cd "$root" || exit 1
  . tests/support/gate-reader.sh
  # EXPORT_READER_WORKSPACE: a workspace of the reader compiled before, shared by runs over several cells directories.
  if reader=$(gate_reader "${EXPORT_READER_WORKSPACE:-$out/reader}" 2> "$out/readtasty.err"); then
    (cd "$out" && timeout 300 java -XX:-UsePerfData -cp "$reader" gate readtasty.jobs > readtasty.out 2>> readtasty.err)
  fi
  for c in $(cut -f2 "$out/readtasty.jobs" | sed 's/ m[12]$//' | uniq); do
    if grep -qxF "ok $c m1" "$out/readtasty.out" 2> /dev/null && grep -qxF "ok $c m2" "$out/readtasty.out"; then row "$c" readtasty "ok:read"
    else
      why=$(awk -v a="FAIL $c m1" -v b="FAIL $c m2" '$0 == a || $0 == b { on = 1; print; next } /^(ok|FAIL) / { on = 0 } on' "$out/readtasty.out" 2> /dev/null | grep -v '^FAIL' | head -1 | cut -c1-240 | tr '\t' ' ')
      row "$c" readtasty "err:${why:-no answer from the reader of scalac: $(head -1 "$out/readtasty.err" 2> /dev/null)}"
    fi
  done
fi
echo "export-inline teq $tag: $(wc -l < "$C/$tag.tsv") rows"
