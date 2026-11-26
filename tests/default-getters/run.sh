#!/bin/bash
# tests/default-getters/run.sh [--update|--add] [--only <prefix>]... [--jobs N]: the default getters' cells against
# scalac 3.8.4's captured lines (tests/default-getters/run.py), with TEQ (default ./target/release/teq) and
# scala-library's jar (tests/support/jars.sh). Writes under out/default-getters.
cd "$(dirname "$0")/../.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
SL=$(scala_library_jar)
[ -f "$SL" ] || { echo "default-getters: no scala-library jar"; exit 2; }
exec python3 tests/default-getters/run.py "$TEQ" "$SL" out/default-getters "$@"
