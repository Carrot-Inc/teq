#!/bin/bash
# Runtime performance of the output. Each program under bench/runtime is compiled by teq (dev,
# --release and --target jvm), by Scala.js 1.20.1 through scala-cli (dev and full link, as a
# script and as an ES module) and by scalac 3.8.4 (plain and with -opt); the outputs of all nine
# are checked to be byte-identical to scalac's; then each artifact runs N times (--runs, default
# 5) under node or java, java both with the flags `run_jvm` passes (tests/support/jars.sh) and with the JVM's defaults.
# A run records the wall time of the process, the program's own first-iteration and steady-state
# times (the mean of the second half of its K iterations, printed on stderr), the peak RSS, and
# one extra logged run counts the garbage collections. Every invocation is bounded by `timeout`.
#
# Artifacts and results live under out/runtime/<size>/<program>/; a call builds only what is
# missing or older than its source or the teq binary, and runs only what has no row in
# results.tsv yet (--fresh reruns everything selected), so a call that stops can be repeated:
# a call stops on its own before starting a program once it has run for --budget seconds
# (default 240) and says so. The matrix can also be split with --only <program|toolchain>
# (repeatable; toolchains: teq-dev teq-release sjs-dev sjs-full sjs-es-dev sjs-es-full teq-jvm
# scalac scalac-opt, or the groups teq, sjs, jvm, node). --quick runs once at the small size
# that tests/cases runs, otherwise the full size is measured; --build-only builds and checks the
# artifacts and runs nothing. The
# sizes are in the table below: a program has its rounds and its iterations K rewritten into its
# `val defaults` line, since Scala.js gives main no arguments. bench/runtime.py aggregates the
# results into out/runtime/report.md (--report-only rewrites it without running anything).
# --std <mode> adds --std=<mode> to the JavaScript invocations of teq (lean, the default, adds
# nothing); teq-jvm links against scala-library 3.8.4's jar, the JVM's one mode, pinned on its
# class path and beside it under java. --out <dir> changes the output directory, which
# tests/runtime.sh uses to keep its runs apart.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
scala_version=3.8.4
sjs_version=1.20.1
teq_java_flags="-Xss512m -Xshare:auto"
# The pinned scala-library jar teq-jvm links against: in $COURSIER_CACHE, or in either
# conventional coursier cache (macOS's, then the XDG one), as `teq` itself looks.
scala_library=
for cache in ${COURSIER_CACHE:+"$COURSIER_CACHE"} "$HOME/Library/Caches/Coursier/v1" "$HOME/.cache/coursier/v1"; do
  jar=$cache/https/repo1.maven.org/maven2/org/scala-lang/scala-library/$scala_version/scala-library-$scala_version.jar
  if [ -f "$jar" ]; then
    scala_library=$jar
    break
  fi
done
build_timeout=300
run_timeout=120

# program, quick rounds, quick K, full rounds, full K; quick is about 25 ms per iteration under node
sizes="
adts          50 6   250 10
boxing       120 6   600 10
closures     100 6   500 10
codec        200 6  1000 10
collections  200 6  1000 10
dispatch      80 6   400 10
numeric      150 6   750 10
strings      200 6  1000 10
toplevel     360 6  1800 10
"

runs=5
size=full
only=()
std=lean
std_flag=""
out=out/runtime
report_only=0
build_only=0
budget=240
started=$(date +%s)
while [ $# -gt 0 ]; do
  case $1 in
    --quick) size=quick; runs=1 ;;
    --runs) runs=$2; shift ;;
    --only) only+=("$2"); shift ;;
    --std) std=$2; std_flag="--std=$2"; shift ;;
    --out) out=$2; shift ;;
    --fresh) export FRESH=1 ;;
    --budget) budget=$2; shift ;;
    --report-only) report_only=1 ;;
    --build-only) build_only=1 ;;
    *) echo "usage: $0 [--quick] [--runs N] [--only program|toolchain]... [--std mode] [--out dir] [--fresh] [--budget seconds] [--build-only] [--report-only]"; exit 2 ;;
  esac
  shift
done

selected() {
  [ ${#only[@]} = 0 ] && return 0
  local name
  for name in "${only[@]}"; do
    case $1 in
      "$name") return 0 ;;
    esac
    case "$name" in
      teq) case $1 in teq-dev|teq-release|teq-jvm) return 0 ;; esac ;;
      sjs) case $1 in sjs-*) return 0 ;; esac ;;
      jvm) case $1 in teq-jvm|scalac|scalac-opt) return 0 ;; esac ;;
      node) case $1 in teq-dev|teq-release|sjs-*) return 0 ;; esac ;;
    esac
  done
  return 1
}
selected_program() {
  [ ${#only[@]} = 0 ] && return 0
  local name
  for name in "${only[@]}"; do
    [ "$name" = "$1" ] && return 0
  done
  return 1
}
any_toolchain_selected() {
  local t
  for t in teq-dev teq-release sjs-dev sjs-full sjs-es-dev sjs-es-full teq-jvm scalac scalac-opt; do
    selected "$t" && return 0
  done
  return 1
}
# With --only naming only toolchains every program is selected, and the other way round.
programs_filtered=0
for p in $(echo "$sizes" | awk 'NF { print $1 }'); do selected_program "$p" && programs_filtered=1; done
[ $programs_filtered = 0 ] && selected_program() { return 0; }
any_toolchain_selected || selected() { return 0; }

machine="$(uname -m) $(sysctl -n machdep.cpu.brand_string 2> /dev/null || grep -m1 'model name' /proc/cpuinfo 2> /dev/null | cut -d: -f2- | sed 's/^ *//') $(sysctl -n hw.ncpu 2> /dev/null || nproc 2> /dev/null) cores"
node_version=$(node --version)
java_version=$(java -version 2>&1 | head -1)
scala_cli_version=$(scala-cli version 2> /dev/null | head -1)
teq_rev=$(git rev-parse --short HEAD 2> /dev/null)

write_report() {
  python3 bench/runtime.py report "$out" "$out/report.md" \
    "machine=$machine" "node=$node_version" "java=$java_version" "scala-cli=$scala_cli_version" \
    "scalac=$scala_version" "Scala.js=$sjs_version" "teq=$teq_rev ($TEQ, --std=$std on JavaScript, the JVM linked against scala-library $scala_version)" \
    "scalac -opt=-opt -opt-inline:**" "quick size (rounds×K)=$(echo "$sizes" | awk 'NF { printf "%s%s %s×%s", (NR > 2 ? ", " : ""), $1, $2, $3 }')" \
    "full size (rounds×K)=$(echo "$sizes" | awk 'NF { printf "%s%s %s×%s", (NR > 2 ? ", " : ""), $1, $4, $5 }')"
}
if [ $report_only = 1 ]; then
  write_report
  exit
fi

# Fresh when missing, or older than the source or (for teq's own output) the compiler.
stale() {
  local artifact=$1 src=$2 tool=$3
  [ ! -s "$artifact" ] || [ "$src" -nt "$artifact" ] || { [ -n "$tool" ] && [ "$tool" -nt "$artifact" ]; }
}

build() {
  local log=$1
  shift
  if ! timeout $build_timeout "$@" > "$log" 2>&1; then
    echo "FAIL build: $* (see $log)"
    return 1
  fi
}

# One timed run per index and one logged run for the GC count; the program's row names the
# size, the program and the toolchain, and its stdout is checked against scalac's output.
run() {
  local toolchain=$1 flags=$2
  shift 2
  local i
  for i in $(seq 1 "$runs"); do
    python3 bench/runtime.py measure "$results" "$size" "$program" "$toolchain" "$flags" "$i" "$expected" $run_timeout -- "$@" || failed=1
  done
}
gc() {
  local toolchain=$1 flags=$2
  shift 2
  python3 bench/runtime.py gc "$results" "$size" "$program" "$toolchain" "$flags" $run_timeout -- "$@" || failed=1
}

failed=0
stopped=
for line in $(echo "$sizes" | awk 'NF { print $1 ":" $2 ":" $3 ":" $4 ":" $5 }'); do
  IFS=: read -r program quick_rounds quick_k full_rounds full_k <<< "$line"
  selected_program "$program" || continue
  if [ $(( $(date +%s) - started )) -ge "$budget" ]; then
    echo "stopped before $program after $(( $(date +%s) - started )) s (--budget $budget); call again to continue"
    stopped=1
    break
  fi
  if [ $size = quick ]; then rounds=$quick_rounds; k=$quick_k; else rounds=$full_rounds; k=$full_k; fi
  dir=$out/$size/$program
  mkdir -p "$dir"
  results=$out/$size/results.tsv
  src=$dir/$program.scala
  # The copy differs from bench/runtime/<program>.scala in its defaults line only; it is
  # rewritten only when its content changes, so the artifacts' timestamps stay meaningful.
  generated=$(sed "s/^  val defaults = \".*\"$/  val defaults = \"$rounds $k 1\"/" "bench/runtime/$program.scala")
  if [ ! -f "$src" ] || [ "$generated" != "$(cat "$src")" ]; then
    printf '%s\n' "$generated" > "$src"
  fi
  grep -q "val defaults = \"$rounds $k 1\"" "$src" || { echo "FAIL $program: no defaults line to rewrite"; failed=1; continue; }

  # Builds, in the order of their cost.
  if selected teq-dev && stale "$dir/teq-dev.js" "$src" "$TEQ"; then
    build "$dir/teq-dev.build.log" "$TEQ" compiler build "$src" $std_flag -o "$dir/teq-dev.js" || failed=1
  fi
  if selected teq-release && stale "$dir/teq-release.js" "$src" "$TEQ"; then
    build "$dir/teq-release.build.log" "$TEQ" compiler build "$src" $std_flag --release -o "$dir/teq-release.js" || failed=1
  fi
  if selected teq-jvm && [ -z "$scala_library" ]; then
    echo "FAIL $program teq-jvm: scala-library $scala_version is in no coursier cache (\$COURSIER_CACHE, ~/Library/Caches/Coursier/v1, ~/.cache/coursier/v1)"
    failed=1
  elif selected teq-jvm && stale "$dir/teq-jvm.jar" "$src" "$TEQ"; then
    build "$dir/teq-jvm.build.log" "$TEQ" compiler build "$src" --classpath "$scala_library" --target jvm -o "$dir/teq-jvm.jar" || failed=1
  fi
  # scalac's output is the reference the other artifacts have to reproduce byte for byte. At the
  # quick size the rounds are the program's own defaults, so the checksum is the one
  # tests/cases holds from scala-cli, and scalac's jar is built only when it is measured.
  expected=$dir/expected.out
  default_rounds=$(sed -n 's/^  val defaults = "\([0-9]*\) .*/\1/p' "bench/runtime/$program.scala")
  if [ $size = quick ] && [ "$default_rounds" = "$rounds" ] && [ -s "tests/cases/bench_$program.expected" ]; then
    cp "tests/cases/bench_$program.expected" "$expected"
  fi
  if { selected scalac || [ ! -s "$expected" ]; } && stale "$dir/scalac.jar" "$src"; then
    build "$dir/scalac.build.log" scala-cli --power package -S $scala_version --server=false "$src" --assembly --preamble=false -o "$dir/scalac.jar" -f || failed=1
  fi
  if selected scalac-opt && stale "$dir/scalac-opt.jar" "$src"; then
    build "$dir/scalac-opt.build.log" scala-cli --power package -S $scala_version --server=false "$src" --assembly --preamble=false -o "$dir/scalac-opt.jar" -f -O -opt -O '-opt-inline:**' || failed=1
  fi
  for variant in sjs-dev:dev:none:js sjs-full:full:none:js sjs-es-dev:dev:es:mjs sjs-es-full:full:es:mjs; do
    IFS=: read -r name mode kind ext <<< "$variant"
    selected "$name" || continue
    stale "$dir/$name.$ext" "$src" || continue
    build "$dir/$name.build.log" scala-cli --power package -S $scala_version --js-version $sjs_version --server=false --js --js-mode $mode --js-module-kind $kind "$src" -o "$dir/$name.$ext" -f || failed=1
  done

  if [ ! -s "$expected" ] || { [ -s "$dir/scalac.jar" ] && [ "$dir/scalac.jar" -nt "$expected" ]; }; then
    timeout $run_timeout java -Xss512m -jar "$dir/scalac.jar" > "$expected" 2> "$dir/scalac.expected.err" || { echo "FAIL $program: scalac's own output failed (see $dir/scalac.expected.err)"; failed=1; continue; }
  fi

  [ $build_only = 1 ] && continue
  for name in teq-dev teq-release sjs-dev sjs-full sjs-es-dev sjs-es-full; do
    selected "$name" || continue
    ext=js
    case $name in sjs-es-*) ext=mjs ;; esac
    [ -s "$dir/$name.$ext" ] || continue
    run "$name" node node "$dir/$name.$ext"
    gc "$name" node node --trace-gc "$dir/$name.$ext"
  done
  for name in teq-jvm scalac scalac-opt; do
    selected "$name" || continue
    [ -s "$dir/$name.jar" ] || continue
    [ $name = teq-jvm ] && [ -z "$scala_library" ] && continue
    if [ $name = teq-jvm ]; then main="-cp $dir/$name.jar:$scala_library TeqMain"; else main="-jar $dir/$name.jar"; fi
    run "$name" teq java $teq_java_flags $main
    gc "$name" teq java $teq_java_flags -Xlog:gc $main
    run "$name" default java $main
    gc "$name" default java -Xlog:gc $main
  done
done
write_report
[ -n "$stopped" ] && exit 3
exit $failed
