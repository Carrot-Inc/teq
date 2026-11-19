// teq: --wconf id=E220:s
// The alternatives of an overloaded method narrowed by a written `using` list before any omitted parameter is searched
// (the reference's staged resolution: `resolveMapped` against the next list, `sizeFits` counting a default and no
// given): the alternative whose clause the list fits, then its omitted parameters searched; an earlier list that left
// one alternative has chosen it.
class A(val s: String = "a")
class B(val s: String = "b")
class C(val s: String = "c")
given B = new B("givenB")

object FirstListsDiffer:
  def f(x: Int)(using a: A, b: B): String = s"int ${a.s} ${b.s}"
  def f(x: String)(using a: A): String = s"str ${a.s}"
  def run(): Unit =
    println(f(1)(using A("x")))
    println(f("s")(using A("y")))

object ShortAndLong:
  def g(x: Int)(using a: A): String = s"one ${a.s}"
  def g(x: Int)(using a: A, b: B): String = s"two ${a.s} ${b.s}"
  def run(): Unit =
    println(g(1)(using A("x")))
    println(g(1)(using A("x"), B("y")))

object PlainBesideUsing:
  def k(x: Int): String = "plain"
  def k(x: Int, y: Int = 0)(using a: A, b: B): String = s"using ${a.s} ${b.s}"
  def run(): Unit =
    println(k(1))
    println(k(1, 2)(using A("x")))

object MoreSpecificHasItsGiven:
  def m(x: Any)(using a: A, c: C): String = s"any ${a.s} ${c.s}"
  def m(x: Int)(using a: A, b: B): String = s"int ${a.s} ${b.s}"
  def run(): Unit =
    println(m(1)(using A("x")))
    println(m("s")(using A("y"), C("z")))

object MembersConstructorsExtensions:
  given C = new C("givenC")
  class K(val x: Int)(using val a: A, val b: B):
    def this(s: String)(using a: A, b: B, c: C) = this(s.length)(using a, b)
    def at(i: Int)(using a: A, b: B): String = s"at-int ${a.s} ${b.s}"
    def at(s: String)(using a: A): String = s"at-str ${a.s}"
  object Obj:
    def apply(x: Int)(using a: A, b: B): String = s"apply-int ${a.s} ${b.s}"
    def apply(x: String)(using b: B, c: C): String = s"apply-str ${b.s} ${c.s}"
  extension (x: Int)
    def ext(y: Int)(using a: A, b: B): String = s"ext-int ${a.s} ${b.s}"
    def ext(y: String)(using a: A, b: B): String = s"ext-str ${a.s} ${b.s}"
  def run(): Unit =
    val k = new K(1)(using A("x"))
    println(k.b.s)
    println(k.at(1)(using A("y")))
    println(k.at("s")(using A("z")))
    println(Obj(1)(using A("p")))
    println(Obj("s")(using B("q")))
    println(1.ext(2)(using A("r")))
    println(1.ext("t")(using A("u")))

object ImplicitClauseOverloads:
  def p(x: Int)(implicit a: A, b: B): String = s"int ${a.s} ${b.s}"
  def p(x: String)(implicit a: A): String = s"str ${a.s}"
  def run(): Unit =
    println(p(1)(using A("x")))
    println(p("s")(using A("y")))

object LeadingUsingList:
  def u(using a: A): String = s"one ${a.s}"
  def u(using a: A, b: B): String = s"two ${a.s} ${b.s}"
  def short(using a: A): String = "short"
  def short(using a: A, b: B = B()): String = "default"
  def run(): Unit =
    println(u(using A("x")))
    println(u(using A("x"), B("y")))
    println(short(using A()))

object DefaultCoversTheSize:
  def d(x: Int)(using a: A, b: B = B("dflt")): String = s"default ${a.s} ${b.s}"
  def d(x: Int)(using a: A, b: B, c: C): String = s"three ${a.s} ${b.s} ${c.s}"
  def first(x: Int)(using a: A = A()): String = "short-default"
  def first(x: Int)(using a: A, b: B): String = "long"
  def run(): Unit =
    println(d(1)(using A("x")))
    println(first(1)(using A()))

object GenericAlternatives:
  trait Show[T]:
    def show(t: T): String
  given Show[Int] = (t: Int) => s"int:$t"
  given Show[String] = (t: String) => s"str:$t"
  def s[T](x: T)(using a: A, sh: Show[T]): String = s"generic ${a.s} ${sh.show(x)}"
  def s(x: String)(using a: A, b: B): String = s"string ${a.s} ${b.s}"
  def t[T](x: T)(using a: A, sh: Show[T]): String = s"generic ${a.s} ${sh.show(x)}"
  def t[T](x: List[T])(using a: A, sh: Show[T]): String = s"list ${a.s} ${x.map(sh.show)}"
  def run(): Unit =
    println(s(1)(using A("x")))
    println(s("q")(using A("y")))
    println(t(1)(using A("z")))
    println(t(List(1))(using A("w")))

object UsingListOverloadsNarrowed:
  def main(args: Array[String]): Unit =
    FirstListsDiffer.run()
    ShortAndLong.run()
    PlainBesideUsing.run()
    MoreSpecificHasItsGiven.run()
    MembersConstructorsExtensions.run()
    ImplicitClauseOverloads.run()
    LeadingUsingList.run()
    DefaultCoversTheSize.run()
    GenericAlternatives.run()
