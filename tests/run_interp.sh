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
# has to be 2, its line on stderr out and nothing printed after the call, as scalac's run has it;
# shutdown_hooks runs again ending by `System.exit`, a SIGTERM and an uncaught exception, and
# stdin_lines with lines on its stdin.
# The programs under tests/using include sources through their headers' directives, which no case
# can (every other suite builds a case alone): each runs once, its output against its .expected.
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

# A `// os: windows` or `// os: unix` line keeps a case to that system (Git's bash is Windows): on the other
# it is skipped, and its line in the list stands as it is.
case $(uname -s) in MINGW* | MSYS* | CYGWIN*) HERE_OS=windows ;; *) HERE_OS=unix ;; esac
other_os() {
  local os
  os=$(grep -h -o '^// os: [a-z]*' "$1" "$1"/*.scala 2> /dev/null | head -1)
  [ -n "$os" ] && [ "${os#// os: }" != "$HERE_OS" ]
}
export HERE_OS
export -f other_os

run_one() {
  src=$1
  name=$(basename "$src" .scala)
  out=out/interp-tests
  expected="tests/cases/$name.expected"
  if other_os "$src"; then
    echo "skip $name" > "$out/$name.result"
    echo "$name" > "$out/$name.otheros"
    return
  fi
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
  # On Windows a line ends with CRLF, as the JDK's println ends it there.
  if [ $code -eq 0 ] && diff -q $([ $HERE_OS = windows ] && echo --strip-trailing-cr) "$expected" "$out/$name.actual" > /dev/null; then
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
cat "$out"/*.otheros 2> /dev/null | sort > "$out/otheros.txt"
skipped=$(grep '^skip ' "$out/results.txt" | cut -d' ' -f2 | sort | comm -23 - "$out/otheros.txt" | tr '\n' ' ')
[ -n "$skipped" ] && echo "skipped (expectations from Scala.js): $skipped"
otheros=$(tr '\n' ' ' < "$out/otheros.txt")
[ -n "$otheros" ] && echo "skipped (another system's): $otheros"
grep '^pass ' "$out/results.txt" | cut -d' ' -f2 | sort > "$out/passing.txt"
touch "$LIST"
if [ $update = 1 ]; then
  sort -u "$out/passing.txt" <(sort "$LIST" | comm -12 - "$out/otheros.txt") > "$LIST.new" && mv "$LIST.new" "$LIST"
fi
regressions=$(comm -23 <(sort "$LIST") <(sort -u "$out/passing.txt" "$out/otheros.txt") | tr '\n' ' ')
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
# The cases whose expectation holds one run alone run again, with arguments (and stdin): the status
# and stdout of each, scalac's under Scala CLI, written here.
again() {
  local name=$1 want=$2 expected=$3 input=$4
  shift 4
  local actual code
  actual=$(printf '%b' "$input" | timeout 30 "$TEQ" interp "tests/cases/$name.scala" -- "$@" 2> "$out/$name.again.err")
  code=$?
  if [ $code != "$want" ] || [ "$actual" != "$(printf '%b' "$expected")" ]; then
    echo "AGAIN: $name $* exited $code, not $want, printing \"$actual\""
    status=1
  fi
}
hooks='twice: Hook previously registered\nremoved: true\nremoved again: false\nmain runs'
if [ $HERE_OS = unix ]; then
  again shutdown_hooks 3 "$hooks\nthe hook runs" "" exit
  again shutdown_hooks 143 "$hooks\nthe hook runs" "" term
  again shutdown_hooks 1 "$hooks\nfinally\nthe hook runs" "" throw
fi
again stdin_lines 0 'first: one\nsecond: twö\nrest: no end (6)\nafter the end: null -1' 'one\ntwö\nno end' read
# A stream holds no descriptor once closed or its child ended (the reaper closes the child's pipes):
# stream_descriptors again under a limit of 256 descriptors, which 400 kept ones would exceed.
if [ $HERE_OS = unix ]; then
  limited=$( (ulimit -n 256 && timeout 120 "$TEQ" interp tests/cases/stream_descriptors.scala) 2> "$out/stream_descriptors.limited.err")
  if [ "$limited" != "$(cat tests/cases/stream_descriptors.expected)" ]; then
    echo "DESCRIPTORS: stream_descriptors under ulimit -n 256 printed \"$limited\""
    status=1
  fi
fi
# A signal stops a program blocked in a native and runs its hooks, and the library's children die
# with the script: socket_write_signal (a write the peer does not take) and script_owned_signal (the
# child of a blocking `Sh.run`) run with a directory, SIGTERM sent to the process its pid file names a
# second after its ready file is there; each ends with 143 and the JDK's lines, no child left.
signalled() {
  local name=$1 ready=$2 want=$3 dir pid code i
  dir=$(mktemp -d)
  "$TEQ" interp "tests/cases/$name.scala" -- "$dir" > "$out/$name.signal.out" 2> "$out/$name.signal.err" &
  pid=$!
  i=0
  while [ ! -s "$dir/$ready" ] && [ $i -lt 300 ]; do sleep 0.1; i=$((i + 1)); done
  sleep 1
  [ -s "$dir/pid" ] && kill -TERM "$(cat "$dir/pid")" 2> /dev/null
  i=0
  while kill -0 $pid 2> /dev/null && [ $i -lt 100 ]; do sleep 0.1; i=$((i + 1)); done
  if kill -0 $pid 2> /dev/null; then
    kill -KILL $pid
    echo "SIGNAL: $name still ran 10 s after SIGTERM"
    status=1
  fi
  wait $pid
  code=$?
  if [ $code != 143 ] || [ "$(cat "$out/$name.signal.out")" != "$(printf '%b' "$want")" ]; then
    echo "SIGNAL: $name exited $code, not 143, printing \"$(cat "$out/$name.signal.out")\""
    status=1
  fi
  if [ -s "$dir/child" ] && kill -0 "$(cat "$dir/child")" 2> /dev/null; then
    kill "$(cat "$dir/child")"
    echo "SIGNAL: $name left its child running"
    status=1
  fi
  rm -rf "$dir"
}
if [ $HERE_OS = unix ]; then
  signalled socket_write_signal pid 'hook'
  signalled script_owned_signal child 'cleanup-second\ncleanup-first'
fi
# The sources a script's header includes (`//> using file`, `//> using files`; tests/using/), which
# tests/cases cannot hold, every other suite building a case alone: each main's output, its exit
# when it is not 0, against its .expected (scalac's under Scala CLI, but where the file says it is
# teq's).
for src in tests/using/*.scala; do
  name=$(basename "$src" .scala)
  actual=$(timeout 30 "$TEQ" interp "$src" 2>&1)
  code=$?
  [ $code = 0 ] || actual="$actual"$'\n'"exit $code"
  if [ "$actual" != "$(cat "tests/using/$name.expected")" ]; then
    echo "USING FILE: $src printed \"$actual\""
    status=1
  fi
done
count() { grep -c "^$1 " "$out/results.txt"; }
echo "$(count pass) passed, $(count unsupported) unsupported, $(count compile) compile, $(count exception) exception, $(count output) output, $(count timeout) timeout, $(count skip) skipped"
exit $status
