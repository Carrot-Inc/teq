#!/bin/bash
# tests/support/inline-getters.sh scalac <cells dir> | teq <cells dir> <tag> <teq binary> [<base tag>]: the runs of the
# table of tests/support/inline_getters.py (`inline_getters.py cells <dir>` writes the cells, `inline_getters.py judge
# <dir> <base> <head>` judges the runs). Not a suite of the gate: a table run by hand.
#   scalac: each cell's Main.scala compiled by scalac 3.8.4 (scala-cli) and run on the JVM, the leg `run`; `getters`,
#     every default getter of its class files (javap -p), `<class>.<method>` sorted; writes <cells>/scalac.tsv,
#     `cell <TAB> leg <TAB> ok:<output> | err:<first error line>`.
#   teq: the binary's legs `interp` (the interpreter), `jvm` (an -o jar run on the JVM), `js` (an -o build run by
#     node), `getters` (the -o jar's class files, as scalac's); with a base tag, `same` (inline_getters.py same: the
#     cell's class files against the base run's, byte for byte but the getters it adds); writes <cells>/<tag>.tsv.
# Each command bounded (120 s; a scalac compilation 300 s, a run 60 s); several output lines joined by '|'.
set -u
mode=$1; C=$(cd "$2" && pwd)
cd "$(dirname "$0")/../.." || exit 1
root=$PWD
. tests/support/jars.sh
SL=$(jar_of scala-library)
[ -f "$SL" ] || { echo "inline-getters: scala-library is not in the coursier cache"; exit 1; }
first_error() { grep -m1 -iE 'error|exception' "$1" | cut -c1-240 | tr '\t' ' '; }
verdict() { # <exit> <out file> <err file>
  if [ "$1" = 0 ]; then echo "ok:$(tr '\n' '|' < "$2" | sed 's/|$//')"; else echo "err:$(first_error "$3")"; fi
}
getters() { # <classes dir>: every default getter of its class files, `<class>.<method>`, sorted, space-joined
  (cd "$1" && find . -name '*.class' | sed 's#^\./##; s#\.class$##' | sort | while read -r k; do
    javap -p -cp . "$k" 2> /dev/null | grep -oE '[A-Za-z0-9_$]+\$default\$[0-9]+' | sed "s#^#${k}.#"
  done) | sort -u | tr '\n' ' ' | sed 's/ $//'
}
if [ "$mode" = scalac ]; then
  out=$C/scalac-out
  rm -rf "${out:?}" && mkdir -p "$out"
  : > "$C/scalac.tsv"
  for c in $(cat "$C/cells.txt"); do
    o=$out/$c; mkdir -p "$o"
    if (cd "$C/$c" && timeout 300 scala-cli compile -S 3.8.4 --server=false -q --compilation-output "$o/classes" Main.scala) > "$o/compile.log" 2>&1; then
      timeout 60 java -cp "$SL:$o/classes" run > "$o/run.out" 2> "$o/run.err"
      printf '%s\trun\t%s\n' "$c" "$(verdict $? "$o/run.out" "$o/run.err")" >> "$C/scalac.tsv"
    else
      printf '%s\trun\terr:%s\n' "$c" "$(first_error "$o/compile.log")" >> "$C/scalac.tsv"
    fi
    printf '%s\tgetters\t%s\n' "$c" "$( [ -d "$o/classes" ] && getters "$o/classes")" >> "$C/scalac.tsv"
  done
  echo "inline-getters scalac: $(wc -l < "$C/scalac.tsv") rows"
  exit 0
fi
tag=$3; B=$(cd "$(dirname "$4")" && pwd)/$(basename "$4"); base=${5:-}
out=$C/teq-$tag
rm -rf "${out:?}" && mkdir -p "$out"
: > "$C/$tag.tsv"
row() { printf '%s\t%s\t%s\n' "$1" "$2" "$3" >> "$C/$tag.tsv"; }
for c in $(cat "$C/cells.txt"); do
  o=$out/$c; mkdir -p "$o"
  cd "$C/$c" || exit 1
  timeout 120 "$B" interp Main.scala > "$o/interp.out" 2> "$o/interp.err"
  row "$c" interp "$(verdict $? "$o/interp.out" "$o/interp.err")"
  if timeout 120 "$B" compiler build --target jvm -o "$o/m.jar" --classpath "$SL" Main.scala > /dev/null 2> "$o/jvm.err"; then
    timeout 60 java -cp "$o/m.jar:$SL" TeqMain > "$o/jvm.out" 2>> "$o/jvm.err"
    row "$c" jvm "$(verdict $? "$o/jvm.out" "$o/jvm.err")"
    mkdir -p "$o/classes" && (cd "$o/classes" && unzip -q -o "$o/m.jar" -x 'META-INF/*' 'TeqMain*' 2> /dev/null)
    row "$c" getters "$(getters "$o/classes")"
  else
    row "$c" jvm "err:$(first_error "$o/jvm.err")"; row "$c" getters ""
  fi
  if timeout 120 "$B" compiler build -o "$o/m.js" Main.scala > /dev/null 2> "$o/js.err"; then
    timeout 60 node "$o/m.js" > "$o/js.out" 2>> "$o/js.err"
    row "$c" js "$(verdict $? "$o/js.out" "$o/js.err")"
  else row "$c" js "err:$(first_error "$o/js.err")"; fi
  if [ -n "$base" ]; then
    bc=$C/teq-$base/$c/classes
    if [ -d "$bc" ] && [ -d "$o/classes" ]; then row "$c" same "$(python3 "$root/tests/support/inline_getters.py" same "$bc" "$o/classes" | tail -1)"
    elif [ ! -d "$bc" ] && [ ! -d "$o/classes" ]; then row "$c" same same
    else row "$c" same "differs: a jar on one side alone"; fi
  fi
done
echo "inline-getters teq $tag: $(wc -l < "$C/$tag.tsv") rows"
