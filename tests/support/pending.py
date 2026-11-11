#!/usr/bin/env python3
"""The lists of the tests expected to fail, one per suite: tests/pending/<suite>.txt, a test a line, sorted, after
a tab the first line of its difference when the line was written (information: only the name is compared).

A run of a suite is held to its list: it fails when a listed test passes (the line goes: a list only shrinks)
and when an unlisted test fails. Every runner calls this with what it ran and what failed:

  python3 tests/support/pending.py <suite> <results> [--update] [--add]

<results> has a line per test of the run: `pass <name>`, `fail <name>` and, after a tab, the first line of the
difference, or `skip <name>` and, after a tab, why the run did not judge it (another system's, a jar not in the
cache): a skipped test's place in the list stands. --update takes out of the list the tests that pass now, and
nothing else: a test that fails unlisted still fails the run. --add also writes the unlisted failures in, for
the part that brings the tests into the tree; it is never the answer to a test that stopped passing. --fails-only is for a
suite that names its failures alone (tests/support/pending.sh, said_fail): a listed check with no failure line
passes now; --passed=<n> is its count of the checks that passed, for the board. --partial is for one piece of a
suite run in pieces: the board's list keeps the rows of the tests the piece did not run.

Under --update or --add a run that is then the list's also writes tests/board/suites/<suite>.tsv, the suite's
row of the board (tests/board/board.py): test, verdict (pass, fail, cannot run for a skipped one), the first
line. A test skipped as another system's keeps the line it had. `passing <suite>` prints the tests that list
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


def write_board_list(suite, ran, failing, skipped, passed=None, partial=False):
    """The suite's list for the board: every test of the run with its verdict; one skipped as another system's
    keeps the line an earlier run on that system wrote. `passed`: for a suite that names its failures alone, how
    many checks it counted as passed, kept on a line `# passed <n>` in the place of their rows."""
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
    for n, why in skipped.items():
        if "another system" in why and n in old and "\tcannot run\t" not in old[n]:
            rows[n] = old[n]
        else:
            rows[n] = f"{n}\tcannot run\t-\t{clean(why)}"
    os.makedirs(os.path.dirname(board_list(suite)), exist_ok=True)
    with open(board_list(suite), "w", encoding="utf-8") as fh:
        fh.write("# test\tverdict\tthe other ways\tthe first line of the difference\n")
        if passed is not None:
            fh.write(f"# passed\t{passed}\n")
        for n in sorted(rows):
            if passed is None or "\tpass\t" not in rows[n]:
                fh.write(rows[n] + "\n")


def hold(suite, ran, failing, update=False, add=False, out=sys.stdout):
    """Holds a run to the suite's list. `ran`: the names the run judged; `failing`: of them, those that failed,
    each with the first line of its difference. Whether the run is the list's, after what --update and --add
    changed."""
    listed = read(suite)
    fixed = sorted(n for n in listed if n in ran and n not in failing)
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
        print(f"{suite}: {len(ran) - len(failing)} pass, {still} fail as tests/pending/{suite}.txt lists", file=out)
    return not fixed and not new


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
    ran, failing, skipped = set(), {}, {}
    with open(results, encoding="utf-8", errors="replace") as fh:
        for line in fh:
            line = line.rstrip("\n")
            if not line:
                continue
            head, _, note = line.partition("\t")
            verdict, _, name = head.partition(" ")
            if verdict not in ("pass", "fail", "skip") or not name:
                print(f"pending.py: a line that is none of `pass <name>`, `fail <name>`, `skip <name>`: {line[:120]}", file=sys.stderr)
                return 2
            if verdict == "skip":
                skipped[name] = note
                continue
            ran.add(name)
            if verdict == "fail":
                failing.setdefault(name, note)
    if "--fails-only" in flags:
        # a suite that names its failures alone: a listed check it named no failure for passes now
        ran |= set(read(suite))
    update, add = "--update" in flags, "--add" in flags
    held = hold(suite, ran, failing, update, add)
    if held and (update or add):
        write_board_list(suite, ran, failing, skipped, passed, "--partial" in flags)
    return 0 if held else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
