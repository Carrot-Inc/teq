#!/usr/bin/env python3
"""tests/support/given_instance_import.py: the table of an IMPORT OVER A GIVEN INSTANCE and over the other stable
values an import's qualifier names: a given without parameters is an object (dotty's `Parsers.givenDef` makes it a
`ModuleDef`), whose members an import brings as an object's (`import h.*`, `import h.given`), and an enum's singleton
case is a stable val (`val A: E`), its members the enum's (`import E.A.given`).

A cell is one program in package `fd`: its library (lib/: the given or the value and what they hold) and its user
(use/Main.scala, `@main def run()`), run WHOLE and SPLIT (the library to products first, the user against them) by
tests/support/pkgobj-given-import.sh, scalac's and teq's legs, and judged by pkgobj_given_import.py's judge. The families:
  given    where the given stands (the top level, an object, a class read through a val of it, a trait an object extends,
           a block of the user) x its form (`with` indented, in braces, a colon body, with a parent trait, with a type
           parameter, with a using clause: the last two are no stable path, scalac's refusal) x the import (a wildcard,
           a selection, a renaming, `given`, `{given T}`, a nested object's path, an exclusion);
  value    the qualifier's kind (an enum's singleton case, a val of the enum's type, a case object, the given instance)
           x the import (a wildcard, a selection, `given`).
Each program prints what it reads through the import: the members a def `f` (3), a val `v` (4), an object `o` with `g`
(5), a type `T` (Int), a given `gi` (`Ordering.Int.reverse`, `compare(1, 2)` is 1); a parent's `b` (6).

  given_instance_import.py cells <out dir>                 the cells and cells.txt
  given_instance_import.py judge <cells dir> <base> <head> the runs against scalac's (pkgobj_given_import.py's judge)
  given_instance_import.py --self-test
"""
import importlib.util
import os
import sys

HOLDERS = ["top", "object", "class", "trait", "block"]
FORMS = ["indented", "braced", "colon", "parent", "typeparam", "using"]
IMPORTS = {
    # the selector, what the user prints
    "wildcard": ("*", "f + v + o.g + (2: T)"),
    "selection": ("{f, v}", "f + v"),
    "renaming": ("{f as ff}", "ff"),
    "given": ("given", "summon[Ordering[Int]].compare(1, 2)"),
    "giventyped": ("{given Ordering[Int]}", "summon[Ordering[Int]].compare(1, 2)"),
    "nested": (None, "g"),
    "exclusion": ("{f as _, *}", "v"),
}
BODY = ["def f: Int = 3", "val v: Int = 4", "object o:", "  def g: Int = 5", "type T = Int",
        "given gi: Ordering[Int] = Ordering.Int.reverse"]
BASE = "trait Base:\n  def b: Int = 6\n"


def given_def(form, pad):
    head = {"indented": "given h: AnyRef with", "braced": "given h: AnyRef with {", "colon": "given h: AnyRef:",
            "parent": "given h: Base with", "typeparam": "given h[X]: AnyRef with", "using": "given h(using DummyImplicit): AnyRef with"}[form]
    lines = [pad + head] + [pad + "  " + l for l in BODY]
    if form == "braced":
        lines.append(pad + "}")
    return "\n".join(lines) + "\n"


def given_cell(holder, form, imp):
    sel, read = IMPORTS[imp]
    if form == "parent" and imp == "wildcard":
        read += " + b"
    lib = "package fd\n\n" + BASE + "\n"
    if holder == "top":
        lib += given_def(form, "")
        path, pre = "fd.h", []
    elif holder == "object":
        lib += "object H:\n" + given_def(form, "  ")
        path, pre = "fd.H.h", []
    elif holder == "class":
        lib += "class C:\n" + given_def(form, "  ")
        path, pre = "c.h", ["val c = new C"]
    elif holder == "trait":
        lib += "trait TH:\n" + given_def(form, "  ") + "object HI extends TH\n"
        path, pre = "fd.HI.h", []
    else:
        path, pre = "h", [l for l in given_def(form, "").splitlines()]
    clause = f"import {path}.o.*" if sel is None else f"import {path}.{sel}"
    lines = pre + [clause, f"println({read})"]
    return {"Lib.scala": lib}, "package fd\n\n@main def run(): Unit =\n" + "".join("  " + l + "\n" for l in lines)


VALUES = {
    # the library, the qualifier's path
    "enumcase": ("enum E:\n  case A\n" + "".join("  " + l + "\n" for l in BODY), "fd.E.A"),
    "enumval": ("enum E:\n  case A\n" + "".join("  " + l + "\n" for l in BODY) + "val e: E = E.A\n", "fd.e"),
    "caseobject": ("case object CO:\n" + "".join("  " + l + "\n" for l in BODY), "fd.CO"),
    "giveninstance": (given_def("indented", ""), "fd.h"),
}
VALUE_IMPORTS = ["wildcard", "selection", "given"]


def value_cell(kind, imp):
    src, path = VALUES[kind]
    sel, read = IMPORTS[imp]
    return {"Lib.scala": "package fd\n\n" + src}, f"package fd\n\n@main def run(): Unit =\n  import {path}.{sel}\n  println({read})\n"


def cells():
    out = []
    for h in HOLDERS:
        for f in FORMS:
            for i in IMPORTS:
                out.append((f"g-{h}-{f}-{i}",) + given_cell(h, f, i))
    for k in VALUES:
        for i in VALUE_IMPORTS:
            out.append((f"v-{k}-{i}",) + value_cell(k, i))
    return out


def write_cells(out):
    names = []
    for name, lib, use in cells():
        for f, text in lib.items():
            p = os.path.join(out, name, "lib", f)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            open(p, "w").write(text)
        os.makedirs(os.path.join(out, name, "use"), exist_ok=True)
        open(os.path.join(out, name, "use", "Main.scala"), "w").write(use)
        names.append(name)
    open(os.path.join(out, "cells.txt"), "w").write("\n".join(names) + "\n")
    return names


def the_judge():
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


def self_test():
    bad = []
    cs = cells()
    names = [c[0] for c in cs]
    want = len(HOLDERS) * len(FORMS) * len(IMPORTS) + len(VALUES) * len(VALUE_IMPORTS)
    if len(set(names)) != len(names) or len(cs) != want:
        bad.append(f"{len(cs)} cells, not every combination ({want}) or not distinct")
    d = dict((c[0], c[1:]) for c in cs)
    lib, use = d["g-class-braced-nested"]
    if "class C:\n  given h: AnyRef with {\n    def f: Int = 3\n" not in lib["Lib.scala"] or "  val c = new C\n  import c.h.o.*\n  println(g)\n" not in use:
        bad.append("the class's braced given, its nested object imported through a val")
    lib, use = d["g-block-using-exclusion"]
    if "  given h(using DummyImplicit): AnyRef with\n    def f: Int = 3\n" not in use or "import h.{f as _, *}" not in use:
        bad.append("the block's given with a using clause, an exclusion")
    lib, use = d["g-top-parent-wildcard"]
    if "given h: Base with\n" not in lib["Lib.scala"] or "println(f + v + o.g + (2: T) + b)" not in use:
        bad.append("the parent's member through a wildcard")
    lib, use = d["v-enumcase-given"]
    if "enum E:\n  case A\n  def f: Int = 3\n" not in lib["Lib.scala"] or "import fd.E.A.given" not in use:
        bad.append("the enum case's given import")
    if bad:
        sys.exit("given_instance_import self-test FAILED: " + "; ".join(bad))
    print(f"given_instance_import self-test: {len(cs)} cells, the templates checked (the judge's planted tables in pkgobj_given_import.py)")


self_test()
cmd = sys.argv[1:2]
if cmd == ["cells"]:
    print(f"given_instance_import: {len(write_cells(sys.argv[2]))} cells in {sys.argv[2]}")
elif cmd == ["judge"]:
    m = the_judge()
    d = sys.argv[2]
    names = [l.strip() for l in open(os.path.join(d, "cells.txt")) if l.strip()]
    tables, malformed = [], []
    for t in ("scalac", sys.argv[3], sys.argv[4]):
        rows, bad = m.read(os.path.join(d, t + ".tsv"))
        tables.append(rows)
        malformed += [f"{t}: {b}" for b in bad]
    code = m.judge(names, *tables, out=sys.stdout)
    if malformed:
        print(f"  MALFORMED lines {malformed[:5]}")
        code = 1
    sys.exit(code)
elif cmd != ["--self-test"]:
    sys.exit(__doc__)
