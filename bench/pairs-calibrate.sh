#!/bin/bash
# bench/pairs-calibrate.sh <dir> <teq A> <teq B> <start> <order> <load> [cycles]: one calibration sequence of the
# machine's timing lines (docs/DEVELOPING.md, "The timing lines on a machine"): the budget and the runtime line of
# bench/pairs.py, A as master and B as the head (one binary twice, or two builds of one commit), <cycles> cycles
# (default 7) and no second attempt, so that bench/pairs.py calibrate can read every window of 3, 5 and 7 of them.
# <start> is the first program's side (master or head), <order> forward or reverse, and <load> the pressure on
# the machine itself during each line: none, arriving (eight busy threads and 4 GB written start at a random
# point of the line, within its first 150 s for the budget line and 75 s for the runtime line, which takes about
# two minutes at seven cycles) or leaving (present from the line's start, stopped at such a point). Each line's runs go to <dir>/<line>/, the pressure's start and stop as epoch seconds to <dir>/load.txt.
dir=$1 a=$2 b=$3 start=$4 order=$5 load=$6 cycles=${7:-7}
[ -n "$load" ] || { echo "usage: bench/pairs-calibrate.sh <dir> <teq A> <teq B> <start> <order> <load> [cycles]" >&2; exit 2; }
case $load in none | arriving | leaving) ;; *) echo "load: none, arriving or leaving" >&2; exit 2 ;; esac
cd "$(dirname "$0")/.." || exit 1
mkdir -p "$dir"
: > "$dir/load.txt"
pressure=()
timer=
press() {
  local i
  [ ${#pressure[@]} = 0 ] || return
  for i in $(seq 1 8); do (while :; do :; done) & pressure+=($!); done
  timeout 900 python3 -c 'import time; b = bytearray(4 << 30); b[::4096] = b"x" * len(b[::4096]); time.sleep(900)' & pressure+=($!)
  echo "start $(date +%s.%N)" >> "$dir/load.txt"
}
release() {
  [ ${#pressure[@]} = 0 ] && return
  kill "${pressure[@]}" 2> /dev/null
  wait "${pressure[@]}" 2> /dev/null
  pressure=()
  echo "stop $(date +%s.%N)" >> "$dir/load.txt"
}
disarm() { [ -z "$timer" ] || kill "$timer" 2> /dev/null; timer=; }
trap 'disarm; release' EXIT
trap press USR1
trap release USR2
flags=(--cycles "$cycles" --no-rerun --start "$start")
[ "$order" = reverse ] && flags+=(--reverse)
for line in ${CAL_LINES:-budget runtime}; do
  at=$((20 + RANDOM % 130))
  [ $line = budget ] || at=$((15 + RANDOM % 60))
  echo "$line $load at ${at}s, from $(date +%s.%N)" >> "$dir/load.txt"
  case $load in
    arriving) (sleep $at; kill -USR1 $$) & timer=$! ;;
    leaving) press; (sleep $at; kill -USR2 $$) & timer=$! ;;
  esac
  # a trapped signal ends the shell's wait for a foreground child, so the line runs in the background
  timeout 1500 python3 bench/pairs.py $line "$a" "$b" "${flags[@]}" --out "$dir/$line" > "$dir/$line.log" 2>&1 &
  child=$!
  for i in $(seq 1 100); do wait $child && break; kill -0 $child 2> /dev/null || break; done
  disarm
  release
done
echo "sequence done $(date +%s)" >> "$dir/load.txt"
