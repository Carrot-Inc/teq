#!/bin/bash
# Builtin typing ancestors need runtime markers only when a reached consumer asks for them.
set -euo pipefail
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
out=out/js-builtin-markers
mkdir -p "$out"
for mode in dev release; do
  flags=()
  [ "$mode" = release ] && flags=(--release)
  (ulimit -v 4000000; timeout 600 "$TEQ" compiler build tests/cases/builtin_ancestor_markers.scala --threads 1 "${flags[@]}" -o "$out/$mode.js")
  node "$out/$mode.js" > "$out/$mode.actual"
  diff -u tests/cases/builtin_ancestor_markers.expected "$out/$mode.actual"
  if [ "$mode" = dev ] && ! grep -Fxq '$cls(Plain, "Plain");' "$out/$mode.js"; then
    echo 'FAIL: untested builtin ancestors still have runtime markers'
    exit 1
  fi
done
echo 'builtin markers: 2 passed'
