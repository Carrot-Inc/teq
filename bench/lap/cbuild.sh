#!/bin/bash
# cbuild.sh <name> <tree> [lt]: a release build of <tree> in the arm64 Linux container, the binary
# to $LX/teq-<name>. The CPU is an Apple M1 (apple-m1) but for its RCpc loads, which valgrind
# cannot decode; with `lt` the build carries line tables, for counts by source line.
# The image: `docker build -t teq-vg bench/lap/container`. $LX (default /tmp/pt3-lap/lx) holds
# the binaries and the counts; the tree must lie under it. The CPU flags go through --config,
# which cargo appends to .cargo/config.toml's rustflags, where RUSTFLAGS would replace them.
LX=${LX:-/tmp/pt3-lap/lx}; n=$1; tree=$2
debug=""; [ "$3" = lt ] && debug="CARGO_PROFILE_RELEASE_DEBUG=line-tables-only"
timeout 350 docker run --rm -v $LX:/lx -v teq-target:/target teq-vg bash -c "cd /lx/${tree#$LX/} && $debug CARGO_TARGET_DIR=/target/$n cargo build --release --config 'build.rustflags=[\"-C\", \"target-cpu=apple-m1\", \"-C\", \"target-feature=-rcpc,-rcpc-immo\"]' > /lx/build-$n.log 2>&1; cp /target/$n/release/teq /lx/teq-$n && echo built-$n"
