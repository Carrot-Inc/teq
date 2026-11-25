#!/bin/bash
# tests/right-assoc/run.sh [--update|--add] [--only <prefix>]...: the right-associative calls' cells against scalac
# 3.8.4's captured lines (tests/right-assoc/run.py), with TEQ (default ./target/release/teq) and scala-library's jar
# (tests/support/jars.sh). Writes under out/right-assoc.
cd "$(dirname "$0")/../.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
SL=$(scala_library_jar)
[ -f "$SL" ] || { echo "right-assoc: no scala-library jar"; exit 2; }
exec python3 tests/right-assoc/run.py "$TEQ" "$SL" out/right-assoc "$@"
