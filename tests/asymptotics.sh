#!/bin/bash
# The asymptotic probes: each program under tests/asymptotics runs an operation of the persistent
# collections at three sizes a decade apart and prints PASS or FAIL per operation with the
# growth it measured against the bound its cost allows (tests/asymptotics/probe.scala). They run
# under node at n = 10^4 (10^6 at the largest size), and in the interpreter at n = 10^2;
# ASYMPTOTICS_BASE overrides the size. They ran under java too while the JVM had the lean std; its
# collections are scala-library's since. Each build and run is bounded by `timeout`. Last, a probe of
# the interpreter's retention, the peak memory of tests/asymptotics/frame_cycle.scala at two counts.
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
# The interpreter's retention: a frame left by a throw while a closure over it waits on the lowered
# code's operand stack is let go, its local def's cycle cut (tests/asymptotics/frame_cycle.scala),
# so the peak resident memory after 200 iterations stays within 30 MB of the peak after 20, where
# each frame kept would add 1 MB.
peak_kb() {
  python3 - "$@" <<'PY'
import resource, subprocess, sys
subprocess.run(sys.argv[1:], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=300)
peak = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
print(peak // 1024 if sys.platform == "darwin" else peak)
PY
}
small=$(peak_kb "$TEQ" interp tests/asymptotics/frame_cycle.scala -- 20)
large=$(peak_kb "$TEQ" interp tests/asymptotics/frame_cycle.scala -- 200)
if [ -n "$small" ] && [ -n "$large" ] && [ $((large - small)) -lt 30720 ]; then
  pass=$((pass + 1))
else
  echo "FAIL frame_cycle under interp: peak ${small} KB after 20 iterations, ${large} KB after 200"
  fail=$((fail + 1))
fi
echo "$pass passed, $fail failed"
[ $fail = 0 ]
