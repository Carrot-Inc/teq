#!/bin/bash
# The planted slowdown (TEQ_BENCH_SLOW, src/planted.rs) on tests/planted/clock.scala, built with the switch unset and
# set, to JavaScript and to the JVM: the four outputs are tests/planted/clock.expected, the clock's readings in code
# (bare and qualified by java.lang.) are planted in the switched JavaScript and only there, and its name in the
# program's strings stays text.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
# The JVM build links against scala-library's jar, pinned on its class path and beside it under java.
sl=$(jar_of scala-library)
mkdir -p out/planted
src=tests/planted/clock.scala
pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() {
  echo "FAIL $1"
  fail=$((fail + 1))
}
for slow in "" 50; do
  for target in js jvm; do
    name=$target${slow:+-slow}
    if [ $target = js ]; then
      TEQ_BENCH_SLOW=$slow timeout 60 "$TEQ" compiler build $src -o out/planted/$name.js > out/planted/$name.build.log 2>&1 &&
        timeout 60 node out/planted/$name.js > out/planted/$name.out 2>&1
    else
      TEQ_BENCH_SLOW=$slow timeout 60 "$TEQ" compiler build $src --target jvm --classpath "$sl" -o out/planted/$name.jar > out/planted/$name.build.log 2>&1 &&
        timeout 60 java -Xss512m -cp "out/planted/$name.jar:$sl" TeqMain > out/planted/$name.out 2>&1
    fi
    if [ $? != 0 ]; then
      bad "$name: $(tail -1 out/planted/$name.out out/planted/$name.build.log 2> /dev/null | grep -v '^==>' | grep . | tail -1)"
    elif cmp -s out/planted/$name.out tests/planted/clock.expected; then
      ok
    else
      bad "$name: the output differs from tests/planted/clock.expected"
    fi
  done
done
if [ -f out/planted/js-slow.js ] && [ -f out/planted/js.js ]; then
  ! grep -q 'TeqBenchSlowClock' out/planted/js.js && ok || bad "the unswitched build holds the planted clock"
  grep -q 'TeqBenchSlowClock' out/planted/js-slow.js && ok || bad "the switched build has no planted clock"
  grep -qF 'System.nanoTime() is the clock' out/planted/js-slow.js && ok || bad "the switched build lost the string's text"
fi
echo "planted: $pass passed, $fail failed"
[ $fail = 0 ]
