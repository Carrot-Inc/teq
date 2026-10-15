#!/bin/bash
# The pool: each machine with its words, free or busy (tests/gate-agent/common.sh).
. "$(dirname "$0")/common.sh"
while read -r name words; do
  [ -n "$name" ] || continue
  if [ -e "$GATE_AGENT_ROOT/claims/$name" ]; then state=busy; else state=$(echo "$words" | awk '{print $1}'); fi
  echo "$name $state $(echo "$words" | cut -s -d' ' -f2-)"
done < "$GATE_AGENT_ROOT/pool"
