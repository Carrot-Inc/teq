#!/usr/bin/env python3
"""tests/support/block_object_imports.py: the table of an IMPORT OVER A BLOCK's LOCAL OBJECT: an object local to a
block (`object h`, `case object h`, a given without parameters, which dotty makes one) is a value of its module class,
and an import over it brings that class's members as an import over a local class's instance does: its own, its
parents', its nested object's through a path, its types (dotty's import qualifier there is the local module's term).

A cell is one program in package `fd`: its library (lib/Lib.scala: `trait Base`) and its user (use/Main.scala,
`@main def run()`, the block and the object in it), run WHOLE and SPLIT by tests/support/pkgobj-given-import.sh. The
families:
  imports     where the block stands (the method's body, a nested block, a lambda's body, a local def's body) x the
              object's form (an object, an object with a parent, a given, a given with a parent, a case object) x the
              import (a wildcard, a selection, a renaming, `given`, a nested object's path, an exclusion, a type);
  selections  the same places and forms read by a selection, no import (`h.o.g`): the control;
  pickled     the block in a method of the library, read through the import in the block's result (an abstract type
              member, a lambda's parameter of it, and a member), whose TASTy writes the result before the object's val (`BLOCK expr stats`), the split's
              interpreter and JavaScript running the library's body from that pickle: an object, a given, an
              object nested in an object x a wildcard, a selection.
Each program prints what it reads: a def `f` (3), a val `v` (4), an object `o` with `g` (5), a type `T` (Int), a given
`gi` (`Ordering.Int.reverse`, `compare(1, 2)` is 1), a parent's `b` (6).

  block_object_imports.py cells <out dir>             the cells and cells.txt
  block_object_imports.py judge <dir> <base> <head>   <dir>/scalac.tsv against <base>.tsv and <head>.tsv: first the legs
                                                      the head accepts where scalac refuses, then right, turned,
                                                      regressed per family; exit 1 unless every leg is right on the
                                                      head and none is missing
  block_object_imports.py --self-test                 the templates and the judge over planted tables
A leg is right when it is scalac's output, or scalac's first error at its file, line and column in its words.
"""
import io
import os
import sys

BODY = ["def f: Int = 3", "val v: Int = 4", "object o:", "  def g: Int = 5", "type T = Int",
        "given gi: Ordering[Int] = Ordering.Int.reverse"]
HEADS = {"object": "object h:", "objectparent": "object h extends Base:", "given": "given h: AnyRef with",
         "givenparent": "given h: Base with", "caseobject": "case object h:"}
IMPORTS = {
    "wildcard": ("*", "f + v + o.g + (2: T)"),
    "selection": ("{f, v}", "f + v"),
    "renaming": ("{f as ff}", "ff"),
    "given": ("given", "summon[Ordering[Int]].compare(1, 2)"),
    "nested": (None, "g"),
    "exclusion": ("{f as _, *}", "v"),
    "types": ("{T}", "(2: T)"),
}
PLACES = ["method", "nested", "lambda", "def"]


def block(place, form, stmts):
    obj = [HEADS[form]] + ["  " + l for l in BODY]
    inner = obj + stmts
    if place == "method":
        return inner
    if place == "nested":
        return ["locally {"] + ["  " + l for l in inner] + ["}"]
    if place == "lambda":
        return ["val go = () =>"] + ["  " + l for l in inner] + ["go()"]
    return ["def go(): Unit ="] + ["  " + l for l in inner] + ["go()"]


def user(lines):
    return "package fd\n\n@main def run(): Unit =\n" + "".join("  " + l + "\n" for l in lines)


LIB = {"Lib.scala": "package fd\n\ntrait Base:\n  def b: Int = 6\n"}


PICKLED = {"object": (["object h:", "  type T <: String", "  def f: Int = 3"], "h"),
           "given": (["given h: AnyRef with", "  type T <: String", "  def f: Int = 3"], "h"),
           "nestedobject": (["object h:", "  object k:", "    type T <: String", "    def f: Int = 3"], "h.k")}


def pickled_cell(form, sel):
    obj, path = PICKLED[form]
    clause = f"import {path}.*" if sel == "wildcard" else f"import {path}.{{f, T}}"
    body = "".join("    " + l + "\n" for l in obj + [clause, "List(\"abc\").map(_.asInstanceOf[T]).map(t => t.length).head + f"])
    lib = {"Lib.scala": "package fd\n\nobject L:\n  def r(): Int =\n" + body}
    return lib, "package fd\n\n@main def run(): Unit =\n  println(L.r())\n"


def cells():
    out = []
    for form in PICKLED:
        for sel in ("wildcard", "selection"):
            l, u = pickled_cell(form, sel)
            out.append((f"p-{form}-{sel}", l, u))
    for place in PLACES:
        for form in HEADS:
            for imp, (sel, read) in IMPORTS.items():
                if imp == "wildcard" and form.endswith("parent"):
                    read += " + b"
                clause = "import h.o.*" if sel is None else f"import h.{sel}"
                out.append((f"i-{place}-{form}-{imp}", LIB, user(block(place, form, [clause, f"println({read})"]))))
            out.append((f"s-{place}-{form}", LIB, user(block(place, form, ["println(h.o.g)"]))))
    return out


def family(name):
    return {"i": "imports", "s": "selections", "p": "pickled"}[name[0]]


def write_cells(out):
    ns = []
    for name, l, u in cells():
        for f, text in l.items():
            p = os.path.join(out, name, "lib", f)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            open(p, "w").write(text)
        os.makedirs(os.path.join(out, name, "use"), exist_ok=True)
        open(os.path.join(out, name, "use", "Main.scala"), "w").write(u)
        ns.append(name)
    open(os.path.join(out, "cells.txt"), "w").write("\n".join(ns) + "\n")
    return ns


def read(path):
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
    if scalac.startswith("ok:"):
        return v == scalac
    return v.startswith("err:") and place(v) == place(scalac) and words(v) == words(scalac)


LEGS = ["interp", "jvm", "js"]


def judge(ns, scalac, base, head, out=sys.stdout):
    fams, accepted, missing, problems = {}, [], [], []
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
                if s.startswith("err:") and h.startswith("ok:"):
                    accepted.append(f"{c} {b} {leg}: {h}")
                hr, mr = right(s, h), right(s, m)
                if hr:
                    f["right"] += 1
                    f["turned"] += not mr
                else:
                    f["problem"] += 1
                    f["regressed"] += mr
                    problems.append(f"{'REGRESSED' if mr else 'PROBLEM'} {c} {b} {leg}: scalac {s} | head {h} | base {m}")
    print(f"accepted by the head and refused by scalac: {len(accepted)}", file=out)
    for a in accepted:
        print("  " + a, file=out)
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
    return 0 if not accepted and tot["problem"] == 0 and not missing else 1


def self_test():
    bad = []
    cs = cells()
    ns = [c[0] for c in cs]
    if len(set(ns)) != len(ns):
        bad.append("duplicate cell names")
    d = {c[0]: c for c in cs}
    u = d["i-method-object-nested"][2]
    if "  object h:\n    def f: Int = 3\n" not in u or "  import h.o.*\n  println(g)\n" not in u:
        bad.append("the method's object, its nested import")
    u = d["i-lambda-givenparent-wildcard"][2]
    if "  val go = () =>\n    given h: Base with\n" not in u or "    println(f + v + o.g + (2: T) + b)\n  go()\n" not in u:
        bad.append("the lambda's given with a parent")
    if "  locally {\n    case object h:\n" not in d["i-nested-caseobject-types"][2] or "    import h.{T}\n" not in d["i-nested-caseobject-types"][2]:
        bad.append("the nested block's case object, its type")
    if "println(h.o.g)" not in d["s-def-object"][2] or "import" in d["s-def-object"][2]:
        bad.append("the selection's control")
    l = d["p-nestedobject-selection"][1]["Lib.scala"]
    if "object L:\n  def r(): Int =\n    object h:\n      object k:\n" not in l or "    import h.k.{f, T}\n    List(\"abc\").map(_.asInstanceOf[T]).map(t => t.length).head + f\n" not in l:
        bad.append("the library's block, its nested object read through the import in the block's result")
    if "    given h: AnyRef with\n      type T <: String\n" not in d["p-given-wildcard"][1]["Lib.scala"]:
        bad.append("the library's block, its given")
    s = {("i-a", b, "run"): "ok:5" for b in ("whole", "split")}
    s.update({("s-e", b, "run"): "err:Main.scala:5:11: Not found: b" for b in ("whole", "split")})
    good = {("i-a", b, l): "ok:5" for b in ("whole", "split") for l in LEGS}
    good.update({("s-e", b, l): "err:Main.scala:5:11: Not found: b" for b in ("whole", "split") for l in LEGS})
    accepting = {**good, ("s-e", "split", "js"): "ok:5"}
    wrong = {**good, ("i-a", "whole", "jvm"): "err:-:-:-: Unit has no member o"}
    missing = {k: v for k, v in good.items() if k != ("i-a", "whole", "interp")}
    # A run that printed scalac's lines and then failed or stood at its bound: the runner's err and hang verdicts.
    trailing = {**good, ("i-a", "split", "jvm"): "err:-:-:-: Exception in thread \"main\" java.lang.RuntimeException"}
    timeout = {**good, ("i-a", "whole", "js"): "hang:60s"}
    null = open(os.devnull, "w")
    for what, base, head, code in (("all right", wrong, good, 0), ("a run failure", good, wrong, 1), ("an acceptance scalac refuses", good, accepting, 1),
                                   ("a missing leg", good, missing, 1), ("a trailing failure", good, trailing, 1), ("a timeout", good, timeout, 1)):
        if judge(["i-a", "s-e"], s, base, head, out=null) != code:
            bad.append(f"the judge on the planted table: {what}")
    o = io.StringIO()
    judge(["i-a", "s-e"], s, good, accepting, out=o)
    if not o.getvalue().startswith("accepted by the head and refused by scalac: 1\n"):
        bad.append("the judge's first line")
    if bad:
        sys.exit("block_object_imports self-test FAILED: " + "; ".join(bad))
    print(f"block_object_imports self-test: {len(cs)} cells, the templates checked, the judge's planted tables judged as they must be",
          file=sys.stdout if sys.argv[1:2] == ["--self-test"] else sys.stderr)


self_test()
cmd = sys.argv[1:2]
if cmd == ["cells"]:
    print(f"block_object_imports: {len(write_cells(sys.argv[2]))} cells in {sys.argv[2]}")
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
