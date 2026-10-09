# bench/ship-manifest.sh, sourced from the root of the tree that built the binary: the manifest of a
# ship binary and its checks (docs/SPEED.md, "The ship from Linux"). bench/cross-ship.sh's use and
# plain and bench/pgo.sh ship's use write <binary>.manifest only after the build, its training and,
# for a binary this machine runs, its suite succeeded, and <binary>.version from it;
# integrations/sbt/binary/stage.sh and check.sh read a version only from a manifest whose
# digest is the binary's, since a Mach-O binary cannot be run where it is built. A manifest is lines
# of `<key> <value>`:
#   binary <sha256>            the file's
#   version <string>           what `teq --version` prints, derived as build.rs stamps it
#   commit <sha1>              the tree's HEAD, in full
#   target <triple>            the build's target; host the build machine's
#   flags <rustflags>          the effective RUSTFLAGS of the build
#   profile <sha256> <file>    the profile the build used; profile-original the one it was translated from
#   training <key> <value>     the training's record (bench/pgo.sh's training.txt, bench/cross-ship.sh's
#                              arm.training), `training none` without one: status, programs, runs, corpus, jars and
#                              profile (their digests); release <version> <commit>, the ship that trained it;
#                              metadata <hex>, teq's crate metadata in the instrumented build, under which the
#                              profile names teq's functions (a guided build is given the same); trainer <target>
#                              native, or <target> under qemu-user <version> <sha256> (the aarch64 trainer emulated
#                              on the x86-64 machine, its give-ways counted); host <triple>, the training machine;
#                              `training tuple <tool> <version...>`, that machine's toolchain as manifest_tuple
#                              writes it, the training's environment apart from the build's; source <tag>
#                              <sha256>, the GitHub release whose asset teq-<version>-profiles.tar the profile was
#                              taken from (bench/ship-profiles.sh), for a ship that trained none of its own
#   tuple <tool> <version...>  the build's toolchain (bench/ship-qualified.txt the qualified one); `tuple image
#                              <reference>` the release workflow's build environment, TEQ_SHIP_IMAGE
#                              (bench/actions/ship-image.txt), `image-aarch64` on an arm64 machine
#   glibc <version> needs <libraries>   a Linux binary's floor, the highest version its version-needs
#                              table names, and the libraries it loads
#   suite <sha256> tests/run.sh passed: <counts>   the suite on those bytes, for a binary this machine runs;
#                              for the Linux aarch64 binary `..., outputs <n> of <n> as the native suite's,` then
#                              `under qemu-user <version> sysroot <file> <sha256>` (emulated on the x86-64 machine,
#                              which records both in its tuple) or `natively in <image> over <file> <sha256>` (on an
#                              arm64 machine, in the floor's container with the floor's glibc package installed)
#   suite-tuple <tool> <value...>   the native suite's environment, apart from the build's: floor-aarch64 (the
#                              container's image by digest), sysroot-aarch64 (the floor's package), node-aarch64
#   smoke <sha256> tests/wine.sh passed: <n> of <n> under <wine>   the smoke on those bytes, for the Windows binary
#   run <id>                   the staging invocation, added by stage.sh to its copy

# The compiler's sources: what a binary is built from, whatever its commit, the JavaScript runtime
# src/emit embeds and the plugin version build.rs bakes in among them.
manifest_sources="src std runtime build.rs Cargo.toml Cargo.lock .cargo integrations/sbt/plugin-version.txt"
# The routes (a classifier's target and tools) and the glibc floor.
[ -n "${release_known:-}" ] || . "$(dirname "${BASH_SOURCE[0]}")/ship-release.sh"

# coreutils' SHA-256 where there is one, shasum on a Mac without it: the same `<digest>  <file>` lines.
if command -v sha256sum > /dev/null 2>&1; then manifest_sha256_tool="sha256sum"; else manifest_sha256_tool="shasum -a 256"; fi
manifest_sha256() { $manifest_sha256_tool < "$1" | cut -c1-64; }

# The string `teq --version` prints for a build under the given RUSTFLAGS, as build.rs's
# emit_git_hash and main.rs's print_version make it: git's short hash of HEAD at git's own
# abbreviation, TEQ_REVISION where git fails, " pgo" with -Cprofile-use among the flags (which
# says nothing of the profile's training), " instrumented" with -Cprofile-generate.
manifest_version() {
  local flags=$1 hash build= cargo
  hash=$(git rev-parse --short HEAD 2> /dev/null) && [ -n "$hash" ] || hash=${TEQ_REVISION:-}
  case " $flags " in
    *" -Cprofile-use"*) build=" pgo" ;;
    *" -Cprofile-generate"*) build=" instrumented" ;;
  esac
  cargo=$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' Cargo.toml | head -1)
  hash="$hash$build"
  hash=${hash# }
  if [ -n "$hash" ]; then echo "teq $cargo $hash"; else echo "teq $cargo"; fi
}

# The toolchain of a build: rustc, its LLVM and cargo as rust-toolchain.toml resolves them here,
# the build machine's release; zig for the cross build, qemu-user for the work emulated here, wine for the
# Windows one's smoke (bench/cross-ship.sh sets $manifest_zig, $manifest_qemu and $manifest_wine); the image,
# `image` on x86-64 and `image-aarch64` on arm64, each architecture's its own.
manifest_tuple() {
  local rustc cargo image=image
  rustc=$(rustc -vV) && cargo=$(cargo -vV) || return 1
  [ "$(sed -n 's/^host: //p' <<< "$rustc")" != aarch64-unknown-linux-gnu ] || image=image-aarch64
  echo "tuple rustc $(sed -n 's/^release: //p' <<< "$rustc") $(sed -n 's/^commit-hash: //p' <<< "$rustc")"
  echo "tuple llvm $(sed -n 's/^LLVM version: //p' <<< "$rustc")"
  echo "tuple cargo $(sed -n 's/^release: //p' <<< "$cargo") $(sed -n 's/^commit-hash: //p' <<< "$cargo")"
  if [ -f /etc/os-release ]; then echo "tuple os $(. /etc/os-release && echo "$NAME $VERSION_ID ${VERSION_CODENAME:-}")"; else echo "tuple os $(uname -sr)"; fi
  [ -z "${manifest_zig:-}" ] || echo "tuple zig $manifest_zig"
  [ -z "${manifest_qemu:-}" ] || echo "tuple qemu-user $manifest_qemu"
  [ -z "${manifest_wine:-}" ] || echo "tuple wine $manifest_wine"
  [ -z "${manifest_sysroot:-}" ] || echo "tuple sysroot-aarch64 $manifest_sysroot"
  [ -z "${TEQ_SHIP_IMAGE:-}" ] || echo "tuple $image $TEQ_SHIP_IMAGE"
}

# manifest_carries <binary> <version> <classifier>: whether the binary is built as the version
# says. One this machine runs prints it; any other holds build.rs's stamp, `<hash>` with ` pgo` or
# ` instrumented` after it, as a string (a plain build's hash with neither), the only identity a
# Mach-O binary gives here. The stamp shares its bytes with neighbouring strings (in the Darwin
# build it follows "Scala"), so the hash is found, not delimited.
manifest_carries() {
  local binary=$1 version=$2 stamp
  if [ "$3" = "$(manifest_host)" ]; then
    [ "$("$binary" --version 2> /dev/null)" = "$version" ]
    return
  fi
  stamp=${version#teq * }
  [ "$stamp" != "$version" ] && [ -n "$stamp" ] && grep -qaF "$stamp" "$binary" || return 1
  case $stamp in
    *" pgo" | *" instrumented") ;;
    *) ! grep -qaE "$stamp (pgo|instrumented)" "$binary" ;;
  esac
}

# manifest_write <binary> <target> <flags> <training file|-> [<line>...]: the manifest and the
# .version sidecar beside the binary, the lines given among them; refuses a version the binary
# does not carry (manifest_carries).
manifest_write() {
  local binary=$1 target=$2 flags=$3 training=$4 version
  shift 4
  rm -f "$binary.manifest" "$binary.version"
  version=$(manifest_version "$flags")
  manifest_carries "$binary" "$version" "$(manifest_classifier "$target")" || { echo "manifest: $binary is not built as '$version' says"; return 1; }
  {
    echo "binary $(manifest_sha256 "$binary")"
    echo "version $version"
    echo "commit $(git rev-parse HEAD)"
    echo "target $target"
    echo "host $(rustc -vV | sed -n 's/^host: //p')"
    echo "flags $flags"
    if [ "$training" = - ]; then echo "training none"; else cat "$training"; fi
    for line in "$@"; do echo "$line"; done
    manifest_tuple
  } > "$binary.manifest.new" || return 1
  mv "$binary.manifest.new" "$binary.manifest" && echo "$version" > "$binary.version"
}

manifest_get() { sed -n "s/^$2 //p" "$1" | head -1; }

# manifest_own_training <record or manifest> <commit>: whether the training it records is the ship's own at that
# commit, not profiles taken from an earlier release's asset (bench/ship-profiles.sh take): no `training source`
# line, and the commit trained, when the record names one, is <commit>. A release's version is no witness: a dry
# run on a master that still names the last release takes that release's profiles under the same version.
manifest_own_training() {
  local trained
  [ -z "$(manifest_get "$1" "training source")" ] || return 1
  trained=$(manifest_get "$1" "training release" | awk '{print $2}')
  [ -z "$trained" ] || [ "$trained" = "$2" ]
}

# The classifier of the build.teq:teq artifact a target's binary is published under (the routes of
# bench/ship-release.sh).
manifest_classifier() {
  local c
  for c in $release_known; do
    [ "$(release_route_target "$c")" = "$1" ] && { echo "$c"; return 0; }
  done
}
# manifest_glibc <binary>: the oldest glibc an ELF binary loads on: the highest version its version-needs
# table names (what the loader checks, a version need carrying symbols or not), every one of which must be
# GLIBC_<n>(.<n>)* (GLIBC_ABI_DT_RELR, GLIBC_PRIVATE and the like refuse), with no RELR relocations, which
# glibc reads from 2.36 on. Prints `<version> needs <libraries>`; fails naming what it refuses.
manifest_glibc() {
  local tools names foreign needed
  tools=$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin
  names=$("$tools/llvm-readobj" --version-info "$1" | awk '/^VersionRequirements/ {f=1} /^VersionDefinitions|^VersionSymbols/ {f=0} f' | sed -n 's/^ *Name: \([^ ]*\).*/\1/p' | sort -u) || return 1
  [ -n "$names" ] || { echo "$1 names no glibc version" >&2; return 1; }
  foreign=$(grep -v -E '^GLIBC_[0-9]+(\.[0-9]+)*$' <<< "$names" | tr '\n' ' ')
  [ -z "$foreign" ] || { echo "$1 needs $foreign, which is no glibc version" >&2; return 1; }
  ! "$tools/llvm-readobj" --dynamic-table "$1" | grep -q RELR || { echo "$1 carries RELR relocations, which glibc reads from 2.36 on" >&2; return 1; }
  needed=$("$tools/llvm-readobj" --needed-libs "$1" | sed -n 's/^  *\(lib[^ ]*\.so[^ ]*\).*/\1/p' | tr '\n' ' ')
  echo "$(sed 's/^GLIBC_//' <<< "$names" | sort -V | tail -1) needs ${needed% }"
}
# This machine's classifier, the binaries it runs.
manifest_host() {
  local os arch
  case "$(uname -s)" in Darwin) os=osx ;; Linux) os=linux ;; *) return 1 ;; esac
  case "$(uname -m)" in arm64 | aarch64) arch=aarch_64 ;; x86_64 | amd64) arch=x86_64 ;; *) return 1 ;; esac
  echo "$os-$arch"
}

# The classifier a binary's header says it runs on: a 64-bit Mach-O or ELF file and its CPU; or a
# PE image (`MZ`, its PE header at the offset in bytes 60 to 63) of AMD64, PE32+, executable and no
# DLL (a DLL may carry the executable flag too).
manifest_header() {
  local bytes at pe flags
  bytes=$(od -An -tx1 -N20 "$1" | tr -d ' \n')
  case $bytes in
    cffaedfe0c000001*) echo osx-aarch_64 ;;
    cffaedfe07000001*) echo osx-x86_64 ;;
    7f454c4602*) case ${bytes:36:4} in 3e00) echo linux-x86_64 ;; b700) echo linux-aarch_64 ;; esac ;;
    4d5a*)
      at=$(od -An -tu4 -j60 -N4 "$1" | tr -d ' ')
      [[ $at =~ ^[0-9]+$ ]] || return 0
      # `PE\0\0`, the machine, then past the COFF header's counts the characteristics and the
      # optional header's magic.
      pe=$(od -An -tx1 -j"$at" -N26 "$1" | tr -d ' \n')
      [ ${#pe} -eq 52 ] && [ "${pe:0:12}" = 504500006486 ] && [ "${pe:48:4}" = 0b02 ] || return 0
      flags=$((16#${pe:46:2}${pe:44:2}))
      ((flags & 0x0002 && !(flags & 0x2000))) && echo windows-x86_64
      ;;
  esac
  return 0
}

# manifest_check <binary> <manifest> <classifier> <head>: whether the manifest is the binary's and
# the binary may be given off as that classifier from a checkout at <head>: its digest, its
# header, the manifest's target, the version the binary carries (manifest_carries), the version
# naming the commit, the commit's compiler sources the head's, a guided build of a successful
# training (TEQ_STAGE_PLAIN=1 lets a plain build through; the Windows binary is plain, PGO for it
# being a later part), for a Linux binary its suite passed on these bytes and for the Windows one
# its smoke under wine passed whole on them. Prints the version.
manifest_check() {
  local binary=$1 manifest=$2 classifier=$3 head=$4 sha version commit target hash
  [ -f "$binary" ] || { echo "no binary at $binary" >&2; return 1; }
  [ -f "$manifest" ] || { echo "$binary has no manifest ($manifest): bench/ship.sh writes one with the build" >&2; return 1; }
  local key
  for key in binary version commit target host flags run smoke suite glibc; do
    [ "$(grep -c "^$key " "$manifest")" -le 1 ] || { echo "$manifest has more than one $key line" >&2; return 1; }
  done
  sha=$(manifest_sha256 "$binary")
  [ "$(manifest_get "$manifest" binary)" = "$sha" ] || { echo "$binary is not the binary of $manifest (sha256 $sha, the manifest's $(manifest_get "$manifest" binary))" >&2; return 1; }
  [ "$(manifest_header "$binary")" = "$classifier" ] || { echo "$binary's header is $(manifest_header "$binary"), not $classifier" >&2; return 1; }
  target=$(manifest_get "$manifest" target)
  [ "$(manifest_classifier "$target")" = "$classifier" ] || { echo "$binary is built for ${target:-no target}, not $classifier" >&2; return 1; }
  version=$(manifest_get "$manifest" version)
  # The binary's own identity before the manifest's commit is believed.
  manifest_carries "$binary" "$version" "$classifier" || { echo "$binary is not built as its manifest's version '$version' says" >&2; return 1; }
  commit=$(manifest_get "$manifest" commit)
  [[ $commit =~ ^[0-9a-f]{40}$ ]] || { echo "$manifest names no commit" >&2; return 1; }
  hash=$(awk '{print $3}' <<< "$version")
  [ -n "$hash" ] && [[ $commit == "$hash"* ]] || { echo "$binary's version '$version' does not name its commit ${commit:0:12}" >&2; return 1; }
  if [ "$commit" != "$head" ]; then
    # The sources by their paths from the top of the checkout, wherever the caller stands.
    git cat-file -e "$commit^{commit}" 2> /dev/null && git -C "$(git rev-parse --show-toplevel)" diff --quiet "$commit" "$head" -- $manifest_sources ||
      { echo "$binary is built from ${commit:0:12}, whose compiler sources are not the head's (${head:0:12})" >&2; return 1; }
  fi
  case $version in
    *" pgo")
      [ "$(manifest_get "$manifest" training)" = "status ok" ] || { echo "$binary is guided by a profile with no successful training recorded ($manifest)" >&2; return 1; } ;;
    *) [ -n "${TEQ_STAGE_PLAIN:-}" ] || [[ $classifier == windows-* ]] || { echo "$binary is not built with its profile ($version): bench/ship.sh first" >&2; return 1; } ;;
  esac
  case $classifier in
    linux-*)
      # The floor: what the binary's own tables say, against the release's and the manifest's; and the
      # suite on these bytes, native for x86-64, under qemu-user over the floor's sysroot for aarch64 with
      # its outputs as the native suite's.
      local glibc suite
      glibc=$(manifest_glibc "$binary") || return 1
      [ "$(printf '%s\n%s\n' "${glibc%% *}" "$release_glibc" | sort -V | tail -1)" = "$release_glibc" ] || { echo "$binary loads on glibc ${glibc%% *} and later, past the floor $release_glibc" >&2; return 1; }
      [ "$(manifest_get "$manifest" glibc)" = "$glibc" ] || { echo "$binary needs glibc $glibc, its manifest says '$(manifest_get "$manifest" glibc)'" >&2; return 1; }
      # The suite's counts as tests/run.sh prints them, some passed and none failed, the skips counted.
      suite=$(manifest_get "$manifest" suite)
      local counts='([0-9]+) passed, 0 failed, ([0-9]+) of them skipped for missing jars' same='outputs ([0-9]+) of ([0-9]+) as the native suite'\''s'
      if [ "$classifier" = linux-aarch_64 ]; then
        # Emulated, under the qemu-user and over the sysroot the build's toolchain records; or natively, in the
        # floor's container and over its glibc package as the suite's environment records them.
        local how ran over
        if [[ $suite =~ ^$sha\ tests/run\.sh\ passed:\ $counts,\ $same,\ under\ qemu-user\ (.+)\ sysroot\ (.+)$ ]]; then
          how=tuple ran=qemu-user over=$(manifest_get "$manifest" "tuple qemu-user")
        elif [[ $suite =~ ^$sha\ tests/run\.sh\ passed:\ $counts,\ $same,\ natively\ in\ (.+)\ over\ (.+)$ ]]; then
          how=suite-tuple ran=floor-aarch64 over=$(manifest_get "$manifest" "suite-tuple floor-aarch64")
        else
          echo "$binary has no passed tests/run.sh on aarch64, its outputs as the native suite's, recorded against its digest" >&2; return 1
        fi
        [ "${BASH_REMATCH[1]}" -gt 0 ] && [ "${BASH_REMATCH[3]}" = "${BASH_REMATCH[4]}" ] && [ "${BASH_REMATCH[3]}" -gt 0 ] ||
          { echo "$binary's suite on aarch64 is not whole, its outputs not all the native suite's ($suite)" >&2; return 1; }
        [ "${BASH_REMATCH[5]}" = "$over" ] || { echo "$binary's suite ran in '${BASH_REMATCH[5]}', not the $ran its $how records ('$over')" >&2; return 1; }
        [ "${BASH_REMATCH[6]}" = "$(manifest_get "$manifest" "$how sysroot-aarch64")" ] || { echo "$binary's suite ran over the sysroot '${BASH_REMATCH[6]}', not the one its $how records ('$(manifest_get "$manifest" "$how sysroot-aarch64")')" >&2; return 1; }
      else
        [[ $suite =~ ^$sha\ tests/run\.sh\ passed:\ $counts$ ]] && [ "${BASH_REMATCH[1]}" -gt 0 ] || { echo "$binary has no passed tests/run.sh recorded against its digest" >&2; return 1; }
      fi
      # A build guided by a trainer under qemu-user names the trainer's, which its training's environment
      # records (a record of before the environment's, the build's own toolchain).
      local trainer env
      trainer=$(manifest_get "$manifest" "training trainer" | sed -n 's/^.* under qemu-user //p')
      env="training tuple"
      grep -q '^training tuple ' "$manifest" || env=tuple
      [ -z "$trainer" ] || [ "$trainer" = "$(manifest_get "$manifest" "$env qemu-user")" ] || { echo "$binary's profile was trained under qemu-user '$trainer', not its training's '$(manifest_get "$manifest" "$env qemu-user")'" >&2; return 1; } ;;
    windows-*)
      # A whole smoke of at least one check on these bytes, under the wine the toolchain records.
      local smoke wine
      smoke=$(manifest_get "$manifest" smoke)
      [[ $smoke =~ ^$sha\ tests/wine\.sh\ passed:\ ([0-9]+)\ of\ ([0-9]+)\ under\ (.+)$ ]] && [ "${BASH_REMATCH[1]}" = "${BASH_REMATCH[2]}" ] && [ "${BASH_REMATCH[1]}" -gt 0 ] ||
        { echo "$binary has no whole tests/wine.sh pass recorded against its digest" >&2; return 1; }
      wine=$(manifest_get "$manifest" "tuple wine")
      [ -n "$wine" ] && [ "${BASH_REMATCH[3]}" = "$wine" ] || { echo "$binary's smoke ran under '${BASH_REMATCH[3]}', not its toolchain's wine '$wine'" >&2; return 1; } ;;
  esac
  echo "$version"
}

# manifest_base_qualified <manifest> <qualified file>: whether the tools every binary records (rustc, LLVM,
# cargo, the build machine's release) are the qualified file's: a machine's toolchain before a ship.
manifest_base_qualified() {
  local manifest=$1 qualified=$2 key want got
  [ -r "$qualified" ] || { echo "no $qualified, the qualified toolchain" >&2; return 1; }
  ! grep -q '^pending' "$qualified" || { echo "$qualified is not settled: $(sed -n 's/^pending //p' "$qualified")" >&2; return 1; }
  for key in rustc llvm cargo os; do
    [ "$(grep -c "^$key " "$qualified")" -eq 1 ] || { echo "$qualified does not name $key once" >&2; return 1; }
    want=$(sed -n "s/^$key //p" "$qualified")
    got=$(manifest_get "$manifest" "tuple $key")
    [ -n "$want" ] && [ "$got" = "$want" ] || { echo "$manifest: $key is '$got', the qualified '$want' ($qualified)" >&2; return 1; }
  done
}
# manifest_qualified <manifest> <qualified file> [<classifier>]: whether the qualification file
# (bench/ship-qualified.txt) publishes the binary the manifest describes, as release_qualified_classifiers
# reads it for the set: a settled file; the manifest recording every tool of the route (rustc, LLVM, cargo and
# the build machine's release for every binary, and the route's own: zig for every binary a ship gives off,
# the floor's glibc package for the Linux aarch64 suite, wine for the Windows smoke; qemu-user for the arm64
# builds when this release's trainer or the Linux aarch64 suite ran emulated, release_route_emulated); every tool the build's toolchain and the
# native suite's environment record (`tuple`, `suite-tuple`) named once in the file with the same value; the
# training's compiler (rustc, LLVM, cargo) the qualified one, and the rest of its environment too when the
# release trained the profile itself (a profile taken from an earlier release keeps the environment of its
# training, qualified then, its trainer's qemu-user among it); a `route <classifier>` line; and the Windows smoke,
# this release's trainer and the aarch64 suite run under the qualified tools. Once the file names an `image`, a binary
# built outside it (no `tuple image`) is not published: releases are built by the release workflow.
manifest_qualified() {
  local manifest=$1 qualified=$2 classifier=${3:-} key want got own
  [ -r "$qualified" ] || { echo "no $qualified, the qualified toolchain" >&2; return 1; }
  ! grep -q '^pending' "$qualified" || { echo "$qualified is not settled: $(sed -n 's/^pending //p' "$qualified")" >&2; return 1; }
  [ -n "$classifier" ] || classifier=$(manifest_classifier "$(manifest_get "$manifest" target)")
  [ -n "$classifier" ] && release_route_tools "$classifier" > /dev/null || { echo "$manifest is built for no route's target ($(manifest_get "$manifest" target))" >&2; return 1; }
  for key in rustc llvm cargo os $(release_route_tools "$classifier"); do
    [ -n "$(manifest_get "$manifest" "tuple $key")$(manifest_get "$manifest" "suite-tuple $key")" ] || { echo "$manifest records no $key, a tool of the $classifier route" >&2; return 1; }
  done
  # The training this ship made itself at the binary's commit, or profiles taken from an earlier release's.
  own=
  ! manifest_own_training "$manifest" "$(manifest_get "$manifest" commit)" || own=1
  # The emulated route's tools, when this release's work ran emulated: its own trainer or its suite under qemu-user.
  if { [ -n "$own" ] && manifest_get "$manifest" "training trainer" | grep -q ' under qemu-user '; } ||
    grep -q '^suite [0-9a-f]* tests/run.sh passed: .* under qemu-user ' "$manifest"; then
    for key in $(release_route_emulated "$classifier"); do
      [ -n "$(manifest_get "$manifest" "tuple $key")$(manifest_get "$manifest" "training tuple $key")" ] || { echo "$manifest records no $key, which its emulated work ran under" >&2; return 1; }
    done
  fi
  while read -r key got; do
    [ "$(grep -c "^$key " "$qualified")" -eq 1 ] || { echo "$qualified does not name $key once" >&2; return 1; }
    want=$(sed -n "s/^$key //p" "$qualified")
    [ -n "$want" ] && [ "$got" = "$want" ] || { echo "$manifest: $key is '$got', the qualified '$want' ($qualified)" >&2; return 1; }
  done < <(sed -n 's/^tuple //p; s/^suite-tuple //p' "$manifest"; sed -n 's/^training tuple //p' "$manifest" | if [ -n "$own" ]; then cat; else grep -E '^(rustc|llvm|cargo) '; fi)
  ! grep -q '^image ' "$qualified" || [ -n "$(manifest_get "$manifest" "tuple image")" ] ||
    { echo "$manifest records no image, and $qualified names the release workflow's (bench/actions/ship-image.txt): a release is built there" >&2; return 1; }
  [ "$(grep -c "^route $classifier\$" "$qualified")" -eq 1 ] || { echo "$qualified does not publish the $classifier route yet (no 'route $classifier' line; docs/SPEED.md, \"The ship from Linux\")" >&2; return 1; }
  # A profile this release trained under qemu-user was trained under the qualified one, which the file names once
  # (an earlier release's keeps its trainer's, as the rest of its environment: manifest_check holds its record to it).
  got=$(manifest_get "$manifest" "training trainer" | sed -n 's/^.* under qemu-user //p')
  want=$(sed -n 's/^qemu-user //p' "$qualified")
  [ -z "$got" ] || [ -z "$own" ] || { [ "$(grep -c '^qemu-user ' "$qualified")" -eq 1 ] && [ "$got" = "$want" ]; } || { echo "$manifest: its profile was trained under qemu-user '$got', the qualified '$want' ($qualified)" >&2; return 1; }
  case $classifier in
    windows-*)
      got=$(sed -n 's/^smoke [0-9a-f]* tests\/wine.sh passed: .* under //p' "$manifest")
      want=$(sed -n 's/^wine //p' "$qualified")
      [ -n "$got" ] && [ "$got" = "$want" ] || { echo "$manifest: its smoke ran under '$got', the qualified wine '$want' ($qualified)" >&2; return 1; } ;;
    linux-aarch_64)
      if grep -q '^suite [0-9a-f]* tests/run.sh passed: .* natively in ' "$manifest"; then
        got=$(sed -n 's/^suite [0-9a-f]* tests\/run.sh passed: .* natively in //p' "$manifest")
        want="$(sed -n 's/^floor-aarch64 //p' "$qualified") over $(sed -n 's/^sysroot-aarch64 //p' "$qualified")"
      else
        got=$(sed -n 's/^suite [0-9a-f]* tests\/run.sh passed: .* under qemu-user //p' "$manifest")
        want="$(sed -n 's/^qemu-user //p' "$qualified") sysroot $(sed -n 's/^sysroot-aarch64 //p' "$qualified")"
      fi
      [ -n "$got" ] && [ "$got" = "$want" ] || { echo "$manifest: its suite ran under '$got', the qualified '$want' ($qualified)" >&2; return 1; } ;;
  esac
}

# manifest_attest <binary> <suite record>: the Linux aarch64 binary's manifest given the native suite's record
# (bench/cross-ship.sh linux-arm-suite, run on an arm64 machine after the build on the x86-64 one): the record's
# `suite` line names the binary's digest, which the manifest names too, and the manifest has no suite of its
# own; its lines go into the manifest, which the stage then checks whole (manifest_check).
manifest_attest() {
  local binary=$1 record=$2 sha
  sha=$(manifest_sha256 "$binary")
  [ "$(manifest_get "$binary.manifest" binary)" = "$sha" ] || { echo "$binary is not the binary of its manifest (sha256 $sha)" >&2; return 1; }
  ! grep -q '^suite ' "$binary.manifest" || { echo "$binary.manifest has a suite of its own, and $record another" >&2; return 1; }
  [ "$(grep -c '^suite ' "$record")" -eq 1 ] && [ "$(sed -n 's/^suite \([0-9a-f]*\) .*/\1/p' "$record")" = "$sha" ] ||
    { echo "$record is not one suite of $binary's bytes (sha256 $sha)" >&2; return 1; }
  [ -z "$(grep -v -E '^(suite|suite-tuple) ' "$record")" ] || { echo "$record holds other lines than the suite's" >&2; return 1; }
  cat "$binary.manifest" "$record" > "$binary.manifest.new" && mv "$binary.manifest.new" "$binary.manifest"
}
