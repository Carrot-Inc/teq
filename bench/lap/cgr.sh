#!/bin/bash
# cgr.sh <name> <program>: cachegrind's instruction count of `teq compiler check` by $LX/teq-<name> in the
# container, by function (and by line with a line-tables build) in $LX/cgr-<name>-<program>.fn.
# The program's inputs, the coursier jars and the JDK are mounted at the host's paths, so the
# arguments are the host's; the jar cache is warmed by a run first.
LX=${LX:-/tmp/pt3-lap/lx}; n=$1; prog=$2
args=$("$(dirname "$0")/args.sh" $prog)
B=$(cd "$(dirname "$0")/../.." && pwd)/out/budget; C=$HOME/Library/Caches/Coursier
JH=${JAVA_HOME:-$HOME/.sdkman/candidates/java/current}; J=$(cd $JH && pwd -P)
timeout 350 docker run --rm -v $LX:/lx -v $B:$B:ro -v $C:$C:ro -v $J:$JH:ro -e JAVA_HOME=$JH teq-vg bash -c "mkdir -p /lx/cache-$n; w=; /lx/teq-$n compiler --help > /dev/null 2>&1 && w=compiler; TEQ_CACHE_DIR=/lx/cache-$n /lx/teq-$n \$w check $args > /dev/null 2>&1; TEQ_CACHE_DIR=/lx/cache-$n valgrind --tool=cachegrind --cache-sim=no --cachegrind-out-file=/lx/cgr-$n-$prog.out /lx/teq-$n \$w check $args > /dev/null 2> /lx/cgr-$n-$prog.log; cg_annotate --threshold=0 --no-annotate --show-percs=no /lx/cgr-$n-$prog.out > /lx/cgr-$n-$prog.fn 2>&1; grep refs /lx/cgr-$n-$prog.log"
