#!/bin/bash
# java-default-super.sh <base teq> <empty out dir> [<head teq>]
# Compile the batch with scalac 3.8.4, run every cell, then judge against its own JVM output.
set -eu
base=$(realpath "${1:?base teq}")
out=$(realpath -m "${2:?out dir}")
head=${3:+$(realpath "$3")}
here=$(cd "$(dirname "$0")" && pwd)
py=$here/java_default_super.py
cd "$here/../.."
mkdir -p "$out"
if [[ -n $(ls -A "$out") ]]; then echo "java-default-super: output directory must be empty: $out" >&2; exit 2; fi
for bin in "$base" ${head:+"$head"}; do
  [[ -f $bin && -x $bin ]] || { echo "java-default-super: no executable at $bin" >&2; exit 2; }
done
export TMPDIR=$out/tmp TEQ_CACHE_DIR=$out/cache
mkdir -p "$TMPDIR" "$out/javap"
. tests/support/jars.sh
sl=$(jar_of scala-library)
[[ -f $sl ]] || { echo "java-default-super: missing $sl" >&2; exit 2; }
record() {
  local prefix=$1 seconds=$2 rc=0
  shift 2
  LC_ALL=C timeout -v -k 2 "$seconds" "$@" > "$prefix.out" 2> "$prefix.err" || rc=$?
  echo "$rc" > "$prefix.exit"
}

checked() {
  local prefix=$1
  record "$@"
  cat "$prefix.out"
  cat "$prefix.err" >&2
  return "$(< "$prefix.exit")"
}

cd "$out"
checked gen 60 python3 -I "$py" gen "$out"
echo base > binaries
if [[ -n $head ]]; then echo head >> binaries; fi

mapfile -t interfaces < <(awk -F '\t' 'NR>1 && !seen[$2]++ {print $2}' cells.tsv)
for iface in "${interfaces[@]}"; do
  record "javap/$iface" 60 javap -public -s "$iface"
done

S=$HOME/.sbt/boot/scala-3.8.4
sbts=("$S"/org.scala-sbt/sbt/*/)
lib=$sl
if [[ -d ${sbts[0]} && -f $S/lib/scala3-compiler_3-3.8.4.jar ]]; then
  lib=$S/lib/scala-library.jar:$S/lib/scala3-library_3-3.8.4.jar
  scalac=(java -cp "$S/lib/*:${sbts[0]}*" dotty.tools.dotc.Main -classpath "$lib" -color:never -d)
else
  scalac=(scala-cli compile -S 3.8.4 --server=false --scalac-option -color:never -d)
fi
printf '%s\n' "${scalac[@]}" > scalac-command
mapfile -t cells < <(awk -F '\t' 'NR>1 {print $1}' cells.tsv)
pending=("${cells[@]}")
attempt=0
while (( ${#pending[@]} )); do
  attempt=$((attempt + 1))
  path=batches/$attempt
  classes=$out/$path/classes
  mkdir -p "$classes"
  printf '%s\n' "${pending[@]}" > "$path/cells"
  record "$path/build" 300 "${scalac[@]}" "$classes" "${pending[@]/%//Main.scala}"
  checked "$path/attribute" 60 python3 -I "$py" batch "$out" "$attempt"
  mapfile -t pending < "$path/remaining"
done

mapfile -t jar_cells < jar-cells
mkdir -p jar/classes
checked jar/build 300 "${scalac[@]}" "$out/jar/classes" "${jar_cells[@]/%//split/Between.scala}"
checked jar/pack 60 jar cf "$out/jar/between.jar" -C "$out/jar/classes" .

jvm_cell() {
  local source=$1 cp=$2
  record "$r/build" 120 "$bin" compiler build "$source" --target jvm --std=scala-library --classpath "$cp" -o "$r/x.jar"
  if [[ $(< "$r/build.exit") == 0 ]]; then
    record "$r/run" 60 java -Xverify:all -cp "$r/x.jar:$cp" "$cell.Main"
  fi
}

run_cell() {
  set -eu
  local cell=$1 ref=$out/runs/scalac/$1 side bin target r
  cd "$out/$cell"
  if [[ $(< "$ref/build.exit") == 0 ]]; then
    record "$ref/run" 60 java -Xverify:all -cp "$classes:$lib" "$cell.Main"
  fi
  while read -r side; do
    bin=$base
    if [[ $side == head ]]; then bin=$head; fi
    for target in jvm js interp; do
      r=$out/runs/$side/$target/$cell
      mkdir -p "$r"
      case $target in
        jvm) jvm_cell Main.scala "$sl" ;;
        js)  record "$r/build" 120 "$bin" compiler build Main.scala -o "$r/x.js"
             if [[ $(< "$r/build.exit") == 0 ]]; then record "$r/run" 60 node "$r/x.js"; fi ;;
        interp) record "$r/run" 120 "$bin" interp Main.scala ;;
      esac
    done
    if [[ -d split ]]; then
      r=$out/runs/$side/jvm-split/$cell
      mkdir -p "$r"
      record "$r/upstream" 120 "$bin" compiler build split/Between.scala --target jvm --std=scala-library \
        --products "$r/products" --sourceroot split --classpath "$sl"
      if [[ $(< "$r/upstream.exit") == 0 ]]; then jvm_cell split/Main.scala "$sl:$r/products"; fi
    fi
    if grep -Fxq "$cell" "$out/jar-cells"; then
      r=$out/runs/$side/jvm-jar/$cell
      mkdir -p "$r"
      jvm_cell split/Main.scala "$sl:$out/jar/between.jar"
    fi
  done < "$out/binaries"
}
export out classes lib base head sl
export -f record run_cell jvm_cell
printf '%s\n' "${cells[@]}" | xargs -P 4 -n 1 bash -c 'run_cell "$1"' _
checked judge 60 python3 -I "$py" judge "$out"
