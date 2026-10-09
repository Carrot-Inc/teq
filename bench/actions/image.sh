#!/bin/bash
# bench/actions/image.sh build | login | record | tag | recipe: the ship's images (bench/actions/ship.Dockerfile;
# docs/DEVELOPING.md, "Releases"), one per architecture, linux/amd64 (every build) and linux/arm64 (the aarch64
# trainer's training), from one recipe. build, in the ship-image workflow (.github/workflows/ship-image.yml) or on
# a machine with docker and buildx (the arm64 image under emulation where the machine is x86-64): both built from
# the checkout's root (the Dockerfile's own ignore file, ship.Dockerfile.dockerignore, keeps the context to the
# recipe's files: the Dockerfile, bench/actions/toolchain.sh, bench/zig.sh and bench/cross-ship.sh), tagged with
# the recipe's digest and pushed to the repository's packages ($image_repo) as one index, logged in by GH_TOKEN
# (never an argument, never printed); then record. record: the lines for bench/actions/ship-image.txt, from the
# index the tag names in the registry, one per architecture, each image by its own digest with the recipe's
# digest and its platform (`<reference by digest> <recipe digest> <platform>`), also into the run's summary; the
# workflow's other builder (Depot's, which pushes the same index under the same tag, docker logged in by login
# before it, the session kept for it) is recorded by it too. tag: the tag of the checkout's recipe. recipe: the
# recipe's digest of this checkout, the one toolchain.sh writes into the images.
set -uo pipefail
cd "$(dirname "$0")/../.." || exit 1
image_repo=ghcr.io/carrot-inc/teq-ship
platforms="linux/amd64 linux/arm64"
fail() { echo "image: $*" >&2; exit 1; }
recipe=$(cat bench/actions/ship.Dockerfile bench/actions/toolchain.sh | sha256sum | cut -c1-64)
tag=$image_repo:${recipe:0:16}

login() {
  [ -n "${GH_TOKEN:-}" ] || fail "no GH_TOKEN, which writes the repository's packages"
  printf '%s' "$GH_TOKEN" | timeout 120 docker login ghcr.io -u "${GITHUB_ACTOR:-x-access-token}" --password-stdin > /dev/null || fail "ghcr.io refuses the token"
}

# record: each platform's image of the index under $tag, by its digest.
record() {
  local index lines
  index=$(timeout 300 docker buildx imagetools inspect --raw "$tag") || fail "no index $tag in the registry"
  lines=$(python3 -c '
import json, sys
index, repo, recipe, platforms = json.loads(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4].split()
found = {}
for m in index.get("manifests", []):
    p = m.get("platform", {})
    name = "%s/%s" % (p.get("os"), p.get("architecture"))
    if name in platforms:
        if name in found:
            sys.exit("two images of %s in the index" % name)
        found[name] = m["digest"]
missing = [p for p in platforms if p not in found]
if missing:
    sys.exit("the index has no image of %s" % " ".join(missing))
for p in platforms:
    print("%s@%s %s %s" % (repo, found[p], recipe, p))
' "$index" "$image_repo" "$recipe" "$platforms") || fail "the index $tag is not one image per platform ($platforms)"
  grep -qvE "^$image_repo@sha256:[0-9a-f]{64} $recipe linux/(amd64|arm64)$" <<< "$lines" && fail "an image of $tag has no digest: $lines"
  echo "image: the images of $tag; the lines for bench/actions/ship-image.txt:"
  echo "$lines"
  [ -z "${GITHUB_STEP_SUMMARY:-}" ] || { printf 'The lines for `bench/actions/ship-image.txt`:\n\n'; sed 's/^/    /' <<< "$lines"; } >> "$GITHUB_STEP_SUMMARY"
}

case ${1:-} in
  recipe) echo "$recipe"; exit 0 ;;
  tag) echo "$tag"; exit 0 ;;
  login) command -v docker > /dev/null || fail "no docker"; login; echo "image: logged in to ghcr.io for $tag"; exit 0 ;;
  build | record) ;;
  *) echo "usage: $0 build | login | record | tag | recipe" >&2; exit 2 ;;
esac
command -v docker > /dev/null || fail "no docker"
timeout 60 docker buildx version > /dev/null 2>&1 || fail "no docker buildx"
trap 'docker logout ghcr.io > /dev/null 2>&1' EXIT
login
if [ "$1" = build ]; then
  # Without attestations, the index holds the two images alone.
  timeout 10800 docker buildx build --pull --platform "${platforms// /,}" --provenance=false --sbom=false \
    -f bench/actions/ship.Dockerfile -t "$tag" --push . || fail "the images could not be built and pushed"
  echo "image: pushed $tag"
fi
record
