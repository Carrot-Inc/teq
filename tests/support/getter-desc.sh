#!/bin/bash
# getter-desc.sh <base teq> <out dir> [<head teq>]
# GETTER_DESC_FROM and GETTER_DESC_TO select an inclusive, one-based slice.
# Resume slices in the same directory, then run getter_desc.py judge <out dir>.
set -eu
base=$(realpath "${1:?base teq}") out=$(realpath -m "${2:?out dir}") head=
if [[ $# == 3 ]]; then head=$(realpath "$3"); fi
cd "$(dirname "$0")/../.."
helper=$PWD/tests/support/getter_desc.py
mkdir -p "$out/tmp"
export TMPDIR="$out/tmp" TEQ_CACHE_DIR="$out/teq-cache"
. tests/support/jars.sh
sl=$(scala_library_jar)
[[ ${sl##*/} == scala-library-3.8.4.jar && -f $sl ]] || { echo "missing scala-library 3.8.4" >&2; exit 2; }
boot=$HOME/.sbt/boot/scala-3.8.4
sbt=("$boot"/org.scala-sbt/sbt/*/)
bootcp='' lib=$sl
if [[ -d ${sbt[0]} && -f $boot/lib/scala3-compiler_3-3.8.4.jar && -f $boot/lib/scala-library.jar && -f $boot/lib/scala3-library_3-3.8.4.jar ]]; then
  bootcp="$boot/lib/*:${sbt[0]}*"
  lib="$boot/lib/scala-library.jar:$boot/lib/scala3-library_3-3.8.4.jar"
fi
timeout 60 python3 -I "$helper" gen "$out"
for side in base ${head:+head}; do
  hash=$(sha256sum < "${!side}")
  if [[ -f $out/$side.sha256 && $(<"$out/$side.sha256") != "$hash" ]]; then
    echo "getter-desc: $side binary changed in $out" >&2; exit 2
  fi
  echo "$hash" > "$out/$side.sha256"
done
mapfile -t cells < <(cut -f1 "$out/cells.tsv")
first=${GETTER_DESC_FROM:-1} last=${GETTER_DESC_TO:-${#cells[@]}}
(( first >= 1 && first <= last && last <= ${#cells[@]} ))

record() {
  local p=$1 seconds=$2 rc=0
  shift 2
  if [[ -s $p.exit && -f $p.out && -f $p.err ]]; then return; fi
  rm -f "$p.exit"
  LC_MESSAGES=C timeout -v -k 5 "$seconds" "$@" > "$p.out" 2> "$p.err" || rc=$?
  echo "$rc" > "$p.exit.tmp"
  mv "$p.exit.tmp" "$p.exit"
}

scalac_build() {
  local p=$1 src=$2 dest=$3 cp=$4
  mkdir -p "$dest"
  if [[ -n $bootcp ]]; then
    record "$p" 120 java -cp "$bootcp" dotty.tools.dotc.Main -color:never -classpath "$cp" -d "$dest" "$src"
  else
    record "$p" 120 scala-cli --power compile -S 3.8.4 --server=false --jvm system --scalac-option -color:never --classpath "$cp" -d "$dest" "$src"
  fi
}

disassemble() (
  shopt -s globstar nullglob
  local classes=("$2"/**/*.class)
  record "$1" 30 javap -c -p "${classes[@]}"
)

run_leg() {
  local cell=$1 side=$2 leg=$3 r="$out/runs/$2/$3/$1" d="$out/build/$2/$3/$1"
  local src=$out/$cell file=$out/$cell/Whole.scala cp=$sl entry=TeqMain
  mkdir -p "$r" "$d"
  case $leg in
    interp) record "$r/run" 30 "${!side}" interp "$file"; return ;;
    js)
      record "$r/build" 30 "${!side}" compiler build "$file" -o "$d/x.js"
      if [[ $(<"$r/build.exit") == 0 ]]; then record "$r/run" 20 node "$d/x.js"; fi
      return ;;
  esac
  if [[ $side == scalac ]]; then cp=$lib; entry=$cell.run; fi
  local args=()
  if [[ $leg == jvm-scalac-lib ]]; then
    if [[ $(<"$out/runs/scalac/jvm-split/$cell/parent.exit") != 0 ]]; then return; fi
    file=$src/Main.scala
    cp=$out/build/scalac/jvm-split/$cell/a.jar:$sl
  elif [[ $leg == jvm-split ]]; then
    args=(--sourceroot "$src")
    if [[ $side == scalac ]]; then
      scalac_build "$r/parent" "$src/A.scala" "$d/products" "$cp"
    else
      record "$r/parent" 60 "${!side}" compiler build --target jvm --products "$d/products" "${args[@]}" --classpath "$cp" "$src/A.scala"
    fi
    if [[ $(<"$r/parent.exit") != 0 ]]; then return; fi
    if [[ $side == scalac ]]; then
      timeout 30 jar cf "$d/a.jar" -C "$d/products" .
    fi
    disassemble "$r/parent-javap" "$d/products"
    file=$src/Main.scala
    cp=$d/products:$cp
  fi
  if [[ $side == scalac ]]; then
    scalac_build "$r/build" "$file" "$d/main" "$cp"
  else
    record "$r/build" 60 "${!side}" compiler build "$file" --target jvm "${args[@]}" --classpath "$cp" -o "$d/main"
  fi
  if [[ $(<"$r/build.exit") == 0 ]]; then
    disassemble "$r/javap" "$d/main"
    record "$r/run" 20 java -Djava.io.tmpdir="$TMPDIR" -cp "$d/main:$cp" "$entry"
    if [[ $side == scalac ]]; then
      record "$r/repeat" 20 java -Djava.io.tmpdir="$TMPDIR" -cp "$d/main:$cp" "$entry"
    fi
  fi
}

run_cell() {
  local side leg
  for side in scalac base ${head:+head}; do
    for leg in jvm jvm-split; do run_leg "$1" "$side" "$leg"; done
    if [[ $side != scalac ]]; then
      for leg in jvm-scalac-lib js interp; do run_leg "$1" "$side" "$leg"; done
    fi
  done
}
export -f run_cell run_leg scalac_build disassemble record
export base head out bootcp lib sl
printf '%s\n' "${cells[@]:first-1:last-first+1}" | xargs -P 4 -n 1 bash -eu -c 'run_cell "$1"' _
if (( first == 1 && last == ${#cells[@]} )); then
  exec timeout 60 python3 -I "$helper" judge "$out"
fi
echo "getter-desc: recorded cells $first through $last of ${#cells[@]}"
