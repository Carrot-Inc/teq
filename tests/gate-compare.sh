#!/bin/bash
# The comparison machine's job of tests/gate.sh: the gate copies this file into out/compare/ of its mirror of the
# landing worktree (~/rw/teq) and starts it there under nohup, with master's sources mirrored in GATE_MASTER_TREE
# and the reference's in GATE_REFERENCE_TREE. It makes sure of master's and the reference's binaries, artifacts
# kept under ~/teq-bin/<source hash>-<toolchain key>/ beside their manifests (tests/support/build-manifest.sh) and
# built from their trees on first use (build_teq: stamped with their revision, the executable checked to name it
# before it is kept) and checked at use; takes the head's binary, built here when GATE_HEAD_FROM=here or when the
# gate writes out/compare/build-head, otherwise the one a shard built that the gate mirrors to ../teq-head and
# announces with out/compare/head-ready (built here instead when its manifest is not master's build); checks every
# pair it is about to compare for one build configuration, refusing a line whose pair differs; then runs the lines
# GATE_LINES names one after another (bench/pairs.py: budget and runtime against master, sentinel against the
# reference). Each step appends
# "<name> <exit> <seconds> <its last line>" to out/compare/status, "head-wanted" says it waits for the head, and
# "done" ends it; the logs, the raw runs and the manifests go to out/compare/logs.
cd "$(dirname "$0")/../.." || exit 1
out=out/compare
logs=$out/logs
status=$out/status
mkdir -p "$logs"
. tests/support/build-manifest.sh
now() { date +%s; }
# step <name> <exit> <started> <log or message>
step() {
  local last=$4
  [ -f "$4" ] && last=$(grep -v '^[[:space:]]*$' "$4" | tail -1 | tr -d '\r' | tr '\t' ' ' | cut -c1-300)
  printf '%s %s %s %s\n' "$1" "$2" $(($(now) - $3)) "$last" >> $status
}
finish() { echo done >> $status; exit "${1:-0}"; }
tail_key=$(toolchain_key .)
lines=" ${GATE_LINES:-budget runtime sentinel} "

# artifact <name> <revision> <source> <tree>: the binary of <revision>'s sources in ~/teq-bin, built when missing;
# its directory on stdout, and a step either way.
artifact() {
  local name=$1 rev=$2 src=$3 tree=$4 dir=$HOME/teq-bin/$3-$tail_key s tmp v
  s=$(now)
  if [ -x "$dir/teq" ] && grep -q -x "source $src" "$dir/manifest" 2> /dev/null; then
    v=$("$dir/teq" --version 2> /dev/null)
    if [ -z "$(echo "$v" | awk '{print $4}')" ] && v=$(echo "$v" | awk '{print $3}') && [ -n "$v" ] && grep -q -x "revision $v" "$dir/manifest"; then
      step "$name" 0 $s "warm: $dir, built from $v ($(sed -n 's/^built //p' "$dir/manifest"))"
      echo "$dir"
      return 0
    fi
  fi
  if ! build_teq "$tree" "$rev" > "$logs/$name.build.log" 2>&1; then
    step "$name" 1 $s "$logs/$name.build.log"
    return 1
  fi
  tmp=$dir.tmp.$$
  rm -rf "$tmp" && mkdir -p "$tmp" && cp "$tree/target/release/teq" "$tmp/teq" && manifest "$tree" "$rev" "$src" > "$tmp/manifest" &&
    rm -rf "$dir" && mv "$tmp" "$dir" || { step "$name" 1 $s "could not keep the build in $dir"; return 1; }
  step "$name" 0 $s "cold: built $dir from $rev"
  echo "$dir"
}

# The head: a shard's build when the gate announces one, else built here, beside master's build when known at once.
build_head() {
  local s=$(now)
  if ! build_teq . "$GATE_HEAD" > "$logs/head.build.log" 2>&1; then
    step head 1 $s "$logs/head.build.log"
    return 1
  fi
  manifest . "$GATE_HEAD" "$GATE_HEAD_SOURCE" > "$logs/head.manifest"
  step head 0 $s "built here: $1"
}
hpid=
if [ "$GATE_HEAD_FROM" = here ]; then
  build_head "no shard builds the head" &
  hpid=$!
fi
master=
reference=
( [[ "$lines" == *" budget "* || "$lines" == *" runtime "* ]] || exit 0
  artifact master "$GATE_MASTER" "$GATE_MASTER_SOURCE" "$GATE_MASTER_TREE" > $out/master.dir ) &
mpid=$!
if [[ "$lines" == *" sentinel "* ]]; then
  if [ "$GATE_REFERENCE_SOURCE" = "$GATE_MASTER_SOURCE" ]; then
    wait $mpid
  fi
  artifact reference "$GATE_REFERENCE" "$GATE_REFERENCE_SOURCE" "$GATE_REFERENCE_TREE" > $out/reference.dir
fi
wait $mpid
master=$(cat $out/master.dir 2> /dev/null)
reference=$(cat $out/reference.dir 2> /dev/null)

head=$PWD/target/release/teq
s=$(now)
if [ -n "$hpid" ]; then
  wait $hpid || finish 1
else
  echo "head-wanted 0 0 waiting for a shard's build" >> $status
  for i in $(seq 1 $(( ${GATE_HEAD_WAIT:-1800} / 5 ))); do
    [ -e $out/head-ready ] || [ -e $out/build-head ] && break
    sleep 5
  done
  if [ -e $out/head-ready ]; then
    from=$(cat $out/head-ready)
    chmod +x ../teq-head/teq 2> /dev/null
    if [ -z "$master" ] || same_build ../teq-head/manifest "$master/manifest"; then
      head=$(cd ../teq-head && pwd)/teq
      cp ../teq-head/manifest "$logs/head.manifest"
      step head 0 $s "copied from $from after $(($(now) - s))s"
    else
      { echo "the shard's build is not master's:"; diff ../teq-head/manifest "$master/manifest"; } > "$logs/head.mismatch"
      build_head "$from's build differs from master's (head.mismatch)" || finish 1
    fi
  elif [ -e $out/build-head ]; then
    build_head "$(cat $out/build-head)" || finish 1
  else
    step head 1 $s "no head binary after ${GATE_HEAD_WAIT:-1800}s"
    finish 1
  fi
fi
version=$("$head" --version 2> /dev/null)
[ "$(echo "$version" | awk '{print $3}')" = "$GATE_HEAD" ] && [ -z "$(echo "$version" | awk '{print $4}')" ] ||
  { step head 1 $(now) "$head is ${version:-no build}, not a plain build of the head $GATE_HEAD"; finish 1; }

# Every pair about to be compared is one build configuration; a line whose pair is not is refused.
# differs <name> <manifest>: whether that build's manifest and the head's disagree, the difference kept.
differs() {
  same_build "$logs/head.manifest" "$2" && return 1
  { echo "$1's build and the head's differ:"; diff "$2" "$logs/head.manifest"; } > "$logs/$1.mismatch"
}

# run_line <name> <base binary dir> <bench/pairs.py arguments...>
run_line() {
  local name=$1 base=$2 s=$(now) code
  shift 2
  if [ -z "$base" ]; then
    step "$name" 1 $s "not run: no binary to compare with (see the master and reference steps)"
    return
  fi
  if differs "$name" "$base/manifest"; then
    step "$name" 1 $s "not run: the builds compared differ ($logs/$name.mismatch)"
    return
  fi
  timeout 1500 python3 bench/pairs.py "$@" "$base/teq" "$head" --out out/compare-runs/$name --label $name > "$logs/$name.log" 2>&1
  code=$?
  cp out/compare-runs/$name/runs.jsonl "$logs/$name.runs.jsonl" 2> /dev/null
  cp out/compare-runs/$name/line.json "$logs/$name.line.json" 2> /dev/null
  step "$name" $code $s "$logs/$name.log"
}
rm -rf out/compare-runs
for line in budget runtime sentinel; do
  [[ "$lines" == *" $line "* ]] || continue
  case $line in
    budget) run_line budget "$master" budget ;;
    runtime) run_line runtime "$master" runtime ;;
    sentinel) run_line sentinel "$reference" budget --programs "$(sed -n 's/^rows //p' bench/reference.txt)" --ceilings bench/reference.txt ;;
  esac
done
cp "$master/manifest" "$logs/master.manifest" 2> /dev/null
cp "$reference/manifest" "$logs/reference.manifest" 2> /dev/null
finish 0
