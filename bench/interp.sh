#!/bin/bash
# The speed of the interpreter: the README benchmark's program (bench/gen.py, 51 files of 22
# modules, its entry point runs every module) under `teq interp`, under node and under java,
# plus a loop benchmark (10^7 iterations of integer arithmetic and a List fold) that gives
# operations per second. Every run is timed whole, the compile included (node's and java's a
# build followed by the runtime under one bound, tests/support/jars.sh's run_js and run_jvm, the
# scala-library jar looked up before the timing); the interpreter's own line also reports the run
# phase alone (`--time`).
cd "$(dirname "$0")/.."
. bench/phases.sh
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
work=out/bench-interp
mkdir -p "$work"
[ -d "$work/readme" ] || timeout 60 python3 bench/gen.py "$work/readme" 51 22 > /dev/null || exit 1
cat > "$work/loop.scala" <<'SCALA'
@main def loopMain(): Unit =
  var i = 0
  var sum = 0
  while i < 10000000 do
    sum = sum + (i & 7) * 3 - (i >> 2)
    i += 1
  println(sum)
  val xs = List.range(0, 1000000)
  var total = 0L
  var k = 0
  while k < 10 do
    total += xs.foldLeft(0L)((acc, x) => acc + x)
    k += 1
  println(total)
SCALA

# Prints the wall time of a command in milliseconds.
ms() {
  local start end
  start=$(python3 -c 'import time; print(int(time.time() * 1000))')
  "$@" > "$work/last.out" 2> "$work/last.err"
  local code=$?
  end=$(python3 -c 'import time; print(int(time.time() * 1000))')
  [ $code -ne 0 ] && { echo "FAIL: $*" >&2; tail -3 "$work/last.err" >&2; }
  echo $((end - start))
}

# The jar java runs the JVM builds beside, found once, outside the timings.
scala_library=$(scala_library_jar)

for program in readme loop; do
  src="$work/$program"
  [ $program = loop ] && src="$work/loop.scala"
  echo "== $program"
  interp=$(ms timeout 300 "$TEQ" interp "$src")
  run_phase=$(timeout 300 "$TEQ" interp "$src" --time 2>&1 >/dev/null | phases | grep -o 'run [0-9.]*')
  node=$(ms run_js 300 "$work/$program.js" "$src")
  java=$(ms run_jvm 300 "$work/$program.jar" "$scala_library" --build "$src" --target jvm)
  echo "interp ${interp} ms (${run_phase} ms), node ${node} ms, java ${java} ms, interp/node $(python3 -c "print(round($interp / max($node, 1), 1))"), interp/java $(python3 -c "print(round($interp / max($java, 1), 1))")"
  if [ $program = loop ]; then
    run_ms=$(timeout 300 "$TEQ" interp "$src" --time 2>&1 >/dev/null | phases | grep -o 'run [0-9.]*' | sed 's/run //')
    python3 - "$run_ms" <<'PY'
import sys
secs = float(sys.argv[1]) / 1000
# 10^7 loop iterations of 5 operations, plus 10 folds over 10^6 elements.
ops = 10_000_000 * 5 + 10 * 1_000_000
print(f"loop: {secs:.2f} s for the run phase, {ops / secs / 1e6:.1f} M operations/s")
PY
  fi
done
