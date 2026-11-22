// A trait of using clauses alone applied without its empty clause where scalac takes it: named arguments out of their
// order that are constructor calls, which the call does not bind first, and a parameter left out that has a default.
class A(val s: String)
class B(val s: String)
class C(val s: String)
trait T1(using a: A, b: B = B("db"), c: C):
  def show = s"${a.s} ${b.s} ${c.s}"
trait T2(using a: A, b: B, c: C):
  def show = s"${a.s} ${b.s} ${c.s}"
class D1 extends T1(using a = A("wa"), c = C("wc"))
class R1 extends T2(using c = C("wc"), a = A("wa"), b = B("wb"))
object Main:
  def main(args: Array[String]): Unit =
    println(new D1().show)
    println(new R1().show)
