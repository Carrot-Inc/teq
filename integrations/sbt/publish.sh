#!/bin/bash
# publish.sh [--check | --preflight | --stage | --rehearse [<version>]]: publishes sbt-teq, the plugin, to Maven Central at its
# own version, plugin-version.txt's (docs/TARGETS.md, "Releases"; bench/ship-release.sh). The compiler's binaries never go
# to Central: this publishes the plugin alone. sbt's publishSigned stages it into the release's record,
# out/central/sbt-teq-<version>/ (central.py), every file with its signature by the release's key and its digests,
# with the sources and javadoc jars Central requires; the staging is checked against the plugin's files and bundled;
# the bundle goes to the Central Portal as one USER_MANAGED deployment, which the Portal validates and nothing
# publishes by itself; that deployment, by the id recorded, is promoted once VALIDATED, its bundle unchanged; then
# every staged file is read back from repo1.maven.org, within three hours. A run that stops after the upload (a
# refusal, a timeout, the ship's bound) leaves the record, and the next run resumes that deployment where it
# stands, never uploading the plugin again: it waits for the validation, promotes, waits for the publication, reads
# back what is not read back yet. An upload left without an answer is looked for by its name at each run; one the
# Portal never lists is given up by the operator alone (`central.py abandon <record>`). A deployment the Portal
# refused (FAILED) waits for `central.py drop <record>`, which frees the version again. One publish of a version
# runs at a time on a machine: each holds ${XDG_CONFIG_HOME:-~/.config}/teq/central-sbt-teq-<version>.lock, beside no
# checkout; across machines the Portal is asked (central.py), since a runner keeps nothing between jobs. The record
# under out/central/ is what a rerun takes up, carried from the job before as an artifact; without it the Portal
# refuses a second upload of the version all the same.
#
# The plugin is packaged from the head and carries it (BuildInfo.commit), and its inputs are as the head commits
# them. Refused before anything is staged: inputs the head does not commit, a version that is not a release's, a
# version Central serves anything of or of which a deployment is recorded (a release is never published again:
# bench/release.sh moves to the next), and what --preflight checks. --check stops there. --stage stops after the
# staging and its check, uploading nothing, so that the ship's smoke reads the staged plugin before its publication
# (bench/ship-publish.sh); the publish that follows uploads that staging when it is the head's and unchanged, and
# stages afresh otherwise.
#
# --preflight checks what the publish needs, before a ship builds anything: the Portal's token
# (~/.sbt/sonatype_central_credentials; the Portal takes it for build.teq), the key (in gpg's keyring, signing a
# scratch file as sbt-pgp signs, with no terminal and stdin closed, its signature verifying as the key's, its public
# key on keyserver.ubuntu.com), and that the Portal publishes nothing of the version and holds no live deployment
# of it.
#
# --rehearse [<version>] rehearses the publish under a version of its own, <major>.<minor>.<patch>-rehearsal (by
# default the plugin's with -rehearsal), never a release's: the plugin of the working tree staged, checked and
# uploaded as one USER_MANAGED deployment as a release is, then dropped once the Portal has validated or refused it;
# its record is out/central/sbt-teq-<version>-<time>. Nothing of a rehearsal is ever promoted (central.py refuses
# it). Each deployment may count against Central's monthly limit of an organisation's releases: rehearse once
# after a change to the publication, not routinely.
#
# Needs sbt, gpg, python3, curl, setsid and flock.
set -uo pipefail
mode=publish rehearsal=
case ${1:-} in
  --check) mode=check; shift ;;
  --preflight) mode=preflight; shift ;;
  --stage) mode=stage; shift ;;
  --rehearse) mode=rehearse; shift; case ${1:-} in *-rehearsal) rehearsal=$1; shift ;; esac ;;
esac
[ $# -eq 0 ] || { echo "usage: $0 [--check | --preflight | --stage | --rehearse [<version>]]"; exit 2; }
cd "$(dirname "$0")" || exit 1
. ../../bench/ship-release.sh || exit 1
head=$(git rev-parse HEAD)
key=$release_signing_key
fail() { echo "publish: $*"; exit 1; }
version=$(release_plugin_version ../..) || fail "the checkout names no plugin version"

# sign_probe: the release's key signs a scratch file as sbt-pgp signs it (CommandLineGpgSigner without a
# passphrase: gpg through its agent), in a session of its own with no terminal, stdin closed and no display,
# as the detached ship runs; the signature verifies as the key's. A key that wants a passphrase fails here.
sign_probe() {
  local dir out
  dir=$(mktemp -d) || return 1
  echo "teq's signing probe, $(date -u)" > "$dir/probe"
  if out=$(env -u GPG_TTY -u DISPLAY -u WAYLAND_DISPLAY -u PGP_PASSPHRASE setsid -w timeout 60 \
    gpg --detach-sign --armor --use-agent --default-key "$key" --output "$dir/probe.asc" "$dir/probe" < /dev/null 2>&1) &&
    gpg --batch --status-fd 1 --verify "$dir/probe.asc" "$dir/probe" < /dev/null 2> /dev/null | grep -q "^\[GNUPG:\] VALIDSIG .* $key\$"; then
    rm -rf "$dir"
    echo "publish: the key $key signs headlessly, as sbt-pgp signs, and its signature verifies"
    return 0
  fi
  rm -rf "$dir"
  echo "publish: the key $key does not sign headlessly as sbt-pgp signs (no terminal, no passphrase): $out"
  return 1
}

# central_ready <version>: what the publish needs, checked before anything is built or staged.
central_ready() {
  local tool
  for tool in gpg python3 curl setsid sbt; do command -v $tool > /dev/null || { echo "publish: no $tool on the PATH"; return 1; }; done
  timeout 300 python3 -B central.py preflight --version "$1" || return 1
  gpg --batch --with-colons --list-secret-keys "$key" 2> /dev/null | grep -q '^sec:' ||
    { echo "publish: no secret key $key in gpg's keyring, which signs the release"; return 1; }
  sign_probe || return 1
  timeout 120 python3 -B central.py keyserver "$key"
}

# stage <version> <record>: the plugin staged afresh into <record>/staging by publishSigned, the version and
# the key set, sbt-pgp's passphrase variable unset.
stage() {
  local staging=$2/staging
  rm -rf "$2" && mkdir -p "$staging" || return 1
  echo "publish: staging build.teq:sbt-teq $1 into $staging"
  env -u PGP_PASSPHRASE timeout 600 sbt --server --batch "set version := \"$1\"" "set Global / stagingDirectory := file(\"$staging\")" \
    "set pgpSigningKey := Some(\"$key\")" publishSigned < /dev/null 9>&- 2>&1 | grep -E "published|error|success" | tail -4
  [ "${PIPESTATUS[0]}" -eq 0 ] || { echo "publish: the plugin's staging failed"; return 1; }
}

if [ $mode = preflight ]; then
  central_ready "$version" || fail "sbt-teq $version cannot be published to Central (above)"
  echo "publish: sbt-teq $version can be published to Central"
  exit 0
fi

if [ $mode = rehearse ]; then
  version=${rehearsal:-$version-rehearsal}
  [[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+-rehearsal$ ]] || fail "$version is not a rehearsal's version, <major>.<minor>.<patch>-rehearsal"
  central_ready "$version" || fail "the rehearsal cannot run (above)"
  record=$(plugin_central_record "$version")-$(date -u +%Y%m%dT%H%M%SZ)
  stage "$version" "$record" || fail "nothing of the rehearsal was uploaded"
  timeout 600 python3 -B central.py check "$record" "$version" "$key" "$head" || fail "nothing of the rehearsal was uploaded"
  timeout 1500 python3 -B central.py upload "$record" rehearsal || fail "the rehearsal's upload failed (above); its record is $record"
  timeout 2000 python3 -B central.py wait "$record" VALIDATED 1800
  validated=$?
  timeout 300 python3 -B central.py drop "$record" || fail "the rehearsal's deployment is not dropped: \`python3 integrations/sbt/central.py drop $record\` drops it"
  [ $validated -eq 0 ] || fail "the Portal did not validate the rehearsal (above); its deployment is dropped, its record is $record"
  echo "publish: rehearsed sbt-teq $version: staged, signed, checked, uploaded, validated by the Portal and dropped; the record is $record"
  exit 0
fi

release_committed ../.. || fail "commit the plugin's inputs, or publish from a checkout of the release's commit"
release_plugin_legacy "$version" && fail "$version is a version of the releases up to 0.1.6, which never go to Central: bench/release.sh --plugin <version>"
record=$(plugin_central_record "$version")
if [ $mode = publish ] || [ $mode = stage ]; then
  # One publish of the version at a time on this machine, whatever the checkout, from its record's inspection to
  # its read-back.
  command -v flock > /dev/null || fail "no flock on the PATH"
  lock=${XDG_CONFIG_HOME:-$HOME/.config}/teq/central-sbt-teq-$version.lock
  mkdir -p "$(dirname "$lock")" "$(dirname "$record")" && exec 9> "$lock" || exit 1
  flock -n 9 || fail "another publish of sbt-teq $version holds $lock"
fi

# A deployment recorded for the version is resumed where it stands: never staged or uploaded again. One that
# never reached the Portal, or was dropped, is set aside, and the version is published afresh.
resume= staged=
if [ $mode != check ] && [ -f "$record/deployment.log" ]; then
  state=$(timeout 300 python3 -B central.py state "$record") || fail "the deployment recorded under $record cannot be told (above)"
  case $state in
    unanswered)
      fail "the upload recorded under $record has no answer, and the Portal lists no deployment of its name yet: publish.sh again looks again; once the Portal's deployments show none for a while, \`python3 integrations/sbt/central.py abandon $record\` gives it up" ;;
    none)
      # Staged and not uploaded: that staging is uploaded when it is the head's (central.py upload checks it is
      # unchanged), and set aside otherwise.
      if [ "$(python3 -B central.py staged "$record")" = "$head" ] && [ $mode = publish ]; then
        staged=1
        echo "publish: uploading the staging of sbt-teq $version under $record, the head's"
      else
        aside=$record.$(date -u +%Y%m%dT%H%M%SZ)
        mv "$record" "$aside" || exit 1
        echo "publish: $record held a staging and no deployment: set aside as $aside"
      fi ;;
    absent | dropped)
      aside=$record.$(date -u +%Y%m%dT%H%M%SZ)
      mv "$record" "$aside" || exit 1
      echo "publish: $record held no live deployment ($state): set aside as $aside" ;;
    *)
      [ $mode = publish ] || fail "$record records a deployment of sbt-teq $version ($state): publish.sh resumes it, and nothing stages it again"
      resume=$state
      echo "publish: resuming the deployment of sbt-teq $version recorded under $record: $state" ;;
  esac
fi

if [ -z "$resume" ] && [ -z "$staged" ]; then
  plugin_unserved "$version" || fail "sbt-teq $version cannot be published"
  if [ $mode = check ]; then
    echo "publish: sbt-teq $version of $head checks out, not published (--check)"
    exit 0
  fi
  central_ready "$version" || fail "sbt-teq $version cannot be published to Central (above); nothing is staged"
  stage "$version" "$record" || fail "nothing is uploaded"
  timeout 600 python3 -B central.py check "$record" "$version" "$key" "$head" || fail "nothing is uploaded"
fi
if [ $mode = stage ]; then
  echo "publish: staged sbt-teq $version of $head under $record/staging, not uploaded (--stage)"
  exit 0
fi
if [ -z "$resume" ]; then
  timeout 1500 python3 -B central.py upload "$record" release || fail "the upload failed (above); publish.sh again tells from the record whether the Portal made the deployment"
fi
timeout 2000 python3 -B central.py wait "$record" VALIDATED 1800 || fail "the deployment is not validated (above); publish.sh again waits on, and a deployment the Portal refused waits for \`central.py drop $record\`"
timeout 300 python3 -B central.py promote "$record" "$head" || fail "the deployment is not promoted (above)"
timeout 3800 python3 -B central.py wait "$record" PUBLISHED 3600 || fail "the deployment is promoted and not published yet (above); publish.sh again waits on"
timeout 11000 python3 -B central.py readback "$record" "$release_central_root" 10800 || fail "the read-back is not complete (above); publish.sh again reads the rest back"
echo "publish: released sbt-teq $version of $head on Maven Central; the record is $record"
