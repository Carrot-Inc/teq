#!/bin/bash
# The short conditional and the jump over the then arm of Wide.f.
set -o pipefail
javap -c -p -classpath "$1" 'Wide$' | awk '
  /^  [^ ]/ { method = ($0 == "  public void f(boolean);") }
  method && $2 ~ /^(if|goto)/ {
    target = $3
    if ($2 == "goto_w" && target - ($1 + 0) > 32767) target = "forward >32767"
    print $1, $2, target
  }'
