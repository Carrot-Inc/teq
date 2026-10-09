#!/bin/bash
# Builds the macOS installer of the extension, zed-teq-<version>.pkg under the output directory
# (the first argument, else the repository's out/), from the bundle package.sh makes: a package
# that installs under the user's home, without an administrator password, on Apple silicon
# (installer/distribution.xml), after installer/preinstall removed an earlier copy, refused while
# Zed's own Scala extension is installed. Unsigned: README.md, "Installing with the macOS installer", has what that
# means and how a Developer ID would sign it.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
out=${1:-$here/../../out}
mkdir -p "$out"
out=$(cd "$out" && pwd)
bundle=$("$here/package.sh" "$out")
version=$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*$/\1/p' "$here/extension.toml")
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/root" "$work/scripts" "$work/res"
# The files' provenance attribute, which macOS puts on whatever a process writes and nothing
# removes, rides along as AppleDouble entries that the installer applies as the attribute again.
tar -xzf "$bundle" -C "$work/root"
cp "$here/installer/preinstall" "$work/scripts/preinstall"
chmod +x "$work/scripts/preinstall"
cp "$here/installer/welcome.html" "$work/res/"
sed "s/@VERSION@/$version/g" "$here/installer/distribution.xml" > "$work/distribution.xml"
pkgbuild --quiet --root "$work/root" --identifier build.teq.zed --version "$version" \
  --install-location "Library/Application Support/Zed/extensions/installed/teq" --scripts "$work/scripts" "$work/zed-teq-component.pkg"
productbuild --quiet --distribution "$work/distribution.xml" --resources "$work/res" --package-path "$work" "$out/zed-teq-$version.pkg"
echo "$out/zed-teq-$version.pkg"
