#!/usr/bin/env python3
"""gen.py <dir> writes rows/<cell>/Main.scala and rows.txt for the qualified-super table.
The name records the qualifier, mix, call site (a through o) and member in A.
Two class probes use plain super[A] and B.this as controls."""
from pathlib import Path
from textwrap import indent
import shutil
import sys

from witnesses import SOURCES, witness_names

QUALIFIERS = {"class": ("B", "new B"), "trait": ("T", "new C"), "object": ("O", "O")}
MEMBERS = {
    "def0": (("def m: String",), "m"),
    "def1": (("def m(i: Int): String",), "m(7)"),
    "generic": (("def m[X](x: X): String",), 'm[String]("x")'),
    "overload": (("def m(): String", "def m(i: Int): String"), "m(7)"),
    "val": (("val m: String",), "m"),
    "abstract": (("def m: String",), "m"),
}


def new_site_cells():
    for places in ("hijk", "lmno"):
        for qualifier in QUALIFIERS:
            for mix in ("plain", "none"):
                for place in places:
                    for member in ("def0", "def1"):
                        yield qualifier, mix, place, member


def js_reference_cells():
    for place in "ab":
        for member in ("def0", "def1", "generic", "overload"):
            yield f"qs_trait_A_{place}_{member}"
    for place in "ab":
        yield f"qs_pin_A_{place}_def0"
    for axes in new_site_cells():
        yield "qs_" + "_".join(axes)
    yield from witness_names()


def members(kind, owner):
    lines = []
    for sig in MEMBERS[kind][0]:
        prefix = "" if owner == "A" else "override "
        label = owner + ("()" if "m()" in sig else "")
        body = "" if kind == "abstract" and owner in ("A", "U") else f' = "{label}"'
        lines.append(prefix + sig + body)
    return "\n".join(lines)


def site(place, call):
    f = f"def f: String = {call}"
    if place == "a":
        return f, "q.f"
    if place == "b":
        return "class Inner:\n  " + f, "(new q.Inner).f"
    if place == "c":
        return "class Inner:\n  class Deep:\n    " + f, "deep.f"
    if place == "d":
        return (f"class Inner:\n  def f: String =\n    val get = () => {call}\n    get()",
                "(new q.Inner).f")
    if place == "e":
        return ("def run: String =\n  val obj = new Reader { " + f + " }\n  obj.f", "q.run")
    if place == "f":
        return ("def run: String =\n  class Local { " + f + " }\n  (new Local).f", "q.run")
    if place == "g":
        return "object Inner:\n  " + f, "q.Inner.f"
    if place == "h":
        return (f"def f: String =\n  val s: java.util.function.Supplier[String] = () => {call}\n  s.get()",
                "q.f")
    if place == "i":
        return f"def f: String =\n  val s: Thunk = () => {call}\n  s()", "q.f"
    if place == "j":
        return f"def by(x: => String): String = x\ndef f: String = by({call})", "q.f"
    if place == "k":
        return f"def f: String =\n  def g(): String = {call}\n  g()", "q.f"
    if place == "l":
        return f"def f: String =\n  val g: () => String = () => {call}\n  g()", "q.f"
    if place == "m":
        return f"def f: String = List(1).map(_ => {call}).head", "q.f"
    if place == "n":
        return f"def f: String =\n  val p: PartialFunction[Int, String] = {{ case _ => {call} }}\n  p(1)", "q.f"
    if place == "o":
        return f"def f: String =\n  val c: Int ?=> String = (x: Int) ?=> {call}\n  c(using 1)", "q.f"
    raise ValueError(place)


def program(qualifier, mix, place, member):
    name = f"qs_{qualifier}_{mix}_{place}_{member}"
    pin = qualifier == "pin"
    if pin:
        qualifier = "trait"
    q, instance = QUALIFIERS[qualifier]
    receiver = {"none": f"{q}.super", "A": f"{q}.super[A]", "U": f"{q}.super[U]",
                "plain": "super", "plainA": "super[A]", "this": f"{q}.this"}[mix]
    body, result = site(place, receiver + "." + MEMBERS[member][1])
    parent = "abstract class A" if member == "abstract" else "class A"
    parts = [f"package g_{name}", parent + ":\n" + indent(members(member, "A"), "  ")]
    if pin:
        parts.append("class Mid extends A:\n" + indent(members(member, "Mid"), "  "))
    if mix == "U":
        parts.append("trait U extends A:\n" + indent(members(member, "U"), "  "))
    if place == "e":
        parts.append("trait Reader:\n  def f: String")
    if place == "i":
        parts.append("trait Thunk:\n  def apply(): String")
    parents = "A with U" if mix == "U" else "A"
    parts.append(f"{qualifier} {q} extends {parents}:\n" + indent(members(member, q) + "\n" + body, "  "))
    if qualifier == "trait":
        parts.append("class C extends Mid with T" if pin else "class C extends T")
    main = f"@main def runCell(): Unit =\n  val q = {instance}\n"
    if place == "c":
        main += "  val inner = new q.Inner\n  val deep = new inner.Deep\n"
    parts.append(main + f"  println({result})")
    return name, "\n\n".join(parts) + "\n"


def programs():
    for qualifier in QUALIFIERS:
        for mix in (("none", "A", "U") if qualifier == "class" else ("none", "A")):
            for place in "abcdefg":
                for member in MEMBERS:
                    yield program(qualifier, mix, place, member)
    yield program("class", "plainA", "a", "def0")
    yield program("class", "this", "b", "def0")
    for place in "ab":
        yield program("pin", "A", place, "def0")
    for axes in new_site_cells():
        yield program(*axes)


def main(out):
    names = []
    for name, source in programs():
        row = out / "rows" / name
        row.mkdir(parents=True, exist_ok=True)
        (row / "Main.scala").write_text(source)
        names.append(name)
    for name in witness_names():
        shutil.copytree(SOURCES / name.removeprefix("qw_"), out / "rows" / name, dirs_exist_ok=True)
        names.append(name)
    (out / "rows.txt").write_text("\n".join(names) + "\n")
    print(len(names))


if __name__ == "__main__":
    main(Path(sys.argv[1]))
