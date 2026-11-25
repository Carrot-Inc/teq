#!/usr/bin/env python3
"""tests/support/singleton_cycles.py: the table of a CYCLE OF SINGLETON TYPES (`val a: b.type = b; val b: a.type = a`)
and of the programs that come near one and must still compile. dotty types a singleton type's path as an expression,
which reads its term's info (`Typer.typedSingletonTypeTree`, `checkStable`); a term met again while its own declared
type is being completed is a `CyclicReference`: "Cyclic reference involving val a" ("method a" for a def), at the
reference that closes the cycle, naming the member whose completion began first.

A cell is one program in package `fd`: its library (lib/*.scala) and its user (use/Main.scala, `@main def run()`). Each
runs WHOLE (lib and use compiled together) and SPLIT (lib compiled first, the user against its products), by
tests/support/pkgobj-given-import.sh's runner, scalac 3.8.4 by tests/support/scalac_cells.scala. The families:
  cycles  the HOLDER (an object, a package's top level, a class, a trait an object extends, a block) x the CYCLE: one
          member naming itself, two and three members of one kind (val, lazy val, given, def), two of mixed kinds in
          both orders, three of mixed kinds, seventeen vals;
  entries a cycle reached otherwise: the given cycle a summon searches (`import O.given`, by name too), a use before
          it in an earlier object (of vals, of givens, of a val and a given), two or three objects' vals and givens,
          two files, a val naming a cycle's later member (of vals, of a val and a given, of givens), an earlier
          object's val naming a later object's given cycle, the user naming it, a val before a val and given cycle, a
          type alias naming it after it and before it, an export clause and a parent's type argument naming it in an
          object before it and after it, a summon over it in an object before it (in its file and in an earlier one);
  edges   VALID programs that come near: chains that do not close (lengths 2, 3, 17 and 40 of vals, lazy vals,
          givens; each names the next, and backwards), a chain of two in every holder, an object's val of the
          object's own type, `val me: this.type = this` in a class and a trait, a path through the val's owner
          (`val y: O.x.type = x`), two objects naming each other's vals with no cycle of types, a lazy val and a
          given naming a later one, a summon over a given naming another, a block's val naming an earlier one; and
          what scalac says of a block's forward reference, a def's singleton and an inferred val's.

  singleton_cycles.py cells <out dir>               the cells (<cell>/lib/Lib.scala, <cell>/use/Main.scala) and
                                                    cells.txt
  singleton_cycles.py judge <dir> <base> <head>     <dir>/scalac.tsv against <base>.tsv and <head>.tsv: first the
                                                    legs the head refuses where scalac compiles, then the hangs, then
                                                    right, turned, regressed per family; exit 1 unless every leg is
                                                    right on the head and none is missing
  singleton_cycles.py --self-test                   the templates and the judge over planted tables (run first by
                                                    every command)
A leg is right when it is scalac's output, or scalac's first error at its file, line and column in its words. A run
the runner's bound stopped is `hang:`.
"""
import os
import sys

KINDS = ["val", "lazy", "given", "def"]


def member(kind, name, target, ty=None):
    ty = ty or f"{target}.type"
    return {"val": f"val {name}: {ty} = {target}", "lazy": f"lazy val {name}: {ty} = {target}",
            "given": f"given {name}: {ty} = {target}", "def": f"def {name}: {ty} = {target}"}[kind]


def held(holder, lines, result="1"):
    """The library's file: `lines` in the holder (a block's result `result`)."""
    if holder == "top":
        return "package fd\n\n" + "".join(l + "\n" for l in lines)
    body = "".join("  " + l + "\n" for l in lines)
    return {"object": "package fd\n\nobject O:\n" + body,
            "class": "package fd\n\nclass C:\n" + body,
            "trait": "package fd\n\ntrait T:\n" + body + "\nobject O extends T\n",
            "block": "package fd\n\ndef f: Int =\n" + body + f"  {result}\n"}[holder]


def user(lines):
    return "package fd\n\n@main def run(): Unit =\n" + "".join("  " + l + "\n" for l in lines)


def ref(holder, name):
    """The user's reading of the holder's member `name` (a block's result is `f`)."""
    return {"object": f"O.{name}", "top": name, "class": f"C().{name}", "trait": f"O.{name}", "block": "f"}[holder]


def names(n):
    return ["a", "b", "c"][:n] if n <= 3 else [f"a{i}" for i in range(1, n + 1)]


CYCLES = ([[k] for k in KINDS] + [[k, k] for k in KINDS]
          + [["val", "given"], ["given", "val"], ["val", "lazy"], ["lazy", "given"], ["def", "val"], ["val", "def"],
             ["given", "def"], ["lazy", "def"]]
          + [[k, k, k] for k in KINDS] + [["val", "given", "lazy"], ["def", "val", "given"]] + [["val"] * 17])


def cycle_lines(kinds):
    ns = names(len(kinds))
    return [member(k, ns[i], ns[(i + 1) % len(ns)]) for i, k in enumerate(kinds)]


def cycle_name(kinds):
    return "val17" if len(kinds) == 17 else "-".join(kinds)


def chain_lines(kind, n, backwards=False):
    """n links of `kind` ending in an Int: a1 names a2 ... a<n> names a<n+1> = 1 (backwards: a<i+1> names a<i>)."""
    ns = [f"a{i}" for i in range(1, n + 2)]
    if not backwards:
        return [member(kind, ns[i], ns[i + 1]) for i in range(n)] + [member(kind, ns[n], "1", "Int")]
    return [member(kind, ns[0], "1", "Int")] + [member(kind, ns[i], ns[i - 1]) for i in range(1, n + 1)]


def cells():
    out = []
    for h in ["object", "top", "class", "trait", "block"]:
        for kinds in CYCLES:
            out.append((f"c-{h}-{cycle_name(kinds)}", {"Lib.scala": held(h, cycle_lines(kinds))}, user(["println(2)"])))
    two = cycle_lines(["given", "given"]) + ["given c: Int = 2"]
    out += [
        ("e-summon", {"Lib.scala": held("object", two)}, user(["import O.given", "println(summon[Int])"])),
        ("e-summon-named", {"Lib.scala": held("object", two)}, user(["import O.c", "println(summon[Int])"])),
        ("e-usefirst", {"Lib.scala": "package fd\n\nobject U:\n  def u = O.b\n" + held("object", cycle_lines(["val", "val"]))[len("package fd\n\n"):]},
         user(["println(2)"])),
        ("e-objects", {"Lib.scala": "package fd\n\nobject A:\n  val x: B.y.type = B.y\nobject B:\n  val y: A.x.type = A.x\n"}, user(["println(2)"])),
        ("e-objects-given", {"Lib.scala": "package fd\n\nobject A:\n  given x: B.y.type = B.y\nobject B:\n  given y: A.x.type = A.x\n"}, user(["println(2)"])),
        ("e-objects3", {"Lib.scala": "package fd\n\nobject A:\n  val x: B.y.type = B.y\nobject B:\n  val y: C.z.type = C.z\nobject C:\n  val z: A.x.type = A.x\n"},
         user(["println(2)"])),
        ("e-files", {"Lib.scala": "package fd\n\nobject A:\n  val x: B.y.type = B.y\n", "Lib2.scala": "package fd\n\nobject B:\n  val y: A.x.type = A.x\n"},
         user(["println(2)"])),
        ("e-into", {"Lib.scala": held("object", ["val x: a.type = a"] + cycle_lines(["val", "val"]))}, user(["println(2)"])),
        ("e-user", {"Lib.scala": held("object", cycle_lines(["val", "val"]))}, user(["println(O.a)"])),
        ("e-before", {"Lib.scala": held("object", ["val z: Int = 0"] + cycle_lines(["val", "given"]))}, user(["println(2)"])),
        ("e-usefirst-given", {"Lib.scala": "package fd\n\nobject U:\n  def u = O.b\n" + held("object", cycle_lines(["given", "given"]))[len("package fd\n\n"):]},
         user(["println(2)"])),
        ("e-usefirst-mixed", {"Lib.scala": "package fd\n\nobject U:\n  def u = O.b\n" + held("object", cycle_lines(["val", "given"]))[len("package fd\n\n"):]},
         user(["println(2)"])),
        ("e-into-given", {"Lib.scala": held("object", ["val x: b.type = b"] + cycle_lines(["val", "given"]))}, user(["println(2)"])),
        ("e-into-givens", {"Lib.scala": held("object", ["val x: b.type = b"] + cycle_lines(["given", "given"]))}, user(["println(2)"])),
        ("e-later-object", {"Lib.scala": "package fd\n\nobject A:\n  val x: B.b.type = B.b\nobject B:\n  given a: b.type = b\n  given b: a.type = a\n"},
         user(["println(2)"])),
        ("e-later-object-mixed", {"Lib.scala": "package fd\n\nobject A:\n  val x: B.b.type = B.b\nobject B:\n  val a: b.type = b\n  given b: a.type = a\n"},
         user(["println(2)"])),
        ("e-alias", {"Lib.scala": held("object", cycle_lines(["val", "val"]) + ["type T = b.type"])}, user(["println(2)"])),
        ("e-alias-first", {"Lib.scala": held("object", ["type T = b.type"] + cycle_lines(["val", "val"]))}, user(["println(2)"])),
        ("e-export-first", {"Lib.scala": "package fd\n\nobject E:\n  export O.b\n" + held("object", cycle_lines(["val", "val"]))[len("package fd\n\n"):]},
         user(["println(2)"])),
        ("e-export-after", {"Lib.scala": held("object", cycle_lines(["val", "val"])) + "object E:\n  export O.b\n"}, user(["println(2)"])),
        ("e-parent-first", {"Lib.scala": "package fd\n\nclass K[T]\nobject P extends K[O.b.type]\n" + held("object", cycle_lines(["val", "val"]))[len("package fd\n\n"):]},
         user(["println(2)"])),
        ("e-parent-after", {"Lib.scala": held("object", cycle_lines(["val", "val"])) + "class K[T]\nobject P extends K[O.b.type]\n"}, user(["println(2)"])),
        ("e-summon-first", {"Lib.scala": "package fd\n\nobject U:\n  import O.given\n  def u: Int = summon[Int]\n" + held("object", two)[len("package fd\n\n"):]},
         user(["println(2)"])),
        ("e-summon-first-file", {"A.scala": "package fd\n\nobject U:\n  import O.given\n  def u: Int = summon[Int]\n", "Lib.scala": held("object", two)},
         user(["println(2)"])),
    ]
    for k in ["val", "lazy", "given"]:
        for n in [2, 3, 17, 40]:
            out.append((f"v-chain-{k}{n}", {"Lib.scala": held("object", chain_lines(k, n))}, user(["println(O.a1)"])))
        out.append((f"v-back-{k}3", {"Lib.scala": held("object", chain_lines(k, 3, backwards=True))}, user(["println(O.a4)"])))
    for h in ["top", "class", "trait", "block"]:
        out.append((f"v-chain-{h}-val2", {"Lib.scala": held(h, chain_lines("val", 2), "a1")}, user([f"println({ref(h, 'a1')})"])))
    for h in ["top", "class"]:
        out.append((f"v-chain-{h}-given2", {"Lib.scala": held(h, chain_lines("given", 2))}, user([f"println({ref(h, 'a1')})"])))
    out += [
        ("v-self", {"Lib.scala": held("object", ["val self: O.type = O"])}, user(["println(O.self eq O)"])),
        ("v-this-class", {"Lib.scala": held("class", ["val me: this.type = this"])}, user(["val c = C()", "println(c.me eq c)"])),
        ("v-this-trait", {"Lib.scala": "package fd\n\ntrait T:\n  val me: this.type = this\nclass C extends T\n"}, user(["val c = C()", "println(c.me eq c)"])),
        ("v-owner", {"Lib.scala": held("object", ["val x = 1", "val y: O.x.type = x"])}, user(["println(O.y)"])),
        ("v-objects", {"Lib.scala": "package fd\n\nobject A:\n  val x: B.y.type = B.y\nobject B:\n  val y: Int = 1\n"}, user(["println(A.x)"])),
        ("v-objects-mutual", {"Lib.scala": "package fd\n\nobject A:\n  val x: B.y.type = B.y\n  val z: Int = 2\nobject B:\n  val y: A.z.type = A.z\n"},
         user(["println(A.x)"])),
        ("v-lazy-later", {"Lib.scala": held("object", ["lazy val a: b.type = b", "val b: Int = 1"])}, user(["println(O.a)"])),
        ("v-given-later", {"Lib.scala": held("object", ["given a: b.type = b", "given b: Int = 1"])}, user(["println(O.a)"])),
        ("v-given-later-summon", {"Lib.scala": held("object", ["given a: b.type = b", "given b: Int = 1"])}, user(["import O.given", "println(summon[Int])"])),
        ("v-block-earlier", {"Lib.scala": held("block", ["val a: Int = 1", "val b: a.type = a"], "b")}, user(["println(f)"])),
        ("v-inferred", {"Lib.scala": held("object", ["val a = 1", "val b: a.type = a"])}, user(["println(O.b)"])),
        ("s-block-forward", {"Lib.scala": held("block", ["val a: b.type = b", "val b: Int = 1"], "a")}, user(["println(f)"])),
        ("s-def-path", {"Lib.scala": held("object", ["def a: Int = 1", "val b: a.type = a"])}, user(["println(O.b)"])),
    ]
    return out


def family(name):
    return {"c": "cycles", "e": "entries", "v": "edges", "s": "scalac's say"}[name[0]]


def write_cells(out):
    ns = []
    for name, lib, use in cells():
        for f, text in lib.items():
            p = os.path.join(out, name, "lib", f)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            open(p, "w").write(text)
        os.makedirs(os.path.join(out, name, "use"), exist_ok=True)
        open(os.path.join(out, name, "use", "Main.scala"), "w").write(use)
        ns.append(name)
    open(os.path.join(out, "cells.txt"), "w").write("\n".join(ns) + "\n")
    return ns


def read(path):
    """{(cell, build, leg): verdict}, and the malformed lines."""
    rows, bad = {}, []
    if not os.path.exists(path):
        return rows, [f"{path}: none"]
    for n, line in enumerate(open(path), 1):
        f = line.rstrip("\n").split("\t")
        if len(f) != 4 or f[1] not in ("whole", "split") or not f[3].startswith(("ok:", "err:", "hang:")):
            bad.append(n)
            continue
        rows[(f[0], f[1], f[2])] = f[3]
    return rows, bad


def place(v):
    head = v[4:].split(": ", 1)[0]
    return head if head.count(":") == 2 else head + "?"


def words(v):
    return v[4:].split(": ", 1)[1] if ": " in v[4:] else ""


def right(scalac, v):
    """scalac's output, or scalac's first error at its place in its words."""
    if scalac.startswith("ok:"):
        return v == scalac
    return v.startswith("err:") and place(v) == place(scalac) and words(v) == words(scalac)


LEGS = ["interp", "jvm", "js"]


def judge(ns, scalac, base, head, out=sys.stdout):
    fams = {}
    refused, hangs, base_hangs, missing = [], [], 0, []
    problems = []
    for c in ns:
        f = fams.setdefault(family(c), {"legs": 0, "right": 0, "turned": 0, "problem": 0, "regressed": 0})
        for b in ("whole", "split"):
            s = scalac.get((c, b, "run"))
            if s is None:
                missing.append(f"scalac {c} {b}")
                continue
            for leg in LEGS:
                h, m = head.get((c, b, leg)), base.get((c, b, leg))
                if h is None or m is None:
                    missing.append(f"{'head' if h is None else 'base'} {c} {b} {leg}")
                    continue
                f["legs"] += 1
                if s.startswith("ok:") and not h.startswith("ok:"):
                    refused.append(f"{c} {b} {leg}: {h}")
                if h.startswith("hang:"):
                    hangs.append(f"{c} {b} {leg}")
                base_hangs += m.startswith("hang:")
                hr, mr = right(s, h), right(s, m)
                if hr:
                    f["right"] += 1
                    f["turned"] += not mr
                else:
                    f["problem"] += 1
                    f["regressed"] += mr
                    problems.append(f"{'REGRESSED' if mr else 'PROBLEM'} {c} {b} {leg}: scalac {s} | head {h} | base {m}")
    print(f"refused by the head and compiled by scalac: {len(refused)}", file=out)
    for r in refused:
        print("  " + r, file=out)
    print(f"hangs: {len(hangs)} on the head ({base_hangs} on the base)", file=out)
    for r in hangs:
        print("  " + r, file=out)
    legs = sum(f["legs"] for f in fams.values())
    tot = {k: sum(f[k] for f in fams.values()) for k in ("right", "turned", "problem", "regressed")}
    print(f"{len(ns)} cells, {legs} legs (whole and split x interp, jvm, js): {tot['right']} right on the head "
          f"({tot['turned']} turned, {tot['right'] - tot['turned']} right on the base too), {tot['problem']} problems "
          f"({tot['regressed']} regressed), {len(missing)} missing", file=out)
    for name, f in fams.items():
        print(f"  {name}: {f['legs']} legs, {f['right']} right ({f['turned']} turned), {f['problem']} problems "
              f"({f['regressed']} regressed)", file=out)
    for p in problems:
        print("  " + p, file=out)
    for m in missing:
        print("  MISSING " + m, file=out)
    return 0 if not refused and not hangs and tot["problem"] == 0 and not missing else 1


def self_test():
    bad = []
    cs = cells()
    ns = [c[0] for c in cs]
    if len(set(ns)) != len(ns):
        bad.append("duplicate cell names")
    d = {c[0]: c for c in cs}
    if d["c-object-val-val"][1]["Lib.scala"] != "package fd\n\nobject O:\n  val a: b.type = b\n  val b: a.type = a\n":
        bad.append("the object's two vals")
    if d["c-top-lazy"][1]["Lib.scala"] != "package fd\n\nlazy val a: a.type = a\n":
        bad.append("the top level's lazy val naming itself")
    if "def f: Int =\n  def a: b.type = b\n  val b: c.type = c\n  given c: a.type = a\n  1\n" not in d["c-block-def-val-given"][1]["Lib.scala"]:
        bad.append("the block's mixed three")
    if "trait T:\n  given a: b.type = b\n  given b: a.type = a\n\nobject O extends T\n" not in d["c-trait-given-given"][1]["Lib.scala"]:
        bad.append("the trait's givens")
    if "  val a17: a1.type = a1\n" not in d["c-class-val17"][1]["Lib.scala"]:
        bad.append("the seventeen's closing link")
    lib = d["v-chain-given40"][1]["Lib.scala"]
    if "  given a40: a41.type = a41\n  given a41: Int = 1\n" not in lib or lib.count("given ") != 41:
        bad.append("the given chain of 40")
    if "  lazy val a1: Int = 1\n  lazy val a2: a1.type = a1\n" not in d["v-back-lazy3"][1]["Lib.scala"]:
        bad.append("the backward chain")
    if not d["v-chain-block-val2"][1]["Lib.scala"].endswith("  val a3: Int = 1\n  a1\n") or "println(f)" not in d["v-chain-block-val2"][2]:
        bad.append("the block chain's result")
    if "object U:\n  def u = O.b\nobject O:\n  val a: b.type = b\n" not in d["e-usefirst"][1]["Lib.scala"]:
        bad.append("the use before the cycle")
    if d["e-later-object"][1]["Lib.scala"] != "package fd\n\nobject A:\n  val x: B.b.type = B.b\nobject B:\n  given a: b.type = b\n  given b: a.type = a\n":
        bad.append("the earlier object's val into a given cycle")
    if not d["e-alias-first"][1]["Lib.scala"].startswith("package fd\n\nobject O:\n  type T = b.type\n  val a: b.type = b\n"):
        bad.append("the alias before the cycle")
    if d["e-parent-first"][1]["Lib.scala"] != "package fd\n\nclass K[T]\nobject P extends K[O.b.type]\nobject O:\n  val a: b.type = b\n  val b: a.type = a\n":
        bad.append("the parent's type argument before the cycle")
    # The judge over planted tables.
    s = {("v-a", b, "run"): "ok:1" for b in ("whole", "split")}
    s.update({("c-e", b, "run"): "err:Lib.scala:5:10: Cyclic reference involving val a" for b in ("whole", "split")})
    good = {("v-a", b, l): "ok:1" for b in ("whole", "split") for l in LEGS}
    good.update({("c-e", b, l): "err:Lib.scala:5:10: Cyclic reference involving val a" for b in ("whole", "split") for l in LEGS})
    hung = {**good, ("c-e", "whole", "jvm"): "hang:120s"}
    refusing = {**good, ("v-a", "split", "js"): "err:Lib.scala:4:10: Cyclic reference involving val a"}
    misplaced = {**good, ("c-e", "whole", "jvm"): "err:Lib.scala:4:10: Cyclic reference involving val b"}
    worded = {**good, ("c-e", "split", "interp"): "err:Lib.scala:5:10: recursive value a needs type"}
    missing = {k: v for k, v in good.items() if k != ("v-a", "whole", "interp")}
    null = open(os.devnull, "w")
    for what, base, head, code in (("all right", hung, good, 0), ("a hang", good, hung, 1), ("a refusal of a valid leg", good, refusing, 1),
                                   ("an error's place", good, misplaced, 1), ("an error's words", good, worded, 1),
                                   ("a missing leg", good, missing, 1)):
        if judge(["v-a", "c-e"], s, base, head, out=null) != code:
            bad.append(f"the judge on the planted table: {what}")
    import io
    o = io.StringIO()
    judge(["v-a", "c-e"], s, good, hung, out=o)
    if not o.getvalue().startswith("refused by the head and compiled by scalac: 0\nhangs: 1 on the head (0 on the base)\n"):
        bad.append("the judge's first two lines")
    o = io.StringIO()
    judge(["v-a", "c-e"], s, good, refusing, out=o)
    if not o.getvalue().startswith("refused by the head and compiled by scalac: 1\n"):
        bad.append("the refusal's count")
    rows, malformed = read(os.devnull)
    if rows or malformed:
        bad.append("an empty table read")
    if bad:
        sys.exit("singleton_cycles self-test FAILED: " + "; ".join(bad))
    # The table's own lines come first: under another command the self-test says it passed on stderr.
    print(f"singleton_cycles self-test: {len(cs)} cells, the templates checked, the judge's planted tables judged as they must be",
          file=sys.stdout if sys.argv[1:2] == ["--self-test"] else sys.stderr)


self_test()
cmd = sys.argv[1:2]
if cmd == ["cells"]:
    print(f"singleton_cycles: {len(write_cells(sys.argv[2]))} cells in {sys.argv[2]}")
elif cmd == ["judge"]:
    d = sys.argv[2]
    ns = [l.strip() for l in open(os.path.join(d, "cells.txt")) if l.strip()]
    tables, malformed = [], []
    for t in ("scalac", sys.argv[3], sys.argv[4]):
        rows, bad = read(os.path.join(d, t + ".tsv"))
        tables.append(rows)
        malformed += [f"{t}: {b}" for b in bad]
    code = judge(ns, *tables)
    if malformed:
        print(f"  MALFORMED lines {malformed[:5]}")
        code = 1
    sys.exit(code)
elif cmd != ["--self-test"]:
    sys.exit(__doc__)
