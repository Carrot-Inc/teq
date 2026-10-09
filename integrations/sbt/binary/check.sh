#!/bin/bash
# check.sh [<classifier>...]: checks the binaries of the release the checkout's head names (docs/TARGETS.md,
# "Releases"; bench/ship-release.sh), which bench/ship.sh staged under binaries/<classifier>/ (stage.sh), as the
# GitHub release's draft takes them (bench/github-release.sh), before the publication: the set is the classifiers
# named, or every one staged. The version is the one Cargo.toml and Cargo.lock agree on, a release's (the plugin's
# is its own). A release is one commit, the head's: the binaries are built from it, and the release's inputs are as
# it commits them. Refused: inputs the head does not commit, files that disagree on the version, a directory of
# binaries/ outside the set, a classifier of the set without its binary and manifest, a binary its manifest does
# not vouch for (stage.sh's checks, bench/ship-manifest.sh: the digest, the platform, the version the binary
# carries, the training, a Linux binary's suite on those bytes), a binary built from another commit than the head
# or printing another version than the release's, manifests of two staging invocations, a toolchain other than
# bench/ship-qualified.txt's, and a version of which the GitHub release serves anything (a release is never
# published again: bench/release.sh moves to the next). Nothing is published.
set -uo pipefail
cd "$(dirname "$0")" || exit 1
. ../../../bench/ship-manifest.sh
. ../../../bench/ship-release.sh
qualified=../../../bench/ship-qualified.txt
head=$(git rev-parse HEAD)
fail() { echo "check: $*"; exit 1; }
release_committed ../../.. || fail "commit the release's inputs, or check from a checkout of the release's commit"
version=$(release_version ../../..) || fail "the checkout names no one release"

staged=$(find binaries -mindepth 1 -maxdepth 1 2> /dev/null | sed 's|^binaries/||' | LC_ALL=C sort)
if [ $# -gt 0 ]; then set=$(printf '%s\n' "$@" | LC_ALL=C sort -u); else set=$staged; fi
[ -n "$set" ] || fail "nothing staged under binaries/: bench/ship.sh stages the ship"
for c in $staged; do
  grep -qxF "$c" <<< "$set" || fail "binaries/$c is not in the set ($(echo $set)): a stale directory; bench/ship.sh empties binaries/ and stages its own"
done

runs=
for c in $set; do
  [ -d "binaries/$c" ] || fail "nothing staged for $c"
  [ "$(find "binaries/$c" -mindepth 1 -maxdepth 1 -printf '%f\n' | LC_ALL=C sort | tr '\n' ' ')" = "teq teq.manifest " ] ||
    fail "binaries/$c holds $(find "binaries/$c" -mindepth 1 -maxdepth 1 -printf '%f ' ), not its binary and manifest alone"
  v=$(manifest_check "binaries/$c/teq" "binaries/$c/teq.manifest" "$c" "$head") || fail "binaries/$c refused"
  commit=$(manifest_get "binaries/$c/teq.manifest" commit)
  [ "$commit" = "$head" ] || fail "binaries/$c is built from ${commit:0:12}, not the head ${head:0:12}"
  # What the plugin checks of the binary it resolves: `teq --version` names the release.
  [[ $v == "teq $version "* ]] || fail "binaries/$c is '$v', not the release $version's binary"
  run=$(manifest_get "binaries/$c/teq.manifest" run)
  [ -n "$run" ] || fail "binaries/$c/teq.manifest records no staging invocation: stage.sh <binary> <classifier> stages"
  runs="$runs$run"$'\n'
  manifest_qualified "binaries/$c/teq.manifest" $qualified "$c" || fail "binaries/$c is not published by $qualified as it stands"
  echo "check: $c: $v, sha256 $(manifest_get "binaries/$c/teq.manifest" binary), sha1 $(sha1sum < "binaries/$c/teq" | cut -c1-40)"
done
[ "$(sort -u <<< "${runs%$'\n'}" | wc -l)" -eq 1 ] || fail "the binaries were staged by different invocations: $(sort -u <<< "$runs" | tr '\n' ' ')"

release_unserved "$version" || fail "$version cannot be published"
echo "check: the set checks out as the release $version of $head, not published: $(echo $set)"
