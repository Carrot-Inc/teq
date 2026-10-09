#!/bin/bash
# tests/support/inline-census.sh <census file>...: what the definition check of inline bodies met
# (TEQ_INLINE_CENSUS), each body counted once however many builds met it (by its
# position and name): the bodies checked, failed by the first error, held back by form, and
# those whose record failed the census's check.
[ $# -gt 0 ] || { echo "usage: tests/support/inline-census.sh <census file>..." >&2; exit 2; }
bodies=$(cat "$@" | awk -F'\t' '!seen[$3 "\t" $4]++')
echo "$bodies" | awk -F'\t' 'NF { n[$1]++; total++ } END { printf "%d bodies: %d checked, %d failed, %d held back, %d records failed\n", total, n["checked"], n["failed"], n["held"], n["record"] }'
echo "$bodies" | awk -F'\t' 'NF && $1 != "checked" { print $1 "\t" $2 }' | sort | uniq -c | sort -rn
