#!/bin/bash
# The determinism of the parallel typer: every program of
# tests/split and the application corpus of bench/app (its frontend, the same sources as the sbt
# example builds them with one module per file for the pages and components, both with the class
# catalog declared a cache as the application declares its own, without which every attempt above
# one worker gives way, and its API side for JavaScript, for the JVM and under the interpreter) is
# built with each thread count of
# THREADS and, unless FORK=0, by one worker through the fork and the merge (`TEQ_FORK=1`), and
# the builds have to be byte-identical with one another: the modules or class files, what the
# program prints, and the diagnostics. THREADS is 1 by default, one worker and its forked path, the
# gate's; `THREADS=1,2,16,8` adds two and sixteen workers and the automatic count's cap, and `auto`
# among them builds with no count, the one the binary selects by itself. With REF=<binary> the builds are compared with that binary's build
# at one worker as well, which stands for the reference. A program's `// teq:` line gives its flags and its `// jars:` line puts those jars on
# its class path (tests/support/jars.sh); without one of them in the coursier cache the program
# counts as passed, and so does the corpus without the jars it reads. Every forked build sweeps
# its merged program for a worker's id left (`TEQ_MERGE_SWEEP=1`), and
# the assertion-enabled build checks the parallel merge against the serial walk's order
# (`TEQ_MERGE_TRACE=1`), which the release build ignores.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
export TEQ_MERGE_SWEEP=${TEQ_MERGE_SWEEP:-1}
export TEQ_MERGE_TRACE=${TEQ_MERGE_TRACE:-1}
THREADS=${THREADS:-1}
FORK=${FORK:-1}
. tests/support/jars.sh
. tests/support/compiler-words.sh
spellings "$TEQ" ${REF:+"$REF"}
work=out/determinism
rm -rf "$work"
mkdir -p "$work"
pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() {
  echo "FAIL $1"
  fail=$((fail + 1))
}
counts=(${THREADS//,/ })

# The diagnostics of a build, without what depends on the run: the times.
diagnostics() {
  grep -v -E '^(checked|built|ran) .* in |^  (read|parse|type|reach|emit|write) ' "$1"
}

# same <what> <first> <others...>: every output equals the first, byte for byte.
same() {
  local what=$1 first=$2
  shift 2
  for other in "$@"; do
    if [ -d "$first" ]; then
      if ! diff -rq "$first" "$other" > "$work/diff.txt" 2>&1; then
        bad "$what: $first and $other differ: $(head -3 "$work/diff.txt" | tr '\n' ' ' | cut -c1-300)"
        return
      fi
    elif ! cmp -s "$first" "$other"; then
      bad "$what: $first and $other differ: $(diff "$first" "$other" | head -3 | tr '\n' ' ' | cut -c1-300)"
      return
    fi
  done
  ok
}

# builds <name> <kind> <args...>: one build per thread count (and the reference's), compared.
# The kind is `js` (--split), `jvm` (-o directory) or `interp` (run, the output compared).
builds() {
  local name=$1 kind=$2
  shift 2
  local outs=() errs=()
  local binaries=("$TEQ")
  local labels=()
  for t in "${counts[@]}"; do
    labels+=("t$t")
  done
  if [ "$FORK" = 1 ]; then
    labels+=(fork)
  fi
  if [ -n "$REF" ]; then
    labels+=(ref)
  fi
  local i=0
  for label in "${labels[@]}"; do
    local bin=$TEQ
    local threads=(--threads "${label#t}")
    local fork=0 unset=()
    if [ "$label" = ref ]; then
      bin=$REF
      threads=(--threads 1)
    elif [ "$label" = tauto ]; then
      threads=()
      unset=(-u TEQ_THREADS)
    elif [ "$label" = fork ]; then
      threads=(--threads 1)
      fork=1
    fi
    local out="$work/$name/$label"
    local err="$work/$name/$label.err"
    mkdir -p "$work/$name"
    case $kind in
      js) env "${unset[@]}" TEQ_FORK=$fork timeout 300 "$bin" $(compiler_words "$bin") build "$@" "${threads[@]}" --split "$out" > "$err" 2>&1 ;;
      jvm) env "${unset[@]}" TEQ_FORK=$fork timeout 300 "$bin" $(compiler_words "$bin") build "$@" "${threads[@]}" --target jvm -o "$out" > "$err" 2>&1 ;;
      interp) env "${unset[@]}" TEQ_FORK=$fork timeout 300 "$bin" $(interp_words "$bin") "$@" "${threads[@]}" > "$out" 2> "$err" ;;
    esac
    local code=$?
    if [ $code -ne 0 ] && [ "$kind" != interp ]; then
      bad "$name ($label): the build failed: $(grep -m1 -E 'error|panicked' "$err" | cut -c1-200)"
      return
    fi
    diagnostics "$err" > "$err.diags"
    outs+=("$out")
    errs+=("$err.diags")
    i=$((i + 1))
  done
  same "$name output" "${outs[@]}"
  same "$name diagnostics" "${errs[@]}"
}

STUB=tests/interop/scalajs-stub
for src in tests/split/*/; do
  src=${src%/}
  name=$(basename "$src")
  flags=$(grep -h -o '^// teq: .*' "$src"/*.scala 2>/dev/null | head -1 | sed 's|^// teq: ||')
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    echo "skip $name: not in the coursier cache:$JARS_MISSING"
    ok
    continue
  fi
  [ -n "$JARS_CP" ] && flags="$flags --classpath $JARS_CP"
  # A program without an expectation is a watch session's (tests/split/classpath,
  # tests/split/owned), built for the JVM below as its session builds it.
  [ -f "$src.expected" ] || continue
  builds "$name" js "$src" $flags
  if ! grep -q -- '--std=scala-library\|--module-per-file' <<< "$flags"; then
    builds "$name-interp" interp "$src" $flags
  fi
done

# tests/split/classpath is the JVM session's program of tests/check-watch.sh, tests/split/owned
# the one of tests/jvm-watch.sh, whose two roots have an entry point each.
if [ -e "$(jar_of scala-library)" ]; then
  builds classpath-jvm jvm tests/split/classpath --std=scala-library --classpath "$(jar_of scala-library)"
  builds owned-jvm jvm tests/split/owned/main tests/split/owned/test --std=scala-library --classpath "$(jar_of scala-library)" --all-mains
else
  echo "skip classpath-jvm and owned-jvm: scala-library is not in the coursier cache"
  ok
fi

cp=""
missing=""
for jar in scala-library cats-kernel cats-core sourcecode; do
  path=$(jar_of "$jar")
  [ -e "$path" ] || missing="$missing $jar"
  cp="$cp:$path"
done
cp=${cp#:}
if [ -n "$missing" ]; then
  echo "skip the application corpus: not in the coursier cache:$missing"
  ok
elif ! timeout 120 python3 bench/app/gen.py "$work/src" > "$work/gen.log" 2>&1; then
  bad "the corpus did not generate: $(tail -1 "$work/gen.log")"
else
  builds app-frontend js "$work/src/shared" "$work/src/frontend" --classpath "$cp" --cacheable-state meridian.web.css.Catalog
  builds app-frontend-sbt js "$work/src/shared" "$work/src/frontend" --classpath "$cp" --max-inlines 80 --module-per-file meridian.frontend.page,meridian.frontend.component --cacheable-state meridian.web.css.Catalog
  builds app-api js "$work/src/shared" "$work/src/api" --classpath "$cp"
  # The flag a JVM build implies since the lean JVM mode's retirement, for a REF from before it.
  builds app-api-jvm jvm "$work/src/shared" "$work/src/api" --classpath "$cp" --std=scala-library
  builds app-api-interp interp "$work/src/shared" "$work/src/api" --classpath "$cp"
fi

echo "split-determinism: $pass passed, $fail failed"
[ $fail -eq 0 ]
