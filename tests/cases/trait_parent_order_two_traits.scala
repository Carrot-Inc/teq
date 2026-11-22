// Two traits of evidence alone, each with a written and a searched argument: every argument is evaluated in its
// trait's turn of the linearization's initialization, the bindings a call makes first (a named argument after a searched
// one) kept in its trait's turn too. The review's program, its order reversed, and named lists with defaults.
class A(val s: String)
class B(val s: String)
class C(val s: String)
given ga(using DummyImplicit): A = { println("given a"); A("ga") }
given gb(using DummyImplicit): B = { println("given b"); B("gb") }
given gc(using DummyImplicit): C = { println("given c"); C("gc") }
trait T1(using a: A, b: B):
  println("T1 " + a.s + " " + b.s)
trait T2(using b: B, c: C):
  println("T2 " + b.s + " " + c.s)
trait D1(using a: A, b: B = { println("default b1"); B("db1") }):
  println("D1 " + a.s + " " + b.s)
trait D2(using c: C, b: B = { println("default b2"); B("db2") }):
  println("D2 " + c.s + " " + b.s)
class Mixed extends T1(using b = { println("b1"); B("b1") })() with T2(using { println("b2"); B("b2") })
class Reversed extends T2(using b = { println("b2"); B("b2") })() with T1(using b = { println("b1"); B("b1") })()
class Named extends D1(using b = { println("b1"); B("b1") })() with D2(using b = { println("b2"); B("b2") })()
object Main:
  def main(args: Array[String]): Unit =
    new Mixed
    println("-")
    new Reversed
    println("-")
    new Named
    println("-")
    new D2(using b = { println("b2"); B("b2") })() with D1(using b = { println("b1"); B("b1") })() {}
