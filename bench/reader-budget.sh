#!/bin/bash
# bench/reader-budget.sh <master teq> <landing teq> [runs]: what the library bodies' reading costs on the
# classpath programs and the derivations after them (control, cats, money, chain, java_time, sttp_client,
# zio_json_derive, kittens_derive, tapir_schemas), the two binaries taking turns run by run, each with a jar cache of its own
# after one run each that warms it. Per program and binary, the minimum over the runs (3 by default) of: the
# TASTy bodies decoded and the classes converted to ASTs (`--profile`'s self times, "library bodies decoded
# from TASTy" and "library classes converted to ASTs", in ms), the library bodies' line of `--time` (the
# conversion and the typing of the bodies the program reaches, in ms), and the peak memory footprint of
# `/usr/bin/time -l` (MB). It prints the table and enforces no budget. macOS only.
absolute() { case $1 in /*) echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
[ $# -ge 2 ] || { echo "usage: bench/reader-budget.sh <master teq> <landing teq> [runs]" >&2; exit 2; }
master=$(absolute "$1") landing=$(absolute "$2")
runs=${3:-3}
[ "$(uname -s)" = Darwin ] || { echo "reader-budget: the footprint of /usr/bin/time -l is macOS's"; exit 2; }
cd "$(dirname "$0")/.."
. tests/support/jars.sh
. tests/support/compiler-words.sh
spellings "$master" "$landing"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p out/reader-budget/master out/reader-budget/landing
# one <binary> <cache> <program>: "decode convert bodies footprint" of one build, nothing when it fails.
one() {
  local src=tests/classpath/js/$3.scala
  jars_of "$src"
  TEQ_CACHE_DIR=$2 timeout 120 "$1" $(compiler_words "$1") build "$src" --classpath "$JARS_CP" -o "$tmp/out.js" --profile > "$tmp/profile.txt" 2>&1 || return 1
  TEQ_CACHE_DIR=$2 timeout 120 /usr/bin/time -l "$1" $(compiler_words "$1") build "$src" --classpath "$JARS_CP" -o "$tmp/out.js" --time > "$tmp/time.txt" 2>&1 || return 1
  local decode convert bodies peak
  decode=$(awk '/library bodies decoded from TASTy/ { for (i = 1; i <= NF; i++) if ($(i+1) == "ms" || $(i+1) == "ns") { v = $i; u = $(i+1); break } } END { if (u == "ns") v = v / 1e6; print v + 0 }' "$tmp/profile.txt")
  convert=$(awk '/library classes converted to ASTs/ { for (i = 1; i <= NF; i++) if ($(i+1) == "ms" || $(i+1) == "ns") { v = $i; u = $(i+1); break } } END { if (u == "ns") v = v / 1e6; print v + 0 }' "$tmp/profile.txt")
  bodies=$(awk '$1 == "methods" && $2 == "typed" { print $4 + 0 }' "$tmp/time.txt")
  peak=$(awk '/peak memory footprint/ { printf "%.1f", $1 / 1048576 }' "$tmp/time.txt")
  echo "${decode:-0} ${convert:-0} ${bodies:-0} ${peak:-0}"
}
min() { sort -g | head -1; }
printf '| program | binary | decoded (ms) | converted (ms) | library bodies (ms) | peak footprint (MB) |\n|---|---|---|---|---|---|\n'
for p in control cats money chain java_time sttp_client zio_json_derive kittens_derive tapir_schemas; do
  one "$master" out/reader-budget/master "$p" > /dev/null
  one "$landing" out/reader-budget/landing "$p" > /dev/null
  : > "$tmp/master.txt"
  : > "$tmp/landing.txt"
  for _ in $(seq "$runs"); do
    one "$master" out/reader-budget/master "$p" >> "$tmp/master.txt" || echo "reader-budget: $p fails with $master" >&2
    one "$landing" out/reader-budget/landing "$p" >> "$tmp/landing.txt" || echo "reader-budget: $p fails with $landing" >&2
  done
  for side in master landing; do
    col() { awk -v c="$1" '{ print $c }' "$tmp/$side.txt" | min; }
    printf '| %s | %s | %s | %s | %s | %s |\n' "$p" "$side" "$(col 1)" "$(col 2)" "$(col 3)" "$(col 4)"
  done
done
