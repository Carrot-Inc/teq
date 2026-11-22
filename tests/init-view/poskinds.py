#!/usr/bin/env python3
"""poskinds.py <tests/init-view dir> <unit dir>...: per unit dir (oracle/pickler.dump and <name>.dump files given by
POS_DUMP, default view.dump), the aligned lines (depth, kind, a definition's name: compare.py's alignment) whose position
differs from scalac's, counted by node kind. Prints the total, the units with any, and the kinds with an example."""
import collections, difflib, os, sys
sys.path.insert(0, sys.argv[1])
import compare as c

dump = os.environ.get("POS_DUMP", "view.dump")
# The reference: scalac's InitDump (oracle/pickler.dump), or with POS_ORACLE=scalac-view.dump the same view of scalac's
# own pickles, which leaves out what no pickle carries: the writer's own differences.
ref = os.environ.get("POS_ORACLE", "oracle/pickler.dump")
kinds = collections.Counter(); ex = {}; units = 0; total = 0; seen = 0
for d in sys.argv[2:]:
    o = os.path.join(d, ref); v = os.path.join(d, dump)
    if not (os.path.exists(o) and os.path.exists(v)) or os.path.getsize(v) == 0:
        continue
    seen += 1
    a = [x for x in open(o, encoding="utf-8").read().splitlines() if not x.startswith("class ")]
    b = [y for y in open(v, encoding="utf-8").read().splitlines() if not y.startswith("class ")]
    ha, hb = [c.head(x) for x in a], [c.head(y) for y in b]
    pa, pb = c.positions(a), c.positions(b)
    n = 0
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, ha, hb, autojunk=False).get_opcodes():
        if tag != "equal":
            continue
        for k in range(i2 - i1):
            if pa[i1 + k] != pb[j1 + k]:
                kind = a[i1 + k].strip().split(" ")[0]
                kinds[kind] += 1; n += 1
                ex.setdefault(kind, (os.path.basename(d.rstrip("/")), a[i1 + k].strip()[:100], b[j1 + k].strip()[:100]))
    total += n; units += n > 0
print(f"units read {seen}; aligned lines whose position differs: {total} in {units} units")
for k, v in kinds.most_common():
    u, x, y = ex[k]
    print(f"{v:6d} {k}   e.g. {u}\n         S {x}\n         T {y}")
