#!/bin/bash
# tests/support/inline-counts.sh <counts file>...: the counts of the expansion by substitution
# (TEQ_INLINE_COUNTS=<file>) summed over the builds that appended to the files: the
# calls expanded by substitution and those left to the retype path, by reason, with the bridges
# taken inside a walk; the nodes the walks copied and visited, those copied then discarded and those of the parts a
# walk never demanded, left uncopied;
# the records with bodies the definition check made and the build kept, and the replaced ones
# a capture's census keeps.
[ $# -gt 0 ] || { echo "usage: tests/support/inline-counts.sh <counts file>..." >&2; exit 2; }
cat "$@" | LC_ALL=C awk '
  /^inline counts: / {
    builds++
    n = split($0, parts, "; ")
    split(parts[1], f, " ")
    sub_ += f[4]; fell += f[7]; bridges += f[9] + 0; copied += f[11]; visited += f[13]; discarded += f[15]; records += f[17]; retained += f[19]; superseded += f[21] + 0; pruned += f[23] + 0
    for (i = 2; i <= n; i++) {
      k = parts[i]; v = k
      sub(/: [0-9]+$/, "", k); sub(/.*: /, "", v)
      reason[k] += v
    }
  }
  END {
    printf "%d builds: %d calls expanded by substitution, %d left to the retype path (%d bridges inside a walk)\n", builds, sub_, fell, bridges
    printf "nodes copied %d, visited %d, copied then discarded %d, left uncopied %d; records made %d, kept %d, replaced ones kept for a capture %d\n", copied, visited, discarded, pruned, records, retained, superseded
    for (k in reason) printf "  %8d  %s\n", reason[k], k
  }' | { IFS= read -r a; IFS= read -r b; echo "$a"; echo "$b"; sort -rn; }
