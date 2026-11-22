// A trait of evidence alone applied by a parent or an anonymous class with a named list that leaves a parameter out
// before a written one: the written argument is evaluated first, then the searched ones in their parameters' order, as
// scalac evaluates them (the givens are defs, so each instance shows the order).
class A(val s: String)
class B(val s: String)
class C(val s: String)
given ga(using DummyImplicit): A = { println("given a"); A("ga") }
given gc(using DummyImplicit): C = { println("given c"); C("gc") }
trait Ti(implicit a: A, b: B, c: C):
  def show = s"${a.s} ${b.s} ${c.s}"
trait Tu(using a: A, b: B, c: C):
  def show = s"${a.s} ${b.s} ${c.s}"
class SubI extends Ti(using b = { println("b"); B("wb") })
class SubI0 extends Ti()(using b = { println("b"); B("wb") })
class SubU extends Tu(using b = { println("b"); B("wb") })()
object Main:
  def main(args: Array[String]): Unit =
    println(new SubI().show)
    println(new SubI0().show)
    println(new SubU().show)
    println(new Ti(using b = { println("b"); B("wb") }) {}.show)
    println(new Tu(using b = { println("b"); B("wb") })() {}.show)
