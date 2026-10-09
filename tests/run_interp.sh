#!/bin/bash
# Runs every tests/cases program through `teq interp` and compares the output with
# the same .expected files as tests/run.sh. The tests whose expectations come from Scala.js (a
# `//> using platform js` directive: doubles print as JS prints them) are compared with
# tests/jvm-expected/<name>.expected, since the interpreter formats numbers as the JVM does;
# without such a file the test is skipped. A `// interp-expected: js` line keeps the Scala.js
# expectation: the interpreter runs the std's JavaScript platform layer, so where that layer is
# a Scala.js library's (scala-java-locales' java.text) its output is Scala.js's.
#
# tests/interp-passing.txt lists the tests that pass; a listed test that fails is a regression and
# fails the script, an unlisted one that passes is reported so that it can be added
# (`--update` rewrites the list). Failures are classified:
#   unsupported the interpreter reported an IR shape or a builtin it lacks (exit code 3)
#   compile     any other compile failure
#   exception   the program threw
#   output      the program ran and printed something else
#   timeout     the program ran too long
# A `// jars: <names>` line puts those jars on the class path (tests/support/jars.sh); a test with
# one of them missing from the coursier cache counts as passed.
# The tests of the JVM's platform (`//> using platform jvm`: files, the process) run here as the
# others do, from the repository's root, where their files go under target/. One of them,
# sys_exit_status, runs a second time with an argument, where it calls `sys.exit(2)`: the status
# has to be 2, its line on stderr out and nothing printed after the call, as scalac's run has it.
# Several tests run at once (JOBS, by default the number of cores).
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
JOBS=${JOBS:-$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)}
LIST=tests/interp-passing.txt
update=0
[ "$1" = "--update" ] && update=1
out=out/interp-tests
rm -rf "$out"
mkdir -p "$out"

run_one() {
  src=$1
  name=$(basename "$src" .scala)
  out=out/interp-tests
  expected="tests/cases/$name.expected"
  if grep -q -h '^// interp-expected: js' "$src" "$src"/*.scala 2>/dev/null; then
    :
  elif grep -q -h '^//> using platform js' "$src" "$src"/*.scala 2>/dev/null; then
    expected="tests/jvm-expected/$name.expected"
    if [ ! -f "$expected" ]; then
      echo "skip $name" > "$out/$name.result"
      return
    fi
  fi
  flags=$(grep -h -o '^// teq: .*' "$src" "$src"/*.scala 2>/dev/null | head -1 | sed 's|^// teq: ||')
  . tests/support/jars.sh
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    echo "pass $name" > "$out/$name.result"
    echo "$name" > "$out/$name.nojar"
    return
  fi
  [ -n "$JARS_CP" ] && flags="$flags --classpath $JARS_CP"
  timeout 30 "$TEQ" interp "$src" $flags > "$out/$name.actual" 2> "$out/$name.err"
  code=$?
  if [ $code -eq 0 ] && diff -q "$expected" "$out/$name.actual" > /dev/null; then
    echo "pass $name" > "$out/$name.result"
  elif [ $code -eq 124 ]; then
    echo "timeout $name" > "$out/$name.result"
  elif [ $code -eq 3 ]; then
    echo "unsupported $name" > "$out/$name.result"
  elif [ $code -eq 1 ] && grep -q 'Exception in thread' "$out/$name.err"; then
    echo "exception $name" > "$out/$name.result"
  elif [ $code -eq 1 ] || [ $code -eq 2 ]; then
    if grep -q 'error' "$out/$name.err"; then echo "compile $name" > "$out/$name.result"; else echo "exception $name" > "$out/$name.result"; fi
  else
    echo "output $name" > "$out/$name.result"
  fi
}
export -f run_one
export TEQ

for src in tests/cases/*.scala tests/cases/*/; do echo "${src%/}"; done | xargs -P "$JOBS" -I{} bash -c 'run_one {}'

cat "$out"/*.result | sort > "$out/results.txt"
status=0
for kind in unsupported compile exception output timeout; do
  names=$(grep "^$kind " "$out/results.txt" | cut -d' ' -f2 | tr '\n' ' ')
  [ -n "$names" ] && echo "$kind: $names"
done
nojar=$(cat "$out"/*.nojar 2> /dev/null | tr '\n' ' ')
[ -n "$nojar" ] && echo "counted as passed, a jar of theirs not in the coursier cache: $nojar"
skipped=$(grep '^skip ' "$out/results.txt" | cut -d' ' -f2 | tr '\n' ' ')
[ -n "$skipped" ] && echo "skipped (expectations from Scala.js): $skipped"
grep '^pass ' "$out/results.txt" | cut -d' ' -f2 | sort > "$out/passing.txt"
if [ $update = 1 ]; then
  cp "$out/passing.txt" "$LIST"
fi
touch "$LIST"
regressions=$(comm -23 <(sort "$LIST") "$out/passing.txt" | tr '\n' ' ')
new=$(comm -13 <(sort "$LIST") "$out/passing.txt" | tr '\n' ' ')
[ -n "$new" ] && echo "passing but not in $LIST: $new"
if [ -n "$regressions" ]; then
  echo "REGRESSIONS: $regressions"
  status=1
fi
exit_out=$(timeout 30 "$TEQ" interp tests/cases/sys_exit_status.scala -- exit 2> "$out/sys_exit_status.exit.err")
exit_code=$?
if [ $exit_code != 2 ] || [ "$exit_out" != "before the exit" ] || [ "$(cat "$out/sys_exit_status.exit.err")" != "usage: sys_exit_status" ]; then
  echo "EXIT STATUS: sys_exit_status with an argument exited $exit_code, not 2, printing \"$exit_out\" and \"$(cat "$out/sys_exit_status.exit.err")\""
  status=1
fi
count() { grep -c "^$1 " "$out/results.txt"; }
echo "$(count pass) passed, $(count unsupported) unsupported, $(count compile) compile, $(count exception) exception, $(count output) output, $(count timeout) timeout, $(count skip) skipped"
exit $status
