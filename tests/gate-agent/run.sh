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
keep=()
for v in USER LOGNAME LANG LC_ALL TERM TMPDIR COURSIER_CACHE JAVA_HOME GATE_AGENT_ROOT GATE_AGENT_TEQ; do
  [ -z "${!v+x}" ] || keep+=("$v=${!v}")
done
cd "$home/rw/$2" && exec env -i "${keep[@]}" HOME="$home" PATH="$GATE_AGENT_ROOT/bin:$PATH" \
  timeout "$4" bash -c "[ ! -f '$home/env' ] || . '$home/env'; $5"
