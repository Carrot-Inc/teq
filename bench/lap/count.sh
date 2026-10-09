#!/bin/bash
# count.sh <binary> <cache> <programs...>: instructions retired and page reclaims of `teq compiler check`, one run each
cd "$(dirname "$0")/../.."
work=out/budget; . bench/programs.sh > /dev/null
bin=$1; cache=$2; shift 2
# The binary's spelling of the compiler's verb, probed once here, outside the counts.
. tests/support/compiler-words.sh; spellings "$bin"; words=$(compiler_words "$bin")
for prog in "$@"; do
  args=$(program_args $prog)
  TEQ_CACHE_DIR=$cache /usr/bin/time -l $bin $words check $args > /dev/null 2> /tmp/pt3-lap/time.txt
  i=$(grep "instructions retired" /tmp/pt3-lap/time.txt | awk '{print $1}')
  r=$(grep "page reclaims" /tmp/pt3-lap/time.txt | awk '{print $1}')
  m=$(grep "maximum resident" /tmp/pt3-lap/time.txt | awk '{print $1}')
  printf "%-18s instr %12s  reclaims %7s  maxrss %11s\n" $prog $i $r $m
done
