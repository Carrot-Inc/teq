#!/usr/bin/env python3
"""cells.py <dir>: writes the package-object table's cells, one directory per cell, each a library of package `p` (a
package object beside a clause of `p`) and `user/Main.scala`, which uses every member and prints what it reads, and
`cell.json` naming the cell (the directory's name, which the table's judge checks).

Axes: the package object's form (`braces`: `package object p { .. }`, `indented`: `package object p:`), the layout
(`one`: the package object and the clause in one file, `two`: in two files), and the members:
- `def`: a def in the package object, another in the clause;
- `same-name`: a method of one name in both, overloaded by its parameter;
- `import`: a val of the package object read through `import p.*`;
- `given`: a given in the package object summoned through `import p.given`;
- `parents`: the package object extends a trait of the clause and adds a member (dotty's pos/i17394);
- `class-object`: a class in the package object, an object of the same name in the clause (pos/t5031_2).
scalac's model (Desugar.packageModuleDef, packageDef): the package object is the module `p.package`, the clause's
definitions are `p` itself and its file's `<file>$package`."""
import json
import os
import sys

MEMBERS = {
    "def": ("def fromObject: Int = 1", "def fromClause: Int = 2", "println((p.fromObject, p.fromClause))"),
    "same-name": ("def shared(x: Int): Int = x + 1", 'def shared(s: String): String = s + "!"',
                  'println((p.shared(1), p.shared("a")))'),
    "import": ('val greeting: String = "hi"', "def other: Int = 3", "import p.*\n  println((greeting, other))"),
    "given": ("given reverse: Ordering[Int] = Ordering.Int.reverse", "def other: Int = 3",
              "import p.given\n  println((summon[Ordering[Int]].compare(1, 2), p.other))"),
    "parents": ("def own: Int = 6", "trait Base:\n  def base: Int = 5", "println((p.own, p.base))"),
    "class-object": ('class ID:\n  override def toString: String = "the class"',
                     'object ID:\n  override def toString: String = "the object"',
                     "println((p.ID, new p.ID))"),
}


def indent(text, pad):
    return "\n".join(pad + line if line else line for line in text.split("\n"))


def package_object(form, members, parents):
    ext = " extends Base" if parents else ""
    if form == "braces":
        return f"package object p{ext} {{\n{indent(members, '  ')}\n}}\n"
    return f"package object p{ext}:\n{indent(members, '  ')}\n"


def cells():
    out = []
    for form in ("braces", "indented"):
        for layout in ("one", "two"):
            for variant, (obj, clause, use) in MEMBERS.items():
                po = package_object(form, obj, variant == "parents")
                files = {}
                if layout == "one":
                    if form == "braces":
                        files["lib/Lib.scala"] = f"package p {{\n{indent(clause, '  ')}\n}}\n\n{po}"
                    else:
                        files["lib/Lib.scala"] = f"package p:\n{indent(clause, '  ')}\n\n{po}"
                else:
                    files["lib/package.scala"] = po
                    files["lib/Clause.scala"] = f"package p\n\n{clause}\n"
                files["user/Main.scala"] = f"package user\n\n@main def run(): Unit =\n  {use}\n"
                out.append((f"{form}--{layout}--{variant}", files))
    return out


def main():
    d = sys.argv[1]
    for name, files in cells():
        for path, text in files.items():
            p = os.path.join(d, name, path)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            with open(p, "w", encoding="utf-8") as f:
                f.write(text)
        with open(os.path.join(d, name, "cell.json"), "w") as f:
            json.dump({"name": name}, f)
    print(f"package-objects: {len(cells())} cells in {d}")


if __name__ == "__main__":
    main()
