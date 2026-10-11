#!/bin/bash
# tests/support/trait_inits/list.sh <dir>: per class of the fixture, the `$init$` calls of its methods, in order, as
# `<class> <method> <trait>`, from javap's code of the class files under <dir> (scalac's for trait_inits.expected,
# teq's in tests/run_jvm.sh).
for c in inits.LazyMix inits.Sub 'inits.Obj$'; do
  javap -c -p -cp "$1" "$c" | awk -v c="$c" '
    /^  [^ ].*;$/ { m = $0; sub(/\(.*/, "", m); n = split(m, w, " "); m = w[n]; if ($0 ~ /static \{\};$/) m = "<clinit>"; if (m == c) m = "<init>" }
    /invokestatic/ && /\$init\$/ { t = $0; sub(/.*InterfaceMethod /, "", t); sub(/\.\$init\$.*/, "", t); print c " " m " " t }'
done
