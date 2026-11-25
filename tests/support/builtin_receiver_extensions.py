#!/usr/bin/env python3
"""tests/support/builtin_receiver_extensions.py: the table of an EXTENSION ON A BUILTIN RECEIVER's PARENT: an extension
whose receiver clause is a class String or Array derives from (`extension (x: CharSequence) def +:(y: ...)`), selected
on a receiver of that builtin type, applies as the subtype check says the receiver conforms (dotty's `tryExtension`:
the receiver is the extension's first argument), whatever the call's form, the extension's holder or the receiver's
expression.

A cell is one program in package `fd`: its library (lib/Lib.scala: the extension, in its holder) and its user
(use/Main.scala, `@main def run()`), run WHOLE and SPLIT by tests/support/pkgobj-given-import.sh. The families:
  forms    the receiver clause CharSequence x the receiver (a val of type String, a call giving a String, a literal) x
           the call (a selection, an infix symbolic operator, a right-associative operator over a java.util.List, an
           Option and a String, the method form `+:(r)(x)`) x the holder (the package's top level, an object imported,
           a value class's instance imported, a plain class's instance imported);
  parents  the receiver clause (Comparable[String], AnyRef, java.lang.Object, Any, String, java.io.Serializable) x a
           val or a literal x a selection or a right-associative operator over a java.util.List, from an object;
  array    an Array receiver (a val, a call) x the receiver clause (AnyRef, Any, Cloneable, java.io.Serializable).
Each program prints what the extension gives: `x|y` of its receiver and argument.

  builtin_receiver_extensions.py cells <out dir>             the cells and cells.txt
  builtin_receiver_extensions.py judge <dir> <base> <head>   <dir>/scalac.tsv against <base>.tsv and <head>.tsv: first
                                                             the legs the head accepts where scalac refuses, then right,
                                                             turned, regressed per family; exit 1 unless every leg is
                                                             right on the head and none is missing
  builtin_receiver_extensions.py --self-test                 the templates and the judge over planted tables
A leg is right when it is scalac's output, or scalac's first error at its file, line and column in its words.
"""
import io
import os
import sys

RECEIVERS = {"val": ("val x = \"a\"", "x"), "call": (None, "note(\"a\")"), "literal": (None, "\"a\"")}
RIGHT = {"list": ("java.util.List[String]", "java.util.List.of(\"b\")"), "option": ("Option[String]", "Some(\"b\")"),
         "string": ("String", "\"b\"")}


def ext(recv_ty, op, arg_ty):
    return f"extension (x: {recv_ty}) def {op}(y: {arg_ty}): String = \"\" + x + \"|\" + y"


def lib(holder, lines):
    body = "".join("  " + l + "\n" for l in lines)
    return {"top": "package fd\n\n" + "".join(l + "\n" for l in lines),
            "object": "package fd\n\nobject E:\n" + body,
            "valueclass": "package fd\n\nclass V(val n: Int) extends AnyVal:\n" + body,
            "plainclass": "package fd\n\nclass P(val n: Int):\n" + body}[holder]


def user(holder, setup, line):
    imp = {"top": [], "object": ["import E.*"], "valueclass": ["val v = new V(7)", "import v.*"],
           "plainclass": ["val p = new P(7)", "import p.*"]}[holder]
    lines = ["def note(s: String): String = s"] + imp + setup + [f"println({line})"]
    return "package fd\n\n@main def run(): Unit =\n" + "".join("  " + l + "\n" for l in lines)


def call(form, recv):
    if form == "selection":
        return f"{recv}.m(1)"
    if form == "infix":
        return f"{recv} |+| 1"
    if form == "method":
        return f"+:(Some(\"b\"))({recv})"
    return f"{recv} +: {RIGHT[form[6:]][1]}"


def ext_lines(recv_ty, form):
    if form == "selection":
        return [ext(recv_ty, "m", "Int")]
    if form == "infix":
        return [ext(recv_ty, "|+|", "Int")]
    if form == "method":
        return [ext(recv_ty, "+:", "Option[String]")]
    return [ext(recv_ty, "+:", RIGHT[form[6:]][0])]


FORMS = ["selection", "infix", "right-list", "right-option", "right-string", "method"]


def cell(name, holder, recv_ty, form, recv_kind, recv_expr=None):
    setup_line, expr = RECEIVERS[recv_kind] if recv_expr is None else recv_expr
    setup = [setup_line] if setup_line else []
    return (name, {"Lib.scala": lib(holder, ext_lines(recv_ty, form))}, user(holder, setup, call(form, expr)))


def cells():
    out = []
    for holder in ["top", "object", "valueclass", "plainclass"]:
        for form in FORMS:
            for kind in RECEIVERS:
                out.append(cell(f"f-{holder}-{form}-{kind}", holder, "CharSequence", form, kind))
    for key, ty in [("comparable", "Comparable[String]"), ("anyref", "AnyRef"), ("object", "java.lang.Object"), ("any", "Any"),
                    ("string", "String"), ("serializable", "java.io.Serializable")]:
        for form in ["selection", "right-list"]:
            for kind in ["val", "literal"]:
                out.append(cell(f"p-{key}-{form}-{kind}", "object", ty, form, kind))
    for key, ty in [("anyref", "AnyRef"), ("any", "Any"), ("cloneable", "Cloneable"), ("serializable", "java.io.Serializable")]:
        for kind, rx in [("val", ("val x = Array(1, 2)", "x")), ("call", (None, "Array(1, 2).clone()"))]:
            name = f"a-{key}-{kind}"
            out.append((name, {"Lib.scala": lib("object", [f"extension (x: {ty}) def m(y: Int): String = \"arr|\" + y"])},
                        user("object", [rx[0]] if rx[0] else [], f"{rx[1]}.m(1)")))
    return out


def family(name):
    return {"f": "forms", "p": "parents", "a": "array"}[name[0]]


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
    if d["f-valueclass-right-list-val"][1]["Lib.scala"] != ("package fd\n\nclass V(val n: Int) extends AnyVal:\n  extension (x: CharSequence) def +:(y: java.util.List[String]): String = \"\" + x + \"|\" + y\n"):
        bad.append("the value class's right-associative extension")
    if "  val v = new V(7)\n  import v.*\n  val x = \"a\"\n  println(x +: java.util.List.of(\"b\"))\n" not in d["f-valueclass-right-list-val"][2]:
        bad.append("its user")
    if "println(+:(Some(\"b\"))(note(\"a\")))" not in d["f-top-method-call"][2] or "import" in d["f-top-method-call"][2]:
        bad.append("the top level's method form")
    if "extension (x: java.io.Serializable) def m(y: Int)" not in d["p-serializable-selection-literal"][1]["Lib.scala"] or "println(\"a\".m(1))" not in d["p-serializable-selection-literal"][2]:
        bad.append("the Serializable parent's literal")
    if "println(Array(1, 2).clone().m(1))" not in d["a-cloneable-call"][2]:
        bad.append("the array's call")
    s = {("f-a", b, "run"): "ok:a|1" for b in ("whole", "split")}
    s.update({("p-e", b, "run"): "err:Main.scala:5:11: value m is not a member of String" for b in ("whole", "split")})
    good = {("f-a", b, l): "ok:a|1" for b in ("whole", "split") for l in LEGS}
    good.update({("p-e", b, l): "err:Main.scala:5:11: value m is not a member of String" for b in ("whole", "split") for l in LEGS})
    accepting = {**good, ("p-e", "split", "js"): "ok:a|1"}
    wrong = {**good, ("f-a", "whole", "jvm"): "ok:Vector(a, b)"}
    missing = {k: v for k, v in good.items() if k != ("f-a", "whole", "interp")}
    null = open(os.devnull, "w")
    for what, base, head, code in (("all right", wrong, good, 0), ("a wrong value", good, wrong, 1), ("an acceptance scalac refuses", good, accepting, 1),
                                   ("a missing leg", good, missing, 1)):
        if judge(["f-a", "p-e"], s, base, head, out=null) != code:
            bad.append(f"the judge on the planted table: {what}")
    o = io.StringIO()
    judge(["f-a", "p-e"], s, good, accepting, out=o)
    if not o.getvalue().startswith("accepted by the head and refused by scalac: 1\n"):
        bad.append("the judge's first line")
    if bad:
        sys.exit("builtin_receiver_extensions self-test FAILED: " + "; ".join(bad))
    print(f"builtin_receiver_extensions self-test: {len(cs)} cells, the templates checked, the judge's planted tables judged as they must be",
          file=sys.stdout if sys.argv[1:2] == ["--self-test"] else sys.stderr)


self_test()
cmd = sys.argv[1:2]
if cmd == ["cells"]:
    print(f"builtin_receiver_extensions: {len(write_cells(sys.argv[2]))} cells in {sys.argv[2]}")
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
