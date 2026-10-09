#!/bin/bash
# What the analysis graph costs (docs/TARGETS.md, "The analysis graph"), on the application corpus
# (bench/app, generated at scale 1): the API side built for the JVM into its products as the
# module model compiles a project (`teq compiler build --target jvm --products`), the frontend side
# checked as sbt-teq's Scala.js compile checks it (`teq compiler check`), each without the request, with
# `--analysis-version 2` and with `3`: the wall time, the type phase and the graph's own time
# (`--time`, the dependencies' part of it under 3), the answer's size and the peak memory; then a session of each
# (`teq compiler watch`) likewise, the graph's time of its first build and of a body edit's retype and what
# the session holds after them (`stats`: in all, and the dependencies' records); then, with sbt,
# the JVM's side of the API's answer of each version (integrations/sbt/analysis, `analysisCost`):
# the JSON's parse, the objects' construction, zinc's hashing and its callbacks.
#
#   bench/analysis-costs.sh [runs]     TEQ names the binary (target/release/teq by default)
#
# Linux (GNU time for the peak memory); the jars of tests/support/jars.sh in the coursier cache.
cd "$(dirname "$0")/.."
root=$PWD
TEQ=${TEQ:-./target/release/teq}
case $TEQ in /*) ;; *) TEQ=$root/$TEQ ;; esac
runs=${1:-3}
. tests/support/jars.sh
cp=""
for name in scala-library cats-kernel cats-core sourcecode; do
  cp="$cp:$(jar_of "$name")"
done
cp=${cp#:}
work=$(mktemp -d "${TMPDIR:-/tmp}/teq-analysis-costs.XXXXXX")
trap 'rm -rf "$work"' EXIT
timeout 120 python3 bench/app/gen.py "$work/src" > /dev/null 2>&1 || { echo "cannot generate the corpus"; exit 1; }

# The best of `runs` builds: wall ms, the graph's ms, the answer's bytes, peak KB.
measure() {
  local name=$1
  shift
  local best=
  for _ in $(seq "$runs"); do
    rm -rf "$work/out"
    local start end
    start=$(date +%s%N)
    if ! /usr/bin/time -v "$@" > "$work/$name.answer" 2> "$work/$name.err"; then
      echo "$name: failed"
      grep -v '^\s' "$work/$name.err" | head -5
      return
    fi
    end=$(date +%s%N)
    local wall=$(( (end - start) / 1000000 ))
    if [ -z "$best" ] || [ "$wall" -lt "$best" ]; then
      best=$wall
      cp "$work/$name.err" "$work/$name.best.err"
    fi
  done
  local type graph deps peak bytes
  type=$(sed -n 's/^  type  *\([0-9.]*\) ms.*/\1/p' "$work/$name.best.err")
  graph=$(sed -n 's/^analysis: graph of [0-9]* files in \([0-9.]*\) ms.*/\1/p' "$work/$name.best.err")
  deps=$(sed -n 's/^analysis: dependencies in \([0-9.]*\) ms.*/\1/p' "$work/$name.best.err")
  peak=$(sed -n 's/.*Maximum resident set size (kbytes): //p' "$work/$name.best.err")
  bytes=$(wc -c < "$work/$name.answer" | tr -d ' ')
  printf '%-20s %6s ms  type %7s ms  graph %7s ms (deps %7s ms)  answer %9s bytes  peak %7s KB\n' "$name" "$best" "${type:--}" "${graph:--}" "${deps:--}" "$bytes" "$peak"
}

api=(compiler build --target jvm --std=scala-library "$work/src/shared" "$work/src/api" --classpath "$cp" --products "$work/out" --time)
front=(compiler check "$work/src/shared" "$work/src/frontend" --classpath "$cp" --time)
measure api-plain "$TEQ" "${api[@]}"
measure api-analysis "$TEQ" "${api[@]}" --analysis-version 2
cp "$work/api-analysis.answer" "$work/api-answer.json"
measure api-deps "$TEQ" "${api[@]}" --analysis-version 3
cp "$work/api-deps.answer" "$work/api-deps-answer.json"
rm -rf "$work/classes"
cp -R "$work/out" "$work/classes"
measure frontend-plain "$TEQ" "${front[@]}"
measure frontend-analysis "$TEQ" "${front[@]}" --analysis-version 2
measure frontend-deps "$TEQ" "${front[@]}" --analysis-version 3

# A session's first build and a body edit's retype, and what it holds after them.
session() {
  local name=$1 file=$2 from=$3 to=$4
  shift 4
  python3 - "$TEQ" "$work" "$file" "$name" "$from" "$to" "$@" <<'EOF'
import json, os, subprocess, sys, time
teq, work, file, name, before, after, args = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5], sys.argv[6], sys.argv[7:]
p = subprocess.Popen([teq, "compiler", "watch"] + args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
def answer():
    return json.loads(p.stdout.readline())
first = answer()
text = open(file).read()
time.sleep(1.1)
open(file, "w").write(text.replace(before, after, 1))
p.stdin.write("build\n"); p.stdin.flush()
edit = answer()
open(file, "w").write(text)
p.stdin.write("stats\n"); p.stdin.flush()
stats = json.loads(p.stdout.readline())["stats"]
p.stdin.write("quit\n"); p.stdin.flush()
p.wait(timeout=60)
def mb(v):
    return "%.1f MB" % (v / 1048576.0) if isinstance(v, (int, float)) else "-"
held = stats.get("session", {}).get("account")
deps = stats.get("session", {}).get("deps")
rss = stats.get("allocator", {}).get("rss")
print("%-28s first %8.1f ms (graph %6.1f ms, deps %5.1f ms)  retype %6.1f ms (graph %5.1f ms, deps %5.1f ms, %s)  held %s (deps %s), resident %s" % (
    name, first["ms"]["total"], first["ms"].get("api", 0), first["ms"].get("deps", 0), edit["ms"]["total"], edit["ms"].get("api", 0), edit["ms"].get("deps", 0),
    "incremental" if edit.get("incremental") else "full", mb(held), mb(deps), mb(rss)))
EOF
}
page=$(ls "$work"/src/frontend/frontend/page/*/*DetailPage.scala | head -1)
service="$work"/src/api/server/view/Health.scala
mkdir -p "$work/jvm-out"
session api-session-plain "$service" 'mkString("\n")' 'mkString("\n ")' --target jvm --std=scala-library -o "$work/jvm-out" --classpath "$cp" "$work/src/shared" "$work/src/api"
session api-session-analysis "$service" 'mkString("\n")' 'mkString("\n ")' --target jvm --std=scala-library -o "$work/jvm-out" --classpath "$cp" --analysis-version 2 "$work/src/shared" "$work/src/api"
session api-session-deps "$service" 'mkString("\n")' 'mkString("\n ")' --target jvm --std=scala-library -o "$work/jvm-out" --classpath "$cp" --analysis-version 3 "$work/src/shared" "$work/src/api"
session frontend-session-plain "$page" '"home"' '"home (edited)"' --check --own "$work/src/frontend" --classpath "$cp" "$work/src/shared" "$work/src/frontend"
session frontend-session-analysis "$page" '"home"' '"home (edited)"' --check --own "$work/src/frontend" --classpath "$cp" --analysis-version 2 "$work/src/shared" "$work/src/frontend"
session frontend-session-deps "$page" '"home"' '"home (edited)"' --check --own "$work/src/frontend" --classpath "$cp" --analysis-version 3 "$work/src/shared" "$work/src/frontend"

# The JVM's side of the API's answer, of each version.
if command -v sbt > /dev/null 2>&1; then
  for v in api-answer api-deps-answer; do
    (cd integrations/sbt/analysis && ANALYSIS_ANSWER="$work/$v.json" ANALYSIS_CLASSES="$work/classes" timeout 300 sbt --server --batch "analysisCost" 2>&1) | grep "analysisCost run 5" | sed "s/.*analysisCost run 5: /jvm side ($v): /"
  done
fi
