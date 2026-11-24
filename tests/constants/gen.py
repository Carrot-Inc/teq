#!/usr/bin/env python3
"""gen.py <out>: the cells of a constant val read from another file of the same compilation, for one form per
compilation: <out>/cells.tsv (name, family), <out>/cells/<cell>/{Defs.scala,Use.scala}. Each cell defines its vals in
Defs.scala and reads them in Use.scala's `@main def run`, printing them.
- KIND, the val:
    inl-single-lib   `inline val b: Int.MaxValue.type = Int.MaxValue` (a declared singleton of a library constant);
    inl-single-loc   `inline val c: t.type = t`, `final val t = true` beside it (a declared singleton of a final val);
    inl-lit          `inline val d: 3 = 3` (a declared literal type);
    inl-infer        `inline val e = 4` (inferred);
    fin-infer        `final val f = 5`;
    fin-lit          `final val g: 6 = 6`;
    fin-single       `final val h: t.type = t`;
    fin-single-lib   `final val k: Long.MaxValue.type = Long.MaxValue`;
    str-inl          `inline val s = "s"`;
- OWNER: an object (`object C`), the top level of Defs.scala, a class's companion object (`object K` of `class K`),
  and through an export (`object X { export C.* }`, read `X.<name>`).
- READ: from Use.scala, as `Owner.name` (or the top-level name)."""
import os, shutil, sys

OUT = sys.argv[1]
shutil.rmtree(OUT, ignore_errors=True)
os.makedirs(os.path.join(OUT, 'cells'))
KINDS = {
    'inl-single-lib': ('inline val b: Int.MaxValue.type = Int.MaxValue', 'b', []),
    'inl-single-loc': ('inline val c: t.type = t', 'c', ['final val t = true']),
    'inl-lit': ('inline val d: 3 = 3', 'd', []),
    'inl-infer': ('inline val e = 4', 'e', []),
    'fin-infer': ('final val f = 5', 'f', []),
    'fin-lit': ('final val g: 6 = 6', 'g', []),
    'fin-single': ('final val h: t.type = t', 'h', ['final val t = true']),
    'fin-single-lib': ('final val k: Long.MaxValue.type = Long.MaxValue', 'k', []),
    'str-inl': ('inline val s = "s"', 's', []),
}
rows = []
for kind, (decl, name, extra) in KINDS.items():
    for owner in ('object', 'toplevel', 'companion', 'export'):
        body = '\n'.join('  ' + x for x in extra + [decl])
        if owner == 'object':
            defs, ref = f'object C:\n{body}\n', f'C.{name}'
        elif owner == 'toplevel':
            defs, ref = '\n'.join(extra + [decl]) + '\n', name
        elif owner == 'companion':
            defs, ref = f'class K\nobject K:\n{body}\n', f'K.{name}'
        else:
            defs, ref = f'object C:\n{body}\nobject X:\n  export C.*\n', f'X.{name}'
        use = f'@main def run(): Unit =\n  println({ref})\n  println(({ref}, {ref}))\n'
        cell = f'{kind}-{owner}'
        d = os.path.join(OUT, 'cells', cell)
        os.makedirs(d)
        open(os.path.join(d, 'Defs.scala'), 'w').write(f'// {cell}: the val read from another file\n' + defs)
        open(os.path.join(d, 'Use.scala'), 'w').write(use)
        rows.append((cell, kind))
open(os.path.join(OUT, 'cells.tsv'), 'w').write(''.join(f'{n}\t{f}\n' for n, f in rows))
print(len(rows), 'cells')
