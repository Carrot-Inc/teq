#!/usr/bin/env python3
"""How tests/workers.sh's session_flat reads a session's growth:

  session_flat.py session <teq> [NAME=value ...] [argument ...]
  session_flat.py series <readings file> [<term>,...] --key <program>@<workers> [--bands <table>]
  session_flat.py band <teq> <workers> <sessions> [NAME=value ...] [argument ...]

`session` runs a watch session over tests/split/cycle (the arguments its own, a NAME=value its environment's)
through 169 full builds, and more while the last five are on the high level (up to fifty-five more), and asks
`stats` twice after each. A build's reading is the higher of its two answers: one answer of two can be lower, that
one alone (in 300 sessions under load, 4,197 of 50,700 builds by 752 bytes, and the first answer after build 124 by
31,584 in 299 of them). It prints three lines (`lines`): the growth, `<longest> <low> <levelled> <spikes>:
<sequence>`, and which reading it is.

**The key.** A session is read by its key, its program and its workers' count (--threads's value, `auto` without
one). The level's band below is the level of one program at one count, a row of tests/workers/session_flat_bands.tsv
that `band` measured (its sessions, its runs up's heights over their flanks, and 900 bytes each side); a key with
no row is read by master's reading, unchanged (the lowest reading after the builds 90 to 109 less the lowest after
11 to 30), and its third line says so. No band is guessed for another program or count: a clean session of
another program of the tree stands its runs up 1,792 to 55,792 bytes above their flanks at eight workers, and
tests/split/cycle at two workers holds a second level to 10,720 bytes.

**The level.** At eight workers the blocks in use sit on one of two levels some 4.2 to 5.4 KB apart, more than the
2 KB the check allows, the same for every `stats` after a build and after a pause of 0.3 s, and in runs of builds
(the longest seen under the load of sixteen busy loops on eight CPUs: 23 builds in 304 sessions on master
0383f3ff0's binary, 15 in 300 on 31ebc6aee's). What it is a level of, as far as `stats` tells: blocks the
allocator counts in use (`live`, its reserve less the free lists of the centre, the compiler thread and the typing
thread) and none of the session's accounts holds (no field of `stats.session` moves with it), in a process whose
threads between builds are those three alone (the workers end with each build, their lists handed back): a block a
build of several workers leaves or does not. A session of one worker never leaves it: none of 300 sessions of one
worker recorded under load has a build 2 KB above its lowest, and its key has no row. So the level is bounded by
what it is, measured apart from any session judged: over the 31,986 runs of it in 900 sessions of tests/split/cycle
at eight workers (three binaries), each build of a run stood 3,952 to 6,192 bytes above the lowest of the five builds
on each side of it; the key's row is that and some 900 bytes each side, 3,072 to 7,168. The band is never taken
from the session judged: a growth's own spike would set it (the re-check's mutant held 8,192 bytes over builds 40-44
and again over 90-109, which a band of the session's short runs took for the level).

`judge` reads, for a key with a band:

- **The level's runs.** A build is up where it is 2 KB or more above the lowest within sixty builds of it. A run of
  up builds is on the level where it ends within fifty builds and each of its builds stands above the lowest of the
  five builds on each side of it by a height of the band. Its builds are no
  evidence either way. A run that reaches the session's last build is judged by nothing after it and is evidence.
- **The spikes.** A run up with a build outside the band (both sides of it read) is a growth's spike: red of its
  own, at any length, its red naming `band`, which measures the row again.
- **The growth.** Every side of twenty builds from the eleventh on, the lowest of its builds that are evidence; the
  growth is the most any side is above an earlier one it does not overlap. So it is at least the reading of the
  sides 11-30 and 90-109 that the check read before, but where a side's lowest builds were on the level.
- **The length.** The longest run of builds 2 KB or more above the session's lowest from the eleventh on: more than
  fifty, twice the longest seen, is a red of its own (the level held past the reading's sizing, or a growth).

Measured on 900 sessions of tests/split/cycle at eight workers (three binaries) recorded under the load of sixteen
busy loops on eight CPUs, each as recorded and raised by 508 shapes of growth (184 of one term: a growth a build of
10 to 126 bytes, a step of 2,100 to 6,000 bytes from a build between 31 and 109, a steady growth with an allocation
freed after a build from 100 to 140, an allocation held from a build of 60 to 95 to one of 109 to 160; and 324 of
two: a spike over builds 40 to 40, 42 or 44 and then a hold from build 90 for 20, 30 or 50 builds, each of the
heights 2,100, 3,000, 4,608, 6,000, 8,192 and 12,000 bytes): none of the 900 is red as recorded, and of the 411,947
cells of those shapes the earlier reading refused none passes. What passes that it refused is the level itself,
planted (a level's spike and then the level held over builds 90 to 109, 119 or 139): of its 7,200 cells in the two
sets of 169 builds, 2,660 and 2,684 pass, the band's one tolerance, which the gates read as a growth. The re-check's
mutant's shape is red in all 900, and in each of 300 sessions of one worker, whose key has no band and which
master's reading reads.

`series` reads the same off a file of readings, a build a line, its two answers on it, the first build's first, each
raised as the terms say: `<bytes>` a build, `<bytes>@<a>` from build a on, `<bytes>@<a>-<b>` from build a to b, and
`level@<a>-<b>`, builds a to b put on the high level (each one below it raised by the level's height: the lowest
high reading less the lowest). That is the reading's decision with no compiler in it, which tests/workers.sh shows
on five recorded sessions under load: tests/workers/session_flat_levels.txt (the one of 31ebc6aee's 300 with the
longest run, 15 builds), session_flat_dip.txt (one with a lower answer on its late side alone), the two of 120
builds the earlier reading was shown on (_levels_120, _dip_120), and session_flat_one.txt, a session of one worker.
"""
import json
import os
import subprocess
import sys

FIRST = 11
BUILDS = 169
SIDE = 20
HIGH = 2048
RUN_MAX = 50
NEAR = 60
FLANK = 5
PROGRAM = "tests/split/cycle"
# The level's band of a session by its key, the program and the workers' count: tests/workers/session_flat_bands.tsv,
# each row measured by `band` (below). A key with no row is read by master's reading, unchanged: no band is guessed.
REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
BANDS_FILE = os.path.join(REPO, "tests", "workers", "session_flat_bands.tsv")
MASTER_EARLY = range(11, 31)
MASTER_LATE = range(90, 110)
MARGIN = 900


class BadTable(Exception):
    """A band table whose rows cannot be held to: no reading is made with it."""


def bands(path=None):
    """The measured bands of the table (tests/workers/session_flat_bands.tsv unless named): (program, workers) ->
    (low, high). BadTable, naming the row, for a row of other than five fields (the program, the workers' count, the
    low, the high, how it was measured), a count or a bound that is no number, a low not below its high, a program
    that is no directory of the tree, and a key given twice, the same row or another band (the last row would win
    silently)."""
    path = path or BANDS_FILE
    out = {}
    for i, line in enumerate(open(path, encoding="utf-8"), 1):
        if line.startswith("#") or not line.strip():
            continue
        where = f"{os.path.relpath(path, REPO) if path.startswith(REPO + os.sep) else path}:{i}"
        fields = line.rstrip("\n").split("\t")
        if len(fields) != 5 or not all(f.strip() for f in fields):
            raise BadTable(f"{where}: not five fields (program, workers, low, high, how it was measured)")
        program, workers, low, high, _ = fields
        if not (workers.isdigit() and low.isdigit() and high.isdigit()):
            raise BadTable(f"{where}: the workers' count, the low and the high are numbers")
        if int(low) >= int(high):
            raise BadTable(f"{where}: the low {low} is not below the high {high}")
        if not os.path.isdir(os.path.join(REPO, program)):
            raise BadTable(f"{where}: {program} is no program of the tree")
        if (program, workers) in out:
            raise BadTable(f"{where}: {program} at {workers} workers has a row already: a key is one row")
        out[(program, workers)] = (int(low), int(high))
    return out


def master_growth(readings):
    """Master's reading, unchanged: the lowest reading after the builds 90 to 109 less the lowest after 11 to 30."""
    return min(build_reading(readings[i]) for i in MASTER_LATE) - min(build_reading(readings[i]) for i in MASTER_EARLY)


def build_reading(answers):
    """A build's reading: the higher of its two answers."""
    return max(answers)


def runs_of(builds, up):
    """The runs of consecutive builds of `builds` for which `up` holds."""
    runs, cur = [], []
    for i in builds:
        if up(i):
            cur.append(i)
        elif cur:
            runs.append(cur)
            cur = []
    if cur:
        runs.append(cur)
    return runs


def judge(readings, band):
    """The session's reading: `growth`, the most any side of SIDE builds' evidence is above an earlier one's (each
    side the lowest of its builds off a level), `longest`, the longest run of builds 2 KB or more above the lowest
    from build FIRST on, `low`, that lowest, `levelled`, how many builds are on a level, `spikes`, how many runs up
    stand outside the level's band (each a growth of its own, red), and `sequence`; `band` the key's (low, high)."""
    r = {i: build_reading(a) for i, a in readings.items()}
    last = max(r)
    builds = range(FIRST, last + 1)
    low = min(r[i] for i in builds)
    sequence = "".join("^" if r[i] - low >= HIGH else "." for i in sorted(r))
    longest = max((len(x) for x in runs_of(builds, lambda i: r[i] - low >= HIGH)), default=0)
    # A build is up where it is 2 KB or more above the lowest within NEAR builds of it: a level's run is up
    # against the low builds around it, a growth's builds against the earlier ones.
    near = {i: min(r[j] for j in range(max(FIRST, i - NEAR), min(last, i + NEAR) + 1)) for i in builds}
    up = runs_of(builds, lambda i: r[i] - near[i] >= HIGH)

    def heights(run):
        """A run's builds above the lowest of the FLANK builds on each side of it; none without both sides."""
        before = [r[j] for j in range(max(FIRST, run[0] - FLANK), run[0])]
        after = [r[j] for j in range(run[-1] + 1, min(last, run[-1] + FLANK) + 1)]
        return [r[i] - min(before + after) for i in run] if before and after else None

    level, spikes = set(), 0
    lo, hi = band
    for run in up:
        h = heights(run)
        if h is None:
            continue  # a run the session's edge cuts: evidence, judged by the sides
        if len(run) <= RUN_MAX and all(lo <= v <= hi for v in h):
            level.update(run)
        elif any(v < lo or v > hi for v in h):
            spikes += 1
    sides = {}
    for s in range(FIRST, last - SIDE + 2):
        evidence = [r[i] for i in range(s, s + SIDE) if i not in level]
        if evidence:
            sides[s] = min(evidence)
    growth = max((sides[b] - sides[a] for a in sides for b in sides if b >= a + SIDE), default=0)
    return {"growth": growth, "longest": longest, "low": low, "levelled": len(level), "spikes": spikes, "sequence": sequence}


def read(readings, key, table=None):
    """The session's reading by its key (program, workers): the band's reading where the key has a measured row of
    the table (`bands`), master's elsewhere; with `reading`, which of the two, as the third line says it."""
    band = bands(table).get(key)
    if band:
        j = judge(readings, band)
        j["reading"] = f"reading: the band of {key[0]} at {key[1]} workers, {band[0]} to {band[1]} bytes (tests/workers/session_flat_bands.tsv)"
        return j
    j = judge(readings, (0, -1))  # every run up is evidence: only the line's levels and sequence are read here
    j.update(growth=master_growth(readings), levelled=0, spikes=0,
             reading=f"reading: master's (no band measured for {key[0]} at {key[1]} workers)")
    return j


def lines(readings, key, table=None):
    """The three lines the reading prints: the growth, `<longest> <low> <levelled> <spikes>: <sequence>`, and which
    reading it is."""
    j = read(readings, key, table)
    return f"{j['growth']}\n{j['longest']} {j['low']} {j['levelled']} {j['spikes']}: {j['sequence']}\n{j['reading']}"


def workers_of(arguments):
    """A session's workers' count by its arguments: --threads's value, else `auto`."""
    for i, a in enumerate(arguments):
        if a == "--threads" and i + 1 < len(arguments):
            return arguments[i + 1]
        if a.startswith("--threads="):
            return a.split("=", 1)[1]
    return "auto"


def band_of(teq, workers, sessions, arguments):
    """The key's row measured: `sessions` sessions at `workers`, each run up's builds above the lowest of the FLANK
    builds on each side of it, and MARGIN bytes each side of the lowest and the highest."""
    hs = []
    for _ in range(sessions):
        r = {i: build_reading(a) for i, a in session(teq, ["--threads", workers] + arguments).items()}
        last = max(r)
        builds = range(FIRST, last + 1)
        near = {i: min(r[j] for j in range(max(FIRST, i - NEAR), min(last, i + NEAR) + 1)) for i in builds}
        for run in runs_of(builds, lambda i: r[i] - near[i] >= HIGH):
            before = [r[j] for j in range(max(FIRST, run[0] - FLANK), run[0])]
            after = [r[j] for j in range(run[-1] + 1, min(last, run[-1] + FLANK) + 1)]
            if before and after:
                hs += [r[i] - min(before + after) for i in run]
    if not hs:
        return f"# {PROGRAM} at {workers} workers: no run up in {sessions} sessions (no level: no row)"
    return f"{PROGRAM}\t{workers}\t{min(hs) - MARGIN}\t{max(hs) + MARGIN}\t{sessions} sessions: {min(hs)} to {max(hs)}"


def session(teq, arguments):
    env = {k: v for k, v in os.environ.items() if not k.startswith("TEQ_")}
    env["TEQ_COMPACT_EVERY"] = "1"
    args = []
    for a in arguments:
        if "=" in a:
            k, v = a.split("=", 1)
            env[k] = v
        else:
            args.append(a)
    p = subprocess.Popen([teq, "compiler", "watch", "--check", PROGRAM, *args], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)

    def send(c):
        p.stdin.write(c + "\n")
        p.stdin.flush()
        return json.loads(p.stdout.readline())

    json.loads(p.stdout.readline())
    readings = {}
    i = 0
    # BUILDS builds, and while the last FLANK are not all on the session's lowest level, more (up to RUN_MAX and
    # FLANK more): a run of the high level that reaches the session's end is judged by the builds after it.
    while True:
        i += 1
        send("build")
        readings[i] = [send("stats")["stats"]["allocator"]["live"] for _ in range(2)]
        if i >= BUILDS:
            low = min(build_reading(readings[j]) for j in range(FIRST, i + 1))
            if i >= BUILDS + RUN_MAX + FLANK or all(build_reading(readings[j]) - low < HIGH for j in range(i - FLANK + 1, i + 1)):
                break
    p.stdin.write("quit\n")
    p.stdin.flush()
    p.wait(timeout=30)
    return readings


def series(path, grown):
    """The file's answers by build, raised as `grown` says: terms separated by commas, `<bytes>` a build, `<bytes>@<a>`
    from build a on, `<bytes>@<a>-<b>` from build a to b, and `level@<a>-<b>`, builds a to b put on the high level
    (each one 2 KB or more below it raised by the level's height: the lowest high reading less the lowest)."""
    with open(path) as f:
        lines = f.read().splitlines()
    for k, line in enumerate(lines, 1):
        if len(line.split()) != 2 or not all(x.lstrip("-").isdigit() for x in line.split()):
            sys.exit(f"session_flat.py: {path}:{k}: not two answers: {line[:40]!r}")
    if len(lines) < FIRST + 4 * SIDE:
        sys.exit(f"session_flat.py: {path}: {len(lines)} builds, fewer than {FIRST + 4 * SIDE}")
    readings = {i: [int(x) for x in line.split()] for i, line in enumerate(lines, 1)}
    for term in filter(None, grown.split(",")):
        if term.startswith("level@"):
            a, b = (int(x) for x in term[len("level@"):].split("-"))
            r = {i: build_reading(v) for i, v in readings.items() if i >= FIRST}
            low = min(r.values())
            height = min(v - low for v in r.values() if v - low >= HIGH)
            readings = {i: [x + height for x in v] if a <= i <= b and build_reading(v) - low < HIGH else v for i, v in readings.items()}
            continue
        step, _, at = term.partition("@")
        if not at:
            readings = {i: [x + int(step) * i for x in v] for i, v in readings.items()}
            continue
        a, _, b = at.partition("-")
        a, b = int(a), int(b) if b else max(readings)
        readings = {i: [x + int(step) if a <= i <= b else x for x in v] for i, v in readings.items()}
    return readings


if __name__ == "__main__":
    argv = sys.argv[1:]
    table = None
    if len(argv) >= 6 and argv[0] == "series" and argv[-2] == "--bands":
        table, argv = argv[-1], argv[:-2]
    try:
        if len(argv) >= 2 and argv[0] == "session":
            print(lines(session(argv[1], argv[2:]), (PROGRAM, workers_of(argv[2:]))))
        elif len(argv) in (4, 5) and argv[0] == "series" and argv[-2] == "--key" and "@" in argv[-1]:
            program, workers = argv[-1].rsplit("@", 1)
            print(lines(series(argv[1], argv[2] if len(argv) == 5 else ""), (program, workers), table))
        elif len(argv) >= 4 and argv[0] == "band" and argv[3].isdigit():
            print(band_of(argv[1], argv[2], int(argv[3]), argv[4:]))
        else:
            sys.exit(__doc__)
    except BadTable as e:
        sys.exit(f"session_flat.py: the band table refused: {e}")
