#!/bin/bash
# bench/app/race-loop.sh <teq> <runs> [<parallel> [<cpus> [<threads>]]]: bench/app/api-check.sh's check of the
# application's API, <runs> times in batches of <parallel> at once (default 2), each check with --threads <threads>
# (default 16) and, with <cpus>, pinned to those CPUs by `taskset -c` (`-` for none; four of the CPUs the process may
# run on oversubscribe sixteen workers): the loop that measured the typer's crash under load. APP_ROOT is the
# application's checkout, as for api-check.sh, read and
# never written. Prints one line per check, `<outcome> <exit> <seconds>`, the outcome `completed` (exit 0, or 1 whose
# one error is the application's several entry points, which a check without --main cannot choose from), `sigsegv`,
# `timeout` (170 s) or `failed` (any other exit or diagnostic: a check that stops before the typing does not count);
# then the counts. A check that did not complete keeps its output under RACE_KEEP when it is set. Exits 1 when any
# check did not complete, 2 on a malformed command line (<runs>, <parallel> and <threads> positive counts). A batch is
# bounded by the timeout, so the whole by <runs>/<parallel> times 170 s.
teq=$1
runs=$2
parallel=${3:-2}
cpus=${4:--}
threads=${5:-16}
[ -x "$teq" ] && [ -n "$runs" ] || { echo "usage: bench/app/race-loop.sh <teq> <runs> [<parallel> [<cpus> [<threads>]]]" >&2; exit 2; }
for count in "$runs" "$parallel" "$threads"; do
  [[ "$count" =~ ^[1-9][0-9]*$ ]] || { echo "race-loop: <runs>, <parallel> and <threads> must be positive counts, not $runs, $parallel, $threads" >&2; exit 2; }
done
case $teq in /*) ;; *) teq=$PWD/$teq ;; esac
[ -n "$APP_ROOT" ] && [ -f "$APP_MODULES" ] && [ -f "$APP_CLASSPATH" ] && [ -n "${APP_FLAGS+set}" ] || { echo "race-loop: set APP_ROOT, APP_MODULES, APP_CLASSPATH and APP_FLAGS"; exit 1; }
app=$APP_ROOT
last=$(grep -v '^#' "$APP_MODULES" | grep . | tail -1 | cut -d' ' -f1)
[ -d "$app/$last" ] || { echo "race-loop: no application checkout at $app (no $last)"; exit 1; }
cp=$(sed "s|^~/|$HOME/|" "$APP_CLASSPATH" | paste -sd: -)
sources=$(grep -v '^#' "$APP_MODULES" | tr '\n' ' ')
pin=()
if [ "$cpus" != - ]; then
  # The CPUs must be among the ones the process may run on (/proc/self/status, Cpus_allowed_list).
  taskset -c "$cpus" true || { echo "race-loop: cannot pin to CPUs $cpus ($(grep Cpus_allowed_list /proc/self/status))"; exit 2; }
  pin=(taskset -c "$cpus")
fi
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
# one <n>: check <n>, its line written to $tmp/<n>.line.
one() {
  local start end code outcome
  start=$(date +%s%N)
  (cd "$app" && timeout 170 "${pin[@]}" "$teq" compiler check $sources --classpath "$cp" $APP_FLAGS \
    --std scala-library --target jvm --threads "$threads") > "$tmp/$1.out" 2>&1
  code=$?
  end=$(date +%s%N)
  case $code in
    0) outcome=completed ;;
    1)
      # The check's one error is the entry-point choice a check without --main leaves open.
      if [ "$(tail -1 "$tmp/$1.out")" = "1 error found" ] && [ "$(grep -c ': error: ' "$tmp/$1.out")" = 1 ] \
        && grep -q ': error: several entry points: ' "$tmp/$1.out"; then
        outcome=completed
      else
        outcome=failed
      fi
      ;;
    124) outcome=timeout ;;
    139) outcome=sigsegv ;;
    *) outcome=failed ;;
  esac
  if [ $outcome != completed ] && [ -n "$RACE_KEEP" ]; then
    mkdir -p "$RACE_KEEP" && cp "$tmp/$1.out" "$RACE_KEEP/check-$$-$1.out"
  fi
  printf '%s %s %d.%02d\n' $outcome $code $(((end - start) / 1000000000)) $(((end - start) / 10000000 % 100)) > "$tmp/$1.line"
}
n=0
while [ $n -lt "$runs" ]; do
  batch=()
  for ((k = 0; k < parallel && n < runs; k++)); do
    one $n 2>/dev/null &
    batch+=($!)
    n=$((n + 1))
  done
  wait "${batch[@]}"
done
cat "$tmp"/*.line
completed=$(cat "$tmp"/*.line | grep -c '^completed ')
sigsegv=$(cat "$tmp"/*.line | grep -c '^sigsegv ')
timeout=$(cat "$tmp"/*.line | grep -c '^timeout ')
failed=$(cat "$tmp"/*.line | grep -c '^failed ')
echo "race-loop: $runs checks, $parallel at once, cpus $cpus, --threads $threads: completed $completed, sigsegv $sigsegv, timeout $timeout, failed $failed"
[ "$completed" = "$runs" ]
