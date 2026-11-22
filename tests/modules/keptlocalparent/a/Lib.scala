// A kept body's local class whose trait parent's call binds its arguments first (a named argument after a searched one):
// the products' reader does not read such a parent back, and refuses the body rather than build the class without it.
package klp
class A(val s: String)
class B(val s: String)
given A = A("ga")
trait T(using a: A, b: B):
  println("T " + a.s + " " + b.s)
object Lib:
  def make: Unit =
    class S extends T(using b = { println("b"); B("wb") })()
    new S
    println("done")
  def other: Int = 7
