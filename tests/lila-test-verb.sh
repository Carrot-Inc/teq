#!/bin/bash
# bench/lila/lila.scala's test verb holds `teq test`'s run to the oracle's inventory: a stand-in teq
# prints a transcript, and the verb passes only where every suite of the inventory ran in its project
# with every case passed, no inventory or an empty one failing at the start
# (tests/support/lila_test_verb.scala). Scenarios: the run as recorded; no inventory; an empty transcript;
# another project's summary alone; a failed case; a missing case; a suite in another project.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
pass=0
fail=0
mkdir -p "$work/here" "$work/empty" "$work/checkout/target/teq" "$work/out"
: > "$work/checkout/target/teq/teq.lock"
cat > "$work/here/tests.json" <<'JSON'
{
 "suites_count": 3,
 "cases_count": 4,
 "suites": {
  "lila.a.OneTest": { "project": "alpha", "cases": [ "adds", "subtracts: twice" ] },
  "lila.a.TwoTest": { "project": "alpha", "cases": [ "parses" ] },
  "lila.b.ThreeTest": { "project": "beta", "cases": [ "renders" ] }
 }
}
JSON
# The transcript the stand-in prints, then the stand-in.
transcript() {
  cat > "$work/transcript.txt"
  printf '#!/bin/sh\nif [ "$1" = test ]; then cat %s; fi\nexit 0\n' "$work/transcript.txt" > "$work/teq"
  chmod +x "$work/teq"
}
verb() {
  local here=$1
  timeout 120 "$TEQ" interp tests/support/lila_test_verb.scala --main lilaTestVerb -- "$here" "$work/checkout" "$work/out" "$work/teq" > "$work/verb.log" 2>&1
}
expect() {
  local what=$1 want=$2 here=${3:-$work/here}
  verb "$here"
  local code=$?
  if { [ "$want" = pass ] && [ $code = 0 ]; } || { [ "$want" = fail ] && [ $code != 0 ]; }; then
    pass=$((pass + 1))
  else
    echo "FAIL $what: expected the verb to $want, exit $code: $(tail -5 "$work/verb.log")"
    fail=$((fail + 1))
  fi
}
recorded='[success] lila.a.OneTest adds
[success] lila.a.OneTest subtracts: twice
[success] lila.a.TwoTest parses
alpha/test: 2 suites, 3 passed, 0 failed in 10 ms (built in 5 ms)
[success] lila.b.ThreeTest renders
beta/test: 1 suite, 1 passed, 0 failed in 4 ms (built in 2 ms)'
printf '%s\n' "$recorded" | transcript
expect "the run as recorded" pass
expect "no inventory" fail "$work/empty"
printf '' | transcript
expect "an empty transcript" fail
printf 'unrelated/test: 3 suites, 4 passed, 0 failed in 1 ms (built in 1 ms)\n' | transcript
expect "another project's summary alone" fail
printf '%s\n' "$recorded" | sed 's/^\[success\] lila.a.TwoTest parses$/[fail] lila.a.TwoTest parses: java.lang.AssertionError: boom/' | transcript
expect "a failed case" fail
printf '%s\n' "$recorded" | grep -v 'subtracts' | transcript
expect "a missing case" fail
printf '%s\n' "$recorded" | sed 's|^alpha/test|gamma/test|' | transcript
expect "a suite in another project" fail
echo "lila-test-verb: $pass passed, $fail failed"
[ $fail = 0 ]
