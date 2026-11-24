#!/usr/bin/env python3
"""tests/support/export_given_body.py: the table of an EXPORT IN A GIVEN INSTANCE's BODY, a structural given's
template (`given h: AnyRef with` and its body), whose export clauses are its class's forwarders as a class's are
(dotty's `Parsers.givenDef` reads a `templateBody`; `Namer.exportForwarders` of the given's class).

A cell is one program: the given stands in a HOLDER (the top level, an object, a class, a trait an object extends, an
enum, a block), its body INDENTED or in BRACES, and its export clause selects by a wildcard, a selection, a renaming
or the `given` selector from a val of the body; the program prints what the forwarder gives, read on the given (the
given selector's forwarder by its name, `h.gi`). Run by tests/support/local-given-params.sh, whose legs take any
cells directory of this form (scalac 3.8.4 compiles and runs each cell; teq runs it in the interpreter, on the JVM and
in JavaScript), and judged by local_given_params.py's judge.

  export_given_body.py cells <out dir>                 the cells (<cell>/Main.scala) and cells.txt
  export_given_body.py judge <cells dir> <base> <head> as local_given_params.py judges its table
  export_given_body.py --self-test
"""
import importlib.util
import os
import sys

HOLDERS = ["top", "object", "class", "trait", "enum", "block"]
FORMS = ["indented", "braced"]
# selector: (the clause's selector, what the program reads on the given `G`)
SELECTORS = {
    "wildcard": ("*", "G.m"),
    "selection": ("m", "G.m"),
    "renaming": ("{m as mm}", "G.mm"),
    "given": ("given", "G.gi.compare(1, 2)"),
}
PRELUDE = "object Codecs:\n  def m: Int = 7\n  given gi: Ordering[Int] = Ordering.Int\n\n"


def given_def(form, sel, ind):
    pad = " " * ind
    clause = f"export c.{sel}"
    if form == "indented":
        return f"{pad}given h: AnyRef with\n{pad}  val c: Codecs.type = Codecs\n{pad}  {clause}\n"
    return f"{pad}given h: AnyRef with {{\n{pad}  val c: Codecs.type = Codecs\n{pad}  {clause}\n{pad}}}\n"


def program(holder, form, selector):
    sel, read = SELECTORS[selector]
    if holder == "top":
        return PRELUDE + given_def(form, sel, 0) + f"\n@main def run(): Unit = println({read.replace('G', 'h')})\n"
    if holder == "object":
        return PRELUDE + "object H:\n" + given_def(form, sel, 2) + f"\n@main def run(): Unit = println({read.replace('G', 'H.h')})\n"
    if holder == "class":
        return PRELUDE + "class H:\n" + given_def(form, sel, 2) + f"\n@main def run(): Unit = println({read.replace('G', '(new H).h')})\n"
    if holder == "trait":
        return PRELUDE + "trait H:\n" + given_def(form, sel, 2) + "object HI extends H\n" + f"\n@main def run(): Unit = println({read.replace('G', 'HI.h')})\n"
    if holder == "enum":
        return PRELUDE + "enum H:\n  case A\n" + given_def(form, sel, 2) + f"\n@main def run(): Unit = println({read.replace('G', 'H.A.h')})\n"
    return PRELUDE + "def f(): String =\n" + given_def(form, sel, 2) + f"  ({read.replace('G', 'h')}).toString\n\n@main def run(): Unit = println(f())\n"


def cells():
    return [(h, f, s) for h in HOLDERS for f in FORMS for s in SELECTORS]


def name(c):
    return "egb-" + "-".join(c)


def write_cells(out):
    names = []
    for c in cells():
        d = os.path.join(out, name(c))
        os.makedirs(d, exist_ok=True)
        open(os.path.join(d, "Main.scala"), "w").write(program(*c))
        names.append(name(c))
    open(os.path.join(out, "cells.txt"), "w").write("\n".join(names) + "\n")
    return names


def the_judge():
    here = os.path.dirname(os.path.abspath(__file__))
    spec = importlib.util.spec_from_file_location("local_given_params", os.path.join(here, "local_given_params.py"))
    argv, sys.argv = sys.argv, [sys.argv[0], "--self-test"]
    out = open(os.devnull, "w")
    stdout, sys.stdout = sys.stdout, out
    try:
        m = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(m)
    finally:
        sys.argv, sys.stdout = argv, stdout
    return m


def self_test():
    bad = []
    p = program("trait", "braced", "renaming")
    if "  given h: AnyRef with {\n    val c: Codecs.type = Codecs\n    export c.{m as mm}\n  }\n" not in p or "println(HI.h.mm)" not in p:
        bad.append("the trait's braced renaming")
    p = program("block", "indented", "given")
    if "  given h: AnyRef with\n    val c: Codecs.type = Codecs\n    export c.given\n" not in p or "(h.gi.compare(1, 2)).toString" not in p:
        bad.append("the block's indented given selector")
    if len(set(map(name, cells()))) != len(cells()) or len(cells()) != len(HOLDERS) * len(FORMS) * len(SELECTORS):
        bad.append("cells not distinct or not every combination")
    if bad:
        sys.exit("export_given_body self-test FAILED: " + "; ".join(bad))
    print(f"export_given_body self-test: {len(cells())} cells, the templates checked (the judge's own planted tables in local_given_params.py)")


self_test()
cmd = sys.argv[1:2]
if cmd == ["cells"]:
    print(f"export_given_body: {len(write_cells(sys.argv[2]))} cells in {sys.argv[2]}")
elif cmd == ["judge"]:
    m = the_judge()
    d = sys.argv[2]
    names = [l.strip() for l in open(os.path.join(d, "cells.txt")) if l.strip()]
    tables, malformed = [], []
    for t in ("scalac", sys.argv[3], sys.argv[4]):
        rows, bad = m.read(os.path.join(d, t + ".tsv"))
        tables.append(rows)
        malformed += bad
    code = m.judge(names, *tables, out=sys.stdout)
    if malformed:
        print(f"  MANIFEST malformed lines {malformed[:5]}")
        code = 1
    sys.exit(code)
elif cmd != ["--self-test"]:
    sys.exit(__doc__)
