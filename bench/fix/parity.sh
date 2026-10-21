#!/bin/bash
# bench/fix/parity.sh <teq> <corpus> <conf> <pairs> <workers...>: `teq fix`'s typing against `teq compiler check`'s at
# the same worker count over the same inputs (fix-parallel-typing's table, §8), the pair interleaved `pairs` times per
# count, cold (each run's TEQ_CACHE_DIR empty). Per count and pair, three runs:
#   fix:      `TEQ_FIX_INDEX=off teq compiler fix <corpus> --config <conf> --check --threads N`, the index off as compile
#             cannot have it (`--index` belongs to `teq compiler watch --check`); round one's attempts from the trace:
#             the terminal one's `type` (the jars and the typer made, set up and run: compile's boundaries) and `run`
#             (the sum of `Typer::run`'s phases), and an attempt that gave way's `run`;
#   check:    `TEQ_WORKERS_TIME=0 teq compiler check <corpus> --threads N --time`, with `--wunused imports` where the
#             pair's fix run read the unused imports (its record's `run` line, `semantic=1`: a semantic rule configured,
#             the engine's own predicate; `PARITY_CHECK_UNUSED=on|off` sets it otherwise, the runner's test of its
#             refusal); its `type` line (the process
#             that typed to the end: after a give-way, the one-worker build's) and the sum of its "workers" section's
#             phases (printed with the counters off where the build forks or gave way), and the sum of its "workers, the
#             attempt that gave way" section's phases;
#   indexed:  the fix run with the index on, as `teq fix` runs: as fix's, the index's own cost.
# Each side says its attempts (`attempts=`, 2 where one gave way), its terminal attempt's outcome (`one` or `joined`: fix's
# from its trace, compile's `joined` where its report's "workers" section has the `fork` phase) and workers
# (`effective=`: fix's from its trace; compile's N where it forked, else one), and the figures of corresponding attempts:
# the terminal `type` and `run` against the terminal ones, the attempt that gave way's `run` (`gave_way_run=`) against
# the other's. A pair is compared only where every side completed (exit 0 or 1, a `type` interval, a `run` where compile
# printed one) and the sides typed alike (the same attempts, outcome and workers): a line `<corpus> <N> <pair> pair
# compared`, else `... pair rejected: <why>`, whose figures are no parity pair. Each side prints `unused=`, whether it read
# the unused imports, which must be alike too. The corpora as bench/fix/run.sh's
# (bench/fix/corpora.sh), or a directory, whose options `PARITY_ARGS` gives (a class path); round two's input exists only
# inside a run, so round one alone is paired. Lines go to stdout, one a run: `<corpus> <N> <pair> <side> attempts=<n>
# type=<ms> run=<ms> gave_way_run=<ms> wall_ms=<ms> load=<1 min> exit=<code>`, then the pair's.
set -u
cd "$(dirname "$0")/../.."
teq=$1 corpus=$2 conf=$3 pairs=$4
shift 4
. tests/support/jars.sh
case $corpus in
  frontend)
    app_cp=""
    for name in scala-library cats-kernel cats-core sourcecode; do app_cp="$app_cp:$(jar_of "$name")"; done
    args=(out/budget/app/shared out/budget/app/frontend --classpath "${app_cp#:}" --cacheable-state meridian.web.css.Catalog) ;;
  jzon) args=("${FIX_CORPORA:-out/corpora}/jzon") ;;
  toy) args=("${FIX_CORPORA:-out/corpora}/toy") ;;
  *)
    [ -d "$corpus" ] || { echo "parity.sh: no corpus $corpus" >&2; exit 2; }
    read -r -a extra <<< "${PARITY_ARGS:-}"
    args=("$corpus" "${extra[@]}")
    corpus=$(basename "$corpus") ;;
esac
confpath=bench/fix/$conf
[ -f "$confpath" ] || confpath=$conf
# Whether a fix run read the unused imports: its record's `run` line (`semantic=`), `-` where it has none.
read_unused() {
  sed -nE 's/^run .*semantic=([01]).*/\1/p' "$1" 2> /dev/null | head -1 | grep . || echo -
}
work=out/measure/parity
mkdir -p "$work"
load() { cut -d' ' -f1 /proc/loadavg; }
# The trace's round-one attempts: the last one's `type` and the sum of its `run` phases, and those of the attempts that
# gave way before it.
fixed() {
  python3 - "$1" <<'PY'
import re, sys
attempts = []
for l in open(sys.argv[1], errors="replace"):
    m = re.match(r"fix: typing job 0 round 1 attempt (\d+): \d+ workers? asked, (\d+) typed, (\w+)(?: \(.*?\))?: read and parse .*?type (\d+) ms \(run: (.*?)\), documents", l)
    if m:
        attempts.append((m.group(3), int(m.group(2)), int(m.group(4)), sum(int(x) for x in re.findall(r" (\d+)(?:,|$)", m.group(5)))))
if not attempts:
    print("attempts=0 outcome=- effective=- type=- run=- gave_way_run=-")
else:
    gave = [a for a in attempts[:-1] if a[0] == "retried"]
    o, e, t, r = attempts[-1]
    print(f"attempts={len(attempts)} outcome={o} effective={e} type={t} run={r} gave_way_run={sum(a[3] for a in gave) if gave else '-'}")
PY
}
# The `--time` report: the `type` row, the "workers" section's phases, the "workers, the attempt that gave way" one's;
# the terminal build forked where its section has the `fork` phase, by N workers but where the mappings were refused.
checked() {
  python3 - "$1" "$2" <<'PY'
import re, sys
section, phases, t, forked, refused = None, {"workers": [], "attempt": []}, None, False, False
for l in open(sys.argv[1], errors="replace"):
    s = l.strip()
    if s == "workers, the attempt that gave way":
        section = "attempt"
    elif s == "workers":
        section = "workers"
    elif not l.startswith(" "):
        section = None
    if "no memory mapping for the type store's overlays" in l:
        refused = True
    m = re.match(r"\s*phase: (.*?)\s+([\d.]+) ms", l)
    if m and section:
        phases[section].append(float(m.group(2)))
        if section == "workers" and m.group(1) == "fork":
            forked = True
    m = re.match(r"\s*type\s+([\d.]+) ms", l)
    if m:
        t = round(float(m.group(1)))
n = 2 if phases["attempt"] else 1
run = round(sum(phases["workers"])) if phases["workers"] else "-"
gave = round(sum(phases["attempt"])) if phases["attempt"] else "-"
effective = int(sys.argv[2]) if forked and not refused else 1
print(f"attempts={n} outcome={'joined' if forked else 'one'} effective={effective} type={t if t is not None else '-'} run={run} gave_way_run={gave}")
PY
}
# verdict: why the pair is no parity pair, or nothing.
verdict() {
  local side why=""
  for side in fix check indexed; do
    local code=${side}_exit typ=${side}_type
    case ${!code} in 0 | 1) ;; *) why="$why $side exited ${!code};" ;; esac
    [ "${!typ}" != - ] || why="$why $side printed no type interval;"
  done
  if [ "$check_run" != - ] && { [ "$fix_run" = - ] || [ "$indexed_run" = - ]; }; then why="$why a side printed no run interval;"; fi
  for key in attempts outcome effective unused; do
    local f=fix_$key c=check_$key x=indexed_$key
    if [ "${!f}" != "${!c}" ] || [ "${!x}" != "${!c}" ]; then why="$why $key: fix ${!f}, check ${!c}, indexed ${!x};"; fi
  done
  echo "$why"
}
fix_unused=-
for n in "$@"; do
  for p in $(seq 1 "$pairs"); do
    for side in fix check indexed; do
      cache=$work/cache-$side
      rm -rf "$cache"
      out=$work/$corpus-$n-$p-$side
      start=$(date +%s%N)
      rm -f "$out.record"
      unused=()
      case ${PARITY_CHECK_UNUSED:-$fix_unused} in on | 1) unused=(--wunused imports) ;; esac
      case $side in
        fix) TEQ_FIX_INDEX=off TEQ_FIX_TRACE=1 TEQ_FIX_RECORD=$PWD/$out.record TEQ_CACHE_DIR=$PWD/$cache timeout 300 "$teq" compiler fix "${args[@]}" --config "$confpath" --check --threads "$n" > /dev/null 2> "$out.err" ;;
        indexed) TEQ_FIX_TRACE=1 TEQ_FIX_RECORD=$PWD/$out.record TEQ_CACHE_DIR=$PWD/$cache timeout 300 "$teq" compiler fix "${args[@]}" --config "$confpath" --check --threads "$n" > /dev/null 2> "$out.err" ;;
        check) TEQ_WORKERS_TIME=0 TEQ_CACHE_DIR=$PWD/$cache timeout 300 "$teq" compiler check "${args[@]}" "${unused[@]}" --threads "$n" --time > /dev/null 2> "$out.err" ;;
      esac
      code=$?
      wall=$((($(date +%s%N) - start) / 1000000))
      if [ "$side" = check ]; then
        got="$(checked "$out.err" "$n") unused=$([ ${#unused[@]} -gt 0 ] && echo 1 || echo 0)"
      else
        got="$(fixed "$out.err") unused=$(read_unused "$out.record")"
      fi
      printf -v "${side}_exit" '%s' "$code"
      for kv in $got; do printf -v "${side}_${kv%%=*}" '%s' "${kv#*=}"; done
      echo "$corpus $n $p $side $got wall_ms=$wall load=$(load) exit=$code"
    done
    why=$(verdict)
    if [ -z "$why" ]; then
      echo "$corpus $n $p pair compared"
    else
      echo "$corpus $n $p pair rejected:$why"
    fi
  done
done
