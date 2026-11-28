#!/bin/bash
set -o pipefail
javap -v -p -classpath "$1" 'Lines$' | awk '
  /^  [^ ]/ { method = ($3 ~ /^(argument|condition|branch)(And|Not|Equal)\(/); lines = 0; if (method) print $0 }
  method && $1 == "LineNumberTable:" { lines = 1; next }
  lines && $1 == "line" { print $1, $2, $3; next }
  { lines = 0 }'
