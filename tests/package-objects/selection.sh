#!/bin/bash
# tests/package-objects/selection.sh <teq> <out dir>: the selection cells (selection.py). Per cell, its upstream modules
# built in order by their writers (`teq`: `compiler build --target jvm --products`; `scalac`: scalac 3.8.4 `-d`;
# `scalac2`: Scala 2.13.16 `-d`; `teq-<name>`: an earlier teq given as TEQ_WRITERS="name=/path ..."), each module's
# product files set to the cell's time when it has one and written into another module's directory when it names one;
# then the client built over the same upstream products, in the cell's class path order:
# - by scalac 3.8.4 and run on the JVM: the oracle;
# - by teq on the JVM at 1 and at 8 workers, run; as JavaScript, run by node; by teq's interpreter.
# The table (selection_table.py): per cell scalac's answer (exit, output, the dropStale warning or error's text, a
# refusal's places) and each of teq's legs against it; a migration cell's legs against the refusal of products of
# another number. Every build and run bounded (scalac 300 s, a run 60 s, every other 120 s) and its exit recorded: a
# cell with a record missing, a build or run past its bound, or an upstream module whose earlier teq TEQ_WRITERS does
# not give (its exit recorded as `NOT JUDGED: no writer ..`) is NOT JUDGED and named, and fails the table. The judge's
# self-test runs first. Writes and deletes only under <out dir>. Env: COURSIER, SCALAC_LOCK, TEQ_WRITERS.
set -u
here=$(cd "$(dirname "$0")" && pwd)
teq=$(cd "$(dirname "${1:?a teq}")" && pwd)/$(basename "$1")
out=${2:?an out dir}
mkdir -p "$out"
out=$(cd "$out" && pwd)
cs=${COURSIER:-cs}
SL=$("$cs" fetch --classpath --intransitive org.scala-lang:scala-library:3.8.4 2> /dev/null | tail -1)
SC=$("$cs" fetch --classpath org.scala-lang:scala3-compiler_3:3.8.4 2> /dev/null | tail -1)
SL2=$("$cs" fetch --classpath --intransitive org.scala-lang:scala-library:2.13.16 2> /dev/null | tail -1)
SC2=$("$cs" fetch --classpath org.scala-lang:scala-compiler:2.13.16 2> /dev/null | tail -1)
[ -n "$SL" ] && [ -n "$SC" ] && [ -n "$SL2" ] && [ -n "$SC2" ] || { echo "selection: scala 3.8.4 or 2.13.16 not fetched"; exit 1; }
lock() { if [ -n "${SCALAC_LOCK:-}" ]; then flock "$SCALAC_LOCK" "$@"; else "$@"; fi; }
declare -A writers
for w in ${TEQ_WRITERS:-}; do writers[${w%%=*}]=${w#*=}; done
case "$out" in /?*) rm -rf "${out:?}/cells" ;; esac
python3 "$here/selection.py" "$out/cells" > /dev/null || exit 1
python3 "$here/selection_table.py" --self-test || { echo "selection: the judge's self-test failed"; exit 1; }
for c in "$out"/cells/*/; do
  c=${c%/}
  r=$c/r
  mkdir -p "$r"
  n=$(python3 -c "import json,sys; print(len(json.load(open(sys.argv[1]))['modules']))" "$c/cell.json")
  main=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['main'])" "$c/cell.json")
  for ((k = 0; k < n; k++)); do
    IFS='|' read -r name writer time into files < <(python3 -c "
import json, sys
m = json.load(open(sys.argv[1]))['modules'][int(sys.argv[2])]
print(m['name'], m['writer'], m.get('time', ''), m.get('out', m['name']), ' '.join(m['files']), sep='|')" "$c/cell.json" "$k")
    srcs=()
    for f in $files; do srcs+=("$c/$name/$f"); done
    if [ "$writer" = client ]; then
      order=$(python3 -c "import json,sys; print(':'.join(sys.argv[2] + '/r/up-' + m for m in json.load(open(sys.argv[1]))['order']))" "$c/cell.json" "$c")
      cp=${order:+$order:}$SL
      mkdir -p "$r/scalac-client"
      lock timeout 300 java -cp "$SC" dotty.tools.dotc.Main -d "$r/scalac-client" -classpath "$cp" "${srcs[@]}" > "$r/scalac.log" 2>&1
      echo $? > "$r/scalac.exit"
      timeout 60 java -cp "$r/scalac-client:$cp" "$main" > "$r/scalac.out" 2>&1
      echo $? > "$r/scalac.run-exit"
      for t in 1 8; do
        timeout 120 "$teq" compiler build --target jvm --threads "$t" --products "$r/jvm$t" --classpath "$cp" "${srcs[@]}" > "$r/jvm$t.log" 2>&1
        echo $? > "$r/jvm$t.exit"
        timeout 60 java -cp "$r/jvm$t:$cp" "$main" > "$r/jvm$t.out" 2>&1
        echo $? > "$r/jvm$t.run-exit"
      done
      timeout 120 "$teq" compiler build --main "$main" -o "$r/js.js" --classpath "$cp" "${srcs[@]}" > "$r/js.log" 2>&1
      echo $? > "$r/js.exit"
      timeout 60 node "$r/js.js" > "$r/js.out" 2>&1
      echo $? > "$r/js.run-exit"
      timeout 120 "$teq" interp --main "$main" --classpath "$cp" "${srcs[@]}" > "$r/interp.out" 2> "$r/interp.log"
      echo $? > "$r/interp.exit"
      continue
    fi
    up=$r/up-$into
    tmp=$r/tmp-$name
    mkdir -p "$up" "$tmp"
    unjudged=
    case $writer in
      scalac)
        lock timeout 300 java -cp "$SC" dotty.tools.dotc.Main -d "$tmp" -classpath "$SL" "${srcs[@]}" > "$r/up-$name.log" 2>&1 ;;
      scalac2)
        lock timeout 300 java -cp "$SC2" scala.tools.nsc.Main -d "$tmp" -classpath "$SL2" "${srcs[@]}" > "$r/up-$name.log" 2>&1 ;;
      teq)
        timeout 120 "$teq" compiler build --target jvm --products "$tmp" --classpath "$SL" "${srcs[@]}" > "$r/up-$name.log" 2>&1 ;;
      teq-*)
        b=${writers[${writer#teq-}]:-}
        if [ -z "$b" ]; then echo "no writer $writer" > "$r/up-$name.log"; unjudged="NOT JUDGED: no writer $writer (TEQ_WRITERS)"
        else timeout 120 "$b" compiler build --target jvm --products "$tmp" --classpath "$SL" "${srcs[@]}" > "$r/up-$name.log" 2>&1
        fi ;;
    esac
    st=$?
    echo "${unjudged:-$st}" > "$r/up-$name.exit"
    # The module's own files get its time, then join its directory (another module's, one container for both),
    # their times kept.
    [ -n "$time" ] && find "$tmp" -type f -exec touch -d "@$time" {} +
    cp -Rp "$tmp"/. "$up"/
  done
done
# The client teq's version and products number, which its refusal of another number's products names.
version=$(timeout 30 "$teq" --version | awk '{ print $2 }')
abi=$(timeout 30 "$teq" --version --verbose | sed -n 's/^products ABI \([0-9][0-9]*\)$/\1/p')
python3 "$here/selection_table.py" "$out/cells" "$version" "${abi:-0}"
