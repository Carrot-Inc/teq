#!/bin/bash
# tests/support/identity.sh <reference teq> <teq> <out dir> [kinds]: every program of tests/cases and tests/errors
# built by both binaries and compared byte for byte, a kind at a time: `js` (the single JavaScript file of `teq compiler build`),
# `jvm` (the class files of `--target jvm`, each jar's entries listed with their digests, never unpacked: a file
# system that folds case merges `Run.class` and `run.class`; linked against scala-library's jar, given as
# `--std=scala-library` so that a binary from before the lean JVM mode's retirement links too; `link` is its old
# name), `diags` (what `teq compiler check` prints, the times left out) and `flagged` (the diagnostics with
# FLAGGED_REF and FLAGGED_NEW added: a base from before the definition's diagnostics were the default runs with
# `FLAGGED_REF=--inline-definition-errors`, a flag the later binaries refuse). kinds defaults to all four. A program's `// teq:` flags and `// jars:` class path are
# passed as the suites pass them; one whose jars are missing is left out. REF_FLAGS and NEW_FLAGS add flags to one
# binary's runs alone (a base built before a flag became the default runs with it, `REF_FLAGS=--inline-substitution`),
# and FLAGGED_REF and FLAGGED_NEW are what `flagged` adds to each (nothing by default). Prints a line per differing program and kind and one summary line per kind; exits 1
# when any differs. JOBS programs at once (the cores by default).
# A program is identical only when both sides ran and left a whole output: a launch that failed, timed out or died of
# a signal, an output missing, empty or cut before its `exit` line, or a build that exited 0 without its JavaScript or
# its jar's entries on either side differs, the reason named. A program whose jars are missing is not run, a line
# `not run <kind> <name>: jars missing: <names>` each, and fails the run unless the file IDENTITY_SKIPS names lists its
# `<kind> <name>` (as --produce reads it); under TEQ_GATE it is a FAIL line too (a gate's want of what it needs; read
# here directly). Every program of a kind has exactly one result, no name is two programs and the out directory
# holds no other program, or the run fails. The judges run first on planted bad records (`--self-test` alone runs
# them): `identity self-test: N of N planted bad records failed`.
# A kind's identical programs are counted apart, those built on both sides and those failed alike (the reference's
# record ending in an exit other than 0), and judged against the corpus's lists (allowed_of): a program failed alike
# that the lists say builds fails the run, the first ten named. Before a kind's programs its canary (canary_of: a
# program its passing list names) is built by the reference; a canary that does not build, or lists that name none,
# make the kind VOID: its line, its programs not run, the run failed. So a reference older than the lists' first
# passing case reads VOID.
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
# compares as identical. Both hold the canaries and the lists as above: --produce builds each kind's canary into the
# bundle (canary/<kind>/) and refuses the bundle when it does not build or when the reference failed a program the
# lists say builds; --compare is void when the bundle's canary is not a build. Either mode refuses a binary whose
# version names neither the checkout's commit nor one with the checkout's compiler sources (the outputs would be
# another compiler's). Both run
# from the root of the checkout on the same relative paths (out/identity/: the sources, the jars, the outputs), with
# COURSIER_CACHE an empty directory, so that nothing of a machine's paths or caches reaches an output; every time is
# left out (the phase lines of `check` and `build`, the figures of the jar report a failed build prints, an analysis
# answer's `ms`), the jar cache is left unused (its rows would say which run warmed it), and nothing else is
# normalised. One run of either mode at a time in a checkout:
# each holds out/identity.lock (its pid) until it exits, a second is refused at once, and a lock whose run is gone
# is taken over. --compare copies the outputs and a report into <out dir>. IDENTITY_PROGRAMS, an extended regular expression, restricts both to the programs whose names it
# matches whole, for a check of this script. Runs under macOS's bash 3.2; needs GNU timeout and python3 (or python).
case ${1:-} in --produce | --compare) across=${1#--}; shift ;; --self-test) across=self-test; shift ;; *) across= ;; esac
absolute() { case $1 in /*) echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
# compare_form <log> <out>: the log without its times: the phase lines of `check` and `build`, the `ms` object of
# an analysis answer, and in the rows of the jar report a failed build prints (two spaces, a label, columns of
# figures) the `<n> ms` figures with the padding that aligns the columns to them.
compare_form() {
  grep -v -E '^(checked|built|ran) .* in |^  (read|parse|type|reach|emit|write) ' "$1" |
    sed -E 's/"ms":[{][^}]*[}]/"ms":{}/g; /^  [A-Za-z].*  +[0-9]/{ s/ +[0-9]+\.[0-9]+ ms//g; s/ {2,}/ /g; }' > "$2"
}
export -f compare_form
# jar_entries <jar> <out>: the jar's entries, each its SHA-256 and name, in the names' order; the jar removed.
jar_entries() {
  "$(command -v python3 || command -v python)" -c 'import hashlib, sys, zipfile
z = zipfile.ZipFile(sys.argv[1])
for name in sorted(z.namelist()):
    print(hashlib.sha256(z.read(name)).hexdigest(), name)' "$1" > "$2" && rm -f -- "$1"
}
export -f jar_entries
# whole_output <kind> <dir>: why a side's output is not whole, nothing when it is.
whole_output() {
  local kind=$1 dir=$2 last
  [ -s "$dir/log.cmp" ] || { echo "no output"; return; }
  last=$(tail -n 1 "$dir/log.cmp")
  [[ $last =~ ^exit\ [0-9]+$ ]] || { echo "output cut before its exit line"; return; }
  [ "$last" = "exit 0" ] || return 0
  case $kind in
    js) [ -s "$dir/out.js" ] || echo "exit 0 without its JavaScript" ;;
    jvm) [ -s "$dir/entries.txt" ] || echo "exit 0 without its jar's entries" ;;
  esac
}
# judge <kind> <name> <ref dir> <new dir> <launch>: the program's result line, `same <name>` or `differs <kind> <name>`
# with its reason: a side that did not run (<launch>), an output not whole, or outputs that differ.
judge() {
  local kind=$1 name=$2 ref=$3 new=$4 launch=$5 r n
  if [ -n "$launch" ]; then echo "differs $kind $name (did not run:$launch)"; return; fi
  r=$(whole_output "$kind" "$ref") n=$(whole_output "$kind" "$new")
  if [ -n "$r$n" ]; then echo "differs $kind $name (no whole output:${r:+ ref: $r}${n:+ new: $n})"; return; fi
  if diff -rq "$ref" "$new" > /dev/null 2>&1; then echo "same $name"; else echo "differs $kind $name"; fi
}
# result_names <results>: the program of each result line, where its form has it: `same <name>`,
# `notrun <name> <why>`, `differs <kind> <name> [<why>]`.
result_names() { awk '{ print ($1 == "differs" ? $3 : $2) }' "$1"; }
# coverage <inventory> <results> <out dir of the kind>: what is wrong with a kind's results, a line each: a name of two
# programs, a program with no result or several, a result of no program, outputs of a program the inventory lacks.
coverage() {
  local inventory=$1 results=$2 dir=$3
  sort "$inventory" | uniq -d | sed 's/^/a name of two programs: /'
  result_names "$results" | sort | uniq -d | sed 's/^/several results of /'
  sort -u "$inventory" > "$results.inv"
  result_names "$results" | sort -u > "$results.got"
  comm -23 "$results.inv" "$results.got" | sed 's/^/no result of /'
  comm -13 "$results.inv" "$results.got" | sed 's/^/a result of no program: /'
  ls "$dir/ref" "$dir/new" 2> /dev/null | grep -v -e '^$' -e ':$' | sort -u | comm -13 "$results.inv" - | sed 's/^/outputs of no program: /'
  rm -f "$results.inv" "$results.got"
}
# The corpus's lists the canaries and the counts are read from: the checkout's (the self-test plants its own).
lists_root=.
# board_of <kind>: the passing list of the suite that builds the kind's programs: tests/run_jvm.sh's for jvm,
# tests/run.sh's for js and for diags and flagged (a program the JavaScript suite builds types clean).
board_of() { case $1 in jvm) echo tests/board/suites/jvm.tsv ;; *) echo tests/board/suites/cases.tsv ;; esac; }
# canary_of <kind>: the first program in the C locale's order that the kind's passing list passes, of one file and no
# `// jars:` line (a want of a jar is counted apart) or `// os:` line (its pass is the board's system's); nothing
# when there is none.
canary_of() {
  local name f
  for name in $(awk -F'\t' '$2 == "pass" { print $1 }' "$lists_root/$(board_of "$1")" 2> /dev/null | LC_ALL=C sort); do
    f=$lists_root/tests/cases/$name.scala
    [ -f "$f" ] && ! grep -q -E '^// (jars|os):' "$f" && { echo "$name"; return; }
  done
}
# allowed_of <kind> <inventory>: the inventory's programs whose build the lists allow to fail: all but those the
# passing list passes and, for jvm, those tests/pending/jvm.txt classes past the build (verify, exception, output,
# timeout: tests/run_jvm.sh's classes); a program of tests/errors is never on a passing list.
allowed_of() {
  local kind=$1 inventory=$2
  {
    awk -F'\t' '$2 == "pass" { print $1 }' "$lists_root/$(board_of "$kind")" 2> /dev/null
    [ "$kind" != jvm ] || awk -F'\t' '{ split($2, w, /[ ;]/) } w[1] ~ /^(verify|exception|output|timeout)$/ { print $1 }' "$lists_root/tests/pending/jvm.txt" 2> /dev/null
  } | LC_ALL=C sort -u > "$inventory.vouched"
  LC_ALL=C sort -u "$inventory" | LC_ALL=C comm -23 - "$inventory.vouched"
  rm -f "$inventory.vouched"
}
# void_line <kind> <canary> <record> <whose>: the line of a void kind when the canary's record is not a whole build (an
# exit 0 with its JavaScript or its jar's entries) or the lists name no canary; nothing otherwise.
void_line() {
  local kind=$1 canary=$2 record=$3 whose=$4 why
  [ -n "$canary" ] || { echo "VOID identity $kind: the lists name no canary: no program of tests/cases without jars that $(board_of "$kind") passes"; return; }
  why=$(whole_output "$kind" "$record")
  [ -n "$why" ] || [ "$(tail -n 1 "$record/log.cmp")" = "exit 0" ] || why=$(head -n 1 "$record/log.cmp")
  [ -z "$why" ] || echo "VOID identity $kind: the canary $canary does not build with $whose: $why"
}
# judge_kind <kind> <results> <records> <inventory> <canary records> <whose>: every mode's verdict on a kind: its void
# line (returns 2), or `counts <built> <failed> <allowed>` and a line `beyond <name>` for each program failed that the
# lists say builds (returns 1). The identical programs are the results' `same <name>` (direct), `same <kind> <name>`
# (--compare) and `built <kind> <name>` (--produce, one side); failed, the reference's record in <records> ending in
# an exit other than 0; the canary's record is <canary records>/<canary>.
judge_kind() {
  local kind=$1 results=$2 records=$3 inventory=$4 canaries=$5 whose=$6 canary void name built=0 failed=0 over
  canary=$(canary_of "$kind")
  void=$(void_line "$kind" "$canary" "$canaries/$canary" "$whose")
  [ -z "$void" ] || { echo "$void"; return 2; }
  : > "$results.failed"
  while read -r name; do
    if [ "$(tail -n 1 "$records/$name/log.cmp" 2> /dev/null)" = "exit 0" ]; then built=$((built + 1)); else failed=$((failed + 1)); echo "$name" >> "$results.failed"; fi
  done < <(awk '$1 == "same" && NF == 2 { print $2 } $1 == "same" && NF == 3 || $1 == "built" { print $3 }' "$results")
  allowed_of "$kind" "$inventory" > "$results.allowed"
  echo "counts $built $failed $(grep -c . "$results.allowed")"
  LC_ALL=C sort "$results.failed" | LC_ALL=C comm -23 - "$results.allowed" | sed 's/^/beyond /' > "$results.beyond"
  cat "$results.beyond"
  over=$(grep -c . "$results.beyond")
  rm -f "$results.failed" "$results.allowed" "$results.beyond"
  [ "$over" = 0 ]
}
# first_ten <lines>: the names of `beyond` lines, the first ten.
first_ten() { sed -n 's/^beyond //p' <<< "$1" | head -10 | tr '\n' ' ' | sed 's/ $//'; }
export -f whole_output judge
# self_test <dir>: the judges on planted bad records, each of which must fail, and the controls, which must pass.
self_test() {
  local d=$1 caught=0 total=0 missed=
  rm -rf "${d:?}"
  mkdir -p "$d"
  good() { mkdir -p "$1"; printf 'built\nexit 0\n' > "$1/log.cmp"; printf 'x\n' > "$1/out.js"; }
  plant() {
    total=$((total + 1))
    if [ -n "$2" ]; then caught=$((caught + 1)); else missed="$missed; $1"; fi
  }
  failed() { case $1 in same\ *) ;; *) echo "$1" ;; esac; }
  good "$d/c/ref"; good "$d/c/new"
  [ "$(judge js p "$d/c/ref" "$d/c/new" "")" = "same p" ] || missed="$missed; the control"
  good "$d/a/new"; mkdir -p "$d/a/ref"
  plant "a missing output on the reference's side" "$(failed "$(judge js p "$d/a/ref" "$d/a/new" "")")"
  good "$d/b/ref"; mkdir -p "$d/b/new"
  plant "a missing output on the new side" "$(failed "$(judge js p "$d/b/ref" "$d/b/new" "")")"
  good "$d/e/ref"; good "$d/e/new"; : > "$d/e/ref/log.cmp"; : > "$d/e/new/log.cmp"
  plant "an empty output on both sides" "$(failed "$(judge diags p "$d/e/ref" "$d/e/new" "")")"
  good "$d/t/ref"; good "$d/t/new"; printf 'built\n' > "$d/t/ref/log.cmp"; printf 'built\n' > "$d/t/new/log.cmp"
  plant "an output cut before its exit line on both sides" "$(failed "$(judge diags p "$d/t/ref" "$d/t/new" "")")"
  good "$d/j/ref"; good "$d/j/new"; rm "$d/j/ref/out.js" "$d/j/new/out.js"
  plant "a build that exited 0 without its JavaScript on both sides" "$(failed "$(judge js p "$d/j/ref" "$d/j/new" "")")"
  good "$d/v/ref"; good "$d/v/new"
  plant "a jar's build without its entries on both sides" "$(failed "$(judge jvm p "$d/v/ref" "$d/v/new" "")")"
  plant "a timeout on the reference's side" "$(failed "$(judge js p "$d/c/ref" "$d/c/new" " ref:124")")"
  plant "a timeout on the new side" "$(failed "$(judge js p "$d/c/ref" "$d/c/new" " new:124")")"
  plant "a death by a signal on both sides" "$(failed "$(judge js p "$d/c/ref" "$d/c/new" " ref:137 new:137")")"
  good "$d/w/ref"; good "$d/w/new"; printf 'y\n' > "$d/w/new/out.js"
  plant "a wrong byte" "$(failed "$(judge js p "$d/w/ref" "$d/w/new" "")")"
  mkdir -p "$d/k/ref/p" "$d/k/new/p"
  # exact <what> <the lines the coverage must give, and no other> <inventory> <results>
  exact() {
    total=$((total + 1))
    if [ "$(coverage "$3" "$4" "$d/k")" = "$2" ]; then caught=$((caught + 1)); else missed="$missed; $1"; fi
  }
  printf 'p\nq\n' > "$d/inv"; printf 'same p\nsame q\n' > "$d/res"
  [ -z "$(coverage "$d/inv" "$d/res" "$d/k")" ] || missed="$missed; the control's coverage"
  # a program that differs, one that did not run and one not run each have their one result: no line of the coverage
  printf 'same p\ndiffers js q\n' > "$d/res5"
  [ -z "$(coverage "$d/inv" "$d/res5" "$d/k")" ] || missed="$missed; the coverage of a program that differs"
  printf 'differs jvm p (did not run: ref:124)\nnotrun q jars missing: x.jar\n' > "$d/res6"
  [ -z "$(coverage "$d/inv" "$d/res6" "$d/k")" ] || missed="$missed; the coverage of a program that did not run beside one not run"
  printf 'p\nq\np\n' > "$d/inv2"
  exact "a program named twice" "a name of two programs: p" "$d/inv2" "$d/res"
  mkdir -p "$d/k/new/stray"
  exact "a stray program in the out directory" "outputs of no program: stray" "$d/inv" "$d/res"
  rm -r "$d/k/new/stray"
  printf 'same p\n' > "$d/res2"
  exact "a program of the inventory with no result" "no result of q" "$d/inv" "$d/res2"
  printf 'same p\nsame q\nsame r\n' > "$d/res3"
  exact "a result of no program" "a result of no program: r" "$d/inv" "$d/res3"
  printf 'same p\nsame q\ndiffers js q\n' > "$d/res4"
  exact "two results of one program" "several results of q" "$d/inv" "$d/res4"
  printf 'same p\ndiffers js r\n' > "$d/res7"
  exact "a differing result of no program beside a program with none" $'no result of q\na result of no program: r' "$d/inv" "$d/res7"
  # The canaries and the lists in each mode's form of results: planted lists passing p (the canary) and q, failing r.
  local saved_root=$lists_root mode n
  lists_root=$d/lists
  mkdir -p "$d/lists/tests/board/suites" "$d/lists/tests/cases"
  printf '# test\tverdict\tthe other ways\tthe first line of the difference\np\tpass\t-\t\nq\tpass\t-\t\nr\tfail\t-\t\n' > "$d/lists/tests/board/suites/cases.tsv"
  for n in p q r; do printf 'object %s\n' "$n" > "$d/lists/tests/cases/$n.scala"; done
  printf 'p\nq\nr\n' > "$d/inv3"
  # lists_case <mode> <programs failed alike> <canary built: yes or no>: judge_kind's return on js records so planted,
  # its lines in $d/case/out.
  lists_case() {
    local r=$d/case
    rm -rf "$r"
    mkdir -p "$r/rec" "$r/can"
    for n in p q r; do
      case " $2 " in *" $n "*) mkdir -p "$r/rec/$n" && printf 'error: no such class\nexit 1\n' > "$r/rec/$n/log.cmp" ;; *) good "$r/rec/$n" ;; esac
      case $1 in direct) echo "same $n" ;; compare) echo "same js $n" ;; produce) echo "built js $n" ;; esac
    done > "$r/res"
    if [ "$3" = yes ]; then good "$r/can/p"; else mkdir -p "$r/can/p" && printf 'error: cannot open scala-library.jar\nexit 1\n' > "$r/can/p/log.cmp"; fi
    judge_kind js "$r/res" "$r/rec" "$d/inv3" "$r/can" "the reference binary" > "$r/out"
    echo $?
  }
  for mode in direct produce compare; do
    [ "$(lists_case $mode r yes)" = 0 ] && [ "$(cat "$d/case/out")" = "counts 2 1 1" ] || missed="$missed; the $mode control of the lists"
    [ "$(lists_case $mode "p q r" no)" = 2 ] && grep -q '^VOID identity js: the canary p does not build with the reference binary: error: cannot open' "$d/case/out" &&
      plant "a $mode run of every build failing alike" 1 || plant "a $mode run of every build failing alike" ""
    [ "$(lists_case $mode r no)" = 2 ] && grep -q '^VOID identity js: the canary p ' "$d/case/out" &&
      plant "a $mode canary that does not build" 1 || plant "a $mode canary that does not build" ""
    [ "$(lists_case $mode "q r" yes)" = 1 ] && [ "$(cat "$d/case/out")" = $'counts 1 2 1\nbeyond q' ] &&
      plant "a $mode failure alike one above the lists'" 1 || plant "a $mode failure alike one above the lists'" ""
  done
  : > "$d/lists/tests/board/suites/cases.tsv"
  [ "$(lists_case direct r yes)" = 2 ] && grep -q '^VOID identity js: the lists name no canary' "$d/case/out" &&
    plant "lists that name no canary" 1 || plant "lists that name no canary" ""
  lists_root=$saved_root
  rm -rf "${d:?}"
  [ -z "$missed" ] || echo "FAIL identity self-test:${missed#;} passed"
  echo "identity self-test: $caught of $total planted bad records failed"
  [ -z "$missed" ]
}
if [ "$across" = self-test ]; then
  cd "$(dirname "$0")/../.."
  self_test out/identity-self-test
  exit $?
fi
# The jar cache left unused: its rows in that report (caches written, files read from it) say which run warmed it.
export TEQ_NO_CACHE=1
[ -z "$across" ] || . "$(dirname "$0")/identity-across.sh"
[ $# -ge 3 ] || { echo "usage: tests/support/identity.sh <reference teq> <teq> <out dir> [js,jvm,diags,flagged] | --produce <teq> <bundle dir> [kinds] | --compare <bundle dir> <teq> <out dir>" >&2; exit 2; }
ref=$(absolute "$1") new=$(absolute "$2") out=$(absolute "$3")
# what every comparison runs: without it nothing is compared, which is no identity
for tool in timeout cmp diff; do
  command -v $tool > /dev/null || { echo "identity: no $tool on the PATH: nothing compared" >&2; exit 2; }
done
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
# run_side <kind> <src> <side> <dir>: the program by one side's binary with the flags the suites give it (jars_of
# called first), its record in <dir>: log.cmp ending in the binary's `exit <code>`, and the JavaScript or the jar's
# entries; returns the code.
run_side() {
  local kind=$1 src=$2 side=$3 dir=$4 bin own flags cp=$JARS_CP code
  bin=${!side}
  flags=$(grep -h -o '^// teq: .*' "$src" "$src"/*.scala 2> /dev/null | head -1 | sed 's|^// teq: ||')
  case $kind in
    jvm) cp=$sl${cp:+:$cp}; flags="$flags --std=scala-library" ;;
  esac
  [ -n "$cp" ] && flags="$flags --classpath $cp"
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
    jvm) timeout 60 "$bin" $(compiler_words "$bin") build "$src" $flags $own --target jvm -o "$dir/out.jar" > "$dir/log" 2>&1 ;;
    diags | flagged) timeout 60 "$bin" $(compiler_words "$bin") check "$src" $flags $own > "$dir/log" 2>&1 ;;
  esac
  code=$?
  [ ! -f "$dir/out.jar" ] || jar_entries "$dir/out.jar" "$dir/entries.txt"
  echo "exit $code" >> "$dir/log"
  compare_form "$dir/log" "$dir/log.cmp"
  rm "$dir/log"
  return $code
}
one() {
  local kind=$1 src=$2
  local name
  name=$(basename "$src" .scala)
  [ "${src#tests/errors/}" != "$src" ] && name=errors-$name
  . tests/support/jars.sh
  jars_of "$src"
  # not run: no program of identical outputs, counted apart
  [ -z "$JARS_MISSING" ] || { echo "notrun $name jars missing:$JARS_MISSING"; return 0; }
  local side code launch=
  for side in ref new; do
    run_side "$kind" "$src" "$side" "$out/$kind/$side/$name"
    code=$?
    # a launch that failed (126, 127), a limit (124, 125) or a signal (128 and up) ran no build: never identical
    case $code in 124 | 125 | 126 | 127 | 1[3-9][0-9] | 2[0-9][0-9]) launch="$launch $side:$code" ;; esac
  done
  judge "$kind" "$name" "$out/$kind/ref/$name" "$out/$kind/new/$name" "$launch"
}
export -f run_side one compiler_words _new_spelling
self_test "$out/self-test" || exit 1
skips=
[ -z "${IDENTITY_SKIPS:-}" ] || skips=$(absolute "$IDENTITY_SKIPS")
status=0
for kind in ${kinds//,/ }; do
  case $kind in
    js | jvm) srcs=$(ls -d tests/cases/*.scala tests/cases/*/ 2> /dev/null | sed 's|/$||') ;;
    diags | flagged) srcs=$(ls -d tests/cases/*.scala tests/cases/*/ tests/errors/*.scala tests/errors/*/ 2> /dev/null | sed 's|/$||') ;;
    *) echo "identity: no kind $kind" >&2; exit 2 ;;
  esac
  n=$(echo "$srcs" | wc -l | tr -d ' ')
  for src in $srcs; do
    case $src in tests/errors/*) echo "errors-$(basename "$src" .scala)" ;; *) basename "$src" .scala ;; esac
  done > "$out/$kind.inventory"
  canary=$(canary_of "$kind")
  if [ -n "$canary" ]; then
    jars_of "tests/cases/$canary.scala"
    run_side "$kind" "tests/cases/$canary.scala" ref "$out/canary/$kind/$canary"
  fi
  void=$(void_line "$kind" "$canary" "$out/canary/$kind/$canary" "the reference binary")
  [ -z "$void" ] || { echo "$void"; status=1; continue; }
  echo "$srcs" | xargs -P "$JOBS" -I{} bash -c "one $kind {}" > "$out/$kind.results"
  sed -n 's/^differs //p' "$out/$kind.results" | sort > "$out/$kind.differ"
  sed -n 's/^notrun //p' "$out/$kind.results" | sort > "$out/$kind.notrun"
  s=$(grep -c '^same ' "$out/$kind.results") d=$(wc -l < "$out/$kind.differ" | tr -d ' ') nr=$(wc -l < "$out/$kind.notrun" | tr -d ' ')
  sed 's/^/differs: /' "$out/$kind.differ"
  unlisted=0
  while read -r name why; do
    if [ -n "$skips" ] && grep -qx "$kind $name" "$skips"; then
      echo "not run $kind $name: $why (listed in IDENTITY_SKIPS)"
    else
      echo "not run $kind $name: $why"
      unlisted=$((unlisted + 1))
    fi
  done < "$out/$kind.notrun"
  [ "$nr" = 0 ] || echo "skip $nr programs of $kind: their jars are not in the coursier cache"
  faults=$(coverage "$out/$kind.inventory" "$out/$kind.results" "$out/$kind")
  [ -z "$faults" ] || { sed "s/^/FAIL identity $kind: /" <<< "$faults"; status=1; }
  verdict=$(judge_kind "$kind" "$out/$kind.results" "$out/$kind/ref" "$out/$kind.inventory" "$out/canary/$kind" "the reference binary") || status=1
  case $verdict in VOID*) echo "$verdict"; continue ;; esac
  read -r _ built failed allowed <<< "$verdict"
  echo "identity $kind: $s of $n programs identical ($built built on both sides, $failed failed alike of $allowed the lists allow), $d differing, $nr not run$([ "$nr" = 0 ] || echo " (jars missing)")"
  over=$(grep -c '^beyond ' <<< "$verdict")
  [ "$over" = 0 ] || echo "FAIL identity $kind: $over programs failed alike that the lists say build: $(first_ten "$verdict")"
  [ "$d" = 0 ] || status=1
  if [ "$unlisted" != 0 ]; then
    status=1
    [ -z "${TEQ_GATE:-}" ] || echo "FAIL identity $kind: $unlisted programs not run for want of their jars, a gate's want of what it needs"
  fi
done
exit $status
