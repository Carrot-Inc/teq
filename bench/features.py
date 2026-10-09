#!/usr/bin/env python3
"""Generates one program per language feature and size of the choice it puts before the compiler,
each with the same 20,000 call sites, so that the cost of a feature per call site can be read off
the difference between two compile times.

  python3 bench/features.py <dir>      writes <dir>/<variant>/<variant>.scala
  python3 bench/features.py --names    prints the variants' names, one per line

  ovl_K    a method with K overloaded alternatives that differ in the first parameter
  lam_K    the same with a function literal as the second argument
  ext_K    K extension methods of one name on K receiver types
  depth_D  a member inherited through a chain of D traits
  extk_K   an extension method called by name with K extension methods in scope
  gextk_K  the same with generic receivers
  view_K   a member reached through one of K Scala 2 implicit conversions in scope
  gview_K  the same with generic conversions
  tm_K     a member selected through a chain of K type members (`a.Next.Next...Elem`), the
           receiver a value whose abstract member each step fixes
  tmr_K    the same, the receiver typed as a refinement (`Node { type Elem = Int }`) K deep
  inl_K    a call of an inline method that expands K levels deep
  inlm_K   an inline match over a K-element tuple type, one case per element
  mac_K    a macro expanded at every site, whose splice builds K nested quotes around the argument
  giv_K    a given search through K nested instances (`given [A: Show]: Show[L[A]]`), where at
           every level a second instance fits too and loses by specificity
  mt_K     a match type reduced K levels deep at each site (`Last` over a K-element tuple type)
           and a `Tuple.Map` over the same K elements
  mtd_K    an `IsMappedBy[List]` evidence over a 4-element tuple type summoned at each site, and
           K match types left stuck on a `Seq` element that the `List[x] *: t` case cannot be
           shown disjoint from (the common bases of the two collections are F-bounded)
  mir_K    K case classes with a type class derived through `scala.deriving.Mirror`
  gcx_K    the same given search from K contexts (one object per file): a `Vt[String, Js]`
           whose generic candidate asks for a `String => Js` among 100 conversions with a
           context bound, each of which searches its type class against 20 tuple instances
  gvar_K   a conversion with a context bound (`toOps[A: Tc](a: A)`) at every site, its type
           class searched with the receiver as the variable's lower bound against K tuple
           instances
  gder_K   K case classes deriving two type classes through an inline recursion over their
           eight fields, each derivation reading the field names through a macro, as
           Magnolia's derivations do
"""
import os
import sys

FUNCS, CALLS, PER_OBJECT = 2000, 10, 500


def classes(k):
    return "".join(f"class A{j}(val n: Int)\n" for j in range(max(k, 4)))


def callers(call, param="a: A3"):
    out = []
    for f in range(FUNCS):
        if f % PER_OBJECT == 0:
            out.append(f"object Calls{f // PER_OBJECT}:\n")
        out.append(f"  def c{f}({param}): Int =\n    var t = 0\n")
        for i in range(CALLS):
            out.append(f"    t = t + {call(f * CALLS + i)}\n")
        out.append("    t\n")
    return "".join(out)


def receivers(k):
    return [3] + [j for j in range(max(k, 4)) if j != 3][: k - 1]


def ovl(k):
    alts = "".join(f"  def go(x: A{j}, n: Int): Int = n + {j}\n" for j in receivers(k))
    return classes(k) + "object O:\n" + alts + callers(lambda i: f"O.go(a, {i % 97})")


def lam(k):
    alts = "".join(f"  def ap(x: A{j}, f: A{j} => Int): Int = f(x) + {j}\n" for j in receivers(k))
    return classes(k) + "object O:\n" + alts + callers(lambda i: f"O.ap(a, v => v.n + {i % 97})")


def ext(k):
    alts = "".join(f"extension (x: A{j}) def go(n: Int): Int = n + {j}\n" for j in receivers(k))
    return classes(k) + alts + callers(lambda i: f"a.go({i % 97})")


def depth(d):
    s = "trait T0:\n  def m: Int = 1\n"
    for j in range(1, d):
        s += f"trait T{j} extends T{j - 1}:\n  def m{j}: Int = {j}\n"
    return s + f"class C extends T{d - 1}\n" + callers(lambda i: "a.m", "a: C")


def extk(k, generic):
    s = classes(4)
    for j in range(k):
        s += f"extension [A](x: A) def m{j}: Int = {j}\n" if generic else f"extension (x: A3) def m{j}: Int = {j}\n"
    return s + callers(lambda i: "a.m0")


def view(k, generic):
    s = "import scala.language.implicitConversions\n" + classes(4)
    for j in range(k):
        if generic:
            s += f"class Ops{j}[A](x: A):\n  def m{j}: Int = {j}\nimplicit def toOps{j}[A](x: A): Ops{j}[A] = new Ops{j}(x)\n"
        else:
            s += f"class Ops{j}(x: A3):\n  def m{j}: Int = {j}\nimplicit def toOps{j}(x: A3): Ops{j} = new Ops{j}(x)\n"
    return s + callers(lambda i: "a.m0")


def tm(k, refined):
    # A chain of K nodes, each fixing its `Elem` and naming the next node's type: the call
    # site selects `n` through `a.next.next...n`, K dependent selections deep.
    s = "trait Node:\n  type Elem\n  type Next <: Node\n  def next: Next\n  def n: Elem\n"
    for j in range(k):
        nxt = f"N{j + 1}" if j + 1 < k else "Leaf"
        s += f"class N{j} extends Node:\n  type Elem = Int\n  type Next = {nxt}\n  def next: Next = new {nxt}\n  def n: Elem = {j}\n"
    s += "class Leaf extends Node:\n  type Elem = Int\n  type Next = Leaf\n  def next: Next = this\n  def n: Elem = 0\n"
    chain = "a" + ".next" * k + ".n"
    param = "a: Node { type Elem = Int }" if refined else "a: N0"
    if refined:
        chain = "a.n"
    return s + callers(lambda i: chain, param)
def inl(k):
    s = classes(4) + "object I:\n  inline def i0(x: Int): Int = x + 1\n"
    for j in range(1, k):
        s += f"  inline def i{j}(x: Int): Int = i{j - 1}(x) + 1\n"
    return s + callers(lambda i: f"I.i{k - 1}({i % 97})")


def mac(k):
    # A macro at every site: the splice runs in the interpreter and builds K nested quotes
    # around the argument, so the expansion instantiates K quote bodies per call. The macro
    # has a file of its own, since a call in the file that defines it is an error.
    m = "import scala.quoted.*\n"
    m += "object M:\n  inline def m(inline x: Int): Int = ${ impl('x) }\n"
    m += "  def impl(x: Expr[Int])(using Quotes): Expr[Int] =\n    var e = x\n"
    m += f"    var i = 0\n    while i < {k} do\n      e = '{{ $e + 1 }}\n      i += 1\n    e\n"
    return {"macro.scala": m, None: classes(4) + callers(lambda i: f"M.m({i % 97})")}


def inlm(k):
    tuple_type = "(" + ", ".join(["A3"] * k) + ")" if k > 1 else "Tuple1[A3]"
    s = "import scala.compiletime.erasedValue\n" + classes(4)
    s += "object I:\n  inline def size[T <: Tuple]: Int = inline erasedValue[T] match\n"
    s += "    case _: EmptyTuple => 0\n    case _: (_ *: ts) => 1 + size[ts]\n"
    s += f"  type K = {tuple_type}\n"
    return s + callers(lambda i: "I.size[I.K]")


def mt(k):
    # A match type reduced K levels deep at each site: `Last[T]` walks a tuple type of K
    # elements one case at a time, and `Tuple.Map` maps the same K elements.
    elems = ", ".join(f"A{j % 4}" for j in range(k))
    tuple_type = f"({elems})" if k > 1 else "Tuple1[A0]"
    s = classes(4)
    s += "object M:\n  type Last[T <: Tuple] = T match\n    case x *: EmptyTuple => x\n    case _ *: xs => Last[xs]\n"
    s += f"  type K = {tuple_type}\n  type Boxed = Tuple.Map[K, List]\n"
    s += "  def last: Last[K] = null.asInstanceOf[Last[K]]\n  def boxed: Boxed = null.asInstanceOf[Boxed]\n"
    return s + callers(lambda i: "(if M.last == null then 1 else 2) + (if M.boxed == null then 1 else 2)")


def mtd(k):
    s = classes(4)
    s += "object M:\n  type Firsts[T <: Tuple] <: Tuple = T match\n    case List[x] *: t => x *: Firsts[t]\n    case EmptyTuple => EmptyTuple\n"
    s += "  type Good = (List[A0], List[A1], List[A2], List[A3])\n"
    s += "  def need[T <: Tuple: Tuple.IsMappedBy[List]]: Int = 1\n"
    for j in range(k):
        stuck = f"Firsts[(List[A{j % 4}], Seq[{j}])]"
        s += f"  def s{j}: {stuck} = null.asInstanceOf[{stuck}]\n"
    return s + callers(lambda i: f"M.need[M.Good] + (if M.s{i % k} == null then 1 else 2)")


def mir(k):
    # K case classes each with a `Show` derived through scala.deriving.Mirror; every site shows
    # one of them.
    s = "import scala.deriving.Mirror\nimport scala.compiletime.{constValue, erasedValue, summonInline}\n" + classes(4)
    s += "trait Show[T]:\n  def show(t: T): String\nobject Show:\n"
    s += "  given Show[Int] with\n    def show(t: Int): String = t.toString\n"
    s += "  inline def elems[E <: Tuple](n: Int)(p: Product): List[String] = inline erasedValue[E] match\n"
    s += "    case _: EmptyTuple => Nil\n    case _: (e *: es) => summonInline[Show[e]].show(p.productElement(n).asInstanceOf[e]) :: elems[es](n + 1)(p)\n"
    s += "  inline def derived[T](using m: Mirror.ProductOf[T]): Show[T] = new Show[T]:\n"
    s += "    def show(t: T): String = constValue[m.MirroredLabel] + elems[m.MirroredElemTypes](0)(t.asInstanceOf[Product]).mkString(\"(\", \",\", \")\")\n"
    for j in range(k):
        s += f"case class R{j}(a: Int, b: Int, c: Int) derives Show\n"
    s += "object V:\n" + "".join(f"  val r{j} = R{j}({j}, 1, 2)\n" for j in range(k))
    return s + callers(lambda i: f"summon[Show[R{i % k}]].show(V.r{i % k}).length")


def giv(k):
    # `Show` is contravariant, so the instance for a base class fits its subclass as well: at
    # every level of the chain two instances fit the target and the more specific one wins, as
    # in a library with instances for a collection and for its parent trait.
    s = classes(4) + "trait Show[-A]:\n  def show(a: A): String\n"
    s += "given Show[Int] with\n  def show(a: Int): String = a.toString\n"
    ty, value = "Int", "1"
    for j in range(k):
        s += f"class B{j}[A](val a: A)\nclass L{j}[A](a: A) extends B{j}[A](a)\n"
        s += f"given [A](using s: Show[A]): Show[B{j}[A]] with\n  def show(b: B{j}[A]): String = \"b\" + s.show(b.a)\n"
        s += f"given [A](using s: Show[A]): Show[L{j}[A]] with\n  def show(l: L{j}[A]): String = \"l\" + s.show(l.a)\n"
        ty, value = f"L{j}[{ty}]", f"new L{j}({value})"
    s += f"object V:\n  val v: {ty} = {value}\n"
    return s + callers(lambda i: f"summon[Show[{ty}]].show(V.v).length")


def tuple_instances(tc, n, arity=8):
    params = ", ".join(f"A{j}" for j in range(arity))
    return "".join(f"  given t{i}[{params}]: {tc}[({params})] = new {tc}[({params})] {{}}\n" for i in range(n))


def gcx(k):
    # `Vt[String, Js]` has a direct instance and a generic one over a `String => Js`, as
    # scalajs-react's attribute values do; both are imported, so that the level tries both,
    # and the generic one sends the search through every conversion in scope, whose context
    # bound searches `Tc[?B >: String]` against the tuple instances. One object per file: K
    # contexts of one search each.
    lib = "trait Js\ntrait Ops extends Js\ntrait Tc[X]\nobject Tc:\n  given Tc[Int] = new Tc[Int] {}\n" + tuple_instances("Tc", 20)
    lib += "trait Vt[A, T]\nobject VtInstances:\n  given vtJs[A](using f: A => Js): Vt[A, Js] = new Vt[A, Js] {}\n  given vtString: Vt[String, Js] = new Vt[String, Js] {}\n"
    lib += "object Conv:\n" + "".join(f"  implicit def conv{i}[B](b: B)(implicit tc: Tc[B]): Ops = new Ops {{}}\n" for i in range(100))
    lib += "class Attr:\n  def :=[A](a: A)(using vt: Vt[A, Js]): Int = 1\nobject Attr:\n  val cls = new Attr\n"
    files = {"lib.scala": lib}
    per_file = FUNCS * CALLS // k
    for f in range(k):
        s = f"import Conv.*\nimport VtInstances.given\nobject Sites{f}:\n  def go(): Int =\n    var t = 0\n"
        for i in range(per_file):
            s += f'    t = t + (Attr.cls := "x{i % 7}")\n'
        files[f"sites{f}.scala"] = s + "    t\n"
    return files


def gvar(k):
    # The type class of the context bound is searched with the receiver as the variable's
    # lower bound (`Tc[?A >: Int]`), which the K tuple instances cannot meet.
    s = classes(4) + "trait Tc[X]\ntrait Ops:\n  def op: Int\nobject Tc:\n  given Tc[Int] = new Tc[Int] {}\n" + tuple_instances("Tc", k)
    s += "object Conv:\n  implicit def toOps[A](a: A)(implicit tc: Tc[A]): Ops = new Ops { def op = 1 }\n"
    return "import Conv.*\n" + s + callers(lambda i: f"({i % 97}).op")


def gder(k):
    # K case classes of eight fields deriving `Enc` and `Dec`, each derivation summoning the
    # instance of every field through an inline recursion and reading the field names through
    # a macro, as Magnolia's `paramAnns` does; every site uses a derived instance.
    m = "import scala.quoted.*\nobject Labels:\n  inline def of[T]: List[String] = ${ impl[T] }\n"
    m += "  def impl[T: Type](using Quotes): Expr[List[String]] =\n    import quotes.reflect.*\n"
    m += "    Expr(TypeRepr.of[T].typeSymbol.caseFields.map(_.name))\n"
    s = "import scala.deriving.Mirror\nimport scala.compiletime.{constValue, erasedValue, summonInline}\n" + classes(4)
    for tc in ("Enc", "Dec"):
        s += f"trait {tc}[T]:\n  def size(t: T): Int\nobject {tc}:\n"
        for ty, n in (("Int", 1), ("String", 2), ("Boolean", 3), ("Long", 4), ("Double", 5)):
            s += f"  given {tc}[{ty}] with\n    def size(t: {ty}): Int = {n}\n"
        s += f"  given [A](using a: {tc}[A]): {tc}[Option[A]] with\n    def size(t: Option[A]): Int = t.fold(0)(a.size)\n"
        s += f"  given [A](using a: {tc}[A]): {tc}[List[A]] with\n    def size(t: List[A]): Int = t.map(a.size).sum\n"
        s += f"  given [A, B](using a: {tc}[A], b: {tc}[B]): {tc}[(A, B)] with\n    def size(t: (A, B)): Int = a.size(t._1) + b.size(t._2)\n"
        s += f"  inline def elems[E <: Tuple](n: Int)(p: Product): Int = inline erasedValue[E] match\n"
        s += f"    case _: EmptyTuple => 0\n    case _: (e *: es) => summonInline[{tc}[e]].size(p.productElement(n).asInstanceOf[e]) + elems[es](n + 1)(p)\n"
        s += f"  inline def derived[T](using m: Mirror.ProductOf[T]): {tc}[T] = new {tc}[T]:\n"
        s += f"    val labels = Labels.of[T]\n"
        s += f"    def size(t: T): Int = labels.length + elems[m.MirroredElemTypes](0)(t.asInstanceOf[Product])\n"
    for j in range(k):
        s += f"case class R{j}(a: Int, b: String, c: Boolean, d: Long, e: Double, f: Option[Int], g: List[String], h: (Int, String)) derives Enc, Dec\n"
    s += "object V:\n" + "".join(f'  val r{j} = R{j}({j}, "s", true, 1L, 2.0, Some(3), List("x"), (4, "y"))\n' for j in range(k))
    return {"labels.scala": m, None: s + callers(lambda i: f"summon[{'Enc' if i % 2 == 0 else 'Dec'}[R{i % k}]].size(V.r{i % k})")}


def main():
    out = sys.argv[1]
    variants = {}
    for k in (1, 2, 4):
        variants[f"giv_{k}"] = giv(k)
    for k in (1, 8, 64):
        variants[f"gcx_{k}"] = gcx(k)
    for k in (1, 16, 64):
        variants[f"gvar_{k}"] = gvar(k)
    for k in (4, 16, 64):
        variants[f"gder_{k}"] = gder(k)
    for k in (1, 4, 16):
        variants[f"mt_{k}"] = mt(k)
        variants[f"mir_{k}"] = mir(k)
        variants[f"mtd_{k}"] = mtd(k)
    for k in (1, 4, 16):
        variants[f"inl_{k}"] = inl(k)
        variants[f"inlm_{k}"] = inlm(k)
        variants[f"mac_{k}"] = mac(k)
    for k in (1, 4, 16):
        variants[f"ovl_{k}"] = ovl(k)
        variants[f"lam_{k}"] = lam(k)
        variants[f"ext_{k}"] = ext(k)
    for d in (1, 8, 32):
        variants[f"depth_{d}"] = depth(d)
    for k in (1, 50, 200):
        variants[f"extk_{k}"] = extk(k, False)
        variants[f"gextk_{k}"] = extk(k, True)
        variants[f"view_{k}"] = view(k, False)
        variants[f"gview_{k}"] = view(k, True)
    for k in (1, 4, 16):
        variants[f"tm_{k}"] = tm(k, False)
    variants["tmr_1"] = tm(1, True)
    if out == "--names":
        print("\n".join(variants))
        return
    for name, src in variants.items():
        os.makedirs(os.path.join(out, name), exist_ok=True)
        files = src if isinstance(src, dict) else {None: src}
        for file, text in files.items():
            with open(os.path.join(out, name, file or name + ".scala"), "w") as f:
                f.write(text)
    print(f"{len(variants)} variants with {FUNCS * CALLS} call sites each in {out}")


main()
