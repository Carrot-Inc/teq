#!/bin/bash
# cgr5.sh <name> [base]: cachegrind over the five programs of the instruction rule, two at a time,
# and each program's delta against the binary <base> (default master) counted the same way.
LX=${LX:-/tmp/pt3-lap/lx}; n=$1; base=${2:-master}; here=$(dirname "$0")
for pair in "depth_1 gview_1" "core-only realistic-api" "realistic-frontend"; do
  for p in $pair; do $here/cgr.sh $n $p > /dev/null & done; wait
done
for p in depth_1 gview_1 core-only realistic-api realistic-frontend; do
  python3 $here/fdiff.py $LX/cgr-$base-$p.fn $LX/cgr-$n-$p.fn 0 | sed "s/^/$p: /"
done
