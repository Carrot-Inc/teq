#!/bin/bash
# tests/right-assoc/capture.sh [cell-prefix ...]: scalac 3.8.4's lines for the cells of tests/right-assoc/gen.py, the
# suite's expectations. Run by hand where scalac is, when a cell or the generator changes (needs coursier and a JDK):
# each cell built and run by scalac (a build bounded at 300 s, a run at 60 s), its line written to
# tests/right-assoc/expected.tsv (`<cell>\t<kind>\t<line>`: `run` and the output, its lines joined by ` | `, where
# the run exited 0, `crash <exit>` or `timeout` where it did not, or
# `refused` and the first error's `<line>:<column>: <first line of the message>`, the column from 1); for a cell
# whose operator is defined above `run` (or in its Defs.scala), the descriptors of scalac's class files for its definitions alone (the
# methods whose names hold the operator's encoded name, its default getters included), by javap -s -p, to
# tests/right-assoc/descriptors.tsv (`<cell>\t<name descriptor> ; ...`). With prefixes, only the cells they name are
# captured again and the rest of each file is kept. SCALA_LIBRARY and SCALAC_CLASSPATH stand for coursier's answers
# where it is not on the path. Writes under out/right-assoc-capture.
set -u
cd "$(dirname "$0")/../.."
cs=${COURSIER:-cs}
SL=${SCALA_LIBRARY:-$("$cs" fetch --classpath --intransitive org.scala-lang:scala-library:3.8.4 2> /dev/null | tail -1)}
SC=${SCALAC_CLASSPATH:-$("$cs" fetch --classpath org.scala-lang:scala3-compiler_3:3.8.4 2> /dev/null | tail -1)}
[ -n "$SL" ] && [ -n "$SC" ] || { echo "right-assoc capture: no scalac 3.8.4 from coursier"; exit 2; }
out=out/right-assoc-capture
rm -rf "${out:?}"
mkdir -p "$out"
python3 tests/right-assoc/gen.py "$out/table" > "$out/gen.log" || { cat "$out/gen.log"; exit 1; }
export SL SC out
capture_one() {
  local cell=$1 d=$out/work/$1 dir=$out/table/cells/$1
  local src=$dir/Test.scala
  mkdir -p "$d/sc" "$d/lib"
  if ! timeout 300 java -XX:-UsePerfData -cp "$SC" dotty.tools.dotc.Main -color:never -classpath "$SL" -d "$d/sc" "$dir"/*.scala > "$d/build.log" 2>&1; then
    python3 tests/right-assoc/lines.py refused "$cell" < "$d/build.log" > "$d/line"
  else
    timeout 60 java -XX:-UsePerfData -cp "$d/sc:$SL" run > "$d/run.out" 2>&1
    local code=$?
    [ "$code" = 124 ] && code=timeout
    python3 tests/right-assoc/lines.py run "$cell" "$code" < "$d/run.out" > "$d/line"
    # The definitions alone, for their descriptors: the text above `run`.
    if [ -f "$dir/Defs.scala" ]; then cp "$dir/Defs.scala" "$d/Lib.scala"; else python3 tests/right-assoc/lines.py lib < "$src" > "$d/Lib.scala"; fi
    if [ -s "$d/Lib.scala" ] && timeout 300 java -XX:-UsePerfData -cp "$SC" dotty.tools.dotc.Main -color:never -classpath "$SL" -d "$d/lib" "$d/Lib.scala" > "$d/lib.log" 2>&1; then
      find "$d/lib" -name '*.class' -print0 | sort -z | xargs -0 -r javap -s -p > "$d/javap.txt" 2> /dev/null
      python3 tests/right-assoc/lines.py descriptors "$cell" < "$d/javap.txt" > "$d/desc"
    fi
  fi
}
export -f capture_one
cut -f1 "$out/table/cells.tsv" | { if [ $# -gt 0 ]; then grep -E "^($(IFS='|'; echo "$*"))"; else cat; fi; } \
  | xargs -P "${JOBS:-8}" -I{} bash -c 'capture_one "$@"' _ {}
python3 tests/right-assoc/lines.py merge "$out" tests/right-assoc/expected.tsv tests/right-assoc/descriptors.tsv "$@"
