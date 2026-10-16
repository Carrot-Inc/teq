#!/usr/bin/env bash
# The methods of ib.Use$ in the class files under $1, each with the number of times it loads the
# companion ib.C$, the mirror an inline call's binding holds: one line a method, from javap.
javap -p -c -classpath "$1" 'ib.Use$' | awk '
  /^  [a-z].*\(/ { if (name != "") print name, n; name = $0; sub(/\(.*/, "", name); sub(/.* /, "", name); n = 0; next }
  /getstatic .*ib\/C\$\.MODULE\$/ { n++ }
  END { if (name != "") print name, n }' | grep '^[A-Za-z0-9]* ' | grep -v '^writeReplace ' | sort
