# bench/ship-release.sh, sourced: the versions a ship releases and their checks (docs/TARGETS.md,
# "Releases"), for bench/release.sh, bench/ship.sh and the publishing scripts. Two version lines, which
# no repository ever takes twice each: the compiler's, the GitHub release v<version>, an exe per classifier,
# each ship a version of its own, the patch after the last (0.1.7, 0.1.8, ...), which Cargo.toml's
# package (build.rs stamps it into `teq --version`) and the teq package of Cargo.lock name and agree
# on; and sbt-teq's, build.teq:sbt-teq:<plugin version>, from 1.0.0, which moves only when the plugin
# does (integrations/sbt/plugin-version.txt: the plugin's build.sbt reads it, build.rs bakes it into
# the compiler as the plugin it selects). A compiler release that changes no plugin publishes none; a
# plugin release carries the compiler of its checkout as its default teqVersion (BuildInfo.compiler).

# Where a release goes, each by its root, asked without credentials:
#
#   what          root                                            published by
#   the binaries  https://github.com/Carrot-Inc/teq/releases/     bench/github-release.sh: the GitHub release v<version>,
#                 download (TEQ_RELEASES_BASE another)            its assets teq-<version>-<classifier> (release_asset:
#                                                                 .exe for Windows alone), SHA256SUMS and the binary
#                                                                 manifest teq-<version>-binaries.txt
#                                                                 (`<classifier> <asset> <sha256> <sha1> <size>` lines
#                                                                 after `teq <version> <commit>`), the profiles, and
#                                                                 NOTICE and LICENSE; the releases from 0.1.7, the
#                                                                 first served
#   sbt-teq       https://repo1.maven.org/maven2                  integrations/sbt/publish.sh: staged and signed by
#                                                                 sbt, uploaded to the Central Portal as a deployment,
#                                                                 validated, promoted by its id (central.py)
#
# The binaries never go to Maven Central, whose monthly limits a release's five would exceed.
release_github_root=${TEQ_RELEASES_BASE:-https://github.com/Carrot-Inc/teq/releases/download}
release_github_root=${release_github_root%/}
release_central_root=${TEQ_CENTRAL_ROOT:-https://repo1.maven.org/maven2}
release_central_root=${release_central_root%/}
# The GitHub release's script, bench/github-release.sh (its header has the interface), which bench/ship.sh and
# bench/ship-publish.sh call as `<script> <version> [<verb>]`, by its verbs:
#   check         what the draft would refuse but for the binaries, before a build, nothing written
#   (none)        the draft v<version>: the five binaries staged under integrations/sbt/binary/binaries/ its assets,
#                 with SHA256SUMS and teq-<version>-binaries.txt from the same bytes, the profiles, and the release's
#                 commit's NOTICE and LICENSE, uploaded and downloaded back,
#                 made or resumed: whole, or a failure; run on a whole draft, it finds it whole again
#   publish       the whole draft published, its tag on the release's commit, and every asset read back from its
#                 URL; run again, it reads back again
#   abandon       every draft of v<version> deleted; a published release never
# Its exit is 0 when the verb is done. The smoke the publication runs, before the release (--mirror) and after.
# TEQ_RELEASES_BASE, TEQ_CENTRAL_ROOT, TEQ_GITHUB_RELEASE and TEQ_RELEASE_SMOKE put stand-ins in the place of the
# real ones, for a check of the scripts (the Portal's is central.py's TEQ_CENTRAL_PORTAL, a loopback address alone).
release_github_script=${TEQ_GITHUB_RELEASE:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/github-release.sh}
release_smoke=${TEQ_RELEASE_SMOKE:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/release-smoke.sh}
# The key that signs every file of the plugin on Central, its fingerprint: in the runner's keyring (the workflow imports it),
# without a passphrase, its public key on keyserver.ubuntu.com, which the Portal reads (publish.sh checks
# the three before anything is built).
release_signing_key=6B5D1AE81940490ACBA428EE4A6C943144E0560F
# The oldest glibc the Linux binaries load on: the version zig links them against (bench/zig.sh) and the
# ceiling of every version a binary's version-needs table may name (bench/ship-manifest.sh's manifest_glibc).
# rust-analyzer's floor; RHEL 8's, Debian 10's and Ubuntu 18.10's glibc.
release_glibc=2.28
# The platforms a ship stages, in bench/ship.sh's order: every classifier a binary of teq can be published
# under (the plugin's TeqPlugin.platformClassifier), any of which spends a version. Which of them a ship
# publishes is the qualification file's say (release_qualified_classifiers).
release_classifiers="osx-aarch_64 osx-x86_64 linux-x86_64 linux-aarch_64 windows-x86_64"
release_known=$release_classifiers

# The routes: per classifier, the target its binary is built for and the tools of its toolchain beyond
# rustc, LLVM, cargo and the build machine's release, all of which the binary's manifest records
# (bench/ship-manifest.sh's manifest_tuple). zig links every binary a ship gives off; wine runs the Windows
# smoke; the floor's glibc package (sysroot-aarch64) is what the Linux aarch64 suite runs over. The aarch64
# trainer, whose profile guides the arm64 builds, and the Linux aarch64 suite run natively on an arm64 Linux
# machine, or emulated on the x86-64 one under qemu-user: release_route_emulated names the tools a manifest
# needs besides when its records say so (the emulated route; bench/ship-manifest.sh's manifest_qualified).
release_route_target() {
  case $1 in
    osx-aarch_64) echo aarch64-apple-darwin ;;
    osx-x86_64) echo x86_64-apple-darwin ;;
    linux-x86_64) echo x86_64-unknown-linux-gnu ;;
    linux-aarch_64) echo aarch64-unknown-linux-gnu ;;
    # gnullvm, not gnu, whose thread-locals go through an OS key (docs/TARGETS.md, "Windows").
    windows-x86_64) echo x86_64-pc-windows-gnullvm ;;
    *) return 1 ;;
  esac
}
release_route_tools() {
  case $1 in
    osx-aarch_64) echo "zig" ;;
    osx-x86_64) echo "zig" ;;
    linux-x86_64) echo "zig" ;;
    linux-aarch_64) echo "zig sysroot-aarch64" ;;
    windows-x86_64) echo "zig wine" ;;
    *) return 1 ;;
  esac
}
# release_route_emulated <classifier>: the tools of the route's emulated work on the x86-64 machine, the aarch64
# training under qemu-user (both arm64 builds) and the Linux aarch64 suite under it; none for the others.
release_route_emulated() {
  case $1 in
    osx-aarch_64 | linux-aarch_64) echo "qemu-user" ;;
    osx-x86_64 | linux-x86_64 | windows-x86_64) ;;
    *) return 1 ;;
  esac
}

# release_qualified_classifiers: the classifiers of the set the qualification file publishes
# (bench/ship-qualified.txt; bench/ship-manifest.sh's manifest_qualified checks a manifest the same way): each
# tool of the route named once in the file, and a `route <classifier>` line, which a commit adds once the
# route's binary has run where it is for (docs/SPEED.md, "The ship from Linux"). A classifier the file does
# not publish yet is staged and held back (bench/ship.sh), and the release is whole without it. None while
# a `pending` line stands.
release_qualified_classifiers() {
  local qualified=${TEQ_SHIP_QUALIFIED:-$(dirname "${BASH_SOURCE[0]}")/ship-qualified.txt} c key ok
  [ -r "$qualified" ] && ! grep -q '^pending' "$qualified" || return 0
  for c in $release_classifiers; do
    ok=1
    for key in rustc llvm cargo os $(release_route_tools "$c"); do [ "$(grep -c "^$key " "$qualified")" -eq 1 ] || ok=; done
    [ "$(grep -c "^route $c\$" "$qualified")" -eq 1 ] || ok=
    [ -n "$ok" ] && echo "$c"
  done
}

# What a release publishes is built from, besides the compiler's sources (bench/ship-manifest.sh): the
# three files, and the sbt plugin's tree, its build definitions (any .sbt file at its root among
# them), sources and the binaries' staging and check, but for the builds of its own beside it (the
# example, the analysis oracle). The publish takes them as the head commits them, the commit the
# binaries are built from: nothing of them changed, added, or present and ignored by git (an
# ignored file is read by sbt all the same), but for what the builds write themselves: their
# target/ directories, sbt's .bsp/, the binaries staged under binary/binaries/. Git pathspecs,
# arrays so that the shell never expands them.
release_inputs=(Cargo.toml Cargo.lock integrations/sbt ':(exclude)integrations/sbt/example' ':(exclude)integrations/sbt/analysis')
release_outputs=(':(exclude,glob)integrations/sbt/**/target/**' ':(exclude)integrations/sbt/.bsp' ':(exclude)integrations/sbt/binary/binaries')

# The example's pin of a release, moved to it once it is published and read back (bench/release.sh
# --pin): the sbt example's plugin, its teq.lock exported under it, and the repository's own teq.lock
# taken from that (release_root_lock). The Zed extension's fallback finds the newest release by itself.
release_pins="integrations/sbt/example/project/plugins.sbt integrations/sbt/example/teq.lock teq.lock"

# release_root_lock <example lock> <lock>: the repository's own lock (docs/DEVELOPING.md, "The repository's
# scripts"), which its launchers read: the example's sections up to its inputs, the compiler's version, the lock's
# format and the binaries; written whole by a rename.
release_root_lock() {
  awk '/^[^ ]/ && !/^(teq|format|binaries):/ { exit } { print }' "$1" > "$2.part" && mv "$2.part" "$2"
}
# The documents whose instructions name the newest release (`"build.teq" % "sbt-teq" % "<version>"`, the
# download links under the Central root), moved with the pin, and the site built from them after it: until
# then they name the release before, whose files are served.
release_documents="README.md docs/TOOLING.md integrations/sbt/README.md"

# release_versions <root>: the compiler's version each file names, a line `<file> <version>` each.
release_versions() {
  echo "Cargo.toml $(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' "$1/Cargo.toml" | head -1)"
  echo "Cargo.lock $(sed -n '/^name = "teq"$/{n;s/^version = "\([^"]*\)"$/\1/p;}' "$1/Cargo.lock" | head -1)"
}

# release_plugin_version <root>: the plugin's version, integrations/sbt/plugin-version.txt's one line,
# <major>.<minor>.<patch>; fails naming the file otherwise.
release_plugin_version() {
  local version
  version=$(tr -d '[:space:]' < "$1/integrations/sbt/plugin-version.txt" 2> /dev/null)
  [[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "integrations/sbt/plugin-version.txt holds '$version', not <major>.<minor>.<patch>" >&2; return 1; }
  echo "$version"
}

# release_plugin_legacy <plugin version>: whether the plugin is a release up to 0.1.6, before the plugin's own
# line, none of which is served any more, nor serves a compiler from 0.1.7; the plugin's own line starts at 1.0.0,
# on Central.
release_plugin_legacy() { [[ $1 =~ ^0\.1\.([0-6]|0-pre\..+)$ ]]; }

# release_version <root>: the compiler's version the two files agree on, a release's, <major>.<minor>.<patch>;
# fails naming them otherwise.
release_version() {
  local versions version
  versions=$(release_versions "$1")
  version=$(awk '{print $2}' <<< "$versions" | sort -u)
  [ -n "$version" ] && [ "$(wc -l <<< "$version")" -eq 1 ] && [ "$(awk 'NF != 2' <<< "$versions")" = "" ] ||
    { echo "the files that name the version disagree: $(tr '\n' ',' <<< "$versions" | sed 's/,$//; s/,/, /g')" >&2; return 1; }
  [[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "$version is not a release's version, <major>.<minor>.<patch>" >&2; return 1; }
  echo "$version"
}

# release_committed <root>: whether the release's inputs are the head's (release_inputs); fails
# naming what is not, `!!` before a file git ignores.
release_committed() {
  local changed ignored
  changed=$(cd "$1" && git status --porcelain --untracked-files=all -- "${release_inputs[@]}") || return 1
  ignored=$(cd "$1" && git ls-files --others --ignored --exclude-standard -- "${release_inputs[@]}" "${release_outputs[@]}" | sed 's/^/!! /') || return 1
  [ -z "$changed$ignored" ] || { echo "the release's inputs are not the head's: $(tr '\n' ' ' <<< "$changed${changed:+$'\n'}$ignored")" >&2; return 1; }
}

# release_pinned <root>: the release the example pins, a line `<file> <version>`.
release_pinned() {
  echo "integrations/sbt/example/project/plugins.sbt $(sed -n 's/.*getOrElse("TEQ_PLUGIN_VERSION", "\([^"]*\)").*/\1/p' "$1/integrations/sbt/example/project/plugins.sbt" | head -1)"
}

# Whether a file is served is asked of the file itself, never of a repository's listing.

# unserved <root> <version> <path>...: whether the repository under <root> answers 404 for each path,
# a redirect followed (a GitHub release serves its assets through one, and answers 404 for one it lacks).
# Fails naming a file it serves, or one it gave another answer for, when whether the version is out
# cannot be told.
unserved() {
  local root=$1 version=$2 path code
  shift 2
  for path in "$@"; do
    code=$(curl -sS -I -L --max-time 30 -o /dev/null -w "%{http_code}" "$root/$path" 2> /dev/null)
    case $code in
      404) ;;
      200) echo "$root/$path is served: $version is released, and a release is never published again (bench/release.sh moves to the next)" >&2; return 1 ;;
      000 | "") echo "$root/$path gave no answer: whether $version is released cannot be told" >&2; return 1 ;;
      *) echo "$root/$path answered $code: whether $version is released cannot be told" >&2; return 1 ;;
    esac
  done
}

# release_asset <version> <classifier>: a binary's name among the GitHub release's assets, teq-<version>-<classifier>,
# with .exe for Windows alone, whose process creation looks for it (as the plugin's Release.assetName and the Zed
# extension's asset_name have it); every script forms or expects an asset's name by it (tests/release.sh).
release_asset() {
  case $2 in
    windows-*) echo "teq-$1-$2.exe" ;;
    *) echo "teq-$1-$2" ;;
  esac
}

# release_links <version> <file>: the file's links to a GitHub release's binary moved to the version, each naming its
# classifier's asset as release_asset does, whatever version or suffix it named before; rewritten in place.
release_links() {
  local c pairs=
  for c in $release_known; do pairs="$pairs $c=$(release_asset "$1" "$c")"; done
  awk -v version="$1" -v pairs="$pairs" '
    BEGIN { n = split(pairs, p, " "); for (i = 1; i <= n; i++) { split(p[i], kv, "="); asset[kv[1]] = kv[2] } }
    { for (c in asset) gsub("/releases/download/v[^/]*/teq-[^-/]*-" c "(\\.exe)?", "/releases/download/v" version "/" asset[c]); print }
  ' "$2" > "$2.new" && mv "$2.new" "$2"
}

# release_assets <version>: the names of a release's files on GitHub: each classifier's asset, SHA256SUMS,
# the binary manifest, the profiles that guided the binaries (bench/ship-profiles.sh), and the release's commit's
# NOTICE and LICENSE, the notices and the license of what the binaries are built from.
release_assets() {
  local c
  for c in $release_known; do release_asset "$1" "$c"; done
  echo SHA256SUMS
  echo "teq-$1-binaries.txt"
  echo "teq-$1-profiles.tar"
  echo NOTICE
  echo LICENSE
}

# The GitHub release's verbs' bounds, the sums of bench/github-release.sh's own over every file the release holds
# (release_assets): release_draft_bound <version>, its checks and calls (900 s), an upload of each file (900), the
# download back (1800); release_publish_bound <version>, its checks and calls (900), the download back (1800), the
# tag looked for six times (60 and a pause of 5), each file read back from its URL (five tries of 300, 10 apart).
release_draft_bound() { echo $(( 900 + $(release_assets "$1" | wc -l) * 900 + 1800 )); }
release_publish_bound() { echo $(( 900 + 1800 + 6 * (60 + 5) + $(release_assets "$1" | wc -l) * (5 * 300 + 4 * 10) )); }

# release_unserved <version>: whether nothing of the version is served: its GitHub release's files each answer 404.
release_unserved() {
  unserved "$release_github_root/v$1" "$1" $(release_assets "$1")
}

# release_served <version> [<classifier>...]: whether the GitHub release serves the version's manifest,
# SHA256SUMS and every named classifier's asset (a redirect followed); fails naming one it does not.
release_served() {
  local version=$1 name c code names=()
  shift
  names=(SHA256SUMS "teq-$version-binaries.txt")
  for c in "$@"; do names+=("$(release_asset "$version" "$c")"); done
  for name in "${names[@]}"; do
    code=$(curl -sS -I -L --max-time 60 -o /dev/null -w "%{http_code}" "$release_github_root/v$version/$name" 2> /dev/null)
    [ "$code" = 200 ] || { echo "$release_github_root/v$version/$name answered ${code:-nothing}, not 200: $version is not released whole" >&2; return 1; }
  done
}
# plugin_paths <version>: what a release of sbt-teq puts on Maven Central that a build resolves, under
# its root: the pom and the jar (central.py reads back the rest, the sources and javadoc jars and every
# file's signature and digests).
plugin_paths() {
  echo "build/teq/sbt-teq_sbt2_3/$1/sbt-teq_sbt2_3-$1.pom"
  echo "build/teq/sbt-teq_sbt2_3/$1/sbt-teq_sbt2_3-$1.jar"
}

# Where a release of the plugin to Central is recorded, by its version (central.py): the staging, the
# manifest, the bundle and the deployment's events, kept so that an interrupted publish resumes it.
plugin_central_record() { echo "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/out/central/sbt-teq-$1"; }

# plugin_unserved <version>: whether Central serves no file of the plugin at the version (404 each), and
# no deployment of it is recorded here (plugin_central_record) but one dropped or an upload given up
# (central.py abandon); fails naming the file or the record, which integrations/sbt/publish.sh resumes.
plugin_unserved() {
  local record
  unserved "$release_central_root" "sbt-teq $1" $(plugin_paths "$1") || return 1
  record=$(plugin_central_record "$1")
  if [ -f "$record/deployment.log" ] && ! grep -q '"event": "\(dropped\|upload-abandoned\)"' "$record/deployment.log" &&
    grep -q '"event": "upload-started"' "$record/deployment.log"; then
    echo "$record records a deployment of sbt-teq $1 to Central: integrations/sbt/publish.sh resumes it, and nothing publishes it anew" >&2
    return 1
  fi
}

# plugin_default <version>: the compiler the plugin was released with, its default teqVersion, as the jar Central
# serves names it in its manifest (Teq-Compiler); fails when it names none.
plugin_default() {
  local jar compiler
  jar=$(mktemp) || return 1
  curl -fsSL --max-time 120 -o "$jar" "$release_central_root/build/teq/sbt-teq_sbt2_3/$1/sbt-teq_sbt2_3-$1.jar" 2> /dev/null &&
    compiler=$(unzip -p "$jar" META-INF/MANIFEST.MF 2> /dev/null | tr -d '\r' | sed -n 's/^Teq-Compiler: //p')
  rm -f "$jar"
  [ -n "${compiler:-}" ] || { echo "sbt-teq $1's jar on Central names no compiler (Teq-Compiler)" >&2; return 1; }
  echo "$compiler"
}

# plugin_served <version>: whether Central serves the plugin at the version, its pom and jar; fails
# naming a file it does not.
plugin_served() {
  local path code
  for path in $(plugin_paths "$1"); do
    code=$(curl -sS -I --max-time 30 -o /dev/null -w "%{http_code}" "$release_central_root/$path" 2> /dev/null)
    [ "$code" = 200 ] || { echo "$release_central_root/$path answered ${code:-nothing}, not 200: sbt-teq $1 is not on Central" >&2; return 1; }
  done
}

# release_records <version> <classifier>...: the release's binaries as its GitHub release serves them, a line
# `<classifier> <url> <sha1> <size>` each, the record a lock pins: the asset's canonical URL (its redirect not
# pinned), the SHA-1 and size the binary manifest gives (SHA256SUMS gives neither), the manifest read from the
# release's own directory and checked against SHA256SUMS (each asset's SHA-256 in exactly one line, the header
# naming the version), and each asset served at that size (a HEAD through its redirect). Asked without
# credentials; fails naming what does not check out.
release_records() {
  local version=$1 dir manifest sums c line asset sha256 sha1 size headers code served
  shift
  dir=$release_github_root/v$version
  manifest=$(curl -fsSL --max-time 60 "$dir/teq-$version-binaries.txt" 2> /dev/null) || { echo "$dir/teq-$version-binaries.txt is not served" >&2; return 1; }
  sums=$(curl -fsSL --max-time 60 "$dir/SHA256SUMS" 2> /dev/null) || { echo "$dir/SHA256SUMS is not served" >&2; return 1; }
  [ "$(head -1 <<< "$manifest" | awk '{print $1 " " $2}')" = "teq $version" ] ||
    { echo "$dir/teq-$version-binaries.txt begins '$(head -1 <<< "$manifest")', not 'teq $version <commit>'" >&2; return 1; }
  for c in "$@"; do
    line=$(awk -v c="$c" '$1 == c' <<< "$manifest")
    [ "$(wc -l <<< "$line")" -eq 1 ] && [ -n "$line" ] || { echo "$dir/teq-$version-binaries.txt names $c ${line:+more than }$([ -z "$line" ] && echo not || echo once)" >&2; return 1; }
    read -r _ asset sha256 sha1 size <<< "$line"
    [ "$asset" = "$(release_asset "$version" "$c")" ] && [[ $sha256 =~ ^[0-9a-f]{64}$ ]] && [[ $sha1 =~ ^[0-9a-f]{40}$ ]] && [[ $size =~ ^[0-9]+$ ]] ||
      { echo "$dir/teq-$version-binaries.txt's line for $c is not '<classifier> $(release_asset "$version" "$c") <sha256> <sha1> <size>': $line" >&2; return 1; }
    [ "$(awk -v a="$asset" '{ n = $2; sub(/^\*/, "", n) } n == a { print tolower($1) }' <<< "$sums")" = "$sha256" ] ||
      { echo "$dir/SHA256SUMS does not give $asset the manifest's $sha256 in exactly one line" >&2; return 1; }
    headers=$(curl -sSIL --max-time 60 "$dir/$asset" 2> /dev/null | tr -d '\r')
    code=$(awk '/^HTTP\// {c = $2} END {print c}' <<< "$headers")
    served=$(awk -F': *' 'tolower($1) == "content-length" {n = $2} END {print n}' <<< "$headers")
    [ "$code" = 200 ] && [ "$served" = "$size" ] || { echo "$dir/$asset is not served at the manifest's $size bytes (HEAD ${code:-unanswered}, ${served:-no} bytes)" >&2; return 1; }
    echo "$c $dir/$asset $sha1 $size"
  done
}
