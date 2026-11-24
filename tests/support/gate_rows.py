#!/usr/bin/env python3
"""gate_rows.py <tests/gate.sh> <tests/gate-times.txt> | --self-test

Every line tests/gate.sh deals to the shards (its lines but the local half's, `local_lines`) has a row of its own in
tests/gate-times.txt: a suite run as pieces (`pieced`) a row for each piece or one for the suite. The dealer charges
a line without a row 120 s, whatever it takes: a suite of 523 s so dealt ended its shard 213 s past the others.

A named kind of machine (a word after `@`, the kind a machine's look names, GATE_KIND) is charged its own rows,
`<line>@<kind>`, else the row of eight over its speed, `speed@<kind>` (in percent of an eight-CPU machine's), so
a kind that has rows has its speed, or a row for every line dealt: a line of that kind with neither is reported as
missing. `-1` marks a line kept off a kind until its row there is measured, and is a kind's row alone; a speed is a
percent from 1 to 400.
Prints each line without a row and each malformed row, and exits 1; --self-test plants a gate.sh and rows and exits 1
on a miss.
"""
import re
import sys


def missing(gate_text, rows_text):
    rows = {}
    for l in rows_text.splitlines():
        if l.strip() and not l.startswith("#"):
            name, value = l.split()[:2]
            rows[name] = value
    lines = re.findall(r'^  "([A-Za-z0-9._-]+) [^"]*"$', gate_text, re.M)
    m = re.search(r'^local_lines="([^"]*)"', gate_text, re.M)
    local = set(m.group(1).split()) if m else set()
    m = re.search(r'^pieced="([^"]*)"', gate_text, re.M)
    pieced = dict(p.split(":") for p in m.group(1).split()) if m else {}
    out = []
    dealt = []
    for n in lines:
        if n in local:
            continue
        if n in pieced:
            dealt.append((n, [f"{n}.{k}/{pieced[n]}" for k in range(1, int(pieced[n]) + 1)]))
            out += [p for p in dealt[-1][1] if p not in rows and n not in rows]
        else:
            dealt.append((n, [n]))
            if n not in rows:
                out.append(n)
    kinds = {name.split("@", 1)[1] for name in rows if "@" in name and not name.split("@", 1)[1].isdigit()}
    for kind in sorted(kinds):
        speed = rows.get(f"speed@{kind}")
        if speed is not None and not (speed.isdigit() and 1 <= int(speed) <= 400):
            out.append(f"speed@{kind} (its value {speed}: a percent from 1 to 400)")
        if speed is not None:
            continue
        for n, parts in dealt:
            if f"{n}@{kind}" in rows:
                continue
            out += [f"{p}@{kind}" for p in parts if f"{p}@{kind}" not in rows]
    for name, value in rows.items():
        if value == "-1" and ("@" not in name or name.split("@", 1)[1].isdigit() or name.startswith(("speed@", "head-wait@"))):
            out.append(f"{name} (-1 marks a line of a named kind alone)")
    return out, len(lines)


def self_test():
    gate = 'local_lines="memory"\npieced="lsp:2"\nlines=(\n  "cases ./tests/run.sh"\n  "lsp ./tests/lsp.sh"\n  "memory ./tests/memory.sh"\n  "fresh ./tests/fresh.sh"\n)\n'
    every = "cases 1\nlsp.1/2 1\nlsp.2/2 1\nfresh 1\n"
    checks = [
        ("every dealt line with a row, the local one without", every, []),
        ("a new line without a row", "cases 1\nlsp.1/2 1\nlsp.2/2 1\n", ["fresh"]),
        ("a piece without a row", "cases 1\nlsp.1/2 1\nfresh 1\n", ["lsp.2/2"]),
        ("the suite's row for its pieces", "cases 1\nlsp 2\nfresh 1\n", []),
        ("a row in a comment is no row", "cases 1\nlsp 2\n# fresh 1\n", ["fresh"]),
        ("a named kind with its speed", every + "speed@guest 70\ncases@guest 3\n", []),
        ("a named kind with neither speed nor a row for a line", every + "cases@guest 3\nfresh@guest 2\n", ["lsp.1/2@guest", "lsp.2/2@guest"]),
        ("a named kind with every line's row", every + "cases@guest 3\nlsp@guest 2\nfresh@guest 2\n", []),
        ("a line kept off a kind until measured", every + "speed@guest 70\nfresh@guest -1\n", []),
        ("-1 on a row of eight", every.replace("fresh 1", "fresh -1"), ["fresh (-1 marks a line of a named kind alone)"]),
        ("-1 on a kind's speed", every + "speed@guest -1\n", ["speed@guest (its value -1: a percent from 1 to 400)", "speed@guest (-1 marks a line of a named kind alone)"]),
        ("a speed out of its range", every + "speed@guest 900\n", ["speed@guest (its value 900: a percent from 1 to 400)"]),
        ("a kind of CPUs is no named kind", every + "cases@16 1\n", []),
    ]
    bad = [what for what, rows, want in checks if missing(gate, rows)[0] != want]
    if missing("", "")[1] != 0 or missing(gate, "")[1] != 4:
        bad.append("the lines counted")
    for b in bad:
        print("gate_rows self-test FAIL:", b)
    print(f"gate_rows self-test: {len(checks) + 1} planted records, {len(bad)} not as they must be")
    return 1 if bad else 0


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        sys.exit(self_test())
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    out, n = missing(open(sys.argv[1]).read(), open(sys.argv[2]).read())
    if n == 0:
        print(f"gate_rows: no lines read from {sys.argv[1]}")
        sys.exit(1)
    for m in out:
        print(f"gate_rows: {m} is dealt to the shards and has no row in {sys.argv[2]}, or its row is malformed: write its seconds on eight CPUs (`<line> <seconds>`), on sixteen (`<line>@16 <seconds>`) where they differ, and a named kind's speed (`speed@<kind> <percent>`) or row")
    print(f"gate_rows: {n} lines, {len(out)} dealt without a row or malformed")
    sys.exit(1 if out else 0)
