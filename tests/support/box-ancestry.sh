#!/bin/bash
# box-ancestry.sh <base teq> <fresh output directory> [<head teq>]
# TEQ=/path/to/teq box-ancestry.sh --record|--check|--update [<empty output directory>]
set -eu
. "$(dirname "$0")/recorded-table.sh"
py=$here/box_ancestry.py
if [[ $table_mode != compare ]]; then table_judge selftest; fi
cd "$here/../.."
. tests/support/jars.sh
sl=$(jar_of scala-library)
[[ -f $sl && ${sl##*/} == scala-library-3.8.4.jar ]] || { echo "box-ancestry: scala-library 3.8.4 jar missing" >&2; exit 2; }
printf '%s\n' base > "$out/binaries"
if [[ -n $head ]]; then echo head >> "$out/binaries"; fi
timeout 60 python3 -I "$here/box_ancestry.py" gen "$out"

step() {
  local record=$1 rc=0
  shift
  timeout 120 "$@" > "$record.out" 2> "$record.err" || rc=$?
  echo "$rc" > "$record.exit"
  return "$rc"
}

run_cell() {
  local name=$1 d="$out/cells/$1" r side bin
  local files=("$d/A.scala")
  if [[ -f $d/B.scala ]]; then files+=("$d/B.scala"); fi
  if [[ -f $d/Lib.scala ]]; then files+=("$d/Lib.scala"); fi
  if [[ $table_mode == compare || $table_mode == record ]]; then
    r="$out/runs/scalac/$name"
    mkdir -p "$r/classes" || return 2
    if step "$r/build" scala-cli compile -S 3.8.4 --server=false --jvm system "${files[@]}" -d "$r/classes"; then
      step "$r/run" java -XX:-UsePerfData -Xverify:all -cp "$r/classes:$sl" "$name" || :
    fi
    if [[ -f $d/Lib.scala ]]; then
      r="$out/runs/scalac-split/$name"
      mkdir -p "$r/classes" "$r/lib" || return 2
      if step "$r/upstream" scala-cli compile -S 3.8.4 --server=false --jvm system "$d/Lib.scala" -d "$r/lib" &&
         step "$r/build" scala-cli compile -S 3.8.4 --server=false --jvm system "$d/A.scala" --classpath "$r/lib" -d "$r/classes"; then
        step "$r/run" java -XX:-UsePerfData -Xverify:all -cp "$r/classes:$r/lib:$sl" "$name" || :
      fi
    fi
  fi
  for side in base ${head:+head}; do
    bin=$base
    if [[ $side == head ]]; then bin=$head; fi
    local run mode threads seed cp sources settings
    while IFS=$'\t' read -r run mode threads seed; do
      r="$out/runs/$side/jvm/$run"
      mkdir -p "$r" || return 2
      cp=$sl
      sources=("${files[@]}")
      settings=(env TEQ_BOX_TESTS=1)
      if [[ $threads != - ]]; then settings+=(TEQ_THREADS="$threads" TEQ_FILE_ORDER="seed:$seed"); fi
      if [[ $mode == split ]]; then
        step "$r/upstream" "${settings[@]}" "$bin" compiler build "$d/Lib.scala" --target jvm --std=scala-library \
          --products "$r/products" --sourceroot "$d" --classpath "$sl" || continue
        cp="$sl:$r/products"
        sources=("$d/A.scala")
      fi
      if step "$r/build" "${settings[@]}" "$bin" compiler build "${sources[@]}" --target jvm --std=scala-library --classpath "$cp" -o "$r/out.jar"; then
        step "$r/run" java -XX:-UsePerfData -Xverify:all -cp "$r/out.jar:$cp" "$name" || :
      fi
    done < "$d/jvm.tsv"
    r="$out/runs/$side/js/$name"
    mkdir -p "$r" || return 2
    if step "$r/build" "$bin" compiler build "${files[@]}" -o "$r/out.js"; then
      step "$r/run" node "$r/out.js" || :
    fi
    r="$out/runs/$side/interp/$name"
    mkdir -p "$r" || return 2
    step "$r/run" "$bin" interp "${files[@]}" || :
  done
}
run_pattern() {
  local name=$1 mode=$2 d="$out/patterns/$1" r side cp bin
  local files=("$d/A.scala" "$d/B.scala")
  if [[ -f $d/Lib.scala && $mode == whole ]]; then files+=("$d/Lib.scala"); fi
  for side in scalac base ${head:+head}; do
    if [[ $side == scalac && $table_mode != compare && $table_mode != record ]]; then continue; fi
    r="$out/runs/patterns/$side/$name"
    mkdir -p "$r/classes" || return 2
    cp=$sl
    if [[ $side == scalac ]]; then
      if [[ $mode == split ]]; then
        mkdir -p "$r/lib" || return 2
        step "$r/upstream" scala-cli compile -S 3.8.4 --server=false --jvm system "$d/Lib.scala" -d "$r/lib" || continue
        cp="$sl:$r/lib"
      fi
      step "$r/build" scala-cli compile -S 3.8.4 --server=false --jvm system "${files[@]}" --classpath "$cp" -d "$r/classes" || continue
    else
      bin=$base
      if [[ $side == head ]]; then bin=$head; fi
      if [[ $mode == split ]]; then
        step "$r/upstream" env TEQ_BOX_TESTS=1 "$bin" compiler build "$d/Lib.scala" --target jvm --std=scala-library \
          --products "$r/products" --sourceroot "$d" --classpath "$cp" || continue
        cp="$sl:$r/products"
      fi
      step "$r/build" env TEQ_BOX_TESTS=1 "$bin" compiler build "${files[@]}" --target jvm --std=scala-library \
        --classpath "$cp" --all-mains -o "$r/classes" || continue
    fi
    step "$r/run" java -XX:-UsePerfData -Xverify:all -cp "$r/classes:$cp" B || :
  done
}
export -f run_cell run_pattern step
export base head out sl table_mode
cut -f1,4 "$out/patterns.tsv" | xargs -P "${BOX_JOBS:-4}" -n2 bash -c 'run_pattern "$1" "$2"' _
if [[ -n $head || $table_mode != compare ]]; then timeout 180 python3 -I "$py" watch "$out" "${head:-$base}" "$sl"; fi
cut -f1 "$out/cells.tsv" | xargs -P "${BOX_JOBS:-4}" -n1 bash -c 'run_cell "$1"' _
if [[ $table_mode == compare ]]; then
  timeout 60 python3 -I "$here/box_ancestry.py" judge "$out"
else
  table_judge "$table_mode"
fi
