#!/usr/bin/env python3
"""tests/right-assoc/gen.py <out>: the cells of right-associative calls' order of evaluation, one program a cell
(<out>/cells.tsv: name and family; <out>/cells/<cell>/Test.scala, `@main def run`; <out>/twofile/<cell>/ for the
cells of a library compiled before its caller). Every cell prints its result and the log of every operand's, prefix's
and initialiser's effect in order. scalac 3.8.4's lines are the expectation (tests/right-assoc/capture.sh).
Families:
- T1, the kinds of operator: a class's member (mem), a top-level extension (ext), an object's with an initialiser,
  imported (extobj), a value class instance's (vcinst), a class instance's (inst), a capturing local trait's (ltrait),
  a local def (ldef); infix against written out; the left and right operands a local val, an impure call, a val of an
  object whose initialiser logs nothing, or of one that logs; the right parameter's default given and left out, and
  a using clause;
- G, on an extension in scope, an object's and an instance's (and the member where the axis exists), infix and
  written out:
    op       the operator's name: `::`, `x_:`;
    lazy     a left or right operand that is a local lazy val, or an object's lazy val, its right side logging;
    java     a Java static as the left operand of an extension on its type: a field and a call;
    byname   the right parameter by-name, the receiver by-name;
    inline   an inline extension with an inline receiver, and a call through an inline method's inline parameter;
    using    a using clause before the receiver, after it, both;
    two      a second clause of the method;
    rdflt    the receiver's default, written `+:(r)()`;
    noimp    written out with no import in scope;
    prefix   an impure prefix written out;
    sub      a capturing local trait reached through a named local subclass, and through super;
- R, the order's harder cases: a selection that throws written first, a null receiver whose left operand throws, an
  effectful given for a leading using clause, an eta-expansion and a selection made a function, an export under
  another name, an interleaved type clause, the later clauses' defaults with a named omission, for-comprehension
  generators, guards and yields, `+=:` and a compound assignment whose right side is such a call, a mutable receiver
  changed by the left operand, `this` as the receiver, a chain `a +: b +: c`, an operator found through a type-class
  given whose initialisation logs;
- S, the scopes of the type and term clauses once the clauses are compiled: a bound or a parameter's type reading the
  receiver, a using clause's parameter, the extension's type parameter or the method's own, each in a right-associative
  method and in a plain one of the same clauses; each cell defines and prints `ok`;
- F, lifted (tests/right-assoc/lifted, one program a cell, each printing a line a case): a local trait's member and a
  local extension def, capturing or not, in one, two and a using clause, infix and written out;
- V, value-class (tests/right-assoc/value-class and gen_erase.py, one program a cell): an instance's, an object's and
  a top-level extension written out with an object path with an initialiser, and the cells whose two operands erase
  alike;
- TWO-FILE: an object compiled before its caller, its initialiser logging, infix and written out; a quote pattern of a
  right-associative call in a macro; a Java static whose class initialisation prints.
A self-test runs first: every cell prints its log, no cell twice; a planted cell without the print and one twice are
each refused."""
import os, shutil, sys

PRE = '''object L { val b = new StringBuilder; def log(s: String): Unit = { if b.nonEmpty then b ++= ","; b ++= s } }
final case class Num(i: Int)
def num(i: Int): Num = { L.log("n" + i); Num(i) }
def note(s: String): String = { L.log(s); s }
def d(): String = { L.log("d"); "d" }
object S { val n = Num(5); val s = "s5" }
object P { L.log("p"); val n = Num(6) }
object P2 { L.log("p2"); val s = "s7" }
'''
PL = 'object PL { lazy val n = num(8); lazy val s = note("ls") }\n'
LEFT = {'val': 'n0', 'impure': 'num(1)', 'path': 'S.n', 'initpath': 'P.n'}
RIGHT = {'lit': '"a"', 'impure': 'note("b")', 'val': 'v0', 'initpath': 'P2.s'}
MEM_RIGHT = {'lit': 'r0', 'impure': 'rr()', 'val': 'S2.r', 'initpath': 'P3.r'}
# the grown operand kinds: (expression, setup lines in run's body)
GLEFT = {'lazy': ('lz', ['lazy val lz = num(9)']), 'objlazy': ('PL.n', [])}
GRIGHT = {'lazy': ('lzs', ['lazy val lzs = note("lz")']), 'objlazy': ('PL.s', [])}

rows = []
OUT = None


def put(name, family, text, files=None):
    rows.append((name, family, text, files))


def mem_program(form, left, right, op='+:', prefix=False):
    pre = PRE + (PL if left == 'objlazy' else '') + (f'class R(t: String) {{ def {op}(x: Num): String = t + x.i }}\n'
                 'def rr(): R = { L.log("r"); R("rr") }\n'
                 'object S2 { val r = R("rs") }\n'
                 'object P3 { L.log("p3"); val r = R("rp") }\n')
    body = ['  val n0 = Num(0)', '  val v0 = "v0"', '  val r0 = R("r0")']
    l, lsetup = (LEFT[left], []) if left in LEFT else GLEFT[left]
    body += ['  ' + s for s in lsetup]
    r = 'rr()' if prefix else MEM_RIGHT[right]
    call = f'{l} {op} {r}' if form == 'infix' else f'{r}.{op}({l})'
    body += [f'  val res = {call}', '  println(res.toString + " " + L.b)']
    return pre + '@main def run(): Unit =\n' + '\n'.join(body) + '\n'


def ext_program(kind, form, left, right, variant='', op='+:', noimp=False, prefix=False):
    """T1's ext/extobj/vcinst/inst/ltrait/ldef programs, and the grown variants on them."""
    pre = PRE + (PL if 'objlazy' in (left, right) else '')
    body = ['  val n0 = Num(0)', '  val v0 = "v0"']
    l, lsetup = (LEFT[left], []) if left in LEFT else GLEFT[left]
    r, rsetup = (RIGHT[right], []) if right in RIGHT else GRIGHT[right]
    body += ['  ' + s for s in lsetup + rsetup]
    x = 'x: => Num' if variant == 'bynamerecv' else ('x: Num = num(5)' if variant == 'rdflt' else 'x: Num')
    y = {'dflt': 'y: String = d()', 'bynameright': 'y: => String'}.get(variant, 'y: String')
    before = '(using u: U)' if variant in ('usingbefore', 'usingboth') else ''
    after = '(using u2: U)' if variant in ('using', 'usingboth') else ''
    if variant == 'using':
        after = '(using u: U)'
    two = '(z: Num)' if variant == 'two' else ''
    res = 'y + x.i' + (' + u.t' if variant in ('using', 'usingbefore', 'usingboth') else '') + \
          (' + u2.t' if variant == 'usingboth' else '') + (' + z.i' if variant == 'two' else '')
    if variant in ('using', 'usingbefore', 'usingboth'):
        pre += 'final case class U(t: String)\ngiven U = U("u")\n'
    defn = f'extension {before}({x}){after} def {op}({y}){two}: String = {res}'
    if variant == 'inline':
        defn = f'extension (inline x: Num) inline def {op}({y}): String = {res}'
    if kind == 'ext':
        pre += defn + '\n'
        path = ''
    elif kind == 'extobj':
        pre += f'object Q {{ L.log("q"); {defn} }}\n'
        if not noimp:
            body.insert(0, '  import Q.*')
        path = 'Q.'
    elif kind == 'vcinst':
        pre += (f'class VC(val t: Int) extends AnyVal {{ {defn} + t }}\n'
                'object QV { L.log("q"); val v = VC(7) }\n')
        body.insert(0, '  import QV.v.*')
        path = 'QV.v.'
    elif kind == 'inst':
        pre += f'class I(t: Int) {{ {defn} + t }}\n' + ('def mkI(): I = { L.log("mk"); I(8) }\n' if prefix else '')
        body.insert(0, '  val inst = I(8)')
        if not noimp:
            body.insert(1, '  import inst.*')
        path = 'mkI().' if prefix else 'inst.'
    elif kind == 'ltrait':
        body.insert(0, '  var k = 10')
        body.insert(1, f'  trait T {{ {defn} + k }}')
        body.insert(2, '  val o = new T {}')
        body.insert(3, '  import o.*')
        path = 'o.'
    elif kind == 'ldef':
        body.insert(0, '  var k = 10')
        body.insert(1, f'  {defn} + k')
        path = ''
    zarg = '(num(2))' if variant == 'two' else ''
    if form == 'infix':
        call = f'({l} {op} {r}){zarg}' if zarg else f'{l} {op} {r}'
    elif variant == 'dflt':
        call = f'{path}{op}()({l})'
    elif variant == 'rdflt':
        call = f'{path}{op}({r})()'
    else:
        call = f'{path}{op}({r})({l}){zarg}'
    body.append(f'  val res = {call}')
    body.append('  println(res.toString + " " + L.b)')
    return pre + '@main def run(): Unit =\n' + '\n'.join(body) + '\n'


def java_program(form, left):
    l = {'field': 'java.util.Locale.ROOT', 'call': 'java.util.Locale.getDefault()'}[left]
    call = f'{l} +: note("b")' if form == 'infix' else f'+:(note("b"))({l})'
    return PRE + ('extension (x: java.util.Locale) def +:(y: String): String = y + (x eq null)\n'
                  f'@main def run(): Unit =\n  val res = {call}\n  println(res.toString + " " + L.b)\n')


def inline_via_program(op='+:'):
    return PRE + (f'extension (x: Num) def {op}(y: String): String = y + x.i\n'
                  f'inline def via(inline a: Num): String = a {op} note("b")\n'
                  '@main def run(): Unit =\n  val res = via(num(1)) + via(P.n)\n  println(res + " " + L.b)\n')


def sub_program(written_super):
    if written_super:
        call = ['  class C extends T { def viaSuper(n: Num): String = super.+:(note("s"))(n) }',
                '  val c = new C', '  val res = c.viaSuper(num(1))']
    else:
        call = ['  class C extends T', '  val c = new C', '  val res = c.+:(note("b"))(num(1))']
    return PRE + ('@main def run(): Unit =\n  var k = 10\n'
                  '  trait T { extension (x: Num) def +:(y: String): String = y + x.i + k }\n'
                  + '\n'.join(call) + '\n  println(res.toString + " " + L.b)\n')



def s_programs():
    """S, the scopes of the type and term clauses once a right-associative extension's clauses are compiled: a bound
    or a parameter's type reading the receiver, a using clause's parameter, the extension's type parameter or the
    method's own, each in a right-associative method (`%:`) and in a plain one of the same clauses (`plus`), which
    the swap leaves as written. Each cell defines and prints `ok`: whether the definition is accepted is the cell."""
    box = 'trait Box { type T }\n'
    forms = {
        # form: (the extension's clauses, the method's clauses after its name)
        'upper-recv': ('(x: Box)', '[A <: x.T](y: A)'),
        'lower-recv': ('(x: Box)', '[A >: x.T](y: A)'),
        'value-recv': ('(x: Box)', '(y: x.T)'),
        'ext-tparam': ('[E](x: List[E])', '[A <: E](y: A)'),
        'ext-tparam-recv': ('[E <: Box](x: E)', '[A <: x.T](y: A)'),
        'fbound': ('(x: Box)', '[A <: Ordered[A]](y: A)'),
        'own-param': ('(x: Box)', '[A <: y.T](y: Box)'),
        'trailing-using-bound': ('(x: Box)(using u: Box)', '[A <: u.T](y: A)'),
        'trailing-using-value': ('(x: Box)(using u: Box)', '(y: u.T)'),
        'leading-using-bound': ('(using u: Box)(x: Box)', '[A <: u.T](y: A)'),
        'trailing-using-reads-own': ('(x: Box)(using u: List[A])', '[A](y: A)'),
        'recv-reads-own': ('(x: List[A])', '[A](y: A)'),
    }
    out = []
    for form, (ext, own) in forms.items():
        for kind, name in (('ra', '%:'), ('plain', 'plus')):
            text = (PRE + box + f'object Lib {{\n  extension {ext}\n    def {name}{own}: String = "d"\n}}\n'
                    '@main def run(): Unit = println("ok " + L.b)\n')
            out.append((f's-{form}-{kind}', form, text))
    return out


def r_programs():
    """The R family: (name, program) pairs."""
    out = []
    pre = PRE
    out.append(('r-throw-pure-written', pre + """class Box(val value: String)
class E { extension (x: Num) def +:(y: String): String = y + x.i }
@main def run(): Unit =
  val e = E()
  val box: Box = null
  val res = try e.+:(box.value)(num(1)) catch case _: NullPointerException => "npe"
  println(res + " " + L.b)
"""))
    out.append(('r-null-receiver-member', pre + """class R { def +:(x: Num): String = "r" + x.i }
def boom(): Num = { L.log("boom"); throw new IllegalArgumentException("boom") }
@main def run(): Unit =
  val r: R = null
  val res = try boom() +: r catch case e: IllegalArgumentException => "iae"
  println(res + " " + L.b)
"""))
    for form in ('infix', 'written'):
        call = 'num(1) +: note("b")' if form == 'infix' else '+:(using mk())(note("b"))(num(1))'
        out.append((f'r-leading-using-given-{form}', pre + f"""final case class U(t: String)
def mk(): U = {{ L.log("u"); U("u") }}
given U = mk()
extension (using u: U)(x: Num) def +:(y: String): String = y + x.i + u.t
@main def run(): Unit =
  val res = {call}
  println(res + " " + L.b)
"""))
    out.append(('r-eta-partial', pre + """class E { extension (x: Num) def +:(y: String): String = y + x.i }
def e(): E = { L.log("e"); E() }
@main def run(): Unit =
  val f: Num => String = e().+:(note("b")) _
  L.log("made")
  val res = f(num(1))
  println(res + " " + L.b)
"""))
    out.append(('r-selection-function', pre + """class E { extension (x: Num) def +:(y: String): String = y + x.i }
@main def run(): Unit =
  val q = E()
  import q.*
  val g: Num => String = note("b").+:
  L.log("made")
  val res = g(num(1))
  println(res + " " + L.b)
"""))
    out.append(('r-export-renamed', pre + """object Q { L.log("q"); extension (x: Num) def +:(y: String): String = y + x.i }
object F { export Q.{+: as join} }
@main def run(): Unit =
  val res = F.join(note("b"))(num(1))
  println(res + " " + L.b)
"""))
    out.append(('r-interleaved-types', pre + """object Q { extension [A](using u: Boolean)(x: A) def +:[B](y: B): String = y.toString + x.toString }
def b(): Boolean = { L.log("u"); true }
@main def run(): Unit =
  val res = Q.+:[Num](using b())[String](note("b"))(num(1))
  println(res + " " + L.b)
"""))
    out.append(('r-later-defaults-named', pre + """class E { extension (x: Num) def +:(y: String)(a: Num = num(7), b: Num = num(8)): String = y + x.i + a.i + b.i }
@main def run(): Unit =
  val res = E().+:(note("b"))(num(1))(b = num(2))
  println(res + " " + L.b)
"""))
    out.append(('r-for-comprehension', pre + """extension (x: Num) def +:(y: String): String = y + x.i
@main def run(): Unit =
  val res = for i <- List(1, 2) if (num(i) +: note("g")).nonEmpty yield num(i + 10) +: note("y" + i)
  println(res.mkString("/") + " " + L.b)
"""))
    out.append(('r-plus-eq-colon', pre + """class Buf { var s = ""; def +=:(x: Num): Buf = { s = s + x.i; this } }
def buf(): Buf = { L.log("buf"); Buf() }
@main def run(): Unit =
  val res = (num(1) +=: buf()).s
  println(res + " " + L.b)
"""))
    out.append(('r-compound-assign-rhs', pre + """extension (x: Num) def +:(y: String): String = y + x.i
@main def run(): Unit =
  var s = "s"
  s += num(1) +: note("b")
  println(s + " " + L.b)
"""))
    out.append(('r-mutable-receiver', pre + """class R(t: String) { def +:(x: Num): String = t + x.i }
@main def run(): Unit =
  var r = R("a")
  val res = { r = R("b"); num(1) } +: r
  println(res + " " + L.b)
"""))
    out.append(('r-this-receiver', pre + """class R(t: String) {
  def +:(x: Num): String = t + x.i
  def go(): String = num(1) +: this
}
@main def run(): Unit =
  val res = R("t").go()
  println(res + " " + L.b)
"""))
    out.append(('r-chain', pre + """class R(t: String) { def +:(x: Num): R = R(t + x.i) ; override def toString = t }
def rr(): R = { L.log("r"); R("r") }
@main def run(): Unit =
  val res = num(1) +: num(2) +: rr()
  println(res.toString + " " + L.b)
"""))
    out.append(('r-typeclass-given', pre + """trait Op[A] { extension (x: A) def +:(y: String): String }
given Op[Num] = { L.log("g"); new Op[Num] { extension (x: Num) def +:(y: String): String = y + x.i } }
@main def run(): Unit =
  val res = num(1) +: note("b")
  println(res + " " + L.b)
"""))
    return out

def generate():
    # T1: Step 0's 248 cells, the same programs.
    for kind in ('mem', 'ext', 'extobj', 'vcinst', 'inst', 'ltrait', 'ldef'):
        for form in ('infix', 'written'):
            for left in LEFT:
                for right in RIGHT:
                    text = mem_program(form, left, right) if kind == 'mem' else ext_program(kind, form, left, right)
                    put(f'{kind}-{form}-{left}-{right}', f'T1:{kind}-{form}', text)
    for kind in ('ext', 'inst'):
        for form in ('infix', 'written'):
            for left in LEFT:
                put(f'{kind}-dflt-{form}-{left}', f'T1:{kind}-dflt-{form}', ext_program(kind, form, left, 'impure', 'dflt'))
    for form in ('infix', 'written'):
        for left in LEFT:
            put(f'ext-using-{form}-{left}', f'T1:ext-using-{form}', ext_program('ext', form, left, 'impure', 'using'))
    # G: the grown axes.
    for kind in ('ext', 'extobj', 'inst'):
        for form in ('infix', 'written'):
            for op, tag in (('::', 'cons'), ('x_:', 'xop')):
                for left in ('impure', 'initpath'):
                    for right in ('impure', 'initpath'):
                        put(f'g-op{tag}-{kind}-{form}-{left}-{right}', f'G:op-{kind}', ext_program(kind, form, left, right, op=op))
            for left in GLEFT:
                for right in ('impure', 'initpath'):
                    put(f'g-left{left}-{kind}-{form}-{right}', f'G:lazy-java-left-{kind}', ext_program(kind, form, left, right))
            for right in GRIGHT:
                for left in ('impure', 'initpath'):
                    put(f'g-right{right}-{kind}-{form}-{left}', f'G:lazy-right-{kind}', ext_program(kind, form, left, right))
            for variant in ('bynameright', 'bynamerecv', 'inline', 'using', 'usingbefore', 'usingboth', 'two'):
                for left in ('impure', 'initpath'):
                    for right in ('impure', 'initpath'):
                        put(f'g-{variant}-{kind}-{form}-{left}-{right}', f'G:{variant}-{kind}', ext_program(kind, form, left, right, variant))
        for right in ('impure', 'initpath', 'lit'):
            put(f'g-rdflt-{kind}-written-{right}', f'G:rdflt-{kind}', ext_program(kind, 'written', 'val', right, 'rdflt'))
    for kind in ('extobj', 'inst'):
        for left in ('impure', 'initpath'):
            for right in ('impure', 'initpath'):
                put(f'g-noimp-{kind}-written-{left}-{right}', f'G:noimp-{kind}', ext_program(kind, 'written', left, right, noimp=True))
    for left in ('impure', 'initpath'):
        for right in ('impure', 'initpath'):
            put(f'g-prefix-inst-written-{left}-{right}', 'G:prefix-inst', ext_program('inst', 'written', left, right, noimp=True, prefix=True))
    for form in ('infix', 'written'):
        for left in ('impure', 'initpath'):
            put(f'g-prefix-mem-{form}-{left}', 'G:prefix-mem', mem_program(form, left, 'impure', prefix=True))
            for op, tag in (('::', 'cons'), ('x_:', 'xop')):
                put(f'g-op{tag}-mem-{form}-{left}', 'G:op-mem', mem_program(form, left, 'impure', op=op))
    put('g-inline-via', 'G:inline-via', inline_via_program())
    for form in ('infix', 'written'):
        for left in ('field', 'call'):
            put(f'g-java-{form}-{left}', 'G:java', java_program(form, left))
    put('g-sub-written', 'G:sub', sub_program(False))
    put('g-sub-super', 'G:sub', sub_program(True))
    for name, text in r_programs():
        put(name, 'R:' + name.split('-', 1)[1], text)
    for name, form, text in s_programs():
        put(name, 'S:' + form, text)
    # M: an extension defined in one file and called from eight, for the workers' merge of one module's files: a
    # file a worker at 8 workers, the definitions' file another's.
    many = {'Defs.scala': PRE + 'object Q { L.log("q"); extension (x: Num) def +:(y: String): String = y + x.i }\n'}
    for k in range(1, 9):
        many[f'C{k}.scala'] = (f'import Q.*\nobject C{k} {{\n  def infix(): String = num({k}) +: note("i{k}")\n'
                               f'  def written(): String = Q.+:(note("w{k}"))(num({10 + k}))\n}}\n')
    many['Test.scala'] = ('@main def run(): Unit =\n'
                          + ''.join(f'  val r{k} = C{k}.infix() + C{k}.written()\n' for k in range(1, 9))
                          + '  println(Seq(' + ', '.join(f'r{k}' for k in range(1, 9)) + ').mkString("/") + " " + L.b)\n')
    put('m-eight-callers', 'M:eight-callers', None, many)
    # A Java static whose class initialisation prints, beside the cell's own prints.
    put('w2-java-static-init', 'TWO:java', None, {
        'J.java': 'public class J { static { System.out.println("J.init"); } public static int v = 6; }\n',
        'Test.scala': ('final case class Num(i: Int)\n'
                       'def num(i: Int): Num = { println("n" + i); Num(i) }\n'
                       'extension (x: Num) def +:(y: Int): String = y.toString + x.i\n'
                       '@main def run(): Unit = println(num(1) +: J.v)\n')})
    # TWO-FILE forms (the runner's two-step form, owed).
    lib_obj = PRE.replace('object P { L.log("p"); val n = Num(6) }\n', '') + \
        'object CP { L.log("cp"); val n = Num(6); extension (x: Num) def +:(y: String): String = y + x.i }\n'
    for form, call in (('infix', 'CP.n +: note("b")'), ('written', 'CP.+:(note("b"))(CP.n)')):
        put(f'w2-cpobj-{form}', 'TWO:cpobj', None,
            {'Lib.scala': lib_obj, 'Test.scala': f'import CP.*\n@main def run(): Unit =\n  val res = {call}\n  println(res + " " + L.b)\n'})
    put('w2-quote-pattern', 'TWO:quote', None, {
        'Lib.scala': ('import scala.quoted.*\nextension (x: Int) def +:(y: String): String = y + x\n'
                      'inline def shape(inline e: String): String = ${ shapeOf(\'e) }\n'
                      'def shapeOf(e: Expr[String])(using Quotes): Expr[String] = e match\n'
                      '  case \'{ ($a: Int) +: ($b: String) } => \'{ "rassoc " + $b + $a }\n'
                      '  case _ => \'{ "other" }\n'),
        'Test.scala': ('object L { val b = new StringBuilder; def log(s: String): Unit = { if b.nonEmpty then b ++= ","; b ++= s } }\n'
                       'def n1(): Int = { L.log("n1"); 1 }\n'
                       '@main def run(): Unit = println(shape(n1() +: "b") + " " + L.b)\n')})
    # V: the value-class programs and the cells whose operands erase alike, one cell a program, judged whole, each
    # run through its own `Main.main`.
    here = os.path.dirname(os.path.abspath(__file__))
    vc = os.path.join(here, 'value-class')
    for f in sorted(os.listdir(vc)):
        if f.endswith('.scala'):
            put('v-' + f[:-len('.scala')], 'V:' + f.split('_')[1], open(os.path.join(vc, f)).read() + '\n@main def run(): Unit = Main.main(Array())\n')
    if True:
        import subprocess, tempfile
        with tempfile.TemporaryDirectory() as ed:
            subprocess.run([sys.executable, '-I', os.path.join(here, 'gen_erase.py'), ed], check=True, capture_output=True)
            for f in sorted(os.listdir(ed)):
                if f.endswith('.scala'):
                    put('e-' + f[:-len('.scala')], 'V:erase-' + f.split('_')[1], open(os.path.join(ed, f)).read() + '\n@main def run(): Unit = Main.main(Array())\n')
    # F: the lifted programs, one cell each.
    nat = os.path.join(here, 'lifted')
    if True:
        for f in sorted(os.listdir(nat)):
            if f.endswith('.scala'):
                # run_fd4 runs `run`: the program's own `Main.main` through it, the program otherwise as it is.
                put('f-' + f[:-len('.scala')], 'F:' + f.split('_')[0], open(os.path.join(nat, f)).read() + '\n@main def run(): Unit = Main.main(Array())\n')


def self_test():
    """The table's own check, with planted faults first: a cell without the log's print, and a name twice."""
    def check(cells):
        seen, bad = set(), []
        for name, family, text, files in cells:
            if name in seen:
                bad.append(f'{name}: twice')
            seen.add(name)
            src = text if text is not None else ''.join((files or {}).values())
            if family[0] not in 'FV' and family != 'TWO:java' and 'L.b' not in src:
                bad.append(f'{name}: prints no log')
            if text is not None and family[0] not in 'FV' and '@main def run' not in text:
                bad.append(f'{name}: no run')
        return bad
    planted = [('p1', 'G:x', '@main def run(): Unit = println(1)\n', None), ('p2', 'G:x', '@main def run(): Unit = println(L.b)\n', None),
               ('p2', 'G:x', '@main def run(): Unit = println(L.b)\n', None)]
    got = check(planted)
    assert got == ['p1: prints no log', 'p2: twice'], got
    return check(rows)


if __name__ == '__main__':
    OUT = sys.argv[1]
    generate()
    bad = self_test()
    print('self-test: 2 of 2 plants refused')
    if bad:
        print('\n'.join(bad))
        sys.exit(1)
    shutil.rmtree(OUT, ignore_errors=True)
    os.makedirs(os.path.join(OUT, 'cells'))
    os.makedirs(os.path.join(OUT, 'twofile'))
    single = 0
    for name, family, text, files in rows:
        if files is None or family.startswith('M:'):
            d = os.path.join(OUT, 'cells', name)
            os.makedirs(d)
            for fn, src in (files or {'Test.scala': text}).items():
                open(os.path.join(d, fn), 'w').write(src)
            single += 1
        else:
            d = os.path.join(OUT, 'twofile', name)
            os.makedirs(d)
            for fn, src in files.items():
                open(os.path.join(d, fn), 'w').write(src)
    open(os.path.join(OUT, 'cells.tsv'), 'w').write(''.join(f'{n}\t{f}\n' for n, f, t, fl in rows if fl is None or f.startswith('M:')))
    fams = {}
    for n, f, t, fl in rows:
        fams[f.split(':')[0]] = fams.get(f.split(':')[0], 0) + 1
    print(single, 'single-file cells;', len(rows) - single, 'two-file;', ', '.join(f'{k} {v}' for k, v in fams.items()))
