#!/bin/bash
# The dead-code pass: the output holds what the entry point or the exports reach and nothing
# else. Each case under tests/dce is built, run under node against its .expected file, and then
# searched for the JS names of definitions that have to be present or absent.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
mkdir -p out/dce
pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() {
  echo "FAIL $1"
  fail=$((fail + 1))
}
absent() {
  local f=$1
  shift
  for name in "$@"; do
    if grep -q -- "$name" "$f"; then bad "$f still holds $name"; else ok; fi
  done
}
present() {
  local f=$1
  shift
  for name in "$@"; do
    if grep -q -- "$name" "$f"; then ok; else bad "$f lacks $name"; fi
  done
}
once() {
  local f=$1
  shift
  for text in "$@"; do
    local n
    n=$(grep -o -- "$text" "$f" | wc -l | tr -d ' ')
    if [ "$n" = 1 ]; then ok; else bad "$f holds $text $n times"; fi
  done
}
runs() {
  if diff "$1" "$2" > /dev/null; then ok; else
    bad "$3 output"
    diff "$1" "$2" | head -${DIFF_LINES:-10}
  fi
}

# Hello world: printing and the entry point, none of the collections, regexes or the formatter.
out=out/dce/hello.js
timeout 20 "$TEQ" compiler build tests/dce/hello.scala -o $out > out/dce/hello.log 2>&1 || bad "hello build"
echo hello > out/dce/hello.expected
timeout 20 node $out > out/dce/hello.actual 2>&1
runs out/dce/hello.expected out/dce/hello.actual hello
size=$(wc -c < $out)
if [ "$size" -lt 6000 ]; then ok; else bad "hello is $size bytes"; fi
absent $out 'class List' 'class Vector' '$HMap' '$reTranslate' '$fmtOne' '$seqPat'
present $out '$println' '$main'

# Unreached defs, methods, objects, classes, enum cases and a file of vals are dropped; a class
# that only a type test names, the cases behind `values` and the sibling vals of a read val stay.
out=out/dce/unused.js
run_js 20 $out tests/dce/unused > out/dce/unused.actual 2>&1 || bad "unused build"
runs tests/dce/unused.expected out/dce/unused.actual unused
absent $out shout unusedScale NeverMade whatever UnusedObj unusedMethod 'talk(' Blue Green unusedFun deadVal 'dead.scala initialised'
present $out 'class Marker' 'instanceof Marker' '"North"' '"West"' 'sibling initialised' norm1 'greet(' 'helper('

# A library: the exports keep their closure, and only the import bindings that closure uses are
# emitted; the other file's initialiser is not.
out=out/dce/library.mjs
timeout 20 "$TEQ" compiler build tests/dce/library -o $out > out/dce/library.log 2>&1 || bad "library build"
if [ "$(grep -c '^import ' $out)" = 2 ]; then ok; else bad "library imports: $(grep '^import ' $out)"; fi
absent $out 'node:os' unusedHelper 'unusedTop initialised' platform
present $out 'from "node:path"' 'describe('
echo 'a.txt (.txt) 3' > out/dce/library.expected
timeout 20 node --input-type=module -e "import { describe, count } from '$PWD/$out'; console.log(describe('/tmp/a.txt') + ' ' + count);" \
  > out/dce/library.actual 2>&1
runs out/dce/library.expected out/dce/library.actual library

# Class inheritance: a reached subclass keeps its superclass with the constructor and the body,
# and a method of the superclass that only `super` calls; a subclass nothing creates goes, as do
# the members nothing calls.
out=out/dce/inherit.js
run_js 20 $out tests/dce/inherit > out/dce/inherit.actual 2>&1 || bad "inherit build"
runs tests/dce/inherit.expected out/dce/inherit.actual inherit
absent $out NeverBuilt neverBuiltSound NeverExtendedBase lonelyMember unusedVehicleMethod unusedTraitMethod
present $out 'class Car extends Vehicle' 'super(4)' '"rumble"' 'super.sound()' 'vehicle with' 'hum('
# Overloaded methods: an alternative that nothing calls is dropped, in the class, below the
# trait and at the top level.
out=out/dce/overloads.js
run_js 20 $out tests/dce/overloads.scala > out/dce/overloads.actual 2>&1 || bad "overloads build"
printf 'text a\nn1\nabc\n' > out/dce/overloads.expected
runs out/dce/overloads.expected out/dce/overloads.actual overloads
absent $out 'send$Int' 'send$Boolean' 'send$String$Int' 'encode$String' 'encode$Int$String' 'shorten$Int' 'shorten$List'
present $out 'send$String(' 'encode$Int(' 'shorten$String('

# Dispatch: the default of an abstract class that every instantiated class overrides is dropped,
# a trait's default that a class inherits stays, and the definitions a `super` call goes through
# all stay.
out=out/dce/dispatch.js
run_js 20 $out tests/dce/dispatch.scala > out/dce/dispatch.actual 2>&1 || bad "dispatch build"
printf 'square 4\ncircle 3\nHELLO\nGOOD DAY\n[GOOD DAY]\n' > out/dce/dispatch.expected
runs out/dce/dispatch.expected out/dce/dispatch.actual dispatch
absent $out '"shape"'
present $out '"hello"' '"good day"' 'super.greet('

# Inline expansions of one shape: the step's body is written once, as the function its three
# sites call with their leaves (the branch a condition on the constant argument selected among
# them), and the class the renderer makes at three sites once.
out=out/dce/expansions.js
run_js 20 $out tests/dce/expansions.scala > out/dce/expansions.actual 2>&1 || bad "expansions build"
runs tests/dce/expansions.expected out/dce/expansions.actual expansions
once $out 'step body' '"rendered "' 'function step\$o'
present $out 'step\$o[0-9a-f]*("b", () => Tag\$().given_Tag_String, "many ", 2)'

# Reflective instantiation costs nothing until a lookup is reached: an annotated class nothing
# names is dropped, and no registration or registry is written.
out=out/dce/reflect_unused.js
run_js 20 $out tests/dce/reflect_unused.scala > out/dce/reflect_unused.actual 2>&1 || bad "reflect_unused build"
echo object > out/dce/reflect_unused.expected
runs out/dce/reflect_unused.expected out/dce/reflect_unused.actual reflect_unused
absent $out 'Unnamed' '$reflect' '$qname'

echo "$pass passed, $fail failed"
[ $fail = 0 ]
