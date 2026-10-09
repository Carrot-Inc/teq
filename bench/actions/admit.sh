#!/bin/bash
# bench/actions/admit.sh: the admission of a run of the release workflow (.github/workflows/release.yml;
# docs/DEVELOPING.md, "Releases"), its first job, from a whole checkout (every commit, the tags) of the
# repository. It decides, from the run's event (GitHub's variables: GITHUB_REPOSITORY, GITHUB_REF,
# GITHUB_EVENT_NAME, GITHUB_SHA, GITHUB_EVENT_PATH, GITHUB_RUN_ID) and the dispatch's inputs (TEQ_VERSION,
# TEQ_DRY, TEQ_RESUME_RUN, TEQ_TRAIN, TEQ_PROFILES, given as variables, never spliced into a script), whether the
# run releases and what:
#
#   - only the canonical repository's master: another repository (a fork) or ref is refused;
#   - a push releases when its head commit's subject is exactly `Release <major>.<minor>.<patch>` (bench/release.sh
#     writes it); a push without such a commit ends the run, releasing nothing; a push holding one below its head
#     is refused, its release dispatched by hand;
#   - a dispatch names the version and releases the one commit of master's first-parent history whose subject is
#     `Release <version>`; with TEQ_DRY=true it rehearses on the head of master the version its files name (never
#     another), a scratch draft in the place of the release (bench/github-release.sh --dry-run);
#   - the release's commit is on master and bumped the version (its parent names another), and its files agree on
#     the version (bench/ship-release.sh's release_version); CHANGELOG.md has its section when bench/changelog.sh is
#     there;
#   - the tag v<version> must not exist, but on a resume (TEQ_RESUME_RUN, the id of the earlier run of this
#     workflow whose record and binaries the run takes up, bench/ship-publish.sh's record, which
#     bench/actions/release-step.sh checks against the binaries),
#     and a tag on the same commit whose release is published ends the run as done;
#   - one run at a time: a run while an earlier run of the workflow is queued, waiting or in progress is refused,
#     not queued behind it (GitHub would replace a queued one by the next);
#   - the images the Linux jobs run in (bench/actions/ship-image.txt), linux/amd64's and linux/arm64's, are the
#     commit's recipe's, and, but for a dry run, bench/ship-qualified.txt qualifies the first (`image`); the
#     second's qualification (`image-aarch64`) is said, a training in it being the stage's to qualify;
#   - the profiles: trained afresh with TEQ_TRAIN=true, else those of the GitHub release TEQ_PROFILES names (a tag),
#     by default the release before this one's, v<the version the release commit's parent names> (for a dry run on
#     a commit that bumped nothing, its own version's), which the profiles job takes up or refuses
#     (bench/actions/profiles.sh);
#   - the plugin the compiler selects (integrations/sbt/plugin-version.txt): one Central serves, which the release
#     publishes again never; one it does not, which the release publishes when the commit "Release sbt-teq <version>"
#     (bench/release.sh --plugin) precedes the release's on master, its intent; never one up to 0.1.6. A resume
#     takes the decision from the record of the run it resumes (its artifact ship-record: plugin.publish), never
#     deciding again, since Central serves the plugin that run promoted; the record must be of the release's
#     version, commit and plugin. A resume of a run that left no record, which put nothing out, decides as a new
#     run does.
#
# Writes its decision as the step's outputs (GITHUB_OUTPUT: go, version, commit, dry, mode, source_run, image,
# image_arm64, qualified, qualified_arm64, train, profiles, plugin, plugin_publish, tag) and into the run's summary; exits 1 on a refusal, 0 when the run goes
# or ends without releasing. The release commit's files are read from an export of it, the checkout left as it
# is (this script among them). Needs git, gh (GH_TOKEN, the workflow's, which reads the repository's runs) and python3.
set -uo pipefail
cd "$(dirname "$0")/../.." || exit 1
repo=Carrot-Inc/teq
workflow=.github/workflows/release.yml
outputs=${GITHUB_OUTPUT:-/dev/stdout}
summary=${GITHUB_STEP_SUMMARY:-/dev/null}
refuse() {
  echo "admit: refused: $*" >&2
  printf 'The release run is refused: %s\n' "$*" >> "$summary"
  exit 1
}
say() { echo "admit: $*"; }
out() { printf '%s=%s\n' "$1" "$2" >> "$outputs"; }
release_subject='^Release ([0-9]+\.[0-9]+\.[0-9]+)$'
dry=false mode=fresh source_run=${GITHUB_RUN_ID:-} version= commit=
[ "${TEQ_DRY:-false}" = true ] && dry=true

[ "${GITHUB_REPOSITORY:-}" = "$repo" ] || refuse "the repository is ${GITHUB_REPOSITORY:-unknown}, not $repo: releases run in the canonical repository alone"
[ "${GITHUB_REF:-}" = refs/heads/master ] || refuse "the ref is ${GITHUB_REF:-unknown}, not refs/heads/master"
timeout 300 git fetch -q --tags origin +refs/heads/master:refs/remotes/origin/master || refuse "origin's master could not be fetched"
[ "$(git rev-parse --is-shallow-repository)" = false ] || refuse "the checkout is shallow"
master=$(git rev-parse refs/remotes/origin/master) || refuse "no origin/master"
tree=$(mktemp -d) || exit 1
held=$(mktemp -d) || { rm -rf -- "$tree"; exit 1; }
trap 'rm -rf -- "$tree" "$held"' EXIT
# export <commit>: the commit's files into $tree, its rules (bench/ship-release.sh) sourced from there.
export_tree() {
  rm -rf -- "${tree:?}"/* && git archive "$1" | tar -x -C "$tree" || refuse "no tree of $1"
  . "$tree/bench/ship-release.sh" || refuse "${1:0:12} has no bench/ship-release.sh"
}

case ${GITHUB_EVENT_NAME:-} in
  push)
    [ "$dry" = false ] && [ -z "${TEQ_RESUME_RUN:-}" ] || refuse "a push takes no dry run and no resume"
    subject=$(git log -1 --format=%s "$GITHUB_SHA") || refuse "no commit $GITHUB_SHA"
    if [[ $subject =~ $release_subject ]]; then
      version=${BASH_REMATCH[1]} commit=$GITHUB_SHA
    else
      # The push's commits, from the event: a release commit below the head is not released by this run.
      before=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1])).get("before", ""))' "$GITHUB_EVENT_PATH" 2> /dev/null)
      held=
      if [[ $before =~ ^[0-9a-f]{40}$ ]] && [ "$before" != 0000000000000000000000000000000000000000 ] && git cat-file -e "$before^{commit}" 2> /dev/null; then
        held=$(git log --format='%h %s' "$before..$GITHUB_SHA" | grep -E ' Release [0-9]+\.[0-9]+\.[0-9]+$')
      fi
      [ -z "$held" ] || refuse "the push's head ${GITHUB_SHA:0:12} is not its release commit ($held): the release is dispatched (workflow_dispatch with its version)"
      say "the push's head ${GITHUB_SHA:0:12} ('$subject') releases nothing"
      out go false
      printf 'The push of %s releases nothing.\n' "${GITHUB_SHA:0:12}" >> "$summary"
      exit 0
    fi
    ;;
  workflow_dispatch)
    if [ "$dry" = true ]; then
      [ -z "${TEQ_RESUME_RUN:-}" ] || refuse "a dry run resumes nothing"
      commit=$GITHUB_SHA
      export_tree "$commit"
      version=$(release_version "$tree") || refuse "the head of master ${commit:0:12} names no one version (above)"
      [ -z "${TEQ_VERSION:-}" ] || [ "$TEQ_VERSION" = "$version" ] || refuse "a dry run rehearses the version master's files name, $version, not $TEQ_VERSION"
    else
      [[ ${TEQ_VERSION:-} =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || refuse "the version '${TEQ_VERSION:-}' is not <major>.<minor>.<patch>"
      version=$TEQ_VERSION
      found=$(git log --first-parent --format='%H %s' "$master" | awk -v s="Release $version" '{ h = $1; $1 = ""; if (substr($0, 2) == s) print h }')
      [ -n "$found" ] || refuse "master has no commit 'Release $version' on its first-parent history"
      [ "$(wc -l <<< "$found")" -eq 1 ] || refuse "master has $(wc -l <<< "$found") commits 'Release $version'"
      commit=$found
    fi
    ;;
  *) refuse "the event ${GITHUB_EVENT_NAME:-unknown} starts no release" ;;
esac

git merge-base --is-ancestor "$commit" "$master" || refuse "${commit:0:12} is not on master (${master:0:12})"
export_tree "$commit"
files=$(release_version "$tree") || refuse "the files of ${commit:0:12} name no one version (above)"
[ "$files" = "$version" ] || refuse "the files of ${commit:0:12} name $files, not $version"
if [ "$dry" = false ]; then
  before=$(git show "$commit^:Cargo.toml" 2> /dev/null | sed -n 's/^version *= *"\([^"]*\)".*/\1/p' | head -1)
  [ "$before" != "$version" ] || refuse "${commit:0:12} did not bump the version: its parent names $version already"
fi
if [ -f "$tree/bench/changelog.sh" ]; then
  (. "$tree/bench/changelog.sh" && changelog_notes "$tree/CHANGELOG.md" "$version" > /dev/null) || refuse "CHANGELOG.md has no section for $version to be the release's notes (above)"
fi

# The tag, and a release done already.
tag=v$version
tag_at=$(git rev-parse -q --verify "refs/tags/$tag^{commit}")
if [ -n "${TEQ_RESUME_RUN:-}" ]; then
  [[ $TEQ_RESUME_RUN =~ ^[0-9]+$ ]] || refuse "resume_run '$TEQ_RESUME_RUN' is not a run's id"
  run=$(timeout 60 gh api "repos/$repo/actions/runs/$TEQ_RESUME_RUN" --jq '[.path, .repository.full_name, .head_branch, .event, .status] | join(" ")') ||
    refuse "GitHub knows no run $TEQ_RESUME_RUN of $repo"
  read -r path full branch event status <<< "$run"
  [ "$path" = "$workflow" ] && [ "$full" = "$repo" ] && [ "$branch" = master ] && { [ "$event" = push ] || [ "$event" = workflow_dispatch ]; } ||
    refuse "the run $TEQ_RESUME_RUN is not a run of $workflow on $repo's master ($run)"
  [ "$status" = completed ] || refuse "the run $TEQ_RESUME_RUN is $status, not completed: a resume follows it"
  mode=resume source_run=$TEQ_RESUME_RUN
  say "a resume of the run $TEQ_RESUME_RUN"
elif [ -n "$tag_at" ] && [ "$dry" = false ]; then
  [ "$tag_at" = "$commit" ] || refuse "the tag $tag names ${tag_at:0:12}, not the release's commit ${commit:0:12}"
  if timeout 60 gh api "repos/$repo/releases/tags/$tag" --jq '.draft' 2> /dev/null | grep -qx false; then
    say "$tag is released on ${commit:0:12} already: nothing to do"
    out go false
    printf '%s is released on %s already: nothing to do.\n' "$tag" "${commit:0:12}" >> "$summary"
    exit 0
  fi
  refuse "the tag $tag exists on ${commit:0:12}, and its release is not published: the run that made it is resumed (resume_run)"
fi

# One run at a time: an earlier run queued, waiting or in progress refuses this one.
others=
for status in queued in_progress waiting pending requested; do
  ids=$(timeout 60 gh api "repos/$repo/actions/workflows/release.yml/runs?status=$status&per_page=100" --jq '.workflow_runs[].id') ||
    refuse "the workflow's runs could not be read"
  for id in $ids; do [ "$id" -lt "${GITHUB_RUN_ID:-0}" ] && others="$others $id"; done
done
[ -z "$others" ] || refuse "the release runs$others are not done: one run at a time, and this one is not queued behind them (run it again once they end)"

# The profiles: a training, or an earlier release's (by default the release before this one's).
train=false
[ "${TEQ_TRAIN:-false}" != true ] || train=true
profiles=${TEQ_PROFILES:-}
if [ -z "$profiles" ]; then
  previous=$(git show "$commit^:Cargo.toml" 2> /dev/null | sed -n 's/^version *= *"\([^"]*\)".*/\1/p' | head -1)
  [ -z "$previous" ] || profiles=v$previous
fi
[ -z "$profiles" ] || [[ $profiles =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]] || refuse "the profiles' release '$profiles' is not a tag"

# The images, one per architecture, and their qualification.
recipe=$(cat "$tree/bench/actions/ship.Dockerfile" "$tree/bench/actions/toolchain.sh" | sha256sum | cut -c1-64)
# image_of <platform>: the image ship-image.txt names for the platform, the one line of it, of the commit's recipe.
image_of() {
  local lines ref of
  lines=$(grep -v '^#' "$tree/bench/actions/ship-image.txt" 2> /dev/null | awk -v p="$1" '$3 == p')
  [ -n "$lines" ] || refuse "bench/actions/ship-image.txt names no $1 image yet: the ship-image workflow builds the two, and their lines are committed"
  [ "$(wc -l <<< "$lines")" -eq 1 ] || refuse "bench/actions/ship-image.txt names more than one $1 image"
  read -r ref of _ <<< "$lines"
  [[ $ref =~ ^ghcr\.io/carrot-inc/teq-ship@sha256:[0-9a-f]{64}$ ]] || refuse "bench/actions/ship-image.txt's $1 image '$ref' is not the repository's by its digest"
  [ "$of" = "$recipe" ] || refuse "the $1 image is of the recipe ${of:-none}, the commit's is $recipe: the ship-image workflow builds the commit's"
  echo "$ref"
}
image=$(image_of linux/amd64) || exit 1
image_arm64=$(image_of linux/arm64) || exit 1
[ "$image" != "$image_arm64" ] || refuse "bench/actions/ship-image.txt names one image for both architectures"
qualified=false qualified_arm64=false
[ "$(sed -n 's/^image //p' "$tree/bench/ship-qualified.txt")" = "$image" ] && qualified=true
[ "$(sed -n 's/^image-aarch64 //p' "$tree/bench/ship-qualified.txt")" = "$image_arm64" ] && qualified_arm64=true
[ "$qualified" = true ] || [ "$dry" = true ] ||
  refuse "bench/ship-qualified.txt does not qualify the image $image: a dry run qualifies it, then its line is committed (docs/DEVELOPING.md, \"Releases\")"

# The plugin the compiler selects: served already (the release publishes none), or published by this release, which
# the commit "Release sbt-teq <version>" on master before the release's commit states (bench/release.sh --plugin);
# never one up to 0.1.6, which serves no later compiler. A resume publishes it as the run it resumes began to, by that run's record: once that run
# promoted the plugin Central serves it, and a decision made again would resume the publication without it, which
# bench/ship-publish.sh refuses.
plugin=$(release_plugin_version "$tree") || refuse "the release's commit names no plugin version (above)"
release_plugin_legacy "$plugin" &&
  refuse "the compiler $version selects sbt-teq $plugin, a release up to 0.1.6, which serves no later compiler: bench/release.sh --plugin <version> first"
plugin_publish=
if [ "$mode" = resume ]; then
  records=$(timeout 60 gh api "repos/$repo/actions/runs/$source_run/artifacts?per_page=100" --jq '[.artifacts[] | select(.name == "ship-record" and (.expired | not))] | length') ||
    refuse "the artifacts of the run $source_run could not be read"
  if [ "${records:-0}" != 0 ]; then
    timeout 600 gh run download "$source_run" --repo "$repo" --name ship-record --dir "$held" > /dev/null ||
      refuse "the record of the run $source_run (its artifact ship-record) could not be downloaded"
    recorded() { python3 -B -c 'import json, sys; print(json.load(open(sys.argv[1])).get(sys.argv[2], ""))' "$held/$version/record.json" "$1"; }
    [ -f "$held/$version/record.json" ] || refuse "the record of the run $source_run holds no record of $version"
    [ "$(recorded compiler.version) $(recorded compiler.commit) $(recorded plugin.version)" = "$version $commit $plugin" ] ||
      refuse "the record of the run $source_run is of $(recorded compiler.version) on $(recorded compiler.commit | cut -c1-12) with sbt-teq $(recorded plugin.version), not of $version on ${commit:0:12} with sbt-teq $plugin"
    case $(recorded plugin.publish) in
      yes) plugin_publish=true ;;
      no) plugin_publish=false ;;
      *) refuse "the record of the run $source_run says not whether it publishes sbt-teq $plugin (plugin.publish '$(recorded plugin.publish)')" ;;
    esac
    say "the record of the run $source_run: its publication $([ "$plugin_publish" = true ] && echo publishes || echo "does not publish") sbt-teq $plugin, and the resume as well"
  else
    say "the run $source_run left no record, and put nothing out: the plugin is decided as for a new run"
  fi
fi
if [ -z "$plugin_publish" ] && plugin_served "$plugin" 2> /dev/null; then
  plugin_publish=false
elif [ -z "$plugin_publish" ]; then
  [ "$dry" = false ] || refuse "the compiler $version selects sbt-teq $plugin, which Central does not serve: a dry run publishes no plugin"
  # grep reads the whole history: one that stops at the match would cut git off (SIGPIPE), which pipefail counts.
  git log --first-parent --format=%s "$commit" | grep -xF "Release sbt-teq $plugin" > /dev/null ||
    refuse "the compiler $version selects sbt-teq $plugin, which Central does not serve, and no commit 'Release sbt-teq $plugin' before the release's states its release (bench/release.sh --plugin $plugin)"
  plugin_publish=true
fi

out go true
out version "$version"
out commit "$commit"
out tag "$tag"
out dry "$dry"
out mode "$mode"
out source_run "$source_run"
out image "$image"
out image_arm64 "$image_arm64"
out qualified "$qualified"
out qualified_arm64 "$qualified_arm64"
out train "$train"
out profiles "$profiles"
out plugin "$plugin"
out plugin_publish "$plugin_publish"
say "$([ "$dry" = true ] && echo "a dry run of" || echo "the release") $version on ${commit:0:12} ($mode, the binaries of the run $source_run), the image $image ($([ "$qualified" = true ] && echo qualified || echo "not qualified yet")), the plugin ${plugin:-none}$([ "$plugin_publish" = true ] && echo ", published by this release")"
{
  echo "| | |"
  echo "|---|---|"
  echo "| release | $version$([ "$dry" = true ] && echo " (dry run)") |"
  echo "| commit | \`$commit\` |"
  echo "| mode | $mode (binaries of run $source_run) |"
  echo "| image | \`$image\` ($([ "$qualified" = true ] && echo qualified || echo "not qualified")) |"
  echo "| arm64 image | \`$image_arm64\` ($([ "$qualified_arm64" = true ] && echo qualified || echo "not qualified")) |"
  echo "| profiles | $([ "$train" = true ] && echo "trained afresh (train)" || echo "${profiles:-none named}'s, unless stale (the profiles job)") |"
  echo "| plugin | ${plugin:-none}$([ "$plugin_publish" = true ] && echo ", published by this release") |"
} >> "$summary"
