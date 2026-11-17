#!/bin/bash
# The parser's recovery: the mutation suite over the
# frozen corpus of tests/parser (no hang, no crash, no swallowed definition, no wrong owner, no
# moved span but those tests/parser/exceptions.tsv allows, and no cascade above
# tests/parser/expect.tsv's), and the complete-list fixtures of tests/parser/fixtures against
# their recorded lists and their departures from scalac's, held to the list of the fixtures expected to fail
# (tests/pending/parser-fixtures.txt; --update and --add as tests/support/pending.py takes them), and the layout
# programs (tests/parser/layout.py: generated programs that print which reading a layout got, each accepted with its
# line or refused as tests/parser/layout.tsv records, scalac's verdict beside it). The last line says whether all
# passed.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
mkdir -p out
fail=0
python3 tests/parser/run.py --gate --teq "$TEQ" > out/parser-gate.log 2>&1 || fail=1
grep -v '^|' out/parser-gate.log
python3 tests/parser/run.py --fixtures --teq "$TEQ" "$@" || fail=1
python3 tests/parser/layout.py --teq "$TEQ" || fail=1
if [ $fail = 0 ]; then echo "parser recovery tests passed"; else echo "parser recovery tests failed"; fi
exit $fail
