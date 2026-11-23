#!/usr/bin/env python3
"""gate_rows.py <tests/gate.sh> <tests/gate-times.txt> | --self-test

Every line tests/gate.sh deals to the shards (its lines but the local half's, `local_lines`) has a row of its own in
tests/gate-times.txt: a suite run as pieces (`pieced`) a row for each piece or one for the suite. The dealer charges
a line without a row 120 s, whatever it takes: a suite of 523 s so dealt ended its shard 213 s past the others.
Prints each line without a row and exits 1; --self-test plants a gate.sh and rows and exits 1 on a miss.
"""
import re
import sys


def missing(gate_text, rows_text):
    rows = {l.split()[0] for l in rows_text.splitlines() if l.strip() and not l.startswith("#")}
    lines = re.findall(r'^  "([A-Za-z0-9._-]+) [^"]*"$', gate_text, re.M)
    m = re.search(r'^local_lines="([^"]*)"', gate_text, re.M)
    local = set(m.group(1).split()) if m else set()
    m = re.search(r'^pieced="([^"]*)"', gate_text, re.M)
    pieced = dict(p.split(":") for p in m.group(1).split()) if m else {}
    out = []
    for n in lines:
        if n in local:
            continue
        if n in pieced:
            out += [f"{n}.{k}/{pieced[n]}" for k in range(1, int(pieced[n]) + 1) if f"{n}.{k}/{pieced[n]}" not in rows and n not in rows]
        elif n not in rows:
            out.append(n)
    return out, len(lines)


def self_test():
    gate = 'local_lines="memory"\npieced="lsp:2"\nlines=(\n  "cases ./tests/run.sh"\n  "lsp ./tests/lsp.sh"\n  "memory ./tests/memory.sh"\n  "fresh ./tests/fresh.sh"\n)\n'
    checks = [
        ("every dealt line with a row, the local one without", "cases 1\nlsp.1/2 1\nlsp.2/2 1\nfresh 1\n", []),
        ("a new line without a row", "cases 1\nlsp.1/2 1\nlsp.2/2 1\n", ["fresh"]),
        ("a piece without a row", "cases 1\nlsp.1/2 1\nfresh 1\n", ["lsp.2/2"]),
        ("the suite's row for its pieces", "cases 1\nlsp 2\nfresh 1\n", []),
        ("a row in a comment is no row", "cases 1\nlsp 2\n# fresh 1\n", ["fresh"]),
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
        print(f"gate_rows: {m} is dealt to the shards and has no row in {sys.argv[2]}: write its seconds on eight CPUs (`{m} <seconds>`), and on sixteen (`{m}@16 <seconds>`) where they differ")
    print(f"gate_rows: {n} lines, {len(out)} dealt without a row")
    sys.exit(1 if out else 0)
