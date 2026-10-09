#!/bin/bash
# bench/github-release.sh <version> [publish | abandon | check] | --dry-run: the GitHub release of teq <version>,
# the publication of its binaries (docs/DEVELOPING.md, "Releases"). The ship's three verbs:
#
#   <version>          the draft, once the five binaries are built, staged and qualified: GitHub's master pushed,
#                      the release v<version> made a draft (or the draft of an earlier run resumed), every asset
#                      uploaded to it, downloaded back and checked. Run again, it resumes a failed upload.
#   <version> publish  after the Central step: the draft, whole, published, which makes the tag on the release's
#                      commit, and every asset read back from its URL. Run again, it reads back again.
#   <version> abandon  when the ship gives the release up: every draft of v<version> deleted, each read again by
#                      its id just before its deletion; a published release is never.
#
# And for a person: `<version> check`, what the draft would refuse but for the binaries, before a build, writing
# nothing; `--dry-run`, the draft and its abandonment on the scratch version 0.0.0-dry (below).
#
# The interface:
#   release    v<version> at github.com/Carrot-Inc/teq, titled `teq <version>`, the tag on the release's commit,
#              the notes CHANGELOG.md's section for the version (bench/changelog.sh).
#   assets     teq-<version>-<classifier> for osx-aarch_64, osx-x86_64, linux-x86_64 and linux-aarch_64, teq-<version>-windows-x86_64.exe and
#              windows-x86_64, each the bytes of integrations/sbt/binary/binaries/<classifier>/teq as the ship
#              staged and qualified them, uncompressed; SHA256SUMS over the five in sha256sum's format, a line
#              `<sha256>  <asset name>` each; and teq-<version>-binaries.txt, the binaries' manifest written from
#              the same bytes: a line `teq <version> <commit>`, then a line `<classifier> <asset name> <sha256>
#              <sha1> <size in bytes>` each. The five in that order everywhere; digests in lowercase hexadecimal.
#              And teq-<version>-profiles.tar, the profiles the binaries were guided by, which a later release
#              takes up (bench/ship-profiles.sh): the stage's out/ship/profiles/teq-<version>-profiles.tar, whole,
#              its header naming the version and the release's commit, each profile the one, and under the
#              metadata, that the guided binaries' manifests record. And NOTICE and LICENSE, the release's commit's
#              own: the notices of what the binaries are built from, and the license.
#   URLs       https://github.com/Carrot-Inc/teq/releases/download/v<version>/<asset name>, which redirect to
#              GitHub's object store: a client follows the redirect (curl --location).
#   exit code  0 done (the draft whole; published and read back; the drafts deleted; check: nothing refused;
#              --dry-run: it passed); 1 refused, nothing written to GitHub by this run; 2 the usage; 3 failed once
#              this run wrote something (the push, the draft, an upload, the publication, a deletion): the same
#              command, run again, resumes; for --dry-run, 3 once it made its scratch draft, which it deletes on
#              its way out (a dry run that passed but for a push the release would refuse among them).
#
# The release's commit is the head, the commit that bumped the version to <version> (bench/release.sh), on the
# hub's master (the remote origin). The binaries are the five or none: a classifier held back or not staged
# refuses the draft before anything is written. Each is its manifest's (the digest), built from the release's
# commit as <version>, staged by the one invocation of stage.sh that staged the others, and published by
# bench/ship-qualified.txt as it stands (bench/ship-manifest.sh's manifest_qualified); stage.sh's own checks of
# the bytes are the ship's. The draft and the publication, in order:
#
# 1. The push: GitHub's master moves to the hub's master, which holds the release's commit, by a fast-forward
#    and without a tag (--no-follow-tags), the hub read again just before the push (a master that moved since
#    is refused). A GitHub master the hub's master does not hold is refused; one that is the hub's already is
#    left as it is.
# 2. The draft, by the gh CLI: made (its id GitHub's answer) or found, edited only where its title, notes or
#    flags differ, its assets uploaded where GitHub's size or digest is not the file's, then downloaded back,
#    compared, and SHA256SUMS and the manifest checked against the five. The notes are the section as the
#    release's commit has it, a bullet's lines joined (GitHub breaks a line where the file does). Refused: two
#    releases of the tag, the tag on another commit, an asset the release should not hold. A draft made while
#    another run made one is deleted by the run that made it, which fails; runs from one clone take turns
#    (flock on the clone's github-release.lock). A published release is left as it is, its assets checked.
# 3. The publication: the draft checked whole as in 2 without a write (else refused: the draft verb resumes
#    it), GitHub's master and the tag looked at again, then the release listed again just before the
#    publication (still a draft, on the release's commit, its assets the files), and published with its target
#    the release's commit, the latest release when no published one has a higher version; then each asset read
#    back from its URL, following the redirect, compared with the file, and SHA256SUMS and the manifest checked
#    against the five read back. Across clones nothing takes turns: these last reads narrow the window between
#    a read and a write that another clone's run could use, and GitHub's API offers no conditional write to
#    close it, so one ship writes a release at a time.
#
# The token, the maintainer's, is read from ~/.config/teq/github-token (this user's file, mode 600) by each command
# that needs it: GH_TOKEN for gh, and over https the password a credential helper hands git, in place of the
# configured helpers so that none stores it; never in this shell's variables, an argument, a trace or the output.
# Git's own tracing (GIT_TRACE*, GIT_CURL_VERBOSE, trace2) is off for every git call over the network, toward
# GitHub and toward origin alike (which in a GitHub Actions run is GitHub, origin's own authentication kept), and
# its redaction forced; gh's debugging (GH_DEBUG, DEBUG) off; whatever the caller's environment says.
# A remote named `github` in the checkout (https, or ssh with the machine's key) is pushed to in place of
# https://github.com/Carrot-Inc/teq.git. TEQ_GITHUB_DOWNLOADS replaces the URLs' part before the tag.
#
# In a GitHub Actions run of the repository's workflow (GITHUB_ACTIONS set) GitHub's master is not pushed: the
# run is on it, and the release's commit must be on it already. The token is then GH_TOKEN's when the file is
# absent, the file preferred when present; outside such a run the file alone. The checkout is a whole one
# (actions/checkout's fetch-depth: 0), since a release is told from its parent.
#
# --dry-run: the draft against the scratch version 0.0.0-dry, whose tag is never made: six small assets and the
# head's NOTICE and LICENSE on GitHub's master, the notes the checkout's newest section under a line that says so;
# the upload stopped after the first asset and resumed by a second pass on the same draft, the draft checked the
# one release of its tag and its assets downloaded back and checked; then abandoned and its absence checked. The
# push is decided and
# tried as git push --dry-run; one the release would refuse fails the dry run once the draft is gone.
#
# Needs git, gh, curl, flock and python3; every command is bounded.
set -uo pipefail
set +x
cd "$(dirname "$0")/.." || exit 1
. bench/ship-manifest.sh
. bench/changelog.sh
usage() { echo "usage: $0 <version> [publish | abandon | check] | --dry-run"; exit 2; }
if [ $# -eq 1 ] && [ "$1" = --dry-run ]; then
  verb=dry version=0.0.0-dry
elif [ $# -ge 1 ] && [ $# -le 2 ] && [[ $1 =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]]; then
  version=$1
  case "${2-}" in
    "") verb=draft ;;
    publish | abandon | check) verb=$2 ;;
    *) usage ;;
  esac
else
  usage
fi
github_repo=Carrot-Inc/teq
github_url=https://github.com/$github_repo.git
github_downloads=${TEQ_GITHUB_DOWNLOADS:-https://github.com/$github_repo/releases/download}
token=$HOME/.config/teq/github-token
binaries=integrations/sbt/binary/binaries
profiles=out/ship/profiles/teq-$version-profiles.tar
tag=v$version
written=
say() { echo "github: $*"; }
# fail: refused (1) before this run wrote anything to GitHub, failed (3) once it did.
fail() {
  say "$*"
  [ -z "$written" ] || [ "$verb" = dry ] || { say "this run wrote to GitHub already: the same command, run again, resumes"; exit 3; }
  [ -z "$written" ] || { say "this dry run wrote its scratch draft to GitHub, deleted on its way out"; exit 3; }
  exit 1
}

for tool in git gh curl flock python3 timeout sha1sum; do command -v $tool > /dev/null || fail "no $tool on the PATH"; done
[ "$(git rev-parse --is-shallow-repository)" = false ] || fail "the checkout is shallow: whether GitHub's master is on the hub's cannot be told"
lock=$(git rev-parse --git-common-dir)/github-release.lock
exec 9> "$lock" || fail "cannot open $lock"
flock -n 9 || fail "another bench/github-release.sh runs from this clone ($lock)"
actions=${GITHUB_ACTIONS:+1}
if [ -f "$token" ]; then
  [ -O "$token" ] || fail "$token is not this user's"
  bits=$(stat -c %a "$token" 2> /dev/null || stat -f %Lp "$token")
  [ "$bits" = 600 ] || fail "$token has mode $bits, not 600"
  [ "$(tr -d '[:space:]' < "$token" | wc -c)" -gt 0 ] || fail "$token is empty"
elif [ -n "$actions" ] && [ -n "${GH_TOKEN:-}" ]; then
  # The workflow's token, from the environment.
  token=
else
  fail "no $token, the maintainer's GitHub token (mode 600)$([ -n "$actions" ] && echo ", and no GH_TOKEN in this GitHub Actions run")"
fi

# gh_ <bound> <args...>: gh with the token, never prompting, never debugging.
gh_() {
  local bound=$1
  shift
  GH_TOKEN=$(if [ -n "$token" ]; then tr -d '[:space:]' < "$token"; else printf '%s' "$GH_TOKEN"; fi) GH_PROMPT_DISABLED=1 GH_NO_UPDATE_NOTIFIER=1 GH_SPINNER_DISABLED=1 NO_COLOR=1 \
    env -u GH_DEBUG -u DEBUG timeout "$bound" gh "$@" < /dev/null
}
# gitnet <bound> <args...>: git over the network, every such call of this script's (origin's reads and fetch
# among them), never prompting and nothing traced: git's tracing variables unset, its redaction forced, trace2
# off, a helper's shell without the caller's tracing; the remote's own authentication as configured.
gitnet() {
  local bound=$1 v unset=()
  shift
  for v in $(compgen -e); do
    case $v in GIT_TRACE* | GIT_CURL_VERBOSE | BASH_ENV | ENV | SHELLOPTS) unset+=(-u "$v") ;; esac
  done
  env "${unset[@]}" GIT_TRACE_REDACT=1 GIT_TERMINAL_PROMPT=0 timeout "$bound" git \
    -c trace2.normalTarget=0 -c trace2.eventTarget=0 -c trace2.perfTarget=0 "$@" < /dev/null
}
# git_ <bound> <args...>: git toward GitHub by gitnet, the token (the file's, else GH_TOKEN's) the password over
# https, by a helper in place of the configured ones.
git_() {
  local bound=$1
  shift
  TEQ_GITHUB_TOKEN_FILE="$token" gitnet "$bound" -c credential.helper= \
    -c 'credential.helper=!f() { test "$1" = get || return 0; echo username=x-access-token; if [ -n "$TEQ_GITHUB_TOKEN_FILE" ]; then printf "password=%s\n" "$(tr -d "[:space:]" < "$TEQ_GITHUB_TOKEN_FILE")"; else printf "password=%s\n" "$GH_TOKEN"; fi; }; f' \
    "$@"
}

if url=$(git remote get-url github 2> /dev/null); then
  # Never printed: a URL may carry credentials.
  [[ $url =~ ^(https://github\.com/|git@github\.com:|ssh://git@github\.com/)Carrot-Inc/teq(\.git)?$ ]] ||
    fail "the remote github names another repository than $github_repo"
  github_url=$url
fi
gh_ 60 api "repos/$github_repo" > /dev/null || fail "GitHub does not take the token of ${token:-GH_TOKEN} (above)"
work=$(mktemp -d "${TMPDIR:-/tmp}/github-release.XXXXXX") || exit 1
dry_started=
cleanup() {
  # The dry run's draft goes whatever happened.
  if [ -n "$dry_started" ]; then
    if (abandon_drafts > /dev/null 2>&1); then say "the dry run's draft is deleted"; else say "the dry run's draft is left: bench/github-release.sh $version abandon deletes it"; fi
  fi
  rm -rf "$work"
}
trap cleanup EXIT

# github_tag: the commit GitHub's tag names, peeled, or nothing.
github_tag() {
  local refs
  refs=$(git_ 60 ls-remote "$github_url" "refs/tags/$tag" "refs/tags/$tag^{}") || return 1
  awk -v t="refs/tags/$tag" '$2 == t || $2 == t "^{}" { sha = $1 } END { print sha }' <<< "$refs"
}
# releases: the releases of the tag on GitHub, drafts among them, a line `<id> <draft|published> <target>
# <same|differs>` each (same: the title, the notes and the pre-release flag this run gives), with each one's
# assets in $work/assets.<id> (`<name> <size> <state> <digest>`, tab-separated), and the tags of the published
# releases but pre-releases and the tag's in $work/published.
releases() {
  gh_ 120 api --paginate "repos/$github_repo/releases?per_page=100" > "$work/releases.json" || return 1
  python3 - "$tag" "$work" "teq $version" "${prerelease:-false}" << 'EOF'
import json, os, sys
tag, work, title, prerelease = sys.argv[1:]
text, at, every = open(work + "/releases.json").read(), 0, []
while True:
    while at < len(text) and text[at].isspace():
        at += 1
    if at == len(text):
        break
    page, at = json.JSONDecoder().raw_decode(text, at)
    every += page if isinstance(page, list) else [page]
notes = open(work + "/notes.md").read() if os.path.exists(work + "/notes.md") else None
plain = lambda s: "\n".join(l.rstrip() for l in (s or "").replace("\r\n", "\n").strip().split("\n"))
published = []
for r in every:
    if r.get("tag_name") == tag:
        same = notes is not None and r.get("name") == title and r["prerelease"] == (prerelease == "true") and plain(r.get("body")) == plain(notes)
        print(r["id"], "draft" if r["draft"] else "published", r.get("target_commitish") or "-", "same" if same else "differs")
        with open("%s/assets.%s" % (work, r["id"]), "w") as out:
            for a in r.get("assets") or []:
                out.write("\t".join([a["name"], str(a["size"]), a.get("state") or "-", a.get("digest") or "-"]) + "\n")
    elif not r["draft"] and not r["prerelease"]:
        published.append(r["tag_name"] + "\n")
open(work + "/published", "w").write("".join(published))
EOF
}
# one_release [made]: the one release of the tag into id, state, target and same, nothing without one (asked
# again a few seconds apart after `made`); fails on two.
one_release() {
  local found try
  for try in 1 2 3; do
    [ "$try" -eq 1 ] || sleep 5
    found=$(releases) || fail "GitHub's releases could not be read"
    [ "$(grep -c . <<< "$found")" -le 1 ] ||
      fail "GitHub has $(grep -c . <<< "$found") releases of $tag ($(awk '{ print "id " $1 ", " $2 }' <<< "$found" | tr '\n' ';')): which one stays is the maintainer's call (bench/github-release.sh $version abandon deletes drafts)"
    [ -z "$found" ] && [ "${1:-}" = made ] && continue
    break
  done
  id= state= target= same=
  read -r id state target same <<< "$found"
}
# abandon_drafts: every draft of the tag deleted, by its id; refused with a published release of the tag.
abandon_drafts() {
  local found rid rest draft gone=0
  found=$(releases) || fail "GitHub's releases could not be read"
  ! grep -q ' published ' <<< "$found" || fail "$tag is published ($(awk '$2 == "published" { print "id " $1 }' <<< "$found")): a published release is never deleted"
  while read -r rid rest; do
    [ -n "$rid" ] || continue
    # Read again just before the deletion: another clone's run may have published it since the listing.
    draft=$(gh_ 60 api "repos/$github_repo/releases/$rid" --jq .draft) || fail "the release $tag (id $rid) could not be read again (above)"
    [ "$draft" = true ] || fail "the release $tag (id $rid) is published now: a published release is never deleted"
    written=1
    gh_ 120 api -X DELETE "repos/$github_repo/releases/$rid" > /dev/null || fail "the draft $tag (id $rid) could not be deleted (above)"
    gone=$((gone + 1))
    say "deleted the draft $tag (id $rid)"
  done <<< "$found"
  # The listing's status apart from its output: a listing that fails is no absence.
  found=$(releases) || fail "GitHub's releases could not be read after the deletion"
  [ -z "$found" ] || fail "GitHub still has a release of $tag after the deletion"
  [ "$gone" -gt 0 ] || say "no draft of $tag to delete"
}

if [ "$verb" = abandon ]; then
  abandon_drafts
  exit 0
fi

commit=$(git rev-parse HEAD) || exit 1
if [ "$verb" != dry ]; then
  current=$(release_version .) || fail "the checkout names no one release"
  [ "$current" = "$version" ] || fail "the checkout's release is $current, not $version"
  before=$(git show "$commit^:Cargo.toml" 2> /dev/null | sed -n 's/^version *= *"\([^"]*\)".*/\1/p' | head -1)
  [ "$before" != "$version" ] ||
    fail "the head ${commit:0:12} is not the commit that bumped the version to $version, which its parent names already: the tag goes on the release's commit, which the ship builds"
fi
prerelease=false
[[ $version == *-* ]] && prerelease=true
hub=$(gitnet 60 ls-remote origin refs/heads/master | awk '$2 == "refs/heads/master" { print $1 }')
[[ $hub =~ ^[0-9a-f]{40}$ ]] || fail "the hub's master (the remote origin) could not be read"
git cat-file -e "$hub^{commit}" 2> /dev/null || gitnet 300 fetch --quiet --no-tags origin master
git cat-file -e "$hub^{commit}" 2> /dev/null || fail "the hub's master ${hub:0:12} could not be fetched"
[ "$verb" = dry ] || git merge-base --is-ancestor "$commit" "$hub" ||
  fail "the release's commit ${commit:0:12} is not on the hub's master (${hub:0:12}): a release is a landed commit of master"

# The notes, from the release's commit.
git show "$commit:CHANGELOG.md" > "$work/CHANGELOG.md" 2> /dev/null || fail "${commit:0:12} has no CHANGELOG.md"
if [ "$verb" = dry ]; then
  newest=$(sed -n 's/^## \([^ ]*\) .*/\1/p' "$work/CHANGELOG.md" | head -1)
  section=$(changelog_notes "$work/CHANGELOG.md" "$newest") || fail "CHANGELOG.md's newest section is refused (above)"
  section=$(printf 'A dry run of the release step, deleted when it is done, with the notes of %s.\n\n%s' "$newest" "$section")
else
  section=$(changelog_notes "$work/CHANGELOG.md" "$version") || fail "the notes are refused (above): CHANGELOG.md's section for $version as ${commit:0:12} has it"
fi
printf '%s\n' "$section" | changelog_join > "$work/notes.md"
say "the notes of $tag: CHANGELOG.md's section, $(grep -c '^- ' "$work/notes.md") bullets"

# The assets: the five binaries, each checked against its manifest and copied under its asset name, then
# SHA256SUMS and the manifest written from them.
manifest=teq-$version-binaries.txt
# asset_name <classifier>: the binary's asset, .exe for the Windows one alone.
asset_name() { case $1 in windows-*) echo "teq-$version-$1.exe" ;; *) echo "teq-$version-$1" ;; esac; }
# Every file of the release, as bench/ship-release.sh's release_assets names them (whose count bounds this script's
# verbs, release_draft_bound and release_publish_bound).
names=$(release_assets "$version" | tr '\n' ' ')
names=${names% }
# sums <dir> <out>: SHA256SUMS and the manifest of the five assets in <dir>, written into <out>.
sums() {
  local c a
  for c in $release_classifiers; do
    a=$(asset_name $c)
    echo "$(manifest_sha256 "$1/$a")  $a"
  done > "$2/SHA256SUMS" || return 1
  {
    echo "teq $version $commit"
    for c in $release_classifiers; do
      a=$(asset_name $c)
      echo "$c $a $(manifest_sha256 "$1/$a") $(sha1sum < "$1/$a" | cut -c1-40) $(wc -c < "$1/$a" | tr -d ' ')"
    done
  } > "$2/$manifest"
}
# sums_agree <dir>: SHA256SUMS and the manifest in <dir> are those of the five assets in it.
sums_agree() {
  rm -rf "$work/expect" && mkdir "$work/expect" && sums "$1" "$work/expect" || return 1
  cmp -s "$work/expect/SHA256SUMS" "$1/SHA256SUMS" || { say "$1's SHA256SUMS is not that of its five assets"; return 1; }
  cmp -s "$work/expect/$manifest" "$1/$manifest" || { say "$1's $manifest is not that of its five assets"; return 1; }
}
assets=$work/assets
mkdir -p "$assets"
if [ "$verb" = draft ] || [ "$verb" = publish ]; then
  missing=
  for c in $release_classifiers; do [ -d "$binaries/$c" ] || missing="$missing $c"; done
  [ -z "$missing" ] || fail "nothing staged for$missing (held back, or not built): the release is the five binaries or none"
  for d in "$binaries"/*/; do
    c=$(basename "$d")
    [[ " $release_classifiers " == *" $c "* ]] || fail "$binaries/$c is no classifier of the release's"
  done
  runs=
  for c in $release_classifiers; do
    b=$binaries/$c/teq m=$binaries/$c/teq.manifest a=$assets/$(asset_name $c)
    [ "$(find "$binaries/$c" -mindepth 1 -maxdepth 1 -printf '%f\n' | LC_ALL=C sort | tr '\n' ' ')" = "teq teq.manifest " ] ||
      fail "$binaries/$c holds $(find "$binaries/$c" -mindepth 1 -maxdepth 1 -printf '%f ' ), not its binary and manifest alone"
    timeout 300 cp "$b" "$a" || fail "$b could not be copied"
    [ "$(manifest_get "$m" binary)" = "$(manifest_sha256 "$a")" ] || fail "$b is not the binary its manifest records"
    [ "$(manifest_get "$m" commit)" = "$commit" ] || fail "$b is built from $(manifest_get "$m" commit | cut -c1-12), not the release's commit ${commit:0:12}"
    [[ $(manifest_get "$m" version) == "teq $version "* ]] || fail "$b is '$(manifest_get "$m" version)', not the release $version's binary"
    run=$(manifest_get "$m" run)
    [ -n "$run" ] || fail "$m records no staging invocation: stage.sh stages"
    runs="$runs$run"$'\n'
    manifest_qualified "$m" bench/ship-qualified.txt "$c" || fail "$b is not published by bench/ship-qualified.txt as it stands (above)"
  done
  [ "$(sort -u <<< "${runs%$'\n'}" | wc -l)" -eq 1 ] || fail "the binaries were staged by different invocations: $(sort -u <<< "$runs" | tr '\n' ' ')"
  # The profiles that guided them, whole, of this release.
  [ -f "$profiles" ] || fail "no $profiles, the profiles the binaries were guided by (bench/ship.sh's stage writes it)"
  why=$(bench/ship-profiles.sh check "$profiles" 2>&1) || fail "$profiles is refused: ${why#profiles: }"
  header=$(tar -xOf "$profiles" profiles.txt)
  [ "$(head -1 <<< "$header")" = "profiles $version $commit" ] || fail "$profiles is not $version's, of ${commit:0:12}"
  # Its profiles the ones each guided binary's manifest names, under the metadata its training records: the
  # aarch64 trainer's for the arm64 builds, the x86-64 one's for the others (the Windows binary is plain).
  for c in $release_classifiers; do
    case $c in osx-aarch_64 | linux-aarch_64) arch=aarch64 ;; osx-x86_64 | linux-x86_64) arch=x86_64 ;; *) continue ;; esac
    m=$binaries/$c/teq.manifest
    [ "$(manifest_get "$m" "training profile")" = "$(sed -n "s/^profile $arch \([0-9a-f]*\) .*/\1/p" <<< "$header")" ] &&
      [ "$(manifest_get "$m" "training metadata")" = "$(sed -n "s/^metadata $arch //p" <<< "$header")" ] ||
      fail "$profiles does not hold the $arch profile that guided $c ($(manifest_get "$m" "training profile" | cut -c1-12), the asset's $(sed -n "s/^profile $arch \([0-9a-f]\{12\}\).*/\1/p" <<< "$header"))"
  done
  timeout 300 cp "$profiles" "$assets/teq-$version-profiles.tar" || fail "$profiles could not be copied"
elif [ "$verb" = dry ]; then
  for c in $release_classifiers; do printf 'teq %s, %s: a dry run of the release step\n' "$version" "$c" > "$assets/$(asset_name $c)"; done
  printf 'teq %s: the profiles of a dry run of the release step\n' "$version" > "$assets/teq-$version-profiles.tar"
fi
# The notices and the license, as the release's commit has them.
for f in NOTICE LICENSE; do
  git show "$commit:$f" > "$assets/$f" 2> /dev/null && [ -s "$assets/$f" ] ||
    fail "${commit:0:12} has no $f, which the release carries beside its binaries"
done
[ "$verb" = check ] || sums "$assets" "$assets" || exit 1

# strays: fails on an asset of the release that the release should not hold.
strays() {
  local name rest
  while IFS=$'\t' read -r name rest; do
    [[ " $names " == *" $name "* ]] ||
      fail "the release $tag holds $name, which it should not: gh release delete-asset $tag $name --repo $github_repo removes it"
  done < "$work/assets.$id"
}
# whole: whether GitHub holds every asset of the release as the file, by its size and digest, naming the first
# that it does not.
whole() {
  local name line
  for name in $names; do
    line=$(awk -F'\t' -v n="$name" '$1 == n { print $2 " " $3 " " $4 }' "$work/assets.$id")
    [ "$line" = "$(wc -c < "$assets/$name" | tr -d ' ') uploaded sha256:$(manifest_sha256 "$assets/$name")" ] || { echo "$name"; return 1; }
  done
}
# back: the assets downloaded back from the release by gh, each the file, SHA256SUMS and the manifest theirs.
back() {
  local name
  rm -rf "$work/back" && mkdir "$work/back" || exit 1
  gh_ 1800 release download "$tag" --repo "$github_repo" --dir "$work/back" > /dev/null || fail "the assets of $tag could not be downloaded back (above)"
  [ "$(cd "$work/back" && ls | LC_ALL=C sort | tr '\n' ' ')" = "$(tr ' ' '\n' <<< "$names" | LC_ALL=C sort | tr '\n' ' ')" ] ||
    fail "the assets of $tag downloaded back are $(ls "$work/back" | tr '\n' ' '), not $names"
  for name in $names; do cmp -s "$work/back/$name" "$assets/$name" || fail "$name downloaded back from $tag is not the file uploaded"; done
  sums_agree "$work/back" || fail "the assets of $tag downloaded back disagree with their sums (above)"
  say "the assets of the $state release $tag downloaded back: each the file, SHA256SUMS and $manifest theirs"
}

# What GitHub has, read before anything is written: its master, the tag, the releases of the tag.
github_master=$(git_ 60 ls-remote "$github_url" refs/heads/master) || fail "GitHub's master could not be read"
github_master=$(awk '$2 == "refs/heads/master" { print $1 }' <<< "$github_master")
github_short=${github_master:0:12}
github_short=${github_short:-nothing}
push= refused=
if [ -n "$actions" ]; then
  # The run is on GitHub's master: nothing to push, the release's commit there already.
  [ "$verb" = dry ] || { [ -n "$github_master" ] && git cat-file -e "$github_master^{commit}" 2> /dev/null && git merge-base --is-ancestor "$commit" "$github_master"; } ||
    fail "GitHub's master $github_short does not hold the release's commit ${commit:0:12}, and a GitHub Actions run pushes nothing"
  say "a GitHub Actions run: GitHub's master $github_short is not pushed"
elif [ "$github_master" = "$hub" ]; then
  say "GitHub's master is the hub's, ${hub:0:12}"
elif [ -z "$github_master" ] || { git cat-file -e "$github_master^{commit}" 2> /dev/null && git merge-base --is-ancestor "$github_master" "$hub"; }; then
  push=1
else
  refused="GitHub's master $github_short is not on the hub's master ${hub:0:12}: GitHub has commits the hub lacks, which a push would rewrite; nothing is pushed"
  [ "$verb" = dry ] || fail "$refused"
  say "the push would be refused: $refused"
fi
tag_at=$(github_tag) || fail "GitHub's tags could not be read"
if [ "$verb" = dry ]; then
  [ -z "$tag_at" ] || fail "GitHub has a tag $tag, on ${tag_at:0:12}: the dry run never makes one, and leaves it to the maintainer"
  target_commit=${github_master:-$hub}
else
  [ -z "$tag_at" ] || [ "$tag_at" = "$commit" ] || fail "GitHub's tag $tag is on ${tag_at:0:12}, not the release's commit ${commit:0:12}"
  target_commit=$commit
fi
one_release
[ "$verb" != dry ] || [ -z "$id" ] || [ "$state" = draft ] || fail "GitHub has a published release $tag (id $id): the dry run leaves it to the maintainer"
[ "$verb" = dry ] || [ -z "$id" ] || strays

# push_master: GitHub's master moved to the hub's when it is behind; in check and the dry run, whether GitHub
# takes it.
push_master() {
  local now
  [ -z "$actions" ] || return 0
  if [ "$verb" = check ] || [ "$verb" = dry ]; then
    [ -z "$refused" ] || return 0
    git_ 300 push --dry-run --no-follow-tags --porcelain "$github_url" "$hub:refs/heads/master" > "$work/push" 2>&1 ||
      fail "GitHub refuses the push of the hub's master (git push --dry-run): $(tr '\n' ' ' < "$work/push")"
    say "the push of the hub's master ${hub:0:12} onto GitHub's $github_short: $([ -n "$push" ] && echo "a fast-forward" || echo "nothing to push"), which GitHub takes (git push --dry-run)"
  elif [ -n "$push" ]; then
    # The hub read again: a master that moved since this run read it is not pushed stale.
    now=$(gitnet 60 ls-remote origin refs/heads/master | awk '$2 == "refs/heads/master" { print $1 }')
    [ "$now" = "$hub" ] || fail "the hub's master is $([ -n "$now" ] && echo "${now:0:12}" || echo nothing) now, not the ${hub:0:12} this run read: the same command, run again, takes it"
    written=1
    git_ 300 push --no-follow-tags --porcelain "$github_url" "$hub:refs/heads/master" > "$work/push" 2>&1 ||
      fail "the push of the hub's master to GitHub failed: $(tr '\n' ' ' < "$work/push")"
    now=$(git_ 60 ls-remote "$github_url" refs/heads/master | awk '$2 == "refs/heads/master" { print $1 }')
    [ "$now" = "$hub" ] || fail "GitHub's master is ${now:-nothing} after the push, not the hub's ${hub:0:12}"
    say "pushed the hub's master ${hub:0:12} to GitHub, from $github_short"
    push= github_short=${hub:0:12}
  fi
}

# draft [<uploads>]: the draft made or edited where it differs, its assets uploaded where they differ (stopping
# after that many uploads, which returns 4, for the dry run), then checked and downloaded back. A published
# release is checked alone.
draft() {
  local stop=${1:-} uploaded=0 name made retarget=()
  if [ "$state" = published ]; then
    name=$(whole) || fail "$tag is published, and GitHub's $name is not the file: a published release is never changed"
    back
    return 0
  fi
  if [ -z "$id" ]; then
    written=1
    made=$(gh_ 120 api -X POST "repos/$github_repo/releases" -f tag_name="$tag" -f target_commitish="$target_commit" \
      -f name="teq $version" -F draft=true -F prerelease="$prerelease" -F body=@"$work/notes.md" --jq .id) ||
      fail "the draft of $tag could not be made (above)"
    [[ $made =~ ^[0-9]+$ ]] || fail "GitHub answered the draft of $tag with no id: '$made'"
    releases > "$work/after" || fail "GitHub's releases could not be read"
    if [ "$(grep -c . "$work/after")" -gt 1 ]; then
      gh_ 120 api -X DELETE "repos/$github_repo/releases/$made" > /dev/null ||
        fail "another run made a release of $tag beside this run's draft (id $made), which could not be deleted: one is deleted by hand"
      fail "another run made a release of $tag beside this run's draft (id $made), deleted; the same command, run again, resumes on the other"
    fi
    one_release made
    [ "$id" = "$made" ] || fail "the draft $made of $tag is not GitHub's one release of the tag (${id:-none})"
    say "made the draft of $tag (id $id), its target ${target_commit:0:12}"
  elif [ "$same" != same ] || [ "$target" != "$target_commit" ]; then
    [ "$target" != "$target_commit" ] && retarget=(--target "$target_commit")
    written=1
    gh_ 120 release edit "$tag" --repo "$github_repo" --title "teq $version" --notes-file "$work/notes.md" --prerelease="$prerelease" \
      "${retarget[@]}" > /dev/null || fail "the draft $tag (id $id) could not be edited (above)"
    say "edited the draft $tag (id $id): its title, notes, flags and target"
    one_release
  else
    say "the draft $tag (id $id) has its title, notes, flags and target already"
  fi
  [ "$state" = draft ] || fail "$tag is $state, not a draft"
  strays
  for name in $names; do
    if [ "$(awk -F'\t' -v n="$name" '$1 == n { print $2 " " $3 " " $4 }' "$work/assets.$id")" = "$(wc -c < "$assets/$name" | tr -d ' ') uploaded sha256:$(manifest_sha256 "$assets/$name")" ]; then
      say "$name is on GitHub already"
      continue
    fi
    [ -z "$stop" ] || [ "$uploaded" -lt "$stop" ] || { say "stopped after $uploaded upload(s), as a failed upload stops"; return 4; }
    written=1
    gh_ 900 release upload "$tag" --repo "$github_repo" --clobber "$assets/$name" > /dev/null || fail "the upload of $name failed (above)"
    uploaded=$((uploaded + 1))
    say "uploaded $name"
  done
  one_release
  strays
  name=$(whole) || fail "GitHub does not hold $name as the file after the upload"
  back
}

case $verb in
  check)
    push_master
    say "the release $tag: $([ -z "$id" ] && echo "none on GitHub yet" || echo "the $state release id $id"); the tag: ${tag_at:-not made yet}"
    say "checked: the token, the notes, the release's commit ${commit:0:12} on the hub's master, the push, the tag, the releases of $tag and their assets"
    ;;
  draft)
    push_master
    draft
    [ "$state" = published ] && say "$tag is published already, its assets the files" ||
      say "the draft $tag (id $id) holds every asset: bench/github-release.sh $version publish publishes it"
    ;;
  publish)
    [ -n "$id" ] || fail "GitHub has no release $tag: bench/github-release.sh $version makes its draft"
    if [ "$state" = draft ]; then
      name=$(whole) || fail "the draft $tag is not whole ($name is not the file): bench/github-release.sh $version resumes it"
      [ "$same" = same ] && [ "$target" = "$commit" ] || fail "the draft $tag has another title, notes, flags or target than this run gives: bench/github-release.sh $version edits it"
      back
      push_master
      # The tag looked at again: a tag pushed since would take the release to its commit.
      tag_at=$(github_tag) || fail "GitHub's tags could not be read"
      [ -z "$tag_at" ] || [ "$tag_at" = "$commit" ] || fail "GitHub's tag $tag is on ${tag_at:0:12} now, not the release's commit ${commit:0:12}: the draft stays a draft"
      # Listed again just before the publication: another writer may have replaced an asset or retargeted the
      # draft since the checks above.
      one_release
      [ "$state" = draft ] || fail "$tag is ${state:-gone} now, not the draft this run checked"
      [ "$target" = "$commit" ] || fail "the draft $tag targets ${target:0:12} now, not the release's commit ${commit:0:12}"
      strays
      name=$(whole) || fail "the draft $tag is not whole now ($name is not the file): another writer is at it"
      latest=false
      [ "$prerelease" = false ] && [ "$( (cat "$work/published"; echo "$tag") | sort -V | tail -1)" = "$tag" ] && latest=true
      written=1
      gh_ 120 release edit "$tag" --repo "$github_repo" --draft=false --target "$commit" --latest="$latest" > /dev/null ||
        fail "the draft $tag could not be published (above)"
      one_release
      [ "$state" = published ] || fail "$tag is ${state:-gone} after its publication"
      say "published $tag$([ "$latest" = true ] && echo ", the latest release")"
    fi
    for try in 1 2 3 4 5 6; do
      tag_at=$(github_tag) || tag_at=
      [ -z "$tag_at" ] || break
      sleep 5
    done
    [ "$tag_at" = "$commit" ] ||
      fail "GitHub's tag $tag is on ${tag_at:-nothing}, not ${commit:0:12}: the release is on the wrong commit; gh release edit $tag --repo $github_repo --draft takes it back"
    mkdir -p "$work/public" || exit 1
    for name in $names; do
      ok=
      for try in 1 2 3 4 5; do
        [ "$try" -eq 1 ] || sleep 10
        rm -f "$work/public/$name"
        curl -sSfL --max-time 300 -o "$work/public/$name" "$github_downloads/$tag/$name" 2> "$work/curl" && cmp -s "$work/public/$name" "$assets/$name" && { ok=1; break; }
      done
      [ -n "$ok" ] || fail "$github_downloads/$tag/$name does not serve the file: $(tail -1 "$work/curl")"
    done
    sums_agree "$work/public" || fail "the assets read back from $github_downloads/$tag disagree with their sums (above)"
    say "read back from $github_downloads/$tag/: $names, each the file, SHA256SUMS and $manifest theirs"
    say "released $tag on ${commit:0:12}"
    ;;
  dry)
    push_master
    dry_started=1
    if [ -n "$id" ]; then
      # A draft an earlier dry run left: the scratch version's, deleted before the run.
      say "an earlier dry run's draft is left (id $id)"
      abandon_drafts
      one_release
    fi
    draft 1
    [ $? -eq 4 ] || fail "the dry run's first pass did not stop after one upload"
    first=$id
    # The second pass sees GitHub as a run started again would.
    one_release
    draft
    [ "$id" = "$first" ] || fail "the second pass is on the release $id, not the first pass's $first"
    found=$(releases) || fail "GitHub's releases could not be read after the second pass"
    [ "$(grep -c . <<< "$found")" -eq 1 ] || fail "GitHub does not have one release of $tag after the second pass"
    say "the second pass resumed the draft $id: one release of $tag"
    say "its notes:"
    sed 's/^/  /' "$work/notes.md"
    abandon_drafts
    dry_started=
    tag_at=$(github_tag) || fail "GitHub's tags could not be read after the dry run"
    [ -z "$tag_at" ] || fail "GitHub has a tag $tag after the dry run"
    say "abandoned the draft $tag: no release of it left, no tag"
    [ -z "$refused" ] || fail "the dry run passed but for the push, which the release would refuse: $refused"
    say "the dry run passed"
    ;;
esac
