#!/usr/bin/env python3
"""The evidence tests/fork-one.sh reads, apart from its shell:

  fork_one.py enforced <checks log> <namespace> <workers asked>
  fork_one.py records <records dir> <names file>
  fork_one.py outputs <skip file> <candidate records dir> <reference records dir>...
  fork_one.py bundles <checks log> <cell,...>

`enforced` reconciles the assertion-enabled build's checks' log (`TEQ_VIEW_CHECKS_LOG`) per compiler
invocation, a process, and per image of it: an image begins at a `start` line, and the parallel
attempt that gives way to one worker is replaced by the build typed again under the same process
(`serial_again`). An image proves its checks when every worker it began is in a checked context of
the namespace asked for in the mode that panics, at the worker count asked for (a test's own
`--threads` apart), every fork it began has every worker of its count named (or its slot named
empty, a worker the queue had no work for and that was never started) and joined or gave way, and its end line's forks are its joins, each with the overlays; an attempt that gave way is
followed by the build typed again, which ends; an image with no end line ended by a signal, which
the invocation's outcome says (`exit <pid> <status>`, tests/support/fork-one-teq.sh). It prints
the counts `enforced` in the runner reads: invocations, checked, checked and given way, checked
with nothing read, ended before their join, typed no program, at a test's own count, unproven,
the checks, the nine crossings summed, and the unproven processes.

`records` digests, per failing test of the names file, the invocations of the compiler kept under
the records dir that name it (an argument with the test's name as a path component): each one's
arguments, exit or signal and everything it wrote, with the schedule's noise left out (a temporary
directory; on the standard error the timings and a panic's thread), in the order of their arguments.

`outputs` compares every invocation under the candidate's records dir with the reference runs'
of the same directory and arguments, the timing report and the durations left out, and prints
the arguments of each one no reference run matches, those naming a test of the skip file apart.

`bundles` reads the bundles' counts each join and attempt given way wrote (`bundles:` after the
crossings), summed per invocation, and holds every invocation that joined
a fork to the cells asked for (`published.<class>`, `released.<class>`, `entered.<class>` with the
class `fun`, `val`, `class`, `done` or `template`; `read.direct`, `read.cache`, `waited.withheld`,
`sealed`): each above zero in that invocation's own counts, so that a cell another invocation of
the log exercised covers none. It prints a line per invocation with the cells' counts and fails
on a join without the counts, a cell no invocation can name, an invocation with a cell at zero,
an invocation whose attempt gave way with no fork joined, or a log with no join.
"""
import hashlib
import os
import re
import sys

EMPTY = re.compile(r"^view checks: empty (\d+): worker (\d+) of (\d+) in fork (\d+)")
WORKER = re.compile(r"^view checks: worker (\d+): the (\S+) namespace, mode (\S+), (in a checked context|outside the checked context); worker (\d+) of (\d+) in fork (\d+)")
JOIN = re.compile(r"^view checks: join (\d+): the (\S+) namespace, (\d+) workers, mode (\S+), (\d+) reads and constructions checked, \d+ publications checked, \d+ items typed by the workers, (\d+) of (\d+) workers in a checked context; fork (\d+);")
GAVE = re.compile(r"^view checks: gave way (\d+): fork (\d+),")
END = re.compile(r"^view checks: end (\d+): (typed|not typed), (\d+) forks, (\d+) with the overlays")
START = re.compile(r"^view checks: start (\d+) ")
EXIT = re.compile(r"^view checks: exit (\d+) (\d+)$")
CROSS = re.compile(r"crossings: [^;]*;[^;]*;[^;]*;[^;]*own;[^;]*runs")


class Image:
    def __init__(self, line):
        self.again = "(typed again by one worker)" in line
        m = re.search(r"; threads (\d+) \(--threads\)", line)
        self.own_count = int(m.group(1)) if m else None
        self.workers = []  # (namespace, mode, checked, k, n, fork)
        self.joins = []  # (namespace, n, mode, reads, active, total, fork)
        self.empty = []  # (k, n, fork): a worker's slot the fork left empty
        self.gave = None  # fork
        self.end = None  # (typed, forks, overlaid)


def enforced(log, ns, requested):
    images = {}
    exits = {}
    crossed = [0] * 9
    for line in open(log, errors="replace"):
        line = line.rstrip("\n")
        m = START.match(line)
        if m:
            images.setdefault(m.group(1), []).append(Image(line))
            continue
        m = EXIT.match(line)
        if m:
            exits[m.group(1)] = int(m.group(2))
            continue
        pid = None
        for pat in (WORKER, EMPTY, JOIN, GAVE, END):
            m = pat.match(line)
            if m:
                pid = m.group(1)
                break
        if pid is None or not images.get(pid):
            continue
        image = images[pid][-1]
        if pat is WORKER:
            image.workers.append((m.group(2), m.group(3), m.group(4) == "in a checked context", int(m.group(5)), int(m.group(6)), int(m.group(7))))
        elif pat is EMPTY:
            image.empty.append((int(m.group(2)), int(m.group(3)), int(m.group(4))))
        elif pat is JOIN:
            image.joins.append((m.group(2), int(m.group(3)), m.group(4), int(m.group(5)), int(m.group(6)), int(m.group(7)), int(m.group(8))))
        elif pat is GAVE:
            image.gave = int(m.group(2))
        else:
            image.end = (m.group(2) == "typed", int(m.group(3)), int(m.group(4)))
        if pat in (JOIN, GAVE):
            c = CROSS.search(line)
            if c:
                for i, x in enumerate(int(v) for v in re.findall(r"\d+", c.group(0))):
                    if i < 9:
                        crossed[i] += x
    counts = dict(n=0, proven=0, gave=0, idle=0, aborted=0, plain=0, own=0, unforked=0, unproven=0, checks=0)
    unproven = []
    for pid, imgs in images.items():
        counts["n"] += 1
        kind, checks, own = classify(imgs, exits.get(pid), ns, requested)
        if kind == "unproven":
            unproven.append(pid)
        else:
            counts["checks"] += checks
            if own:
                counts["own"] += 1
        counts[kind] += 1
    listed = ""
    for pid in unproven:
        if len(listed) < 60:
            listed = (listed + " " + pid).strip()
    fields = [counts[k] for k in ("n", "proven", "gave", "idle", "aborted", "plain", "own", "unforked", "unproven", "checks")]
    print(" ".join(str(x) for x in fields + crossed) + (" " + listed if listed else ""))


def classify(imgs, status, ns, requested):
    """What one invocation proves: its first image is the build or the parallel attempt, a second
    the build typed again by one worker after the attempt gave way."""
    first = imgs[0]
    want = first.own_count or requested
    own = first.own_count is not None
    if not image_sound(first, ns, want):
        return "unproven", 0, own
    if first.gave is not None:
        # The fallback's evidence: the build typed again, and its end or its outcome.
        if len(imgs) != 2 or not imgs[1].again or (imgs[1].end is None and status is None):
            return "unproven", 0, own
        return "gave", 0, own
    if len(imgs) != 1:
        return "unproven", 0, own
    if not first.workers and first.own_count == 1 and requested > 1 and not first.joins:
        # A test's own `--threads 1` above one worker: one worker's build, which forks nothing,
        # ended by itself or by a signal (an abort the reference has too).
        ended = first.end is not None and first.end[0] and first.end[1] == 0
        if ended or (first.end is None and status is not None and status > 128):
            return "unforked", 0, own
    if not first.workers:
        if first.end is not None and not first.end[0] and not first.joins:
            return "plain", 0, own
        return "unproven", 0, own
    if first.end is None:
        # An abort: the invocation's outcome says the binary ended by a signal.
        if status is None or status <= 128:
            return "unproven", 0, own
    reads = sum(j[3] for j in first.joins)
    if reads > 0:
        return "proven", reads, own
    if first.joins:
        return "idle", 0, own
    return ("aborted" if first.end is None else "unproven"), 0, own


def image_sound(image, ns, want):
    """Every worker checked in the namespace and the mode asked for at the count asked for, every
    fork with every worker named, as begun or as its slot left empty but never both, and joined
    (or given way), each join agreeing with the workers named as begun, and the end line's forks the
    joins, each with the overlays."""
    began, empty = {}, {}
    for (wns, mode, checked, k, n, fork) in image.workers:
        if wns != ns or mode != "panic" or not checked or n != want:
            return False
        began.setdefault(fork, set()).add(k)
    for (k, n, fork) in image.empty:
        if n != want:
            return False
        empty.setdefault(fork, set()).add(k)
    forks = set(began) | set(empty)
    joined = set()
    for (jns, n, mode, _reads, active, total, fork) in image.joins:
        if jns != ns or mode != "panic" or n != want or total <= 0 or active != total or total != len(began.get(fork, ())):
            return False
        joined.add(fork)
    if image.gave is not None:
        joined.add(image.gave)
    for fork in forks:
        b, e = began.get(fork, set()), empty.get(fork, set())
        if b & e:
            return False
        if fork in joined and b | e != set(range(want)):
            return False
    if image.end is not None:
        _typed, nforks, overlaid = image.end
        if nforks != overlaid or overlaid != len(image.joins) or forks - joined:
            return False
    elif image.gave is None and forks - joined and image.joins:
        return False
    return True


# A temporary directory's name, in any stream and in the arguments.
NOISE = [
    (re.compile(r"/tmp/[A-Za-z0-9._-]*\.[A-Za-z0-9]{6,}"), "/tmp/N"),
    (re.compile(r"(/var/folders/[^ :]*/T/)[A-Za-z0-9._-]*\.[A-Za-z0-9]{6,}"), r"\1N"),
]

# The compiler's own reports, which it writes to the standard error and a program's standard
# output never carries: a panic's thread and the driver's one-line summaries.
REPORTS = [
    (re.compile(r"^(thread '[^']*') \(\d+\) panicked at ", re.M), r"\1 (N) panicked at "),
    (re.compile(r"^(checked|built|ran) .* in .*\n", re.M), ""),
]

# The sections of the compiler's timing report (src/report.rs, written whole on the standard
# error), whose indented rows carry times that differ from run to run.
TIMED_SECTIONS = {
    "workers", "workers, the attempt that gave way", "bodies typed", "the attempt's bodies typed",
    "waits and holds", "the attempt's waits and holds", "macro reach", "the attempt's macro reach",
    "classpath", "Java", "library bodies", "phases", "shape census", "the type store's overlays",
    "types made by the workers",
}
TIME = re.compile(r" *\b[0-9]+\.[0-9]+ ms\b")


# The first rows the compiler's reports open their sections with, which a run of sections has to
# hold to be one of them: `--time`'s last, and the forked build's first, which `TEQ_FORK=1` prints
# without `--time` (src/measure.rs).
SIGNATURES = {("phases", "  lines "), ("workers", "  phase: signatures "), ("workers, the attempt that gave way", "  phase: signatures ")}


def reports(lines):
    """The timing report's runs of section titles and indented rows in `lines`, as (first, end)
    ranges: a run that holds one of its sections' first rows (`SIGNATURES`); a program's lines of
    the same shape are none."""
    i = 0
    while i < len(lines):
        if lines[i] not in TIMED_SECTIONS:
            i += 1
            continue
        j = i + 1
        while j < len(lines) and (lines[j].startswith("  ") or lines[j] in TIMED_SECTIONS):
            j += 1
        if any((lines[k], lines[k + 1][: len(row)]) == (title, row) for k in range(i, j - 1) for title, row in SIGNATURES):
            yield i, j
        i = j


def untimed(text):
    """The timing report's figures in `text` replaced."""
    lines = text.split("\n")
    for i, j in list(reports(lines)):
        for k in range(i, j):
            if lines[k].startswith("  "):
                lines[k] = TIME.sub(" N ms", lines[k])
    return "\n".join(lines)


# The durations `teq tasty` prints in its own lines: `--bodies`' "decoded in <t> (<n> bodies per
# second)" and `--load`'s "<n> errors in <t>".
DURATION = [
    (re.compile(r"\bdecoded in [0-9]+(\.[0-9]+)?(ns|µs|ms|s) \([0-9]+ bodies per second\)"), "decoded in N (N bodies per second)"),
    (re.compile(r"\b([0-9]+ errors) in [0-9]+(\.[0-9]+)?(ns|µs|ms|s)$", re.M), r"\1 in N"),
]

# A failed build's sections of what it read from the jars (src/main.rs, `loaded_report`), which hold
# no signature row: each title, then rows of a label, two spaces or more and a figure.
LOADED_SECTIONS = {"classpath", "Java", "library bodies"}
LOADED_ROW = re.compile(r"^  \S.*?\S {2,}[0-9]")


def loaded_reports(lines):
    i = 0
    while i < len(lines):
        if lines[i] not in LOADED_SECTIONS:
            i += 1
            continue
        j = i + 1
        while j < len(lines) and (LOADED_ROW.match(lines[j]) or lines[j] in LOADED_SECTIONS and j + 1 < len(lines) and LOADED_ROW.match(lines[j + 1])):
            j += 1
        if j > i + 1:
            yield i, j
        i = j


def unreported(text):
    """The compiler's standard error, or a stream it shares, without the timing report, whose
    sections a forked build's has more of than one worker's without the fork and whose columns are
    aligned over all of them: the runs a signature row marks (`reports`) and a failed build's
    sections of what it read from the jars (`loaded_reports`)."""
    lines = text.split("\n")
    for find in (reports, loaded_reports):
        for i, j in reversed(list(find(lines))):
            del lines[i:j]
    return untimed_lines("\n".join(lines))


def untimed_lines(text):
    for pat, rep in DURATION:
        text = pat.sub(rep, text)
    return text


def denoise(text):
    for pat, rep in NOISE:
        text = pat.sub(rep, text)
    return text


def denoise_err(text, report=untimed):
    text = denoise(text)
    for pat, rep in REPORTS:
        text = pat.sub(rep, text)
    return report(text)


def read(path):
    try:
        with open(path, "rb") as f:
            return f.read().decode("utf-8", "replace")
    except OSError:
        return ""


def records(root, names_file):
    kept = []
    if os.path.isdir(root):
        for r in sorted(os.listdir(root)):
            d = os.path.join(root, r)
            args = read(os.path.join(d, "args")).splitlines()
            kept.append((denoise("\n".join(args)), args, d))
    for name in open(names_file):
        name = name.strip()
        if not name:
            continue
        key = os.path.basename(name)
        if key.endswith(".scala"):
            key = key[: -len(".scala")]
        part = re.compile(r"(^|/)" + re.escape(key) + r"($|/|\.)")
        mine = sorted(digest(shown, d) for (shown, args, d) in kept if any(part.search(a) for a in args))
        h = hashlib.sha256()
        for one in mine:
            h.update(one.encode())
        print(name, h.hexdigest()[:16] if mine else "-")


def digest(shown, d, report=untimed):
    """One invocation's arguments, its exit or signal and what it wrote to each stream."""
    h = hashlib.sha256()
    # A stream the run wrote both to (`2>&1`) holds the compiler's reports too (fork-one-teq.sh).
    out = read(os.path.join(d, "out"))
    if os.path.exists(os.path.join(d, "merged")):
        out = denoise_err(out, report)
    elif report is unreported:
        out = untimed_lines(denoise(out))
    else:
        out = denoise(out)
    for piece in (shown, read(os.path.join(d, "status")), out, denoise_err(read(os.path.join(d, "err")), report)):
        h.update(piece.encode())
        h.update(b"\0")
    return h.hexdigest()


def outcomes(root):
    """Each invocation under `root` by its directory and arguments, the outcomes of those that
    share them in order."""
    table = {}
    if os.path.isdir(root):
        for r in sorted(os.listdir(root)):
            d = os.path.join(root, r)
            shown = denoise(read(os.path.join(d, "cwd")).strip() + "\n" + read(os.path.join(d, "args")))
            table.setdefault(shown, []).append(digest(shown, d, unreported))
    return {k: sorted(v) for k, v in table.items()}


def outputs(skip_file, candidate, references):
    """The invocations under `candidate` whose outcome is no reference run's for the same
    directory and arguments (the timing report left out), or that no reference run made, and the
    reference's that the candidate did not make; those that name a test of the skip file (a
    failure the runner compares by its digest, a known one) are left out. One line each: its
    arguments."""
    skip = [os.path.basename(n.strip()).removesuffix(".scala") for n in open(skip_file) if n.strip()]
    parts = [re.compile(r"(^|/)" + re.escape(k) + r"($|/|\.)") for k in skip]
    cand = outcomes(candidate)
    refs = [outcomes(r) for r in references]
    for key in sorted(set(cand).union(*refs)):
        if all(r.get(key) != cand.get(key) for r in refs):
            args = key.split("\n")[1:]
            if not any(p.search(a) for p in parts for a in args):
                print(" ".join(a for a in args if a)[:200])


BUNDLES = re.compile(r"bundles: published ([^;]*); released ([^;]*); entered ([^;]*); read direct (\d+), cache (\d+); sealed (\d+); by the lock inside a hold (\d+), for an export (\d+); waited while withheld (\d+)")


def bundle_counts(line):
    """The cells of one join's or attempt's bundles' counts, or None without them."""
    m = BUNDLES.search(line)
    if not m:
        return None
    cells = {}
    for what, part in (("published", m.group(1)), ("released", m.group(2)), ("entered", m.group(3))):
        for item in part.split(", "):
            name, n = item.rsplit(" ", 1)
            cells[f"{what}.{name}"] = int(n)
    for name, i in (("read.direct", 4), ("read.cache", 5), ("sealed", 6), ("held", 7), ("exported", 8), ("waited.withheld", 9)):
        cells[name] = int(m.group(i))
    return cells


def bundles(log, wanted):
    cells = [c for c in wanted.split(",") if c]
    per = {}
    joined = set()
    problems = []
    for line in open(log, errors="replace"):
        m = JOIN.match(line) or GAVE.match(line)
        if not m:
            continue
        pid = m.group(1)
        if JOIN.match(line):
            joined.add(pid)
        counts = bundle_counts(line)
        if counts is None:
            problems.append(f"invocation {pid}: a join without the bundles' counts")
            continue
        unknown = [c for c in cells if c not in counts]
        if unknown:
            problems.append(f"no counter named {', '.join(unknown)}")
            break
        total = per.setdefault(pid, {})
        for k, v in counts.items():
            total[k] = total.get(k, 0) + v
    if not per and not problems:
        problems.append("no invocation joined a fork")
    for pid, total in per.items():
        zero = [c for c in cells if total.get(c, 0) == 0]
        print(f"invocation {pid}: " + ", ".join(f"{c} {total.get(c, 0)}" for c in cells))
        if pid not in joined:
            problems.append(f"invocation {pid}: gave way, no fork joined")
        elif zero:
            problems.append(f"invocation {pid}: {', '.join(zero)} at zero")
    for p in problems:
        print("FAIL " + p)
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    if len(sys.argv) == 5 and sys.argv[1] == "enforced":
        enforced(sys.argv[2], sys.argv[3], int(sys.argv[4]))
    elif len(sys.argv) == 4 and sys.argv[1] == "records":
        records(sys.argv[2], sys.argv[3])
    elif len(sys.argv) >= 5 and sys.argv[1] == "outputs":
        outputs(sys.argv[2], sys.argv[3], sys.argv[4:])
    elif len(sys.argv) == 4 and sys.argv[1] == "bundles":
        bundles(sys.argv[2], sys.argv[3])
    else:
        sys.exit("usage: fork_one.py enforced <log> <namespace> <workers> | records <dir> <names> | outputs <skip> <candidate> <reference>... | bundles <log> <cells>")
