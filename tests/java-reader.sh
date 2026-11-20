#!/bin/bash
# tests/java-reader.sh [--sample N]: the Java reader's scanner and parser against broken and unusual sources.
# - ident: the characters an identifier may start with and go on with, src/java/ident.rs, against the JDK's own
#   Character over all 1,114,112 code points (tests/java-reader/ident/gen.py --check): 0 apart.
# - progress: from 300 of dotty's .java tests at the tag, some 4,700 inputs cut after every k-th token or with a
#   delimiter removed or doubled (tests/java-reader/corpus.py, seeded), and the 30 sources of
#   tests/java-reader/broken (one of them, a 5 MiB comment, written by the script), each checked at 1 and 8 workers
#   (tests/java-reader/progress.py): every one ends within two seconds, built or refused at a place (an input that is
#   not UTF-8: at its file), and none it builds is one scalac refuses, by tests/java-reader/judged.tsv (scalac
#   3.8.4's verdict, under a 512 MB stack, and master's for every input; the rows of a listed class counted apart,
#   each with its reached twin, a stale one failing). The
#   whole suite is some 17 seconds on a 16-core machine with the release build, and the gate runs it whole; --sample
#   N checks every N-th input.
# tests/java-reader/judge.py writes judged.tsv again (scalac over every input, master's binary and the head's), and
# fence.py (the records against the class files, every column failing) and claims.py (the import and name programs
# against scalac) run apart, by hand: some five, ten and three minutes; the gate runs none of them.
# Without javac, java, scala-library's jar or dotty's tests at the tag nothing runs (counted not run). TEQ names the
# binary (default: the repository's release build).
cd "$(dirname "$0")/.." || exit 1
root=$PWD
TEQ=${TEQ:-./target/release/teq}
case $TEQ in /*) ;; *) TEQ=$root/$TEQ ;; esac
. tests/support/jars.sh
SL=$(jar_of scala-library)
sample=1
[ "$1" = --sample ] && sample=$2
tag=${SCALA3_TAG:-$(dirname "${SCALA3:-$root/../scala3}")/scala3-3.8.4}
if ! command -v javac > /dev/null 2>&1 || ! command -v java > /dev/null 2>&1 || [ ! -f "$SL" ] || [ ! -d "$tag/tests" ]; then
  echo "skip: javac, java, scala-library 3.8.4 or dotty's tests at the tag is missing"
  echo "java-reader: 0 passed, 0 failed, 2 not run"
  exit 0
fi
out=$(mktemp -d "${TMPDIR:-/tmp}/teq-java-reader.XXXXXX") || exit 2
trap 'rm -rf "$out"' EXIT
pass=0
fail=0
if python3 tests/java-reader/ident/gen.py --check > "$out/ident.log" 2>&1; then pass=$((pass + 1)); echo "ok   $(tail -1 "$out/ident.log")"; else fail=$((fail + 1)); echo "FAIL ident"; tail -5 "$out/ident.log"; fi
if SCALA3_TAG=$tag python3 tests/java-reader/progress.py "$TEQ" "$out/progress" --sample "$sample" > "$out/progress.log" 2>&1; then
  pass=$((pass + 1)); echo "ok   $(tail -1 "$out/progress.log")"
else
  fail=$((fail + 1)); echo "FAIL progress"; tail -22 "$out/progress.log"
fi
echo "java-reader: $pass passed, $fail failed"
[ $fail = 0 ]
