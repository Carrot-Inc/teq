#!/bin/bash
# Captures scalac 3.8.4's `unused import` warnings under -Wunused:imports alone into the pinned
# oracle of tests/warnings/unused-imports.sh, one `line:col:width` per warning in source order
# (columns from 1). Without arguments: the files of dotty's tests/warn that name -Wunused:imports
# or -Wunused:all ($SCALA3/tests/warn), their other -W options and -Werror dropped, their language
# and source options kept (tests/warnings/options.py), into tests/warnings/expected/<file>.txt.
# With --probes: the probes of tests/warnings/probes into tests/warnings/expected/probes/. Run by
# hand when the corpus or a probe changes; needs scala-cli and a JDK; each file bounded to 5 min.
#   tests/warnings/capture-unused.sh [--probes] [file.scala ...]
cd "$(dirname "$0")/../.."
if [ -z "$SCALA3" ] && common=$(git rev-parse --path-format=absolute --git-common-dir 2> /dev/null); then
  SCALA3="$(dirname "$common")/../teq-ref/scala3"
fi
src=$SCALA3/tests/warn
out=tests/warnings/expected
if [ "$1" = --probes ]; then
  shift
  src=tests/warnings/probes
  out=tests/warnings/expected/probes
fi
mkdir -p "$out"
jobs=${JOBS:-4}
files=("$@")
if [ ${#files[@]} -eq 0 ]; then
  if [ "$src" = tests/warnings/probes ]; then
    mapfile -t files < <(cd "$src" && ls -d *.scala */ | sed 's|/$||')
  else
    mapfile -t files < <(cd "$src" && grep -lE -- '-Wunused:imports|-Wunused:all' *.scala | sort)
  fi
fi
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
capture() {
  local f=$1 dir=$tmp/${1%.scala}
  mkdir -p "$dir"
  # A probe of several files (a directory of tests/warnings/probes) is compiled as one program.
  local inputs=("$f")
  [ -d "$src/$f" ] && mapfile -t inputs < <(cd "$src" && ls "$f"/*.scala)
  local opts=()
  for i in "${inputs[@]}"; do
    python3 tests/warnings/options.py strip < "$src/$i" > "$dir/$(basename "$i")"
    while IFS= read -r o; do opts+=(-O "$o"); done < <(python3 tests/warnings/options.py scalac < "$src/$i")
  done
  timeout 300 scala-cli compile -S 3.8.4 --jvm system --server=false -O -color:never "${opts[@]}" "$dir" > "$dir.log" 2>&1
  local target=$dir/$f
  [ -d "$src/$f" ] && target=$dir
  python3 tests/warnings/parse.py scalac "$target" < "$dir.log" > "$out/${f%.scala}.txt"
  # A file scalac does not compile gives no oracle: said, and its log kept beside the expectation.
  if grep -qE '^-- (\[E[0-9]+\] )?[A-Za-z ]*Error:|error(s)? found' "$dir.log"; then
    echo "$f: scalac reported an error"
    cp "$dir.log" "$out/${f%.scala}.scalac-error.log"
  fi
}
export -f capture
export tmp src out
printf '%s\n' "${files[@]}" | xargs -P "$jobs" -I{} bash -c 'capture {}'
echo "captured ${#files[@]} files"
