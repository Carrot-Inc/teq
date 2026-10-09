#!/bin/bash
# bench/actions/credentials.sh write <dir> | remove <dir>: the credentials of the plugin's Central publication on a
# runner of the release workflow (docs/DEVELOPING.md, "Releases"), from the `release` environment's secrets given
# to this step alone as CENTRAL_USER, CENTRAL_PASSWORD and GPG_PRIVATE_KEY (an armored secret key without a
# passphrase). write, under umask 077, into a directory of the runner's that no artifact or cache names:
#   <dir>/central  the Portal's user token as Java properties (host central.sonatype.com, user, password), the file
#                  integrations/sbt/central.py reads by TEQ_CENTRAL_CREDENTIALS
#   <dir>/gnupg    a GNUPGHOME of its own holding the key, which must be the release's (bench/ship-release.sh's
#                  release_signing_key) and sign with no terminal and no passphrase
# then prints the two variables' lines (paths alone) for GITHUB_ENV. Neither value is ever an argument, printed or
# traced. remove: the home's gpg agent stopped and <dir> removed, whatever is left of it; the workflow runs it in
# an always() step after the publication.
set -uo pipefail
set +x
cd "$(dirname "$0")/../.." || exit 1
fail() { echo "credentials: $*" >&2; exit 1; }
[ $# -eq 2 ] || { echo "usage: $0 write <dir> | remove <dir>" >&2; exit 2; }
dir=$2
case $dir in /*) ;; *) fail "<dir> must be absolute" ;; esac

if [ "$1" = remove ]; then
  [ ! -d "$dir/gnupg" ] || GNUPGHOME=$dir/gnupg timeout 30 gpgconf --kill all > /dev/null 2>&1
  rm -rf -- "$dir"
  [ ! -e "$dir" ] || fail "$dir is left"
  echo "credentials: $dir removed"
  exit 0
fi
[ "$1" = write ] || { echo "usage: $0 write <dir> | remove <dir>" >&2; exit 2; }
. bench/ship-release.sh
[ -n "${release_signing_key:-}" ] || fail "bench/ship-release.sh names no release_signing_key, the key a release is signed with"
for name in CENTRAL_USER CENTRAL_PASSWORD GPG_PRIVATE_KEY; do
  [ -n "${!name:-}" ] || fail "no $name: the release environment's secret is not set"
done
# A properties value is taken as written but for a backslash, which escapes, and a line's end.
for name in CENTRAL_USER CENTRAL_PASSWORD; do
  case ${!name} in *\\* | *$'\n'* | *$'\r'*) fail "$name holds a backslash or a line's end, which the properties file cannot hold as written" ;; esac
done
command -v gpg > /dev/null || fail "no gpg"
umask 077
[ ! -e "$dir" ] || fail "$dir is there already"
mkdir -p "$dir/gnupg" || exit 1
printf 'host=central.sonatype.com\nuser=%s\npassword=%s\n' "$CENTRAL_USER" "$CENTRAL_PASSWORD" > "$dir/central" || fail "$dir/central not written"
export GNUPGHOME=$dir/gnupg
printf '%s\n' "$GPG_PRIVATE_KEY" | timeout 60 gpg --batch --quiet --import > /dev/null 2>&1 || fail "GPG_PRIVATE_KEY is not a secret key gpg imports"
timeout 30 gpg --batch --with-colons --list-secret-keys "$release_signing_key" 2> /dev/null | grep -q '^sec:' ||
  fail "GPG_PRIVATE_KEY is not the release's key $release_signing_key"
# Headless, as sbt-pgp signs: no terminal, no passphrase.
probe=$dir/probe
echo "teq's signing probe" > "$probe"
timeout 60 gpg --batch --pinentry-mode error --no-tty --local-user "$release_signing_key" --detach-sign --armor --output "$probe.asc" "$probe" < /dev/null > /dev/null 2>&1 &&
  timeout 30 gpg --batch --status-fd 1 --verify "$probe.asc" "$probe" < /dev/null 2> /dev/null | grep -q "^\[GNUPG:\] VALIDSIG .* $release_signing_key\$" ||
  fail "the key $release_signing_key does not sign with no terminal and no passphrase"
rm -f -- "$probe" "$probe.asc"
echo "TEQ_CENTRAL_CREDENTIALS=$dir/central"
echo "GNUPGHOME=$dir/gnupg"
echo "credentials: the Portal's token and the key $release_signing_key under $dir" >&2
