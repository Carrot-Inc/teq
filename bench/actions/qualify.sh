#!/bin/bash
# bench/actions/qualify.sh <classifier> <dir> [<identity bundle>]: a release's binary qualified natively on the
# platform it is for, by a job of the release workflow (docs/DEVELOPING.md, "Releases"), from the root of a checkout
# of the release's commit. <dir> is bench/ship.sh stage's (binaries/<classifier>/ or, held back, held/<classifier>/:
# the binary and its manifest), or the products of the classifier's build (bench/ship.sh --step's carry: the binary
# at the path the stage takes it from, its manifest beside it), which the workflow qualifies as soon as the build
# made them; the bundle is tests/support/identity.sh --produce's of the Linux x86-64 binary, whose digest the report
# names (the staged set's own Linux x86-64 binary's, when <dir> is a staged set). The checks, in order, each
# printed, the first failure ending the run:
#   - the machine is the classifier's natively (no Rosetta on an Intel Mac's job; Git's bash on Windows);
#   - the binary is its manifest's (the digest), built from the checkout's commit, and prints the manifest's version;
#   - macOS arm64: its ad hoc signature verifies (codesign --verify --strict), as the linker made it;
#   - tests/run.sh on the binary: every case passes and none is skipped for missing jars (the jars fetched by their
#     coordinates and the built ones restored from the reference's, bench/actions/jars.sh); the cases of the JVM's
#     platform, which the suite skips on every machine, are counted in the record;
#   - the POSIX launcher (tools/launcher/teq) with a lock pinning the binary at a local URL: a cold cache fetches it
#     and runs it, a warm one runs it with the server gone, a lock pinning another sha1 is refused and places nothing;
#   - Windows aside, tests/support/identity.sh --compare against the bundle when one is given (the reference of the
#     staged Linux x86-64 binary, by its digest): every program's outputs the reference's.
# The Windows binary, after its digest, commit and version, by tests/windows.sh, the native smoke, in the place of
# the suite, the POSIX launcher and identity (its paths are Windows'). Writes out/qualify/<classifier>.txt, the
# record of what ran (the binary's digest, the reference's; the draft reads it), and <classifier>.identity.txt,
# identity's report; exits 1 on a failure.
set -uo pipefail
cd "$(dirname "$0")/../.." || exit 1
. bench/ship-manifest.sh
[ $# -ge 2 ] && [ $# -le 3 ] || { echo "usage: $0 <classifier> <staged dir> [<identity bundle>]" >&2; exit 2; }
classifier=$1 staged=$2 bundle=${3:-}
report=out/qualify/$classifier.txt
mkdir -p out/qualify && : > "$report" || exit 1
say() { echo "qualify: $classifier: $*" | tee -a "$report"; }
fail() { say "FAILED: $*"; exit 1; }
command -v timeout > /dev/null || fail "no GNU timeout on the PATH"

case $classifier/$(uname -s) in
  windows-x86_64/MINGW64*) [ "$(uname -m)" = x86_64 ] || fail "this Windows is $(uname -m), not x86_64" ;;
  windows-*) fail "this is $(uname -s), not Windows' Git bash" ;;
  *) [ "$(manifest_host)" = "$classifier" ] || fail "this machine is $(manifest_host), not $classifier" ;;
esac
if [ "$(uname -s)" = Darwin ] && [ "$(sysctl -n sysctl.proc_translated 2> /dev/null)" = 1 ]; then fail "this shell runs under Rosetta, not natively"; fi
say "on $(uname -s) $(uname -r) $(uname -m)$([ "$(uname -s)" = Darwin ] && echo ", macOS $(sw_vers -productVersion)")"

# The binary and its manifest: the staged set's, or the build's products at the paths the stage takes them from.
case $classifier in
  osx-aarch_64) built=out/cross/teq-guided-arm ;;
  osx-x86_64) built=out/cross/teq-guided-intel ;;
  linux-x86_64) built=target/ship/teq ;;
  linux-aarch_64) built=out/cross/teq-guided-linux-arm ;;
  windows-x86_64) built=out/cross/teq-windows-x86_64.exe ;;
  *) fail "no classifier $classifier" ;;
esac
dir=$staged/binaries/$classifier
[ -d "$dir" ] || dir=$staged/held/$classifier
if [ -f "$dir/teq" ]; then
  from=$dir/teq manifest=$dir/teq.manifest
else
  from=$staged/$built manifest=$staged/$built.manifest
fi
[ -f "$from" ] && [ -f "$manifest" ] || fail "no binary and manifest of $classifier in $staged"
binary=$PWD/out/qualify/teq-$classifier
[ "${classifier#windows-}" = "$classifier" ] || binary=$binary.exe
cp "$from" "$binary" && chmod 755 "$binary" || exit 1
sha=$(manifest_sha256 "$binary")
[ "$sha" = "$(manifest_get "$manifest" binary)" ] || fail "the binary's digest $sha is not its manifest's $(manifest_get "$manifest" binary)"
[ "$(manifest_get "$manifest" commit)" = "$(git rev-parse HEAD)" ] || fail "the binary is built from $(manifest_get "$manifest" commit), the checkout is $(git rev-parse HEAD)"
version=$(timeout 30 "$binary" --version | tr -d '\r') || fail "the binary does not run here"
[ "$version" = "$(manifest_get "$manifest" version)" ] || fail "the binary prints '$version', its manifest says '$(manifest_get "$manifest" version)'"
say "the binary $sha, '$version', runs"

if [ "${classifier#windows-}" != "$classifier" ]; then
  TEQ_EXE=$binary timeout 2400 tests/windows.sh 2>&1 | tee out/qualify/$classifier.smoke | tee -a "$report"
  [ "${PIPESTATUS[0]}" -eq 0 ] || fail "tests/windows.sh failed: $(tail -1 out/qualify/$classifier.smoke)"
  say "qualified"
  exit 0
fi

if [ "$classifier" = osx-aarch_64 ]; then
  timeout 60 codesign --verify --strict "$binary" 2>&1 | tee -a "$report" && [ "${PIPESTATUS[0]}" -eq 0 ] || fail "codesign --verify --strict refuses the binary"
  say "its signature verifies (codesign --verify --strict): $(codesign -dv "$binary" 2>&1 | grep -E '^(Signature|CodeDirectory)' | tr '\n' ' ')"
fi

# The suite, no case skipped.
rm -rf -- out/tests-qualify
suite=$(TEQ=$binary TEQ_TEST_OUT=out/tests-qualify timeout 2400 tests/run.sh 2>&1)
status=$?
echo "$suite" > out/qualify/$classifier.suite
# A case skipped for a jar the cache lacks is a failure; a case of the JVM's platform (`//> using platform jvm`) is
# skipped by tests/run.sh on every machine, and those are counted, not refused.
skips=$(grep -c '^skip .*not in the coursier cache' <<< "$suite")
platform=$(grep -c "^skip .*the JVM's platform" <<< "$suite")
[ $status -eq 0 ] || { grep -E '^(FAIL|REF FAIL)' <<< "$suite" | head -20 | tee -a "$report"; fail "tests/run.sh failed (exit $status): $(tail -1 <<< "$suite")"; }
[ "$skips" -eq 0 ] || { grep '^skip .*not in the coursier cache' <<< "$suite" | tee -a "$report"; fail "tests/run.sh skipped $skips cases for missing jars"; }
say "tests/run.sh passed: $(tail -1 <<< "$suite"), none skipped for jars, $platform of the JVM's platform"

# The launcher, against a local server of the binary.
work=$(mktemp -d) || exit 1
mirror=
trap '[ -z "$mirror" ] || kill "$mirror" 2> /dev/null; rm -rf -- "$work"' EXIT
mkdir -p "$work/build" "$work/wrong" "$work/cache" "$work/coursier" || exit 1
release=$(awk '{print $2}' <<< "$version")
# The launcher's cache names the binary teq-<version>-<classifier>; the release's asset is release_asset's name.
asset=teq-$release-$classifier
mkdir -p "$work/root/v$release" "$work/maven" && cp "$binary" "$work/root/v$release/$(release_asset "$release" "$classifier")" || exit 1
# The landed mirror of a release (bench/release-mirror.py), its asset at the direct path, since the launchers follow
# a redirect to https alone.
timeout 900 python3 -B bench/release-mirror.py "$work/port" "$work/root" "$work/maven" 2> "$work/mirror.log" &
mirror=$!
# Up to 30 s: a hosted macOS runner's first python3 starts slowly.
for _ in $(seq 1 300); do [ -s "$work/port" ] && break; kill -0 "$mirror" 2> /dev/null || break; sleep 0.1; done
[ -s "$work/port" ] || fail "the local server did not start ($(kill -0 "$mirror" 2> /dev/null && echo "still starting after 30 s" || echo "exited"); python3 $(python3 --version 2>&1 | head -1)): $(cat "$work/mirror.log")"
url=http://localhost:$(cat "$work/port")/objects/releases/download/v$release/$(release_asset "$release" "$classifier")
sha1=$( (command -v sha1sum > /dev/null && sha1sum < "$binary" || shasum -a 1 < "$binary") | cut -c1-40)
size=$(wc -c < "$binary" | tr -d ' ')
lock() { printf 'teq: %s\nbinaries:\n  %s: %s %s %s\n' "$release" "$classifier" "$url" "$2" "$size" > "$1/teq.lock"; }
cp tools/launcher/teq "$work/build/teq" && cp tools/launcher/teq "$work/wrong/teq" && lock "$work/build" "$sha1" || exit 1
wrong=$(printf '%040d' 0)
lock "$work/wrong" "$wrong"
launch() { env -u TEQ TEQ_CACHE_DIR="$work/cache" COURSIER_CACHE="$work/coursier" timeout 300 sh "$@"; }
out=$(launch "$work/build/teq" --version 2>&1) && [ "$out" = "$(printf 'teq: fetching teq %s for %s from %s\n%s' "$release" "$classifier" "$url" "$version")" ] ||
  fail "the launcher's cold fetch: $out"
[ -x "$work/cache/bin/$sha1/$asset" ] || fail "the launcher placed no executable $work/cache/bin/$sha1/$asset"
refused=$(env -u TEQ TEQ_CACHE_DIR="$work/cache-wrong" COURSIER_CACHE="$work/coursier" timeout 300 sh "$work/wrong/teq" --version 2>&1) &&
  fail "the launcher ran a binary whose sha1 is not the lock's: $refused"
[[ $refused == *"where the lock pins $wrong"*"refused" ]] || fail "the launcher's refusal of another sha1: $refused"
[ -z "$(find "$work/cache-wrong" -type f 2> /dev/null)" ] || fail "the launcher left a file of a binary whose sha1 is not the lock's"
kill "$mirror" 2> /dev/null
wait "$mirror" 2> /dev/null
mirror=
out=$(launch "$work/build/teq" --version 2>&1) && [ "$out" = "$version" ] || fail "the launcher's warm run, the server gone: $out"
say "the launcher: a cold cache fetches the binary from $url and runs it, a warm one runs it with the server gone, a lock of another sha1 is refused ($(tail -1 <<< "$refused"))"

if [ -n "$bundle" ]; then
  # The bundle is the reference of the Linux x86-64 binary this release publishes: the staged set's, which a staged
  # set holds; else the draft checks the digest the report names against the staged set's
  # (bench/actions/release-step.sh).
  reference=$staged/binaries/linux-x86_64/teq.manifest
  [ -f "$reference" ] || reference=$staged/held/linux-x86_64/teq.manifest
  [ -f "$reference" ] || [ "$classifier" != linux-x86_64 ] || reference=$manifest
  [ ! -f "$reference" ] || [ "$(sed -n 's/^binary //p' "$bundle/identity.txt" 2> /dev/null)" = "$(manifest_get "$reference" binary)" ] ||
    fail "the identity bundle is not of the staged Linux x86-64 binary $(manifest_get "$reference" binary)"
  rm -rf -- out/identity-qualify
  timeout 3600 tests/support/identity.sh --compare "$bundle" "$binary" out/identity-qualify 2>&1 | tail -20 | tee -a "$report"
  identity_status=${PIPESTATUS[0]}
  # The comparison's report and the differing programs' outputs beside the record, in the job's artifact (one root,
  # out/qualify/, which the draft reads the records from).
  rm -rf -- out/qualify/identity && mkdir -p out/qualify/identity && cp out/identity-qualify/report.txt out/qualify/identity/ 2> /dev/null
  [ ! -d out/identity-qualify/differs ] || cp -R out/identity-qualify/differs out/qualify/identity/
  [ "$identity_status" -eq 0 ] || fail "its outputs are not the reference's (above; out/qualify/identity)"
  say "its outputs are the Linux x86-64 reference's, the binary $(sed -n 's/^binary //p' "$bundle/identity.txt") ($(sed -n 's/^reference //p' out/identity-qualify/report.txt | cut -c1-16))"
  cp out/identity-qualify/report.txt "out/qualify/$classifier.identity.txt" || exit 1
fi
say "qualified"
