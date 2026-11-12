#!/bin/bash
# tests/std-kinds.sh [--update] [--add]: the std's classes and objects are of the kinds scala-library 3.8.4
# declares them with. A rule of dotty's that asks whether a class is a trait, sealed or final (a type test's
# checkability, `TypeTestsCasts.typeArgsDeterminable`; a pattern's reachability) answers for a program by the
# std's declaration on JavaScript and in the interpreter and by scala-library's on the JVM, so the two have to
# agree. The std's side is teq's own symbol table: `TEQ_STD_KINDS` (src/tasty/write/pickle/shapes.rs, beside
# `TEQ_STD_SHAPES`) lists every class and object the std declares in a package, the nested ones and the
# companions, under scala-library's name, with its kind (trait, abstract class, class, object) and `sealed`,
# `final`, `case`. The library's side is scalac's: tests/tasty/stdshapes/Kinds.scala resolves each name on
# scalac 3.8.4's class path and reads the same of the symbol.
#
# A test is a name both sides declare as a class or an object (an object's name ends in `$`), and it passes when
# the kind and the three flags are the same. Outside the suite, and no test: the names the library does not have
# (the std's own internals); the std's opaque types, which have no class kind (the `scala.compiletime.ops` types,
# type members in the library, and `IArray` and the named tuples' two, opaque there too); the classes the compiler
# builds in and the std does not declare (the primitives, `AnyVal`, `Null`, `Array`, `AnyRef`, `Singleton`); and the
# std's classes of the JDK's and the Scala.js libraries' names, which scala-library does not declare.
#
# The tests expected to fail are tests/pending/std-kinds.txt (tests/support/pending.py; --update and --add as
# it takes them): the suite fails when a listed name agrees now and when an unlisted one differs, and when a name
# of its board list (tests/board/suites/std-kinds.tsv, every test of the last recorded run) is not compared.
# Without scala-cli or scalac 3.8.4 in the coursier cache it fails as incomplete validation.
cd "$(dirname "$0")/.." || exit 1
TEQ=${TEQ:-./target/release/teq}
. tests/support/pending.sh
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
pending_begin std-kinds "$work" "$@"
echo 'object StdShapes' > "$work/StdShapes.scala"
if ! TEQ_STD_SHAPES="$work/keys.txt" TEQ_STD_KINDS="$work/kinds.txt" timeout 120 "$TEQ" compiler check "$work/StdShapes.scala" --products "$work/p" > "$work/teq.log" 2>&1 || [ ! -s "$work/kinds.txt" ]; then
  echo "FAIL std-kinds: teq listing the std's kinds: $(head -3 "$work/teq.log")"
  exit 1
fi
if ! COURSIER_MODE=offline timeout 300 scala-cli --power run -S 3.8.4 --jvm system --server=false --offline -q \
    tests/tasty/stdshapes/Kinds.scala --dep org.scala-lang:scala3-compiler_3:3.8.4 -- "$work/kinds.txt" > "$work/out.txt" 2> "$work/check.log"; then
  echo "FAIL std-kinds: the checker (incomplete validation): $(grep -v hint "$work/check.log" | head -3)"
  exit 1
fi
grep -E '^(pass|fail) ' "$work/out.txt" >> "$PENDING_RESULTS"
if [ ! -s "$PENDING_RESULTS" ]; then
  echo "FAIL std-kinds: the checker compared no class: $(head -3 "$work/out.txt" "$work/check.log")"
  exit 1
fi
# A name the board's list holds that this run did not compare is a class one side no longer declares under it
# (or a listing cut short): the run is not the list's until --update or --add writes the list again.
if [ -z "$PENDING_FLAGS" ] && [ -f tests/board/suites/std-kinds.tsv ]; then
  gone=$(LC_ALL=C comm -23 <(grep -v '^#' tests/board/suites/std-kinds.tsv | cut -f1 | LC_ALL=C sort) <(cut -f1 "$PENDING_RESULTS" | cut -d' ' -f2- | LC_ALL=C sort))
  if [ -n "$gone" ]; then
    echo "FAIL std-kinds: $(grep -c . <<< "$gone") names of tests/board/suites/std-kinds.tsv were not compared: $(head -5 <<< "$gone" | tr '\n' ' ')"
    exit 1
  fi
fi
# A difference the list does not hold, with both sides' words.
while IFS=$'\t' read -r verdict why; do
  name=${verdict#fail }
  pending_listed "$name" || echo "FAIL $name: $why"
done < <(grep '^fail ' "$work/out.txt")
pending_end
