#!/usr/bin/env python3
"""places.py: the table of the places the TASTy writer takes from what the parser read, each construct written with
what may stand between its keyword and its definition or just before its keyword (tests/init-view/places.sh runs it).

  places.py generate <dir>   writes one program per cell, `<construct>--<trivia>--<slot>.scala`
  places.py judge <corpus work dir> <head dir>

A construct is a place one of the writer's records gives: an extension group's `extension` (each written form), an
enum case's `case` (alone, a list, a class case), a package object's `package`, a template's body after its parents
(`extends`, `with`, `derives`, a self type alone, a colon), a file's first statement after each header form, a type
parameter's variance sign, a function type's arrow, a type selection's dot, and the dot of a path in `new`. A trivia is
what stands in one of two slots, `before` the keyword or `between` the keyword and the definition: nothing, a line
comment, a block comment and a nested one holding the keyword, a string, a triple-quoted string and an interpolated
one holding it, a backticked identifier that is it, identifiers that end and start with it, an annotation, a modifier,
another construct of the same keyword. A cell whose program scalac refuses is named, not dropped.

`judge` reads the corpus run of tests/init-view-corpus.sh over the cells (per cell `scalac-view.dump`, the view of
scalac's own pickles, and `view.dump`, the view of the base's products) and the head's views (`<head dir>/<cell>/
view.dump`, or `not-judged` holding why there is none), aligns each to scalac's as regressions.py aligns (compare.py
--positions), and prints the table: per cell its verdict, the lines scalac's on the base and not on the head (lost),
the lines the head gains, and the lines neither has as scalac's (each a named row). Exits 1 when a cell loses one."""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

# What stands in a slot. `kind` says where it may stand: `trivia` anywhere a blank may (a comment), `stmt` where a
# statement or a definition may, `mod` before a definition. `{k}` is the construct's keyword.
TRIVIA = [
    ("nothing", "trivia", ""),
    ("line-comment", "trivia", "// {k}\n"),
    ("block-comment", "trivia", "/* {k} */ "),
    ("nested-comment", "trivia", "/* a /* {k} */ b */ "),
    ("string", "stmt", 'def s1 = "{k}"\n'),
    ("triple-string", "stmt", 'def s2 = """{k}"""\n'),
    ("interpolated", "stmt", 'def s3 = s"{k} ${{1}}"\n'),
    ("backticked", "stmt", "def `{k}` = 1\n"),
    ("affixes", "stmt", "def my{w} = 1\ndef {w}x = 2\n"),
    ("annotation", "mod", '@deprecated("{k}", "1") '),
    ("modifier", "mod", "private "),
]


def fill(text, k, w):
    return text.replace("{k}", k).replace("{w}", w).replace("{{", "{").replace("}}", "}")


def indent(text, pad):
    """Each line of a slot's text at the construct's indentation; a text with no newline stays inline."""
    if "\n" not in text:
        return text
    return "".join(pad + line + "\n" for line in text.rstrip("\n").split("\n"))


# Each construct: its keyword as trivia hold it, an identifier word for the affixes, the program with its two slots
# (`<B>` before the keyword, `<I>` between it and the definition), each slot's indentation and the kinds it takes, and
# the same keyword's construct written once more before it.
CONSTRUCTS = [
    # An extension group at the top level, its methods in the file's wrapper: the wrapper's place is the group's
    # `extension` (Parsers.scala 4658, `ExtMethods` at `atSpan(start)`).
    ("ext-oneline", "extension", "extension", "<B>extension (x: Int) <I>def twice = x + x\n",
     {"B": ("", "trivia stmt"), "I": ("", "trivia mod")}, "extension (y: String) def size2 = y.length\n"),
    ("ext-braces", "extension", "extension", "<B>extension (x: Int) {\n<I>  def twice = x + x\n}\n",
     {"B": ("", "trivia stmt"), "I": ("  ", "trivia stmt mod")}, "extension (y: String) def size2 = y.length\n"),
    ("ext-colon", "extension", "extension", "<B>extension (x: Int):\n<I>  def twice = x + x\n",
     {"B": ("", "trivia stmt"), "I": ("  ", "trivia stmt mod")}, "extension (y: String) def size2 = y.length\n"),
    ("ext-indented", "extension", "extension", "<B>extension (x: Int)\n<I>  def twice = x + x\n",
     {"B": ("", "trivia stmt"), "I": ("  ", "trivia stmt mod")}, "extension (y: String) def size2 = y.length\n"),
    ("ext-collective", "extension", "extension",
     "<B>extension [T](xs: List[T])\n<I>  def second: T = xs(1)\n  def third: T = xs(2)\n",
     {"B": ("", "trivia stmt"), "I": ("  ", "trivia stmt mod")}, "extension (y: String) def size2 = y.length\n"),
    ("ext-using", "extension", "extension",
     "<B>extension (x: Int)(using o: Ordering[Int])\n<I>  def max2(y: Int): Int = o.max(x, y)\n",
     {"B": ("", "trivia stmt"), "I": ("  ", "trivia stmt mod")}, "extension (y: String) def size2 = y.length\n"),
    # An enum case from its `case` where it is alone (Desugar's enum case at the `case`), a list's from its name.
    ("enum-case", "case", "case", "enum Color:\n<B>  case <I>Red\n  case Green\n",
     {"B": ("  ", "trivia stmt mod"), "I": ("", "trivia")}, "case Blue\n"),
    ("enum-case-list", "case", "case", "enum Color:\n<B>  case <I>Red, Green, Blue\n",
     {"B": ("  ", "trivia stmt mod"), "I": ("", "trivia")}, "case Black\n"),
    ("enum-class-case", "case", "case", "enum Shape:\n<B>  case <I>Circle(r: Double)\n  case Square(s: Double)\n",
     {"B": ("  ", "trivia stmt mod"), "I": ("", "trivia")}, "case Dot\n"),
    # A package object's package at its `package` (`package object p` is `package p { object package }`).
    ("package-object", "package", "package", "<B>package <I>object pkgo {\n  def f = 1\n}\n",
     {"B": ("", "trivia stmt"), "I": ("", "trivia")}, "package object other {\n  def g = 2\n}\n"),
    # A template's self definition from the body's first token, after its parents (`Parsers.selfType`).
    ("template-extends", "{", "body", "trait B\ntrait C\ntrait A extends B <B>{<I> self: C =>\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("template-with", "{", "body", "trait B\ntrait C\ntrait D\ntrait A extends B with D <B>{<I> self: C =>\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("template-derives", "{", "body", "case class A(x: Int) derives CanEqual <B>{<I> self =>\n  def f = x\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("template-self", "{", "body", "trait C\ntrait A <B>{<I> this: C =>\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("template-colon", ":", "body", "trait B\ntrait C\ntrait A extends B<B>:\n<I>  self: C =>\n  def f = 1\n",
     {"B": ("", "trivia"), "I": ("  ", "trivia")}, ""),
    # The same construct nested in it and before it: each template's self definition its own, past the bodies nested
    # in it (the parser hands a body's self start to its class once the body ends).
    ("template-nested", "{", "body",
     "trait C\ntrait D\ntrait A <B>{<I> self: C =>\n  trait N { inner: D =>\n    def g = 1\n  }\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("template-nested-plain", "{", "body",
     "trait C\ntrait D\ntrait A <B>{<I>\n  trait N { inner: D =>\n    def g = 1\n  }\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("template-anonymous", "{", "body", "trait B\nval x = new B <B>{<I> me =>\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("template-after-given", "{", "body",
     "trait C\ngiven g: C with { me =>\n  def h = 1\n}\ntrait A extends C <B>{<I> self: C =>\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    # `val a, b = e` reads `e` once for each name (the parser's rewind).
    ("template-rewound", "{", "body", "trait B\nval a, b = new B <B>{<I> me =>\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("template-parent-brace", "{", "body",
     "class P(s: String)\ntrait C\nclass A extends P(\"{\") <B>{<I> self: C =>\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    # A file's first statement, the empty package's identifier where its statements are the file's (Parsers.scala 5050
    # `compilationUnit`: past leading `;`, the first statement's start; SourceFile.scala 27-37 a script's header).
    ("first-import", "import", "import", "<B>import scala.Int\nval x: Int = 1\n",
     {"B": ("", "trivia stmt mod header")}, ""),
    ("first-def", "def", "def", "<B>def f = 1\nval x = f\n", {"B": ("", "trivia stmt mod header")}, ""),
    ("first-object", "object", "object", "<B>object O:\n  def f = 1\nval x = O.f\n",
     {"B": ("", "trivia stmt mod header")}, ""),
    # A variant type parameter from its sign (`Parsers.typeParam`'s `atSpan(start, nameStart)`).
    ("variance", "+", "plus", "class Box[<B>+<I>A, -B](a: A)\n", {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    # A parenthesized function type's arrow (its `Function` at the arrow), a dependent one's start.
    ("arrow", "=>", "arrow", "type F = (Int) <B>=><I> Int\n", {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("arrow-context", "?=>", "arrow", "type G = (Int) <B>?=><I> Int\n", {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    # A type selection's point at its name (`Parsers.selector`), the path of a `new` the same.
    ("selection", ".", "dot", "val xs: scala.collection.immutable<B>.<I>List[Int] = Nil\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("new-path", ".", "dot", "object P:\n  class C\nval c = new P<B>.<I>C\n", {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
]

# A script's header forms (SourceFile.scala 27-37: a `#!` line, or through a line `!#` or `::!#`), and leading `;`,
# which only a file's first statement has before it.
HEADERS = [
    ("header-hash-bang", "#!/bin/sh\n"),
    ("header-bang-hash", "#!/bin/sh\nexec scala \"$0\"\n!#\n"),
    ("header-colon-bang-hash", "#!/bin/sh\nexec scala \"$0\"\n::!#\n"),
    ("leading-semicolon", ";\n"),
    ("leading-semicolons", ";;\n\n"),
]


def cells():
    """Every cell: (name, program)."""
    out = []
    for name, k, w, program, slots, again in CONSTRUCTS:
        for slot, (pad, kinds) in slots.items():
            kinds = kinds.split()
            trivia = [(t, kind, text) for t, kind, text in TRIVIA if kind in kinds]
            if "stmt" in kinds and again:
                trivia.append(("same-construct", "stmt", again))
            marker = "<B>" if slot == "B" else "<I>"
            before = program.split(marker)[0].replace("<B>", "").replace("<I>", "")
            inline = not (before == "" or before.endswith("\n"))
            # A slot inside a line: what ends with a newline continues on a line indented past the slot's line.
            line = before.rsplit("\n", 1)[-1]
            cont = " " * (len(line) - len(line.lstrip(" ")) + 2)
            for t, kind, text in trivia:
                if t == "nothing" and slot != "B":
                    continue
                body = fill(text, k, w)
                if body == "":
                    pass
                elif inline:
                    body = body + cont if body.endswith("\n") else body
                elif body.endswith("\n"):
                    body = indent(body, pad)
                else:
                    body = pad + body
                src = program.replace(marker, body).replace("<B>", "").replace("<I>", "")
                out.append((f"{name}--{t}--{'before' if slot == 'B' else 'between'}", src))
            if "header" in kinds:
                for h, text in HEADERS:
                    out.append((f"{name}--{h}--before", program.replace("<B>", text).replace("<I>", "")))
    return out


def generate(d):
    os.makedirs(d, exist_ok=True)
    for f in os.listdir(d):
        if f.endswith(".scala"):
            os.remove(os.path.join(d, f))
    for name, src in cells():
        with open(os.path.join(d, name + ".scala"), "w", encoding="utf-8") as f:
            f.write(src)
    print(f"places: {len(cells())} cells in {d}")


def judge(work, head_dir):
    import compare as c
    import regressions as r

    def read(p):
        return [x for x in open(p, encoding="utf-8").read().splitlines() if not x.startswith("class ")]

    rows, lost_total, verdicts = [], 0, {}
    for name, _ in cells():
        u = os.path.join(work, name)
        hd = os.path.join(head_dir, name)
        if not os.path.isfile(os.path.join(u, "scalac-view.dump")) or os.path.getsize(os.path.join(u, "scalac-view.dump")) == 0:
            verdict = "scalac refuses or no view of its pickles"
        elif not os.path.isfile(os.path.join(u, "view.dump")) or os.path.getsize(os.path.join(u, "view.dump")) == 0:
            verdict = "the base makes no products"
        elif os.path.exists(os.path.join(hd, "not-judged")):
            verdict = "NOT JUDGED: " + open(os.path.join(hd, "not-judged")).read().strip()
        else:
            o, base, head = read(os.path.join(u, "scalac-view.dump")), read(os.path.join(u, "view.dump")), read(os.path.join(hd, "view.dump"))
            on_base, pos = r.aligned(o, base)
            on_head, _ = r.aligned(o, head)
            both = [i for i in range(len(pos)) if i in on_base and i in on_head]
            lost = [i for i in both if on_base[i] == pos[i] and on_head[i] != pos[i]]
            gained = [i for i in both if on_base[i] != pos[i] and on_head[i] == pos[i]]
            neither = [i for i in both if on_base[i] != pos[i] and on_head[i] != pos[i]]
            lost_total += len(lost)
            verdict = "judged"
            rows.append((name, lost, gained, neither, pos, on_base, on_head))
        verdicts[name] = verdict
    # Of the lines neither has as scalac's, the head's place is master's (unchanged) or one the head moved: each moved
    # one is a named row; the unchanged ones are master's places, counted by construct and tree kind.
    def split(neither, on_base, on_head):
        moved = [i for i in neither if on_head[i] != on_base[i]]
        return moved, [i for i in neither if on_head[i] == on_base[i]]

    print("| cell | verdict | lost | gained | moved, not scalac's | master's, unchanged |")
    print("|---|---|---|---|---|---|")
    by = {n: (l, g, nb, ob, oh) for n, l, g, nb, _, ob, oh in rows}
    for name, _ in cells():
        if name in by:
            l, g, nb, ob, oh = by[name]
            moved, same = split(nb, ob, oh)
            print(f"| {name} | judged | {len(l)} | {len(g)} | {len(moved)} | {len(same)} |")
        else:
            print(f"| {name} | {verdicts[name]} | | | | |")
    totals = {"lost": 0, "gained": 0, "moved": 0, "same": 0}
    kinds = {}
    for name, lost, gained, neither, pos, on_base, on_head in rows:
        moved, same = split(neither, on_base, on_head)
        totals["lost"] += len(lost)
        totals["gained"] += len(gained)
        totals["moved"] += len(moved)
        totals["same"] += len(same)
        for title, lines in (("LOST", lost), ("MOVED, not scalac's (a row)", moved)):
            for i in lines:
                print(f"{name} {title}: scalac `{pos[i].strip()}` base `{on_base[i].strip()}` head `{on_head[i].strip()}`")
        for i in same:
            key = (name.split("--")[0], pos[i].strip().split(" ", 1)[0])
            kinds[key] = kinds.get(key, 0) + 1
    print("master's places neither has as scalac's, unchanged by the head, by construct and tree kind:")
    for (construct, kind), n in sorted(kinds.items()):
        print(f"  {construct} {kind}: {n}")
    judged = len(rows)
    counts = {}
    for v in verdicts.values():
        counts[v] = counts.get(v, 0) + 1
    print(f"places: {len(verdicts)} cells, {judged} judged, {lost_total} lines scalac's on the base and not on the head; "
          f"{totals['gained']} gained; {totals['moved']} moved and not scalac's; {totals['same']} master's unchanged; "
          + "; ".join(f"{v}: {n}" for v, n in sorted(counts.items()) if v != "judged"))
    return 1 if lost_total else 0


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "generate":
        generate(sys.argv[2])
    elif len(sys.argv) == 4 and sys.argv[1] == "judge":
        sys.exit(judge(sys.argv[2], sys.argv[3]))
    else:
        sys.exit(__doc__)
