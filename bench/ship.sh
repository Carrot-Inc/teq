#!/bin/bash
# bench/ship.sh [--publish [--plugin <version>] | --step <step>[,<step>...] --carry <dir>] [<revision>]: the ship,
# made on a Linux x86-64 machine (docs/SPEED.md, "The ship from Linux"), of the release the checkout names
# (docs/TARGETS.md, "Releases": the version Cargo.toml and Cargo.lock agree on, which bench/release.sh bumps in a
# commit of its own). It resolves the revision (the
# head by default) to its commit, refuses a checkout with changes and a commit whose compiler sources are not the
# head's, and builds in a worktree of that commit it makes itself (target/ship-tree, replaced at each run), so that
# nothing edited during the run enters a binary: the macOS arm64 binary by bench/cross-ship.sh (arm, the
# aarch64 trainer under qemu-user; use, the guided Darwin build), the Linux x86-64 one by bench/pgo.sh ship (gen,
# train, use with its tests/run.sh; linked by zig against glibc 2.28, bench/zig.sh), the macOS x86-64 one by
# bench/cross-ship.sh intel (guided by the Linux training's profile), the Linux aarch64 one by bench/cross-ship.sh
# linux-arm (guided by the aarch64 trainer's, its tests/run.sh under qemu-user over the floor's glibc) and the
# Windows x86-64 one by bench/cross-ship.sh windows (plain, its smoke under wine). Both trainings must have read
# the same corpus (bench/app's and the budget's programs, their jars). Then it empties
# integrations/sbt/binary/binaries/, stages the five (stage.sh, which checks each against its manifest), holds
# back those bench/ship-qualified.txt does not publish yet, checks the set as the release's draft would
# (integrations/sbt/binary/check.sh), and with
# --publish publishes it as the release by bench/ship-publish.sh, in its order: the GitHub release as a draft with
# every asset, the plugin staged, the smoke against the staged set through a local mirror, the plugin's publication to
# Maven Central, the draft published and read back, the public smoke (with --plugin <version> the release of sbt-teq
# integrations/sbt/plugin-version.txt names, which must be <version>, the intent stated; without it the release
# publishes no plugin, and the plugin the compiler selects must be on Central already). --publish refuses before it
# builds what the publication would refuse: a revision other than the head, files that disagree on the version, a
# version anything of which is served, a record of it that put something out (bench/ship-publish.sh --resume takes
# that up), what the GitHub release lacks (bench/github-release.sh <version> check), with --plugin what the plugin's publish refuses
# (integrations/sbt/publish.sh --preflight: a version Central serves, the Portal's token, the signing key), without
# it a selected plugin Central does not serve; and after the build, a classifier held back: the release carries all
# five, which the README links. The GitHub release is bench/ship-publish.sh's to make, by bench/github-release.sh's
# verbs in the publication's order (the draft, after the Central step its publication, or its abandonment), so that
# the ship runs no `step github` of its own, which the order would put before the smoke. It prints each step's
# wall time and exit, the digests, the versions, the suite's skips and the readback; each step's log is in out/ship/,
# kept when a step fails, and nothing of a failed step is staged or published.
#
# --step runs the steps it names alone, in the ship's order whatever the order named: cross-arm, cross-use, pgo-gen,
# pgo-train, pgo-use, cross-intel, cross-linux-arm, linux-arm-build, linux-arm-suite, cross-windows, and stage (the
# trainings' corpora compared, the five staged by one invocation, those bench/ship-qualified.txt does not publish
# held back, the set checked), each
# from the products of the steps before it, which <dir> carries between runs on one machine or several (the jobs of
# .github/workflows/release.yml, docs/DEVELOPING.md, "Releases"). Before the steps every product <dir> holds of a step
# not named is copied into the tree at its place; after them each product of the steps that ran goes into <dir> at
# the same place: the profiles with their trainings' records, the binaries with their manifests and .version, the
# native suite's outputs (which the Linux aarch64 suite compares with), and stage's binaries/ and held/ under <dir>.
# A step whose products before it are missing fails as the script it runs fails without them. The Linux aarch64
# binary is cross-linux-arm's, built with its suite under qemu-user here (the ship without --step), or
# linux-arm-build's, its suite linux-arm-suite's on an arm64 Linux machine, whose record the stage gives the
# manifest (bench/ship-manifest.sh's manifest_attest); a run names one route. An arm64 Linux machine runs
# cross-arm (the trainer natively) and linux-arm-suite alone, with --step. Held back or not,
# every staged binary is in <dir>, since the qualification of the five on their platforms reads them; with no
# classifier qualified the run says so and does not fail, nothing being published by it. --publish takes no --step.
# TEQ_SHIP_CACHE=<dir> names a directory of the pinned downloads (zig's tarball, qemu-user's and the sysroot's
# .debs, under their file names), copied into the tree's target/cross/ before the steps, where the scripts take a
# file only with its pinned digest.
#
# Needs: Linux on x86-64, whose zig links the five, rustup with the llvm-tools component (the five std targets
# it adds itself), curl and dpkg (zig, qemu-user and the aarch64 sysroot are fetched by their pinned digests), wine, python3, node and the
# coursier cache of the budget's jars (COURSIER_CACHE), and for --publish sbt, what the route needs (above) and a
# bench/ship-qualified.txt that names this toolchain; each step checks the tools of its own (an arm64 machine's
# cross-arm no qemu-user, wine or node; linux-arm-suite docker and curl, and no Rust). Each step is bounded by the sum of its commands' own bounds (a
# fat-LTO build 1200 s, a training run 120 s, the suite 300 s), so no bound kills a build its own command allows; the
# run, about an hour on a 16-thread Xeon and up to five more for Central's publication and read-back, is a detached
# job (`setsid nohup bench/ship.sh > out/ship.log 2>&1 < /dev/null &`), never a foreground shell's.
set -u
cd "$(dirname "$0")/.." || exit 1
publish= plugin_release= rev= only= carry=
usage() { echo "usage: $0 [--publish [--plugin <version>] | --step <step>[,<step>...] --carry <dir>] [<revision>]"; exit 2; }
while [ $# -gt 0 ]; do
  case $1 in
    --publish) publish=1 ;;
    --plugin) plugin_release=${2:-}; [ -n "$plugin_release" ] || usage; shift ;;
    --step) [ $# -ge 2 ] || usage; only=$2; shift ;;
    --carry) [ $# -ge 2 ] || usage; carry=$2; shift ;;
    -*) usage ;;
    *) rev=$1 ;;
  esac
  shift
done
[ -z "$plugin_release" ] || [ -n "$publish" ] || { echo "ship: --plugin goes with --publish"; exit 2; }
# The steps in the ship's order, and what each leaves for the steps after it: paths in the tree, but stage's,
# which are the checkout's.
ship_steps="cross-arm cross-use pgo-gen pgo-train pgo-use cross-intel cross-linux-arm linux-arm-build linux-arm-suite cross-windows stage"
ship_products() {
  case $1 in
    cross-arm) echo target/cross/arm.profdata target/cross/arm.training ;;
    cross-use) echo out/cross/teq-guided-arm out/cross/teq-guided-arm.manifest out/cross/teq-guided-arm.version ;;
    pgo-gen) echo target/pgo/ship/gen/ship/teq target/pgo/ship/linker.stamp target/pgo/ship/gen.metadata ;;
    pgo-train) echo target/pgo/ship/teq.profdata target/pgo/ship/training.txt target/pgo/ship/corpus.txt target/pgo/ship/jars.txt ;;
    pgo-use) echo target/ship/teq target/ship/teq.manifest target/ship/teq.version target/pgo/ship/suite.log out/tests out/tests-jars.tgz ;;
    cross-intel) echo out/cross/teq-guided-intel out/cross/teq-guided-intel.manifest out/cross/teq-guided-intel.version ;;
    cross-linux-arm) echo out/cross/teq-guided-linux-arm out/cross/teq-guided-linux-arm.manifest out/cross/teq-guided-linux-arm.version target/cross/guided-linux-arm.suite ;;
    linux-arm-build) echo out/cross/teq-guided-linux-arm out/cross/teq-guided-linux-arm.manifest out/cross/teq-guided-linux-arm.version ;;
    linux-arm-suite) echo out/cross/teq-guided-linux-arm.suite target/cross/guided-linux-arm.suite ;;
    cross-windows) echo out/cross/teq-windows-x86_64.exe out/cross/teq-windows-x86_64.exe.manifest out/cross/teq-windows-x86_64.exe.version target/cross/windows.smoke ;;
    stage) echo integrations/sbt/binary/binaries out/ship/held out/ship/profiles ;;
  esac
}
if [ -n "$only" ]; then
  [ -z "$publish" ] || { echo "ship: --publish takes no --step: a release is published by its own steps (docs/DEVELOPING.md, \"Releases\")"; exit 2; }
  [ -n "$carry" ] || usage
  for s in ${only//,/ }; do
    [[ " $ship_steps " == *" $s "* ]] || { echo "ship: no step $s; the steps: $ship_steps"; exit 2; }
  done
  [[ ",$only," != *,cross-linux-arm,* ]] || [[ ",$only," != *,linux-arm-build,* && ",$only," != *,linux-arm-suite,* ]] ||
    { echo "ship: the Linux aarch64 binary is cross-linux-arm's (its suite under qemu-user) or linux-arm-build's and linux-arm-suite's, not both"; exit 2; }
  mkdir -p "$carry" && carry=$(cd "$carry" && pwd) || exit 1
elif [ -n "$carry" ]; then
  usage
fi
# wanted <step>: whether the run runs the step: without --step every one but the native route's linux-arm-build and
# linux-arm-suite.
wanted() {
  if [ -z "$only" ]; then [ "$1" != linux-arm-build ] && [ "$1" != linux-arm-suite ]; else [[ ",$only," == *",$1,"* ]]; fi
}
# only_suite: whether the run is linux-arm-suite's alone, on a machine with no Rust toolchain.
only_suite() { [ "$only" = linux-arm-suite ]; }
. bench/ship-manifest.sh
. bench/ship-release.sh
fail() { echo "ship: $*"; exit 1; }
# The canonical route: no flags, profile, qemu or revision of the caller's reach the builds.
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS TEQ_CROSS_PROFILE TEQ_CROSS_NAME TEQ_CROSS_QEMU TEQ_CROSS_METADATA TEQ_CROSS_KEEP_DEAD TEQ_STAGE_PLAIN TEQ_REVISION TEQ TEQ_TEST_OUT TEQ_TEST_TIMEOUT TEQ_PGO_RUN_BOUND
# The jars of the corpus and the suites, where coursier keeps them on Linux unless told otherwise
# (tests/support/jars.sh defaults to the Mac's place).
export COURSIER_CACHE=${COURSIER_CACHE:-$HOME/.cache/coursier/v1}
root=$PWD
tree=$root/target/ship-tree
logs=$root/out/ship
binaries=$root/integrations/sbt/binary/binaries
run=ship-$(date -u +%Y%m%dT%H%M%SZ)-$$

# The machine: Linux x86-64 makes the ship; an arm64 Linux machine (uname's word, the kernel's own) the steps of
# the native route alone, the aarch64 trainer's training and the Linux aarch64 suite.
case "$(uname -s) $(uname -m)" in
  "Linux x86_64")
    ! wanted linux-arm-suite || fail "linux-arm-suite runs on an arm64 Linux machine; cross-linux-arm runs the suite here under qemu-user" ;;
  "Linux aarch64")
    [ -n "$only" ] || fail "the ship is made on Linux x86-64; an arm64 Linux machine runs --step cross-arm and --step linux-arm-suite alone"
    for s in ${only//,/ }; do
      case $s in cross-arm | linux-arm-suite) ;; *) fail "the step $s is made on Linux x86-64; an arm64 Linux machine runs cross-arm and linux-arm-suite alone" ;; esac
    done ;;
  *) fail "the ship is made on Linux x86-64 (cross-arm and linux-arm-suite on an arm64 Linux machine too), not $(uname -s) $(uname -m)" ;;
esac
[ -z "$(git status --porcelain)" ] || fail "the checkout has changes; commit them or ship from a clean one"
commit=$(git rev-parse --verify --quiet "${rev:-HEAD}^{commit}") || fail "no commit ${rev:-HEAD}"
head=$(git rev-parse HEAD)
git diff --quiet "$commit" "$head" -- $manifest_sources || fail "${rev:-HEAD} (${commit:0:12}) has other compiler sources than the head (${head:0:12}), which check.sh would refuse"
# The tools of the steps the run runs.
tools="git sha256sum"
only_suite || tools="$tools rustup cargo curl python3"
! wanted pgo-use && ! wanted cross-linux-arm || tools="$tools node"
! wanted cross-linux-arm && { ! wanted cross-arm || [ "$(uname -m)" != x86_64 ]; } || tools="$tools dpkg"
! wanted cross-windows || tools="$tools wine"
! wanted linux-arm-suite || tools="$tools curl docker"
for tool in $tools; do
  command -v $tool > /dev/null || fail "no $tool on the PATH"
done
if ! only_suite; then
  host=$(rustc -vV | sed -n 's/^host: //p')
  [ -x "$(rustc --print sysroot)/lib/rustlib/$host/bin/llvm-profdata" ] || fail "no llvm-profdata; rustup component add llvm-tools"
fi
if [ -n "$publish" ]; then
  command -v sbt > /dev/null || fail "no sbt on the PATH, which publishes"
  # This machine's toolchain as a manifest of a native build records it, against the qualified one.
  mkdir -p "$root/target" && { echo "target $host"; echo "host $host"; manifest_tuple; } > "$root/target/ship-toolchain.txt" &&
    manifest_base_qualified "$root/target/ship-toolchain.txt" bench/ship-qualified.txt || fail "this machine's toolchain is not the qualified one"
  # A release is one commit, the head's: the head's plugin goes with the binaries.
  [ "$commit" = "$head" ] || fail "${rev:-HEAD} (${commit:0:12}) is not the head (${head:0:12}): a release is built from the head's commit"
  version=$(release_version .) || fail "the checkout names no one release"
  release_unserved "$version" || fail "$version cannot be published"
  [ ! -f "out/ship/$version/record.json" ] || case $(python3 -B bench/ship-record.py "out/ship/$version/record.json" get github.state) in
    none | deleted) ;;
    *) fail "out/ship/$version/record.json records a publication of $version under way: bench/ship-publish.sh --resume $version takes it up" ;;
  esac
  [ -x "$release_github_script" ] || fail "no $release_github_script, which publishes the GitHub release"
  "$release_github_script" "$version" check || fail "the GitHub release v$version cannot be published (above)"
  plugin=$(release_plugin_version .) || fail "the checkout names no plugin version"
  if [ -n "$plugin_release" ]; then
    # The plugin this release publishes: the one the compiler selects, as the caller stated.
    [ "$plugin_release" = "$plugin" ] || fail "--plugin $plugin_release, and integrations/sbt/plugin-version.txt names $plugin: bench/release.sh --plugin <version> moves it"
    integrations/sbt/publish.sh --preflight || fail "sbt-teq $plugin cannot be published to Central (above)"
  else
    # A compiler release alone: the plugin it selects is served already, and serves compilers after 0.1.6.
    release_plugin_legacy "$plugin" && fail "the compiler $version selects sbt-teq $plugin, a release up to 0.1.6, which serves no later compiler: bench/release.sh --plugin <version>, then --plugin <version> here"
    plugin_served "$plugin" || fail "the compiler $version selects sbt-teq $plugin, which Central does not serve: --plugin $plugin publishes it with this release"
  fi
  echo "ship: the release $version of ${head:0:12}, which no repository serves yet, selecting sbt-teq $plugin$([ -n "$plugin_release" ] && echo ", published with it")"
fi

mkdir -p "$logs" && rm -f "$logs"/*.log || exit 1
if [ -e "$tree" ]; then
  git worktree remove --force "$tree" 2> /dev/null || rm -rf "$tree"
  git worktree prune
fi
git worktree add --quiet --detach "$tree" "$commit" || fail "no worktree of ${commit:0:12} at $tree"
echo "ship: ${commit:0:12} ($(git log -1 --format=%s "$commit")) in $tree, run $run"
if [ -n "${TEQ_SHIP_CACHE:-}" ]; then
  # The pinned downloads where the scripts look for them, each taken only with its digest.
  mkdir -p "$tree/target/cross" && find "$TEQ_SHIP_CACHE" -maxdepth 1 -type f -exec cp {} "$tree/target/cross/" \; ||
    fail "the downloads of $TEQ_SHIP_CACHE could not be copied into the tree"
  echo "ship: the pinned downloads of $TEQ_SHIP_CACHE in the tree: $(find "$TEQ_SHIP_CACHE" -maxdepth 1 -type f -printf '%f ')"
fi
# carry <from> <to> <step>: the step's products copied from one root to the other where there are some, a
# directory replaced whole.
carry() {
  local p base
  for p in $(ship_products "$3"); do
    [ -e "$1/$p" ] || continue
    rm -rf "${2:?}/$p" && mkdir -p "$(dirname "$2/$p")" && cp -a "$1/$p" "$2/$p" || return 1
  done
}
if [ -n "$only" ]; then
  for s in $ship_steps; do
    wanted "$s" && continue
    if [ "$s" = stage ]; then base=$root; else base=$tree; fi
    carry "$carry" "$base" "$s" || fail "the products of $s in $carry could not be copied in"
  done
  echo "ship: the steps $only, from the products in $carry"
fi

# The corpus, generated in the tree as the trainings generate it: every budget program, its jars at hand; for the
# steps that train or compare the trainings. A training is bounded by its runs' bounds (bench/pgo.sh's train: 60 s
# each natively, 120 s under qemu-user, the aarch64 trainer's on x86-64), the merge and the corpus's digests (600).
train_bound=0 arm_train_bound=0
if wanted cross-arm || wanted pgo-train || wanted stage; then
  programs=$(cd "$tree" && work=out/budget && . bench/programs.sh > /dev/null && [ -z "$left_out" ] && echo $programs | wc -w) ||
    fail "the training corpus is not whole (bench/programs.sh leaves programs out for missing jars; COURSIER_CACHE=${COURSIER_CACHE:-unset})"
  echo "ship: the training corpus: $programs programs, bench/app's corpus and its jars"
  train_bound=$(( 600 + (programs + 8) * 60 ))
  arm_train_bound=$train_bound
  [ "$(uname -m)" != x86_64 ] || arm_train_bound=$(( 600 + (programs + 8) * 120 ))
fi
# The jars the suites build (tests/support/jars.sh's jars_warm), built in the tree now under a bound of their own,
# not at their first use inside a suite's, which a fresh machine's scala-cli would pass.
if wanted pgo-use || wanted cross-linux-arm; then
  (cd "$tree" && timeout 1800 bash -c '. tests/support/jars.sh && jars_warm') || fail "the jars the suites build could not be built in the tree (above)"
  echo "ship: the jars the suites build are built in the tree"
fi

steps=()
staging=
# step <name> <bound in s> <command...>: in the tree, its output into out/ship/<name>.log and here.
step() {
  local name=$1 bound=$2 start status
  shift 2
  start=$(date +%s)
  echo "ship: $name: $* (bound ${bound} s)"
  (cd "$tree" && timeout "$bound" "$@") < /dev/null 2>&1 | tee "$logs/$name.log" | sed -u 's/^/  /'
  status=${PIPESTATUS[0]}
  local took=$(( $(date +%s) - start ))
  steps+=("$(printf '%-13s exit %-3s %3d min %02d s' "$name" $status $((took / 60)) $((took % 60)))")
  echo "ship: $name: exit $status in $((took / 60)) min $((took % 60)) s"
  [ $status -eq 0 ] && return 0
  [ $status -ne 124 ] || echo "ship: $name ran past its bound"
  # The set is staged whole or not at all.
  [ -z "$staging" ] || rm -rf "$binaries"
  summary
  fail "$name failed; its log is $logs/$name.log, nothing of it is staged"
}
summary() {
  echo "ship: steps"
  printf '  %s\n' "${steps[@]}"
}

# arm: the std target (600 s), qemu-user fetched (2 x 300 + 120; none on an arm64 machine), the metadata probe
# (300), the instrumented build (1200), the trainer's --version natively (60), the training.
wanted cross-arm && step cross-arm $(( 600 + 720 + 300 + 1200 + 60 + arm_train_bound )) bench/cross-ship.sh arm
# use: the std target (600 s), zig fetched (300 + 120), the profile translated (300), the build (1200), the
# metadata probe (300) for a training that records none.
wanted cross-use && step cross-use $(( 600 + 420 + 300 + 1200 + 300 )) bench/cross-ship.sh use
# gen: zig fetched (300 + 120), the metadata probe (300), the instrumented build (1200), the copy (60).
wanted pgo-gen && step pgo-gen $(( 420 + 300 + 1200 + 60 )) bench/pgo.sh ship gen
wanted pgo-train && step pgo-train "$train_bound" bench/pgo.sh ship train
# use: zig (420), the guided build (1200), the suite (300), the copy and the floor (120); then the jars its suite
# built packed for the Linux aarch64 suite on another machine (600).
wanted pgo-use && step pgo-use $(( 420 + 1200 + 300 + 120 + 600 )) bash -c 'bench/pgo.sh ship use && timeout 600 bench/actions/jars.sh pack out/tests-jars.tgz'
# intel: the std target (600 s), zig (420), the profile translated (300), the build (1200), the metadata probe (300).
wanted cross-intel && step cross-intel $(( 600 + 420 + 300 + 1200 + 300 )) bench/cross-ship.sh intel
# linux-arm: the std target (600 s), zig (420), the sysroot and qemu-user fetched (420 + 720), the profile
# translated (300), the build (1200), the metadata probe (300), the suite under qemu-user (1800).
wanted cross-linux-arm && step cross-linux-arm $(( 600 + 420 + 420 + 720 + 300 + 1200 + 300 + 1800 )) bench/cross-ship.sh linux-arm
# linux-arm-build: the same but the suite, its sysroot and qemu-user.
wanted linux-arm-build && step linux-arm-build $(( 600 + 420 + 300 + 1200 + 300 )) bench/cross-ship.sh linux-arm-build
# linux-arm-suite, on an arm64 machine: the floor's package and node fetched (2 x 300) and unpacked (120), the
# built jars restored (300), the floor's image pulled (900), the suite in its container (2100), its removal (60).
wanted linux-arm-suite && step linux-arm-suite $(( 600 + 120 + 300 + 900 + 2100 + 60 )) bench/cross-ship.sh linux-arm-suite
# windows: the std target (600 s), zig (420), the build (1200), the smoke under wine (1200).
wanted cross-windows && step cross-windows $(( 600 + 420 + 1200 + 1200 )) bench/cross-ship.sh windows

# carry_out <step>...: the products of the steps that ran into the carried directory.
carry_out() {
  local s
  [ -n "$only" ] || return 0
  for s in "$@"; do
    wanted "$s" || continue
    if [ "$s" = stage ]; then carry "$root" "$carry" "$s"; else carry "$tree" "$carry" "$s"; fi || fail "the products of $s could not be copied into $carry"
  done
}
if ! wanted stage; then
  carry_out $ship_steps
  summary
  echo "ship: the steps $only of ${commit:0:12} passed; their products are in $carry"
  exit 0
fi

mac=$tree/out/cross/teq-guided-arm
intel=$tree/out/cross/teq-guided-intel
linux=$tree/target/ship/teq
linux_arm=$tree/out/cross/teq-guided-linux-arm
windows=$tree/out/cross/teq-windows-x86_64.exe
# The two trainings read the same corpus: the programs, the files and the jars, by their digests.
for key in programs corpus jars; do
  a=$(manifest_get "$tree/target/cross/arm.training" "training $key")
  b=$(manifest_get "$tree/target/pgo/ship/training.txt" "training $key")
  [ -n "$a" ] && [ "$a" = "$b" ] || { summary; fail "the trainings read different corpora: $key '$a' for the aarch64 trainer, '$b' for the x86-64 one"; }
done
# A training of this ship's own read every program of the tree; profiles taken from an earlier release's ship
# (bench/ship-profiles.sh) read that one's corpus, whatever the tree holds since (bench/ship-manifest.sh's
# manifest_own_training).
if manifest_own_training "$tree/target/pgo/ship/training.txt" "$commit"; then
  [ "$(manifest_get "$tree/target/pgo/ship/training.txt" "training programs")" = "$programs" ] || { summary; fail "the training ran $(manifest_get "$tree/target/pgo/ship/training.txt" "training programs") programs, not $programs"; }
else
  echo "ship: the profiles are $(manifest_get "$tree/target/pgo/ship/training.txt" "training release" | awk '{print $1}')'s ship's, taken from $(manifest_get "$tree/target/pgo/ship/training.txt" "training source" | awk '{print $1}'), on its corpus of $(manifest_get "$tree/target/pgo/ship/training.txt" "training programs") programs ($programs in this tree)"
fi

# The Linux aarch64 binary of the native route: its suite's record, run on an arm64 machine against these bytes,
# given to its manifest; one without (linux-arm-build's alone) the stage refuses below.
if [ -f "$tree/out/cross/teq-guided-linux-arm.suite" ]; then
  manifest_attest "$linux_arm" "$tree/out/cross/teq-guided-linux-arm.suite" || { summary; fail "the Linux aarch64 suite's record is not of the binary staged (above)"; }
  echo "ship: linux-aarch_64: $(sed -n 's/^suite [0-9a-f]* //p' "$tree/out/cross/teq-guided-linux-arm.suite")"
fi
grep -q '^suite ' "$linux_arm.manifest" 2> /dev/null ||
  { summary; fail "the Linux aarch64 binary has no suite: linux-arm-build's waits for linux-arm-suite's record, from an arm64 machine"; }
rm -rf "$binaries" "$root/out/ship/held" && mkdir -p "$binaries" || exit 1
export TEQ_SHIP_RUN=$run
staging=1
step stage-mac 120 "$root/integrations/sbt/binary/stage.sh" "$mac" osx-aarch_64
step stage-intel 120 "$root/integrations/sbt/binary/stage.sh" "$intel" osx-x86_64
step stage-linux 120 "$root/integrations/sbt/binary/stage.sh" "$linux" linux-x86_64
step stage-linux-arm 120 "$root/integrations/sbt/binary/stage.sh" "$linux_arm" linux-aarch_64
step stage-windows 120 "$root/integrations/sbt/binary/stage.sh" "$windows" windows-x86_64
staging=
# The profiles that guided the five, the release's asset beside them (bench/ship-profiles.sh), which a later
# release takes up.
rm -rf "$root/out/ship/profiles" || exit 1
step profiles 120 "$root/bench/ship-profiles.sh" write "$tree" "$root/out/ship/profiles"
# A classifier bench/ship-qualified.txt does not publish yet (a tool of its route the file does not name, or no
# `route` line, the maintainer's run on the platform not done: the Windows one until its wine line and run, the macOS
# x86-64 and Linux aarch64 ones until their runs) is staged and held back: its directory leaves binaries/, which
# the release takes whole, for out/ship/held/<classifier>/, where the publish that follows the qualification
# finds it.
publish_classifiers= held_classifiers= held=$root/out/ship/held
for c in $release_classifiers; do
  if reason=$(manifest_qualified "$binaries/$c/teq.manifest" "$root/bench/ship-qualified.txt" "$c" 2>&1); then
    publish_classifiers="$publish_classifiers $c"
  else
    mkdir -p "$held" && rm -rf "$held/$c" && mv "$binaries/$c" "$held/$c" || exit 1
    held_classifiers="$held_classifiers $c"
    echo "ship: $c held back from the publish: $reason; staged under $held/$c"
  fi
done
# Run alone, the stage hands the five to their platforms' qualification whatever the file says: a run that
# qualifies a new build environment comes before the file names it (docs/DEVELOPING.md, "Releases").
[ -n "$publish_classifiers" ] || [ -n "$only" ] || { summary; fail "no classifier of the set is qualified"; }
# The release carries the five binaries the README links, or none: one held back stops the publication.
[ -z "$publish" ] || [ -z "$held_classifiers" ] ||
  { summary; fail "a release carries all five binaries, and$held_classifiers are held back (above); nothing is published"; }
if [ -n "$publish" ]; then
  # The set checked as the release's draft checks it, then the publication: the preflights (1000 s), the draft
  # (bench/ship-release.sh's release_draft_bound: an upload of each of the release's files), the plugin's staging
  # (1300), the smoke against the staged set (3600), the plugin's publication (20000), the draft published and read
  # back (release_publish_bound: each file read back), the public smoke (3600).
  step publish-check 600 "$root/integrations/sbt/binary/check.sh" $publish_classifiers
  step publish $(( 1000 + $(release_draft_bound "$version") + 1300 + 3600 + 20000 + $(release_publish_bound "$version") + 3600 )) "$root/bench/ship-publish.sh" "$version" ${plugin_release:+--plugin "$plugin_release"}
else
  # Without --publish the set is checked as the publish would check it; a refusal fails the run
  # after the summary, the binaries staged, since nothing publishes them as they stand.
  echo "ship: the set as the release's draft checks it (not published):"
  if [ -n "$publish_classifiers" ]; then
    timeout 900 "$root/integrations/sbt/binary/check.sh" $publish_classifiers 2>&1 | sed 's/^/  /'
    checked=${PIPESTATUS[0]}
  else
    echo "  no classifier qualified, none to check"
    checked=0
  fi
  steps+=("$(printf '%-13s exit %-3s' publish-check "$checked")")
fi

summary
echo "ship: commit $commit"
for c in $release_classifiers; do
  d=$binaries/$c; [ -d "$d" ] || d=$held/$c
  m=$d/teq.manifest
  echo "ship: $c: $(manifest_get "$m" version), sha256 $(manifest_get "$m" binary), sha1 $(sha1sum < "$d/teq" | cut -c1-40)$([[ " $held_classifiers " == *" $c "* ]] && echo ", held back")"
done
echo "ship: training (both): $(manifest_get "$tree/target/pgo/ship/training.txt" "training programs") programs, corpus $(manifest_get "$tree/target/pgo/ship/training.txt" "training corpus"), jars $(manifest_get "$tree/target/pgo/ship/training.txt" "training jars")"
echo "ship: runs merged: $(manifest_get "$tree/target/cross/arm.training" "training runs") by the aarch64 trainer ($(manifest_get "$tree/target/cross/arm.training" "training trainer")$(gives=$(manifest_get "$tree/target/cross/arm.training" "training give-ways"); [ -z "$gives" ] || echo ", $gives given way and run again serially")), $(manifest_get "$tree/target/pgo/ship/training.txt" "training runs") by the x86-64 one"
manifest_of() { local m=$binaries/$1/teq.manifest; [ -f "$m" ] || m=$held/$1/teq.manifest; echo "$m"; }
echo "ship: linux-x86_64 $(sed -n 's/^suite [0-9a-f]* //p' "$(manifest_of linux-x86_64)"); glibc $(manifest_get "$(manifest_of linux-x86_64)" glibc)"
grep -h '^skip ' "$tree/target/pgo/ship/suite.log" | sed 's/^/ship: suite skip: /'
echo "ship: linux-aarch_64 $(sed -n 's/^suite [0-9a-f]* //p' "$(manifest_of linux-aarch_64)"); glibc $(manifest_get "$(manifest_of linux-aarch_64)" glibc)"
echo "ship: windows-x86_64 $(sed -n 's/^smoke [0-9a-f]* //p' "$(manifest_of windows-x86_64)")"
echo "ship: macOS: no suite runs here; the arm64 binary's outputs' equality with the native build is the comparison's on the reference machine, a Mac's smoke after the publish; the x86-64 binary's is the maintainer's run under Rosetta (docs/SPEED.md, \"The ship from Linux\")"
echo "ship: Linux aarch64: its suite $(grep -q '^suite .* natively in ' "$(manifest_of linux-aarch_64)" && echo "natively in the floor's container" || echo "under qemu-user") over the floor's glibc; its qualification runs it on an arm64 Linux"
echo "ship: Windows: plain (no profile), its smoke under wine; the run on Windows is the maintainer's (docs/TARGETS.md, \"Windows\")"
# The binaries of out/cross/, which a step run alone wrote in another tree than its neighbours: their digests again.
[ -z "$only" ] || (cd "$tree/out/cross" && find . -maxdepth 1 -name 'teq-*' \( ! -name '*.*' -o -name '*.exe' \) -printf '%f\n' | sort | xargs sha256sum > SHA256SUMS)
sed 's/^/ship: SHA256SUMS: /' "$tree/out/cross/SHA256SUMS"
[ "${checked:-0}" -eq 0 ] || fail "the staged set would not be published as it stands (check.sh above)"
carry_out stage
