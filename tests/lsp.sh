#!/bin/bash
# The language server (`teq lsp`, docs/TARGETS.md "The language server"): tests/lsp/driver.mjs
# starts it over a copy of tests/lsp/ws (two projects sharing a root, one for the JVM and one
# for JavaScript with a macro, the teq.lock describing them written by the driver, and a
# file of no project) and of tests/lsp/bare (a root without export), and of tests/lsp/libs
# (projects reading jars with and without their sources jars and an upstream module's products),
# then over the sessions' cap and idle stop and the sbt fallback (tests/lsp/fake-sbt), and checks
# every answer against what the sources say; the builds a newer edit overtakes are driven through
# the server's test hooks (TEQ_LSP_TRACE_FILE, TEQ_LSP_TEST_BARRIER), never by sleeps. Without node, scala-library in the coursier cache (which the JVM
# project needs) or the sources jars the libraries' scenario reads, it fails as incomplete
# validation. Under TEQ_FORK=1 the sessions the server starts build through the parallel typer's
# fork and merge with one worker, their navigation index merged with the rest;
# with the assertion-enabled build the checks' log is the evidence
# that the children forked, the forks its end lines count summed, and a run in which none did
# fails. Then the driver runs again with each session's first full build typed by two and by
# sixteen workers through the merge (`TEQ_SESSION_THREADS`, the state give-way bypassed), whose navigation index holds several workers' records: every answer is checked as at one,
# and the assertion-enabled build's checks' log proves the joins: every session (a `watch` process)
# that typed a program joined its first build's workers, once and at that count, the driver's own
# build kept at one worker, and as many sessions at sixteen as at two. The first pass types every
# session's full builds at the automatic count (tests/support/sessions.sh), its line saying which
# forked. After it, once, the driver's `idleMinute` alone: an idle session's polls over a minute,
# read from the server's trace against the rule, its line giving their count and spacing.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
TEQ=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
if ! command -v node > /dev/null; then
  echo "FAIL: no node, which drives the language server (incomplete validation)"
  exit 1
fi
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
. tests/support/sessions.sh
if ! "$TEQ" compiler check tests/lsp/ws/shared/src --target jvm > "$work/jar.log" 2>&1; then
  echo "FAIL: the JVM project does not check under scala-library (incomplete validation): $(tail -1 "$work/jar.log")"
  exit 1
fi
# A jar for the scenario that removes one from a class path and restores it, and another version
# of it for the libraries' scenario, which reads both with their sources jars.
. tests/support/jars.sh
jar=$(jar_of sourcecode)
jar2=$(jar_of sourcecode-0.4.4)
for j in "$jar" "$jar2" "$(jar_of scala-library)"; do
  if [ ! -f "${j%.jar}-sources.jar" ]; then
    echo "FAIL: no sources jar beside $j, which the libraries' scenario reads (incomplete validation)"
    exit 1
  fi
done
if [ "${TEQ_FORK:-}" = 1 ]; then
  export TEQ_VIEW_CHECKS_LOG=$work/checks.log
  : > "$TEQ_VIEW_CHECKS_LOG"
fi
sessions_log "$work/workers.log"
timeout 330 node tests/lsp/driver.mjs "$TEQ" "$work/run" "$jar" "$jar2"
status=$?
sessions_forked lsp
unset TEQ_SESSION_WORKERS_LOG
# The idle minute, once: the server's schedule, which the worker count does not touch.
timeout 150 env LSP_ONLY=idleMinute node tests/lsp/driver.mjs "$TEQ" "$work/idle" "$jar" "$jar2" > "$work/idle.log" 2>&1
code=$?
grep -E '^(lsp: an idle|FAIL)' "$work/idle.log"
echo "lsp: the idle minute: $(tail -1 "$work/idle.log")"
[ $code = 0 ] || status=1
if [ "${TEQ_FORK:-}" = 1 ]; then
  if "$TEQ" --version | grep -q assertions; then
    forks=$(sed -nE 's/^view checks: end [0-9]+: (typed|not typed), ([0-9]+) forks.*/\2/p' "$TEQ_VIEW_CHECKS_LOG" | awk '{ n += $1 } END { print n + 0 }')
    echo "lsp: under TEQ_FORK=1 the sessions forked and merged $forks times"
    [ "$forks" -gt 0 ] || { echo "FAIL: no session forked under TEQ_FORK=1"; status=1; }
  else
    echo "lsp: under TEQ_FORK=1, the forks unproven: the evidence is the assertion-enabled build's checks' log"
  fi
fi
for n in 2 16; do
  log=$work/checks-$n.log
  : > "$log"
  env -u TEQ_FORK TEQ_SESSION_THREADS=$n TEQ_STATE_GIVEWAY=off TEQ_VIEW_CHECKS_LOG=$log \
    timeout 330 node tests/lsp/driver.mjs "$TEQ" "$work/run-$n" "$jar" "$jar2" > "$work/driver-$n.log" 2>&1
  code=$?
  echo "lsp: the sessions' first builds at $n workers: $(tail -1 "$work/driver-$n.log")"
  if [ $code != 0 ]; then
    grep -E '^FAIL' "$work/driver-$n.log" | head -5
    status=1
  fi
  if "$TEQ" --version | grep -q assertions; then
    verdict=$(python3 - "$log" "$n" <<'PY'
import re, sys
log, n = sys.argv[1], sys.argv[2]
sessions, joins, typed = set(), {}, set()
for line in open(log):
    if m := re.match(r"view checks: start (\d+) watch ", line):
        sessions.add(m.group(1))
    elif m := re.match(r"view checks: join (\d+): the \S+ namespace, (\d+) workers,", line):
        joins.setdefault(m.group(1), []).append(m.group(2))
    elif m := re.match(r"view checks: end (\d+): typed,", line):
        typed.add(m.group(1))
wrong = [p for p, js in joins.items() if p not in sessions or js != [n]]
unjoined = [p for p in sessions & typed if p not in joins]
print(f"{len(joins)} {len(wrong)} {len(unjoined)} {len(sessions)}")
PY
)
    read -r joined wrong unjoined started <<< "$verdict"
    echo "lsp: $joined of $started sessions joined their first build's $n workers, once each"
    [ "$joined" -gt 0 ] && [ "$wrong" = 0 ] && [ "$unjoined" = 0 ] || { echo "FAIL: at $n workers $wrong joins not one session's first at that count, $unjoined sessions typed without one"; status=1; }
    if [ -n "${first_joined:-}" ] && [ "$joined" != "$first_joined" ]; then
      echo "FAIL: $joined sessions joined at $n workers, $first_joined at 2"
      status=1
    fi
    first_joined=${first_joined:-$joined}
  fi
done
exit $status
