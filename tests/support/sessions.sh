# Sourced by the suites of sessions (split-watch, check-watch, jvm-watch, lsp, app): their sessions
# type every full build at the automatic count (docs/TARGETS.md, "The typer's workers"), the counts
# a caller's environment may ask for cleared, and every reference that stands for one worker's
# output is pinned to one (`$ONE`). `sessions_log <file>` logs each full build a session answers
# (`TEQ_SESSION_WORKERS_LOG`), `session_started <pid> <label>` names a session by its program, and
# `sessions_forked <suite>` prints the line saying which sessions forked and at what count.
unset TEQ_THREADS TEQ_SESSION_WORKERS TEQ_SESSION_THREADS
ONE="--threads 1"
SESSIONS_LABELS=
sessions_log() {
  export TEQ_SESSION_WORKERS_LOG=$1
  : > "$1"
  SESSIONS_LABELS=$1.labels
  : > "$SESSIONS_LABELS"
}
session_started() {
  [ -n "$SESSIONS_LABELS" ] && echo "$1 $(basename "$2")" >> "$SESSIONS_LABELS"
}
sessions_forked() {
  python3 - "$1" "$TEQ_SESSION_WORKERS_LOG" "$SESSIONS_LABELS" <<'PY'
import collections, sys
suite, log, labels = sys.argv[1:]
names = dict(l.split(" ", 1) for l in open(labels).read().splitlines() if " " in l) if labels else {}
builds, forked = 0, collections.OrderedDict()
for line in open(log):
    f = line.split()
    if len(f) < 4 or f[2] == "end":
        continue
    builds += 1
    if f[3] in ("joined", "retried", "refused"):
        who = names.get(f[0], names.get(f[1], "a child of " + f[1]))
        forked.setdefault(who, collections.Counter())[f[3] + " " + f[4]] += 1
said = "; ".join(f"{who}: " + ", ".join(f"{n} {how}" for how, n in c.items()) for who, c in forked.items())
print(f"{suite}: {builds} full builds of sessions at the automatic count, {sum(sum(c.values()) for c in forked.values())} forked" + (f" ({said})" if said else "") + ", the rest under the threshold or pinned to one worker")
PY
}
