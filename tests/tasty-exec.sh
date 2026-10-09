#!/bin/bash
# The execution gate: the programs of tests/cases, and of
# tests/tasty/exec/cases (programs teq's own builds do not run as scalac's do yet), that
# tests/tasty/exec/closure.txt lists, whose products hold every body written (no `ELIDED`),
# compiled to class files by scalac 3.8.4 from teq's TASTy alone (-from-tasty, the tree checker
# after every phase: tests/tasty/fromtasty/Gate.scala's `regen`) and run on the JVM with
# scala-library alone beside them (tests/tasty/exec/Launch.java runs the main
# tests/tasty/exec/main_of.py names), each printing its .expected, which scalac's own build
# printed. A listed program whose products withhold a body fails the line (the closure shrank);
# `tests/tasty-exec.sh --update` rewrites the list as every case without jars, Scala.js or the
# interpreter, of one main, that builds with every body written, regenerates and runs as
# expected, to be read against the previous list. Without scala-cli, java or javac a failure
# (incomplete validation).
cd "$(dirname "$0")/.."
teq=${TEQ:-./target/release/teq}
teq=$(cd "$(dirname "$teq")" && pwd)/$(basename "$teq")
update=0
[ "$1" = "--update" ] && update=1
for tool in scala-cli java javac; do
  command -v $tool > /dev/null || { echo "tasty-exec: no $tool (incomplete validation)"; echo "tasty-exec: 0 passed, 1 failed"; exit 1; }
done
. tests/support/jars.sh
lib=$(jar_of scala-library)
work=$(mktemp -d "${TMPDIR:-/tmp}/tasty-exec.XXXXXX") || exit 1
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/p" "$work/c" "$work/o" "$work/jobs.d" "$work/logs" "$work/launch"
javac -d "$work/launch" tests/tasty/exec/Launch.java || { echo "tasty-exec: Launch.java does not compile"; exit 1; }
closure=tests/tasty/exec/closure.txt
if [ $update = 1 ]; then
  for src in tests/cases/*.scala tests/cases/*/ tests/tasty/exec/cases/*.scala; do src=${src%/}; basename "$src" .scala; done > "$work/names"
else
  grep -v '^#' "$closure" > "$work/names"
fi
cpus=$(getconf _NPROCESSORS_ONLN 2> /dev/null || echo 4)
# Each program's products, its census and its regeneration job; a line in out.d says why one left.
build() {
  name=$1; work=$2; teq=$3
  dir=tests/cases
  [ -e "$dir/$name.scala" ] || [ -e "$dir/$name" ] || dir=tests/tasty/exec/cases
  src=$dir/$name.scala
  [ -f "$src" ] || src=$dir/$name
  [ -e "$src" ] || { echo "no case" > "$work/c/$name.why"; return; }
  if grep -q -h '^// jars:\|^//> using platform js\|^// teq:.*--target interp' "$src" "$src"/*.scala 2> /dev/null; then
    echo "jars, Scala.js or the interpreter" > "$work/c/$name.why"; return
  fi
  [ -f "$dir/$name.expected" ] || { echo "no expected output" > "$work/c/$name.why"; return; }
  echo "$dir/$name.expected" > "$work/c/$name.expected"
  main=$(python3 tests/tasty/exec/main_of.py "$src")
  [ -n "$main" ] || { echo "no main the source names alone" > "$work/c/$name.why"; return; }
  echo "$main" > "$work/c/$name.main"
  flags=$(grep -h -o '^// teq: .*' "$src" "$src"/*.scala 2> /dev/null | head -1 | sed 's|^// teq: ||')
  kept=()
  for f in $flags; do case $f in --target | jvm | js | --all-mains) ;; *) kept+=("$f") ;; esac; done
  if ! TEQ_BODIES_CENSUS=$work/c/$name.census timeout 60 "$teq" compiler check --products "$work/p/$name" "$src" "${kept[@]}" > "$work/logs/$name.log" 2>&1; then
    echo "the build fails" > "$work/c/$name.why"; return
  fi
  withheld=$(awk -F'\t' '$2 ~ /^elided/ && $3 > 0 { print substr($2, 9) }' "$work/c/$name.census" | head -1)
  [ -z "$withheld" ] || { echo "a body withheld: $withheld" > "$work/c/$name.why"; return; }
  tastys=$(find "$work/p/$name" -name '*.tasty' | LC_ALL=C sort | tr '\n' '\t' | sed 's/\t$//')
  # The options the source gives scalac but its warnings' (`//> using option -Xmax-inlines 80`),
  # which the regeneration's expansions of its inline bodies need as its compilation did.
  options=$(grep -h -o -E '^//> using options? .*' "$src" "$src"/*.scala 2> /dev/null | sed -E 's|^//> using options? ||' | tr ' ' '\n' | grep -v '^-W' | tr '\n' '\t')
  mkdir -p "$work/o/$name"
  printf 'regen\t%s\t%s\t%s\t%s%s\n' "$name" "$work/p/$name" "$work/o/$name" "$options" "$tastys" > "$work/jobs.d/$name"
}
export -f build
xargs -P "$cpus" -I{} bash -c 'build "$@"' _ {} "$work" "$teq" < "$work/names"
cat "$work"/jobs.d/* > "$work/jobs" 2> /dev/null
parts=$(( cpus / 4 )); [ $parts -lt 1 ] && parts=1; [ $parts -gt 6 ] && parts=6
python3 -c "
l = open('$work/jobs').read().splitlines()
for i in range($parts): open('$work/part.%d' % i, 'w').write(''.join(x + '\n' for x in l[i::$parts]))"
root=$PWD
for i in $(seq 0 $((parts - 1))); do
  (cd "$work" && COURSIER_MODE=offline timeout 330 scala-cli --power run -S 3.8.4 --jvm system --server=false --offline -q \
    "$root/tests/tasty/fromtasty/Gate.scala" --dep org.scala-lang:scala3-compiler_3:3.8.4 -- "part.$i" > "part.$i.out" 2> "part.$i.err") &
done
wait
cat "$work"/part.*.out > "$work/regen.out" 2> /dev/null
run() {
  name=$1; work=$2; lib=$3
  grep -qxF "ok $name" "$work/regen.out" || return
  if timeout 20 java -XX:-UsePerfData -cp "$work/o/$name:$lib:$work/launch" Launch $(cat "$work/c/$name.main") > "$work/o/$name.out" 2> "$work/o/$name.err" \
      && cmp -s "$work/o/$name.out" "$(cat "$work/c/$name.expected")"; then
    touch "$work/c/$name.ran"
  fi
}
export -f run
xargs -P "$cpus" -I{} bash -c 'run "$@"' _ {} "$work" "$lib" < "$work/names"
pass=0
fail=0
: > "$work/closure"
while read -r name; do
  if [ -f "$work/c/$name.ran" ]; then
    pass=$((pass + 1))
    echo "$name" >> "$work/closure"
    continue
  fi
  # In --update, each case left out of the closure with its reason.
  if [ $update = 1 ]; then
    left="left"
  else
    left="FAIL"
    fail=$((fail + 1))
  fi
  if [ -f "$work/c/$name.why" ]; then
    echo "$left $name: $(cat "$work/c/$name.why")"
  elif ! grep -qxF "ok $name" "$work/regen.out"; then
    echo "$left $name: scalac's regeneration"
    [ $update = 1 ] || awk -v j="FAIL $name" '$0 == j { on = 1; next } /^(ok|FAIL) / { on = 0 } on' "$work/regen.out" | head -3
  else
    echo "$left $name: the regenerated program's run"
    [ $update = 1 ] || { diff "$work/o/$name.out" "$(cat "$work/c/$name.expected")" | head -5; head -3 "$work/o/$name.err"; }
  fi
done < "$work/names"
if [ $update = 1 ]; then
  { echo "# The programs of tests/cases and tests/tasty/exec/cases the execution gate runs (tests/tasty-exec.sh --update)."; cat "$work/closure"; } > "$closure"
  echo "tasty-exec: $pass programs in the closure"
  exit 0
fi
echo "tasty-exec: $pass passed, $fail failed"
[ $fail = 0 ]
