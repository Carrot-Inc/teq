#!/bin/bash
# The compile-time budget: `teq compiler check --time` on the core-only benchmark (bench/gen.py, 51 files of
# 22 modules), on every variant of bench/features.py, on the class-string validator of
# bench/macros/gen.py (macro-cls: 4,000 macro sites), on the two sides of the application
# corpus (bench/app/gen.py: realistic-frontend is shared + frontend, realistic-api shared + api, both over the cats
# and sourcecode jars; left out without them in the coursier cache) and on bench/derive/zio30.scala
# (derive30: thirty zio-json derivations from the jar; left out without its jars), bench/derive/kittens30.scala and
# bench/derive/schema30.scala (kittens' and tapir's derivations alike), N runs per program, against
# the medians recorded in bench/budgets.txt per phase (parse, type, total). A phase whose fastest run takes
# more than BUDGET_TOLERANCE percent (default 10) longer than its budget fails: a regression
# shows in every run, a busy machine in some. BUDGET_RECORD=1 rewrites the budgets together with
# the identity of the machine and the kind of build; on another machine the suite skips, since the
# budgets say nothing about it. The binary is TEQ, by default the build guided by a profile,
# target/pgo/use/release/teq (bench/pgo.sh), the one the budgets are recorded on; a plain
# `cargo build --release` against them fails, since it is 10 to 40% slower on every row. The
# machine's noise, the deviation of a run from its program's median, is reported as the median
# and the largest deviation over all runs; BUDGET_RUNS (default 3) sets N. The suite is whole or
# it fails: a program left out for want of its jars fails a record, which would drop its lines,
# and a check, which would say nothing of it; so do a budget line no program measures, a
# measured phase without a line and a program measured in fewer runs than the others. BUDGET_PHASES (default "parse type total") limits the phases a
# check judges; a record writes all three. These are the reference machine's budgets, for runs by hand and
# the comparisons over time: the landing gate's budget line compares the head with master on a remote machine instead
# (bench/pairs.py), and judges the rows this file lists, every one present.
cd "$(dirname "$0")/.." || exit 1
# A check that fails fails the run, whatever of its report the pipe after it reads.
set -o pipefail
TEQ=${TEQ:-./target/pgo/use/release/teq}
budgets=bench/budgets.txt
runs=${BUDGET_RUNS:-3}
tolerance=${BUDGET_TOLERANCE:-10}
work=out/budget
phases=$(echo ${BUDGET_PHASES:-parse type total})
[ -n "$phases" ] || { echo "budget: BUDGET_PHASES names no phase"; exit 2; }
for phase in $phases; do
  case $phase in parse | type | total) ;; *) echo "budget: no phase $phase (parse, type, total)"; exit 2 ;; esac
done
build=$("$TEQ" --version 2> /dev/null | awk '{print ($NF == "pgo") ? "pgo" : "plain"}')
machine="$(uname -m) $(sysctl -n machdep.cpu.brand_string 2> /dev/null || grep -m1 'model name' /proc/cpuinfo 2> /dev/null | cut -d: -f2- | sed 's/^ *//') $(sysctl -n hw.ncpu 2> /dev/null || nproc 2> /dev/null) cores"

if [ -z "$BUDGET_RECORD" ]; then
  if [ ! -f "$budgets" ]; then
    echo "budget: no $budgets; record one with BUDGET_RECORD=1 $0"
    exit 0
  fi
  recorded=$(sed -n 's/^# machine: //p' "$budgets")
  if [ "$recorded" != "$machine" ]; then
    echo "budget: skipped, the budgets were recorded on another machine ($recorded)"
    exit 0
  fi
fi
[ -x "$TEQ" ] || { echo "budget: no binary at $TEQ; bench/pgo.sh builds target/pgo/use/release/teq"; exit 1; }
if [ -z "$BUDGET_RECORD" ]; then
  recorded=$(sed -n 's/^# build: //p' "$budgets")
  if [ "${recorded:-plain}" != "$build" ]; then
    echo "budget: the budgets are of a ${recorded:-plain} build and $TEQ is a $build one; bench/pgo.sh builds target/pgo/use/release/teq"
    exit 1
  fi
fi

. bench/programs.sh || exit 1
. bench/phases.sh
if [ -n "$left_out" ]; then
  printf 'budget: the suite is not whole, programs left out for want of their jars:\n%s' "$left_out"
  exit 1
fi

# One line per run: `<program> <parse ms> <type ms> <total ms>`. A run of every program, then the
# next: a burst of the machine's load falls on one run of the programs it meets, which the median
# leaves out, where runs one after another would take it into all of one program's.
measure() {
  for _ in $(seq 1 "$runs"); do
    for p in $programs; do
      line=$(timeout 60 "$TEQ" compiler check $(program_args "$p") --time 2>&1 | phases) || { echo "FAIL $p: teq compiler check failed" >&2; return 1; }
      echo "$p $line"
    done
  done
}
loadavg() { { sysctl -n vm.loadavg 2> /dev/null || cat /proc/loadavg; } | tr -d '{}' | awk '{print $1}'; }
load_before=$(loadavg)
measure > "$work/runs.txt" || exit 1
load=$(loadavg)
load="load $load_before at the start and $load at the end"

BUDGETS="$budgets" MACHINE="$machine" BUILD="$build" RUNS="$runs" TOLERANCE="$tolerance" RECORD="$BUDGET_RECORD" PHASES="$phases" LOAD="$load" python3 - "$work/runs.txt" <<'EOF'
import os, re, statistics, sys, datetime

runs = {}
for line in open(sys.argv[1]):
    m = re.match(r"(\S+) lines \d+ read \S+ parse (\S+) type (\S+) total (\S+)", line)
    if not m:
        continue
    p = m.group(1)
    runs.setdefault(p, []).append({"parse": float(m.group(2)), "type": float(m.group(3)), "total": float(m.group(4))})

medians = {}
fastest = {}
deviations = []
short = [p for p, rs in runs.items() if len(rs) != int(os.environ["RUNS"])]
if short:
    print(f"budget: {', '.join(short)} measured in fewer than {os.environ['RUNS']} runs; nothing judged or recorded")
    sys.exit(1)
for p, rs in runs.items():
    medians[p] = {}
    fastest[p] = {}
    for phase in ("parse", "type", "total"):
        values = [r[phase] for r in rs]
        med = statistics.median(values)
        medians[p][phase] = med
        fastest[p][phase] = min(values)
        if med > 0:
            deviations.extend(abs(v - med) / med * 100 for v in values)
noise = f"noise {statistics.median(deviations):.1f}% (median deviation of a run), {max(deviations):.1f}% at most"

if os.environ["RECORD"]:
    with open(os.environ["BUDGETS"], "w") as f:
        f.write(f"# machine: {os.environ['MACHINE']}\n")
        f.write(f"# build: {os.environ['BUILD']}\n")
        f.write(f"# recorded: {datetime.date.today()}, median of {os.environ['RUNS']} runs of teq compiler check --time, in ms; {noise}; {os.environ['LOAD']}\n")
        for p in medians:
            for phase in ("parse", "type", "total"):
                f.write(f"{p} {phase} {medians[p][phase]:.2f}\n")
    print(f"budget: recorded {len(medians)} programs, {3 * len(medians)} phases, {noise}, {os.environ['LOAD']}")
    sys.exit(0)

judged = os.environ["PHASES"].split()
budget = {}
for line in open(os.environ["BUDGETS"]):
    parts = line.split()
    if len(parts) == 3 and not line.startswith("#") and parts[1] in judged:
        budget[(parts[0], parts[1])] = float(parts[2])
tolerance = float(os.environ["TOLERANCE"])
failed = 0
over = 0
checked = 0
for p, phase in budget:
    if p not in medians:
        print(f"FAIL {p} {phase}: a budget no program measured")
        failed += 1
for p in medians:
    for phase in judged:
        limit = budget.get((p, phase))
        if limit is None:
            print(f"FAIL {p} {phase}: measured without a budget; re-record with BUDGET_RECORD=1")
            failed += 1
            continue
        checked += 1
        best, med = fastest[p][phase], medians[p][phase]
        change = (best - limit) / limit * 100 if limit else 0
        if change > tolerance:
            print(f"FAIL {p} {phase}: {best:.2f} ms at best (median {med:.2f}) against a budget of {limit:.2f} ms (+{change:.0f}%)")
            failed += 1
            over += 1
        elif (med - limit) / limit * 100 < -tolerance:
            print(f"note: {p} {phase} is {med:.2f} ms against a budget of {limit:.2f} ms ({(med - limit) / limit * 100:.0f}%); lower it with BUDGET_RECORD=1")
print(f"budget: {checked - over} of {checked} phases within {tolerance:.0f}% of their budget over {os.environ['RUNS']} runs, {noise}, {os.environ['LOAD']}" + (f", {failed} failed" if failed else ""))
sys.exit(1 if failed else 0)
EOF
