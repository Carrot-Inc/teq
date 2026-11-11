#!/bin/bash
# Type-only inline receivers keep their evaluation, but need no anonymous-class field.
set -euo pipefail
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
out=out/js-unused-captures
mkdir -p "$out"
for mode in dev release; do
  flags=()
  [ "$mode" = release ] && flags=(--release)
  (ulimit -v 4000000; timeout 600 "$TEQ" compiler build tests/cases/anon_unused_capture.scala --threads 1 "${flags[@]}" -o "$out/$mode.js")
  node "$out/$mode.js" > "$out/$mode.actual"
  diff -u tests/cases/anon_unused_capture.expected "$out/$mode.actual"
  if [ "$mode" = dev ]; then
    sed -n '/^class Snapshot.*anon/,/^}/p' "$out/$mode.js" > "$out/anonymous.js"
    test -s "$out/anonymous.js"
    assignments=$(grep -Ec 'this\..*\$c[0-9]+ = \$c[0-9]' "$out/anonymous.js" || true)
    if [ "$assignments" != 0 ]; then
      echo "FAIL: expected no constructor-only capture fields, found $assignments"
      exit 1
    fi
  fi
done
echo 'unused captures: 2 passed'
