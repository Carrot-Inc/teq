#!/bin/bash
# tests/support/prim-typetest.sh <teq> <out dir>: the type patterns of a primitive scrutinee (tests/support/prim_typetest.py
# writes the cells and judges them; its self-test runs first), each cell built on its own by scalac 3.8.4 (the sbt
# boot's compiler jars) and by <teq> for the JVM against scala-library, beside the cells' shared classes, and run under
# -Xverify:all. A cell's exit is its run's, or `build:<code>` where its build fails; its output is the run's. Prints the
# judge's lines; exits 1 when a cell's output or exit is not scalac's, or a run is missing.
set -u
teq=$(readlink -f "${1:?teq}") out=$(readlink -m "${2:?out dir}")
here=$(cd "$(dirname "$0")" && pwd)
cd "$here/../.." || exit 2
python3 -I "$here/prim_typetest.py" selftest "$out.selftest" || exit 1
rm -rf "${out:?}"; mkdir -p "$out/runs/scalac" "$out/runs/teq"
python3 -I "$here/prim_typetest.py" gen "$out" || exit 2
. tests/support/jars.sh
. tests/support/compiler-words.sh
spellings "$teq"
sl=$(jar_of scala-library)
[ -n "$sl" ] || { echo "prim-typetest: no scala-library jar in the coursier cache" >&2; exit 2; }
S=$HOME/.sbt/boot/scala-3.8.4
SBT=$(ls -d "$S"/org.scala-sbt/sbt/*/ 2> /dev/null | head -1)
[ -n "$SBT" ] || { echo "prim-typetest: no scala 3.8.4 in the sbt boot" >&2; exit 2; }
lib=$S/lib/scala-library.jar:$S/lib/scala3-library_3-3.8.4.jar
while IFS=$'\t' read -r name _ _ _; do
  for side in scalac teq; do
    d=$out/build/$side/$name r=$out/runs/$side/$name
    mkdir -p "$d"
    if [ $side = scalac ]; then
      timeout 300 java -cp "$S/lib/*:$SBT*" dotty.tools.dotc.Main -classpath "$lib" -d "$d" "$out/Shared.scala" "$out/$name.scala" > "$r.build" 2>&1
      rc=$? cp="$d:$lib"
    else
      timeout 300 "$teq" $(compiler_words "$teq") build "$out/Shared.scala" "$out/$name.scala" --target jvm --std=scala-library --classpath "$sl" -o "$d/out.jar" > "$r.build" 2>&1
      rc=$? cp="$d/out.jar:$sl"
    fi
    if [ $rc != 0 ]; then
      : > "$r.out"; : > "$r.err"; echo "build:$rc" > "$r.exit"
      continue
    fi
    timeout 120 java -Xverify:all -cp "$cp" "$name" > "$r.out" 2> "$r.err"
    echo $? > "$r.exit"
  done
done < "$out/cells.tsv"
python3 -I "$here/prim_typetest.py" judge "$out"
