#!/bin/bash
# tests/init-view/places.sh <base teq> <head teq>: the table of the places the TASTy writer takes from what the parser
# read (tests/init-view/places.py): every cell's program through tests/init-view-corpus.sh with the base (scalac 3.8.4
# first, the view of scalac's own pickles, the base's products and their view), then the head's products of each cell,
# printed by the base's view as the base's are, then the table. Every step is bounded: scalac 300 s and the base's
# build 120 s and view 60 s (the corpus runner's), the head's build 120 s and its view 60 s; a cell whose head step
# fails or passes its bound is NOT JUDGED and named, never counted equal. Writes under out/places and
# out/init-view-corpus/places; exits 1 when a cell loses a line scalac's on the base. Env: COURSIER, SCALAC_LOCK.
cd "$(dirname "$0")/../.."
abs() { echo "$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"; }
base=$(abs "${1:?the base teq}")
head=$(abs "${2:?the head teq}")
out=out/places
work=out/init-view-corpus/places
library=$("${COURSIER:-cs}" fetch --classpath --intransitive org.scala-lang:scala-library:3.8.4 2> /dev/null | tail -1)
[ -n "$library" ] || { echo "places: scala-library 3.8.4 not fetched"; exit 1; }
python3 tests/init-view/places.py generate "$out/cells" || exit 1
TEQ=$base bash tests/init-view-corpus.sh "$out/cells" places > "$out/corpus.log" 2>&1
tail -3 "$out/corpus.log"
rm -rf "${out:?}/head"
for f in "$out"/cells/*.scala; do
  stem=$(basename "$f" .scala)
  u=$work/$stem
  h=$out/head/$stem
  mkdir -p "$h"
  [ -d "$u/src" ] || { echo "the corpus run has no source" > "$h/not-judged"; continue; }
  src=$(cd "$u/src" && pwd)
  (cd "$src" && TEQ_INIT_VIEW_INVENTORY="$OLDPWD/$h/inventory.txt" timeout 120 "$head" compiler check *.scala --std scala-library --products "$OLDPWD/$h/p") > "$h/check.txt" 2>&1
  rc=$?
  if [ ! -d "$h/p" ]; then
    echo "the head makes no products (exit $rc)" > "$h/not-judged"
    continue
  fi
  if ! TEQ_INIT_VIEW_INVENTORY="$h/inventory.txt" TEQ_INIT_VIEW_CLASSPATH="$library" TEQ_INIT_VIEW="$src" timeout 60 "$base" tasty "$h/p" > "$h/view.dump" 2> "$h/view.err" || [ ! -s "$h/view.dump" ]; then
    echo "no view of the head's products in 60 s" > "$h/not-judged"
  fi
  rm -rf "${h:?}/p"
done
timeout 600 python3 tests/init-view/places.py judge "$work" "$out/head" > "$out/table.md"
rc=$?
grep -E '^places:|LOST' "$out/table.md"
exit $rc
