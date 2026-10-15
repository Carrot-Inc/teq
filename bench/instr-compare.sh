#!/bin/bash
# bench/instr-compare.sh <master teq> <landing teq> [runs] [program...]: the instructions retired by `teq compiler check` of
# the budget's programs (bench/programs.sh; realistic-frontend and realistic-api when none is named), the two
# binaries taking turns run by run, each with a jar cache of its own, after one run each that warms the caches; per
# program the landing's minimum and median against master's (5 runs by default) and master's spread, its largest
# run over its smallest. Fails when a program's minimum is more than INSTR_TOLERANCE percent (default 0.5, the
# instruction gate) over master's, when a check fails or gives no positive count, or when a program has fewer counts
# than runs; INSTR_TOLERANCE=none records the programs apart, failing only on those failures. The counts are those of
# `/usr/bin/time -l` on macOS and cachegrind's instruction references on Linux, where one run is enough. The programs
# are measured INSTR_JOBS at a time (by default the processors), each one's runs in the order above, and printed in the
# order named; INSTR_COUNTS=<file> keeps every count, a line per program and side.
absolute() { case $1 in /*) echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
[ $# -ge 2 ] || { echo "usage: bench/instr-compare.sh <master teq> <landing teq> [runs] [program...]" >&2; exit 2; }
master=$(absolute "$1") landing=$(absolute "$2")
[ -z "$INSTR_COUNTS" ] || INSTR_COUNTS=$(absolute "$INSTR_COUNTS")
runs=${3:-5}
[[ "$runs" =~ ^[1-9][0-9]*$ ]] || { echo "instr-compare: runs must be a positive count, not $runs" >&2; exit 2; }
shift 2
[ $# = 0 ] || shift
wanted=${*:-realistic-frontend realistic-api}
# On Linux the counts are cachegrind's instruction references (deterministic, so one run unless INSTR_RUNS says otherwise).
if [ "$(uname -s)" = Darwin ]; then counter=time; else
  command -v valgrind > /dev/null || { echo "instr-compare: needs /usr/bin/time -l (macOS) or valgrind (Linux)"; exit 2; }
  counter=cachegrind; runs=${INSTR_RUNS:-1}
fi
cd "$(dirname "$0")/.."
work=${INSTR_WORK:-out/budget}
. bench/programs.sh || { echo "instr-compare: the programs could not be generated"; exit 1; }
# Each binary gets a jar cache of its own per program, named by its printed version and the program, warmed by the
# program's first run: two binaries never rewrite each other's entries (a cache file holds one binary's key; another
# binary's run finds it stale, reads the jar cold and writes the file anew), and programs measured at once never
# write one cache file (a writer merges into the entries of the snapshot it read, src/jarcache.rs, so one would
# drop the other's), each program's counts those of its runs alone, at a time or at once.
version_of() { "$1" --version 2> /dev/null | tr -c 'A-Za-z0-9.\n' '-' | tr -d '\n'; }
version_master=$(version_of "$master") version_landing=$(version_of "$landing")
# Each binary's spelling of the compiler's verb, probed once here, outside the counts.
. tests/support/compiler-words.sh
spellings "$master" "$landing"
words_master=$(compiler_words "$master") words_landing=$(compiler_words "$landing")
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
# one <binary> <cache> <args> <log>: the instructions retired by one check at one worker, which both binaries are
# asked for whatever their defaults; a failure when the check fails or no positive count comes out of it.
one() {
  local n words=$words_landing
  [ "$1" != "$master" ] || words=$words_master
  if [ "$counter" = time ]; then
    TEQ_THREADS=1 TEQ_CACHE_DIR=$2 /usr/bin/time -l "$1" $words check $3 > /dev/null 2> "$4" || return 1
    n=$(awk '/instructions retired/ {print $1}' "$4")
  else
    TEQ_THREADS=1 TEQ_CACHE_DIR=$2 valgrind --tool=cachegrind --cache-sim=no --cachegrind-out-file=/dev/null --log-file="$4" "$1" $words check $3 > /dev/null 2>&1 || return 1
    n=$(awk '/I +refs:/ {gsub(",", "", $NF); print $NF}' "$4")
  fi
  [[ "$n" =~ ^[1-9][0-9]*$ ]] || { echo "no count of instructions retired" >> "$4"; return 1; }
  echo "$n"
}
# measure <program>: its runs, the binaries taking turns after a warming run each, and its line on stdout; its
# verdict in $tmp/<program>/verdict ("<status> <program> <change, missing or failed>") and its counts in
# $tmp/<program>/counts.
measure() {
  local prog=$1 t=$tmp/$1 args cm cl a=() b=() ok=1 r x y line verdict change
  mkdir -p "$t"
  if [[ " $programs " != *" $prog "* ]]; then
    echo "$prog: not among the programs here (its jars missing?)"
    echo "1 $prog missing" > "$t/verdict"
    return
  fi
  args=$(program_args "$prog")
  cm=out/instr-cache/$version_master/$prog cl=out/instr-cache/$version_landing/$prog
  mkdir -p "$cm" "$cl"
  one "$master" "$cm" "$args" "$t/time.txt" > /dev/null && one "$landing" "$cl" "$args" "$t/time.txt" > /dev/null || ok=0
  for r in $(seq 1 "$runs"); do
    [ $ok = 1 ] || break
    x=$(one "$master" "$cm" "$args" "$t/time.txt") && y=$(one "$landing" "$cl" "$args" "$t/time.txt") || { ok=0; break; }
    a+=("$x")
    b+=("$y")
  done
  printf '%s master %s\n%s landing %s\n' "$prog" "${a[*]}" "$prog" "${b[*]}" > "$t/counts"
  if [ $ok = 0 ]; then
    echo "$prog: a check failed: $(tail -3 "$t/time.txt" | head -1)"
    echo "1 $prog failed" > "$t/verdict"
    return
  fi
  line=$(python3 - "$prog" "${a[*]}" "${b[*]}" "${INSTR_TOLERANCE:-0.5}" "$runs" <<'PY'
import sys, statistics
prog, a, b, tol, runs = sys.argv[1], [int(x) for x in sys.argv[2].split()], [int(x) for x in sys.argv[3].split()], sys.argv[4], int(sys.argv[5])
if len(a) != runs or len(b) != runs or runs < 1 or min(a + b) <= 0:
    print(f"error - {prog}: {len(a)} and {len(b)} counts for {runs} runs, the least {min(a + b, default=0)}")
    sys.exit(1)
mn = (min(b) / min(a) - 1) * 100
md = (statistics.median(b) / statistics.median(a) - 1) * 100
verdict = 'recorded' if tol == 'none' else 'over' if mn > float(tol) else 'within'
print(f"{verdict} {mn:+.2f}% {prog}: master min {min(a)/1e9:.3f}G landing min {min(b)/1e9:.3f}G  min {mn:+.2f}%  median {md:+.2f}%  master spread {(max(a)/min(a)-1)*100:.2f}%")
PY
) || { echo "$prog: the comparison failed: ${line#* * }"; echo "1 $prog failed" > "$t/verdict"; return; }
  echo "${line#* * }"
  read -r verdict change _ <<< "$line"
  # A program counts only with a verdict: within the tolerance, or recorded apart when that was asked for.
  case "$verdict,${INSTR_TOLERANCE:-}" in
    within,* | recorded,none) echo "0 $prog $change" > "$t/verdict" ;;
    *) echo "1 $prog $change" > "$t/verdict" ;;
  esac
}
jobs_at_once=${INSTR_JOBS:-$(getconf _NPROCESSORS_ONLN 2> /dev/null || echo 4)}
for prog in $wanted; do
  while [ "$(jobs -rp | wc -l)" -ge "$jobs_at_once" ]; do sleep 1; done
  measure "$prog" > "$tmp/$prog.out" 2>&1 &
done
wait
status=0
verdicts=()
[ -z "$INSTR_COUNTS" ] || : > "$INSTR_COUNTS"
for prog in $wanted; do
  cat "$tmp/$prog.out"
  [ -z "$INSTR_COUNTS" ] || cat "$tmp/$prog/counts" >> "$INSTR_COUNTS" 2> /dev/null
  if read -r code name change 2> /dev/null < "$tmp/$prog/verdict"; then
    [ "$code" = 0 ] || status=1
    verdicts+=("$prog $change")
  else
    echo "$prog: its measurement ended without a verdict"
    status=1
    verdicts+=("$prog failed")
  fi
done
summary=$(IFS=,; echo "${verdicts[*]}" | sed 's/,/, /g')
if [ "${INSTR_TOLERANCE:-}" = none ]; then
  echo "instructions: $summary (minimum of $runs, recorded apart)"
elif [ $status = 0 ]; then
  echo "instructions: $summary (minimum of $runs, within ${INSTR_TOLERANCE:-0.5}%)"
else
  echo "instructions: $summary (minimum of $runs; the gate is ${INSTR_TOLERANCE:-0.5}%)"
fi
exit $status
