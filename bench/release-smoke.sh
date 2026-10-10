#!/bin/bash
# bench/release-smoke.sh [--mirror <releases dir> <maven dir>] <version>: a release of the compiler, and the
# plugin it selects (integrations/sbt/plugin-version.txt), as a new build meets them (docs/TARGETS.md, "Releases"),
# on a Linux runner: after the publish and its read-back (from teq's GitHub release and Maven Central), or, with
# --mirror, before it, from the staged set (a release's files under <releases dir>/v<version>/, the plugin's Maven
# tree under <maven dir>) served by bench/release-mirror.py, which stands in for both; an empty <maven dir> is a
# release that publishes no plugin, whose plugin comes from Central (its root, TEQ_CENTRAL_ROOT's for a check). A build of its own under
# out/release-smoke/<version>-<time>/: its project/plugins.sbt the one line the language server writes for the plugin
# (src/lsp/export.rs, plugin_file_text), `addSbtPlugin("build.teq" % "sbt-teq" % "<plugin>")`, no resolver in either
# scope, its build.sbt the Scala version, `teqVersion := "<version>"` and teqBuildTool; sbt run with a global base, Ivy
# home, action cache, coursier cache and teq cache of the build's own, so that nothing this machine resolved, fetched
# or published locally answers, and none of TEQ, TEQ_PLUGIN_VERSION, TEQ_VERSION and TEQ_COMPILER. Then:
#  - teqResolvedBinary gives the release's binary for this machine, which prints the version: the plugin's jar in the
#    build's coursier cache under Central's root (the mirror's), nothing published locally, and the binary in the
#    plugin's cache beside its receipt, fetched from the release's base (the mirror's);
#  - teqExportAll writes teq.lock, whose binaries are exactly the release's as its GitHub release serves them, for
#    every classifier bench/ship-qualified.txt publishes (release_records: the asset's canonical URL, the manifest's
#    SHA-1 and size; check-export.py binaries);
#  - the Unix launcher the export wrote, with teq's and coursier's caches empty and no TEQ, fetches this machine's
#    binary by the lock (through its redirect), which prints the version, and compiles the build through it; under
#    --mirror the launcher reads a copy of the lock whose URLs are the mirror's direct paths, since it follows a
#    redirect to https alone and the mirror's is plain http (the plugin and the export go through the redirect);
#  - every other classifier's binary, fetched by the lock's URL, has the lock's sha1 and size (each runs on its own
#    platform, which is the maintainer's run, teq.cmd's fetch on Windows among them: wine's cmd cannot run teq.cmd,
#    docs/DEVELOPING.md, "The launchers");
#  - the language server's export of a build that names no plugin: the plugin added by its plugin file
#    (-addPluginSbtFile) and TEQ_VERSION the compiler's version, as `teq lsp` runs it, pins the release.
# Each sbt run is bounded by 900 s, each fetch and launcher run by 300 s; every log stays in the run's directory.
# Prints a line per check, FAIL before a failed one, and exits 1 when one failed.
set -u
cd "$(dirname "$0")/.." || exit 1
usage() { echo "usage: $0 [--mirror <releases dir> <maven dir>] <version>"; exit 2; }
mirror_releases= mirror_maven=
if [ "${1:-}" = --mirror ]; then
  [ $# -ge 3 ] && [ -d "$2" ] && [ -d "$3" ] || usage
  mirror_releases=$(cd "$2" && pwd) mirror_maven=$(cd "$3" && pwd)
  shift 3
fi
[ $# -eq 1 ] || usage
version=$1
repo=$PWD
run=$repo/out/release-smoke/$version-$(date -u +%Y%m%dT%H%M%SZ)
mkdir -p "$run" || exit 1
if [ -n "$mirror_releases" ]; then
  timeout 5400 python3 -B bench/release-mirror.py "$run/mirror.port" "$mirror_releases" "$mirror_maven" > "$run/mirror.log" 2>&1 &
  server=$!
  trap 'kill $server 2> /dev/null' EXIT
  for _ in $(seq 100); do [ -s "$run/mirror.port" ] && break; sleep 0.1; done
  [ -s "$run/mirror.port" ] || { echo "smoke: the mirror did not start: $(cat "$run/mirror.log")"; exit 1; }
  export TEQ_RELEASES_BASE=http://127.0.0.1:$(cat "$run/mirror.port")/releases/download
  # The staged plugin through the mirror; none staged, the plugin Central serves.
  [ -z "$(ls -A "$mirror_maven")" ] || central=http://127.0.0.1:$(cat "$run/mirror.port")/maven
fi
. bench/ship-release.sh || exit 1
central=${central:-$release_central_root}
releases=$release_github_root
plugin=$(release_plugin_version .) || exit 1
status=0
pass() { echo "smoke: $*"; }
failed() { echo "FAIL smoke: $*"; status=1; }
# prints <line> <version>: whether a binary's --version line names the version (`teq <version> <commit>...`).
prints() { [[ $1 == "teq $2" || $1 == "teq $2 "* ]]; }
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) host=linux-x86_64 ;;
  Linux-aarch64) host=linux-aarch_64 ;;
  Darwin-arm64) host=osx-aarch_64 ;;
  Darwin-x86_64) host=osx-x86_64 ;;
  *) echo "smoke: no release classifier for $(uname -s) $(uname -m)"; exit 1 ;;
esac
echo "smoke: teq $version from $releases and sbt-teq $plugin from $central on $host, in $run"

# The directory of a repository root in coursier's cache, relative to it, as coursier's CachePath.localFile names it
# (integrations/sbt/example/check-export.py's coursier_file: a port's colon escaped, a user, a query kept).
cached() {
  python3 -c 'import importlib.util, sys
spec = importlib.util.spec_from_file_location("lock", "integrations/sbt/example/check-export.py")
lock = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lock)
print(lock.coursier_file("", sys.argv[1]))' "$1"
}

# build <dir> <plugins.sbt> <build.sbt>: a build of the run's, its sources one main.
build() {
  mkdir -p "$1/project" "$1/src/main/scala" &&
    cp integrations/sbt/example/project/build.properties "$1/project/" &&
    printf '%s' "$2" > "$1/project/plugins.sbt" && printf '%s' "$3" > "$1/build.sbt" &&
    printf '@main def hello(): Unit =\n  println("hello from teq")\n' > "$1/src/main/scala/Hello.scala"
}

# sbt_in <dir> <log> [<env>...] -- <command>...: sbt in a build of the run's, with sbt's, coursier's and teq's state
# of its own beside it (<dir>.state), none of teq's variables but those given, bounded; a plugin repository other than
# Maven Central (the mirror's) among the resolvers of the build and of its plugins, and releases other than GitHub's
# as the releases' base, by the global settings.
sbt_in() {
  local dir=$1 log=$2 state=$1.state vars=()
  shift 2
  while [ "$1" != -- ]; do vars+=("$1"); shift; done
  shift
  mkdir -p "$state/global/plugins" || return 1
  if [ "$central" != https://repo1.maven.org/maven2 ]; then
    printf 'resolvers += "smoke-central" at "%s/"\n' "$central" > "$state/global/plugins/central.sbt"
    printf 'resolvers += "smoke-central" at "%s/"\n' "$central" > "$state/global/central.sbt"
  fi
  if [ "$releases" != https://github.com/Carrot-Inc/teq/releases/download ]; then
    printf 'SettingKey[String]("teqReleases") := "%s"\n' "$releases" > "$state/global/releases.sbt"
  fi
  (cd "$dir" && env -u TEQ -u TEQ_PLUGIN_VERSION -u TEQ_VERSION -u TEQ_COMPILER COURSIER_CACHE="$state/coursier" TEQ_CACHE_DIR="$state/teq-cache" \
    SBT_OPTS="${SBT_OPTS:--Xmx6g}" "${vars[@]}" timeout 900 sbt -Dsbt.global.base="$state/global" -Dsbt.ivy.home="$state/ivy" \
    -Dsbt.global.localcache="$state/sbt-cache" -Dsbt.server.autostart=false --server --batch "$@" < /dev/null > "$log" 2>&1)
}

# The release's build: the plugin's line alone, no resolver, and the compiler named.
new=$run/new
build "$new" "addSbtPlugin(\"build.teq\" % \"sbt-teq\" % \"$plugin\")
" "scalaVersion := \"3.8.4\"
teqVersion := \"$version\"
teqBuildTool := true
" || exit 1
if sbt_in "$new" "$run/new-sbt.log" -- 'show teqResolvedBinary' teqExportAll; then
  binary=$new/target/teq/bin/teq-$version-$host
  printed=$("$binary" --version 2>&1)
  if prints "$printed" "$version"; then pass "teqResolvedBinary gave $binary, which prints '$printed'"; else failed "teqResolvedBinary's $binary prints '$printed' (see $run/new-sbt.log)"; fi
  plugin_jar=$new.state/coursier/$(cached "$central")/build/teq/sbt-teq_sbt2_3/$plugin/sbt-teq_sbt2_3-$plugin.jar
  receipt=$(find "$new.state/teq-cache/releases" -path "*/$version/$host/$(release_asset "$version" "$host").receipt" 2> /dev/null | head -1)
  if [ -f "$plugin_jar" ] && [ -z "$(find "$new.state/ivy" -path '*/local/*' -print -quit 2> /dev/null)" ] &&
    [ -n "$receipt" ] && grep -qx "base $releases" "$receipt"; then
    pass "the plugin came from $central (the build's coursier cache, nothing published locally) and the binary from $releases (the plugin's cache, its receipt: $(grep '^sha256 ' "$receipt"))"
  else
    failed "the plugin's jar is not in the build's coursier cache under $central, or the binary has no receipt of $releases: $(ls "$plugin_jar" 2>&1), receipt '${receipt:-none}'"
  fi
else
  failed "sbt over the release's build (see $run/new-sbt.log): $(grep -m3 -E '\[error\]' "$run/new-sbt.log" | tr '\n' ' ')"
fi

records=$(release_records "$version" $(release_qualified_classifiers)) || failed "the release's binaries are not served whole (above)"
mapfile -t records <<< "$records"
if [ -f "$new/teq.lock" ] && out=$(python3 -B integrations/sbt/example/check-export.py binaries "$new/teq.lock" "$version" "${records[@]}"); then
  pass "teq.lock pins exactly the release's binaries as $releases serves them: $(printf '%s\n' "${records[@]}" | awk '{print $1}' | tr '\n' ' ')"
else
  failed "teq.lock does not pin the release's binaries: ${out:-no lock}"
fi

# The Unix launcher, with teq's and coursier's caches empty: the binary fetched by the lock.
launcher=$run/launcher
mkdir -p "$launcher/teq-cache" "$launcher/coursier"
pinned=$(printf '%s\n' "${records[@]}" | awk -v c="$host" '$1 == c {print $3}')
if [ -n "$mirror_releases" ] && [ -f "$new/teq.lock" ]; then
  # The mirror's direct paths: the launcher follows a redirect to https alone.
  sed "s|$releases/|${releases%/releases/download}/objects/releases/download/|" "$new/teq.lock" > "$new/teq.lock.direct" &&
    cp "$new/teq.lock" "$new/teq.lock.release" && mv "$new/teq.lock.direct" "$new/teq.lock"
fi
printed=$(cd "$new" && env -u TEQ TEQ_CACHE_DIR="$launcher/teq-cache" COURSIER_CACHE="$launcher/coursier" timeout 300 ./teq --version 2> "$run/launcher.log")
fetched=$launcher/teq-cache/bin/$pinned/teq-$version-$host
if prints "$printed" "$version" && [ -n "$pinned" ] && [ "$(sha1sum < "$fetched" 2> /dev/null | cut -c1-40)" = "$pinned" ]; then
  pass "./teq fetched the $host binary by the lock into an empty cache ($fetched; $(tr '\n' ' ' < "$run/launcher.log")), which prints '$printed'"
  project=$(python3 -B integrations/sbt/example/check-export.py projects "$new/teq.lock" | head -1)
  if out=$(cd "$new" && env -u TEQ TEQ_CACHE_DIR="$launcher/teq-cache" COURSIER_CACHE="$launcher/coursier" timeout 300 ./teq compile "$project" 2>&1); then
    pass "./teq compile $project through the fetched binary"
  else
    failed "./teq compile $project: $out"
  fi
  (cd "$new" && env -u TEQ TEQ_CACHE_DIR="$launcher/teq-cache" timeout 30 ./teq stop > /dev/null 2>&1)
else
  failed "./teq --version printed '$printed' ($(tr '\n' ' ' < "$run/launcher.log")), the cache holding $(ls -R "$launcher/teq-cache" 2>&1 | tr '\n' ' ')"
fi
[ ! -f "$new/teq.lock.release" ] || mv "$new/teq.lock.release" "$new/teq.lock"

# The other classifiers' binaries, by the lock's URLs: the bytes it pins.
mkdir -p "$run/download"
for record in "${records[@]}"; do
  read -r c url sha1 size <<< "$record"
  [ "$c" != "$host" ] || continue
  if curl -fsSL --max-time 300 -o "$run/download/$c" "$url" && [ "$(sha1sum < "$run/download/$c" | cut -c1-40)" = "$sha1" ] &&
    [ "$(wc -c < "$run/download/$c")" -eq "$size" ]; then
    pass "$c: $url is the lock's $size bytes, sha1 $sha1"
  else
    failed "$c: $url is not the lock's $size bytes with sha1 $sha1"
  fi
done

# The language server's export: a build naming no plugin, the plugin added by the file teq lsp writes and the
# compiler by TEQ_VERSION.
lsp=$run/lsp
build "$lsp" "" 'scalaVersion := "3.8.4"
' || exit 1
rm -f "$lsp/project/plugins.sbt"
printf 'addSbtPlugin("build.teq" %% "sbt-teq" %% "%s")\n' "$plugin" > "$run/lsp-plugin.sbt"
if sbt_in "$lsp" "$run/lsp-sbt.log" TEQ_VERSION="$version" -- "-addPluginSbtFile=$run/lsp-plugin.sbt" teqExportAll &&
  out=$(python3 -B integrations/sbt/example/check-export.py binaries "$lsp/target/teq/teq.lock" "$version" "${records[@]}"); then
  pass "the language server's export (sbt-teq $plugin added, TEQ_VERSION $version) pins the release"
else
  failed "the language server's export (see $run/lsp-sbt.log): ${out:-}"
fi


[ $status -eq 0 ] && echo "smoke: teq $version and sbt-teq $plugin pass" || echo "smoke: teq $version FAILS (above); the logs are in $run"
exit $status
