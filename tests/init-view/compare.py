#!/usr/bin/env python3
"""compare.py [--positions] <oracle dump> <teq's dump>: whether teq's initialization view of a program is scalac's
(tests/init-view.sh).

The two dumps are InitDump.scala's after `pickler` and `TEQ_INIT_VIEW=<root> teq tasty`. Without --positions they
agree when equal line for line. With --positions only the positions are judged: the `unit` and `source` lines and, of
each tree's line, its depth, kind and position (`[...]`). The two are aligned (difflib), and the lines that stand
against a line of the same depth and kind must have its position; the lines of a tree whose shape differs (a line of
one side with none of the same depth and kind against it) are counted apart as SHAPE lines, which are not
positions. Prints `equal` (with the shape lines' count where there are any) and exits 0, or the aligned lines whose
positions differ, their count and the first, and exits 1."""
import difflib
import re
import sys

POS = re.compile(r"^(\s*)(\S+) (\[[^\]]*\])")


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
    if not only_positions:
        if a == b:
            print("equal")
            return 0
        first = next((i for i, (x, y) in enumerate(zip(a, b)) if x != y), min(len(a), len(b)))
        sa = a[first] if first < len(a) else "<end>"
        sb = b[first] if first < len(b) else "<end>"
        print(f"first difference at line {first + 1}: scalac `{sa.strip()}` teq `{sb.strip()}`")
        return 1
    # Aligned by depth, kind and a definition's name, so that a position never decides what stands against what.
    ha, hb = [head(x) for x in a if not x.startswith("class ")], [head(y) for y in b if not y.startswith("class ")]
    a, b = positions(a), positions(b)
    sm = difflib.SequenceMatcher(None, ha, hb, autojunk=False)
    differ, shape_a, shape_b = [], 0, 0
    for tag, i1, i2, j1, j2 in sm.get_opcodes():
        if tag == "equal":
            for x, y in zip(a[i1:i2], b[j1:j2]):
                if x != y:
                    differ.append((x, y))
        else:
            shape_a += i2 - i1
            shape_b += j2 - j1
    shape = f"; shape lines: {shape_a} scalac's, {shape_b} teq's" if shape_a or shape_b else ""
    if not differ:
        print("equal" + shape)
        return 0
    x, y = differ[0]
    print(f"{len(differ)} aligned lines' positions differ{shape}; first: scalac `{x.strip()}` teq `{y.strip()}`")
    return 1


if __name__ == "__main__":
    sys.exit(main())
