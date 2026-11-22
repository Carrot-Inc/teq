// Traits of evidence alone whose calls bind nothing first: each argument, written or searched or a default, is still
// evaluated in its trait's turn of the linearization's initialization, after the superclass and the traits before it,
// as Mixin's trait initializers run (the review's twin), in every kind of class, a local class capturing a local and a
// local trait that captures one.
class A(val s: String)
class B(val s: String)
class C(val s: String)
given ga(using DummyImplicit): A = { println("given a"); A("ga") }
given gc(using DummyImplicit): C = { println("given c"); C("gc") }
trait T1(using a: A, b: B):
  println("T1 " + a.s + " " + b.s)
trait T2(using b: B, c: C):
  println("T2 " + b.s + " " + c.s)
trait D1(using a: A, b: B = { println("default b1"); B("db1") }):
  println("D1 " + a.s + " " + b.s)
trait D2(using c: C, b: B = { println("default b2"); B("db2") }):
  println("D2 " + c.s + " " + b.s)
trait G[X](using x: X):
  println("G")
trait U extends T1:
  println("U")
class Base(x: Int):
  println("Base " + x)
def w(n: String): B = { println(n); B(n) }
class Twin extends T1(using a = summon[A], b = w("b1"))() with T2(using w("b2"))
class Positional extends T2(using w("b2"), { println("c2"); C("c2") }) with T1(using { println("a1"); A("a1") }, w("b1"))
class Defaults extends D1(using { println("a1"); A("a1") }) with D2(using { println("c2"); C("c2") })
class Searched extends D2(using { println("c2"); C("c2") }) with D1
// A given B only where the searched B is meant: beside the defaults of D1 and D2 scalac warns that they shadow it.
object WithB:
  given gb(using DummyImplicit): B = { println("given b"); B("gb") }
  class Indirect extends U with T2(using w("b2"), { println("c2"); C("c2") })
class Super extends Base({ println("x"); 1 }) with T1(using { println("a1"); A("a1") }, w("b1")) with T2(using b = w("b2"))()
object O extends T1(using { println("a1"); A("a1") }, w("b1")) with T2(using b = w("b2"))()
class Secondary(x: Int) extends T1(using { println("a1"); A("a1") }, w("b1")) with T2(using b = w("b2"))():
  def this() = this({ println("x"); 0 })
class Generic extends G[A](using { println("a1"); A("a1") }) with T2(using w("b2"), { println("c2"); C("c2") })
enum E extends T2(using w("b2"), { println("c2"); C("c2") }):
  case K
def local(): Unit =
  val tag = { println("tag"); "t" }
  class S extends T1(using A(tag), w("b1")) with T2(using b = B(tag))()
  new S
// A local trait of evidence alone that captures a local: its call binding nothing, binding first, a member reading it.
def localTrait(): Unit =
  val tag = { println("tag"); "t" }
  trait L(using a: A, b: B):
    println("L " + tag + " " + a.s + " " + b.s)
  trait M(using a: A, b: B):
    def show = "M " + tag + " " + a.s + " " + b.s
  class S1 extends L(using { println("a1"); A("a1") }, w("b1"))
  class S2 extends L(using b = w("b2"))()
  class S3 extends M(using { println("a3"); A("a3") }, w("b3"))
  new S1
  new S2
  println(new S3().show)
object Main:
  def main(args: Array[String]): Unit =
    new Twin; println("-")
    new Positional; println("-")
    new Defaults; println("-")
    new Searched; println("-")
    new Super; println("-")
    new WithB.Indirect; println("-")
    O.hashCode; println("-")
    new Secondary(); println("-")
    new Generic; println("-")
    E.K.ordinal; println("-")
    new T1(using { println("a1"); A("a1") }, w("b1")) with T2(using b = w("b2"))() {}; println("-")
    local(); println("-")
    localTrait()
