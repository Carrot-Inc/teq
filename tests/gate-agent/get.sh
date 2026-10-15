#!/bin/bash
# get.sh <handle> <project> <path> <local path>: a file or a directory of the project's mirror copied here.
. "$(dirname "$0")/common.sh"
m=$(machine_of "$1")
claimed "$m" "$1" || { echo "get: $1 holds no claim" >&2; exit 1; }
src=$GATE_AGENT_ROOT/machines/$m/rw/$2/$3
[ -e "$src" ] || { echo "get: no $3 on $m" >&2; exit 1; }
if [ -d "$src" ]; then mkdir -p "$4" && cp -R "$src/." "$4/"; else cp "$src" "$4"; fi
