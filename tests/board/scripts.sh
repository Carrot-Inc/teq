#!/bin/bash
# The test scripts the stopped branch added and master's tests/all.sh does not run, each run as it is, a test of
# the suite `branch-scripts` that passes when the script exits 0: tests/function-xxl.sh (a suite of the branch's
# gate) and the five tests/js-*.sh. Held to tests/pending/branch-scripts.txt (tests/support/pending.sh; --update
# and --add as it takes them), with the suite's row of the board. tests/modules-scalac-split.sh is no test (it
# records scalac's own builds for tests/modules.sh) and is not run.
cd "$(dirname "$0")/../.." || exit 1
export TEQ=${TEQ:-$PWD/target/release/teq}
out=out/board/scripts
mkdir -p "$out"
. tests/support/pending.sh
pending_begin branch-scripts "$out" "$@"
for s in function-xxl js-builtin-markers js-closure-shape js-constructor-fields js-inline-folded-origin js-unused-captures; do
  timeout "${SCRIPT_LIMIT:-300}" "tests/$s.sh" > "$out/$s.log" 2>&1
  code=$?
  if [ $code = 0 ]; then
    pending_pass "$s"
  else
    pending_listed "$s" || { echo "FAIL $s: exit $code"; tail -5 "$out/$s.log"; }
    pending_fail "$s" "exit $code: $(grep -m1 -E '^FAIL|error:|^[-+][^-+]' "$out/$s.log" || tail -1 "$out/$s.log")"
  fi
done
pending_end
