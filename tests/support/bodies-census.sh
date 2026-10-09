#!/bin/bash
# tests/support/bodies-census.sh <file>...: the census of the right-hand sides (TEQ_BODIES_CENSUS)
# of every build whose lines the files hold, summed: per producer the bodies
# written, per reason those left ELIDED, in the C locale's order, the same on every machine.
awk -F'\t' '$1 == "body" { n[$2] += $3 } END { for (k in n) printf "%-50s %10d\n", k, n[k] }' "$@" | LC_ALL=C sort
