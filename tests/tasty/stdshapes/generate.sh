#!/bin/bash
# tests/tasty/stdshapes/generate.sh [--check]: every key of a library member a body can select
# for a std definition (`TEQ_STD_SHAPES`), filtered by Check.scala to those scala-library 3.8.4,
# scalajs-library and scalajs-dom resolve, into src/tasty/write/std_shapes.txt, the writer's
# allowlist, and by Reflect.scala the reflection API's keys
# into src/tasty/write/std_reflect.txt; with --check compared with those files instead,
# printing the difference and failing on one. The inverse table's hand entries,
# src/tasty/write/std_inverse.txt, are checked in both modes: each lean key one the std gives and
# the allowlist refuses, each of scala-library's keys one the checker resolves (a compared class's
# comparisons with an Int among them), the appended forms (`varargs <E>`) exactly the Java varargs
# members the checker finds the lean std declaring without their varargs, and a widened form's
# shapes the same but for `Char` as `Int`.
cd "$(dirname "$0")/../../.."
teq=${TEQ:-./target/release/teq}
. tests/support/jars.sh
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
echo 'object StdShapes' > "$work/StdShapes.scala"
if ! TEQ_STD_SHAPES="$work/keys.txt" timeout 120 "$teq" compiler check "$work/StdShapes.scala" --products "$work/p" > "$work/teq.log" 2>&1; then
  echo "FAIL std shapes: teq listing the std's keys: $(head -3 "$work/teq.log")"
  exit 1
fi
cp="$(jar_of scalajs-library):$(jar_of scalajs-dom)"
inverse=src/tasty/write/std_inverse.txt
# scala-library's keys of the table: each entry's, its conversion's, and a compared class's comparisons with an Int.
grep -v '^#' "$inverse" | awk -F'\t' 'NF { print $4 "\t" $5 "\t" $6
  if ($7 ~ /^compared /) { split($7, v, " "); n = split("< <= > >= == !=", ops, " "); for (i = 1; i <= n; i++) print v[2] "\t" ops[i] "\tscala.Int:scala.Boolean" }
  else if ($7 != "-" && $7 !~ /^varargs / && $7 != "widened" && $7 != "receiver") { split($7, v, " "); print v[1] "\t" v[2] "\t" v[3] } }' > "$work/inverse.txt"
if ! COURSIER_MODE=offline timeout 300 scala-cli --power run -S 3.8.4 --jvm system --server=false --offline -q \
    tests/tasty/stdshapes/Check.scala --dep org.scala-lang:scala3-compiler_3:3.8.4 -- "$work/keys.txt" "$cp" "--inverse=$work/inverse.txt" > "$work/out.txt" 2> "$work/check.log"; then
  echo "FAIL std shapes: the checker: $(grep -v hint "$work/check.log" | head -3)"
  exit 1
fi
grep -v '^inverse	\|^varargs	' "$work/out.txt" > "$work/allowed.txt"
# The Java varargs members the lean std declares without their varargs, as Check.scala finds them,
# against the inverse table's appended forms (`varargs <E>`): the two the same.
grep '^varargs	' "$work/out.txt" | awk -F'\t' '{ print $2 "\t" $3 "\t" $4 "\t" $2 "\t" $3 "\t" $5 "\tvarargs " $6 }' | LC_ALL=C sort > "$work/varargs.found"
grep -v '^#' "$inverse" | awk -F'\t' '$7 ~ /^varargs / { print $1 "\t" $2 "\t" $3 "\t" $4 "\t" $5 "\t" $6 "\t" $7 }' | LC_ALL=C sort > "$work/varargs.listed"
# The reflection API's members, each with scala-library's through a `Quotes`'s `reflect`.
if ! COURSIER_MODE=offline timeout 300 scala-cli --power run -S 3.8.4 --jvm system --server=false --offline -q \
    tests/tasty/stdshapes/Reflect.scala --dep org.scala-lang:scala3-compiler_3:3.8.4 -- "$work/keys.txt" > "$work/reflect.txt" 2> "$work/reflect.log"; then
  echo "FAIL std shapes: the reflection API's checker: $(grep -v hint "$work/reflect.log" | head -3)"
  exit 1
fi
stale=$(grep -v '^#' "$inverse" | awk -F'\t' 'NF { print $1 "\t" $2 "\t" $3 }' | while IFS= read -r key; do
  grep -qxF "$key" "$work/keys.txt" || echo "the std gives no $key"
  grep -qxF "$key" "$work/allowed.txt" && echo "the allowlist has $key"
done; grep '^inverse	' "$work/out.txt" | grep -v '	resolves$' | sed 's/^inverse	/scala-library does not resolve /'
  LC_ALL=C comm -23 "$work/varargs.found" "$work/varargs.listed" | sed 's/^/a Java varargs member with no appended form: /'
  LC_ALL=C comm -13 "$work/varargs.found" "$work/varargs.listed" | sed 's/^/an appended form of no Java varargs member: /'
  grep -v '^#' "$inverse" | awk -F'\t' '$7 == "widened" { lean = $3; gsub(/scala\.Char/, "scala.Int", lean); if (lean != $6 || $2 != $5) print "a widened form whose shapes differ otherwise: " $1 "\t" $2 "\t" $3 }')
if [ -n "$stale" ]; then
  echo "FAIL std shapes: $inverse holds entries that do not stand:"
  printf '%s\n' "$stale" | head -10
  exit 1
fi
if [ "$1" = "--check" ]; then
  for f in std_shapes:allowed std_reflect:reflect; do
    if ! diff "$work/${f#*:}.txt" "src/tasty/write/${f%:*}.txt" > "$work/diff.txt"; then
      echo "FAIL std shapes: src/tasty/write/${f%:*}.txt is not what tests/tasty/stdshapes/generate.sh makes"
      head -20 "$work/diff.txt"
      exit 1
    fi
  done
else
  cp "$work/allowed.txt" src/tasty/write/std_shapes.txt
  cp "$work/reflect.txt" src/tasty/write/std_reflect.txt
fi
echo "std shapes: $(wc -l < "$work/allowed.txt" | tr -d ' ') of $(wc -l < "$work/keys.txt" | tr -d ' ') keys resolve, $(wc -l < "$work/reflect.txt" | tr -d ' ') of $(grep -c '^scala.quoted.Reflect\$' "$work/keys.txt") of the reflection API's through Quotes"

