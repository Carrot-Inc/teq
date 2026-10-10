#!/bin/bash
# Captures scalac 3.8.4's diagnostics and exit for each run of tests/warnings/policy.txt into
# tests/warnings/expected/policy/<probe>@<n>.txt (the probe's n-th run, from 1), normalized by
# tests/warnings/policy.py, scalac's own output kept under out/policy-capture. Run by hand when a probe or a run changes; needs scala-cli and a JDK;
# each run bounded to 5 min.
#   tests/warnings/capture-policy.sh [probe ...]
cd "$(dirname "$0")/../.."
out=tests/warnings/expected/policy
logs=out/policy-capture
mkdir -p "$out" "$logs"
jobs=${JOBS:-4}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
runs=()
declare -A seen
while IFS= read -r line; do
  case "$line" in '#'*|'') continue ;; esac
  probe=${line%%:*}
  options=${line#*:}
  seen[$probe]=$(( ${seen[$probe]:-0} + 1 ))
  if [ $# -gt 0 ] && [[ " $* " != *" $probe "* ]]; then continue; fi
  runs+=("$probe@${seen[$probe]}|$options")
done < tests/warnings/policy.txt
capture() {
  local name=${1%%|*} options=${1#*|}
  local probe=${name%@*}
  local dir=$tmp/$name
  mkdir -p "$dir"
  cp "tests/warnings/policy/$probe.scala" "$dir/"
  local opts=()
  for o in $options; do opts+=(-O "$o"); done
  timeout 300 scala-cli compile -S 3.8.4 --jvm system --server=false -O -color:never "${opts[@]}" "$dir/$probe.scala" > "$dir.log" 2>&1
  local code=$?
  python3 tests/warnings/policy.py scalac "$code" "$probe.scala" < "$dir.log" > "$out/$name.txt"
  cp "$dir.log" "$logs/$name.log"
}
export -f capture
export tmp out logs
printf '%s\n' "${runs[@]}" | xargs -P "$jobs" -I{} bash -c 'capture "{}"'
echo "captured ${#runs[@]} runs"
