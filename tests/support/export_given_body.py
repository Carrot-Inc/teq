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
  export_given_body.py split-cells <out dir>           the same programs as a library and its user, in package egb
                                                       (<cell>/lib/Lib.scala: the holder; <cell>/use/Main.scala: the
                                                       read), run by tests/support/pkgobj-given-import.sh whole and
                                                       split, and scalac's user over teq's library (`--over`)
  export_given_body.py split-judge <cells dir> <base> <head>   those runs against scalac's
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


def split_program(holder, form, selector):
    """The program as a library, the holder of the given, and its user, another module: the read on the given's
    forwarder (the block's through the library's method)."""
    sel, read = SELECTORS[selector]
    head = "package egb\n\n" + PRELUDE
    if holder == "top":
        lib, at = head + given_def(form, sel, 0), read.replace("G", "h")
    elif holder == "object":
        lib, at = head + "object H:\n" + given_def(form, sel, 2), read.replace("G", "H.h")
    elif holder == "class":
        lib, at = head + "class H:\n" + given_def(form, sel, 2), read.replace("G", "(new H).h")
    elif holder == "trait":
        lib, at = head + "trait H:\n" + given_def(form, sel, 2) + "object HI extends H\n", read.replace("G", "HI.h")
    elif holder == "enum":
        lib, at = head + "enum H:\n  case A\n" + given_def(form, sel, 2), read.replace("G", "H.A.h")
    else:
        lib, at = head + "def f(): String =\n" + given_def(form, sel, 2) + f"  ({read.replace('G', 'h')}).toString\n", "f()"
    return lib, f"package egb\n\n@main def run(): Unit = println({at})\n"


def write_split_cells(out):
    names = []
    for c in cells():
        lib, use = split_program(*c)
        for sub, f, text in (("lib", "Lib.scala", lib), ("use", "Main.scala", use)):
            os.makedirs(os.path.join(out, name(c), sub), exist_ok=True)
            open(os.path.join(out, name(c), sub, f), "w").write(text)
        open(os.path.join(out, name(c), "main.txt"), "w").write("egb.run\n")
        names.append(name(c))
    open(os.path.join(out, "cells.txt"), "w").write("\n".join(names) + "\n")
    return names


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


def the_split_judge():
    """pkgobj_given_import.py's judge of a library and its user's legs, imported as the_judge imports
    local_given_params'."""
    here = os.path.dirname(os.path.abspath(__file__))
    spec = importlib.util.spec_from_file_location("pkgobj_given_import", os.path.join(here, "pkgobj_given_import.py"))
    argv, sys.argv = sys.argv, [sys.argv[0], "--self-test"]
    stdout, sys.stdout = sys.stdout, open(os.devnull, "w")
    try:
        m = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(m)
    finally:
        sys.argv, sys.stdout = argv, stdout
    return m


def split_judge(m, names, scalac, base, head, over_base, over_head, out=sys.stdout):
    """The six teq legs by pkgobj_given_import's judge; then scalac's user over teq's library, against scalac's own
    split build: right on the head, turned, regressed."""
    code = m.judge(names, scalac, base, head, out=out)
    counts = {"right": 0, "turned": 0, "problem": 0, "regressed": 0, "missing": 0}
    lines = []
    for c in names:
        s, h, b = scalac.get((c, "split", "run")), over_head.get((c, "split", "over")), over_base.get((c, "split", "over"))
        if s is None or h is None or b is None:
            counts["missing"] += 1
            lines.append(f"MISSING over {c}")
            continue
        if m.right(s, h):
            counts["right"] += 1
            counts["turned"] += not m.right(s, b)
        else:
            counts["problem"] += 1
            counts["regressed"] += m.right(s, b)
            lines.append(f"{'REGRESSED' if m.right(s, b) else 'PROBLEM'} over {c}: scalac {s} | head {h} | base {b}")
    print(f"scalac's user over teq's library: {len(names)} cells, {counts['right']} right on the head ({counts['turned']} turned), "
          f"{counts['problem']} problems ({counts['regressed']} regressed), {counts['missing']} missing", file=out)
    for l in lines:
        print("  " + l, file=out)
    return 1 if code or counts["problem"] or counts["missing"] else 0


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
    lib, use = split_program("block", "braced", "renaming")
    if "package egb\n\nobject Codecs:" not in lib or "def f(): String =\n  given h: AnyRef with {\n" not in lib or "println(f())" not in use:
        bad.append("the block's braced renaming as a library and its user")
    lib, use = split_program("enum", "indented", "given")
    if "enum H:\n  case A\n  given h: AnyRef with\n" not in lib or "println(H.A.h.gi.compare(1, 2))" not in use:
        bad.append("the enum's given selector read by another module")
    # The split judge over planted tables: the over leg right, and planted wrong on the head.
    m = the_split_judge()
    legs = [(b, l) for b in ("whole", "split") for l in m.LEGS]
    sc = {("a", b, "run"): "ok:7" for b in ("whole", "split")}
    teq = {("a", b, l): "ok:7" for b, l in legs}
    null = open(os.devnull, "w")
    ov = {("a", "split", "over"): "ok:7"}
    if split_judge(m, ["a"], sc, teq, teq, ov, ov, out=null) != 0:
        bad.append("the split judge on a right table")
    if split_judge(m, ["a"], sc, teq, teq, ov, {("a", "split", "over"): "err:Main.scala:3:37: value m is not a member of h"}, out=null) != 1:
        bad.append("the split judge passed a wrong over leg")
    if split_judge(m, ["a"], sc, teq, teq, ov, {}, out=null) != 1:
        bad.append("the split judge passed a missing over leg")
    if bad:
        sys.exit("export_given_body self-test FAILED: " + "; ".join(bad))
    print(f"export_given_body self-test: {len(cells())} cells, whole and as a library and its user, the templates checked, the split judge's planted tables judged as they must be (the judge's own in local_given_params.py)")


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
elif cmd == ["split-cells"]:
    print(f"export_given_body: {len(write_split_cells(sys.argv[2]))} split cells in {sys.argv[2]}")
elif cmd == ["split-judge"]:
    m = the_split_judge()
    d = sys.argv[2]
    names = [l.strip() for l in open(os.path.join(d, "cells.txt")) if l.strip()]
    tables, malformed = [], []
    for t in ("scalac", sys.argv[3], sys.argv[4]):
        rows, bad = m.read(os.path.join(d, t + ".tsv"))
        tables.append(rows)
        malformed += [f"{t}: {b}" for b in bad]
    overs = []
    for t in (sys.argv[3], sys.argv[4]):
        rows = {}
        path = os.path.join(d, f"scalac-over-{t}.tsv")
        for l in (open(path) if os.path.exists(path) else []):
            f = l.rstrip("\n").split("\t")
            if len(f) == 4:
                rows[(f[0], f[1], f[2])] = f[3]
        overs.append(rows)
    code = split_judge(m, names, *tables, *overs)
    if malformed:
        print(f"  MALFORMED lines {malformed[:5]}")
        code = 1
    sys.exit(code)
elif cmd != ["--self-test"]:
    sys.exit(__doc__)
