#!/usr/bin/env python3
"""tests/support/export_inline.py: the table of inline methods called through an export from a third module.

A cell is one method of an object `lib.Codecs` (module m1), exported by module m2 (package util), called by module
m3 (package app), each module compiled apart over the earlier ones' products (scalac: over their class files and
TASTy) and the three whole. The shapes: one type parameter, two, a bounded one, a context bound and an aggregated
one, a using clause, a default argument, a by-name parameter, an inline parameter, an inline parameter with a default,
a transparent inline method, and the controls a non-generic inline method and a generic method that is not inline. The forms: `export lib.Codecs.*`,
`export lib.Codecs.{m, given}`, a renamed export, an export of an export, and an export inside a class instance (a
path-dependent forwarder over a val). Dotty's forwarder of an inline member is inline, its body the member applied
to the forwarder's parameter lists, its type parameters first (`Namer.addForwarder`).

  export_inline.py cells <out dir>             the 65 cells (<cell>/m1/A.scala, m2/U.scala, m3/B.scala) and cells.txt
  export_inline.py cases <tests/modules dir>   the modules cases exportinline<form>: one a form, its shapes in it
                                               but those CASE_LEAVES_OUT names, the object and the export in a/,
                                               the calls in b/
  export_inline.py judge <cells dir> <tag>...  each teq leg of tests/support/export-inline.sh (<cells>/<tag>.tsv)
                                               against scalac's separate compilation (<cells>/scalac.tsv)
  export_inline.py --self-test

The judge: `=` where teq's output is scalac's, `ERR` where teq refused or threw and scalac ran, `DIFF` where both ran
and differ, `both-err` where neither ran; on a reading leg (READ_LEGS) `=` where scalac read teq's pickles and `ERR`
where it did not; on a leg KEPT_REFUSALS names, `kept` where teq refused with the products'
withholding line and `NOT-KEPT` otherwise. The manifest first: every cell has every leg in every file, none twice,
none a cell the list lacks; a hole fails, named, and the judging still reads every row. A self-test of planted
tables, and of the templates, runs first.
"""
import collections
import os
import sys

# The type classes the context bounds and the using clause take: instances in their companions (implicit scope),
# so that no form needs an export of givens.
PRELUDE = '''trait Show[A] { def show(a: A): String }
object Show:
  given Show[Int] = a => "#" + a
trait Size[A] { def size(a: A): Int }
object Size:
  given Size[Int] = a => a * 10
'''

# name: (the method in object Codecs, the call printed, NAME where the called name stands)
SHAPES = {
    "t1": ("inline def NAME[A](a: A): A = a", "NAME(3)"),
    "t2": ("inline def NAME[A, B](a: A, b: B): String = a.toString + b.toString", 'NAME(1, "a")'),
    "bnd": ("inline def NAME[A <: AnyVal](a: A): String = a.toString + \"!\"", "NAME(3)"),
    "cbo": ("inline def NAME[A: Show](a: A): String = summon[Show[A]].show(a)", "NAME(3)"),
    "cba": ("inline def NAME[A: {Show, Size}](a: A): String = summon[Show[A]].show(a) + summon[Size[A]].size(a)", "NAME(3)"),
    "usg": ("inline def NAME[A](a: A)(using s: Show[A]): String = s.show(a)", "NAME(3)"),
    "dft": ("inline def NAME[A](a: A, n: Int = 2): String = a.toString * n", "NAME(3)"),
    "bnm": ("inline def NAME[A](a: => A): A = a", "NAME(3)"),
    "inp": ("inline def NAME[A](inline a: A): A = a", "NAME(3)"),
    "ipd": ("inline def NAME[A](a: A, inline n: Int = 2): String = a.toString * n", "NAME(3)"),
    "tri": ("transparent inline def NAME[A](a: A): A = a", "NAME(3) + 1"),
    "plain": ("inline def NAME(x: Int): Int = x + 1", "NAME(3)"),
    "noinl": ("def NAME[A](a: A): A = a", "NAME(3)"),
}

# name: (m2's body with EXPORTED the export's selectors or wildcard, m3's import, the name m3 calls)
FORMS = {
    "star": ("object Utils:\n  export lib.Codecs.*\n", "import util.Utils.*", "NAME"),
    "sel": ("object Utils:\n  export lib.Codecs.{SELECTORS, given}\n", "import util.Utils.*", "NAME"),
    "ren": ("object Utils:\n  export lib.Codecs.{RENAMES}\n", "import util.Utils.*", "NAME2"),
    "exex": ("object Inner:\n  export lib.Codecs.*\nobject Utils:\n  export Inner.*\n", "import util.Utils.*", "NAME"),
    "cls": ("class Holder:\n  val c: lib.Codecs.type = lib.Codecs\n  export c.*\nobject Utils:\n  val h = new Holder\n",
            "import util.Utils.h", "h.NAME"),
}

LEGS = ["whole", "whole-jvm", "whole-js", "lean", "js", "jvm", "m23-lean", "m23-js", "m23-jvm", "readtasty"]
# The legs that read rather than run: scalac's reader over teq's pickles, `ok:read` where it read them.
READ_LEGS = {"readtasty"}

# The shapes the modules cases leave out, each for a difference of its own that a forwarder of a member that is not
# generic shows as well, and that the table's runs do not show: an inline parameter's (`inp`: the forwarder's
# parameter pickled without `inline` where the upstream member is read from its products, so that the split
# build's pickle is not the whole build's), a default argument's (`dft`: the forwarder of the member's default
# getter pickled without its type arguments, which scalac's tree checker refuses on reading; and, through an export
# inside a class instance, the omitted default evaluated on the caller's object in a whole build), and a
# transparent method's (`tri`: its expansion over the products places the forwarder's call at the call site, where
# the whole build places it at the export); and, in the class form, the method that is not inline (`noinl`: the
# JavaScript build over the products calls the module where the whole build calls the forwarder's path,
# `this.c.m(a)`, a bundle of other bytes). The table runs them.
# The shape whose legs over the products keep the refusal (`ipd`: an inline parameter with a default, the default
# omitted): the forwarder's body stays withheld, as without its type arguments, because its getter of that parameter
# applies the member's, which no one writes (an inline parameter's getter is inline, `Desugar.defaultGetters`). The
# whole builds expand the default at the call. The cases leave it out too.
KEPT_REFUSALS = {"ipd": ("lean", "js", "jvm")}
KEPT_LINE = "is withheld from the products of"
CASE_LEAVES_OUT = {f: ["inp", "dft", "tri", "ipd"] for f in FORMS}
CASE_LEAVES_OUT["cls"] = CASE_LEAVES_OUT["cls"] + ["noinl"]


def method_name(shape):
    return "m" + shape


def sources(shapes, form):
    """The three modules' sources of `form` over `shapes`: (m1, m2, m3), m3 printing each call on a line."""
    m2, imp, called = FORMS[form]
    names = [method_name(s) for s in shapes]
    m2 = m2.replace("SELECTORS", ", ".join(names)).replace("RENAMES", ", ".join(f"{n} as {n}2" for n in names))
    a = "package lib\n\n" + PRELUDE + "\nobject Codecs:\n" + "".join(
        "  " + SHAPES[s][0].replace("NAME", method_name(s)) + "\n" for s in shapes)
    u = "package util\n\n" + m2
    calls = [SHAPES[s][1].replace("NAME", called.replace("NAME", method_name(s))) for s in shapes]
    b = "package app\n\n" + imp + "\n\n@main def run(): Unit =\n" + "".join(f"  println({c})\n" for c in calls)
    return a, u, b


def write_cells(out):
    names = []
    for shape in SHAPES:
        for form in FORMS:
            c = f"{shape}-{form}"
            for sub, f, text in zip(("m1", "m2", "m3"), ("A.scala", "U.scala", "B.scala"), sources([shape], form)):
                os.makedirs(os.path.join(out, c, sub), exist_ok=True)
                open(os.path.join(out, c, sub, f), "w").write(text)
            names.append(c)
    open(os.path.join(out, "cells.txt"), "w").write("\n".join(names) + "\n")
    return names


def write_cases(modules):
    """Two modules a case: the object and its export in a (A.scala, U.scala), the calls in b (B.scala): the
    forwarders' bodies are read from a's products, as the third module of a cell reads them from the second's."""
    names = []
    for form in FORMS:
        shapes = [s for s in SHAPES if s not in CASE_LEAVES_OUT.get(form, [])]
        case = os.path.join(modules, "exportinline" + form)
        for sub, f, text in zip(("a", "a", "b"), ("A.scala", "U.scala", "B.scala"), sources(shapes, form)):
            os.makedirs(os.path.join(case, sub), exist_ok=True)
            open(os.path.join(case, sub, f), "w").write(text)
        names.append("exportinline" + form)
    return names


def read(path):
    rows, seen = {}, collections.Counter()
    for line in open(path):
        line = line.rstrip("\n")
        if not line:
            continue
        parts = line.split("\t", 2)
        if len(parts) != 3:
            raise SystemExit(f"export_inline: a truncated row in {path}: {line[:80]!r}")
        cell, leg, verdict = parts
        seen[(cell, leg)] += 1
        rows[(cell, leg)] = verdict
    return rows, seen


def mark(teq, scalac, leg=None):
    if leg in READ_LEGS:
        return "=" if teq == "ok:read" else "ERR"
    if scalac.startswith("ok:"):
        if teq.startswith("ok:"):
            return "=" if teq == scalac else "DIFF"
        return "ERR"
    return "both-err" if not teq.startswith("ok:") else "teq-only"


def judge(cells, scalac, scalac_seen, tags, out=sys.stdout):
    problems = []
    for c in cells:
        for leg in ("sep", "whole"):
            if (c, leg) not in scalac:
                problems.append(f"scalac lacks {c} {leg}")
    problems += [f"scalac doubled {k}" for k, n in scalac_seen.items() if n > 1]
    problems += [f"scalac stray {k[0]}" for k in scalac if k[0] not in cells]
    for tag, (rows, seen) in tags.items():
        for c in cells:
            for leg in LEGS:
                if (c, leg) not in rows:
                    problems.append(f"{tag} lacks {c} {leg}")
        problems += [f"{tag} doubled {k}" for k, n in seen.items() if n > 1]
        problems += [f"{tag} stray {k[0]}" for k in rows if k[0] not in cells]
    counts = collections.Counter()
    off = []
    for c in cells:
        sep = scalac.get((c, "sep"), "err:missing")
        line = [c, f"scalac sep {sep[:60]}", f"whole {scalac.get((c, 'whole'), 'err:missing')[:30]}"]
        for tag, (rows, _) in tags.items():
            for leg in LEGS:
                v = rows.get((c, leg), "err:missing")
                m = mark(v, sep, leg)
                if leg in KEPT_REFUSALS.get(c.split("-")[0], ()):
                    m = "kept" if not v.startswith("ok:") and KEPT_LINE in v else "NOT-KEPT"
                counts[(tag, leg, m)] += 1
                line.append(f"{tag}/{leg} {m}")
                if m not in ("=", "both-err", "kept"):
                    off.append(f"{c} {tag}/{leg} {m}: teq {v[:160]} | scalac {sep[:80]}")
        print("  ".join(line), file=out)
    for tag in tags:
        print(f"{tag}: " + "; ".join(f"{leg} " + ", ".join(f"{m} {counts[(tag, leg, m)]}" for m in ("=", "ERR", "DIFF", "both-err", "teq-only", "kept", "NOT-KEPT") if counts[(tag, leg, m)]) for leg in LEGS), file=out)
    for o in off:
        print("  OFF " + o, file=out)
    if problems:
        print(f"FAILED the manifest: {len(problems)}: " + "; ".join(problems), file=out)
    print(f"export_inline read every row: {len(cells)} cells, {len(tags)} tags, manifest problems {len(problems)}", file=out)
    return 1 if problems else 0


def self_test():
    import io
    bad = []
    # The templates: every shape names its method and its call, every form reaches what it exports.
    for s, (d, call) in SHAPES.items():
        if "NAME" not in d or "NAME" not in call:
            bad.append(f"the shape {s} without its name")
    for f, (m2, imp, called) in FORMS.items():
        if "NAME" not in called or not imp.startswith("import util.Utils"):
            bad.append(f"the form {f} reaches nothing")
    a, u, b = sources(["t1", "t2"], "ren")
    if "mt1 as mt12, mt2 as mt22" not in u or "println(mt12(3))" not in b:
        bad.append("the renamed form does not call the renamed members")
    a, u, b = sources(["t1", "plain"], "sel")
    if "export lib.Codecs.{mt1, mplain, given}" not in u:
        bad.append("the selecting form does not select every member")
    a, u, b = sources(["tri"], "cls")
    if "println(h.mtri(3) + 1)" not in b or "transparent inline def mtri" not in a:
        bad.append("the class form's call or the transparent shape")
    # The judge, over planted tables.
    cells = ["a", "b"]
    sc = {("a", "sep"): "ok:3", ("a", "whole"): "ok:3", ("b", "sep"): "ok:4", ("b", "whole"): "ok:4"}
    good = {(c, leg): ("ok:read" if leg in READ_LEGS else sc[(c, "sep")]) for c in cells for leg in LEGS}

    def run(rows, scalac=sc):
        buf = io.StringIO()
        code = judge(cells, scalac, collections.Counter(scalac.keys()), {"t": (rows, collections.Counter(rows.keys()))}, buf)
        return code, buf.getvalue()
    code, text = run(good)
    if code != 0 or "OFF" in text:
        bad.append("the good table")
    r = dict(good); r[("a", "lean")] = "err:withheld"
    if "OFF a t/lean ERR" not in run(r)[1]:
        bad.append("a refusal not named ERR")
    r = dict(good); r[("b", "jvm")] = "ok:5"
    if "OFF b t/jvm DIFF" not in run(r)[1]:
        bad.append("another output not named DIFF")
    r = dict(good); del r[("a", "js")]
    code, text = run(r)
    if code != 1 or "t lacks a js" not in text or "OFF a t/js ERR" not in text:
        bad.append("a missing leg not failed and judged")
    s = dict(sc); del s[("b", "sep")]
    if run(good, s)[0] != 1:
        bad.append("a missing scalac row not failed")
    r = dict(good); r[("c", "whole")] = "ok:1"
    if run(r)[0] != 1:
        bad.append("a stray cell not failed")
    r = dict(good); r[("b", "readtasty")] = "err:-Ycheck: a tree of the wrong type"
    if "OFF b t/readtasty ERR" not in run(r)[1]:
        bad.append("a pickle scalac's reader refuses not named ERR")
    # A leg that keeps the refusal: the withholding line is `kept`; a run, or another refusal, is `NOT-KEPT`.
    kc = ["ipd-star"]
    ksc = {("ipd-star", "sep"): "ok:33", ("ipd-star", "whole"): "ok:33"}
    kgood = {("ipd-star", leg): ("ok:read" if leg in READ_LEGS else "ok:33") for leg in LEGS if leg not in KEPT_REFUSALS["ipd"]}
    kgood.update({("ipd-star", leg): "err:m3/B.scala:5:11: error: the body of util.Utils.mipd " + KEPT_LINE + " /x"
                  for leg in KEPT_REFUSALS["ipd"]})

    def krun(rows):
        buf = io.StringIO()
        judge(kc, ksc, collections.Counter(ksc.keys()), {"t": (rows, collections.Counter(rows.keys()))}, buf)
        return buf.getvalue()
    if "OFF" in krun(kgood) or "t/lean kept" not in krun(kgood):
        bad.append("a kept refusal not judged kept")
    r = dict(kgood); r[("ipd-star", "jvm")] = "ok:33"
    if "OFF ipd-star t/jvm NOT-KEPT" not in krun(r):
        bad.append("a run where the refusal is kept not named NOT-KEPT")
    r = dict(kgood); r[("ipd-star", "js")] = "err:TypeError: Codecs$(...).mipd$default$2 is not a function"
    if "OFF ipd-star t/js NOT-KEPT" not in krun(r):
        bad.append("another failure where the refusal is kept not named NOT-KEPT")
    if bad:
        sys.exit("export_inline self-test FAILED: " + "; ".join(bad))
    print("export_inline self-test: the templates of 13 shapes and 5 forms, 10 planted tables judged as they must be")


self_test()
cmd = sys.argv[1:2]
if cmd == ["cells"]:
    print(f"export_inline: {len(write_cells(sys.argv[2]))} cells in {sys.argv[2]}")
elif cmd == ["cases"]:
    print("export_inline: the cases " + " ".join(write_cases(sys.argv[2])))
elif cmd == ["judge"]:
    d = sys.argv[2]
    cells = [l.strip() for l in open(os.path.join(d, "cells.txt")) if l.strip()]
    scalac, scalac_seen = read(os.path.join(d, "scalac.tsv"))
    tags = {t: read(os.path.join(d, t + ".tsv")) for t in sys.argv[3:]}
    sys.exit(judge(cells, scalac, scalac_seen, tags))
elif cmd != ["--self-test"]:
    sys.exit(__doc__)
