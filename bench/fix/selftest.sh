#!/bin/bash
# bench/fix/selftest.sh <teq>: the runner's own checks, the saved controls of the push pass of fix-speed's becf46896: an
# eight-worker run over the toy corpus accepted, its row appended; and each of these rejected, no row appended: the same
# run serialized (`TEQ_FIX_PLANT=serial`, its record's every document on worker 0), a corpus that is absent, a job teq
# refuses (exit 2: a corpus that does not type), a run whose exit and bytes are not the baseline's (`FIX_EXPECT`), and a
# corpus that is not the baseline's (empty, or one file of the thousand, flat or behind a directory link the compiler
# follows: refused before the measured run, the files it lacks counted against the engine's own input set).
# Then the record's validator over the serialized run's record with its documents spread round-robin over the workers
# ready at each dispatch (the pass of 1bd060d10's control: no two evaluations overlap, every worker id appears),
# refused, and over the accepted run's, accepted; and the cost-probe runner (bench/fix/probes.sh) over an absent corpus
# and over one teq refuses, each failing with no row appended. And the cost cell: jzon's two documents at eight workers
# accepted, its utilization reported, and a record of that shape accepted by the validator.
cd "$(dirname "$0")/../.."
teq=${1:?usage: bench/fix/selftest.sh <teq>}
work=out/measure/selftest
rm -rf "$work"
mkdir -p "$work/bad/toy" "$work/none"
printf 'object Bad:\n  val x: Int = "not an Int"\n' > "$work/bad/toy/Bad.scala"
export FIX_RESULTS=$work/results.tsv
: > "$FIX_RESULTS"
bench/fix/corpora.sh > /dev/null || { echo "selftest: the corpora: bench/fix/corpora.sh failed"; exit 1; }
pass=0 fail=0
# check <name> <accept|reject> <env and command...>: its exit and whether it appended a row.
check() {
  local name=$1 want=$2 before after code
  shift 2
  before=$(wc -l < "$FIX_RESULTS")
  env "$@" > "$work/$name.log" 2>&1
  code=$?
  after=$(wc -l < "$FIX_RESULTS")
  if { [ "$want" = accept ] && [ $code = 0 ] && [ $after = $((before + 1)) ]; } || { [ "$want" = reject ] && [ $code != 0 ] && [ $after = $before ]; }; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL $name: expected $want, exit $code, rows $before -> $after: $(tail -1 "$work/$name.log")"
  fi
}
record=out/measure/runs/selftest-toy-all-on-8-cold.record
check accepted accept bench/fix/run.sh "$teq" selftest toy all-on.conf 8 cold
expect=$(tail -1 "$FIX_RESULTS" | awk -F'\t' '{print $6 ":" $9 ":" $12}')
toy=$(tail -1 "$FIX_RESULTS" | awk -F'\t' '{print $12}')
cp "$record" "$work/accepted.record"
check serialized reject bench/fix/run.sh "$teq" selftest toy all-on.conf 8 cold TEQ_FIX_PLANT=serial
cp "$record" "$work/serialized.record"
python3 - "$work/serialized.record" "$work/spread.record" <<'PY'
import re, sys
lines = open(sys.argv[1]).read().split("\n")
fields = lambda l: dict(kv.split("=", 1) for kv in l.split()[1:] if "=" in kv)
ready = {int(fields(l)["w"]): int(fields(l)["ready_us"]) for l in lines if l.startswith("worker ")}
assign = {}
for i, l in enumerate(lines):
    if l.startswith("dispatch "):
        f = fields(l)
        free = sorted(w for w, t in ready.items() if t <= int(f["at_us"])) or [0]
        assign[int(f["slot"])] = free[int(f["slot"]) % len(free)]
    if l.startswith(("dispatch ", "done ")):
        lines[i] = re.sub(r"worker=\d+", "worker=%d" % assign[int(fields(l)["slot"])], l)
open(sys.argv[2], "w").write("\n".join(lines))
PY
# check-record <name> <accept|reject> <record>: the validator's verdict at eight workers.
check_record() {
  python3 bench/fix/record.py "$3" 8 > "$work/$1.log" 2>&1
  local code=$?
  if { [ "$2" = accept ] && [ $code = 0 ]; } || { [ "$2" = reject ] && [ $code != 0 ]; }; then pass=$((pass + 1)); else fail=$((fail + 1)); echo "FAIL $1: expected $2, the validator exited $code: $(head -1 "$work/$1.log")"; fi
}
cat > "$work/cost-cell.record" <<'REC'
worker w=0 ready_us=20000 boot_us=19000
run eligible=1 workers=8
round job=0 round=1 docs=2 first_gen=0
dispatch slot=0 gen=0 worker=0 at_us=30000
done slot=0 gen=0 worker=0 start_us=30100 finish_us=40000 cpu_us=9800 queries=0
dispatch slot=1 gen=1 worker=0 at_us=40100
done slot=1 gen=1 worker=0 start_us=40200 finish_us=41000 cpu_us=790 queries=0
worker w=1 ready_us=45000 boot_us=15000
round-end wall_us=11000 service_us=10700 cpu_us=10590
REC
check_record record-accepted accept "$work/accepted.record"
check_record record-spread reject "$work/spread.record"
check_record record-cost-cell accept "$work/cost-cell.record"
[ "$(grep -o 'worker=[0-9]*' "$work/spread.record" | sort -u | wc -l)" -gt 2 ] || { fail=$((fail + 1)); echo "FAIL record-spread: the control spread its documents over too few workers"; }
check absent reject FIX_CORPORA=$work/none bench/fix/run.sh "$teq" selftest jzon all-on.conf 8 cold
check refused reject FIX_CORPORA=$work/bad bench/fix/run.sh "$teq" selftest toy all-on.conf 8 cold
check baseline reject FIX_EXPECT=0:000000000000:$toy bench/fix/run.sh "$teq" selftest toy all-on.conf 8 cold
check same-bytes accept FIX_EXPECT=$expect bench/fix/run.sh "$teq" selftest toy all-on.conf 4 cold
# The corpus a row ran over (the pass of 621d498f0): an empty `toy`, with the full corpus's baseline and without one, and
# a `toy` of one file of the thousand against that baseline, each refused before teq runs, no row appended.
mkdir -p "$work/empty/toy" "$work/one/toy"
cp "$(ls out/corpora/toy/*.scala | head -1)" "$work/one/toy/"
check empty-corpus reject FIX_CORPORA=$work/empty FIX_EXPECT=$expect bench/fix/run.sh "$teq" selftest toy all-on.conf 8 cold
check empty-alone reject FIX_CORPORA=$work/empty bench/fix/run.sh "$teq" selftest toy all-on.conf 8 cold
check truncated-corpus reject FIX_CORPORA=$work/one FIX_EXPECT=$expect bench/fix/run.sh "$teq" selftest toy all-on.conf 8 cold
grep -q '999 of its files missing' "$work/truncated-corpus.log" || { fail=$((fail + 1)); echo "FAIL truncated-corpus: the files it lacks not named: $(tail -1 "$work/truncated-corpus.log")"; }
# A linked directory (the pass of 672e24ab6): `toy` holding one file and a link to a directory of the other 999, which
# the compiler follows: accepted whole, its identity the engine's input set of 1,000 files; with the link removed, the
# same baseline refused, the 999 it lacks counted.
mkdir -p "$work/linked/toy" "$work/linked/rest"
toys=(out/corpora/toy/*.scala)
cp "${toys[0]}" "$work/linked/toy/"
cp "${toys[@]:1}" "$work/linked/rest/"
ln -s ../rest "$work/linked/toy/linked"
check linked-full accept FIX_CORPORA=$work/linked bench/fix/run.sh "$teq" selftest toy all-on.conf 8 cold
linked=$(tail -1 "$FIX_RESULTS" | awk -F'\t' '{print $6 ":" $9 ":" $12}')
case $linked in *:1000:*) pass=$((pass + 1)) ;; *) fail=$((fail + 1)); echo "FAIL linked-full: the identity $linked, not the 1,000 files the compiler reads" ;; esac
rm "$work/linked/toy/linked"
check linked-truncated reject FIX_CORPORA=$work/linked FIX_EXPECT=$linked bench/fix/run.sh "$teq" selftest toy all-on.conf 8 cold
grep -q '999 of its files missing' "$work/linked-truncated.log" || { fail=$((fail + 1)); echo "FAIL linked-truncated: the files it lacks not counted: $(tail -1 "$work/linked-truncated.log")"; }
# A cost cell (the pass of 2a3b3a095): jzon's two documents with NoUnusedImports alone at eight workers, which worker 0
# may finish before worker 1 has started, accepted with its utilization reported, no witness asked of it; and the record
# of that shape (both documents on worker 0, one after the other) accepted by the validator as a cost cell, where the
# toy corpus's serialized record stays refused as a witness.
check jzon-baseline accept bench/fix/run.sh "$teq" selftest jzon one-NoUnusedImports.conf 1 cold
jzon=$(tail -1 "$FIX_RESULTS" | awk -F'\t' '{print $6 ":" $9 ":" $12}')
check jzon-cost-cell accept FIX_EXPECT=$jzon bench/fix/run.sh "$teq" selftest jzon one-NoUnusedImports.conf 8 cold
[ "$(tail -1 "$FIX_RESULTS" | awk -F'\t' '{print $11}')" != - ] || { fail=$((fail + 1)); echo "FAIL jzon-cost-cell: no utilization reported"; }
# The cost probes: an absent corpus, and one teq refuses (exit 2), each a failure with no row.
mkdir -p "$work/bad/jzon" && cp "$work/bad/toy/Bad.scala" "$work/bad/jzon/"
export FIX_PROBES=$work/probes.tsv
: > "$FIX_PROBES"
probe_check() {
  local name=$1 corpora=$2 before after code
  before=$(wc -l < "$FIX_PROBES")
  FIX_CORPORA=$corpora bench/fix/probes.sh selftest "$teq" > "$work/$name.log" 2>&1
  code=$?
  after=$(wc -l < "$FIX_PROBES")
  if [ $code != 0 ] && [ $after = $before ]; then pass=$((pass + 1)); else fail=$((fail + 1)); echo "FAIL $name: exit $code, rows $before -> $after: $(tail -1 "$work/$name.log")"; fi
}
probe_check probes-absent "$PWD/$work/none"
probe_check probes-refused "$PWD/$work/bad"
echo "selftest: $pass passed, $fail failed"
[ $fail = 0 ]
