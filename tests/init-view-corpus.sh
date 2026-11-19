#!/bin/bash
# tests/init-view-corpus.sh <dir of programs> <name>: the view against scalac's InitDump over a corpus outside
# tests/init-view/programs (dotty's tests/init/warn, tests/init-global/warn): each unit (a file, or the files of one
# stem `x_1.scala`, `x_2.scala` compiled together) through tests/init-view/initdump.py (scalac 3.8.4 first, under
# $SCALAC_LOCK where it is set), then the view ALONE of scalac's own pickles wherever scalac made them (scalac-view.dump,
# its own count), then teq's products and their view where teq makes products. Prints per unit `equal`, `positions` (the
# positions equal, the rest not), `differ`, `none` (teq made no products) or `no scalac dump`, and each view that fails
# or prints nothing (FAILED); then the totals of each. No pending list: a census, not a verdict; it exits 1 when a view
# failed.
# Env: TEQ, COURSIER, SCALAC_LOCK (a lock file for flock around each scalac run).
cd "$(dirname "$0")/.."
src=${1:?a directory of programs}
name=${2:?a name for the run}
TEQ=${TEQ:-./target/release/teq}
TEQ=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
work=out/init-view-corpus/$name
mkdir -p "$work"
library=$("${COURSIER:-cs}" fetch --classpath --intransitive org.scala-lang:scala-library:3.8.4 2> /dev/null | tail -1)
[ -n "$library" ] || { echo "corpus: scala-library 3.8.4 not fetched"; exit 1; }
# What the dumps depend on beside the units' sources, first: both JDKs, the library, the corpus's commit (dotty's tests
# at another commit differ: java1 at main writes a type its 3.8.4 tag infers). The counts are of that environment.
bash tests/init-view/environment.sh "$library" "$src"
env_key=$(bash tests/init-view/environment.sh --key "$library")
stems=$(cd "$src" && ls *.scala | sed 's/_[0-9]*\.scala$//; s/\.scala$//' | sort -u)
equal=0; positions=0; differ=0; none=0; lines=0; same=0
alone_units=0; alone_lines=0; alone_same=0; alone_whole=0; no_scalac=0; failed=0
# count <oracle dump> <view dump>: scalac's lines and the lines the view matches, and whether the two are equal whole.
count() {
  python3 -c "
import difflib, sys
a=open(sys.argv[1]).read().splitlines(); b=open(sys.argv[2]).read().splitlines()
print(len(a), sum(m.size for m in difflib.SequenceMatcher(None,a,b,autojunk=False).get_matching_blocks()), int(a==b))" "$1" "$2"
}
for stem in $stems; do
  files=$(cd "$src" && ls "$stem".scala "$stem"_[0-9]*.scala 2> /dev/null | sort)
  out="$work/$stem"
  rm -rf "${out:?}/src"
  mkdir -p "$out/src"
  for f in $files; do cp "$src/$f" "$out/src/"; done
  # scalac's dump is kept while the unit's sources, InitDump and the environment are those it was made from.
  stamp=$( { cat "$out"/src/*.scala tests/init-view/InitDump.scala tests/init-view/initdump.py; echo "$env_key"; } | sha256sum | cut -c1-16)
  if [ ! -s "$out/oracle/pickler.dump" ] || [ "$(cat "$out/oracle/stamp" 2> /dev/null)" != "$stamp" ]; then
    rm -rf "${out:?}/oracle"
    if [ -n "$SCALAC_LOCK" ]; then
      flock "$SCALAC_LOCK" timeout 300 python3 tests/init-view/initdump.py --out "$out/oracle" "$out"/src/*.scala > "$out/initdump.log" 2>&1
    else
      timeout 300 python3 tests/init-view/initdump.py --out "$out/oracle" "$out"/src/*.scala > "$out/initdump.log" 2>&1
    fi
    [ -s "$out/oracle/pickler.dump" ] && echo "$stamp" > "$out/oracle/stamp"
  fi
  # The view alone, of scalac's own pickles, wherever scalac made its dump and classes, whatever teq does with the
  # source: its own count (tests/init-view/view_census.py and by_kind.py read scalac-view.dump). A view that fails or
  # prints nothing is a failure, named and counted, never left out.
  if [ -s "$out/oracle/pickler.dump" ] && [ -d "$out/oracle/classes" ]; then
    rm -f "$out/scalac-view.dump"
    if ! TEQ_INIT_VIEW_CLASSPATH="$library" TEQ_INIT_VIEW="$(cd "$out/src" && pwd)" timeout 60 "$TEQ" tasty "$out/oracle/classes" > "$out/scalac-view.dump" 2> "$out/scalac-view.err" || [ ! -s "$out/scalac-view.dump" ]; then
      echo "$stem FAILED: the view of scalac's pickles ($(head -1 "$out/scalac-view.err"))"
      failed=$((failed + 1))
    fi
    set -- $(count "$out/oracle/pickler.dump" "$out/scalac-view.dump")
    alone_units=$((alone_units + 1)); alone_lines=$((alone_lines + $1)); alone_same=$((alone_same + $2)); alone_whole=$((alone_whole + $3))
  else
    echo "$stem no scalac dump"
    no_scalac=$((no_scalac + 1))
    continue
  fi
  rm -rf "${out:?}/p" "${out:?}/inventory.txt" "${out:?}/view.dump"
  (cd "$out/src" && TEQ_INIT_VIEW_INVENTORY="$OLDPWD/$out/inventory.txt" timeout 120 "$TEQ" compiler check *.scala --std scala-library --products "$OLDPWD/$out/p") > "$out/check.txt" 2>&1
  if [ ! -d "$out/p" ]; then
    echo "$stem none: teq made no products ($(head -1 "$out/check.txt"))"
    none=$((none + 1))
    continue
  fi
  if ! TEQ_INIT_VIEW_INVENTORY="$out/inventory.txt" TEQ_INIT_VIEW_CLASSPATH="$library" TEQ_INIT_VIEW="$(cd "$out/src" && pwd)" timeout 60 "$TEQ" tasty "$out/p" > "$out/view.dump" 2> "$out/view.err" || [ ! -s "$out/view.dump" ]; then
    echo "$stem FAILED: the view of teq's products ($(head -1 "$out/view.err"))"
    failed=$((failed + 1))
  fi
  set -- $(count "$out/oracle/pickler.dump" "$out/view.dump")
  lines=$((lines + $1)); same=$((same + $2))
  if python3 tests/init-view/compare.py "$out/oracle/pickler.dump" "$out/view.dump" > /dev/null; then
    echo "$stem equal"; equal=$((equal + 1))
  elif python3 tests/init-view/compare.py --positions "$out/oracle/pickler.dump" "$out/view.dump" > /dev/null; then
    echo "$stem positions"; positions=$((positions + 1))
  else
    echo "$stem differ"; differ=$((differ + 1))
  fi
done
echo "corpus $name, teq's products: $equal equal whole, $positions with positions equal, $differ differ, $none without products; lines $same of $lines equal"
echo "corpus $name, the view alone (of scalac's pickles): $alone_whole of $alone_units units equal whole; lines $alone_same of $alone_lines equal ($no_scalac without scalac's dump)"
echo "corpus $name: $failed views failed"
[ "$failed" = 0 ]
