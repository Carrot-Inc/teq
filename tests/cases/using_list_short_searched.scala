// A written `using` list shorter than its clause: a parameter it leaves without an argument and without a default is
// searched as a given at the application, after the arguments before it (the reference's `matchArgs`), in a using
// clause or an implicit one applied with `using`, a constructor's, a parent call's, an enum case's and an extension's;
// the written arguments are evaluated in written order, then the searched ones.
object Cut:
  case class Config(html: Boolean)
  class Translate
  given Translate = Translate()
  def getJson(page: Int)(using c: Config, t: Translate): String = s"$page ${c.html}"
  def run(): Unit = println(getJson(1)(using Config(html = false)))

object NamedOneGivenRest:
  case class Config(html: Boolean)
  class Translate(val lang: String)
  given Config = Config(true)
  def getJson(page: Int)(using c: Config, t: Translate): String = s"$page ${c.html} ${t.lang}"
  def run(): Unit = println(getJson(1)(using t = Translate("fr")))

object ImplicitClauseWithUsing:
  case class Config(html: Boolean)
  class Translate(val lang: String)
  implicit val tr: Translate = Translate("implicit")
  def getJson(page: Int)(implicit c: Config, t: Translate): String = s"$page ${c.html} ${t.lang}"
  def run(): Unit = println(getJson(1)(using Config(false)))

object ConstructorAndExtension:
  case class Config(html: Boolean)
  class Translate(val lang: String)
  given Translate = Translate("given")
  class K(val x: Int)(using val c: Config, val t: Translate)
  extension (n: Int) def json(using c: Config, t: Translate): String = s"$n ${c.html} ${t.lang}"
  object Mk:
    def apply(x: Int)(using c: Config, t: Translate): String = s"$x ${t.lang}"
  def run(): Unit =
    val k = new K(1)(using Config(false))
    println((k.c.html, k.t.lang))
    println(2.json(using Config(true)))
    println(Mk(3)(using Config(true)))

object ThreeFirstTwoGiven:
  class A(val s: String); class B(val s: String); class C(val s: String)
  given C = C("c-given")
  def f(using a: A, b: B, c: C): String = s"${a.s} ${b.s} ${c.s}"
  def g(x: Int)(using a: A, b: B): String = s"$x ${a.s} ${b.s}"
  def run(): Unit =
    println(f(using A("a"), B("b")))
    given B = B("b-given")
    println(g(1)(using A("a2")))

object DependentAndInferred:
  class A(val s: String = "a")
  trait Show[T]:
    def show(t: T): String
  given Show[Int] = (t: Int) => s"int:$t"
  given Show[String] = (t: String) => s"str:$t"
  trait Build[T]:
    type Out
    def build(t: T): Out
  given Build[Int] with
    type Out = String
    def build(t: Int): String = "built" + t
  // A given of a type parameter, which is no path: the dependent slot reads its type.
  given [T]: Build[List[T]] with
    type Out = Int
    def build(t: List[T]): Int = t.length
  def both[T](x: T)(using a: A, s: Show[T]): String = s"${a.s} ${s.show(x)}"
  def dep[T](x: T)(using a: A, bld: Build[T], s: Show[bld.Out]): String = s"${a.s} ${s.show(bld.build(x))}"
  def later[T, U](x: T)(using a: A, conv: T => U, s: Show[U]): String = s"${a.s} ${s.show(conv(x))}"
  given (Int => String) = i => "c" + i
  def run(): Unit =
    println(both(1)(using A("x")))
    println(both("s")(using A("y")))
    println(dep(2)(using A("z")))
    println(dep(List(1, 2))(using A("v")))
    println(later(3)(using A("w")))

object ParentCallAndEnum:
  class A(val s: String = "a")
  class B(val s: String = "b")
  given B = new B("givenB")
  class Base(x: Int)(using val a: A, val b: B)
  class Sub extends Base(1)(using A("sub"))
  enum E(using val a: A, val b: B):
    case One extends E(using A("one"))
  def run(): Unit =
    println(new Sub().b.s)
    println(E.One.b.s)

object Order:
  class A(val s: String = "a")
  class B(val s: String = "b")
  class C(val s: String = "c")
  var log = List.empty[String]
  def mark[X](s: String, a: X): X = { log ::= s; a }
  def flush(): Unit = { println(log.reverse.mkString(" ")); log = Nil }
  def gb: B = mark("givenB", new B("gb"))
  def gc: C = mark("givenC", new C("gc"))
  def ga: A = mark("givenA", new A("ga"))
  // Each given made at each use, so that the order shows.
  given givenB(using DummyImplicit): B = gb
  given givenC(using DummyImplicit): C = gc
  given givenA(using DummyImplicit): A = ga
  class D(val s: String)
  def w(x: Int)(using a: A, b: B, c: C): String = s"${a.s} ${b.s} ${c.s}"
  def v(x: Int)(using c: C, b: B, a: A): String = s"${c.s} ${b.s} ${a.s}"
  def d(using x: D = mark("defaultD", D("dd")), b: B, c: C): String = s"${x.s} ${b.s} ${c.s}"
  def run(): Unit =
    println(w(mark("x", 1))(using mark("argA", A("aa")))); flush()
    println(v(mark("x", 1))(using mark("argC", C("cc")))); flush()
    println(w(1)(using b = mark("argB", B("bb")))); flush()
    println(v(1)(using a = mark("argA", A("aa")), c = mark("argC", C("cc")))); flush()
    // A default and a searched given after the written argument, in the parameters' order.
    println(d(using c = mark("argC", C("cc")))); flush()

object UsingListShortSearched:
  def main(args: Array[String]): Unit =
    Cut.run()
    NamedOneGivenRest.run()
    ImplicitClauseWithUsing.run()
    ConstructorAndExtension.run()
    ThreeFirstTwoGiven.run()
    DependentAndInferred.run()
    ParentCallAndEnum.run()
    Order.run()
