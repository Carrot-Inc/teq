#!/bin/bash
# The controls of the pairs' proofs: each changes one named pair of the record a class keeps, before anything reads
# it (TEQ_PAIRS_CONTROL, crate::tir::pairs_control, under the pairs' dump alone), and the proofs must turn red on
# that pair: the twin (tests/pairs-twin.sh: the stored record held to the enumeration) and the equality
# (tests/pairs-equality.sh: the program held to its expectation on every leg), and with --claim the claim's cells.
# First the head unmutated over the same programs: every proof green. Then each control:
#   drop      C1's pair O.f > T1.f of tests/cases/inherited_pair_default, the code review's shape: the interpreter,
#             which takes the default's declarer from the receiver class's pair, loses it (JavaScript's default getter
#             is gathered by method from every class's pairs, and E's same pair still makes it; the review's own
#             mutant, every pair of f dropped, turns JavaScript too)
#   swap      C's pair O.f(a: Int) > T.f(a: A) of tests/cases/inherited_pair_generic_overload, its target made O's
#             other alternative f(a: String)
#   unbridge  Rest's pair Base.at > Reads.at of tests/cases/methods_of_namesake_parameters, its bridged mark
#             cleared (the superclass's bridge answers it: no value moves, so the twin alone can see it)
# For each: whether the control fired (its `control` line in the head's dump), whether the twin and the equality
# name the pair or its program, and with --claim whether the claim names the cells (drop on C's pair O.f > T.f of
# two cells). A Ready's run, master's binary beside the head's.
# usage: tests/pairs-controls.sh --master <teq> --head <teq> --out <dir> [--claim <table dir>]...
cd "$(dirname "$0")/.." || exit 1
master= head= out= claims=()
while [ $# -gt 0 ]; do
  case $1 in
    --master) master=$2; shift ;;
    --head) head=$2; shift ;;
    --out) out=$2; shift ;;
    --claim) claims+=(--claim "$2"); shift ;;
  esac
  shift
done
[ -n "$master" ] && [ -n "$head" ] && [ -n "$out" ] || { echo "usage: tests/pairs-controls.sh --master <teq> --head <teq> --out <dir> [--claim <table dir>]..."; exit 2; }
mkdir -p "$out" || exit 2
# control | program | the class | what names the pair in the twin's line | the equality's legs expected red
controls=(
  "drop:C1:f|cases/inherited_pair_default|C1|O.f(a: Int): Int > T1.f(a: Int): Int|interp"
  "swap:C:f|cases/inherited_pair_generic_overload|C|> T.f(a: A): Int|jvm"
  "unbridge:Rest:at|cases/methods_of_namesake_parameters|Rest|Base.at(slot: Slot): Int > Reads.at(slot: Slot): Int|-"
)
cells="default-mixing-run,def-def-mixing-run"
programs=$(for c in "${controls[@]}"; do IFS='|' read -r _ p _ <<< "$c"; printf '%s,' "$p"; done)
programs=${programs%,}
status=0
proofs() {
  # proofs <dir> <control spec or empty>: the twin and the equality over the controls' programs, the claim's cells
  local d=$1 spec=$2
  mkdir -p "$d"
  TEQ_PAIRS_CONTROL=$spec timeout 1200 tests/pairs-twin.sh --head "$head" --out "$d/twin" --cases --only "$programs" --jobs 3 > "$d/twin.txt" 2>&1
  echo $? > "$d/twin.exit"
  TEQ_PAIRS_CONTROL=$spec timeout 1200 python3 tests/support/pairs_equality.py --master "$master" --head "$head" --out "$d/equality" --cases --only "$programs" --legs js,jvm,interp --jobs 3 > "$d/equality.txt" 2>&1
  echo $? > "$d/equality.exit"
  if [ ${#claims[@]} -gt 0 ]; then
    TEQ_PAIRS_CONTROL=$spec timeout 1200 python3 tests/support/pairs_claim.py --head "$head" --master "$master" "${claims[@]}" --only "$cells" --out "$d/claim" --jobs 2 > "$d/claim.txt" 2>&1
    echo $? > "$d/claim.exit"
  fi
}
proofs "$out/unmutated" ""
line="unmutated: twin exit $(cat "$out/unmutated/twin.exit"), equality exit $(cat "$out/unmutated/equality.exit")"
[ ${#claims[@]} -gt 0 ] && line+=", claim exit $(cat "$out/unmutated/claim.exit")"
echo "$line"
for f in "$out"/unmutated/*.exit; do [ "$(cat "$f")" = 0 ] || status=1; done
for c in "${controls[@]}"; do
  IFS='|' read -r spec program class pair legs <<< "$c"
  d=$out/${spec%%:*}
  proofs "$d" "$spec"
  fired=$(grep -h $'\tcontrol\t' "$d"/twin/*/dump-*.txt 2> /dev/null | grep -c "^$class"$'\t')
  twin=$(grep -F "$class: stored record:" "$d/twin.txt" | grep -cF "$pair")
  eq=0
  if [ "$legs" != - ]; then
    for leg in ${legs//,/ }; do
      eq=$((eq + $(grep -P "^\Q$program\E\t$leg\tVALUE\t" "$d/equality.txt" | wc -l)))
    done
  fi
  verdict="ok"
  [ "$fired" -gt 0 ] || verdict="NOT FIRED"
  [ "$twin" -gt 0 ] || verdict="TWIN GREEN"
  [ "$legs" = - ] || [ "$eq" -gt 0 ] || verdict="EQUALITY GREEN"
  claimed=
  if [ ${#claims[@]} -gt 0 ] && [ "${spec%%:*}" = drop ]; then
    # the claim's cells under the same control aimed at their class C: run apart, C's pair dropped in each
    TEQ_PAIRS_CONTROL=drop:C:f timeout 1200 python3 tests/support/pairs_claim.py --head "$head" --master "$master" "${claims[@]}" --only "$cells" --out "$d/claim-c" --jobs 2 > "$d/claim-c.txt" 2>&1
    red=$(grep -cP "^cells/($(echo "$cells" | tr ',' '|'))\t" "$d/claim-c.txt")
    claimed=", claim (C's pair of $cells dropped): $red legs red"
    [ "$red" -gt 0 ] || verdict="CLAIM GREEN"
  fi
  [ "$verdict" = ok ] || status=1
  echo "$spec on $program: $verdict: fired $fired, twin $twin line(s) naming $class's pair, equality $eq leg(s) red (${legs})$claimed"
done
echo "pairs-controls: $([ $status = 0 ] && echo "every control turned its proofs red and the head unmutated is green" || echo "FAILED")"
exit $status
