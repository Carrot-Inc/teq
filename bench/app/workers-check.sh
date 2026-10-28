#!/bin/bash
# bench/app/workers-check.sh <teq>: the application API's diagnostics at one typer worker against those at the
# default count, the build failing or passing the same way whatever the schedule (the gate's line app-workers), for
# the main configuration and, where its lists are given, the test configuration. The main lists are api-check.sh's
# (APP_ROOT, APP_MODULES, APP_CLASSPATH, APP_FLAGS; bench/app/app-lists.sh writes them); the test configuration's are
# APP_TEST_MODULES, APP_TEST_CLASSPATH and APP_TEST_FLAGS, over the main lists' products as api-check.sh --test reads
# them: the binary's build of the main lists (`--products`, APP_FLAGS less --werror, the JVM over scala-library), made
# once and read by every arm, in the place of the class path's first `@<project>/<configuration>` line, the other
# such lines left out. Each configuration's flags are its own with the warning flags its scalacOptions
# (APP_SCALAC_OPTIONS, APP_TEST_SCALAC_OPTIONS, one per line) map onto that they lack (tests/warnings/policy.py flags:
# -Werror, -Wtostring-interpolated, the other -W, -deprecation, -feature, -language), printed first, the options that
# are not the warning policy's left out (teq's flags carry them: -source, -Xkind-projector, -Xmax-inlines); a warning
# option it maps onto no flag stops the line. Each arm is `teq compiler check` on the JVM over scala-library: `--threads 1`, then the
# default count, then, where the default arm's attached workers typed no item (`TEQ_FORK_ITEMS=1`, printed whatever
# the exit) or the build gave way to one worker, an arm whose every item an attached worker types
# (`TEQ_IDLE_WORKERS=0 --threads 2`). Each arm's exit (a nonzero one kept: the build may fail the same way at every
# count), its diagnostics as `--diagnostics` writes them, in their order, the items each worker typed and the reason a
# build gave way (`TEQ_SERIAL_TRACE`) and the records the report leaves out (`TEQ_UNPRESENTED`) are recorded under the
# output directory (OUT, default out/app-workers), each arm
# printed with its items on attached workers and any give-way's reason. A configuration passes when every arm's exit
# and diagnostics are the one-worker arm's byte for byte and an arm's attached workers typed items with no give-way;
# where every arm whose attached workers typed gave way, the line says so with each reason. The one-worker arm types
# the configuration on one core: its time is printed, for the gate's bound.
teq=$1
[ -x "$teq" ] || { echo "usage: bench/app/workers-check.sh <teq>" >&2; exit 2; }
case $teq in /*) ;; *) teq=$PWD/$teq ;; esac
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
. "$root/tests/support/compiler-words.sh"
spellings "$teq"
[ -n "$APP_ROOT" ] && [ -f "$APP_MODULES" ] && [ -f "$APP_CLASSPATH" ] && [ -n "${APP_FLAGS+set}" ] || { echo "workers-check: set APP_ROOT, APP_MODULES, APP_CLASSPATH and APP_FLAGS"; exit 1; }
[ -d "$APP_ROOT" ] || { echo "workers-check: no application checkout at $APP_ROOT"; exit 1; }
out=${OUT:-$root/out/app-workers}
rm -rf "$out"
mkdir -p "$out"
# with_warning_flags <flags> <scalac options file>: the flags with each option, with its value, the options map onto
# and the flags lack (`--language a,b` beside a `--language c`, which teq adds up).
with_warning_flags() {
  local flags=$1 mapped f i
  if [ -f "$2" ]; then
    mapped=$(python3 "$root/tests/warnings/policy.py" flags --warnings-only <<< "$(grep -v '^#' "$2" | tr '\n' ' ')") || { echo "workers-check: $2 holds a warning option tests/warnings/policy.py maps to no teq flag" >&2; return 1; }
    read -ra words <<< "$mapped"
    i=0
    while [ $i -lt ${#words[@]} ]; do
      f=${words[$i]}
      case $f in --wunused|--wconf|--language) f="$f ${words[$((i + 1))]}"; i=$((i + 1)) ;; esac
      i=$((i + 1))
      case " $flags " in *" $f "*) ;; *) flags="$flags $f" ;; esac
    done
  fi
  echo "$flags"
}
# attached_items <arm>: the items the attached workers (1 and after) typed in the arm, from its `teq: forked` lines.
attached_items() {
  grep -o 'teq: forked .*' "$out/$1.out" | grep -o 'worker [1-9][0-9]*: [0-9]* items' | awk '{ n += $3 } END { print n + 0 }'
}
# gave_way <arm>: why the arm's build gave way to one worker, if it did.
gave_way() {
  grep -m1 -o 'typed again by one worker: .*\|gave way to one worker: .*' "$out/$1.out" | cut -c1-400
}
# arm <name> <env...> -- <threads...>: one check of the configuration in `sources`, `cp` and `flags`; its exit, its
# streams, its diagnostics and its time.
arm() {
  local name=$1
  shift
  local env=(TEQ_SERIAL_TRACE=1 TEQ_FORK_ITEMS=1 TEQ_UNPRESENTED=1)
  while [ "$1" != -- ]; do env+=("$1"); shift; done
  shift
  local start=$(date +%s)
  (cd "$APP_ROOT" && env "${env[@]}" timeout 1200 "$teq" $(compiler_words "$teq") check $sources --classpath "$cp" $flags \
    --std scala-library --target jvm "$@" --diagnostics "$out/$name.json") > "$out/$name.out" 2>&1
  echo $? > "$out/$name.exit"
  touch "$out/$name.json"
  local why
  why=$(gave_way "$name")
  echo "workers-check: $name exit $(cat "$out/$name.exit") in $(($(date +%s) - start)) s, $(grep -c '' "$out/$name.json") diagnostics, $(attached_items "$name") items on attached workers ($(grep -m1 -o 'forked .*' "$out/$name.out" | cut -c1-200))${why:+; $why}"
}
# configuration <name>: its arms and its verdict, from `sources`, `cp` and `flags`; 1 where it fails.
configuration() {
  local c=$1 a why typed=0 kept=0 reasons="" fail=0
  echo "workers-check: $c: flags $flags"
  arm $c.one -- --threads 1
  arm $c.default --
  local arms="$c.default"
  if [ "$(attached_items $c.default)" = 0 ] || [ -n "$(gave_way $c.default)" ]; then
    arm $c.attached TEQ_IDLE_WORKERS=0 -- --threads 2
    arms="$c.default $c.attached"
  fi
  for a in $arms; do
    why=$(gave_way $a)
    if [ "$(attached_items $a)" != 0 ]; then
      typed=1
      if [ -z "$why" ]; then kept=1; else reasons="$reasons; $a: $why"; fi
    fi
    if ! cmp -s "$out/$c.one.exit" "$out/$a.exit"; then
      echo "workers-check: FAIL $a exits $(cat "$out/$a.exit") where one worker's exits $(cat "$out/$c.one.exit")"
      fail=1
    elif ! cmp -s "$out/$c.one.json" "$out/$a.json"; then
      echo "workers-check: FAIL $a's diagnostics are not one worker's: $(diff "$out/$c.one.json" "$out/$a.json" | grep '^[<>]' | head -1 | cut -c1-200)"
      fail=1
    fi
  done
  if [ $kept = 0 ]; then
    if [ $typed = 1 ]; then
      echo "workers-check: FAIL $c: every arm whose attached workers typed items gave way to one worker${reasons}"
    else
      echo "workers-check: FAIL $c: no arm typed an item on an attached worker"
    fi
    fail=1
  fi
  [ $fail = 0 ] && echo "workers-check: $c: the diagnostics and exit at every count are one worker's ($arms against one)"
  return $fail
}
failed=0
sources=$(grep -v '^#' "$APP_MODULES" | tr '\n' ' ')
main_cp=$(sed "s|^~/|$HOME/|" "$APP_CLASSPATH" | paste -sd: -)
cp=$main_cp
flags=$(with_warning_flags "$APP_FLAGS" "$APP_SCALAC_OPTIONS") || exit 1
configuration main || failed=1
if [ -f "$APP_TEST_MODULES" ] && [ -f "$APP_TEST_CLASSPATH" ] && [ -n "${APP_TEST_FLAGS+set}" ]; then
  products=$out/main-products
  main_sources=$sources
  if ! (cd "$APP_ROOT" && timeout 600 "$teq" $(compiler_words "$teq") build --products "$products" $main_sources --classpath "$main_cp" \
    $(echo " $APP_FLAGS " | sed 's/ --werror / /g') --std scala-library --target jvm) > "$out/main-products.log" 2>&1; then
    echo "workers-check: FAIL the build of the main lists' products, which the test configuration reads ($(tail -1 "$out/main-products.log"))"
    exit 1
  fi
  sources=$(grep -v '^#' "$APP_TEST_MODULES" | tr '\n' ' ')
  cp=$(awk -v p="$products" -v home="$HOME" '/^@/ { if (!placed) print p; placed = 1; next } { sub(/^~\//, home "/"); print }' "$APP_TEST_CLASSPATH" | paste -sd: -)
  flags=$(with_warning_flags "$APP_TEST_FLAGS" "$APP_TEST_SCALAC_OPTIONS") || exit 1
  configuration test || failed=1
else
  echo "workers-check: the test configuration not checked: set APP_TEST_MODULES, APP_TEST_CLASSPATH and APP_TEST_FLAGS"
fi
exit $failed
