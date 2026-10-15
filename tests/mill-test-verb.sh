#!/bin/bash
# bench/mill-reference.scala's test verb holds `teq test`'s run to the oracle's inventory over the export's modules: a
# stand-in teq lists each module's suites and prints a transcript, and the verb passes only where the export's modules,
# the suites listed and run and the cases are tests.json's, identity for identity, every case passed
# (tests/support/mill_test_verb.scala). Scenarios: the run as recorded (an ignored suite reports its one case under
# no heading), and with its headings coloured as the JVM's runner prints them; no inventory; a module removed from
# tests.json (its cases then not compared); a module of tests.json the export lacks; an empty suite of tests.json not
# listed; a suite listed that tests.json lacks; a listed suite that did not run; a renamed case; a failed case; an
# export made with other settings than the pin's; a module whose test sources gain a file, whose listing then fails.
# A module is asked for its suites only where its test configuration's sources hold one (app.js's directory is empty,
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
  "core.js": { "warnings": 2, "suites": [ "p.EmptySuite", "p.IgnoredSuite", "p.OneSuite", "p.TwoSuite" ], "cases": { "p.IgnoredSuite.<unknown>": "skipped", "p.OneSuite.adds": "passed", "p.OneSuite.skips": "skipped", "p.TwoSuite.parses: twice": "passed" } },
  "core.jvm": { "warnings": 0, "suites": [], "cases": {} }
 }
}'
# The listing and the transcript the stand-in prints for core.js (app.js lists no suite), then the stand-in.
standin() {
  printf '%s\n' "$1" > "$work/listing.txt"
  printf '%s\n' "$2" > "$work/transcript.txt"
  cat > "$work/teq" <<SH
#!/bin/sh
case "\$1 \$2 \$3" in
  "test --list core.js") cat $work/listing.txt ;;
  "test --verbose core.js") cat $work/transcript.txt ;;
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
expect "a listed suite that did not run" fail "suite p.EmptySuite was listed and did not run"
standin "$listing" "$(printf '%s\n' "$transcript" | sed 's/OneSuite.adds/OneSuite.added/')"
expect "a renamed case" fail 'case "p.OneSuite.adds" did not run'
standin "$listing" "$(printf '%s\n' "$transcript" | sed 's/^\[success\] p.TwoSuite.parses: twice$/[fail] p.TwoSuite.parses: twice: java.lang.AssertionError: boom/')"
expect "a failed case" fail 'case "p.TwoSuite.parses: twice" failed, tests.json has passed'
standin "$listing" "$transcript"
lock bb22
expect "an export made with other settings than the pin's" fail "records teq.mill.json as bb22"
lock
: > "$work/checkout/app/test/Added.scala"
expect "a module whose test sources gain a file, whose listing then fails" fail "app.js --list: teq test exits 1"
rm "$work/checkout/app/test/Added.scala"
expect "the run as recorded, again" pass
echo "mill-test-verb: $pass passed, $fail failed"
[ $fail = 0 ]
