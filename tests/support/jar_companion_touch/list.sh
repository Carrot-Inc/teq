#!/usr/bin/env bash
# The companions touched before a creation in the class files under $1: each `getstatic C$.MODULE$`, `pop` and
# `new C` in a row, one line a companion with the number of such creations, from javap.
find "$1" -name '*.class' | sort | while IFS= read -r c; do javap -p -c "$c"; done | awk '
  /getstatic .*\.MODULE\$/ { s = $0; sub(/.*Field /, "", s); sub(/\$\.MODULE\$.*/, "", s); got = s; step = 1; next }
  step == 1 && / pop$/ { step = 2; next }
  step == 2 && /: new / { n = $0; sub(/.*\/\/ class /, "", n); if (n == got) count[got]++ }
  { step = 0 }
  END { for (k in count) print k, count[k] }' | sort
