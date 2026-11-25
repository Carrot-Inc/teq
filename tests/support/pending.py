#!/usr/bin/env python3
"""The lists of the tests expected to fail, one per suite: tests/pending/<suite>.txt, a test a line, sorted, after
a tab the first line of its difference when the line was written (information: only the name is compared).

A run of a suite is held to its list: it fails when a listed test passes (the line goes: a list only shrinks)
and when an unlisted test fails; and to the suite's board list (below): without --update or --add, it fails when a
test it names (judged, skipped or held to the suite's own lists) has no row there, a test the tree gained that no
--update recorded, listed or not (`unrecorded`; a suite under --fails-only keeps no row of a pass and is not held to
it). Every runner calls this with what it ran and what failed:

  python3 tests/support/pending.py <suite> <results> [--update] [--add]

<results> has a line per test of the run: `pass <name>`, `fail <name>` and, after a tab, the first line of the
difference, `skip <name>` and, after a tab, why the run did not judge it (another system's, a jar not in the
cache), or `known <name>` and, after a tab, the first line of a failure the suite holds to a list of its own checks
(tests/modules/known.txt and deferred.txt) instead of this one. A test has one outcome: a failure, then a skip,
then a pass. A skipped test is not run: it is in no count of passes or failures, and its place in the list stands
(it is never fixed, never taken out). A `known` one is a failure on the board and in no count of passes, and this
list neither holds it nor takes it in. --update takes out of the list the tests that pass now, and
nothing else: a test that fails unlisted still fails the run. --add also writes the unlisted failures in, for
the part that brings the tests into the tree; it is never the answer to a test that stopped passing. --fails-only is for a
suite that names its failures alone (tests/support/pending.sh, said_fail): a listed check leaves the list only where
a line names it passing (`pass <name>`, said_pass); one named neither failing, skipped nor passing is unaccounted:
the run fails on it and no --update takes it out (an aggregate count names no check, and an empty or cut results
file is no evidence that anything ran); --passed=<n> is its count of the checks that passed, for the board. --partial is for one piece of a
suite run in pieces: the board's list keeps the rows of the tests the piece did not run.

Under --update or --add a run that is then the list's also writes tests/board/suites/<suite>.tsv, the suite's
row of the board (tests/board/board.py): test, verdict (pass, fail, cannot run for a skipped one), the first
line. A test skipped as another system's is `cannot run` with that reason: no other machine's verdict is copied
into this one's list. `passing <suite>` prints the tests that list
says pass, for the scripts that run over what a suite passes (tests/jvm-watch.sh); `listed <suite> <name>`
writes a new test into that list as passing, at its place, for the script that brings a case it has just run
(tests/prop-known.sh).

The exit status is 0 when the run is the list's, 1 when not, 2 for a command it cannot read.
"""
import os
import sys

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
DIR = os.path.join(REPO, "tests", "pending")


def clean(note):
    """A note as a list keeps it: one line, no control character (a program's output may hold any byte, and a
    list is read and compared as text)."""
    return " ".join("".join(c if c >= " " and c != "\x7f" else " " for c in str(note)).split())


def path(suite):
    return os.path.join(DIR, suite + ".txt")


def read(suite):
    """The suite's list: name -> note. A suite without a file expects no failure."""
    out = {}
    if os.path.exists(path(suite)):
        with open(path(suite), encoding="utf-8") as fh:
            for line in fh:
                line = line.rstrip("\n")
                if not line or line.startswith("#"):
                    continue
                name, _, note = line.partition("\t")
                out[name] = note
    return out


def write(suite, listed):
    os.makedirs(DIR, exist_ok=True)
    with open(path(suite), "w", encoding="utf-8") as fh:
        for name in sorted(listed):
            note = clean(listed[name])
            fh.write(name + ("\t" + note if note else "") + "\n")


def board_list(suite):
    return os.path.join(REPO, "tests", "board", "suites", suite + ".tsv")


def passing(suite):
    """The tests the suite's board list says pass, in its order."""
    out = []
    if os.path.exists(board_list(suite)):
        with open(board_list(suite), encoding="utf-8") as fh:
            for line in fh:
                parts = line.rstrip("\n").split("\t")
                if not line.startswith("#") and len(parts) > 1 and parts[1] == "pass":
                    out.append(parts[0])
    return out


def write_board_list(suite, ran, failing, skipped, passed=None, partial=False, known=None):
    """The suite's list for the board: every test of the run with its verdict, a skipped one `cannot run` with why
    it did not run (another system's included: no earlier machine's line is kept), a `known` one `fail`.
    `passed`: for a suite that names its failures alone, how many checks it counted as passed, kept on a line
    `# passed <n>` in the place of their rows."""
    known = known or {}
    old = {}
    if os.path.exists(board_list(suite)):
        with open(board_list(suite), encoding="utf-8") as fh:
            for line in fh:
                if not line.startswith("#") and line.strip():
                    old[line.split("\t")[0]] = line.rstrip("\n")
    # a piece of a suite keeps the rows of the tests the other pieces run
    rows = dict(old) if partial else {}
    for n in ran:
        note = clean(failing.get(n, ""))
        rows[n] = f"{n}\t{'fail' if n in failing else 'pass'}\t-\t{note}"
    for n, why in known.items():
        rows[n] = f"{n}\tfail\t-\t{clean(why)}"
    for n, why in skipped.items():
        rows[n] = f"{n}\tcannot run\t-\t{clean(why)}"
    os.makedirs(os.path.dirname(board_list(suite)), exist_ok=True)
    with open(board_list(suite), "w", encoding="utf-8") as fh:
        fh.write("# test\tverdict\tthe other ways\tthe first line of the difference\n")
        if passed is not None:
            fh.write(f"# passed\t{passed}\n")
        for n in sorted(rows):
            if passed is None or "\tpass\t" not in rows[n]:
                fh.write(rows[n] + "\n")


def hold(suite, ran, failing, update=False, add=False, out=sys.stdout, marks=0, known=(), skipped=0):
    """Holds a run to the suite's list. `ran`: the names the run judged; `failing`: of them, those that failed,
    each with the first line of its difference. Whether the run is the list's, after what --update and --add
    changed. `marks`: how many of those that did not fail are no pass either (the board's `marks` verdict), for
    the count the last line gives. `known`: the names whose failure the suite holds to a list of its own checks:
    no name of this list's (a listed one is fixed here, its failure now the other list's), no pass. `skipped`: how
    many tests did not run, counted apart."""
    listed = read(suite)
    fixed = sorted(n for n in listed if (n in ran and n not in failing) or n in known)
    new = sorted(n for n in failing if n not in listed)
    changed = False
    if update and fixed:
        for n in fixed:
            del listed[n]
        print(f"{suite}: {len(fixed)} pass now and left tests/pending/{suite}.txt: {' '.join(fixed[:20])}{' ...' if len(fixed) > 20 else ''}", file=out)
        fixed, changed = [], True
    if add and new:
        for n in new:
            listed[n] = failing[n]
        print(f"{suite}: {len(new)} failing tests written into tests/pending/{suite}.txt", file=out)
        new, changed = [], True
    if changed or ((update or add) and not os.path.exists(path(suite))):
        write(suite, listed)
    if fixed:
        print(f"{suite}: {len(fixed)} listed in tests/pending/{suite}.txt pass now (take them out: --update): {' '.join(fixed[:40])}{' ...' if len(fixed) > 40 else ''}", file=out)
    if new:
        print(f"{suite}: {len(new)} fail and are not in tests/pending/{suite}.txt:", file=out)
        for n in new[:40]:
            print(f"  {n}\t{' '.join(str(failing[n]).split())[:200]}", file=out)
        if len(new) > 40:
            print(f"  ... and {len(new) - 40} more", file=out)
    still = len([n for n in listed if n in failing])
    if not fixed and not new:
        print(f"{suite}: {len(ran) - len(failing) - marks} pass, " + (f"{marks} by marks alone, " if marks else "") + f"{still} fail as tests/pending/{suite}.txt lists"
              + (f", {len(known)} fail as the suite's own lists of checks hold them" if known else "")
              + (f", {skipped} not run" if skipped else ""), file=out)
    return not fixed and not new


def unrecorded(suite, names, out=sys.stdout):
    """The tests of a run with no row in the suite's board list (tests/board/suites/<suite>.tsv), printed with
    their names: a test the tree gained that no --update recorded, whether the pending list names it or not (only
    a run under --update or --add writes the board's rows). Whether every test of the run has its row."""
    rows = set()
    if os.path.exists(board_list(suite)):
        with open(board_list(suite), encoding="utf-8") as fh:
            rows = {l.split("\t")[0] for l in fh if l.strip() and not l.startswith("#")}
    missing = sorted(n for n in names if n not in rows)
    if missing:
        print(f"{suite}: no row in tests/board/suites/{suite}.tsv for {len(missing)} of the run's tests (record them: --update): {' '.join(missing[:40])}{' ...' if len(missing) > 40 else ''}", file=out)
    return not missing


def main(argv):
    args = [a for a in argv if not a.startswith("--")]
    flags = [a for a in argv if a.startswith("--")]
    if len(args) == 2 and args[0] == "passing" and not flags:
        print("\n".join(passing(args[1])))
        return 0
    if len(args) == 3 and args[0] == "listed" and not flags:
        suite, name = args[1], args[2]
        lines = []
        if os.path.exists(board_list(suite)):
            with open(board_list(suite), encoding="utf-8") as fh:
                lines = [l.rstrip("\n") for l in fh]
        head = [l for l in lines if l.startswith("#")]
        rows = {l.split("\t")[0]: l for l in lines if l and not l.startswith("#")}
        rows.setdefault(name, f"{name}\tpass\t-\t")
        os.makedirs(os.path.dirname(board_list(suite)), exist_ok=True)
        with open(board_list(suite), "w", encoding="utf-8") as fh:
            fh.write("\n".join(head + [rows[n] for n in sorted(rows)]) + "\n")
        return 0
    passed = next((int(f.split("=", 1)[1]) for f in flags if f.startswith("--passed=")), None)
    flags = [f for f in flags if not f.startswith("--passed=")]
    if len(args) != 2 or any(f not in ("--update", "--add", "--fails-only", "--partial") for f in flags):
        print(__doc__, file=sys.stderr)
        return 2
    suite, results = args
    ran, failing, skipped, known = set(), {}, {}, {}
    with open(results, encoding="utf-8", errors="replace") as fh:
        for line in fh:
            line = line.rstrip("\n")
            if not line:
                continue
            head, _, note = line.partition("\t")
            verdict, _, name = head.partition(" ")
            if verdict not in ("pass", "fail", "skip", "known") or not name:
                print(f"pending.py: a line that is none of `pass <name>`, `fail <name>`, `skip <name>`, `known <name>`: {line[:120]}", file=sys.stderr)
                return 2
            if verdict == "skip":
                skipped.setdefault(name, note)
            elif verdict == "known":
                known.setdefault(name, note)
            else:
                ran.add(name)
                if verdict == "fail":
                    failing.setdefault(name, note)
    # one outcome a test: a failure (this list's), then a failure the suite holds to its own lists, then a skip
    # (not run), then a pass
    for name in failing:
        known.pop(name, None)
        skipped.pop(name, None)
    for name in known:
        skipped.pop(name, None)
        ran.discard(name)
    for name in skipped:
        ran.discard(name)
    unaccounted = []
    if "--fails-only" in flags:
        # a suite that names its failures alone: a listed check leaves the list by a line naming it passing alone; one
        # named neither failing, skipped nor passing is unaccounted, the run's failure, kept on the list
        unaccounted = sorted(set(read(suite)) - ran - set(skipped) - set(known))
    update, add = "--update" in flags, "--add" in flags
    held = hold(suite, ran, failing, update, add, known=known, skipped=len(skipped))
    if "--fails-only" not in flags and not (update or add):
        held = unrecorded(suite, ran | set(failing) | set(skipped) | set(known)) and held
    if unaccounted:
        print(f"{suite}: {len(unaccounted)} listed in tests/pending/{suite}.txt named neither failing, skipped nor passing (kept: no line names its check passing): {' '.join(unaccounted[:40])}{' ...' if len(unaccounted) > 40 else ''}")
        held = False
    if held and (update or add):
        write_board_list(suite, ran, failing, skipped, passed, "--partial" in flags, known)
    return 0 if held else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
