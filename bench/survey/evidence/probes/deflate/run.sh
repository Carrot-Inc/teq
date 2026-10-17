#!/bin/bash
# Each flate2 backend against the JDK's deflate outputs (oracle.log), then for C zlib what was linked: the feature
# graph, the binary's dynamic libraries, zlibVersion(). Then a host with system zlib simulated (a libz.so beside the
# bundled headers on the compiler's paths, a target directory apart): the `zlib-c` feature still builds the bundled
# sources, and the control (`flate2/zlib` alone, the feature as the first record had it) links the system library.
cd "$(dirname "$0")"
echo "LIBZ_SYS_STATIC=${LIBZ_SYS_STATIC-unset}"
for b in miniz zlib-rs zlib-c; do
  echo "== $b"
  timeout 400 cargo run --release -q -j 8 --no-default-features --features $b -- oracle.log 2>&1 | tail -40
done
echo "== zlib-c: features"
timeout 120 cargo tree -e features -i libz-sys --no-default-features --features zlib-c 2>&1
echo "== zlib-c: dynamic libraries"
ldd target/release/survey-deflate-probe
headers=$(ls -d "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/libz-sys-1.1.30/src/zlib | head -1)
sys=$PWD/target/system-zlib; mkdir -p $sys/lib; ln -sf /lib/x86_64-linux-gnu/libz.so.1 $sys/lib/libz.so
for f in zlib-c flate2/zlib; do
  echo "== simulated system zlib, --features $f"
  CFLAGS="-I$headers" LIBRARY_PATH=$sys/lib CARGO_TARGET_DIR=$sys/target-${f//\//-} timeout 400 cargo run --release -q -j 8 --no-default-features --features $f -- oracle.log 2>&1 | tail -3
  ldd $sys/target-${f//\//-}/release/survey-deflate-probe | grep -i libz || echo "no libz among the dynamic libraries"
done
echo "== done"
