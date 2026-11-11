#!/bin/bash
# Constant folding retains argument provenance for inline-body sharing.
set -euo pipefail
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
out=out/js-inline-folded-origin
mkdir -p "$out"
for mode in dev release; do
  flags=()
  [ "$mode" = release ] && flags=(--release)
  (ulimit -v 4000000; timeout 600 "$TEQ" compiler build tests/cases/inline_folded_origin.scala --threads 1 "${flags[@]}" -o "$out/$mode.js")
  node "$out/$mode.js" > "$out/$mode.actual"
  diff -u tests/cases/inline_folded_origin.expected "$out/$mode.actual"
  if [ "$mode" = dev ]; then
    test "$(grep -c '^function calculate\$o' "$out/$mode.js")" = 1
    grep -Fq 'const first = $k0;' "$out/$mode.js"
  fi
done
echo 'inline folded origin: 2 passed'
