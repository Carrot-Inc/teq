#!/bin/bash
# tests/init-view/ranges.sh <base teq> <head teq> [<cell pattern>]: the language server's hover and definition answers
# at the constructs of tests/init-view/places.py whose read of the source's text the parser is to record
# (tests/init-view/ranges.py). Per cell: a `teq compiler watch --check --index` session over the cell alone, run by the
# base and by the head, asked at each identifier of the cell (the index of the cell's own parse); and the cell's
# products, built by the base and by the head as tests/init-view/places.sh builds them, each read by a session of the
# base over a client that names the cell's definitions, asked the same at the cell's source, a document of those
# products whose answers come from their TASTy positions. The pattern is a shell pattern over the cells' names,
# every one by default (`'*--nothing--before'` the plain cells alone). Every step is bounded: a build 120 s, a session
# 60 s; a step that fails or passes its bound is NOT JUDGED and named, and so is a products' session whose every answer
# is empty. Writes under out/ranges; prints the table's totals and its moved answers (ranges.py judge, which reads
# out/places/table.md where there is one); exits 1 when an answer of a cell's own session moved, or one of its
# products' moved where the places table has the cell losing a line of scalac's or moving one away from it, when a leg
# is not judged (a record missing, empty, cut or past its bound), when the run's records do not match its cells
# (`cells.txt`), or when the judge's self-test (`ranges.py --self-test`, run first) fails. Each build's and session's
# exit is recorded beside its output (`build-<side>.exit`, `<leg>-<side>.exit`), and each output's sum right after it
# (`cells.sum`, `queries.sum`, `build-<side>.sum`, `<leg>-<side>.sum`: tests/init-view/records.sh), which the judge
# holds the output to; the places table is held to its own (`table.sum`, places.sh's).
cd "$(dirname "$0")/../.."
. tests/init-view/records.sh
# The judge on planted records first: a judge that passes a bad record judges nothing.
python3 tests/init-view/ranges.py --self-test || exit 1
abs() { echo "$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"; }
base=$(abs "${1:?the base teq}")
head=$(abs "${2:?the head teq}")
pattern=${3:-*}
out=out/ranges
rm -rf "${out:?}/cells" "${out:?}/run"
python3 tests/init-view/places.py generate "$out/cells" > /dev/null || exit 1
mkdir -p "$out/run"
run=$(cd "$out/run" && pwd)
# session <teq> <log> <queries> <dir> [<args>..]: a session over the directory's sources, its answers in the log, the
# build's first, then one per query in order.
session() {
  local teq=$1 log=$2 queries=$3 dir=$4
  shift 4
  { cat "$queries"; echo quit; } | timeout 60 "$teq" compiler watch --check --index --std scala-library "$@" "$dir" > "$log" 2> "$log.err"
  local rc=$?
  echo $rc > "${log%.jsonl}.exit"
  record_seal "$(dirname "$log")" "$log"
  return $rc
}
# judged <file> <leg> <what> <exit>: a step's failure, named in the leg's file.
judged() {
  [ "$4" = 0 ] && return 0
  local why="exit $4"
  [ "$4" = 124 ] && why="past its bound"
  echo "$3: $why" >> "$1/not-judged-$2"
  return 1
}
cells=$(python3 tests/init-view/ranges.py cells "$pattern")
# The cells the run is given, which the judge holds its records to.
echo "$cells" | grep . > "$run/cells.txt"
record_seal "$run" "$run/cells.txt"
echo "ranges: $(echo "$cells" | grep -c .) cells"
for cell in $cells; do
  d=$run/$cell
  mkdir -p "$d/src" "$d/client"
  cp "$out/cells/$cell.scala" "$d/src/"
  src=$(cd "$d/src" && pwd -P)
  file=$src/$cell.scala
  python3 tests/init-view/ranges.py queries "$file" "$file" > "$d/queries.txt"
  record_seal "$d" "$d/queries.txt"
  python3 tests/init-view/ranges.py client "$file" > "$d/client/RangesClient.scala"
  # The cell as its own program, by each binary.
  session "$base" "$d/own-base.jsonl" "$d/queries.txt" "$src"
  judged "$d" own "the base's own session" $?
  session "$head" "$d/own-head.jsonl" "$d/queries.txt" "$src"
  judged "$d" own "the head's own session" $?
  # The cell's products by each binary, read by the base: the products alone differ.
  for leg in base head; do
    teq=$base
    [ $leg = head ] && teq=$head
    rm -rf "${d:?}/p-$leg"
    (cd "$src" && timeout 120 "$teq" compiler check "$cell.scala" --std scala-library --products "$d/p-$leg") > "$d/build-$leg.txt" 2>&1
    rc=$?
    echo $rc > "$d/build-$leg.exit"
    record_seal "$d" "$d/build-$leg.txt"
    if [ ! -d "$d/p-$leg" ]; then
      echo "the $leg makes no products (exit $rc$([ $rc = 124 ] && echo ', past its bound'))" >> "$d/not-judged-products"
      continue
    fi
    session "$base" "$d/products-$leg.jsonl" "$d/queries.txt" "$d/client" --classpath "$d/p-$leg"
    judged "$d" products "the session over the $leg's products" $?
    rm -rf "${d:?}/p-$leg"
  done
done
timeout 600 python3 tests/init-view/ranges.py judge "$run" out/places/table.md > "$out/table.md"
rc=$?
grep -E '^ranges:| MOVED |^MANIFEST' "$out/table.md"
exit $rc
