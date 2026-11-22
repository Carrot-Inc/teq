#!/usr/bin/env python3
"""regressions.py <oracle dump> <base dump> <head dump>: the aligned lines whose positions equal scalac's in the base's
dump (master's products under the head's printer) and differ in the head's: what a change of the writer's positions
makes worse, the head's having no line aligned to it among them. Each dump is aligned to scalac's as compare.py
--positions aligns it (depth, kind, a definition's own name). Prints each such line and their count, and exits 1 when
there is one; exits 2, NOT JUDGED, where a dump is missing, empty, not what its sum says (another unit's, a line cut
off its end or out of its middle; the head's sealed as the head's) or holds its unit twice (tests/init-view/records.py),
or the three are of different units. `regressions.py --self-test` runs it on planted dumps (`self_test`)."""
import bisect
import collections
import os
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


def main(paths=None):
    import records as rec
    dumps = []
    for role, side, p in zip(("scalac's", "the base's", "the head's"), (None, None, "head"), paths or sys.argv[1:4]):
        lines, why = rec.dump(p, side=side)
        if why:
            print(f"NOT JUDGED: {role} dump {why}")
            return 2
        dumps.append(lines)
    # The three dumps are of one unit, its `unit` header the same: another unit's is no dump of this one.
    units = {next((l for l in d if l.startswith("unit ")), None) for d in dumps}
    if len(units) != 1:
        print("NOT JUDGED: the dumps are of different units")
        return 2
    o, base, head = ([x for x in d if not x.startswith("class ")] for d in dumps)
    on_base, pos = aligned(o, base)
    on_head, _ = aligned(o, head)
    # A line scalac's on the base is worse where the head places it otherwise or has no line aligned to it.
    worse = [i for i in sorted(on_base) if on_base[i] == pos[i] and on_head.get(i) != pos[i]]
    for i in worse:
        at = on_head.get(i)
        print(f"  scalac `{pos[i].strip()}` head `{at.strip() if at else 'none aligned'}`")
    print(f"{len(worse)} aligned lines equal to scalac's on the base and not on the head")
    return 1 if worse else 0


DUMP = ("unit src/cell.scala\nsource s1 name=cell.scala path=src/cell.scala content=yes sha256=0\n"
        "  PackageDef [s1 0..30@0 1:0-2:10] pid=<empty>\n"
        "    DefDef [s1 0..30@4 1:0-2:10] sym=<empty>.cell$package$.f\n"
        "      Apply [s1 10..30@12 1:10-2:10] fn=g\n"
        "        Ident [s1 10..11@10 1:10-1:11] sym=<empty>.cell$package$.g\n")


def self_test():
    """The comparison on planted dumps (`--self-test`): three equal dumps pass (exit 0); each of scalac's, the base's or
    the head's missing, empty, cut or holding its unit twice, and a head's of another unit, is not judged (exit 2); a
    line the head places otherwise, has another tree at, or has none of (its trees cut at a line, sealed as its
    producer wrote it) is worse (exit 1). Each dump changed after its producer sealed it (its last line gone with its
    line break kept, a line gone from its middle, a byte changed, its sum gone, another unit's dump copied in), and the
    base's dump copied over the head's, is not judged (exit 2) for what its sum says."""
    import shutil
    import records as rec
    t = rec.SelfTest("regressions.py")
    sides = {"scalac": None, "base": None, "head": "head"}

    def plant(d, files):
        paths = []
        for role in ("scalac", "base", "head"):
            p = os.path.join(d, role + ".dump")
            if files.get(role) is not None:
                rec.put(p, files[role], side=sides[role])
            paths.append(p)
        return paths

    def case(name, files, want, after=None, says=None):
        d = t.dir(name)
        paths = plant(os.path.join(d, "cell"), files)
        if after:
            # The same dumps of another unit, sealed by their producer.
            plant(os.path.join(d, "other"), files)
            after(paths)
        rc, out = t.quiet(main, paths)
        t.expect(name, rc == want and (says is None or any(w in out for w in says)),
                 f"exit {rc}: {out.strip().splitlines()[-1:] if out.strip() else ''}")

    good = {"scalac": DUMP, "base": DUMP, "head": DUMP}
    case("three equal dumps", good, 0)
    for k, role in enumerate(("scalac", "base", "head")):
        case(f"{role}'s dump missing", dict(good, **{role: None}), 2)
        case(f"{role}'s dump empty", dict(good, **{role: ""}), 2)
        case(f"{role}'s dump cut", good, 2, after=lambda ps, k=k: rec.write(ps[k], DUMP[: len(DUMP) // 2]))
        case(f"{role}'s dump holding its unit twice", dict(good, **{role: DUMP + DUMP}), 2)
        other = lambda p: p.replace(os.sep + "cell" + os.sep, os.sep + "other" + os.sep)
        for name, mutation in rec.after_the_producer(other):
            case(f"{role}'s dump: {name}", good, 2, after=lambda ps, k=k, mutation=mutation: mutation(ps[k]), says=rec.SUM_WHY)
    case("the base's dump copied over the head's (its output and sum)", good, 2,
         after=lambda ps: rec.copy_record(ps[1], ps[2]), says=rec.SUM_WHY)
    case("the head's dump of another unit", dict(good, head=DUMP.replace("src/cell.scala", "src/other.scala")), 2)
    case("a line the head places otherwise", dict(good, head=DUMP.replace("Ident [s1 10..11@10", "Ident [s1 10..11@11")), 1)
    case("a line the head has another tree at", dict(good, head=DUMP.replace("        Ident [s1", "        Select [s1")), 1)
    case("the head's trees cut at a line", dict(good, head="".join(DUMP.splitlines(keepends=True)[:-1])), 1)
    return t.finish()


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        sys.exit(self_test())
    sys.exit(main())
