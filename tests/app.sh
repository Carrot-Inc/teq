#!/bin/bash
# The application corpus (bench/app): generated from its seed at full scale, the frontend side built
# by teq as a split JS bundle over the jars it reads (cats, sourcecode) and run under node against
# scalac's output in bench/app/expected/frontend.txt, the API side likewise against api.txt, and the
# API side once more with --target jvm, whose output is compared too when that build succeeds (a
# JVM build that does not succeed is noted, not failed). The frontend is then built a second time,
# which has to give the same bytes, and a watch session over it gets one string literal of one
# detail page changed, which has to rewrite that page's module alone and leave the output equal
# to a fresh build's. The session types at the automatic count, the counts a caller's environment
# asks for cleared for it alone: its first build forks and, the corpus's class catalog undeclared,
# gives way and is typed again by one worker; the fresh build it is compared with is pinned to one
# worker. A check session with the navigation index over the same frontend at the automatic count
# answers its first build, two retypes, a build that takes the full path again and the queries after
# each as the session at one worker does (tests/support/session.py). Missing expectations are produced with
# scala-cli (--regen redoes them; the corpus pins Scala 3.8.4 in its project.scala); without the
# jars in the coursier cache the suite skips and counts as passed. SCALE scales the corpus
# (default 1, the scale the expectations are recorded at).
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
scale=${SCALE:-1}
regen=0
[ "$1" = "--regen" ] && regen=1
work=out/app-test
expectations=bench/app/expected
pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() {
  echo "FAIL $1"
  fail=$((fail + 1))
}
cp=""
missing=""
for name in scala-library cats-kernel cats-core sourcecode; do
  path=$(jar_of "$name")
  [ -e "$path" ] || missing="$missing $name"
  cp="$cp:$path"
done
cp=${cp#:}
if [ -n "$missing" ]; then
  echo "app: skipped, not in the coursier cache:$missing"
  exit 0
fi
rm -rf "$work"
mkdir -p "$work" "$expectations"
if ! timeout 120 python3 bench/app/gen.py "$work/src" --scale "$scale" > "$work/gen.log" 2>&1; then
  echo "FAIL generation: $(tail -3 "$work/gen.log")"
  exit 1
fi

# scalac's output of a side: the recorded expectation, produced here when missing or on --regen
reference() {
  local name=$1 main=$2
  shift 2
  local expected=$expectations/$name.txt
  if [ $regen = 1 ] || [ ! -f "$expected" ]; then
    if [ "$scale" != 1 ]; then
      bad "$name: the expectations are recorded at scale 1, not $scale"
      return 1
    fi
    if ! (cd "$work/src" && timeout 360 scala-cli --power run "$@" --jvm system --server=false --offline --main-class "$main" 2> "../$name.ref.err") > "$expected"; then
      bad "$name reference (see $work/$name.ref.err)"
      rm -f "$expected"
      return 1
    fi
  fi
}
# a side built as a split bundle and run under node against its expectation
js_side() {
  local name=$1
  shift
  local expected=$expectations/$name.txt
  [ -f "$expected" ] || return
  if ! timeout 120 "$TEQ" compiler build "$@" --classpath "$cp" --split "$work/$name" > "$work/$name.build.log" 2>&1; then
    bad "$name build: $(grep -m1 'error' "$work/$name.build.log" | cut -c1-200)"
    return
  fi
  timeout 120 node "$work/$name/main.mjs" > "$work/$name.actual" 2>&1
  if diff -q "$expected" "$work/$name.actual" > /dev/null; then ok; else
    bad "$name under node differs from scalac's output"
    diff "$expected" "$work/$name.actual" | head -${DIFF_LINES:-10}
  fi
}
# the API side for the JVM: compared when it builds, noted when it does not
jvm_side() {
  local expected=$expectations/api.txt
  [ -f "$expected" ] || return
  if ! timeout 300 "$TEQ" compiler build "$work/src/shared" "$work/src/api" --classpath "$cp" --target jvm -o "$work/api.jar" > "$work/api.jvm.log" 2>&1; then
    echo "note: the API side does not build for the JVM: $(grep -m1 -E 'error|not supported' "$work/api.jvm.log" | cut -c1-200)"
    return
  fi
  timeout 120 java -Xss512m -XX:+UseSerialGC -XX:TieredStopAtLevel=1 -XX:-UsePerfData -cp "$work/api.jar:$cp" TeqMain > "$work/api.jvm.actual" 2>&1
  if diff -q "$expected" "$work/api.jvm.actual" > /dev/null; then ok; else
    bad "api under the JVM differs from scalac's output"
    diff "$expected" "$work/api.jvm.actual" | head -${DIFF_LINES:-10}
  fi
}

# a second build of the frontend against the first: the output is a function of the sources
same_again() {
  [ -d "$work/frontend" ] || return
  if ! timeout 120 "$TEQ" compiler build "$work/src/shared" "$work/src/frontend" --classpath "$cp" --split "$work/frontend-again" > "$work/frontend-again.build.log" 2>&1; then
    bad "frontend second build: $(grep -m1 'error' "$work/frontend-again.build.log" | cut -c1-200)"
    return
  fi
  if diff -rq "$work/frontend" "$work/frontend-again" > "$work/frontend-again.diff" 2>&1; then ok; else
    bad "two builds of the frontend differ: $(head -3 "$work/frontend-again.diff" | tr '\n' ' ')"
  fi
}
# one string literal of one detail page changed under `teq compiler watch`: the page's module is rewritten
# and nothing else, and the output is what a fresh build gives (the former gap watch_inline_names)
watch_edit() {
  [ -d "$work/frontend" ] || return
  local file
  file=$(ls "$work"/src/frontend/frontend/page/*/*DetailPage.scala | head -1)
  local pkg
  pkg=$(sed -n 's/^package \(.*\)$/\1/p' "$file" | head -1)
  rm -f "$work/cmd" "$work/ans"
  mkfifo "$work/cmd" "$work/ans"
  : > "$work/watch.log"
  env -u TEQ_THREADS -u TEQ_SESSION_WORKERS -u TEQ_SESSION_THREADS TEQ_SESSION_WORKERS_LOG="$work/watch.log" timeout 300 "$TEQ" compiler watch "$work/src/shared" "$work/src/frontend" --classpath "$cp" --split "$work/frontend-watch" < "$work/cmd" > "$work/ans" 2> "$work/watch.err" &
  local wpid=$!
  exec 3> "$work/cmd" 4< "$work/ans"
  local result=
  read -r -t 120 result <&4 || result='{"ok":false}'
  sed -i.bak 's/"home[0-9]*"/"home (edited)"/' "$file"
  printf 'build %s\n\n' "$file" >&3
  read -r -t 120 result <&4 || result='{"ok":false}'
  echo quit >&3
  exec 3>&- 4<&-
  wait "$wpid" 2> /dev/null
  local changed
  changed=$(echo "$result" | grep -o '"changed":\[[^]]*\]' | head -1 | sed 's/"changed"://')
  if [ "$changed" = "[\"$pkg.mjs\"]" ]; then ok; else
    bad "a literal edited in ${file#$work/src/} rewrote $changed, not [\"$pkg.mjs\"] ($(echo "$result" | cut -c1-200))"
  fi
  rm -rf "$work/frontend-fresh"
  if ! timeout 120 "$TEQ" compiler build "$work/src/shared" "$work/src/frontend" --classpath "$cp" --threads 1 --split "$work/frontend-fresh" > "$work/frontend-fresh.build.log" 2>&1; then
    bad "frontend build after the edit: $(grep -m1 'error' "$work/frontend-fresh.build.log" | cut -c1-200)"
    return
  fi
  if diff -rq "$work/frontend-watch" "$work/frontend-fresh" > "$work/frontend-watch.diff" 2>&1; then ok; else
    bad "the watch output after the edit differs from a fresh build: $(head -3 "$work/frontend-watch.diff" | tr '\n' ' ')"
  fi
  echo "app: the watch session's first build at the automatic count: $(cut -d' ' -f4- "$work/watch.log" | head -1)"
}
# The check session with the navigation index at the automatic count against one worker's, byte for byte.
session_as_one() {
  [ -d "$work/frontend" ] || return
  local file
  file=$(ls "$work"/src/frontend/frontend/page/*/*DetailPage.scala | head -1)
  timeout 300 python3 tests/support/session.py "$TEQ" "$file" DetailPage -- "$work/src/shared" "$work/src/frontend" --classpath "$cp" --threads 1 > "$work/session.one" 2>&1
  : > "$work/session.log"
  env -u TEQ_THREADS -u TEQ_SESSION_WORKERS -u TEQ_SESSION_THREADS TEQ_SESSION_WORKERS_LOG="$work/session.log" timeout 300 python3 tests/support/session.py "$TEQ" "$file" DetailPage -- "$work/src/shared" "$work/src/frontend" --classpath "$cp" > "$work/session.auto" 2> "$work/session.auto.err"
  if cmp -s "$work/session.one" "$work/session.auto"; then ok; else
    bad "the check session at the automatic count answers otherwise than at one worker: $(diff "$work/session.one" "$work/session.auto" | head -3 | cut -c1-200 | tr '\n' ' ')"
  fi
  # The note of a build whose automatic attempt gave way, once for each such build.
  local retried notes
  retried=$(grep -c ' retried ' "$work/session.log")
  notes=$(grep -c '^teq: a parallel attempt gave way to one worker' "$work/session.auto.err")
  if [ "$notes" = "$retried" ] && [ "$(wc -l < "$work/session.auto.err" | tr -d ' ')" = "$notes" ]; then ok; else
    bad "the check session's stderr: $notes notes for $retried builds retried: $(head -c 300 "$work/session.auto.err")"
  fi
  echo "app: the check session's full builds at the automatic count: $(cut -d' ' -f4- "$work/session.log" | grep -v '^[0-9]*$' | tr '\n' ';')"
}

reference frontend meridian.frontend.Main shared frontend
reference api meridian.server.Main shared api
js_side frontend "$work/src/shared" "$work/src/frontend"
js_side api "$work/src/shared" "$work/src/api"
jvm_side
same_again
watch_edit
session_as_one
echo "app: $pass passed, $fail failed"
[ $fail -eq 0 ]
