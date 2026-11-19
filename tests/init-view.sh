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
# scalac's dumps are kept under out/init-view/oracle/<program>-<digest>/, the digest over the program's sources,
# InitDump.scala, initdump.py and the environment (both JDKs and scala-library's jar, tests/init-view/environment.sh,
# printed as the first line), and made again only when one changes. The programs expected to fail are
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
: > "$work/view-failures"
# The class path scalac compiles the programs against (initdump.py's): the view reads its classes from the same jar.
library=$("${COURSIER:-cs}" fetch --classpath --intransitive org.scala-lang:scala-library:3.8.4 2> /dev/null | tail -1)
# What the dumps depend on beside the sources, first.
bash tests/init-view/environment.sh "$library"
# The digest of the scalac dumps kept also names the environment they depend on (tests/init-view/environment.sh:
# both JDKs and the library), so a dump made under another JDK is made again, not judged.
env_key=$(bash tests/init-view/environment.sh --key "$library")
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
  # A program's members.txt names the library classes whose trees both dumps print (INITDUMP_MEMBERS,
  # TEQ_INIT_VIEW_MEMBERS); it is part of the digest.
  members=; [ -f "$dir/members.txt" ] && members=$(cd "$dir" && pwd)/members.txt
  digest=$( { cat "${files[@]}" ${members:+"$members"} tests/init-view/InitDump.scala tests/init-view/initdump.py; echo "$env_key"; } | sha256sum | cut -c1-16)
  oracle="$work/oracle/$name-$digest"
  if [ ! -f "$oracle/exit" ] && [ $runnable = 0 ]; then
    pending_skip "$name" "scalac 3.8.4 not available: no coursier (${COURSIER:-cs}) or java"
    skipped=$((skipped + 1))
    rm -rf "${work:?}/teq/${name:?}"
    mkdir -p "$work/teq/$name" && echo notrun > "$work/teq/$name/status"
    continue
  fi
  # The view reads scala-library 3.8.4's pickles (TEQ_INIT_VIEW_CLASSPATH), which coursier fetches: without them, NOT
  # RUN. A program not run leaves no dump of teq's, which the whole dump's verdict below would judge.
  if [ -z "$library" ]; then
    pending_skip "$name" "scala-library 3.8.4 not fetched (no coursier: ${COURSIER:-cs})"
    skipped=$((skipped + 1))
    rm -rf "${work:?}/teq/${name:?}"
    mkdir -p "$work/teq/$name" && echo notrun > "$work/teq/$name/status"
    continue
  fi
  if [ ! -f "$oracle/exit" ]; then
    INITDUMP_MEMBERS=$members python3 tests/init-view/initdump.py --out "$oracle" "${files[@]}" > /dev/null 2>&1
  fi
  if [ ! -s "$oracle/pickler.dump" ]; then
    pending_fail "$name" "scalac gave no dump: $(head -1 "$oracle/scalac.txt" 2> /dev/null)"
    fail=$((fail + 1))
    continue
  fi
  out="$work/teq/$name"
  rm -rf "${out:?}"
  mkdir -p "$out"
  # The program's status for the whole dump's and the view alone's verdicts: `ran` (its dump is this run's), `failed`
  # (no dump: a failure of all three judgments), `notrun` (NOT RUN). Written on every path, so that no stale dump is
  # judged and no failure is skipped.
  echo failed > "$out/status"
  abs=$(cd "$dir" && pwd)
  # The typed program's classes (TEQ_INIT_VIEW_INVENTORY): a class in a body the writer withholds is printed
  # unavailable, never absent.
  if ! (cd "$dir" && TEQ_INIT_VIEW_INVENTORY="$OLDPWD/$out/inventory.txt" timeout 120 "$TEQ" compiler check *.scala --std scala-library --products "$OLDPWD/$out/p") > "$out/check.txt" 2>&1; then
    pending_fail "$name" "teq's products: $(head -1 "$out/check.txt")"
    fail=$((fail + 1))
    continue
  fi
  # A view that fails or prints nothing is a failure of all three judgments, which no pending list takes: the suite
  # fails (fails closed).
  if ! TEQ_INIT_VIEW_INVENTORY="$out/inventory.txt" TEQ_INIT_VIEW_MEMBERS=$members TEQ_INIT_VIEW_CLASSPATH="$library" TEQ_INIT_VIEW="$abs" timeout 60 "$TEQ" tasty "$out/p" > "$out/pickler.dump" 2> "$out/view.err" || [ ! -s "$out/pickler.dump" ]; then
    echo "VIEW FAILED $name: the view of teq's products (exit or empty): $(head -1 "$out/view.err")" | tee -a "$work/view-failures"
    pending_fail "$name" "the view failed"
    fail=$((fail + 1))
    continue
  fi
  echo ran > "$out/status"
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
status=$?
# Over the same dumps, one suite: the whole dump (tests/init-view-full.sh) and the view alone, of scalac's own pickles
# (tests/init-view-scalac.sh); each held to its own pending list.
TEQ=$TEQ bash tests/init-view-full.sh "$@" || status=1
TEQ=$TEQ bash tests/init-view-scalac.sh "$@" || status=1
# A view that failed fails the suite, listed or not.
if [ -s "$work/view-failures" ]; then
  echo "init-view: $(wc -l < "$work/view-failures") views failed (a failure no pending list takes)"
  status=1
fi
# The judgments' own self-test: the mutations they must refuse (tests/init-view/selftest.sh).
TEQ=$TEQ bash tests/init-view/selftest.sh || status=1
exit $status
