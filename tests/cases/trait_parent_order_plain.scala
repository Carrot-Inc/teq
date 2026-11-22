// A trait with a plain clause and then a using or implicit one, applied by a parent or an anonymous class with a named
// list that leaves a parameter out before a written one, or names two out of their order: the written arguments are
// evaluated first, in the order written, then the searched ones in their parameters' order, as scalac evaluates them.
class A(val s: String)
class B(val s: String)
class C(val s: String)
given ga(using DummyImplicit): A = { println("given a"); A("ga") }
given gc(using DummyImplicit): C = { println("given c"); C("gc") }
trait Tr(x: Int)(using a: A, b: B, c: C):
  def show = s"$x ${a.s} ${b.s} ${c.s}"
trait Ti(x: Int)(implicit a: A, b: B, c: C):
  def show = s"$x ${a.s} ${b.s} ${c.s}"
def one = { println("x"); 1 }
class Sub extends Tr(one)(using b = { println("b"); B("wb") })
class SubI extends Ti(one)(using b = { println("b"); B("wb") })
class Sub2 extends Tr(one)(using c = { println("c"); C("wc") }, b = { println("b"); B("wb") })
object Main:
  def main(args: Array[String]): Unit =
    println(new Sub().show)
    println(new Sub().show)
    println(new SubI().show)
    println(new Sub2().show)
    println(new Tr(one)(using b = { println("b"); B("wb") }) {}.show)
