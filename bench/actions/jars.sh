#!/bin/bash
# bench/actions/jars.sh fetch | pack <file> | restore <file>: the jars of the corpus and the suites on a machine of the
# release workflow (docs/DEVELOPING.md, "Releases"), from the root of a checkout.
#   fetch    every Maven jar tests/support/jars.sh names (its `$M2/...` paths, the coordinates), each missing one
#            fetched from Maven Central into COURSIER_CACHE's layout and kept only when its SHA-1 is the one Central
#            serves beside it; the workflow caches the directory, keyed by tests/support/jars.sh.
#   pack     the jars tests/support/jars.sh builds (jars_warm: scala-cli's, python's), built here, into <file>, a
#            tarball whose names are the builds' without this checkout's prefix.
#   restore  <file>'s jars put where tests/support/jars.sh looks for them from this checkout, newer than their
#            sources, so that the suites take them as they are: the same bytes on every platform, with neither
#            scala-cli nor a JDK there.
# Every download and build is bounded; prints what it did; exits 1 on a jar it could not have.
set -uo pipefail
cd "$(dirname "$0")/../.." || exit 1
export COURSIER_CACHE=${COURSIER_CACHE:-$HOME/.cache/coursier/v1}
central=https://repo1.maven.org/maven2
fail() { echo "jars: $*" >&2; exit 1; }
. tests/support/jars.sh

fetch() {
  local path file got want missing=0 fetched=0 failed=
  for path in $(grep -o '"\$M2/[^"]*"' tests/support/jars.sh | sed 's|^"\$M2/||; s|"$||' | LC_ALL=C sort -u); do
    file=$M2/$path
    [ -f "$file" ] && continue
    missing=$((missing + 1))
    mkdir -p "$(dirname "$file")" || exit 1
    want=$(timeout 60 curl -sSfL "$central/$path.sha1" | cut -c1-40) &&
      timeout 300 curl -sSfLo "$file.part" "$central/$path" &&
      got=$(python3 -c 'import hashlib, sys; print(hashlib.sha1(open(sys.argv[1], "rb").read()).hexdigest())' "$file.part") &&
      [ "$got" = "$want" ] && mv -f "$file.part" "$file" && fetched=$((fetched + 1)) ||
      { rm -f -- "$file.part"; failed="$failed $path"; }
  done
  [ -z "$failed" ] || fail "not fetched from Maven Central:$failed"
  echo "jars: $fetched of the $missing jars missing from $COURSIER_CACHE fetched from Maven Central"
}

pack() {
  local out=$1 stage name
  timeout 1800 bash -c '. tests/support/jars.sh && jars_warm' || fail "the built jars could not be built (above)"
  stage=$(mktemp -d) || exit 1
  for name in "$scratch"-*; do
    case $name in *.[0-9]*) continue ;; esac
    cp -R "$name" "$stage/scratch${name#"$scratch"}" || { rm -rf -- "$stage"; fail "$name not copied"; }
  done
  (cd "$stage" && tar -czf - scratch-*) > "$out" || { rm -rf -- "$stage"; fail "$out not written"; }
  echo "jars: $(cd "$stage" && find . -type f | wc -l | tr -d ' ') built files into $out"
  rm -rf -- "$stage"
}

restore() {
  local in=$1 stage name
  stage=$(mktemp -d) || exit 1
  tar -xzf "$in" -C "$stage" || { rm -rf -- "$stage"; fail "$in could not be read"; }
  for name in "$stage"/scratch-*; do
    rm -rf -- "$scratch${name#"$stage/scratch"}" && mv "$name" "$scratch${name#"$stage/scratch"}" || { rm -rf -- "$stage"; fail "$name not restored"; }
    # Newer than every source, which the checkout wrote before.
    find "$scratch${name#"$stage/scratch"}" -exec touch {} +
  done
  rm -rf -- "$stage"
  echo "jars: the built jars of $in under $scratch-*"
}

case ${1:-}/$# in
  fetch/1) fetch ;;
  pack/2) pack "$2" ;;
  restore/2) restore "$2" ;;
  *) echo "usage: $0 fetch | pack <file> | restore <file>" >&2; exit 2 ;;
esac
