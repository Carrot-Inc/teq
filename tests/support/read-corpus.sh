#!/bin/bash
# tests/support/read-corpus.sh: the read gate over the corpora. Every program of
# tests/cases, tests/split and tests/analysis/cases without jars, built in the product mode (`teq
# check --products`, a Scala.js module's check build, its `// teq:` flags kept but the target's),
# read by scalac 3.8.4 with every right-hand side forced and the tree checker after readTasty
# (tests/tasty/fromtasty/Gate.scala's `readjs`, scalajs-library beside the pickles). Prints a line
# per program that does not build or does not read, then a summary; exits 1 on a program that builds
# and does not read.
cd "$(dirname "$0")/../.."
teq=${TEQ:-./target/release/teq}
teq=$(cd "$(dirname "$teq")" && pwd)/$(basename "$teq")
. tests/support/jars.sh
sjs=$(jar_of scalajs-library)
work=$(mktemp -d "${TMPDIR:-/tmp}/read-corpus.XXXXXX") || exit 1
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/p" "$work/jobs.d" "$work/why" "$work/logs"
for src in tests/cases/*.scala tests/cases/*/ tests/split/*/ tests/analysis/cases/*/; do
  src=${src%/}
  name=$(basename "$src" .scala)
  case $src in tests/split/*) name="split-$name" ;; tests/analysis/*) name="analysis-$name" ;; esac
  printf '%s\t%s\n' "$name" "$src"
done > "$work/names"
build() {
  name=$1; src=$2; work=$3; teq=$4; sjs=$5
  if grep -q -h '^// jars:\|^// teq:.*--target interp' "$src" "$src"/*.scala 2> /dev/null; then
    echo "jars or the interpreter" > "$work/why/$name"; return
  fi
  flags=$(grep -h -o '^// teq: .*' "$src" "$src"/*.scala 2> /dev/null | head -1 | sed 's|^// teq: ||')
  kept=()
  for f in $flags; do case $f in --target | jvm | js | interp | --all-mains) ;; *) kept+=("$f") ;; esac; done
  if ! timeout 60 "$teq" compiler check --products "$work/p/$name" "$src" "${kept[@]}" > "$work/logs/$name.log" 2>&1; then
    why=$(grep -m1 'error' "$work/logs/$name.log" || head -1 "$work/logs/$name.log")
    echo "the build fails: $(cut -c1-160 <<< "$why")" > "$work/why/$name"; return
  fi
  tastys=$(find "$work/p/$name" -name '*.tasty' | LC_ALL=C sort | tr '\n' '\t' | sed 's/\t$//')
  [ -n "$tastys" ] || { echo "no pickles" > "$work/why/$name"; return; }
  printf 'readjs\t%s\t%s\t%s\n' "$name" "$work/p/$name:$sjs" "$tastys" > "$work/jobs.d/$name"
}
export -f build
cpus=$(getconf _NPROCESSORS_ONLN 2> /dev/null || echo 4)
export RC_WORK=$work RC_TEQ=$teq RC_SJS=$sjs
tr '\t' '\n' < "$work/names" | xargs -n 2 -P "$cpus" bash -c 'build "$1" "$2" "$RC_WORK" "$RC_TEQ" "$RC_SJS"' _
cat "$work"/jobs.d/* > "$work/jobs" 2> /dev/null
parts=$(( cpus / 4 )); [ $parts -lt 1 ] && parts=1; [ $parts -gt 6 ] && parts=6
python3 -c "
l = open('$work/jobs').read().splitlines()
for i in range($parts): open('$work/part.%d' % i, 'w').write(''.join(x + '\n' for x in l[i::$parts]))"
root=$PWD
for i in $(seq 0 $((parts - 1))); do
  (cd "$work" && COURSIER_MODE=offline timeout 330 scala-cli --power run -S 3.8.4 --jvm system --server=false --offline -q \
    --java-opt -Xss64m "$root/tests/tasty/fromtasty/Gate.scala" --dep org.scala-lang:scala3-compiler_3:3.8.4 -- "part.$i" > "part.$i.out" 2> "part.$i.err") &
done
wait
cat "$work"/part.*.out > "$work/read.out" 2> /dev/null
read_ok=0; read_fail=0; built=0; left=0
while IFS=$'\t' read -r name src; do
  if [ -f "$work/why/$name" ]; then
    left=$((left + 1))
    echo "left $name: $(cat "$work/why/$name")"
  elif grep -qxF "ok $name" "$work/read.out"; then
    read_ok=$((read_ok + 1))
  else
    read_fail=$((read_fail + 1))
    echo "FAIL $name: scalac's read"
    awk -v j="FAIL $name" '$0 == j { on = 1; next } /^(ok|FAIL) / { on = 0 } on' "$work/read.out" | head -3
  fi
done < "$work/names"
echo "read-corpus: $read_ok read clean, $read_fail refused, $left left out"
[ $read_fail = 0 ]
