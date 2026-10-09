# shellcheck shell=bash
# tests/support/identity-across.sh, sourced by tests/support/identity.sh for --produce and --compare (its header has
# the contract), with $across the mode and the mode's arguments as the positional parameters; exits.
usage() { echo "usage: tests/support/identity.sh --produce <teq> <bundle dir> [js,jvm,diags] | --compare <bundle dir> <teq> <out dir>" >&2; exit 2; }
if [ "$across" = produce ]; then
  [ $# -ge 2 ] && [ $# -le 3 ] || usage
  bin=$(absolute "$1") bundle=$(absolute "$2") kinds=${3:-js,jvm,diags}
else
  [ $# -eq 3 ] || usage
  bundle=$(absolute "$1") bin=$(absolute "$2") out=$(absolute "$3")
fi
skips=
[ -z "${IDENTITY_SKIPS:-}" ] || skips=$(absolute "$IDENTITY_SKIPS")
cd "$(dirname "${BASH_SOURCE[0]}")/../.." || exit 1
. tests/support/compiler-words.sh
JOBS=${JOBS:-$(getconf _NPROCESSORS_ONLN 2> /dev/null || echo 4)}
py=$(command -v python3 || command -v python) || { echo "identity: no python3" >&2; exit 2; }
command -v timeout > /dev/null || { echo "identity: no GNU timeout on the PATH" >&2; exit 2; }
# The working directory of both modes, under the checkout: the same relative paths on every machine.
work=out/identity
refuse() { echo "identity: $*" >&2; exit 1; }
# The checkout's root, as the file system spells it: what an output that embeds a source's path holds.
checkout=$(cd "$(git rev-parse --show-toplevel)" && pwd -P) || refuse "no checkout root"
export checkout
# revision_of <version line>: the commit a binary says it is built from, which must be the checkout's or one whose
# compiler sources are the checkout's (bench/ship-manifest.sh's rule), else its outputs are no evidence of this
# commit.
built_here() {
  local hash
  hash=$(awk '{print $3}' <<< "$1")
  [[ $hash =~ ^[0-9a-f]{7,40}$ ]] && git cat-file -e "$hash^{commit}" 2> /dev/null || return 1
  [[ $(git rev-parse HEAD) == "$hash"* ]] || git diff --quiet "$hash" HEAD -- src std runtime build.rs Cargo.toml Cargo.lock .cargo
}
digest() { if command -v sha256sum > /dev/null; then sha256sum < "$1" | cut -c1-64; else shasum -a 256 < "$1" | cut -c1-64; fi; }
# programs <kind>: the kind's programs, `<kind> <name> <source>` lines in the C locale's order.
programs() {
  local srcs src name
  case $1 in
    js | jvm) srcs=$(ls -d tests/cases/*.scala tests/cases/*/ 2> /dev/null | sed 's|/$||') ;;
    diags) srcs=$(ls -d tests/cases/*.scala tests/cases/*/ tests/errors/*.scala tests/errors/*/ 2> /dev/null | sed 's|/$||') ;;
  esac
  for src in $srcs; do
    name=$(basename "$src" .scala)
    [ "${src#tests/errors/}" != "$src" ] && name=errors-$name
    echo "$1 $name $src"
  done | LC_ALL=C sort | if [ -n "${IDENTITY_PROGRAMS:-}" ]; then grep -E "^[a-z]+ ($IDENTITY_PROGRAMS) "; else cat; fi
}
# run_case <dir> <args...>: the binary on the arguments, bounded, its output, exit code and products in <dir> (a jar
# unpacked into classes/), the times left out of the log.
run_case() {
  local dir=$1
  shift
  mkdir -p "$dir"
  COURSIER_CACHE=$work/no-cache timeout 60 "$bin" "$@" > "$dir/log" 2>&1
  echo "exit $?" >> "$dir/log"
  if [ -f "$dir/out.jar" ]; then
    mkdir -p "$dir/classes" && "$py" -c 'import sys, zipfile; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])' "$dir/out.jar" "$dir/classes" &&
      rm -f -- "$dir/out.jar"
  fi
  grep -v -E '^(checked|built|ran) .* in |^  (read|parse|type|reach|emit|write) ' "$dir/log" > "$dir/log.cmp"
  rm -f -- "$dir/log"
  # The checkout's absolute path, which a macro writes into an output (sourcecode's File), the same placeholder on
  # every machine (tests/support/identity-root.py: text files, and class files by their constant pool).
  "$py" tests/support/identity-root.py "$checkout" "$dir" || echo "identity: the checkout's path could not be normalised under $dir" >&2
}
# invalid <exit code>: a run that is no evidence: a timeout, a command not run, a signal.
invalid() { case $1 in 124 | 125 | 126 | 127 | 1[3-9][0-9] | 2[0-9][0-9]) return 0 ;; esac; return 1; }
# produce_one <kind> <name> <source>: one program by the reference, its arguments into args/; prints `missing`,
# `skip` or `invalid` for one that is no evidence.
produce_one() {
  [ $# -eq 3 ] || return 0
  local kind=$1 name=$2 src=$3 flags cp entry sum file canon= dir words status
  local -a args entries
  . tests/support/jars.sh
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    if [ -n "$skips" ] && grep -qx "$kind $name" "$skips"; then echo "skip $kind $name$JARS_MISSING"; else echo "missing $kind $name:$JARS_MISSING"; fi
    return 0
  fi
  flags=$(grep -h -o '^// teq: .*' "$src" "$src"/*.scala 2> /dev/null | head -1 | sed 's|^// teq: ||')
  cp=$JARS_CP
  if [ "$kind" = jvm ]; then
    cp=$sl${cp:+:$cp}
    flags="$flags --std=scala-library"
  fi
  if [ -n "$cp" ]; then
    # A jar outside the checkout under out/identity/jars/, named by its digest, which the class path names.
    IFS=: read -ra entries <<< "$cp"
    for entry in "${entries[@]}"; do
      case $entry in
        /*)
          [ -f "$entry" ] || { echo "invalid $kind $name: the class path's $entry is no file"; return 0; }
          sum=$(digest "$entry")
          file=$work/jars/${sum:0:16}-$(basename "$entry")
          [ -f "$file" ] || { cp "$entry" "$file.$$" && mv -f "$file.$$" "$file"; } || { echo "invalid $kind $name: $entry not copied"; return 0; }
          canon="$canon:$file" ;;
        *) canon="$canon:$entry" ;;
      esac
    done
    flags="$flags --classpath ${canon#:}"
  fi
  dir=$work/$kind/$name
  words=$(compiler_words "$bin")
  case $kind in
    js) args=($words build "$src" $flags -o "$dir/out.js") ;;
    jvm) args=($words build "$src" $flags --target jvm -o "$dir/out.jar") ;;
    diags) args=($words check "$src" $flags) ;;
  esac
  printf '%s\n' "${args[@]}" > "$work/args/$kind/$name"
  run_case "$dir" "${args[@]}"
  status=$(tail -1 "$dir/log.cmp")
  if invalid "${status#exit }"; then echo "invalid $kind $name: $status"; else echo "built $kind $name"; fi
}
# compare_one <kind> <name>: one program by <teq> with the reference's arguments; prints `differs` when its outputs
# are not the bundle's.
compare_one() {
  # GNU xargs calls once on no input.
  [ $# -eq 2 ] || return 0
  local kind=$1 name=$2 dir=$work/$1/$2 a status
  local -a args
  [ -f "$bundle/args/$kind/$name" ] && [ -d "$bundle/$kind/$name" ] || { echo "differs $kind $name: the bundle has no outputs of it"; return 0; }
  while IFS= read -r a; do args+=("$a"); done < "$bundle/args/$kind/$name"
  run_case "$dir" "${args[@]}"
  status=$(tail -1 "$dir/log.cmp")
  if invalid "${status#exit }" && [ "$status" != "$(tail -1 "$bundle/$kind/$name/log.cmp")" ]; then
    echo "differs $kind $name: $status"
  elif ! diff -rq "$dir" "$bundle/$kind/$name" > /dev/null 2>&1; then
    echo "differs $kind $name"
  else
    echo "same $kind $name"
  fi
}
export -f run_case invalid produce_one compare_one digest compiler_words _new_spelling
export bin bundle work py skips

# One identity run at a time in the checkout: both modes work in out/identity/, a fixed path the bundle's arguments
# name, which a second run would replace under the first. The lock is a directory, made atomically on every system
# (macOS has no flock), holding the run's pid and removed at its exit; a lock whose pid runs no more is taken over.
mkdir -p out || exit 1
if ! mkdir out/identity.lock 2> /dev/null; then
  holder=$(cat out/identity.lock/pid 2> /dev/null)
  # A lock just made has its pid a moment later.
  [ -n "$holder" ] || { sleep 2; holder=$(cat out/identity.lock/pid 2> /dev/null); }
  [ -z "$holder" ] || ! kill -0 "$holder" 2> /dev/null ||
    refuse "another identity run uses out/identity in this checkout, pid $holder (out/identity.lock, removed by hand if that pid is no identity run)"
  # Its run gone: the lock set aside, and taken when it is still the one read (another run may have taken it since).
  mv out/identity.lock "out/identity.lock.$$" 2> /dev/null || refuse "another identity run took out/identity.lock in this checkout meanwhile"
  if [ "$(cat "out/identity.lock.$$/pid" 2> /dev/null)" != "$holder" ]; then
    mv "out/identity.lock.$$" out/identity.lock 2> /dev/null
    refuse "another identity run took out/identity.lock in this checkout meanwhile"
  fi
  rm -rf -- "out/identity.lock.$$"
  mkdir out/identity.lock 2> /dev/null || refuse "another identity run took out/identity.lock in this checkout meanwhile"
  echo "identity: the lock of pid ${holder:-none}, whose run is gone, taken over" >&2
fi
echo $$ > out/identity.lock/pid || exit 1
trap 'rm -rf -- out/identity.lock' EXIT

if [ "$across" = produce ]; then
  [ -x "$bin" ] && version=$("$bin" --version) || refuse "$bin does not run here"
  built_here "$version" || refuse "$bin is '$version', not built from this checkout's commit $(git rev-parse --short HEAD) or its compiler sources"
  for kind in ${kinds//,/ }; do
    case $kind in js | jvm | diags) ;; *) refuse "no kind $kind across machines (js, jvm, diags)" ;; esac
  done
  . tests/support/jars.sh
  sl=$(jar_of scala-library)
  export sl
  spellings "$bin"
  rm -rf -- out/identity && mkdir -p "$work/jars" "$work/no-cache" || exit 1
  : > "$work/inventory.txt"
  status=0
  for kind in ${kinds//,/ }; do
    mkdir -p "$work/args/$kind"
    programs "$kind" > "$work/$kind.programs"
    cat "$work/$kind.programs" >> "$work/inventory.txt"
    xargs -P "$JOBS" -n 3 bash -c 'produce_one "$@"' _ < "$work/$kind.programs" > "$work/$kind.results"
    launched=$?
    grep -v '^built ' "$work/$kind.results" > "$work/$kind.refused"
    LC_ALL=C sort "$work/$kind.refused" | sed 's/^/identity: /'
    echo "identity $kind: $(grep -c . "$work/$kind.programs") programs built by the reference, $(grep -c '^skip ' "$work/$kind.refused") skipped as allowed"
    ! grep -q -v '^skip ' "$work/$kind.refused" || status=1
    # Every program a result, and every worker launched.
    [ "$launched" -eq 0 ] && [ "$(grep -c . "$work/$kind.results")" -eq "$(grep -c . "$work/$kind.programs")" ] ||
      { echo "identity: $kind: $(grep -c . "$work/$kind.results") results of $(grep -c . "$work/$kind.programs") programs (xargs exited $launched)"; status=1; }
  done
  {
    echo "identity 1"
    echo "version $version"
    echo "binary $(digest "$bin")"
    echo "commit $(git rev-parse HEAD)"
    echo "kinds $kinds"
    (cd "$work" && find jars -type f | LC_ALL=C sort | while read -r f; do echo "jar $f $(digest "$f")"; done)
    cat "$work"/*.refused | grep '^skip ' | LC_ALL=C sort
  } > "$work/identity.txt"
  (cd "$work" && rm -f -- ./*.programs ./*.refused ./*.results)
  [ $status -eq 0 ] || refuse "the reference is not whole (above): no bundle"
  echo complete >> "$work/identity.txt"
  if [ "$bundle" != "$PWD/$work" ]; then
    [ ! -e "$bundle" ] || refuse "$bundle is there already"
    mkdir -p "$(dirname "$bundle")" && cp -a "$work" "$bundle" || exit 1
  fi
  echo "identity: the reference of '$version' ($(sed -n 's/^binary //p' "$work/identity.txt" | cut -c1-16)), $(grep -c . "$work/inventory.txt") programs, $(grep -c '^jar ' "$work/identity.txt") jars, in $bundle"
  exit 0
fi

# --compare
[ "$(tail -1 "$bundle/identity.txt" 2> /dev/null)" = complete ] || refuse "$bundle is not a whole bundle (no identity.txt ending in complete)"
head=$(git rev-parse HEAD)
[ "$(sed -n 's/^commit //p' "$bundle/identity.txt")" = "$head" ] || refuse "the bundle is of $(sed -n 's/^commit //p' "$bundle/identity.txt"), the checkout of $head"
[ -x "$bin" ] && version=$("$bin" --version) || refuse "$bin does not run here"
[ "$version" = "$(sed -n 's/^version //p' "$bundle/identity.txt")" ] || refuse "$bin prints '$version', the reference '$(sed -n 's/^version //p' "$bundle/identity.txt")'"
built_here "$version" || refuse "$bin is '$version', not built from this checkout's commit $(git rev-parse --short HEAD) or its compiler sources"
kinds=$(sed -n 's/^kinds //p' "$bundle/identity.txt")
[ ! -e "$out" ] || refuse "$out is there already"
rm -rf -- out/identity && mkdir -p "$work/jars" "$work/no-cache" || exit 1
for kind in ${kinds//,/ }; do programs "$kind"; done > "$work/inventory.txt"
cmp -s "$work/inventory.txt" "$bundle/inventory.txt" || refuse "the bundle's inventory is not the checkout's programs: $(diff "$bundle/inventory.txt" "$work/inventory.txt" | grep '^[<>]' | head -3 | tr '\n' ' ')"
while read -r key file sum; do
  [ "$key" = jar ] || continue
  cp "$bundle/$file" "$work/$file" && [ "$(digest "$work/$file")" = "$sum" ] || refuse "the bundle's $file is not the jar its header names ($sum)"
done < "$bundle/identity.txt"
status=0
: > "$work/summary"
for kind in ${kinds//,/ }; do
  # The programs the reference skipped as allowed are skipped here too, and counted.
  awk -v k="$kind" '$1 == k { print $1, $2 }' "$bundle/inventory.txt" | while read -r k n; do
    grep -q "^skip $k $n " "$bundle/identity.txt" || echo "$k $n"
  done | xargs -P "$JOBS" -n 2 bash -c 'compare_one "$@"' _ > "$work/$kind.results"
  launched=${PIPESTATUS[2]}
  n=$(awk -v k="$kind" '$1 == k' "$bundle/inventory.txt" | wc -l | tr -d ' ')
  skipped=$(grep -c "^skip $kind " "$bundle/identity.txt")
  grep -v '^same ' "$work/$kind.results" > "$work/$kind.differ"
  # A program without a result (a worker not launched, one killed) differs.
  awk -v k="$kind" '$1 == k { print $2 }' "$bundle/inventory.txt" | while read -r name; do
    grep -q "^skip $kind $name " "$bundle/identity.txt" || grep -q -E "^(same|differs) $kind $name(:|\$)" "$work/$kind.results" || echo "differs $kind $name: no result"
  done >> "$work/$kind.differ"
  [ "$launched" -eq 0 ] || echo "differs $kind: xargs exited $launched" >> "$work/$kind.differ"
  d=$(grep -c . "$work/$kind.differ")
  LC_ALL=C sort "$work/$kind.differ" | sed 's/^/identity: /'
  echo "identity $kind: $(grep -c '^same ' "$work/$kind.results") of $((n - skipped)) programs identical to the reference's, $skipped skipped as the reference skipped them" | tee -a "$work/summary"
  [ "$d" = 0 ] || status=1
done
{
  echo "identity compare $(uname -s) $(uname -m)"
  echo "version $version"
  echo "binary $(digest "$bin")"
  echo "reference $(sed -n 's/^binary //p' "$bundle/identity.txt")"
  echo "commit $head"
  cat "$work/summary"
  cat "$work"/*.differ
} > "$work/report.txt"
mkdir -p "$(dirname "$out")" && cp -a "$work" "$out" || exit 1
# The differing programs' outputs, this machine's beside the reference's, under <out dir>/differs/, which a
# qualification keeps as its job's artifact.
for kind in ${kinds//,/ }; do
  sed -n "s/^differs $kind \([^: ]*\).*/\1/p" "$work/$kind.differ" | while read -r name; do
    [ -d "$work/$kind/$name" ] || continue
    mkdir -p "$out/differs/$kind/$name" && cp -a "$work/$kind/$name" "$out/differs/$kind/$name/here" || exit 1
    [ ! -d "$bundle/$kind/$name" ] || cp -a "$bundle/$kind/$name" "$out/differs/$kind/$name/reference" || exit 1
  done
done
exit $status
