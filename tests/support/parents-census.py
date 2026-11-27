#!/usr/bin/env python3
"""tests/support/parents-census.py <teq> <products dir>...: the classes of teq's products whose first parent the writer
states as `Object()` while the class's superclass is another class, which the parents' normalisation must state
in full or refuse: each template of the pickles (`teq tasty --trees`) whose first parent is a call of
`<init>(): java.lang.Object`, and one of whose later parents is a trait whose own template, in the same products,
begins with a class other than Object, AnyRef or Any (a trait's first parent is its class parent, a type). Prints a
row a site (`<products dir> <tasty> <class> <trait> <its class>`), then a line a products dir with its count, then the
total. A trait found in no pickle given (a library's) counts as Object's: the census is a floor, which does not see a
class over a library trait whose class parent is a class (`scala.util.control.NoStackTrace`'s Throwable: an exception's
`Quiet extends NoStackTrace`)."""
import os, re, subprocess, sys

LINE = re.compile(r"^ *(\d+):( *)(\S.*)$")
OBJECTS = {"Object", "AnyRef", "Any"}


def nodes(teq, path):
    out = subprocess.run([teq, "tasty", "--trees", path], capture_output=True, text=True, timeout=60).stdout
    rows, started = [], False
    for line in out.split("\n"):
        if line.startswith("Trees ("):
            started = True
            continue
        m = LINE.match(line) if started else None
        if m:
            rows.append((len(m.group(2)), m.group(3), int(m.group(1))))
        elif started and rows:
            break
    return rows


def tag(text):
    return text.split(" ")[0].split("(")[0]


def name(text):
    m = re.search(r"\[([^\]]*)\]", text)
    return m.group(1) if m else ""


def templates(rows):
    """Each template: its definition's name, and its parents as (tag, the name they mention first)."""
    out = []
    at = {r[2]: j for j, r in enumerate(rows)}

    def named(k):
        # the first name the subtree at k mentions, a share followed to the tree it shares
        for j in range(k, len(rows)):
            if j > k and rows[j][0] <= rows[k][0]:
                break
            t = rows[j][1]
            if name(t):
                return name(t)
            m = re.match(r"^SHARED\w*\s+(\d+)", t)
            if m and int(m.group(1)) in at:
                return named(at[int(m.group(1))])
        return ""

    for i, (depth, text, _) in enumerate(rows):
        if tag(text) != "TEMPLATE":
            continue
        owner = next((name(rows[j][1]) for j in range(i - 1, -1, -1) if tag(rows[j][1]) == "TYPEDEF" and rows[j][0] < depth), "?")
        child = depth + 2
        parents, k = [], i + 1
        while k < len(rows) and rows[k][0] > depth:
            d, t, _ = rows[k]
            if d == child:
                if tag(t) in ("DEFDEF", "VALDEF", "TYPEDEF", "SELFDEF"):
                    break
                if not tag(t).startswith("TYPEPARAM") and not tag(t).startswith("PARAM"):
                    parents.append((tag(t), named(k), k))
            k += 1
        out.append((owner, parents))
    return out


def main():
    teq, dirs = sys.argv[1], sys.argv[2:]
    total = total_traits = 0
    for d in dirs:
        tastys = sorted(os.path.join(dp, f) for dp, _, fs in os.walk(d) for f in fs if f.endswith(".tasty"))
        decoded = {t: templates(nodes(teq, t)) for t in tastys}
        # A trait's class parent: its template's first parent, a type, by the trait's simple name.
        trait_parents = {}
        for ts in decoded.values():
            for owner, parents in ts:
                if parents and parents[0][0] != "APPLY":
                    trait_parents.setdefault(owner, [p[1] for p in parents])
        # A trait's class parent: its first parent where that is a class, else the class parent of a trait among its
        # other parents (teq writes `Object` first for a trait whose class parent comes through another trait).
        def class_of(trait, seen=()):
            ps = trait_parents.get(trait)
            if not ps or trait in seen:
                return None
            if ps[0] not in OBJECTS and ps[0] not in trait_parents:
                return ps[0]
            for p in ps:
                c = class_of(p, seen + (trait,))
                if c:
                    return c
            return None
        class_parent = {t: class_of(t) for t in trait_parents}
        traits = 0
        for t, ts in decoded.items():
            for owner, parents in ts:
                if parents and parents[0][0] != "APPLY" and parents[0][1] in OBJECTS and class_parent.get(owner):
                    print(f"{d} {os.path.relpath(t, d)} {owner} trait-with-Object-first {class_parent[owner]}")
                    traits += 1
        count = 0
        for t, ts in decoded.items():
            for owner, parents in ts:
                if not parents or parents[0][0] != "APPLY" or "java.lang.Object" not in parents[0][1] and parents[0][1] != "<init>(): java.lang.Object":
                    continue
                for _, trait, _ in parents[1:]:
                    cls = class_parent.get(trait)
                    if cls and cls not in OBJECTS:
                        print(f"{d} {os.path.relpath(t, d)} {owner} class-with-Object-first {trait} {cls}")
                        count += 1
                        break
        print(f"census {d}: {count} classes with `Object()` first over a class parent, {traits} traits with `Object` first over one")
        total += count
        total_traits += traits
    print(f"census total, a floor: {total} classes and {total_traits} traits over {len(dirs)} products directories;"
          " a trait no pickle given holds counts as Object's, so a class over a library trait whose class parent is a"
          " class (`Quiet extends NoStackTrace`, an exception) is not seen")


main()
