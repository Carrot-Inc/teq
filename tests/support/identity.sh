#!/bin/bash
# tests/support/identity.sh <reference teq> <teq> <out dir> [kinds]: every program of tests/cases and tests/errors
# built by both binaries and compared byte for byte, a kind at a time: `js` (the single JavaScript file of `teq compiler build`),
# `jvm` (the class files of `--target jvm`, each jar unpacked, linked against scala-library's jar, given as
# `--std=scala-library` so that a binary from before the lean JVM mode's retirement links too; `link` is its old
# name), `diags` (what `teq compiler check` prints, the times left out) and `flagged` (the diagnostics with
# FLAGGED_REF and FLAGGED_NEW added: a base from before the definition's diagnostics were the default runs with
# `FLAGGED_REF=--inline-definition-errors`, a flag the later binaries refuse). kinds defaults to all four. A program's `// teq:` flags and `// jars:` class path are
# passed as the suites pass them; one whose jars are missing is left out. REF_FLAGS and NEW_FLAGS add flags to one
# binary's runs alone (a base built before a flag became the default runs with it, `REF_FLAGS=--inline-substitution`),
# and FLAGGED_REF and FLAGGED_NEW are what `flagged` adds to each (nothing by default). Prints a line per differing program and kind and one summary line per kind; exits 1
# when any differs. JOBS programs at once (the cores by default).
#
# tests/support/identity.sh --produce <teq> <bundle dir> [kinds] | --compare <bundle dir> <teq> <out dir>: the same
# comparison across machines, one binary on each, for a release's binaries on their platforms (docs/DEVELOPING.md,
# "Releases"). --produce, on the machine of the reference (Linux x86-64, every jar of the corpus at hand): each
# program of the kinds (js, jvm and diags by default; `flagged` adds nothing here) built by <teq>, its outputs,
# the arguments it was given, the jars it read and the inventory of the programs into <bundle dir>, under a header
# binding them to the binary's version and digest and the checkout's commit; a program whose jars are missing or a
# run that timed out or died of a signal refuses the bundle (IDENTITY_SKIPS names a file of programs, `<kind>
# <name>` lines, allowed to lack their jars, each then listed in the bundle; none by default). --compare, on the
# platform of <teq>, from a checkout of the bundle's commit: the bundle's jars by their digests, then every program
# of its inventory built by <teq> with the arguments of the reference, its outputs compared byte for byte with the
# bundle's; refused before a run when the bundle is not whole, its commit is not the checkout's, its inventory is not
# the checkout's programs or <teq> prints another version than the reference; a program the bundle lists without
# outputs, a program left without a result, or a run that timed out or died where the reference's did not, never
# compares as identical. Either mode refuses a binary whose version names neither the checkout's commit nor one with
# the checkout's compiler sources (the outputs would be another compiler's). Both run
# from the root of the checkout on the same relative paths (out/identity/: the sources, the jars, the outputs), with
# COURSIER_CACHE an empty directory, so that nothing of a machine's paths or caches reaches an output; the times of
# `check` are left out, as above, and nothing else is normalised. One run of either mode at a time in a checkout:
# each holds out/identity.lock (its pid) until it exits, a second is refused at once, and a lock whose run is gone
# is taken over. --compare copies the outputs and a report into <out dir>. IDENTITY_PROGRAMS, an extended regular expression, restricts both to the programs whose names it
# matches whole, for a check of this script. Runs under macOS's bash 3.2; needs GNU timeout and python3 (or python).
case ${1:-} in --produce | --compare) across=${1#--}; shift ;; *) across= ;; esac
absolute() { case $1 in /*) echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
[ -z "$across" ] || . "$(dirname "$0")/identity-across.sh"
[ $# -ge 3 ] || { echo "usage: tests/support/identity.sh <reference teq> <teq> <out dir> [js,jvm,diags,flagged] | --produce <teq> <bundle dir> [kinds] | --compare <bundle dir> <teq> <out dir>" >&2; exit 2; }
ref=$(absolute "$1") new=$(absolute "$2") out=$(absolute "$3")
kinds=${4:-js,jvm,diags,flagged}
kinds=${kinds//link/jvm}
cd "$(dirname "$0")/../.."
. tests/support/jars.sh
. tests/support/compiler-words.sh
spellings "$ref" "$new"
JOBS=${JOBS:-$(getconf _NPROCESSORS_ONLN 2> /dev/null || echo 4)}
sl=$(jar_of scala-library)
rm -rf "$out"
mkdir -p "$out"
REF_FLAGS=${REF_FLAGS-} NEW_FLAGS=${NEW_FLAGS-}
FLAGGED_REF=${FLAGGED_REF-} FLAGGED_NEW=${FLAGGED_NEW-}
export ref new out sl REF_FLAGS NEW_FLAGS FLAGGED_REF FLAGGED_NEW
one() {
  local kind=$1 src=$2
  local name
  name=$(basename "$src" .scala)
  [ "${src#tests/errors/}" != "$src" ] && name=errors-$name
  . tests/support/jars.sh
  jars_of "$src"
  [ -z "$JARS_MISSING" ] || return 0
  local flags
  flags=$(grep -h -o '^// teq: .*' "$src" "$src"/*.scala 2> /dev/null | head -1 | sed 's|^// teq: ||')
  local cp=$JARS_CP
  case $kind in
    jvm) cp=$sl${cp:+:$cp}; flags="$flags --std=scala-library" ;;
  esac
  [ -n "$cp" ] && flags="$flags --classpath $cp"
  local side bin dir own
  for side in ref new; do
    bin=${!side}
    dir=$out/$kind/$side/$name
    mkdir -p "$dir"
    case $side in
      ref) own=$REF_FLAGS ;;
      new) own=$NEW_FLAGS ;;
    esac
    if [ "$kind" = flagged ]; then
      case $side in
        ref) own="$own $FLAGGED_REF" ;;
        new) own="$own $FLAGGED_NEW" ;;
      esac
    fi
    case $kind in
      js) timeout 60 "$bin" $(compiler_words "$bin") build "$src" $flags $own -o "$dir/out.js" > "$dir/log" 2>&1 ;;
      jvm)
        timeout 60 "$bin" $(compiler_words "$bin") build "$src" $flags $own --target jvm -o "$dir/out.jar" > "$dir/log" 2>&1
        [ -f "$dir/out.jar" ] && (cd "$dir" && mkdir -p classes && cd classes && unzip -q -o ../out.jar && rm ../out.jar) ;;
      diags | flagged) timeout 60 "$bin" $(compiler_words "$bin") check "$src" $flags $own > "$dir/log" 2>&1 ;;
    esac
    echo "exit $?" >> "$dir/log"
    grep -v -E '^(checked|built|ran) .* in |^  (read|parse|type|reach|emit|write) ' "$dir/log" > "$dir/log.cmp"
    rm "$dir/log"
  done
  if ! diff -rq "$out/$kind/ref/$name" "$out/$kind/new/$name" > /dev/null 2>&1; then
    echo "$kind $name"
  fi
}
export -f one compiler_words _new_spelling
status=0
for kind in ${kinds//,/ }; do
  case $kind in
    js | jvm) srcs=$(ls -d tests/cases/*.scala tests/cases/*/ 2> /dev/null | sed 's|/$||') ;;
    diags | flagged) srcs=$(ls -d tests/cases/*.scala tests/cases/*/ tests/errors/*.scala tests/errors/*/ 2> /dev/null | sed 's|/$||') ;;
    *) echo "identity: no kind $kind" >&2; exit 2 ;;
  esac
  n=$(echo "$srcs" | wc -l | tr -d ' ')
  echo "$srcs" | xargs -P "$JOBS" -I{} bash -c "one $kind {}" > "$out/$kind.differ"
  d=$(wc -l < "$out/$kind.differ" | tr -d ' ')
  sort "$out/$kind.differ" | sed 's/^/differs: /'
  echo "identity $kind: $((n - d)) of $n programs identical"
  [ "$d" = 0 ] || status=1
done
exit $status
