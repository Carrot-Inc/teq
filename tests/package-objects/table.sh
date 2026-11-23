#!/bin/bash
# tests/package-objects/table.sh <teq> <out dir>: the package-object table (cells.py), every cell through its legs:
# - scalac 3.8.4 (the oracle): its products' TASTy paths and its run's output (`java user.run`);
# - teq's JVM build of the whole cell at 1 and at 8 workers (`--threads`): the manifest's shared paths, the TASTy
#   paths, the product files' bytes (the two builds must be the same), the run's output;
# - teq's JavaScript build at 1 and at 8 workers, run by node; teq's interpreter;
# - the split build (the library alone, then user/Main.scala over its products), run;
# - scalac reading teq's products: user/Main.scala compiled by scalac over the split build's library products, run.
# Every build and run bounded (scalac 300 s, a run 60 s, every other 120 s) and its exit recorded (`<leg>.exit`,
# `<leg>.run-exit`). Then table.py prints the table: a cell with a record missing, or a build or run past its bound,
# is NOT JUDGED and named, and fails the table. The judge's self-test runs first. Writes and deletes only under
# <out dir>. Env: COURSIER, SCALAC_LOCK.
set -u
here=$(cd "$(dirname "$0")" && pwd)
teq=$(cd "$(dirname "${1:?a teq}")" && pwd)/$(basename "$1")
out=${2:?an out dir}
mkdir -p "$out"
out=$(cd "$out" && pwd)
python3 "$here/table.py" --self-test || { echo "package-objects: the judge's self-test failed"; exit 1; }
cs=${COURSIER:-cs}
SL=$("$cs" fetch --classpath --intransitive org.scala-lang:scala-library:3.8.4 2> /dev/null | tail -1)
SC=$("$cs" fetch --classpath org.scala-lang:scala3-compiler_3:3.8.4 2> /dev/null | tail -1)
[ -n "$SL" ] && [ -n "$SC" ] || { echo "package-objects: scala 3.8.4 not fetched"; exit 1; }
lock() { if [ -n "${SCALAC_LOCK:-}" ]; then flock "$SCALAC_LOCK" "$@"; else "$@"; fi; }
case "$out" in "$out") rm -rf "${out:?}/cells" ;; esac
python3 "$here/cells.py" "$out/cells" || exit 1
for c in "$out"/cells/*/; do
  c=${c%/}
  r=$c/r
  mkdir -p "$r"
  (
    cd "$c" || exit
    lib=$(find lib -name '*.scala' | sort)
    # scalac
    mkdir -p "$r/scalac"
    lock timeout 300 java -cp "$SC" dotty.tools.dotc.Main -d "$r/scalac" -classpath "$SL" $lib user/Main.scala > "$r/scalac.log" 2>&1
    echo $? > "$r/scalac.exit"
    (cd "$r/scalac" && find . -name '*.tasty' | sed 's|^\./||' | sort) > "$r/scalac.tasty"
    timeout 60 java -cp "$r/scalac:$SL" user.run > "$r/scalac.out" 2>&1
    echo $? > "$r/scalac.run-exit"
    for n in 1 8; do
      timeout 120 "$teq" compiler build --target jvm --threads "$n" --products "$r/jvm$n" --classpath "$SL" $lib user/Main.scala > "$r/jvm$n.log" 2>&1
      echo $? > "$r/jvm$n.exit"
      timeout 60 java -cp "$r/jvm$n:$SL" user.run > "$r/jvm$n.out" 2>&1
      echo $? > "$r/jvm$n.run-exit"
      timeout 120 "$teq" compiler build --threads "$n" -o "$r/js$n.js" $lib user/Main.scala > "$r/js$n.log" 2>&1
      echo $? > "$r/js$n.exit"
      timeout 60 node "$r/js$n.js" > "$r/js$n.out" 2>&1
      echo $? > "$r/js$n.run-exit"
    done
    timeout 120 "$teq" interp $lib user/Main.scala > "$r/interp.out" 2>&1
    echo $? > "$r/interp.exit"
    timeout 120 "$teq" compiler build --target jvm --products "$r/split-lib" --classpath "$SL" $lib > "$r/split-lib.log" 2>&1
    echo $? > "$r/split-lib.exit"
    timeout 120 "$teq" compiler build --target jvm --products "$r/split-user" --classpath "$r/split-lib:$SL" user/Main.scala > "$r/split-user.log" 2>&1
    echo $? > "$r/split-user.exit"
    timeout 60 java -cp "$r/split-lib:$r/split-user:$SL" user.run > "$r/split.out" 2>&1
    echo $? > "$r/split.run-exit"
    mkdir -p "$r/scalac-reads"
    lock timeout 300 java -cp "$SC" dotty.tools.dotc.Main -d "$r/scalac-reads" -classpath "$r/split-lib:$SL" user/Main.scala > "$r/scalac-reads.log" 2>&1
    echo $? > "$r/scalac-reads.exit"
    timeout 60 java -cp "$r/split-lib:$r/scalac-reads:$SL" user.run > "$r/scalac-reads.out" 2>&1
    echo $? > "$r/scalac-reads.run-exit"
  )
done
python3 "$here/table.py" "$out/cells"
