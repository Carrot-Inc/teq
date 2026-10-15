#!/bin/bash
# claim.sh <label> <machine>: the machine's handle, or a failure when it is claimed or not free.
. "$(dirname "$0")/common.sh"
grep -q "^$2 free" "$GATE_AGENT_ROOT/pool" || { echo "claim: $2 is not free" >&2; exit 1; }
mkdir -p "$GATE_AGENT_ROOT/claims"
( set -o noclobber; echo "$2/$1-$$" > "$GATE_AGENT_ROOT/claims/$2" ) 2> /dev/null || { echo "claim: $2 is claimed" >&2; exit 1; }
mkdir -p "$GATE_AGENT_ROOT/machines/$2/rw"
echo "$2/$1-$$"
