#!/bin/bash
# Packs the extension as Zed installs it, for a machine without the Rust toolchain: what Zed wrote
# here when it built the dev extension (extension.wasm, grammars/scala.wasm) with the manifest, the
# language files, the README, the licences and the repository's NOTICE, as zed-teq-<version>.tar.gz under the output
# directory (the first argument, else the repository's out/), to unpack under Zed's
# extensions/installed/teq (README.md, "Installing the bundle"). WebAssembly and text only, so one
# bundle serves every platform.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
out=${1:-$here/../../out}
mkdir -p "$out"
out=$(cd "$out" && pwd)
cd "$here"
rebuild="install this directory as a dev extension in Zed, or Rebuild it there, which writes it"
for built in extension.wasm grammars/scala.wasm; do
  if [ ! -f "$built" ]; then
    echo "no $built: $rebuild" >&2
    exit 1
  fi
done
# What Zed built must be newer than what it built it from, or the bundle would carry an old
# extension under a new label.
for input in src/lib.rs src/lock.rs Cargo.toml Cargo.lock extension.toml; do
  if [ "$input" -nt extension.wasm ]; then
    echo "extension.wasm is older than $input: $rebuild" >&2
    exit 1
  fi
done
if [ extension.toml -nt grammars/scala.wasm ]; then
  echo "grammars/scala.wasm is older than extension.toml, which pins the grammar: $rebuild" >&2
  exit 1
fi
version=$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*$/\1/p' extension.toml)
if [ -z "$version" ]; then
  echo "no version in extension.toml" >&2
  exit 1
fi
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/grammars"
cp extension.toml extension.wasm README.md LICENSE-tree-sitter-scala "$stage/"
cp ../../LICENSE ../../NOTICE "$stage/"
cp grammars/scala.wasm "$stage/grammars/"
cp -R languages "$stage/languages"
bundle=$out/zed-teq-$version.tar.gz
COPYFILE_DISABLE=1 tar -czf "$bundle" -C "$stage" extension.toml extension.wasm grammars languages README.md LICENSE LICENSE-tree-sitter-scala NOTICE
echo "$bundle"
