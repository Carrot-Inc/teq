#!/bin/bash
# Captures scalac 3.8.4's unused warnings of every kind (tests/warnings/kinds.py) for the files of
# dotty's tests/warn that name -Wunused ($SCALA3/tests/warn), each under its own -Wunused, -Wconf,
# -language, -source and -Y options, into tests/warnings/expected/kinds/<file>.txt, the oracle of
# tests/warnings/unused-kinds.sh. Run by hand when the corpus changes; needs scala-cli and a JDK;
# each file bounded to 5 min.
#   tests/warnings/capture-kinds.sh [file.scala ...]
cd "$(dirname "$0")/../.."
if [ -z "$SCALA3" ] && common=$(git rev-parse --path-format=absolute --git-common-dir 2> /dev/null); then
  SCALA3="$(dirname "$common")/../teq-ref/scala3"
fi
src=$SCALA3/tests/warn
out=tests/warnings/expected/kinds
logs=out/kinds-capture
mkdir -p "$out" "$logs"
jobs=${JOBS:-4}
files=("$@")
if [ ${#files[@]} -eq 0 ]; then
  mapfile -t files < <(cd "$src" && grep -l -- '-Wunused' *.scala | sort)
fi
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
capture() {
  local f=$1 dir=$tmp/${1%.scala}
  mkdir -p "$dir"
  python3 tests/warnings/kinds.py strip < "$src/$f" > "$dir/$f"
  local opts=()
  while IFS= read -r o; do [ -n "$o" ] && opts+=(-O "$o"); done < <(python3 tests/warnings/kinds.py options scalac < "$src/$f")
  timeout 300 scala-cli compile -S 3.8.4 --jvm system --server=false -O -color:never "${opts[@]}" "$dir" > "$dir.log" 2>&1
  python3 tests/warnings/kinds.py scalac "$dir/$f" < "$dir.log" > "$out/${f%.scala}.txt"
  cp "$dir.log" "$logs/${f%.scala}.log"
  if grep -qE '^-- (\[E[0-9]+\] )?[A-Za-z ]*Error:|error(s)? found' "$dir.log"; then
    echo "$f: scalac reported an error"
  fi
}
export -f capture
export tmp src out logs
printf '%s\n' "${files[@]}" | xargs -P "$jobs" -I{} bash -c 'capture {}'
echo "captured ${#files[@]} files"
