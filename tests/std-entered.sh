#!/bin/bash
# The std files a program enters are those it names, as dotty's package lookup completes a member by
# its name (`SymDenotations.computeMembersNamed`): a std val or alias the program does not name enters
# no file (the prelude's deprecated `Stream`, typed by its first reader, `Worker::std_val_body`). Read
# from `--profile`'s `std:` line. Scenarios: an empty object enters the files the typer needs of every
# program, exactly (core.scala, prelude.scala, enum.scala, evidence.scala, compiletime_ops.scala,
# javalib/serializable.scala, and deriving.scala and mirrors.scala for the mirrors, `scala.deriving.Mirror`
# and `scala.runtime.ProductMirror`, as master enters them); one naming `Stream` enters stream.scala; one naming `NumericRange`
# enters numeric_range.scala and not stream.scala; one naming `Ordering` enters `java.util.Comparator`,
# its parent, from javalib/comparator.scala, a file of its own as dotty reads the interface's class
# file alone, not with the rest of java.util.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
pass=0
fail=0
# The files a check of the program enters, sorted, one a line.
entered() {
  "$TEQ" compiler check "$1" --profile 2>&1 | grep -o '^std: .*' | sed -E 's/^std: [0-9]+ of [0-9]+ files entered( \([0-9]+ while typing\))?: //; s/ \(as [^)]*\)//g' | tr ',' '\n' | sed 's/^ *//' | grep -v '^$' | sort
}
check() {
  local what=$1 got=$2 want=$3
  if [ "$got" = "$want" ]; then
    pass=$((pass + 1))
  else
    echo "FAIL $what: $(echo "$got" | tr '\n' ' ')"
    fail=$((fail + 1))
  fi
}
echo 'object Models {}' > "$work/empty.scala"
every=$(printf '%s\n' compiletime_ops.scala core.scala deriving.scala enum.scala evidence.scala javalib/serializable.scala mirrors.scala prelude.scala)
check "an empty object enters the typer's own files alone" "$(entered "$work/empty.scala")" "$every"
printf '@deprecated("uses Stream", "")\nobject S:\n  val s = Stream(1, 2)\n' > "$work/stream.scala"
check "a program naming Stream enters stream.scala" "$(entered "$work/stream.scala" | grep -x 'stream.scala')" "stream.scala"
printf 'object R:\n  val r = scala.collection.immutable.NumericRange(1L, 5L, 2L)\n' > "$work/range.scala"
got=$(entered "$work/range.scala")
check "a program naming NumericRange enters numeric_range.scala" "$(echo "$got" | grep -x 'numeric_range.scala')" "numeric_range.scala"
check "a program naming NumericRange enters no stream.scala" "$(echo "$got" | grep -x 'stream.scala')" ""
printf 'object O:\n  val c = Ordering.Int.compare(1, 2)\n' > "$work/ordering.scala"
check "a program naming Ordering enters Comparator from javalib/comparator.scala" "$("$TEQ" compiler check "$work/ordering.scala" --profile 2>&1 | grep '^std: ' | grep -o 'javalib/[a-z_]*\.scala (as java\.util\.Comparator)')" "javalib/comparator.scala (as java.util.Comparator)"
echo "std-entered: $pass passed, $fail failed"
[ $fail = 0 ]
