#!/bin/bash
# tests/support/lir-gaps.sh <teq> <out>: every program of tests/cases built for the JVM by <teq> with TEQ_LIR_STATS
# set, the statistics lines of each (`LIRGAPS<TAB><class>.<name><desc><TAB>...`, src/jvm/classes.rs's
# `lir_stats`) kept in <out>/<program>.gaps with the build's exit, then the count of methods a reason holds back:
# `python3 tests/support/lir_gaps.py summary <out>` (docs/LIR.md, "How the extraction goes"). A program whose jars are
# not in the coursier cache is not built and is named.
set -u
teq=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
out=${2:?out}
cd "$(dirname "$0")/../.."
. tests/support/jars.sh
. tests/support/compiler-words.sh
spellings "$teq"
python3 tests/support/lir_gaps.py --self-test || exit 1
JOBS=${JOBS:-$(getconf _NPROCESSORS_ONLN 2> /dev/null || echo 4)}
sl=$(jar_of scala-library)
rm -rf "${out:?}"
mkdir -p "$out"
export teq out sl
one() {
  local src=$1 name flags cp dir
  name=$(basename "$src" .scala)
  . tests/support/jars.sh
  jars_of "$src"
  [ -z "$JARS_MISSING" ] || { echo "notrun $name jars missing:$JARS_MISSING" > "$out/$name.gaps"; return 0; }
  flags=$(grep -h -o '^// teq: .*' "$src" "$src"/*.scala 2> /dev/null | head -1 | sed 's|^// teq: ||')
  cp=$sl${JARS_CP:+:$JARS_CP}
  dir=$(mktemp -d "$out/.build-XXXXXX")
  TEQ_LIR_STATS=1 timeout 120 "$teq" $(compiler_words "$teq") build "$src" $flags --std=scala-library --classpath "$cp" --target jvm -o "$dir/out.jar" 2> "$dir/err" > /dev/null
  local code=$?
  { grep '^LIRGAPS	' "$dir/err"; echo "exit $code"; } > "$out/$name.gaps"
  rm -rf "${dir:?}"
}
export -f one compiler_words _new_spelling
ls -d tests/cases/*.scala tests/cases/*/ 2> /dev/null | sed 's|/$||' | xargs -P "$JOBS" -I{} bash -c 'one {}'
python3 tests/support/lir_gaps.py summary "$out"
