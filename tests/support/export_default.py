#!/usr/bin/env python3
"""tests/support/export_default.py: the table of a method's omitted default argument called through an export.

A cell is a method `m` of `object lib.Codecs` (module m1), exported by a holder in module m2 (package util), called
by module m3 (package app) with a default omitted and its result printed; each module compiled apart over the
earlier ones' products and the three whole, by tests/support/export-inline.sh (the same cells' layout and legs as
tests/support/export_inline.py's). The cells are the product of
  the HOLDER and the path its export reads: a class instance (`val c`), an object, a trait's instance, an enum
    case, a given instance, a class's constructor parameter, `this.c`, a nested path (`w.c`), a path through
    another export;
  the METHOD's kind: inline and generic, inline, generic, plain, transparent inline;
  WHICH DEFAULT is omitted: the last, the first, the middle one, two, one that computes, one that reads an earlier
    clause's parameter, one with an effect (it prints the object it runs on), an inline parameter's (inline kinds
    only), and the control that passes the argument;
and four cells of the receiver, on the class and the object holder: an expression with an effect, and a second
export of the holder. Dotty: the exporting class holds the forwarder's default getter, `m$default$N = this.c.m$default$N`
(`Namer.addForwarder`), which a call that omits the argument applies on the call's receiver; an inline parameter's
getter is inline (`Desugar.defaultGetters`) and no class file holds it.

  export_default.py cells <out dir>                   the cells (<cell>/m1/A.scala, m2/U.scala, m3/B.scala), cells.txt
  export_default.py judge <cells dir> <base> <head>   the head's legs against scalac's and against the base's
  export_default.py --self-test

The judge, per cell and leg (the runner's LEGS): a cell of the part (PART: what the change turns) holds scalac's
output on the head; every other cell holds the base's verdict on the head (paths aside); every cell whose head is
not scalac's is listed by name with its line. A hole in the manifest fails, named; the judging still reads every row.
A self-test of planted tables, and of the templates, runs first.
"""
import collections
import itertools
import os
import re
import sys

KINDS = {
    "inlgen": ("inline ", "[A]", "A"),
    "inl": ("inline ", "", "Int"),
    "gen": ("", "[A]", "A"),
    "plain": ("", "", "Int"),
    "trans": ("transparent inline ", "[A]", "A"),
}
INLINE_KINDS = {"inlgen", "inl", "trans"}
# The default's place and kind: (the parameter lists after the name with T the first parameter's type, the call's
# arguments with A the first argument). The effect prints which object the default runs on.
DEFAULTS = {
    "last": ("(a: T, n: Int = 2): String = a.toString * n", "(A)"),
    "first": ("(n: Int = 2, a: T): String = a.toString * n", "(a = A)"),
    "middle": ("(a: T, n: Int = 2, s: String): String = a.toString * n + s", '(A, s = "!")'),
    "two": ('(a: T, n: Int = 2, s: String = "?"): String = a.toString * n + s', "(A)"),
    "reads": ("(a: T, n: Int = 1 + 1): String = a.toString * n", "(A)"),
    "readsearlier": ('(a: T, s: String = "x")(t: String = s * 2): String = a.toString + t', "(A)()"),
    "effect": ('(a: T, n: Int = { println("default on " + this); 2 }): String = a.toString * n', "(A)"),
    "inlparam": ("(a: T, inline n: Int = 2): String = a.toString * n", "(A)"),
    "passed": ("(a: T, n: Int = 2): String = a.toString * n", "(A, 2)"),
}
# The holder: m2's source, the expression m3 calls on.
HOLDERS = {
    "class": ('class Holder:\n  override def toString = "Holder"\n  val c: lib.Codecs.type = lib.Codecs\n  export c.*\nobject Utils:\n  val h = new Holder\n', "util.Utils.h"),
    "object": ('object Utils:\n  override def toString = "Utils"\n  export lib.Codecs.*\n', "util.Utils"),
    "trait": ('trait HolderT:\n  override def toString = "HolderT"\n  val c: lib.Codecs.type = lib.Codecs\n  export c.*\nobject Utils:\n  val h = new HolderT {}\n', "util.Utils.h"),
    "enumcase": ("enum E:\n  case A\n  val c: lib.Codecs.type = lib.Codecs\n  export c.*\n", "util.E.A"),
    "given": ('object Utils:\n  given h: AnyRef with\n    override def toString = "given h"\n    val c: lib.Codecs.type = lib.Codecs\n    export c.*\n', "util.Utils.h"),
    "param": ('class Holder(val c: lib.Codecs.type):\n  override def toString = "Holder"\n  export c.*\nobject Utils:\n  val h = new Holder(lib.Codecs)\n', "util.Utils.h"),
    "this": ('class Holder:\n  override def toString = "Holder"\n  val c: lib.Codecs.type = lib.Codecs\n  export this.c.*\nobject Utils:\n  val h = new Holder\n', "util.Utils.h"),
    "nested": ('class Wrap:\n  val c: lib.Codecs.type = lib.Codecs\nclass Holder:\n  override def toString = "Holder"\n  val w = new Wrap\n  export w.c.*\nobject Utils:\n  val h = new Holder\n', "util.Utils.h"),
    "throughexport": ('object Mid:\n  export lib.Codecs.*\nclass Holder:\n  override def toString = "Holder"\n  val c: Mid.type = Mid\n  export c.*\nobject Utils:\n  val h = new Holder\n', "util.Utils.h"),
}
# The holders whose export reads a path of the instance (`this.c`), which a call elsewhere must not read; teq refuses
# the given's and the `this` path's export at the source (not this table's: their cells are held to the base).
INSTANCE_HOLDERS = {"class", "trait", "enumcase", "given", "param", "this", "nested", "throughexport"}
REFUSED_AT_SOURCE = {"given", "this"}
# The receiver of the call: (m3's definitions before the call, the expression), H the holder's expression.
RECEIVERS = {
    "stable": ("", "H"),
    "effect": ('def mk() = { println("receiver evaluated"); H }\n', "mk()"),
    "secondexport": ("object Again:\n  val h = H\n  export h.*\n", "Again"),
}
LEGS = ["whole", "whole-jvm", "whole-js", "lean", "js", "jvm", "m23-lean", "m23-js", "m23-jvm", "readtasty"]
READ_LEGS = {"readtasty"}
# The legs that build the forwarder and its call together: the three whole ones, and m2 with m3 over m1's products.
TOGETHER_LEGS = {"whole", "whole-jvm", "whole-js", "m23-lean", "m23-js", "m23-jvm"}


def part(cell, leg):
    """Whether the change turns (cell, leg): an inline method's omitted default through an export reading an
    instance's path, where the forwarder and the call are built together (the whole builds, m2 with m3). An inline parameter's default is not this change's (no getter written
    for it, a forwarder's inline getter): its cells are held to the base."""
    holder, kind, default, _receiver = cell
    if default == "inlparam":
        return False
    return (holder in INSTANCE_HOLDERS and holder not in REFUSED_AT_SOURCE and kind in INLINE_KINDS and default != "passed"
            and leg in TOGETHER_LEGS)


def sources(holder, kind, default, receiver):
    mods, tps, t = KINDS[kind]
    params, call = DEFAULTS[default]
    m1 = 'package lib\n\nobject Codecs:\n  override def toString = "Codecs"\n  ' + mods + "def m" + tps + params.replace("T", t) + "\n"
    m2_src, h = HOLDERS[holder]
    pre, recv = RECEIVERS[receiver]
    arg = "3"
    m3 = ("package app\n\n" + pre.replace("H", h) + "\n@main def run(): Unit =\n  println(" + recv.replace("H", h) + ".m"
          + call.replace("A", arg) + ")\n")
    return m1, "package util\n\n" + m2_src, m3


def cells():
    out = []
    for holder, kind, default in itertools.product(HOLDERS, KINDS, DEFAULTS):
        if default == "inlparam" and kind not in INLINE_KINDS:
            continue
        out.append((holder, kind, default, "stable"))
    for holder in ("class", "object"):
        for receiver in ("effect", "secondexport"):
            out.append((holder, "inlgen", "last", receiver))
    return out


def name(c):
    return "-".join(c)


def parse(n):
    h, k, d, r = n.split("-")
    return (h, k, d, r)


def write_cells(out):
    names = []
    for c in cells():
        n = name(c)
        for sub, f, text in zip(("m1", "m2", "m3"), ("A.scala", "U.scala", "B.scala"), sources(*c)):
            os.makedirs(os.path.join(out, n, sub), exist_ok=True)
            open(os.path.join(out, n, sub, f), "w").write(text)
        names.append(n)
    open(os.path.join(out, "cells.txt"), "w").write("\n".join(names) + "\n")
    return names


def read(path):
    rows, seen = {}, collections.Counter()
    for line in open(path):
        line = line.rstrip("\n")
        if not line:
            continue
        parts = line.split("\t", 2)
        if len(parts) != 3:
            raise SystemExit(f"export_default: a truncated row in {path}: {line[:80]!r}")
        seen[(parts[0], parts[1])] += 1
        rows[(parts[0], parts[1])] = parts[2]
    return rows, seen


def same(a, b):
    """Two verdicts alike but for the runs' own directories (`teq-<tag>`)."""
    norm = lambda v: re.sub(r"/teq-[^/ ]+/", "/teq-TAG/", v)
    return norm(a) == norm(b)


def scalac_like(v, sep, leg):
    if leg in READ_LEGS:
        return v == "ok:read"
    return sep.startswith("ok:") and v == sep


def judge(names, scalac, scalac_seen, base, head, out=sys.stdout):
    problems = []
    for c in names:
        for leg in ("sep", "whole"):
            if (c, leg) not in scalac:
                problems.append(f"scalac lacks {c} {leg}")
        if scalac.get((c, "sep")) != scalac.get((c, "whole")):
            problems.append(f"scalac's apart and whole differ in {c}")
    problems += [f"scalac doubled {k}" for k, n in scalac_seen.items() if n > 1]
    for tag, (rows, seen) in (("base", base), ("head", head)):
        for c in names:
            for leg in LEGS:
                if (c, leg) not in rows:
                    problems.append(f"{tag} lacks {c} {leg}")
        problems += [f"{tag} doubled {k}" for k, n in seen.items() if n > 1]
        problems += [f"{tag} stray {k[0]}" for k in rows if k[0] not in names]
    counts = collections.Counter()
    fails, listed = [], []
    for c in names:
        cell = parse(c)
        sep = scalac.get((c, "sep"), "err:missing")
        for leg in LEGS:
            b = base[0].get((c, leg), "err:missing")
            h = head[0].get((c, leg), "err:missing")
            ok_h, ok_b = scalac_like(h, sep, leg), scalac_like(b, sep, leg)
            if part(cell, leg):
                if ok_h:
                    counts["part turned" if not ok_b else "part already scalac's"] += 1
                else:
                    counts["part NOT scalac's"] += 1
                    fails.append(f"PART {c} {leg}: head {h[:160]} | scalac {sep[:60]}")
            elif same(h, b):
                counts["kept as the base"] += 1
                if not ok_h:
                    listed.append(f"{c} {leg}: {h[:160]}")
            else:
                counts["MOVED from the base"] += 1
                fails.append(f"MOVED {c} {leg}: head {h[:120]} | base {b[:120]}")
    print("export_default: " + "; ".join(f"{k} {n}" for k, n in sorted(counts.items())), file=out)
    for f in fails:
        print("  FAIL " + f, file=out)
    for l in listed:
        print("  NOT SCALAC'S, AS THE BASE " + l, file=out)
    if problems:
        print(f"FAILED the manifest: {len(problems)}: " + "; ".join(problems[:20]), file=out)
    print(f"export_default read every row: {len(names)} cells, {len(LEGS)} legs, {len(fails)} failing, "
          f"{len(listed)} listed, manifest problems {len(problems)}", file=out)
    return 1 if problems or fails else 0


def self_test():
    import io
    bad = []
    cs = cells()
    if len(cs) != len(set(map(name, cs))):
        bad.append("a cell twice")
    if any(d == "inlparam" and k not in INLINE_KINDS for _, k, d, _ in cs):
        bad.append("an inline parameter in a method that is not inline")
    a, u, b = sources("class", "inlgen", "first", "stable")
    if "inline def m[A](n: Int = 2, a: A)" not in a or "println(util.Utils.h.m(a = 3))" not in b or "export c.*" not in u:
        bad.append("the class holder's first-default cell")
    a, u, b = sources("object", "inl", "inlparam", "effect")
    if "inline def m(a: Int, inline n: Int = 2)" not in a or "println(mk().m(3))" not in b:
        bad.append("the object's inline-parameter cell with an effect")
    if not part(("class", "inlgen", "last", "stable"), "whole") or part(("class", "inlgen", "last", "stable"), "lean"):
        bad.append("the part's first shape")
    if part(("object", "inlgen", "last", "stable"), "whole") or part(("class", "gen", "last", "stable"), "whole") or part(("class", "inlgen", "passed", "stable"), "whole") or part(("given", "inlgen", "last", "stable"), "whole"):
        bad.append("a cell outside the part counted in it")
    if part(("class", "inl", "inlparam", "stable"), "whole") or part(("object", "inl", "inlparam", "stable"), "jvm"):
        bad.append("an inline parameter's cell counted in the part")
    # The judge, over planted tables of two cells: one of the part, one outside it.
    pc, oc = "class-inlgen-last-stable", "object-inlgen-last-stable"
    names = [pc, oc]
    sc = {(c, leg): "ok:33" for c in names for leg in ("sep", "whole")}
    base = {(c, leg): ("ok:read" if leg in READ_LEGS else "ok:33") for c in names for leg in LEGS}
    base.update({(pc, leg): "err:Unit has no member c" for leg in TOGETHER_LEGS})
    head = dict(base)
    head.update({(pc, leg): "ok:33" for leg in TOGETHER_LEGS})

    def run(h, b=base, s=sc):
        buf = io.StringIO()
        code = judge(names, s, collections.Counter(s.keys()), (b, collections.Counter(b.keys())), (h, collections.Counter(h.keys())), buf)
        return code, buf.getvalue()
    code, text = run(head)
    if code != 0 or f"part turned {len(TOGETHER_LEGS)}" not in text:
        bad.append("the good table")
    h = dict(head); h[(pc, "whole-js")] = "err:TypeError"
    if run(h)[0] != 1 or f"FAIL PART {pc} whole-js" not in run(h)[1]:
        bad.append("a part's cell not turned")
    h = dict(head); h[(oc, "jvm")] = "ok:34"
    if run(h)[0] != 1 or f"FAIL MOVED {oc} jvm" not in run(h)[1]:
        bad.append("a cell outside the part moved")
    b = dict(base); h = dict(head); b[(oc, "readtasty")] = "err:/x/teq-base/o/y2 refused"; h[(oc, "readtasty")] = "err:/x/teq-head/o/y2 refused"
    code, text = run(h, b)
    if code != 0 or f"NOT SCALAC'S, AS THE BASE {oc} readtasty" not in text:
        bad.append("a difference the base has too not listed, or its paths not set aside")
    h = dict(head); del h[(oc, "lean")]
    if run(h)[0] != 1 or "head lacks" not in run(h)[1]:
        bad.append("a missing leg not failed")
    s = dict(sc); s[(oc, "whole")] = "ok:34"
    if run(head, base, s)[0] != 1:
        bad.append("scalac's apart and whole differing not failed")
    if bad:
        sys.exit("export_default self-test FAILED: " + "; ".join(bad))
    print(f"export_default self-test: {len(cs)} cells, the templates and the part checked, 6 planted tables judged as they must be")


self_test()
cmd = sys.argv[1:2]
if cmd == ["cells"]:
    print(f"export_default: {len(write_cells(sys.argv[2]))} cells in {sys.argv[2]}")
elif cmd == ["judge"]:
    d = sys.argv[2]
    names = [l.strip() for l in open(os.path.join(d, "cells.txt")) if l.strip()]
    scalac, scalac_seen = read(os.path.join(d, "scalac.tsv"))
    sys.exit(judge(names, scalac, scalac_seen, read(os.path.join(d, sys.argv[3] + ".tsv")), read(os.path.join(d, sys.argv[4] + ".tsv"))))
elif cmd != ["--self-test"]:
    sys.exit(__doc__)
