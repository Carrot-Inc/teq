#!/bin/bash
# bench/ship-profiles.sh write <ship tree> <dir> | check <asset> | take <asset> <tag> <carry dir>: the profiles that
# guided a release's binaries, published with the release as its asset teq-<version>-profiles.tar
# (bench/github-release.sh), which the next release's builds take up in the place of trainings of their own
# (docs/DEVELOPING.md, "Releases"; the release workflow's profiles job, bench/actions/profiles.sh).
#
# The asset is a tar of the ship tree's paths, the products of bench/ship.sh's pgo-train and cross-arm steps (a
# carry of theirs, bench/ship.sh --step), and a header:
#   profiles.txt                      `profiles <version> <commit>`, the ship whose binaries the profiles guided;
#                                     then for x86_64 and aarch64 `profile <arch> <sha256> <path>`, `training <arch>
#                                     <sha256> <path>` (the record), `metadata <arch> <hex>`, `trained <arch>
#                                     <version> <commit>` (the ship that trained it), `trainer <arch> <trainer>`;
#                                     and `corpus`, `jars` and `programs`, the trainings' one corpus
#   target/pgo/ship/teq.profdata      the x86-64 trainer's profile, with its training.txt, corpus.txt and jars.txt
#   target/cross/arm.profdata         the aarch64 trainer's, with its arm.training
# Each record whole (bench/ship-manifest.sh's `training` lines): status ok and the profile's digest; the metadata
# of teq's crate under which the profile names teq's functions (bench/ship-metadata.sh), which every build it
# guides is given; the ship that trained it; the trainer; the training machine and its toolchain. The tar is
# written sorted, its times and owners fixed, so that the same profiles make the same bytes.
#
#   write <tree> <dir>   the asset of a ship's tree (target/ship-tree, or the checkout a ship's steps ran in) into
#                        <dir>/teq-<version>-profiles.tar, <version> and <commit> the tree's. A record written
#                        before its lines existed (the ships up to 0.1.6) is completed from the tree: the metadata
#                        TEQ_PROFILES_METADATA's, or the tree's target/cross/metadata (bench/cross-ship.sh's probe of
#                        the native build's, which both trainers were given), the ship that trained it the tree's,
#                        the x86-64 trainer native, the machine and its toolchain the Linux x86-64 binary's manifest
#                        (target/ship/teq.manifest, the same machine and run), the qemu-user of an emulated trainer
#                        its trainer line's. Prints the asset's name and digest.
#   check <asset>        whether a file is such an asset: its entries those alone, regular files; the header's
#                        digests the files'; each record whole and of its profile; the two of one corpus.
#   take <asset> <tag> <carry>   the asset checked and its files put into a carry directory at their places, each
#                        record given `training source <tag> <the asset's sha256>`, the GitHub release and the file
#                        it was taken from, which every manifest of a binary it guides keeps (an earlier source
#                        line replaced).
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
. bench/ship-manifest.sh || exit 1
fail() { echo "profiles: $*" >&2; exit 1; }
# The scratch directory of a verb, removed on the way out.
stage=
trap '[ -z "$stage" ] || rm -rf -- "$stage"' EXIT
files="target/cross/arm.profdata target/cross/arm.training target/pgo/ship/corpus.txt target/pgo/ship/jars.txt target/pgo/ship/teq.profdata target/pgo/ship/training.txt"
# The architectures' profiles and records, in the header's order.
x86_profile=target/pgo/ship/teq.profdata x86_training=target/pgo/ship/training.txt
arm_profile=target/cross/arm.profdata arm_training=target/cross/arm.training

# whole <dir>: the profiles and records under <dir> as an asset holds them, or a refusal naming what is not.
whole() {
  local d=$1 arch profile training key
  for arch in x86_64 aarch64; do
    if [ $arch = x86_64 ]; then profile=$x86_profile training=$x86_training; else profile=$arm_profile training=$arm_training; fi
    [ -f "$d/$profile" ] && [ -f "$d/$training" ] || { echo "no $arch profile and record ($profile, $training)"; return 1; }
    [ "$(manifest_get "$d/$training" "training status")" = ok ] || { echo "$training records no successful training"; return 1; }
    [ "$(manifest_get "$d/$training" "training profile")" = "$(manifest_sha256 "$d/$profile")" ] || { echo "$training is not the record of $profile"; return 1; }
    [[ $(manifest_get "$d/$training" "training metadata") =~ ^[0-9a-f]+$ ]] || { echo "$training records no metadata of teq's crate"; return 1; }
    [[ $(manifest_get "$d/$training" "training release") =~ ^[0-9]+\.[0-9]+\.[0-9]+\ [0-9a-f]{40}$ ]] || { echo "$training records no ship that trained it"; return 1; }
    [ -n "$(manifest_get "$d/$training" "training trainer")" ] && [ -n "$(manifest_get "$d/$training" "training host")" ] &&
      [ -n "$(manifest_get "$d/$training" "training tuple rustc")" ] || { echo "$training records no trainer, machine or toolchain"; return 1; }
  done
  for key in programs corpus jars; do
    [ -n "$(manifest_get "$d/$x86_training" "training $key")" ] && [ "$(manifest_get "$d/$x86_training" "training $key")" = "$(manifest_get "$d/$arm_training" "training $key")" ] ||
      { echo "the two trainings read different corpora ($key)"; return 1; }
  done
  [ "$(manifest_get "$d/$x86_training" "training corpus" | awk '{print $1}')" = "$(manifest_sha256 "$d/target/pgo/ship/corpus.txt")" ] &&
    [ "$(manifest_get "$d/$x86_training" "training jars" | awk '{print $1}')" = "$(manifest_sha256 "$d/target/pgo/ship/jars.txt")" ] ||
    { echo "the corpus's and the jars' lists are not the training's"; return 1; }
}

# header <dir> <version> <commit>: profiles.txt of the files under <dir>.
header() {
  local d=$1 arch profile training
  echo "profiles $2 $3"
  for arch in x86_64 aarch64; do
    if [ $arch = x86_64 ]; then profile=$x86_profile training=$x86_training; else profile=$arm_profile training=$arm_training; fi
    echo "profile $arch $(manifest_sha256 "$d/$profile") $profile"
    echo "training $arch $(manifest_sha256 "$d/$training") $training"
    echo "metadata $arch $(manifest_get "$d/$training" "training metadata")"
    echo "trained $arch $(manifest_get "$d/$training" "training release")"
    echo "trainer $arch $(manifest_get "$d/$training" "training trainer")"
  done
  for key in corpus jars programs; do echo "$key $(manifest_get "$d/$x86_training" "training $key" | awk '{print $1}')"; done
}

# complete <record> <tree> <version> <commit> <metadata>: a record of before its lines given them from the tree.
complete() {
  local r=$1 tree=$2 manifest=$2/target/ship/teq.manifest trainer qemu
  [ -n "$(manifest_get "$r" "training metadata")" ] || echo "training metadata $5" >> "$r"
  [ -n "$(manifest_get "$r" "training release")" ] || echo "training release $3 $4" >> "$r"
  [ -n "$(manifest_get "$r" "training trainer")" ] || echo "training trainer x86_64-unknown-linux-gnu native" >> "$r"
  if [ -z "$(manifest_get "$r" "training host")" ] || [ -z "$(manifest_get "$r" "training tuple rustc")" ]; then
    [ -f "$manifest" ] || { echo "profiles: $r records no machine or toolchain, and $tree has no target/ship/teq.manifest to take them from" >&2; return 1; }
    [ -n "$(manifest_get "$r" "training host")" ] || echo "training host $(manifest_get "$manifest" host)" >> "$r"
    grep -q '^training tuple ' "$r" || sed -n 's/^tuple /training tuple /p' "$manifest" >> "$r"
  fi
  trainer=$(manifest_get "$r" "training trainer")
  qemu=$(sed -n 's/^.* under qemu-user //p' <<< "$trainer")
  [ -z "$qemu" ] || [ -n "$(manifest_get "$r" "training tuple qemu-user")" ] || echo "training tuple qemu-user $qemu" >> "$r"
}

# pack <dir> <out>: the asset's tar of <dir>, the same bytes for the same files.
pack() {
  (cd "$1" && tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner --mode='u=rw,go=r' --format=ustar -cf - profiles.txt $files) > "$2.new" &&
    mv "$2.new" "$2"
}

# unpack <asset> <dir>: the asset's files under <dir>, its entries checked first.
unpack() {
  local entries
  entries=$(tar -tvf "$1" 2> /dev/null) || { echo "$1 is not a tar"; return 1; }
  [ "$(awk '{print $NF}' <<< "$entries" | LC_ALL=C sort | tr '\n' ' ')" = "$(printf '%s\n' profiles.txt $files | LC_ALL=C sort | tr '\n' ' ')" ] ||
    { echo "$1 holds $(awk '{print $NF}' <<< "$entries" | tr '\n' ' '), not the asset's files"; return 1; }
  ! awk '{print substr($1, 1, 1)}' <<< "$entries" | grep -qv -- - || { echo "$1 holds other than regular files"; return 1; }
  mkdir -p "$2" && tar -xf "$1" -C "$2" profiles.txt $files || { echo "$1 could not be unpacked"; return 1; }
  [ "$(header "$2" "$(awk 'NR == 1 {print $2}' "$2/profiles.txt")" "$(awk 'NR == 1 {print $3}' "$2/profiles.txt")")" = "$(cat "$2/profiles.txt")" ] ||
    { echo "$1's profiles.txt is not of its files"; return 1; }
  whole "$2" || return 1
  [[ $(head -1 "$2/profiles.txt") =~ ^profiles\ [0-9]+\.[0-9]+\.[0-9]+\ [0-9a-f]{40}$ ]] || { echo "$1's profiles.txt names no release"; return 1; }
}

write() {
  local tree=$1 out=$2 version commit metadata f name why
  [ -d "$tree/target" ] || fail "$tree is no ship's tree"
  version=$(release_version "$tree") || fail "$tree names no one version (above)"
  commit=$(git -C "$tree" rev-parse --verify HEAD 2> /dev/null) || fail "$tree is no checkout"
  metadata=${TEQ_PROFILES_METADATA:-$(cat "$tree/target/cross/metadata" 2> /dev/null)}
  stage=$(mktemp -d) || exit 1
  for f in $files; do
    [ -f "$tree/$f" ] || fail "$tree has no $f"
    mkdir -p "$stage/$(dirname "$f")" && cp "$tree/$f" "$stage/$f" || exit 1
  done
  for f in $x86_training $arm_training; do
    if [ -z "$(manifest_get "$stage/$f" "training metadata")" ]; then
      [[ $metadata =~ ^[0-9a-f]+$ ]] || fail "$f records no metadata, and neither TEQ_PROFILES_METADATA nor $tree/target/cross/metadata gives one"
    fi
    complete "$stage/$f" "$tree" "$version" "$commit" "$metadata" || exit 1
  done
  header "$stage" "$version" "$commit" > "$stage/profiles.txt" || exit 1
  why=$(whole "$stage") || fail "$tree's profiles are not whole: $why"
  name=teq-$version-profiles.tar
  mkdir -p "$out" && pack "$stage" "$out/$name" || fail "$out/$name not written"
  echo "$name $(manifest_sha256 "$out/$name")"
}

check() {
  local why
  stage=$(mktemp -d) || exit 1
  why=$(unpack "$1" "$stage") || fail "$why"
  echo "profiles: $1, of the ship $(head -1 "$stage/profiles.txt" | cut -d' ' -f2-): $(sed -n 's/^trained \([a-z0-9_]*\) \([^ ]*\) .*/\1 trained by \2/p' "$stage/profiles.txt" | tr '\n' ',' | sed 's/,$//; s/,/, /g')"
}

take() {
  local asset=$1 tag=$2 carry=$3 why sha f
  [[ $tag =~ ^[A-Za-z0-9._-]+$ ]] || fail "'$tag' is no release's tag"
  stage=$(mktemp -d) || exit 1
  why=$(unpack "$asset" "$stage") || fail "$why"
  sha=$(manifest_sha256 "$asset")
  for f in $x86_training $arm_training; do
    { grep -v '^training source ' "$stage/$f"; echo "training source $tag $sha"; } > "$stage/$f.new" && mv "$stage/$f.new" "$stage/$f" || exit 1
  done
  for f in $files; do
    mkdir -p "$carry/$(dirname "$f")" && cp "$stage/$f" "$carry/$f" || fail "$carry/$f not written"
  done
  echo "profiles: the profiles of $tag ($(basename "$asset"), sha256 $sha) in $carry: $(sed -n 's/^trained \([a-z0-9_]*\) \([^ ]*\) .*/\1 trained by \2/p' "$stage/profiles.txt" | tr '\n' ',' | sed 's/,$//; s/,/, /g')"
}

case ${1:-}/$# in
  write/3) write "$2" "$3" ;;
  check/2) check "$2" ;;
  take/4) take "$2" "$3" "$4" ;;
  *) echo "usage: $0 write <ship tree> <dir> | check <asset> | take <asset> <tag> <carry dir>" >&2; exit 2 ;;
esac
