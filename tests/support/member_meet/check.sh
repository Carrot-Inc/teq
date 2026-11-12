#!/bin/bash
# The client is compiled against scalac's library, and is to link against teq's library.
# Usage: TEQ=/absolute/path/to/the/binary bash tests/support/member_meet/check.sh [output-dir]
set -euo pipefail
cd "$(dirname "$0")/../../.."
work=${1:-out/member-meet-link}
mkdir -p "$work"
work=$(cd "$work" && pwd)
export TMPDIR="$work/tmp"
mkdir -p "$TMPDIR"
export JAVA_TOOL_OPTIONS='-Xmx768m -XX:ReservedCodeCacheSize=128m -XX:CompressedClassSpaceSize=128m'
lib=${M2:-$HOME/.cache/coursier/v1/https/repo1.maven.org/maven2}/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar
source=tests/support/abi/member_meet.scala
client=tests/support/member_meet/Main.scala
bounded() { (ulimit -v 4000000; timeout 600 "$@"); }
bounded scala-cli compile -S 3.8.4 --jvm system --server=false "$source" --compilation-output "$work/sc-lib" > "$work/sc-lib.log" 2>&1
bounded scala-cli compile -S 3.8.4 --jvm system --server=false --classpath "$work/sc-lib" "$client" --compilation-output "$work/sc-client" > "$work/sc-client.log" 2>&1
bounded "${TEQ:?set TEQ to the compiler binary}" compiler build "$source" --target jvm --products "$work/teq-lib" --classpath "$lib" > "$work/teq-lib.log" 2>&1
for compiler in sc teq; do
  javap -p -s "$work/$compiler-lib/meetmin/Api\$.class" > "$work/$compiler.javap"
  python3 tests/support/abi/listing.py "$work/$compiler-lib" $(sed -n 's|^// abi: ||p' "$source") > "$work/$compiler.descriptors"
  diff -u tests/support/abi/member_meet.expected "$work/$compiler.descriptors"
  bounded java -cp "$work/sc-client:$work/$compiler-lib:$lib" meetmin.Main > "$work/$compiler.stdout" 2> "$work/$compiler.stderr"
  diff -u tests/support/member_meet/client.expected "$work/$compiler.stdout"
done
echo 'member meet: scalac descriptors and client linkage match'
