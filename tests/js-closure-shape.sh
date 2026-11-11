#!/bin/bash
# Builtin typing ancestors do not prevent a stateless function class becoming an arrow.
set -euo pipefail
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
out=out/js-closure-shape
mkdir -p "$out"
for probe in closure_builtin_bases closure_unused_receiver; do
for mode in dev release; do
  flags=()
  [ "$mode" = release ] && flags=(--release)
  (ulimit -v 4000000; timeout 600 "$TEQ" compiler build "tests/cases/$probe.scala" --threads 1 "${flags[@]}" -o "$out/$probe-$mode.js")
  node "$out/$probe-$mode.js" > "$out/$probe-$mode.actual"
  diff -u "tests/cases/$probe.expected" "$out/$probe-$mode.actual"
  if grep -Eq '^class .*\$anon\$|\$callable\(' "$out/$probe-$mode.js"; then
    echo "FAIL $probe $mode: function-only anonymous class was not lowered to an arrow"
    exit 1
  fi
done
done
echo 'closure shape: 4 passed'
