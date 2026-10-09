#!/bin/bash
# The type store's overlays through the fork: the gate
# of the crossings' migration, at one worker (the base's later growth, stage 2) and at two and more
# (the peers' crossings and the interpreter's, stages 3 and 4), which tests/workers.sh (one worker
# against two and sixteen, the output compared) is not.
#
#   tests/fork-one.sh [--suites <name,...>|none] [--overlays on|off] [--threads <n>] [--giveways bypassed|normal]
#                     [--reference one|off] [--repeat <n>] [--baseline-runs <n>] [--app <build.json>] [--diagnostics [<counts>]]
#                     [--bundles [<counts>]] [--known on|off]
#   tests/fork-one.sh --self-test      # the runner's own logic on made-up inputs
#
# Every suite named (by default the ones below) runs at the worker count asked for (1 by default,
# under TEQ_FORK=1, whose teq runs take the fork's path with one worker; more under TEQ_THREADS,
# with the state give-way bypassed by default (`TEQ_STATE_GIVEWAY=off`) so that the parallel path runs to its end, `--giveways
# normal` for the default policy, whose attempt may give way to one worker) with the type store's
# overlays (`--overlays on`, every forked build's default; `off` the control, TEQ_TYPE_OVERLAYS=off,
# whose runs prove no enforcement), `--repeat` times (default once), each run
# against the reference: one worker's run without the fork (`--reference one`, the default), the
# output the parallel path has to give; or the overlays off at the same count (`--reference off`).
# A suite's failures are its lines that name a failing test (`FAIL <test>`,
# the names after `REGRESSIONS:`), each with its failure's text normalised (its block of the log,
# its files in the suite's directory and every run of the compiler that names it, which the suite
# makes through tests/support/fork-one-teq.sh); the gate is that the overlays add no failing test
# and fail none of the baseline's otherwise, the suite's own expected failures and the fork's own,
# which the baseline has too, kept, and that every run completes (a run its bound or a signal
# ended fails). Against one worker, every run of the compiler is compared as well, a test the suite
# passes in both included: its exit and its streams, by its directory and arguments, the timing
# report and `teq tasty`'s durations left out (`outputs otherwise`). Against the overlays off
# above one worker the baseline runs three times (`--baseline-runs`), since the parallel path's
# own schedule moves some diagnostics from run to run with the overlays off as well: a failure is
# new where no baseline run failed so, and the tests the baseline's runs fail otherwise from one to
# the next are listed apart; one worker's reference runs once. A test that gives `--threads 1` itself types without the fork above one worker and is counted
# with its own count, not as unproven. A new failure tests/support/fork-one-known.txt names for its
# suite and the give-ways' setting is counted apart as known (a deferral with what exposes it), and
# its invocations are left out of the comparison of every invocation; `--known off` applies no
# known list, every test compared and counted (a known entry's evidence, or its removal's). With --app, the application's two lines as bench/app runs them, the overlays against
# the baseline at the same count: the export (the build file sbt-teq's teqExport writes) built file
# by file identical, and the API's check with the same diagnostics (today one error, the several
# entry points a check without --main cannot choose from). --diagnostics (the counts 2, 4 and 16
# unless given, `2,4,16`) runs the application's two checks with the overlays on at each count,
# `--repeat` times (default 1), and gates each run's diagnostics against one worker's (the plain
# path, no fork): the same exit and the same diagnostics, text and location, every run of every
# count, with the enforcement proven for each as below; under the give-ways' policy asked for.
# --bundles (the counts 2, 4 and 16 unless given) runs the bundles' enforcement matrix
# (tests/support/fork-one-bundles.txt) with the assertion-enabled
# build, the overlays and the state give-way bypassed: each named case at each count, `--repeat` times,
# its one invocation proven as below and its output one worker's without the fork (its files too),
# or, for a refusal test, ended by the refusal named; and each of its cells, the bundles' counts its
# own invocation's joins wrote (`fork_one.py bundles`), above zero in every run apart.
#
# TEQ names the binary (default target/release/teq). With the assertion-enabled build (`cargo build
# --profile checks`, target/checks/teq, whose `--version` says `assertions`) every violation of the
# worker-view checks is a panic (src/types/view.rs), so the same run is the enforcement's gate; its
# outcome is apart from any timing. The enforcement is proven active, not assumed: the runner sets
# TEQ_VIEW_CHECKS=panic itself and refuses an environment that asks for another mode (`report` or
# `off`, which let a violation pass); it prints the binary's revision and profile and the switches
# in effect; each overlays run writes the checks' log (TEQ_VIEW_CHECKS_LOG, a start with the worker
# count given, a line per worker naming it and its fork (or its slot left empty, a worker the queue had
# no work for and that was never started), a line per join with the crossings it
# exercised or per attempt given way, and an end per compiler invocation with its forks), the suites
# run the compiler through tests/support/fork-one-teq.sh, which adds each invocation's outcome, and
# a run fails whose log does not account for every invocation (`enforced`: every worker checked in
# the namespace and the mode asked for at the count asked for, every fork's workers named and
# joined, the end's forks the joins, an attempt given way followed by the build typed again, an
# image with no end ended by a signal); each line says the crossings its runs exercised, since a
# checked context proves nothing about whether a peer's record was read; and it runs the arenas',
# the store's, the symbols' and the program's unit tests, the checks' refusals among them (a peer's
# record read raw at two threads refused, the same record read through the view accepted, an owner's
# write after a peer's read refused, a write to a sealed record refused with no peer's read), built
# from the same tree (`cargo test --profile checks`). The suites' logs
# go to out/fork-one/. The workers and split-determinism suites set their own worker counts and are
# not among the suites.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
export TEQ
suites="cases errors interop dce fold interp jvm stdlib classpath split app modules"
overlays=on
app=
diagnostics=0
diag_counts="2 4 16"
bundles=0
bundle_counts="2 4 16"
threads=1
giveways=bypassed
repeat=1
base_runs=
reference=one
known_list=tests/support/fork-one-known.txt
usage() {
  echo "usage: tests/fork-one.sh [--suites <name,...>] [--overlays on|off] [--threads <n>] [--giveways bypassed|normal] [--reference one|off] [--app <build.json>] [--diagnostics [<counts>]] [--bundles [<counts>]] [--known on|off] [--repeat <n>] | --self-test" >&2
  exit 2
}
while [ $# -gt 0 ]; do
  case $1 in
    --suites) [ -n "$2" ] || usage; suites=${2//,/ }; [ "$suites" = none ] && suites=; shift 2 ;;
    --overlays) case $2 in on|off) overlays=$2 ;; *) usage ;; esac; shift 2 ;;
    --threads) [[ $2 =~ ^[0-9]+$ ]] && [ "$2" -ge 1 ] || usage; threads=$2; shift 2 ;;
    --giveways) case $2 in bypassed|normal) giveways=$2 ;; *) usage ;; esac; shift 2 ;;
    --repeat) [[ $2 =~ ^[0-9]+$ ]] && [ "$2" -ge 1 ] || usage; repeat=$2; shift 2 ;;
    --reference) case $2 in one|off) reference=$2 ;; *) usage ;; esac; shift 2 ;;
    --baseline-runs) [[ $2 =~ ^[0-9]+$ ]] && [ "$2" -ge 1 ] || usage; base_runs=$2; shift 2 ;;
    --app) [ -n "$2" ] || usage; app=$(cd "$(dirname "$2")" && pwd)/$(basename "$2"); shift 2 ;;
    --diagnostics)
      diagnostics=1; shift
      if [[ ${1:-} =~ ^[0-9]+(,[0-9]+)*$ ]]; then diag_counts=${1//,/ }; shift; fi ;;
    --bundles)
      bundles=1; shift
      if [[ ${1:-} =~ ^[0-9]+(,[0-9]+)*$ ]]; then bundle_counts=${1//,/ }; shift; fi ;;
    --known) case $2 in on) known_list=tests/support/fork-one-known.txt ;; off) known_list=/dev/null ;; *) usage ;; esac; shift 2 ;;
    --self-test) self_test=1; shift ;;
    *) usage ;;
  esac
done
# The reference's runs per suite: one worker's once; the overlays off at the count once at one
# worker and three times above, where the parallel path's own schedule moves some failures from run
# to run.
[ -n "$base_runs" ] || { [ $reference = one ] || [ $threads = 1 ] && base_runs=1 || base_runs=3; }
[ $reference = one ] && ref_name="one worker" || ref_name=baseline
# reference_env: the environment of the reference's runs: one worker without the fork, or the
# overlays off at the count asked for.
reference_env() {
  if [ $reference = one ]; then
    echo -u TEQ_TYPE_OVERLAYS -u TEQ_FORK -u TEQ_STATE_GIVEWAY TEQ_THREADS=1
  else
    echo TEQ_TYPE_OVERLAYS=off $(at $threads)
  fi
}
# reference_shell: the same as shell commands, for a run in a subshell.
reference_shell() {
  if [ $reference = one ]; then
    echo "unset TEQ_TYPE_OVERLAYS TEQ_FORK TEQ_STATE_GIVEWAY; export TEQ_THREADS=1"
  else
    echo export TEQ_TYPE_OVERLAYS=off $(at $threads)
  fi
}
# overlays_env: the overlays' runs' switch.
overlays_env() {
  [ $overlays = on ] && echo TEQ_TYPE_OVERLAYS=shared || echo TEQ_TYPE_OVERLAYS=off
}
# at <n>: the environment of a run at n workers: one through the fork, or n under TEQ_THREADS with
# the give-ways as asked.
at() {
  if [ "$1" = 1 ]; then
    echo TEQ_FORK=1 TEQ_THREADS=1
  else
    echo TEQ_THREADS=$1
    [ $giveways = bypassed ] && echo TEQ_STATE_GIVEWAY=off
  fi
}
# normalise: a suite's log, the runner's own text, with the numbers of the reports in it that
# differ from run to run and say nothing of a failure (a panic's thread number, a view check's store
# id, a temporary directory in a tool's report of a missing file, `diff: /tmp/tmp.XXXXXX/walk: No
# such file or directory`, a phase's time in a diff of a `--time` report, which a forked build
# prints), and without the shell's report of a killed command, which the shell
# prints where it reaps the command, in that test's block or the one before, from run to run. A
# test's own files, the program's output and the compiler's diagnostics among them, are never
# normalised (`identities`): their digest is of their bytes, which `TEQ_PANIC_REPORT=plain` keeps
# the same from run to run where the compiler panics (its report without the thread).
normalise() {
  # A panic's report comes after an empty line or not, as the threads' output falls, in a log and
  # in a diff of the output alike (whose lines carry `< ` or `> `).
  awk '{ c = $0; sub(/^[<>] ?/, "", c) }
    c == "" { held[n++] = $0; next }
    { if (c !~ /^(a )?thread (.* )?panicked at /) for (i = 0; i < n; i++) print held[i]; n = 0; print }
    END { for (i = 0; i < n; i++) print held[i] }' |
  sed -E -e "s/^([<>] )?thread '([^']*)' \([0-9]+\) panicked at /\1thread '\2' (N) panicked at /" \
    -e '/^view check: /s/ \(id [0-9]+\)/ (id N)/g' \
    -e '/^[<>] +phase: /s/[0-9]+\.[0-9]+ ms/N ms/' \
    -e '/^[a-z][a-z0-9_-]*: \/.*: (No such file or directory|Not a directory|Is a directory|Permission denied)$/s/\/tmp\.[A-Za-z0-9]{6,}\//\/tmp.N\//g' \
    -e '/^.*: line [0-9]+: +[0-9]+ (Aborted|Killed|Segmentation fault|Bus error|Trace\/BPT trap) /d'
}
# identities <log> <dir> <names file> [<records>]: per failing test, its name and a digest of its
# failure: its block of the log (its FAIL line and the lines after it up to the next), normalised,
# its files in the suite's directory byte for byte, the compiled output left out, and every
# invocation of the compiler the run kept that names it (`records`: the suite ran the compiler
# through tests/support/fork-one-teq.sh), each with its arguments, its exit or signal and all it
# wrote, per target and std variant, the schedule's noise left out (tests/support/fork_one.py); a
# test failing in both runs fails the same way only when the two digests are one.
identities() {
  local log=$1 dir=$2 name f
  local kept=${4:-}
  local runs
  runs=$(python3 tests/support/fork_one.py records "${kept:-/nonexistent}" "$3")
  while IFS= read -r name; do
    [ -n "$name" ] || continue
    { awk -v n="$name" '/^(REF )?FAIL / { t = ($1 == "REF") ? $3 : $2; sub(/:$/, "", t); on = (t == n); } /^(REGRESSIONS:|[0-9]+ passed)/ { on = 0 } on' "$log" | normalise
      if [ -n "$dir" ]; then
        for f in "$dir/$(basename "$name" .scala)".*; do
          case $f in *.js|*.mjs|*.jar|*.class|*.map) ;; *) [ -f "$f" ] && cat "$f" ;; esac
        done
      fi
      awk -v n="$name" '$1 == n { print $2 }' <<< "$runs"
    } | cksum | awk -v n="$name" '{ print n, $1 }'
  done < "$3"
}
# enforced <checks log>: whether the assertion-enabled build's log accounts for every compiler
# invocation of the run with the enforcement active at the worker count asked for ($requested)
# (tests/support/fork_one.py, `enforced`). Each invocation writes a start line (with the worker
# count it was given and where from: the command line, `TEQ_THREADS`, or none), a line as each
# worker begins (its namespace, the mode, whether its reads are checked, which worker of how many
# in which fork), a line per fork at its join (the counts and the crossings exercised) or, for a
# parallel attempt that gave way to one worker, a gave-way line before the process's image is
# replaced by the build typed again (whose start line says so), and an end line at its exit with
# its forks (`TEQ_VIEW_CHECKS_LOG`); the suites' runs of the compiler add each invocation's outcome
# (`exit <pid> <status>`, tests/support/fork-one-teq.sh). One is proven when every worker it
# began is in a checked context in the namespace asked for and the mode that panics, each of its
# forks has every worker of its count named and joined, that count is the one asked for (or,
# where a test gives `--threads` itself, the test's own, counted apart), every join it wrote
# agrees, and its end line's forks are its joins, each with the overlays: checked when a join
# counted checks; checked and given way when the attempt's workers were and the build typed again
# by one worker followed and ended; with nothing read when its joins counted none (every body
# typed before the fork, or the items left to the workers reading no type); ended before its join
# when it wrote no end and its outcome is a signal (an abort the baseline has too). One that typed
# no program (its end line says so, with no worker) is counted apart. Any other fails the run: a
# program typed with no worker in the overlays, a worker outside the checked context, another
# namespace, mode or worker count, a worker of a fork missing, a join that disagrees, an end whose
# forks are not the joins, an attempt given way with no build after it, an image that ended with
# neither an end line nor a signal. `enforcement` says what was counted, `crossings` the
# crossings the joins and the attempts exercised, summed. A release build's run has nothing to
# prove.
enforced() {
  [ $checks = 1 ] && [ $overlays = on ] || return 0
  local counts
  counts=$(python3 tests/support/fork_one.py enforced "$1" "$ns_name" "${requested:-1}" 2> /dev/null)
  local n proven gave idle aborted plain own unforked unproven total c1 c2 c3 c4 c5 c6 c7 c8 c9 list
  read -r n proven gave idle aborted plain own unforked unproven total c1 c2 c3 c4 c5 c6 c7 c8 c9 list <<< "${counts:-0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0}"
  # A test's own `--threads 1` above one worker types without the fork, which leaves nothing to prove.
  local of_one=
  [ "${unforked:-0}" -gt 0 ] && of_one=" ($unforked of them its own one worker's, which forks nothing)"
  enforcement="$n invocations at ${requested:-1} workers: $proven checked, $gave checked and given way, $idle checked with nothing read, $aborted ended before their join, $plain typed no program, $own at a test's own count$of_one, $unproven unproven${list:+ ($list)}; $total checks"
  crossings="crossings: peer records $c1 signatures, $c2 classes, $c3 type parameters, $c4 aliases ($c5 translated); $c6 expression types; $c7 consumers' types; $c8 holder's own; $c9 holder's runs"
  [ "$unproven" = 0 ] && [ $((proven + gave)) -gt 0 ]
}
# compare <prefix> <runs>: the overlays' run's failures against the baseline's runs, from
# <prefix>.base.<r>.fail and .ids (r from 1) and <prefix>.flag.fail and .ids: `new`, failing in no
# baseline run; `changed`, failing in one but otherwise than in every one (another message, exit or
# output); `gone`, failing in every baseline run and passing now; `unstable`, the tests the
# baseline's runs fail otherwise from one to the next or fail in some and pass in others.
compare() {
  local p=$1 runs=$2 flag=${3:-flag} r
  sort -u "$p".base.*.fail > "$p.base.fail"
  sort -u "$p".base.*.ids > "$p.base.ids"
  new=$(comm -13 "$p.base.fail" "$p.$flag.fail")
  changed=$(comm -12 "$p.base.fail" "$p.$flag.fail" | while IFS= read -r t; do
    grep -qxF "$(grep -F "$t " "$p.$flag.ids" | head -1)" "$p.base.ids" || echo "$t"
  done)
  gone=$(for r in $(seq 1 $runs); do cat "$p.base.$r.fail"; done | sort | uniq -c | awk -v n=$runs '$1 == n { $1 = ""; sub(/^ /, ""); print }' | comm -23 - "$p.$flag.fail")
  unstable=$( { for r in $(seq 1 $runs); do comm -3 "$p.base.fail" "$p.base.$r.fail" | tr -d '\t'; done
    awk '{ NF--; print }' "$p.base.ids" | uniq -d; } | grep . | sort -u)
}
# --self-test: the runner's own logic on made-up inputs, no binary run: the enforcement's
# reconciliation of the checks' log per invocation and the normalisation of a failure's text.
if [ "${self_test:-0}" = 1 ]; then
  checks=1
  ns_name=Shared
  tmp=$(mktemp -d "${TMPDIR:-/tmp}/fork-one.XXXXXX") || exit 1
  fails=0
  expect() {
    if [ "$2" = "$3" ]; then echo "ok    $1"; else echo "FAIL  $1: got '$2', wanted '$3'"; fails=$((fails + 1)); fi
  }
  requested=1
  # w <pid> <k> <n> [<namespace> <mode> <context>]: a worker's line; j <pid> <n> <checks> [...]: a join's.
  w() { echo "view checks: worker $1: the ${4:-Shared} namespace, mode ${5:-panic}, ${6:-in a checked context}; worker $2 of $3 in fork 1"; }
  j() { echo "view checks: join $1: the ${4:-Shared} namespace, $2 workers, mode ${5:-panic}, $3 reads and constructions checked, 1 publications checked, 3 items typed by the workers, ${6:-$2} of $2 workers in a checked context; fork 1; crossings: peer records 1 signatures, 2 classes, 3 type parameters, 4 aliases, 5 translated; 6 expression types; 7 consumers' types; 8 holder's own; 9 holder's runs"; }
  checked="view checks: start 10 run a.scala; threads automatic
$(w 10 0 1)
$(j 10 1 900)
view checks: end 10: typed, 1 forks, 1 with the overlays"
  # The bundles' cells (`fork_one.py bundles`): each invocation's own counts, every cell above zero.
  bj() { echo "view checks: join $1: the Shared namespace, 2 workers, mode panic, 9 reads and constructions checked, 1 publications checked, 3 items typed by the workers, 2 of 2 workers in a checked context; fork 1; crossings: peer records 1 signatures, 2 classes, 3 type parameters, 4 aliases, 5 translated; 6 expression types; 7 consumers' types; 8 holder's own; 9 holder's runs; bundles: published fun $2, val 1, class 3, done 3, template 0; released fun 0, val 0, class 0, done 0, template 0; entered fun $3, val 0, class 5, done 0, template 0; read direct $4, cache 0; sealed 40; by the lock inside a hold 2, for an export 0; waited while withheld 0"; }
  bundled() { python3 tests/support/fork_one.py bundles "$tmp/blog" "$1" > "$tmp/bout" 2>&1; echo "$?: $(grep -m1 '^FAIL' "$tmp/bout" | sed 's/^FAIL //')"; }
  printf '%s\n' "$(bj 30 4 2 7)" > "$tmp/blog"
  expect "a bundles' cell each above zero in its invocation" "$(bundled published.fun,entered.fun,read.direct)" "0: "
  printf '%s\n' "$(bj 30 4 0 7)" > "$tmp/blog"
  expect "a required cell at zero" "$(bundled published.fun,entered.fun)" "1: invocation 30: entered.fun at zero"
  printf '%s\n' "$(bj 30 4 0 7)" "$(bj 31 4 6 7)" > "$tmp/blog"
  expect "a cell another invocation covers" "$(bundled entered.fun)" "1: invocation 30: entered.fun at zero"
  printf '%s\n' "$(j 32 2 9)" > "$tmp/blog"
  expect "a join without the bundles' counts" "$(bundled published.fun)" "1: invocation 32: a join without the bundles' counts"
  printf '%s\n' "$(bj 30 4 2 7)" > "$tmp/blog"
  expect "a cell no counter names" "$(bundled entered.everything)" "1: no counter named entered.everything"
  : > "$tmp/blog"
  expect "a log with no join" "$(bundled published.fun)" "1: no invocation joined a fork"
  printf '%s\n' "$(bj 33 4 2 7 | sed 's/^view checks: join 33: [^;]*; fork 1;/view checks: gave way 33: fork 1,/')" > "$tmp/blog"
  expect "an attempt that gave way, its counts with no join" "$(bundled published.fun,entered.fun)" "1: invocation 33: gave way, no fork joined"
  printf '%s\n' "$checked" 'view checks: start 11 run bad.scala; threads automatic' 'view checks: end 11: not typed, 0 forks, 0 with the overlays' > "$tmp/log"
  enforced "$tmp/log"; expect "a checked invocation and one that typed no program" "$?: $enforcement" "0: 2 invocations at 1 workers: 1 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 1 typed no program, 0 at a test's own count, 0 unproven; 900 checks"
  expect "the crossings summed from the joins" "$crossings" "crossings: peer records 1 signatures, 2 classes, 3 type parameters, 4 aliases (5 translated); 6 expression types; 7 consumers' types; 8 holder's own; 9 holder's runs"
  printf '%s\n' "$checked" 'view checks: start 12 run b.scala; threads automatic' 'view checks: end 12: typed, 1 forks, 0 with the overlays' > "$tmp/log"
  enforced "$tmp/log"; expect "an invocation forked without the overlays" "$?: $enforcement" "1: 2 invocations at 1 workers: 1 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 1 unproven (12); 900 checks"
  printf '%s\n' "$checked" 'view checks: start 13 run c.scala; threads automatic' "$(w 13 0 1 Shared report)" "$(j 13 1 4 Shared report)" 'view checks: end 13: typed, 1 forks, 1 with the overlays' > "$tmp/log"
  enforced "$tmp/log"; expect "an invocation whose checks reported instead of panicking" "$?: $enforcement" "1: 2 invocations at 1 workers: 1 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 1 unproven (13); 900 checks"
  printf '%s\n' "$checked" 'view checks: start 14 run d.scala; threads automatic' > "$tmp/log"
  enforced "$tmp/log"; expect "an invocation with no join and no end" "$?: $enforcement" "1: 2 invocations at 1 workers: 1 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 1 unproven (14); 900 checks"
  printf '%s\n' 'view checks: start 15 run e.scala; threads automatic' "$(w 15 0 1 Frozen)" "$(j 15 1 5 Frozen)" > "$tmp/log"
  enforced "$tmp/log"; expect "a join in another namespace" "$?: $enforcement" "1: 1 invocations at 1 workers: 0 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 1 unproven (15); 0 checks"
  printf '%s\n' 'view checks: start 16 run f.scala; threads automatic' "$(w 16 0 1)" "$(j 16 1 7)" 'view checks: exit 16 134' > "$tmp/log"
  enforced "$tmp/log"; expect "a checked invocation that aborted after its join" "$?: $enforcement" "0: 1 invocations at 1 workers: 1 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 0 unproven; 7 checks"
  printf '%s\n' 'view checks: start 16 run f.scala; threads automatic' "$(w 16 0 1)" "$(j 16 1 7)" 'view checks: exit 16 0' > "$tmp/log"
  enforced "$tmp/log"; expect "an invocation with no end whose outcome is no signal" "$?: $enforcement" "1: 1 invocations at 1 workers: 0 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 1 unproven (16); 0 checks"
  printf '%s\n' 'view checks: start 16 run f.scala; threads automatic' "$(w 16 0 1)" "$(j 16 1 7)" > "$tmp/log"
  enforced "$tmp/log"; expect "an invocation with no end and no outcome" "$?: $enforcement" "1: 1 invocations at 1 workers: 0 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 1 unproven (16); 0 checks"
  requested=2
  printf '%s\n' 'view checks: start 20 run a.scala; threads 2 (TEQ_THREADS)' "$(w 20 0 1)" "$(j 20 1 900)" 'view checks: end 20: typed, 1 forks, 1 with the overlays' > "$tmp/log"
  enforced "$tmp/log"; expect "a two-worker run whose log has one worker of one checked" "$?: $enforcement" "1: 1 invocations at 2 workers: 0 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 1 unproven (20); 0 checks"
  printf '%s\n' 'view checks: start 21 run a.scala; threads 2 (TEQ_THREADS)' "$(w 21 0 2)" "$(w 21 1 2)" "$(j 21 2 900)" 'view checks: end 21: typed, 1 forks, 1 with the overlays' > "$tmp/log"
  enforced "$tmp/log"; expect "a two-worker run with both workers named and checked" "$?: $enforcement" "0: 1 invocations at 2 workers: 1 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 0 unproven; 900 checks"
  printf '%s\n' 'view checks: start 22 run a.scala; threads 2 (TEQ_THREADS)' "$(w 22 0 2)" "$(w 22 0 2)" "$(j 22 2 900)" 'view checks: end 22: typed, 1 forks, 1 with the overlays' > "$tmp/log"
  enforced "$tmp/log"; expect "a two-worker run whose second worker is never named" "$?: $enforcement" "1: 1 invocations at 2 workers: 0 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 1 unproven (22); 0 checks"
  printf '%s\n' 'view checks: start 24 run a.scala; threads 2 (TEQ_THREADS)' "$(w 24 0 2)" 'view checks: empty 24: worker 1 of 2 in fork 1' "$(j 24 2 900 Shared panic 1 | sed 's/1 of 2 workers/1 of 1 workers/')" 'view checks: end 24: typed, 1 forks, 1 with the overlays' > "$tmp/log"
  enforced "$tmp/log"; expect "a two-worker run whose second worker's slot was left empty" "$?: $enforcement" "0: 1 invocations at 2 workers: 1 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 0 unproven; 900 checks"
  printf '%s\n' 'view checks: start 25 run a.scala; threads 2 (TEQ_THREADS)' "$(w 25 0 2)" 'view checks: empty 25: worker 1 of 3 in fork 1' "$(j 25 2 900 Shared panic 1 | sed 's/1 of 2 workers/1 of 1 workers/')" 'view checks: end 25: typed, 1 forks, 1 with the overlays' > "$tmp/log"
  enforced "$tmp/log"; expect "an empty slot of another count" "$?: $enforcement" "1: 1 invocations at 2 workers: 0 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 1 unproven (25); 0 checks"
  printf '%s\n' 'view checks: start 26 run a.scala; threads 2 (TEQ_THREADS)' "$(w 26 0 2)" 'view checks: empty 26: worker 1 of 2 in fork 1' "$(j 26 2 900)" 'view checks: end 26: typed, 1 forks, 1 with the overlays' > "$tmp/log"
  enforced "$tmp/log"; expect "a join of two begun beside one worker named and one slot empty" "$?: $enforcement" "1: 1 invocations at 2 workers: 0 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 1 unproven (26); 0 checks"
  printf '%s\n' 'view checks: start 27 run a.scala; threads 2 (TEQ_THREADS)' "$(w 27 0 2)" "$(w 27 1 2)" 'view checks: empty 27: worker 0 of 2 in fork 1' "$(j 27 2 900)" 'view checks: end 27: typed, 1 forks, 1 with the overlays' > "$tmp/log"
  enforced "$tmp/log"; expect "a worker named both as begun and as an empty slot" "$?: $enforcement" "1: 1 invocations at 2 workers: 0 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 1 unproven (27); 0 checks"
  printf '%s\n' 'view checks: start 23 run a.scala; threads 2 (TEQ_THREADS)' "$(w 23 0 2)" "$(w 23 1 2)" "view checks: gave way 23: fork 1, crossings: peer records 1 signatures, 0 classes, 0 type parameters, 0 aliases, 1 translated; 0 expression types; 0 consumers' types; 0 holder's own; 0 holder's runs; the macro m changed a map" 'view checks: start 23 run a.scala (typed again by one worker); threads 2 (TEQ_THREADS)' 'view checks: end 23: typed, 0 forks, 0 with the overlays' > "$tmp/log"
  enforced "$tmp/log"; expect "an attempt that gave way, typed again by one worker" "$?: $enforcement" "0: 1 invocations at 2 workers: 0 checked, 1 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 0 unproven; 0 checks"
  printf '%s\n' 'view checks: start 27 run a.scala; threads 2 (TEQ_THREADS)' "$(w 27 0 2)" "$(w 27 1 2)" "view checks: gave way 27: fork 1, crossings: peer records 0 signatures, 0 classes, 0 type parameters, 0 aliases, 0 translated; 0 expression types; 0 consumers' types; 0 holder's own; 0 holder's runs; a map changed" > "$tmp/log"
  enforced "$tmp/log"; expect "an attempt that gave way with no build typed again after it" "$?: $enforcement" "1: 1 invocations at 2 workers: 0 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 1 unproven (27); 0 checks"
  printf '%s\n' 'view checks: start 28 run a.scala; threads 2 (TEQ_THREADS)' "$(w 28 0 2)" "$(w 28 1 2)" "$(j 28 2 900)" 'view checks: end 28: typed, 2 forks, 2 with the overlays' > "$tmp/log"
  enforced "$tmp/log"; expect "an end reporting two forks of which one joined" "$?: $enforcement" "1: 1 invocations at 2 workers: 0 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 1 unproven (28); 0 checks"
  printf '%s\n' 'view checks: start 24 run a.scala --threads; threads 16 (--threads)' "$(for k in $(seq 0 15); do w 24 $k 16; done)" "$(j 24 16 50)" 'view checks: end 24: typed, 1 forks, 1 with the overlays' "$(printf '%s\n' 'view checks: start 25 run b.scala; threads 2 (TEQ_THREADS)' "$(w 25 0 2)" "$(w 25 1 2)" "$(j 25 2 60)" 'view checks: end 25: typed, 1 forks, 1 with the overlays')" > "$tmp/log"
  enforced "$tmp/log"; expect "a test that gives its own worker count beside one at the count asked for" "$?: $enforcement" "0: 2 invocations at 2 workers: 2 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 1 at a test's own count, 0 unproven; 110 checks"
  printf '%s\n' 'view checks: start 29 check --products; threads 1 (--threads)' 'view checks: end 29: typed, 0 forks, 0 with the overlays' "$(printf '%s\n' 'view checks: start 25 run b.scala; threads 2 (TEQ_THREADS)' "$(w 25 0 2)" "$(w 25 1 2)" "$(j 25 2 60)" 'view checks: end 25: typed, 1 forks, 1 with the overlays')" > "$tmp/log"
  enforced "$tmp/log"; expect "a test's own one worker above one, unforked beside one checked" "$?: $enforcement" "0: 2 invocations at 2 workers: 1 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 1 at a test's own count (1 of them its own one worker's, which forks nothing), 0 unproven; 60 checks"
  printf '%s\n' 'view checks: start 31 build r.scala; threads 1 (--threads)' 'view checks: exit 31 134' "$(printf '%s\n' 'view checks: start 25 run b.scala; threads 2 (TEQ_THREADS)' "$(w 25 0 2)" "$(w 25 1 2)" "$(j 25 2 60)" 'view checks: end 25: typed, 1 forks, 1 with the overlays')" > "$tmp/log"
  enforced "$tmp/log"; expect "a test's own one worker above one, aborted unforked" "$?: $enforcement" "0: 2 invocations at 2 workers: 1 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 1 at a test's own count (1 of them its own one worker's, which forks nothing), 0 unproven; 60 checks"
  printf '%s\n' 'view checks: start 26 build a.scala; threads 2 (TEQ_THREADS)' "$(w 26 0 2)" 'view checks: exit 26 134' > "$tmp/log"
  enforced "$tmp/log"; expect "an abort before the second worker began, the first checked" "$?: $enforcement" "1: 1 invocations at 2 workers: 0 checked, 0 checked and given way, 0 checked with nothing read, 1 ended before their join, 0 typed no program, 0 at a test's own count, 0 unproven; 0 checks"
  requested=1
  expect "a panic's report in a diff, after an empty line or not, its thread's number" "$(printf '%s\n' '> ' "> thread 'w' (123) panicked at x" | normalise)" "> thread 'w' (N) panicked at x"
  expect "a plain panic's report after an empty line or not" "$(printf '%s\n' '' 'a thread panicked at x' | normalise)" "a thread panicked at x"
  expect "a tool's report of a missing file in a temporary directory" "$(echo 'diff: /tmp/tmp.5f0Rb4gRSy/walk: No such file or directory' | normalise)" "diff: /tmp/tmp.N/walk: No such file or directory"
  expect "a program's output naming a temporary directory, byte for byte" "$(printf '%s\n' '/tmp/tmp.ABCDEF/result' '> /tmp/tmp.UVWXYZ/result' | normalise)" "$(printf '%s\n' '/tmp/tmp.ABCDEF/result' '> /tmp/tmp.UVWXYZ/result')"
  expect "a program's output of timings and ids, byte for byte" "$(printf '%s\n' '1.0 ms' '(id 17)' | normalise)" "$(printf '%s\n' '1.0 ms' '(id 17)')"
  printf '%s\n' "$checked" 'view checks: start 18 run h.scala; threads automatic' "$(w 18 0 1 Shared panic 'outside the checked context')" "$(j 18 1 0 Shared panic 0)" 'view checks: end 18: typed, 1 forks, 1 with the overlays' > "$tmp/log"
  enforced "$tmp/log"; expect "an invocation whose worker was outside the checked context" "$?: $enforcement" "1: 2 invocations at 1 workers: 1 checked, 0 checked and given way, 0 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 1 unproven (18); 900 checks"
  printf '%s\n' "$checked" 'view checks: start 17 run g.scala; threads automatic' "$(w 17 0 1)" "$(j 17 1 0)" 'view checks: end 17: typed, 1 forks, 1 with the overlays' > "$tmp/log"
  enforced "$tmp/log"; expect "an invocation whose worker, in the checked context, read nothing" "$?: $enforcement" "0: 2 invocations at 1 workers: 1 checked, 0 checked and given way, 1 checked with nothing read, 0 ended before their join, 0 typed no program, 0 at a test's own count, 0 unproven; 900 checks"
  printf '%s\n' "$checked" 'view checks: start 19 build i.scala; threads automatic' "$(w 19 0 1)" 'view checks: exit 19 134' > "$tmp/log"
  enforced "$tmp/log"; expect "an invocation that ended before its join, its worker checked" "$?: $enforcement" "0: 2 invocations at 1 workers: 1 checked, 0 checked and given way, 0 checked with nothing read, 1 ended before their join, 0 typed no program, 0 at a test's own count, 0 unproven; 900 checks"
  # The comparison against three baseline runs: a failure one of them has is no new one, whatever
  # the others did; one no baseline run has is.
  printf '%s\n' 'a' 'flaky test' > "$tmp/c.base.1.fail"; printf '%s\n' 'a 1' 'flaky test 7' > "$tmp/c.base.1.ids"
  printf '%s\n' 'a' > "$tmp/c.base.2.fail"; printf '%s\n' 'a 1' > "$tmp/c.base.2.ids"
  printf '%s\n' 'a' 'flaky test' > "$tmp/c.base.3.fail"; printf '%s\n' 'a 1' 'flaky test 8' > "$tmp/c.base.3.ids"
  printf '%s\n' 'flaky test' 'fresh' > "$tmp/c.flag.fail"; printf '%s\n' 'flaky test 8' 'fresh 3' > "$tmp/c.flag.ids"
  compare "$tmp/c" 3
  expect "a failure of a baseline run, the others passing or failing otherwise" "new [$new] changed [$changed] gone [$gone] unstable [$unstable]" "new [fresh] changed [] gone [a] unstable [flaky test]"
  printf '%s\n' 'flaky test 9' 'fresh 3' > "$tmp/c.flag.ids"
  compare "$tmp/c" 3
  expect "a failure otherwise than every baseline run's" "new [$new] changed [$changed]" "new [fresh] changed [flaky test]"
  # A test's files are its bytes: two outputs that differ only in a temporary directory in what
  # looks like a tool's report are two failures; the same text in the suite's log is one.
  mkdir -p "$tmp/a" "$tmp/b"
  printf '%s\n' 'FAIL sample' '1c1' > "$tmp/log"
  echo sample > "$tmp/names"
  echo 'diff: /tmp/tmp.ABCDEF/result: No such file or directory' > "$tmp/a/sample.actual"
  echo 'diff: /tmp/tmp.UVWXYZ/result: No such file or directory' > "$tmp/b/sample.actual"
  a=$(identities "$tmp/log" "$tmp/a" "$tmp/names")
  b=$(identities "$tmp/log" "$tmp/b" "$tmp/names")
  expect "a test's output, byte for byte in its digest" "$([ "$a" = "$b" ] && echo same || echo differs)" differs
  cp "$tmp/a/sample.actual" "$tmp/b/sample.actual"
  printf '%s\n' 'FAIL sample' 'diff: /tmp/tmp.ABCDEF/walk: No such file or directory' > "$tmp/log.a"
  printf '%s\n' 'FAIL sample' 'diff: /tmp/tmp.UVWXYZ/walk: No such file or directory' > "$tmp/log.b"
  a=$(identities "$tmp/log.a" "$tmp/a" "$tmp/names")
  b=$(identities "$tmp/log.b" "$tmp/b" "$tmp/names")
  expect "a tool's report in the suite's log, normalised in its digest" "$([ "$a" = "$b" ] && echo same || echo differs)" same
  # The shell's report of a killed command falls in the test's block or the one before.
  printf '%s\n' first sample > "$tmp/names2"
  printf '%s\n' 'FAIL first' '1c1' "./tests/x.sh: line 9: 123 Aborted                 timeout 20 teq run" 'FAIL sample' '1c1' > "$tmp/log.c"
  printf '%s\n' 'FAIL first' '1c1' 'FAIL sample' '1c1' "./tests/x.sh: line 9: 456 Aborted                 timeout 20 teq run" > "$tmp/log.d"
  a=$(identities "$tmp/log.c" "" "$tmp/names2")
  b=$(identities "$tmp/log.d" "" "$tmp/names2")
  expect "the shell's report of a killed command, in either block, left out of the digests" "$([ "$a" = "$b" ] && echo same || echo differs)" same
  # A failure's digest holds every run of the compiler that names the test: its arguments, its exit
  # or signal and all it wrote, the schedule's noise left out.
  printf '%s\n' 'FAIL sample (js run)' '1,25c1,2' '< expected' > "$tmp/log.e"
  echo sample > "$tmp/names3"
  kept() {
    mkdir -p "$tmp/$1/r.$2"; printf '%s\n' run "$3" > "$tmp/$1/r.$2/args"; echo "$4" > "$tmp/$1/r.$2/status"; printf '%s\n' "$5" > "$tmp/$1/r.$2/out"
    [ -n "${6:-}" ] && printf '%s\n' "$6" > "$tmp/$1/r.$2/err"
    [ -n "${7:-}" ] && : > "$tmp/$1/r.$2/merged"
    return 0
  }
  kept aborted 1 tests/x/sample.scala 134 'a thread panicked at x'
  kept segfault 1 tests/x/sample.scala 139 'a thread panicked at x'
  kept elsewhere 1 tests/x/sample.scala 134 'a thread panicked at x'
  kept elsewhere 2 tests/x/other.scala 0 'fine'
  kept tmpa 1 /tmp/tmp.ABCDEF/sample.scala 134 'at /tmp/tmp.ABCDEF/sample.scala'
  kept tmpb 1 /tmp/tmp.UVWXYZ/sample.scala 134 'at /tmp/tmp.UVWXYZ/sample.scala'
  a=$(identities "$tmp/log.e" "" "$tmp/names3" "$tmp/aborted")
  b=$(identities "$tmp/log.e" "" "$tmp/names3" "$tmp/segfault")
  expect "an abort and a segmentation fault behind one excerpt, two failures" "$([ "$a" = "$b" ] && echo same || echo differs)" differs
  b=$(identities "$tmp/log.e" "" "$tmp/names3" "$tmp/elsewhere")
  expect "another test's run left out of a failure's digest" "$([ "$a" = "$b" ] && echo same || echo differs)" same
  a=$(identities "$tmp/log.e" "" "$tmp/names3" "$tmp/tmpa")
  b=$(identities "$tmp/log.e" "" "$tmp/names3" "$tmp/tmpb")
  expect "a run's temporary directory left out of its digest" "$([ "$a" = "$b" ] && echo same || echo differs)" same
  # The timing report's figures are left out where the compiler wrote it, on the standard error or
  # in the one stream a run wrote both to, by its `phases` section; a program's output of that
  # shape is digested as written.
  kept outa 1 tests/x/sample.scala 0 "$(printf 'workers\n  result 12.34 ms')"
  kept outb 1 tests/x/sample.scala 0 "$(printf 'workers\n  result 56.78 ms')"
  a=$(identities "$tmp/log.e" "" "$tmp/names3" "$tmp/outa")
  b=$(identities "$tmp/log.e" "" "$tmp/names3" "$tmp/outb")
  expect "a program's output shaped like the timing report, digested as written" "$([ "$a" = "$b" ] && echo same || echo differs)" differs
  kept mergedouta 1 tests/x/sample.scala 0 "$(printf 'workers\n  result 12.34 ms')" "" merged
  kept mergedoutb 1 tests/x/sample.scala 0 "$(printf 'workers\n  result 56.78 ms')" "" merged
  a=$(identities "$tmp/log.e" "" "$tmp/names3" "$tmp/mergedouta")
  b=$(identities "$tmp/log.e" "" "$tmp/names3" "$tmp/mergedoutb")
  expect "the same in one stream with the compiler's, digested as written" "$([ "$a" = "$b" ] && echo same || echo differs)" differs
  kept erra 1 tests/x/sample.scala 0 out "$(printf 'workers\n  phase: merge  1.00 ms\nphases\n  lines  3\n  read  1.00 ms')"
  kept errb 1 tests/x/sample.scala 0 out "$(printf 'workers\n  phase: merge  2.50 ms\nphases\n  lines  3\n  read  1.75 ms')"
  a=$(identities "$tmp/log.e" "" "$tmp/names3" "$tmp/erra")
  b=$(identities "$tmp/log.e" "" "$tmp/names3" "$tmp/errb")
  expect "the timing report on the standard error, its figures left out" "$([ "$a" = "$b" ] && echo same || echo differs)" same
  kept mergeda 1 tests/x/sample.scala 0 "$(printf 'out\nphases\n  lines  3\n  read  1.00 ms')" "" merged
  kept mergedb 1 tests/x/sample.scala 0 "$(printf 'out\nphases\n  lines  3\n  read  1.75 ms')" "" merged
  a=$(identities "$tmp/log.e" "" "$tmp/names3" "$tmp/mergeda")
  b=$(identities "$tmp/log.e" "" "$tmp/names3" "$tmp/mergedb")
  expect "the timing report in one stream with the output, its figures left out" "$([ "$a" = "$b" ] && echo same || echo differs)" same
  kept forkeda 1 tests/x/sample.scala 1 "$(printf 'error\nworkers\n  phase: signatures  0.38 ms\nclasspath\n  jars opened  2  0.05 ms')" "" merged
  kept forkedb 1 tests/x/sample.scala 1 "$(printf 'error\nworkers\n  phase: signatures  0.41 ms\nclasspath\n  jars opened  2  0.07 ms')" "" merged
  a=$(identities "$tmp/log.e" "" "$tmp/names3" "$tmp/forkeda")
  b=$(identities "$tmp/log.e" "" "$tmp/names3" "$tmp/forkedb")
  expect "a forked build's report without --time, its figures left out" "$([ "$a" = "$b" ] && echo same || echo differs)" same
  # outputs <candidate> <reference>: what the every-invocation comparison prints, one line.
  outputs() { python3 tests/support/fork_one.py outputs "$tmp/skip" "$tmp/$1" "$tmp/$2" | tr '\n' ' '; }
  : > "$tmp/skip"
  kept plaina 1 tests/x/sample.scala 1 "$(printf 'error\nclasspath\n  jars opened  2  0.05 ms\nlibrary bodies\n  members missing  3')" "" merged
  kept plaina 2 tests/x/other.scala 0 "decoded in 13.8ms (127338 bodies per second)"
  kept forkeda2 1 tests/x/sample.scala 1 "$(printf 'error\nworkers\n  phase: signatures   0.38 ms\nclasspath\n  jars opened           2  0.07 ms\nlibrary bodies\n  members missing        3')" "" merged
  kept forkeda2 2 tests/x/other.scala 0 "decoded in 9.1ms (90210 bodies per second)"
  expect "a forked build's output against one worker's, the reports and durations left out" "$(outputs forkeda2 plaina)" ""
  kept warned 1 tests/x/sample.scala 1 "$(printf 'error\nworkers\n  phase: signatures   0.38 ms')" "" merged
  kept warned 2 tests/x/other.scala 0 "$(printf 'decoded in 9.1ms (90210 bodies per second)\nwarning: match may not be exhaustive')"
  expect "a passing run's output otherwise than one worker's" "$(outputs warned plaina)" "run tests/x/other.scala "
  echo tests/x/other.scala > "$tmp/skip"
  expect "a test the runner compares otherwise, left out" "$(outputs warned plaina)" ""
  : > "$tmp/skip"
  kept fewer 1 tests/x/sample.scala 1 "error" "" merged
  expect "a run one worker made and the fork did not" "$(outputs fewer plaina)" "run tests/x/other.scala "
  kept otherwise 1 tests/x/sample.scala 1 "$(printf 'error\nanother error')" "" merged
  kept otherwise 2 tests/x/other.scala 0 "decoded in 9.1ms (90210 bodies per second)"
  expect "a run failing in both, its text otherwise" "$(outputs otherwise plaina)" "run tests/x/sample.scala "
  kept programa 1 tests/x/sample.scala 0 "$(printf 'workers\n  answer=1\nelapsed limit = 10ms')"
  kept programb 1 tests/x/sample.scala 0 "$(printf 'workers\n  answer=2\nelapsed limit = 10ms')"
  kept programc 1 tests/x/sample.scala 0 "$(printf 'workers\n  answer=1\nelapsed limit = 20ms')"
  expect "a program's own lines of a report's shape, kept" "$(outputs programb programa)" "run tests/x/sample.scala "
  expect "a program's own durations, kept" "$(outputs programc programa)" "run tests/x/sample.scala "
  kept passing 1 tests/x/sample.scala 0 "" "" merged
  kept passing 2 tests/x/other.scala 0 "decoded in 9.1ms (90210 bodies per second)"
  expect "a run one worker failed and the fork passed" "$(outputs passing plaina)" "run tests/x/sample.scala "
  # The compiler run through the record keeps the suite's streams, their order and its outcome.
  if [ -d /proc/$$/fd ]; then
    printf '%s\n' '#!/bin/bash' 'echo out1; echo err1 >&2; echo out2; [ "$1" = segv ] && kill -SEGV $$; exit 3' > "$tmp/fake"
    chmod +x "$tmp/fake"
    mkdir -p "$tmp/rec"
    FORK_ONE_TEQ=$tmp/fake FORK_ONE_RECORDS=$tmp/rec tests/support/fork-one-teq.sh a b > "$tmp/both" 2>&1
    code=$?
    r=$(ls -d "$tmp"/rec/r.* | head -1)
    expect "a run into one file: its exit, its streams in order, the record" "$code|$(tr '\n' ' ' < "$tmp/both")|$(cat "$r/status")|$(tr '\n' ' ' < "$r/out")|$(tr '\n' ' ' < "$r/args")" "3|out1 err1 out2 |3|out1 err1 out2 |a b "
    rm -rf "$tmp/rec"; mkdir -p "$tmp/rec"
    got=$(FORK_ONE_TEQ=$tmp/fake FORK_ONE_RECORDS=$tmp/rec tests/support/fork-one-teq.sh x 2>&1)
    r=$(ls -d "$tmp"/rec/r.* | head -1)
    expect "a run into one pipe: its streams in order, the record" "$(echo $got)|$(tr '\n' ' ' < "$r/out")" "out1 err1 out2|out1 err1 out2 "
    rm -rf "$tmp/rec"; mkdir -p "$tmp/rec"
    code=$( (FORK_ONE_TEQ=$tmp/fake FORK_ONE_RECORDS=$tmp/rec tests/support/fork-one-teq.sh segv > "$tmp/sig" 2>&1); echo $?) 2> /dev/null
    r=$(ls -d "$tmp"/rec/r.* | head -1)
    expect "a run its signal ended: the same signal to the suite, its streams alone, the record's" "$code|$(tr '\n' ' ' < "$tmp/sig")|$(cat "$r/status")|$(tr '\n' ' ' < "$r/out")" "139|out1 err1 out2 |139|out1 err1 out2 "
  else
    echo "skip  the compiler's runs through the record (no /proc here)"
  fi
  rm -rf "$tmp"
  [ $fails = 0 ]
  exit $?
fi
[ -x "$TEQ" ] || { echo "fork-one: no binary at $TEQ" >&2; exit 2; }
teq_abs=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
# recorded <dir>: the environment of a suite's run whose every run of the compiler is kept in <dir>
# (tests/support/fork-one-teq.sh), for the failures' digests and the invocations' outcomes.
recorded() {
  echo "FORK_ONE_TEQ=$teq_abs FORK_ONE_RECORDS=$PWD/$1 TEQ=$PWD/tests/support/fork-one-teq.sh"
}
case ${TEQ_VIEW_CHECKS:-panic} in
  panic) ;;
  *) echo "fork-one: TEQ_VIEW_CHECKS=$TEQ_VIEW_CHECKS lets a violation pass; the runner sets the mode itself (panic)" >&2; exit 2 ;;
esac
export TEQ_VIEW_CHECKS=panic
export TEQ_PANIC_REPORT=plain
unset TEQ_VIEW_CHECKS_LOG
version=$("$TEQ" --version)
checks=0
case $version in *assertions*) checks=1 ;; esac
# The namespace the checks' log names, the shared base's (src/types/view.rs).
ns_name=Shared
command_of() {
  case $1 in
    cases) echo ./tests/run.sh ;;
    errors) echo ./tests/run_errors.sh ;;
    interop) echo ./tests/run_interop.sh ;;
    dce) echo ./tests/dce.sh ;;
    fold) echo ./tests/fold.sh ;;
    interp) echo ./tests/run_interp.sh ;;
    jvm) echo ./tests/run_jvm.sh ;;
    stdlib) echo ./tests/run_stdlib.sh ;;
    classpath) echo ./tests/classpath.sh ;;
    split) echo ./tests/split.sh ;;
    app) echo ./tests/app.sh ;;
    modules) echo ./tests/modules.sh ;;
    tasty) echo ./tests/tasty.sh ;;
    scala3-run) echo "tests/scala3/run.sh" ;;
    scala3-interp) echo "tests/scala3/run.sh --target interp" ;;
    scala3-macros) echo "tests/scala3/run.sh --suite run-macros" ;;
    scala3-typing) echo "python3 tests/scala3-typing/harness.py" ;;
    *) return 1 ;;
  esac
}
for s in $suites; do
  command_of "$s" > /dev/null || { echo "fork-one: no suite named $s" >&2; exit 2; }
done
work=out/fork-one
mkdir -p "$work"
revision=$(git rev-parse --short HEAD 2> /dev/null || echo "${TEQ_REVISION:-unknown}")
requested=$threads
echo "fork-one: $version (the tree at $revision), $([ $checks = 1 ] && echo 'the assertion-enabled build' || echo 'no assertions'); at $threads workers, the give-ways $giveways$([ "$known_list" = /dev/null ] && echo ', no known list'); switches: $(at $threads | tr '\n' ' ')$(overlays_env) $(env | grep -E '^TEQ_' | grep -vE '^TEQ=|^TEQ_TYPE_OVERLAYS=|^TEQ_REVISION=' | sort | tr '\n' ' ')"
if [ $checks = 1 ]; then
  case $version in
    *" $revision assertions"|*"$revision"*) ;;
    *) echo "fork-one: the binary ($version) is not the tree's revision ($revision)" >&2; exit 2 ;;
  esac
fi
# failures <log>: the tests a suite's log names as failing, one per line, sorted.
failures() {
  { grep -E '^(FAIL|REF FAIL) ' "$1" | sed -E 's/^(REF )?FAIL ([^ :]*).*/\2/'
    grep -E '^REGRESSIONS:' "$1" | sed 's/^REGRESSIONS://' | tr ' ' '\n'
  } | grep -v '^$' | sort -u
}
# out_dir_of <suite>: where the suite keeps its files per test, a failure's own record beside the
# log's.
out_dir_of() {
  case $1 in
    cases) echo out/tests ;;
    errors) echo out/errors-tests ;;
    interp) echo out/interp-tests ;;
    jvm) echo out/jvm-tests ;;
    stdlib) echo out/stdlib-tests ;;
  esac
}
# incomplete <exit>: whether a run ended by its bound (`timeout`'s 124) or by a signal (128 and
# above) rather than by itself.
incomplete() {
  [ "$1" = 124 ] || [ "$1" -ge 128 ]
}
# completed <log>: whether a suite printed its own last line, its count of the tests it ran.
completed() {
  tail -n 5 "$1" | grep -qE '[0-9]+ passed|^error tests (passed|failed)$|still pass|no category changed'
}
# check_completed <exit> <log>: whether a check ran to its end, exiting 0, or 1 with its count of
# errors as its last line (as bench/app/api-check.sh has it).
check_completed() {
  [ "$1" = 0 ] || { [ "$1" = 1 ] && tail -1 "$2" | grep -qE '^[0-9]+ errors? found$'; }
}
status=0
rows=()
# row <text>: a line of the table, printed as it is made.
row() {
  rows+=("$1")
  echo "$1"
}
for s in $suites; do
  cmd=$(command_of "$s")
  start=$(date +%s)
  dir=$(out_dir_of "$s")
  # The reference: one worker's run without the fork (`--reference one`), which the parallel path's
  # output has to be; or the overlays off at the count asked for (`--reference off`), once at one
  # worker and $base_runs times above (the parallel path's own schedule moves some diagnostics from
  # run to run, the overlays off as on), a failure new only where no such run failed so.
  base_code=0 base_unfinished=
  rm -rf "$work/$s".base.* "$work/$s".flag.* "$work/$s".*.checks
  for r in $(seq 1 $base_runs); do
    mkdir -p "$work/$s.base.$r.rec"
    env $(reference_env) $(recorded "$work/$s.base.$r.rec") timeout 1800 $cmd > "$work/$s.base.$r.log" 2>&1
    code=$?
    [ $code != 0 ] && base_code=$code
    { incomplete $code || ! completed "$work/$s.base.$r.log"; } && base_unfinished="the reference's run $r did not complete (exit $code, its last line: $(tail -1 "$work/$s.base.$r.log" | cut -c1-80))"
    [ -n "$dir" ] && [ -d "$dir" ] && cp -R "$dir" "$work/$s.base.$r.d"
    failures "$work/$s.base.$r.log" > "$work/$s.base.$r.fail"
    identities "$work/$s.base.$r.log" "${dir:+$work/$s.base.$r.d}" "$work/$s.base.$r.fail" "$work/$s.base.$r.rec" | sort > "$work/$s.base.$r.ids"
  done
  # The overlays' runs, `--repeat` times, each against the reference.
  flag_code=0 all_new= all_changed= all_outputs= unenforced= unfinished=$base_unfinished enforcement= crossings=
  for k in $(seq 1 $repeat); do
    mkdir -p "$work/$s.flag.$k.rec"
    env $(at $threads) $(overlays_env) TEQ_VIEW_CHECKS_LOG="$PWD/$work/$s.$k.checks" $(recorded "$work/$s.flag.$k.rec") timeout 1800 $cmd > "$work/$s.flag.$k.log" 2>&1
    code=$?
    [ $code != 0 ] && flag_code=$code
    enforced "$work/$s.$k.checks" || unenforced="the assertion-enabled build's checks are not proven for every invocation of run $k in the $ns_name namespace: $enforcement"
    failures "$work/$s.flag.$k.log" > "$work/$s.flag.$k.fail"
    identities "$work/$s.flag.$k.log" "$dir" "$work/$s.flag.$k.fail" "$work/$s.flag.$k.rec" | sort > "$work/$s.flag.$k.ids"
    compare "$work/$s" $base_runs "flag.$k"
    # Against one worker, every invocation's output: a test the suite passes in both runs can
    # still have printed otherwise (a warning more or less); the failures and the known ones are
    # compared above.
    if [ $reference = one ]; then
      { cat "$work/$s".base.*.fail "$work/$s.flag.$k.fail"; awk -v s="$s" -v g="$giveways" '$1 == s && $3 == g { print $2 }' "$known_list"; } > "$work/$s.flag.$k.skip"
      outputs=$(python3 tests/support/fork_one.py outputs "$work/$s.flag.$k.skip" "$work/$s.flag.$k.rec" "$work/$s".base.*.rec)
      all_outputs=$(printf '%s\n%s' "$all_outputs" "$outputs" | grep . | sort -u)
    fi
    all_new=$(printf '%s\n%s' "$all_new" "$new" | grep . | sort -u)
    all_changed=$(printf '%s\n%s' "$all_changed" "$changed" | grep . | sort -u)
    { incomplete $code || ! completed "$work/$s.flag.$k.log"; } && unfinished="the overlays' run $k did not complete (exit $code, its last line: $(tail -1 "$work/$s.flag.$k.log" | cut -c1-80))"
  done
  new=$all_new changed=$all_changed
  # A new failure, or one failing otherwise, that the known list names for this suite and setting
  # is counted apart (`known`).
  known=
  if [ -n "$new$changed" ]; then
    known=$(printf '%s\n%s\n' "$new" "$changed" | grep . | sort -u | while IFS= read -r t; do awk -v s="$s" -v t="$t" -v g="$giveways" '$1 == s && $2 == t && $3 == g { print t }' "$known_list"; done)
    if [ -n "$known" ]; then
      new=$(printf '%s\n' "$new" | grep -vxF "$known")
      changed=$(printf '%s\n' "$changed" | grep -vxF "$known")
    fi
  fi
  n_new=$(printf '%s' "$new" | grep -c .)
  n_changed=$(printf '%s' "$changed" | grep -c .)
  n_outputs=$(printf '%s' "$all_outputs" | grep -c .)
  n_base=$(grep -c . "$work/$s.base.1.fail")
  n_unstable=$(printf '%s' "$unstable" | grep -c .)
  if [ -n "$unfinished" ] || [ -n "$unenforced" ] || [ "$n_new" -gt 0 ] || [ "$n_changed" -gt 0 ] || [ "$n_outputs" -gt 0 ] || { [ $flag_code != 0 ] && [ $base_code = 0 ] && [ -z "$known" ]; }; then
    status=1
    verdict=FAIL
  else
    verdict=ok
  fi
  row "$(printf '%-5s %-14s %4ss  %s exit %s, %s failing%s; overlays exit %s%s, %s new%s, %s failing otherwise%s%s%s%s' "$verdict" "$s" $(($(date +%s) - start)) "$ref_name" $base_code "$n_base" "$([ $base_runs -gt 1 ] && echo " ($base_runs runs, $n_unstable failing otherwise from run to run)")" $flag_code "$([ $repeat -gt 1 ] && echo " ($repeat runs)")" "$n_new" "$([ -n "$known" ] && echo " ($(printf '%s\n' "$known" | grep -c .) known)")" "$n_changed" "$([ $reference = one ] && echo ", $n_outputs outputs otherwise")" "${gone:+, $(printf '%s' "$gone" | grep -c .) passing now}" "${enforcement:+; $enforcement}" "${crossings:+; $crossings}")"
  [ -n "$unfinished" ] && echo "        $unfinished"
  [ -n "$unenforced" ] && echo "        $unenforced"
  [ "$n_new" -gt 0 ] && printf '%s\n' "$new" | head -20 | sed 's/^/        new: /'
  [ -n "$known" ] && printf '%s\n' "$known" | sed 's/^/        known (tests\/support\/fork-one-known.txt): /'
  [ "$n_changed" -gt 0 ] && printf '%s\n' "$changed" | head -20 | sed 's/^/        failing otherwise: /'
  [ "$n_outputs" -gt 0 ] && printf '%s\n' "$all_outputs" | head -20 | sed 's/^/        output otherwise: /'
  [ "$n_unstable" -gt 0 ] && printf '%s\n' "$unstable" | head -10 | sed 's/^/        the baseline'"'"'s own from run to run: /'
  [ $flag_code != 0 ] && [ $base_code = 0 ] && [ "$n_new" = 0 ] && [ -z "$known" ] && echo "        the overlays' run exits $flag_code where the $ref_name's exits 0: $(tail -1 "$work/$s.flag.$repeat.log")"
done

# The application, as bench/app/export-identity.sh and api-check.sh read it (the checkout read,
# never written; its API's module list and class path the files APP_MODULES and APP_CLASSPATH name, its flags APP_FLAGS).
app_root=${APP_ROOT:-}
api() {
  local cp sources
  [ -d "$app_root" ] && [ -f "${APP_MODULES:-}" ] && [ -f "${APP_CLASSPATH:-}" ] && [ -n "${APP_FLAGS+set}" ] || { echo "set APP_ROOT, APP_MODULES, APP_CLASSPATH and APP_FLAGS"; return 99; }
  cp=$(sed "s|^~/|$HOME/|" "$APP_CLASSPATH" | paste -sd: -)
  sources=$(grep -v '^#' "$APP_MODULES" | tr '\n' ' ')
  (cd "$app_root" && timeout 300 "$teq_abs" compiler check $sources --classpath "$cp" $APP_FLAGS \
    --std scala-library --target jvm "$@")
}
# frontend <teq argument>...: the export's check, its sources, class path and flags as
# bench/app/export-build.mjs builds them, in the root the build file names.
frontend() {
  local root args=()
  root=$(node -e 'const d = require(process.argv[1]); console.log(require("path").resolve(process.argv[1], "..", d.root ?? "../../.."))' "$app")
  while IFS= read -r -d '' a; do args+=("$a"); done < <(node -e '
    const d = require(process.argv[1])
    const out = [...d.sources, "--classpath", d.classpath.join(":"), ...(d.maxInlines ? ["--max-inlines", String(d.maxInlines)] : []),
      ...(d.strictEquality ? ["--strict-equality"] : []), ...(d.kindProjector ? ["--kind-projector"] : []), ...(d.werror ? ["--werror"] : []),
      ...(d.cacheableState ?? []).flatMap((n) => ["--cacheable-state", n])]
    process.stdout.write(out.map((a) => a + "\0").join(""))' "$app")
  (cd "$root" && timeout 300 "$teq_abs" compiler check "${args[@]}" "$@")
}
if [ -n "$app" ]; then
  rm -rf "$work/export.base" "$work/export.flag"
  env $(reference_env) node bench/app/export-build.mjs "$app" "$teq_abs" "$work/export.base" > "$work/export.base.log" 2>&1
  base_code=$?
  rm -f "$work/export.checks"
  env $(at $threads) $(overlays_env) TEQ_VIEW_CHECKS_LOG="$PWD/$work/export.checks" node bench/app/export-build.mjs "$app" "$teq_abs" "$work/export.flag" > "$work/export.flag.log" 2>&1
  flag_code=$?
  n=$(find "$work/export.base" -type f 2> /dev/null | wc -l | tr -d ' ')
  enforcement=
  crossings=
  if ! enforced "$work/export.checks"; then
    status=1
    row "FAIL  app-export            the assertion-enabled build's checks are not proven for every invocation: $enforcement"
  elif [ $base_code = 0 ] && [ $flag_code = 0 ] && diff -rq "$work/export.base" "$work/export.flag" > "$work/export.diff" 2>&1; then
    row "ok    app-export            the overlays' export identical to the baseline's, $n files${enforcement:+; $enforcement}${crossings:+; $crossings}"
  else
    status=1
    if [ $base_code = 0 ] && [ $flag_code = 0 ]; then
      row "FAIL  app-export            $(wc -l < "$work/export.diff" | tr -d ' ') of $n files differ (the list in $work/export.diff)"
    elif grep -q '; exit null;' "$work/export.base.log" "$work/export.flag.log"; then
      row "FAIL  app-export            a build did not complete: $(grep -h '; exit null;' "$work/export.base.log" "$work/export.flag.log" | head -1 | cut -c1-120)"
    else
      row "FAIL  app-export            baseline exit $base_code, overlays exit $flag_code: $(tail -1 "$work/export.flag.log" | cut -c1-120)"
    fi
  fi
  (eval "$(reference_shell)"; api > "$work/api.base.log" 2>&1)
  base_code=$?
  rm -f "$work/api.checks"
  (export $(at $threads) $(overlays_env) TEQ_VIEW_CHECKS_LOG="$PWD/$work/api.checks"; api > "$work/api.flag.log" 2>&1)
  flag_code=$?
  enforcement=
  crossings=
  if ! check_completed $base_code "$work/api.base.log" || ! check_completed $flag_code "$work/api.flag.log"; then
    status=1
    row "FAIL  app-api               a check did not complete: the baseline's exit $base_code, the overlays' $flag_code"
  elif ! enforced "$work/api.checks"; then
    status=1
    row "FAIL  app-api               the assertion-enabled build's checks are not proven for every invocation: $enforcement"
  elif [ $base_code = $flag_code ] && cmp -s "$work/api.base.log" "$work/api.flag.log"; then
    row "ok    app-api               the overlays' diagnostics the baseline's (exit $flag_code, errors $(grep -c ': error: ' "$work/api.flag.log"))${enforcement:+; $enforcement}${crossings:+; $crossings}"
  else
    status=1
    row "FAIL  app-api               baseline exit $base_code with $(grep -c ': error: ' "$work/api.base.log") errors, overlays exit $flag_code with $(grep -c ': error: ' "$work/api.flag.log")"
  fi
fi
if [ $diagnostics = 1 ]; then
  [ -n "$app" ] || { echo "fork-one: --diagnostics reads the export's build file: pass --app" >&2; exit 2; }
  # One worker's diagnostics, the plain path's, which every run at every count gives exactly: the
  # same exit and output.
  for p in frontend api; do
    (unset TEQ_TYPE_OVERLAYS TEQ_FORK TEQ_THREADS; $p --threads 1 > "$work/$p.one.log" 2>&1)
    echo $? > "$work/$p.one.exit"
  done
  for t in $diag_counts; do
    requested=$t
    for p in frontend api; do
      exact=0 done_runs=0 proven=0 summed=
      for r in $(seq 1 $repeat); do
        log="$work/$p.$t.$r.log"
        rm -f "$work/$p.$t.$r.checks"
        (export $(at $t) $(overlays_env) TEQ_VIEW_CHECKS_LOG="$PWD/$work/$p.$t.$r.checks"; $p --threads $t > "$log" 2>&1)
        code=$?
        check_completed $code "$log" && done_runs=$((done_runs + 1))
        [ "$code" = "$(cat "$work/$p.one.exit")" ] && cmp -s "$log" "$work/$p.one.log" && exact=$((exact + 1))
        enforcement=
        crossings=
        if enforced "$work/$p.$t.$r.checks"; then proven=$((proven + 1)); fi
        summed=$crossings
      done
      if [ $exact = $repeat ] && [ $done_runs = $repeat ] && [ $proven = $repeat ]; then
        verdict=ok
      else
        verdict=FAIL
        status=1
      fi
      row "$(printf '%-5s %-9s at %-2s     %s of %s runs with one worker'"'"'s diagnostics exactly (exit %s, %s errors), %s complete, %s proven%s%s' $verdict "$p" $t $exact $repeat "$(cat "$work/$p.one.exit")" "$(grep -c ': error: ' "$work/$p.one.log")" $done_runs $proven "${enforcement:+; the last: $enforcement}" "${summed:+; $summed}")"
      [ $exact = $repeat ] || for r in $(seq 1 $repeat); do cmp -s "$work/$p.$t.$r.log" "$work/$p.one.log" || diff "$work/$p.one.log" "$work/$p.$t.$r.log" | grep '^[<>]' | head -4 | sed "s/^/        run $r: /"; done
    done
  done
  requested=$threads
fi
# The bundles' matrix (--bundles): every named case at every count and repetition, its cells each
# counted from its own invocation.
if [ $bundles = 1 ]; then
  if [ $checks != 1 ]; then
    status=1
    row "FAIL  bundles               the matrix's counts are the assertion-enabled build's"
  fi
  trim() { sed -E 's/^ +//; s/ +$//'; }
  # A case's command line: `run <inputs...> -o <file>` is a build of the inputs followed by node on the
  # file, the streams in that order and under one bound, as the raw `teq run` printed and bounded them; any
  # other first word is the binary's.
  bundle_case() {
    if [ "$1" = run ]; then
      shift
      timeout 300 bash -c '"$0" compiler build "$@" && exec node "${@: -1}"' "$teq_abs" "$@"
    else
      timeout 300 "$teq_abs" "$@"
    fi
  }
  while IFS= read -r line; do
    case $line in ''|'#'*) continue ;; esac
    name=$(echo "$line" | awk -F' [|] ' '{print $1}' | trim)
    expected=$(echo "$line" | awk -F' [|] ' '{print $2}' | trim)
    cells=$(echo "$line" | awk -F' [|] ' '{print $3}' | trim)
    case_env=$(echo "$line" | awk -F' [|] ' '{print $4}' | trim)
    case_args=$(echo "$line" | awk -F' [|] ' '{print $5}' | trim)
    [ "$cells" = - ] && cells=
    [ "$case_env" = - ] && case_env=
    b=$work/bundles/$name
    rm -rf "$b"
    mkdir -p "$b/one"
    # One worker's run without the fork, the case's environment kept, the reference of `same`.
    o=$PWD/$b/one
    (unset TEQ_TYPE_OVERLAYS TEQ_FORK TEQ_STATE_GIVEWAY; export TEQ_THREADS=1; [ -z "$case_env" ] || export ${case_env//\{tmp\}/$o}; bundle_case ${case_args//\{tmp\}/$o} > "$o.out" 2>&1; echo $? > "$o.exit") 2> /dev/null
    runs=0 good=0 problems=
    for t in $bundle_counts; do
      requested=$t
      for r in $(seq 1 $repeat); do
        d=$b/$t.$r
        mkdir -p "$d"
        runs=$((runs + 1))
        o=$PWD/$d
        (export $(at $t) $(overlays_env) TEQ_VIEW_CHECKS_LOG="$o.checks"; [ -z "$case_env" ] || export ${case_env//\{tmp\}/$o}; bundle_case ${case_args//\{tmp\}/$o} > "$o.out" 2>&1; echo $? > "$o.exit") 2> /dev/null
        code=$(cat "$d.exit")
        case $expected in
          same)
            enforcement=
            crossings=
            why=
            [ "$code" = "$(cat "$b/one.exit")" ] && cmp -s "$d.out" "$b/one.out" || why="its output is not one worker's (exit $code against $(cat "$b/one.exit"))"
            [ -z "$why" ] && ! diff -rq "$b/one" "$d" > /dev/null 2>&1 && why="its files are not one worker's"
            [ -z "$why" ] && ! enforced "$d.checks" && why="its checks are not proven: $enforcement"
            if [ -z "$why" ] && [ -n "$cells" ]; then
              counted=$(python3 tests/support/fork_one.py bundles "$d.checks" "$cells")
              [ $? = 0 ] || why=$(printf '%s' "$counted" | grep '^FAIL' | head -2 | sed 's/^FAIL //' | tr '\n' ';')
            fi ;;
          refuse:*)
            why=
            if [ "$code" = 0 ] || ! grep -qF "${expected#refuse:}" "$d.out"; then
              why="no refusal holding \"${expected#refuse:}\" (exit $code: $(grep -m1 -E 'panicked|error' "$d.out" | cut -c1-100))"
            fi ;;
          *) why="no expectation named $expected" ;;
        esac
        if [ -z "$why" ]; then
          good=$((good + 1))
        else
          problems="$problems
        at $t, run $r: $why"
        fi
      done
    done
    requested=$threads
    least=
    if [ "${expected}" = same ] && [ -n "$cells" ]; then
      least=$(cat "$b"/*.checks 2> /dev/null > "$b/all.checks"; python3 tests/support/fork_one.py bundles "$b/all.checks" "$cells" 2> /dev/null | grep '^invocation' | sed 's/^invocation [0-9]*: //' | python3 -c 'import sys
rows=[dict((kv.rsplit(" ",1)[0], int(kv.rsplit(" ",1)[1])) for kv in l.strip().split(", ")) for l in sys.stdin if l.strip()]
print(", ".join(f"{k} {min(r[k] for r in rows)}" for k in rows[0]) if rows else "")')
    fi
    if [ $good = $runs ]; then verdict=ok; else verdict=FAIL; status=1; fi
    row "$(printf '%-5s bundles %-22s %s of %s runs at %s (%s each)%s%s' $verdict "$name" $good $runs "$(echo $bundle_counts | tr ' ' ',')" $repeat "$([ "$expected" = same ] && echo ", one worker's output" || echo ", ${expected%%:*}d")" "${least:+; the least counts: $least}")"
    [ -n "$problems" ] && printf '%s\n' "$problems" | grep . | head -6
  done < tests/support/fork-one-bundles.txt
fi
# The checks' refusals, built from this tree: the arenas', the store's, the symbols' and the
# program's unit tests with the assertions compiled in.
if [ $checks = 1 ] && [ -f Cargo.toml ]; then
  timeout 900 cargo test --profile checks --bin teq -- arena:: types:: symbols:: tir:: > "$work/refusals.log" 2>&1
  code=$?
  summary=$(grep -E '^test result:' "$work/refusals.log" | tail -1)
  if [ $code = 0 ]; then
    row "ok    refusals              the arenas', the store's, the symbols' and the program's unit tests with the assertions, from this tree: ${summary#test result: }"
  else
    status=1
    row "FAIL  refusals              the arenas', the store's, the symbols' and the program's unit tests with the assertions (exit $code): ${summary:-$(tail -1 "$work/refusals.log")}"
  fi
fi
echo
echo "fork-one: $version at $revision, $(overlays_env) at $threads workers ($(at $threads | tr '\n' ' ' | sed 's/ $//')) against $([ $reference = one ] && echo "one worker's output without the fork" || echo "the overlays off at the same count"), $repeat run(s) each, and TEQ_VIEW_CHECKS=panic"
printf '%s\n' "${rows[@]}"
exit $status
