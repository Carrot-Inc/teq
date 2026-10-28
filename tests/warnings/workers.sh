#!/bin/bash
# The diagnostics of a program typed by attached workers against one worker's (`Worker::attach`, src/typer/mod.rs):
# each case of tests/warnings/workers, a directory of files whose `runs` lists the runs (per line the exit teq gives at
# one worker and teq's flags), is checked at `--threads 1`, under `TEQ_IDLE_WORKERS=0 --threads 2`, which keeps worker
# 0 out of the body work so that every item is an attached worker's whatever the program's size, and at
# `--threads 8`; on JavaScript, on the JVM against scala-library 3.8.4 from the coursier cache, and by the
# interpreter. Each run's diagnostics as `--diagnostics` writes them, in their order, and its exit are compared with
# the one-worker run's byte for byte, and that run's exit with the line's. The runs under the hook have to have typed
# items on an attached worker (`TEQ_FORK_ITEMS=1`, printed whatever the exit); no run may give way to one worker
# (`TEQ_SERIAL_TRACE`). A case's `env` gives every run the variables it lists (`TEQ_PHASE_FORK=1`); its `units`
# says what the inlining phase does (`TEQ_INLINE_COUNTS`): `attached`, every unit under the hook expanded by an
# attached worker, the hook keeping worker 0 from the phase's units too, or `none`, no fork at any count; its
# `gives_way` lets a run give way to one worker, whose output is one worker's by construction, where placing the
# reports as one worker's walk makes them cannot be established (an event told away from its first meeting, a
# history read the walk's order answers otherwise). A case with a `lib` directory has its macros there, an upstream
# module's, built once into products (`--products`, at one worker) that every run of its `use` directory reads on its
# class path: a macro of the run suspends the unit that meets it, an upstream one expands where the unit stands. Run by
# tests/run_errors.sh; the last line gives the counts. Without the jar the JVM runs are skipped.
cd "$(dirname "$0")/../.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
library=$(scala_library_jar)
records=out/workers-warnings
rm -rf "$records"
mkdir -p "$records"
fail=0
matched=0
total=0
# run <record> <mode> <arm> <case> <flags...>: one run, its diagnostics to <record>.json, its streams to <record>.out,
# its exit to <record>.exit; a case with a library reads its products (`lib_products`) and checks its `use` directory.
run() {
  local record=$1 mode=$2 arm=$3 dir=$4
  shift 4
  local verb=(compiler check) env=(TEQ_SERIAL_TRACE=1 TEQ_INLINE_COUNTS=1 TEQ_FORK_ITEMS=1) threads=(--threads 1) extra=() src=$dir cp=()
  [ -f "$dir/env" ] && env+=($(cat "$dir/env"))
  [ -n "$lib_products" ] && { src=$dir/use; cp=("$lib_products"); }
  case $mode in
    jvm) cp=("$library" "${cp[@]}"); extra=(--target jvm --std scala-library) ;;
    interp) verb=(interp) ;;
  esac
  [ ${#cp[@]} -gt 0 ] && extra+=(--classpath "$(IFS=:; echo "${cp[*]}")")
  case $arm in
    hook) env+=(TEQ_IDLE_WORKERS=0) threads=(--threads 2) ;;
    eight) threads=(--threads 8) ;;
  esac
  env "${env[@]}" timeout 120 "$TEQ" "${verb[@]}" "$src" "$@" "${threads[@]}" "${extra[@]}" --diagnostics "$record.json" > "$record.out" 2>&1
  echo $? > "$record.exit"
  touch "$record.json"
}
# units <attached|none> <arm> <streams>: whether the inlining phase's units went as the case says.
units() {
  local counts
  counts=$(grep -o 'forked units [0-9]* attached units [0-9]*' "$3" | tail -1)
  case $1 in
    none) [ "$counts" = "forked units 0 attached units 0" ] ;;
    attached) [ $2 != hook ] || { local f a; read -r _ _ f _ _ a <<< "$counts"; [ "${f:-0}" -gt 0 ] && [ "$f" = "$a" ]; } ;;
  esac
}
for dir in tests/warnings/workers/*/; do
  name=$(basename "$dir")
  n=0
  lib_products=""
  if [ -d "$dir/lib" ]; then
    lib_products=$records/$name.lib
    if ! timeout 120 "$TEQ" compiler check --products "$lib_products" --sourceroot "$dir/lib" "$dir"/lib/*.scala --threads 1 > "$records/$name.lib.out" 2>&1; then
      echo "FAIL $name: its library's products did not build ($(tail -1 "$records/$name.lib.out"))"
      fail=1
      continue
    fi
  fi
  while read -r expected flags; do
    case "$expected" in '#'*|'') continue ;; esac
    n=$((n + 1))
    for mode in js jvm interp; do
      [ $mode = jvm ] && [ ! -f "$library" ] && continue
      base=$records/$name@$n.$mode
      run "$base.one" $mode one "$dir" $flags
      if [ "$(cat "$base.one.exit")" != "$expected" ]; then
        echo "FAIL $name@$n ($mode, $flags): exit $(cat "$base.one.exit") at one worker, not $expected"
        fail=1
      fi
      for arm in hook eight; do
        total=$((total + 1))
        run "$base.$arm" $mode $arm "$dir" $flags
        why=""
        if ! cmp -s "$base.one.json" "$base.$arm.json"; then
          why="its diagnostics are not one worker's: $(diff "$base.one.json" "$base.$arm.json" | grep '^[<>]' | head -1 | cut -c1-160)"
        elif [ "$(cat "$base.one.exit")" != "$(cat "$base.$arm.exit")" ]; then
          why="exit $(cat "$base.$arm.exit") where one worker's is $(cat "$base.one.exit")"
        elif [ ! -f "$dir/gives_way" ] && grep -q 'typed again by one worker' "$base.$arm.out"; then
          why="it gave way to one worker: $(grep -m1 'typed again by one worker' "$base.$arm.out")"
        elif [ $arm = hook ] && ! grep -m1 'teq: forked ' "$base.$arm.out" | grep -Eq 'worker 1: [1-9][0-9]* items'; then
          why="no attached worker typed an item"
        elif [ -f "$dir/units" ] && ! units "$(cat "$dir/units")" $arm "$base.$arm.out"; then
          why="the inlining phase's units are not $(cat "$dir/units") ($(grep -o 'forked units [0-9]* attached units [0-9]*' "$base.$arm.out"))"
        fi
        if [ -z "$why" ]; then
          matched=$((matched + 1))
        else
          echo "FAIL $name@$n ($mode, $arm, $flags): $why"
          fail=1
        fi
      done
    done
  done < "$dir/runs"
done
# A resident check's answers, under the hook and at sixteen workers, against a one-worker session's, their `ok` and
# diagnostics in order: the lint program built twice under its flags; and tests/warnings/workers-watch/retype, whose
# edit makes a retype run a macro that changes its state in the inlining phase, forked (`TEQ_PHASE_FORK=1`), which
# gives the build to one worker: the session answers with a full build of one worker, then the next edit; and
# tests/warnings/workers-watch/definition, whose edit of an inline method with a deprecated reference makes the retype
# check the definition again, the inlining phase forked with every unit an attached worker's: the warning reported
# again after the claims of the full build's fork (`merge::Claims`, made anew at each fork; the retype's walk, of one
# worker, claims nothing, so this passed before them too).
# session <record> <arm> <dir> <edit> <flags...>: the answers of `build`, the edit's text and `build`, then `build`.
session() {
  local record=$1 arm=$2 dir=$3 edit=$4
  shift 4
  local env=(TEQ_SERIAL_TRACE=1) threads=(--threads 1)
  case $arm in
    hook) env+=(TEQ_IDLE_WORKERS=0) threads=(--threads 2) ;;
    sixteen) env+=(TEQ_PHASE_FORK=1) threads=(--threads 16) ;;
    phase-hook) env+=(TEQ_PHASE_FORK=1 TEQ_IDLE_WORKERS=0) threads=(--threads 2) ;;
    one-phase) env+=(TEQ_PHASE_FORK=1) ;;
  esac
  {
    echo build
    if [ -n "$edit" ]; then
      echo "text $PWD/$dir/f0.scala $(wc -c < "$edit" | tr -d ' ')"
      cat "$edit"
    fi
    echo build
    echo build
    echo quit
  } | env "${env[@]}" timeout 300 "$TEQ" compiler watch --check "$dir" "$@" "${threads[@]}" > "$record.out" 2>&1
  echo $? > "$record.exit"
  python3 -c '
import json, sys
for line in open(sys.argv[1]):
    if line.startswith("{"):
        d = json.loads(line)
        print(json.dumps([d.get("ok"), d.get("diagnostics"), d.get("warnings")]))
' "$record.out" > "$record.answers"
}
for spec in "lint|hook||--wtostring-interpolated --werror" "workers-watch/retype|sixteen|tests/warnings/workers-watch/retype/edit-f0.txt|" "workers-watch/definition|phase-hook|tests/warnings/workers-watch/definition/edit-f0.txt|--deprecation"; do
  IFS='|' read -r case arm edit flags <<< "$spec"
  dir=tests/warnings/$case
  [ -d "$dir" ] || dir=tests/warnings/workers/$case
  name=watch-$(basename "$case")
  base=$records/$name
  total=$((total + 1))
  session "$base.one" $(case $arm in sixteen | phase-hook) echo one-phase ;; *) echo one ;; esac) "$dir" "$edit" $flags
  session "$base.$arm" $arm "$dir" "$edit" $flags
  why=""
  if [ "$(cat "$base.one.exit")" != 0 ] || [ "$(cat "$base.$arm.exit")" != 0 ]; then
    why="a session ended with exit $(cat "$base.one.exit") at one worker, $(cat "$base.$arm.exit") $arm"
  elif [ ! -s "$base.one.answers" ] || ! cmp -s "$base.one.answers" "$base.$arm.answers"; then
    why="its answers are not one worker's: $(diff "$base.one.answers" "$base.$arm.answers" | grep '^[<>]' | head -1 | cut -c1-160)"
  elif [ $arm = sixteen ] && ! grep -q 'a retype gave way' "$base.$arm.out"; then
    why="the retype did not give way"
  fi
  if [ -z "$why" ]; then
    matched=$((matched + 1))
  else
    echo "FAIL $name ($arm, $flags): $why"
    fail=1
  fi
done
# Sessions of successive edits (a case's `edits` the files and their texts in order) whose builds read the history:
# tests/warnings/workers-watch/history_retype, where the retained reports are a retype's history before its own
# walk's, as one worker's session has them, whatever the full build's workers (before, the secondary constructor's
# check read none of them after a full build under the hook and reported a third error at two and eight workers, on
# every build after the edit), at one worker against the hook at two and at eight workers; and
# tests/warnings/workers-watch/syntax_history, where a syntax error answers every read of whether an error was
# reported (its `edits` says how), at one worker against two workers under the hook and eight and sixteen. Each answer
# one worker's, no build given to one worker (the watch prints why under `TEQ_SERIAL_TRACE`).
# edits_session <record> <arm> <dir>: `build`, then each edit's text and `build`, then `build`.
edits_session() {
  local record=$1 arm=$2 dir=$3
  local env=(TEQ_SERIAL_TRACE=1) threads=(--threads 1)
  case $arm in
    hook) env+=(TEQ_IDLE_WORKERS=0) threads=(--threads 2) ;;
    hook8) env+=(TEQ_IDLE_WORKERS=0) threads=(--threads 8) ;;
    plain8) threads=(--threads 8) ;;
    plain16) threads=(--threads 16) ;;
  esac
  {
    echo build
    while read -r file edit; do
      case "$file" in '#'*|'') continue ;; esac
      echo "text $PWD/$dir/$file $(wc -c < "$dir/$edit" | tr -d ' ')"
      cat "$dir/$edit"
      echo build
    done < "$dir/edits"
    echo build
    echo quit
  } | env "${env[@]}" timeout 300 "$TEQ" compiler watch --check "$dir" "${threads[@]}" > "$record.out" 2>&1
  echo $? > "$record.exit"
  python3 -c '
import json, sys
for line in open(sys.argv[1]):
    if line.startswith("{"):
        d = json.loads(line)
        print(json.dumps([d.get("ok"), d.get("diagnostics"), d.get("warnings")]))
' "$record.out" > "$record.answers"
}
for spec in "history_retype|hook hook8" "syntax_history|hook plain8 plain16"; do
  IFS='|' read -r case arms <<< "$spec"
  dir=tests/warnings/workers-watch/$case
  base=$records/watch-$case
  work_copy=$records/$case-src
  for arm in one $arms; do
    rm -rf "$work_copy.$arm"
    cp -r "$dir" "$work_copy.$arm"
    edits_session "$base.$arm" $arm "$work_copy.$arm"
  done
  for arm in $arms; do
    total=$((total + 1))
    why=""
    if [ "$(cat "$base.one.exit")" != 0 ] || [ "$(cat "$base.$arm.exit")" != 0 ]; then
      why="a session ended with exit $(cat "$base.one.exit") at one worker, $(cat "$base.$arm.exit") $arm"
    elif [ ! -s "$base.one.answers" ] || ! diff -q <(sed "s|$work_copy.one|SRC|g" "$base.one.answers") <(sed "s|$work_copy.$arm|SRC|g" "$base.$arm.answers") > /dev/null; then
      why="its answers are not one worker's: $(diff <(sed "s|$work_copy.one|SRC|g" "$base.one.answers") <(sed "s|$work_copy.$arm|SRC|g" "$base.$arm.answers") | grep '^[<>]' | head -1 | cut -c1-160)"
    elif grep -q 'typed again by one worker' "$base.$arm.out"; then
      why="it gave way to one worker: $(grep -m1 'typed again by one worker' "$base.$arm.out" | cut -c1-200)"
    fi
    if [ -z "$why" ]; then
      matched=$((matched + 1))
    else
      echo "FAIL watch-$case ($arm): $why"
      fail=1
    fi
  done
done
# Dotty's own tests of the checks that read the policy, at one worker and under the hook (tests/warnings/workers_dotty.py,
# whose docstring says how; each one's difference from scalac's committed diagnostics is printed with it).
dotty=$(python3 tests/warnings/workers_dotty.py "$TEQ" "$records/dotty" 2>&1)
[ $? -ne 0 ] && fail=1
grep '^FAIL' <<< "$dotty"
summary="workers' diagnostics: $matched of $total runs as one worker's; $(tail -1 <<< "$dotty")"
if [ $fail = 0 ]; then echo "$summary"; else echo "$summary; failed"; fi
exit $fail
