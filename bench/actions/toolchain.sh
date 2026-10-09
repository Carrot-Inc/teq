#!/bin/bash
# bench/actions/toolchain.sh install <recipe dir> | check: the ship's build environment, the images the release
# workflow's Linux jobs run in, one per architecture (bench/actions/ship.Dockerfile, built by
# bench/actions/image.sh; docs/DEVELOPING.md, "Releases"): x86-64's, where every build is made, and arm64's,
# where the aarch64 trainer trains natively (bench/cross-ship.sh arm). The machine's architecture (uname -m)
# chooses each archive by its own pinned digest.
#
# install, as root on Ubuntu 26.04 (the image's build, or a machine of its own): every package from Ubuntu's
# snapshot of $snapshot (whose archive serves both architectures), so that the same recipe installs the same
# versions whenever it runs (wine among them on x86-64, whose version the binaries' manifests record); rustup's
# installer and the Rust toolchain at their pinned versions with llvm-tools and the std of every target the
# machine builds for (x86-64: the five the ship builds for and the musl one of the aarch64 trainer; arm64: the
# trainer's alone); node, sbt's launcher, scala-cli and gh from their release archives by their digests; and the
# ship's pinned downloads under /opt/teq-ship/cache, bench/ship.sh's TEQ_SHIP_CACHE (zig's tarball, its digest
# read from bench/zig.sh in <recipe dir>; on x86-64 also qemu-user's and the aarch64 sysroot's .debs, from
# bench/cross-ship.sh: an arm64 machine emulates nothing and runs no suite in the image). It writes
# /etc/teq-ship/recipe, the digest of the recipe (this file and the Dockerfile), which names the image's build
# environment.
#
# check, the preflight of a job in the image, from the root of a checkout: the image is the checkout's recipe
# (its digest), every tool is there at its pinned version, the pinned downloads have their digests, on x86-64 the
# pinned qemu-user runs here (`--version`, from the .deb unpacked) and wine does, and the tuple the manifests
# record (rustc, LLVM, cargo, the OS's release, wine on x86-64) is bench/ship-qualified.txt's when the file names
# this image (TEQ_SHIP_IMAGE, the workflow's reference by digest, as `image` on x86-64 and `image-aarch64` on
# arm64); otherwise the tuple is printed for the qualifying commit. Prints the tuple as `tuple <tool>
# <version...>` lines; exits 1 on a refusal.
set -uo pipefail
snapshot=20261001T000000Z
rust=1.98.1
rustup_version=1.29.1
node_version=24.21.0
sbt_version=1.11.7
sbt_sha256=1232818f91c39639a93bbe1108e12d94c7044a646a7847f1a3977b9e46716cd6
scala_cli_version=1.17.1
gh_version=2.102.0
# The Ubuntu packages: the builds' (gcc links build.rs), the suites' (wine for the Windows smoke, a JDK for the
# fixtures scala-cli compiles and for sbt), the scripts' (flock, gpg for a release's signature, time for the
# steps' peak memory).
packages="ca-certificates curl git python3 xz-utils dpkg gcc libc6-dev file time util-linux procps unzip zip gnupg
  openjdk-21-jdk-headless"
# Each architecture's archives by their digests, its Rust host and the std targets it builds for, and the image's
# key in the manifests' tuple.
case $(uname -m) in
  x86_64)
    triple=x86_64-unknown-linux-gnu image_key=image
    targets="aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-pc-windows-gnullvm aarch64-unknown-linux-musl"
    packages="$packages wine wine64"
    rustup_sha256=dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71
    node_arch=x64 node_sha256=fd8e59d5a511510f6a298afb548f18c7d2b1be404d8b4a27d94fbe49f56cb2d6
    scala_cli_arch=x86_64 scala_cli_sha256=4186bfac6552097fbfd3939bdf2ecdec1cb55baafcbf6259bef5c04e08858eb3
    gh_arch=amd64 gh_sha256=bb766f710eef8ede859c18578c72c327597cd4c8a85b06001b1f3843c6019386 ;;
  aarch64)
    triple=aarch64-unknown-linux-gnu image_key=image-aarch64
    targets="aarch64-unknown-linux-musl"
    rustup_sha256=15f6e4ce9f583b929c996c91562bad6d4454f3281de858b02cdfdef615fac433
    node_arch=arm64 node_sha256=6ad1325edbdb5649c379b75a237147a666c95d4f9ae8d340fef2d1575d289ad2
    scala_cli_arch=aarch64 scala_cli_sha256=a12ed53f4723f3e9312a2383e47664b4beaccf2be366cff40adf6b054426156f
    gh_arch=arm64 gh_sha256=7862c86c72f43df3a2d93ddde6f473285b4e2af61b494849846827e513ef6484 ;;
  *) echo "toolchain: no image for $(uname -m)" >&2; exit 1 ;;
esac
cache=/opt/teq-ship/cache
fail() { echo "toolchain: $*" >&2; exit 1; }
recipe_digest() { cat "$1/ship.Dockerfile" "$1/toolchain.sh" | sha256sum | cut -c1-64; }

# fetch <url> <sha256> <file>: the file by its digest, bounded.
fetch() {
  timeout 600 curl -sSfLo "$3" "$1" || fail "$1 could not be fetched"
  echo "$2  $3" | sha256sum -c --quiet || fail "$1's digest is not the pinned $2"
}

install() {
  local recipe=$1 work
  [ "$(id -u)" = 0 ] || fail "install runs as root"
  . /etc/os-release && [ "$VERSION_ID $VERSION_CODENAME" = "26.04 resolute" ] || fail "install is for Ubuntu 26.04, not $PRETTY_NAME"
  export DEBIAN_FRONTEND=noninteractive
  # The certificates first, from the base image's sources, the snapshot being served over https alone.
  timeout 600 apt-get update -q && timeout 600 apt-get install -y -q --no-install-recommends ca-certificates || fail "no certificates"
  rm -f /etc/apt/sources.list.d/*.sources /etc/apt/sources.list
  cat > /etc/apt/sources.list.d/snapshot.sources << EOF
Types: deb
URIs: https://snapshot.ubuntu.com/ubuntu/$snapshot
Suites: resolute resolute-updates resolute-security
Components: main universe
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg
Check-Valid-Until: no
EOF
  timeout 600 apt-get update -q || fail "the snapshot $snapshot could not be read"
  # shellcheck disable=SC2086
  timeout 1800 apt-get install -y -q --no-install-recommends $packages || fail "the packages could not be installed"
  apt-get clean && rm -rf /var/lib/apt/lists/*
  work=$(mktemp -d) || exit 1

  fetch "https://nodejs.org/dist/v$node_version/node-v$node_version-linux-$node_arch.tar.xz" $node_sha256 "$work/node.tar.xz"
  mkdir -p /opt/node && tar -xJf "$work/node.tar.xz" -C /opt/node --strip-components 1 || fail "node could not be unpacked"
  fetch "https://github.com/sbt/sbt/releases/download/v$sbt_version/sbt-$sbt_version.tgz" $sbt_sha256 "$work/sbt.tgz"
  tar -xzf "$work/sbt.tgz" -C /opt || fail "sbt could not be unpacked"
  fetch "https://github.com/VirtusLab/scala-cli/releases/download/v$scala_cli_version/scala-cli-$scala_cli_arch-pc-linux.gz" $scala_cli_sha256 "$work/scala-cli.gz"
  gzip -dc "$work/scala-cli.gz" > /usr/local/bin/scala-cli && chmod 755 /usr/local/bin/scala-cli || fail "scala-cli could not be unpacked"
  fetch "https://github.com/cli/cli/releases/download/v$gh_version/gh_${gh_version}_linux_$gh_arch.tar.gz" $gh_sha256 "$work/gh.tgz"
  tar -xzf "$work/gh.tgz" -C "$work" && cp "$work/gh_${gh_version}_linux_$gh_arch/bin/gh" /usr/local/bin/gh || fail "gh could not be unpacked"

  # Rust: the toolchain the qualified file names, never the channel rust-toolchain.toml floats on (the image
  # sets RUSTUP_TOOLCHAIN, which rustup prefers to the file).
  fetch "https://static.rust-lang.org/rustup/archive/$rustup_version/$triple/rustup-init" $rustup_sha256 "$work/rustup-init"
  chmod +x "$work/rustup-init"
  export RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo
  timeout 600 "$work/rustup-init" -y -q --no-modify-path --profile minimal --default-toolchain none || fail "rustup could not be installed"
  # shellcheck disable=SC2086
  timeout 1800 /opt/cargo/bin/rustup toolchain install "$rust" --profile minimal -c llvm-tools $(printf -- '-t %s ' $targets) ||
    fail "the toolchain $rust could not be installed"
  /opt/cargo/bin/rustup default "$rust"

  # The ship's pinned downloads, by the digests its scripts pin: zig's for this architecture; qemu-user's and the
  # sysroot's on x86-64, where the aarch64 work is emulated.
  mkdir -p "$cache" || exit 1
  local zig_version zig_sha256 qemu_version qemu_deb qemu_sha256 qemu_urls sysroot_deb sysroot_sha256 sysroot_url url
  eval "$(grep -E '^(zig_version|zig_sha256_(x86_64|aarch64))=' "$recipe/zig.sh")"
  zig_sha256=$(eval echo "\$zig_sha256_$(uname -m)")
  fetch "https://ziglang.org/download/$zig_version/zig-$(uname -m)-linux-$zig_version.tar.xz" "$zig_sha256" "$cache/zig-$(uname -m)-linux-$zig_version.tar.xz"
  if [ "$(uname -m)" = x86_64 ]; then
    eval "$(grep -E '^(qemu_version|qemu_sha256|qemu_deb|qemu_urls|sysroot_deb|sysroot_sha256|sysroot_url)=' "$recipe/cross-ship.sh")"
    for url in $qemu_urls; do timeout 600 curl -sSfLo "$cache/$qemu_deb" "$url" && break; done
    echo "$qemu_sha256  $cache/$qemu_deb" | sha256sum -c --quiet || fail "qemu-user's .deb could not be fetched by its digest"
    fetch "$sysroot_url" "$sysroot_sha256" "$cache/$sysroot_deb"
  fi

  mkdir -p /etc/teq-ship && recipe_digest "$recipe" > /etc/teq-ship/recipe || exit 1
  rm -rf "$work"
  echo "toolchain: installed, recipe $(cat /etc/teq-ship/recipe)"
}

# check: the preflight, from the root of a checkout.
check() {
  local want got tool qemu_version qemu_deb qemu_sha256 sysroot_deb sysroot_sha256 zig_version zig_sha256 work tuple wine= keys="rustc llvm cargo os"
  [ -f bench/actions/toolchain.sh ] || fail "check runs from the root of a checkout"
  want=$(recipe_digest bench/actions)
  got=$(cat /etc/teq-ship/recipe 2> /dev/null)
  [ "$got" = "$want" ] || fail "this is not the image of the checkout's recipe (${got:-no /etc/teq-ship/recipe}, the recipe's $want): bench/actions/image.sh builds it"
  for tool in git rustup cargo rustc curl dpkg node java sbt scala-cli gh python3 gcc flock gpg sha256sum /usr/bin/time; do
    command -v "$tool" > /dev/null || fail "no $tool on the PATH"
  done
  [ "$(rustc -V | awk '{print $2}')" = "$rust" ] || fail "rustc is $(rustc -V), not $rust (RUSTUP_TOOLCHAIN=${RUSTUP_TOOLCHAIN:-unset})"
  [ "$(rustc -vV | sed -n 's/^host: //p')" = "$triple" ] || fail "rustc's host is not $triple, this machine's"
  [ -x "$(rustc --print sysroot)/lib/rustlib/$triple/bin/llvm-profdata" ] || fail "no llvm-tools in $rust"
  for tool in $targets; do rustup target list --installed | grep -qx "$tool" || fail "no std for $tool in $rust"; done
  [ "$(node --version)" = "v$node_version" ] || fail "node is $(node --version), not $node_version"
  eval "$(grep -E '^(zig_version|zig_sha256_(x86_64|aarch64))=' bench/zig.sh)"
  zig_sha256=$(eval echo "\$zig_sha256_$(uname -m)")
  echo "$zig_sha256  $cache/zig-$(uname -m)-linux-$zig_version.tar.xz" | sha256sum -c --quiet 2> /dev/null || echo "toolchain: the cache has no zig $zig_version by bench/zig.sh's digest: the ship fetches it"
  if [ "$(uname -m)" = x86_64 ]; then
    command -v wine > /dev/null || fail "no wine on the PATH"
    eval "$(grep -E '^(qemu_version|qemu_sha256|qemu_deb|sysroot_deb|sysroot_sha256)=' bench/cross-ship.sh)"
    echo "$sysroot_sha256  $cache/$sysroot_deb" | sha256sum -c --quiet 2> /dev/null || echo "toolchain: the cache has no $sysroot_deb by bench/cross-ship.sh's digest: the ship fetches it"
    echo "$qemu_sha256  $cache/$qemu_deb" | sha256sum -c --quiet 2> /dev/null || fail "the cache has no $qemu_deb by bench/cross-ship.sh's digest"
    # The pinned qemu-user, which an emulated trainer and Linux aarch64 suite run under, loads here.
    work=$(mktemp -d) || exit 1
    timeout 120 dpkg -x "$cache/$qemu_deb" "$work" && timeout 30 "$work/usr/bin/qemu-aarch64" --version > /dev/null ||
      { rm -rf "$work"; fail "the pinned qemu-user does not run in this environment"; }
    rm -rf "$work"
    wine=$(wine --version 2> /dev/null) || fail "wine does not run"
    keys="$keys wine"
  fi
  # The tuple, as bench/ship-manifest.sh's manifest_tuple writes it.
  tuple=$(. bench/ship-manifest.sh && manifest_tuple && { [ -z "$wine" ] || echo "tuple wine $wine"; }) || fail "no tuple"
  echo "$tuple"
  if [ -n "${TEQ_SHIP_IMAGE:-}" ] && [ "$(sed -n "s/^$image_key //p" bench/ship-qualified.txt)" = "$TEQ_SHIP_IMAGE" ]; then
    for tool in $keys; do
      [ "$(sed -n "s/^tuple $tool //p" <<< "$tuple")" = "$(sed -n "s/^$tool //p" bench/ship-qualified.txt)" ] ||
        fail "$tool is '$(sed -n "s/^tuple $tool //p" <<< "$tuple")' in the qualified image, bench/ship-qualified.txt's '$(sed -n "s/^$tool //p" bench/ship-qualified.txt)'"
    done
    echo "toolchain: the image $TEQ_SHIP_IMAGE, the one bench/ship-qualified.txt qualifies, with its tuple"
  else
    echo "toolchain: the image ${TEQ_SHIP_IMAGE:-(unnamed)} is not the one bench/ship-qualified.txt qualifies: its binaries are held back from a publication"
  fi
}

case ${1:-} in
  install) [ $# -eq 2 ] || fail "usage: $0 install <recipe dir> | check"; install "$2" ;;
  check) [ $# -eq 1 ] || fail "usage: $0 install <recipe dir> | check"; check ;;
  *) echo "usage: $0 install <recipe dir> | check" >&2; exit 2 ;;
esac
