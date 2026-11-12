#!/bin/bash
# Functions past 22 parameters across the two compilers (on the JVM they are to be scala-library's
# scala.runtime.FunctionXXL, as scalac 3.8.4 erases them, `Definitions.functionTypeErasure`).
# tests/function-xxl/callee is an API that takes, returns, builds and applies a Function23, a spelled
# Function023, a Function64 and a ContextFunction23, with a class and an object implementing Function23;
# tests/function-xxl/caller calls all of it. Each leg's run is to print tests/function-xxl/expected.txt,
# scalac-on-scalac's output (the caller built by scalac over scalac's callee, recorded once):
#   teq-callee      teq is to build the callee in the product mode, writing no class under scala/Function
#                   (scala-library holds every class a function erases to); scalac is to build the caller over
#                   teq's products, run over teq's class files;
#   linked          the caller scalac built over scalac's callee is to run over teq's class files of the callee: the
#                   descriptors and the classes scalac's own erasure links against;
#   teq-caller      teq is to build the caller over scalac's callee, run over scalac's class files;
#   teq-on-teq      teq is to build the caller over its own callee products, run over them;
#   scalac-on-scalac  the recorded baseline, so that a scala-cli other than 3.8.4's stands out.
# scalac is the suites' (`scala-cli --power compile -S 3.8.4 --jvm system --server=false --offline`, as
# tests/modules.sh runs it). Skips without scala-cli or scala-library's jar in the coursier cache. TEQ names the
# binary (default: the repository's release build). tests/function-xxl/unused checks the synthetic classes' deferred
# completion under a memory limit, tests/function-xxl/workers the same across the workers (below).
cd "$(dirname "$0")/.." || exit 1
root=$PWD
TEQ=${TEQ:-./target/release/teq}
case $TEQ in /*) ;; *) TEQ=$root/$TEQ ;; esac
command -v scala-cli > /dev/null 2>&1 || { echo "skip: scala-cli not found"; exit 0; }
. tests/support/jars.sh
SL=$(jar_of scala-library)
[ -f "$SL" ] || { echo "skip: scala-library 3.8.4 is not in the coursier cache"; exit 0; }
src=$root/tests/function-xxl
want=$(cat "$src/expected.txt")
out=$(mktemp -d "${TMPDIR:-/tmp}/teq-function-xxl.XXXXXX") || exit 2
trap 'rm -rf "$out"' EXIT
callee=("$src"/callee/*.scala)
caller=("$src"/caller/*.scala)
pass=0
fail=0
ok() { echo "ok   $1"; pass=$((pass + 1)); }
bad() { echo "FAIL $1"; printf '%s\n' "$2" | head -40; fail=$((fail + 1)); }
# scalac <class path> <dest> <sources...>: scalac 3.8.4 over the sources; prints its output.
scalac() {
  local cp=$1 dest=$2
  shift 2
  local ws
  ws=$(mktemp -d "$out/ws.XXXXXX") && mkdir -p "$dest" || return 1
  (cd "$ws" && COURSIER_MODE=offline timeout 300 scala-cli --power compile -S 3.8.4 --jvm system --server=false --offline -q \
    --workspace "$ws" --classpath "$cp" -d "$dest" "$@" 2>&1)
}
# run <leg> <class path> <main class>: java over the class path; the leg passes when the run prints expected.txt.
run() {
  local got
  got=$(timeout 60 java -Xss512m -cp "$2:$SL" "$3" 2>&1)
  if [ "$got" = "$want" ]; then ok "$1"; else bad "$1" "$(diff <(printf '%s\n' "$want") <(printf '%s\n' "$got"))"; fi
}

if log=$(scalac "$SL" "$out/callee-sc" "${callee[@]}") && log=$(scalac "$out/callee-sc:$SL" "$out/caller-sc-sc" "${caller[@]}"); then
  run scalac-on-scalac "$out/caller-sc-sc:$out/callee-sc" fxl.use.Main
else
  bad scalac-on-scalac "scalac: $log"
fi

if log=$(timeout 300 "$TEQ" compiler build --target jvm --products "$out/callee-teq" --sourceroot "$src/callee" "${callee[@]}" 2>&1); then
  held=$(cd "$out/callee-teq" && find . -path './scala/Function*' | sort)
  if [ -n "$held" ]; then bad "teq-callee: no scala/Function class" "$held"; else ok "teq-callee: no scala/Function class"; fi
  if log=$(scalac "$out/callee-teq:$SL" "$out/caller-sc-teq" "${caller[@]}"); then
    run teq-callee "$out/caller-sc-teq:$out/callee-teq" fxl.use.Main
  else
    bad teq-callee "scalac over teq's callee: $log"
  fi
  if [ -d "$out/caller-sc-sc" ]; then
    run linked "$out/caller-sc-sc:$out/callee-teq" fxl.use.Main
  else
    bad linked "no caller scalac built over its own callee"
  fi
  if log=$(timeout 300 "$TEQ" compiler build --target jvm -o "$out/caller-teq-teq" --classpath "$out/callee-teq" --sourceroot "$src/caller" "${caller[@]}" 2>&1); then
    run teq-on-teq "$out/caller-teq-teq:$out/callee-teq" TeqMain
  else
    bad teq-on-teq "teq over its own callee: $log"
  fi
else
  bad teq-callee "teq: $log"
fi

log=""
if [ -d "$out/callee-sc" ] && log=$(timeout 300 "$TEQ" compiler build --target jvm -o "$out/caller-teq-sc" --classpath "$out/callee-sc" --sourceroot "$src/caller" "${caller[@]}" 2>&1); then
  run teq-caller "$out/caller-teq-sc:$out/callee-sc" TeqMain
else
  bad teq-caller "teq over scalac's callee: ${log:-no callee scalac built}"
fi

# A synthetic function class's information, and an impure alias's, wait for their first completion (`newFunctionNType`'s
# completer): tests/function-xxl/unused imports a ContextFunction, a Function and an ImpureFunction of 100000000
# parameters, reading none, which each target's build is to compile under a 4 GiB address-space limit and run as
# scalac's (unused/expected.txt, scalac's run); the JVM runs outside the limit, which its heap's reservation exceeds.
unused=$src/unused/UnusedFunction.scala
unused_want=$(cat "$src/unused/expected.txt")
limited() { (ulimit -v 4194304 && timeout 120 "$TEQ" "$@") 2>&1; }
if log=$(limited compiler build "$unused" -o "$out/unused.js") && got=$(timeout 60 node "$out/unused.js" 2>&1) && [ "$got" = "$unused_want" ]; then
  ok "unused: JavaScript under 4 GiB"
else
  bad "unused: JavaScript under 4 GiB" "$log$got"
fi
if log=$(limited compiler build "$unused" --target jvm --classpath "$SL" -o "$out/unused.jar") && got=$(timeout 60 java -cp "$out/unused.jar:$SL" TeqMain 2>&1) && [ "$got" = "$unused_want" ]; then
  ok "unused: JVM under 4 GiB"
else
  bad "unused: JVM under 4 GiB" "$log$got"
fi
if log=$(limited compiler check "$unused" --products "$out/unused-products"); then
  ok "unused: products under 4 GiB"
else
  bad "unused: products under 4 GiB" "$log"
fi

# An unused arity is to allocate nothing across the workers:
# tests/function-xxl/workers imports ContextFunction10000000 unused
# beside twenty objects the workers type, checked at 1, 2 and 8 threads
# outside any address-space limit, its peak resident size expected
# within the same program's with ContextFunction23 plus 32 MiB.
workers=$src/workers
mkdir -p "$out/workers23"
sed 's/ContextFunction10000000/ContextFunction23/' "$workers/LargeImport.scala" > "$out/workers23/LargeImport.scala"
cp "$workers/Multi.scala" "$out/workers23/"
# peak <sources> <threads>: the check's exit code and peak resident size in KiB, by GNU time (without it, skipped).
[ -x /usr/bin/time ] || echo "skip workers: no /usr/bin/time"
peak() { /usr/bin/time -f '%x %M' timeout 120 "$TEQ" compiler check "$1" --threads "$2" 2>&1 > /dev/null | tail -1; }
for threads in $([ -x /usr/bin/time ] && echo 1 2 8); do
  read -r large_exit large <<< "$(peak "$workers" $threads)"
  read -r small_exit small <<< "$(peak "$out/workers23" $threads)"
  if [ "$large_exit" = 0 ] && [ "$small_exit" = 0 ] && [ "$large" -le $((small + 32768)) ]; then
    ok "workers: an unused arity allocates nothing at $threads threads"
  else
    bad "workers: an unused arity allocates nothing at $threads threads" "ContextFunction10000000: exit $large_exit, peak $large KiB; ContextFunction23: exit $small_exit, peak $small KiB"
  fi
done

echo "function-xxl: $pass passed, $fail failed"
[ $fail = 0 ]
