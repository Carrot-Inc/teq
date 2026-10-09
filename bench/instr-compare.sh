#!/bin/bash
# bench/instr-compare.sh <master teq> <landing teq> [runs] [program...]: the instructions retired by `teq compiler check` of
# the budget's programs (bench/programs.sh; realistic-frontend and realistic-api when none is named), the two
# binaries taking turns run by run, each with a jar cache of its own, after one run each that warms the caches; per
# program the landing's minimum and median against master's (5 runs by default) and master's spread, its largest
# run over its smallest. Fails when a program's minimum is more than INSTR_TOLERANCE percent (default 0.5, the
# instruction gate) over master's, when a check fails or gives no positive count, or when a program has fewer counts
# than runs; INSTR_TOLERANCE=none records the programs apart, failing only on those failures. The counts are those of
# `/usr/bin/time -l` on macOS and cachegrind's instruction references on Linux, where one run is enough.
absolute() { case $1 in /*) echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
[ $# -ge 2 ] || { echo "usage: bench/instr-compare.sh <master teq> <landing teq> [runs] [program...]" >&2; exit 2; }
master=$(absolute "$1") landing=$(absolute "$2")
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
# Each binary gets a jar cache of its own, named by its printed version, so two comparisons running at once never
# rewrite each other's entries (a cache file holds one binary's key; another binary's run finds it stale, reads the
# jar cold and writes the file anew).
cache_of() { echo "out/instr-cache/$("$1" --version 2> /dev/null | tr -c 'A-Za-z0-9.\n' '-' | tr -d '\n')"; }
cache_master=$(cache_of "$master") cache_landing=$(cache_of "$landing")
# Each binary's spelling of the compiler's verb, probed once here, outside the counts.
. tests/support/compiler-words.sh
spellings "$master" "$landing"
words_master=$(compiler_words "$master") words_landing=$(compiler_words "$landing")
mkdir -p "$cache_master" "$cache_landing"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
# one <binary> <cache> <args>: the instructions retired by one check at one worker, which both binaries are
# asked for whatever their defaults; a failure when the check fails or no positive count comes out of it.
one() {
  local n words=$words_landing
  [ "$1" != "$master" ] || words=$words_master
  if [ "$counter" = time ]; then
    TEQ_THREADS=1 TEQ_CACHE_DIR=$2 /usr/bin/time -l "$1" $words check $3 > /dev/null 2> "$tmp/time.txt" || return 1
    n=$(awk '/instructions retired/ {print $1}' "$tmp/time.txt")
  else
    TEQ_THREADS=1 TEQ_CACHE_DIR=$2 valgrind --tool=cachegrind --cache-sim=no --cachegrind-out-file=/dev/null --log-file="$tmp/time.txt" "$1" $words check $3 > /dev/null 2>&1 || return 1
    n=$(awk '/I +refs:/ {gsub(",", "", $NF); print $NF}' "$tmp/time.txt")
  fi
  [[ "$n" =~ ^[1-9][0-9]*$ ]] || { echo "no count of instructions retired" >> "$tmp/time.txt"; return 1; }
  echo "$n"
}
status=0
verdicts=()
for prog in $wanted; do
  if [[ " $programs " != *" $prog "* ]]; then
    echo "$prog: not among the programs here (its jars missing?)"
    status=1
    verdicts+=("$prog missing")
    continue
  fi
  args=$(program_args "$prog")
  a=()
  b=()
  ok=1
  one "$master" "$cache_master" "$args" > /dev/null && one "$landing" "$cache_landing" "$args" > /dev/null || ok=0
  for r in $(seq 1 "$runs"); do
    [ $ok = 1 ] || break
    x=$(one "$master" "$cache_master" "$args") && y=$(one "$landing" "$cache_landing" "$args") || { ok=0; break; }
    a+=("$x")
    b+=("$y")
  done
  if [ $ok = 0 ]; then
    echo "$prog: a check failed: $(tail -3 "$tmp/time.txt" | head -1)"
    status=1
    verdicts+=("$prog failed")
    continue
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
) || { echo "$prog: the comparison failed: ${line#* * }"; status=1; verdicts+=("$prog failed"); continue; }
  echo "${line#* * }"
  read -r verdict change _ <<< "$line"
  # A program counts only with a verdict: within the tolerance, or recorded apart when that was asked for.
  case "$verdict,${INSTR_TOLERANCE:-}" in
    within,* | recorded,none) ;;
    *) status=1 ;;
  esac
  verdicts+=("$prog $change")
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
