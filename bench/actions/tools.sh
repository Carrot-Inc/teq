#!/bin/bash
# bench/actions/tools.sh <dir>: the coursier launcher (cs) and scala-cli of the gate's jobs (.github/workflows/gate.yml: the
# jars job and every job that reads jars), from their GitHub release archives at pinned versions, each archive checked
# against its pinned digest before it is unpacked into <dir>, and <dir> added to $GITHUB_PATH where that is set. Nothing is
# resolved from Maven Central: coursier/setup-action resolved its apps channel (io.get-coursier:apps:latest.release) there
# in every job, and Central answers the runners 429. scala-cli is the image's (bench/actions/toolchain.sh's version and
# digest, which tests/release-actions.sh holds equal); cs is the version coursier/setup-action v1.3.9 installed. Each
# download is tried at most $attempts times, $delay seconds apart (TOOLS_RETRY_DELAY, 10 by default), each try bounded at
# $try seconds (TOOLS_TRY_SECONDS, 120 by default). On x86-64 Linux alone (the runners); a directory where a tool goes
# refused before any download; exits 1 on a refusal, nothing of a refused archive left in <dir>.
set -uo pipefail
cs_version=2.1.17
cs_sha256=940338ccd5445ec7f683bc3a21c86d64592c66b66394c5ad69141342c88b4e2b
scala_cli_version=1.17.1
scala_cli_sha256=4186bfac6552097fbfd3939bdf2ecdec1cb55baafcbf6259bef5c04e08858eb3
attempts=3
delay=${TOOLS_RETRY_DELAY:-10}
try=${TOOLS_TRY_SECONDS:-120}
fail() { echo "tools: $*" >&2; exit 1; }
[ $# = 1 ] || { echo "usage: bench/actions/tools.sh <dir>" >&2; exit 2; }
dir=$1
[ "$(uname -s)" = Linux ] && [ "$(uname -m)" = x86_64 ] || fail "for x86-64 Linux alone, not $(uname -s) $(uname -m)"
mkdir -p "$dir" || fail "$dir could not be made"
# A directory of a tool's name would take the binary inside it, and the command would not run.
for name in cs scala-cli; do
  [ ! -d "$dir/$name" ] || fail "$dir/$name is a directory, where the $name binary goes"
done
work=$(mktemp -d) || exit 1
trap 'rm -rf -- "$work"' EXIT

# get <url> <sha256> <name>: the archive at <url>, tried up to $attempts times and checked against <sha256>, unpacked
# into <dir>/<name>.
get() {
  local url=$1 sha=$2 name=$3 n=1
  until timeout "$try" curl -sSfLo "$work/$name.gz" "$url"; do
    [ $n -lt $attempts ] || fail "$url could not be fetched in $attempts attempts"
    echo "tools: $url not fetched (attempt $n of $attempts), again in ${delay}s" >&2
    sleep "$delay"
    n=$((n + 1))
  done
  echo "$sha  $work/$name.gz" | sha256sum -c --quiet > /dev/null 2>&1 || fail "$url's digest is not the pinned $sha"
  gzip -dc "$work/$name.gz" > "$work/$name" && chmod 755 "$work/$name" && mv -f "$work/$name" "$dir/$name" ||
    fail "$name could not be unpacked into $dir"
  echo "tools: $name from $url"
}

get "https://github.com/coursier/coursier/releases/download/v$cs_version/cs-x86_64-pc-linux.gz" $cs_sha256 cs
get "https://github.com/VirtusLab/scala-cli/releases/download/v$scala_cli_version/scala-cli-x86_64-pc-linux.gz" $scala_cli_sha256 scala-cli
[ -z "${GITHUB_PATH:-}" ] || (cd "$dir" && pwd) >> "$GITHUB_PATH" || fail "$GITHUB_PATH not written"
