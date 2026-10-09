#!/bin/bash
# The memory of a resident session: `teq compiler watch` kept open over edits (tests/watch-memory/driver.mjs),
# its memory sampled after every build, against the budgets of tests/watch-memory/budgets.txt.
# The sessions: the split JavaScript build, the resident check and the JVM build over the core
# corpus (bench/gen.py, 51 files of 22 modules), each under body edits alone (`body`), with a
# definition added every tenth edit (`tenth`) and under the edits of a day (`day`: body edits, a
# text saved twice, a type error, parse errors with and without new names, their fixes, a
# definition added under a new name, its signature changed, the definition and a file added and
# removed); the development loop's shape, `--split --module-per-file --hot`, under the day's
# edits, and under full builds alone (`full`: a signature changed with every edit, half as many
# edits as the others, which is what a `dev` session grows by where it grows); the resident
# check with the navigation index (`--check --index`), asked its queries
# between the builds, and asked besides the references of a std definition, which demand every std
# file (`index-std-day`); the language server (`teq lsp`) over the corpus as a workspace, the
# edits the texts of open documents, its own process and its child measured each; and the
# frontend of the application corpus (bench/app) in the loop's shape, under the day's edits
# and under full builds alone, and under `--check`, its edits going round files that expand
# macros as well, its catalogue of class names declared to hold cacheable state as the
# application's is, left out without its jars in the coursier cache.
#
# The parallel typer's rows: a session without a count and `@1` type
# with one worker (`TEQ_SESSION_WORKERS=1`), the count their budgets were recorded at; a session
# named `<session>@<n>` types its first and every full build with n workers through the merge, its
# retypes one worker's (`TEQ_SESSION_WORKERS`, which the language server's child inherits), and
# passes only when each of those builds logged its n workers joined (`TEQ_SESSION_WORKERS_LOG`, the
# driver's check): the application's frontend under full builds and under the day's edits with
# `--check`, and the language server, at 2, 8 and 16 workers, and its full builds at 12, judged as
# the others are; and
# `frontend-hot-full@auto` at the automatic count, which passes only when every full build logged
# the one count of two or more it chose, held to the budget of `@<that count>` (on both platforms
# the cap's, 8). A build whose only errors are
# master's race of several workers (`MirroredElemLabels` rejected at an inline `derived`)
# counts as its edit's answer and is printed with the row, as known. A peak row,
# `peak-<corpus>@<w>`, is a fresh process's full build of the application's frontend or the core
# corpus with `--check`, no edit made, its peak through the typing and the merge (the physical
# footprint's lifetime peak on macOS, the resident size's high-water mark on Linux): `@1` the
# plain build, `@fork` one worker through the fork, `@8`, `@12` and `@16` workers; its ceiling is in
# MB, or at 8, 12 and 16 the parallel typer's memory aim as a ratio of the same run's plain
# build (twice, 2.5 times, provisional while the cap stays 8, and three times), where the recording
# measured it to hold.
# Selecting a ratio row runs its baseline too.
#
# The memory is the physical footprint on macOS and the resident size plus swap on Linux, where the
# language server's own process is measured by its anonymous memory plus swap, its other pages
# being the binary's (the driver's header). A budget holds two numbers per session and platform
# in the metric its row names: the growth per edit (least squares over the samples once the
# session is warm) and the memory at the end. A session fails when its memory at the end is more
# than TOLERANCE percent (default 15) over its budget, when its growth is more than GROWTH_SLACK KB
# per edit (default 1500) and more than TOLERANCE percent over its budget, when it has no budget,
# and when its line's metric is not its row's. A session that fails its budget runs again alone,
# no other session of the suite running and the machine's load fallen first where it falls
# (WATCH_MEMORY_QUIET_LOAD, WATCH_MEMORY_QUIET_SECONDS, below the sessions' list), and is judged by
# that run, the first failure printed; and one run of the suite at a time holds the machine
# (WATCH_MEMORY_LOCK, there too). The tolerances are what four runs of the
# suite spread over on the reference machine, which recorded the budgets: the memory at the end by 1 to 10%
# of a session's, the growth over forty edits by up to 1,030 KB per edit in the sessions that
# emit, whose arrays of a build are mapped and unmapped around the samples. The allocator this
# suite was written against grew the split sessions by 20 to 25 MB per edit and the others, with
# full builds, by 3.6 to 7 MB. WATCH_MEMORY_RECORD=1 rewrites the budgets of the sessions it
# runs on this platform and keeps the others; WATCH_MEMORY_RECORD=max raises a session's budget
# to the run's numbers where they are larger, which the rows at 2, 8 and 16 workers took from
# eight runs: a parallel full build's footprint moves with its workers' mappings, and their growth
# over twenty edits spread from 0.5 to 6.9 MB per edit with the live heap flat (a worker's heap left
# behind each build was 8 MB per worker, 64 MB per edit at eight). WATCH_MEMORY_EDITS sets the edits per session (default 40; 3000 is the long run; a session
# of full builds runs half of them). A session is judged by its own count
# (tests/watch-memory/judge.py): at the count its budget was recorded at its memory at the end
# is held to the budget; at 200 edits and more to the memory after its hundredth edit, which
# it may exceed by WATCH_MEMORY_FACTOR (default 1.5); any other count is refused before a
# session starts, since nothing would hold its memory at the end. A session's time is bounded
# by its edits. A session whose budget the reference machine recorded on its platform alone is
# declared so beside its row (an `only` line, judge.py's header) and is not run on another
# platform, which says so: `index-std-day`, recorded on Linux. WATCH_MEMORY_ONLY limits the
# sessions (a space-separated list), WATCH_MEMORY_JOBS the sessions running at once (default 4).
#
# The budgets are a gate against a regression and no proof of a bound: a growth of 1,000 KB
# per edit passes a budget of forty edits. The proof is the long run, whose factor holds the
# memory after 3,000 edits to the memory after the first hundred.
#
# What the numbers cannot see: memory the session freed and the system's allocator has not
# given back is counted as the session's (the requests under 1 MB, which the session does not
# map itself); a leak under the slack per edit passes the short run and shows in the long one,
# as memory at the end against the hundredth edit's.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
TEQ=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
. tests/support/jars.sh
edits=${WATCH_MEMORY_EDITS:-40}
jobs=${WATCH_MEMORY_JOBS:-4}
budgets=tests/watch-memory/budgets.txt
platform=$(uname -s)-$(uname -m)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

timeout 60 python3 bench/gen.py "$work/core" 51 22 > /dev/null || { echo "FAIL the core corpus"; exit 1; }
cp=""
for name in scala-library cats-kernel cats-core sourcecode; do
  path=$(jar_of "$name")
  [ -e "$path" ] || cp=missing
  [ "$cp" = missing ] || cp="$cp:$path"
done
cp=${cp#:}
if [ "$cp" != missing ]; then
  timeout 120 python3 bench/app/gen.py "$work/app" > /dev/null || { echo "FAIL the application corpus"; exit 1; }
fi

export BUDGETS=$budgets PLATFORM=$platform RECORD=$WATCH_MEMORY_RECORD TOLERANCE=${TOLERANCE:-15} GROWTH_SLACK=${GROWTH_SLACK:-1500} FACTOR=${WATCH_MEMORY_FACTOR:-1.5}
pass=0
fail=0
# The judge first: whether it fails what it has to.
if judged=$(python3 tests/watch-memory/judge.py self-test); then pass=$((pass + 1)); else
  echo "$judged"
  fail=$((fail + 1))
fi

# The sessions: name, workload, corpus and the watch arguments, SRC and OUT standing for the
# session's copy of the corpus and its output directory.
sessions=()
session() { sessions+=("$*"); }
for workload in body tenth day; do
  session split-$workload $workload core SRC --split OUT
  session check-$workload $workload core --check SRC
  session jvm-$workload $workload core SRC --target jvm -o OUT
done
session hot-day day core SRC --split OUT --module-per-file bench --hot
session hot-full full core SRC --split OUT --module-per-file bench --hot
session index-day day core --check --index SRC
session index-std-day day core --check --index SRC
session lsp-day day core lsp SRC
session frontend-hot-day day app SRC/shared SRC/frontend --classpath "$cp" --cacheable-state meridian.web.css.Catalog --split OUT --module-per-file meridian.frontend --hot
session frontend-hot-full full app SRC/shared SRC/frontend --classpath "$cp" --cacheable-state meridian.web.css.Catalog --split OUT --module-per-file meridian.frontend --hot
session frontend-check-day day app --check SRC/shared SRC/frontend --classpath "$cp" --cacheable-state meridian.web.css.Catalog
session frontend-hot-full@auto full app SRC/shared SRC/frontend --classpath "$cp" --cacheable-state meridian.web.css.Catalog --split OUT --module-per-file meridian.frontend --hot
for n in 2 8 16; do
  session frontend-hot-full@$n full app SRC/shared SRC/frontend --classpath "$cp" --cacheable-state meridian.web.css.Catalog --split OUT --module-per-file meridian.frontend --hot
  session frontend-check-day@$n day app --check SRC/shared SRC/frontend --classpath "$cp" --cacheable-state meridian.web.css.Catalog
  session lsp-day@$n day core lsp SRC
done
session frontend-hot-full@12 full app SRC/shared SRC/frontend --classpath "$cp" --cacheable-state meridian.web.css.Catalog --split OUT --module-per-file meridian.frontend --hot
for w in 1 fork 8 12 16; do
  session peak-frontend@$w peak app --check SRC/shared SRC/frontend --classpath "$cp" --cacheable-state meridian.web.css.Catalog
  session peak-core@$w peak core --check SRC
done

# The edits of a session and the seconds it may take: a build of the frontend that is full
# takes eight seconds with an unoptimised binary.
edits_of() {
  case $1 in
    full) echo $((edits / 2)) ;;
    peak) echo 0 ;;
    *) echo "$edits" ;;
  esac
}
# The settings a session's name asks for: its workers, and the log their joins are checked by. A
# session without a count and `@1` type with one worker, the count of their budgets.
settings_of() {
  case $1 in
    *@fork) echo "TEQ_FORK=1 TEQ_SESSION_WORKERS=1 TEQ_SESSION_WORKERS_LOG=$2/workers.log" ;;
    *@auto) echo "WATCH_MEMORY_WORKERS=auto TEQ_SESSION_WORKERS_LOG=$2/workers.log" ;;
    *@1) echo "TEQ_SESSION_WORKERS=1" ;;
    *@*) echo "TEQ_SESSION_WORKERS=${1##*@} TEQ_SESSION_WORKERS_LOG=$2/workers.log" ;;
    *) echo "TEQ_SESSION_WORKERS=1" ;;
  esac
}
# What a session's name asks for besides: `-std-` the references of a std definition, which demand
# every std file, its documents written under the session's own cache.
extra_of() {
  case $1 in
    *-std-*) echo "WATCH_MEMORY_STD=1 TEQ_CACHE_DIR=$2/cache" ;;
  esac
}
# A ratio row's baseline runs with it.
only=$WATCH_MEMORY_ONLY
for name in $WATCH_MEMORY_ONLY; do
  base=$(python3 tests/watch-memory/judge.py baseline "$name")
  [ -n "$base" ] && [[ " $only " != *" $base "* ]] && only="$only $base"
done
seconds_of() {
  local each=2
  [ "$2" = app ] && each=4
  [ "$1" = full ] && each=$((each * 3))
  local bound=$((60 + $3 * each))
  [ $bound -lt 300 ] && bound=300
  echo $bound
}
chosen=()
for line in "${sessions[@]}"; do
  set -- $line
  if [ -n "$only" ] && [[ " $only " != *" $1 "* ]]; then continue; fi
  if [ "$3" = app ] && [ "$cp" = missing ]; then continue; fi
  why=$(python3 tests/watch-memory/judge.py accepts "$1" "$(edits_of "$2")")
  case $? in
    0) ;;
    2)
      # By design, not for want of anything the machine should hold: the gate allows the line.
      echo "skipped (a budget of one platform): $why"
      continue
      ;;
    *)
      echo "FAIL $why"
      fail=$((fail + 1))
      continue
      ;;
  esac
  chosen+=("$line")
done
if [ $fail -gt 0 ]; then
  echo "$pass passed, $fail failed (no session started)"
  exit 1
fi

# One run of the suite at a time on the machine: a second run, of another gate or by hand, waits for the
# first to end (an exclusive lock on WATCH_MEMORY_LOCK, by default /tmp/teq-watch-memory.lock, which the
# sessions do not inherit), at most WATCH_MEMORY_LOCK_SECONDS (default 900), and then runs beside it and
# says so.
lock=${WATCH_MEMORY_LOCK:-/tmp/teq-watch-memory.lock}
if exec 9>> "$lock"; then
  python3 -c '
import fcntl, sys, time
end = time.time() + float(sys.argv[1])
while True:
    try:
        fcntl.flock(9, fcntl.LOCK_EX | fcntl.LOCK_NB)
        sys.exit(0)
    except BlockingIOError:
        if time.time() > end:
            sys.exit(1)
        time.sleep(1)' "${WATCH_MEMORY_LOCK_SECONDS:-900}" ||
    echo "watch-memory: another run of the suite held $lock for ${WATCH_MEMORY_LOCK_SECONDS:-900} s; running beside it"
else
  echo "watch-memory: no lock at $lock; running beside any other run of the suite"
fi

# start <session> <directory>: the session started in the background, on a copy of its corpus under the
# directory, its line and its errors there.
start() {
  local dir=$2 name workload corpus count a roots args
  set -- $1
  name=$1 workload=$2 corpus=$3
  shift 3
  rm -rf "$dir"
  mkdir -p "$dir"
  cp -R "$work/$corpus" "$dir/src"
  roots=("$dir/src")
  args=()
  [ "$corpus" = app ] && roots=("$dir/src/frontend")
  for a in "$@"; do
    a=${a//SRC/$dir/src}
    args+=("${a//OUT/$dir/out}")
  done
  count=$(edits_of "$workload")
  env -u TEQ_THREADS -u TEQ_SESSION_WORKERS -u TEQ_SESSION_THREADS -u TEQ_FORK -u TEQ_SESSION_WORKERS_LOG -u WATCH_MEMORY_WORKERS $(settings_of "$name" "$dir") $(extra_of "$name" "$dir") WATCH_MEMORY_WORKLOAD=$workload TEQ_NO_CACHE=1 timeout "$(seconds_of "$workload" "$corpus" "$count")" node tests/watch-memory/driver.mjs "$name" "$TEQ" "$count" "${roots[@]}" -- "${args[@]}" > "$dir/line" 2> "$dir/err" 9>&- &
}

running=0
names=()
for line in "${chosen[@]}"; do
  set -- $line
  names+=("$1")
  start "$line" "$work/$1"
  running=$((running + 1))
  if [ $running -ge "$jobs" ]; then
    wait
    running=0
  fi
done
wait

: > "$work/lines"
for name in "${names[@]}"; do
  if [ -s "$work/$name/line" ]; then
    cat "$work/$name/line" >> "$work/lines"
  else
    echo "FAIL $name: $(tail -2 "$work/$name/err" | tr '\n' ' ')"
    fail=$((fail + 1))
  fi
done

# A session that fails its budget runs again alone, nothing else of the suite running, and is judged by
# that run, its first failure printed: a parallel build's memory moves with the load beside it, which
# `frontend-hot-full@12` grows past its budget under (3.1 and 3.4 MB per edit where its builds took 1.35 s,
# against 0.3 to 1.2 MB at 0.82 to 0.87 s). So a run again first waits, at most WATCH_MEMORY_QUIET_SECONDS
# (default 120), for the machine's load over the last minute to fall under WATCH_MEMORY_QUIET_LOAD (default
# half its cores), and says what it waited and at what load it ran. A ratio row's baseline runs again with
# it. A recording runs nothing again.
quiet() {
  python3 - "${WATCH_MEMORY_QUIET_SECONDS:-120}" "${WATCH_MEMORY_QUIET_LOAD:-}" <<'EOF'
import os, sys, time
bound, limit = float(sys.argv[1]), float(sys.argv[2]) if sys.argv[2] else (os.cpu_count() or 2) / 2
start = time.time()
while os.getloadavg()[0] >= limit and time.time() - start < bound:
    time.sleep(5)
load = os.getloadavg()[0]
print(f"after {time.time() - start:.0f} s for the load to fall under {limit:g}, at {load:.1f}" + ("" if load < limit else ", over it"))
EOF
}
if [ -z "$WATCH_MEMORY_RECORD" ]; then
  again=""
  while IFS=$'\t' read -r name why; do
    [ -n "$name" ] || continue
    name=${name%-child}
    [[ "$again" == *" $name "* ]] || again="$again $name $(python3 tests/watch-memory/judge.py baseline "$name") "
    echo "watch-memory: $name runs again alone, after: $why"
  done <<< "$(python3 tests/watch-memory/judge.py failing "$work/lines")"
  for line in "${chosen[@]}"; do
    set -- $line
    name=$1
    [[ "$again" == *" $name "* ]] || continue
    echo "watch-memory: $name runs again $(quiet)"
    start "$line" "$work/$name.again"
    wait
    # The first run's lines go either way: a run again that gives none fails the session, and a
    # ratio row whose baseline it was is then held to no baseline, which fails it too.
    awk -v n="$name" '$1 != n && $1 != n "-child"' "$work/lines" > "$work/lines.kept"
    if [ -s "$work/$name.again/line" ]; then
      cat "$work/lines.kept" "$work/$name.again/line" > "$work/lines"
    else
      echo "FAIL $name, run again alone: $(tail -2 "$work/$name.again/err" | tr '\n' ' ')"
      mv "$work/lines.kept" "$work/lines"
      fail=$((fail + 1))
    fi
  done
fi
[ -n "$WATCH_MEMORY_KEEP" ] && cp "$work/lines" "$WATCH_MEMORY_KEEP"
FAILED=$fail PASSED=$pass TEQ_VERSION=$("$TEQ" --version) python3 tests/watch-memory/judge.py lines "$work/lines"
