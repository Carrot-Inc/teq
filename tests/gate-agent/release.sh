#!/bin/bash
# release.sh <handle>
. "$(dirname "$0")/common.sh"
m=$(machine_of "$1")
claimed "$m" "$1" || { echo "release: $1 holds no claim" >&2; exit 1; }
rm -f "$GATE_AGENT_ROOT/claims/$m"
