#!/bin/bash
# The test verb of the JavaScript target's reference projects (bench/reference/scalajs.scala, Airstream's and
# Laminar's) holds `teq test`'s run to the oracle's inventory: a stand-in teq writes the run's events where
# `--events` names (the project, the suite, the selector, the status and the throwable, a line each, as `teq test
# --events` writes them) and exits with a status, and the verb passes only where every suite of the inventory ran in
# its project with each case's statuses the inventory's, as many times, and the run ended as it should
# (tests/support/reference_test_verb.scala). Scenarios: the run as recorded; no inventory; an empty one; no event;
# an ignored case that passed; a passed case ignored; a failed case; a missing case; a suite in another project; the
# run's abort (an exit of 2 after every event); a case reported twice; an ignored case pending; the review's
# two-case inventory, its passed case twice and its ignored case pending; a case the inventory records twice, run
# twice; the review's two events, a case of another project's suite of the name qualifying none of this one's; a
# suite of one name in two projects. And the preflight, which refuses a checkout whose directory sbt names Airstream's root project after
# (tests/support/reference_preflight.scala), and the peak memory the verbs sample, which counts a node the script
# starts (the test runner's), though it names no teq binary (tests/support/reference_sampler.scala).
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
pass=0
fail=0
mkdir -p "$work/checkout/target/teq" "$work/out"
printf 'projects:\n  airstream:\n    base: .\n' > "$work/checkout/target/teq/teq.lock"
# A checkout is a repository, whose tracked files the verb holds to the pin.
git init -q "$work/checkout"
cat > "$work/tests.json" <<'JSON'
{
 "suites_count": 2,
 "cases_count": 4,
 "suites": {
  "com.raquo.airstream.OneSpec": { "project": "airstream", "cases": [ "adds", "subtracts: twice" ], "ignored": [ "glitches" ] },
  "com.raquo.airstream.TwoSpec": { "project": "airstream", "cases": [ "parses" ] }
 }
}
JSON
printf '{\n "suites_count": 0,\n "cases_count": 0,\n "suites": {}\n}\n' > "$work/empty.json"
printf '{"suites": {"S": {"project": "airstream", "cases": ["x"], "ignored": ["y"]}}}\n' > "$work/two.json"
printf '{"suites": {"S": {"project": "airstream", "cases": ["x", "x"]}}}\n' > "$work/twice.json"
printf '{"suites": {"S": {"project": "airstream", "cases": ["x", "y"]}}}\n' > "$work/xy.json"
printf '{"suites": {"S": [{"project": "airstream", "cases": ["x"]}, {"project": "laminar", "cases": ["y"]}]}}\n' > "$work/both.json"
# The events and the exit status the stand-in gives `test`, writing the events to the file after `--events`;
# `stop` exits 0.
events() {
  local status=${1:-0}
  cat > "$work/events.txt"
  cat > "$work/teq" <<SH
#!/bin/sh
if [ "\$1" = test ]; then
  while [ \$# -gt 0 ]; do [ "\$1" = --events ] && cp "$work/events.txt" "\$2"; shift; done
  echo 'airstream/test: done'
  exit $status
fi
exit 0
SH
  chmod +x "$work/teq"
}
expect() {
  local what=$1 want=$2 inventory=${3:-$work/tests.json}
  timeout 120 "$TEQ" interp tests/support/reference_test_verb.scala --main referenceTestVerb -- "$inventory" "$work/checkout" "$work/out" "$work/teq" > "$work/verb.log" 2>&1
  local code=$?
  if { [ "$want" = pass ] && [ $code = 0 ]; } || { [ "$want" = fail ] && [ $code != 0 ]; }; then
    pass=$((pass + 1))
  else
    echo "FAIL $what: expected the verb to $want, exit $code: $(tail -5 "$work/verb.log")"
    fail=$((fail + 1))
  fi
}
T=$(printf '\t')
recorded="airstream${T}com.raquo.airstream.OneSpec${T}test adds${T}Success${T}
airstream${T}com.raquo.airstream.OneSpec${T}test subtracts: twice${T}Success${T}
airstream${T}com.raquo.airstream.OneSpec${T}test glitches${T}Ignored${T}
airstream${T}com.raquo.airstream.TwoSpec${T}test parses${T}Success${T}"
printf '%s\n' "$recorded" | events
expect "the run as recorded" pass
expect "no inventory" fail "$work/none.json"
expect "an empty inventory" fail "$work/empty.json"
printf '' | events
expect "no event" fail
printf '%s\n' "$recorded" | sed "s/glitches${T}Ignored/glitches${T}Success/" | events
expect "an ignored case that passed" fail
printf '%s\n' "$recorded" | sed "s/parses${T}Success/parses${T}Ignored/" | events
expect "a passed case ignored" fail
printf '%s\n' "$recorded" | sed "s/parses${T}Success${T}/parses${T}Failure${T}java.lang.AssertionError: boom/" | events 1
expect "a failed case" fail
printf '%s\n' "$recorded" | grep -v 'subtracts' | events
expect "a missing case" fail
printf '%s\n' "$recorded" | sed 's/^airstream/laminar/' | events
expect "a suite in another project" fail
printf '%s\n' "$recorded" | events 2
expect "the run's abort" fail
{ printf '%s\n' "$recorded"; printf '%s\n' "$recorded" | grep 'parses'; } | events
expect "a case reported twice" fail
printf '%s\n' "$recorded" | sed "s/glitches${T}Ignored/glitches${T}Pending/" | events
expect "an ignored case pending" fail
printf 'airstream%sS%stest x%sSuccess%s\nairstream%sS%stest x%sSuccess%s\nairstream%sS%stest y%sPending%s\n' "$T" "$T" "$T" "$T" "$T" "$T" "$T" "$T" "$T" "$T" "$T" "$T" | events
expect "the review's two-case inventory, its passed case twice and its ignored case pending" fail "$work/two.json"
printf 'airstream%sS%stest x%sSuccess%s\nairstream%sS%stest x%sSuccess%s\n' "$T" "$T" "$T" "$T" "$T" "$T" "$T" "$T" | events
expect "a case the inventory records twice, run twice" pass "$work/twice.json"
printf 'foreign%sS%stest x%sSuccess%s\nairstream%sS%stest y%sSuccess%s\n' "$T" "$T" "$T" "$T" "$T" "$T" "$T" "$T" | events
expect "the review's two events, a case of another project's suite of the name before this one's" fail "$work/xy.json"
printf 'airstream%sS%stest x%sSuccess%s\nlaminar%sS%stest y%sSuccess%s\n' "$T" "$T" "$T" "$T" "$T" "$T" "$T" "$T" | events
expect "a suite of one name in two projects, as recorded" pass "$work/both.json"
# The preflight: a checkout whose directory sbt names Airstream's root project after (`Airstream-oracle`, the build
# naming none) is refused with the reason before anything runs; one named `airstream` passes the name.
mkdir -p "$work/named/Airstream-oracle" "$work/named/airstream"
renamed=$(timeout 120 "$TEQ" interp tests/support/reference_preflight.scala --main referencePreflight -- "$work/named/Airstream-oracle" 2>&1)
named=$(timeout 120 "$TEQ" interp tests/support/reference_preflight.scala --main referencePreflight -- "$work/named/airstream" 2>&1)
if [[ "$renamed" == *"makes sbt's root project airstream-oracle"* ]] && [[ "$named" != *"makes sbt's root project"* ]]; then
  pass=$((pass + 1))
else
  echo "FAIL a checkout whose directory renames the root project is refused at the preflight: $renamed / $named"
  fail=$((fail + 1))
fi
# The admission's memory: a node the verb starts, which names no teq binary, is in the sampled peak.
sampled=$(timeout 120 "$TEQ" interp tests/support/reference_sampler.scala --main referenceSampler -- "$work/out" 2>&1 | tail -1)
child=${sampled% *}
peak=${sampled#* }
if [ "${peak:-0}" -ge 262144 ] 2> /dev/null; then
  pass=$((pass + 1))
else
  echo "FAIL the sampled peak counts the node the script starts: node's $child KiB, the peak ${peak:-none} KiB ($sampled)"
  fail=$((fail + 1))
fi
echo "reference-test-verb: $pass passed, $fail failed"
[ $fail = 0 ]
