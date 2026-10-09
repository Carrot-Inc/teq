#!/bin/bash
# stage.sh [<binary> [<os>-<arch>]]: copies a ship build into binaries/<classifier>/, whose binaries
# bench/github-release.sh makes the GitHub release's assets once check.sh has checked the set. With a classifier named (bench/ship.sh's route, for a binary this
# machine may not run): the binary's manifest beside it (<binary>.manifest, bench/ship-manifest.sh)
# must hold the binary's digest, and the version comes from it, never from the binary or a sidecar
# alone; the binary's header and the manifest's target must be that classifier's, the commit's
# compiler sources the head's, the build guided by a profile whose training succeeded (or
# TEQ_STAGE_PLAIN=1), a Linux binary's tests/run.sh passed on those bytes; the copy goes with the
# manifest, which records the staging invocation (TEQ_SHIP_RUN, bench/ship.sh's, or one of its own).
# Without a classifier, the local route: the repository's ship build (bench/pgo.sh ship's, or the
# binary given) for this machine's platform, its `--version` naming the head
# or a commit with the head's compiler sources; no manifest goes with it, so neither check.sh nor
# the release takes it.
# The paths a caller names are the caller's.
given=$1
case $given in /* | "") ;; *) given=$PWD/$given ;; esac
cd "$(dirname "$0")" || exit 1
. ../../../bench/ship-manifest.sh
head=$(git rev-parse HEAD)
if [ -n "$2" ]; then
  binary=$given classifier=$2
  version=$(manifest_check "$binary" "$binary.manifest" "$classifier" "$head") || exit 1
  # The sidecar beside a cross build says the same, or something is out of step.
  [ ! -f "$binary.version" ] || [ "$(cat "$binary.version")" = "$version" ] || { echo "$binary.version reads '$(cat "$binary.version")', its manifest '$version'"; exit 1; }
  mkdir -p "binaries/$classifier" && rm -f "binaries/$classifier/teq.manifest" || exit 1
  cp "$binary" "binaries/$classifier/teq.new" && chmod +x "binaries/$classifier/teq.new" && mv "binaries/$classifier/teq.new" "binaries/$classifier/teq" || exit 1
  # This staging's invocation, in the place of any a copy taken from binaries/ carries.
  { grep -v '^run ' "$binary.manifest"; echo "run ${TEQ_SHIP_RUN:-stage-$(date -u +%Y%m%dT%H%M%SZ)-$$}"; } > "binaries/$classifier/teq.manifest.new" &&
    mv "binaries/$classifier/teq.manifest.new" "binaries/$classifier/teq.manifest" || exit 1
  manifest_check "binaries/$classifier/teq" "binaries/$classifier/teq.manifest" "$classifier" "$head" > /dev/null || { rm -rf "binaries/$classifier"; exit 1; }
  echo "binaries/$classifier/teq: $version, sha256 $(manifest_get "binaries/$classifier/teq.manifest" binary)"
  exit 0
fi
binary=${given:-../../../target/ship/teq}
[ -x "$binary" ] || { echo "no ship build at $binary: bench/pgo.sh ship first"; exit 1; }
version=$("$binary" --version) || { echo "$binary does not run"; exit 1; }
built=$(awk '{print $3}' <<< "$version")
top=$(git rev-parse --show-toplevel)
same_sources() { git -C "$top" diff --quiet "$built" "$head" -- $manifest_sources 2>/dev/null; }
[ -n "$built" ] && { [[ "$head" == "$built"* ]] || same_sources; } || { echo "$binary is built from ${built:-an unknown commit} ($version), whose compiler sources are not the head's (${head:0:8}): bench/pgo.sh ship first"; exit 1; }
case "$version" in
  *" pgo") ;;
  *) [ -n "$TEQ_STAGE_PLAIN" ] || { echo "$binary is not built with its profile ($version): bench/pgo.sh ship first"; exit 1; } ;;
esac
classifier=$(manifest_host) || { echo "unsupported platform $(uname -s) $(uname -m)"; exit 1; }
mkdir -p binaries/$classifier && rm -f binaries/$classifier/teq.manifest && cp "$binary" binaries/$classifier/teq && chmod +x binaries/$classifier/teq || exit 1
echo "binaries/$classifier/teq: $(binaries/$classifier/teq --version)"
