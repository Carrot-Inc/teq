#!/bin/bash
# bench/mill-reference.scala's test verb holds `teq test`'s run to the oracle's inventory over the export's modules: a
# stand-in teq lists each module's suites and prints a transcript, and the verb passes only where the export's modules,
# the suites listed and run and the cases are tests.json's, identity for identity, every case passed
# (tests/support/mill_test_verb.scala). Scenarios: the run as recorded (an ignored suite reports its one case under
# no heading), and with its headings coloured as the JVM's runner prints them; no inventory; a module removed from
# tests.json (its cases then not compared); a module of tests.json the export lacks; an empty suite of tests.json not
# listed; a suite listed that tests.json lacks; a listed suite that did not run; a renamed case; a failed case; a case
# reported twice, a skipped case ignored, a case failed then passed, each a difference of its statuses as many times
# as each side reports it; an inventory of outcomes alone, without the framework's statuses; an export made with other
# settings than the pin's; a failing run of a module followed by a passing one, a module completed twice; every case as
# recorded with an exit of 1, judged and recorded; a module whose listing fails in its own build, blocked by it; a
# blocked module's baseline holding its build's error, another error and a count without its diagnostic failing, and
# its listing printing that error yet its test running to an error, an execution the failure contradicts, failing; a
# new build error of no module of tests.json's failing; over a record of core.js's run blocked by its build, an executed
# failure after the run's last summary with its events missing or in them, a test framework not loading beside the known
# error and a summary counting a failure no outcome or event shows each failing; a passing test printing a diagnostic,
# a line of teq's, a count, a summary or an outcome before its run's summary passing over a record of none, the first
# over a record of a failing case too, judged and recorded, the record unchanged, and over the latter, the run without its summary and a
# diagnostic before core.js's end with nothing run each failing, over a record of a known build error and the failing
# case, a new error on teq's stderr without its count before every case passing failing, and over a record of an error
# whose message holds a caret line, the message changed after it failing;
# over Tyrian's committed run record (blocked), a run reaching a counter of 0 to 2 failing, one holding every
# assertion passing, a build failing in another error and one failing with none failing, the recorded error with an
# exit of 1 within the record and with a signal's 137 failing, judged and recorded, and counted as two errors failing,
# and every assertion held while the smoke exits 1 or ends `FAIL smoke:` failing over that record and over an empty
# one, judged and recorded, the record's bytes unchanged (tests/support/mill_run_verb.scala);
# a case's status its framework's from the run's events, an error where tests.json has an error passing and a failure
# failing, and over a record of the case's failure a later error failing; a module whose test sources gain a file, whose listing then fails.
# A listing naming a path of non-ASCII characters, in the module's own listing and in a module's before it, leaves each
# run's transcript its own. A module is asked for its suites only where its test configuration's sources hold one (app.js's directory is empty,
# core.jvm has no test configuration). A failing scenario fails with its own line in the verb's output.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
pass=0
fail=0
mkdir -p "$work/checkout/target/teq" "$work/out" "$work/checkout/app/test" "$work/checkout/core/test/p"
: > "$work/checkout/core/test/p/OneSuite.scala"
printf '{"build": ".", "overlay": {"teq.mill.json": "aa11"}}\n' > "$work/pin.json"
lock() {
  cat > "$work/checkout/target/teq/teq.lock" <<LOCK
teq: 0.1.8
inputs:
  files:
    build.mill: 00ff
    teq.mill.json: ${1:-aa11}
  sha256: 1234
projects:
  app.js:
    base: app
    configurations:
      compile:
        sources:
          - app/src
      test:
        sources:
          - app/test
    platform: js
  core.js:
    base: core
    configurations:
      compile:
        sources:
          - core/src
      test:
        sources:
          - core/test
    platform: js
  core.jvm:
    base: core
    configurations:
      compile:
        sources:
          - core/src
    platform: jvm
LOCK
}
inventory() {
  rm -rf "$work/here"
  mkdir -p "$work/here"
  cp "$work/pin.json" "$work/here/"
  cat > "$work/here/tests.json"
}
recorded_inventory='{
 "modules": {
  "app.js": { "warnings": 0, "suites": [], "cases": {} },
  "core.js": { "warnings": 2, "suites": [ "p.EmptySuite", "p.IgnoredSuite", "p.OneSuite", "p.TwoSuite" ], "cases": { "Success": [ "p.OneSuite.adds", "p.TwoSuite.parses: twice" ], "Skipped": [ "p.OneSuite.skips" ], "Ignored": [ "p.IgnoredSuite.<unknown>" ] } },
  "core.jvm": { "warnings": 0, "suites": [], "cases": {} }
 }
}'
# The listing and the transcript the stand-in prints for core.js (app.js lists no suite), its summaries counted as
# teq's tally counts them (tests/support/tally_summaries.py; as written where $work/raw is there), each printed on teq's
# two channels (tests/support/teq_channels.py: teq's own lines on stderr, a line `1> ...` a suite's printing on
# stdout), then the stand-in, which writes the run's events where --events names (tests/support/mill_events.py: the
# transcript's outcomes, `[fail]` a Failure; events-override.txt in their place where it is there, an Error among them).
standin() {
  printf '%s\n' "$1" > "$work/listing.txt"
  printf '%s\n' "$2" > "$work/transcript.txt"
  cat > "$work/teq" <<SH
#!/bin/sh
if [ "\$1 \$2 \$3" = "test --verbose --events" ]; then
  events=\$4
  shift 4
  set -- test --verbose "\$@"
fi
diag=
if [ "\$3" = --diagnostics ]; then diag=\$4; set -- "\$1" "\$2" "\$5"; fi
[ "\$4" != --diagnostics ] || diag=\$5
[ -z "\$diag" ] || [ -e $work/no-diagnostics ] || : > "\$diag"
records=; [ -z "\$diag" ] || [ -e $work/no-diagnostics ] || records="--records \$diag"
case "\$1 \$2 \$3" in
  "test --list core.js") python3 $PWD/tests/support/teq_channels.py \$records < $work/listing.txt ;;
  "test --verbose core.js") { if [ -e $work/raw ]; then cat $work/transcript.txt; else python3 $PWD/tests/support/tally_summaries.py $work/transcript.txt; fi; } | python3 $PWD/tests/support/teq_channels.py \$records; [ -z "\$events" ] || [ -e $work/no-events ] || python3 $PWD/tests/support/mill_events.py $work core.js > "\$events" ;;
  "test --list app.js") [ ! -e $work/listing-app.txt ] || { python3 $PWD/tests/support/teq_channels.py \$records < $work/listing-app.txt; exit 0; }; echo "teq: \$3/test: no suite of the test frameworks among the test sources: a run of nothing is no pass" >&2; exit 1 ;;
  "test --list "*) echo "teq: \$3/test: no suite of the test frameworks among the test sources: a run of nothing is no pass" >&2; exit 1 ;;
esac
exit 0
SH
  chmod +x "$work/teq"
}
listing='Test p.EmptySuite : subclass(false, munit.Suite)
Test p.IgnoredSuite : subclass(false, munit.Suite)
Test p.OneSuite : subclass(false, munit.Suite)
Test p.TwoSuite : subclass(false, munit.Suite)'
transcript='p.EmptySuite:
p.OneSuite:
[success] p.OneSuite.adds
[skipped] p.OneSuite.skips
p.TwoSuite:
[success] p.TwoSuite.parses: twice
==> i <unknown> ignored
[ignored] p.IgnoredSuite.<unknown>
core.js/test: 4 suites, 2 passed, 0 failed, 2 skipped in 10 ms (built in 5 ms)'
expect() {
  local what=$1 want=$2 line=$3
  timeout 120 "$TEQ" interp tests/support/mill_test_verb.scala --main millTestVerb -- "$work/here" "$work/checkout" "$work/out" "$work/teq" > "$work/verb.log" 2>&1
  local code=$?
  if { [ "$want" = pass ] && [ $code = 0 ]; } || { [ "$want" = fail ] && [ $code != 0 ] && grep -qF -- "$line" "$work/verb.log"; }; then
    pass=$((pass + 1))
  else
    echo "FAIL $what: expected the verb to $want${line:+ with \"$line\"}, exit $code: $(tail -5 "$work/verb.log")"
    fail=$((fail + 1))
  fi
}
lock
standin "$listing" "$transcript"
printf '%s\n' "$recorded_inventory" | inventory
expect "the run as recorded" pass
standin "$listing" "$(printf '%s\n' "$transcript" | sed 's/^\(p\.[A-Za-z]*:\)$/\x1b[32m\1\x1b[0m/')"
expect "the run as recorded, its headings coloured" pass
grep -q $'\x1b\[32mp.OneSuite:' "$work/transcript.txt" || { echo "FAIL the coloured transcript holds no coloured heading"; fail=$((fail + 1)); }
# Each invocation's transcript is its own, whatever an invocation before it printed in UTF-8: the run as recorded passes
# with a non-ASCII source path in core.js's listing, and in app.js's listing before it.
standin "$(printf '%s\n' "$listing" '[warn] source path /tmp/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/Test.scala')" "$transcript"
expect "the run as recorded, its listing naming a path of non-ASCII characters" pass
standin "$listing" "$transcript"
printf '%s\n' '[warn] source path /tmp/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/日本語/Test.scala' > "$work/listing-app.txt"
: > "$work/checkout/app/test/Uni.scala"
expect "the run as recorded, a module's listing before it naming such a path" pass
rm -f "$work/checkout/app/test/Uni.scala" "$work/listing-app.txt"
standin "$listing" "$transcript"
rm "$work/here/tests.json"
expect "no inventory" fail "no inventory at"
printf '%s\n' "$recorded_inventory" | grep -v '"app.js"' | inventory
expect "a module removed from tests.json" fail "module app.js is not in tests.json"
printf '%s\n' "$recorded_inventory" | sed 's|^  "core.jvm": .*|  "core.jvm": { "warnings": 0, "suites": [], "cases": {} },\n  "extra.js": { "warnings": 0, "suites": [], "cases": {} }|' | inventory
expect "a module of tests.json the export lacks" fail "module extra.js of tests.json"
printf '%s\n' "$recorded_inventory" | inventory
standin "$(printf '%s\n' "$listing" | grep -v EmptySuite)" "$(printf '%s\n' "$transcript" | grep -v EmptySuite)"
expect "an empty suite of tests.json not listed" fail "suite p.EmptySuite of tests.json is not the run's"
printf '%s\n' "$recorded_inventory" | sed 's|"p.EmptySuite", ||' | inventory
standin "$listing" "$transcript"
expect "a suite listed that tests.json lacks" fail "suite p.EmptySuite is not in tests.json"
printf '%s\n' "$recorded_inventory" | inventory
standin "$listing" "$(printf '%s\n' "$transcript" | grep -v '^p.EmptySuite:$')"
expect "a listed suite whose run the events do not name" fail "suite p.EmptySuite was listed and did not run"
# An event names its suite's test (`<suite>.<test>`, as Scala.js's test bridge names it), counted toward its listed
# suite: the run as recorded with each event so named passes.
standin "$listing" "$transcript"
python3 tests/support/mill_events.py "$work" core.js | python3 -c '
import sys
for line in sys.stdin:
    f = line.rstrip("\n").split("\t")
    f[1] = f[2][5:] if f[2].startswith("test ") else f[1]
    print("\t".join(f))' > "$work/events.new" && mv "$work/events.new" "$work/events-override.txt"
expect "the run as recorded, each event named by its suite's test" pass
rm -f "$work/events-override.txt"
standin "$listing" "$(printf '%s\n' "$transcript" | sed 's/OneSuite.adds/OneSuite.added/')"
expect "a renamed case" fail 'case "p.OneSuite.adds" did not run'
standin "$listing" "$(printf '%s\n' "$transcript" | sed 's/^\[success\] p.TwoSuite.parses: twice$/[fail] p.TwoSuite.parses: twice: java.lang.AssertionError: boom/')"
expect "a failed case" fail 'case "p.TwoSuite.parses: twice" Failure, tests.json has Success'
# The cases' identity: as many times as each side reports a case, each with its framework's status.
standin "$listing" "$(printf '%s\n' "$transcript" | sed 's/^\[success\] p.OneSuite.adds$/&\n&/')"
expect "a case reported twice" fail 'case "p.OneSuite.adds" Success x2, tests.json has Success'
standin "$listing" "$(printf '%s\n' "$transcript" | sed 's/^\[skipped\] p.OneSuite.skips$/[ignored] p.OneSuite.skips/')"
expect "a skipped case ignored" fail 'case "p.OneSuite.skips" Ignored, tests.json has Skipped'
standin "$listing" "$(printf '%s\n' "$transcript" | sed 's/^\[success\] p.OneSuite.adds$/[fail] p.OneSuite.adds: java.lang.AssertionError: boom\n&/')"
expect "a case failed, then passed" fail 'case "p.OneSuite.adds" Failure, Success, tests.json has Success'
printf '%s\n' "$recorded_inventory" | sed 's|"cases": { "Success": .* } },$|"cases": { "p.OneSuite.adds": "passed" } },|' | inventory
standin "$listing" "$transcript"
expect "an inventory of outcomes alone, without the statuses" fail "no status of sbt.testing.Status's"
printf '%s\n' "$recorded_inventory" | inventory
standin "$listing" "$transcript"
lock bb22
expect "an export made with other settings than the pin's" fail "records teq.mill.json as bb22"
lock
: > "$work/checkout/app/test/Added.scala"
expect "a module whose test sources gain a file, whose listing then fails" fail "app.js --list: teq test exits 1"
rm "$work/checkout/app/test/Added.scala"
expect "the run as recorded, again" pass
# A module's later section never hides an earlier one: a failing run of core.js followed by a passing one is a
# module that completed twice, and fails. An exit no failing case and no blocked build explains fails, judged and
# recorded, the record left as it was.
standin "$listing" "$(printf '%s\n' "$transcript" | sed 's/^\[success\] p.OneSuite.adds$/[fail] p.OneSuite.adds: java.lang.AssertionError: boom/'; printf '%s\n' "$transcript")"
expect "a failing run of a module followed by a passing one" fail "core.js: its tests completed 2 times"
standin "$listing" "$transcript"
sed -i 's|^\(  "test --verbose core.js") .*\) ;;$|\1; exit 1 ;;|' "$work/teq"
expect "every case as recorded and an exit of 1" fail "exits 1, no case failing and no build failing in it"
cp "$work/here/deviations.json" "$work/deviations.kept"
timeout 120 "$TEQ" interp tests/support/mill_test_verb.scala --main millTestVerb -- "$work/here" "$work/checkout" "$work/out" "$work/teq" record > "$work/verb.log" 2>&1
code=$?
[ $code != 0 ] && cmp -s "$work/here/deviations.json" "$work/deviations.kept" && pass=$((pass + 1)) || { echo "FAIL the unexplained exit's --record: exit $code, or the record changed"; fail=$((fail + 1)); }
# A module whose listing fails in its own build is blocked by it: its cases are departures (over a record of none,
# new ones), never a listing that failed otherwise.
standin "$(printf '%s\n' 'core/src/A.scala:1:1: error: Not found: x' 'x' '^' 'core.js/test: 1 error')" ""
sed -i 's|^  "test --list core.js") .*|  "test --list core.js") python3 '"$PWD"'/tests/support/teq_channels.py $records < '"$work"'/listing.txt; exit 1 ;;|' "$work/teq"
expect "a module blocked by its own build" fail "core.js: blocked by core.js/test core/src/A.scala:1:1: error: Not found: x"
grep -q "no build of the module's failing" "$work/verb.log" && { echo "FAIL a module blocked by its own build: taken for a listing that failed otherwise"; fail=$((fail + 1)); } || pass=$((pass + 1))
# A blocked module's record holds its build's errors as a compile's record does: a baseline of core.js's listing
# failing in S.scala:1:1, then another error, then the count alone without its diagnostic, each fail.
mverb() { timeout 120 "$TEQ" interp tests/support/mill_test_verb.scala --main millTestVerb -- "$work/here" "$work/checkout" "$work/out" "$work/teq" $1 > "$work/verb.log" 2>&1; }
blocked_listing() {
  standin "$1" "$transcript"
  sed -i 's|^  "test --list core.js") .*|  "test --list core.js") python3 '"$PWD"'/tests/support/teq_channels.py $records < '"$work"'/listing.txt; exit 1 ;;|' "$work/teq"
}
printf '%s\n' "$recorded_inventory" | inventory
blocked_listing "$(printf '%s\n' 'S.scala:1:1: error: Not found: a' 'a' '^' 'core.js/test: 1 error')"
mverb baseline
code=$?
grep -q -F '"core.js: blocked by core.js/test S.scala:1:1: error: Not found: a\na\n^"' "$work/here/deviations.json" 2> /dev/null && [ $code = 0 ] && pass=$((pass + 1)) ||
  { echo "FAIL a blocked module's baseline holds no build error: exit $code, $(grep blocked "$work/here/deviations.json" 2> /dev/null)"; fail=$((fail + 1)); }
blocked_listing "$(printf '%s\n' 'S.scala:2:1: error: Not found: b' 'b' '^' 'core.js/test: 1 error')"
mverb kept
code=$?
[ $code != 0 ] && grep -q "core.js: blocked by core.js/test S.scala:2:1: error: Not found: b" "$work/verb.log" && pass=$((pass + 1)) || { echo "FAIL a blocked module, another error: exit $code"; fail=$((fail + 1)); }
blocked_listing "core.js/test: 1 error"
mverb kept
code=$?
[ $code != 0 ] && grep -q "core.js/test: 1 errors reported without a diagnostic" "$work/verb.log" && pass=$((pass + 1)) || { echo "FAIL a blocked module, a count without its diagnostic: exit $code"; fail=$((fail + 1)); }
# A blocked module's allowance covers the cases its build kept from running, never one that ran: over the record of
# core.js blocked by S.scala:1:1, its listing printing that error yet listing p.TwoSuite, whose test then runs to an
# Error with its summary, is an execution its build's failure contradicts, and fails, judged and recorded.
blocked_listing "$(printf '%s\n' 'S.scala:1:1: error: Not found: a' 'a' '^' 'core.js/test: 1 error' 'Test p.TwoSuite : subclass(false, munit.Suite)')"
printf '%s\n' 'p.TwoSuite:' '[fail] p.TwoSuite.parses: twice: java.lang.Error: boom' 'core.js/test: 1 suite, 0 passed, 1 failed in 1 ms (built in 1 ms)' > "$work/transcript.txt"
sed -i 's|^\(  "test --verbose core.js") .*\) ;;$|\1; exit 1 ;;|' "$work/teq"
printf 'core.js\tp.TwoSuite\ttest p.TwoSuite.parses: twice\tError\t\n' > "$work/events.new" && mv "$work/events.new" "$work/events-override.txt"
cp "$work/here/deviations.json" "$work/deviations.kept"
mverb kept
code=$?
mverb record
code2=$?
[ $code != 0 ] && [ $code2 != 0 ] && grep -q "core.js: a build of its failed and 1 of its cases ran: an execution its build's failure contradicts" "$work/verb.log" &&
  cmp -s "$work/here/deviations.json" "$work/deviations.kept" && pass=$((pass + 1)) ||
  { echo "FAIL a blocked module whose test ran to an error: exits $code and $code2, or the record changed: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
rm -f "$work/events-override.txt"
# The class of the records' judgment of a test run (`Deviations.judge`, one function for every driver; the same cases
# in tests/lila-test-verb.sh and tests/reference-test-verb.sh), each over a record of core.js blocked, judged and
# recorded: a new build error of a configuration of no module of tests.json's (gen.js) fails; over a record of core.js's
# run blocked by its build, an executed failure after the run's last summary, with its events missing, or in them (its outcomes are its events), fails.
mjudged_and_recorded() {
  local what=$1
  cp "$work/here/deviations.json" "$work/deviations.kept"
  mverb kept
  local code=$?
  mverb record
  local code2=$?
  if [ $code != 0 ] && [ $code2 != 0 ] && cmp -s "$work/here/deviations.json" "$work/deviations.kept" && grep -q "$2" "$work/verb.log"; then pass=$((pass + 1)); else
    echo "FAIL $what: exits $code and $code2, or the record changed, or no \"$2\": $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); fi
}
rm -f "$work/events-override.txt"
blocked_listing "$(printf '%s\n' 'S.scala:1:1: error: Not found: a' 'a' '^' 'core.js/test: 1 error' 'gen.js/G.scala:9:1: error: Not found: n' 'n' '^' 'gen.js/test: 1 error')"
mjudged_and_recorded "a new build error of no module of tests.json's" "^  gen.js/test gen.js/G.scala:9:1: error: Not found: n$"
run_blocked='core/test/R.scala:1:1: error: Not found: r
r
^
core.js/test: 1 error'
standin "$listing" "$run_blocked"
sed -i 's|^\(  "test --verbose core.js") .*\) ;;$|\1; exit 1 ;;|' "$work/teq"
printf '' > "$work/events.new" && mv "$work/events.new" "$work/events-override.txt"
mverb baseline
code=$?
grep -q -F '"core.js: blocked by core.js/test core/test/R.scala:1:1: error: Not found: r\nr\n^"' "$work/here/deviations.json" 2> /dev/null && [ $code = 0 ] && pass=$((pass + 1)) ||
  { echo "FAIL a baseline of core.js's run blocked: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
printf '%s\n' "$run_blocked" '[fail] p.OneSuite.adds: java.lang.Error: newly executed failure' > "$work/transcript.txt"
touch "$work/no-events"
mjudged_and_recorded "an executed failure after the last summary, no events" "it wrote no events"
rm -f "$work/no-events"
rm -f "$work/events-override.txt"
mjudged_and_recorded "an executed failure after the last summary, in the run's events" "core.js: a build of its failed and 1 of its cases ran"
touch "$work/raw"
printf '%s\n' "$run_blocked" 'teq: core.js/test: the test framework example.Framework does not load: java.lang.NoClassDefFoundError: NewlyMissing' > "$work/transcript.txt"
mjudged_and_recorded "a test framework not loading beside the known build error" "the test framework example.Framework does not load: java.lang.NoClassDefFoundError: NewlyMissing: a failure of the test run's"
printf '%s\n' "$run_blocked" "teq: core.js/test: no memory mapping for the type store's overlays; one worker types the build" 'teq: core.js/test: the test framework example.Framework does not load: java.lang.NoClassDefFoundError: NewlyMissing' > "$work/transcript.txt"
mjudged_and_recorded "the overlays' note, then a test framework not loading, beside the known build error" "the test framework example.Framework does not load: java.lang.NoClassDefFoundError: NewlyMissing: a failure of the test run's"
printf '%s\n' "$run_blocked" "teq: core.js/test: no memory mapping for the type store's overlays; one worker types the build" 'core/test/New.scala:7:1: error: Not found: NEW' 'NEW' '^' > "$work/transcript.txt"
mjudged_and_recorded "the overlays' note, then a new error no summary claims, beside the known build error" "1 errors after core.js/test's line of teq's, claimed by no summary"
printf '%s\n' 'p.OneSuite:' 'core.js/test: 1 suite, 0 passed, 1 failed in 1 ms (built in 1 ms)' > "$work/transcript.txt"
mjudged_and_recorded "a summary counting a failure no outcome and no event shows" "core.js: its summary counts 1 failed, its events none"
rm -f "$work/events-override.txt" "$work/raw"
# A suite ran where the run's events name it (its own line, `Deviations.ran`), never where a count has room for it
# (the review of b4d60c3aa): over core.js's listing of p.EmptySuite and p.OneSuite, a run naming p.OneSuite alone with
# its summary counting 2 suites fails, over a record of none and over a record of a failing case, judged and recorded,
# the record unchanged; and over a baseline of p.EmptySuite missing under a summary of 1 suite, the same run with the
# count alone made 2 fails, judged and recorded, the record unchanged (it shrank to none where the count stood for the
# suite's run).
two_listing='Test p.EmptySuite : subclass(false, munit.Suite)
Test p.OneSuite : subclass(false, munit.Suite)'
two_inventory='{
 "modules": {
  "app.js": { "warnings": 0, "suites": [], "cases": {} },
  "core.js": { "warnings": 0, "suites": [ "p.EmptySuite", "p.OneSuite" ], "cases": { "Success": [ "p.OneSuite.adds" ] } },
  "core.jvm": { "warnings": 0, "suites": [], "cases": {} }
 }
}'
missing_run() { printf '%s\n' 'p.OneSuite:' "[$1] p.OneSuite.adds" "core.js/test: $2 suite$([ "$2" = 1 ] || echo s), $([ "$1" = success ] && echo 1 || echo 0) passed, $([ "$1" = fail ] && echo 1 || echo 0) failed in 1 ms (built in 1 ms)"; }
touch "$work/raw"
standin "$two_listing" "$(missing_run success 2)"
printf '%s\n' "$two_inventory" | inventory
expect "a listed suite the events do not name, its summary counting both, over a record of none" fail "suite p.EmptySuite was listed and did not run"
standin "$two_listing" "$(printf '%s\n' 'p.EmptySuite:' 'p.OneSuite:' '[fail] p.OneSuite.adds' 'core.js/test: 2 suites, 0 passed, 1 failed in 1 ms (built in 1 ms)')"
sed -i 's|^\(  "test --verbose core.js") .*\) ;;$|\1; exit 1 ;;|' "$work/teq"
mverb baseline
code=$?
grep -q -F '"core.js: p.OneSuite.adds: Failure (tests.json: Success)"' "$work/here/deviations.json" 2> /dev/null && [ $code = 0 ] && pass=$((pass + 1)) ||
  { echo "FAIL a baseline of a failing case beside both suites run: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
standin "$two_listing" "$(missing_run fail 2)"
sed -i 's|^\(  "test --verbose core.js") .*\) ;;$|\1; exit 1 ;;|' "$work/teq"
mjudged_and_recorded "a listed suite the events do not name, its summary counting both, over a record of a failing case" "suite p.EmptySuite was listed and did not run"
standin "$two_listing" "$(missing_run success 1)"
mverb baseline
code=$?
grep -q -F '"core.js: suite p.EmptySuite was listed and did not run"' "$work/here/deviations.json" 2> /dev/null && [ $code = 0 ] && pass=$((pass + 1)) ||
  { echo "FAIL a baseline of p.EmptySuite missing under a summary of 1 suite: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
standin "$two_listing" "$(missing_run success 2)"
mjudged_and_recorded "the count alone made 2, the suite still missing" "its summary counts 2 suites, its events name 1"
rm -f "$work/raw"
printf '%s\n' "$recorded_inventory" | inventory
# A diagnostic's message is never a configuration's summary (`Transcript.scan`'s `summaries`, read past the records'
# lines): core.js's build warning whose message holds core.js's summary, then the run as recorded, passes over a
# record of none, judged and recorded, unchanged; the same with the run's summary left out fails, judged and recorded,
# the record unchanged (the warning standing for the run's end).
warned=$(printf '%s\n' 'core/test/W.scala:1:1: warning: method value in object Old is deprecated since 1: sample' '+> core.js/test: 4 suites, 2 passed, 0 failed, 2 skipped in 10 ms (built in 5 ms)' 'w' '^')
standin "$listing" "$(printf '%s\n' "$warned" "$transcript")"
printf '%s\n' "$recorded_inventory" | inventory
expect "a warning holding core.js's summary, over a record of none" pass
cp "$work/here/deviations.json" "$work/deviations.kept"
mverb kept
code=$?
mverb record
code2=$?
[ $code = 0 ] && [ $code2 = 0 ] && cmp -s "$work/here/deviations.json" "$work/deviations.kept" && pass=$((pass + 1)) ||
  { echo "FAIL a warning holding core.js's summary, judged and recorded: exits $code and $code2, or the record changed: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
standin "$listing" "$(printf '%s\n' "$warned" "$transcript" | grep -v '^core.js/test: ')"
mjudged_and_recorded "core.js's summary left out, its warning holding one" "ran without its tests' completing"
# What a suite prints is its run's stream on teq's stdout, never one of teq's own lines, which teq writes on stderr
# (Transcript.scan over teq's stderr; the same cases in tests/lila-test-verb.sh and tests/reference-test-verb.sh): a
# passing test printing `Example.scala:1:1: error: ...` before its run's summary passes over a record of none and over
# a record of a failing case, and one printing a line of teq's, a count, a summary or an outcome over a record of none,
# judged and recorded, the record's bytes unchanged; over the failing case's record, the printing run without its
# summary, and a diagnostic on stderr before core.js's end with nothing run, each fail, judged and recorded. A
# diagnostic on stderr no count claims fails whatever run's summary follows it (the review's sequence): over a record of
# gen.js's known build error and the failing case, gen.js's known error and a new one without its count, then every
# case passing, fails, judged and recorded, the record unchanged. A diagnostic is its whole block up to teq's next own
# line, whatever its message holds: over a record of core.js blocked by an error whose message is `prefix`, `^`, `Example.scala:1:1: warning: note`, `OLD`,
# the same passes again and `NEW` for `OLD` fails, judged and recorded.
printed='Example.scala:1:1: error: an expected diagnostic from the test'
printed_transcript() { printf '%s\n' "$1" | sed "s|^core.js/test: |1> $2\\n&|"; }
mheld_unchanged() {
  local what=$1
  cp "$work/here/deviations.json" "$work/deviations.kept"
  mverb kept
  local code=$?
  mverb record
  local code2=$?
  if [ $code = 0 ] && [ $code2 = 0 ] && cmp -s "$work/here/deviations.json" "$work/deviations.kept" && grep -q -F "1> $2" "$work/transcript.txt"; then pass=$((pass + 1)); else
    echo "FAIL $what: exits $code and $code2, or the record changed: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); fi
}
for shown in "$printed" 'teq: core.js/test: the test framework example.Framework does not load: expected fixture output' \
  'core.js/test: 1 error' 'core.js/test: 1 suite, 1 passed, 0 failed in 1 ms (built in 1 ms)' '[fail] p.OneSuite.adds: java.lang.AssertionError: printed'; do
  standin "$listing" "$(printed_transcript "$transcript" "$shown")"
  printf '%s\n' "$recorded_inventory" | inventory
  expect "a passing test printing \"$shown\", over a record of none" pass
  mheld_unchanged "a passing test printing \"$shown\", over a record of none, judged and recorded" "$shown"
done
failing_run=$(printf '%s\n' "$transcript" | sed 's/^\[success\] p.OneSuite.adds$/[fail] p.OneSuite.adds: java.lang.AssertionError: boom/')
standin "$listing" "$(printed_transcript "$failing_run" "$printed")"
sed -i 's|^\(  "test --verbose core.js") .*\) ;;$|\1; exit 1 ;;|' "$work/teq"
mverb baseline
code=$?
grep -q -F '"core.js: p.OneSuite.adds: Failure (tests.json: Success)"' "$work/here/deviations.json" 2> /dev/null && [ $code = 0 ] && pass=$((pass + 1)) ||
  { echo "FAIL a baseline of a failing case beside a test printing a diagnostic: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
mheld_unchanged "a passing test printing a diagnostic beside the known failing case" "$printed"
grep -v '^core.js/test: ' "$work/transcript.txt" > "$work/transcript.new" && mv "$work/transcript.new" "$work/transcript.txt"
mjudged_and_recorded "a test printing a diagnostic in a run without its summary" "of its cases ran without its tests' completing (no summary)"
printf '%s\n' "$printed" 'core.js/test: no suite to run' > "$work/transcript.txt"
mjudged_and_recorded "a diagnostic before core.js's end with nothing run" "1 errors before core.js/test's end, claimed by no count"
gen_known='gen.js/Known.scala:1:1: error: Not found: KNOWN
KNOWN
^
gen.js/test: 1 error'
printf '%s\n' "$gen_known" "$failing_run" > "$work/transcript.txt"
mverb baseline
code=$?
grep -q -F 'gen.js/test gen.js/Known.scala:1:1: error: Not found: KNOWN' "$work/here/deviations.json" 2> /dev/null && [ $code = 0 ] && pass=$((pass + 1)) ||
  { echo "FAIL a baseline of gen.js's known build error and the failing case: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
printf '%s\n' "$gen_known" 'gen.js/New.scala:7:1: error: Not found: NEW' 'NEW' '^' "$transcript" > "$work/transcript.txt"
mjudged_and_recorded "a new error on teq's stderr without its count, then every case passing" "1 errors before core.js/test's end, claimed by no count"
# A run's events are its outcomes, never its report's lines on stdout, whose names and messages are the suites': a real
# `teq test` (tests/support/probe_run.sh) of a case passing under a name that runs over lines reading as a diagnostic
# and a count, and of a case failing with a message of those lines and a summary, makes a baseline of the failure alone
# through the production driver, judged and recorded again unchanged; the run's report holds those lines on stdout. Its
# messages' lines separated by LF and by CR LF (`PROBE_CRLF`): a diagnostic's lines are its rendering's, whatever
# separates them (`Transcript.lines`), so the warning's message holding the summary is never a completion.
export LOCK_PY=$PWD/integrations/sbt/example/check-export.py
. tests/support/probe_run.sh
for sep in lf crlf; do
  if why=$(PROBE_CRLF=$(probe_crlf $sep) probe_checkout "$work/probe-co" alpha); then
    rm -rf "$work/probe-here" "$work/probe-out" && mkdir -p "$work/probe-here" "$work/probe-out"
    printf '{"build": "."}\n' > "$work/probe-here/pin.json"
    printf '{"modules": {"alpha": {"warnings": 0, "suites": ["probes.Empty", "probes.Named", "probes.Thrown"], "cases": {"Success": ["%s", "%s"]}}}}\n' "$probe_named" "$probe_thrown" > "$work/probe-here/tests.json"
    teq_abs=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
    real() { timeout 300 "$TEQ" interp tests/support/mill_test_verb.scala --main millTestVerb -- "$work/probe-here" "$work/probe-co" "$work/probe-out" "$teq_abs" "$1" > "$work/verb.log" 2>&1; }
    real baseline
    code=$?
    items=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["deviations"])' "$work/probe-here/deviations.json" 2> /dev/null)
    cp "$work/probe-here/deviations.json" "$work/deviations.kept" 2> /dev/null
    real kept
    code2=$?
    real record
    code3=$?
    if [ $code = 0 ] && [ $code2 = 0 ] && [ $code3 = 0 ] && [ "$items" = "['alpha: reports: Failure (tests.json: Success)']" ] && cmp -s "$work/probe-here/deviations.json" "$work/deviations.kept" \
      && grep -q '^Example.scala:1:1: error: sample$' "$work/probe-out/test-1-alpha.log" && ! grep -q 'Example.scala' "$work/probe-out/test-1-alpha.err"; then pass=$((pass + 1)); else
      echo "FAIL a real run whose event names and messages read as teq's lines ($sep): exits $code, $code2 and $code3, items $items: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); fi
    (cd "$work/probe-co" && timeout 60 "$teq_abs" stop > /dev/null 2>&1)
  else
    echo "FAIL a real run whose event names and messages read as teq's lines ($sep): $why"; fail=$((fail + 1))
  fi
done
caret_error() { printf '%s\n' 'src/Printed.scala:4:28: error: prefix' '^' '+> Example.scala:1:1: warning: note' "$1" '  test("prints a count") { Message.broken }' '                           ^^^^^^^^^^^^^^' 'core.js/test: 1 error'; }
standin "$listing" "$(caret_error OLD)"
sed -i 's|^\(  "test --verbose core.js") .*\) ;;$|\1; exit 1 ;;|' "$work/teq"
printf '' > "$work/events-override.txt"
mverb baseline
code=$?
grep -q -F '"core.js: blocked by core.js/test src/Printed.scala:4:28: error: prefix\n^\nExample.scala:1:1: warning: note\nOLD\n  test(\"prints a count\") { Message.broken }\n                           ^^^^^^^^^^^^^^"' "$work/here/deviations.json" 2> /dev/null && [ $code = 0 ] && pass=$((pass + 1)) ||
  { echo "FAIL a baseline of core.js blocked by an error whose message holds a caret line: exit $code: $(grep -o 'core.js: blocked[^,]*' "$work/here/deviations.json")"; fail=$((fail + 1)); }
mverb kept
code=$?
[ $code = 0 ] && pass=$((pass + 1)) || { echo "FAIL the error whose message holds a caret line, again: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
standin "$listing" "$(caret_error NEW)"
sed -i 's|^\(  "test --verbose core.js") .*\) ;;$|\1; exit 1 ;;|' "$work/teq"
mjudged_and_recorded "the error's message changed after its caret line" "NEW"
rm -f "$work/events-override.txt"
# A blocked run's record holds its block and its assertions unexecuted, none allowed to fail: over Tyrian's committed
# record (its build blocked), a run reaching the counter that goes from 0 to 2 fails, a run holding every assertion
# passes with the block gone, a build failing in another error and one failing with none each fail. The production
# `Ref.run` (tests/support/mill_run_verb.scala) under a stand-in teq and a stand-in node for bench/smoke.mjs.
rv=$work/rv
mkdir -p "$rv/here/run" "$rv/co/sandbox" "$rv/co/target/teq" "$rv/out" "$rv/bin"
cp bench/tyrian/pin.json bench/tyrian/generated.json bench/tyrian/run.json "$rv/here/" && cp bench/tyrian/run/tyrianapp.js "$rv/here/run/"
printf 'teq: 0.1.8\ninputs:\n  files:\n    teq.mill.json: %s\n  sha256: 1234\nprojects:\n  sandbox.js:\n    base: sandbox\n' \
  "$(python3 -c "import json; print(json.load(open('bench/tyrian/pin.json'))['overlay']['teq.mill.json'])")" > "$rv/co/target/teq/teq.lock"
echo '<html></html>' > "$rv/co/sandbox/index.html"
cat > "$rv/teq" <<SH
#!/bin/sh
case \$1 in
  build) [ "\$3" != --diagnostics ] || : > "\$4"
    if [ -s $rv/build.txt ] && [ -e $rv/build.shown ]; then python3 -u $PWD/tests/support/teq_channels.py --records "\$4" < $rv/build.txt > /dev/null 2>&1; cat $rv/build.shown >&2; exit 1; fi
    if [ -s $rv/build.txt ]; then python3 -u $PWD/tests/support/teq_channels.py --records "\$4" < $rv/build.txt 2>&1; exit \$(cat $rv/build.exit 2> /dev/null || echo 1); elif [ -e $rv/build.txt ]; then exit 1; fi
    mkdir -p target/teq/sandbox.js/out && echo x > target/teq/sandbox.js/out/main.js ;;
  --version) echo "teq 0.1.8 abcdef123" ;;
esac
exit 0
SH
cat > "$rv/bin/node" <<SH
#!/bin/sh
if [ "\$1" = bench/smoke.mjs ] && [ "\$3" = --assertions ] && [ -e $rv/listing.txt ]; then cat $rv/listing.txt; exit \$(cat $rv/listing.exit); fi
if [ "\$1" = bench/smoke.mjs ] && [ "\$3" != --assertions ] && [ "\$2" != webgl ]; then cat $rv/smoke.txt; exit \$(cat $rv/smoke.exit); fi
exec $(command -v node) "\$@"
SH
chmod +x "$rv/teq" "$rv/bin/node"
rrun() { (PATH="$rv/bin:$PATH" timeout 120 "$TEQ" interp tests/support/mill_run_verb.scala --main millRunVerb -- "$rv/here" "$rv/co" "$rv/out" "$rv/teq" $1) > "$work/verb.log" 2>&1; }
smoke() { printf '%s\n' "$@" > "$rv/smoke.txt"; }
smoke "assert the app's page 2: ok" "assert a counter after insert: ok" "assert the counter one more after +: FAIL the counter went from 0 to 2, not 1" \
  "assert no exception, console error or failed request: not reached" "FAIL smoke: tyrian the counter went from 0 to 2, not 1 (1.0 s)"
echo 1 > "$rv/smoke.exit"
rrun
code=$?
[ $code != 0 ] && grep -q "the counter one more after +: failed" "$work/verb.log" && pass=$((pass + 1)) || { echo "FAIL a run reaching a counter of 0 to 2 over the blocked record: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
smoke "assert the app's page 2: ok" "assert a counter after insert: ok" "assert the counter one more after +: ok" "assert no exception, console error or failed request: ok" "smoke: tyrian counter 0 -> 1 (1.0 s)"
echo 0 > "$rv/smoke.exit"
rrun
code=$?
[ $code = 0 ] && grep -q "gone" "$work/verb.log" && pass=$((pass + 1)) || { echo "FAIL a run holding every assertion over the blocked record: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
printf '%s\n' 'S.scala:1:1: error: Not found: zz' 'zz' '^' > "$rv/build.txt"
rrun
code=$?
[ $code != 0 ] && grep -q "blocked: teq build sandbox.js: S.scala:1:1: error: Not found: zz" "$work/verb.log" && pass=$((pass + 1)) || { echo "FAIL a build blocked by another error: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
: > "$rv/build.txt"
rrun
code=$?
[ $code != 0 ] && grep -q "failed with no error of its" "$work/verb.log" && pass=$((pass + 1)) || { echo "FAIL a build failing with no error: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
# A case's status is its framework's, an error apart from a failure (the transcript's `[fail]` is both): with tests.json
# holding p.TwoSuite.parses: twice as an Error, the run's Error passes and its Failure fails; over a record of its
# Failure departing from the oracle's Success, a later Error fails.
blocked_listing_off() { standin "$listing" "$1"; }
printf '%s\n' "$recorded_inventory" | sed 's|"Success": \[ "p.OneSuite.adds", "p.TwoSuite.parses: twice" \]|"Success": [ "p.OneSuite.adds" ], "Error": [ "p.TwoSuite.parses: twice" ]|' | inventory
failing=$(printf '%s\n' "$transcript" | sed 's/^\[success\] p.TwoSuite.parses: twice$/[fail] p.TwoSuite.parses: twice: java.lang.Error: boom/')
blocked_listing_off "$failing"
python3 tests/support/mill_events.py "$work" core.js | sed 's/\tFailure\t/\tError\t/' > "$work/events.new" && mv "$work/events.new" "$work/events-override.txt"
expect "an error where tests.json has an error" pass
rm "$work/events-override.txt"
expect "a failure where tests.json has an error" fail 'p.TwoSuite.parses: twice: Failure (tests.json: Error)'
printf '%s\n' "$recorded_inventory" | inventory
mverb baseline
code=$?
grep -q -F '"core.js: p.TwoSuite.parses: twice: Failure (tests.json: Success)"' "$work/here/deviations.json" 2> /dev/null && [ $code = 0 ] && pass=$((pass + 1)) ||
  { echo "FAIL a baseline of a failure: exit $code, $(grep twice "$work/here/deviations.json" 2> /dev/null)"; fail=$((fail + 1)); }
python3 tests/support/mill_events.py "$work" core.js | sed 's/\tFailure\t/\tError\t/' > "$work/events.new" && mv "$work/events.new" "$work/events-override.txt"
mverb kept
code=$?
[ $code != 0 ] && grep -q "p.TwoSuite.parses: twice: Error (tests.json: Success)" "$work/verb.log" && pass=$((pass + 1)) || { echo "FAIL an error over a record of the case's failure: exit $code"; fail=$((fail + 1)); }
rm -f "$work/events-override.txt"
# A blocked build is its errors and teq's exit of 1 on them: the build printing Tyrian's recorded error and exiting 1
# is within the record; printing it and dying by a signal (137) fails, judged and recorded, the record left as it was.
python3 -c "
import json
for v in json.load(open('bench/tyrian/run.json'))['violations']:
    if v.startswith('blocked: teq build sandbox.js: '): print(v[len('blocked: teq build sandbox.js: '):])" > "$rv/build.txt"
rm -f "$rv/build.exit"
rrun
code=$?
[ $code = 0 ] && grep -q "5 violations, within the record's 5" "$work/verb.log" && pass=$((pass + 1)) || { echo "FAIL the recorded blocked build, exit 1: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
echo 137 > "$rv/build.exit"
cp "$rv/here/run.json" "$rv/run.kept"
rrun
code=$?
rrun record
code2=$?
[ $code != 0 ] && [ $code2 != 0 ] && grep -q "teq build sandbox.js exits 137, not as a build failing on its errors does (1)" "$work/verb.log" && cmp -s "$rv/here/run.json" "$rv/run.kept" && pass=$((pass + 1)) ||
  { echo "FAIL the recorded blocked build killed by a signal: exits $code and $code2: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
rm -f "$rv/build.exit"
# A build's count of its errors is its diagnostics': the recorded error followed by `2 errors found` fails, judged and
# recorded, the record left as it was; followed by `1 error found` it is within the record.
echo '2 errors found' >> "$rv/build.txt"
rrun
code=$?
rrun record
code2=$?
[ $code != 0 ] && [ $code2 != 0 ] && grep -q "2 errors counted, 1 printed: errors without their diagnostics" "$work/verb.log" && cmp -s "$rv/here/run.json" "$rv/run.kept" && pass=$((pass + 1)) ||
  { echo "FAIL the recorded error counted twice: exits $code and $code2, or the record changed: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
sed -i 's/^2 errors found$/1 error found/' "$rv/build.txt"
rrun
code=$?
[ $code = 0 ] && pass=$((pass + 1)) || { echo "FAIL the recorded error counted once: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
# A one-shot build's output shows its records whole: over Tyrian's committed record, the build recording its known
# error and showing it whole with `1 error found` passes, judged and recorded, unchanged; recording it and showing
# only `1 error found`, or the error's header alone and the count, each fail, judged and recorded, the record's bytes
# unchanged (`Transcript.scan`'s problems, never dropped for a one-shot build).
cp "$rv/here/run.json" "$rv/run.kept"
rrun record
code=$?
[ $code = 0 ] && cmp -s "$rv/here/run.json" "$rv/run.kept" && pass=$((pass + 1)) || { echo "FAIL the recorded error shown whole and counted, recorded: exit $code, or the record changed: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
for shown in count header; do
  if [ $shown = count ]; then echo '1 error found' > "$rv/build.shown"; else { head -1 "$rv/build.txt"; echo '1 error found'; } > "$rv/build.shown"; fi
  rrun
  code=$?
  rrun record
  code2=$?
  [ $code != 0 ] && [ $code2 != 0 ] && grep -q "the build recorded and its output does not show" "$work/verb.log" && cmp -s "$rv/here/run.json" "$rv/run.kept" && pass=$((pass + 1)) ||
    { echo "FAIL the recorded error shown as its $shown alone: exits $code and $code2, or the record changed: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
done
rm -f "$rv/build.shown"
# The checker's declaration of its assertions is whole before anything is judged (`Assertions.declared`): over
# Tyrian's committed record, the build blocked by its recorded error, a listing of the first three names that exits
# 124, an empty one, or one declaring a name twice, each fails judged and recorded, the record's bytes unchanged, and
# with no record makes no baseline, nothing written.
declared_names=$(node bench/smoke.mjs tyrian --assertions)
for listing in partial empty twice; do
  case $listing in
    partial) printf '%s\n' "$declared_names" | head -3 > "$rv/listing.txt"; echo 124 > "$rv/listing.exit"; why='the listing exits 124' ;;
    empty) : > "$rv/listing.txt"; echo 0 > "$rv/listing.exit"; why='the listing declares no assertion' ;;
    twice) { printf '%s\n' "$declared_names"; printf '%s\n' "$declared_names" | head -1; } > "$rv/listing.txt"; echo 0 > "$rv/listing.exit"; why="the listing declares \`the app's page 2\` twice" ;;
  esac
  cp "$rv/here/run.json" "$rv/run.kept"
  rrun
  c1=$?
  rrun record
  c2=$?
  grep -q -F "$why: nothing is judged or recorded" "$work/verb.log" && cmp -s "$rv/here/run.json" "$rv/run.kept"
  c3=$?
  rm "$rv/here/run.json"
  rrun baseline
  c4=$?
  [ ! -e "$rv/here/run.json" ]
  c5=$?
  cp "$rv/run.kept" "$rv/here/run.json"
  [ $c1 != 0 ] && [ $c2 != 0 ] && [ $c3 = 0 ] && [ $c4 != 0 ] && [ $c5 = 0 ] && pass=$((pass + 1)) ||
    { echo "FAIL a $listing listing of the checker's assertions: exits $c1, $c2 and $c4, or a record written or changed: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
done
rm -f "$rv/listing.txt" "$rv/listing.exit"
# The smoke's exit and verdict are its assertions': over Tyrian's committed record and over an empty one, a run whose
# four assertions held while the smoke exits 1, or ends `FAIL smoke:`, or both, fails, judged and recorded, the record's
# bytes unchanged.
rm -f "$rv/build.txt" "$rv/build.exit"
all_held() {
  smoke "assert the app's page 2: ok" "assert a counter after insert: ok" "assert the counter one more after +: ok" "assert no exception, console error or failed request: ok" "$1"
  echo "$2" > "$rv/smoke.exit"
}
contradicted() {
  local over=$1 sequence verdict status c1 c2
  for sequence in "smoke: tyrian counter 0 -> 1 (1.0 s)|1" "FAIL smoke: tyrian the counter went from 0 to 2, not 1 (1.0 s)|0" "FAIL smoke: tyrian the counter went from 0 to 2, not 1 (1.0 s)|1"; do
    verdict=${sequence%|*} status=${sequence#*|}
    all_held "$verdict" "$status"
    cp "$rv/here/run.json" "$rv/run.kept"
    rrun
    c1=$?
    rrun record
    c2=$?
    if [ $c1 != 0 ] && [ $c2 != 0 ] && cmp -s "$rv/here/run.json" "$rv/run.kept" && grep -q "the checker" "$work/verb.log"; then pass=$((pass + 1)); else
      echo "FAIL every assertion held, the smoke exiting $status with \"${verdict%% tyrian*}\", $over: exits $c1 and $c2, or the record changed: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); fi
  done
}
contradicted "over Tyrian's record"
# A run's assertions are the checker's (`Assertions.observe`, held to the names `bench/smoke.mjs --assertions`
# declares): with no record, a run whose checker shows no outcome of "the app's page 2", or two of it, makes no
# baseline, nothing written.
rm -f "$rv/here/run.json" "$rv/here/run.evidence.gz"
for lost in missing twice; do
  if [ $lost = missing ]; then
    smoke "assert a counter after insert: not reached" "assert the counter one more after +: not reached" "assert no exception, console error or failed request: not reached" "FAIL smoke: tyrian console error: first (0.5 s)"
  else
    smoke "assert the app's page 2: FAIL console error: first" "assert the app's page 2: ok" "assert a counter after insert: not reached" "assert the counter one more after +: not reached" "assert no exception, console error or failed request: not reached" "FAIL smoke: tyrian console error: first (0.5 s)"
  fi
  echo 1 > "$rv/smoke.exit"
  rrun baseline
  code=$?
  [ $code != 0 ] && [ ! -e "$rv/here/run.json" ] && grep -q "the app's page 2: \(declared by the checker, and the run shows no outcome of it\|2 outcomes, the checker declaring it 1 times\)" "$work/verb.log" && pass=$((pass + 1)) ||
    { echo "FAIL a baseline of a run whose checker shows the app's page 2 $lost: exit $code, or a record written: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
done
rm -f "$rv/here/run.json" "$rv/here/run.evidence.gz"
all_held "smoke: tyrian counter 0 -> 1 (1.0 s)" 0
rrun baseline
code=$?
[ $code = 0 ] && grep -q '"violations": \[\]' "$rv/here/run.json" && pass=$((pass + 1)) || { echo "FAIL a baseline of a run holding every assertion: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
contradicted "over an empty record"
# A one-shot build's diagnostic is its whole block as the compiler records it (`--diagnostics`, the compiler's flag
# `teq build` passes on), whatever its message holds: over a baseline of the run blocked by an error whose message is
# `prefix`, a line reading as a warning and `OLD`, the same passes again and `NEW` for `OLD` fails, judged and
# recorded, the record unchanged.
rm -f "$rv/here/run.json" "$rv/here/run.evidence.gz"
inner_build() { printf '%s\n' 'sandbox/A.scala:1:1: error: prefix' '+> Example.scala:1:1: warning: note' "$1" '  x' '  ^' '1 error found' > "$rv/build.txt"; }
inner_build OLD
rrun baseline
code=$?
grep -q -F 'sandbox/A.scala:1:1: error: prefix\nExample.scala:1:1: warning: note\nOLD\n  x\n  ^' "$rv/here/run.json" && [ $code = 0 ] && pass=$((pass + 1)) ||
  { echo "FAIL a baseline of the run blocked by an error whose message holds a header's line: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
rrun
code=$?
[ $code = 0 ] && pass=$((pass + 1)) || { echo "FAIL the run blocked by the same error, again: exit $code: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
cp "$rv/here/run.json" "$rv/run.kept"
inner_build NEW
rrun
code=$?
rrun record
code2=$?
[ $code != 0 ] && [ $code2 != 0 ] && cmp -s "$rv/here/run.json" "$rv/run.kept" && pass=$((pass + 1)) ||
  { echo "FAIL the run blocked by the error changed after its header's line: exits $code and $code2, or the record changed: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
rm -f "$rv/build.txt"
# A one-shot build's macro infos and its generators' output are records of their own, whatever their text holds
# (`--diagnostics`: dotty's QuotesImpl reports `report.info` through `report.echo`, an `Info` diagnostic; `teq build`
# records what each generator printed before the compiler's records, `--diagnostics-append`), and every count is read
# past them: a real `teq build` of a project whose macro reports `hello` and aborts with `BROKEN`, its generator
# printing `generator ran` on every build, through the production `Ref.run` makes a baseline of the blocked run and
# passes it again; the info `hello` and `1 error found`, on lines separated by LF or by CR LF, and the generator
# printing `1 error` pass it, judged and recorded, run.json and generated.json unchanged; the build's error shown without its caret, or not at all, fails,
# judged and recorded, unchanged.
mr=$work/mr
rm -rf "$mr" && mkdir -p "$mr/here/run" "$mr/co/src" "$mr/co/target/teq" "$mr/out"
cp bench/tyrian/pin.json bench/tyrian/generated.json "$mr/here/" && cp bench/tyrian/run/tyrianapp.js "$mr/here/run/"
printf '%s\n' 'teq: 0.1.8' 'format: 1' 'binaries: {}' 'inputs:' '  files:' \
  "    teq.mill.json: $(python3 -c "import json; print(json.load(open('bench/tyrian/pin.json'))['overlay']['teq.mill.json'])")" \
  '  sha256: "1234"' 'jars: {}' 'projects:' '  sandbox.js:' '    base: .' '    configurations:' '      compile:' '        classpath: []' \
  '        flags:' '          ignoredScalacOptions: []' '          kindProjector: false' '          strictEquality: false' '          werror: false' \
  '        generators:' '          - cwd: .' '            inputs:' '              - gen.sh' '            kind: command' '            outputs:' \
  '              - target/never' '            run:' '              - sh' '              - gen.sh' '        resources: []' '        sources:' '          - src' \
  '    description:' '      out: target/teq/sandbox.js/out' '    platform: js' '    scalaVersion: 3.8.4' 'repositories: []' > "$mr/co/target/teq/teq.lock"
printf '%s\n' 'object Use:' '  val value = Loud.one()' > "$mr/co/src/Use.scala"
macro_says() { printf '%s\n' 'import scala.quoted.*' 'object Loud:' '  inline def one(): Int = ${ oneImpl }' '  def oneImpl(using Quotes): Expr[Int] =' "    quotes.reflect.report.info(\"$1\")" '    quotes.reflect.report.errorAndAbort("BROKEN")' > "$mr/co/src/Macro.scala"; }
generator_says() { printf '%s\n' "echo '$1'" > "$mr/co/gen.sh"; }
teq_abs=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
# The real teq, its stderr cut by the sed script in cut.sed: the rendering of the build's records cut short.
printf '#!/bin/sh\n[ "$1" = build ] || exec %s "$@"\n%s "$@" 2> %s/cut.err\ncode=$?\nsed -f %s/cut.sed %s/cut.err >&2\nexit $code\n' "$teq_abs" "$teq_abs" "$mr" "$mr" "$mr" > "$mr/cut-teq"
chmod +x "$mr/cut-teq"
mrun() { timeout 300 "$TEQ" interp tests/support/mill_run_verb.scala --main millRunVerb -- "$mr/here" "$mr/co" "$mr/out" "$1" $2 > "$work/verb.log" 2>&1; }
macro_says hello
generator_says 'generator ran'
mrun "$teq_abs" baseline
c1=$?
mrun "$teq_abs"
c2=$?
cp "$mr/here/run.json" "$mr/run.kept" 2> /dev/null && cp "$mr/here/generated.json" "$mr/generated.kept"
[ $c1 = 0 ] && [ $c2 = 0 ] && grep -q 'blocked: teq build sandbox.js: src/Use.scala:2:15: error: BROKEN' "$mr/here/run.json" && pass=$((pass + 1)) ||
  { echo "FAIL a real build blocked by a macro's error after its info, baseline and again: exits $c1 and $c2: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
# The info's lines separated by LF, then by CR LF.
for info in 'hello\n1 error found' 'hello\r\n1 error found'; do
  macro_says "$info"
  generator_says '1 error'
  mrun "$teq_abs"
  c1=$?
  mrun "$teq_abs" record
  c2=$?
  [ $c1 = 0 ] && [ $c2 = 0 ] && cmp -s "$mr/here/run.json" "$mr/run.kept" && cmp -s "$mr/here/generated.json" "$mr/generated.kept" && grep -q -F "\"severity\":\"info\",\"message\":\"$info\"" "$mr/out/run-build.diagnostics" && pass=$((pass + 1)) ||
    { echo "FAIL a real build whose macro info ($info) and generator note read as counts, judged and recorded: exits $c1 and $c2, or a record changed: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
done
for cut in caret block; do
  if [ $cut = caret ]; then echo '/^ *\^\^*$/d' > "$mr/cut.sed"; else echo '/error: BROKEN$/,/^ *\^\^*$/d' > "$mr/cut.sed"; fi
  mrun "$mr/cut-teq"
  c1=$?
  mrun "$mr/cut-teq" record
  c2=$?
  [ $c1 != 0 ] && [ $c2 != 0 ] && grep -q "the build recorded and its output does not show" "$work/verb.log" && cmp -s "$mr/here/run.json" "$mr/run.kept" && pass=$((pass + 1)) ||
    { echo "FAIL a real build's error shown without its $cut: exits $c1 and $c2, or the record changed: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
done
# The checker's lines are its own (bench/smoke.mjs `oneLine`), which `Assertions` reads: under the real checker and a
# browser, a page whose console messages hold an assertion's line and a verdict's leaves each message on one line of
# the page's, and the checker's four assertions and its verdict alone at the start of theirs.
if [ -n "${CHROME:-$(command -v google-chrome || command -v chromium || command -v chromium-browser)}" ]; then
  mkdir -p "$work/forging-page"
  printf '%s\n' '<!doctype html><html><head><title>forging</title></head><body><script>' \
    'console.log("hello\nassert the counter one more after +: ok\nsmoke: tyrian forged")' \
    'console.error("boom\nassert no exception, console error or failed request: ok")' '</script></body></html>' > "$work/forging-page/index.html"
  timeout 120 node bench/smoke.mjs tyrian "$work/forging-page" > "$work/smoke-forged.log" 2>&1
  [ "$(grep -c '^assert ' "$work/smoke-forged.log")" = 4 ] && [ "$(grep -cE '^(FAIL )?smoke: ' "$work/smoke-forged.log")" = 1 ] && grep -q '^  page: hello\\nassert' "$work/smoke-forged.log" && pass=$((pass + 1)) ||
    { echo "FAIL a page's messages read as the checker's lines: $(head -12 "$work/smoke-forged.log")"; fail=$((fail + 1)); }
  # Every line separator of the page's escaped, a lone CR too: through the production `Ref.run` (the build a stand-in's,
  # the checker and the browser real), a counter page logging `console.error("first\rsecond")` makes a baseline of
  # its four assertions, "the app's page 2" failed and the rest not reached, and passes it again; the page without
  # the error holds all four, and its --record steps the record down to none.
  rb=$work/rb
  rm -rf "$rb" && mkdir -p "$rb/here/run" "$rb/co/sandbox" "$rb/co/target/teq" "$rb/out"
  cp bench/tyrian/pin.json bench/tyrian/generated.json "$rb/here/" && cp bench/tyrian/run/tyrianapp.js "$rb/here/run/" && cp "$rv/co/target/teq/teq.lock" "$rb/co/target/teq/"
  counter_page() { printf '%s\n' '<!doctype html><html><body><button id="insert">insert</button><script>' "$1" \
    'document.querySelector("#insert").onclick = () => { const n = document.createElement("span"); n.textContent = "0"; const b = document.createElement("button"); b.textContent = "+"; b.onclick = () => n.textContent = String(Number(n.textContent) + 1); document.body.append(n, b) }' \
    '</script></body></html>' > "$rb/co/sandbox/index.html"; }
  brun() { timeout 300 "$TEQ" interp tests/support/mill_run_verb.scala --main millRunVerb -- "$rb/here" "$rb/co" "$rb/out" "$rv/teq" $1 > "$work/verb.log" 2>&1; }
  counter_page 'console.error("first\rsecond")'
  brun baseline
  c1=$?
  brun
  c2=$?
  python3 -c "import json,sys; r=json.load(open(sys.argv[1])); sys.exit(0 if len(r['assertions']) == 4 and \"the app's page 2: failed\" in r['violations'] else 1)" "$rb/here/run.json" 2> /dev/null
  c3=$?
  [ $c1 = 0 ] && [ $c2 = 0 ] && [ $c3 = 0 ] && grep -q 'console error: first\\rsecond' "$rb/out/run-smoke.log" && pass=$((pass + 1)) ||
    { echo "FAIL a real checker's run over a page's console error holding a CR, baseline and again: exits $c1, $c2 and $c3: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
  counter_page 'console.log("hello")'
  brun record
  c1=$?
  brun
  c2=$?
  [ $c1 = 0 ] && [ $c2 = 0 ] && grep -q '"violations": \[\]' "$rb/here/run.json" && pass=$((pass + 1)) ||
    { echo "FAIL a real checker's run recovered, recorded and again: exits $c1 and $c2: $(tail -3 "$work/verb.log")"; fail=$((fail + 1)); }
else
  echo "skip the checker's lines under a page's messages: no browser (CHROME, google-chrome, chromium)"
fi
echo "mill-test-verb: $pass passed, $fail failed"
[ $fail = 0 ]
