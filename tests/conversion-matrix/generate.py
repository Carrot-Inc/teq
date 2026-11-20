#!/usr/bin/env python3
"""The conversion search's matrix: two-file programs (a library `lib`, a user `use`) whose output, or whose refusal,
says which implicit conversion of several a selection `true.shout` (or an expected type) takes. scalac 3.8.4's answer
for each is recorded in scalac.jsonl beside this file; run.py builds each with teq and holds it to that answer.

generate.py <out dir> writes <out>/<set>/<name>/*.scala for eleven sets:
  axes      an import under test and a rival of one name (`conv`): form x source x candidate x rival x the inner
            candidate's fit;
  holes     a rival of another precedence (named or renamed against a wildcard, `given` or `{given, *}`), and a
            package as the source;
  frames    the two candidates in the file, an object's body, a method's block, a block inside it, a using
            parameter, an object's given or member;
  fits      the inner candidate's parameter of a numeric, union, boxed, by-name or polymorphic shape under a fitting
            outer one;
  access    a candidate of an access modifier imported in a block from outside its package, inside it, from a
            subclass, or from a trait whose self type is its class;
  defining  a given conversion used inside its own right-hand side, and the package's own given beside an imported
            conversion of another name;
  bounds    a `given` selector with a type bound beside the clause's other selectors (`bounds` below);
  singletons a candidate whose parameter is a singleton: a literal's type, a path's, a `Singleton` bound
            (`singletons` below), and a Conversion value's branch;
  late      a given used in its own right-hand side beside outer givens of its name and of another (`late` below);
  latescope the late set over a class of the library's, crossed with a conversion in the implicit scope, the code
            review's shape and a deepened ambiguity (`latescope` below);
  entries   the search's entries (the first search, the retry, the deepened search, a deepened ambiguity) x where the
            conversion is (the first level, an outer context, the implicit scope, nowhere) (`entries` below).
The axes:
  form       wild `X.*`, named `X.conv`, renamed `X.{conv as cv}`, hidden `X.{other as _, *}`, given `X.given`,
             bytype `X.given Conversion[?, ?]`, givenwild `X.{given, *}`;
  source     obj (`lib.O`, the rival `lib.P`), val (`lib.H.v`, the rival `lib.K.w`), pkg (`lib.pk`, `lib.pq`);
  candidate  def (an implicit def), iclass (an implicit class), given (a given Conversion), ival (an implicit val of
             a function type);
  rival      none, earlier (the rival's import before), later (after), pkg (a definition of the file's package),
             block (the rival's import in main's block);
  fit        fits; arg (takes an Int); member (its result has no `shout`); private (`shout` is private); missing (needs
             a given the program lacks)."""
import os
import sys

FORMS = {
    'wild': '{x}.*', 'named': '{x}.conv', 'renamed': '{x}.{{conv as cv}}', 'hidden': '{x}.{{other as _, *}}',
    'given': '{x}.given', 'bytype': '{x}.given Conversion[?, ?]', 'givenwild': '{x}.{{given, *}}',
}
SOURCES = {'obj': ('lib.O', 'lib.P'), 'val': ('lib.H.v', 'lib.K.w')}
FITS = ['fits', 'arg', 'member', 'private', 'missing']
LIB = '''package lib
import scala.language.implicitConversions
class Rich(val tag: String) {{ def shout: String = tag }}
class Quiet(val tag: String) {{ def quiet: String = tag }}
class Hidden(val tag: String) {{ private def shout: String = tag; def peek: String = shout }}
trait Missing
{holders}
'''
USE_HEAD = 'package use\nimport scala.language.implicitConversions\n'


def cand(kind, fit, tag):
    """The candidate `conv` of a kind and a fit, printing `tag`."""
    res = {'fits': 'Rich', 'arg': 'Rich', 'member': 'Quiet', 'private': 'Hidden', 'missing': 'Rich'}[fit]
    arg = 'Int' if fit == 'arg' else 'Boolean'
    if kind == 'def':
        using = '(implicit m: Missing)' if fit == 'missing' else ''
        return f'implicit def conv(b: {arg}){using}: {res} = new {res}("{tag}")'
    if kind == 'iclass':
        using = '(implicit m: Missing)' if fit == 'missing' else ''
        body = {'member': f'def quiet: String = "{tag}"',
                'private': f'private def shout: String = "{tag}"; def peek: String = shout'}.get(fit, f'def shout: String = "{tag}"')
        return f'implicit class conv(b: {arg}){using} {{ {body} }}'
    if kind == 'given':
        if fit == 'missing':
            return f'given conv(using m: Missing): Conversion[Boolean, Rich] = b => new Rich("{tag}")'
        return f'given conv: Conversion[{arg}, {res}] = b => new {res}("{tag}")'
    if kind == 'ival':
        if fit == 'missing':
            return f'implicit def conv(implicit m: Missing): Boolean => Rich = b => new Rich("{tag}")'
        return f'implicit val conv: {arg} => {res} = b => new {res}("{tag}")'
    raise ValueError(kind)


def holders(src, outer, inner):
    """The two holders of the candidates: objects O and P, or values H.v and K.w of classes C and D."""
    if src == 'obj':
        return f'object O {{\n  {outer}\n  def other: Int = 0\n}}\nobject P {{\n  {inner}\n  def other: Int = 0\n}}'
    return (f'class C {{\n  {outer}\n  def other: Int = 0\n}}\nobject H {{ val v: C = new C }}\n'
            f'class D {{\n  {inner}\n  def other: Int = 0\n}}\nobject K {{ val w: D = new D }}')


def write(out, name, files):
    d = os.path.join(out, name)
    os.makedirs(d, exist_ok=True)
    for fn, text in files.items():
        with open(os.path.join(d, fn), 'w') as f:
            f.write(text)


def main_of(top, blk=''):
    return USE_HEAD + '\n'.join(top) + f'\n@main def run(): Unit = {{\n{blk}  println(true.shout)\n}}\n'


def axes(out):
    for form, ftext in FORMS.items():
        for src, (x, r) in SOURCES.items():
            for kind in ['def', 'iclass', 'given', 'ival']:
                for rival in ['none', 'earlier', 'later', 'pkg', 'block']:
                    for fit in FITS:
                        inner_is_test = rival in ('none', 'earlier', 'pkg')
                        tfit, rfit = (fit, 'fits') if inner_is_test else ('fits', fit)
                        ttag = 'object' if src == 'obj' else 'value'
                        ti, ri = 'import ' + ftext.format(x=x), 'import ' + ftext.format(x=r)
                        top = {'none': [ti], 'earlier': [ri, ti], 'later': [ti, ri], 'pkg': [ti], 'block': [ti]}[rival]
                        blk = f'  {ri}\n' if rival == 'block' else ''
                        files = {'a.scala': LIB.format(holders=holders(src, cand(kind, tfit, ttag), cand(kind, rfit, 'other'))),
                                 'b.scala': main_of(top, blk)}
                        if rival == 'pkg':
                            files['c.scala'] = USE_HEAD + 'import lib.*\n' + cand(kind, 'fits', 'package') + '\n'
                        write(out, f'{form}-{src}-{kind}-{rival}-{fit}', files)


def holes(out):
    nameds, wilds = ['named', 'renamed'], ['wild', 'given', 'givenwild']
    for tf, rf in [(t, r) for t in nameds for r in wilds] + [(t, r) for t in wilds for r in nameds]:
        for src, (x, r) in SOURCES.items():
            for kind in ['def', 'iclass', 'given']:
                for rival in ['earlier', 'later']:
                    for fit in FITS:
                        tfit, rfit = (fit, 'fits') if rival == 'earlier' else ('fits', fit)
                        tc, rc = cand(kind, tfit, 'object' if src == 'obj' else 'value'), cand(kind, rfit, 'other')
                        ti, ri = 'import ' + FORMS[tf].format(x=x), 'import ' + FORMS[rf].format(x=r)
                        top = [ri, ti] if rival == 'earlier' else [ti, ri]
                        write(out, f'x{tf}~{rf}-{src}-{kind}-{rival}-{fit}',
                              {'a.scala': LIB.format(holders=holders(src, tc, rc)), 'b.scala': main_of(top)})
    lib = LIB.format(holders='')
    for form, ftext in FORMS.items():
        for kind in ['def', 'iclass', 'given']:
            for rival in ['none', 'earlier', 'later', 'pkg', 'block']:
                for fit in FITS:
                    inner_is_test = rival in ('none', 'earlier', 'pkg')
                    tfit, rfit = (fit, 'fits') if inner_is_test else ('fits', fit)
                    pk = f'package lib.pk\nimport scala.language.implicitConversions\nimport lib.*\n{cand(kind, tfit, "object")}\ndef other: Int = 0\n'
                    pq = f'package lib.pq\nimport scala.language.implicitConversions\nimport lib.*\n{cand(kind, rfit, "other")}\ndef other: Int = 0\n'
                    ti, ri = 'import ' + ftext.format(x='lib.pk'), 'import ' + ftext.format(x='lib.pq')
                    top = {'none': [ti], 'earlier': [ri, ti], 'later': [ti, ri], 'pkg': [ti], 'block': [ti]}[rival]
                    blk = f'  {ri}\n' if rival == 'block' else ''
                    files = {'a.scala': lib, 'b.scala': main_of(top, blk), 'p.scala': pk, 'q.scala': pq}
                    if rival == 'pkg':
                        files['c.scala'] = USE_HEAD + 'import lib.*\n' + cand(kind, 'fits', 'package') + '\n'
                    write(out, f'{form}-pkg-{kind}-{rival}-{fit}', files)


# The frames: X in the outer place, Y in the inner; F the file's imports, C the body of `object Q`, B1 the block of
# `Q.f`, B2 a block inside it, B1+ a later import of B1; Cdef a member of Q, B1def a definition of f's block, using a
# using parameter of f, given a given of Q.
PLACES = [('F', 'C'), ('F', 'B1'), ('C', 'B1'), ('B1', 'B2'), ('F', 'B2'), ('B1', 'B1+'),
          ('Cdef', 'B1'), ('B1def', 'B1'), ('using', 'B1'), ('given', 'B1'), ('F', 'B1def'), ('C', 'B1def'), ('F', 'Cdef')]
IMPORT_PLACES = ('F', 'C', 'B1', 'B2', 'B1+')


def frames(out):
    for xp, yp in PLACES:
        x_imp, y_imp = xp in IMPORT_PLACES, yp in IMPORT_PLACES
        forms = ['wild', 'named', 'renamed', 'given', 'hidden']
        kinds = ['given'] if 'using' in (xp, yp) else ['def', 'iclass', 'given']
        for form in forms:
            for src in ['obj', 'val']:
                xsrc, ysrc = SOURCES[src]
                for kind in kinds:
                    for fit in FITS:
                        imp = {}
                        if x_imp:
                            imp[xp] = 'import ' + FORMS[form].format(x=xsrc)
                        if y_imp:
                            imp[yp] = 'import ' + FORMS[form].format(x=ysrc)
                        defs = {}
                        if not x_imp and xp not in ('using', 'given'):
                            defs[xp] = cand(kind, 'fits', 'outer')
                        if not y_imp and yp not in ('using', 'given'):
                            defs[yp] = cand(kind, fit, 'inner')
                        oc = cand(kind, 'fits', 'outer') if x_imp else 'def unused: Int = 0'
                        pc = cand(kind, fit, 'inner') if y_imp else 'def unused: Int = 0'
                        params, call = '', ''
                        if 'using' in (xp, yp):
                            ufit = 'fits' if xp == 'using' else fit
                            utag = 'outer' if xp == 'using' else 'inner'
                            res = {'fits': 'Rich', 'arg': 'Rich', 'member': 'Quiet', 'private': 'Hidden', 'missing': 'Rich'}[ufit]
                            argt = 'Int' if ufit == 'arg' else 'Boolean'
                            params = f'(using conv: Conversion[{argt}, {res}])'
                            call = f'(using (b: {argt}) => new {res}("{utag}"))'
                        lines = [USE_HEAD.rstrip('\n'), 'import lib.*']
                        if 'F' in imp:
                            lines.append(imp['F'])
                        lines.append('object Q {')
                        if 'C' in imp:
                            lines.append('  ' + imp['C'])
                        if 'Cdef' in defs:
                            lines.append('  ' + defs['Cdef'])
                        if 'given' in (xp, yp):
                            lines.append('  ' + cand('given', 'fits' if xp == 'given' else fit, 'outer' if xp == 'given' else 'inner'))
                        lines.append(f'  def f{params}: String = {{')
                        if 'B1' in imp:
                            lines.append('    ' + imp['B1'])
                        if 'B1def' in defs:
                            lines.append('    ' + defs['B1def'])
                        if 'B1+' in imp:
                            lines.append('    ' + imp['B1+'])
                        if 'B2' in imp:
                            lines += ['    {', '      ' + imp['B2'], '      true.shout', '    }']
                        else:
                            lines.append('    true.shout')
                        lines += ['  }', '}', f'@main def run(): Unit = println(Q.f{call})']
                        name = f'{xp}~{yp}-{form}-{src}-{kind}-{fit}'.replace('+', 'p')
                        write(out, name, {'a.scala': LIB.format(holders=holders(src, oc, pc)), 'b.scala': '\n'.join(lines) + '\n'})


FIT_PARAMS = {
    'Long': 'x: Long', 'Double': 'x: Double', 'Int': 'x: Int', 'Short': 'x: Short', 'Byte': 'x: Byte', 'Char': 'x: Char',
    'Any': 'x: Any', 'AnyVal': 'x: AnyVal', 'IntOrString': 'x: Int | String', 'Integer': 'x: java.lang.Integer',
    'byname': 'x: => Int', 'polyAnyVal': '[T <: AnyVal](x: T', 'poly': '[T](x: T', 'polyString': '[T <: String](x: T',
}
FIT_ARGS = {'lit': '1', 'var': 'n', 'char': "'a'", 'long': '1L', 'byte': 'bt'}


def fits(out):
    for pn, p in FIT_PARAMS.items():
        sig = f'conv{p})' if p.startswith('[') else f'conv({p})'
        lib = ('package lib\nimport scala.language.implicitConversions\nclass Rich(val tag: String) { def shout: String = tag }\n'
               f'object O {{\n  implicit def {sig}: Rich = new Rich("inner")\n}}\n'
               'object P {\n  implicit def conv(i: Int): Rich = new Rich("outer")\n}\n')
        for an, a in FIT_ARGS.items():
            for pick in ['sel', 'exp']:
                use = f'println(({a}).shout)' if pick == 'sel' else f'val r: Rich = {a}\n    println(r.shout)'
                b = (USE_HEAD + 'import lib.*\nimport lib.P.*\n@main def run(): Unit = {\n  val n: Int = 1\n  val bt: Byte = 1\n'
                     f'  {{\n    import lib.O.*\n    {use}\n  }}\n}}\n')
                write(out, f'{pn}-{an}-{pick}', {'a.scala': lib, 'b.scala': b})


ACCESS_MODS = {'pub': '', 'private': 'private ', 'protected': 'protected ', 'privlib': 'private[lib] ', 'protlib': 'protected[lib] '}
ACCESS_SITES = {'obj': ['out', 'inlib'], 'val': ['out', 'inlib', 'subother', 'subme', 'selftype']}


def access(out):
    """A candidate of an access modifier, imported in a block (the inner place) from outside its package, inside it,
    from a subclass through another value or through itself, or from a trait whose self type is its class; a public
    rival `lib.P.conv` at the file's top, earlier in the same block, or none."""
    for src, sites in ACCESS_SITES.items():
        for site in sites:
            for mod, m in ACCESS_MODS.items():
                for kind in ['def', 'given']:
                    tag = 'object' if src == 'obj' else 'value'
                    made = f'new Rich("{tag} " + origin)' if src == 'val' else f'new Rich("{tag}")'
                    c = (f'{m}implicit def conv(b: Boolean): Rich = {made}' if kind == 'def'
                         else f'{m}given conv: Conversion[Boolean, Rich] = b => {made}')
                    lib = ('package lib\nimport scala.language.implicitConversions\nclass Rich(val tag: String) { def shout: String = tag }\n'
                           f'class C(val origin: String) {{\n  {c if src == "val" else "def unused: Int = 0"}\n  def other: Int = 0\n}}\nobject H {{ val v: C = new C("v") }}\n'
                           f'object O {{\n  {c if src == "obj" else "def unused: Int = 0"}\n  def other: Int = 0\n}}\n'
                           'object P { implicit def conv(b: Boolean): Rich = new Rich("other") }\n')
                    for form in ['wild', 'named']:
                        for rival in ['none', 'file', 'same']:
                            prefix = {'obj': 'lib.O', 'val': 'lib.H.v'}[src]
                            if site == 'subme':
                                prefix = 'me'
                            sel = (('*' if kind == 'def' else 'given') if form == 'wild' else 'conv')
                            imp = f'import {prefix}.{sel}'
                            pkg = 'lib.app' if site == 'inlib' else 'use'
                            top = [f'package {pkg}', 'import scala.language.implicitConversions']
                            if rival == 'file':
                                top.append('import lib.P.*')
                            body = (['    import lib.P.*'] if rival == 'same' else []) + [f'    {imp}', '    true.shout']
                            fdef = ['  def f: String = {'] + body + ['  }']
                            if site in ('out', 'inlib'):
                                cls, call = ['object Q {'] + fdef + ['}'], 'Q.f'
                            elif site == 'subother':
                                cls, call = ['class Sub extends lib.C("sub") {'] + fdef + ['}'], 'new Sub().f'
                            elif site == 'subme':
                                cls, call = ['class Sub extends lib.C("sub") {', '  val me: Sub = this'] + fdef + ['}'], 'new Sub().f'
                            else:
                                cls, call = (['trait Side { self: lib.C =>'] + fdef + ['}', 'class Both extends lib.C("this") with Side'],
                                             'new Both().f')
                            b = '\n'.join(top + cls + [f'@main def run(): Unit = println({call})']) + '\n'
                            write(out, f'{src}-{site}-{mod}-{kind}-{form}-{rival}', {'a.scala': lib, 'b.scala': b})


DEFINING_USES = {
    'sel': ['println("in " + 1.shout)'],
    'exp': ['val early: Rich = 7', 'println("in " + early.tag)'],
    'lambda': ['val g = (q: Int) => q.shout', 'println("in " + g(1))'],
    'localdef': ['def local(q: Int): String = q.shout', 'println("in " + local(1))'],
}


def defining(out):
    """A given conversion used inside its own right-hand side (`comesTooLate`): at the top of the file's package, in an
    object, in a class, in a block; without a rival, or with a conversion of another name imported at the file's top."""
    lib = ('package lib\nimport scala.language.implicitConversions\nclass Rich(val tag: String) { def shout: String = tag }\n'
           'object Far { given far: Conversion[Int, Rich] = i => new Rich("far" + i) }\n')
    for place in ['top', 'obj', 'cls', 'blk']:
        for use, stmts in DEFINING_USES.items():
            for rival in ['none', 'outer']:
                ind = '  ' if place in ('obj', 'cls') else ''
                inner = ind + '  '
                g = ([f'{ind}given conv: Conversion[Int, Rich] = {{'] + [inner + x for x in stmts]
                     + [inner + '(i: Int) => new Rich("conv" + i)', ind + '}'])
                lines = ['package use', 'import scala.language.implicitConversions', 'import lib.*']
                if rival == 'outer':
                    lines.append('import lib.Far.given')
                if place == 'top':
                    lines += g + ['@main def run(): Unit = println(5.shout)']
                elif place == 'obj':
                    lines += ['object Q {'] + g + ['}', '@main def run(): Unit = {', '  import Q.given', '  println(5.shout)', '}']
                elif place == 'cls':
                    lines += ['class K {'] + g + ['}', '@main def run(): Unit = {', '  val k = new K', '  import k.given',
                                                   '  println(5.shout)', '}']
                else:
                    lines += ['@main def run(): Unit = {'] + ['  ' + x for x in g] + ['  println(5.shout)', '}']
                write(out, f'{place}-{use}-{rival}', {'a.scala': lib, 'b.scala': '\n'.join(lines) + '\n'})


BOUND_FORMS = {
    'only': '{x}.{{given {b}}}', 'wild': '{x}.{{given {b}, *}}', 'named': '{x}.{{conv, given {b}}}',
    'renamed': '{x}.{{conv as cv, given {b}}}', 'unimport': '{x}.{{conv as _, given {b}, *}}',
}
BOUNDS = {'conv': 'Conversion[Boolean, lib.Rich]', 'fun': '(Boolean => lib.Rich)', 'other': 'Conversion[String, lib.Rich]', 'any': 'Any'}


def bounds(out):
    """A `given` selector with a type bound beside the clause's other selectors (`ImportInfo.importedImplicits`: a
    statement with a `given` wildcard brings every implicit member it neither excludes nor names by the given bound,
    `*` adding nothing of its own; a named or renamed member comes in whatever the bound): the form (the bounded
    `given` alone, beside `*`, beside the named `conv`, beside its renaming, beside its unimport and `*`) x the bound
    (fitting a given Conversion, fitting a function, fitting neither, `Any`) x the candidate's kind x the source
    (an object, a value) x a rival implicit def of `conv`'s name imported by `*` (none, before, after)."""
    for form, ftext in BOUND_FORMS.items():
        for bname, btext in BOUNDS.items():
            for src, (x, r) in SOURCES.items():
                for kind in ['def', 'iclass', 'given', 'ival']:
                    for rival in ['none', 'earlier', 'later']:
                        ttag = 'object' if src == 'obj' else 'value'
                        ti, ri = 'import ' + ftext.format(x=x, b=btext), f'import {r}.*'
                        top = {'none': [ti], 'earlier': [ri, ti], 'later': [ti, ri]}[rival]
                        files = {'a.scala': LIB.format(holders=holders(src, cand(kind, 'fits', ttag), cand('def', 'fits', 'other'))),
                                 'b.scala': main_of(top)}
                        write(out, f'{form}-{bname}-{src}-{kind}-{rival}', files)


SINGLE_PARAMS = {'lit': 'true', 'path': 'S.b.type', 'single': 'T', 'wide': 'Boolean'}


def single_cand(kind, shape):
    """The inner candidate `conv` taking a parameter of the shape, printing `inner`."""
    tp = '[T <: Singleton]' if shape == 'single' else ''
    p = SINGLE_PARAMS[shape]
    if kind == 'def':
        return f'implicit def conv{tp}(b: {p}): Rich = new Rich("inner")'
    if kind == 'given':
        return f'given conv{tp}: Conversion[{p}, Rich] = b => new Rich("inner")'
    return f'implicit class conv{tp}(b: {p}) {{ def shout: String = "inner" }}'


SINGLE_FIXED = {
    # dotty's tests/pos/i7413.scala made to run: a context function's parameter of a Conversion from a literal type.
    'fixture-context-function': """import scala.language.implicitConversions
trait Fixture[A] extends Conversion[0, A]
trait TestFramework[A]:
  def fixture: Fixture[A]
  extension (testName: String) def in(test: Fixture[A] ?=> Unit): Unit = test(using fixture)
trait Greeter:
  def greet(name: String): String = s"Hello $name"
case class MyFixture(name: String, greeter: Greeter)
class MyTest extends TestFramework[MyFixture]:
  def fixture = new Fixture[MyFixture] { def apply(x: 0) = MyFixture("ann", new Greeter {}) }
  "say hello" in { println(0.greeter.greet(0.name)) }
@main def run(): Unit = new MyTest
""",
    'literal-given-top': """import scala.language.implicitConversions
case class Fx(name: String)
given c: Conversion[0, Fx] = _ => Fx("top")
@main def run(): Unit = println(0.name)
""",
    'literal-context-function': """import scala.language.implicitConversions
case class Fx(name: String)
def in(test: Conversion[0, Fx] ?=> Unit): Unit = test(using _ => Fx("ctx"))
@main def run(): Unit = in { println(0.name) }
""",
    'literal-def': """import scala.language.implicitConversions
case class Fx(name: String)
implicit def lit(x: 0): Fx = Fx("method")
@main def run(): Unit = println(0.name)
""",
    'literal-using': """import scala.language.implicitConversions
case class Fx(name: String)
def show(using c: Conversion[0, Fx]): Unit = println(0.name)
@main def run(): Unit = show(using _ => Fx("using"))
""",
    'wide-context-function': """import scala.language.implicitConversions
case class Fx(name: String)
def in(test: Conversion[Boolean, Fx] ?=> Unit): Unit = test(using _ => Fx("ctxbool"))
@main def run(): Unit = in { println(true.name) }
""",
}

# A value of a Conversion type against a rival (candidateKind's value branch), each in a block under an import.
VALUE_INNER = {
    'unfit-namesake': 'given conv: Conversion[Int, Rich] = _ => Rich("inner")',
    'unfit-other-name': 'given other: Conversion[Int, Rich] = _ => Rich("inner")',
    'literal-namesake': 'given conv: Conversion[true, Rich] = _ => Rich("literal")',
}


def singletons(out):
    """A candidate whose parameter is a singleton (`ContextualImplicits.widenSingleton` widens it before the fit): its
    shape (a literal `true`, a path's `S.b.type`, a `T <: Singleton` bound, `Boolean` the control) x its kind (an
    implicit def, a given Conversion, an implicit class) x its place (imported at the file's top, imported in main's
    block, a method's using parameter, a context function's parameter; the last two a Conversion's) x a rival
    `conv(b: Boolean)` of the file's imports (none, outer); the receiver `true`, or `lib.S.b` for the path. And the
    programs that showed the family (dotty's pos/i7413 made to run, a literal's conversion at the top, in a context
    function, by a def, by a using parameter) and a Conversion value's branch (a namesake or another name that does
    not fit, a literal `from`, a `<:<` evidence, a function value that is no view)."""
    lib_head = ('package lib\nimport scala.language.implicitConversions\nclass Rich(val tag: String) { def shout: String = tag }\n'
                'object S { val b: true = true }\n'
                'object P {\n  implicit def conv(b: Boolean): Rich = new Rich("outer")\n}\n')
    for shape in SINGLE_PARAMS:
        recv = 'lib.S.b' if shape == 'path' else 'true'
        for kind in ['def', 'given', 'iclass']:
            lib = lib_head + f'object O {{\n  {single_cand(kind, shape)}\n  def other: Int = 0\n}}\n'
            for rival in ['none', 'outer']:
                top = ['import lib.P.*'] if rival == 'outer' else []
                write(out, f'{shape}-{kind}-file-{rival}', {'a.scala': lib, 'b.scala': USE_HEAD + '\n'.join(top + ['import lib.O.*'])
                      + f'\n@main def run(): Unit = {{\n  println({recv}.shout)\n}}\n'})
                write(out, f'{shape}-{kind}-block-{rival}', {'a.scala': lib, 'b.scala': USE_HEAD + '\n'.join(top)
                      + f'\n@main def run(): Unit = {{\n  import lib.O.*\n  println({recv}.shout)\n}}\n'})
            if kind != 'given' or shape == 'single':
                continue
            p = SINGLE_PARAMS[shape].replace('S.b', 'lib.S.b')
            for rival in ['none', 'outer']:
                top = ['import lib.P.*'] if rival == 'outer' else []
                use = USE_HEAD + 'import lib.Rich\n' + '\n'.join(top) + '\n'
                write(out, f'{shape}-given-using-{rival}', {'a.scala': lib_head, 'b.scala': use
                      + f'def show(using conv: Conversion[{p}, Rich]): Unit = println({recv}.shout)\n'
                      + '@main def run(): Unit = show(using b => new Rich("inner"))\n'})
                write(out, f'{shape}-given-context-{rival}', {'a.scala': lib_head, 'b.scala': use
                      + f'def in(test: Conversion[{p}, Rich] ?=> Unit): Unit = test(using b => new Rich("inner"))\n'
                      + f'@main def run(): Unit = in {{ println({recv}.shout) }}\n'})
    for name, text in SINGLE_FIXED.items():
        write(out, name, {'a.scala': text})
    for name, inner in VALUE_INNER.items():
        write(out, f'value-{name}', {'a.scala': 'import scala.language.implicitConversions\nclass Rich(val tag: String) { def shout = tag }\n'
              'object Outer { given conv: Conversion[Boolean, Rich] = _ => Rich("outer") }\nimport Outer.given\n'
              f'object Main:\n  def run() =\n    {inner}\n    println(true.shout)\n@main def m(): Unit = Main.run()\n'})
    write(out, 'value-subtype-evidence', {'a.scala': 'class Rich(val tag: String) { def shout = tag }\n'
          'def f[A](a: A)(using ev: A <:< Rich): String = a.shout\n@main def m(): Unit = println(f(Rich("ev")))\n'})
    write(out, 'value-function-no-view', {'a.scala': 'class Rich(val tag: String) { def shout = tag }\n'
          'object Main:\n  def run() =\n    implicit val fn: Boolean => Rich = _ => Rich("fn")\n    println(true.shout)\n'
          '@main def m(): Unit = Main.run()\n'})


LATE_LIB = ('package lib\nimport scala.language.implicitConversions\nclass Rich(val tag: String) { def shout: String = tag }\n'
            'object Far { given conv: Conversion[Int, Rich] = i => new Rich("far" + i) }\n'
            'object Near { given alt: Conversion[Int, Rich] = i => new Rich("alt" + i) }\n'
            'object Mid { given alt2: Conversion[Int, Rich] = i => new Rich("mid" + i) }\n')
LATE_RIVALS = {'none': [], 'namesake': ['lib.Far'], 'other': ['lib.Near'], 'both': ['lib.Far', 'lib.Near'],
               'twoothers': ['lib.Near', 'lib.Mid']}
LATE_GIVEN = ('given conv: Conversion[Int, Rich] = {{\n{ind}  println("in " + 1.shout)\n{ind}  (i: Int) => new Rich("conv" + i)\n{ind}}}')


def late(out, sink=None):
    """A given used in its own right-hand side (`comesTooLate`, Implicits.scala 1717-1744, and searchImplicit's retry,
    1786-1809): the given eligible and hiding its namesakes, left out after the hiding, and the search again from the
    next context out where nothing else is found and no ambiguity stops it. Its place (an object's body, a class's, a
    method's block, a block inside it) x the outer givens (none, a namesake `Far.conv`, another name's `Near.alt`, both,
    two of other names, an ambiguity the retry must not pass) x where they are imported (the file's top, inside the
    place); a plain method beside the given, whose search drops nothing; and a namesake of a base class reached through
    a value typed as that base."""
    sink = sink or write
    for place in ['object', 'class', 'block', 'nested']:
        for rname, rivals in LATE_RIVALS.items():
            for where in (['file', 'inner'] if rivals else ['file']):
                imports = [f'import {r}.given' for r in rivals]
                top = imports if where == 'file' else []
                inner = imports if where == 'inner' else []
                if place in ('object', 'class'):
                    ind = '  '
                    body = ''.join(f'  {i}\n' for i in inner) + '  ' + LATE_GIVEN.format(ind=ind) + '\n'
                    if place == 'object':
                        b = USE_HEAD + 'import lib.*\n' + ''.join(f'{i}\n' for i in top) + 'object Q {\n' + body + '}\n' + \
                            '@main def run(): Unit = { import Q.given; println(5.shout) }\n'
                    else:
                        b = USE_HEAD + 'import lib.*\n' + ''.join(f'{i}\n' for i in top) + 'class K {\n' + body + '}\n' + \
                            '@main def run(): Unit = { val k = new K; import k.given; println(5.shout) }\n'
                else:
                    ind = '  ' if place == 'block' else '    '
                    stmts = ''.join(f'{ind}{i}\n' for i in inner) + ind + LATE_GIVEN.format(ind=ind) + f'\n{ind}println(5.shout)\n'
                    if place == 'nested':
                        stmts = '  {\n' + stmts + '  }\n'
                    b = USE_HEAD + 'import lib.*\n' + ''.join(f'{i}\n' for i in top) + '@main def run(): Unit = {\n' + stmts + '}\n'
                sink(out, f'{place}-{rname}-{where}', {'a.scala': LATE_LIB, 'b.scala': b})
    for place in ['object', 'class']:
        for rname in ['namesake', 'both']:
            imports = ''.join(f'import {r}.given\n' for r in LATE_RIVALS[rname])
            holder = 'object Q' if place == 'object' else 'class K'
            make = 'Q' if place == 'object' else 'new K'
            b = (USE_HEAD + 'import lib.*\n' + imports + f'{holder} {{\n'
                 + '  given conv: Conversion[Int, Rich] = (i: Int) => new Rich("conv" + i)\n'
                 + '  def plain: String = "in " + 1.shout\n}\n'
                 + f'@main def run(): Unit = println(({make}).plain)\n')
            sink(out, f'{place}-{rname}-plain', {'a.scala': LATE_LIB, 'b.scala': b})
    base_lib = ('package lib\nimport scala.language.implicitConversions\nclass Rich(val tag: String) { def shout: String = tag }\n'
                'trait Base { given conv: Conversion[Int, Rich] }\n'
                'class Other extends Base { given conv: Conversion[Int, Rich] = i => new Rich("other" + i) }\n'
                'object H { val b: Base = new Other }\n')
    for name, parent, in_given in [('base-namesake', ' extends Base', True), ('base-unrelated', '', True), ('base-plain', ' extends Base', False)]:
        if in_given:
            member = ('  given conv: Conversion[Int, Rich] = {\n    import lib.H.b.given\n    println("in " + 1.shout)\n'
                      '    (i: Int) => new Rich("k" + i)\n  }\n')
            run = '@main def run(): Unit = { val k = new K; import k.given; println(5.shout) }\n'
        else:
            member = ('  given conv: Conversion[Int, Rich] = (i: Int) => new Rich("k" + i)\n'
                      '  def plain: String = { import lib.H.b.given; "in " + 1.shout }\n')
            run = '@main def run(): Unit = println(new K().plain)\n'
        sink(out, name, {'a.scala': base_lib, 'b.scala': USE_HEAD + 'import lib.*\n' + f'class K{parent} {{\n' + member + '}\n' + run})



LATESCOPE = {
    'argument': 'case class Num(v: Int)\nobject Num { implicit def scope(n: Num): Rich = new Rich("scope" + n.v) }\n',
    'expected': 'case class Num(v: Int)\nobject Rich { implicit def scope(n: Num): Rich = new Rich("scope" + n.v) }\n',
    'prefix': 'object Pre {\n  case class Num(v: Int)\n  implicit def scope(n: Num): Rich = new Rich("scope" + n.v)\n}\n',
    'none': 'case class Num(v: Int)\n',
}


def latescope(out):
    """The late set's programs over a class of the library's (`Num`) instead of `Int`, crossed with THE IMPLICIT SCOPE: a
    conversion in the argument's companion (`Num`'s), in the expected type's (`Rich`'s, the uses then ascribed `:
    Rich`), in a prefix's (`Num` a member of `object Pre`), or none. searchImplicit (Implicits.scala 1786-1809) searches
    the implicit scope only once the contextual searches, the retry from the outer contexts among them, have found
    nothing, and not after an ambiguity. With them, per place, the code review's shape (a given of another type being
    defined, the object's later conversion too late and hiding an imported namesake) and its `val` twin; and an
    ambiguity among the contexts searched again with the member's expected type revealed, which reaches the implicit
    scope where the contexts give nothing."""
    progs = {}
    late(None, lambda _o, name, files: progs.__setitem__(name, files))
    for kind, decl in LATESCOPE.items():
        num = 'Pre.Num' if kind == 'prefix' else 'Num'
        def use(v):
            return f'({num}({v}): Rich).shout' if kind == 'expected' else f'{num}({v}).shout'
        for name, files in progs.items():
            a = files['a.scala'].replace('Conversion[Int, Rich]', f'Conversion[{num}, Rich]').replace(' + i)', ' + i.v)')
            a = a.replace('class Rich(val tag: String) { def shout: String = tag }\n',
                          'class Rich(val tag: String) { def shout: String = tag }\n' + decl, 1)
            b = (files['b.scala'].replace('Conversion[Int, Rich]', f'Conversion[{num}, Rich]').replace('(i: Int) =>', f'(i: {num}) =>')
                 .replace(' + i)', ' + i.v)').replace('1.shout', use(1)).replace('5.shout', use(5)))
            assert 'Int, Rich' not in a + b and '1.shout' not in b.replace(use(1), ''), name
            write(out, f'{kind}-{name}', {'a.scala': a, 'b.scala': b})
        # The review's shape: a given of another type is being defined, and the object's later conversion comes too late
        # and hid a namesake imported at the file's top; a `val` in its place, which no candidate comes too late for.
        lib = ('package lib\nimport scala.language.implicitConversions\nclass Rich(val tag: String) { def shout: String = tag }\n'
               + decl + f'object Outer {{ implicit def conv(a: {num}): Rich = new Rich("outer" + a.v) }}\n')
        for holder in ['given', 'val']:
            b = (USE_HEAD + 'import lib.*\nimport lib.Outer.*\nobject Q {\n'
                 + f'  {holder} answer: String = "in " + {use(1)}\n'
                 + f'  implicit def conv(a: {num}): Rich = new Rich("later" + a.v)\n}}\n'
                 + '@main def run(): Unit = println(Q.answer)\n')
            write(out, f'{kind}-later-{holder}', {'a.scala': lib, 'b.scala': b})
    # An ambiguity among the contexts searched again with the member's expected type revealed (`deepenProto`): the
    # search anew reaches the implicit scope where the contexts give nothing.
    deep = {'argument': ('class A\nobject A { implicit def toR3(a: A): R3 = new R3 }\n', 'A'),
            'prefix': ('object Pre {\n  class A\n  implicit def toR3(a: A): R3 = new R3\n}\n', 'Pre.A'),
            'none': ('class A\n', 'A')}
    for kind, (decl, a_ty) in deep.items():
        lib = ('package lib\nimport scala.language.implicitConversions\nclass R1 { def m: Int = 1 }\nclass R2 { def m: Int = 2 }\n'
               'class R3 { def m: String = "scope" }\n' + decl
               + f'object I1 {{ implicit def c1(a: {a_ty}): R1 = new R1 }}\nobject I2 {{ implicit def c2(a: {a_ty}): R2 = new R2 }}\n')
        b = (USE_HEAD + 'import lib.*\nimport lib.I1.*\nimport lib.I2.*\n'
             + f'@main def run(): Unit = {{\n  val s: String = new {a_ty}().m\n  println(s)\n}}\n')
        write(out, f'deepened-{kind}', {'a.scala': lib, 'b.scala': b})


ENTRY_POS = {
    'argument': ('case class Num(v: Int)\nobject Num {\n  implicit def scope(n: Num): RS = new RS("scope")\n@MORE@}\n', 'Num'),
    'prefix': ('object Pre {\n  case class Num(v: Int)\n  implicit def scope(n: Num): RS = new RS("scope")\n@MORE@}\n', 'Pre.Num'),
    'expected': ('case class Num(v: Int)\nobject RS {\n  implicit def scope(n: Num): RS = new RS("scope")\n@MORE@}\n', 'Num'),
}
# Each cell: the holder of `answer` ('val', or 'given' with a later conversion of its scope that comes too late), the
# imports at the file's top and inside the right-hand side, whether the implicit scope has its conversion (1, 2 or 0),
# and the use (a selection `.shout`, or `.m` under the expected type String, which an ambiguity deepens).
ENTRY_CELLS = {
    ('first', 'inner'): ('val', ['lib.Outer2'], ['lib.Inner'], 1, 'shout'),
    ('first', 'outer'): ('val', ['lib.Outer2'], [], 1, 'shout'),
    ('first', 'scope'): ('val', [], [], 1, 'shout'),
    ('first', 'none'): ('val', [], [], 0, 'shout'),
    ('retry', 'outer'): ('given', ['lib.Outer'], [], 1, 'shout'),
    ('retry', 'scope'): ('given', [], [], 1, 'shout'),
    ('retry', 'none'): ('given', [], [], 0, 'shout'),
    ('deep', 'inner'): ('val', [], ['lib.AmbIS'], 1, 'm'),
    ('deep', 'outer'): ('val', ['lib.Outer2'], ['lib.AmbI'], 1, 'm'),
    ('deep', 'retry'): ('given', ['lib.Outer'], ['lib.AmbI'], 1, 'm'),
    ('deep', 'scope'): ('val', [], ['lib.AmbI'], 1, 'm'),
    ('deep', 'none'): ('val', [], ['lib.AmbI'], 0, 'm'),
    ('deepamb', 'inner'): ('val', [], ['lib.AmbI', 'lib.AmbS'], 1, 'm'),
    ('deepamb', 'outer'): ('val', ['lib.AmbS'], ['lib.AmbI'], 1, 'm'),
    ('deepamb', 'scope'): ('val', [], ['lib.AmbI'], 2, 'm'),
}


def entries(out):
    """THE SEARCH'S ENTRIES (searchImplicit, Implicits.scala 1700-1712, 1786-1809, and the deepened search anew,
    1813-1830): the first search; the retry after a candidate that comes too late; the search deepened after an
    ambiguity (the member's expected type `String` revealed); a deepened search itself ambiguous. Crossed with where
    the conversion is: the first contextual level, an outer context (a farther level, or the retry's), the implicit
    scope, nowhere. Crossed with the place (an object's body, a class's, a method's block, a block inside it) and where
    the implicit scope's conversion is (the argument's companion, a prefix's, the expected type's for a selection that
    no ambiguity deepens). scalac 3.8.4 answers each program; the labels are the design's, scalac's answer the bar."""
    for (entry, where), (holder, top, rhs_imports, scope_n, member) in ENTRY_CELLS.items():
        for pos, (decl, num) in ENTRY_POS.items():
            if pos == 'expected' and member == 'm':
                continue
            more = '  implicit def scope2(n: Num): RS = new RS("scope2")\n' if scope_n == 2 else ''
            if scope_n == 0:
                d = 'case class Num(v: Int)\n' if pos != 'prefix' else 'object Pre { case class Num(v: Int) }\n'
            else:
                d = decl.replace('@MORE@', more)
            lib = ('package lib\nimport scala.language.implicitConversions\n'
                   'class RI(val tag: String) { def m: Int = tag.length; def shout: String = tag }\n'
                   'class RS(val tag: String) { def m: String = tag; def shout: String = tag }\n' + d
                   + f'object Outer {{ implicit def conv(n: {num}): RS = new RS("outer") }}\n'
                   + f'object Outer2 {{ implicit def oc(n: {num}): RS = new RS("outer") }}\n'
                   + f'object Inner {{ implicit def ic(n: {num}): RS = new RS("inner") }}\n'
                   + f'object AmbI {{\n  implicit def a1(n: {num}): RI = new RI("amb1")\n  implicit def a2(n: {num}): RI = new RI("amb2")\n}}\n'
                   + f'object AmbS {{\n  implicit def a3(n: {num}): RS = new RS("amb3")\n  implicit def a4(n: {num}): RS = new RS("amb4")\n}}\n'
                   + f'object AmbIS {{\n  implicit def a1(n: {num}): RI = new RI("amb1")\n  implicit def a2(n: {num}): RI = new RI("amb2")\n'
                   + f'  implicit def a3(n: {num}): RS = new RS("amb3")\n}}\n')
            if member == 'shout':
                use = f'({num}(1): RS).shout' if pos == 'expected' else f'{num}(1).shout'
            else:
                use = f'{{ val s: String = {num}(1).m; s }}'
            for place in ['object', 'class', 'block', 'nested']:
                ind = {'object': '  ', 'class': '  ', 'block': '  ', 'nested': '    '}[place]
                rhs = '"in " + ' + use
                if rhs_imports:
                    rhs = '{\n' + ''.join(f'{ind}  import {i}.*\n' for i in rhs_imports) + f'{ind}  "in " + {use}\n{ind}}}'
                body = f'{ind}{holder} answer: String = {rhs}\n'
                if holder == 'given':
                    body += f'{ind}implicit def conv(n: {num}): RS = new RS("later")\n'
                head = USE_HEAD + 'import lib.*\n' + ''.join(f'import {t}.*\n' for t in top)
                if place == 'object':
                    b = head + 'object Q {\n' + body + '}\n@main def run(): Unit = println(Q.answer)\n'
                elif place == 'class':
                    b = head + 'class K {\n' + body + '}\n@main def run(): Unit = println(new K().answer)\n'
                elif place == 'block':
                    b = head + '@main def run(): Unit = {\n' + body + '  println(answer)\n}\n'
                else:
                    b = head + '@main def run(): Unit = {\n  {\n' + body + '    println(answer)\n  }\n}\n'
                write(out, f'{entry}-{where}-{pos}-{place}', {'a.scala': lib, 'b.scala': b})


if __name__ == '__main__':
    out = sys.argv[1]
    for name, gen in [('axes', axes), ('holes', holes), ('frames', frames), ('fits', fits), ('access', access), ('defining', defining),
                      ('bounds', bounds), ('singletons', singletons), ('late', late),
                      ('latescope', latescope), ('entries', entries)]:
        gen(os.path.join(out, name))
