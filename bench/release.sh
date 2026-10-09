#!/bin/bash
# bench/release.sh <version> | --plugin <version> | --pin [--commit]: the commits around a ship (docs/TARGETS.md,
# "Releases").
#
# bench/release.sh <version>: the compiler's version bump a ship starts from. It sets <version> in the two
# files that name the compiler's version (bench/ship-release.sh): Cargo.toml's package and the teq package of
# Cargo.lock; checks that they agree, that Cargo takes the lock as it stands and that the GitHub release
# serves nothing of the version; and prints the commit to make on master, "Release <version>", whose landing starts
# the release workflow (.github/workflows/release.yml). The version is a release's, <major>.<minor>.<patch>, after the checkout's (the scheme
# moves the patch: 0.1.7, 0.1.8, ...). It leaves the plugin's version alone: a compiler release publishes no
# plugin unless the plugin moves too. CHANGELOG.md has the version's section, committed before the bump: the
# newest, `## <version> (<yyyy-mm-dd>)` over its bullets in public words (bench/changelog.sh), which are the notes
# of the release's GitHub release; it is printed for a last read.
#
# bench/release.sh --plugin <version>: the plugin's version bump, for a release of sbt-teq after a change to its
# code. It sets <version> in integrations/sbt/plugin-version.txt, the plugin's own line (from 1.0.0), which the
# compiler then selects (build.rs bakes it in) and whose release carries the checkout's compiler as its default
# teqVersion; checks that the version comes after the plugin's and that Central serves nothing of it; and prints
# the commit to make, "Release sbt-teq <version>", which bench/ship.sh --publish --plugin <version> then publishes
# with the compiler's release.
#
# bench/release.sh --pin: once the checkout's release is published and read back, moves the sbt example to it:
# its project/plugins.sbt to the plugin the compiler selects and its build.sbt's `ThisBuild / teqVersion` to the
# compiler (the line added when missing), after checking that the GitHub release serves every asset of it and
# Central the plugin; exports the example (sbt teqExportAll), so that its teq.lock, which the committed launcher
# reads, pins the release's compiler and binaries and records the new build files among its inputs; checks that the
# lock changed in those alone and that its binaries are exactly the release's as the GitHub release serves them
# (check-export.py pin against release_records: each qualified classifier, its asset's canonical URL, the SHA-1 and
# size the release's binary manifest gives, checked against its SHA256SUMS, each asset served at that size); moves
# the documents' instructions for a new release to it (release_documents: the plugin's coordinates, `teqVersion` and
# the download links, which name the last release on purpose until its files are served, each link named as
# release_asset names its classifier's asset); and
# prints the commit to make, "Pin the example and the documents to <version>", after which the site is built and
# deployed from those documents (site/README.md); with --commit it makes that commit itself, for the workflow that
# pushes it: its committer GIT_COMMITTER_NAME and GIT_COMMITTER_EMAIL where the environment sets them (the
# workflow's bot), its author GIT_AUTHOR_* where set, else the committer. A pin that names the release already is exported all the same,
# its lock moving alone when it does not pin the release yet; nothing changed, the lock checked all the same, is
# "pins already". The export needs sbt, and scala-cli for the example's api/lib/util.jar when it is missing.
#
# Each refuses a checkout with changes, and commits nothing but --pin --commit.
set -u
cd "$(dirname "$0")/.." || exit 1
. bench/ship-release.sh
. bench/changelog.sh
fail() { echo "release: $*"; exit 1; }
current=$(release_version .) || fail "the checkout's version is not one release's"
plugin=$(release_plugin_version .) || fail "the checkout names no plugin version"
next=$(awk -F. '{print $1 "." $2 "." $3 + 1}' <<< "$current")
commit=
case ${1:-} in
  --plugin) [ $# -eq 2 ] ;;
  --pin) [ $# -eq 1 ] || { [ $# -eq 2 ] && [ "$2" = --commit ] && commit=1; } ;;
  *) [ $# -eq 1 ] ;;
esac || { echo "usage: $0 <version> | --plugin <version> | --pin [--commit] (the checkout's release is $current, the next patch $next; its plugin $plugin)"; exit 2; }
# after <a> <b>: whether version b comes after version a.
after() { [ "$1" != "$2" ] && [ "$(printf '%s\n%s\n' "$1" "$2" | sort -t. -k1,1n -k2,2n -k3,3n | tail -1)" = "$2" ]; }
[ -z "$(git status --porcelain)" ] || fail "the checkout has changes; the release's commits are commits of their own"

# rewrite <file> <awk program>: the file through the program, replaced whole, `version` the compiler's and
# `plugin` the plugin's.
rewrite() { awk -v version="$version" -v plugin="$plugin" "$2" "$1" > "$1.new" && mv "$1.new" "$1"; }
# only <file> <pattern>...: whether every line the file changed in matches one of the patterns.
only() {
  local file=$1 pattern args=()
  shift
  for pattern in "$@"; do args+=(-e "$pattern"); done
  ! git diff -U0 -- "$file" | awk '/^@@/ { hunk = 1; next } hunk && /^[-+]/' | grep -v "${args[@]}" > /dev/null
}
# changed_alone <file>... [-- <file>...]: no file changed but those named, each before `--` in one
# line, its version's; those after it (the example's lock), changed or not, are checked by their own
# comparison.
changed_alone() {
  local lines="" named f
  while [ $# -gt 0 ] && [ "$1" != -- ]; do lines="$lines $1"; shift; done
  [ $# -gt 0 ] && shift
  named=" $lines $* "
  for f in $(git diff --name-only); do
    case $named in
      *" $f "*) ;;
      *) fail "other files changed than the versions' and the lock: $(git diff --numstat | tr '\n' ' ')${undo:+; $undo}" ;;
    esac
  done
  for f in $lines; do
    [ "$(git diff --numstat -- "$f" | awk '{print $1 $2}')" = 11 ] ||
      fail "other lines changed than the versions: $(git diff --numstat | tr '\n' ' ')${undo:+; $undo}"
  done
}

if [ "$1" = --pin ]; then
  version=$current
  release_served "$version" $(release_qualified_classifiers) || fail "the pin waits for $version's publish"
  release_plugin_legacy "$plugin" && fail "the compiler $version selects the plugin $plugin, a release up to 0.1.6, which serves no compiler after 0.1.6: bench/release.sh --plugin <version> first"
  plugin_served "$plugin" || fail "the pin waits for sbt-teq $plugin's publish"
  example=integrations/sbt/example
  rewrite $example/project/plugins.sbt '{ sub(/getOrElse\("TEQ_PLUGIN_VERSION", "[^"]*"\)/, "getOrElse(\"TEQ_PLUGIN_VERSION\", \"" plugin "\")") } { print }' ||
    fail "the example's pin could not be rewritten"
  [ "$(release_pinned . | awk '{print $2}' | sort -u)" = "$plugin" ] || fail "the example does not name $plugin after the rewrite: $(release_pinned . | tr '\n' ' ')"
  # The compiler the example's export pins: its own line, which the plugin's default (the compiler it was
  # released with) is not once a compiler release changes no plugin.
  rewrite $example/build.sbt '/^ThisBuild \/ teqVersion := "/ { $0 = "ThisBuild / teqVersion := \"" version "\""; done = 1 } { print } END { if (!done) print "ThisBuild / teqVersion := \"" version "\"" }' ||
    fail "the example's compiler could not be rewritten"
  # The export under the pinned plugin, from the repository (TEQ_PLUGIN_VERSION unset), into the
  # committed teq.lock, even when the pin names the release already: the lock may not; the jar the
  # example's api lists made when missing, as check-export.sh makes it.
  undo="git checkout -- $example $release_documents undoes the pin, the export and the documents' move"
  [ -f $example/api/lib/util.jar ] ||
    (cd $example && timeout 200 scala-cli --power package lib-src --library -o api/lib/util.jar -f -S 3.8.4 --server=false > target-util-jar.log 2>&1) ||
    fail "the example's api/lib/util.jar could not be made (see $example/target-util-jar.log); $undo"
  (cd $example && env -u TEQ_PLUGIN_VERSION -u TEQ_VERSION timeout 600 sbt --server --batch teqExportAll > target-pin-export.log 2>&1) ||
    fail "the example's export under $version failed (see $example/target-pin-export.log); $undo"
  pinned=
  git diff --quiet -- $example/project/plugins.sbt || pinned=$example/project/plugins.sbt
  # shellcheck disable=SC2086
  changed_alone $pinned -- $example/teq.lock $example/build.sbt
  only $example/build.sbt '^[-+]ThisBuild / teqVersion := "' || fail "the example's build.sbt changed in other lines than its teqVersion; $undo"
  git show HEAD:$example/teq.lock > $example/target-export-pin-before.lock
  # The release's binaries as the repository serves them, which the lock must pin and nothing else.
  records=$(release_records "$version" $(release_qualified_classifiers)) || fail "the release's binaries are not served whole (above); $undo"
  mapfile -t records <<< "$records"
  python3 $example/check-export.py pin $example/teq.lock $example/target-export-pin-before.lock "$version" "${records[@]}" ||
    fail "the example's lock changed in more than its header and the digests of plugins.sbt and build.sbt, or does not pin $version's binaries as the repository serves them (above); $undo"
  # The documents' instructions for a new release, in the lines that name one alone: the plugin, with no
  # resolver, from Central, and the compiler; served both (above).
  for doc in $release_documents; do
    rewrite "$doc" '{ gsub(/"build\.teq" % "sbt-teq" % "[^"]*"/, "\"build.teq\" % \"sbt-teq\" % \"" plugin "\""); gsub(/teqVersion := "[^"]*"/, "teqVersion := \"" version "\"") } { print }' &&
      release_links "$version" "$doc" || fail "$doc could not be rewritten; $undo"
    only "$doc" '"build\.teq" % "sbt-teq" % "' 'teqVersion := "' '/releases/download/v' || fail "$doc changed in other lines than its release's: $(git diff --numstat -- "$doc"); $undo"
  done
  # Nothing changed: the pin and its lock, checked above like any other, name the release already.
  [ -z "$(git diff --numstat)" ] && { echo "release: the example, its lock and the documents pin $version and sbt-teq $plugin already"; exit 0; }
  git diff
  if [ -n "$commit" ]; then
    # The workflow's identity where the environment gives it, the author the committer unless set apart.
    [ -z "${GIT_COMMITTER_NAME:-}" ] || export GIT_AUTHOR_NAME=${GIT_AUTHOR_NAME:-$GIT_COMMITTER_NAME}
    [ -z "${GIT_COMMITTER_EMAIL:-}" ] || export GIT_AUTHOR_EMAIL=${GIT_AUTHOR_EMAIL:-$GIT_COMMITTER_EMAIL}
    git commit -q -m "Pin the example and the documents to $version" -- $release_pins $example/build.sbt $release_documents ||
      fail "the pin's commit could not be made; $undo"
    echo "release: committed $(git rev-parse --short HEAD), 'Pin the example and the documents to $version', as $(git log -1 --format='%cn <%ce>'); then the site built and deployed from it (site/README.md)"
    exit 0
  fi
  echo "release: the example's pin and the documents on $version and sbt-teq $plugin; the commit to make on master, then the site built and deployed from it (site/README.md):"
  echo "  git commit -m 'Pin the example and the documents to $version' $release_pins $example/build.sbt $release_documents"
  exit 0
fi

if [ "$1" = --plugin ]; then
  version=$current plugin=$2
  [[ $plugin =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "$plugin is not a release's version, <major>.<minor>.<patch>"
  after "$(release_plugin_version .)" "$plugin" || fail "$plugin does not come after the checkout's plugin $(release_plugin_version .)"
  release_plugin_legacy "$plugin" && fail "$plugin is a version of the releases up to 0.1.6: sbt-teq's own line starts at 1.0.0"
  plugin_unserved "$plugin" || fail "sbt-teq $plugin cannot be the next release of the plugin"
  echo "$plugin" > integrations/sbt/plugin-version.txt || fail "integrations/sbt/plugin-version.txt could not be rewritten"
  [ "$(release_plugin_version .)" = "$plugin" ] || fail "integrations/sbt/plugin-version.txt does not name $plugin after the rewrite"
  changed_alone integrations/sbt/plugin-version.txt
  git diff
  echo "release: sbt-teq $(git show HEAD:integrations/sbt/plugin-version.txt | tr -d '[:space:]') to $plugin, released with the compiler $current; the commit to make on master:"
  echo "  git commit -m 'Release sbt-teq $plugin' integrations/sbt/plugin-version.txt"
  echo "release: then bench/ship.sh --publish --plugin $plugin, with the compiler's release or after it"
  exit 0
fi

version=$1
[[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "$version is not a release's version, <major>.<minor>.<patch>"
after "$current" "$version" || fail "$version does not come after the checkout's $current"
notes=$(changelog_notes CHANGELOG.md "$version") || fail "the release's notes are CHANGELOG.md's section for $version (above), written in a commit before the bump"
release_unserved "$version" || fail "$version cannot be the next release"
rewrite Cargo.toml '!done && /^version *= *"/ { sub(/"[^"]*"/, "\"" version "\""); done = 1 } { print }' &&
  rewrite Cargo.lock 'previous == "name = \"teq\"" && /^version = "/ { $0 = "version = \"" version "\"" } { previous = $0; print }' || fail "the files could not be rewritten"
[ "$(release_version .)" = "$version" ] || fail "the files do not name $version after the rewrite: $(release_versions . | tr '\n' ' ')"
timeout 120 cargo metadata --locked --offline --no-deps --format-version 1 > /dev/null || fail "Cargo does not take Cargo.lock as it stands"
changed_alone Cargo.toml Cargo.lock
git diff
echo "release: the notes of the GitHub release $version, CHANGELOG.md's section:"
sed 's/^/  /' <<< "$notes"
echo "release: $current to $version, the plugin staying $plugin; the commit to make on master:"
echo "  git commit -m 'Release $version' Cargo.toml Cargo.lock"
echo "release: then, after a change to the plugin, bench/release.sh --plugin <version>; land the commit: its push to GitHub starts the release workflow, which builds, qualifies, publishes and pins it (docs/DEVELOPING.md, \"Releases\")"
