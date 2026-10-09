#!/bin/bash
# bench/view-cost.sh <a> <b> [runs] [program...]: the cost of the type store's overlays and the worker
# views at one worker through the fork, `teq compiler check` of each program
# under two settings taking turns run by run, after one run
# each that warms the caches. A setting is a binary with its switches, `<binary>[:<VAR=value>,...]`, run
# under TEQ_FORK=1 and TEQ_THREADS=1 with the switches added (`-` as a value unsets the variable). Per program the minimum of
# the instructions retired and of the wall time, and the median of the peak resident size, for each
# setting and b's against a's. The programs are the budget's (bench/programs.sh; by default the rule's
# five), `frontend` (the application's export check, its build file in VIEW_COST_APP) and `api` (the
# application's API check, as bench/app/api-check.sh runs it from APP_ROOT, APP_MODULES, APP_CLASSPATH, APP_FLAGS).
# The counts are those of `/usr/bin/time -l`, macOS only.
#
#   bench/view-cost.sh target/release/teq:TEQ_VIEW_READS=0 target/release/teq     # the views' cost
#   bench/view-cost.sh target/release/teq:TEQ_TYPE_OVERLAYS=off target/release/teq  # the overlays' path, against the control
absolute() { case $1 in /*) echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
[ $# -ge 2 ] || { echo "usage: bench/view-cost.sh <binary[:VAR=value,...]> <binary[:VAR=value,...]> [runs] [program...]" >&2; exit 2; }
[ "$(uname -s)" = Darwin ] || { echo "view-cost: the counts of /usr/bin/time -l are macOS's" >&2; exit 2; }
setting_a=$1 setting_b=$2
runs=${3:-3}
[[ "$runs" =~ ^[1-9][0-9]*$ ]] || { echo "view-cost: runs must be a positive count, not $runs" >&2; exit 2; }
shift 2
[ $# = 0 ] || shift
wanted=${*:-realistic-frontend realistic-api core-only depth_1 gview_1}
bin_of() { absolute "${1%%:*}"; }
env_of() { case $1 in *:*) echo "${1#*:}" | tr ',' ' ' ;; esac; }
a_bin=$(bin_of "$setting_a") b_bin=$(bin_of "$setting_b")
[ -x "$a_bin" ] && [ -x "$b_bin" ] || { echo "view-cost: no binary at $a_bin or $b_bin" >&2; exit 2; }
cd "$(dirname "$0")/.."
work=${VIEW_COST_WORK:-out/budget}
. bench/programs.sh || { echo "view-cost: the programs could not be generated" >&2; exit 1; }
. tests/support/compiler-words.sh
spellings "$a_bin" "$b_bin"
app_root=${APP_ROOT:-}
cache=out/view-cost-cache
mkdir -p "$cache/a" "$cache/b"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
# The application's two checks as tests/fork-one.sh runs them: the directory to run in and the arguments.
frontend_args() {
  node -e '
    const d = require(process.argv[1])
    const out = [...d.sources, "--classpath", d.classpath.join(":"), ...(d.maxInlines ? ["--max-inlines", String(d.maxInlines)] : []),
      ...(d.strictEquality ? ["--strict-equality"] : []), ...(d.kindProjector ? ["--kind-projector"] : []),
      ...(d.cacheableState ?? []).flatMap((n) => ["--cacheable-state", n])]
    process.stdout.write(out.map((a) => a + "\0").join(""))' "$VIEW_COST_APP"
}
frontend_root() { node -e 'const d = require(process.argv[1]); console.log(require("path").resolve(process.argv[1], "..", d.root ?? "../../.."))' "$VIEW_COST_APP"; }
api_args() {
  local cp
  cp=$(sed "s|^~/|$HOME/|" "$APP_CLASSPATH" | paste -sd: -)
  printf '%s\0' $(grep -v '^#' "$APP_MODULES") --classpath "$cp" $APP_FLAGS --std scala-library --target jvm
}
# one <binary> <switches> <cache> <program>: "instructions wall-seconds peak-bytes" of one check, a failure when
# no count comes out (a check's own errors are its program's; the application's API has one).
one() {
  local dir=. args=()
  case $4 in
    frontend) dir=$(frontend_root); while IFS= read -r -d '' x; do args+=("$x"); done < <(frontend_args) ;;
    api) dir=$app_root; while IFS= read -r -d '' x; do args+=("$x"); done < <(api_args) ;;
    *) read -r -a args <<< "$(program_args "$4")" ;;
  esac
  local envs=(TEQ_FORK=1 TEQ_THREADS=1 TEQ_CACHE_DIR="$PWD/$3") unset_args=() kv e kept
  for kv in $2; do
    case $kv in
      *=-)
        unset_args+=(-u "${kv%%=*}")
        kept=()
        for e in "${envs[@]}"; do [ "${e%%=*}" = "${kv%%=*}" ] || kept+=("$e"); done
        envs=("${kept[@]}") ;;
      *) envs+=("$kv") ;;
    esac
  done
  (cd "$dir" && env "${unset_args[@]}" "${envs[@]}" /usr/bin/time -l "$1" $(compiler_words "$1") check "${args[@]}" > /dev/null 2> "$tmp/time.txt")
  awk '/instructions retired/ { i = $1 } / real / { w = $1 } /maximum resident set size/ { m = $1 } END { if (i > 0) print i, w, m; else exit 1 }' "$tmp/time.txt"
}
a_env=$(env_of "$setting_a") b_env=$(env_of "$setting_b")
echo "view-cost: a = $a_bin ($($a_bin --version)) ${a_env:-no switches}; b = $b_bin ($($b_bin --version)) ${b_env:-no switches}; TEQ_FORK=1, $runs runs, load $(sysctl -n vm.loadavg | tr -d '{}' | awk '{print $1}')"
status=0
for prog in $wanted; do
  case $prog in
    frontend) [ -f "${VIEW_COST_APP:-}" ] || { echo "$prog: VIEW_COST_APP names no build file"; status=1; continue; } ;;
    api) [ -d "$app_root" ] && [ -f "${APP_MODULES:-}" ] && [ -f "${APP_CLASSPATH:-}" ] && [ -n "${APP_FLAGS+set}" ] || { echo "$prog: set APP_ROOT, APP_MODULES, APP_CLASSPATH and APP_FLAGS"; status=1; continue; } ;;
    *) [[ " $programs " == *" $prog "* ]] || { echo "$prog: not among the programs here"; status=1; continue; } ;;
  esac
  one "$a_bin" "$a_env" "$cache/a" "$prog" > /dev/null && one "$b_bin" "$b_env" "$cache/b" "$prog" > /dev/null || { echo "$prog: a check gave no count: $(tail -2 "$tmp/time.txt" | head -1)"; status=1; continue; }
  : > "$tmp/a" && : > "$tmp/b"
  for r in $(seq 1 "$runs"); do
    one "$a_bin" "$a_env" "$cache/a" "$prog" >> "$tmp/a" && one "$b_bin" "$b_env" "$cache/b" "$prog" >> "$tmp/b" || { status=1; break; }
  done
  python3 - "$prog" "$tmp/a" "$tmp/b" <<'PY' || status=1
import sys, statistics
prog, a, b = sys.argv[1], [list(map(float, l.split())) for l in open(sys.argv[2])], [list(map(float, l.split())) for l in open(sys.argv[3])]
if not a or len(a) != len(b):
    print(f"{prog}: {len(a)} and {len(b)} runs counted"); sys.exit(1)
ia, ib = min(x[0] for x in a), min(x[0] for x in b)
wa, wb = min(x[1] for x in a), min(x[1] for x in b)
ma, mb = statistics.median(x[2] for x in a), statistics.median(x[2] for x in b)
print(f"{prog}: instructions {ia/1e9:.2f}G -> {ib/1e9:.2f}G ({ib/1e9 - ia/1e9:+.2f}G, {(ib/ia - 1)*100:+.2f}%); "
      f"wall {wa:.2f}s -> {wb:.2f}s ({(wb/wa - 1)*100:+.1f}%); peak {ma/2**20:.0f} MB -> {mb/2**20:.0f} MB ({(mb - ma)/2**20:+.0f} MB)")
PY
done
exit $status
