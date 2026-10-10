#!/bin/bash
# bench/cross-ship.sh [plain|arm|use|intel|linux-arm|linux-arm-build|linux-arm-suite|repeat|keys|windows]: the
# ship builds of teq for the platforms other than the Linux x86-64 machine they are made on, guided by a profile
# of bench/pgo.sh's training (docs/SPEED.md, "The ship from Linux"), linked by the pinned zig (bench/zig.sh).
# Without a stage, arm then use: the macOS arm64 binary bench/ship.sh stages and publishes. arm trains an
# aarch64 build, natively on an arm64 Linux machine or under qemu-user on the x86-64 one, into
# target/cross/arm.profdata (below); use translates a profile
# (bench/cross-profile.py) and builds aarch64-apple-darwin with it, through the flags bench/pgo.sh's use
# passes, into out/cross/teq-guided-arm, or with TEQ_CROSS_PROFILE=<file> into out/cross/teq-guided
# (teq-$TEQ_CROSS_NAME for another name); intel builds x86_64-apple-darwin the same way, guided by the
# Linux x86-64 training's profile (target/pgo/ship/teq.profdata, bench/pgo.sh ship train first), into
# out/cross/teq-guided-intel; linux-arm builds aarch64-unknown-linux-gnu against glibc $release_glibc,
# guided by the aarch64 trainer's profile, runs tests/run.sh on it under qemu-user over the floor's glibc
# (the sysroot below) with its outputs compared against the native suite's (bench/pgo.sh ship use first),
# into out/cross/teq-guided-linux-arm; linux-arm-build is that build alone, its manifest without a suite, and
# linux-arm-suite, on an arm64 Linux machine, its suite natively in the floor's container (below), whose
# record bench/ship.sh's stage gives the manifest (bench/ship-manifest.sh's manifest_attest); windows builds
# the one for x86_64 Windows (below). plain builds the
# ship profile for aarch64-apple-darwin without a profile, the lower bound, into out/cross/teq-plain; repeat
# builds the guided arm64 binary again in a clean directory and compares the two byte for byte; keys builds
# the instrumented Darwin binary and reports which of the profile's records it would take, weighed by
# their counts. Each binary's digest is in out/cross/SHA256SUMS, and beside it, written only once its build
# and the training it is guided by succeeded, its manifest (<binary>.manifest: its digest, the version,
# the commit, the toolchain, the profiles', the corpus's and the jars' digests, the flags;
# bench/ship-manifest.sh) and the string its `teq --version` prints (<binary>.version), since a Mach-O
# binary cannot be run here and the aarch64 ELF one only under qemu; out/cross/inputs.txt holds the last
# build's inputs. The logs, the effective compiler and linker commands among them, are in target/cross/.
#
# Measured on the reference machine against its native guided build (eleven interleaved
# `bench/compare.sh` runs each, 2026-10-05): the aarch64-trained binary +1.3% on the budget's
# totals, realistic-frontend +0.4%, realistic-api +0.0%, its instructions -0.3% and -0.5%, its
# outputs byte-identical on every budget program and target; the translated x86-64 profile +7.4%;
# no profile +28%. The binary is signed ad hoc by the linker as a native arm64 build is
# (`codesign -dv`: flags=0x20002(adhoc,linker-signed)), `codesign --verify --strict` passes, and
# coursier's download, the plugin's route, attaches no quarantine attribute. The Intel binary carries no
# signature (zig signs arm64 alone, and macOS asks none of an Intel binary).
#
# The profile is the aarch64 trainer's, target/cross/arm.profdata, or TEQ_CROSS_PROFILE=<file>:
# target/pgo/ship/teq.profdata from `bench/pgo.sh ship gen && bench/pgo.sh ship train` on this
# machine, the translated x86-64 route, or a Mac's (whose std is already Darwin's). LLVM
# keys a function's counts by its name, and two things in a Rust name depend on the build: teq's crate
# id and module name, which follow from the `-C metadata` cargo derives, among others from the host
# and the `--target` (13fbcf9b... on Linux without --target, 32fbd1f1... with this target); and the
# ids of the std crates, built per target. A rustc wrapper gives teq's crate the metadata of the
# profile's instrumented build, which its training records (`training metadata`, the aarch64 trainer's the
# metadata cargo gives the native build where it is trained), read here from cargo for a training without
# that line (TEQ_CROSS_METADATA=<hex> in its place, for a Mac's profile: `cargo build --profile ship -v`
# there prints it); bench/cross-profile.py rewrites the std ids.
# Each ship build is fat LTO in one codegen unit, 7 to 8 minutes here, hence the bound of
# bench/pgo.sh's ship stages. The paths a caller names are the caller's, whatever directory the
# script works in.
absolute() { case $1 in /* | "") echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
TEQ_CROSS_PROFILE=$(absolute "$TEQ_CROSS_PROFILE")
TEQ_CROSS_QEMU=$(absolute "$TEQ_CROSS_QEMU")
cd "$(dirname "$0")/.." || exit 1
. bench/ship-manifest.sh
. bench/ship-metadata.sh
stage=${1:-ship}
case $stage in
  plain | arm | use | intel | linux-arm | linux-arm-build | linux-arm-suite | repeat | keys | windows | ship) ;;
  *) echo "usage: $0 [plain|arm|use|intel|linux-arm|linux-arm-build|linux-arm-suite|repeat|keys|windows]"; exit 2 ;;
esac
# The machine: Linux x86-64 makes every stage but the native suite, the aarch64 trainer and the Linux aarch64
# suite under qemu-user; an arm64 Linux machine (uname's, the kernel's own word) the trainer's training and the
# Linux aarch64 suite natively, and nothing else.
native=
case "$(uname -s) $(uname -m)" in
  "Linux x86_64")
    [ "$stage" != linux-arm-suite ] || { echo "cross-ship: linux-arm-suite runs on an arm64 Linux machine; linux-arm runs the suite here under qemu-user"; exit 1; } ;;
  "Linux aarch64")
    native=1
    case $stage in arm | linux-arm-suite) ;; *) echo "cross-ship: $stage is made on Linux x86-64; an arm64 Linux machine makes arm and linux-arm-suite alone"; exit 1 ;; esac ;;
  *) echo "cross-ship: made on Linux x86-64 (arm and linux-arm-suite on an arm64 Linux machine too), not $(uname -s) $(uname -m)"; exit 1 ;;
esac
target=aarch64-apple-darwin
intel_target=x86_64-apple-darwin
linux_arm_target=aarch64-unknown-linux-gnu
windows_target=$(release_route_target windows-x86_64)
# qemu-user as Ubuntu 26.04 (resolute-updates) ships it, the .deb pinned by its digest, from the
# archive's pool or, once superseded there, Launchpad's copy of the same file.
qemu_version=10.2.1+ds-1ubuntu3.2
qemu_sha256=f0585a9676a039f46607f185b3657c1e78c1ba4187595724cc567fcc1ae0d1b9
qemu_deb=qemu-user_${qemu_version}_amd64.deb
qemu_urls="https://archive.ubuntu.com/ubuntu/pool/universe/q/qemu/$qemu_deb https://launchpad.net/ubuntu/+archive/primary/+files/$qemu_deb"
# The glibc the Linux aarch64 suite runs over under qemu-user: the floor's own, Debian 10's libc6 for
# arm64, pinned by its digest, from Debian's archive, which keeps a release's files for good; its loader
# at /lib/ld-linux-aarch64.so.1, the binary's interpreter, since Debian 10 is not merged-usr.
sysroot_deb=libc6_2.28-10+deb10u1_arm64.deb
sysroot_sha256=09f5e92b1527cdc06b757499bb4d97ddf1b064fc3d163f978e5de18b131cd330
sysroot_url=http://archive.debian.org/debian/pool/main/g/glibc/$sysroot_deb
# The Linux aarch64 suite's floor on an arm64 machine: Debian 10's own arm64 image by its digest (the arm64
# image of debian:10's index sha256:58ce6f1271ae..., Debian 10.13), where the binary's loader and libraries,
# and teq's restart of itself (/proc/self/exe), are the system's own, the floor's glibc package above
# installed over the image's later patch (2.28-10+deb10u3) and verified there; and node for the suite's
# programs, the toolchain's version for arm64 from its release archive by its digest (it needs glibc 2.28 and
# libstdc++'s GLIBCXX_3.4.21, which the image has).
floor_image=docker.io/library/debian@sha256:fba020fe61e2b15959ef887ea67c7fc61f77943d074889debe8d92e29402191f
floor_node_version=24.21.0
floor_node_sha256=6ad1325edbdb5649c379b75a237147a666c95d4f9ae8d340fef2d1575d289ad2
floor_node=node-v$floor_node_version-linux-arm64.tar.xz
floor_libc=${sysroot_deb#libc6_}
floor_libc=${floor_libc%_arm64.deb}
dir=$PWD/target/cross
out=$PWD/out/cross
bound=1200
profile=${TEQ_CROSS_PROFILE:-$dir/arm.profdata}
# The guided build's name: its directory and translated profile in target/cross/, its binary.
if [ -n "$TEQ_CROSS_PROFILE" ]; then name=${TEQ_CROSS_NAME:-guided}; else name=${TEQ_CROSS_NAME:-guided-arm}; fi
# A build that starts leaves no manifest of an earlier one to be staged, whatever fails after
# this, the toolchain's setup and the training among it: the binary the stage leads to loses its
# manifest and .version here, and gets them back only from ship() once the build checked out.
invalidate() { rm -f "$out/$1.manifest" "$out/$1.version"; }
case $stage in
  plain) invalidate teq-plain ;;
  windows) invalidate teq-windows-x86_64.exe ;;
  intel) invalidate teq-guided-intel ;;
  linux-arm | linux-arm-build) invalidate teq-guided-linux-arm ;;
  linux-arm-suite) rm -f "$out/teq-guided-linux-arm.suite" ;;
  use | ship) invalidate "teq-$name" ;;
  arm) [ -n "$TEQ_CROSS_PROFILE" ] || invalidate "teq-$name" ;;
esac
# The toolchain's host and LLVM tools, and zig's script, which reads them: every stage but the native suite,
# whose machine has no Rust toolchain.
if [ "$stage" != linux-arm-suite ]; then
  . bench/zig.sh
  host=$(rustc -vV | sed -n 's/^host: //p')
  tools=$(rustc --print sysroot)/lib/rustlib/$host/bin
fi
mkdir -p "$dir/bin" "$out" || exit 1

# std <triple>: the toolchain's std for that target, added through rustup when missing.
std() {
  rustup target list --installed | grep -qx "$1" && return 0
  echo "cross-ship: adding the std for $1"
  timeout 600 rustup target add "$1" > /dev/null || { echo "cross-ship: no std for $1; rustup target add $1"; return 1; }
}

# The rustc wrapper and the LLVM tools; the std of each target and zig are taken where a build needs them.
toolchain() {
  [ -x "$tools/llvm-profdata" ] || { echo "cross-ship: no llvm-profdata; rustup component add llvm-tools"; return 1; }
  # The compiler: teq's crate under its profile's metadata, every other crate as cargo has it.
  metadata_wrapper "$dir/bin/rustc"
}

# zig_ready: the pinned zig fetched once (bench/zig.sh), for the builds it links.
zig_ready() { [ -n "${zig:-}" ] || zig_fetch; }

# metadata [<training record>]: teq's crate metadata for the build, in TEQ_CROSS_METADATA: the caller's, or
# the one the training of the build's profile records (`training metadata`), or the one cargo gives teq in
# bench/pgo.sh's ship builds here (no --target, so for the host), read by a build that stops at teq's crate.
# No linker of the ship's reaches the probe: it never links.
metadata() {
  [ -z "$TEQ_CROSS_METADATA" ] || return 0
  if [ -n "${1:-}" ] && [ -n "$(manifest_get "$1" "training metadata")" ]; then
    TEQ_CROSS_METADATA=$(manifest_get "$1" "training metadata")
    [[ $TEQ_CROSS_METADATA =~ ^[0-9a-f]+$ ]] || { echo "cross-ship: $1 records the metadata '$TEQ_CROSS_METADATA'"; return 1; }
    echo "cross-ship: teq's crate under its profile's training's metadata, $TEQ_CROSS_METADATA"
    return 0
  fi
  TEQ_CROSS_METADATA=$(metadata_probe "$dir/bin/rustc" "$dir") || { echo "cross-ship: no metadata read for teq's native build"; return 1; }
  echo "cross-ship: teq's crate under the native build's metadata, $TEQ_CROSS_METADATA"
}

# build <name> <rustflags...>: the ship profile for aarch64-apple-darwin into target/cross/<name>.
build() {
  build_for $target "$@"
}

# build_for <triple> <name> <rustflags...>: the ship profile for that target, linked by zig's wrapper for
# the target (bench/zig.sh), or by the toolchain's rust-lld for the musl trainer (its C runtime is the
# target's own).
build_for() {
  local triple=$1 name=$2 linker
  shift 2
  std "$triple" || return 1
  if [ "$triple" = $arm_target ]; then linker=rust-lld; else zig_ready && zig_wrapper "$triple" && linker=$zig_cc || return 1; fi
  export TEQ_CROSS_LOG=$dir/$name
  rm -f "$TEQ_CROSS_LOG.link" "$TEQ_CROSS_LOG.rustc"
  # What cargo does not track reaches rustc and the linker through the wrappers: the pinned
  # metadata, the wrappers' own text, -dead_strip kept or dropped, the deployment target. Under
  # another of these a build would stay fresh and keep the binary of the earlier ones, so they
  # stamp the directory, and a build under another stamp starts from an empty one.
  local stamp
  stamp=$({ [ "$linker" = rust-lld ] || cat "$linker"; cat "$dir/bin/rustc"; echo "$TEQ_CROSS_METADATA ${TEQ_CROSS_KEEP_DEAD:-0} $MACOSX_DEPLOYMENT_TARGET"; } | sha256sum | cut -c1-16)
  if [ "$(cat "$dir/$name.stamp" 2> /dev/null)" != "$stamp" ]; then
    rm -rf "${dir:?}/$name" && echo "$stamp" > "$dir/$name.stamp" || return 1
  fi
  local flags=()
  # RUSTFLAGS replaces .cargo/config.toml's rustflags, as in bench/pgo.sh's stages; the plain build
  # keeps them, as a plain `cargo build --profile ship` does.
  # The caller's RUSTFLAGS do not enter.
  [ $# -eq 0 ] || flags=(RUSTFLAGS="$*")
  env -u RUSTFLAGS -u CARGO_ENCODED_RUSTFLAGS "${flags[@]}" RUSTC_WRAPPER="$dir/bin/rustc" TEQ_CROSS_METADATA="$TEQ_CROSS_METADATA" \
    "CARGO_TARGET_$(echo "$triple" | tr a-z- A-Z_)_LINKER=$linker" \
    timeout $bound cargo build -v --profile ship --target "$triple" --target-dir "$dir/$name" > "$TEQ_CROSS_LOG.log" 2>&1
  local status=$?
  grep -E '^error|Finished' "$TEQ_CROSS_LOG.log"
  return $status
}

# qemu-aarch64 for the trainer and the Linux aarch64 suite: TEQ_CROSS_QEMU's, or the pinned .deb's
# unpacked under target/cross/qemu (the archive's file read without root, `dpkg -x`).
qemu() {
  if [ -n "$TEQ_CROSS_QEMU" ]; then
    qemu=$TEQ_CROSS_QEMU
    [ -x "$qemu" ] || { echo "cross-ship: no qemu-aarch64 at $qemu"; return 1; }
    manifest_qemu="named $(manifest_sha256 "$qemu")"
    return 0
  fi
  qemu=$dir/qemu/usr/bin/qemu-aarch64
  local deb=$dir/$qemu_deb url
  if ! echo "$qemu_sha256  $deb" | sha256sum -c --quiet > /dev/null 2>&1; then
    rm -f "$deb"
    for url in $qemu_urls; do
      timeout 300 curl -sSfLo "$deb" "$url" && break
      rm -f "$deb"
    done
    echo "$qemu_sha256  $deb" | sha256sum -c --quiet || { echo "cross-ship: qemu-user's .deb is missing or its digest differs"; rm -f "$deb"; return 1; }
    rm -rf "$dir/qemu"
  fi
  if [ ! -x "$qemu" ]; then
    rm -rf "$dir/qemu" "$dir/qemu.new"
    timeout 120 dpkg -x "$deb" "$dir/qemu.new" && mv "$dir/qemu.new" "$dir/qemu" || { rm -rf "$dir/qemu.new"; return 1; }
  fi
  "$qemu" --version > /dev/null || { echo "cross-ship: $qemu does not run"; return 1; }
  manifest_qemu="$qemu_version $qemu_sha256"
}

# The floor's glibc for aarch64 under target/cross/sysroot-aarch64, the pinned .deb unpacked.
sysroot() {
  sysroot=$dir/sysroot-aarch64
  local deb=$dir/$sysroot_deb
  if ! echo "$sysroot_sha256  $deb" | sha256sum -c --quiet > /dev/null 2>&1; then
    rm -f "$deb"
    timeout 300 curl -sSfLo "$deb" "$sysroot_url" || { rm -f "$deb"; return 1; }
    echo "$sysroot_sha256  $deb" | sha256sum -c --quiet || { echo "cross-ship: the sysroot's .deb is missing or its digest differs"; rm -f "$deb"; return 1; }
    rm -rf "$sysroot"
  fi
  if [ ! -f "$sysroot/lib/ld-linux-aarch64.so.1" ]; then
    rm -rf "$sysroot" "$sysroot.new"
    timeout 120 dpkg -x "$deb" "$sysroot.new" && mv "$sysroot.new" "$sysroot" || { rm -rf "$sysroot.new"; return 1; }
    [ -f "$sysroot/lib/ld-linux-aarch64.so.1" ] || { echo "cross-ship: $sysroot_deb holds no /lib/ld-linux-aarch64.so.1"; return 1; }
  fi
  manifest_sysroot="$sysroot_deb $sysroot_sha256"
}

# qemu_wrapper <file> <binary> [<sysroot>]: a script that runs the aarch64 binary under qemu-user as the
# native binary would be run, over the sysroot when one is given. A build whose parallel attempt gives way
# is typed again by one worker in a process image that teq execs over its own (`serial_again`), which
# qemu-user cannot follow: the host has no handler for an aarch64 binary. The native exec writes no
# profile counts for the attempt, its image going without exiting; here the attempt's counts are dropped
# when counts are being taken (LLVM_PROFILE_FILE) and the build runs as the exec runs it, each such run a
# line of <file>.give-way.log: the files dropped, the serial run's status and files.
qemu_wrapper() {
  local file=$1 binary=$2 load=
  [ -z "${3:-}" ] || load="-L $3"
  cat > "$file" <<WRAPPER
#!/bin/bash
err=\$(mktemp)
raw=\${LLVM_PROFILE_FILE%/*}
before=
[ -z "\$LLVM_PROFILE_FILE" ] || before=\$(ls "\$raw" 2> /dev/null)
"$qemu" $load "$binary" "\$@" 2> "\$err"
status=\$?
if [ \$status -ne 0 ] && grep -q 'cannot type the build again by one worker: Exec format error' "\$err"; then
  dropped=0
  if [ -n "\$LLVM_PROFILE_FILE" ]; then
    for f in "\$raw"/*.profraw; do grep -qxF "\${f##*/}" <<< "\$before" || { rm -f "\$f"; dropped=\$((dropped + 1)); }; done
    before=\$(ls "\$raw" | wc -l)
  fi
  rm -f "\$err"
  TEQ_SERIAL="0;the exec under qemu-user" "$qemu" $load "$binary" "\$@"
  status=\$?
  kept=
  [ -z "\$LLVM_PROFILE_FILE" ] || kept=", \$((\$(ls "\$raw" | wc -l) - before)) count files kept"
  echo "teq \$*: the attempt's \$dropped count files dropped; the serial run exited \$status\$kept" >> "$file.give-way.log"
  exit \$status
fi
cat "\$err" >&2
rm -f "\$err"
exit \$status
WRAPPER
  chmod +x "$file"
}

# teq's crate id, the one most of the names read from the input carry.
teq_id() {
  grep -o 'Cs[0-9A-Za-z]*_3teq' | sort | uniq -c | sort -rn | awk 'NR == 1 {print $2}'
}

# stamped <binary> [<profile>]: what the binary claims of its build, read without running it: its
# `teq --version` is `teq <version> <revision>`, with " pgo" after a guided build, a string in the binary.
stamped() {
  local binary=$1 revision
  revision=$(git rev-parse --short HEAD)
  if [ -n "$2" ]; then
    grep -qa "$revision pgo" "$binary" || { echo "cross-ship: $binary does not read '$revision pgo'"; return 1; }
  else
    grep -qa "$revision" "$binary" && ! grep -qaE "$revision (pgo|instrumented)" "$binary" || { echo "cross-ship: $binary does not read '$revision' alone"; return 1; }
  fi
}

# named_as <binary> [<profile>]: a guided build names teq's crate as its profile does, or no record
# would find its function.
named_as() {
  local binary=$1 id want
  id=$("$tools/llvm-nm" -j "$binary" | teq_id)
  [ -n "$2" ] || { echo "cross-ship: teq's crate in $binary is $id"; return 0; }
  want=$("$tools/llvm-profdata" show --all-functions "$2" | teq_id)
  [ "$id" = "$want" ] || { echo "cross-ship: teq's crate is $id in $binary but $want in its profile: TEQ_CROSS_METADATA is not the metadata of the profile's build"; return 1; }
  echo "cross-ship: teq's crate is $id in $binary and in its profile"
}

# check <binary> <triple> [<profile>]: a Darwin binary as it claims to be, read without running it: the
# stamp, a Mach-O for the target's CPU, its load commands, the dylibs it links, and its crate's name.
check() {
  local binary=$1 triple=$2 want
  stamped "$binary" "$3" || return 1
  "$tools/llvm-objdump" --macho --private-headers "$binary" > "$binary.headers" || return 1
  local cpu minos sdk stack sign dylibs
  cpu=$(awk '/^Mach header/ {getline; getline; print $2; exit}' "$binary.headers")
  case $triple in aarch64-*) want=ARM64 ;; x86_64-*) want=X86_64 ;; esac
  [ "$cpu" = "$want" ] || { echo "cross-ship: $binary is a Mach-O for $cpu, not $want"; return 1; }
  minos=$(awk '/cmd LC_BUILD_VERSION/ {b=1} b && /minos/ {print $2; exit}' "$binary.headers")
  sdk=$(awk '/cmd LC_BUILD_VERSION/ {b=1} b && /sdk/ {print $2; exit}' "$binary.headers")
  stack=$(awk '/cmd LC_MAIN/ {b=1} b && /stacksize/ {print $2; exit}' "$binary.headers")
  sign=$(grep -c 'cmd LC_CODE_SIGNATURE' "$binary.headers")
  dylibs=$("$tools/llvm-objdump" --macho --dylibs-used "$binary" | tail -n +2 | awk '{print $1}' | tr '\n' ' ')
  [ "$minos" = "$MACOSX_DEPLOYMENT_TARGET" ] || { echo "cross-ship: $binary claims macOS $minos, not $MACOSX_DEPLOYMENT_TARGET"; return 1; }
  [ "$dylibs" = "/usr/lib/libSystem.B.dylib " ] || { echo "cross-ship: $binary links $dylibs, not libSystem alone"; return 1; }
  echo "cross-ship: $binary: $cpu, macOS $minos and later (SDK $sdk), main stack $stack, code signature $sign, links $dylibs"
  named_as "$binary" "$3"
}

# check_elf <binary> [<profile>]: the Linux aarch64 binary as it claims to be, read without running it: the
# stamp, an ELF for AArch64 with the system loader as its interpreter, the libraries it loads and the
# glibc floor its version-needs table names (bench/ship-manifest.sh's manifest_glibc, against the
# release's), and its crate's name. Sets $elf_glibc, the manifest's line.
check_elf() {
  local binary=$1 machine interp
  stamped "$binary" "$2" || return 1
  "$tools/llvm-readobj" --file-headers "$binary" > "$binary.headers" || return 1
  machine=$(sed -n 's/^  Machine: EM_\([A-Za-z0-9_]*\).*/\1/p' "$binary.headers")
  [ "$machine" = AARCH64 ] || { echo "cross-ship: $binary is an ELF for ${machine:-no machine}, not AArch64 ($binary.headers)"; return 1; }
  interp=$("$tools/llvm-readobj" --string-dump=.interp "$binary" | sed -n 's/^\[ *0\] //p')
  [ "$interp" = /lib/ld-linux-aarch64.so.1 ] || { echo "cross-ship: $binary's interpreter is '$interp', not /lib/ld-linux-aarch64.so.1"; return 1; }
  elf_glibc=$(manifest_glibc "$binary") || return 1
  [ "$(printf '%s\n%s\n' "${elf_glibc%% *}" "$release_glibc" | sort -V | tail -1)" = "$release_glibc" ] || { echo "cross-ship: $binary loads on glibc ${elf_glibc%% *} and later, past the floor $release_glibc"; return 1; }
  echo "cross-ship: $binary: ELF AArch64, interpreter $interp, loads on glibc ${elf_glibc%% *} and later, needs ${elf_glibc#* needs }"
  named_as "$binary" "$2"
}

# The digests of the binaries in out/cross/, their sidecars left out.
sums() {
  (cd "$out" && find . -maxdepth 1 -name 'teq-*' \( ! -name '*.*' -o -name '*.exe' \) -printf '%f\n' | sort | xargs sha256sum > SHA256SUMS)
}

# ship <name> <triple> <flags> <training file|-> [<manifest line>...]: the build's binary into out/cross/,
# its digest into SHA256SUMS, then its manifest and .version, and the inputs of the build.
ship() {
  local name=$1 triple=$2 flags=$3 training=$4 binary=$dir/$1/$2/ship/teq
  shift 4
  mkdir -p "$out" && rm -f "$out/teq-$name" "$out/teq-$name.manifest" "$out/teq-$name.version" && cp "$binary" "$out/teq-$name" || return 1
  sums
  grep " teq-$name\$" "$out/SHA256SUMS"
  manifest_write "$out/teq-$name" "$triple" "$flags" "$training" "$@" "metadata $TEQ_CROSS_METADATA" "wrappers $(cat "$dir/$name.stamp")" || return 1
  grep -v '^binary \|^version ' "$out/teq-$name.manifest" > "$out/inputs.txt"
  echo "cross-ship: $out/teq-$name: $(cat "$out/teq-$name.version"), manifest ${out#$PWD/}/teq-$name.manifest"
}

plain() {
  build plain || return 1
  check "$dir/plain/$target/ship/teq" $target && ship plain $target "" -
}

# guided <triple> <name> <profile>: the profile translated to the target's std ids, the guided build, its
# counts; $guided_flags the flags of the build.
guided() {
  local triple=$1 name=$2 profile=$3
  std "$triple" || return 1
  timeout 300 python3 bench/cross-profile.py translate "$profile" "$dir/$name.profdata" "$triple" || return 1
  guided_flags="-Cprofile-use=$dir/$name.profdata -Cllvm-args=-pgo-warn-missing-function"
  build_for "$triple" "$name" $guided_flags || return 1
  echo "cross-ship: $(grep -c 'no profile data available for function' "$dir/$name.log") functions without a record, $(grep -c 'hash mismatch' "$dir/$name.log") whose control flow differs from the record's"
}

# use: the guided arm64 Darwin build. The trainer's profile goes with its training's record, which must
# be of that profile; a profile named by TEQ_CROSS_PROFILE has none, and its binary is not staged.
use() {
  [ -f "$profile" ] || { echo "cross-ship: no profile at $profile; $0 arm, or TEQ_CROSS_PROFILE=<file>"; return 1; }
  local training=- original
  original=$(manifest_sha256 "$profile")
  if [ -z "$TEQ_CROSS_PROFILE" ]; then
    training=$dir/arm.training
    [ "$(manifest_get "$training" "training profile")" = "$original" ] || { echo "cross-ship: $profile is not the profile of the training recorded in $training; $0 arm"; return 1; }
  fi
  # The trainer's qemu-user, when it ran under one, is its training's to record, not this build's.
  manifest_qemu=
  metadata "$([ "$training" = - ] || echo "$training")" && guided $target "$name" "$profile" || return 1
  check "$dir/$name/$target/ship/teq" $target "$dir/$name.profdata" &&
    ship "$name" $target "$guided_flags" "$training" "profile-original $original ${profile#$PWD/}" "profile $(manifest_sha256 "$dir/$name.profdata") ${dir#$PWD/}/$name.profdata"
}

# intel: the guided x86-64 Darwin build, from the Linux x86-64 training's profile (bench/pgo.sh ship
# train), whose names are the native build's already; the std ids translated to the target's. The build
# keeps the target's default CPU (penryn, whose features the installed std is compiled with: a lower one
# would keep LLVM from inlining std into teq); the counts say how much of the x86-64 profile it takes.
intel() {
  local profile=$PWD/target/pgo/ship/teq.profdata training=$PWD/target/pgo/ship/training.txt original
  [ -f "$profile" ] && [ -f "$training" ] || { echo "cross-ship: no Linux training at ${profile#$PWD/}; bench/pgo.sh ship gen && bench/pgo.sh ship train"; return 1; }
  original=$(manifest_sha256 "$profile")
  [ "$(manifest_get "$training" "training profile")" = "$original" ] || { echo "cross-ship: $profile is not the profile of the training recorded in $training; bench/pgo.sh ship train"; return 1; }
  metadata "$training" && guided $intel_target guided-intel "$profile" || return 1
  check "$dir/guided-intel/$intel_target/ship/teq" $intel_target "$dir/guided-intel.profdata" &&
    ship guided-intel $intel_target "$guided_flags" "$training" "profile-original $original ${profile#$PWD/}" "profile $(manifest_sha256 "$dir/guided-intel.profdata") ${dir#$PWD/}/guided-intel.profdata"
}

# linux-arm: the guided Linux aarch64 build, from the aarch64 trainer's profile (arm), built for the
# generic CPU (Graviton, Ampere and Docker on Apple silicon alike) where the trainer runs apple-m1's;
# the counts say what of the profile it takes. Its suite: tests/run.sh under qemu-user over the floor's
# glibc, the outputs compared file by file with the native suite's (bench/pgo.sh ship use, out/tests).
# linux-arm-build: the build alone, its manifest without a suite, which linux-arm-suite gives it on an arm64
# machine; nothing of qemu-user or the sysroot enters it.
linux_arm_build() {
  local profile=$dir/arm.profdata training=$dir/arm.training original
  [ -f "$profile" ] && [ -f "$training" ] || { echo "cross-ship: no aarch64 training at ${profile#$PWD/}; $0 arm"; return 1; }
  original=$(manifest_sha256 "$profile")
  [ "$(manifest_get "$training" "training profile")" = "$original" ] || { echo "cross-ship: $profile is not the profile of the training recorded in $training; $0 arm"; return 1; }
  metadata "$training" || return 1
  guided $linux_arm_target guided-linux-arm "$profile" || return 1
  linux_arm_binary=$dir/guided-linux-arm/$linux_arm_target/ship/teq
  check_elf "$linux_arm_binary" "$dir/guided-linux-arm.profdata" || return 1
  linux_arm_lines=("profile-original $original ${profile#$PWD/}" "profile $(manifest_sha256 "$dir/guided-linux-arm.profdata") ${dir#$PWD/}/guided-linux-arm.profdata" "glibc $elf_glibc")
  [ "$stage" = linux-arm-build ] || return 0
  ship guided-linux-arm $linux_arm_target "$guided_flags" "$training" "${linux_arm_lines[@]}" || return 1
  echo "cross-ship: its suite is linux-arm-suite's, on an arm64 Linux machine; the stage takes the binary once its record names these bytes"
}

# compare_suites <outputs>: the suite's outputs under <outputs> and the native suite's (out/tests) the same
# files, each the same bytes; sets $same and $total.
compare_suites() {
  local names f
  names=$(cd out/tests && find . -maxdepth 1 -type f \( -name '*.js' -o -name '*.actual' \) -printf '%f\n' | LC_ALL=C sort)
  [ "$names" = "$(cd "$1" && find . -maxdepth 1 -type f \( -name '*.js' -o -name '*.actual' \) -printf '%f\n' | LC_ALL=C sort)" ] ||
    { echo "cross-ship: the suite on aarch64 wrote other files than the native suite ($(wc -l <<< "$names") native): $(diff <(echo "$names") <(cd "$1" && find . -maxdepth 1 -type f \( -name '*.js' -o -name '*.actual' \) -printf '%f\n' | LC_ALL=C sort) | grep '^[<>]' | head -5 | tr '\n' ' ')"; return 1; }
  # A macro reads a source's path as the suite gives it, relative (`SourceFileMethods.path`), so the two
  # machines' outputs compare as they are.
  same=0 total=0
  local differing=
  for f in $names; do
    total=$((total + 1))
    if cmp -s "out/tests/$f" "$1/$f"; then same=$((same + 1)); else differing="$differing $f"; fi
  done
  [ $total -gt 0 ] && [ $same -eq $total ] || { echo "cross-ship: $same of $total outputs of the suite on aarch64 are the native suite's; differing:$differing"; return 1; }
}

linux_arm() {
  local training=$dir/arm.training binary
  [ -d out/tests ] && [ -n "$(ls out/tests 2> /dev/null)" ] || { echo "cross-ship: no native suite output under out/tests to compare with; bench/pgo.sh ship use first"; return 1; }
  [ -f "$training" ] || { echo "cross-ship: no aarch64 training at ${training#$PWD/}; $0 arm"; return 1; }
  qemu && sysroot || return 1
  # A trainer under qemu-user ran under the one the suite runs under, the one the manifest records.
  local trainer
  trainer=$(manifest_get "$training" "training trainer" | sed -n 's/^.* under qemu-user //p')
  [ -z "$trainer" ] || [ "$trainer" = "$manifest_qemu" ] || { echo "cross-ship: the profile was trained under qemu-user '$trainer', the suite runs under '$manifest_qemu'; $0 arm under the same"; return 1; }
  linux_arm_build || return 1
  binary=$linux_arm_binary
  # The suite over the floor's glibc, each case given six times the native bound, the outputs beside the
  # native run's; a build that gives way runs again serially (qemu_wrapper).
  qemu_wrapper "$dir/guided-linux-arm.teq" "$binary" "$sysroot"
  rm -rf out/tests-arm "$dir/guided-linux-arm.teq.give-way.log"
  local sha smoke status skipped same total
  sha=$(manifest_sha256 "$binary")
  smoke=$(TEQ=$dir/guided-linux-arm.teq TEQ_TEST_OUT=out/tests-arm TEQ_TEST_TIMEOUT=120 timeout 1800 tests/run.sh 2>&1)
  status=$?
  echo "$smoke" > "$dir/guided-linux-arm.suite"
  [ ! -f "$dir/guided-linux-arm.teq.give-way.log" ] || sed 's/^/cross-ship: give-way: /' "$dir/guided-linux-arm.teq.give-way.log"
  [ $status -eq 0 ] || { tail -20 <<< "$smoke"; echo "cross-ship: tests/run.sh under qemu-user failed (exit $status, $dir/guided-linux-arm.suite)"; return 1; }
  skipped=$(grep -c '^skip ' <<< "$smoke")
  # The two suites wrote the same files, and each file the same bytes.
  compare_suites out/tests-arm || return 1
  [ "$(manifest_sha256 "$binary")" = "$sha" ] || { echo "cross-ship: $binary changed while its suite ran"; return 1; }
  local counts
  counts="$(tail -1 <<< "$smoke"), $skipped of them skipped for missing jars, outputs $same of $total as the native suite's, under qemu-user $manifest_qemu sysroot $manifest_sysroot"
  echo "cross-ship: tests/run.sh under qemu-user over $sysroot_deb: $counts"
  ship guided-linux-arm $linux_arm_target "$guided_flags" "$training" "${linux_arm_lines[@]}" "suite $sha tests/run.sh passed: $counts"
}

# floor_fetch <url> <sha256> <file>: a file of the floor's by its pinned digest, under target/cross (a copy
# there already, a cache's, taken when its digest is the pinned one).
floor_fetch() {
  echo "$2  $3" | sha256sum -c --quiet > /dev/null 2>&1 && return 0
  rm -f "$3"
  timeout 300 curl -sSfLo "$3" "$1" || { rm -f "$3"; echo "cross-ship: $1 could not be fetched"; return 1; }
  echo "$2  $3" | sha256sum -c --quiet || { rm -f "$3"; echo "cross-ship: $1's digest is not the pinned $2"; return 1; }
}

# linux-arm-suite, on an arm64 Linux machine: the suite of linux-arm-build's binary natively, in the floor's
# container (floor_image) with the floor's glibc package installed there (dpkg -i, dpkg --verify), as the
# machine's user, each case given three times the native bound, no network, the corpus's jars read from
# COURSIER_CACHE and the jars the native suite built (out/tests-jars.tgz) restored as bench/actions/jars.sh
# does; its outputs compared with the native suite's (out/tests), the bytes unchanged throughout. Writes its
# record, out/cross/teq-guided-linux-arm.suite: the `suite` line against the binary's digest and the
# environment's `suite-tuple` lines (bench/ship-manifest.sh), which the stage gives the manifest.
linux_arm_suite() {
  local binary=$out/teq-guided-linux-arm sha deb=$dir/$sysroot_deb node=$dir/node-aarch64 name=teq-floor-$$ status smoke skipped same total
  [ -f "$binary" ] && [ -f "$binary.manifest" ] || { echo "cross-ship: no Linux aarch64 build at ${binary#$PWD/}; $0 linux-arm-build on the x86-64 machine first"; return 1; }
  sha=$(manifest_sha256 "$binary")
  [ "$(manifest_get "$binary.manifest" binary)" = "$sha" ] || { echo "cross-ship: ${binary#$PWD/} is not the binary of its manifest"; return 1; }
  ! grep -q '^suite ' "$binary.manifest" || { echo "cross-ship: ${binary#$PWD/}'s manifest has a suite already"; return 1; }
  [ -d out/tests ] && [ -n "$(ls out/tests 2> /dev/null)" ] || { echo "cross-ship: no native suite output under out/tests to compare with; bench/pgo.sh ship use's"; return 1; }
  [ -f out/tests-jars.tgz ] || { echo "cross-ship: no out/tests-jars.tgz, the jars the native suite built"; return 1; }
  command -v docker > /dev/null || { echo "cross-ship: no docker, which runs the floor's container"; return 1; }
  floor_fetch "$sysroot_url" $sysroot_sha256 "$deb" &&
    floor_fetch "https://nodejs.org/dist/v$floor_node_version/$floor_node" $floor_node_sha256 "$dir/$floor_node" || return 1
  rm -rf "$node" && mkdir -p "$node" && timeout 120 tar -xJf "$dir/$floor_node" -C "$node" --strip-components 1 || return 1
  # The built jars where the suite looks for them from this tree, under a TMPDIR the container sees at the same path.
  mkdir -p "$dir/tmp" && TMPDIR=$dir/tmp timeout 300 bench/actions/jars.sh restore out/tests-jars.tgz || return 1
  timeout 900 docker pull -q --platform linux/arm64 "$floor_image" > /dev/null || { echo "cross-ship: $floor_image could not be pulled"; return 1; }
  rm -rf out/tests-arm && mkdir -p out/tests-arm || return 1
  export COURSIER_CACHE=${COURSIER_CACHE:-$HOME/.cache/coursier/v1}
  # The JDK's lib/ct.sym, which the cases with jars type java.lang against: this machine's JDK, mounted read-only
  # at its own path (teq reads the archive and runs nothing of the JDK, so the container's libc does not matter).
  local jdk; jdk=${JAVA_HOME:-$(dirname "$(dirname "$(readlink -f "$(command -v java)")")")}
  [ -f "$jdk/lib/ct.sym" ] || { echo "cross-ship: no JDK with lib/ct.sym for the floor's suite (JAVA_HOME or java on the PATH)"; return 1; }
  # Root installs the floor's package and checks it; the suite runs as this machine's user, so that what it
  # writes is the user's.
  timeout -k 60 2100 docker run --rm --init --name "$name" --platform linux/arm64 --network none \
    -v "$PWD:$PWD" -v "$COURSIER_CACHE:$COURSIER_CACHE:ro" -v "$jdk:$jdk:ro" -w "$PWD" \
    -e FLOOR_DEB="$deb" -e FLOOR_VERSION="$floor_libc" -e FLOOR_USER="$(id -u):$(id -g)" -e FLOOR_NODE="$node" \
    -e TEQ="$binary" -e COURSIER_CACHE="$COURSIER_CACHE" -e TMPDIR="$dir/tmp" -e HOME="$dir/tmp" -e DIFF_LINES="${DIFF_LINES:-10}" -e JAVA_HOME="$jdk" \
    "$floor_image" bash -c '
      installed=$(dpkg -i "$FLOOR_DEB" 2>&1) || { echo "floor: $FLOOR_DEB could not be installed: $installed"; exit 3; }
      version=$(dpkg-query -W -f "\${Version}" libc6)
      [ "$version" = "$FLOOR_VERSION" ] && [ -z "$(dpkg --verify libc6)" ] || { echo "floor: libc6 is $version, not its package'"'"'s $FLOOR_VERSION whole"; exit 3; }
      user() { setpriv --reuid="${FLOOR_USER%:*}" --regid="${FLOOR_USER#*:}" --clear-groups env PATH="$FLOOR_NODE/bin:/usr/bin:/bin" "$@"; }
      teq=$(user "$TEQ" --version) || { echo "floor: $TEQ does not run here"; exit 3; }
      echo "floor: $(. /etc/os-release && echo "$PRETTY_NAME"), libc6 $version, $(uname -m), node $(user node --version), $teq"
      user env TEQ_TEST_OUT=out/tests-arm TEQ_TEST_TIMEOUT=60 DIFF_LINES="$DIFF_LINES" timeout 1800 tests/run.sh' > "$dir/guided-linux-arm.suite" 2>&1
  status=$?
  timeout 60 docker rm -f "$name" > /dev/null 2>&1
  smoke=$(cat "$dir/guided-linux-arm.suite")
  grep '^floor: ' <<< "$smoke" | sed 's/^/cross-ship: /'
  [ $status -eq 0 ] || { tail -20 <<< "$smoke"; echo "cross-ship: tests/run.sh in the floor's container failed (exit $status, $dir/guided-linux-arm.suite)"; return 1; }
  skipped=$(grep -c '^skip ' <<< "$smoke")
  compare_suites out/tests-arm || return 1
  [ "$(manifest_sha256 "$binary")" = "$sha" ] || { echo "cross-ship: $binary changed while its suite ran"; return 1; }
  {
    echo "suite $sha tests/run.sh passed: $(tail -1 <<< "$smoke"), $skipped of them skipped for missing jars, outputs $same of $total as the native suite's, natively in $floor_image over $sysroot_deb $sysroot_sha256"
    echo "suite-tuple floor-aarch64 $floor_image"
    echo "suite-tuple sysroot-aarch64 $sysroot_deb $sysroot_sha256"
    echo "suite-tuple node-aarch64 $floor_node_version $floor_node_sha256"
  } > "$out/teq-guided-linux-arm.suite.new" && mv "$out/teq-guided-linux-arm.suite.new" "$out/teq-guided-linux-arm.suite" || return 1
  echo "cross-ship: tests/run.sh natively in the floor's container: $(sed -n 's/^suite [0-9a-f]* tests\/run.sh passed: //p' "$out/teq-guided-linux-arm.suite")"
}

# The guided arm64 Darwin build again from a clean directory, the same translated profile: the bytes must agree.
repeat() {
  [ -f "$dir/$name.profdata" ] || { echo "cross-ship: no translated profile; $0 use"; return 1; }
  rm -rf "$dir/$name-repeat"
  build "$name-repeat" -Cprofile-use="$dir/$name.profdata" -Cllvm-args=-pgo-warn-missing-function || return 1
  local a=$dir/$name/$target/ship/teq b=$dir/$name-repeat/$target/ship/teq
  cmp "$a" "$b" || { echo "cross-ship: $b differs from $a"; return 1; }
  echo "cross-ship: the guided build again from a clean directory is the same, byte for byte ($(sha256sum < "$b" | cut -c1-16))"
}

# The instrumented build, linked without -dead_strip, which drops most of __llvm_prf_data's records
# under zig's linker: a diagnostic, its binary read and never run.
keys() {
  [ -f "$dir/$name.profdata" ] || { echo "cross-ship: no translated profile; $0 use"; return 1; }
  TEQ_CROSS_KEEP_DEAD=1 build keys -Cprofile-generate="$dir/keys/raw" || return 1
  timeout 300 python3 bench/cross-profile.py keys "$dir/$name.profdata" "$dir/keys/$target/ship/teq"
}

# arm: the profile of an aarch64 trainer. The x86_64 profile's records miss a fifth of its counts
# on Darwin: LLVM shapes a function's control flow before counting it, with the target's cost
# models (the pre-inliner, SimplifyCFG's speculation), and a record whose control flow differs is
# dropped, among them the lexer's loop, the interpreter's eval and the typer's subtyping. The trainer
# is aarch64 Linux on musl, static, linked by the toolchain's rust-lld, built for the Darwin build's CPU
# (apple-m1, that target's default), under the metadata cargo gives teq's native build where it is trained
# (metadata above); bench/pgo.sh's training runs on it as it is, in a copy of the files it reads under
# target/cross/arm-train, whose instrumented binary is the trainer itself on an arm64 Linux machine (which
# must have apple-m1's features: the trainer's `--version` is run first), or on the x86-64 one the trainer
# under qemu-user (qemu_wrapper), which runs it without a sysroot: the pinned .deb's (qemu() above), or
# TEQ_CROSS_QEMU's. The two read the same corpus, and the native run is the one qemu_wrapper reproduces
# (a give-way's attempt keeps no counts). Its record, target/cross/arm.training, is pgo.sh's (the programs,
# the runs merged, the corpus's and the jars' digests, the profile's) with the release and commit trained,
# the metadata, the training machine, the trainer (`native`, or under qemu-user with the give-ways counted)
# and the machine's toolchain (`training tuple`), which travel with the profile into every manifest it
# guides (bench/ship-manifest.sh).
arm_target=aarch64-unknown-linux-musl
arm() {
  local train=$dir/arm-train trainer=$dir/arm/$arm_target/ship/teq
  rm -f "$dir/arm.profdata" "$dir/arm.training"
  if [ -z "$native" ]; then
    qemu || return 1
    echo "cross-ship: $("$qemu" --version | head -1), $qemu"
  else
    manifest_qemu=
    echo "cross-ship: the aarch64 trainer runs natively on this $(uname -m) machine"
  fi
  std $arm_target && metadata || return 1
  rm -rf "$dir/arm/raw"
  build_for $arm_target arm -Cprofile-generate="$dir/arm/raw" -Ctarget-cpu=apple-m1 || return 1
  rm -rf "$train" && mkdir -p "$train/target/pgo/ship/gen/ship" || return 1
  tar -c rust-toolchain.toml bench tests/support tests/cases/end_markers.scala | tar -x -C "$train" || return 1
  if [ -n "$native" ]; then
    cp "$trainer" "$train/target/pgo/ship/gen/ship/teq" || return 1
    LLVM_PROFILE_FILE=$dir/arm/version.profraw timeout 60 "$trainer" --version > /dev/null 2>&1 ||
      { rm -f "$dir/arm/version.profraw"; echo "cross-ship: the aarch64 trainer does not run on this machine (apple-m1's features?)"; return 1; }
    rm -f "$dir/arm/version.profraw"
  else
    qemu_wrapper "$train/target/pgo/ship/gen/ship/teq" "$trainer"
  fi
  # A run under qemu-user takes six times or more a native one's time: twice the native bound.
  TEQ_PGO_RUN_BOUND=$([ -n "$native" ] && echo 60 || echo 120) "$train/bench/pgo.sh" ship train || return 1
  local gives=0 log=$train/target/pgo/ship/gen/ship/teq.give-way.log
  if [ -f "$log" ]; then
    sed 's/^/cross-ship: give-way: /' "$log"
    gives=$(wc -l < "$log")
    # A serial run that kept no counts would leave its program out of the profile.
    ! grep -qv 'exited 0, [1-9][0-9]* count files kept$' "$log" || { echo "cross-ship: a serial run under qemu failed or kept no counts"; return 1; }
  fi
  [ "$(manifest_get "$train/target/pgo/ship/training.txt" "training status")" = ok ] || { echo "cross-ship: no successful training recorded"; return 1; }
  cp "$train/target/pgo/ship/teq.profdata" "$dir/arm.profdata.new" &&
    {
      grep -v -E '^training (release|metadata|trainer|host|give-ways|tuple) ' "$train/target/pgo/ship/training.txt"
      echo "training release $(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' Cargo.toml | head -1) $(git rev-parse HEAD)"
      echo "training metadata $TEQ_CROSS_METADATA"
      echo "training host $host"
      if [ -n "$native" ]; then
        echo "training trainer $arm_target native"
      else
        echo "training give-ways $gives"
        echo "training trainer $arm_target under qemu-user $manifest_qemu"
      fi
      manifest_tuple | sed 's/^/training /'
    } > "$dir/arm.training.new" &&
    [ "$(manifest_get "$dir/arm.training.new" "training profile")" = "$(manifest_sha256 "$dir/arm.profdata.new")" ] &&
    mv "$dir/arm.profdata.new" "$dir/arm.profdata" && mv "$dir/arm.training.new" "$dir/arm.training" || return 1
  echo "cross-ship: the aarch64 trainer's profile in ${dir#$PWD/}/arm.profdata, its training in ${dir#$PWD/}/arm.training ($(manifest_get "$dir/arm.training" "training trainer"))"
}

# windows: the ship profile for x86_64 Windows, plain: no profile guides it (PGO for Windows is a
# later part, docs/SPEED.md, "The ship from Linux"). The pinned zig links it as it links Darwin's,
# against the MinGW-w64 startup objects and import libraries it ships and the UCRT, the C runtime
# of Windows 10 and later, with LLVM's libunwind (bench/zig.sh's wrapper); no profile, so no
# metadata of the native build. Windows is not where it is built, so in the place of the native
# suite the smoke under wine (tests/wine.sh) runs on its bytes, its line in the manifest bound to
# their digest: out/cross/teq-windows-x86_64.exe, its manifest and .version.
windows() {
  local build=$dir/windows binary stamp status smoke
  std $windows_target && zig_ready && zig_wrapper $windows_target || return 1
  command -v wine > /dev/null || { echo "cross-ship: no wine, which runs the smoke; apt-get install wine64"; return 1; }
  export TEQ_CROSS_LOG=$dir/windows
  rm -f "$TEQ_CROSS_LOG.link"
  # The wrapper's text, which cargo does not track, stamps the directory: a build under another
  # starts from an empty one.
  stamp=$(sha256sum < "$zig_cc" | cut -c1-16)
  if [ "$(cat "$dir/windows.stamp" 2> /dev/null)" != "$stamp" ]; then
    rm -rf "$build" && echo "$stamp" > "$dir/windows.stamp" || return 1
  fi
  env -u RUSTFLAGS -u CARGO_ENCODED_RUSTFLAGS -u RUSTC_WRAPPER "CARGO_TARGET_$(echo $windows_target | tr a-z- A-Z_)_LINKER=$zig_cc" \
    timeout $bound cargo build -v --profile ship --target $windows_target --target-dir "$build" > "$TEQ_CROSS_LOG.log" 2>&1
  status=$?
  grep -E '^error|Finished' "$TEQ_CROSS_LOG.log"
  [ $status -eq 0 ] || return $status
  binary=$build/$windows_target/ship/teq.exe
  check_windows "$binary" || return 1
  manifest_wine="$(wine --version)"
  # The smoke's exit and its last line, `wine: passed: <n> of <n>`, both say it passed whole.
  TEQ_EXE="$binary" timeout 1200 tests/wine.sh > "$dir/windows.smoke" 2>&1
  status=$?
  smoke=$(tail -1 "$dir/windows.smoke")
  if [ $status -ne 0 ] || ! [[ $smoke =~ ^wine:\ passed:\ ([0-9]+)\ of\ ([0-9]+)$ ]] || [ "${BASH_REMATCH[1]}" != "${BASH_REMATCH[2]}" ]; then
    tail -20 "$dir/windows.smoke"
    echo "cross-ship: tests/wine.sh failed under $manifest_wine (exit $status, $dir/windows.smoke): $smoke"
    return 1
  fi
  echo "cross-ship: tests/wine.sh under $manifest_wine: ${smoke#wine: }"
  local name=teq-windows-x86_64.exe sha
  mkdir -p "$out" && rm -f "$out/$name" "$out/$name.manifest" "$out/$name.version" && cp "$binary" "$out/$name" || return 1
  sums
  grep " $name\$" "$out/SHA256SUMS"
  sha=$(manifest_sha256 "$out/$name")
  manifest_write "$out/$name" $windows_target "" - "smoke $sha tests/wine.sh ${smoke#wine: } under $manifest_wine" "wrappers $stamp" || return 1
  grep -v '^binary \|^version \|^smoke ' "$out/$name.manifest" > "$out/inputs.txt"
  echo "cross-ship: $out/$name: $(cat "$out/$name.version"), manifest ${out#$PWD/}/$name.manifest"
}

# check_windows <exe>: what the exe claims, read without running it: a plain build's stamp, `teq
# --version`'s revision alone; a PE32+ image for AMD64, executable and not a DLL (which may carry
# the executable flag too), of the console subsystem; and
# DLLs of the system alone: Windows' own and the api-ms-win-* sets, the UCRT's among them, and none
# of MinGW's runtime (libgcc_s_seh-1, libwinpthread-1, libstdc++-6) or Visual C++'s (vcruntime140),
# which a machine may not have.
check_windows() {
  local binary=$1 imports dll
  stamped "$binary" || return 1
  "$tools/llvm-readobj" --file-headers "$binary" > "$binary.headers" || return 1
  grep -q 'Machine: IMAGE_FILE_MACHINE_AMD64 ' "$binary.headers" && grep -q '^  Magic: 0x20B$' "$binary.headers" && grep -q 'IMAGE_FILE_EXECUTABLE_IMAGE ' "$binary.headers" &&
    ! grep -q 'IMAGE_FILE_DLL ' "$binary.headers" && grep -q 'Subsystem: IMAGE_SUBSYSTEM_WINDOWS_CUI ' "$binary.headers" || { echo "cross-ship: $binary is not a PE32+ console executable for AMD64 ($binary.headers)"; return 1; }
  imports=$("$tools/llvm-objdump" -p "$binary" | sed -n 's/^ *DLL Name: //p' | sort -f)
  [ -n "$imports" ] || { echo "cross-ship: $binary imports no DLL"; return 1; }
  for dll in $imports; do
    case ${dll,,} in
      kernel32.dll | kernelbase.dll | ntdll.dll | ws2_32.dll | bcryptprimitives.dll | userenv.dll | dbghelp.dll | advapi32.dll | user32.dll | api-ms-win-*.dll) ;;
      *) echo "cross-ship: $binary imports $dll, which is not the system's"; return 1 ;;
    esac
  done
  echo "cross-ship: $binary: PE32+ AMD64 console executable, stack $(sed -n 's/^  SizeOfStackReserve: //p' "$binary.headers") bytes, imports $(echo $imports)"
}

[ "$stage" = linux-arm-suite ] || toolchain || exit 1
case $stage in
  plain) metadata && plain ;;
  windows) windows ;;
  use) use ;;
  intel) intel ;;
  linux-arm) linux_arm ;;
  linux-arm-build) linux_arm_build ;;
  linux-arm-suite) linux_arm_suite ;;
  repeat) metadata "$([ -n "$TEQ_CROSS_PROFILE" ] || echo "$dir/arm.training")" && repeat ;;
  keys) metadata "$([ -n "$TEQ_CROSS_PROFILE" ] || echo "$dir/arm.training")" && keys ;;
  arm) arm ;;
  ship)
    [ -z "$TEQ_CROSS_PROFILE" ] || { echo "cross-ship: without a stage the trainer's profile is built and used; TEQ_CROSS_PROFILE goes with use"; exit 2; }
    arm && use ;;
esac
