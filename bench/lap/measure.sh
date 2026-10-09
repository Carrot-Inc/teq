#!/bin/bash
# measure.sh <name> <binary> <runs> <programs...>: the minimum instructions retired of `teq compiler check`
# over the runs, one line per program into /tmp/pt3-lap/m-<name>.txt
cd "$(dirname "$0")/../.."
work=out/budget; . bench/programs.sh > /dev/null
name=$1; bin=$2; runs=$3; shift 3
# The binary's spelling of the compiler's verb, probed once here, outside the counts.
. tests/support/compiler-words.sh; spellings "$bin"; words=$(compiler_words "$bin")
cache=/tmp/pt3-lap/cache-$name; mkdir -p $cache
out=/tmp/pt3-lap/m-$name.txt; : > $out
for prog in "$@"; do
  args=$(program_args $prog)
  TEQ_CACHE_DIR=$cache $bin $words check $args > /dev/null 2>&1
  best=""
  for r in $(seq 1 $runs); do
    TEQ_CACHE_DIR=$cache /usr/bin/time -l $bin $words check $args > /dev/null 2> /tmp/pt3-lap/time-$name.txt
    i=$(grep "instructions retired" /tmp/pt3-lap/time-$name.txt | awk '{print $1}')
    if [ -z "$best" ] || [ "$i" -lt "$best" ]; then best=$i; fi
  done
  echo "$prog $best" >> $out
done
cat $out
