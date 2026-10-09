# Sourced by tests/gate-shard.sh and tests/gate-compare.sh, from a tree's root: the one way the timing lines build a
# teq binary and the identity they compare builds by. The configuration is fixed: `cargo build --release` into the
# tree's own target directory, whatever CARGO_TARGET_DIR says, for the host, with no flags from the environment; an
# environment that would change what cargo builds is refused rather than recorded. A manifest names what is left
# to differ between two machines, the toolchain and the cargo configuration from outside the tree (the tree's own
# .cargo/config.toml is part of its sources, so a branch that changes it is compared as it builds).
#   build_teq <tree> <revision>        builds <tree>/target/release/teq stamped with <revision> and checks that the
#                                      executable names it and is a plain build; the build's output on stdout
#   manifest <tree> <revision> <source hash>  prints the manifest of that build
#   same_build <manifest> <manifest>   whether two manifests agree on everything but the revision and the source
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
  echo "built $(date -u +%Y-%m-%dT%H:%MZ) on $(hostname)"
}
same_build() {
  [ "$(grep -E '^(rustc|cargo|target|profile|kind|flags|config) ' "$1")" = "$(grep -E '^(rustc|cargo|target|profile|kind|flags|config) ' "$2")" ]
}
toolchain_key() {
  local v
  v=$(cd "$1" && rustc -vV)
  printf '%s-%s-plain' "$(echo "$v" | sed -n 's/^release: //p')" \
    "$(printf '%s\n%s\n%s' "$v" "$(cd "$1" && cargo -V)" "$(config_hash "$1")" | sha256sum | cut -c1-8)"
}
