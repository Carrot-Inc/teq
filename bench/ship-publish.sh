#!/bin/bash
# bench/ship-publish.sh [--resume] [--step <step>[,<step>...]] <version> [--plugin <plugin version>]: the publication
# of a release the ship built,
# qualified and staged under integrations/sbt/binary/binaries/ (bench/ship.sh --publish runs it; docs/TARGETS.md,
# "Releases"), in its order, each step recorded in out/ship/<version>/record.json before it runs and after
# (bench/ship-record.py):
#  1. the GitHub release v<version> as a draft, which no one but its publisher sees: every binary as its asset,
#     SHA256SUMS and the binary manifest, uploaded and read back (bench/github-release.sh <version>, the draft verb;
#     its header has the interface);
#  2. with --plugin, the plugin the compiler selects (integrations/sbt/plugin-version.txt, which <plugin version>
#     must name) staged and checked for Maven Central, not uploaded (integrations/sbt/publish.sh --stage);
#  3. the consumer smoke against the staged set through a local mirror (bench/release-smoke.sh --mirror): the
#     release's files as the draft holds them, written from the same binaries under out/github-release/v<version>/
#     (each binary checked against the record's digest first), and the staged plugin, or without --plugin the one
#     Central serves;
#  4. with --plugin, the plugin's upload to the Central Portal (integrations/sbt/central.py upload, the intent recorded
#     before it), then its validation, its promotion, which nothing undoes, and its publication (central.py), and its
#     read-back (integrations/sbt/publish.sh, which resumes the deployment recorded under out/central/sbt-teq-<plugin>/);
#  5. the draft published and every asset read back from its URL (github-release.sh <version> publish);
#  6. the public cold smoke (bench/release-smoke.sh <version>).
# Then the pin, the documents and the site (bench/release.sh --pin --commit), which it names.
#
# A new publication refuses: another version than the checkout's, a plugin version other than the one the compiler
# selects, a version of which anything is served, with --plugin a plugin version Central serves (and what the
# plugin's publish lacks: integrations/sbt/publish.sh --preflight), without it a selected plugin Central does not
# serve or one up to 0.1.6, which serves no later compiler; a binary of the five missing or of another commit or
# version; and a record of the version unless nothing of it went out (the draft deleted, no plugin promoted), which
# it then sets aside. A compiler-only publication asks for no Portal token or key, and never whether the plugin is
# unserved. --resume takes the record up where it stands, this runner's or one carried from an earlier job as an
# artifact (out/ship/<version>/, with out/central/sbt-teq-<plugin>/ for the plugin's deployment and
# out/github-release/ for the staged set), never doing again a step recorded as done: the commit must be the
# record's.
#
# A failure before the plugin's promotion abandons the draft (github-release.sh <version> abandon), so that nothing
# is public, keeps the record, and the next publication starts afresh; one after it leaves the plugin published, its
# default compiler this release, and says that --resume finishes the GitHub release, which is then never deleted.
# The record says `deleting` before the abandonment is asked for, so that a job cut off between the two leaves a
# word the next run, a fresh start or --resume, acts on first: it abandons the draft again, which ends a deletion
# begun and changes nothing after one done, and then starts afresh. A resumed publication promotes the plugin only
# once the draft verb, run again, has found the draft whole on GitHub (or made it whole from the staged binaries,
# the record's), whatever the record says. A record that cannot be written stops the run before the next step.
#
# --step runs the steps it names alone, in the order above whatever the order named: draft, stage, smoke,
# upload-begin (the upload's intent recorded), upload, promote, read-back, publish, smoke-public; each still skipped when the
# record says it is done, and refused when the record does not hold the steps before it. The release workflow
# (.github/workflows/release.yml) runs them a few at a time in jobs of their own, carrying the record and the
# plugin's out/central/ between them and keeping both before each step that writes outside the job: the staging and
# the intent before the upload, the deployment before the promotion, so that a runner lost in a step loses neither.
# The first runs as a new publication, the others with --resume.
# Each step is bounded by its commands' bounds. Needs what the scripts it runs need, and python3.
set -u
cd "$(dirname "$0")/.." || exit 1
. bench/ship-manifest.sh || exit 1
usage() { echo "usage: $0 [--resume] [--step <step>[,<step>...]] <version> [--plugin <plugin version>]"; exit 2; }
resume= version= plugin_release= only=
steps="draft stage smoke upload-begin upload promote read-back publish smoke-public"
while [ $# -gt 0 ]; do
  case $1 in
    --resume) resume=1 ;;
    --step) [ -n "${2:-}" ] || usage; only=$2; shift ;;
    --plugin) [ -n "${2:-}" ] || usage; plugin_release=$2; shift ;;
    -*) usage ;;
    *) [ -z "$version" ] || usage; version=$1 ;;
  esac
  shift
done
[ -n "$version" ] || usage
fail() { echo "ship-publish: $*"; exit 1; }
for s in ${only//,/ }; do [[ " $steps " == *" $s "* ]] || { echo "ship-publish: no step $s; the steps: $steps"; exit 2; }; done
# wanted <step>: whether the run makes the step, every one without --step.
wanted() { [ -z "$only" ] || [[ ",$only," == *",$1,"* ]]; }
record=$PWD/out/ship/$version/record.json
rec() { python3 -B bench/ship-record.py "$record" "$@"; }
# put <key> <value>...: the record written, synced, or the run stopped: no step goes on unrecorded.
put() { rec set "$@" || { echo "ship-publish: the record cannot be written at $record: nothing more is done"; exit 1; }; }
gh_release() { local bound=$1; shift; timeout "$bound" "$release_github_script" "$version" "$@"; }
# The draft and publish verbs' bounds, over every file of the release (bench/ship-release.sh).
draft_bound=$(release_draft_bound "$version")
publish_bound=$(release_publish_bound "$version")
head=$(git rev-parse HEAD)
[ "$(release_version .)" = "$version" ] || fail "the checkout's release is $(release_version . 2> /dev/null), not $version"
plugin=$(release_plugin_version .) || fail "the checkout names no plugin version"
[ -z "$plugin_release" ] || [ "$plugin_release" = "$plugin" ] ||
  fail "--plugin $plugin_release, and integrations/sbt/plugin-version.txt names $plugin: bench/release.sh --plugin <version> moves it"
[ -x "$release_github_script" ] || fail "no $release_github_script, which publishes the GitHub release"
central_record=$(plugin_central_record "$plugin")
binaries=integrations/sbt/binary/binaries

# promoted: whether this publication put its plugin out: one that publishes the plugin, its promotion requested,
# recorded (a compiler-only release's plugin went out with an earlier release, and nothing of this one did).
promoted() {
  [ -n "$plugin_release" ] && [ -f "$central_record/deployment.log" ] && grep -q '"event": "promote-requested"' "$central_record/deployment.log"
}
# abandon: the draft abandoned, the record saying so before and after.
abandon() { put github.state deleting && gh_release 600 abandon && put github.state deleted; }
# stop <why>: the failure's state said, the draft abandoned while nothing went out.
stop() {
  echo "ship-publish: $1"
  if promoted; then
    echo "ship-publish: sbt-teq $plugin is published to Central, its default compiler $version: the GitHub release v$version is never deleted now; bench/ship-publish.sh --resume $version${plugin_release:+ --plugin $plugin} finishes it"
  elif case $(rec get github.state) in drafting | draft | deleting) true ;; *) false ;; esac; then
    if abandon; then
      echo "ship-publish: the draft v$version is deleted and nothing of $version is public; the record stays, and the next publication starts afresh"
    else
      echo "ship-publish: the draft v$version could not be abandoned (above): the next run, a fresh start or --resume, abandons it first; nothing of $version is public"
    fi
  fi
  exit 1
}
# recorded_binaries: whether the staged binaries are the record's, by their digests.
recorded_binaries() {
  local c
  for c in $release_classifiers; do
    [ "$(sha256sum < "$binaries/$c/teq" 2> /dev/null | cut -c1-64)" = "$(rec get "compiler.$c.sha256")" ] ||
      { echo "ship-publish: $binaries/$c/teq is not the binary the record names (compiler.$c.sha256): the publication's binaries are its record's"; return 1; }
  done
}
# staged_set <dir>: the release's files as the draft holds them, written under <dir>/v<version>/ from the staged
# binaries for the smoke's mirror: each binary under its asset's name (release_asset), SHA256SUMS and the binary
# manifest in github-release.sh's format, the five in their order.
staged_set() {
  local dir=$1/v$version c a
  rm -rf "$dir" && mkdir -p "$dir" || return 1
  for c in $release_classifiers; do cp "$binaries/$c/teq" "$dir/$(release_asset "$version" "$c")" || return 1; done
  for c in $release_classifiers; do
    a=$(release_asset "$version" "$c")
    echo "$(sha256sum < "$dir/$a" | cut -c1-64)  $a"
  done > "$dir/SHA256SUMS" || return 1
  {
    echo "teq $version $head"
    for c in $release_classifiers; do
      a=$(release_asset "$version" "$c")
      echo "$c $a $(sha256sum < "$dir/$a" | cut -c1-64) $(sha1sum < "$dir/$a" | cut -c1-40) $(wc -c < "$dir/$a" | tr -d ' ')"
    done
  } > "$dir/teq-$version-binaries.txt"
}

if [ -n "$resume" ]; then
  [ -f "$record" ] || fail "no record of $version at $record: a publication starts without --resume"
  [ "$(rec get compiler.commit)" = "$head" ] || fail "the record is of $(rec get compiler.commit | cut -c1-12), not the head ${head:0:12}"
  [ "$(rec get plugin.publish)" = "$([ -n "$plugin_release" ] && echo yes || echo no)" ] ||
    fail "the record's publication $( [ "$(rec get plugin.publish)" = yes ] && echo publishes || echo does not publish) sbt-teq $plugin: resume it as it began"
  # A draft deleted after a failure before the plugin's promotion: nothing went out, and the publication starts
  # afresh (its record set aside); resuming it would publish the plugin with no release of its compiler.
  afresh="bench/ship-publish.sh $version${plugin_release:+ --plugin $plugin_release}, without --resume, starts it afresh"
  [ "$(rec get github.state)" != deleted ] ||
    fail "the draft v$version was deleted after a failure before the plugin's promotion: nothing of $version went out; $afresh"
  # An abandonment asked for and not recorded as done: it is ended, and the publication starts afresh.
  if [ "$(rec get github.state)" = deleting ]; then
    abandon || fail "the abandonment of the draft v$version, begun after a failure before the plugin's promotion, did not end (above): run again"
    fail "the draft v$version is abandoned, after a failure before the plugin's promotion: nothing of $version went out; $afresh"
  fi
  echo "ship-publish: resuming the publication of $version: GitHub $(rec get github.state), Central $(rec get central.state), smokes $(rec get smoke.mirror)/$(rec get smoke.public)"
else
  if [ -f "$record" ]; then
    case $(rec get github.state) in
      none | deleted) ;;
      # An abandonment asked for and not recorded as done: ended first.
      deleting) abandon || fail "$record records the abandonment of the draft v$version, which did not end (above): run again" ;;
      *) fail "$record records the GitHub release v$version as $(rec get github.state): --resume takes it up" ;;
    esac
    promoted && fail "$central_record records sbt-teq $plugin's promotion: --resume takes the publication up"
    aside=$record.$(date -u +%Y%m%dT%H%M%SZ)
    mv "$record" "$aside" || exit 1
    echo "ship-publish: the record of a publication that put nothing out is set aside as $aside"
  fi
  release_unserved "$version" || fail "$version cannot be published"
  if [ -n "$plugin_release" ]; then
    integrations/sbt/publish.sh --preflight || fail "sbt-teq $plugin cannot be published to Central (above)"
  else
    release_plugin_legacy "$plugin" && fail "the compiler $version selects sbt-teq $plugin, a release up to 0.1.6, which serves no later compiler: bench/release.sh --plugin <version>, then --plugin <version>"
    plugin_served "$plugin" || fail "the compiler $version selects sbt-teq $plugin, which Central does not serve: --plugin $plugin publishes it with this release"
  fi
  digests=()
  for c in $release_classifiers; do
    [ -f "$binaries/$c/teq" ] && [ -f "$binaries/$c/teq.manifest" ] || fail "no binary staged for $c: the release carries all five, bench/ship.sh stages them"
    [ "$(manifest_get "$binaries/$c/teq.manifest" commit)" = "$head" ] || fail "$binaries/$c is built from $(manifest_get "$binaries/$c/teq.manifest" commit | cut -c1-12), not the head ${head:0:12}"
    [[ $(manifest_get "$binaries/$c/teq.manifest" version) == "teq $version "* ]] || fail "$binaries/$c is '$(manifest_get "$binaries/$c/teq.manifest" version)', not $version's"
    digests+=("compiler.$c.sha256" "$(sha256sum < "$binaries/$c/teq" | cut -c1-64)")
  done
  gh_release 300 check || fail "the GitHub release cannot be published (above)"
  # The plugin's default compiler: this release's when it publishes the plugin, else the one the served plugin was
  # released with, which its jar's manifest names (Teq-Compiler).
  default=$version
  [ -n "$plugin_release" ] || default=$(plugin_default "$plugin") || default=unknown
  put compiler.version "$version" compiler.commit "$head" "${digests[@]}" plugin.version "$plugin" plugin.default "$default" \
    plugin.publish "$([ -n "$plugin_release" ] && echo yes || echo no)" github.state none central.state "$([ -n "$plugin_release" ] && echo none || echo served)" \
    smoke.mirror none smoke.public none
  echo "ship-publish: publishing $version of ${head:0:12}, selecting sbt-teq $plugin$([ -n "$plugin_release" ] && echo ", published with it"); the record is $record"
fi

# drafted_whole: whether the record has the draft whole, or past it.
drafted_whole() { case $(rec get github.state) in draft | publishing | published | read-back) true ;; *) false ;; esac; }
# 1. The draft, every asset uploaded to it and read back.
drafted=
wanted draft && case $(rec get github.state) in
  none | drafting)
    put github.state drafting
    gh_release $draft_bound || stop "the draft v$version could not be made whole (above)"
    put github.state draft
    drafted=1 ;;
esac
# 2. The plugin staged, not uploaded.
if wanted stage && [ -n "$plugin_release" ] && [ "$(rec get central.state)" = none ]; then
  drafted_whole || fail "the plugin is staged after the draft, which the record does not hold whole ($(rec get github.state))"
  integrations/sbt/publish.sh --stage || stop "sbt-teq $plugin could not be staged (above)"
  put central.state staged
fi
# 3. The consumer smoke against the staged set, through a local mirror.
if wanted smoke && [ "$(rec get smoke.mirror)" != passed ]; then
  drafted_whole || fail "the smoke comes after the draft, which the record does not hold whole ($(rec get github.state))"
  [ -z "$plugin_release" ] || [ "$(rec get central.state)" != none ] || fail "the smoke reads the plugin's staging, which the record does not hold: the stage step first"
  maven=$PWD/out/ship/$version/no-plugin
  mkdir -p "$maven" "$PWD/out/github-release"
  [ -z "$plugin_release" ] || maven=$central_record/staging
  recorded_binaries && staged_set "$PWD/out/github-release" || stop "the release's files could not be staged for the smoke (above)"
  timeout 3600 "$release_smoke" --mirror "$PWD/out/github-release" "$maven" "$version" || stop "the smoke against the staged set failed (above)"
  put smoke.mirror passed
fi
# 4. The plugin's publication to Central. The upload's intent, then the upload of the staging the smoke read, its
# deployment recorded under the plugin's record (an upload recorded there already, its upload-started event, is not
# made again: the promotion's wait asks the Portal about it, which refuses a second deployment of the version all
# the same). The staging is the head's, as publish.sh uploads one, and central.py checks it is unchanged.
if wanted upload-begin && [ -n "$plugin_release" ] && [ "$(rec get central.state)" = staged ]; then
  [ "$(rec get smoke.mirror)" = passed ] || fail "the upload comes after the smoke against the staged set, which the record does not hold passed"
  put central.state uploading
fi
if wanted upload && [ -n "$plugin_release" ] && case $(rec get central.state) in staged | uploading) true ;; *) false ;; esac; then
  [ "$(rec get smoke.mirror)" = passed ] || fail "the upload comes after the smoke against the staged set, which the record does not hold passed"
  put central.state uploading
  if ! grep -q '"event": "upload-started"' "$central_record/deployment.log" 2> /dev/null; then
    [ "$(python3 -B integrations/sbt/central.py staged "$central_record")" = "$head" ] ||
      stop "$central_record holds no staging of the head ${head:0:12}: the plugin is staged afresh by a new publication"
    timeout 1500 python3 -B integrations/sbt/central.py upload "$central_record" release ||
      stop "sbt-teq $plugin's upload to the Portal failed (above); a deployment of the version the Portal holds and $central_record lacks, an upload whose job was lost, is reconciled on the Portal before the next run"
  fi
  put central.state uploaded
fi
# Then the promotion, irreversible, which never comes before the whole draft of the plugin's default compiler: a
# resumed publication has the draft verb find it again on GitHub (a record's word may be stale: a draft deleted since
# by a hand).
# The promotion itself is central.py's (the deployment validated, promoted, published), the read-back after it
# integrations/sbt/publish.sh's, which resumes the deployment where it stands (a promotion made is not made again).
if wanted promote && [ -n "$plugin_release" ] && case $(rec get central.state) in read-back | published) false ;; *) true ;; esac; then
  case $(rec get central.state) in
    uploaded | publishing) ;;
    *) fail "the promotion comes after the upload, which the record does not hold ($(rec get central.state)): the upload step first" ;;
  esac
  if ! promoted && [ -z "$drafted" ]; then
    recorded_binaries && gh_release $draft_bound ||
      fail "the draft v$version is not found whole on GitHub, nor made whole (above), and nothing promotes sbt-teq $plugin, whose default compiler is $version, without it: bench/ship-publish.sh --resume $version --plugin $plugin_release with the record's binaries staged tries again"
  fi
  put central.state publishing
  { timeout 2000 python3 -B integrations/sbt/central.py wait "$central_record" VALIDATED 1800 &&
    timeout 300 python3 -B integrations/sbt/central.py promote "$central_record" "$head" &&
    timeout 3800 python3 -B integrations/sbt/central.py wait "$central_record" PUBLISHED 3600; } ||
    stop "sbt-teq $plugin's promotion did not end (above); the promote step again takes it up from $central_record"
  put central.state published
fi
if wanted read-back && [ -n "$plugin_release" ] && [ "$(rec get central.state)" != read-back ]; then
  case $(rec get central.state) in
    published) ;;
    *) fail "the read-back comes after the promotion, which the record does not hold ($(rec get central.state)): the promote step first" ;;
  esac
  timeout 20000 integrations/sbt/publish.sh || stop "sbt-teq $plugin's read-back from Central did not end (above); integrations/sbt/publish.sh resumes it from $central_record"
  put central.state read-back
fi
# 5. The draft published, then every asset read back from its URL (the publish verb, which run again reads back).
wanted publish && { [ "$(rec get smoke.mirror)" = passed ] && { [ -z "$plugin_release" ] || [ "$(rec get central.state)" = read-back ]; } ||
  fail "the draft is published after the smoke and, with the plugin, its read-back from Central, which the record does not hold (smoke $(rec get smoke.mirror), Central $(rec get central.state))"; }
wanted publish && case $(rec get github.state) in
  draft | publishing | published)
    put github.state publishing
    gh_release $publish_bound publish || stop "the draft v$version could not be published and read back whole (above); --resume publishes or reads it back"
    put github.state read-back ;;
esac
# 6. The public cold smoke.
if wanted smoke-public && [ "$(rec get smoke.public)" != passed ]; then
  [ "$(rec get github.state)" = read-back ] || fail "the public smoke comes after the release's publication, which the record does not hold ($(rec get github.state))"
  timeout 3600 "$release_smoke" "$version" || stop "the public smoke of $version failed (above): the release is out; a fix goes into the next release"
  put smoke.public passed
fi
[ -z "$only" ] || { echo "ship-publish: the steps $only of $version done; the record is $record"; exit 0; }
echo "ship-publish: released $version of ${head:0:12} on GitHub$([ -n "$plugin_release" ] && echo " and sbt-teq $plugin on Maven Central"); the record is $record"
echo "ship-publish: then bench/release.sh --pin --commit (the example and the documents, pushed), and the site built and deployed from them (site/README.md)"
