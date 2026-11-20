#!/bin/bash
# The equality of a class's pairs: master's JavaScript and class files against the head's, partitioned by the
# head's record of each class's pairs (`TEQ_PAIRS_DUMP`, `Worker::class_pairs`): a program whose classes have their
# own pairs alone is held to master byte for byte; one with an inherited pair to its value (master's, or the
# expectation's where master's is not), its changed outputs listed by the consumer that changed them (a bridge, an
# accessor, a default getter). tests/support/pairs_equality.py says the rest.
# No suite runs it: it needs master's binary beside the head's, which no suite has. It is a Ready's run, of a part
# that moves what the record decides, its tables in that Ready's proof; tests/pairs-twin.sh is the suite of the
# record's own half.
# With --claim <table dir> (repeated: the tables of the pairs' proof, each its cells and pairs.tsv or extra.tsv) it
# also holds the claim to scalac: every cell on every leg against scalac's recorded answer, master's accepted only
# in the rows tests/support/pairs_rows.tsv names (tests/support/pairs_claim.py, its output under <out>/claim).
# usage: tests/pairs-equality.sh --master <teq> --head <teq> --out <dir> [--cases] [--board <scala3>] [--dir <d>]...
#          [--claim <table dir>]... [--rows <file>] [--legs js,jvm,interp] [--jobs N]
cd "$(dirname "$0")/.." || exit 1
out= claim=
prev=
for a in "$@"; do
  [ "$prev" = --out ] && out=$a
  [ "$prev" = --claim ] && claim=1
  prev=$a
done
python3 tests/support/pairs_equality.py "$@"
code=$?
if [ -n "$claim" ]; then
  python3 tests/support/pairs_claim.py "$@" --out "$out/claim"
  c=$?
  [ $code -ne 0 ] || code=$c
fi
exit $code
