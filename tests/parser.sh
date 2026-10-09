#!/bin/bash
# The parser's recovery: the mutation suite over the
# frozen corpus of tests/parser (no hang, no crash, no swallowed definition, no wrong owner, no
# moved span but those tests/parser/exceptions.tsv allows, and no cascade above
# tests/parser/expect.tsv's), and the complete-list fixtures of tests/parser/fixtures against
# their recorded lists and their departures from scalac's. The last line says whether both passed.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
mkdir -p out
fail=0
python3 tests/parser/run.py --gate --teq "$TEQ" > out/parser-gate.log 2>&1 || fail=1
grep -v '^|' out/parser-gate.log
python3 tests/parser/run.py --fixtures --teq "$TEQ" || fail=1
if [ $fail = 0 ]; then echo "parser recovery tests passed"; else echo "parser recovery tests failed"; fi
exit $fail
