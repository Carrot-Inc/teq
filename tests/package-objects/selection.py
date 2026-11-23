#!/usr/bin/env python3
"""selection.py <dir>: the package-object table's selection cells: members of one package `overlap` defined in a
package object (A), a top-level definition (B), or both, split over modules and written by teq, scalac (or Scala 2 for
a package object) or one each, and a client reading them. scalac's answer (its client's run, the warning or error of
PackageClassDenotation.dropStale, a refusal) is each cell's.

A cell is a directory with `cell.json`: its name (the directory's, which the table's judge checks), its modules in
build order, each `{name, files, writer, time, out}` (`writer` `teq`, `scalac`, `scalac2` (Scala 2.13, a package
object only), `teq-<name>` (an earlier teq, the migration's), or `client` for the last module, which both compilers
build over the same upstream products; `time`, Unix seconds, given to the module's product files; `out`, another
module's directory to write into: one container for both), the class path order of the client's upstreams, the
client's main class, and the files beside it.

The variants (what is defined, how the client reads it):
- `term`: `def value`, read `overlap.value` (the design read's program);
- `type`: `type T` (Int in A, String in B), read by `val x: overlap.T = 1` (a refusal where String is chosen);
- `import-all`, `import-one`: `def value`, read through `import overlap.*`, `import overlap.value`;
- `given`: `given Int`, read through `import overlap.given` and `summon[Int]`;
- `extension`: `extension (x: Int) def tag`, read `1.tag` through `import overlap.*`;
- `inside`: `def value`, read unqualified by a client in `package overlap`;
- `overloads`: `value(x: Int)` in A, `value(s: String)` in B, both called;
- `private`: A's `value` private, so only B's is a member (nonPrivateMembersNamed);
- `two-names`: `v1` and `v2` in both, read both (dropStale warns once per package);
- `top-class`: `overlap.hashCode`, a member of a top class, dropped (a refusal).
The shapes: one module; two (one definition upstream, the other with the client); three (A and B apart, the client
over both, each class path order) with the times fixed (b younger, a younger, equal: dropStale's error, half a
second apart: its granularity); A and B written into one directory (one container: no warning)."""
import json
import os
import sys

T1, T2 = 1700000000, 1700000100

VARIANTS = {
    "term": ("def value: Int = 11", "def value: Int = 22", "println(overlap.value)"),
    "type": ("type T = Int", "type T = String", "val x: overlap.T = 1\n    println(x)"),
    "import-all": ("def value: Int = 11", "def value: Int = 22", "import overlap.*\n    println(value)"),
    "import-one": ("def value: Int = 11", "def value: Int = 22", "import overlap.value\n    println(value)"),
    "given": ("given Int = 11", "given Int = 22", "import overlap.given\n    println(summon[Int])"),
    "extension": ("extension (x: Int) def tag: Int = 11", "extension (x: Int) def tag: Int = 22",
                  "import overlap.*\n    println(1.tag)"),
    "inside": ("def value: Int = 11", "def value: Int = 22", None),
    "overloads": ("def value(x: Int): Int = 11", "def value(s: String): Int = 22",
                  'println((overlap.value(1), overlap.value("s")))'),
    "private": ("private def value: Int = 11", "def value: Int = 22", "println(overlap.value)"),
    "two-names": ("def v1: Int = 11\n  def v2: Int = 12", "def v1: Int = 21\ndef v2: Int = 22",
                  "println((overlap.v1, overlap.v2))"),
    "top-class": ("def value: Int = 11", "def value: Int = 22", "println(overlap.hashCode)"),
}


def sources(variant):
    a, b, use = VARIANTS[variant]
    po = f"package object overlap {{\n  {a}\n}}\n"
    top = f"package overlap\n{b}\n"
    if use is None:
        client = ("package overlap\n\nobject Client {\n  def main(args: Array[String]): Unit =\n    println(value)\n}\n",
                  "overlap.Client")
    else:
        client = (f"object Client {{\n  def main(args: Array[String]): Unit =\n    {use}\n}}\n", "Client")
    return po, top, client


def cells():
    out = []
    for variant in VARIANTS:
        po, top, (client, main) = sources(variant)
        a, b, cl = ("A.scala", po), ("B.scala", top), ("Client.scala", client)
        out.append((f"{variant}--one", [{"name": "m", "files": [a, b, cl], "writer": "client"}], [], main))
        for (up, down, tag) in ((a, b, "a-up"), (b, a, "b-up")):
            for w in ("teq", "scalac"):
                out.append((f"{variant}--two--{tag}--{w}", [{"name": "up", "files": [up], "writer": w},
                                                             {"name": "client", "files": [down, cl], "writer": "client"}],
                            ["up"], main))
        times = [(T1, T2, "b-younger"), (T2, T1, "a-younger")]
        if variant == "term":
            times += [(T1, T1, "equal"), (T1, T1 + 0.5, "half-second")]
        writers = [("teq", "teq"), ("teq", "scalac"), ("scalac", "teq"), ("scalac", "scalac")]
        if variant in ("term", "type", "given"):
            writers.append(("scalac2", "scalac"))
        for wa, wb in writers:
            for order in (["a", "b"], ["b", "a"]):
                for ta, tb, tname in times:
                    out.append((f"{variant}--three--{wa}-{wb}--{''.join(order)}--{tname}",
                                [{"name": "a", "files": [a], "writer": wa, "time": ta},
                                 {"name": "b", "files": [b], "writer": wb, "time": tb},
                                 {"name": "client", "files": [cl], "writer": "client"}], order, main))
        if variant == "term":
            for w in ("teq", "scalac"):
                out.append((f"{variant}--one-container--{w}",
                            [{"name": "a", "files": [a], "writer": w, "time": T1},
                             {"name": "b", "files": [b], "writer": w, "time": T2, "out": "a"},
                             {"name": "client", "files": [cl], "writer": "client"}], ["a"], main))
    # The migration: upstream modules written by an earlier teq, the flattened form of products number 0: the released
    # 0.1.10 (`teq-old`) and a master before number 1 (`teq-master`), their binaries given by TEQ_WRITERS old=<path>
    # master=<path>.
    po, top, (client, main) = sources("term")
    a, b, cl = ("A.scala", po), ("B.scala", top), ("Client.scala", client)
    for w in ("teq-old", "teq-master"):
        out.append((f"migration--{w}--po--two", [{"name": "up", "files": [a], "writer": w},
                                                 {"name": "client", "files": [cl], "writer": "client"}], ["up"], main))
        out.append((f"migration--{w}--both--two--a-up", [{"name": "up", "files": [a], "writer": w},
                                                         {"name": "client", "files": [b, cl], "writer": "client"}],
                    ["up"], main))
        for order in (["a", "b"], ["b", "a"]):
            for ta, tb, tname in ((T1, T2, "b-younger"), (T2, T1, "a-younger")):
                out.append((f"migration--{w}--both--three--{''.join(order)}--{tname}",
                            [{"name": "a", "files": [a], "writer": w, "time": ta},
                             {"name": "b", "files": [b], "writer": w, "time": tb},
                             {"name": "client", "files": [cl], "writer": "client"}], order, main))
    return out


def main():
    d = sys.argv[1]
    for name, modules, order, main_class in cells():
        c = os.path.join(d, name)
        for m in modules:
            for fname, text in m["files"]:
                p = os.path.join(c, m["name"], fname)
                os.makedirs(os.path.dirname(p), exist_ok=True)
                with open(p, "w", encoding="utf-8") as f:
                    f.write(text)
        spec = {"name": name,
                "modules": [{k: (v if k != "files" else [f for f, _ in v]) for k, v in m.items()} for m in modules],
                "order": order, "main": main_class}
        with open(os.path.join(c, "cell.json"), "w") as f:
            json.dump(spec, f, indent=1)
    print(f"selection: {len(cells())} cells in {d}")


if __name__ == "__main__":
    main()
