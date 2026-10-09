#!/bin/bash
# The compiler as tests/fork-one.sh runs a suite with it: the real binary (FORK_ONE_TEQ) runs as
# the suite asked, its arguments, input and streams where the suite put them, and the invocation
# is kept under FORK_ONE_RECORDS for the runner's failure digests: the arguments, the directory,
# the exit or the signal, and everything the binary wrote to each stream. A stream to a regular
# file is read back from the size the file had at the start, so that the binary writes the file
# itself; a pipe or a fifo is copied through `tee` as the binary writes it, one copy for both
# streams where they are one, so that their order holds. With the checks' log asked for
# (TEQ_VIEW_CHECKS_LOG), the binary's process and its outcome go there too (`exit <pid>
# <status>`), which the runner reconciles with the invocation's own lines. A signal that ended
# the binary ends this script too, so that the suite's shell sees what it would have.
real=${FORK_ONE_TEQ:?the real binary}
[ -n "$FORK_ONE_RECORDS" ] && [ -d "/proc/$$/fd" ] || exec "$real" "$@"
rec=$(mktemp -d "$FORK_ONE_RECORDS/r.XXXXXXXX") || exec "$real" "$@"
printf '%s\n' "$@" > "$rec/args"
pwd > "$rec/cwd"
out=$(readlink "/proc/$$/fd/1")
err=$(readlink "/proc/$$/fd/2")
kind() {
  case $1 in
    /dev/null | "") echo none ;;
    *) if [ -f "$1" ]; then echo file; else echo stream; fi ;;
  esac
}
ko=$(kind "$out")
ke=$(kind "$err")
# One stream for both (`2>&1`): the compiler's reports and the program's output together.
[ "$ko" != none ] && [ "$out" = "$err" ] && : > "$rec/merged"
[ "$ko" = file ] && so=$(stat -c %s "$out")
[ "$ke" = file ] && [ "$err" != "$out" ] && se=$(stat -c %s "$err")
child=
forward() { [ -n "$child" ] && kill -s "$1" "$child" 2> /dev/null; }
trap 'forward TERM' TERM
trap 'forward INT' INT
trap 'forward HUP' HUP
if [ "$ko" = stream ] && [ "$out" = "$err" ]; then
  "$real" "$@" <&0 > >(tee -a "$rec/out") 2>&1 &
elif [ "$ko" = stream ] && [ "$ke" = stream ]; then
  "$real" "$@" <&0 > >(tee -a "$rec/out") 2> >(tee -a "$rec/err" >&2) &
elif [ "$ko" = stream ]; then
  "$real" "$@" <&0 > >(tee -a "$rec/out") &
elif [ "$ke" = stream ]; then
  "$real" "$@" <&0 2> >(tee -a "$rec/err" >&2) &
else
  "$real" "$@" <&0 &
fi
child=$!
status=0
# The shell's report of a signal that ended its job stays out of the suite's streams: the suite's
# own shell reports this script's end by the same signal, as it would the binary's.
for _ in $(seq 1 100000); do
  wait "$child" 2> /dev/null
  status=$?
  kill -0 "$child" 2> /dev/null || break
done
[ "$ko" = file ] && tail -c +$((so + 1)) "$out" >> "$rec/out"
[ -n "$se" ] && tail -c +$((se + 1)) "$err" >> "$rec/err"
echo "$status" > "$rec/status"
[ -n "$TEQ_VIEW_CHECKS_LOG" ] && echo "view checks: exit $child $status" >> "$TEQ_VIEW_CHECKS_LOG"
if [ "$status" -gt 128 ] && [ "$status" -lt 160 ]; then
  sig=$((status - 128))
  trap - "$sig" 2> /dev/null
  ulimit -c 0
  kill -s "$sig" $$
fi
exit "$status"
