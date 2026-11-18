#!/bin/bash
# The initialization checker's view (src/init/) held to scalac's: each program of tests/init-view/programs is
# compiled by scalac 3.8.4 FIRST, through tests/init-view/InitDump.scala (initdump.py: the trees after `pickler` and
# after `initChecker`, its header has the format), then by teq into products, whose view `TEQ_INIT_VIEW=<root> teq tasty`
# prints in the same format; the program passes where the two after-`pickler` dumps' POSITIONS are equal line for line
# (tests/init-view/compare.py --positions: the unit and source lines, each tree's depth, kind and position), which is
# what a product's TASTy gives its trees. The rest of each line (the classes collected, the symbols, what the checker
# reads) is the view's and judged when the view lands; the checker's own phase (after `initChecker`) is its second
# part's.
#
# scalac's dumps are kept under out/init-view/oracle/<program>-<digest>/, the digest over the program's sources and
# InitDump.scala, and made again only when either changes. The programs expected to fail are
# tests/pending/init-view.txt (tests/support/pending.sh: --update, --add).
#
# Env: TEQ (default ./target/release/teq); COURSIER (the coursier launcher initdump.py resolves scalac with, `cs`).
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
TEQ=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
. tests/support/pending.sh
work=out/init-view
mkdir -p "$work/oracle" "$work/teq"
pending_begin init-view "$work" "$@"
pass=0
fail=0
skipped=0
# scalac 3.8.4 runs through coursier and a JVM (initdump.py): without either, a program whose dump is not kept is not
# judged, NOT RUN and never passed (tests/support/pending.sh's skip).
runnable=1
command -v "${COURSIER:-cs}" > /dev/null 2>&1 || runnable=0
command -v java > /dev/null 2>&1 || runnable=0
for dir in tests/init-view/programs/*/; do
  name=$(basename "$dir")
  files=("$dir"*.scala)
  digest=$(cat "${files[@]}" tests/init-view/InitDump.scala tests/init-view/initdump.py | sha256sum | cut -c1-16)
  oracle="$work/oracle/$name-$digest"
  if [ ! -f "$oracle/exit" ] && [ $runnable = 0 ]; then
    pending_skip "$name" "scalac 3.8.4 not available: no coursier (${COURSIER:-cs}) or java"
    skipped=$((skipped + 1))
    continue
  fi
  if [ ! -f "$oracle/exit" ]; then
    python3 tests/init-view/initdump.py --out "$oracle" "${files[@]}" > /dev/null 2>&1
  fi
  if [ ! -s "$oracle/pickler.dump" ]; then
    pending_fail "$name" "scalac gave no dump: $(head -1 "$oracle/scalac.txt" 2> /dev/null)"
    fail=$((fail + 1))
    continue
  fi
  out="$work/teq/$name"
  rm -rf "${out:?}"
  mkdir -p "$out"
  abs=$(cd "$dir" && pwd)
  if ! (cd "$dir" && timeout 120 "$TEQ" compiler check *.scala --std scala-library --products "$OLDPWD/$out/p") > "$out/check.txt" 2>&1; then
    pending_fail "$name" "teq's products: $(head -1 "$out/check.txt")"
    fail=$((fail + 1))
    continue
  fi
  TEQ_INIT_VIEW="$abs" timeout 60 "$TEQ" tasty "$out/p" > "$out/pickler.dump" 2> "$out/view.err"
  if result=$(python3 tests/init-view/compare.py --positions "$oracle/pickler.dump" "$out/pickler.dump"); then
    pending_pass "$name"
    pass=$((pass + 1))
  else
    pending_listed "$name" || echo "FAIL $name: $result"
    pending_fail "$name" "$result"
    fail=$((fail + 1))
  fi
done
echo "init-view: $pass equal, $fail differ, $skipped not run"
pending_end
