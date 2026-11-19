"""gen_contexts.py <out dir> [def|given|structural]: the owner phrase's generated proof. For each CONTEXT a local method can stand in, times
each OWNER, one file: a local `def call(using need: Numeric[String]): Int` called in that context, inside a member of
that owner. One directory per owner (a family), one file per context, every name suffixed by the context's number.
Writes <out>/<owner>/k<NN>.scala and a manifest <out>/cells.tsv (owner, context number, context name)."""
import os
import sys

out = sys.argv[1]
# the callee: a local method (the default), or with `given` a local given instance with a context parameter, called by
# its name (the typer's other entry of a local definition)
CALLEE = sys.argv[2] if len(sys.argv) > 2 else "def"
E = {"def": "{{ def call(using need: Numeric[String]): Int = 1; call }}",
     "given": "{{ given call(using need: Numeric[String]): Int = 1; call }}",
     # a structural local given with a context parameter: teq's local class beside the given (its entry,
     # `enter_local_given_class`), the alias form refused (`local givens cannot take parameters`)
     "structural": "{{ given call(using need: Numeric[String]): Serializable with {{}}; call.hashCode }}"}[CALLEE]
# (name, member template; {E} the expression, {n} the cell's suffix), and the owners it stands in (None: every one)
CONTEXTS = [
    ("a strict val's initializer", "val r{n} = {E}", None),
    ("a pattern val with a tuple", "val (r{n}, s{n}) = ({E}, 2)", None),
    ("a pattern val with an extractor", "val List(r{n}) = List({E}): @unchecked", None),
    ("a pattern val with a typed pattern", "val (r{n}: Int) = {E}", None),
    ("a var's initializer", "var r{n} = {E}", None),
    ("a lazy val's initializer", "lazy val r{n} = {E}", None),
    ("a block in a block", "val r{n} = {{ {{ {E} }} }}", None),
    ("a default argument", "def m{n}(x: Int = {E}): Int = x", None),
    ("a by-name argument", "def byName{n}(x: => Int): Int = x\n  val r{n} = byName{n}({E})", None),
    ("an anonymous function", "val r{n} = (i: Int) => {E}", None),
    ("a partial function's case", "val r{n}: PartialFunction[Int, Int] = {{ case i => {E} }}", None),
    ("a for comprehension's generator", "val r{n} = for (i <- List({E})) yield i", None),
    ("a for comprehension's guard", "val r{n} = for (i <- List(1) if {E} > 0) yield i", None),
    ("a for comprehension's yield", "val r{n} = for (i <- List(1)) yield {E}", None),
    ("a while body", "val r{n} = {{ while (false) {{ {E} }}; 1 }}", None),
    ("a try", "val r{n} = try {E} catch {{ case _: Throwable => 1 }}", None),
    ("a try's handler", "val r{n} = try 1 catch {{ case _: Throwable => {E} }}", None),
    ("a string interpolation's argument", "val r{n} = s\"${{{E}}}\"", None),
    ("a secondary constructor's body", "def this(s{n}: String) = {{ this(); {E} }}", {"class"}),
    ("a class's body statement", "{E}", {"object", "class", "trait", "enum_companion", "given_instance"}),
    ("an early given in a block", "val r{n} = {{ given Int = 1; {E} }}", None),
    ("an if in an initializer", "val r{n} = if (true) {E} else 0", None),
    # the interpolation's argument a block holding the block (k18 writes the definition in `${ }` itself)
    ("a string interpolation's argument, a block in it", "val r{n} = s\"${{ ({E}) }}\"", None),
]
OWNERS = {
    "object": lambda n, m: f"object O{n} {{\n  {m}\n}}\n",
    "class": lambda n, m: f"class C{n}() {{\n  {m}\n}}\n",
    "trait": lambda n, m: f"trait T{n} {{\n  {m}\n}}\n",
    "package_object": lambda n, m: f"package p{n}\npackage object po{n} {{\n  {m}\n}}\n",
    "named_package": lambda n, m: f"package p{n}\n{m}\n",
    "empty_package": lambda n, m: f"{m}\n",
    "enum_companion": lambda n, m: f"enum E{n} {{ case A{n} }}\nobject E{n} {{\n  {m}\n}}\n",
    "given_instance": lambda n, m: f"trait G{n}\ngiven gi{n}: G{n} with {{\n  {m}\n}}\n",
}
cells = []
for owner, wrap in OWNERS.items():
    d = os.path.join(out, owner)
    os.makedirs(d, exist_ok=True)
    for i, (name, member, only) in enumerate(CONTEXTS, 1):
        if only is not None and owner not in only:
            continue
        n = f"{i:02d}"
        text = wrap(n, member.format(E=E.format(), n=n).replace("{{", "{").replace("}}", "}"))
        open(os.path.join(d, f"k{n}.scala"), "w").write(text)
        cells.append((owner, n, name))
with open(os.path.join(out, "cells.tsv"), "w") as f:
    for c in cells:
        f.write("\t".join(c) + "\n")
print(len(cells), "cells in", len(OWNERS), "owners")
