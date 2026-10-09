#!/bin/bash
# The compiler's own Rust warnings, denied: `cargo check` of every target (the binary and its unit tests) under
# the release profile and under the assertion-enabled one (`checks`), which compile different code (what
# `cfg(debug_assertions)` gates), each with `-D warnings` added to the tree's flags. The checks have a target
# directory of their own (target/rust-warnings): the flag changes what cargo builds, which the gate's stamped build
# refuses from the environment (tests/support/build-manifest.sh), so it stays out of that build and its directory.
# A check's warnings are printed; the last line says whether both checks were free of them.
cd "$(dirname "$0")/.."
fail=0
for profile in release checks; do
  if ! out=$(CARGO_TARGET_DIR=target/rust-warnings timeout 1200 cargo check --profile "$profile" --all-targets --message-format short --config 'build.rustflags = ["-D", "warnings"]' 2>&1); then
    echo "$out" | grep -E '^(src/|build\.rs|error)' | head -40
    echo "FAIL the $profile profile's check"
    fail=1
  fi
done
if [ $fail = 0 ]; then echo "rust warnings: none in the release and checks profiles' targets"; else echo "rust warnings: denied ones found"; fi
exit $fail
