#!/bin/bash
# The complete LineNumberTable of the underlying-value selection in Underlying.f.
set -o pipefail
javap -v -p -classpath "$1" 'Underlying$' | awk '
  /^  [^ ]/ { method = ($0 == "  public int f();"); lines = 0 }
  method && $1 == "LineNumberTable:" { lines = 1; next }
  lines && $1 == "line" { print $1, $2, $3; next }
  { lines = 0 }'
