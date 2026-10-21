#!/bin/bash
# bench/fix/run.sh <teq> <label> <corpus> <conf> <workers|-> <cold|warm> [extra env...]: one `teq compiler fix --check`
# run, fix-verb's measurement (its report's "teq fix's times", its script recreated by fix-speed): TEQ_FIX_TRACE's
# phases, /usr/bin/time -v's wall time and peak RSS, the exit, and a digest of both streams with the trace's own lines
# left out. Cold is an empty TEQ_CACHE_DIR of the label's own, warm the second process over the cache the cold run
# wrote. The corpora: `frontend` bench/app's generated `shared` and `frontend` (bench/programs.sh's), `jzon` scala3's
# tests/pos/jzon at 3.8.4, `toy` 1,000 files of 4 lines, under FIX_CORPORA (default out/corpora; bench/fix/corpora.sh
# makes them); `<conf>` a configuration of bench/fix, or a path. A run is accepted only as the measurement it claims to
# be (below): its row is then appended to FIX_RESULTS (default out/measure/results.tsv), its streams kept under
# out/measure/runs/; a rejected one appends to out/measure/rejected.tsv and exits 1. bench/fix/selftest.sh holds the
# runner to its rejections.
set -u
cd "$(dirname "$0")/../.."
teq=$1 label=$2 corpus=$3 conf=$4 workers=$5 temp=$6
shift 6
. tests/support/jars.sh
app_cp=""
for name in scala-library cats-kernel cats-core sourcecode; do app_cp="$app_cp:$(jar_of "$name")"; done
app_cp=${app_cp#:}
case $corpus in
  frontend) dirs=(out/budget/app/shared out/budget/app/frontend) opts=(--classpath "$app_cp" --cacheable-state meridian.web.css.Catalog) ;;
  jzon) dirs=("${FIX_CORPORA:-out/corpora}/jzon") opts=() ;;
  toy) dirs=("${FIX_CORPORA:-out/corpora}/toy") opts=() ;;
  *) echo "run.sh: no corpus $corpus" >&2; exit 2 ;;
esac
args=("${dirs[@]}" "${opts[@]}")
[ "$workers" = - ] || args+=(--threads "$workers")
# FIX_TYPING=<n> (fix-parallel-typing): the typing's workers apart from the rules' (`TEQ_FIX_TYPING_THREADS`), the label
# suffixed `-t<n>` so that the rows and the caches stay apart; the record's typing then held to that count.
# `FIX_TYPING=-`: the typing not held, for a binary whose record predates the typing's lines (master's before the part).
typing=$workers typing_env=()
if [ "${FIX_TYPING:-}" = - ]; then
  typing=-
elif [ -n "${FIX_TYPING:-}" ]; then
  typing=$FIX_TYPING label=$label-t$FIX_TYPING typing_env=(TEQ_FIX_TYPING_THREADS="$FIX_TYPING")
fi
results=${FIX_RESULTS:-out/measure/results.tsv}
cache=out/measure/cache/$label-$corpus-$(basename "$conf" .conf)-$workers
[ "$temp" = cold ] && rm -rf "$cache"
mkdir -p "$cache" out/measure/runs
base=out/measure/runs/$label-$corpus-$(basename "$conf" .conf)-$workers-$temp
rm -f "$base.record"
# A measurement accepted or none (fix-speed's ruling 7(e)): a corpus with no source, or another than the baseline's (its
# identity, the count and the digest of the paths and bytes of the inputs the engine resolves for the job, which
# `TEQ_FIX_INPUTS` asks of it — the compiler's own input set, its roots, links and exclusions — taken by
# bench/fix/inventory.py and refused before the measured run, the files it lacks against the baseline's inventory
# counted; `FIX_INVENTORY_TEQ` names the engine asked where the measured one predates the switch, the base's), a failed or incomplete job, a run whose record is not the run
# asked for (bench/fix/record.py), or one whose exit and bytes are not the baseline's (`FIX_EXPECT`,
# `<exit>:<digest>:<files>:<corpus digest>`, which bench/fix/matrix.sh passes from the cell's baseline run) appends no
# row and exits 1. An accepted row carries the corpus's identity, its twelfth field.
reject() {
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$label" "$corpus" "$(basename "$conf" .conf)" "$workers" "$temp" "$*" >> out/measure/rejected.tsv
  echo "REJECTED $label $corpus $(basename "$conf" .conf) w=$workers $temp: $*"
  exit 1
}
confpath=bench/fix/$conf
[ -f "$confpath" ] || confpath=$conf
[ -f "$confpath" ] || reject "no configuration $conf"
for d in "${dirs[@]}"; do
  [ -d "$d" ] || reject "no corpus at $d (bench/fix/corpora.sh makes it)"
done
mkdir -p out/measure/inventories
rm -f "$base.inputs"
TEQ_FIX_INPUTS=$PWD/$base.inputs timeout 300 "${FIX_INVENTORY_TEQ:-$teq}" compiler fix "${args[@]}" --config "$confpath" --check > /dev/null 2> "$base.inputs.err"
[ -f "$base.inputs" ] || reject "the engine listed no inputs ($(head -1 "$base.inputs.err")): FIX_INVENTORY_TEQ names one that lists them"
identity=$(python3 bench/fix/inventory.py "$base.inventory" "$base.inputs" "${dirs[@]}") || reject "the corpus's inventory failed"
mv "$base.inventory" "out/measure/inventories/${identity#*:}.txt"
[ "${identity%%:*}" -gt 0 ] || reject "no source file under ${dirs[*]}"
if [ -n "${FIX_EXPECT:-}" ]; then
  expected=$(echo "$FIX_EXPECT" | cut -d: -f3-4)
  if [ "$identity" != "$expected" ]; then
    inv=out/measure/inventories/${expected#*:}.txt
    lacks=-
    [ -f "$inv" ] && lacks=$(LC_ALL=C comm -23 <(cut -d' ' -f1 "$inv") <(cut -d' ' -f1 "out/measure/inventories/${identity#*:}.txt") | wc -l)
    reject "the corpus holds ${identity%%:*} source files ($identity), the baseline's $expected: $lacks of its files missing or changed"
  fi
fi
env "$@" "${typing_env[@]}" TEQ_CACHE_DIR=$PWD/$cache TEQ_FIX_TRACE=1 TEQ_FIX_RECORD=$PWD/$base.record timeout 300 /usr/bin/time -v -o "$base.time" "$teq" compiler fix "${args[@]}" --config "$confpath" --check > "$base.out" 2> "$base.err"
code=$?
wall=$(awk -F': ' '/Elapsed \(wall clock\)/ {n=split($2,a,":"); s=0; for(i=1;i<=n;i++) s=s*60+a[i]; print s}' "$base.time")
rss=$(awk -F': ' '/Maximum resident set size/ {print $2}' "$base.time")
phases=$(grep '^fix: .* ms in all' "$base.err" | sed 's/^fix: //')
digest=$( (cat "$base.out"; grep -v '^fix: ' "$base.err") | sha1sum | cut -c1-12)
# Exit 0 (nothing to report) and 1 (lints) are runs; 2 (a refusal, a program that does not type), a timeout or a crash
# are failures.
[ "$code" = 0 ] || [ "$code" = 1 ] || reject "teq exited $code: $(grep -v '^fix: ' "$base.err" | head -2 | tr '\n' ' ')"
[ -n "$phases" ] || reject "no trace of the run's phases"
# The rounds' utilization (bench/fix/record.py): a parallel-execution witness where a round has four documents a worker,
# a cost cell's utilization as it was elsewhere; a row's eleventh field, `<job>/<round>:<docs>d,<peak>/<workers>[w]`, `w`
# a witness round.
utilization=-
if [ -s "$base.record" ]; then
  verdict=$(python3 bench/fix/record.py "$base.record" "$workers" "$typing") || reject "the record: $(echo "$verdict" | grep -v '^utilization ' | head -3 | tr '\n' ';')"
  utilization=$(echo "$verdict" | awk '/^utilization / { for (i = 2; i <= NF; i++) { split($i, kv, "="); f[kv[1]] = kv[2] } printf "%s%s:%sd,%s/%s%s", (n++ ? " " : ""), $2, f["docs"], f["peak"], f["of"], (f["witness"] == "yes" ? "w" : "") }')
elif [ "$workers" != - ]; then
  reject "no execution record at $workers workers"
fi
if [ -n "${FIX_EXPECT:-}" ] && [ "$code:$digest" != "$(echo "$FIX_EXPECT" | cut -d: -f1-2)" ]; then
  reject "exit and bytes $code:$digest, the baseline's $(echo "$FIX_EXPECT" | cut -d: -f1-2)"
fi
# Each typing attempt (the trace's `fix: typing` lines): `<job>/<round>/<attempt>:<outcome>:<workers>:type=<ms>:run=<ms>`,
# `type` as `teq compiler check --time` counts its type phase, `run` the sum of `Typer::run`'s phases; a row's 13th field.
typings=$(python3 - "$base.err" <<'PY'
import re, sys
out = []
for l in open(sys.argv[1], errors="replace"):
    m = re.match(r"fix: typing job (\d+) round (\d+) attempt (\d+): (\d+) workers? asked, (\d+) typed, (\w+).*?type (\d+) ms \(run: (.*?)\), documents", l)
    if m:
        run = sum(int(x) for x in re.findall(r" (\d+)(?:,|$)", m.group(8)))
        out.append(f"{m.group(1)}/{m.group(2)}/{m.group(3)}:{m.group(6)}:{m.group(5)}:type={m.group(7)}:run={run}")
print(" ".join(out) or "-")
PY
)
printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$label" "$corpus" "$(basename "$conf" .conf)" "$workers" "$temp" "$code" "$wall" "$rss" "$digest" "$phases" "$utilization" "$identity" "$typings" >> "$results"
printf '%s %s %s w=%s %s: exit %s, %s s, %s KiB, %s; corpus %s; utilization %s; %s\n' "$label" "$corpus" "$(basename "$conf" .conf)" "$workers" "$temp" "$code" "$wall" "$rss" "$digest" "$identity" "$utilization" "$phases"
