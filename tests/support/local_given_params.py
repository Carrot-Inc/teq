#!/usr/bin/env python3
"""tests/support/local_given_params.py: the table of a LOCAL GIVEN THAT TAKES PARAMETERS, a given alias with a using
clause or type parameters defined in a block, which dotty's `Parsers.givenDef` makes a method (`hasParams`: a
`DefDef`, called where the search finds it).

A cell is one program: a KIND of local given (an alias with a using clause, with a defaulted using clause, with a type
parameter, an inline one, a transparent inline one, the structural form `given g(using ..): Show with`, an anonymous
one) in a CONTEXT whose block defines it (a method's body, a lambda's, a val's initialiser, a class's initialiser
block, a block nested in a method's, a match case's body), found HOW (summoned; as the implicit argument of a call;
through another local given's using clause; at two types, the generic kind alone; shadowing a given of the same type
the file defines outside; beside a second local candidate of its type, which is scalac's ambiguity error). Each cell
prints what is found, or is scalac's diagnostic. Run by tests/support/local-given-params.sh: scalac 3.8.4 compiles
and runs each cell (its first error, at its line and column, where it does not compile); teq runs it in the
interpreter, on the JVM and in JavaScript.

  local_given_params.py cells <out dir>                 the cells (<cell>/Main.scala) and cells.txt
  local_given_params.py judge <cells dir> <base> <head> every leg of the head against scalac's run or diagnostic; the
                                                        base's legs that were scalac's unchanged on the head; every
                                                        cell not scalac's listed by name
  local_given_params.py --self-test
"""
import os
import re
import sys

# kind: (the local definitions, giving a T; the type T; an expression of T printed; a second local candidate of T)
KINDS = {
    "using": ("given Int = 3\ngiven g(using n: Int): String = \"g\" * n", "String", "X", "given g2(using n: Int): String = \"h\" * n"),
    "default": ("given g(using n: Int = 2): String = \"g\" * n", "String", "X", "given g2(using n: Int = 2): String = \"h\" * n"),
    "typed": ("given g[A](using o: Ordering[A]): Ordering[List[A]] = Ordering.by(_.headOption)", "Ordering[List[Int]]",
              "X.compare(List(1), List(2))", "given g2[A](using o: Ordering[A]): Ordering[List[A]] = Ordering.by(_.lastOption)"),
    "inline": ("inline given g(using n: Int = 2): String = \"g\" * n", "String", "X", "inline given g2(using n: Int = 2): String = \"h\" * n"),
    "transparent": ("transparent inline given g(using n: Int = 2): String = \"g\" * n", "String", "X",
                    "transparent inline given g2(using n: Int = 2): String = \"h\" * n"),
    "structural": ("given Int = 3\ngiven g(using n: Int): Show with\n  def show = \"s\" * n", "Show", "X.show",
                   "given g2(using n: Int): Show with\n  def show = \"t\" * n"),
    "anonymous": ("given Int = 3\ngiven (using n: Int): String = \"a\" * n", "String", "X", "given (using n: Int): String = \"b\" * n"),
}
# The file's own definitions: the trait the structural kind gives, the carrier of a given found through another's using
# clause, a method whose using clause the call's implicit argument fills.
PRELUDE = "trait Show:\n  def show: String\n\ncase class W(s: String)\n\ndef use[T](using t: T): T = t\n\n"
# A given of the kind's type the file defines outside the context (`shadow`).
OUTER = {
    "String": "given outer: String = \"outer\"\n\n",
    "Ordering[List[Int]]": "given outer: Ordering[List[Int]] = Ordering.by(_.size)\n\n",
    "Show": "given outer: Show with\n  def show = \"outer\"\n\n",
}
# context: a template with BODY the local definitions and EXPR the expression printed, and the indentation BODY takes.
CONTEXTS = {
    "def": ("def f(): String =\n  BODY\n  (EXPR).toString\n\n@main def run(): Unit = println(f())\n", 2),
    "lambda": ("@main def run(): Unit =\n  val k = () =>\n    BODY\n    (EXPR).toString\n  println(k())\n", 4),
    "valblock": ("object O:\n  val v: String =\n    BODY\n    (EXPR).toString\n\n@main def run(): Unit = println(O.v)\n", 4),
    "classinit": ("class C {\n  {\n    BODY\n    println((EXPR).toString)\n  }\n}\n\n@main def run(): Unit = { new C; () }\n", 4),
    "nested": ("def f(): String =\n  val r =\n    BODY\n    (EXPR).toString\n  r\n\n@main def run(): Unit = println(f())\n", 4),
    "matchcase": ("def f(k: Int): String = k match\n  case _ =>\n    BODY\n    (EXPR).toString\n\n@main def run(): Unit = println(f(1))\n", 4),
}
HOWS = ["summon", "callarg", "chained", "twotypes", "shadow", "ambiguous"]


def program(kind, context, how):
    """The cell's source, or None where the combination has no form (two types: the generic kind alone)."""
    defs, ty, show, second = KINDS[kind]
    if how == "twotypes" and kind != "typed":
        return None
    found = show.replace("X", f"summon[{ty}]")
    pre = PRELUDE
    if how == "callarg":
        found = show.replace("X", f"use[{ty}]")
    elif how == "chained":
        # A second local given whose using clause the kind's given fills.
        given_w = f"given hw(using t: {ty}): W = W(({show.replace('X', 't')}).toString)"
        defs = defs + "\n" + given_w
        found = "summon[W].s"
    elif how == "twotypes":
        found = found + " + summon[Ordering[List[String]]].compare(List(\"b\"), List(\"a\"))"
    elif how == "shadow":
        pre = pre + OUTER[ty]
    elif how == "ambiguous":
        defs = defs + "\n" + second
    template, ind = CONTEXTS[context]
    body = defs.replace("\n", "\n" + " " * ind)
    return pre + template.replace("BODY", body).replace("EXPR", found)


def cells():
    return [(k, c, h) for k in KINDS for c in CONTEXTS for h in HOWS if program(k, c, h) is not None]


def name(c):
    return "lgp-" + "-".join(c)


def write_cells(out):
    names = []
    for c in cells():
        d = os.path.join(out, name(c))
        os.makedirs(d, exist_ok=True)
        open(os.path.join(d, "Main.scala"), "w").write(program(*c))
        names.append(name(c))
    open(os.path.join(out, "cells.txt"), "w").write("\n".join(names) + "\n")
    return names


LEGS = ["interp", "jvm", "js"]


def read(path):
    """{cell: {leg: value}} of a runs file, and its malformed lines."""
    rows, bad = {}, []
    for i, line in enumerate(open(path).read().splitlines()):
        parts = line.split("\t")
        if len(parts) != 3 or not parts[2].startswith(("ok:", "err:")):
            bad.append(f"{os.path.basename(path)}:{i + 1}")
            continue
        rows.setdefault(parts[0], {})[parts[1]] = parts[2]
    return rows, bad


def judge(names, scalac, base, head, out=sys.stdout):
    """Every leg of the head against scalac's run (`run`): its line where scalac runs the cell, scalac's diagnostic
    (`err:<line>:<col>: <message>`) where scalac refuses it. Returns the exit code: a cell missing a leg, a base leg
    that was scalac's and moved, a manifest problem."""
    problems, turned, right, listed = [], [], [], []
    for c in names:
        want = scalac.get(c, {}).get("run")
        if want is None:
            problems.append(f"{c}: no scalac run")
            continue
        legs_h = [head.get(c, {}).get(l) for l in LEGS]
        legs_b = [base.get(c, {}).get(l) for l in LEGS]
        if None in legs_h or None in legs_b:
            problems.append(f"{c}: a leg missing")
            continue
        for l, b, h in zip(LEGS, legs_b, legs_h):
            if b == want and h != want:
                problems.append(f"{c}: {l} REGRESSED: base {b} head {h}")
        if all(h == want for h in legs_h):
            (right if all(b == want for b in legs_b) else turned).append(c)
        else:
            listed.append(c)
            print(f"  LISTED {c}: head {legs_h} | scalac {want}", file=out)
    print(f"local_given_params: not scalac's {len(listed)}; right on the base, unmoved {len(right)}; turned {len(turned)}", file=out)
    for p in problems:
        print(f"  PROBLEM {p}", file=out)
    print(f"local_given_params read every row: {len(names)} cells, {len(problems)} problems, {len(listed)} listed", file=out)
    return 1 if problems else 0


def self_test():
    bad = []
    p = program("structural", "classinit", "ambiguous")
    if "    given g2(using n: Int): Show with\n      def show = \"t\" * n" not in p or "println((summon[Show].show).toString)" not in p:
        bad.append("the structural kind's second candidate in a class's initialiser block")
    p = program("typed", "matchcase", "twotypes")
    if "summon[Ordering[List[String]]]" not in p or "    given g[A](using o: Ordering[A])" not in p:
        bad.append("the generic kind at two types in a match case")
    p = program("using", "nested", "chained")
    if "given hw(using t: String): W = W((t).toString)" not in p or "(summon[W].s).toString" not in p:
        bad.append("the chained given")
    if "given outer: String" not in program("anonymous", "def", "shadow") or program("using", "def", "twotypes") is not None:
        bad.append("the outer given or the two types' cell")
    if len(set(map(name, cells()))) != len(cells()):
        bad.append("cells not distinct")
    # Planted tables: a regressed base leg, a cell missing a leg, a turned cell and a listed one.
    names = ["c1", "c2", "c3"]
    scalac = {"c1": {"run": "ok:gg"}, "c2": {"run": "err:5:3: Ambiguous"}, "c3": {"run": "ok:x"}}
    base = {"c1": {l: "ok:gg" for l in LEGS}, "c2": {l: "err:5:3: other" for l in LEGS}, "c3": {l: "ok:x" for l in LEGS}}
    head = {"c1": {l: "ok:gg" for l in LEGS}, "c2": {l: "err:5:3: Ambiguous" for l in LEGS}, "c3": {"interp": "ok:x", "jvm": "ok:y", "js": "ok:x"}}
    sink = open(os.devnull, "w")
    if judge(names, scalac, base, head, sink) != 1:
        bad.append("a regressed leg passed")
    head2 = dict(head, c3={l: "ok:x" for l in LEGS})
    if judge(names, scalac, base, head2, sink) != 0:
        bad.append("a right table failed")
    head3 = dict(head2, c1={"interp": "ok:gg"})
    if judge(names, scalac, base, head3, sink) != 1:
        bad.append("a missing leg passed")
    if judge(names + ["c4"], scalac, base, head2, sink) != 1:
        bad.append("a cell without scalac's run passed")
    if bad:
        sys.exit("local_given_params self-test FAILED: " + "; ".join(bad))
    print(f"local_given_params self-test: {len(cells())} cells, the templates checked, 4 planted tables judged as they must be")


self_test()
cmd = sys.argv[1:2]
if cmd == ["cells"]:
    print(f"local_given_params: {len(write_cells(sys.argv[2]))} cells in {sys.argv[2]}")
elif cmd == ["judge"]:
    d = sys.argv[2]
    names = [l.strip() for l in open(os.path.join(d, "cells.txt")) if l.strip()]
    tables, malformed = [], []
    for t in ("scalac", sys.argv[3], sys.argv[4]):
        rows, bad = read(os.path.join(d, t + ".tsv"))
        tables.append(rows)
        malformed += bad
    code = judge(names, *tables)
    if malformed:
        print(f"  MANIFEST malformed lines {malformed[:5]}")
        code = 1
    sys.exit(code)
elif cmd != ["--self-test"]:
    sys.exit(__doc__)
