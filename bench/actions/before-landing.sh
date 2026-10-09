#!/bin/bash
# bench/actions/before-landing.sh [<hub remote> [<github remote>]]: what a landing checks and does before it moves
# the hub's master (docs/DEVELOPING.md, "Releases"), from a checkout whose remotes are the hub (`origin` by default)
# and GitHub (`github`), with gh holding a token that reads the repository's runs:
#   - no run of the release workflow is queued, waiting or in progress: a release in flight pushes its pin to
#     GitHub's master, which a landing would race; refused while one runs;
#   - GitHub's master and the hub's fetched: GitHub's ahead by the workflow's pins alone ("Pin the example and the
#     documents to <version>" commits), the hub's master is fast-forwarded to it, never forced; GitHub's holding any
#     other commit the hub lacks, or the two diverged, refused, the hub and GitHub to be reconciled by hand (the
#     commits named) before any landing.
# The landing then commits on the hub's master and pushes it to the hub and to GitHub, each a plain push: one that
# GitHub refuses (a release started in between) is followed by this script again, never by a force. Prints what it
# did; exits 0 when the landing may go on, 1 when it may not.
set -uo pipefail
hub=${1:-origin} github=${2:-github}
repo=Carrot-Inc/teq
fail() { echo "before-landing: refused: $*" >&2; exit 1; }
say() { echo "before-landing: $*"; }
command -v gh > /dev/null || fail "no gh, which reads the release workflow's runs"

active=$(timeout 60 gh run list --repo "$repo" --workflow release.yml --limit 50 --json databaseId,status \
  --jq '.[] | select(.status != "completed") | "\(.databaseId) \(.status)"') || fail "the release workflow's runs could not be read"
[ -z "$active" ] || fail "a release runs ($(tr '\n' ';' <<< "$active")): no landing until it ends"

timeout 300 git fetch -q "$github" "+refs/heads/master:refs/remotes/$github/master" || fail "GitHub's master could not be fetched ($github)"
timeout 300 git fetch -q "$hub" "+refs/heads/master:refs/remotes/$hub/master" || fail "the hub's master could not be fetched ($hub)"
g=$(git rev-parse "refs/remotes/$github/master") h=$(git rev-parse "refs/remotes/$hub/master")
if [ "$g" = "$h" ]; then
  say "GitHub's master is the hub's, ${h:0:12}"
elif git merge-base --is-ancestor "$g" "$h"; then
  say "the hub's master ${h:0:12} is ahead of GitHub's ${g:0:12}: the landing pushes both"
elif git merge-base --is-ancestor "$h" "$g"; then
  others=$(git log --format='%h %s' "$h..$g" | grep -v -E '^[0-9a-f]+ Pin the example and the documents to [0-9]+\.[0-9]+\.[0-9]+$')
  [ -z "$others" ] || fail "GitHub's master holds commits the hub lacks that are no release's pin: $(tr '\n' ';' <<< "$others")"
  timeout 300 git push --no-follow-tags "$hub" "$g:refs/heads/master" || fail "the hub's master could not be fast-forwarded to GitHub's ${g:0:12}"
  say "the hub's master fast-forwarded to GitHub's ${g:0:12}: $(git log --format='%s' "$h..$g" | tr '\n' ';')"
else
  fail "the hub's master ${h:0:12} and GitHub's ${g:0:12} diverged (since $(git merge-base "$g" "$h" | cut -c1-12)): GitHub's $(git log --format='%h %s' "$h..$g" | tr '\n' ';'); the hub's $(git log --format='%h %s' "$g..$h" | tr '\n' ';')"
fi
