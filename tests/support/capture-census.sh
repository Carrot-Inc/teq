#!/bin/bash
# tests/support/capture-census.sh <census file>...: the capture's census (TEQ_CAPTURE_CENSUS)
# summed over the builds that appended to the files: per construct
# the nodes whose typed form is derived, captured, missing and unsupported, and the pickled ones
# without a source position; the records the compactions dropped and the orphans left; a few
# places of every missing construct. Exits 1 when a construct is missing anywhere or an orphan
# is left.
[ $# -gt 0 ] || { echo "usage: tests/support/capture-census.sh <census file>..." >&2; exit 2; }
cat "$@" | LC_ALL=C awk -F'\t' '
  $1 == "row" { d[$2] += $3; c[$2] += $4; m[$2] += $5; u[$2] += $6; n[$2] += $7; rows[$2] = 1 }
  $1 == "missing" && places[$2] < 5 { at[$2] = at[$2] "\n    " $3 "  " $4 "  " $5; places[$2]++ }
  $1 == "orphans" { orphans += $2 }
  $1 == "discarded" { discarded += $2 }
  $1 == "unsolved" { unsolved += $2 }
  $1 == "orphan" && shown < 10 { orphan_at = orphan_at "\n    " $2 " " $3 " " $4; shown++ }
  END {
    printf "%-34s %10s %10s %8s %12s %10s\n", "construct", "derived", "captured", "missing", "unsupported", "unplaced"
    k = 0
    for (r in rows) names[k++] = r
    for (i = 0; i < k; i++) for (j = i + 1; j < k; j++) if (names[j] < names[i]) { t = names[i]; names[i] = names[j]; names[j] = t }
    missing = 0
    for (i = 0; i < k; i++) {
      r = names[i]
      printf "%-34s %10d %10d %8d %12d %10d\n", r, d[r], c[r], m[r], u[r], n[r]
      missing += m[r]
    }
    printf "discarded records %d, orphans %d, captured types with an unsolved variable %d\n", discarded, orphans, unsolved
    for (i = 0; i < k; i++) if (at[names[i]] != "") printf "missing %s:%s\n", names[i], at[names[i]]
    if (orphan_at != "") printf "orphans:%s\n", orphan_at
    exit (missing > 0 || orphans > 0) ? 1 : 0
  }'
