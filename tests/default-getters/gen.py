#!/usr/bin/env python3
"""tests/default-getters/gen.py <out>: the cells of the default getters' table (the local traits' port's Step 0,
owner-shared/packs/ltd-port/step0): <out>/cells/<cell>/Test.scala (the program), Twin.scala (its member twin, where it
has one) or Lib.scala and Main.scala (a library and its client, the split leg's), and <out>/cells.tsv: the cell, its
family, then its marks: `twin` (it has a member twin), `lifted` (the enumerated lifted-override family, by its rule as
coded below: an A-typed call, A's getter reading the block's val, a winner that is a class or a trait whose getter reads
the var or nothing, which scalac's LambdaLift lifts with another signature so that the override is lost; and its member
form, diamond.lifted_member_override), `diamond` (the second family, diamond.unforwarded_diamond), `moves` (state whose allocation or
initialisation the twin moves: a captured var, a lazy or by-name default, a forward reference), `split`.
FAMILIES, generated or read from programs/:
- AXIS: a local trait hierarchy in a def's block, generated from five axes: the DECLARER of the default that wins
  (A, B overriding, B_inherits overriding without one, the class, a superclass, a superclass beside B, a subclass,
  nobody); what the WINNER reads (the block's val, a var set again after the classes, a local def, nothing); what the
  overridden defaults read (the val, nothing); an OVERLOAD in A (none, another arity, a String twin, a zero-argument
  twin); the CALL (direct, through an A-typed value, super.m() from a subclass). Each default and body names its
  declarer (A 100, B 200, C 300, S 400, D 500; the body a * 10 + its owner's digit). Its twin: the same traits and
  classes as members of an object.
- MEMBER: the declarers with the traits as an object's members (no capture): controls.
- NESTED: a local def's own default reading a capture, and a trait default reading a def local to it.
- REVIEW: the two code reviews' programs (programs/review: their probe directory, one file each).
- NATIVE: the native session's cells placed with the port (programs/native: a captured default an override drops,
  and a local trait overriding one alternative of an overloaded method).
- WITNESS: the design read's witnesses (programs/witness/<name>-local.scala, its member twin <name>-member.scala).
- OVERRIDE: the rules session's six escape cases, a local trait's method reading its block overridden by an anonymous
  class (programs/override: a library, its client, the member twin).
- SPLIT: AXIS cells with no overload, called directly or through super, as a library (the hierarchy in Lib.value's
  body) and a client; and the design read's two-module witness (programs/split/products-local).
Written fresh."""
import os, re, shutil, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import diamond  # noqa: E402
OUT = sys.argv[1]
shutil.rmtree(OUT, ignore_errors=True)
os.makedirs(os.path.join(OUT, 'cells'))
rows = []


def put(name, family, files, twin=False, lifted=False, moves=False, split=False, unforwarded=False):
    d = os.path.join(OUT, 'cells', name)
    os.makedirs(d)
    for f, text in files.items():
        open(os.path.join(d, f), 'w').write(text)
    marks = [m for m, on in (('twin', twin), ('lifted', lifted), ('diamond', unforwarded), ('moves', moves), ('split', split)) if on]
    rows.append((name, family, ','.join(marks) or '-'))


BASE = {'A': 100, 'B': 200, 'C': 300, 'S': 400, 'D': 500}
BODY = {'A': 1, 'B': 2, 'C': 3, 'S': 4, 'D': 5}
READ = {'val': ' + x', 'var': ' + v', 'none': '', 'localdef': ' + g()'}
DECLARERS = {'A': ('A', []), 'B': ('B', ['A']), 'B_inherits': ('A', []), 'class': ('C', ['A']), 'superclass': ('S', ['A']),
             'superclass_trait': ('B', ['A', 'S']), 'subclass': ('D', ['A']), 'nobody': (None, [])}
CLASSY = {'class', 'superclass', 'subclass'}


def decls(declarer, wr, lr, overload):
    win, _ = DECLARERS[declarer]

    def dflt(o):
        if declarer == 'nobody':
            return ''
        return f' = {BASE[o]}{READ[wr] if o == win else READ[lr]}'

    def m(o, override=True, default=True):
        return f'{"override " if override else ""}def m(a: Int{dflt(o) if default else ""}): Int = a * 10 + {BODY[o]}'
    ov = {'none': '', 'arity': '; def m(a: Int, b: Int): Int = a + b + 1000', 'string': '; def m(s: String): Int = s.length + 2000',
          'zero': '; def m(): Int = 99'}[overload]
    a = f'trait A {{ {m("A", override=False)}{ov} }}'
    shapes = {
        'A': ([a, 'class C extends A'], 'C'), 'nobody': ([a, 'class C extends A'], 'C'),
        'B': ([a, f'trait B extends A {{ {m("B")} }}', 'class C extends A with B'], 'C'),
        'B_inherits': ([a, f'trait B extends A {{ {m("B", default=False)} }}', 'class C extends A with B'], 'C'),
        'class': ([a, f'class C extends A {{ {m("C")} }}'], 'C'),
        'superclass': ([a, f'class S extends A {{ {m("S")} }}', 'class C extends S'], 'C'),
        'superclass_trait': ([a, f'class S extends A {{ {m("S")} }}', f'trait B extends A {{ {m("B")} }}', 'class C extends S with B'], 'C'),
        'subclass': ([a, 'class C extends A', f'class D extends C {{ {m("D")} }}'], 'D'),
    }
    return shapes[declarer]


def call(declarer, inst, how):
    arg = '7' if declarer == 'nobody' else ''
    if how == 'direct':
        return [f'println(new {inst}().m({arg}))']
    if how == 'super':
        return [f'class E extends {inst} {{ def go: Int = super.m({arg}) }}', 'println(new E().go)']
    return [f'val c: A = new {inst}()', f'println(c.m({arg}))']


def lifted_override(d, wr, lr, ov, how):
    """The enumerated family: an A-typed call, A's getter capturing the block's val, a winner whose getter scalac lifts
    with another signature (a class's, which keeps its captures as fields; a trait's reading the var or nothing)."""
    return how == 'as_A' and lr == 'val' and ov != 'zero' and d != 'nobody' and (d in CLASSY or wr in ('var', 'none'))


for d, (win, losers) in DECLARERS.items():
    for wr in (['none'] if d == 'nobody' else ['val', 'var', 'none', 'localdef']):
        for lr in (['val', 'none'] if losers else ['-']):
            for ov in ('none', 'arity', 'string', 'zero'):
                for how in ('direct', 'as_A', 'super'):
                    if (ov == 'zero' or how == 'super') and lr == 'val' and wr not in ('val', 'none'):
                        continue
                    ds, inst = decls(d, wr, lr if lr != '-' else 'none', ov)
                    calls = call(d, inst, how)
                    local = ('def f(x0: Int): Unit = {\n    val x = x0\n    var v = x0\n    def g(): Int = x + 2\n'
                             + ''.join(f'    {l}\n' for l in ds) + '    v = x0 + 5\n' + ''.join(f'    {l}\n' for l in calls)
                             + '}\n@main def run(): Unit =\n  f(10)\n')
                    twin = ('object Lib {\n  val x = 10\n  var v = 15\n  def g(): Int = x + 2\n' + ''.join(f'  {l}\n' for l in ds)
                            + '}\nimport Lib.*\n@main def run(): Unit =\n' + ''.join(f'  {l}\n' for l in calls))
                    name = f'AXIS-{d}-w{wr}-l{lr}-{ov}-{how}'
                    lifted = lifted_override(d, wr, lr, ov, how)
                    put(name, 'AXIS', {'Test.scala': local, 'Twin.scala': twin}, twin=True, lifted=lifted, moves=lifted and wr == 'var')
                    if ov == 'none' and how in ('direct', 'super'):
                        lib = ('object Lib {\n  def value(x0: Int): Int = {\n    val x = x0\n    var v = x0\n    def g(): Int = x + 2\n'
                               + ''.join(f'    {l}\n' for l in ds) + '    v = x0 + 5\n'
                               + ''.join(f'    {l}\n' for l in calls[:-1]) + f'    {calls[-1][len("println("):-1]}\n  }}\n}}\n')
                        put('SPLIT-' + name[5:], 'SPLIT', {'Lib.scala': lib, 'Main.scala': '@main def run(): Unit = println(Lib.value(10))\n'}, split=True)

for d in DECLARERS:
    for wr in (['none'] if d == 'nobody' else ['none', 'val']):
        ds, inst = decls(d, wr, 'none', 'none')
        text = ('object Lib {\n  val x = 10\n  var v = 10\n' + ''.join(f'  {l}\n' for l in ds) + '}\nimport Lib.*\n@main def run(): Unit =\n'
                + ''.join(f'  {l}\n' for l in call(d, inst, 'direct')))
        put(f'MEMBER-{d}-w{wr}', 'MEMBER', {'Test.scala': text})

NESTED = {
    'def_default_val': 'def f(x: Int): Unit = {\n  def g(a: Int = x + 1): Int = a * 2\n  def h(): Int = g()\n  println(h())\n}',
    'def_default_var': 'def f(x0: Int): Unit = {\n  var v = x0\n  def g(a: Int = v + 1): Int = a * 2\n  v = 20\n  println(g())\n}',
    'def_in_def_default': 'def f(x: Int): Unit = {\n  def outer(): Int = {\n    def inner(a: Int = x + 3): Int = a\n    inner()\n  }\n  println(outer())\n}',
    'trait_default_local_def': 'def f(x: Int): Unit = {\n  trait A { def m(a: Int = { def k(): Int = x + 4; k() }): Int = a }\n  class C extends A\n  println(new C().m())\n}',
}
for n, src in NESTED.items():
    put(f'NESTED-{n}', 'NESTED', {'Test.scala': src + '\n@main def run(): Unit =\n  f(10)\n'})


def program(path):
    """A program file as a cell: its package line dropped, an entry where it has none."""
    src = re.sub(r'^package [A-Za-z0-9_.]+\n', '', open(path).read()).rstrip('\n')
    return src + ('' if '@main def run()' in src else '\n@main def run(): Unit = Main.main(Array())') + '\n'


def program_cell(name, family, path):
    """A program file's cell; a program the second family's rule marks (diamond.py: scalac's lifted getters get no
    forwarder) gets its member twin made by diamond.member_twin."""
    raw = open(path).read()
    files = {'Test.scala': program(path)}
    unf = diamond.unforwarded_diamond(raw)
    if unf:
        twin = diamond.member_twin(re.sub(r'^package [A-Za-z0-9_.]+\n', '', raw))
        if twin:
            files['Twin.scala'] = twin.rstrip('\n') + '\n@main def run(): Unit = Main.main(Array())\n'
    put(name, family, files, twin='Twin.scala' in files, unforwarded=unf, moves=unf and diamond.moves_state(raw))


P = os.path.join(HERE, 'programs')
for f in sorted(os.listdir(os.path.join(P, 'review'))):
    program_cell('REVIEW-' + f[:-len('.scala')], 'REVIEW', os.path.join(P, 'review', f))
for f in sorted(os.listdir(os.path.join(P, 'native'))):
    program_cell('NATIVE-' + f[:-len('.scala')], 'NATIVE', os.path.join(P, 'native', f))
for f in sorted(os.listdir(os.path.join(P, 'witness'))):
    name = f[:-len('.scala')]
    files = {'Test.scala': program(os.path.join(P, 'witness', f))}
    if name.endswith('-local'):
        twin = os.path.join(P, 'witness', name[:-len('-local')] + '-member.scala')
        if os.path.exists(twin):
            files['Twin.scala'] = program(twin)
    # The read's own exceptions to a twin: a repeated invocation (repeat) and a local refusal (init) are its
    # negatives, never judged by the twin; `lift` is the family's witness.
    base = name.rsplit('-', 1)[0]
    raw = open(os.path.join(P, 'witness', f)).read()
    put('WITNESS-' + name, 'WITNESS', files, twin='Twin.scala' in files, lifted=base == 'lift' and name.endswith('-local'),
        moves=base in ('repeat', 'init', 'lazybyname'), unforwarded=diamond.unforwarded_diamond(raw))
# OVERRIDE: the first family's member form (the rules session's six escape cases): a local trait's method reading its
# block, overridden by an anonymous class; a library, its client, and the member twin (both made one file).
op = os.path.join(P, 'override')
for d in sorted(os.listdir(op)):
    files = {f: open(os.path.join(op, d, f)).read() for f in sorted(os.listdir(os.path.join(op, d)))}
    lib = files['Lib.scala']
    put('OVERRIDE-' + d, 'OVERRIDE', files, twin='Twin.scala' in files, lifted=diamond.lifted_member_override(lib),
        moves=diamond.moves_state(lib), split=True)
sp = os.path.join(P, 'split')
for d in sorted(os.listdir(sp)):
    files = {f: open(os.path.join(sp, d, f)).read() for f in sorted(os.listdir(os.path.join(sp, d)))}
    put('SPLIT-witness-' + d, 'SPLIT', files, split=True)

open(os.path.join(OUT, 'cells.tsv'), 'w').write(''.join(f'{n}\t{f}\t{m}\n' for n, f, m in rows))
fams = {}
for _, f, m in rows:
    fams[f] = fams.get(f, 0) + 1
lifted = [n for n, _, m in rows if 'lifted' in m]
unf = [n for n, _, m in rows if 'diamond' in m]
print(f'{len(rows)} cells {fams}; the lifted-override family {len(lifted)} cells ({sum(1 for n in lifted if n.startswith("AXIS-"))} AXIS, '
      f'{sum(1 for n in lifted if n.startswith("OVERRIDE-"))} OVERRIDE, {sum(1 for n in lifted if n.startswith("WITNESS-"))} WITNESS), '
      f'{sum(1 for n, _, m in rows if "lifted" in m and "moves" in m)} of them moving state; the unforwarded-diamond family '
      f'{len(unf)} cells: ' + ' '.join(unf))
