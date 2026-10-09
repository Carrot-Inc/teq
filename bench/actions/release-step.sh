#!/bin/bash
# bench/actions/release-step.sh draft | smoke | publish <step>[,<step>...] | assets <dir> [--against-draft] | pin
#                               | site | conclude:
# a release's publication in the release workflow (.github/workflows/release.yml; docs/DEVELOPING.md, "Releases"),
# from the root of a checkout of the release's commit, its staged set under TEQ_STAGED (bench/ship.sh stage's
# binaries/, held/ and profiles/, the profiles that guided them). The release is TEQ_VERSION on TEQ_COMMIT, the binaries of the run TEQ_RUN (another run's when
# TEQ_MODE is resume), TEQ_DRY=true for a dry run; TEQ_PLUGIN is the plugin the compiler selects and
# TEQ_PLUGIN_PUBLISH whether this release publishes it; GH_TOKEN the workflow's token. The record is
# bench/ship-publish.sh's, out/ship/<version>/record.json (bench/ship-record.py), which the workflow carries between
# its jobs with the plugin's out/central/, and to which this adds the workflow's own keys (workflow.*, pin.*, site.*).
#
#   publish <steps>  bench/ship-publish.sh --step <steps> on the staged set, put where it reads it
#                    (integrations/sbt/binary/binaries/): a new publication for its first step (no record, or one
#                    whose draft is none yet or was abandoned), --resume after; with --plugin <TEQ_PLUGIN> when this
#                    release publishes it. Refused: binaries that are not the record's (a resume publishes the bytes
#                    the first run qualified), a step after the first with no record (a resume whose run left none),
#                    and one with a record whose draft was abandoned (a failure before the plugin's promotion: the
#                    publication starts afresh from its first job). The record gains workflow.run, workflow.image
#                    (the builds'), workflow.image_arm64 (the aarch64 training's), workflow.profiles (the release
#                    and asset whose profiles guided the builds, or `trained`) and workflow.commit at its making, the
#                    draft's job coming after the five qualifications.
#   draft            the draft: publish draft, then the draft's assets compared with the staged set's (assets
#                    --against-draft); a dry run's, bench/github-release.sh --dry-run, the scratch draft of 0.0.0-dry
#                    made, resumed and abandoned. First, for a run that built its binaries (TEQ_MODE fresh), the
#                    staged five are the bytes their qualifications ran, which ran on each build's own products as
#                    soon as it made them (the reports under TEQ_QUALIFIED, bench/actions/qualify.sh's), and those
#                    that compared their outputs compared them with the staged Linux x86-64 binary's reference
#   smoke            the consumer's smoke before anything irreversible, for a release that publishes no plugin:
#                    publish smoke; a dry run's, bench/release-smoke.sh --mirror against the five binaries the run
#                    built (held back or not: a new image's), the plugin the one Central serves
#   assets <dir>     the release's files as the draft holds them, under <dir>, written from the staged set
#                    (release_asset's names, SHA256SUMS and the binaries' manifest in bench/github-release.sh's
#                    format, the profiles' asset, the commit's NOTICE and LICENSE); with --against-draft compared
#                    with the draft's own, downloaded
#   pin              bench/release.sh --pin --commit on the release's commit, committed as the Actions bot
#                    (GIT_COMMITTER_*, GIT_AUTHOR_*), and pushed to master without force: onto the master it finds,
#                    when master moved since the release without touching what the pin changes or releasing again
#                    (the pin then made again on master's head), three tries, TEQ_RETRY_PAUSE seconds times the try
#                    apart; else it stops and says so; pin.state and pin.commit recorded
#   site             the site of the pinned commit built and deployed, when NETLIFY_AUTH_TOKEN and NETLIFY_SITE_ID
#                    are set; site.state recorded
#   conclude         after a failed run, what stands, by the record and GitHub's and the Portal's answers. Before
#                    the plugin's promotion nothing is irreversible: the draft is abandoned (github.state deleting,
#                    bench/github-release.sh <version> abandon, deleted) when the step that failed did not abandon it
#                    (a job lost or cut off), a deployment recorded and never promoted is dropped, and the next run
#                    starts afresh. Nothing is deleted before GitHub and the Portal are asked: a record persisted
#                    before a promotion whose job was lost says less than the Portal, which is asked about every
#                    deployment the record holds. After the promotion, or when an answer does not say whether it
#                    came (a word this does not know, a deployment it cannot be asked about, GitHub silent), the
#                    draft and its assets are kept, and the run's failed jobs rerun, or a dispatch with
#                    resume_run=<this run's id>, publish the same bytes without promoting again. After the GitHub publication the assets are immutable: the
#                    read-back, the smoke, the pin or the site runs again, and a defect is a new version's. Says what
#                    the next run must do in the run's summary.
set -uo pipefail
cd "$(dirname "$0")/../.." || exit 1
. bench/ship-manifest.sh || exit 1
repo=Carrot-Inc/teq
summary=${GITHUB_STEP_SUMMARY:-/dev/null}
fail() { echo "release: $*" >&2; printf '%s\n' "$*" >> "$summary"; exit 1; }
say() { echo "release: $*"; }
for v in TEQ_VERSION TEQ_COMMIT TEQ_RUN TEQ_DRY TEQ_STAGED; do [ -n "${!v:-}" ] || fail "no $v"; done
version=$TEQ_VERSION commit=$TEQ_COMMIT dry=$TEQ_DRY staged=$TEQ_STAGED
plugin=${TEQ_PLUGIN:-} plugin_publish=${TEQ_PLUGIN_PUBLISH:-false}
tag=v$version
record=out/ship/$version/record.json
central_record=$(plugin_central_record "${plugin:-none}")
rget() { python3 -B bench/ship-record.py "$record" get "$1"; }
rput() { python3 -B bench/ship-record.py "$record" set "$@" || fail "the record cannot be written at $record"; }
step=${1:-}
[ "$(git rev-parse HEAD)" = "$commit" ] || [ "$step" = site ] || fail "the checkout is $(git rev-parse HEAD), not the release's commit $commit"

# github_word: GitHub's answer for v<version>: `published`, `none` (no published release: a draft is not found by
# its tag), or `unknown` when GitHub does not answer.
github_word() {
  local answer
  if answer=$(timeout 60 gh api "repos/$repo/releases/tags/$tag" --jq .draft 2>&1); then
    case $answer in false) echo published ;; true) echo none ;; *) echo unknown ;; esac
  else
    case $answer in *"HTTP 404"*) echo none ;; *) echo unknown ;; esac
  fi
}
# binary_of <classifier>: the staged binary, held back too in a dry run (a new image's binaries are all held back).
binary_of() {
  local d=$staged/binaries/$1
  [ -f "$d/teq" ] || [ "$dry" != true ] || d=$staged/held/$1
  [ -f "$d/teq" ] || fail "no staged binary of $1: the release is the five or none"
  echo "$d/teq"
}

# qualified_set: the staged five, each the binary its qualification's report names and passed, the reference of
# the macOS and Linux aarch64 ones' identity the staged Linux x86-64 binary.
qualified_set() {
  local reports=${TEQ_QUALIFIED:-} c r ran ref linux
  [ -n "$reports" ] && [ -d "$reports" ] || fail "no TEQ_QUALIFIED, the qualifications' reports"
  linux=$(manifest_sha256 "$(binary_of linux-x86_64)") || exit 1
  for c in $release_classifiers; do
    r=$reports/$c.txt
    [ -f "$r" ] && [ "$(tail -1 "$r")" = "qualify: $c: qualified" ] || fail "no passed qualification of $c ($r)"
    ran=$(sed -n "s/^qualify: $c: the binary \([0-9a-f]*\), .*/\1/p" "$r")
    [ "$ran" = "$(manifest_sha256 "$(binary_of "$c")")" ] || fail "the staged $c is not the binary its qualification ran (${ran:-none named})"
    ref=$(sed -n "s/^qualify: $c: its outputs are the Linux x86-64 reference's, the binary \([0-9a-f]*\) .*/\1/p" "$r")
    case $c in osx-* | linux-aarch_64) [ -n "$ref" ] || fail "the qualification of $c compared no outputs with the Linux x86-64 reference" ;; esac
    [ -z "$ref" ] || [ "$ref" = "$linux" ] || fail "$c's outputs were compared with the reference of $ref, not the staged Linux x86-64 binary $linux"
  done
  say "the staged five are the binaries their qualifications ran, the reference the staged Linux x86-64 binary's"
}

# assets <dir> [--against-draft]: the release's files, as bench/github-release.sh writes them, and the draft's
# compared with them.
assets() {
  local out=$1 against=${2:-} c a back
  rm -rf -- "$out" && mkdir -p "$out" || exit 1
  for c in $release_classifiers; do cp "$(binary_of "$c")" "$out/$(release_asset "$version" "$c")" || exit 1; done
  for c in $release_classifiers; do a=$(release_asset "$version" "$c"); echo "$(manifest_sha256 "$out/$a")  $a"; done > "$out/SHA256SUMS"
  {
    echo "teq $version $commit"
    for c in $release_classifiers; do
      a=$(release_asset "$version" "$c")
      echo "$c $a $(manifest_sha256 "$out/$a") $(sha1sum < "$out/$a" | cut -c1-40) $(wc -c < "$out/$a" | tr -d ' ')"
    done
  } > "$out/teq-$version-binaries.txt"
  cp "$staged/profiles/teq-$version-profiles.tar" "$out/" 2> /dev/null || [ "$dry" = true ] || fail "no profiles in the staged set: the release publishes the profiles that guided its binaries"
  for a in NOTICE LICENSE; do git show "$commit:$a" > "$out/$a" 2> /dev/null || fail "the release's commit has no $a, which the release carries beside its binaries"; done
  [ "$against" = --against-draft ] || return 0
  # The draft's own, which bench/github-release.sh checked against the staged set.
  back=$(mktemp -d) || exit 1
  timeout 1800 gh release download "$tag" --repo "$repo" --dir "$back" > /dev/null || { rm -rf -- "$back"; fail "the draft's assets could not be downloaded"; }
  for a in $(cd "$out" && ls); do cmp -s "$out/$a" "$back/$a" || { rm -rf -- "$back"; fail "the draft's $a is not the staged set's"; }; done
  [ "$(ls "$back" | LC_ALL=C sort)" = "$(ls "$out" | LC_ALL=C sort)" ] || { rm -rf -- "$back"; fail "the draft holds other assets than the release's: $(ls "$back" | tr '\n' ' ')"; }
  rm -rf -- "$back"
}

# publish <steps>: bench/ship-publish.sh's steps on the staged set.
publish() {
  local steps=$1 c into=integrations/sbt/binary/binaries state status
  local -a flags=(--step "$steps")
  [ "$dry" != true ] || fail "a dry run publishes nothing"
  rm -rf -- "$into" out/ship/profiles && mkdir -p "$into" out/ship/profiles && cp -R "$staged/binaries/." "$into/" &&
    cp -R "$staged/profiles/." out/ship/profiles/ || fail "the staged set could not be put in $into and out/ship/profiles"
  state=
  [ ! -f "$record" ] || state=$(rget github.state)
  if [ -n "$state" ] && [ "$state" != none ] && [ "$state" != deleted ]; then
    flags+=(--resume)
    # A resume publishes the bytes the first run qualified, the record's.
    for c in $release_classifiers; do
      [ "$(manifest_sha256 "$into/$c/teq")" = "$(rget "compiler.$c.sha256")" ] ||
        fail "the staged $c is not the binary the record names (compiler.$c.sha256): a release is published from the bytes it qualified"
    done
  elif [[ ",$steps," != *,draft,* ]]; then
    [ -f "$record" ] || fail "no record of $version: the publication starts with its draft's job, and a resume of a run that left no record has nothing to take up"
    fail "the record holds the draft of $version as ${state:-none}: after a failure before the plugin's promotion the publication starts afresh, from its draft's job (a new run, or all of its jobs run again)"
  fi
  [ "$plugin_publish" != true ] || flags+=(--plugin "$plugin")
  bench/ship-publish.sh "${flags[@]}" "$version"
  status=$?
  if [ -f "$record" ] && [ -z "$(rget workflow.run)" ]; then
    rput workflow.run "$TEQ_RUN" workflow.image "${TEQ_IMAGE:-none}" workflow.image_arm64 "${TEQ_IMAGE_ARM64:-none}" \
      workflow.profiles "${TEQ_PROFILES_SOURCE:-trained}" workflow.commit "$commit"
  fi
  return $status
}

# dry_smoke: the consumer's smoke of a dry run against a local mirror of the five binaries.
dry_smoke() {
  local root status
  [ -x bench/release-smoke.sh ] || fail "no bench/release-smoke.sh in the tree"
  root=$(mktemp -d) || exit 1
  assets "$root/releases/v$version"
  mkdir -p "$root/maven"
  timeout 3600 bench/release-smoke.sh --mirror "$root/releases" "$root/maven" "$version"
  status=$?
  rm -rf -- "$root"
  return $status
}

# pin: the pin on the release's commit, committed by bench/release.sh as the Actions bot and pushed onto master.
pin() {
  local subject="Pin the example and the documents to $version" head try newer changed
  local -a git_push=(git -c credential.helper= -c 'credential.helper=!f() { test "$1" = get || return 0; echo username=x-access-token; printf "password=%s\n" "$GH_TOKEN"; }; f')
  local -a bot=(GIT_COMMITTER_NAME='github-actions[bot]' GIT_COMMITTER_EMAIL='41898282+github-actions[bot]@users.noreply.github.com'
    GIT_AUTHOR_NAME='github-actions[bot]' GIT_AUTHOR_EMAIL='41898282+github-actions[bot]@users.noreply.github.com')
  [ "$(rget github.state)" = read-back ] && [ "$(rget smoke.public)" = passed ] || fail "the pin comes after the publication and the public smoke, which the record does not hold"
  timeout 300 git fetch -q origin +refs/heads/master:refs/remotes/origin/master || fail "master could not be fetched"
  head=$(git log --format='%H %s' "$commit..origin/master" | awk -v s="$subject" '{ h = $1; $1 = ""; if (substr($0, 2) == s) { print h; exit } }')
  if [ -n "$head" ]; then
    rput pin.state pushed pin.commit "$head"
    echo "commit=$head" >> "${GITHUB_OUTPUT:-/dev/null}"
    say "the pin of $version is on master already, $head"
    return 0
  fi
  rput pin.state pinning
  env "${bot[@]}" timeout 3600 bench/release.sh --pin --commit || fail "bench/release.sh --pin --commit failed (above): nothing is pushed"
  if [ "$(git rev-parse HEAD)" = "$commit" ]; then
    rput pin.state pushed pin.commit "$commit"
    echo "commit=$commit" >> "${GITHUB_OUTPUT:-/dev/null}"
    say "the example and the documents pin $version already"
    return 0
  fi
  [ "$(git log -1 --format=%s)" = "$subject" ] && [ "$(git rev-parse HEAD~1)" = "$commit" ] ||
    fail "bench/release.sh --pin --commit made $(git log -1 --format='%h %s'), not the pin on the release's commit"
  changed=$(git diff --name-only HEAD~1 HEAD)
  head=$(git rev-parse HEAD)
  for try in 1 2 3; do
    if GIT_TERMINAL_PROMPT=0 timeout 300 "${git_push[@]}" push --no-follow-tags --porcelain origin "HEAD:refs/heads/master" < /dev/null; then
      rput pin.state pushed pin.commit "$(git rev-parse HEAD)"
      say "pushed the pin $(git rev-parse --short HEAD) to master"
      echo "commit=$(git rev-parse HEAD)" >> "${GITHUB_OUTPUT:-/dev/null}"
      return 0
    fi
    [ "$try" -lt 3 ] || break
    sleep $((try * ${TEQ_RETRY_PAUSE:-15}))
    timeout 300 git fetch -q origin +refs/heads/master:refs/remotes/origin/master || continue
    # Master moved since the release: the pin goes onto it when nothing it changes, the example or a release
    # moved there; never a force.
    newer=$(git log --format='%h %s' "$commit..origin/master" | grep -E ' Release (sbt-teq )?[0-9]+\.[0-9]+\.[0-9]+$')
    [ -z "$newer" ] || { rput pin.state refused; fail "master releases again since $version ($newer): the pin of $version is not pushed, the next release's pins"; }
    # shellcheck disable=SC2086
    git diff --quiet "$commit" origin/master -- $changed integrations/sbt/example ||
      { rput pin.state refused; fail "master changed what the pin changes since ${commit:0:12}: the pin is a landing's now (bench/release.sh --pin on master)"; }
    git checkout -q --detach origin/master && env "${bot[@]}" git cherry-pick "$head" > /dev/null ||
      { git cherry-pick --abort 2> /dev/null; fail "the pin could not be made on master's head"; }
    say "master moved since the release; the pin made again on $(git rev-parse --short origin/master)"
  done
  rput pin.state refused
  fail "the push of the pin was refused three times: the run's pin job, run again, tries again"
}

# site: the site of the pinned commit, deployed.
site() {
  [ "$(rget pin.state)" = pushed ] || fail "the pin is not on master"
  if [ -z "${NETLIFY_AUTH_TOKEN:-}" ] || [ -z "${NETLIFY_SITE_ID:-}" ]; then
    rput site.state skipped
    say "no NETLIFY_AUTH_TOKEN and NETLIFY_SITE_ID: the site is deployed by hand"
    return 0
  fi
  (cd site && timeout 900 npm ci --no-audit --no-fund && timeout 900 npm run build) || fail "the site could not be built"
  mkdir -p out || exit 1
  (cd site && timeout 900 npx -y netlify-cli@27.11.2 deploy --prod --no-build --dir dist --site "$NETLIFY_SITE_ID" < /dev/null > ../out/site-deploy.log 2>&1) ||
    { tail -5 out/site-deploy.log; fail "the site could not be deployed"; }
  rput site.state deployed site.commit "$(git rev-parse HEAD)"
}

# recorded <event>: whether the plugin's record holds the event (central.py's deployment.log: `staged` from the
# staging on, `upload-started` once an upload began, `promote-requested` once the promotion was asked for).
recorded() { grep -q "\"event\": \"$1\"" "$central_record/deployment.log" 2> /dev/null; }
# central_state: the Portal's word for the plugin's recorded deployment (integrations/sbt/central.py state), lower
# case; `unknown` when it cannot be asked.
central_state() {
  local word
  [ -n "$plugin" ] && recorded upload-started && [ -n "${TEQ_CENTRAL_CREDENTIALS:-}" ] || { echo unknown; return; }
  word=$(timeout 300 python3 -B integrations/sbt/central.py state "$central_record" 2> /dev/null | tail -1 | tr '[:upper:]' '[:lower:]')
  echo "${word:-unknown}"
}

# drop_unpromoted: a deployment the record holds that was never promoted dropped, which frees the version on the
# Portal (central.py drop refuses one whose promotion was requested).
drop_unpromoted() {
  local word
  recorded upload-started && ! recorded promote-requested || return 0
  word=$(central_state)
  case $word in pending | validating | validated | failed) timeout 300 python3 -B integrations/sbt/central.py drop "$central_record" || true ;; esac
}
# conclude: what stands after a failed run, and what the next run does.
conclude() {
  local next promoted=no word= central msg github
  if [ "$dry" = true ]; then
    # The scratch draft, which the dry run deletes itself unless its runner was lost.
    timeout 600 bench/github-release.sh 0.0.0-dry abandon || fail "the dry run's scratch draft could not be deleted: bench/github-release.sh 0.0.0-dry abandon"
    msg="A dry run: nothing is published, and no scratch draft is left."
  elif [ ! -f "$record" ]; then
    msg="Nothing was published: the run failed before its publication began."
  else
    next="re-run the failed jobs of run ${GITHUB_RUN_ID:-?}, or dispatch the workflow with version $version and resume_run=${GITHUB_RUN_ID:-<this run>}"
    github=$(github_word)
    if [ "$github" = published ] || [ "$(rget github.state)" = read-back ]; then
      msg="$tag is public: its assets are immutable. To finish: $next; the steps after the publication (read-back, smoke, pin, site) run again. A defect needs a new version."
    elif [ "$(rget github.state)" = deleted ]; then
      drop_unpromoted
      msg="The draft $tag was abandoned after a failure before the plugin's promotion: nothing of $version is public. The publication starts afresh: a new run (a dispatch with version $version), or all of this run's jobs run again."
    else
      # Promoted, not, or not known. The record says promoted once the promotion was asked for; short of that, a
      # deployment it holds is the Portal's to tell, since the job that promoted it may have been lost before its
      # records were persisted (they then stop at `uploaded`). A word this does not know, or no answer, keeps the
      # draft, as does a GitHub that does not answer.
      central=$(rget central.state)
      if recorded promote-requested || [ "$central" = published ] || [ "$central" = read-back ]; then
        promoted=yes
      elif recorded upload-started; then
        word=$(central_state)
        case $word in
          published | publishing | read-back) promoted=yes ;;
          absent | pending | validating | validated | failed | dropped) promoted=no ;;
          *) promoted=unknown ;;
        esac
      fi
      [ "$promoted" != no ] || [ "$github" != unknown ] || promoted=github
      case $promoted in
        yes) msg="sbt-teq $plugin is on Central and $tag is a draft: the draft and its assets are kept. To finish: $next; the same bytes are published, nothing is promoted again." ;;
        unknown) msg="sbt-teq $plugin's deployment is recorded and the Portal's answer ('$word') does not say whether it was promoted: the draft is kept. Ask the Portal (python3 integrations/sbt/central.py state $central_record), then: $next." ;;
        github) msg="GitHub did not answer for $tag: the draft is kept, nothing is deleted. Once GitHub answers: $next." ;;
        *)
          case $(rget github.state) in
            drafting | draft | deleting | publishing)
              rput github.state deleting
              timeout 600 bench/github-release.sh "$version" abandon || fail "the draft could not be abandoned (above): bench/github-release.sh $version abandon"
              rput github.state deleted ;;
          esac
          drop_unpromoted
          msg="Nothing irreversible was done: the draft is abandoned and the record kept. A new run (a dispatch with version $version) publishes afresh."
          [ "$central" != uploading ] || recorded upload-started ||
            msg="$msg An upload of sbt-teq $plugin was begun and its job lost before its deployment was recorded: a deployment of it the Portal holds is dropped there first." ;;
      esac
    fi
  fi
  say "$msg"
  printf '%s\n' "$msg" >> "$summary"
}

case $step in
  publish) [ $# -eq 2 ] || fail "usage: $0 publish <step>[,<step>...]"; publish "$2" ;;
  draft)
    [ "${TEQ_MODE:-fresh}" != fresh ] || qualified_set
    if [ "$dry" = true ]; then timeout 7200 bench/github-release.sh --dry-run; else publish draft && assets out/actions/assets --against-draft; fi ;;
  smoke)
    if [ "$dry" = true ]; then dry_smoke; else publish smoke; fi ;;
  assets) [ $# -ge 2 ] || fail "usage: $0 assets <dir> [--against-draft]"; assets "$2" "${3:-}"; say "the assets of $tag in $2" ;;
  pin) [ "$dry" != true ] || fail "a dry run publishes nothing"; pin ;;
  site) [ "$dry" != true ] || fail "a dry run publishes nothing"; site ;;
  conclude) conclude ;;
  *) echo "usage: $0 draft | smoke | publish <step>[,<step>...] | assets <dir> [--against-draft] | pin | site | conclude" >&2; exit 2 ;;
esac
