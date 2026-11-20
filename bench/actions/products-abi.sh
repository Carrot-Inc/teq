#!/bin/bash
# bench/actions/products-abi.sh <binary>: the number of the products the binary writes and reads (`teq --version
# --verbose`, its line `products ABI <n>`) is the source's (src/products.rs, `PRODUCTS_ABI`), and the environment sets
# no TEQ_PRODUCTS_ABI_BUMP, the build-time variable that raises the number for tests/products-abi.sh's second binary
# alone. bench/actions/qualify.sh runs it on every release binary, from the root of the checkout it is built from
# (bench/ship.sh's builds unset the variable); tests/release-actions.sh runs it on stand-ins. Exits 1 on a
# difference, saying what it saw, 2 on a wrong call.
set -uo pipefail
cd "$(dirname "$0")/../.." || exit 1
[ $# = 1 ] || { echo "usage: bench/actions/products-abi.sh <binary>" >&2; exit 2; }
fail() { echo "products-abi: $*"; exit 1; }
[ -z "${TEQ_PRODUCTS_ABI_BUMP+set}" ] || fail "TEQ_PRODUCTS_ABI_BUMP is set ('$TEQ_PRODUCTS_ABI_BUMP'), which no release build takes"
source=$(sed -n 's/^pub const PRODUCTS_ABI: u32 = \([0-9][0-9]*\);$/\1/p' src/products.rs)
[ -n "$source" ] || fail "src/products.rs names no PRODUCTS_ABI"
out=$(timeout 30 "$1" --version --verbose | tr -d '\r') || fail "$1 --version --verbose failed: $out"
binary=$(printf '%s\n' "$out" | sed -n 's/^products ABI \([0-9][0-9]*\)$/\1/p')
[ -n "$binary" ] || fail "$1 prints no products ABI: $out"
[ "$binary" = "$source" ] || fail "$1 reads products of ABI $binary, the source's PRODUCTS_ABI is $source"
echo "products-abi: the binary reads products of ABI $binary, the source's"
