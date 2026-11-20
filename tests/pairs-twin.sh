#!/bin/bash
# The twin of a class's own pairs: the pairs the record enumerates whose overriding member is the class's own,
# against what the own pairs' walks record of them (the override check's pairs, the names, the accessor, the JVM's
# bridges), 0 apart. tests/support/pairs_twin.py says the rest.
#
# As a suite, tests/pairs-twin.sh [--update] [--add]: every program of tests/cases built once for JavaScript and once
# for the JVM by TEQ (default: the repository's release build) under the pairs' dump, a test a program. The programs
# expected to differ are tests/pending/pairs-twin.txt (tests/support/pending.py; --update and --add as it takes them),
# each with its first difference; the suite's row of the board is tests/board/suites/pairs-twin.tsv. Without
# scala-library's jar in the coursier cache nothing runs (counted not run).
#
# The proof's form, over tests/cases and the tables' one-run cells, the outputs kept:
#   tests/pairs-twin.sh --head <teq> --out <dir> [--cases] [--dir <d>]... [--jobs N] [--limit N]
cd "$(dirname "$0")/.." || exit 1
case " $* " in
  *" --head "*) exec python3 tests/support/pairs_twin.py "$@" ;;
esac
root=$PWD
TEQ=${TEQ:-./target/release/teq}
case $TEQ in /*) ;; *) TEQ=$root/$TEQ ;; esac
. tests/support/jars.sh
SL=$(jar_of scala-library)
update= add=
for arg in "$@"; do
  case $arg in
    --update) update=--update ;;
    --add) add=--add ;;
  esac
done
if [ ! -f "$SL" ]; then
  echo "skip: scala-library 3.8.4 is not in the coursier cache"
  echo "pairs-twin: 0 passed, 0 failed, $(python3 -c 'import sys; sys.path.insert(0, "tests/support"); import pairs_programs as pp; print(len(list(pp.case_programs(".", ["js", "jvm"]))))') not run"
  exit 0
fi
out=$(mktemp -d "${TMPDIR:-/tmp}/teq-pairs-twin.XXXXXX") || exit 2
trap 'rm -rf "${out:?}"' EXIT
python3 tests/support/pairs_twin.py --head "$TEQ" --out "$out/builds" --cases --sl "$SL" --results "$out/results.txt" --discard --jobs "${JOBS:-4}" > "$out/twin.txt"
code=$?
[ $code -ne 2 ] && [ -s "$out/results.txt" ] || { cat "$out/twin.txt"; echo "pairs-twin: the twin held nothing (exit $code)"; exit 2; }
tail -n 1 "$out/twin.txt"
python3 tests/support/pending.py pairs-twin "$out/results.txt" $update $add
