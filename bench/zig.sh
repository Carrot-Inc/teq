# bench/zig.sh, sourced from the root of the tree by bench/cross-ship.sh and bench/pgo.sh ship: the pinned
# zig, fetched by its digest into target/cross/ (zig_fetch, which sets $zig and $manifest_zig), and the
# linker wrappers the ship's builds are linked through, one per target (zig_wrapper <triple>, which writes
# target/cross/bin/cc-<triple> and sets $zig_cc): a script that takes rustc's arguments for the target's
# native linker and passes them to zig's cc for the target. zig links against what it ships: the libSystem
# stub for Darwin (the macOS SDK is not available on Linux), its glibc stubs of the release's floor for
# Linux (docs/TARGETS.md, "Releases": $release_glibc, so the binary loads on any glibc from that one on,
# wherever it is built), MinGW-w64 on the UCRT for Windows; nothing else is linked natively, teq having no
# dependencies. Each wrapper joins a `-u <symbol>` of rustc's (the profiler runtime's, under
# -Cprofile-generate) into one argument, which is how zig's cc reads it, and appends the command it runs to
# $TEQ_CROSS_LOG.link when the caller names one.
# The wrappers' text is not tracked by cargo: a build stamps its directory with it (bench/cross-ship.sh's
# build_for, bench/pgo.sh ship) and starts from an empty one under another stamp. zig's cache, where it keeps
# what it builds for a target (the C runtime's objects, libunwind, the libc stubs), is TEQ_ZIG_CACHE when the
# caller names one that outlives the tree (the release workflow's restored cache), else target/cross/zig-cache.
[ -n "${release_glibc:-}" ] || . "$(dirname "${BASH_SOURCE[0]}")/ship-release.sh"
zig_version=0.16.0
# Each host's archive by its digest: x86-64's links every binary the ship gives off; arm64's is the arm64
# image's (bench/actions/toolchain.sh), for a link compared there, never a release's (docs/DEVELOPING.md).
zig_sha256_x86_64=70e49664a74374b48b51e6f3fdfbf437f6395d42509050588bd49abe52ba3d00
zig_sha256_aarch64=ea4b09bfb22ec6f6c6ceac57ab63efb6b46e17ab08d21f69f3a48b38e1534f17
zig_dir=$PWD/target/cross
zig_cache=${TEQ_ZIG_CACHE:-$zig_dir/zig-cache}
zig_objcopy=$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/llvm-objcopy
# The oldest macOS the Darwin binaries claim (LC_BUILD_VERSION's minos): rustc's default for arm64 and
# above its default for x86_64 (10.12), written out so that the compiler's target triple and the linker's
# agree whatever the caller's environment says.
export MACOSX_DEPLOYMENT_TARGET=11.0

# zig_fetch: the pinned zig of this host under target/cross/, fetched when missing (a tarball there already, a
# cache's, is taken when its digest is the pinned one); $zig its path, $manifest_zig its manifest line.
zig_fetch() {
  local host arch sha256
  host=$(rustc -vV | sed -n 's/^host: //p')
  case $host in
    x86_64-unknown-linux-gnu) arch=x86_64 sha256=$zig_sha256_x86_64 ;;
    aarch64-unknown-linux-gnu) arch=aarch64 sha256=$zig_sha256_aarch64 ;;
    *) echo "zig: the digest is pinned for x86_64 and aarch64 Linux, not $host"; return 1 ;;
  esac
  zig=$zig_dir/zig-$arch-linux-$zig_version/zig
  if [ ! -x "$zig" ]; then
    local tarball=$zig_dir/zig-$arch-linux-$zig_version.tar.xz
    mkdir -p "$zig_dir" || return 1
    echo "$sha256  $tarball" | sha256sum -c --quiet > /dev/null 2>&1 ||
      timeout 300 curl -sSfLo "$tarball" "https://ziglang.org/download/$zig_version/zig-$arch-linux-$zig_version.tar.xz" || return 1
    echo "$sha256  $tarball" | sha256sum -c --quiet || { echo "zig: the digest of $tarball differs"; return 1; }
    timeout 120 tar -xf "$tarball" -C "$zig_dir" || return 1
  fi
  [ "$("$zig" version)" = $zig_version ] || { echo "zig: $zig is not zig $zig_version"; return 1; }
  manifest_zig="$zig_version $sha256"
}

# zig_target <triple>: zig's name of the target rustc's triple links for, the OS version or the glibc the
# binary claims included.
zig_target() {
  case $1 in
    aarch64-apple-darwin) echo "aarch64-macos.$MACOSX_DEPLOYMENT_TARGET-none" ;;
    x86_64-apple-darwin) echo "x86_64-macos.$MACOSX_DEPLOYMENT_TARGET-none" ;;
    x86_64-unknown-linux-gnu) echo "x86_64-linux-gnu.$release_glibc" ;;
    aarch64-unknown-linux-gnu) echo "aarch64-linux-gnu.$release_glibc" ;;
    # Windows 10, the oldest Rust's std for the target runs on.
    x86_64-pc-windows-gnullvm) echo x86_64-windows.win10-gnu ;;
    *) return 1 ;;
  esac
}

# zig_wrapper <triple>: the linker wrapper for the target, written whole and put in place; $zig_cc its path.
zig_wrapper() {
  local triple=$1 target file=$zig_dir/bin/cc-$1
  target=$(zig_target "$triple") || { echo "zig: no target of zig's for $triple"; return 1; }
  mkdir -p "$zig_dir/bin" || return 1
  case $triple in
    *-apple-darwin)
      # rustc's arguments for Apple's cc, but the two zig takes from its target. zig gives the main thread a
      # 16 MiB stack in LC_MAIN where Apple's ld writes 0, the system's default: -stack_size 0 keeps the
      # native value. -S links no debug stabs, which rustc strips after the link but whose object paths
      # zig's LC_UUID is computed over: with them the same build in another directory differs.
      cat > "$file.new" <<EOF
#!/bin/bash
# Written by bench/zig.sh: rustc's linker for $triple.
args=()
skip= undef=
for a in "\$@"; do
  if [ -n "\$skip" ]; then skip=; continue; fi
  if [ -n "\$undef" ]; then undef=; args+=("-u\$a"); continue; fi
  case \$a in
    -arch) skip=1 ;;
    -u) undef=1 ;;
    -mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET | -mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET.0) ;;
    -mmacosx-version-min=*) echo "zig: rustc links for \${a#*=}, the linker for $MACOSX_DEPLOYMENT_TARGET" >&2; exit 1 ;;
    -Wl,-dead_strip) [ -n "\$TEQ_CROSS_KEEP_DEAD" ] || args+=("\$a") ;;
    *) args+=("\$a") ;;
  esac
done
export ZIG_GLOBAL_CACHE_DIR=$zig_cache ZIG_LOCAL_CACHE_DIR=$zig_cache
set -- "$zig" cc -target $target "\${args[@]}" -Wl,-stack_size,0 -Wl,-S
[ -z "\$TEQ_CROSS_LOG" ] || echo "\$*" >> "\$TEQ_CROSS_LOG.link"
exec "\$@"
EOF
      ;;
    *-unknown-linux-gnu)
      # rustc's arguments for the system's cc, but three: the unwinder std's objects call is zig's libunwind,
      # static, in the place of libgcc_s, so the binary needs no libgcc_s.so; --strip-debug, under which zig
      # strips the symbol table too, is taken off the link and done after it by llvm-objcopy, which drops the
      # std's line tables and keeps the symbols as the native link does; and the Cortex-A53 erratum
      # 843419 workaround rustc asks the linker for on aarch64 is dropped, since zig refuses the linker
      # argument and its own -mfix-cortex-a53-843419 is unsupported for the target: the binary is not for
      # Cortex-A53 r0p0 to r0p4 cores (docs/TARGETS.md, "Releases").
      cat > "$file.new" <<EOF
#!/bin/bash
# Written by bench/zig.sh: rustc's linker for $triple, against glibc $release_glibc.
args=()
undef= output= prev=
for a in "\$@"; do
  if [ -n "\$undef" ]; then undef=; args+=("-u\$a"); continue; fi
  [ "\$prev" != -o ] || output=\$a
  prev=\$a
  case \$a in
    -u) undef=1 ;;
    -lgcc_s) args+=(-lunwind) ;;
    -Wl,--strip-debug) strip=1 ;;
    -Wl,--fix-cortex-a53-843419) ;;
    *) args+=("\$a") ;;
  esac
done
export ZIG_GLOBAL_CACHE_DIR=$zig_cache ZIG_LOCAL_CACHE_DIR=$zig_cache
set -- "$zig" cc -target $target "\${args[@]}"
[ -z "\$TEQ_CROSS_LOG" ] || echo "\$*" >> "\$TEQ_CROSS_LOG.link"
"\$@" || exit
[ -z "\$strip" ] || [ -z "\$output" ] || exec "$zig_objcopy" --strip-debug "\$output"
EOF
      ;;
    *-pc-windows-gnullvm)
      # rustc's arguments for llvm-mingw's clang but two, which zig takes, building LLVM's libunwind
      # (-lunwind) and the MinGW-w64 objects for the target: zig's MinGW-w64 is built on the UCRT and has no
      # msvcrt to link besides it, and zig ignores -O1, with a warning rustc passes on. /Brepro stamps the
      # header with a hash of the image, not the time of the link, so that the same build links to the
      # same bytes.
      cat > "$file.new" <<EOF
#!/bin/bash
# Written by bench/zig.sh: rustc's linker for $triple.
args=()
undef=
for a in "\$@"; do
  if [ -n "\$undef" ]; then undef=; args+=("-u\$a"); continue; fi
  case \$a in
    -u) undef=1 ;;
    -lmsvcrt | -Wl,-O1) ;;
    *) args+=("\$a") ;;
  esac
done
export ZIG_GLOBAL_CACHE_DIR=$zig_cache ZIG_LOCAL_CACHE_DIR=$zig_cache
set -- "$zig" cc -target $target "\${args[@]}" -Wl,/Brepro
[ -z "\$TEQ_CROSS_LOG" ] || echo "\$*" >> "\$TEQ_CROSS_LOG.link"
exec "\$@"
EOF
      ;;
    *) echo "zig: no wrapper for $triple"; return 1 ;;
  esac
  chmod +x "$file.new" && mv "$file.new" "$file" && zig_cc=$file
}
