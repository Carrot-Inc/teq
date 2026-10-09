#!/bin/bash
# The gate's application lists over the example (bench/app/app-lists.sh through sbt-teq's teqInputs), api standing for
# the application's API side: the corpus generated into src/ and api's test fixtures added as check.sh adds them (an
# inline def making an anonymous class among the main sources, the jar api/lib/util.jar packaged with scala-cli), the
# lists written into a directory outside the repository and checked: the main module one row, src/shared, src/api
# and sbt-buildinfo's generated directory, its sources the corpus's and BuildInfo; the main class path sbt's whole
# (api depends on no project); the test module api-test-src with its sources; the test class path beginning with
# `@api/compile`, the products sbt takes through Runtime, and holding the unmanaged jar; the flags the scalacOptions
# `-Wconf:any:s -Xmax-inlines 80` map onto, the suppression among the ignored options. Then a hidden source sbt leaves
# out under a source directory (`src/api/.Hidden.scala`, which teq's expansion of the directory reads) makes the lists
# refused. Then bench/app/api-check.sh --test over the lists: the tests checked over teq's own build of the main
# lists, exit 0. With SCALAC=1, bench/app/api-scalac-diff.sh over them as well (scalac's two compiles, minutes), which
# completes both scopes. Needs sbt, scala-cli and sbt-teq published locally with teqInputs (TEQ_PLUGIN_VERSION as
# check.sh takes it); TEQ names the binary (default: the repository's release build).
cd "$(dirname "$0")" || exit 1
repo=$(cd ../../.. && pwd)
export TEQ=${TEQ:-$repo/target/release/teq}
unset TEQ_COMPILER
status=0
pass() { echo "lists: $1"; }
fail() { echo "FAIL lists: $1"; status=1; }
lists=$(mktemp -d)
trap 'rm -rf "$lists"; rm -f src/api/.Hidden.scala' EXIT

rm -rf src && timeout 120 python3 "$repo/bench/app/gen.py" src > /dev/null || { echo "FAIL generation"; exit 1; }
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
scala=$(find src/shared src/api -name '*.scala' | wc -l)

timeout 900 "$repo/bench/app/app-lists.sh" . api "$lists" > target-lists.log 2>&1 || { tail -5 target-lists.log; echo "FAIL lists: app-lists.sh (see target-lists.log)"; exit 1; }
rows() { grep -v '^#' "$lists/$1"; }
[ "$(rows api-modules.txt)" = "src/shared src/api target/out/jvm/scala-3.8.4/api/src_managed/main" ] && pass "the main module's directories, the generated one beside them" || fail "the main module is $(rows api-modules.txt)"
grep -q "^# api/compile and its 0 upstream modules, upstream first: $((scala + 1)) sources, 1 generated" "$lists/api-modules.txt" && pass "the main list's configuration and its $((scala + 1)) sources" || fail "the main list's header: $(head -1 "$lists/api-modules.txt")"
! grep -q '^@' "$lists/api-classpath.txt" && grep -q 'cats-core_3-2.13.0.jar$' "$lists/api-classpath.txt" && pass "the main class path, no product on it" || fail "the main class path: $(head -3 "$lists/api-classpath.txt")"
[ "$(rows api-test-modules.txt)" = api-test-src ] && grep -q "^# api/test: $(find api-test-src -name '*.scala' | wc -l) sources, 0 generated" "$lists/api-test-modules.txt" && pass "the test module" || fail "the test module: $(cat "$lists/api-test-modules.txt")"
[ "$(head -1 "$lists/api-test-classpath.txt")" = "@api/compile" ] && [ "$(grep -c '^@' "$lists/api-test-classpath.txt")" = 1 ] && pass "the test class path takes api's products first" || fail "the test class path begins $(head -2 "$lists/api-test-classpath.txt" | tr '\n' ' ')"
grep -q "/api/lib/util.jar\$" "$lists/api-test-classpath.txt" && pass "the unmanaged jar on the test class path" || fail "no api/lib/util.jar on the test class path"
[ "$(cat "$lists/api-flags.txt")" = "--max-inlines 80" ] && [ "$(cat "$lists/api-test-scalac-options.txt" | tr '\n' ' ')" = "-Wconf:any:s -Xmax-inlines 80 " ] && grep -q 'ignored: -Wconf:any:s' "$lists/summary.txt" && pass "the flags and the options" || fail "the flags $(cat "$lists/api-flags.txt"), the options $(cat "$lists/api-test-scalac-options.txt" | tr '\n' ' ')"

echo 'object Hidden' > src/api/.Hidden.scala
if timeout 900 "$repo/bench/app/app-lists.sh" . api "$lists/hidden" > target-lists-hidden.log 2>&1; then
  fail "a hidden source teq reads and sbt does not was accepted"
elif grep -q 'refused: api/compile: its directories expanded give .* not sbt.s: src/api/.Hidden.scala' target-lists-hidden.log; then
  pass "a hidden source teq reads and sbt does not: refused"
else
  fail "the hidden source: $(tail -1 target-lists-hidden.log)"
fi
rm -f src/api/.Hidden.scala

. "$lists/lists.env"
if timeout 600 "$repo/bench/app/api-check.sh" --test "$TEQ" > target-lists-check.log 2>&1; then
  pass "the tests checked over teq's products of the main lists"
else
  fail "api-check.sh --test: $(tail -1 target-lists-check.log) (see target-lists-check.log)"
fi
if [ -n "$SCALAC" ]; then
  if timeout 1800 "$repo/bench/app/api-scalac-diff.sh" "$TEQ" "$lists/scalac" > target-lists-scalac.log 2>&1 || grep -q '^scalac oracle: .* differences' target-lists-scalac.log; then
    grep -q '^main: ' target-lists-scalac.log && grep -q '^test: ' target-lists-scalac.log && ! grep -q -E '^(main|test): (teq|scalac).s ' target-lists-scalac.log \
      && pass "scalac's oracle completes both scopes: $(tail -1 target-lists-scalac.log)" || fail "scalac's oracle: $(grep -E '^(main|test): ' target-lists-scalac.log | head -2 | tr '\n' ' ')"
  else
    fail "scalac's oracle: $(tail -1 target-lists-scalac.log) (see target-lists-scalac.log)"
  fi
fi
exit $status
