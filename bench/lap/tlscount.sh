#!/bin/bash
# tlscount.sh <name> <program>: how often `teq compiler check` by $LX/teq-<name> reads the thread pointer
# (`mrs tpidr_el0`, a thread-local's address), in all and by function into
# $LX/tls-<name>-<program>.txt: on Linux an instruction, on Darwin a call of `_tlv_get_addr`, some
# nine instructions more, which the container's counts leave out of the host's.
LX=${LX:-/tmp/pt3-lap/lx}; n=$1; prog=$2
args=$("$(dirname "$0")/args.sh" $prog)
B=$(cd "$(dirname "$0")/../.." && pwd)/out/budget; C=$HOME/Library/Caches/Coursier
JH=${JAVA_HOME:-$HOME/.sdkman/candidates/java/current}; J=$(cd $JH && pwd -P)
cp "$(dirname "$0")/tlscount.py" $LX/tlscount.py
timeout 350 docker run --rm -v $LX:/lx -v $B:$B:ro -v $C:$C:ro -v $J:$JH:ro -e JAVA_HOME=$JH teq-vg bash -c "
w=; /lx/teq-$n compiler --help > /dev/null 2>&1 && w=compiler
TEQ_CACHE_DIR=/lx/cache-$n valgrind --tool=callgrind --dump-instr=yes --callgrind-out-file=/tmp/cgi.out /lx/teq-$n \$w check $args > /dev/null 2>&1
objdump -d --no-show-raw-insn /lx/teq-$n | awk '/^[0-9a-f]+ <.*>:\$/{fn=\$2} /mrs.*tpidr_el0/{sub(\":\",\"\",\$1); print \$1, fn}' > /tmp/tls-addrs.txt
python3 /lx/tlscount.py /tmp/cgi.out /tmp/tls-addrs.txt /lx/tls-$n-$prog.txt"
