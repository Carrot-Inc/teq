#!/bin/bash
# tests/prop-known.sh <name>: whether the defect that an exclusion of proptests/src/known.rs keeps
# out of the generators is repaired, and the case that retires the exclusion.
#
# 1. With TEQ_PROP_KNOWN=<name>, the trigger put back, every property whose sources ask for the
#    name runs against TEQ (target/release/teq): KNOWN_INPUTS programs or histories (200) on each
#    of the seeds KNOWN_SEEDS ("1 2"), shrinking off, scalac asked about every program, in batches
#    of KNOWN_PROGRAMS (20). A failure means the defect stands: its report is printed and the
#    script exits 1.
# 2. On a pass, and with TEQ_BEFORE the binary before the repair, the same runs against TEQ_BEFORE
#    must fail (exit 1 if none does: the trigger does not reach the defect there), one program to
#    an input and scalac asked about a sample and every disagreement, and the first failure that
#    passes there once the trigger is out again (the trigger's, not another defect's) is
#    shrunk (a shrink run cut by its bound gives the smallest
#    failure it kept), and replayed once more with TEQ_PROP_REDUCE to reduce the program to its
#    failing cases. The shrunk input must pass on TEQ_BEFORE once the trigger is out again (the
#    shrinker may have traded the trigger's failure for another defect's of one kind), and on TEQ.
# 3. For a trigger of generated expressions, the shrunk program becomes tests/cases/prop_<name>.scala
#    with scalac 3.8.4's output as its .expected; a program that prints a floating value's text
#    (a `.t` line) is a Scala.js case (`//> using platform js`, the .expected Scala.js's) with the
#    JVM's output in tests/jvm-expected/, as the suites read them. The case is run on the three
#    targets and added to the interpreter's and the JVM's passing lists where it passes. A session
#    trigger shrinks to a history, which the script keeps and names; its case is a split-watch
#    scenario written by hand.
# 4. What is left by hand: the guard (the sites that ask for the name), the entry in known.rs, the
#    README's row and the task-list item, in one commit with the case.
#
# Exit 0 (RETIRABLE) only with the whole evidence: the witness, one shrunk input the trigger
# generates, fails on TEQ_BEFORE, passes there with the trigger out and passes on TEQ, and its
# case fails before and passes on the three targets against scalac's output. Exit 1: the defect stands (NOT REPAIRED), a run was cut (INCOMPLETE)
# or a step failed. Exit 3: the trigger passes but the script retires nothing (no TEQ_BEFORE, or
# a session trigger, whose witness is a history). The random runs on TEQ are coverage beside the
# witness: a pass there alone never retires an exclusion.
#
# Its work goes to KNOWN_DIR (out/prop-known/<name>); each run is bounded by KNOWN_BOUND (600 s),
# a shrink run by 420 s.
cd "$(dirname "$0")/.." || exit 1
repo=$(pwd)
name=$1
[ -n "$name" ] || { awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "$0"; exit 2; }
absolute() { case $1 in /*) echo "$1" ;; *) echo "$repo/$1" ;; esac; }
TEQ=$(absolute "${TEQ:-target/release/teq}")
[ -z "$TEQ_BEFORE" ] || TEQ_BEFORE=$(absolute "$TEQ_BEFORE")
work=$(absolute "${KNOWN_DIR:-out/prop-known/$name}")
inputs=${KNOWN_INPUTS:-200}
seeds=${KNOWN_SEEDS:-1 2}
programs=${KNOWN_PROGRAMS:-20}
bound=${KNOWN_BOUND:-600}
case_name=prop_$(echo "$name" | tr '-' '_')

sites=$(grep -n "asked(\"$name\")" proptests/src/*.rs proptests/tests/*.rs)
[ -n "$sites" ] || { echo "no exclusion named $name: nothing in proptests/ asks for it"; exit 2; }
# The tests that own a source that asks for the name, as <test file>:<test>.
owners() {
  local file
  for file in $(echo "$sites" | cut -d: -f1 | sort -u); do
    case $(basename "$file" .rs) in
      exprs|targets) echo targets:targets_agree ;;
      model|edits|driver|session) echo session:split_session_agrees_with_fresh; echo session:check_session_agrees_with_fresh ;;
      order) echo order:order_of_inputs_does_not_matter ;;
      fixture) for t in bodies_split bodies_check types_split types_check; do echo controls:fixture_$t; done ;;
      *) echo "unknown:$(basename "$file" .rs)" ;;
    esac
  done
}
tests=$(owners | sort -u)

rm -rf "$work"
mkdir -p "$work" || exit 1
cd proptests || exit 1
if ! timeout 300 cargo test --no-run --offline --locked > "$work/build.log" 2>&1; then
  echo "FAIL the crate does not build (see $work/build.log)"
  exit 1
fi
cat > "$work/hegel.toml" <<'EOF'
[profiles.discover]
extends = "base"
phases = ["explicit", "reuse", "generate", "target"]

[profiles.shrink]
extends = "base"
phases = ["reuse", "shrink"]

[profiles.replay]
extends = "base"
phases = ["reuse"]
EOF
binary_of() { sed -n "s|^ *Executable tests/$1\\.rs (\\(.*\\))\$|$repo/proptests/\\1|p" "$work/build.log" | head -1; }
panic_block() {
  awk '/panicked at/ && !seen { seen = 1; on = 1; next }
       on && (/^To reproduce this failure/ || /^note: run with/ || /^---- /) { exit }
       on { print }' "$1"
}

# run <dir> <binary> <file:test> <seed> <profile> [bound]: one run of a test with the trigger asked
# (or the triggers of `asked` where that is set, none where it is empty), keeping to the kind in
# `keep` where that is set and to none otherwise.
run() {
  local at=$1 teq=$2 file=${3%%:*} test=${3#*:} seed=$4 profile=$5 limit=${6:-$bound}
  local each=1 cases=$inputs
  [ "$file" != targets ] || { each=${each_input:-$programs}; cases=$(( inputs / each + 1 )); }
  mkdir -p "$at"
  local scalac=sampled
  [ "$each" = 1 ] || scalac=always
  TEQ=$teq TEQ_PROP_KNOWN=${asked-$name} TEQ_PROP_KEEP=${keep-} TEQ_PROP_REPLAYS=0 TEQ_PROP_SCALAC=$scalac TEQ_PROP_PROGRAMS=$each HEGEL_SEED=$seed HEGEL_TEST_CASES=$cases \
    HEGEL_DATABASE=$at/db TEQ_PROP_WORK=$at/work HEGEL_CONFIG=$work/hegel.toml HEGEL_DEFAULT_PROFILE=$profile \
    timeout -k 10 "$limit" "$(binary_of "$file")" "$test" --exact --test-threads=1 > "$at/log.txt" 2>&1
}

# requeue <database>: see tests/prop-campaign.sh, whose function this runs.
requeue() { "$repo/tests/prop-campaign.sh" requeue "$1" > /dev/null; }

echo "== $name: $(echo $tests | tr ' ' ',') with the trigger asked, $inputs inputs on seeds $seeds, against $TEQ"
for spec in $tests; do
  for seed in $seeds; do
    at=$work/after/${spec#*:}-$seed
    run "$at" "$TEQ" "$spec" "$seed" discover
    code=$?
    case $code in
      0) echo "   ${spec#*:} seed $seed: passes" ;;
      101)
        echo "NOT REPAIRED: ${spec#*:} on seed $seed fails ($at/log.txt):"
        panic_block "$at/log.txt" | head -${SHOWN:-24}
        exit 1 ;;
      124|137) echo "INCOMPLETE: ${spec#*:} on seed $seed was cut by its bound of $bound s ($at/log.txt)"; exit 1 ;;
      *) echo "ERROR: ${spec#*:} on seed $seed ended with $code ($at/log.txt)"; exit 1 ;;
    esac
  done
done
echo "the trigger passes on $TEQ, $inputs inputs on each seed: coverage beside the witness, which decides"
[ -n "$TEQ_BEFORE" ] || { echo "NOT RETIRED: no TEQ_BEFORE, so no witness failing before the repair"; exit 3; }

echo "== the same against $TEQ_BEFORE"
found=""
for spec in $tests; do
  for seed in $seeds; do
    at=$work/before/${spec#*:}-$seed
    each_input=1 run "$at" "$TEQ_BEFORE" "$spec" "$seed" discover
    code=$?
    case $code in
      0) echo "   ${spec#*:} seed $seed: passes" ;;
      101)
        # The failure is the trigger's only if it goes once the trigger is out again: a defect
        # of the binary before the repair that another retired exclusion kept out fails either way.
        mkdir -p "$at/unasked"
        cp -R "$at/db" "$at/unasked/db"
        asked= each_input=1 run "$at/unasked" "$TEQ_BEFORE" "$spec" "$seed" replay 300
        if [ $? = 0 ]; then found="$spec $seed $at"; break 2; fi
        echo "   ${spec#*:} seed $seed: fails there, but with the trigger out too ($at/log.txt): another defect's" ;;
      *) echo "   ${spec#*:} seed $seed: incomplete (exit $code, $at/log.txt)" ;;
    esac
  done
done
[ -n "$found" ] || { echo "FAIL the trigger does not fail on $TEQ_BEFORE either: raise KNOWN_INPUTS or give other KNOWN_SEEDS"; exit 1; }
set -- $found
spec=$1 seed=$2 at=$3
# The witness keeps the kind of the failure found: a failure of another kind counts as a pass
# while it is shrunk and reduced.
kind=$(panic_block "$at/log.txt" | sed -n "s/^the failure's kind: //p" | head -1)
echo "   ${spec#*:} seed $seed fails there (${kind:-no kind named}); shrinking"
shrunk=$work/shrunk
mkdir -p "$shrunk"
cp -R "$at/db" "$shrunk/db"
requeue "$shrunk/db"
keep=$kind each_input=1 run "$shrunk" "$TEQ_BEFORE" "$spec" "$seed" shrink 420
code=$?
case $code in
  101) panic_block "$shrunk/log.txt" > "$work/shrunk.txt" ;;
  124|137)
    last=$(ls -t "$shrunk"/work/*/failures/*/report.txt 2>/dev/null | head -1)
    [ -n "$last" ] || { echo "FAIL the shrink run was cut before it kept a failure ($shrunk/log.txt)"; exit 1; }
    echo "   the shrink run was cut by its bound; its smallest failure is taken"
    cp "$last" "$work/shrunk.txt" ;;
  *) echo "FAIL the shrink run ended with $code ($shrunk/log.txt)"; exit 1 ;;
esac
if [ "${spec%%:*}" = targets ]; then
  mkdir -p "$work/reduced"
  cp -R "$shrunk/db" "$work/reduced/db"
  keep=$kind TEQ_PROP_REDUCE=1 each_input=1 run "$work/reduced" "$TEQ_BEFORE" "$spec" "$seed" replay 300
  [ $? = 101 ] && panic_block "$work/reduced/log.txt" > "$work/shrunk.txt"
fi
sed 's/^/   /' "$work/shrunk.txt" | head -${SHOWN:-24}
# Shrinking may trade the trigger's failure for another defect's of the same kind: the final
# witness must still pass before the repair once the trigger is out again.
mkdir -p "$work/unasked"
cp -R "$shrunk/db" "$work/unasked/db"
asked= each_input=1 run "$work/unasked" "$TEQ_BEFORE" "$spec" "$seed" replay 300
code=$?
[ $code = 0 ] || { echo "NOT RETIRED: the shrunk witness fails on $TEQ_BEFORE with the trigger out too (exit $code, $work/unasked/log.txt): another defect's"; exit 1; }
echo "   the shrunk witness passes on $TEQ_BEFORE with the trigger out"
mkdir -p "$work/replayed"
cp -R "$shrunk/db" "$work/replayed/db"
each_input=1 run "$work/replayed" "$TEQ" "$spec" "$seed" replay 300
code=$?
[ $code = 0 ] || { echo "NOT REPAIRED: the witness fails on $TEQ too (exit $code, $work/replayed/log.txt)"; exit 1; }
echo "   the witness passes on $TEQ"

kept=$(sed -n 's/^the program: //p; s/^the history.s files: //p; s/^the program reduced to its failing cases: //p' "$work/shrunk.txt" | tail -1)
cd "$repo" || exit 1
ok=1
if [ "${spec%%:*}" != targets ]; then
  echo "== a history: $kept"
  echo "NOT RETIRED: the witness fails before the repair and passes after; its case is a split-watch scenario (tests/split/, tests/split-watch.sh), written by hand"
  exit 3
else
  source=tests/cases/$case_name.scala
  floating=$(grep -c 'show("[0-9]*\.[fr]\.t"' "$kept")
  {
    echo "// The trigger of \`$name\` in proptests/src/known.rs, shrunk by tests/prop-known.sh:"
    grep -v '^the program\|^input ' "$(dirname "$kept")/reduced.txt" 2>/dev/null | head -12 | sed 's|^|// |' | grep . \
      || grep -v '^the program\|^input ' "$work/shrunk.txt" | head -12 | sed 's|^|// |'
    [ "$floating" = 0 ] || echo "//> using platform js"
    cat "$kept"
  } > "$source"
  if [ "$floating" = 0 ]; then
    timeout 300 scala-cli run -S 3.8.4 --jvm system "$source" --server=false -q > "tests/cases/$case_name.expected" 2> "$work/ref.err" || { echo "FAIL scalac refuses $source ($work/ref.err)"; exit 1; }
    jvm_expected=tests/cases/$case_name.expected
  else
    mkdir -p "$work/jvm"
    grep -v '^//> using platform js' "$source" > "$work/jvm/$case_name.scala"
    timeout 300 scala-cli run -S 3.8.4 --jvm system "$work/jvm/$case_name.scala" --server=false -q > "tests/jvm-expected/$case_name.expected" 2> "$work/ref.err" || { echo "FAIL scalac refuses the program ($work/ref.err)"; exit 1; }
    timeout 300 scala-cli run -S 3.8.4 --jvm system "$source" --server=false -q > "tests/cases/$case_name.expected" 2> "$work/ref-js.err" || { echo "FAIL Scala.js refuses $source ($work/ref-js.err)"; exit 1; }
    jvm_expected=tests/jvm-expected/$case_name.expected
  fi
  echo "== $source, its expected output scalac's"
  listed() {
    local list=$1
    # In at its place, the list's other lines as they stand.
    grep -qx "$case_name" "$list" || { awk -v name="$case_name" '!done && $0 > name { print name; done = 1 } { print } END { if (!done) print name }' "$list" > "$work/list" && cp "$work/list" "$list"; }
  }
  . tests/support/jars.sh
  run_js 60 "$work/case.js" "$source" > "$work/case.js.out" 2>&1 && cmp -s "$work/case.js.out" "tests/cases/$case_name.expected" \
    && echo "   JavaScript: passes" || { ok=0; echo "   JavaScript: FAILS ($work/case.js.out)"; }
  if timeout 60 "$TEQ" interp "$source" > "$work/case.interp.out" 2>&1 && cmp -s "$work/case.interp.out" "$jvm_expected"; then
    listed tests/interp-passing.txt; echo "   interpreter: passes, listed"
  else ok=0; echo "   interpreter: FAILS ($work/case.interp.out)"; fi
  # As tests/run_jvm.sh runs a case, linked against scala-library's jar.
  jvm() {
    local out=$work/jvm-$1 list=$2; shift 2
    timeout 60 "$TEQ" compiler build "$source" "$@" --target jvm -o "$out.jar" > "$out.compile" 2>&1 \
      && timeout 60 java -Xss512m -XX:+UseSerialGC -cp "$out.jar${cp:+:$cp}" TeqMain > "$out.out" 2>&1 \
      && cmp -s "$out.out" "$jvm_expected" && listed "$list"
  }
  cp=$M2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar
  jvm link tests/jvm-passing.txt --classpath "$cp" && echo "   JVM: passes, listed" || { ok=0; echo "   JVM: FAILS ($work/jvm-link.*)"; }
fi
[ $ok = 1 ] || { echo "NOT RETIRED: the case fails on $TEQ"; exit 1; }
echo "== by hand, in one commit with the case: take the guard out"
echo "$sites" | sed 's/^/   /'
echo "   and the entry of \`$name\` in proptests/src/known.rs, its row in proptests/README.md and its task-list item"
echo "RETIRABLE: the witness fails on $TEQ_BEFORE and passes on $TEQ, and so does its case"
