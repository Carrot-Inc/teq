#!/bin/bash
# tests/support/final_select/list.sh <dir>: per class of the fixture, the fields its constructor writes other than the
# parameter's own, each with every read of what it writes, in order: `param` where the constructor loads its parameter
# (`iload_1`), `accessor` where it reads the field or calls the accessor, as `<class> <field> <read>,...`, from javap's
# code of the class files under <dir> (scalac's for final_select.expected, teq's in tests/run_jvm.sh).
for c in A P F K Q O V; do
  javap -c -p -cp "$1" "fs.$c" | awk -v c="$c" '
    function note(k) { reads = reads (reads == "" ? "" : ",") k }
    /^  [^ ].*;$/ { ctor = ($0 ~ /fs\.[A-Za-z]+\(int\);$/); reads = "" }
    ctor && /iload_1/ { note("param") }
    ctor && (/getfield .*Field s:I/ || /invoke.*Method s:\(\)I/) { note("accessor") }
    ctor && /putfield/ { f = $0; sub(/.*Field /, "", f); sub(/:.*/, "", f); if (f != "s") print c " " f " " reads; reads = "" }'
done
