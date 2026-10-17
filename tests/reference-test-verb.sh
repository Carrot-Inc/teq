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
# suite of one name in two projects; the events of two suites interleaved, as a pool sends them, one suite's case
# after the other's; a suite whose task threw, one Error of its SuiteSelector in the place of its cases (the JVM
# runner's, docs/TOOLING.md, "Tests."). And the preflight, which refuses a checkout whose directory sbt names Airstream's root project after
# (tests/support/reference_preflight.scala), and the peak memory the verbs sample, which counts a node the invocation
# starts with its token (the test runner's), though it names no teq binary (tests/support/reference_sampler.scala).
# An export whose plugin is in no repository sbt reads naming the plugin, the sbt it is for and TEQ_PLUGIN_VERSION that
# named it (tests/support/reference_export_verb.scala). Over a record of airstream blocked by its test build, a test
# framework not loading beside the known error, a summary counting a failure no outcome or event shows, a new build
# error of no project of tests.json's and an executed
# failure after the last summary with the events missing or in them each fail, judged and recorded. A passing test
# printing a diagnostic, a line of teq's, a count, a summary or an outcome before its run's summary passes over a
# record of none, the first over a record of a failing case too, judged and recorded, the record unchanged; over the
# latter, the run without its summary and a diagnostic before airstream's end with nothing run each fail, over a record
# of a known build error and the failing case, a new error on teq's stderr without its count before every case passing
# and a count without its diagnostic each fail, and over a record of an error whose message holds a caret line, the
# message changed after it fails, judged and recorded.
# And the compile
# verb's generated files held to the record by the digest of their bytes
# (tests/support/reference_compile_verb.scala): a baseline of wrong bytes, the same bytes within it, other wrong bytes
# failing, the pin's bytes passing with the item gone and the checkout put back.
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
# The events and the exit status the stand-in gives `test`, writing the events to the file after `--events` (none where
# $work/no-events is there) and printing the transcript teq prints with them (tests/support/transcript_of_events.py),
# or $work/transcript.txt where it is there, on teq's two channels (tests/support/teq_channels.py: teq's own lines on
# stderr, a line `1> ...` a suite's printing on stdout); `stop` exits 0.
events() {
  local status=${1:-0}
  cat > "$work/events.txt"
  cat > "$work/teq" <<SH
#!/bin/sh
if [ "\$1" = test ]; then
  records=
  while [ \$# -gt 0 ]; do
    [ "\$1" = --events ] && [ ! -e "$work/no-events" ] && cp "$work/events.txt" "\$2"
    [ "\$1" = --diagnostics ] && [ ! -e "$work/no-diagnostics" ] && records="--records \$2"
    shift
  done
  { if [ -e "$work/transcript.txt" ]; then cat "$work/transcript.txt"; else python3 $PWD/tests/support/transcript_of_events.py "$work/events.txt"; fi; } | python3 $PWD/tests/support/teq_channels.py \$records
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
printf '%s\n' "$recorded" | sed -n '1p; 4p; 2p; 3p' | events
expect "two suites' events interleaved, one suite's case after the other's" pass
printf '%s\n' "$recorded" | grep -v 'OneSpec' | { cat; printf 'airstream%scom.raquo.airstream.OneSpec%ssuite%sError%sjava.lang.RuntimeException: boom\n' "$T" "$T" "$T" "$T"; } | events 1
expect "a suite whose task threw" fail
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
# The admission's memory: a node the verb starts with the invocation's token, which names no teq binary, is in the
# sampled peak.
sampled=$(timeout 120 "$TEQ" interp tests/support/reference_sampler.scala --main referenceSampler -- "$work/out" 2>&1 | tail -1)
child=${sampled% *}
peak=${sampled#* }
if [ "${peak:-0}" -ge 262144 ] 2> /dev/null; then
  pass=$((pass + 1))
else
  echo "FAIL the sampled peak counts the node the script starts: node's $child KiB, the peak ${peak:-none} KiB ($sampled)"
  fail=$((fail + 1))
fi
# The class of the records' judgment of a test run (`Deviations.judge`, one function for every driver; the same cases
# in tests/lila-test-verb.sh and tests/mill-test-verb.sh), over a record of airstream blocked by its test build, judged
# and recorded: a new build error of a configuration of no project of tests.json's (extra) fails; an executed failure
# after the last summary, with the run's events missing, or in them (its outcomes are its events), fails.
sverb() { timeout 120 "$TEQ" interp tests/support/reference_test_verb.scala --main referenceTestVerb -- "$work/tests.json" "$work/checkout" "$work/out" "$work/teq" "$1" > "$work/verb.log" 2>&1; }
blocked_test='src/A.scala:1:1: error: Not found: x
x
^
airstream/test: 1 error'
printf '' | events 1
printf '%s\n' "$blocked_test" > "$work/transcript.txt"
sverb baseline
code=$?
grep -q -F '"airstream: blocked by airstream/test src/A.scala:1:1: error: Not found: x\nx\n^"' "$work/out/here/deviations.json" 2> /dev/null && [ $code = 0 ] && pass=$((pass + 1)) ||
  { echo "FAIL a baseline of airstream blocked: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
sjudged_and_recorded() {
  local what=$1
  cp "$work/out/here/deviations.json" "$work/deviations.kept"
  sverb kept
  local code=$?
  sverb record
  local code2=$?
  if [ $code != 0 ] && [ $code2 != 0 ] && cmp -s "$work/out/here/deviations.json" "$work/deviations.kept" && grep -q "$2" "$work/verb.log"; then pass=$((pass + 1)); else
    echo "FAIL $what: exits $code and $code2, or the record changed, or no \"$2\": $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); fi
}
{ printf '%s\n' "$blocked_test"; printf '%s\n' 'extra/N.scala:9:1: error: Not found: n' 'n' '^' 'extra/test: 1 error'; } > "$work/transcript.txt"
sjudged_and_recorded "a new build error of no project of tests.json's" "^  extra/test extra/N.scala:9:1: error: Not found: n$"
{ printf '%s\n' "$blocked_test"; echo '[fail] S x: java.lang.Error: newly executed failure'; } > "$work/transcript.txt"
touch "$work/no-events"
sjudged_and_recorded "an executed failure after the last summary, no events" "it wrote no events"
rm -f "$work/no-events"
printf 'airstream%scom.raquo.airstream.TwoSpec%stest parses%sFailure%sjava.lang.Error: newly executed failure\n' "$T" "$T" "$T" "$T" | events 1
sjudged_and_recorded "an executed failure after the last summary, in the run's events" "airstream: a build of its failed and 1 of its cases ran"
printf '' | events 1
{ printf '%s\n' "$blocked_test"; echo 'teq: airstream/test: the test framework example.Framework does not load: java.lang.NoClassDefFoundError: NewlyMissing'; } > "$work/transcript.txt"
sjudged_and_recorded "a test framework not loading beside the known build error" "the test framework example.Framework does not load: java.lang.NoClassDefFoundError: NewlyMissing: a failure of the test run's"
{ printf '%s\n' "$blocked_test"; echo "teq: airstream/test: no memory mapping for the type store's overlays; one worker types the build"; echo 'teq: airstream/test: the test framework example.Framework does not load: java.lang.NoClassDefFoundError: NewlyMissing'; } > "$work/transcript.txt"
sjudged_and_recorded "the overlays' note, then a test framework not loading, beside the known build error" "the test framework example.Framework does not load: java.lang.NoClassDefFoundError: NewlyMissing: a failure of the test run's"
{ printf '%s\n' "$blocked_test"; echo "teq: airstream/test: no memory mapping for the type store's overlays; one worker types the build"; printf '%s\n' 'src/New.scala:7:1: error: Not found: NEW' 'NEW' '^'; } > "$work/transcript.txt"
sjudged_and_recorded "the overlays' note, then a new error no summary claims, beside the known build error" "1 errors after airstream/test's line of teq's, claimed by no summary"
echo 'airstream/test: 1 suite, 0 passed, 1 failed in 1 ms (built in 1 ms)' > "$work/transcript.txt"
sjudged_and_recorded "a summary counting a failure no outcome and no event shows" "airstream: its summary counts 1 failed, its events none"
rm -f "$work/transcript.txt"
# What a suite prints is its run's stream on teq's stdout, never one of teq's own lines, which teq writes on stderr
# (Transcript.scan over teq's stderr; the same cases in tests/lila-test-verb.sh and tests/mill-test-verb.sh): a passing
# test printing `Example.scala:1:1: error: ...` before its run's summary passes over a record of none and over a record
# of a failing case, and one printing a line of teq's, a count, a summary or an outcome over a record of none, judged
# and recorded, the record's bytes unchanged; over the failing case's record, the printing run without its summary, and
# a diagnostic on stderr before airstream's end with nothing run, each fail, judged and recorded. A diagnostic on stderr
# no count claims fails whatever run's summary follows it (the review's sequence): over a record of beta's known build
# error and the failing case, beta's known error and a new one without its count, then every case passing, fails, and
# so does a count on stderr without its diagnostic, judged and recorded, the record unchanged. A diagnostic is its
# whole block up to teq's next own line, whatever its message holds: over a record of airstream blocked by an error
# whose message is `prefix`, `^`, `Example.scala:1:1: warning: note`, `OLD`, the same passes again and `NEW` for `OLD` fails, judged and recorded.
printed='Example.scala:1:1: error: an expected diagnostic from the test'
printed_transcript() { python3 tests/support/transcript_of_events.py "$work/events.txt" | sed "s|^airstream/test: |1> $1\\n&|" > "$work/transcript.txt"; }
sheld_unchanged() {
  local what=$1
  cp "$work/out/here/deviations.json" "$work/deviations.kept"
  sverb kept
  local code=$?
  sverb record
  local code2=$?
  if [ $code = 0 ] && [ $code2 = 0 ] && cmp -s "$work/out/here/deviations.json" "$work/deviations.kept" && grep -q -F "1> $2" "$work/transcript.txt"; then pass=$((pass + 1)); else
    echo "FAIL $what: exits $code and $code2, or the record changed: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); fi
}
printf '%s\n' "$recorded" | events
for shown in "$printed" 'teq: airstream/test: the test framework example.Framework does not load: expected fixture output' \
  'airstream/test: 1 error' 'airstream/test: 1 suite, 1 passed, 0 failed in 1 ms (built in 1 ms)' '[fail] com.raquo.airstream.TwoSpec parses: java.lang.AssertionError: printed'; do
  printed_transcript "$shown"
  expect "a passing test printing \"$shown\", over a record of none" pass
  sheld_unchanged "a passing test printing \"$shown\", over a record of none, judged and recorded" "$shown"
done
# A diagnostic's message is never a configuration's summary (`Transcript.scan`'s `summaries`): airstream's build
# warning whose message holds airstream's summary, then the run, passes over a record of none, judged and recorded,
# unchanged; the same with the run's summary left out fails, judged and recorded, the record unchanged.
warned_transcript() { { printf '%s\n' 'src/W.scala:1:1: warning: method value in object Old is deprecated since 1: sample' '+> airstream/test: 2 suites, 3 passed, 0 failed, 1 ignored in 1 ms (built in 1 ms)' 'w' '^'; python3 tests/support/transcript_of_events.py "$work/events.txt"; } > "$work/transcript.txt"; }
warned_transcript
expect "a warning holding airstream's summary, over a record of none" pass
cp "$work/out/here/deviations.json" "$work/deviations.kept"
sverb kept
code=$?
sverb record
code2=$?
[ $code = 0 ] && [ $code2 = 0 ] && cmp -s "$work/out/here/deviations.json" "$work/deviations.kept" && pass=$((pass + 1)) ||
  { echo "FAIL a warning holding airstream's summary, judged and recorded: exits $code and $code2, or the record changed: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
warned_transcript
grep -v '^airstream/test: ' "$work/transcript.txt" > "$work/transcript.new" && mv "$work/transcript.new" "$work/transcript.txt"
sjudged_and_recorded "airstream's summary left out, its warning holding one" "ran without its tests' completing"
printf '%s\n' "$recorded" | sed "s/parses${T}Success${T}/parses${T}Failure${T}java.lang.AssertionError: boom/" | events 1
printed_transcript "$printed"
sverb baseline
code=$?
grep -q -F '"airstream: com.raquo.airstream.TwoSpec: parses: Failure (tests.json: Success)"' "$work/out/here/deviations.json" 2> /dev/null && [ $code = 0 ] && pass=$((pass + 1)) ||
  { echo "FAIL a baseline of a failing case beside a test printing a diagnostic: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
sheld_unchanged "a passing test printing a diagnostic beside the known failing case" "$printed"
printed_transcript "$printed"
grep -v '^airstream/test: ' "$work/transcript.txt" > "$work/transcript.new" && mv "$work/transcript.new" "$work/transcript.txt"
sjudged_and_recorded "a test printing a diagnostic in a run without its summary" "of its cases ran without its tests' completing (no summary)"
printf '%s\n' "$printed" 'airstream/test: no suite to run' > "$work/transcript.txt"
sjudged_and_recorded "a diagnostic before airstream's end with nothing run" "1 errors before airstream/test's end, claimed by no count"
beta_known='beta/Known.scala:1:1: error: Not found: KNOWN
KNOWN
^
beta/test: 1 error'
{ printf '%s\n' "$beta_known"; python3 tests/support/transcript_of_events.py "$work/events.txt"; } > "$work/transcript.txt"
sverb baseline
code=$?
grep -q -F 'beta/test beta/Known.scala:1:1: error: Not found: KNOWN' "$work/out/here/deviations.json" 2> /dev/null && [ $code = 0 ] && pass=$((pass + 1)) ||
  { echo "FAIL a baseline of beta's known build error and the failing case: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
printf '%s\n' "$recorded" | events 1
{ printf '%s\n' "$beta_known" 'beta/New.scala:7:1: error: Not found: NEW' 'NEW' '^'; python3 tests/support/transcript_of_events.py "$work/events.txt"; } > "$work/transcript.txt"
sjudged_and_recorded "a new error on teq's stderr without its count, then every case passing" "1 errors before airstream/test's end, claimed by no count"
# A run's events are its outcomes, never its report's lines on stdout, whose names and messages are the suites': a real
# `teq test` (tests/support/probe_run.sh) of a case passing under a name that runs over lines reading as a diagnostic
# and a count, and of a case failing with a message of those lines and a summary, makes a baseline of the failure alone
# through the production driver, judged and recorded again unchanged; the run's report holds those lines on stdout. Its
# messages' lines separated by LF and by CR LF (`PROBE_CRLF`): a diagnostic's lines are its rendering's, whatever
# separates them (`Transcript.lines`), so the warning's message holding the summary is never a completion.
export LOCK_PY=$PWD/integrations/sbt/example/check-export.py
. tests/support/probe_run.sh
for sep in lf crlf; do
  if why=$(PROBE_CRLF=$(probe_crlf $sep) probe_checkout "$work/probe-co" airstream); then
    git init -q "$work/probe-co"
    rm -rf "$work/probe-out" && mkdir -p "$work/probe-out"
    printf '{"suites": {"probes.Empty": {"project": "airstream", "cases": []}, "probes.Named": {"project": "airstream", "cases": ["%s"]}, "probes.Thrown": {"project": "airstream", "cases": ["%s"]}}}\n' "$probe_named" "$probe_thrown" > "$work/probe-tests.json"
    teq_abs=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
    real() { timeout 300 "$TEQ" interp tests/support/reference_test_verb.scala --main referenceTestVerb -- "$work/probe-tests.json" "$work/probe-co" "$work/probe-out" "$teq_abs" "$1" > "$work/verb.log" 2>&1; }
    real baseline
    code=$?
    items=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["deviations"])' "$work/probe-out/here/deviations.json" 2> /dev/null)
    cp "$work/probe-out/here/deviations.json" "$work/deviations.kept" 2> /dev/null
    real kept
    code2=$?
    real record
    code3=$?
    if [ $code = 0 ] && [ $code2 = 0 ] && [ $code3 = 0 ] && [ "$items" = "['airstream: probes.Thrown: reports: Failure (tests.json: Success)']" ] && cmp -s "$work/probe-out/here/deviations.json" "$work/deviations.kept" \
      && grep -q '^Example.scala:1:1: error: sample$' "$work/probe-out/test-1.log" && ! grep -q 'Example.scala' "$work/probe-out/test-1.err"; then pass=$((pass + 1)); else
      echo "FAIL a real run whose event names and messages read as teq's lines ($sep): exits $code, $code2 and $code3, items $items: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); fi
    (cd "$work/probe-co" && timeout 60 "$teq_abs" stop > /dev/null 2>&1)
  else
    echo "FAIL a real run whose event names and messages read as teq's lines ($sep): $why"; fail=$((fail + 1))
  fi
done
# A build's diagnostics are teq's records of them (`--diagnostics`), whatever a message holds: a real run of a project
# whose test build fails with `compiletime.error("prefix\nExample.scala:1:1: warning: note\nOLD")`
# (tests/support/probe_run.sh) through the production driver makes a baseline of the error's whole block, passes again
# unchanged, and with `NEW` for `OLD` fails, judged and recorded, the record's bytes unchanged, the message's lines
# separated by LF and by CR LF alike (`PROBE_CRLF`).
for sep in lf crlf; do
  if why=$(PROBE_CRLF=$(probe_crlf $sep) probe_checkout "$work/inline-co" airstream OLD); then
    git init -q "$work/inline-co"
    rm -rf "$work/inline-out" && mkdir -p "$work/inline-out"
    printf '{"suites": {"probes.Named": {"project": "airstream", "cases": ["%s"]}, "probes.Thrown": {"project": "airstream", "cases": ["%s"]}}}\n' "$probe_named" "$probe_thrown" > "$work/inline-tests.json"
    teq_abs=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
    real() { timeout 300 "$TEQ" interp tests/support/reference_test_verb.scala --main referenceTestVerb -- "$work/inline-tests.json" "$work/inline-co" "$work/inline-out" "$teq_abs" "$1" > "$work/verb.log" 2>&1; }
    word() { sed -i "s/[A-Z]*\")\$/$1\")/" "$work/inline-co/main/probes/Message.scala"; }
    whole='main/probes/Message.scala:7:21: error: prefix\nExample.scala:1:1: warning: note\nOLD\n  def run(): Unit = Message.broken\n                    ^^^^^^^^^^^^^^'
    real baseline
    c1=$?
    real kept
    c2=$?
    cp "$work/inline-out/here/deviations.json" "$work/inline.kept" 2> /dev/null
    word NEW
    real kept
    c3=$?
    real record
    c4=$?
    if [ $c1 = 0 ] && [ $c2 = 0 ] && [ $c3 != 0 ] && [ $c4 != 0 ] && grep -q -F "$whole" "$work/inline-out/here/deviations.json" && cmp -s "$work/inline-out/here/deviations.json" "$work/inline.kept"; then pass=$((pass + 1)); else
      echo "FAIL a real test of an error whose message holds a header's line ($sep): exits $c1, $c2, $c3 and $c4, or no whole block, or the record changed: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); fi
    (cd "$work/inline-co" && timeout 60 "$teq_abs" stop > /dev/null 2>&1)
  else
    echo "FAIL a real test of an error whose message holds a header's line ($sep): $why"; fail=$((fail + 1))
  fi
done
# A generator's failure is teq's record of it (`--diagnostics`, `failure`), its command's whole output, whatever the
# command prints: a real run of a project whose test configuration's command generator prints `generator started`,
# `helper/test: no suite to run` and `OLD`, and exits 1 (tests/support/probe_run.sh), through the production driver
# makes a baseline of the whole failure, passes again unchanged, and with `NEW` for `OLD` fails, judged and recorded,
# the record's bytes unchanged.
if why=$(probe_checkout "$work/gen-co" airstream "" test); then
  git init -q "$work/gen-co"
  rm -rf "$work/gen-out" && mkdir -p "$work/gen-out"
  printf '{"suites": {"probes.Named": {"project": "airstream", "cases": ["%s"]}, "probes.Thrown": {"project": "airstream", "cases": ["%s"]}}}\n' "$probe_named" "$probe_thrown" > "$work/gen-tests.json"
  teq_abs=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
  real() { timeout 300 "$TEQ" interp tests/support/reference_test_verb.scala --main referenceTestVerb -- "$work/gen-tests.json" "$work/gen-co" "$work/gen-out" "$teq_abs" "$1" > "$work/verb.log" 2>&1; }
  whole='teq: command generator 0 of airstream/test: `sh gen.sh` failed (exit status: 1): generator started\nhelper/test: no suite to run\nOLD'
  real baseline
  c1=$?
  real kept
  c2=$?
  cp "$work/gen-out/here/deviations.json" "$work/gen.kept" 2> /dev/null
  sed -i 's/^echo OLD$/echo NEW/' "$work/gen-co/gen.sh"
  real kept
  c3=$?
  real record
  c4=$?
  if [ $c1 = 0 ] && [ $c2 = 0 ] && [ $c3 != 0 ] && [ $c4 != 0 ] && grep -q -F "$whole" "$work/gen-out/here/deviations.json" && cmp -s "$work/gen-out/here/deviations.json" "$work/gen.kept"; then pass=$((pass + 1)); else
    echo "FAIL a real test of a generator whose output reads as its configuration's end: exits $c1, $c2, $c3 and $c4, or no whole failure, or the record changed: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); fi
  (cd "$work/gen-co" && timeout 60 "$teq_abs" stop > /dev/null 2>&1)
else
  echo "FAIL a real test of a generator whose output reads as its configuration's end: $why"; fail=$((fail + 1))
fi
{ printf '%s\n' "$beta_known" 'beta/test: 1 error'; python3 tests/support/transcript_of_events.py "$work/events.txt"; } > "$work/transcript.txt"
sjudged_and_recorded "a count on teq's stderr without its diagnostic, then every case passing" "beta/test: 1 errors reported without a diagnostic"
caret_error() { printf '%s\n' 'src/Printed.scala:4:28: error: prefix' '^' '+> Example.scala:1:1: warning: note' "$1" '  test("prints a count") { Message.broken }' '                           ^^^^^^^^^^^^^^' 'airstream/test: 1 error'; }
printf '' | events 1
caret_error OLD > "$work/transcript.txt"
sverb baseline
code=$?
grep -q -F '"airstream: blocked by airstream/test src/Printed.scala:4:28: error: prefix\n^\nExample.scala:1:1: warning: note\nOLD\n  test(\"prints a count\") { Message.broken }\n                           ^^^^^^^^^^^^^^"' "$work/out/here/deviations.json" 2> /dev/null && [ $code = 0 ] && pass=$((pass + 1)) ||
  { echo "FAIL a baseline of airstream blocked by an error whose message holds a caret line: exit $code: $(grep -o 'airstream: blocked[^,]*' "$work/out/here/deviations.json")"; fail=$((fail + 1)); }
sverb kept
code=$?
[ $code = 0 ] && pass=$((pass + 1)) || { echo "FAIL the error whose message holds a caret line, again: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
caret_error NEW > "$work/transcript.txt"
sjudged_and_recorded "the error's message changed after its caret line" "NEW"
rm -f "$work/transcript.txt"
# An export whose plugin is in no repository sbt reads says which, in its own words (tests/support/reference_export_verb.scala):
# a stand-in sbt failing as the gate's holder failed, TEQ_PLUGIN_VERSION naming a plugin published for sbt 2 alone.
mkdir -p "$work/sbtbin" "$work/jdk/bin"
printf '%s\n' '#!/bin/sh' 'echo "[info] loading global plugins"' \
  'echo "[error] sbt.librarymanagement.ResolveException: Error downloading build.teq:sbt-teq;sbtVersion=1.0;scalaVersion=2.12:1.0.0-gate-SNAPSHOT"' \
  'echo "[error]   Not found"' 'exit 1' > "$work/sbtbin/sbt"
java_major=$(python3 -c "import json; print(json.load(open('bench/airstream/pin.json'))['java'])")
printf '#!/bin/sh\necho %s >&2\n' "'openjdk version \"$java_major\" 2026-01-01'" > "$work/jdk/bin/java"
chmod +x "$work/sbtbin/sbt" "$work/jdk/bin/java"
(PATH="$work/sbtbin:$PATH" JAVA_HOME="$work/jdk" TEQ_PLUGIN_VERSION=1.0.0-gate-SNAPSHOT timeout 120 "$TEQ" interp tests/support/reference_export_verb.scala --main referenceExportVerb -- \
  "$work/checkout" "$work/export-out" "$work/teq") > "$work/verb.log" 2>&1
code=$?
[ $code = 1 ] && grep -v '^[[:space:]]*$' "$work/verb.log" | tail -1 | grep -q "^airstream: the export exits 1: sbt-teq 1.0.0-gate-SNAPSHOT for sbt 1.0 (Scala 2.12) is in no repository sbt reads: TEQ_PLUGIN_VERSION names it" &&
  pass=$((pass + 1)) || { echo "FAIL an export missing its plugin: exit $code: $(tail -2 "$work/verb.log")"; fail=$((fail + 1)); }
# The compile verb's generated files (tests/support/reference_compile_verb.scala): a file the generators write with
# other bytes than the pin's is an item of the record with the digest of those bytes, so that a baseline of wrong bytes
# admits those alone: other wrong bytes fail, the pin's bytes pass with the item gone. The stand-in's compile writes
# src/Gen.scala from gen.txt; the checkout is put back at the pin after each run.
rc=$work/rc
mkdir -p "$rc/here" "$rc/co/src" "$rc/co/target/teq" "$rc/out" && cp bench/airstream/pin.json "$rc/here/" && echo '{}' > "$rc/here/generated.json"
printf 'projects:\n  airstream:\n    base: .\n    configurations:\n      compile:\n        generators:\n          - kind: command\n            outputs:\n              - src/Gen.scala\n' > "$rc/co/target/teq/teq.lock"
echo pin > "$rc/co/src/Gen.scala"
git -C "$rc/co" init -q && git -C "$rc/co" add src && git -C "$rc/co" -c user.name=t -c user.email=t@localhost commit -q -m pin
cat > "$rc/teq" <<SH
#!/bin/sh
case \$1 in
  compile) [ "\$2" != --diagnostics ] || : > "\$3"; cp $rc/gen.txt src/Gen.scala; echo "airstream/compile: checked in 1 ms" ;;
  --version) echo "teq 0.1.8 abcdef123" ;;
esac
exit 0
SH
chmod +x "$rc/teq"
rverb() {
  timeout 120 "$TEQ" interp tests/support/reference_compile_verb.scala --main referenceCompileVerb -- "$rc/here" "$rc/co" "$rc/out" "$rc/teq" "$1" > "$work/verb.log" 2>&1
  local code=$? want=$2 what=$3
  if { [ "$want" = pass ] && [ $code = 0 ]; } || { [ "$want" = fail ] && [ $code != 0 ]; }; then pass=$((pass + 1)); else
    echo "FAIL $what: expected the compile verb to $want, exit $code: $(tail -5 "$work/verb.log")"; fail=$((fail + 1)); fi
}
echo A > "$rc/gen.txt"
rverb baseline pass "a baseline of a generated file with wrong bytes"
grep -q "^error generated src/Gen.scala: $(printf 'A\n' | sha256sum | cut -d' ' -f1), not the pin's bytes$" "$rc/here/errors.txt" && pass=$((pass + 1)) ||
  { echo "FAIL the baseline's item names no digest of the bytes: $(grep '^error' "$rc/here/errors.txt")"; fail=$((fail + 1)); }
rverb judge pass "the same wrong bytes again"
echo B > "$rc/gen.txt"
rverb judge fail "other wrong bytes"
grep -q "FAIL airstream: compile run 1: 1 error new" "$work/verb.log" || { echo "FAIL other wrong bytes: not a new item"; fail=$((fail + 1)); }
echo pin > "$rc/gen.txt"
rverb judge pass "the pin's bytes"
grep -q "gone 1 of the record's 1" "$work/verb.log" && [ -z "$(git -C "$rc/co" status --porcelain --untracked-files=no)" ] && pass=$((pass + 1)) ||
  { echo "FAIL the pin's bytes: not gone, or the checkout not put back: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
echo "reference-test-verb: $pass passed, $fail failed"
[ $fail = 0 ]
