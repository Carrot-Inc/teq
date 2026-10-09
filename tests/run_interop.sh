#!/bin/bash
# JS interop is not Scala, so these cases cannot be checked against scala-cli: every
# tests/interop/<name>.scala (or directory) has a hand-checked <name>.expected file.
# With a <name>.harness.mjs next to it the program is only built and the harness, which imports
# ../../out/interop/<name>.mjs, is what runs under node; otherwise the program runs itself. With a
# <name>.check file in place of the .expected the program is only type checked, and that file holds
# the expected diagnostics (empty for a program that is accepted). A case that names
# scala.scalajs is compiled together with tests/interop/scalajs-stub, a minimal stand-in for the
# Scala.js library written in the facade syntax. A `// jars: <names>` line puts those jars on the
# class path (tests/support/jars.sh); without one of them in the coursier cache the case counts as
# passed.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
STUB=tests/interop/scalajs-stub
mkdir -p out/interop
pass=0
fail=0
check() {
  if diff -q "$1" "$2" > /dev/null; then
    pass=$((pass + 1))
  else
    echo "FAIL $3"
    diff "$1" "$2" | head -${DIFF_LINES:-10}
    fail=$((fail + 1))
  fi
}
for src in tests/interop/*.scala tests/interop/*/; do
  src=${src%/}
  name=$(basename "$src" .scala)
  [ "$src" = "$STUB" ] && continue
  extra=""
  if grep -rq 'scala\.scalajs' "$src" && ! grep -rq '^package scala\.scalajs' "$src"; then
    extra=$STUB
  fi
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    echo "skip $name: not in the coursier cache:$JARS_MISSING"
    pass=$((pass + 1))
    continue
  fi
  [ -n "$JARS_CP" ] && extra="$extra --classpath $JARS_CP"
  actual="out/interop/$name.actual"
  if [ -f "tests/interop/$name.check" ]; then
    timeout 20 "$TEQ" compiler check "$src" $extra > "$actual" 2>&1
    check "tests/interop/$name.check" "$actual" "$name"
    continue
  fi
  # The same extension teq picks by default: a program with imports or exports is an ES module.
  ext=js
  grep -rqE '@(jsImport|jsExport|JSImport|JSExportTopLevel)\(' "$src" && ext=mjs
  out="out/interop/$name.$ext"
  harness="tests/interop/$name.harness.mjs"
  if [ -f "$harness" ]; then
    timeout 20 "$TEQ" compiler build "$src" $extra -o "$out" > "$actual" 2>&1 && timeout 20 node "$harness" >> "$actual" 2>&1
  else
    run_js 20 "$out" "$src" $extra > "$actual" 2>&1
  fi
  check "tests/interop/$name.expected" "$actual" "$name"
done

# Without -o the kind of output decides the file name, out/main.js or out/main.mjs for a module, which
# node then runs as such by the extension.
root=$PWD
case $TEQ in /*) teq=$TEQ ;; *) teq=$root/$TEQ ;; esac
for name in imports jsvalues; do
  dir=$(mktemp -d)
  ext=js
  [ $name = imports ] && ext=mjs
  # The build and node on the file it wrote under one bound, as the raw `teq run` that did both was.
  (cd "$dir" && timeout 20 bash -c '"$0" compiler build "$1" && exec node "$2"' "$teq" "$root/tests/interop/$name.scala" "out/main.$ext" > actual 2>&1; ls out >> actual)
  (cat "tests/interop/$name.expected"; echo "main.$ext") > "$dir/expected"
  check "$dir/expected" "$dir/actual" "$name with the default output path"
  rm -rf "$dir"
done
# Shape of the output: bindings of the same (module, name) share one import statement, and a
# program without imports or exports is the same plain script as ever.
expect() {
  if [ "$2" = "$3" ]; then
    pass=$((pass + 1))
  else
    echo "FAIL $1: expected $3, found $2"
    fail=$((fail + 1))
  fi
}
expect "import statements of multi" "$(grep -c '^import ' out/interop/multi.mjs)" 3
expect "import statements of jsvalues" "$(grep -cE '^(import|export) ' out/interop/jsvalues.js)" 0
expect "first line of jsvalues" "$(head -1 out/interop/jsvalues.js)" '"use strict";'
expect "first line of imports" "$(head -1 out/interop/imports.mjs)" 'import { join as $imp0 } from "node:path";'
expect "export statement of exports" "$(grep -c '^export { \$exp0 as version, ' out/interop/exports.mjs)" 1
echo "$pass passed, $fail failed"
[ $fail = 0 ]
