#!/usr/bin/env bash
# The methods of lvr.Use$ in the class files under $1 (and the class path $2), each with the number of calls it
# makes of the library's members (givenfold's, scala.math's `max`, String's): one line a method, from javap.
javap -p -c -classpath "$1${2:+:$2}" 'lvr.Use$' | awk '
  /^  [a-z].*\(/ { if (name != "") print name, n; name = $0; sub(/\(.*/, "", name); sub(/.* /, "", name); n = 0; next }
  /invoke[a-z]* .*Method (givenfold\/|scala\/math\/package\$\.max|java\/lang\/String\.)/ { n++ }
  END { if (name != "") print name, n }' | grep '^[A-Za-z0-9]* ' | grep -v '^writeReplace ' | sort
