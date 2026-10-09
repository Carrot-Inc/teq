#!/bin/bash
# rot.sh <runs> <program> <binary>...: instructions retired of `teq compiler check`, the binaries taking turns; min and median per binary
runs=$1; prog=$2; shift 2
args=$("$(dirname "$0")/args.sh" $prog)
declare -a bins=("$@")
# Each binary's spelling of the compiler's verb, probed once here, outside the counts.
. "$(dirname "$0")/../../tests/support/compiler-words.sh"; spellings "${bins[@]}"
for b in "${bins[@]}"; do c=/tmp/pt3-lap/rotcache/$(echo $b | md5 | cut -c1-8); mkdir -p $c; TEQ_CACHE_DIR=$c $b $(compiler_words "$b") check $args > /dev/null 2>&1; done
out=/tmp/pt3-lap/rot-$prog.txt; : > $out
for r in $(seq 1 $runs); do
  for b in "${bins[@]}"; do
    c=/tmp/pt3-lap/rotcache/$(echo $b | md5 | cut -c1-8)
    w=$(compiler_words "$b")
    TEQ_CACHE_DIR=$c /usr/bin/time -l $b $w check $args > /dev/null 2> /tmp/pt3-lap/rot-time.txt
    echo "$b $(grep 'instructions retired' /tmp/pt3-lap/rot-time.txt | awk '{print $1}')" >> $out
  done
done
python3 - $out "$@" <<'PY'
import sys, statistics, collections
d = collections.defaultdict(list)
for line in open(sys.argv[1]):
    b, n = line.split(); d[b].append(int(n))
base = min(d[sys.argv[2]])
for b in sys.argv[2:]:
    v = d[b]
    print(f'{min(v)/1e6:10.2f}M min {(min(v)/base-1)*100:+.2f}%  median {statistics.median(v)/1e6:10.2f}M  {b}')
PY
