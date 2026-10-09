#!/bin/bash
# The asymptotic probes: each program under tests/asymptotics runs an operation of the persistent
# collections at three sizes a decade apart and prints PASS or FAIL per operation with the
# growth it measured against the bound its cost allows (tests/asymptotics/probe.scala). They run
# under node at n = 10^4 (10^6 at the largest size), and in the interpreter at n = 10^2;
# ASYMPTOTICS_BASE overrides the size. They ran under java too while the JVM had the lean std; its
# collections are scala-library's since. Each build and run is bounded by `timeout`.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
base=${ASYMPTOTICS_BASE:-10000}
mkdir -p out/asymptotics
pass=0
fail=0
for src in tests/asymptotics/*_probe.scala; do
  name=$(basename "$src" _probe.scala)
  for target in js interp; do
    out=out/asymptotics/$name-$target
    case $target in
      js) timeout 60 "$TEQ" compiler build "$src" tests/asymptotics/probe.scala -o "$out.js" > "$out.log" 2>&1 && timeout 300 node "$out.js" "$base" > "$out.out" 2>&1 ;;
      interp) timeout 300 "$TEQ" interp "$src" tests/asymptotics/probe.scala -- $((base / 100)) > "$out.out" 2>&1 ;;
    esac
    code=$?
    if [ $code = 0 ] && grep -q '^all probes passed' "$out.out" && ! grep -q '^FAIL' "$out.out"; then
      pass=$((pass + 1))
    else
      echo "FAIL $name under $target (exit $code)"
      grep '^FAIL\|error' "$out.out" "$out.log" 2> /dev/null | head -5
      fail=$((fail + 1))
    fi
  done
done
echo "$pass passed, $fail failed"
[ $fail = 0 ]
