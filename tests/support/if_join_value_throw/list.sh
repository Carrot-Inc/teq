#!/bin/bash
set -o pipefail
javap -v -p -classpath "$1" 'Layout$' | awk '
  /^  [^ ]|^}/ { method = ($0 == "  public int f(boolean);"); table = 0 }
  method && /^      [^ ]/ { table = ($0 ~ /^      (Exception table:|StackMapTable:)/) }
  method && ($1 ~ /^[0-9]+:$/ || table) {
    gsub(/#[0-9]+/, "#")
    $1 = $1
    print
  }'
