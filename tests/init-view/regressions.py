#!/usr/bin/env python3
"""regressions.py <oracle dump> <base dump> <head dump>: the aligned lines whose positions equal scalac's in the base's
dump (master's products under the head's printer) and differ in the head's: what a change of the writer's positions
makes worse. Each dump is aligned to scalac's as compare.py --positions aligns it (depth, kind, a definition's own
name). Prints each such line and their count, and exits 1 when there is one."""
import bisect
import collections
import sys

import compare as c


def keyed(h, lo, hi, by_occurrence):
    """The lines of a stretch as keys: a line itself, or with `by_occurrence` the line and how many like it come before
    it in the stretch (the n-th `Apply`), which makes every key unique."""
    if not by_occurrence:
        return [h[i] for i in range(lo, hi)]
    seen = collections.Counter()
    out = []
    for i in range(lo, hi):
        out.append((h[i], seen[h[i]]))
        seen[h[i]] += 1
    return out


def unique_anchors(ha, hb, a0, a1, b0, b1, by_occurrence=False):
    """The lines each stretch holds exactly once, in both, as a longest chain in order on both sides (patience
    sorting): the alignment's anchors. With `by_occurrence`, the n-th line of a kind on one side against the n-th
    on the other (`keyed`)."""
    ka, kb = keyed(ha, a0, a1, by_occurrence), keyed(hb, b0, b1, by_occurrence)
    ca, cb = collections.Counter(ka), collections.Counter(kb)
    at_b = {k: b0 + n for n, k in enumerate(kb) if cb[k] == 1}
    pairs = [(a0 + n, at_b[k]) for n, k in enumerate(ka) if ca[k] == 1 and k in at_b]
    tails, back, links = [], [], []
    for n, (i, j) in enumerate(pairs):
        k = bisect.bisect_left(tails, j)
        if k == len(tails):
            tails.append(j)
            back.append(n)
        else:
            tails[k] = j
            back[k] = n
        links.append(back[k - 1] if k else None)
    chain, n = [], back[-1] if back else None
    while n is not None:
        chain.append(pairs[n])
        n = links[n]
    return chain[::-1]


def matches(ha, hb):
    """The pairs (i, j) of lines of the two dumps aligned with each other, in order. The lines both hold once anchor the
    alignment, so that a dump of thousands of like lines (`Tpt`, `Ident` at one depth) costs about its length times
    its logarithm: difflib over the whole of two such dumps (its longest-match search with no junk heuristic) grows
    with the product of their lengths and more, hours on dotty's tests/pos/large2 (32,767 lines against 49,134 of 31
    kinds). A stretch between two anchors with no line it holds once aligns its lines by their order among their
    like (`keyed`'s occurrences)."""
    out = []
    stack = [(0, len(ha), 0, len(hb))]
    while stack:
        a0, a1, b0, b1 = stack.pop()
        while a0 < a1 and b0 < b1 and ha[a0] == hb[b0]:
            out.append((a0, b0))
            a0, b0 = a0 + 1, b0 + 1
        while a0 < a1 and b0 < b1 and ha[a1 - 1] == hb[b1 - 1]:
            a1, b1 = a1 - 1, b1 - 1
            out.append((a1, b1))
        if a0 >= a1 or b0 >= b1:
            continue
        anchors = unique_anchors(ha, hb, a0, a1, b0, b1) or unique_anchors(ha, hb, a0, a1, b0, b1, by_occurrence=True)
        if not anchors:
            continue
        pa, pb = a0, b0
        for i, j in anchors:
            out.append((i, j))
            stack.append((pa, i, pb, j))
            pa, pb = i + 1, j + 1
        stack.append((pa, a1, pb, b1))
    return sorted(out)


def aligned(a, b):
    """For each line of scalac's (`a`, its class lines left out), the position the other dump's aligned line has."""
    ha, hb = [c.head(x) for x in a], [c.head(y) for y in b]
    pa, pb = c.positions(a), c.positions(b)
    return {i: pb[j] for i, j in matches(ha, hb)}, pa


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
