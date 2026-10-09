#!/bin/bash
# bench/pgo.sh [ship] [gen|train|use]: the release build guided by a profile (docs/SPEED.md, "The
# release build"), the binary the budget measures. gen builds an instrumented binary under
# target/pgo/gen; train runs it over the budget's programs (bench/programs.sh), the application
# corpus built both ways, a watch session and a JVM and an interpreter build, and merges the counts
# into target/pgo/teq.profdata; use builds under target/pgo/use with the profile, the binary
# target/pgo/use/release/teq, whose `teq --version` ends in "pgo", the one bench/budget.sh and the
# gate's budget line measure; target/release/teq stays the plain build. Without a stage, all three.
# `ship` runs the same stages under the ship profile (fat LTO, one codegen unit) in
# target/pgo/ship, and its use copies the binary to target/ship/teq, the one stage.sh distributes
# and the one given off, then runs tests/run.sh on it, since the suites run on the release build,
# and only once it passed writes target/ship/teq.manifest (bench/ship-manifest.sh), the suite's
# result recorded against the binary's digest, which stage.sh checks. A plain `cargo build
# --profile ship` later replaces target/ship/teq and leaves target/pgo alone; the manifest no longer
# names it. train records itself in training.txt beside the profile: the programs, the runs
# merged, the digests of the corpus and the jars it read and of the profile, the release and commit
# trained, the machine and its toolchain (bench/ship-manifest.sh's `training` lines).
# A ship's builds name teq's crate by a metadata of their own (bench/ship-metadata.sh): gen the native build's,
# which train records (`training metadata`), use and stale the one the training of their profile records, so
# that a profile of an earlier release's ship or of another machine finds its functions. use takes
# target/pgo/ship/teq.profdata and its training.txt, or TEQ_PGO_PROFILE=<file> and TEQ_PGO_TRAINING=<its record>
# (bench/ship-profiles.sh's asset unpacked; without a record the binary's manifest says `training none`, and the
# stage refuses it). ship keys [<n>] lists the profile's n hottest functions (200), by their largest block count;
# ship stale builds as use does into target/pgo/ship/stale, neither copied nor tested, and says whether the
# profile has gone stale for this tree: more of its hottest functions changed than $stale_limit (below), exit 3.
# Needs the llvm-tools component (rustup component add llvm-tools). The profile's flags go through
# RUSTFLAGS, which replaces .cargo/config.toml's rustflags on purpose: the profile-guided build
# keeps thin LTO's default import limit, the regime its budgets were recorded on, since at 250 its
# macro rows type 10 to 12% slower on the branch that pinned it (docs/SPEED.md, "The import limit").
absolute() { case $1 in /* | "") echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
TEQ_PGO_PROFILE=$(absolute "${TEQ_PGO_PROFILE:-}")
TEQ_PGO_TRAINING=$(absolute "${TEQ_PGO_TRAINING:-}")
cd "$(dirname "$0")/.." || exit 1
. bench/ship-manifest.sh
. bench/ship-metadata.sh
profile=release
bound=360
mode=
dir=$PWD/target/pgo
if [ "$1" = ship ]; then
  profile=ship
  mode="ship "
  dir=$dir/ship
  # Fat LTO in one codegen unit: about four minutes on a quiet reference machine, past six with
  # several builds beside it.
  bound=1200
  shift
fi
# A ship stage that starts leaves target/ship/teq without the manifest of an earlier build, whatever
# fails after this (a missing profile, the training): use writes it again once the suite passed. keys and
# stale make no binary of target/ship's.
case $profile/${1:-all} in ship/keys | ship/stale) ;; ship/*) rm -f target/ship/teq.manifest target/ship/teq.version ;; esac
host=$(rustc -vV | sed -n 's/^host: //p')
profdata=$(rustc --print sysroot)/lib/rustlib/$host/bin/llvm-profdata
[ -x "$profdata" ] || { echo "pgo: no llvm-profdata; rustup component add llvm-tools"; exit 1; }
# The profile use and stale build with, and its training's record (none for a profile named without one).
use_profile=${TEQ_PGO_PROFILE:-$dir/teq.profdata}
use_training=$dir/training.txt
[ -z "$TEQ_PGO_PROFILE" ] || use_training=$TEQ_PGO_TRAINING

# The ship's Linux binary is linked by the pinned zig against the stubs of glibc $release_glibc (bench/zig.sh),
# on the x86-64 Linux machine the ship is made on, so that it loads on any glibc from the floor on wherever
# it is built; a Mac's ship build (the comparison's on the reference machine) links natively. gen and use both link through it,
# so the instrumented binary exercises the link. The wrapper's text, which cargo does not track, stamps the
# directory: a build under another starts from an empty one. $linker is the environment the builds get.
linker=()
ship_linker() {
  [ $profile = ship ] && [ "$host" = x86_64-unknown-linux-gnu ] && [ ${#linker[@]} -eq 0 ] || return 0
  . bench/zig.sh
  zig_fetch && zig_wrapper $host || return 1
  linker=("CARGO_TARGET_$(echo "$host" | tr a-z- A-Z_)_LINKER=$zig_cc" "TEQ_CROSS_LOG=$dir/link")
  local stamp
  stamp=$(sha256sum < "$zig_cc" | cut -c1-16)
  if [ "$(cat "$dir/linker.stamp" 2> /dev/null)" != "$stamp" ]; then
    rm -rf "$dir/gen" "$dir/use" && mkdir -p "$dir" && echo "$stamp" > "$dir/linker.stamp" || return 1
  fi
  echo "pgo: linked by zig $zig_version for $(zig_target $host), the wrapper ${zig_cc#$PWD/}"
}

# ship_metadata [<metadata>]: a ship build's crate metadata, the one given or the native build's (bench/ship-metadata.sh's
# probe, 300 s), in $metadata, and the wrapper that gives it in $meta; none for the release profile's builds.
meta=() metadata=
ship_metadata() {
  [ $profile = ship ] || return 0
  mkdir -p "$dir/bin" && metadata_wrapper "$dir/bin/rustc" || return 1
  metadata=${1:-}
  [ -n "$metadata" ] || metadata=$(metadata_probe "$dir/bin/rustc" "$dir") || { echo "pgo: no metadata read for teq's native build"; return 1; }
  meta=(RUSTC_WRAPPER="$dir/bin/rustc" TEQ_CROSS_METADATA="$metadata")
  echo "pgo: teq's crate under the metadata $metadata"
}
# restamp <directory>: a build directory under target/pgo/ship emptied when its metadata or the wrapper's text,
# which cargo does not track, is another than its last build's.
restamp() {
  [ $profile = ship ] || return 0
  local stamp
  stamp="$metadata $(sha256sum < "$dir/bin/rustc" | cut -c1-16)"
  [ "$(cat "$dir/$1.stamp" 2> /dev/null)" = "$stamp" ] || { rm -rf "${dir:?}/$1" && echo "$stamp" > "$dir/$1.stamp"; }
}

gen() {
  ship_linker && ship_metadata && restamp gen || return 1
  rm -f "$dir/gen.metadata"
  env "${linker[@]}" "${meta[@]}" RUSTFLAGS="$RUSTFLAGS -Cprofile-generate=$dir/raw" timeout "$bound" cargo build --profile $profile --target-dir "$dir/gen" 2>&1 | grep -E '^error|Finished'
  [ "${PIPESTATUS[0]}" -eq 0 ] || return 1
  [ -z "$metadata" ] || echo "$metadata" > "$dir/gen.metadata"
}

# corpus: the files the training reads, the generated programs and the application corpus by their
# paths in the tree, the jars by their names, as `<sha256>  <name>` lines.
corpus() {
  local t prev= files=() jars=() cp
  for t in $(for p in $programs; do program_args "$p"; done) "$work/core-only" "$work/app" tests/cases/end_markers.scala --classpath "$app_cp"; do
    if [ "$prev" = --classpath ]; then
      IFS=: read -ra cp <<< "$t"
      jars+=("${cp[@]}")
    elif [ -e "$t" ]; then
      files+=("$t")
    fi
    prev=$t
  done
  find "${files[@]}" -type f | LC_ALL=C sort -u | xargs $manifest_sha256_tool > "$dir/corpus.txt" || return 1
  for t in $(printf '%s\n' "${jars[@]}" | LC_ALL=C sort -u); do
    echo "$(manifest_sha256 "$t")  ${t##*/}"
  done > "$dir/jars.txt"
}

train() {
  local teq=$dir/gen/$profile/teq out=$dir/out work=out/budget
  rm -f "$dir/training.txt"
  [ -x "$teq" ] || { echo "pgo: no instrumented build; $0 ${mode}gen"; return 1; }
  . bench/programs.sh || return 1
  # A profile trained without a program moves that program's rows: every program or none.
  [ -z "$left_out" ] || { printf 'pgo: not training, programs left out for missing jars:\n%s' "$left_out"; return 1; }
  rm -rf "$dir/raw" "$out" && mkdir -p "$dir/raw" "$out"
  export LLVM_PROFILE_FILE="$dir/raw/teq-%p-%m.profraw"
  # Each run's bound: a ship's native run takes seconds (7 s for the slowest on four x86-64 cores, the 65
  # checks 46 s together), so 60; an emulated trainer's (TEQ_PGO_RUN_BOUND, bench/cross-ship.sh
  # arm under qemu-user) and the release profile's 120. bench/ship.sh bounds the training by their sum.
  local run_bound=120
  [ $profile != ship ] || run_bound=${TEQ_PGO_RUN_BOUND:-60}
  run() { timeout "$run_bound" "$teq" "$@" < /dev/null > /dev/null 2>&1 || { echo "pgo: training run failed: teq $*"; return 1; }; }
  for p in $programs; do
    run compiler check $(program_args "$p") || return 1
  done
  run compiler build "$work/core-only" -o "$out/core.js" &&
    run compiler build "$work/core-only" --release -o "$out/core-release.js" &&
    run compiler build "$work/core-only" --target jvm -o "$out/core.jar" &&
    run interp tests/cases/end_markers.scala &&
    run compiler build "$work/app/shared" "$work/app/frontend" --classpath "$app_cp" --split "$out/frontend" &&
    run compiler build "$work/app/shared" "$work/app/api" --classpath "$app_cp" --release -o "$out/api.js" || return 1
  # The session is trained on a batch run's allocator: with the counts of a resident's, whose
  # lists are refilled a hundred times as often, at every allocation site, the rows of macros
  # and given searches type 4 to 9% slower at the same instructions (mac_4, mac_16, gcx_1, gcx_64).
  printf 'build\n\nquit\n' | TEQ_RESIDENT=0 timeout 120 "$teq" compiler watch "$work/core-only" --split "$out/watch" > /dev/null 2>&1 || { echo "pgo: training watch session failed"; return 1; }
  timeout 120 "$profdata" merge -o "$dir/teq.profdata" "$dir"/raw/*.profraw || return 1
  local runs
  runs=$(ls "$dir"/raw | wc -l | tr -d ' ')
  echo "pgo: $runs runs merged into ${dir#$PWD/}/teq.profdata"
  corpus || return 1
  {
    echo "training status ok"
    echo "training programs $(echo $programs | wc -w)"
    echo "training runs $runs"
    echo "training corpus $(manifest_sha256 "$dir/corpus.txt") $(wc -l < "$dir/corpus.txt" | tr -d ' ') files"
    echo "training jars $(manifest_sha256 "$dir/jars.txt") $(wc -l < "$dir/jars.txt" | tr -d ' ') jars"
    echo "training profile $(manifest_sha256 "$dir/teq.profdata")"
    [ ! -f "$dir/gen.metadata" ] || echo "training metadata $(cat "$dir/gen.metadata")"
    # The ship that trained it, and where: this machine's own build, its toolchain (bench/ship-manifest.sh). The
    # aarch64 trainer's training (bench/cross-ship.sh arm) writes these lines of its own.
    [ ! -f Cargo.toml ] || echo "training release $(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' Cargo.toml | head -1) $(git rev-parse HEAD 2> /dev/null)"
    echo "training host $host"
    echo "training trainer $host native"
    manifest_tuple | sed 's/^/training /'
  } > "$dir/training.txt"
  echo "pgo: $(sed -n 's/^training corpus /corpus /p; s/^training jars /jars /p' "$dir/training.txt" | tr '\n' ' ')in ${dir#$PWD/}/training.txt"
}

# guided <directory>: the build guided by the profile into target/pgo[/ship]/<directory>, its log <directory>.log,
# under the metadata its training records; $flags its flags, $training its record or -.
guided() {
  local log=$dir/$1.log status
  [ -f "$use_profile" ] || { echo "pgo: no profile at $use_profile; $0 ${mode}gen && $0 ${mode}train, or TEQ_PGO_PROFILE=<file>"; return 1; }
  training=-
  [ -z "$use_training" ] || [ ! -f "$use_training" ] || [ "$(manifest_get "$use_training" "training profile")" != "$(manifest_sha256 "$use_profile")" ] || training=$use_training
  flags="$RUSTFLAGS -Cprofile-use=$use_profile -Cllvm-args=-pgo-warn-missing-function"
  ship_linker || return 1
  if [ "$training" != - ] && [ -n "$(manifest_get "$training" "training metadata")" ]; then ship_metadata "$(manifest_get "$training" "training metadata")" || return 1; fi
  restamp "$1" || return 1
  env "${linker[@]}" "${meta[@]}" RUSTFLAGS="$flags" timeout "$bound" cargo build --profile $profile --target-dir "$dir/$1" > "$log" 2>&1
  status=$?
  grep -E '^error|Finished' "$log"
  [ $status -eq 0 ] || return 1
  echo "pgo: $(grep -c 'no profile data available for function' "$log") functions without a profile, after a fresh training about 1,100 under release and 900 under ship, more with a stale profile; $(grep -c 'hash mismatch' "$log") whose control flow differs from the profile's"
}

# keys [<n>]: the profile's n hottest functions, by their largest block count, as LLVM names them (a local one
# without its module's prefix, as the build's warnings name it).
keys() {
  [ -f "$use_profile" ] || { echo "pgo: no profile at $use_profile" >&2; return 1; }
  "$profdata" show --topn="${1:-200}" "$use_profile" | sed -n 's/^  \([^;, ]*;\)\{0,1\}\([^, ]*\), max count = [0-9]*$/\2/p'
}

# stale: whether the profile has gone stale for this tree, by the guided build's own report: of the profile's
# $stale_hot hottest functions, those whose control flow the build finds changed (LLVM's `hash mismatch`
# warning, on by default; -pgo-warn-mismatch is no option of LLVM's, -no-pgo-warn-mismatch turns it off). A
# changed function falls back to unguided code; a few each release are the profile's wear, many its end. Past
# $stale_limit the release trains afresh. Set from the releases on record, each tree's guided build under the
# earlier profile's metadata: 0.1.5's profile against 0.1.6's tree, 3 of the 200 hottest changed (179 functions
# in all, 2.0% of the profile's weight on them); 0.1.6's against the tree 0.1.7 is released from, 5 (298, 3.4%);
# 0.1.5's against that tree, two releases on, 7 (423, 4.3%); a fresh profile against its own tree, none. At that
# pace a profile serves two releases and the third trains. Writes target/pgo/ship/stale.txt; exits 0 when the
# profile serves, 3 when stale.
stale_hot=200 stale_limit=10
stale() {
  local hot changed n
  guided stale || return 1
  hot=$(keys $stale_hot) || return 1
  changed=$(sed -n 's/.*hash mismatch) \([^ ]*\) Hash = .*/\1/p' "$dir/stale.log" | LC_ALL=C sort -u)
  n=$(LC_ALL=C comm -12 <(LC_ALL=C sort -u <<< "$hot") <(echo "$changed") | grep -c .)
  {
    echo "stale $n of the $(grep -c . <<< "$hot") hottest functions changed, $stale_limit allowed"
    echo "stale changed $(grep -c . <<< "$changed") functions in all, $(grep -c 'no profile data available for function' "$dir/stale.log") without a record"
    echo "stale profile $(manifest_sha256 "$use_profile") $([ "$training" = - ] || manifest_get "$training" "training release")"
  } > "$dir/stale.txt"
  sed 's/^stale /pgo: /' "$dir/stale.txt"
  [ "$n" -le "$stale_limit" ] || { echo "pgo: the profile is stale for this tree: $n of its $stale_hot hottest functions changed, past $stale_limit"; return 3; }
}

use() {
  local training flags
  guided use || return 1
  local binary=${dir#$PWD/}/use/$profile/teq
  if [ $profile = ship ]; then
    binary=target/ship/teq
    mkdir -p target/ship && rm -f $binary && cp "$dir/use/ship/teq" $binary || return 1
  fi
  echo "pgo: $binary: $($binary --version)"
  [ $profile = ship ] || return 0
  local smoke sha
  sha=$(manifest_sha256 $binary)
  # The suite into out/tests, emptied first, at its own bound: the outputs the Linux aarch64 suite compares with.
  rm -rf out/tests
  smoke=$(TEQ=./$binary TEQ_TEST_OUT=out/tests TEQ_TEST_TIMEOUT=20 timeout 300 tests/run.sh 2>&1)
  # The checkout's path as the file system spells it, which a macro writes into an output (sourcecode's File):
  # bench/cross-ship.sh's comparison of another machine's suite with this one normalises both by their roots.
  pwd -P > out/tests/.root
  local smoke_status=$?
  echo "$smoke" > "$dir/suite.log"
  [ $smoke_status -eq 0 ] || echo "$smoke"
  # tests/run.sh counts a test whose jars are missing as passed; the manifest says how many.
  local skipped
  skipped=$(grep -c '^skip ' <<< "$smoke")
  echo "pgo: tests/run.sh on $binary: $(tail -1 <<< "$smoke"), $skipped of them skipped for missing jars"
  [ $smoke_status -eq 0 ] || return 1
  [ "$(manifest_sha256 $binary)" = "$sha" ] || { echo "pgo: $binary changed while its suite ran"; return 1; }
  local floor=()
  # The floor the zig-linked binary's own tables say, against the release's.
  if [ ${#linker[@]} -gt 0 ]; then
    local glibc
    glibc=$(manifest_glibc $binary) || return 1
    [ "$(printf '%s\n%s\n' "${glibc%% *}" "$release_glibc" | sort -V | tail -1)" = "$release_glibc" ] || { echo "pgo: $binary loads on glibc ${glibc%% *} and later, past the floor $release_glibc"; return 1; }
    echo "pgo: $binary loads on glibc ${glibc%% *} and later; needs ${glibc#* needs }"
    floor=("glibc $glibc")
  fi
  manifest_write $binary "$host" "$(echo $flags)" "$training" "profile $(manifest_sha256 "$use_profile") ${use_profile#$PWD/}" \
    "suite $sha tests/run.sh passed: $(tail -1 <<< "$smoke"), $skipped of them skipped for missing jars" "${floor[@]}" || return 1
  echo "pgo: $binary.manifest"
}

case ${1:-all} in
  gen) gen ;;
  train) train ;;
  use) use ;;
  keys) keys "${2:-}" ;;
  stale) [ $profile = ship ] || { echo "pgo: stale is a ship's"; exit 2; }; stale ;;
  all) gen && train && use ;;
  *) echo "usage: $0 [ship] [gen|train|use|keys [<n>]|stale]"; exit 2 ;;
esac
