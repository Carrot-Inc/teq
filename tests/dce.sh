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
# all stay; a call through a class that declares a trait's member abstract again keeps none of the
# trait's default for an unrelated class of the trait.
out=out/dce/dispatch.js
run_js 20 $out tests/dce/dispatch.scala > out/dce/dispatch.actual 2>&1 || bad "dispatch build"
printf 'square 4\ncircle 3\nHELLO\nGOOD DAY\n[GOOD DAY]\n2\nloose\n' > out/dce/dispatch.expected
runs out/dce/dispatch.expected out/dce/dispatch.actual dispatch
absent $out '"shape"' '"sized default"'
present $out '"hello"' '"good day"' 'super.greet('

# Inline expansions of one shape: the step's body is written once, as the function its three
# sites call with their leaves (the branch a condition on the constant argument selected among
# them), and the class the renderer makes at three sites once.
out=out/dce/expansions.js
run_js 20 $out tests/dce/expansions.scala > out/dce/expansions.actual 2>&1 || bad "expansions build"
runs tests/dce/expansions.expected out/dce/expansions.actual expansions
once $out 'step body' '"rendered "' 'function step\$o'
present $out 'step\$o[0-9a-f]*("b", () => Tag\$().given_Tag_String, "many ", 2)'

# The same with the type class's member a val its givens implement: each site reads it on the given
# object its search found, typed at that object's class, and the step is still written once.
out=out/dce/expansions_val.js
run_js 20 $out tests/dce/expansions_val.scala > out/dce/expansions_val.actual 2>&1 || bad "expansions_val build"
runs tests/dce/expansions_val.expected out/dce/expansions_val.actual expansions_val
once $out 'step body' 'function step\$o'
present $out 'step\$o[0-9a-f]*("b", () => Tag\$().given_Tag_String, "many ", 2)'

# A jar's getter is a member of its pickle, which a body of the jar applies by name: the getter an
# override's parameter calls is that one, written once in the class.
out=out/dce/jar_getter_once.js
timeout 60 "$TEQ" compiler build tests/dce/jar_getter_once.scala --classpath "$(fixtures_jar)" -o $out > out/dce/jar_getter_once.log 2>&1 || bad "jar_getter_once build"
timeout 20 node $out > out/dce/jar_getter_once.actual 2>&1
runs tests/dce/jar_getter_once.expected out/dce/jar_getter_once.actual jar_getter_once
once $out 'send\$default\$2() {' 'send\$default\$3() {'

# Reflective instantiation costs nothing until a lookup is reached: an annotated class nothing
# names is dropped, and no registration or registry is written.
out=out/dce/reflect_unused.js
run_js 20 $out tests/dce/reflect_unused.scala > out/dce/reflect_unused.actual 2>&1 || bad "reflect_unused build"
echo object > out/dce/reflect_unused.expected
runs out/dce/reflect_unused.expected out/dce/reflect_unused.actual reflect_unused
absent $out 'Unnamed' '$reflect' '$qname'

# A map's `filter` and `tail` are its own members' calls: a class's `filter` and `tail` that nothing
# calls are dropped (the std writes those of the maps without a `super` call, which the reach answers
# with every class's member of the name).
out=out/dce/map_filter.js
run_js 20 $out tests/dce/map_filter.scala > out/dce/map_filter.actual 2>&1 || bad "map_filter build"
runs tests/dce/map_filter.expected out/dce/map_filter.actual map_filter
absent $out 'new Box(-1)' 'Box(this.x - 1'

# A concatenation's operand that is a conditional of two constants, typed by their union, is a String
# or an Int to the emitter (dotty's `widenUnion`): written as it is, with no conversion around it (the
# std's `Range.toString`, a conditional of strings and one of numbers).
out=out/dce/string_operands.js
run_js 20 $out tests/dce/string_operands.scala > out/dce/string_operands.actual 2>&1 || bad "string_operands build"
runs tests/dce/string_operands.expected out/dce/string_operands.actual string_operands
present $out 'this.start) + (this.isInclusive ? " to " : " until ")) + this.end)' '("item" + (flag ? "" : "s"))' '"code " + (flag ? 4 : 5)'
absent $out '$str(this.isInclusive' '$str(flag'

echo "$pass passed, $fail failed"
[ $fail = 0 ]
