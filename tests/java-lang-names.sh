#!/bin/bash
# tests/java-lang-names.sh [--update] [--add]: a java.lang class written in full (`java.lang.String`,
# `_root_.java.lang.Integer`, `import java.lang.{String => S}`, `classOf[java.lang.Thread]`, ...) is the class scalac
# 3.8.4 reads, on every target, and the user's own type of the simple name is not taken from it. The cells are
# tests/java-lang-names/cells.tsv: 74 java.lang classes in up to nineteen positions each (`scala.<Name>`,
# `Predef.<Name>` and the user's own type beside `import scala.*` and `import java.lang.*` among them), and
# `Predef.String` and `Predef.Class` written in full or not, each a program with scalac's output, written by
# tests/java-lang-names/gen.py from the JDK's class files and scalac's runs. A cell passes when JavaScript, JavaScript
# over scala-library, the interpreter and the JVM each print scalac's output, or each refuse a program scalac refuses
# (tests/java-lang-names/run.py).
# The cells expected to fail are tests/pending/java-lang-names.txt (tests/support/pending.py; --update and --add as it
# takes them), each with the leg that differs and scalac's answer; the suite's row of the board is
# tests/board/suites/java-lang-names.tsv. Without node, java or scala-library's jar in the coursier cache nothing runs
# (counted not run). TEQ names the binary (default: the repository's release build).
cd "$(dirname "$0")/.." || exit 1
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
cells=$(grep -vc '^#' tests/java-lang-names/cells.tsv)
if ! command -v node > /dev/null 2>&1 || ! command -v java > /dev/null 2>&1 || [ ! -f "$SL" ]; then
  echo "skip: node, java or scala-library 3.8.4 is missing"
  echo "java-lang-names: 0 passed, 0 failed, $cells not run"
  exit 0
fi
out=$(mktemp -d "${TMPDIR:-/tmp}/teq-java-lang-names.XXXXXX") || exit 2
trap 'rm -rf "$out"' EXIT
python3 tests/java-lang-names/run.py "$TEQ" "$SL" "$out" || exit 2
python3 tests/support/pending.py java-lang-names "$out/results.txt" $update $add
