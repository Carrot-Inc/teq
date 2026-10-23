#!/bin/bash
# bench/app/fix.sh <checkout> <teq> <out dir>: the application's lint and rewrite rules under teq fix, the gate's
# app-fix line (tests/gate.sh). The checkout's HEAD is copied by `git archive` into <out dir>/copy (nothing written in
# the checkout); there `<teq> fix --check` runs every rule each configuration's file names (the lock's `scalafix`
# records give a configuration its file and leave out those scalafix skips), so that a rule teq lacks refuses it; its
# record's documents are checked against the frozen manifest of what scalafix covers (fix/reference/manifest.tsv, through the application's
# fix/Compare.scala), and the rules' fixtures run (fix/fixtures/run.sh); the copy is removed after. The check's record
# is this run's or none: what an earlier run left in <out dir> is removed first, and a check that writes no record
# fails the line (a run of no configuration writes one, empty, which the coverage counts as no document). Prints the
# steps' times; exits 1 on a lint, an omitted or extra document, a failing fixture or a refused run, 2 where it cannot
# run.
[ $# = 3 ] || { echo "usage: bench/app/fix.sh <checkout> <teq> <out dir>" >&2; exit 2; }
absolute() { case $1 in /*) echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
checkout=$(absolute "$1") teq=$(absolute "$2") out=$(absolute "$3")
copy=$out/copy
rm -rf "$copy" && mkdir -p "$copy" || exit 2
trap 'rm -rf "$copy"' EXIT
git -C "$checkout" archive HEAD | tar -x -C "$copy" || { echo "app-fix: cannot copy $checkout's HEAD"; exit 2; }
# The copy has no repository: a generator reading the commit takes it from SOURCE_VERSION.
head=$(git -C "$checkout" rev-parse HEAD) || { echo "app-fix: no HEAD in $checkout"; exit 2; }
for f in fix/fixtures/run.sh fix/Compare.scala fix/reference/manifest.tsv; do
  [ -e "$copy/$f" ] || { echo "app-fix: the checkout has no $f"; exit 1; }
done
rm -f "$out/check.record" "$out/check.log" "$out/coverage.log" "$out/fixtures.log"
now() { date +%s; }
code=0
s=$(now)
(cd "$copy" && SOURCE_VERSION=$head TEQ_FIX_TRACE=1 TEQ_FIX_RECORD=$out/check.record timeout 1200 "$teq" fix --check) > "$out/check.log" 2>&1
check=$?
check_time=$(($(now) - s))
[ $check = 0 ] || code=1
s=$(now)
if [ -e "$out/check.record" ]; then
  (cd "$copy" && timeout 300 "$teq" interp fix/Compare.scala -- coverage fix/reference/manifest.tsv "$out/check.record") > "$out/coverage.log" 2>&1
  coverage=$?
else
  echo "coverage: the check wrote no record" > "$out/coverage.log"
  coverage=1
fi
coverage_time=$(($(now) - s))
[ $coverage = 0 ] || code=1
s=$(now)
(cd "$copy" && TEQ=$teq timeout 1200 fix/fixtures/run.sh) > "$out/fixtures.log" 2>&1
fixtures=$?
fixtures_time=$(($(now) - s))
[ $fixtures = 0 ] || code=1
grep -h "^fix: .* ms in all" "$out/check.log" | sed 's/^/  /'
echo "app-fix: check exit $check in ${check_time}s, $(tail -1 "$out/coverage.log") in ${coverage_time}s, $(tail -1 "$out/fixtures.log") in ${fixtures_time}s"
exit $code
