#!/usr/bin/env python3
"""gen.py <dir> writes rows/<cell>/Main.scala and rows.txt for the qualified-super table.
The name records the qualifier, mix, call site (a through g) and member in A.
Two class probes use plain super[A] and B.this as controls."""
from pathlib import Path
from textwrap import indent
import sys

QUALIFIERS = {"class": ("B", "new B"), "trait": ("T", "new C"), "object": ("O", "O")}
MEMBERS = {
    "def0": (("def m: String",), "m"),
    "def1": (("def m(i: Int): String",), "m(7)"),
    "generic": (("def m[X](x: X): String",), 'm[String]("x")'),
    "overload": (("def m(): String", "def m(i: Int): String"), "m(7)"),
    "val": (("val m: String",), "m"),
    "abstract": (("def m: String",), "m"),
}


def js_reference_cells():
    for place in "ab":
        for member in ("def0", "def1", "generic", "overload"):
            yield f"qs_trait_A_{place}_{member}"
    for place in "ab":
        yield f"qs_pin_A_{place}_def0"


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
    raise ValueError(place)


def program(qualifier, mix, place, member):
    name = f"qs_{qualifier}_{mix}_{place}_{member}"
    pin = qualifier == "pin"
    if pin:
        qualifier = "trait"
    q, instance = QUALIFIERS[qualifier]
    receiver = {"none": f"{q}.super", "A": f"{q}.super[A]", "U": f"{q}.super[U]",
                "plainA": "super[A]", "this": f"{q}.this"}[mix]
    body, result = site(place, receiver + "." + MEMBERS[member][1])
    parent = "abstract class A" if member == "abstract" else "class A"
    parts = [f"package g_{name}", parent + ":\n" + indent(members(member, "A"), "  ")]
    if pin:
        parts.append("class Mid extends A:\n" + indent(members(member, "Mid"), "  "))
    if mix == "U":
        parts.append("trait U extends A:\n" + indent(members(member, "U"), "  "))
    if place == "e":
        parts.append("trait Reader:\n  def f: String")
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


def main(out):
    names = []
    for name, source in programs():
        row = out / "rows" / name
        row.mkdir(parents=True, exist_ok=True)
        (row / "Main.scala").write_text(source)
        names.append(name)
    (out / "rows.txt").write_text("\n".join(names) + "\n")
    print(len(names))


if __name__ == "__main__":
    main(Path(sys.argv[1]))
