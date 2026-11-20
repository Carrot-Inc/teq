#!/bin/bash
# tests/tasty/file-alone.sh <teq> <out dir> <unit dir>...: every file's TASTy as a function of the file alone. For each
# unit (a directory: its .scala files at any depth, a tests/modules case's modules among them; a case with a
# generate.py is made in a copy first), the whole unit is built into products once; then each of its files is built
# alone over those products (`--classpath`: the other files' products as the whole build wrote them, the file's own
# beside them, which its source stands for), and each TASTy file that build writes is compared byte for byte with the
# whole build's of the same path. A whole build past the parallel parse's size parses on threads and a file alone on
# one: what a file's products take from anything but its own parse shows. Prints per unit its files, the ones judged,
# the TASTy files differing (named) and the files not judged with why. Every build is bounded at 120 s; one past its
# bound is NOT JUDGED and named, never counted equal. Writes and deletes only under <out dir>. Exits 1 when a TASTy
# file differs. Env: TEQ_ALONE_MODE (`check`, the default: `compiler check --products`, the JS check's pickles; `jvm`:
# `compiler build --target jvm --products`).
set -u
teq=$(cd "$(dirname "${1:?a teq}")" && pwd)/$(basename "$1")
out=${2:?an out dir}
shift 2
mkdir -p "$out"
out=$(cd "$out" && pwd)
mode=${TEQ_ALONE_MODE:-check}
build() { # build <products> <classpath or -> <files...>, from the unit's directory
  local p=$1 cp=$2
  shift 2
  local extra=()
  [ "$cp" = - ] || extra=(--classpath "$cp")
  if [ "$mode" = jvm ]; then
    timeout 120 "$teq" compiler build --target jvm --products "$p" "${extra[@]}" "$@" > "$p.log" 2>&1
  else
    timeout 120 "$teq" compiler check --std scala-library --products "$p" "${extra[@]}" "$@" > "$p.log" 2>&1
  fi
}
total_units=0 total_files=0 total_judged=0 total_differ=0 total_left=0 units_built=0
for unit in "$@"; do
  unit=$(cd "$unit" && pwd)
  name=$(basename "$unit")
  key=$(echo "$unit" | tr '/' '_')
  w=$out/$key
  case "$w" in "$out"/*) rm -rf "${w:?}" ;; esac
  mkdir -p "$w"
  src=$unit
  if [ -f "$unit/generate.py" ]; then
    cp -R "$unit" "$w/case" && timeout 120 python3 "$unit/generate.py" "$w/case" > /dev/null 2>&1 || { echo "$name: NOT JUDGED: generate.py"; continue; }
    src=$w/case
  fi
  mapfile -t files < <(cd "$src" && find . -name '*.scala' | sed 's|^\./||' | sort)
  total_units=$((total_units + 1))
  [ "${#files[@]}" -ge 2 ] || { echo "$name: one file"; continue; }
  (cd "$src" && build "$w/whole" - "${files[@]}")
  st=$?
  if [ "$st" != 0 ]; then
    [ "$st" = 124 ] && echo "$name: NOT JUDGED: the whole build past 120 s" || echo "$name: the whole unit does not build ($(tail -1 "$w/whole.log" | cut -c1-120))"
    continue
  fi
  units_built=$((units_built + 1))
  judged=0 differ=0 left=0
  for f in "${files[@]}"; do
    total_files=$((total_files + 1))
    fk=$(echo "$f" | tr '/' '_')
    (cd "$src" && build "$w/alone-$fk" "$w/whole" "$f")
    st=$?
    if [ "$st" != 0 ]; then
      [ "$st" = 124 ] && why="NOT JUDGED: past 120 s" || why="it does not build alone ($(tail -1 "$w/alone-$fk.log" | cut -c1-100))"
      echo "$name: $f not judged: $why"
      left=$((left + 1))
      continue
    fi
    judged=$((judged + 1))
    while IFS= read -r t; do
      if ! cmp -s "$w/alone-$fk/$t" "$w/whole/$t"; then
        echo "$name: $f: DIFFER $t"
        differ=$((differ + 1))
      fi
    done < <(cd "$w/alone-$fk" && find . -name '*.tasty' | sed 's|^\./||' | sort)
    case "$w" in "$out"/*) rm -rf "${w:?}/alone-$fk" ;; esac
  done
  total_judged=$((total_judged + judged)) total_differ=$((total_differ + differ)) total_left=$((total_left + left))
  echo "$name: ${#files[@]} files, $judged judged, $differ TASTy files differ, $left not judged"
  case "$w" in "$out"/*) rm -rf "${w:?}" ;; esac
done
echo "file-alone: $total_units units, $units_built built whole; $total_files files, $total_judged judged, $total_left not judged; $total_differ TASTy files differ"
[ "$total_differ" = 0 ]
