#!/bin/bash
# The reporting policy of warnings against scalac 3.8.4's (src/warnings.rs): each run of
# tests/warnings/policy.txt, a probe of tests/warnings/policy with scalac's options, is checked
# by teq with the flags they map to (tests/warnings/policy.py flags) and run by teq's interpreter,
# and each one's exit and diagnostics, normalized by tests/warnings/policy.py (one line per
# diagnostic: its severity, place and first line, so that one given twice counts twice), are
# compared with scalac's, pinned in tests/warnings/expected/policy by
# tests/warnings/capture-policy.sh. The flags `-deprecation`, `-feature`, `-Wconf`, `@nowarn`,
# `--werror` in both directions: a warning given, summarized, silenced, promoted or demoted, and
# the exit that follows. Run by tests/run_errors.sh; the last line gives the counts.
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
  for mode in check interp; do
    total=$((total + 1))
    if [ $mode = check ]; then
      out=$(eval timeout 60 "$TEQ" compiler check "$src" "$flags" 2>&1)
    else
      out=$(eval timeout 60 "$TEQ" interp "$src" "$flags" 2>&1)
    fi
    code=$?
    printf '%s\n' "$out" > "$records/$name.$mode.out"
    got=$(python3 tests/warnings/policy.py teq "$code" "$probe.scala" <<< "$out")
    if [ "$got" = "$(cat "$expected")" ]; then
      matched=$((matched + 1))
    else
      echo "FAIL $name ($mode, $options): teq's diagnostics or exit differ from scalac's ($src)"
      diff <(cat "$expected") <(printf '%s\n' "$got") | sed 's/^/  /'
      fail=1
    fi
  done
done < tests/warnings/policy.txt
summary="warning policy: $matched of $total runs as scalac"
if [ $fail = 0 ]; then echo "$summary"; else echo "$summary; failed"; fi
exit $fail
