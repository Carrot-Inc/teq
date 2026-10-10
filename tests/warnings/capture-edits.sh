#!/bin/bash
# Captures scalac 3.8.4's rewrite of each probe of tests/warnings/edits under -Wunused:imports
# (`-rewrite` applies the actions of the unused imports' warnings, CheckUnused's `checkImports`)
# into tests/warnings/expected/edits/<probe>.scala, and the unused imports scalac finds in the
# rewritten text (a hiding selector left without its wildcard) into <probe>.after.txt
# (`line:col:width`, tests/warnings/parse.py): the oracle of the code action's test
# (tests/lsp/driver.mjs, `unusedImportActions`), scalac's output beside it in out/edits-capture.
# Run by hand when a probe changes; needs scala-cli and a JDK; each compile bounded to 5 min.
#   tests/warnings/capture-edits.sh [probe.scala ...]
cd "$(dirname "$0")/../.."
src=tests/warnings/edits
out=tests/warnings/expected/edits
logs=out/edits-capture
mkdir -p "$out" "$logs"
files=("$@")
if [ ${#files[@]} -eq 0 ]; then
  mapfile -t files < <(cd "$src" && ls *.scala)
fi
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
for f in "${files[@]}"; do
  dir=$tmp/${f%.scala}
  mkdir -p "$dir"
  cp "$src/$f" "$dir/$f"
  timeout 300 scala-cli compile -S 3.8.4 --jvm system --server=false -O -color:never -O -Wunused:imports -O -rewrite "$dir" > "$logs/${f%.scala}.log" 2>&1
  cp "$dir/$f" "$out/$f"
  timeout 300 scala-cli compile -S 3.8.4 --jvm system --server=false -O -color:never -O -Wunused:imports "$dir" > "$logs/${f%.scala}.after.log" 2>&1
  python3 tests/warnings/parse.py scalac "$dir/$f" < "$logs/${f%.scala}.after.log" > "$out/${f%.scala}.after.txt"
  if grep -qE '^-- (\[E[0-9]+\] )?[A-Za-z ]*Error:|error(s)? found' "$logs/${f%.scala}.log"; then
    echo "$f: scalac reported an error"
  fi
done
echo "captured ${#files[@]} probes"
