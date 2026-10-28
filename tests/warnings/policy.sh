#!/bin/bash
# The reporting policy of warnings against scalac 3.8.4's (src/warnings.rs): each run of
# tests/warnings/policy.txt, a probe of tests/warnings/policy with scalac's options, is checked
# by teq with the flags they map to (tests/warnings/policy.py flags) and run by teq's interpreter,
# and each one's exit and diagnostics, normalized by tests/warnings/policy.py (one line per
# diagnostic: its severity, place and first line, so that one given twice counts twice), are
# compared with scalac's, pinned in tests/warnings/expected/policy by
# tests/warnings/capture-policy.sh. The flags `-deprecation`, `-feature`, `-Wconf`, `@nowarn`,
# `--werror` in both directions: a warning given, summarized, silenced, promoted or demoted, and
# the exit that follows. Each run once more under `TEQ_IDLE_WORKERS=0 --threads 2`, every item an
# attached worker's: against scalac's as the others, and its whole output and exit against the
# one-worker run's, unsorted. Run by tests/run_errors.sh; the last line gives the counts.
cd "$(dirname "$0")/../.."
TEQ=${TEQ:-./target/release/teq}
records=out/warnings-policy
rm -rf "$records"
mkdir -p "$records"
fail=0
matched=0
total=0
declare -A seen
while IFS= read -r line; do
  case "$line" in '#'*|'') continue ;; esac
  probe=${line%%:*}
  options=${line#*:}
  seen[$probe]=$(( ${seen[$probe]:-0} + 1 ))
  name="$probe@${seen[$probe]}"
  expected=tests/warnings/expected/policy/$name.txt
  src=tests/warnings/policy/$probe.scala
  flags=$(python3 tests/warnings/policy.py flags <<< "$options")
  for mode in check interp check-attached interp-attached; do
    total=$((total + 1))
    case $mode in
      check) out=$(eval timeout 60 "$TEQ" compiler check "$src" "$flags" 2>&1) ;;
      interp) out=$(eval timeout 60 "$TEQ" interp "$src" "$flags" 2>&1) ;;
      check-attached) out=$(eval TEQ_IDLE_WORKERS=0 timeout 60 "$TEQ" compiler check "$src" "$flags" --threads 2 2>&1) ;;
      interp-attached) out=$(eval TEQ_IDLE_WORKERS=0 timeout 60 "$TEQ" interp "$src" "$flags" --threads 2 2>&1) ;;
    esac
    code=$?
    printf '%s\n' "$out" > "$records/$name.$mode.out"
    echo $code > "$records/$name.$mode.exit"
    got=$(python3 tests/warnings/policy.py teq "$code" "$probe.scala" <<< "$out")
    if [ "$got" != "$(cat "$expected")" ]; then
      echo "FAIL $name ($mode, $options): teq's diagnostics or exit differ from scalac's ($src)"
      diff <(cat "$expected") <(printf '%s\n' "$got") | sed 's/^/  /'
      fail=1
    elif [[ $mode == *-attached ]] && ! { cmp -s "$records/$name.${mode%-attached}.out" "$records/$name.$mode.out" && cmp -s "$records/$name.${mode%-attached}.exit" "$records/$name.$mode.exit"; }; then
      # The attached workers' output, unsorted, against one worker's, its exit too (policy.py sorts).
      echo "FAIL $name ($mode, $options): attached workers' output or exit is not one worker's ($src)"
      diff "$records/$name.${mode%-attached}.out" "$records/$name.$mode.out" | head -5 | sed 's/^/  /'
      fail=1
    else
      matched=$((matched + 1))
    fi
  done
done < tests/warnings/policy.txt
summary="warning policy: $matched of $total runs as scalac"
if [ $fail = 0 ]; then echo "$summary"; else echo "$summary; failed"; fi
exit $fail
