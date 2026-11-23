#!/usr/bin/env python3
"""tests/support/inline_getters.py: the table of an inline definition's ORDINARY default argument (not an inline
parameter's), whose getter `Desugar.addDefaultGetters` makes a method of its own that a call applies.

A cell is one program: a DEFINITION KIND that is inline and carries an ordinary parameter with a default (an inline
def, a transparent inline def, an inline given alias with a defaulted using clause, a transparent one, an inline
conditional given, an inline extension method, an inline method of an opaque type's companion), in a HOLDER (a class,
an object, a trait, an enum, a given instance, a package object, the top level), REACHED directly, through an export
of an instance (`export h.*`, `export h.given`), or through an import of an instance (`import h.*`, `import
h.given`), the call leaving the default out and its result printed. Run by tests/support/inline-getters.sh: scalac
3.8.4 compiles and runs each cell and lists its class files' default getters (javap); teq runs it in the interpreter,
on the JVM and in JavaScript, and lists its JVM products' default getters.

  inline_getters.py cells <out dir>                 the cells (<cell>/Main.scala) and cells.txt
  inline_getters.py judge <cells dir> <base> <head> the head against scalac's line and getters; the base's right cells
                                                    unchanged on the head (inline-getters.sh's `same` rows); every
                                                    cell not scalac's listed by name
  inline_getters.py same <base classes> <head classes>  `same` where every class file of the head is the base's byte
                                                    for byte, or differs from it only by default getters it adds
                                                    (javap -c -p, the getters' methods set aside); `differs: <class>`
  inline_getters.py --self-test

A cell scalac does not compile is no program of the table: listed as such, judged for nothing.
"""
import collections
import itertools
import os
import re
import sys

# kind: (the definition, with NAME its name; the call's selection after the prefix, with NAME; whether it is a given)
KINDS = {
    "idef": ("inline def NAME(a: Int, n: Int = 2): String = a.toString * n", "NAME(3)", False),
    "tdef": ("transparent inline def NAME(a: Int, n: Int = 2): String = a.toString * n", "NAME(3)", False),
    "igiven": ('inline given NAME(using n: Int = 2): String = "g" * n', None, True),
    "tgiven": ('transparent inline given NAME(using n: Int = 2): String = "g" * n', None, True),
    "icond": ("inline given NAME[A](using n: Int = 2): (A => String) = a => a.toString * n", None, True),
    "iext": ("extension (x: Int) inline def NAME(n: Int = 2): String = x.toString * n", None, False),
    "iopaque": ("opaque type T = Int\nobject T:\n  inline def NAME(a: Int, n: Int = 2): String = a.toString * n", "T.NAME(3)", False),
}
# holder: (the source, with D the definition indented two; the instance's expression; an object path or None)
HOLDERS = {
    "class": ("class H:\n  D\n", "new H", None),
    "object": ("object H:\n  D\n", "H", "H"),
    "trait": ("trait H:\n  D\nobject HI extends H\n", "HI", "HI"),
    "enum": ("enum H:\n  case A\n  D\n", "H.A", None),
    "given": ("given h: AnyRef with\n  D\n", "h", "h"),
    "pkgobj": ("package object hp:\n  D\n", None, "hp"),
    "toplevel": ("D\n", None, None),
}
REACHES = ["direct", "export", "import"]


def indent(d, k):
    return d.replace("\n", "\n" + " " * k)


def summon_of(kind):
    return {"igiven": "summon[String]", "tgiven": "summon[String]", "icond": "summon[Int => String](3)"}[kind]


def program(kind, holder, reach):
    """The cell's source, or None where the cell has no form (a reach the holder cannot give)."""
    d, call, given = KINDS[kind]
    d = d.replace("NAME", "f")
    hsrc, inst, path = HOLDERS[holder]
    body = hsrc.replace("D", indent(d, 2)) if holder != "toplevel" else d + "\n"
    if kind == "iext":
        sel = "3.f()"
    elif kind == "iopaque":
        sel = "T.f(3)"
    else:
        sel = None if given else "f(3)"
    lines = []
    # The prefix the call reaches the definition through.
    # A package is no prefix of a wildcard export (scalac's implementation restriction).
    if holder == "pkgobj" and reach == "export":
        return None
    if holder == "toplevel":
        if reach != "direct":
            return None
        expr = "f[Int](3)" if kind == "icond" else (summon_of(kind) if given else sel)
        return body + "\n@main def run(): Unit =\n  println(" + expr + ")\n"
    if reach == "direct":
        if holder == "pkgobj":
            pre, prefix = "", "hp"
        elif path:
            pre, prefix = "", path
        else:
            pre, prefix = "", "inst"
            lines.append(f"  val inst = {inst}")
        if kind == "icond":
            expr = f"{prefix}.f[Int](3)"
        elif given:
            expr = f"{prefix}.f"
        elif kind == "iext":
            lines.append(f"  import {prefix}.*")
            expr = sel
        else:
            expr = f"{prefix}.{sel}"
    elif reach == "import":
        target = "hp" if holder == "pkgobj" else (path or "inst")
        if target == "inst":
            lines.append(f"  val inst = {inst}")
        lines.append(f"  import {target}.{'given' if given else '*'}")
        expr = summon_of(kind) if given else sel
    else:  # export: a class holding the instance (or the object) and exporting it
        sel_kind = "given" if given else "*"
        body += f"class W:\n  val h0 = {path or inst}\n  export h0.{sel_kind}\n"
        lines.append("  val w = new W")
        if given:
            lines.append("  import w.given")
            expr = summon_of(kind)
        elif kind == "iext":
            lines.append("  import w.*")
            expr = sel
        else:
            expr = f"w.{sel}"
    return body + "\n@main def run(): Unit =\n" + "".join(l + "\n" for l in lines) + "  println(" + expr + ")\n"


def cells():
    out = []
    for k, h, r in itertools.product(KINDS, HOLDERS, REACHES):
        if program(k, h, r) is not None:
            out.append((k, h, r))
    return out


def name(c):
    return "-".join(c)


def write_cells(d):
    names = []
    for c in cells():
        n = name(c)
        os.makedirs(os.path.join(d, n), exist_ok=True)
        open(os.path.join(d, n, "Main.scala"), "w").write(program(*c))
        names.append(n)
    open(os.path.join(d, "cells.txt"), "w").write("\n".join(names) + "\n")
    return names


def read(path):
    rows, seen = {}, collections.Counter()
    for line in open(path):
        line = line.rstrip("\n")
        if not line:
            continue
        p = line.split("\t", 2)
        if len(p) != 3:
            raise SystemExit(f"inline_getters: a truncated row in {path}: {line[:80]!r}")
        seen[(p[0], p[1])] += 1
        rows[(p[0], p[1])] = p[2]
    return rows, seen


RUN_LEGS = ["interp", "jvm", "js"]


def judge(names, scalac, base, head, out=sys.stdout):
    problems = []
    for c in names:
        for leg in ("run", "getters"):
            if (c, leg) not in scalac:
                problems.append(f"scalac lacks {c} {leg}")
        for tag, rows in (("base", base), ("head", head)):
            for leg in RUN_LEGS + ["getters", "same"]:
                if tag == "base" and leg == "same":
                    continue
                if (c, leg) not in rows:
                    problems.append(f"{tag} lacks {c} {leg}")
    counts = collections.Counter()
    fails, listed, programs = [], [], 0
    for c in names:
        sc = scalac.get((c, "run"), "err:missing")
        if not sc.startswith("ok:"):
            listed.append(f"{c}: scalac does not run it: {sc[:120]}")
            counts["no program"] += 1
            continue
        programs += 1
        sg = scalac.get((c, "getters"), "")
        right_base = all(base.get((c, l)) == sc for l in RUN_LEGS) and base.get((c, "getters")) == sg
        right_head = all(head.get((c, l)) == sc for l in RUN_LEGS) and head.get((c, "getters")) == sg
        same = head.get((c, "same"), "")
        if right_base:
            # Master was right: the head must not move (inline-getters.sh's `same`: the run equal on the three
            # targets, the JVM's class files equal but the getters added).
            if same != "same" or not right_head:
                fails.append(f"MOVED {c}: head {[head.get((c, l), '')[:40] for l in RUN_LEGS]} same={same[:80]}")
                counts["MOVED"] += 1
            else:
                counts["right on master, unmoved"] += 1
        elif right_head:
            counts["turned"] += 1
        else:
            counts["not scalac's"] += 1
            listed.append(f"{c}: head {[head.get((c, l), '')[:50] for l in RUN_LEGS]} getters {head.get((c, 'getters'), '')} | scalac {sc[:40]} getters {sg}")
    print("inline_getters: " + "; ".join(f"{k} {n}" for k, n in sorted(counts.items())), file=out)
    for f in fails:
        print("  FAIL " + f, file=out)
    for l in listed:
        print("  LISTED " + l, file=out)
    if problems:
        print(f"FAILED the manifest: {len(problems)}: " + "; ".join(problems[:20]), file=out)
    print(f"inline_getters read every row: {len(names)} cells ({programs} programs), {len(fails)} failing, {len(listed)} listed, manifest problems {len(problems)}", file=out)
    return 1 if problems or fails else 0


def javap_without_getters(d, cls):
    import subprocess
    r = subprocess.run(["javap", "-c", "-p", "-cp", d, cls], capture_output=True, text=True)
    out, skip = [], False
    for line in r.stdout.splitlines():
        # A member's header is indented two, its code more: a getter's header and code set aside; a line of the class
        # itself (its header, its closing brace) ends a member.
        if not line.startswith("  "):
            skip = False
        elif not line.startswith("   "):
            skip = "$default$" in line
        if not skip and line.strip():
            # A constant pool index shifts with the getters' entries: set aside.
            out.append(re.sub(r"#\d+", "#", line))
    return out


def same(base_dir, head_dir):
    def classes(d):
        res = {}
        for root, _, fs in os.walk(d):
            for f in fs:
                if f.endswith(".class"):
                    p = os.path.join(root, f)
                    res[os.path.relpath(p, d)] = open(p, "rb").read()
        return res
    b, h = classes(base_dir), classes(head_dir)
    diffs = [f"only in base {k}" for k in sorted(set(b) - set(h))] + [f"only in head {k}" for k in sorted(set(h) - set(b))]
    for k in sorted(set(b) & set(h)):
        if b[k] == h[k]:
            continue
        cls = k[:-len(".class")].replace(os.sep, ".")
        if javap_without_getters(base_dir, cls) != javap_without_getters(head_dir, cls):
            diffs.append(f"{k} beyond its getters")
    return "same" if not diffs else "differs: " + ", ".join(diffs)


def self_test():
    import io
    bad = []
    cs = cells()
    if len(cs) != len(set(map(name, cs))):
        bad.append("a cell twice")
    p = program("igiven", "class", "export")
    if "val h0 = new H" not in p or "export h0.given" not in p or "import w.given" not in p or "inline given f(using n: Int = 2)" not in p:
        bad.append("the given's export cell")
    p = program("idef", "object", "direct")
    if "println(H.f(3))" not in p:
        bad.append("the object's direct cell")
    if program("idef", "toplevel", "export") is not None:
        bad.append("a top-level export cell made")
    if "println(H.f[Int](3))" not in program("icond", "object", "direct") or program("idef", "pkgobj", "export") is not None:
        bad.append("the conditional given's body or the package object's export")
    names = ["a", "b", "c"]
    sc = {(n, "run"): "ok:33" for n in names}
    sc.update({(n, "getters"): "H.f$default$2" for n in names})
    base = {(n, l): "ok:33" for n in names for l in RUN_LEGS}
    base.update({(n, "getters"): "H.f$default$2" for n in names})
    base[("b", "jvm")] = "err:NoSuchMethodError"
    base[("b", "getters")] = ""
    head = dict(base)
    head.update({(n, "same"): "same" for n in names})
    head[("b", "jvm")] = "ok:33"
    head[("b", "getters")] = "H.f$default$2"
    head[("b", "same")] = "differs: H.class"

    def run(h, b=base, s=sc):
        buf = io.StringIO()
        return judge(names, s, b, h, buf), buf.getvalue()
    code, text = run(head)
    if code != 0 or "turned 1" not in text or "right on master, unmoved 2" not in text:
        bad.append("the good table")
    h = dict(head); h[("a", "same")] = "differs: H.class (a method other than the getters)"
    if run(h)[0] != 1 or "FAIL MOVED a" not in run(h)[1]:
        bad.append("a right cell's class file moved")
    h = dict(head); h[("c", "js")] = "ok:3"
    if run(h)[0] != 1:
        bad.append("a right cell's JavaScript moved")
    h = dict(head); h[("b", "interp")] = "err:x"
    code, text = run(h)
    if code != 0 or "LISTED b" not in text:
        bad.append("a cell not scalac's not listed")
    h = dict(head); del h[("c", "getters")]
    if run(h)[0] != 1:
        bad.append("a missing row not failed")
    if bad:
        sys.exit("inline_getters self-test FAILED: " + "; ".join(bad))
    print(f"inline_getters self-test: {len(cs)} cells, the templates checked, 5 planted tables judged as they must be")


self_test()
cmd = sys.argv[1:2]
if cmd == ["cells"]:
    print(f"inline_getters: {len(write_cells(sys.argv[2]))} cells in {sys.argv[2]}")
elif cmd == ["judge"]:
    d = sys.argv[2]
    names = [l.strip() for l in open(os.path.join(d, "cells.txt")) if l.strip()]
    sys.exit(judge(names, read(os.path.join(d, "scalac.tsv"))[0], read(os.path.join(d, sys.argv[3] + ".tsv"))[0], read(os.path.join(d, sys.argv[4] + ".tsv"))[0]))
elif cmd == ["same"]:
    print(same(sys.argv[2], sys.argv[3]))
elif cmd != ["--self-test"]:
    sys.exit(__doc__)
