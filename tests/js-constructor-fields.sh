#!/bin/bash
# Constructor-only private parameters need no field; accessors and later uses still do.
set -euo pipefail
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
out=out/js-constructor-fields
mkdir -p "$out"
for mode in dev release; do
  flags=()
  [ "$mode" = release ] && flags=(--release)
  (ulimit -v 4000000; timeout 600 "$TEQ" compiler build tests/cases/ctor_parameter_storage.scala --threads 1 "${flags[@]}" -o "$out/$mode.js")
  node "$out/$mode.js" > "$out/$mode.actual"
  diff -u tests/cases/ctor_parameter_storage.expected "$out/$mode.actual"
  if [ "$mode" = dev ]; then
    sed -n '/^class SeededCell /,/^}/p' "$out/$mode.js" > "$out/seeded-class.js"
    if grep -q 'this.seed =' "$out/seeded-class.js"; then
      echo 'FAIL: constructor-only seed is still stored in a field'
      exit 1
    fi
    for pair in 'PrivateValue start' 'ReadLater kept' 'MutableValue count' 'OtherReceiver number' 'LazyValue seed' 'PublicValue visible'; do
      read -r cls field <<< "$pair"
      if ! sed -n "/^class $cls /,/^}/p" "$out/$mode.js" | grep -q "this.$field ="; then
        echo "FAIL: $cls lost its retained $field field"
        exit 1
      fi
    done
  fi
done
echo 'constructor fields: 2 passed'
