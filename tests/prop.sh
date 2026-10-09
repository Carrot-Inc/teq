#!/bin/bash
# The property-based tests of proptests/ (Hegel), against the release binary: a resident
# session against a fresh build under generated edit sequences, generated expressions on the
# three targets and under scalac, and the order of the inputs. A fixed case count and seed per
# property, so that a run is the same as the last; PROP_CASES and PROP_SEED change them, and an
# empty PROP_SEED asks the engine for a fresh seed (`HEGEL_SEED=none`; under a CI profile the
# engine derandomises all the same). Whatever the seed, a failure kept in the database
# directory (HEGEL_DATABASE, out/proptests/.hegel by default) replays before generation. The crate's dependencies come from crates.io
# through cargo: where they were never fetched the suite skips with a message and exits zero,
# so that no gate needs the network; `cargo fetch --locked` in proptests/ fetches them once.
# Every run is bounded: a property that exceeds its bound fails. A relative TEQ, TEQ_PROP_WORK
# or HEGEL_DATABASE is taken from the directory the script is run in.
absolute() { case $1 in /*) echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
[ -z "$TEQ" ] || TEQ=$(absolute "$TEQ")
[ -z "$TEQ_PROP_WORK" ] || TEQ_PROP_WORK=$(absolute "$TEQ_PROP_WORK")
[ -z "$HEGEL_DATABASE" ] || HEGEL_DATABASE=$(absolute "$HEGEL_DATABASE")
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
export TEQ=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
export TEQ_PROP_WORK=${TEQ_PROP_WORK:-$(pwd)/out/proptests}
export HEGEL_TEST_CASES=${PROP_CASES:-100}
export HEGEL_SEED=${PROP_SEED-1}
[ -n "$HEGEL_SEED" ] || export HEGEL_SEED=none
export HEGEL_DATABASE=${HEGEL_DATABASE:-$TEQ_PROP_WORK/.hegel}
logs=$TEQ_PROP_WORK/logs/$$
mkdir -p "$logs" || exit 1
cd proptests || exit 1
if ! timeout 60 cargo metadata --offline --locked --format-version 1 > /dev/null 2>&1; then
  echo "skip: the crates of proptests/ were never fetched (cd proptests && cargo fetch --locked)"
  exit 0
fi
if ! timeout 300 cargo build --tests --offline --locked --quiet 2> "$logs/build.log"; then
  echo "FAIL the crate does not build: $(tail -3 "$logs/build.log")"
  exit 1
fi
pass=0
fail=0
for test in session targets order; do
  if timeout ${PROP_BOUND:-340} cargo test --offline --locked --quiet --test $test -- --test-threads=1 > "$logs/$test.log" 2>&1; then
    pass=$((pass + 1))
  else
    echo "FAIL $test (see $logs/$test.log)"
    grep -B1 -A14 "panicked" "$logs/$test.log" | head -${DIFF_LINES:-30}
    grep -A1 "To reproduce" "$logs/$test.log" | tail -1
    fail=$((fail + 1))
  fi
done
echo "$pass passed, $fail failed"
[ $fail = 0 ]
