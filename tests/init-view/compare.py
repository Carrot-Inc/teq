#!/usr/bin/env python3
"""compare.py [--positions] <oracle dump> <teq's dump>: whether teq's initialization view of a program is scalac's
(tests/init-view.sh). It fails closed: an empty dump on either side fails both judgments.

The two dumps are InitDump.scala's after `pickler` and `TEQ_INIT_VIEW=<root> teq tasty`. Without --positions they
agree when equal line for line. With --positions only the positions are judged: every line's depth, kind and position
(`[...]`), the `unit` and `source` lines whole, in order. The two are aligned (difflib, by depth, kind and a definition's
own name); an aligned line must have its partner's position, and a line of either dump with no partner fails the
judgment unless its hunk is a fact no pickle carries, by the one list of them, tests/init-view/oracle.py (a parameter
accessor's place, moved and not lost; a package clause's qualified id). Prints `equal` and exits 0, or the differing and
unpartnered lines, their counts and the first of each, and exits 1.

A line it prints holds no path of the machine, as the suites record its lines (tests/pending, tests/board/suites): a
path under the suite's root (this script's tree) is printed relative to it, one in coursier's cache by the fixed name
`<coursier cache>`, in both dumps' lines alike, so the same program prints the same line on every machine."""
import difflib
import os
import re
import sys

import oracle

POS = re.compile(r"^(\s*)(\S+) (\[[^\]]*\])")
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CACHE = re.compile(r"/\S*?/[Cc]oursier/v1/")


def placeless(line):
    """A dump's line as the suites print and record it: no path of the machine."""
    return CACHE.sub("<coursier cache>/", line.replace(ROOT + "/", ""))


def positions(lines):
    out = []
    for l in lines:
        if l.startswith("class "):
            continue
        if l.startswith("unit ") or l.startswith("source "):
            out.append(l)
            continue
        m = POS.match(l)
        out.append(m.group(0) if m else l)
    return out


DEFS = ("ValDef", "DefDef", "TypeDef", "Bind")
SYM = re.compile(r" sym=(\S+)")


def head(l):
    """A line's depth and kind (a unit or source line its first word), and a definition's name: what aligns the two
    dumps, so that a position never decides what stands against what and a definition stands against its own."""
    m = POS.match(l)
    if not m:
        return (0, l.split(" ", 1)[0], "")
    name = ""
    if m.group(2) in DEFS:
        s = SYM.search(l)
        if s:
            # The definition's own name, the owners' signatures left out (the two printers' owners differ in what they
            # resolve): a method's is before its own signature (`C.m#(scala.Int)scala.Int`), anything else's the last
            # component (`C.m#(1,Box)java.lang.Object.A`).
            sym = s.group(1)
            if m.group(2) == "DefDef" and "#" in sym:
                sym = sym.rsplit("#", 1)[0]
            name = sym.rsplit(".", 1)[-1]
            # A placeholder's parameter by its place, not its number: scalac numbers them from 1, teq from 0 (a name,
            # not a position).
            name = re.sub(r"^_\$\d+$", "_$", name)
    return (len(m.group(1)), m.group(2), name)


def main():
    args = sys.argv[1:]
    only_positions = bool(args) and args[0] == "--positions"
    if only_positions:
        args = args[1:]
    a = open(args[0], encoding="utf-8").read().splitlines()
    b = open(args[1], encoding="utf-8").read().splitlines()
    # An empty dump is no view and no oracle: a failure, never compared (fails closed).
    if not a or not b:
        print(f"empty dump: scalac's {len(a)} lines, teq's {len(b)}")
        return 1
    if not only_positions:
        if a == b:
            print("equal")
            return 0
        first = next((i for i, (x, y) in enumerate(zip(a, b)) if x != y), min(len(a), len(b)))
        sa = a[first] if first < len(a) else "<end>"
        sb = b[first] if first < len(b) else "<end>"
        print(f"first difference at line {first + 1}: scalac `{placeless(sa.strip())}` teq `{placeless(sb.strip())}`")
        return 1
    # Every line's depth, kind and position, in order. Aligned by depth, kind and a definition's name, so that a position
    # never decides what stands against what; a line of either dump with no partner fails the judgment unless its hunk is
    # a fact no pickle carries (tests/init-view/oracle.py, the one list by_kind.py files by too).
    ra, rb = [x for x in a if not x.startswith("class ")], [y for y in b if not y.startswith("class ")]
    ha, hb = [head(x) for x in ra], [head(y) for y in rb]
    pa, pb = positions(ra), positions(rb)
    sm = difflib.SequenceMatcher(None, ha, hb, autojunk=False)
    differ, hunks, excused = [], [], 0
    for tag, i1, i2, j1, j2 in sm.get_opcodes():
        if tag == "equal":
            for x, y in zip(pa[i1:i2], pb[j1:j2]):
                if x != y:
                    differ.append((x, y))
        else:
            hunks.append((ra[i1:i2], rb[j1:j2]))
    pos_of = dict(zip(ra, pa)) | dict(zip(rb, pb))
    failing = oracle.hunks_excused(hunks, key=lambda l: pos_of.get(l, l))
    excused = sum(len(h[0]) + len(h[1]) for h in hunks) - sum(len(f[0]) + len(f[1]) for f in failing)
    note = f"; {excused} lines excused as ORACLE hunks" if excused else ""
    if not differ and not failing:
        print("equal" + note)
        return 0
    parts = []
    if differ:
        x, y = differ[0]
        parts.append(f"{len(differ)} aligned lines' positions differ; first: scalac `{placeless(x.strip())}` teq `{placeless(y.strip())}`")
    if failing:
        fa, fb, why = failing[0]
        first = placeless((fa or fb)[0].strip()) if (fa or fb) else ""
        side = "scalac's" if fa else "teq's"
        parts.append(f"{sum(len(f[0]) + len(f[1]) for f in failing)} lines with no partner ({why}); first, {side}: `{first}`")
    print("; ".join(parts) + note)
    return 1


if __name__ == "__main__":
    sys.exit(main())
