#!/bin/bash
# tests/support/byname_ctor/list.sh <dir>: the fields bn.A's constructor writes other than the thunk's own, each with
# every read the value it writes makes, in order: `param` where the constructor loads its parameter (`aload_1`),
# `field` where it reads the thunk's field, `this` where a closure captures the instance, as `<field> <read>,...`, from
# javap's code of the class files under <dir> (scalac's for byname_ctor.expected, teq's in tests/run_jvm.sh).
javap -c -p -cp "$1" bn.A | awk '
  function note(k) { reads = reads (reads == "" ? "" : ",") k }
  /^  [^ ].*;$/ { ctor = ($0 ~ /bn\.A\(scala\.Function0(<[^)]*>)?\);$/); reads = "" }
  ctor && /aload_1/ { note("param") }
  ctor && /getfield .*Function0/ { note("field") }
  ctor && /invokedynamic .*\(Lbn\/A;\)/ { note("this") }
  ctor && /putfield/ { f = $0; sub(/.*Field /, "", f); sub(/:.*/, "", f); if (f !~ /(^|\$)s$/) print f " " reads; reads = "" }'
