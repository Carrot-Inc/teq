#!/bin/bash
# The GitHub release's asset names as the release scripts form and expect them
# (bench/ship-release.sh's release_asset): teq-<version>-<classifier>, with .exe for Windows alone;
# release_assets lists them beside SHA256SUMS and the manifest, and release_links (bench/release.sh --pin) names
# each document's download link by the same rule, whatever suffix it had. The plugin's side is ReleaseSuite's, the
# Zed extension's its test of asset_name.
cd "$(dirname "$0")/.." || exit 1
. bench/ship-release.sh || exit 1
status=0
expect() { [ "$2" = "$3" ] || { echo "FAIL release: $1 is '$2', not '$3'"; status=1; }; }
for c in osx-aarch_64 osx-x86_64 linux-x86_64 linux-aarch_64; do
  expect "$c's asset" "$(release_asset 0.1.7 "$c")" "teq-0.1.7-$c"
done
expect "windows-x86_64's asset" "$(release_asset 0.1.7 windows-x86_64)" teq-0.1.7-windows-x86_64.exe
expect "the release's files" "$(release_assets 0.1.7 | tr '\n' ' ')" \
  "teq-0.1.7-osx-aarch_64 teq-0.1.7-osx-x86_64 teq-0.1.7-linux-x86_64 teq-0.1.7-linux-aarch_64 teq-0.1.7-windows-x86_64.exe SHA256SUMS teq-0.1.7-binaries.txt teq-0.1.7-profiles.tar NOTICE LICENSE "
doc=$(mktemp) || exit 1
base=https://github.com/Carrot-Inc/teq/releases/download
printf '%s\n' "- [mac]($base/v0.1.6/teq-0.1.6-osx-aarch_64.exe)" "- [linux]($base/v0.1.6/teq-0.1.6-linux-x86_64)" \
  "- [windows]($base/v0.1.6/teq-0.1.6-windows-x86_64.exe)" "- [elsewhere](https://example.invalid/teq-0.1.6-linux-x86_64.exe)" > "$doc"
release_links 0.1.7 "$doc"
expect "the links --pin writes" "$(cat "$doc")" "$(printf '%s\n' "- [mac]($base/v0.1.7/teq-0.1.7-osx-aarch_64)" \
  "- [linux]($base/v0.1.7/teq-0.1.7-linux-x86_64)" "- [windows]($base/v0.1.7/teq-0.1.7-windows-x86_64.exe)" \
  "- [elsewhere](https://example.invalid/teq-0.1.6-linux-x86_64.exe)")"
rm -f "$doc"
[ $status -eq 0 ] && echo "release: the asset names, .exe for Windows alone, in the release's files and the documents' links"
exit $status
