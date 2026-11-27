#!/bin/bash
# run.sh <handle> <project> -t <seconds> <command>: the command in the project's mirror on the machine, in an
# environment of the machine's own, as a login shell's: this machine's PATH, locale, temporary directory, coursier
# cache and JDK and the stand-ins' own variables, then the machine's env file; nothing else of the caller's (the
# variables a machine that holds a project's checkout exports, GATE_LILA_CHECKOUT and the like, among them).
. "$(dirname "$0")/common.sh"
m=$(machine_of "$1")
claimed "$m" "$1" || { echo "run: $1 holds no claim" >&2; exit 1; }
[ "$3" = -t ] || { echo "run: -t <seconds> expected" >&2; exit 2; }
home=$GATE_AGENT_ROOT/machines/$m
mkdir -p "$home/rw/$2"
if [ -n "$GATE_AGENT_STALE_DONE" ]; then
  dir=$home/rw/$2/out/compare
  if [ "$5" = 'cat out/compare/status 2> /dev/null' ]; then
    mapfile -t lines 2> /dev/null < "$dir/status" || exit 1
    [ "${#lines[@]}" -gt 0 ] || exit 0
    if [ "${lines[-1]}" = done ]; then
      now=$(date +%s)
      [ -f "$dir/done-seen" ] || echo "$now" > "$dir/done-seen"
      if [ $((now - $(cat "$dir/done-seen"))) -lt "$GATE_AGENT_STALE_DONE" ]; then unset 'lines[-1]'; fi
    fi
    [ "${#lines[@]}" -eq 0 ] || printf '%s\n' "${lines[@]}"
    exit 0
  elif [ -f "$dir/done-seen" ]; then
    echo "$5" >> "$dir/after-done"
  fi
fi
keep=()
for v in USER LOGNAME LANG LC_ALL TERM TMPDIR COURSIER_CACHE JAVA_HOME GATE_AGENT_ROOT GATE_AGENT_TEQ; do
  [ -z "${!v+x}" ] || keep+=("$v=${!v}")
done
cd "$home/rw/$2" && exec env -i "${keep[@]}" HOME="$home" PATH="$GATE_AGENT_ROOT/bin:$PATH" \
  timeout "$4" bash -c "[ ! -f '$home/env' ] || . '$home/env'; $5"
