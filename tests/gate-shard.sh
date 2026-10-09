#!/bin/bash
# One shard of tests/gate.sh, on a remote machine: the gate copies this file into out/gate/ of its mirror of the
# landing worktree and starts it there under nohup with the shard's list (one suite a line: its name, then its
# command, run from the mirror's root as tests/all.sh runs it). It builds the release binary, and beside it what
# the list needs (the unit tests' build for cargo-test; for split-determinism, master's binary from the mirror
# GATE_REF names, when the gate has synced one), then runs the suites one after another, each under TEQ=that build
# and a bound of GATE_SUITE_SECONDS (default 1800), its output in out/gate/<name>.log; a suite that exits 0 but
# says it skipped for want of something (a line matching GATE_SKIPS and not GATE_ALLOWED) fails. Each step appends
# "<name> <exit> <seconds> <its last line>" to out/gate/status, and "done" ends it; the gate polls that file. The
# build is the timing lines' (build_teq of tests/support/build-manifest.sh): stamped with the head's revision,
# GATE_REVISION, checked to name it, and described in out/gate/build.manifest (GATE_SOURCE the hash of its
# sources), since the gate's comparison machine may measure this very binary.
cd "$(dirname "$0")/../.."
list=$1
gate=out/gate
status=$gate/status
export TEQ=$PWD/target/release/teq
now() { date +%s; }
# step <name> <exit> <started> <log>
step() {
  printf '%s %s %s %s\n' "$1" "$2" $(($(now) - $3)) "$(grep -v '^[[:space:]]*$' "$4" 2> /dev/null | tail -1 | tr -d '\r' | tr '\t' ' ' | cut -c1-300)" >> $status
}
started=$(now)
if grep -q '^cargo-test ' "$list"; then
  CARGO_TARGET_DIR=target/gate-test cargo test --release --no-run > $gate/cargo-test.build.log 2>&1 &
  test_build=$!
fi
if [ -n "$GATE_REF" ] && grep -q '^split-determinism ' "$list"; then
  (cd "$GATE_REF" && cargo build --release) > $gate/ref.build.log 2>&1 &
  ref_build=$!
fi
. tests/support/build-manifest.sh
build_teq . "$GATE_REVISION" > $gate/build.log 2>&1
code=$?
[ $code != 0 ] || manifest . "$GATE_REVISION" "$GATE_SOURCE" > $gate/build.manifest
step build $code $started $gate/build.log
if [ $code != 0 ]; then
  echo done >> $status
  exit 1
fi
while read -r name cmd; do
  [ -n "$name" ] || continue
  started=$(now)
  log=$gate/$name.log
  if [ "$name" = cargo-test ] && ! wait $test_build; then
    cp $gate/cargo-test.build.log $log
    step $name 1 $started $log
    continue
  fi
  if [ "$name" = split-determinism ] && [ -n "$ref_build" ]; then
    if ! wait $ref_build; then
      { tail -20 $gate/ref.build.log; echo "FAIL: master's build for REF in $GATE_REF"; } > $log
      step $name 1 $started $log
      continue
    fi
    export REF=$(cd "$GATE_REF" && pwd)/target/release/teq
  fi
  timeout -k 10 "${GATE_SUITE_SECONDS:-1800}" bash -c "$cmd" > $log 2>&1 < /dev/null
  code=$?
  lack=$(grep -E "$GATE_SKIPS" $log | grep -v -E "$GATE_ALLOWED" | head -1)
  if [ $code = 0 ] && [ -n "$GATE_SKIPS" ] && [ -n "$lack" ]; then
    echo "gate: a skip for want of what the machine should hold: $lack" >> $log
    code=1
  fi
  step $name $code $started $log
done < "$list"
echo done >> $status
