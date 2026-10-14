#!/bin/bash
# tests/support/anon_final/list.sh <dir>: the methods of the anonymous classes under <dir>, each class named by
# the trait it implements (`anon(<trait>)`, the names scalac and teq give it apart, as is the package a quote's
# copy is made in), with their descriptors and flags as tests/support/abi/listing.py reads them: scalac's for
# anon_final.expected, teq's in tests/run_jvm.sh.
python3 "$(dirname "$0")/../abi/listing.py" "$1" | awk '
  $1 ~ /\$\$anon\$/ && $2 == "implements" { name[$1] = "anon(" $3 ")"; next }
  $1 ~ /\$\$anon\$/ && $2 != "class" { lines[NR] = $0; owner[NR] = $1 }
  END { for (i in lines) { l = lines[i]; sub(/^[^ ]+/, name[owner[i]], l); print l } }' | LC_ALL=C sort
