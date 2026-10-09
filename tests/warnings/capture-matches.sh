#!/bin/bash
# Captures scalac 3.8.4's warnings about the cases of matches (exhaustivity, unreachable cases,
# a block's match not made a partial function) for each probe of tests/warnings/matches into
# tests/warnings/expected/matches/<probe>.txt, the oracle of tests/warnings/matches.sh, one
# `line:col kind` per warning (tests/warnings/parse_matches.py). Run by hand when a probe
# changes or is added; needs scala-cli and a JDK; each probe bounded to 5 min.
#   tests/warnings/capture-matches.sh [probe.scala ...]
cd "$(dirname "$0")/../.."
src=tests/warnings/matches
out=tests/warnings/expected/matches
mkdir -p "$out"
files=("$@")
[ ${#files[@]} -eq 0 ] && mapfile -t files < <(cd "$src" && ls *.scala)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
for f in "${files[@]}"; do
  f=$(basename "$f")
  mkdir -p "$tmp/${f%.scala}"
  cp "$src/$f" "$tmp/${f%.scala}/"
  timeout 300 scala-cli compile -S 3.8.4 --jvm system --server=false -O -color:never "$tmp/${f%.scala}" > "$tmp/$f.log" 2>&1
  python3 tests/warnings/parse_matches.py scalac "$f" < "$tmp/$f.log" > "$out/${f%.scala}.txt"
  # A probe scalac does not compile gives no oracle: said, and its log kept beside it.
  if grep -qE '^-- (\[E[0-9]+\] )?[A-Za-z ]*Error:|error(s)? found' "$tmp/$f.log"; then
    echo "$f: scalac reported an error"
    cp "$tmp/$f.log" "$out/${f%.scala}.scalac-error.log"
  fi
done
echo "captured ${#files[@]} probes"
