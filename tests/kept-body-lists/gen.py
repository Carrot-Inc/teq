#!/usr/bin/env python3
"""tests/kept-body-lists/gen.py <out dir>: the programs of the kept-body-lists suite, each <out>/rows/<name>/A.scala (a
library whose kept body applies a USER-DEFINED callee's lists) and B.scala (a program printing its value), and
<out>/rows.txt naming them in order. Deterministic: the same files on every run.

Two families, both crossing every axis below (no program is picked by hand):

kb_*: a library A whose kept body (a `def` or a `val` of object Lib) applies a callee's using or implicit clause of three
  parameters (a, b, c) with a written list. Axes:
    the callee: a method; an overloaded method told apart by its first list, in an object and top-level; one told
      apart by its using list, in an object and top-level; an extractor (`unapply`, `unapplySeq`; the using clause
      after the scrutinee, searched); a constructor by `new`; a class's parent; an anonymous class of a class; a
      trait's parent; an anonymous class of a trait; an enum case; an extension; a context function value; a context
      function result
    x the clause: using; implicit applied with `using`
    x the list: complete (positional; named; named reordered); SHORT (`using a, b`: c searched); SHORT NAMED (`using a =
      .., c = ..`: b searched between them); SEARCHED (the clause left out whole: the chosen givens imported into the
      library's scope, `import Chosen.given`, a fallback in each type's companion)
    x the clause after a plain `(x: Int)` or FIRST (no plain list)
    x the place in A: a def; a val of object Lib
    x a competing given of each parameter's type in A's scope (where a kept body searched again would find it) or none
    x the parameters' types: classes A, B, C (arguments A(1), B(2), C(3); givens A(100), B(200), C(300)) or Int,
      String, Long (1, "w", 3L; givens 100, "g", 300L), so that a lost or swapped argument shows as a value.
  Every program's user file (B) holds a competing given of each type in its own scope (object UserSide), which a kept
  body searched again in the user's compilation would find. A short list where no given is in scope is scalac's
  refusal: those programs hold the refusal.

kc_*: a constructor whose clauses the reference normalizes or does not (NamerOps.normalizeIfConstructor: an empty clause
  added before a leading implicit clause, after a using-only one), its lists applied by a kept body. Axes:
    the class's clauses: a real empty clause after a value clause (`class K(val n: Int)()`), before one
      (`class K()(val n: Int)`), two (`(val n: Int)()()`), between a value clause and a using clause; a using clause
      alone with the normalization's empty clause added or written; an implicit clause alone, its added clause written
      or not; a secondary constructor beside a primary of an empty clause
    x the use: `new`; a class's parent; an anonymous class
    x the place in A: a def; a val of object Lib

kr_*: a right-associative extension (`extension (x: Int) def +:(y: Int)(..)`, whose lists TASTy numbers in the
  desugared order, Desugar.extMethod) applied by a kept body. Axes:
    the clauses: one using or implicit clause of three; two (`(using a: A)(using b: B, c: C)`)
    x the call: the operator (`2 +: 1`, the clauses searched from the library's `import Chosen.given`); a selection with
      every list written (`1.+:(2)(using A(4), B(5), C(6))`); a selection with a short list (one clause: c searched)
    x the place in A: a def; a val of object Lib
  Two implicit clauses are scalac's syntax error (E040): those programs hold the refusal.
"""
import os
import shutil
import sys

SHOW = 's"$x ${a.v} ${b.v} ${c.v}"'


def callee(kind, cl):
    sig = f"{cl} a: A, b: B, c: C"
    if kind == "method":
        return f"def take(x: Int)({sig}): String = {SHOW}", "take(1)({L})", ""
    if kind in ("overload", "overloadtop"):
        defs = (f"def take(x: Int)({sig}): String = {SHOW}\n"
                f"def take(s: String)({sig}): String = \"s\" + {SHOW.replace('$x', '$s')}")
        if kind == "overloadtop":
            return defs, "take(1)({L})", ""
        return "object O:\n" + "\n".join("  " + l for l in defs.split("\n")), "O.take(1)({L})", ""
    if kind in ("overusing", "overusingtop"):
        defs = (f"def take(x: Int)({sig}): String = {SHOW}\n"
                f"def take(x: Int)({cl} a: A, b: B): String = \"two \" + {SHOW.replace(' ${c.v}', '')}")
        if kind == "overusingtop":
            return defs, "take(1)({L})", ""
        return "object O:\n" + "\n".join("  " + l for l in defs.split("\n")), "O.take(1)({L})", ""
    if kind == "extractor":
        return f"object X:\n  def unapply(x: Int)({sig}): Option[String] = Some({SHOW})", "(1 match { case X(s) => s })", ""
    if kind == "extractorseq":
        return f"object X:\n  def unapplySeq(x: Int)({sig}): Option[Seq[String]] = Some(Seq({SHOW}))", "(1 match { case X(s) => s })", ""
    if kind == "new":
        return f"class K(x: Int)({sig}):\n  def value: String = {SHOW}", "new K(1)({L}).value", ""
    if kind == "classparent":
        return f"class K(x: Int)({sig}):\n  def value: String = {SHOW}", "(new S).value", "class S extends K(1)({L})"
    if kind == "classanon":
        return f"class K(x: Int)({sig}):\n  def value: String = {SHOW}", "(new K(1)({L}) {}).value", ""
    if kind == "traitparent":
        return f"trait T(x: Int)({sig}):\n  def value: String = {SHOW}", "(new S).value", "class S extends T(1)({L})"
    if kind == "traitanon":
        return f"trait T(x: Int)({sig}):\n  def value: String = {SHOW}", "(new T(1)({L}) {}).value", ""
    if kind == "enumcase":
        return f"enum E(x: Int)({sig}):\n  def value: String = {SHOW}\n  case One extends E(1)({{L}})", "E.One.value", ""
    if kind == "extension":
        return f"extension (x: Int) def take({sig}): String = {SHOW}", "1.take({L})", ""
    if kind == "ctxvalue":
        return 'val f: (A, B, C) ?=> String = s"1 ${summon[A].v} ${summon[B].v} ${summon[C].v}"', "f({L})", ""
    if kind == "ctxresult":
        return 'def mk(x: Int): (A, B, C) ?=> String = s"$x ${summon[A].v} ${summon[B].v} ${summon[C].v}"', "mk(1)({L})", ""
    raise ValueError(kind)


LISTS = {"pos": "using A(1), B(2), C(3)", "named": "using a = A(1), b = B(2), c = C(3)", "swapped": "using c = C(3), a = A(1), b = B(2)",
         "short": "using A(1), B(2)", "shortnamed": "using a = A(1), c = C(3)", "searched": ""}


def first(defs, call, extra):
    for a, b in [("(x: Int)(", "("), ("$x ", "0 ")]:
        defs = defs.replace(a, b)
    # the clause left out whole on a callee of that clause alone: no list at all
    for a, b in [("new K(1).value", "(new K).value"), ("new K(1) {}", "new K {}"), ("new T(1) {}", "new T {}"), ("extends K(1)", "extends K"),
                 ("extends T(1)", "extends T"), ("extends E(1)", "extends E"), ("take(1)", "take")]:
        if not call.endswith(")") or a in call or a in extra or a in defs:
            call = call.replace(a + "(", "\0").replace(a, b).replace("\0", a + "(")
            extra = extra.replace(a + "(", "\0").replace(a, b).replace("\0", a + "(")
            defs = defs.replace(a + "(", "\0").replace(a, b).replace("\0", a + "(")
    for a, b in [("take(1)(", "take("), ("K(1)(", "K("), ("T(1)(", "T("), ("E(1)(", "E(")]:
        call = call.replace(a, b)
        extra = extra.replace(a, b)
        defs = defs.replace(a, b)
    return defs, call, extra


KINDS = ["method", "overload", "overloadtop", "overusing", "overusingtop", "extractor", "extractorseq", "new", "classparent", "classanon",
         "traitparent", "traitanon", "enumcase", "extension", "ctxvalue", "ctxresult"]
NO_FIRST = ("overload", "overloadtop", "overusing", "overusingtop", "extractor", "extractorseq", "extension", "ctxvalue", "ctxresult")


def kb_cases():
    for kind in KINDS:
        for cl in (["using", "implicit"] if not kind.startswith("ctx") else ["using"]):
            for lk, lst in LISTS.items():
                if kind.startswith("ctx") and lk != "pos":
                    continue
                if kind.startswith("extractor") and lk != "searched":
                    continue
                for at in (["after", "first"] if kind not in NO_FIRST else ["after"]):
                    for place in ["def", "val"]:
                        for comp in (["comp", "none"] if lk != "searched" else ["chosen"]):
                            for prim in [False, True]:
                                yield kind, cl, lk, lst, at, place, comp, prim


def kb_program(kind, cl, lk, lst, at, place, comp, prim):
    n = f"kb_{kind}_{cl}{'' if at == 'after' else 'first'}_{lk}_{place}_{comp}{'_prim' if prim else ''}"
    defs, call, extra = callee(kind, cl)
    if lk == "searched":
        defs, call, extra = (s.replace("({L}) {}", " {}").replace("({L})", "") for s in (defs, call, extra))
    defs, call, extra = defs.replace("{L}", lst), call.replace("{L}", lst), extra.replace("{L}", lst)
    if at == "first":
        defs, call, extra = first(defs, call, extra)
    rivals = 'given A = A(100)\ngiven B = B(200)\ngiven C = C(300)\n' if comp == "comp" else ""
    if comp == "chosen":
        rivals = ('object A:\n  given fallbackA: A = A(901)\nobject B:\n  given fallbackB: B = B(902)\nobject C:\n  given fallbackC: C = C(903)\n'
                  'object Chosen:\n  given A = A(1)\n  given B = B(2)\n  given C = C(3)\nimport Chosen.given\n')
    if prim:
        defs = defs.replace("a: A", "a: Int").replace("b: B", "b: String").replace("c: C", "c: Long")
        defs = defs.replace("${a.v}", "$a").replace("${b.v}", "$b").replace("${c.v}", "$c")
        defs = defs.replace("(A, B, C) ?=>", "(Int, String, Long) ?=>").replace("summon[A].v", "summon[Int]").replace("summon[B].v", "summon[String]").replace("summon[C].v", "summon[Long]")
        for x, y in [("A(1)", "1"), ("B(2)", '"w"'), ("C(3)", "3L")]:
            call = call.replace(x, y)
            extra = extra.replace(x, y)
            defs = defs.replace(x, y)
        rivals = 'given Int = 100\ngiven String = "g"\ngiven Long = 300L\n' if comp == "comp" else ""
        if comp == "chosen":
            rivals = 'object Chosen:\n  given Int = 1\n  given String = "w"\n  given Long = 3L\nimport Chosen.given\n'
    a = (f"package g_{n}\nclass A(val v: Int)\nclass B(val v: Int)\nclass C(val v: Int)\n{rivals}{defs}\n"
         f"object Lib:\n" + (f"  {extra}\n" if extra else "") + f"  {place} value: String = {call}\n")
    # the user's file: a competing given of each type in the user's own scope (an object of B), which a kept body searched
    # again in the user's compilation would be the one to find
    user = ('  given Int = 700\n  given String = "u"\n  given Long = 900L\n' if prim else '  given A = A(700)\n  given B = B(800)\n  given C = C(900)\n')
    b = f"package g_{n}\nobject Main:\n  def main(args: Array[String]): Unit = UserSide.run()\nobject UserSide:\n{user}  def run(): Unit = println(Lib.value)\n"
    return n, a, b


# kc_*: the class's clauses, members beside `value`, the value it shows, the arguments a `new` and a parent write
ABC = "a: A, b: B, c: C"
SHAPES = {
    "emptyafter": ("(val n: Int)()", "", 's"$n"', "(1)()"),
    "emptybefore": ("()(val n: Int)", "", 's"$n"', "()(1)"),
    "emptytwice": ("(val n: Int)()()", "", 's"$n"', "(1)()()"),
    "emptybetween": (f"(val n: Int)()(using {ABC})", "", 's"$n ${a.v} ${b.v} ${c.v}"', "(1)()(using A(1), B(2), C(3))"),
    "emptybetweenshort": (f"(val n: Int)()(using {ABC})", "", 's"$n ${a.v} ${b.v} ${c.v}"', "(1)()(using A(1), B(2))"),
    "usingadded": (f"(using {ABC})", "", 's"${a.v} ${b.v} ${c.v}"', "(using A(1), B(2), C(3))"),
    "usingwritten": (f"(using {ABC})()", "", 's"${a.v} ${b.v} ${c.v}"', "(using A(1), B(2), C(3))()"),
    "implicitadded": (f"(implicit {ABC})", "", 's"${a.v} ${b.v} ${c.v}"', "(using A(1), B(2), C(3))"),
    "implicitwritten": (f"(implicit {ABC})", "", 's"${a.v} ${b.v} ${c.v}"', "()(using A(1), B(2), C(3))"),
    "secondary": ("(val n: Int)()", "  def this(s: String)() = this(s.length)()\n", 's"$n"', '("ab")()'),
}


def kc_program(shape, use, place):
    n = f"kc_{shape}_{use}_{place}"
    clauses, members, show, args = SHAPES[shape]
    cls = f"class K{clauses}:\n{members}  def value: String = {show}\n"
    if use == "new":
        extra, call = "", f"new K{args}.value"
    elif use == "parent":
        extra, call = f"class S extends K{args}", "(new S).value"
    else:
        extra, call = "", f"(new K{args} {{}}).value"
    a = (f"package g_{n}\nclass A(val v: Int)\nclass B(val v: Int)\nclass C(val v: Int)\ngiven A = A(100)\ngiven B = B(200)\ngiven C = C(300)\n"
         f"{cls}object Lib:\n" + (f"  {extra}\n" if extra else "") + f"  {place} value: String = {call}\n")
    b = (f"package g_{n}\nobject Main:\n  def main(args: Array[String]): Unit = UserSide.run()\nobject UserSide:\n"
         f"  given A = A(700)\n  given B = B(800)\n  given C = C(900)\n  def run(): Unit = println(Lib.value)\n")
    return n, a, b


def kr_program(cl, clauses, call, place):
    n = f"kr_{cl}_{clauses}_{call}_{place}"
    if clauses == "one":
        ext = f"extension (x: Int) def +:(y: Int)({cl} a: A, b: B, c: C): String = s\"$x $y ${{a.v}} ${{b.v}} ${{c.v}}\""
        sel = {"sel": "1.+:(2)(using A(4), B(5), C(6))", "short": "1.+:(2)(using A(4), B(5))"}
    else:
        ext = f"extension (x: Int) def +:(y: Int)({cl} a: A)({cl} b: B, c: C): String = s\"$x $y ${{a.v}} ${{b.v}} ${{c.v}}\""
        sel = {"sel": "1.+:(2)(using A(4))(using B(5), C(6))"}
    expr = "2 +: 1" if call == "op" else sel[call]
    a = (f"package g_{n}\nclass A(val v: Int)\nclass B(val v: Int)\nclass C(val v: Int)\n"
         "object Chosen:\n  given A = A(1)\n  given B = B(2)\n  given C = C(3)\nimport Chosen.given\n"
         f"{ext}\nobject Lib:\n  {place} value: String = {expr}\n")
    b = (f"package g_{n}\nobject Main:\n  def main(args: Array[String]): Unit = UserSide.run()\nobject UserSide:\n"
         f"  given A = A(700)\n  given B = B(800)\n  given C = C(900)\n  def run(): Unit = println(Lib.value)\n")
    return n, a, b


def programs():
    for case in kb_cases():
        yield kb_program(*case)
    for shape in SHAPES:
        for use in ["new", "parent", "anon"]:
            for place in ["def", "val"]:
                yield kc_program(shape, use, place)
    for cl in ["using", "implicit"]:
        for clauses, calls in [("one", ["op", "sel", "short"]), ("two", ["op", "sel"])]:
            for call in calls:
                for place in ["def", "val"]:
                    yield kr_program(cl, clauses, call, place)


def main(argv):
    out = argv[0]
    rows = os.path.join(out, "rows")
    if os.path.isdir(rows):
        shutil.rmtree(rows)
    os.makedirs(rows)
    names = []
    for n, a, b in programs():
        os.makedirs(os.path.join(rows, n))
        with open(os.path.join(rows, n, "A.scala"), "w") as fh:
            fh.write(a)
        with open(os.path.join(rows, n, "B.scala"), "w") as fh:
            fh.write(b)
        names.append(n)
    with open(os.path.join(out, "rows.txt"), "w") as fh:
        fh.write("\n".join(names) + "\n")
    print(len(names))


if __name__ == "__main__":
    main(sys.argv[1:])
