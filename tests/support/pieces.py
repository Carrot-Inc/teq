#!/usr/bin/env python3
# The pieces of the long suites (tests/lsp.sh, tests/jvm-watch.sh, tests/modules.sh, tests/split.sh), which the gate
# runs as pieces on several machines (docs/DEVELOPING.md, "The landing gate"); tests/support/pieces.sh is the
# suites' side.
#   pieces.py list <suite> <n>             every piece's units: a line `# <k>/<n>: <count> units, weight <w>` for each
#                                          piece, then `<k>/<n> <unit>` a line, each piece's units in the suite's order
#   pieces.py select <suite> <k> <n>       piece k's units, one a line, in the suite's order
#   pieces.py merge <suite> <n> [--results <file>] <log>...
#                                          the pieces' summaries merged into the suite's result (below)
#   pieces.py bounds <suite> <seconds> [--cpus=<n>]
#                                          each unit a run timed (`<unit> <seconds>` a line) against its bound, three
#                                          times its weight (below, seconds on sixteen CPUs) times 16 over the
#                                          machine's CPUs where it has fewer, and never under 180 s: a line `FAIL units: ...`
#                                          for each unit over it and exit 1, else one line and exit 0; for a suite whose
#                                          units run one after another (tests/gate-scripts.sh), so that a unit grown by
#                                          minutes fails at its own gate and not as the gate's last shard
# The units come on stdin, one a line, in the order the suite runs them (its inventory); a unit named twice, an
# empty inventory or a selector out of range is refused (exit 2). A unit's piece is a function of the units, n and
# the weights committed in tests/support/piece-weights.txt (`<suite> <unit> <seconds>`; `<suite> * <seconds>` the
# weight of a unit not listed there, 1 without one): the units heaviest first, by name on a tie, each to the piece
# with the least weight so far, the first of them on a tie, so that with no weights the names are dealt round in
# order. The gate's timing file places the pieces on machines and never chooses their units, and a unit added to a
# suite lands in exactly one piece without a weight of its own. PIECES_WEIGHTS names another file of weights (a cell's
# planted one).
#
# A piece's last line is its summary (pieces_end of tests/support/pieces.sh), and a whole run's too when
# PIECES_SUMMARY names a file for it:
#   pieces: <suite> <k>/<n> summary: exit=<code> units=<count> <field>=<value> ...
# with `whole` in the place of <k>/<n> for the whole run. A field `<name>=<integer>` is a counter, summed over the
# pieces; `and:<name>=ok|fail` a verdict of the whole suite's that each piece gives of its own units (split's modules
# that parse), one assertion of the suite's, passed when every piece's is ok; `any:<name>=0|1` a failure of the whole
# suite's that any piece may find (tests/modules.sh's scalac missing), one failure when one does; `pos:<name>=<integer>`
# evidence that must be there somewhere (the sessions that joined their workers in tests/lsp.sh), summed, the suite
# failing when the sum is 0; `note:<name>=<text>,<text>...` a note the suite's line carries, no verdict (a text's `_`
# read as spaces), the distinct texts of the pieces in their order. `merge` prints `<exit>\t<seconds>\t<line>`: the suite's verdict, its pieces' total time (with
# --results; 0 without) and its line, `<suite>: <n> pieces, <passed> passed, <failed> failed` and the other
# counters, then what failed. With --results (the gate's rows, `<name>\t<where>\t<seconds>\t<exit>\t<line>`) the
# pieces' rows are the verdicts: every piece `<suite>.<k>/<n>` has exactly one, and a row of the suite's under
# another denominator or past n fails it; without, each summary's exit is its piece's.
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def refuse(message):
    print(f"pieces: {message}", file=sys.stderr)
    sys.exit(2)


def inventory():
    units = [line.strip() for line in sys.stdin if line.strip()]
    if not units:
        refuse("the suite lists no units")
    seen = set()
    for unit in units:
        if unit in seen:
            refuse(f"the unit {unit} is listed twice")
        if any(c.isspace() for c in unit):
            refuse(f"the unit '{unit}' has a space")
        seen.add(unit)
    return units


# a unit's bound (bounds): this many times its weight, the weights being seconds on WEIGHT_CPUS, times WEIGHT_CPUS
# over a machine's CPUs where it has fewer (a gate's units ran up to 2.6 times their weights on eight), never under
# BOUND_FLOOR seconds
BOUND_TIMES, BOUND_FLOOR, WEIGHT_CPUS = 3, 180, 16


def weights(suite):
    table, default = {}, 1.0
    path = os.environ.get("PIECES_WEIGHTS") or os.path.join(HERE, "piece-weights.txt")
    for line in open(path) if os.path.exists(path) else []:
        fields = line.split()
        if not fields or fields[0].startswith("#") or fields[0] != suite:
            continue
        if fields[1] == "*":
            default = float(fields[2])
        else:
            table[fields[1]] = float(fields[2])
    return table, default


def count(text, what):
    if not re.fullmatch(r"[1-9][0-9]*", text):
        refuse(f"{what} {text!r} is not a whole number from 1")
    return int(text)


def deal(suite, units, n):
    """The piece (1..n) of each unit, and each piece's weight."""
    table, default = weights(suite)
    load = [0.0] * n
    piece = {}
    for unit in sorted(units, key=lambda u: (-table.get(u, default), u)):
        i = min(range(n), key=lambda i: load[i])
        piece[unit] = i + 1
        load[i] += table.get(unit, default)
    return piece, load


SUMMARY = re.compile(r"pieces: (\S+) (whole|(\d+)/(\d+)) summary: (.*)$")


def summary(log):
    """The fields of the log's last summary line, or None."""
    found = None
    try:
        for line in open(log, errors="replace"):
            m = SUMMARY.match(line.rstrip("\n"))
            if m:
                found = m
    except OSError:
        return None
    return found


def merge(suite, n, logs, results):
    problems = []
    seconds = 0
    verdicts = {}
    if results is not None:
        rows = {}
        for line in open(results):
            name, _where, secs, code, _ = (line.rstrip("\n").split("\t") + [""] * 5)[:5]
            if not name.startswith(suite + "."):
                continue
            rows.setdefault(name, []).append((secs, code))
        expected = {f"{suite}.{k}/{n}" for k in range(1, n + 1)}
        for name in sorted(set(rows) - expected):
            problems.append(f"an unexpected result {name}")
        for k in range(1, n + 1):
            got = rows.get(f"{suite}.{k}/{n}", [])
            if len(got) != 1:
                problems.append(f"piece {k}/{n}: {len(got)} results" if got else f"piece {k}/{n}: no result")
            for secs, code in got:
                seconds += int(secs) if secs.isdigit() else 0
            verdicts[k] = got[0][1] if len(got) == 1 else "1"
    counters, ands, anys, positives, notes = {}, {}, {}, {}, {}
    seen = {}
    for log in logs:
        m = summary(log)
        if m is None:
            continue
        if m.group(1) != suite:
            problems.append(f"{os.path.basename(log)} is a summary of {m.group(1)}")
            continue
        k, of = (1, 1) if m.group(2) == "whole" else (int(m.group(3)), int(m.group(4)))
        if of != n or not 1 <= k <= n:
            problems.append(f"{os.path.basename(log)} is the summary of piece {k}/{of}, not of one of {n}")
            continue
        if k in seen:
            problems.append(f"two summaries of piece {k}/{n}")
            continue
        seen[k] = log
        for field in m.group(5).split():
            name, _, value = field.partition("=")
            if name.startswith("and:"):
                ands.setdefault(name[4:], []).append(value)
            elif name.startswith("any:"):
                anys[name[4:]] = anys.get(name[4:], 0) + (value != "0")
            elif name.startswith("pos:"):
                positives[name[4:]] = positives.get(name[4:], 0) + int(value)
            elif name.startswith("note:"):
                notes.setdefault(name[5:], [])
                for text in value.replace("_", " ").split(","):
                    if text and text not in notes[name[5:]]:
                        notes[name[5:]].append(text)
            elif name == "exit":
                if results is None:
                    verdicts[k] = value
            else:
                counters[name] = counters.get(name, 0) + int(value)
    for k in range(1, n + 1):
        if k not in seen:
            problems.append(f"piece {k}/{n}: no summary")
            verdicts.setdefault(k, "1")
    failed_pieces = [f"{k}/{n}" for k in sorted(verdicts) if verdicts[k] != "0"]
    if failed_pieces:
        problems.append(f"piece{'s' if len(failed_pieces) > 1 else ''} {', '.join(failed_pieces)} failed")
    passed, failed = counters.pop("passed", 0), counters.pop("failed", 0)
    counters.pop("units", None)
    for name, values in ands.items():
        if all(v == "ok" for v in values):
            passed += 1
        else:
            failed += 1
            problems.append(f"{name} failed in {sum(v != 'ok' for v in values)} of {len(values)} pieces")
    for name, found in anys.items():
        if found:
            failed += 1
            problems.append(f"{name} in {found} pieces")
    for name, total in positives.items():
        if total <= 0:
            failed += 1
            problems.append(f"no {name} in any piece")
    line = f"{suite}: {n} pieces, {passed} passed, {failed} failed"
    line += "".join(f", {name} {value}" for name, value in counters.items())
    line += "".join(f"; {name}: {', '.join(texts)}" for name, texts in notes.items())
    if problems:
        line += "; " + "; ".join(problems)
    return (1 if problems or failed else 0), seconds, line


def main(argv):
    if len(argv) >= 3 and argv[0] == "list":
        suite, n = argv[1], count(argv[2], "the number of pieces")
        units = inventory()
        piece, load = deal(suite, units, n)
        for k in range(1, n + 1):
            mine = [u for u in units if piece[u] == k]
            print(f"# {k}/{n}: {len(mine)} units, weight {load[k - 1]:g}")
            for unit in mine:
                print(f"{k}/{n} {unit}")
    elif len(argv) == 4 and argv[0] == "select":
        suite, k, n = argv[1], count(argv[2], "the piece"), count(argv[3], "the number of pieces")
        if k > n:
            refuse(f"no piece {k} of {n}")
        units = inventory()
        piece, _ = deal(suite, units, n)
        for unit in units:
            if piece[unit] == k:
                print(unit)
    elif len(argv) >= 3 and argv[0] == "merge":
        suite, n = argv[1], count(argv[2], "the number of pieces")
        rest = argv[3:]
        results = None
        if rest[:1] == ["--results"]:
            results, rest = rest[1], rest[2:]
        code, seconds, line = merge(suite, n, rest, results)
        print(f"{code}\t{seconds}\t{line}")
    elif len(argv) in (3, 4) and argv[0] == "bounds" and (len(argv) == 3 or re.fullmatch(r"--cpus=[1-9][0-9]*", argv[3])):
        suite, path = argv[1], argv[2]
        cpus = int(argv[3].split("=")[1]) if len(argv) == 4 else WEIGHT_CPUS
        factor = max(1.0, WEIGHT_CPUS / cpus)
        machine = f", times {factor:g} on this machine's {cpus} CPUs" if factor > 1 else ""
        table, default = weights(suite)
        timed = [(l.split()[0], int(l.split()[1])) for l in open(path, encoding="utf-8") if len(l.split()) == 2]
        over = []
        for unit, secs in timed:
            weight = table.get(unit, default)
            bound = max(BOUND_FLOOR, round(BOUND_TIMES * weight * factor))
            if secs > bound:
                over.append(f"FAIL units: {unit} ran {secs} s, over its bound of {bound} s ({BOUND_TIMES} times its weight of "
                            f"{weight:g} s in tests/support/piece-weights.txt{machine}, {BOUND_FLOOR} s at least)")
        print("\n".join(over) or f"units: {len(timed)} timed, each within its bound ({BOUND_TIMES} times its weight{machine}, {BOUND_FLOOR} s at least)")
        sys.exit(1 if over else 0)
    else:
        refuse("usage: pieces.py list <suite> <n> | select <suite> <k> <n> | merge <suite> <n> [--results <file>] <log>... | bounds <suite> <seconds> [--cpus=<n>]")


if __name__ == "__main__":
    main(sys.argv[1:])
