#!/bin/bash
# getclass-receiver.sh <base teq> <empty out dir> [<head teq>]
# Compile the scalac batch, then build and run each cell separately on its applicable teq legs.
set -eu
base=$(realpath "${1:?base teq}")
out=$(realpath -m "${2:?empty out dir}")
head=${3:+$(realpath "$3")}
here=$(cd "$(dirname "$0")" && pwd)
py=$here/getclass_receiver.py
mkdir -p "$out"
[[ -z $(ls -A "$out") ]] || { echo "output directory must be empty: $out" >&2; exit 2; }
for bin in "$base" ${head:+"$head"}; do
  [[ -x $bin && -f $bin ]] || { echo "no executable at $bin" >&2; exit 2; }
done
export TMPDIR=$out/tmp TEQ_CACHE_DIR=$out/cache
mkdir -p "$TMPDIR"
cd "$here/../.."
. tests/support/jars.sh
sl=$(jar_of scala-library)
[[ -f $sl ]] || { echo "missing $sl" >&2; exit 2; }
S=$HOME/.sbt/boot/scala-3.8.4
sbts=("$S"/org.scala-sbt/sbt/*/)
[[ -d ${sbts[0]} && -f $S/lib/scala3-compiler_3-3.8.4.jar ]] || { echo "missing scalac 3.8.4 in sbt boot" >&2; exit 2; }
lib=$S/lib/scala-library.jar:$S/lib/scala3-library_3-3.8.4.jar
scalac_cp=$S/lib/*:${sbts[0]}*

record() {
  local prefix=$1 seconds=$2 rc=0
  shift 2
  LC_ALL=C timeout -v -k 2 "$seconds" "$@" > "$prefix.out" 2> "$prefix.err" || rc=$?
  echo "$rc" > "$prefix.exit"
}
scalac_build() {
  local prefix=$1 seconds=$2
  shift 2
  record "$prefix" "$seconds" java -XX:-UsePerfData -cp "$scalac_cp" dotty.tools.dotc.Main -color:never "$@"
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
mapfile -t cells < <(awk -F '\t' 'NR>1 {print $1}' cells.tsv)
pending=("${cells[@]}")
attempt=0
while (( ${#pending[@]} )); do
  attempt=$((attempt + 1))
  path=batches/$attempt
  classes=$out/$path/classes
  mkdir -p "$classes"
  printf '%s\n' "${pending[@]}" > "$path/cells"
  sources=()
  for cell in "${pending[@]}"; do sources+=("$cell"/*.scala); done
  scalac_build "$path/build" 300 -classpath "$lib" -d "$classes" "${sources[@]}"
  checked "$path/attribute" 60 python3 -I "$py" batch "$out" "$attempt"
  mapfile -t pending < "$path/remaining"
done

jvm_cell() {
  local source=$1 cp=$2 caller=$r/x.jar
  if [[ $call == reflection ]]; then
    caller=$r/caller
    mkdir -p "$caller"
    scalac_build "$r/build" 120 -classpath "$cp" -d "$caller" "$source"
  else
    record "$r/build" 120 "$bin" compiler build "$source" --target jvm --std=scala-library --classpath "$cp" -o "$caller"
  fi
  if [[ $(< "$r/build.exit") == 0 ]]; then
    record "$r/run" 60 java -XX:-UsePerfData -Xverify:all -cp "$caller:$cp" "$cell.Main"
  fi
}
run_cell() {
  set -eu
  local cell=$1 call=$2 ref=$out/runs/scalac/$1 bin side r
  cd "$out/$cell"
  if [[ $(< "$ref/build.exit") == 0 ]]; then
    record "$ref/run" 60 java -XX:-UsePerfData -Xverify:all -cp "$classes:$lib" "$cell.Main"
  fi
  local source=Main.scala
  if [[ -f Lib.scala ]]; then source=.; fi
  while read -r side; do
    bin=$base
    if [[ $side == head ]]; then bin=$head; fi
    r=$out/runs/$side/jvm/$cell
    mkdir -p "$r"
    if [[ $call == watch ]]; then
      record "$r/build" 300 python3 -I "$py" watch "$PWD" "$bin" "$r/classes" --target jvm --all-mains --std=scala-library --classpath "$sl"
      if [[ $(< "$r/build.exit") == 0 ]]; then
        record "$r/run" 60 java -XX:-UsePerfData -Xverify:all -cp "$r/classes:$sl" "$cell.Main"
      fi
      r=$out/runs/$side/js/$cell
      mkdir -p "$r"
      record "$r/build" 300 python3 -I "$py" watch "$PWD" "$bin" "$r/watch.js"
      if [[ $(< "$r/build.exit") == 0 ]]; then record "$r/run" 60 node "$r/watch.js"; fi
      continue
    fi
    if [[ $call == reflection ]]; then
      record "$r/upstream" 120 "$bin" compiler build split/Lib.scala --target jvm --std=scala-library \
        --all-mains --classpath "$sl" -o "$r/m.jar"
      if [[ $(< "$r/upstream.exit") == 0 ]]; then jvm_cell split/Main.scala "$sl:$r/m.jar"; fi
    else
      jvm_cell "$source" "$sl"
    fi
    if [[ -d split ]]; then
      r=$out/runs/$side/jvm-split/$cell
      mkdir -p "$r"
      record "$r/upstream" 120 "$bin" compiler build split/Lib.scala --target jvm --std=scala-library \
        --products "$r/products" --sourceroot split --classpath "$sl"
      if [[ $(< "$r/upstream.exit") == 0 ]]; then jvm_cell split/Main.scala "$sl:$r/products"; fi
    fi
    if [[ $call == reflection ]]; then continue; fi
    r=$out/runs/$side/js/$cell
    mkdir -p "$r"
    record "$r/build" 120 "$bin" compiler build "$source" -o "$r/x.js"
    if [[ $(< "$r/build.exit") == 0 ]]; then record "$r/run" 60 node "$r/x.js"; fi
    r=$out/runs/$side/interp/$cell
    mkdir -p "$r"
    record "$r/run" 120 "$bin" interp "$source"
  done < "$out/binaries"
}
export py out classes lib base head sl scalac_cp
export -f record scalac_build jvm_cell run_cell
awk -F '\t' 'NR>1 {print $1, $5}' cells.tsv | xargs -P 8 -n 2 bash -c 'run_cell "$1" "$2"' _
checked judge 60 python3 -I "$py" judge "$out"
