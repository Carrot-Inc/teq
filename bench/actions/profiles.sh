#!/bin/bash
# bench/actions/profiles.sh <carry dir> <out tar>: the release workflow's profiles job (.github/workflows/release.yml;
# docs/DEVELOPING.md, "Releases"), from the root of a checkout of the release's commit in the ship's image: whether
# the release trains its profiles or takes up an earlier release's, and which. TEQ_TRAIN=true trains (the
# dispatch's `train`). Else the asset teq-<version>-profiles.tar of the GitHub release TEQ_PROFILES (a tag: the
# dispatch's `profiles`, or the admission's default, the release before this one's) is taken into <carry dir>
# (bench/ship-profiles.sh take, each record naming the tag and the asset's digest) and tried; the release trains
# when that release or its asset is not there, when the asset is not whole, when its profiles were trained by
# another compiler than this image's (teq's crate, named by the compiler's version, would match none of their
# records), or when the x86-64 profile has gone stale for this tree (bench/pgo.sh ship stale: a guided build's
# count of its hottest functions changed). Writes the step's outputs (GITHUB_OUTPUT: train, source, reason) and the
# run's summary; when the profiles are taken, <out tar> holds them as the carry of bench/ship.sh's cross-arm and
# pgo-train steps, for the builds. Exits 0 once decided; 1 when nothing could be decided (GitHub unanswered, the
# probe's build failed), the job's failure. Needs gh (GH_TOKEN, the repository's contents read).
set -uo pipefail
cd "$(dirname "$0")/../.." || exit 1
. bench/ship-manifest.sh || exit 1
[ $# -eq 2 ] || { echo "usage: $0 <carry dir> <out tar>" >&2; exit 2; }
carry=$1 out=$2
repo=Carrot-Inc/teq
outputs=${GITHUB_OUTPUT:-/dev/stdout}
summary=${GITHUB_STEP_SUMMARY:-/dev/null}
tag=${TEQ_PROFILES:-}
fail() { echo "profiles: $*" >&2; printf 'The profiles could not be decided: %s\n' "$*" >> "$summary"; exit 1; }
# decide <train> <source> <reason>: the outputs and the summary.
decide() {
  printf 'train=%s\nsource=%s\nreason=%s\n' "$1" "$2" "$3" >> "$outputs"
  echo "profiles: $([ "$1" = true ] && echo "the release trains its own" || echo "the release takes $2's"): $3"
  printf '%s: %s.\n' "$([ "$1" = true ] && echo "The release trains its profiles" || echo "The release takes the profiles of $2")" "$3" >> "$summary"
  exit 0
}
[ "${TEQ_TRAIN:-false}" != true ] || decide true "" "the dispatch asks for a training (train)"
[ -n "$tag" ] || decide true "" "no earlier release is named to take the profiles of"
[[ $tag =~ ^[A-Za-z0-9._-]+$ ]] || fail "'$tag' is not a release's tag (TEQ_PROFILES)"

# The asset, by its name in the release's list: teq-<version>-profiles.tar, one.
work=$(mktemp -d) || exit 1
trap 'rm -rf -- "$work"' EXIT
if ! names=$(timeout 60 gh release view "$tag" --repo "$repo" --json assets --jq '.assets[].name' 2> "$work/err"); then
  grep -qi 'release not found\|HTTP 404' "$work/err" || fail "GitHub did not answer for the release $tag: $(head -3 "$work/err")"
  decide true "" "GitHub has no release $tag, whose profiles the release would take"
fi
asset=$(grep -E '^teq-[0-9]+\.[0-9]+\.[0-9]+-profiles\.tar$' <<< "$names")
[ -n "$asset" ] || decide true "" "the release $tag has no profiles asset (teq-<version>-profiles.tar)"
[ "$(wc -l <<< "$asset")" -eq 1 ] || decide true "" "the release $tag has more than one profiles asset: $(tr '\n' ' ' <<< "$asset")"
timeout 600 gh release download "$tag" --repo "$repo" --pattern "$asset" --dir "$work" > /dev/null 2> "$work/err" && [ -f "$work/$asset" ] ||
  fail "the asset $asset of $tag could not be downloaded: $(head -3 "$work/err")"
sha=$(manifest_sha256 "$work/$asset")
rm -rf -- "$carry" && mkdir -p "$carry" || exit 1
why=$(bench/ship-profiles.sh take "$work/$asset" "$tag" "$carry" 2>&1) || decide true "" "the asset $asset of $tag is not whole: ${why#profiles: }"
echo "$why"

# The compiler that named their functions, this image's.
rustc=$(manifest_tuple | sed -n 's/^tuple rustc //p')
for f in target/pgo/ship/training.txt target/cross/arm.training; do
  got=$(manifest_get "$carry/$f" "training tuple rustc")
  [ "$got" = "$rustc" ] || decide true "" "$asset's profiles were trained by rustc $got, this toolchain is $rustc: no function of teq's would find its record"
done

# The x86-64 profile against this tree, by the guided build's own count; the aarch64 one, of the same ship's
# trainings and corpus, ages with it. The pinned zig of the image's cache where the build looks for it.
[ -z "${TEQ_SHIP_CACHE:-}" ] || { mkdir -p target/cross && find "$TEQ_SHIP_CACHE" -maxdepth 1 -name 'zig-*.tar.xz' -exec cp {} target/cross/ \; ; }
TEQ_PGO_PROFILE=$carry/target/pgo/ship/teq.profdata TEQ_PGO_TRAINING=$carry/target/pgo/ship/training.txt timeout 2100 bench/pgo.sh ship stale
case $? in
  0) ;;
  3) decide true "" "$asset's profiles are stale for this tree: $(sed -n '1s/^stale //p' target/pgo/ship/stale.txt)" ;;
  *) fail "the guided build that tries $asset's x86-64 profile failed (above)" ;;
esac
(cd "$carry" && tar -cf - target) > "$out" || fail "$out not written"
decide false "$tag $sha" "$asset ($(sed -n 's/^training release \([^ ]*\) .*/trained by \1/p' "$carry/target/pgo/ship/training.txt")) serves this tree: $(sed -n '1s/^stale //p' target/pgo/ship/stale.txt)"
