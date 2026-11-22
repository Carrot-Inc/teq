// A kept body's anonymous class whose trait parent's call binds its arguments first: the products' reader refuses it at
// the class's creation rather than build the class without its parent.
package kap
class A(val s: String)
class B(val s: String)
given A = A("ga")
trait T(using a: A, b: B):
  println("T " + a.s + " " + b.s)
object Lib:
  def make: Unit =
    new T(using b = { println("b"); B("wb") })() {}
    println("done")
