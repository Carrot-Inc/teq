#!/bin/bash
# Compile-time evaluation of inline arguments: tests/cases/inline_fold_std.scala is built and
# its JavaScript searched for the constants the interpreter folded and for the calls that
# have to stay (an effect, a computation over the budget).
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
mkdir -p out/fold
pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() {
  echo "FAIL $1"
  fail=$((fail + 1))
}
js=out/fold/inline_fold_std.js
if ! timeout 20 "$TEQ" compiler build tests/cases/inline_fold_std.scala -o "$js" > out/fold/build.log 2>&1; then
  bad "build: $(tail -1 out/fold/build.log)"
else
  for text in 'println(3 + 3 | 0)' 'println(6 + 6 | 0)' '"aaa-4/5" + "!"' 'println("yes")' '"n=" + 3' 'println("four")' '"8" + "!"' 'bump() + bump()' 'counter$()' '.sum() + ' '0 + 0 | 0'; do
    if grep -qF -- "$text" "$js"; then ok; else bad "the output lacks $text"; fi
  done
  for text in '"ab".length' 'ArraySeq([1, 2, 3])' 'startsWith' '$split("x,y,z"' 'Math.sqrt' 'Vector$().apply(' 'Option$().apply(7)' 'Map$().apply(' 'new Random(3n)'; do
    if grep -qF -- "$text" "$js"; then bad "the output still holds $text"; else ok; fi
  done
fi
# tests/cases/fold_targets.scala with and without the evaluation (TEQ_NO_FOLD), on JavaScript
# and on the JVM: the four outputs have to agree, and the values the evaluation must not fold
# (a string made from a Double, a hash code) have to stay computations in the JavaScript.
src=tests/cases/fold_targets.scala
outputs=()
for target in js jvm; do
  for mode in folded plain; do
    out=out/fold/targets-$target-$mode
    if [ $mode = plain ]; then export TEQ_NO_FOLD=1; else unset TEQ_NO_FOLD; fi
    if [ $target = js ]; then
      built=$(run_js 60 "$out.js" "$src" 2>&1)
    else
      built=$(run_jvm 120 "$out.jar" --build "$src" --target jvm 2>&1)
    fi
    code=$?
    unset TEQ_NO_FOLD
    if [ $code -ne 0 ]; then
      bad "fold_targets on $target ($mode): $(tail -1 <<< "$built")"
      continue
    fi
    printf '%s\n' "$built" > "$out.txt"
    outputs+=("$out.txt")
  done
done
if [ ${#outputs[@]} = 4 ]; then
  if diff -q "${outputs[0]}" "${outputs[1]}" > /dev/null && diff -q "${outputs[0]}" "${outputs[2]}" > /dev/null && diff -q "${outputs[0]}" "${outputs[3]}" > /dev/null; then ok; else
    bad "fold_targets prints differently with and without the evaluation, or between the targets"
    diff "${outputs[0]}" "${outputs[1]}" | head -5
    diff "${outputs[0]}" "${outputs[2]}" | head -5
  fi
  js=out/fold/targets-js-folded.js
  for text in 'println(4 + 4 | 0)' '"[" + "1,2"' 'println("yes")' '$dstr(1.4142135623730951)'; do
    if grep -qF -- "$text" "$js"; then ok; else bad "fold_targets lacks $text"; fi
  done
  for text in '"0.3333333333333333"' '"2.5"' '"1.5;2.5"' '"0.30000000000000004"' '"96354"' '"1.5"'; do
    if grep -qF -- "$text" "$js"; then bad "fold_targets folded $text"; else ok; fi
  done
fi
# `!` of a constant is the constant, as scalac folds it: `LinkingInfo.developmentMode` under
# --release is `false`, which a minifier's dead-code pass removes with its branch.
js=out/fold/linking_info_release.js
if ! timeout 20 "$TEQ" compiler build tests/cases/linking_info_release.scala --release -o "$js" > out/fold/linking.log 2>&1; then
  bad "linking_info_release: $(tail -1 out/fold/linking.log)"
elif grep -qF -- '!true' "$js" || ! grep -qF -- 'if (false)' "$js"; then
  bad "linking_info_release keeps !true or lacks if (false)"
else
  ok
fi
# `&` and `|` of a Boolean evaluate both operands (tests/cases/boolean_strict_operators): the
# JavaScript keeps the operator, as a Boolean, next to a constant operand, a condition folds
# over constants alone, and the right operand of `|` is no tail position.
js=out/fold/boolean_strict_operators.js
if ! timeout 20 "$TEQ" compiler build tests/cases/boolean_strict_operators -o "$js" > out/fold/boolean.log 2>&1; then
  bad "boolean_strict_operators: $(tail -1 out/fold/boolean.log)"
else
  for pair in 'effect("true or")|!!(true | ' 'effect("false and")|!!(false & ' 'effect("K.T or")|!!(true | ' \
    'effect("if not and")|if (!(f & ' 'effect("while "|while (!!((i < 2) & ' 'rec(n - 1|!!((n === 0) | rec('; do
    if grep -F -- "${pair%%|*}" "$js" | grep -qF -- "${pair#*|}"; then ok; else bad "boolean_strict_operators lacks ${pair#*|}"; fi
  done
  if grep -qF -- 'println("folded")' "$js" && ! grep -qF -- '"kept"' "$js"; then ok; else bad "boolean_strict_operators keeps inline if true | false"; fi
fi
echo "$pass passed, $fail failed"
[ $fail = 0 ]
