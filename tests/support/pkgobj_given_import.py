#!/usr/bin/env python3
"""tests/support/pkgobj_given_import.py: the table of an IMPORT OF EXPORTED GIVENS, a package object's above all: an
import of a package's givens (`import fd.pf.given`, `import fd.pf.{given T}`) brings the givens its package object
exports, as it brings an object's (dotty's `ImportInfo.importedImplicits` over the package's members, the package
object's among them, its export forwarders included).

A cell is one program in package `fd`: its library sources (lib/: the export's source objects, the holder or holders of
the export) and its user (use/Main.scala, `@main def run()`, printing what a summon finds). Each runs WHOLE (lib and use
compiled together) and SPLIT (lib compiled first, the user against its products). The families:
  kinds    the HOLDER of the export (a package object, an object, a package's top-level file; the path is `fd.pf` in
           each) x the KIND of the exported given (an alias, a structural given, one with a using clause, one with a type
           parameter, an anonymous one, one in a nested object of the export's source) x the CLAUSE (`given`,
           `{given T}`, by name, `*`) x HOW the user imports (`given`, `{given T}`, `*`); and per holder and import the
           holder's own given, the control;
  scopes   per kind, the holder's import in a nested scope against an outer import of another given of the kind's
           type, the other way round, both in one scope, and beside an import of the given the holder forwards to;
  twice    the holder exporting two givens of one type, or defining them, by `given` and `{given T}` (scalac's ambiguity);
  written  a given of the holder written as a path's singleton, an alias given's or a structural one's
           (`given a: Impl.gi.type = Impl.gi`), beside another import's given (scalac's ambiguity), and one that is
           another given of the holder by its type (`given y: x.type = x` beside `x`: one value, no ambiguity);
  paths    two package objects in one path (`fd.pf` and `fd.pf.qf`), each exporting a given of its own source;
  beside   a package's top-level file beside its package object, each exporting or defining a given;
  chains   a chain of givens each a path to the one before (`given a1: a0.type = a0`, 1, 2, 16, 17 and 40 links),
           imported by its two ends or by its end and a middle link: one value, no ambiguity;
  lexical  no import: the user in the package itself, under chained package clauses (`package fd`, `package pf`,
           `package sub`: the outer packages' members in scope), in a dotted one (`package fd.pf.sub`: not in scope).
The givens print apart: an export source's `Ordering.Int.reverse` 1, a second source's or another import's 7, the
implicit scope's `Ordering.Int` -1.

  pkgobj_given_import.py cells <out dir>                 the cells (<cell>/lib/*.scala, <cell>/use/Main.scala, and
                                                         <cell>/main.txt where the entry point is not fd.run), cells.txt
  pkgobj_given_import.py judge <cells dir> <base> <head> the runs of tests/support/pkgobj-given-import.sh against scalac's
  pkgobj_given_import.py --self-test
The judge reads <cells>/scalac.tsv and <cells>/<tag>.tsv (`cell <TAB> build <TAB> leg <TAB> ok:<output> |
err:<file>:<line>:<col>: <message>`): a cell's leg is RIGHT when its output is scalac's, or its error is at scalac's file,
line and column (an ambiguity's words apart: teq's own wording, `ambiguous given instances`, is counted as WORDING);
TURNED when right on the head and not on the base; REGRESSED the other way; a problem when wrong on the head.
"""
import os
import sys

HOLDERS = ["pkgobj", "object", "toplevel"]
# kind: (the export source's body, the path the clause exports from, the given's name, the type the user summons, what
# the user prints)
ALIAS = "  given gi: Ordering[Int] = Ordering.Int.reverse\n"
KINDS = {
    "alias": (ALIAS, "fd.Impl", "gi", "Ordering[Int]", "summon[Ordering[Int]].compare(1, 2)"),
    "structural": ("  given gs: Ordering[Int] with\n    def compare(x: Int, y: Int): Int = y.compare(x)\n", "fd.Impl", "gs",
                   "Ordering[Int]", "summon[Ordering[Int]].compare(1, 2)"),
    "using": ("  given gu(using DummyImplicit): Ordering[Int] = Ordering.Int.reverse\n", "fd.Impl", "gu", "Ordering[Int]",
              "summon[Ordering[Int]].compare(1, 2)"),
    "typeparam": ("  given gp[T](using o: Ordering[T]): Ordering[Option[T]] = Ordering.Option(o).reverse\n", "fd.Impl", "gp",
                  "Ordering[Option[Int]]", "summon[Ordering[Option[Int]]].compare(Some(1), Some(2))"),
    "anonymous": ("  given Ordering[Int] = Ordering.Int.reverse\n", "fd.Impl", "given_Ordering_Int", "Ordering[Int]",
                  "summon[Ordering[Int]].compare(1, 2)"),
    "nested": ("  object Inner:\n    given gn: Ordering[Int] = Ordering.Int.reverse\n", "fd.Impl.Inner", "gn", "Ordering[Int]",
               "summon[Ordering[Int]].compare(1, 2)"),
}
CLAUSES = ["given", "giventyped", "byname", "wildcard"]
HOWS = ["given", "giventyped", "wildcard"]
SEVEN = "new Ordering[Int] { def compare(x: Int, y: Int): Int = 7 }"
READ = "summon[Ordering[Int]].compare(1, 2)"


def clause(what, kind):
    _, path, name, wanted, _ = KINDS[kind]
    return {"given": f"export {path}.given", "giventyped": f"export {path}.{{given {wanted}}}", "byname": f"export {path}.{name}",
            "wildcard": f"export {path}.*"}[what]


def selector(how, wanted):
    return {"given": "given", "giventyped": f"{{given {wanted}}}", "wildcard": "*"}[how]


def holder_src(holder, body):
    """The holder of `body` (its lines, indented by two) at the path fd.pf."""
    if holder == "pkgobj":
        return "package fd\n\npackage object pf:\n" + body
    if holder == "object":
        return "package fd\n\nobject pf:\n" + body
    # A package's top-level file: its definitions are the package's.
    return "package fd.pf\n\n" + "".join(l[2:] + "\n" for l in body.splitlines())


def user(lines):
    return "package fd\n\n@main def run(): Unit =\n" + "".join("  " + l + "\n" for l in lines)


def kinds_cell(holder, kind, what, how):
    src, _, _, wanted, read = KINDS[kind]
    if what == "own":
        lib = {"Impl.scala": "package fd\n\nobject Impl:\n" + ALIAS, "Holder.scala": holder_src(holder, ALIAS)}
    else:
        lib = {"Impl.scala": "package fd\n\nobject Impl:\n" + src, "Holder.scala": holder_src(holder, "  " + clause(what, kind) + "\n")}
    return lib, user([f"import fd.pf.{selector(how, wanted)}", f"println({read})"])


def other(wanted):
    """Another given of the kind's type, printing 7."""
    arg = wanted[len("Ordering["):-1]
    return f"package fd\n\nobject Other:\n  given go: {wanted} = new {wanted} {{ def compare(x: {arg}, y: {arg}): Int = 7 }}\n"


def scopes(variant, path, read):
    return {
        # the holder's import inner, another given's outer
        "inner": ["import fd.Other.given", "locally {", "  import fd.pf.given", f"  println({read})", "}"],
        # the other way round
        "outer": ["import fd.pf.given", "locally {", "  import fd.Other.given", f"  println({read})", "}"],
        # both in one scope
        "same": ["import fd.pf.given", "import fd.Other.given", f"println({read})"],
        # the forwarder beside the given it forwards to (`Applications.compare`'s alternatives that are one path)
        "target": ["import fd.pf.given", f"import {path}.given", f"println({read})"],
    }[variant]


SCOPES = ["inner", "outer", "same", "target"]


def scopes_cell(holder, kind, variant):
    src, path, _, wanted, read = KINDS[kind]
    lib = {"Impl.scala": "package fd\n\nobject Impl:\n" + src, "Other.scala": other(wanted),
           "Holder.scala": holder_src(holder, f"  export {path}.given\n")}
    return lib, user(scopes(variant, path, read))


TWO = "  given gi: Ordering[Int] = Ordering.Int.reverse\n  given gj: Ordering[Int] = " + SEVEN + "\n"


def twice_cell(holder, what, how):
    if what == "export":
        lib = {"ImplTwo.scala": "package fd\n\nobject ImplTwo:\n" + TWO, "Holder.scala": holder_src(holder, "  export fd.ImplTwo.given\n")}
    else:
        lib = {"Holder.scala": holder_src(holder, TWO)}
    return lib, user([f"import fd.pf.{selector(how, 'Ordering[Int]')}", f"println({READ})"])


IMPL2 = "package fd\n\nobject Impl2:\n  given g7: Ordering[Int] = " + SEVEN + "\n"
PATHS = {
    "outer": ["import fd.pf.given", f"println({READ})"],
    "inner": ["import fd.pf.qf.given", f"println({READ})"],
    "nested": ["import fd.pf.given", "locally {", "  import fd.pf.qf.given", f"  println({READ})", "}"],
    "same": ["import fd.pf.given", "import fd.pf.qf.given", f"println({READ})"],
}


def paths_cell(variant):
    lib = {"Impl.scala": "package fd\n\nobject Impl:\n" + ALIAS, "Impl2.scala": IMPL2,
           "Pf.scala": "package fd\n\npackage object pf:\n  export fd.Impl.given\n",
           "Qf.scala": "package fd.pf\n\npackage object qf:\n  export fd.Impl2.given\n"}
    return lib, user(PATHS[variant])


# beside: (the package object's body, the top-level file's body, the user's import)
BESIDE = {
    "bothexport": ("  export fd.Impl.given\n", "export fd.Impl2.given\n", "given"),
    "bothexporttyped": ("  export fd.Impl.given\n", "export fd.Impl2.given\n", "{given Ordering[Int]}"),
    "objexport": ("  export fd.Impl.given\n", "val unrelated: Int = 3\n", "given"),
    "topexport": ("  val unrelated: Int = 3\n", "export fd.Impl2.given\n", "given"),
    "ownandexport": (ALIAS, "export fd.Impl2.given\n", "given"),
    "sameexport": ("  export fd.Impl.given\n", "export fd.Impl.{gi as gt}\n", "given"),
}


def beside_cell(variant):
    obj, top, how = BESIDE[variant]
    lib = {"Impl.scala": "package fd\n\nobject Impl:\n" + ALIAS, "Impl2.scala": IMPL2,
           "Pf.scala": "package fd\n\npackage object pf:\n" + obj, "Top.scala": "package fd.pf\n\n" + top}
    return lib, user([f"import fd.pf.{how}", f"println({READ})"])


LEXICAL = {
    "same": ("package fd.pf\n\n", "fd.pf.run"),
    "chained": ("package fd\npackage pf\npackage sub\n\n", "fd.pf.sub.run"),
    "dotted": ("package fd.pf.sub\n\n", "fd.pf.sub.run"),
}


def lexical_cell(holder, variant):
    head, main = LEXICAL[variant]
    lib = {"Impl.scala": "package fd\n\nobject Impl:\n" + ALIAS, "Holder.scala": holder_src(holder, "  export fd.Impl.given\n")}
    return lib, head + f"@main def run(): Unit =\n  println({READ})\n", main


STRUCTURAL = "  given gs: Ordering[Int] with\n    def compare(x: Int, y: Int): Int = y.compare(x)\n"
WRITTEN = {
    "alias": ("package fd\n\nobject Impl:\n" + ALIAS, "  given a: fd.Impl.gi.type = fd.Impl.gi\n", True),
    "structural": ("package fd\n\nobject Impl:\n" + STRUCTURAL, "  given a: fd.Impl.gs.type = fd.Impl.gs\n", True),
    "onevalue": (None, "  given x: Ordering[Int] = Ordering.Int.reverse\n  given y: x.type = x\n", False),
}


def written_cell(holder, variant):
    impl, body, other_import = WRITTEN[variant]
    lib = {"Holder.scala": holder_src(holder, body)}
    if impl:
        lib["Impl.scala"] = impl
    lines = ["import fd.pf.given"]
    if other_import:
        lib["Other.scala"] = other("Ordering[Int]")
        lines.append("import fd.Other.given")
    return lib, user(lines + [f"println({READ})"])


CHAINS = [1, 2, 16, 17, 40]


def chain_cell(links, route):
    lib = "package fd\n\nobject O:\n  given a0: Ordering[Int] = Ordering.Int.reverse\n" + "".join(
        f"  given a{i}: a{i - 1}.type = a{i - 1}\n" for i in range(1, links + 1))
    other = 0 if route == "ends" else links // 2
    return {"Chain.scala": lib}, user([f"import fd.O.a{other}", f"import fd.O.a{links}", f"println({READ})"])


def cells():
    """[(name, lib {file: text}, use text[, entry point])]"""
    out = []
    for h in HOLDERS:
        for k in KINDS:
            for w in CLAUSES:
                for i in HOWS:
                    out.append((f"k-{h}-{k}-{w}-{i}",) + kinds_cell(h, k, w, i))
        for i in HOWS:
            out.append((f"k-{h}-alias-own-{i}",) + kinds_cell(h, "alias", "own", i))
        for k in KINDS:
            for v in SCOPES:
                out.append((f"s-{h}-{k}-{v}",) + scopes_cell(h, k, v))
        for w in ("export", "own"):
            for i in ("given", "giventyped"):
                out.append((f"t-{h}-{w}-{i}",) + twice_cell(h, w, i))
        for v in WRITTEN:
            out.append((f"w-{h}-{v}",) + written_cell(h, v))
    for v in PATHS:
        out.append((f"p-{v}",) + paths_cell(v))
    for v in BESIDE:
        out.append((f"b-{v}",) + beside_cell(v))
    for n in CHAINS:
        for r in ("ends", "middle"):
            out.append((f"n-{n}-{r}",) + chain_cell(n, r))
    # An object `pf` beside a package `fd.pf` is a double definition: the lexical family's holders are the package's.
    for h in ("pkgobj", "toplevel"):
        for v in LEXICAL:
            out.append((f"l-{h}-{v}",) + lexical_cell(h, v))
    return out


def write_cells(out):
    names = []
    for name, lib, use, *main in cells():
        if main:
            os.makedirs(os.path.join(out, name), exist_ok=True)
            open(os.path.join(out, name, "main.txt"), "w").write(main[0] + "\n")
        for f, text in lib.items():
            p = os.path.join(out, name, "lib", f)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            open(p, "w").write(text)
        os.makedirs(os.path.join(out, name, "use"), exist_ok=True)
        open(os.path.join(out, name, "use", "Main.scala"), "w").write(use)
        names.append(name)
    open(os.path.join(out, "cells.txt"), "w").write("\n".join(names) + "\n")
    return names


def read(path):
    """{(cell, build, leg): verdict}, and the malformed lines."""
    rows, bad = {}, []
    if not os.path.exists(path):
        return rows, [f"{path}: none"]
    for n, line in enumerate(open(path), 1):
        f = line.rstrip("\n").split("\t")
        if len(f) != 4 or f[1] not in ("whole", "split") or not (f[3].startswith("ok:") or f[3].startswith("err:")):
            bad.append(n)
            continue
        rows[(f[0], f[1], f[2])] = f[3]
    return rows, bad


def place(v):
    """An error's file, line and column, or None for an output."""
    if not v.startswith("err:"):
        return None
    head = v[4:].split(": ", 1)[0]
    return head if head.count(":") == 2 else head + "?"


def words(v):
    return v[4:].split(": ", 1)[1] if ": " in v[4:] else ""


def wording(scalac, v):
    """An error at scalac's place whose words differ as teq's ambiguity message does everywhere (`ambiguous given
    instances for T: a, b` for scalac's `Ambiguous given instances: both ... and ... match type T ...`): the
    wording part of its own."""
    return (scalac.startswith("err:") and v.startswith("err:") and place(v) == place(scalac) and words(v) != words(scalac)
            and words(scalac).startswith("Ambiguous given instances: ") and words(v).startswith("ambiguous given instances for "))


def right(scalac, v):
    """scalac's output, or scalac's error at its place in its words (teq's ambiguity wording counted apart)."""
    if scalac.startswith("ok:"):
        return v == scalac
    return v.startswith("err:") and place(v) == place(scalac) and (words(v) == words(scalac) or wording(scalac, v))


LEGS = ["interp", "jvm", "js"]


def judge(names, scalac, base, head, out=sys.stdout):
    counts = {"right": 0, "turned": 0, "unmoved": 0, "regressed": 0, "problem": 0, "wording": 0, "missing": 0}
    problems = []
    for c in names:
        for b in ("whole", "split"):
            s = scalac.get((c, b, "run"))
            if s is None:
                counts["missing"] += 1
                problems.append(f"MISSING scalac {c} {b}")
                continue
            for leg in LEGS:
                h, m = head.get((c, b, leg)), base.get((c, b, leg))
                if h is None or m is None:
                    counts["missing"] += 1
                    problems.append(f"MISSING {'head' if h is None else 'base'} {c} {b} {leg}")
                    continue
                hr, mr = right(s, h), right(s, m)
                if hr:
                    counts["right"] += 1
                    counts["turned" if not mr else "unmoved"] += 1
                    if wording(s, h):
                        counts["wording"] += 1
                else:
                    counts["problem"] += 1
                    if mr:
                        counts["regressed"] += 1
                    problems.append(f"{'REGRESSED' if mr else 'PROBLEM'} {c} {b} {leg}: scalac {s} | head {h} | base {m}")
    total = len(names) * 2 * len(LEGS)
    print(f"{len(names)} cells, {total} legs (whole and split x interp, jvm, js): {counts['right']} right on the head "
          f"({counts['turned']} turned, {counts['unmoved']} right on the base too; {counts['wording']} of them an ambiguity at "
          f"scalac's place in teq's own words), {counts['problem']} problems ({counts['regressed']} regressed), "
          f"{counts['missing']} missing", file=out)
    for p in problems:
        print("  " + p, file=out)
    return 0 if counts["problem"] == 0 and counts["missing"] == 0 else 1


def self_test():
    bad = []
    cs = cells()
    names = [c[0] for c in cs]
    if len(set(names)) != len(names):
        bad.append("cell names not distinct")
    want = len(HOLDERS) * (len(KINDS) * len(CLAUSES) * len(HOWS) + len(HOWS) + len(KINDS) * len(SCOPES) + 4 + len(WRITTEN)) + len(PATHS) + len(BESIDE) + 2 * len(LEXICAL) + 2 * len(CHAINS)
    if len(cs) != want:
        bad.append(f"{len(cs)} cells, not every combination ({want})")
    d = dict((c[0], c[1:3]) for c in cs)
    if [c[3] for c in cs if c[0] == "l-toplevel-chained"] != ["fd.pf.sub.run"] or "package fd\npackage pf\npackage sub\n" not in d["l-toplevel-chained"][1]:
        bad.append("the chained package clauses' user and its entry point")
    lib, use = d["k-pkgobj-typeparam-giventyped-giventyped"]
    if "package object pf:\n  export fd.Impl.{given Ordering[Option[Int]]}\n" not in lib["Holder.scala"] or "import fd.pf.{given Ordering[Option[Int]]}" not in use:
        bad.append("the package object's typed export of the parameterised given")
    lib, use = d["k-toplevel-nested-byname-given"]
    if lib["Holder.scala"] != "package fd.pf\n\nexport fd.Impl.Inner.gn\n" or "object Inner:\n    given gn" not in lib["Impl.scala"]:
        bad.append("the top-level file's export by name of a nested object's given")
    lib, use = d["s-object-alias-inner"]
    if "import fd.Other.given\n  locally {\n    import fd.pf.given\n" not in use:
        bad.append("the nested scope's import")
    lib, use = d["s-pkgobj-typeparam-target"]
    if "given go: Ordering[Option[Int]] = new Ordering[Option[Int]] { def compare(x: Option[Int], y: Option[Int]): Int = 7 }" not in lib["Other.scala"] or "import fd.Impl.given" not in use:
        bad.append("the scope family's competitor of the kind's type")
    lib, use = d["n-17-middle"]
    if "  given a17: a16.type = a16\n" not in lib["Chain.scala"] or "import fd.O.a8\n  import fd.O.a17\n" not in use:
        bad.append("the 17-link chain imported by its end and a middle link")
    lib, use = d["w-object-structural"]
    if "object pf:\n  given a: fd.Impl.gs.type = fd.Impl.gs\n" not in lib["Holder.scala"] or "import fd.Other.given" not in use:
        bad.append("the written singleton of a structural given")
    lib, use = d["w-toplevel-onevalue"]
    if lib["Holder.scala"] != "package fd.pf\n\ngiven x: Ordering[Int] = Ordering.Int.reverse\ngiven y: x.type = x\n" or "Other" in use:
        bad.append("the given that is another by its type")
    lib, use = d["t-toplevel-export-giventyped"]
    if lib["Holder.scala"] != "package fd.pf\n\nexport fd.ImplTwo.given\n" or "import fd.pf.{given Ordering[Int]}" not in use:
        bad.append("the two exported givens of one type")
    lib, use = d["p-nested"]
    if "package object qf:\n  export fd.Impl2.given" not in lib["Qf.scala"] or "import fd.pf.qf.given" not in use:
        bad.append("the second package object of the path")
    # The judge over planted tables: a turned, an unmoved, a regressed, a wrong error place, an ambiguity's wording, a
    # missing leg.
    s = {("a", b, "run"): "ok:1" for b in ("whole", "split")}
    s.update({("e", b, "run"): "err:Main.scala:6:3: Ambiguous given instances: both given instance gi" for b in ("whole", "split")})
    good = {("a", b, l): "ok:1" for b in ("whole", "split") for l in LEGS}
    good.update({("e", b, l): "err:Main.scala:6:3: ambiguous given instances for Ordering[Int]: gi, go" for b in ("whole", "split") for l in LEGS})
    wrong = {**good, ("a", "split", "js"): "ok:-1"}
    misplaced = {**good, ("e", "whole", "jvm"): "err:Main.scala:6:4: ambiguous given instances"}
    missing = {k: v for k, v in good.items() if k != ("a", "whole", "interp")}
    other_error = {**good, ("e", "split", "js"): "err:Main.scala:6:3: value gi is not a member of fd.pf"}
    null = open(os.devnull, "w")
    for what, base, head, code in (("all right", wrong, good, 0), ("a regressed leg", good, wrong, 1), ("an error's column", good, misplaced, 1),
                                   ("a missing leg", good, missing, 1), ("another error at scalac's place", good, other_error, 1)):
        if judge(["a", "e"], s, base, head, out=null) != code:
            bad.append(f"the judge on the planted table: {what}")
    if not wording(s[("e", "whole", "run")], good[("e", "whole", "jvm")]):
        bad.append("an ambiguity in teq's words not counted as wording")
    if bad:
        sys.exit("pkgobj_given_import self-test FAILED: " + "; ".join(bad))
    print(f"pkgobj_given_import self-test: {len(cs)} cells, the templates checked, the judge's planted tables judged as they must be")


self_test()
cmd = sys.argv[1:2]
if cmd == ["cells"]:
    print(f"pkgobj_given_import: {len(write_cells(sys.argv[2]))} cells in {sys.argv[2]}")
elif cmd == ["judge"]:
    d = sys.argv[2]
    names = [l.strip() for l in open(os.path.join(d, "cells.txt")) if l.strip()]
    tables, malformed = [], []
    for t in ("scalac", sys.argv[3], sys.argv[4]):
        rows, bad = read(os.path.join(d, t + ".tsv"))
        tables.append(rows)
        malformed += [f"{t}: {b}" for b in bad]
    code = judge(names, *tables)
    if malformed:
        print(f"  MALFORMED lines {malformed[:5]}")
        code = 1
    sys.exit(code)
elif cmd != ["--self-test"]:
    sys.exit(__doc__)
