#!/bin/bash
# tests/support/synchronized/list.sh <dir>: per method of sync.Blocks, the monitor instructions in order with the
# throws and returns around them (`monitorenter`, `monitorexit`, `athrow`, `*return`), then its exception table's
# entries by catch type, from javap's code of the class files under <dir> (scalac's for synchronized.expected, teq's
# in tests/run_jvm.sh): dotty's genSynchronized enters the monitor, exits it on the normal path, and protects the
# body by a catch-any handler that exits it and rethrows.
javap -c -p -cp "$1" sync.Blocks | awk '
  /^  [^ ].*\(.*\);$/ { m = $0; sub(/\(.*/, "", m); sub(/.* /, "", m); ops = ""; table = 0; next }
  /Exception table:/ { table = 1; next }
  table && /^ +[0-9]+ +[0-9]+ +[0-9]+ +/ { print m " handler " $4; next }
  /^    Code:/ { next }
  /: (monitorenter|monitorexit|athrow|[ailfd]?return)$/ { op = $0; sub(/.*: /, "", op); ops = ops (ops == "" ? "" : " ") op }
  /^$/ && m != "" && ops != "" { print m " " ops; ops = "" }
  END { if (ops != "") print m " " ops }'
