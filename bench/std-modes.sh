#!/bin/bash
# The two std modes measured on the same programs: the phases of `teq compiler build --time`, the output
# in bytes (the development split, `--release`, and the release output through esbuild --minify
# and gzip), the run time under node, whether the output matches scalac's (the program's
# .expected where it has one, else the other mode's output), and an incremental build of watch
# mode after one string edit. Programs: hello world, the README benchmark (bench/gen.py, 51 files
# of 22 modules), the five programs of tests/size, money.scala and chain.scala over cats
# (tests/classpath/js), and bench/std-modes/library.scala over collections only scala-library
# has. Every figure is the median of RUNS (default 5) with the range in brackets, milliseconds
# unless stated. The Scala.js sizes of money.scala come from scala-cli when it can link offline
# (SJS=0 leaves that part out). The tables go to stdout in Markdown.
cd "$(dirname "$0")/.."
. bench/phases.sh
TEQ=${TEQ:-./target/release/teq}
runs=${RUNS:-5}
work=out/std-modes
mkdir -p "$work"
M2=${COURSIER_M2:-$HOME/Library/Caches/Coursier/v1/https/repo1.maven.org/maven2}
esbuild=${ESBUILD:-$(command -v esbuild)}
for candidate in node_modules/.bin/esbuild "$HOME/.config/yarn/global/node_modules/esbuild/bin/esbuild"; do
  [ -z "$esbuild" ] && [ -x "$candidate" ] && esbuild=$candidate
done
jar_of() {
  case $1 in
    scala-library) ls "$M2"/org/scala-lang/scala-library/3.*/scala-library-3.*.jar 2> /dev/null | grep -v sources | grep -v javadoc | sort -V | tail -1 ;;
    cats-kernel) echo "$M2/org/typelevel/cats-kernel_3/2.13.0/cats-kernel_3-2.13.0.jar" ;;
    cats-core) echo "$M2/org/typelevel/cats-core_3/2.13.0/cats-core_3-2.13.0.jar" ;;
  esac
}
# median and range of the numbers on stdin, as `median [min–max]` with the given unit
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
bytes_of() { wc -c < "$1" | tr -d ' '; }
dir_bytes() { cat "$1"/*.mjs | wc -c | tr -d ' '; }
# runs node on a file RUNS times: prints the median run time, saves the first output
node_time() {
  local file=$1 out=$2
  python3 -c '
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
' "$file" "$out" "$runs"
}

if [ ! -d "$work/readme" ]; then
  python3 bench/gen.py "$work/readme" 51 22 > /dev/null
fi
# name|source|jars|expected
programs="hello|bench/std-modes/hello.scala||
readme|$work/readme||
adt|tests/size/adt.scala||tests/size/adt.expected
classes|tests/size/classes.scala||tests/size/classes.expected
collections|tests/size/collections.scala||tests/size/collections.expected
typeclasses|tests/size/typeclasses.scala||tests/size/typeclasses.expected
money|tests/classpath/js/money.scala|scala-library cats-kernel cats-core|tests/classpath/js/money.expected
chain|tests/classpath/js/chain.scala|scala-library cats-kernel cats-core|tests/classpath/js/chain.expected
library|bench/std-modes/library.scala||"

echo "Machine: $(uname -m) $(sysctl -n machdep.cpu.brand_string 2> /dev/null), node $(node --version), $runs runs"
echo
echo "## Build, size and run time"
echo
echo "| program | std | parse | type | reach | emit | total | dev split | release | minified gz | node | scalac |"
echo "|---|---|---|---|---|---|---|---|---|---|---|---|"
while IFS='|' read -r name src jars expected; do
  [ -n "$name" ] || continue
  cp=""
  missing=""
  for jar in $jars; do
    path=$(jar_of "$jar")
    [ -e "$path" ] || missing="$missing $jar"
    cp="$cp:$path"
  done
  if [ -n "$missing" ]; then
    echo "| $name | | skipped: not in the coursier cache:$missing | | | | | | | | | |"
    continue
  fi
  cpflag=()
  [ -n "$cp" ] && cpflag=(--classpath "${cp#:}")
  for mode in lean scala-library; do
    tag=$name-$mode
    rm -rf "$work/$tag-dev"
    phases=""
    ok=1
    for i in $(seq 1 "$runs"); do
      line=$(timeout 120 "$TEQ" compiler build "$src" "${cpflag[@]}" --std=$mode --split "$work/$tag-dev" --time 2>&1 | phases) || { ok=0; break; }
      phases="$phases
$line"
    done
    if [ $ok = 0 ]; then
      reason=$(timeout 120 "$TEQ" compiler build "$src" "${cpflag[@]}" --std=$mode --split "$work/$tag-dev" 2>&1 | grep -m1 error | cut -c1-80)
      echo "| $name | $mode | does not build: $reason | | | | | | | | | |"
      continue
    fi
    phase() {
      echo "$phases" | grep -o "$1 [0-9.]*" | awk '{ print $2 }' | stats ""
    }
    dev=$(dir_bytes "$work/$tag-dev")
    timeout 120 "$TEQ" compiler build "$src" "${cpflag[@]}" --std=$mode --release -o "$work/$tag.js" > /dev/null 2>&1
    release=$(bytes_of "$work/$tag.js")
    if [ -n "$esbuild" ]; then
      "$esbuild" "$work/$tag.js" --minify --log-level=error > "$work/$tag.min.js"
      mingz=$(gzip -9 -c "$work/$tag.min.js" | wc -c | tr -d ' ')
    else
      mingz="-"
    fi
    run=$(node_time "$work/$tag.js" "$work/$tag.out")
    if [ -n "$expected" ]; then
      if diff -q "$expected" "$work/$tag.out" > /dev/null; then match=yes; else match=no; fi
    elif [ "$mode" = scala-library ] && [ -f "$work/$name-lean.out" ]; then
      if diff -q "$work/$name-lean.out" "$work/$tag.out" > /dev/null; then match="same as lean"; else match="differs from lean"; fi
    else
      match="n/a"
    fi
    echo "| $name | $mode | $(phase parse) | $(phase type) | $(phase reach) | $(phase emit) | $(phase total) | $dev | $release | $mingz | $run | $match |"
  done
done <<< "$programs"

echo
echo "## Watch mode: an incremental build after one string edit"
echo
echo "| std | first build | type | incremental total | modules rewritten |"
echo "|---|---|---|---|---|"
for mode in lean scala-library; do
  rm -rf "$work/watch-src" "$work/watch-$mode"
  cp -r tests/split/classpath "$work/watch-src"
  python3 - "$TEQ" "$work/watch-src" "$work/watch-$mode" "$mode" "$runs" <<'EOF'
import json, subprocess, sys, statistics
teq, src, out, mode, runs = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], int(sys.argv[5])
p = subprocess.Popen([teq, "compiler", "watch", src, "--split", out, "--std=" + mode], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
first = json.loads(p.stdout.readline())
if not first.get("ok"):
    print(f"| {mode} | does not build: {first.get('errors', [{}])[0].get('message', '')[:60]} | | | |")
    p.stdin.write("quit\n"); p.stdin.close(); p.wait()
    sys.exit()
path = src + "/util.scala"
totals, types, changed = [], [], None
for i in range(runs):
    text = open(path).read()
    text = text.replace('"["', '"<"') if '"["' in text else text.replace('"<"', '"["')
    open(path, "w").write(text)
    p.stdin.write(f"build {path}\n\n"); p.stdin.flush()
    r = json.loads(p.stdout.readline())
    if not r.get("ok") or not r.get("incremental"):
        print(f"| {mode} | {first['ms']['total']:.1f} | not incremental: {r.get('fallback') or r.get('errors')} | | |")
        break
    totals.append(r["ms"]["total"]); types.append(r["ms"]["type"]); changed = r["changed"]
else:
    f = lambda xs: f"{statistics.median(xs):.1f} [{min(xs):.1f}–{max(xs):.1f}]"
    print(f"| {mode} | {first['ms']['total']:.0f} (type {first['ms']['type']:.0f}) | {f(types)} | {f(totals)} | {', '.join(changed)} |")
p.stdin.write("quit\n"); p.stdin.close(); p.wait()
EOF
done

if [ "${SJS:-1}" != 0 ] && command -v scala-cli > /dev/null; then
  echo
  echo "## Scala.js: money.scala"
  echo
  echo "| module kind | mode | raw | minified | minified gz | node |"
  echo "|---|---|---|---|---|---|"
  for kind in default es; do
    for jsmode in dev full; do
      kindflag=()
      ext=js
      if [ $kind = es ]; then
        kindflag=(--js-module-kind es)
        ext=mjs
      fi
      file=$work/money-sjs-$kind-$jsmode.$ext
      if ! timeout 300 scala-cli --power package tests/classpath/js/money.scala --js --js-mode $jsmode "${kindflag[@]}" -o "$file" -f --offline > "$work/sjs-$kind-$jsmode.log" 2>&1; then
        echo "| $kind | $jsmode | skipped: $(grep -m1 -i 'not found\|error' "$work/sjs-$kind-$jsmode.log" | cut -c1-90) | | | |"
        continue
      fi
      raw=$(bytes_of "$file")
      if [ -n "$esbuild" ]; then
        "$esbuild" "$file" --minify --log-level=error > "$file.min"
        min=$(bytes_of "$file.min")
        mingz=$(gzip -9 -c "$file.min" | wc -c | tr -d ' ')
      else
        min="-"
        mingz="-"
      fi
      run=$(node_time "$file" "$file.out")
      echo "| $kind | $jsmode | $raw | $min | $mingz | $run |"
    done
  done
fi
