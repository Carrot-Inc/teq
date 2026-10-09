#!/bin/bash
# tests/support/abi/expected.sh [name...]: writes tests/support/abi/<name>.expected from scalac 3.8.4's class files of
# <name>.scala (every program of the corpus without names): javap's layout of the members the program's `// abi:`
# line names, as listing.py lists them. tests/run_jvm.sh compares teq's class files of each program with it.
cd "$(dirname "$0")/../../.."
names=("$@")
[ ${#names[@]} -gt 0 ] || names=($(ls tests/support/abi/*.scala | xargs -n1 basename | sed 's/\.scala$//'))
work=$(mktemp -d "${TMPDIR:-/tmp}/teq-abi.XXXXXX")
trap 'rm -rf "$work"' EXIT
for name in "${names[@]}"; do
  src=tests/support/abi/$name.scala
  if ! timeout 300 scala-cli compile -S 3.8.4 --jvm system --server=false "$src" --compilation-output "$work/$name" > "$work/$name.log" 2>&1; then
    echo "$name: scalac refused it"; cat "$work/$name.log"; exit 1
  fi
  python3 tests/support/abi/listing.py "$work/$name" $(sed -n 's|^// abi: ||p' "$src") > "tests/support/abi/$name.expected"
  echo "$name: $(wc -l < "tests/support/abi/$name.expected") lines"
done
