#!/bin/bash
# The example's suites through sbt and through `teq test`, in two parts kept apart.
#
# sbt, in two server sessions (`sbt --client`), scalac's (TEQ_COMPILER=0) and teq's
# (TEQ_COMPILER=1), each asserting `show api/teqCompiler`: `definedTests` of api after `api/clean`
# under each, and under teq after a batch of api's test configuration teq compiled (a source of
# it carries this run's number, so that sbt's cache restores no earlier compile), are the same
# seven definitions, their fingerprints included. Then sbt 2's success cache (`test` is
# `testQuick` under either compiler, its successes outlive `clean`):
# with this run's number among api's `Test / extraTestDigests`, so that no success recorded
# before matches, `test` runs the six suites build.sbt leaves in (JunitStyleSuite is found and
# excluded), each once, eight tests, and a second `test` runs none; after `api/clean` the
# definitions and their digests are as they were, so `test` runs none while `testOnly` runs the
# six; a failing suite fails `test`, and the next `test` runs it again, alone. A suite run is
# told by its report, `test-reports/TEST-<suite>.xml`, cleared before each run; the store of
# sbt's cache (`Global / cacheStores`) is printed, and api's test options are its one exclusion.
#
# teq's own driver, no sbt: the suites `teq test <project> --list` finds in the analysis are those
# sbt's definedTests finds under scalac, for each JVM project with test frameworks, api and
# jvmapp; then api's suites run on the warm runner, all of them passing, and an edit to a suite
# between two runs changes the outcome, its revert changing it back.
#
# The corpus is generated into src/, with check.sh's Actions.scala, and api's test jar packaged
# with scala-cli as check.sh does. Needs sbt, scala-cli and sbt-teq published locally
# (TEQ_PLUGIN_VERSION as check.sh takes it); TEQ names the binary (default: the repository's
# release build).
cd "$(dirname "$0")" || exit 1
repo=../../..
export TEQ=${TEQ:-$(cd $repo && pwd)/target/release/teq}
unset TEQ_COMPILER
status=0
pass() { echo "tests: $1"; }
fail() { echo "FAIL tests: $1"; status=1; }
task() { timeout 300 "$TEQ" "$@" 2>&1; }
discovery=api-test-src/meridian/apitest/Discovery.scala
failing=api-test-src/meridian/apitest/Failing.scala
cleanup() {
  timeout 60 sbt --client shutdown > /dev/null 2>&1
  timeout 30 "$TEQ" stop > /dev/null 2>&1
  [ -f target-tests-discovery.bak ] && mv target-tests-discovery.bak $discovery
  rm -f $failing
}
trap cleanup EXIT

rm -rf src && timeout 120 python3 $repo/bench/app/gen.py src > /dev/null || { echo "FAIL generation"; exit 1; }
mkdir -p src/api/meridian/check api/lib
cat > src/api/meridian/check/Actions.scala <<'EOF'
package meridian.check

trait Action:
  def run(): Int

object Actions:
  inline def make(): Action = new Action:
    def run(): Int = 42
EOF
timeout 200 scala-cli --power package lib-src --library -o api/lib/util.jar -f -S 3.8.4 --server=false > target-util-jar.log 2>&1 || { echo "FAIL packaging util.jar (see target-util-jar.log)"; exit 1; }

# --- sbt ------------------------------------------------------------------------------------------
sbtlog=target-tests-sbt.log
: > $sbtlog
client() {
  local code
  timeout 300 sbt --client "$@" > target-tests-client.log 2>&1
  code=$?
  # sbt 2's client ends its output with a cursor movement of its own: the codes go.
  CLIENT_OUT=$(sed 's/\x1b\[[0-9;]*[A-Za-z]//g' target-tests-client.log)
  printf '== sbt --client %s (exit %s)\n%s\n' "$*" "$code" "$CLIENT_OUT" >> $sbtlog
  return $code
}
has_output() { case "$CLIENT_OUT" in *"$1"*) return 0 ;; *) return 1 ;; esac; }
# `show` prints a sequence's elements as `[info] * <element>`.
defined() { printf '%s\n' "$CLIENT_OUT" | sed -n 's/^\[info\] \* \(Test .*\)$/\1/p' | LC_ALL=C sort -u; }
batches() { printf '%s\n' "$CLIENT_OUT" | grep -o 'teq: [0-9]* sources* of [a-z-]*' | tr '\n' ';'; }
reports=target/out/jvm/scala-3.8.4/api/test-reports
# The suites the last run reported, `<suite> <tests> <failures>` each, from their reports.
ran() {
  for f in "$reports"/TEST-*.xml; do
    [ -f "$f" ] || continue
    sed -n 's/.*<testsuite [^>]*name="\([^"]*\)" tests="\([0-9]*\)" errors="\([0-9]*\)" failures="\([0-9]*\)".*/\1 \2 \4/p' "$f" | head -1
  done | LC_ALL=C sort
}
run_tests() { rm -rf "$reports"; client "$@"; }
suites="meridian.apitest.IndirectSuite 1 0
meridian.apitest.InlineActionSuite 1 0
meridian.apitest.JarSuite 1 0
meridian.apitest.MunitSuite 2 0
meridian.apitest.ScalatestSuite 1 0
meridian.apitest.ZioSuite 2 0"
session() {
  timeout 60 sbt --client shutdown > /dev/null 2>&1
  export TEQ_COMPILER=$1
  if client "show api/teqCompiler" && printf '%s\n' "$CLIENT_OUT" | grep -qx "\[info\] $2"; then
    pass "sbt with TEQ_COMPILER=$1: api/teqCompiler is $2"
  else
    fail "sbt with TEQ_COMPILER=$1: api/teqCompiler is not $2 (see $sbtlog)"
  fi
}

session 0 false
client api/clean || fail "scalac: api/clean (see $sbtlog)"
client "show api/Test/definedTests" || fail "scalac: definedTests of api (see $sbtlog)"
scalac_api=$(defined)
client "show jvmapp/Test/definedTests" || fail "scalac: definedTests of jvmapp (see $sbtlog)"
scalac_jvmapp=$(defined)
n=$(printf '%s\n' "$scalac_api" | grep -c '^Test ')
[ "$n" = 7 ] && pass "scalac: definedTests after clean finds api's seven suites" || fail "scalac: definedTests after clean finds $n of api's suites, not seven (see $sbtlog)"

session 1 true
cp $discovery target-tests-discovery.bak
nonce="$(date +%s%N) $$"
printf '\n// check-tests.sh, run %s\n' "$nonce" >> $discovery
client api/clean || fail "teq: api/clean (see $sbtlog)"
client "show api/Test/definedTests" || fail "teq: definedTests (see $sbtlog)"
teq_api=$(defined)
if [[ ";$(batches)" == *";teq: 7 sources of test-classes;"* ]] && [ -n "$scalac_api" ] && [ "$teq_api" = "$scalac_api" ]; then
  pass "teq: definedTests after clean and a batch of the test configuration teq compiled is scalac's, the seven with their fingerprints"
else
  fail "teq: definedTests after clean and a batch of teq's ($(batches)) differs from scalac's:"
  diff <(printf '%s\n' "$scalac_api") <(printf '%s\n' "$teq_api") | sed 's/^/  /'
fi
client "show Global / cacheStores" && pass "sbt's success cache: $(printf '%s\n' "$CLIENT_OUT" | grep -o 'DiskActionCacheStore([^,]*' | head -1))"
if client "show api/Test/testOptions" && printf '%s\n' "$CLIENT_OUT" | grep -q '^\[info\] \* Exclude(List(meridian.apitest.JunitStyleSuite))$' && [ "$(printf '%s\n' "$CLIENT_OUT" | grep -c '^\[info\] \* ')" = 1 ]; then
  pass "api's test options exclude JunitStyleSuite alone: seven suites defined, six to run"
else
  fail "api's test options (see $sbtlog)"
fi
client "set api / Test / extraTestDigests += sbt.util.Digest.sha256Hash(\"check-tests.sh $nonce\".getBytes(\"UTF-8\"))" || fail "the salt of the digests (see $sbtlog)"
if run_tests api/test && [ "$(ran)" = "$suites" ] && has_output "Passed: Total 8, Failed 0"; then
  pass "test with no success recorded runs the six suites build.sbt leaves in, each once, eight tests"
else
  fail "test with no success recorded ran: $(ran | tr '\n' ';') (see $sbtlog)"
fi
if run_tests api/test && [ -z "$(ran)" ] && has_output "No tests to run for api / Test / testQuick"; then
  pass "a second test runs none: each suite's digest has its success"
else
  fail "the second test ran: $(ran | tr '\n' ';') (see $sbtlog)"
fi
client "show api/Test/definedTestDigests"
digests=$(printf '%s\n' "$CLIENT_OUT" | grep '^\[info\] ' | grep -v '^\[info\] \(compiling\|teq:\|done\)')
client api/clean || fail "teq: the second api/clean (see $sbtlog)"
client "show api/Test/definedTests"
after=$(defined)
client "show api/Test/definedTestDigests"
if [ "$after" = "$scalac_api" ] && [ -n "$digests" ] && [ "$(printf '%s\n' "$CLIENT_OUT" | grep '^\[info\] ' | grep -v '^\[info\] \(compiling\|teq:\|done\)')" = "$digests" ]; then
  pass "after clean the seven definitions and their digests are as before clean"
else
  fail "after clean the definitions or their digests moved (see $sbtlog)"
fi
if run_tests api/test && [ -z "$(ran)" ] && has_output "No tests to run for api / Test / testQuick"; then
  pass "test after clean runs none: the successes outlive clean, as sbt keeps them"
else
  fail "test after clean ran: $(ran | tr '\n' ';') (see $sbtlog)"
fi
if run_tests api/testOnly && [ "$(ran)" = "$suites" ] && has_output "Passed: Total 8, Failed 0"; then
  pass "testOnly after clean runs the six suites, each once"
else
  fail "testOnly after clean ran: $(ran | tr '\n' ';') (see $sbtlog)"
fi
cat > $failing <<'EOF'
package meridian.apitest

import org.scalatest.funsuite.AnyFunSuite

class Failing extends AnyFunSuite:
  test("fails on purpose") {
    assert(1 + 1 == 3, "arithmetic")
  }
EOF
for attempt in "a failing suite fails test, the others skipped" "the next test runs the failing suite again, alone"; do
  if ! run_tests api/test && [ "$(ran)" = "meridian.apitest.Failing 1 1" ] && has_output "meridian.apitest.Failing"; then
    pass "$attempt"
  else
    fail "$attempt: ran $(ran | tr '\n' ';') (see $sbtlog)"
  fi
done
rm $failing
if run_tests api/test && [ -z "$(ran)" ]; then pass "the failing suite removed, test runs none"; else fail "test after the failing suite's removal ran: $(ran | tr '\n' ';') (see $sbtlog)"; fi
mv target-tests-discovery.bak $discovery
timeout 60 sbt --client shutdown > /dev/null 2>&1
unset TEQ_COMPILER

# --- teq's own driver -----------------------------------------------------------------------------
for p in api jvmapp; do
  if [ $p = api ]; then sbt_set=$scalac_api; else sbt_set=$scalac_jvmapp; fi
  teq_set=$(task test "$p" --list | grep '^Test ' | LC_ALL=C sort -u)
  if [ -n "$sbt_set" ] && [ "$sbt_set" = "$teq_set" ]; then
    pass "$p: the suites of the analysis are sbt's definedTests ($(printf '%s\n' "$sbt_set" | wc -l | tr -d ' '))"
  else
    fail "$p: sbt's definedTests and teq's differ:"
    diff <(printf '%s\n' "$sbt_set") <(printf '%s\n' "$teq_set") | sed 's/^/  /'
  fi
done

out=$(task test api)
if [ $? = 0 ] && [[ "$out" == *"api/test: 6 suites, 8 passed, 0 failed in "* ]]; then pass "api's suites run and pass, the excluded one left out"; else fail "api's suites: $out"; fi
suite=api-test-src/meridian/apitest/MunitSuite.scala
cp $suite target-MunitSuite.scala
sed -i.bak 's/"Get")/"Post")/' $suite && rm -f $suite.bak
out=$(task test api '*MunitSuite')
code=$?
cp target-MunitSuite.scala $suite && rm target-MunitSuite.scala
again=$(task test api '*MunitSuite')
if [ $code = 1 ] && [[ "$out" == *"[fail] meridian.apitest.MunitSuite"* ]] && [[ "$again" == *"api/test: 1 suite, 2 passed, 0 failed"* ]]; then
  pass "an edit to a suite between two warm runs changes the outcome, its revert changes it back"
else
  fail "the edit between runs ($code): $out // $again"
fi
[ $status = 0 ] && echo "tests: all passed"
exit $status
