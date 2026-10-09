#!/bin/bash
# bench/compare.sh <teq A> <teq B>: `teq compiler check --time` on every budget program with the two binaries
# interleaved, run by run, so that the machine's load falls on both alike. Prints each phase per
# program (the mean of the runs without the fastest and the slowest) and B's change against A; a
# type or total phase that moves more than COMPARE_TOLERANCE percent (default 3) and 0.1 ms fails,
# a parse phase is shown only (its 4 ms are where the noise is). COMPARE_RUNS (default 11) sets the
# runs per program and binary, COMPARE_STAT=median|min another statistic, COMPARE_PROGRAMS limits
# the programs (a space-separated list) and COMPARE_KEEP=<dir> keeps the runs. Each binary reads the
# jars through a cache of its own: the cache is keyed by the binary's version, so two versions
# taking turns over one cache would inflate the jars again on every run.
cd "$(dirname "$0")/.." || exit 1
set -o pipefail
[ $# -eq 2 ] || { echo "usage: $0 <teq A> <teq B>"; exit 2; }
A=$1; B=$2
runs=${COMPARE_RUNS:-11}
work=out/budget
. bench/programs.sh || exit 1
. bench/phases.sh
. tests/support/compiler-words.sh
spellings "$A" "$B"
programs=${COMPARE_PROGRAMS:-$programs}
out=$(mktemp -d)
caches=$(mktemp -d)
trap 'rm -rf "$caches"' EXIT
for i in $(seq 1 "$runs"); do
  order="A B"; [ $((i % 2)) -eq 0 ] && order="B A"
  for p in $programs; do
    args=$(program_args "$p")
    for side in $order; do
      bin=$A; [ $side = B ] && bin=$B
      line=$(TEQ_CACHE_DIR=$caches/$side timeout 60 "$bin" $(compiler_words "$bin") check $args --time 2>&1 | phases) || { echo "FAIL $p: $bin compiler check failed" >&2; exit 1; }
      echo "$p $line" >> "$out/$side"
    done
  done
done
STAT=${COMPARE_STAT:-trimmed} TOLERANCE=${COMPARE_TOLERANCE:-3} python3 - "$out" "$runs" <<'PY'
import os, re, statistics, sys

def medians(path):
    runs = {}
    for line in open(path):
        m = re.match(r"(\S+) lines \d+ read \S+ parse (\S+) type (\S+) total (\S+)", line)
        if m:
            runs.setdefault(m.group(1), []).append((float(m.group(2)), float(m.group(3)), float(m.group(4))))
    stat = {"min": min, "median": statistics.median}.get(os.environ["STAT"], lambda v: statistics.mean(sorted(v)[1:-1] if len(v) > 2 else v))
    return {p: [stat([r[i] for r in rs]) for i in range(3)] for p, rs in runs.items()}

a, b = medians(os.path.join(sys.argv[1], "A")), medians(os.path.join(sys.argv[1], "B"))
tolerance = float(os.environ["TOLERANCE"])
failed = []
print(f"{'program':20} {'parse A':>9} {'B':>9} {'':>7} {'type A':>9} {'B':>9} {'':>7} {'total A':>9} {'B':>9} {'':>7}")
for p in a:
    cells = []
    for phase, (x, y) in zip(("parse", "type", "total"), zip(a[p], b[p])):
        change = (y - x) / x * 100 if x else 0
        mark = "*" if abs(change) > tolerance and abs(y - x) > 0.1 else " "
        if abs(change) > tolerance and abs(y - x) > 0.1 and phase != "parse":
            failed.append(f"{p} {phase} {change:+.1f}%")
        cells.append(f"{x:9.2f} {y:9.2f} {change:+6.1f}%{mark}")
    print(f"{p:20} " + " ".join(cells))
total_a = sum(v[2] for v in a.values()); total_b = sum(v[2] for v in b.values())
print(f"sum of totals: {total_a:.1f} -> {total_b:.1f} ms ({(total_b - total_a) / total_a * 100:+.1f}%) over {sys.argv[2]} runs")
if failed:
    print(f"compare: {len(failed)} type and total phases moved more than {tolerance:g}%: " + ", ".join(failed[:8]) + (", ..." if len(failed) > 8 else ""))
    sys.exit(1)
print(f"compare: every type and total phase within {tolerance:g}%")
PY
status=$?
[ -n "$COMPARE_KEEP" ] && mkdir -p "$COMPARE_KEEP" && cp "$out"/A "$out"/B "$COMPARE_KEEP"/
rm -rf "$out"
exit $status
