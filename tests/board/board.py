#!/usr/bin/env python3
"""The board: one row per suite and per reference project, read from what the tree records. It runs nothing.

  python3 tests/board/board.py [--tsv]

  dotty's suites at 3.8.4   tests/board/dotty/<row>.tsv, written by tests/board/dotty.py --update: a row per
                            directory dotty's own runner compiles or runs (tests/board/rows.py)
  the suites of this tree   tests/board/suites/<suite>.tsv, written by the suite's runner under --update
                            (tests/support/pending.py --list): the tests of tests/cases and the others
  the branch's scenarios    tests/board/copied.tsv: the fixtures of the stopped branch whose checks are lines of
                            its runner scripts, not ported yet (a count of files, no verdict)
  the reference projects    bench/<project>/errors.txt, deviations.json and run.json, the gate's records
                            (docs/DEVELOPING.md, "The project records"), with the oracle's inventory tests.json

Columns: total, passing, by marks alone (a neg or warn test of dotty's that holds its marked lines where the row's
oracle of scalac's own diagnostics gives no answer to hold its diagnostics to: no pass and no failure,
tests/board/dotty.py), failing (a failure, a limit reached, a crash), cannot run, and a note. A project's rows
are its three lines: compile (the configurations of the record's scope; failing, those with an accepted error),
test (the oracle's cases; failing, those of a project blocked by its build and those whose outcome departs; cannot
run, those the oracle's own run did not run, Ignored, Skipped, Pending or Canceled, which no run of teq's passes
either; a pass, a case the oracle ran that teq's run ran as it did) and run (the named assertions; failing, those
violated). A file tests/board/projects/<project>-<line>.today, where
there is one, adds what a judge run of master gave beside the record (items outside the record, items gone).
"""
import collections
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
FAILING = ("fail", "timeout", "crash")
# The statuses of an oracle's case that say it did not run (sbt.testing.Status; the sbt inventories' keys are the
# lowercase words, tests/support's and bench/reference's writers): no pass and no failure, `cannot run`.
NOT_RUN = ("ignored", "skipped", "pending", "canceled")


def read_list(path):
    """A list's verdicts: (counts, the other ways' counts)."""
    counts, others = collections.Counter(), collections.Counter()
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            if line.startswith("# passed\t"):
                # a suite that names its failures alone: its count of the checks that passed
                counts["pass"] += int(line.split("\t")[1])
            if line.startswith("#") or not line.strip():
                continue
            parts = line.rstrip("\n").split("\t")
            counts[parts[1]] += 1
            if len(parts) > 2 and parts[2] not in ("", "-"):
                others[parts[2]] += 1
    return counts, others


def list_row(name, path, note=""):
    counts, others = read_list(path)
    total = sum(counts.values())
    failing = sum(counts[k] for k in FAILING)
    extra = []
    if counts["timeout"] or counts["crash"]:
        extra.append(", ".join(f"{counts[k]} {k}" for k in ("timeout", "crash") if counts[k]))
    way = collections.Counter()
    for o, n in others.items():
        w, _, v = o.partition(":")
        if v == "pass":
            way[w] += n
    extra += [f"{w}: {n} pass" for w, n in sorted(way.items())]
    return (name, total, counts["pass"], counts["marks"], failing, counts["cannot run"], "; ".join([note] * bool(note) + extra))


def dotty_rows():
    sys.path.insert(0, HERE)
    sys.dont_write_bytecode = True
    import rows as ROWS
    out = []
    for r in ROWS.rows():
        path = os.path.join(HERE, "dotty", r.name + ".tsv")
        if not os.path.exists(path):
            out.append((f"dotty {r.name}", "-", "-", "-", "-", "-", "no list: tests/board/dotty.py --row " + r.name + " --update --add"))
            continue
        out.append(list_row(f"dotty {r.name}", path, f"cannot run: {r.cannot}" if r.cannot else ""))
    return out


def suite_rows():
    out = []
    d = os.path.join(HERE, "suites")
    for f in sorted(os.listdir(d)) if os.path.isdir(d) else []:
        if f.endswith(".tsv"):
            out.append(list_row(f[:-4], os.path.join(d, f)))
    copied = os.path.join(HERE, "copied.tsv")
    if os.path.exists(copied):
        with open(copied, encoding="utf-8") as fh:
            for line in fh:
                if line.startswith("#") or not line.strip():
                    continue
                name, files, note = (line.rstrip("\n").split("\t") + ["", ""])[:3]
                out.append((name, files + " files", "-", "-", "-", "-", "the branch's checks are not ported: " + note))
    return out


def project_rows():
    out = []
    bench = os.path.join(REPO, "bench")
    for p in sorted(os.listdir(bench)):
        here = os.path.join(bench, p)
        if not os.path.exists(os.path.join(here, "pin.json")):
            continue
        today = lambda line: read_today(p, line)
        errors = os.path.join(here, "errors.txt")
        if os.path.exists(errors):
            keys, items, revision = [], collections.Counter(), ""
            with open(errors, encoding="utf-8") as fh:
                for line in fh:
                    if line.startswith("key "):
                        keys.append(line[4:].strip())
                    elif line.startswith("error "):
                        items[line.split(" ")[1]] += 1
                    elif line.startswith("revision "):
                        revision = line.split()[1]
            failing = len([k for k in keys if items[k]])
            n = sum(items.values())
            note = f"{n} accepted {'error' if n == 1 else 'errors'} in the record of {revision}" + today("compile")
            out.append((f"{p} compile", len(keys), len(keys) - failing, 0, failing, 0, note))
        tests, deviations = os.path.join(here, "tests.json"), os.path.join(here, "deviations.json")
        if os.path.exists(tests) and os.path.exists(deviations):
            with open(tests, encoding="utf-8") as fh:
                t = json.load(fh)
            with open(deviations, encoding="utf-8") as fh:
                d = json.load(fh)
            # every case of the inventory by its project, and those the oracle's run did not run: (project, name)
            cases, unrun = collections.Counter(), []
            if "suites" in t:
                for s in t["suites"].values():
                    for status, names in s.items():
                        if status == "project":
                            continue
                        cases[s["project"]] += len(names)
                        unrun += [(s["project"], n) for n in names if status.lower() in NOT_RUN]
            else:
                for m, v in t["modules"].items():
                    for status, names in v["cases"].items():
                        cases[m] += len(names)
                        unrun += [(m, n) for n in names if status.lower() in NOT_RUN]
            blocked, departing, heads = set(), 0, []
            for item in d["deviations"]:
                head = item.split("\n")[0]
                project, _, rest = head.partition(": ")
                if rest.startswith("blocked by") and project in cases:
                    blocked.add(project)
                elif project in cases and project not in blocked and rest.count(": ") >= 2:
                    departing += 1
                    heads.append((project, rest))
            # a case the oracle did not run is not run on any line, unless teq's run ran it otherwise (a departure
            # that names it: then it is that failure)
            # (bench/reference/reference.scala's compare: `<suite>: case "<name>" ...`, the name whole and quoted)
            unrun = [(pr, n) for pr, n in unrun if not any(pr == hp and f'case "{n}"' in rest for hp, rest in heads)]
            not_run = len(unrun)
            # the inventory's own count is the total: a case it holds twice is two
            total = t.get("cases_count", sum(cases.values()))
            failing = total - not_run if blocked and all(p_ in blocked for p_ in cases if cases[p_]) else \
                min(total - not_run, sum(cases[b] for b in blocked) - sum(pr in blocked for pr, _ in unrun) + departing)
            note = (f"{len(blocked)} of {len(d['projects'])} projects blocked by their build" if blocked else "") + today("test")
            out.append((f"{p} test", total, total - failing - not_run, 0, failing, not_run, note))
        run = os.path.join(here, "run.json")
        if os.path.exists(run):
            with open(run, encoding="utf-8") as fh:
                r = json.load(fh)
            violated = {a for a in r["assertions"] if any(v == a or v.startswith(a + ":") for v in r["violations"])}
            blocked = any(v.startswith("blocked:") for v in r["violations"])
            out.append((f"{p} run", len(r["assertions"]), len(r["assertions"]) - len(violated), 0, len(violated), 0,
                        ("blocked by its build" if blocked else "") + today("run")))
    return out


def read_today(project, line):
    path = os.path.join(HERE, "projects", f"{project}-{line}.today")
    if not os.path.exists(path):
        return ""
    with open(path, encoding="utf-8") as fh:
        first = next((l.strip() for l in fh if l.strip() and not l.startswith("#")), "")
    return "; today: " + first if first else ""


def dotty_header():
    """What dotty's lists were written under: the corpus, the JDK and node (a test dotty's own filter keeps to a
    JDK follows the machine's)."""
    import re
    seen = set()
    d = os.path.join(HERE, "dotty")
    for f in sorted(os.listdir(d)) if os.path.isdir(d) else []:
        if not f.endswith(".tsv"):
            continue  # a row's oracle stands beside its list
        with open(os.path.join(d, f), encoding="utf-8") as fh:
            m = re.match(r"# (dotty \S+ \(\w+\)).*; (JDK \d+, node \d+)", fh.readline())
            if m:
                seen.add(f"{m.group(1)}, {m.group(2)}")
    return "dotty's suites: " + "; ".join(sorted(seen)) if seen else ""


def self_checks_header():
    """scalac's checks of its own trees and output that dotty's rows drop (tests/board/dotty.py SELF_CHECKS), with
    how many tests carry each (tests/board/dotty/self-checks.tsv): named once, no verdict of a test moved by them."""
    path = os.path.join(HERE, "dotty", "self-checks.tsv")
    if not os.path.exists(path):
        return ""
    with open(path, encoding="utf-8") as fh:
        rows = [l.rstrip("\n").split("\t") for l in fh if l.strip() and not l.startswith("#")]
    return ("dotty's rows drop scalac's checks of its own trees and output, for which no check of teq's stands: "
            + ", ".join(f"{o} ({int(n):,} tests)" for o, n in rows))


def main():
    if "--tsv" not in sys.argv and dotty_header():
        print(dotty_header())
    if "--tsv" not in sys.argv and self_checks_header():
        print(self_checks_header())
    rows = [("suite", "total", "pass", "marks", "fail", "cannot run", "note")]
    rows += dotty_rows() + suite_rows() + project_rows()
    if "--tsv" in sys.argv:
        for r in rows:
            print("\t".join(str(c) for c in r))
        return 0
    widths = [max(len(str(r[i])) for r in rows) for i in range(6)]
    for i, r in enumerate(rows):
        print("  ".join(str(c).ljust(widths[j]) if j == 0 else str(c).rjust(widths[j]) for j, c in enumerate(r[:6])) + ("  " + str(r[6]) if r[6] else ""))
        if i == 0:
            print("  ".join("-" * w for w in widths))
    return 0


if __name__ == "__main__":
    sys.exit(main())
