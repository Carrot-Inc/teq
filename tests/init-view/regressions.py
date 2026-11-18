#!/usr/bin/env python3
"""regressions.py <oracle dump> <base dump> <head dump>: the aligned lines whose positions equal scalac's in the base's
dump (master's products under the head's printer) and differ in the head's: what a change of the writer's positions
makes worse. Each dump is aligned to scalac's as compare.py --positions aligns it (depth, kind, a definition's own
name). Prints each such line and their count, and exits 1 when there is one."""
import difflib
import sys

import compare as c


def aligned(a, b):
    """For each line of scalac's (`a`, its class lines left out), the position the other dump's aligned line has."""
    ha, hb = [c.head(x) for x in a], [c.head(y) for y in b]
    pa, pb = c.positions(a), c.positions(b)
    out = {}
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, ha, hb, autojunk=False).get_opcodes():
        if tag == "equal":
            for k in range(i2 - i1):
                out[i1 + k] = pb[j1 + k]
    return out, pa


def main():
    o, base, head = (open(p, encoding="utf-8").read().splitlines() for p in sys.argv[1:4])
    o, base, head = ([x for x in d if not x.startswith("class ")] for d in (o, base, head))
    on_base, pos = aligned(o, base)
    on_head, _ = aligned(o, head)
    worse = [i for i in sorted(on_base) if on_base[i] == pos[i] and i in on_head and on_head[i] != pos[i]]
    for i in worse:
        print(f"  scalac `{pos[i].strip()}` head `{on_head[i].strip()}`")
    print(f"{len(worse)} aligned lines equal to scalac's on the base and not on the head")
    return 1 if worse else 0


if __name__ == "__main__":
    sys.exit(main())
