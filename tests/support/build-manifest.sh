# Sourced by tests/gate-shard.sh and tests/gate-compare.sh, from a tree's root: the one way the timing lines build a
# teq binary and the identity they compare builds by. The configuration is fixed: `cargo build --release` into the
# tree's own target directory, whatever CARGO_TARGET_DIR says, for the host, with no flags from the environment; an
# environment that would change what cargo builds is refused rather than recorded. A manifest names what is left
# to differ between two machines, the toolchain and the cargo configuration from outside the tree (the tree's own
# .cargo/config.toml is part of its sources, so a branch that changes it is compared as it builds).
#   build_teq <tree> <revision>        builds <tree>/target/release/teq stamped with <revision> and checks that the
#                                      executable names it and is a plain build; the build's output on stdout
#   manifest <tree> <revision> <source hash>  prints the manifest of that build: with the toolchain, the binary's
#                                      digest and what it needs of a machine to run (its libraries, the newest
#                                      glibc symbol it links)
#   same_build <manifest> <manifest>   whether two manifests agree on everything but the revision and the source
#   received <dir> <revision> <source hash> <digest>  whether <dir>/teq, shipped from the build <dir>/manifest
#                                      describes, is that build of <revision> with that digest and runs here as a
#                                      plain build naming <revision>; why not on stdout
#   toolchain_key <tree>               the toolchain and configuration part of an artifact's key under ~/teq-bin
build_env_problem() {
  env | grep -E '^(RUSTFLAGS|RUSTDOCFLAGS|RUSTC|RUSTC_WRAPPER|RUSTC_WORKSPACE_WRAPPER|CARGO_ENCODED_RUSTFLAGS|CARGO_INCREMENTAL|CARGO_BUILD_[A-Z0-9_]*|CARGO_PROFILE_[A-Z0-9_]*|CARGO_TARGET_[A-Z0-9_]*|CARGO_UNSTABLE_[A-Z0-9_]*)=' |
    cut -d= -f1 | grep -v -x CARGO_TARGET_DIR | tr '\n' ' '
}
# cargo_config <tree>: the cargo configuration files cargo reads for the tree from outside it, each with its contents.
cargo_config() {
  local d
  d=$(cd "$1" && pwd)
  d=$(dirname "$d")
  while :; do
    for f in "$d/.cargo/config.toml" "$d/.cargo/config"; do [ -f "$f" ] && { echo "== $f"; cat "$f"; }; done
    [ "$d" = / ] && break
    d=$(dirname "$d")
  done
  for f in "${CARGO_HOME:-$HOME/.cargo}/config.toml" "${CARGO_HOME:-$HOME/.cargo}/config"; do [ -f "$f" ] && { echo "== $f"; cat "$f"; }; done
}
config_hash() {
  local c
  c=$(cargo_config "$1")
  [ -n "$c" ] && echo "$c" | sha256sum | cut -c1-12 || echo none
}
build_teq() {
  local tree=$1 rev=$2 problem v
  problem=$(build_env_problem)
  [ -z "$problem" ] || { echo "refused: the environment sets $problem, which would change the build"; return 1; }
  (cd "$tree" && TEQ_REVISION=$rev timeout 1500 cargo build --release --target-dir "$PWD/target" 2>&1) || return 1
  v=$("$tree/target/release/teq" --version 2> /dev/null)
  [ "$(echo "$v" | awk '{print $3}')" = "$rev" ] && [ -z "$(echo "$v" | awk '{print $4}')" ] ||
    { echo "refused: $tree/target/release/teq is ${v:-no build}, not a plain build of $rev"; return 1; }
}
manifest() {
  local v
  v=$(cd "$1" && rustc -vV)
  echo "revision $2"
  echo "source $3"
  echo "rustc $(echo "$v" | sed -n 's/^release: //p') $(echo "$v" | sed -n 's/^commit-hash: //p') LLVM $(echo "$v" | sed -n 's/^LLVM version: //p')"
  echo "cargo $(cd "$1" && cargo -V)"
  echo "target $(echo "$v" | sed -n 's/^host: //p')"
  echo "profile release, the tree's target directory"
  echo "kind plain"
  echo "flags none"
  echo "config $(config_hash "$1")"
  echo "sha256 $(sha256sum "$1/target/release/teq" | cut -d' ' -f1)"
  echo "needs $(ldd "$1/target/release/teq" 2> /dev/null | awk '{print $1}' | LC_ALL=C sort | tr '\n' ' ')glibc $(objdump -T "$1/target/release/teq" 2> /dev/null | grep -o 'GLIBC_[0-9.]*' | sort -u -V | tail -1 | sed 's/GLIBC_//')"
  echo "built $(date -u +%Y-%m-%dT%H:%MZ) on $(hostname)"
}
same_build() {
  [ "$(grep -E '^(rustc|cargo|target|profile|kind|flags|config) ' "$1")" = "$(grep -E '^(rustc|cargo|target|profile|kind|flags|config) ' "$2")" ]
}
received() {
  local v
  [ -x "$1/teq" ] && [ -f "$1/manifest" ] || { echo "no binary or no manifest in $1"; return 1; }
  [ "$(sha256sum "$1/teq" | cut -d' ' -f1)" = "$4" ] || { echo "its digest is not the builder's $4"; return 1; }
  grep -q -x "sha256 $4" "$1/manifest" && grep -q -x "revision $2" "$1/manifest" && grep -q -x "source $3" "$1/manifest" ||
    { echo "its manifest is not the build of $2 ($3) with the digest $4"; return 1; }
  v=$("$1/teq" --version 2>&1 | head -1)
  [ "$(echo "$v" | awk '{print $3}')" = "$2" ] && [ -z "$(echo "$v" | awk '{print $4}')" ] ||
    { echo "it does not run here as a plain build of $2: ${v:-no output} (this machine: $(ldd --version 2>&1 | head -1))"; return 1; }
}
toolchain_key() {
  local v
  v=$(cd "$1" && rustc -vV)
  printf '%s-%s-plain' "$(echo "$v" | sed -n 's/^release: //p')" \
    "$(printf '%s\n%s\n%s' "$v" "$(cd "$1" && cargo -V)" "$(config_hash "$1")" | sha256sum | cut -c1-8)"
}
