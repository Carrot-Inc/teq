#!/bin/bash
# Runs every tests/cases/*.scala through teq and compares the output with the .expected file.
# Missing .expected files are produced with real Scala (scala-cli, the version below) on the
# JDK the suites run on (`--jvm system`: without JAVA_HOME scala-cli takes a JDK 17 of its own,
# which prints some Float and Double values otherwise than JDK 19 and later); pass --regen to
# redo all.
# A `// teq: <flags>` line in a test passes those flags to teq, a `// jars: <names>` line puts
# those jars on its class path (tests/support/jars.sh); without one of them in the coursier cache
# the test is skipped and counts as passed. A test whose output depends on
# the platform (doubles print as in Scala.js) carries `//> using platform js`, which scala-cli
# honours; linking takes longer than a JVM run, the first time by a download. A test of the JVM's
# platform, the files and the process JavaScript has none of, carries `//> using platform jvm`:
# its expectation is written as any other's, run from the repository's root, and it does not run
# here (tests/run_interp.sh and tests/run_jvm.sh run it).
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
export TZ=UTC  # an archive entry's DOS time is local time: the recorded bytes are UTC's
# Where the outputs and the actual results go, and a case's bound: a binary run under an emulator writes
# beside the native run's and takes longer (bench/cross-ship.sh linux-arm).
out=${TEQ_TEST_OUT:-out/tests}
limit_case=${TEQ_TEST_TIMEOUT:-20}
. tests/support/jars.sh
scala_version=${SCALA_VERSION:-3.8.4}
regen=0
[ "$1" = "--regen" ] && regen=1
mkdir -p "$out"
pass=0
fail=0
for src in tests/cases/*.scala tests/cases/*/; do
  src=${src%/}
  name=$(basename "$src" .scala)
  expected="tests/cases/$name.expected"
  if [ $regen = 1 ] || [ ! -f "$expected" ]; then
    limit=120
    grep -q -h '^//> using platform js' "$src" "$src"/*.scala 2>/dev/null && limit=300
    if ! timeout $limit scala-cli run -S "$scala_version" --jvm system "$src" --server=false -q > "$expected" 2> "$out/$name.ref.err"; then
      echo "REF FAIL $name (see $out/$name.ref.err)"
      rm -f "$expected"
      fail=$((fail + 1))
      continue
    fi
  fi
  if grep -q -h '^//> using platform jvm' "$src" "$src"/*.scala 2>/dev/null; then
    echo "skip $name: the JVM's platform"
    pass=$((pass + 1))
    continue
  fi
  flags=$(grep -h -o '^// teq: .*' "$src" "$src"/*.scala 2>/dev/null | head -1 | sed 's|^// teq: ||')
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    echo "skip $name: not in the coursier cache:$JARS_MISSING"
    pass=$((pass + 1))
    continue
  fi
  [ -n "$JARS_CP" ] && flags="$flags --classpath $JARS_CP"
  if run_js "$limit_case" "$out/$name.js" "$src" $flags > "$out/$name.actual" 2>&1 \
    && diff -q "$expected" "$out/$name.actual" > /dev/null; then
    pass=$((pass + 1))
  else
    echo "FAIL $name"
    diff "$expected" "$out/$name.actual" | head -${DIFF_LINES:-10}
    fail=$((fail + 1))
  fi
done
echo "$pass passed, $fail failed"
[ $fail = 0 ]
