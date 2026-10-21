#!/bin/bash
# bench/fix/matrix.sh <label> <teq> [<base label> <base teq>]: the stage's matrix (fix-speed's table, §9): the three
# corpora, the all-on and the application's settings, 1, 4, 8 and 16 workers, cold then warm; the frontend with every rule
# cold at typing 1, 4, 8 and 16 × rules 1, 4, 8 and 16 (fix-parallel-typing); each rule alone over jzon at 1 and 8 workers. A base binary given runs once per corpus, settings and temperature (its rules' phase is serial),
# before the candidate's runs of that cell, and its exit, its bytes and its corpus's identity are the cell's baseline,
# which each candidate run must give (`FIX_EXPECT`), the base's corpus identity asked of the candidate's engine
# (`FIX_INVENTORY_TEQ`: the base predates `TEQ_FIX_INPUTS`); without one, the candidate's one-worker run is the baseline. A run bench/fix/run.sh rejects (a
# failed or incomplete job, a record not the parallel run asked for, other bytes) is no measurement: the matrix goes on,
# and exits 1 at its end, naming how many it rejected (out/measure/rejected.tsv). Lines go to FIX_RESULTS (run.sh's).
cd "$(dirname "$0")/../.."
label=$1 teq=$2 blabel=${3:-} bteq=${4:-}
m=bench/fix/run.sh
results=${FIX_RESULTS:-out/measure/results.tsv}
rejected=0
# The exit, the digest and the corpus's identity of the row the last accepted run appended.
last() { tail -1 "$results" | awk -F'\t' '{print $6 ":" $9 ":" $12}'; }
# cell <corpus> <conf> <temp> <worker counts...>
cell() {
  local corpus=$1 conf=$2 temp=$3 expect="" w
  shift 3
  if [ -n "$bteq" ]; then
    if FIX_INVENTORY_TEQ=$teq $m "$bteq" "$blabel" "$corpus" "$conf" - "$temp"; then expect=$(last); else rejected=$((rejected + 1)); return; fi
  fi
  for w in "$@"; do
    if FIX_EXPECT=$expect $m "$teq" "$label" "$corpus" "$conf" "$w" "$temp"; then
      [ -n "$expect" ] || expect=$(last)
    else
      rejected=$((rejected + 1))
    fi
  done
}
for corpus in toy jzon frontend; do
  for conf in all-on.conf app.conf; do
    for temp in cold warm; do cell $corpus $conf $temp 1 4 8 16; done
  done
done
# The typing's workers apart from the rules' (fix-parallel-typing): the frontend with every rule, cold, typing × rules,
# each run's bytes the cell's baseline's (`FIX_TYPING`, the rows labelled `<label>-t<n>`).
tcell() {
  local corpus=$1 conf=$2 temp=$3 expect="" t w
  if [ -n "$bteq" ]; then
    if FIX_INVENTORY_TEQ=$teq $m "$bteq" "$blabel" "$corpus" "$conf" - "$temp"; then expect=$(last); else rejected=$((rejected + 1)); return; fi
  fi
  for t in 1 4 8 16; do
    for w in 1 4 8 16; do
      if FIX_TYPING=$t FIX_EXPECT=$expect $m "$teq" "$label" "$corpus" "$conf" "$w" "$temp"; then
        [ -n "$expect" ] || expect=$(last)
      else
        rejected=$((rejected + 1))
      fi
    done
  done
}
tcell frontend all-on.conf cold
for r in DisableSyntax CatsImplicitsImport TypelevelAs EmptyCollectionsUnified NoUnusedImports; do
  cell jzon one-$r.conf cold 1 8
done
if [ $rejected -gt 0 ]; then
  echo "matrix: $rejected runs rejected (out/measure/rejected.tsv): no measurement of theirs"
  exit 1
fi
echo matrix-done
