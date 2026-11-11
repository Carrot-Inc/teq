#!/bin/bash
# A loaded inline body's match over a selector ascribed with its own singleton type prints what
# the whole program prints (Typer.typedMatch's `otherTpe.widen`), built for JavaScript and run in
# the interpreter.
set -euo pipefail
teq=${TEQ:-./target/release/teq}
out=${TEQ_TEST_OUT:-out/classpath/loaded-selector-singleton}
case_dir=tests/classpath/loaded_selector_singleton
mkdir -p "$out"
compile() { (ulimit -v 4000000; timeout -k 10 120 "$teq" "$@"); }
compile compiler check --products "$out/upstream" "$case_dir/a"
for mode in whole split; do
  if [ "$mode" = whole ]; then
    inputs=("$case_dir/a" "$case_dir/b")
  else
    inputs=(--classpath "$out/upstream" "$case_dir/b")
  fi
  compile compiler build -o "$out/$mode.js" "${inputs[@]}"
  timeout -k 10 30 node "$out/$mode.js" > "$out/$mode.actual"
  diff -u "$case_dir/expected" "$out/$mode.actual"
  compile interp "${inputs[@]}" > "$out/$mode.interp"
  diff -u "$case_dir/expected" "$out/$mode.interp"
done
