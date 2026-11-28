#!/bin/bash
# inline-placeholder.sh <base teq> <empty out dir> [<head teq>]
# Each teq caller has its own compilation. Scalac can compile distinct packages in a batch.
# Run the self-test first. Exit 1 on a difference on the last binary or an invalid record.
set -eu
base=$(realpath "${1:?base teq}") out=$(realpath -m "${2:?out dir}") head=${3:+$(realpath "$3")}
here=$(cd "$(dirname "$0")" && pwd)
py=$here/inline_placeholder.py
cd "$here/../.."
mkdir -p "$out"
python3 -I "$py" selftest "$out"
[[ -z $(ls -A "$out") ]] || { echo "inline-placeholder: output directory must be empty" >&2; exit 2; }
sides=(base)
if [[ -n $head ]]; then sides+=(head); fi
for bin in "$base" ${head:+"$head"}; do
  [[ -x $bin ]] || { echo "inline-placeholder: no executable at $bin" >&2; exit 2; }
done
. tests/support/compiler-words.sh
spellings "$base" ${head:+"$head"}
. tests/support/jars.sh
sl=$(jar_of scala-library)
[[ -f $sl ]] || { echo "inline-placeholder: missing scala-library jar" >&2; exit 2; }
export TMPDIR=$out/tmp TEQ_CACHE_DIR=$out/cache
mkdir -p "$TMPDIR"
cd "$out"
python3 -I "$py" gen "$out"
mapfile -t cells < <(cut -f1 cells.tsv)

record() {
  local prefix=$1 seconds=$2 rc=0
  shift 2
  LC_ALL=C timeout -v -k 2 "$seconds" "$@" > "$prefix.out" 2> "$prefix.err" || rc=$?
  echo "$rc" > "$prefix.exit"
}

S=$HOME/.sbt/boot/scala-3.8.4
boots=("$S"/org.scala-sbt/sbt/*/)
lib=$sl
if [[ -d ${boots[0]} && -f $S/lib/scala3-compiler_3-3.8.4.jar ]]; then
  lib=$S/lib/scala-library.jar:$S/lib/scala3-library_3-3.8.4.jar
  scalac=(java -cp "$S/lib/*:${boots[0]}*" dotty.tools.dotc.Main)
else
  scalac=(scala-cli compile -S 3.8.4 --server=false)
fi

# If one source invalidates a batch, compile each cell on its own to attribute the refusal.
oracle_build() {
  local dir=$1 leg=$2
  shift 2
  local -a libs=("${@/%//Lib.scala}") mains=("${@/%//Main.scala}")
  local -a sources=("${libs[@]}" "${mains[@]}") stages=(build)
  local cp=$lib cell stage ext r
  mkdir -p "$dir/classes"
  if [[ $leg == jvm-split ]]; then
    mkdir -p "$dir/up"
    record "$dir/upstream" 300 "${scalac[@]}" -classpath "$lib" -d "$dir/up" "${libs[@]}"
    cp="$dir/up:$lib"
    sources=("${mains[@]}")
    stages=(upstream build)
  fi
  if [[ $leg == jvm-whole || $(< "$dir/upstream.exit") == 0 ]]; then
    record "$dir/build" 300 "${scalac[@]}" -classpath "$cp" -d "$dir/classes" "${sources[@]}"
  fi
  if [[ $# -gt 1 && ( ! -f $dir/build.exit || $(< "$dir/build.exit") != 0 ) ]]; then
    for cell in "$@"; do oracle_build "$dir/$cell" "$leg" "$cell"; done
    return
  fi
  for cell in "$@"; do
    r=$out/runs/scalac/$leg/$cell
    mkdir -p "$r"
    for stage in "${stages[@]}"; do
      [[ -f $dir/$stage.exit ]] || continue
      for ext in out err exit; do ln -s "$dir/$stage.$ext" "$r/$stage.$ext"; done
    done
    echo "$dir/classes:$cp" > "$r/classpath"
  done
}
for leg in jvm-whole jvm-split; do oracle_build "$out/scalac/$leg" "$leg" "${cells[@]}"; done

run_cell() {
  set -eu
  local cell=$1 leg r side bin cp
  local -a sources compiler interp
  cd "$out/$cell"
  for leg in jvm-whole jvm-split; do
    r=$out/runs/scalac/$leg/$cell
    if [[ -f $r/build.exit && $(< "$r/build.exit") == 0 ]]; then
      record "$r/run" 60 java -Xverify:all -cp "$(< "$r/classpath")" "$cell.Main"
    fi
  done
  for side in base ${head:+head}; do
    bin=$base
    if [[ $side == head ]]; then bin=$head; fi
    read -ra compiler <<< "$(compiler_words "$bin")"
    read -ra interp <<< "$(interp_words "$bin")"
    for leg in jvm-whole jvm-split js-whole interp-whole; do
      r=$out/runs/$side/$leg/$cell
      mkdir -p "$r"
      sources=(Lib.scala Main.scala)
      cp=$sl
      if [[ $leg == jvm-split ]]; then
        record "$r/upstream" 120 "$bin" "${compiler[@]}" build Lib.scala --target jvm --std=scala-library \
          --products "$r/products" --sourceroot . --classpath "$sl"
        [[ $(< "$r/upstream.exit") == 0 ]] || continue
        cp="$sl:$r/products"
        sources=(Main.scala)
      fi
      case $leg in
        jvm-*)
          record "$r/build" 120 "$bin" "${compiler[@]}" build "${sources[@]}" --target jvm --std=scala-library --classpath "$cp" -o "$r/x.jar"
          if [[ $(< "$r/build.exit") == 0 ]]; then record "$r/run" 60 java -Xverify:all -cp "$r/x.jar:$cp" "$cell.Main"; fi ;;
        js-whole)
          record "$r/build" 120 "$bin" "${compiler[@]}" build "${sources[@]}" --target js --classpath "$sl" -o "$r/x.js"
          if [[ $(< "$r/build.exit") == 0 ]]; then record "$r/run" 60 node "$r/x.js"; fi ;;
        interp-whole)
          record "$r/interp" 120 "$bin" "${interp[@]}" "${sources[@]}" --classpath "$sl" ;;
      esac
    done
  done
}
export out sl base head
export -f record run_cell compiler_words interp_words _new_spelling
export _spellings_nl
printf '%s\n' "${cells[@]}" | xargs -P 4 -n 1 bash -c 'run_cell "$1"' _
python3 -I "$py" judge "$out" "${sides[@]}"
