#!/bin/bash
# tests/tasty/annotations/check.sh <teq> <work dir>: the annotations' check. lib.scala's products,
# scalac 3.8.4's and teq's (the lean std's check build and the JVM build over scala-library), each
# read by scalac compiling inspect.scala and use.scala against
# them under -deprecation: what the inspector's macro reports of every annotation, the inherited
# `@org.junit.Test` methods it finds and the deprecation warnings of use.scala's uses are scalac's
# own over teq's pickles; and lib.scala's own pickles compiled to class files from TASTy alone
# under -deprecation warn of the uses of a deprecated member as scalac's own do (scalac registers
# a `@nowarn` in its typer, which a compilation from TASTy does not run: `quiet`'s use warns over
# its own pickles too). Prints a line per comparison, `ok` or
# `FAIL` with the difference; exits 1 on a failure.
teq=$1 work=$2
here=$(cd "$(dirname "$0")" && pwd)
cd "$here/../../.."
. tests/support/jars.sh
junit=$(jar_of junit)
sl=$(jar_of scala-library)
mkdir -p "$work"
status=0
cli() { COURSIER_MODE=offline timeout 300 scala-cli --power "$@" -S 3.8.4 --jvm system --server=false --offline -q; }
# scalac's own products, and teq's two.
rm -rf "$work"/scalac "$work"/lean "$work"/jvm "$work"/jvm-build
if ! cli compile --dep junit:junit:4.13.2 -d "$work/scalac" "$here/lib.scala" > "$work/scalac.log" 2>&1; then
  echo "FAIL annotations: scalac does not compile lib.scala: $(grep -v hint "$work/scalac.log" | head -3)"
  exit 1
fi
if ! timeout 60 "$teq" compiler check --products "$work/lean" "$here/lib.scala" --classpath "$junit" > "$work/lean.log" 2>&1; then
  echo "FAIL annotations: teq's check build of lib.scala: $(head -3 "$work/lean.log")"
  exit 1
fi
if ! timeout 60 "$teq" compiler build --target jvm --std=scala-library --products "$work/jvm-build" "$here/lib.scala" --classpath "$sl:$junit" > "$work/jvm.log" 2>&1; then
  echo "FAIL annotations: teq's JVM build of lib.scala: $(head -3 "$work/jvm.log")"
  exit 1
fi
# The JVM build's own products, without the std's runtime classes the directory also holds
# (`scala/runtime`), whose class files have no TASTy for scalac to read.
mkdir -p "$work/jvm" && cp -R "$work/jvm-build/annot" "$work/jvm/"
# What scalac reports compiling the downstream against products, its positions and colours left out.
report() {
  cli compile --dep junit:junit:4.13.2 --scalac-option -deprecation --scalac-option -color:never --classpath "$1" "$here/inspect.scala" "$here/use.scala" 2>&1 |
    grep -v 'hint\|outdated\|^Compiling\|^Compiled' | sed -E 's/^\[warn\] //; s/^.*use\.scala:[0-9]+:[0-9]+: ?//'
}
report "$work/scalac" > "$work/scalac.read"
grep -q '@Test methods: expecting, inherited, own, timed' "$work/scalac.read" || { echo "FAIL annotations: scalac's own read lacks the inherited @Test (see $work/scalac.read)"; status=1; }
for side in lean jvm; do
  report "$work/$side" > "$work/$side.read"
  if diff "$work/scalac.read" "$work/$side.read" > "$work/$side.diff"; then
    echo "ok annotations read: scalac reads teq's $side pickles' annotations and warns of their uses as of its own"
  else
    echo "FAIL annotations read: teq's $side pickles (< scalac's, > teq's):"
    head -20 "$work/$side.diff"
    status=1
  fi
done
# lib.scala's own pickles compiled from TASTy alone: the deprecation warnings of its bodies.
regen() {
  local out=$work/regen-$1
  rm -rf "$out" && mkdir -p "$out"
  printf 'warn\t%s\t%s\t%s\t%s\n' "$1" "$2:$junit" "$out" "$(find "$2" -name '*.tasty' | LC_ALL=C sort | tr '\n' '\t' | sed 's/\t$//')" > "$work/regen-$1.jobs"
  (cd "$work" && COURSIER_MODE=offline timeout 300 scala-cli --power run -S 3.8.4 --jvm system --server=false --offline -q \
    "$here/Regen.scala" --dep org.scala-lang:scala3-compiler_3:3.8.4 -- "regen-$1.jobs" 2>&1 | grep -v 'hint\|outdated')
}
regen scalac "$work/scalac" > "$work/scalac.regen"
loud=$(grep -n 'def loud' "$here/lib.scala" | cut -d: -f1)
grep -q "^$loud: method old in class Lib is deprecated since 1.0: use neu" "$work/scalac.regen" || { echo "FAIL annotations: scalac's own regeneration does not warn of loud (see $work/scalac.regen)"; status=1; }
for side in lean jvm; do
  regen "$side" "$work/$side" > "$work/$side.regen"
  if diff "$work/scalac.regen" "$work/$side.regen" > "$work/$side.regen.diff"; then
    echo "ok annotations regen: teq's $side pickles compiled from TASTy warn as scalac's own"
  else
    echo "FAIL annotations regen: teq's $side pickles (< scalac's, > teq's):"
    head -10 "$work/$side.regen.diff"
    status=1
  fi
done
exit $status
