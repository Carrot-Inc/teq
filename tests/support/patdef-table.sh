#!/bin/bash
# tests/support/patdef-table.sh <teq> <out dir> [<reference teq>]: the table of pattern definitions against scalac 3.8.4's own class
# files (tests/support/patdef_table.py writes it and judges it; its self-test runs first). The local-scope cells check
# each raw load and store slot in the disassemblies. The fixtures are built together by scalac 3.8.4 (the sbt boot's
# compiler jars) and by <teq> for the JVM against scala-library; each build's classes are read by `javap -c -p`
# and each is run, its exit status kept beside its output.
# Prints the judge's lines; exits 1 when a cell's values are not scalac's, a cell is missing or a run ends otherwise
# than with 0. With a reference (the parent commit's binary), they are built
# by it too, and a cell of a rung the head does not take must keep the reference's code.
set -u
teq=$(readlink -f "${1:?teq}") out=$(readlink -m "${2:?out dir}") ref=${3:+$(readlink -f "$3")}
here=$(cd "$(dirname "$0")" && pwd)
cd "$here/../.." || exit 2
python3 -I "$here/patdef_table.py" selftest "$out.selftest" || exit 1
rm -rf "${out:?}"; mkdir -p "$out/out" "$out/scalac" "$out/teq" "$out/ref"
python3 -I "$here/patdef_table.py" gen "$out" || exit 2
. tests/support/jars.sh
. tests/support/compiler-words.sh
spellings "$teq" ${ref:+"$ref"}
sl=$(jar_of scala-library)
[ -n "$sl" ] || { echo "patdef-table: no scala-library jar in the coursier cache" >&2; exit 2; }
S=$HOME/.sbt/boot/scala-3.8.4
SBT=$(ls -d "$S"/org.scala-sbt/sbt/*/ 2> /dev/null | head -1)
[ -n "$SBT" ] || { echo "patdef-table: no scala 3.8.4 in the sbt boot" >&2; exit 2; }
lib=$S/lib/scala-library.jar:$S/lib/scala3-library_3-3.8.4.jar
o=$out/out
timeout 900 java -cp "$S/lib/*:$SBT*" dotty.tools.dotc.Main -classpath "$lib" -d "$out/scalac" "$out"/*.scala > "$o/scalac.build" 2>&1
timeout 120 java -cp "$out/scalac:$lib" PatDef > "$o/scalac.out" 2> "$o/scalac.err"; echo "exit $?" > "$o/scalac.exit"
timeout 900 "$teq" $(compiler_words "$teq") build "$out"/*.scala --target jvm --std=scala-library --classpath "$sl" -o "$out/teq.jar" > "$o/teq.build" 2>&1
(cd "$out/teq" && unzip -q -o ../teq.jar)
main=PatDef; [ -f "$out/teq/TeqMain.class" ] && main=TeqMain
timeout 120 java -Xverify:all -cp "$out/teq:$sl" $main > "$o/teq.out" 2> "$o/teq.err"; echo "exit $?" > "$o/teq.exit"
sides="scalac teq"
if [ -n "$ref" ]; then
  timeout 900 "$ref" $(compiler_words "$ref") build "$out"/*.scala --target jvm --std=scala-library --classpath "$sl" -o "$out/ref.jar" > "$o/ref.build" 2>&1
  (cd "$out/ref" && unzip -q -o ../ref.jar)
  sides="$sides ref"
fi
for side in $sides; do
  (cd "$out/$side" && find . -name '*.class' | sort | while read -r c; do javap -c -p "$c"; done) > "$o/$side.javap" 2> /dev/null
  (cd "$out/$side" && find . -name '*.class' | sort | while read -r c; do javap -v -p "$c"; done) > "$o/$side.javapv" 2> /dev/null
done
python3 -I "$here/patdef_table.py" judge "$out"
