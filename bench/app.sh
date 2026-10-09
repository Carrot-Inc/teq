#!/bin/bash
# The application corpus (bench/app) as a benchmark, on the release binary. For each side (frontend:
# shared + frontend; api: shared + api; both under JS over the cats and sourcecode jars) the phases of
# `teq compiler build --time` (parse, type, reach, emit, total, ms), the bytes of the development output
# (--split, every module summed) and of the --release output as one file, the run under node, the
# macro expansions of --profile (how many, their milliseconds, their share of the type phase, and
# the busiest macro), and a watch session: one file edited (a comment appended), rebuilt
# incrementally, its total and the modules rewritten. Then the API side under --target jvm:
# `teq compiler check --time`, or its first error. Every figure is the median of RUNS (default 3) with the
# range in brackets. The tables go to stdout in Markdown. SCALE scales the corpus (default 1).
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
TEQ=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
. tests/support/jars.sh
. bench/phases.sh
runs=${RUNS:-3}
scale=${SCALE:-1}
work=out/app-bench
cp=""
for name in scala-library cats-kernel cats-core sourcecode; do
  path=$(jar_of "$name")
  [ -e "$path" ] || { echo "bench/app: $name is not in the coursier cache"; exit 1; }
  cp="$cp:$path"
done
cp=${cp#:}
rm -rf "$work"
mkdir -p "$work"
timeout 120 python3 bench/app/gen.py "$work/src" --scale "$scale" > "$work/gen.log" 2>&1 || { tail -3 "$work/gen.log"; exit 1; }

# median [min–max] of the numbers on stdin, with a unit suffix
stats() {
  python3 -c '
import sys, statistics
xs = [float(x) for x in sys.stdin.read().split()]
if not xs: print("—"); sys.exit()
m = statistics.median(xs)
f = (lambda v: f"{v:.0f}") if m >= 100 else (lambda v: f"{v:.1f}")
print(f"{f(m)} [{f(min(xs))}–{f(max(xs))}]" + sys.argv[1])
' "$1"
}
# a duration such as 12.3ms, 850µs or 1.2s, in milliseconds
to_ms() {
  python3 -c '
import re, sys
t = sys.argv[1]
n = float(re.match(r"[0-9.]+", t).group())
u = t[len(re.match(r"[0-9.]+", t).group()):]
print(n * {"s": 1000, "ms": 1, "µs": 0.001, "us": 0.001, "ns": 0.000001}[u])
' "$1"
}
bytes_of() { wc -c < "$1" | tr -d ' '; }
dir_bytes() { cat "$1"/*.mjs | wc -c | tr -d ' '; }
count_lines() { find "$@" -name '*.scala' | xargs cat | wc -l | tr -d ' '; }
count_files() { find "$@" -name '*.scala' | wc -l | tr -d ' '; }

sides="frontend api"
dirs_of() {
  case $1 in
    frontend) echo "$work/src/shared $work/src/frontend" ;;
    api) echo "$work/src/shared $work/src/api" ;;
  esac
}

# --- Build, size and run time -------------------------------------------------------------------
echo "## Build, size and run time"
echo
echo "| side | files | lines | parse | type | reach | emit | total | dev split | release | node | output lines |"
echo "|---|---|---|---|---|---|---|---|---|---|---|---|"
for side in $sides; do
  dirs=$(dirs_of $side)
  : > "$work/$side.phases"
  for _ in $(seq 1 "$runs"); do
    line=$(timeout 300 "$TEQ" compiler build $dirs --classpath "$cp" --time -o "$work/$side.js" 2>&1 | phases) || { echo "| $side | build failed: $(timeout 300 "$TEQ" compiler build $dirs --classpath "$cp" -o "$work/$side.js" 2>&1 | grep -m1 error) | | | | | | | | | | |"; continue 2; }
    echo "$line" | python3 -c '
import re, sys
m = dict(re.findall(r"(read|parse|type|reach|emit|write|total) ([0-9.]+)", sys.stdin.read()))
print(" ".join(f"{float(m[k]):.2f}" for k in ("parse", "type", "reach", "emit", "total")))
' >> "$work/$side.phases"
  done
  timeout 300 "$TEQ" compiler build $dirs --classpath "$cp" --split "$work/$side-split" > /dev/null 2>&1
  timeout 300 "$TEQ" compiler build $dirs --classpath "$cp" --release -o "$work/$side.release.js" > /dev/null 2>&1
  node_ms=$(python3 -c '
import subprocess, sys, time, statistics
file, out, runs = sys.argv[1], sys.argv[2], int(sys.argv[3])
ts = []
for i in range(runs):
    t = time.perf_counter()
    r = subprocess.run(["node", file], capture_output=True, text=True, timeout=120)
    ts.append((time.perf_counter() - t) * 1000)
    if i == 0:
        open(out, "w").write(r.stdout + r.stderr)
m = statistics.median(ts)
print(f"{m:.0f} [{min(ts):.0f}–{max(ts):.0f}]")
' "$work/$side.release.js" "$work/$side.out" "$runs")
  cols=""
  for i in 1 2 3 4 5; do
    cols="$cols | $(cut -d' ' -f$i "$work/$side.phases" | stats "")"
  done
  echo "| $side | $(count_files $dirs) | $(count_lines $dirs)$cols | $(dir_bytes "$work/$side-split") | $(bytes_of "$work/$side.release.js") | $node_ms | $(wc -l < "$work/$side.out" | tr -d ' ') |"
done
echo

# --- Macro expansion ------------------------------------------------------------------------------
echo "## Macro expansion (\`teq compiler check --profile\`)"
echo
echo "| side | type phase | expansions | expansion time | share | busiest macro | sites | self time |"
echo "|---|---|---|---|---|---|---|---|"
for side in $sides; do
  dirs=$(dirs_of $side)
  timeout 300 "$TEQ" compiler check $dirs --classpath "$cp" --profile > "$work/$side.profile" 2>&1
  python3 - "$work/$side.profile" "$side" <<'PY'
import re, sys
text = open(sys.argv[1]).read()
phase = re.search(r"profile: type phase (\d+) ms", text)
exp = re.search(r"^\s+macro expansion\s+(\d+)\s+([0-9.]+) ms\s+([0-9.]+)%", text, re.M)
callees = text.split("macro expansions by callee:", 1)[1] if "macro expansions by callee:" in text else ""
sites = re.findall(r"^  (\S+) \((\d+) x, \d+ interpreter steps\): self ([0-9.]+) ms", callees, re.M)
top = max(sites, key=lambda s: float(s[2])) if sites else ("—", "0", "0")
print(f"| {sys.argv[2]} | {phase.group(1) if phase else '—'} ms | {exp.group(1) if exp else '—'} | {exp.group(2) if exp else '—'} ms | {exp.group(3) if exp else '—'}% | `{top[0]}` | {top[1]} | {top[2]} ms |")
PY
done
echo

# --- Watch mode -----------------------------------------------------------------------------------
echo "## Watch mode (one string literal changed in one file, rebuilt incrementally)"
echo
echo "| side | edited file | first build | incremental rebuild | modules rewritten |"
echo "|---|---|---|---|---|"
WPID=
start() {
  rm -f "$work/cmd" "$work/ans"
  mkfifo "$work/cmd" "$work/ans"
  "$TEQ" compiler watch "$@" < "$work/cmd" > "$work/ans" 2> "$work/watch.err" &
  WPID=$!
  exec 3> "$work/cmd" 4< "$work/ans"
  answer
}
stop() {
  if [ -n "$WPID" ]; then
    echo quit >&3 2> /dev/null
    exec 3>&- 4<&-
    wait "$WPID" 2> /dev/null
    WPID=
  fi
}
trap stop EXIT
RESULT=
answer() {
  RESULT=
  if ! read -r -t 120 RESULT <&4; then
    RESULT='{"ok":false,"timeout":true}'
  fi
}
total_of() { echo "$RESULT" | grep -o '"total":[0-9.]*' | head -1 | cut -d: -f2; }
changed_of() { echo "$RESULT" | grep -o '"changed":\[[^]]*\]' | head -1 | tr ',' '\n' | grep -c mjs; }
for side in $sides; do
  dirs=$(dirs_of $side)
  case $side in
    frontend) file=$(ls "$work"/src/frontend/frontend/page/*/*DetailPage.scala | head -1); pattern='"home[0-9]*"'; replacement='"home%d"' ;;
    api) file=$(ls "$work"/src/api/business/*/*Rules.scala | head -1); pattern='"id[0-9]*"'; replacement='"id%d"' ;;
  esac
  rm -rf "$work/$side-watch"
  start $dirs --classpath "$cp" --split "$work/$side-watch"
  first=$(total_of)
  : > "$work/$side.watch"
  : > "$work/$side.watch.modules"
  for i in $(seq 1 "$runs"); do
    sed -i.bak "s/$pattern/$(printf "$replacement" "$i")/" "$file"
    printf 'build %s\n\n' "$file" >&3
    answer
    total_of >> "$work/$side.watch"
    changed_of >> "$work/$side.watch.modules"
  done
  stop
  echo "| $side | ${file#$work/src/} | $first ms | $(stats " ms" < "$work/$side.watch") | $(sort -u "$work/$side.watch.modules" | tr '\n' ' ') |"
done
echo

# --- The JVM target -------------------------------------------------------------------------------
echo "## The API side under --target jvm (\`teq compiler check --time\`)"
echo
checked=$(timeout 300 "$TEQ" compiler check "$work/src/shared" "$work/src/api" --classpath "$cp" --target jvm --time 2>&1)
if line=$(echo "$checked" | phases); then
  echo "$line" | awk '{ printf "checked %s lines in %s ms\n", $2, $NF }'
else
  echo "$checked" | grep -m1 error
fi
