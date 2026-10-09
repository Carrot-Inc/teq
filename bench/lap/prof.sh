#!/bin/bash
# prof.sh <name> <binary> <program> <repeats>: leaf-function sample counts of `teq compiler check`, accumulated
cd "$(dirname "$0")/../.."
work=out/budget; . bench/programs.sh > /dev/null
name=$1; bin=$2; prog=$3; reps=$4
args=$(program_args $prog)
# The binary's spelling of the compiler's verb, probed once here, outside the samples.
. tests/support/compiler-words.sh; spellings "$bin"; words=$(compiler_words "$bin")
cache=/tmp/pt3-lap/cache-$name; mkdir -p $cache
TEQ_CACHE_DIR=$cache $bin $words check $args > /dev/null 2>&1
: > /tmp/pt3-lap/leaves-$name.txt
for r in $(seq 1 $reps); do
  TEQ_CACHE_DIR=$cache $bin $words check $args > /dev/null 2>&1 &
  pid=$!
  sample $pid 3 1 -mayDie -file /tmp/pt3-lap/sample-$name-$r.txt > /dev/null 2>&1
  wait $pid
  awk '/Sort by top of stack/{p=1;next} /Binary Images/{p=0} p' /tmp/pt3-lap/sample-$name-$r.txt | grep -E "^\s+[0-9]+ " >> /tmp/pt3-lap/leaves-$name.txt
done
python3 - "$name" <<'PY'
import sys, re, collections
name = sys.argv[1]
tot = collections.Counter(); total = 0
for line in open(f"/tmp/pt3-lap/leaves-{name}.txt"):
    m = re.match(r"\s*(\d+)\s+(.*?)\s+\(in ", line)
    if not m: continue
    n = int(m.group(1)); fn = re.sub(r"<[^<>]*>", "", m.group(2))[:90]
    tot[fn] += n; total += n
print(f"{name}: {total} samples")
for fn, n in tot.most_common(28):
    print(f"{n:6} {100*n/total:5.1f}%  {fn}")
PY
