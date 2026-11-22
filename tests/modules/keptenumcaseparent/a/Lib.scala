// An enum's value case of a second parent whose trait runs a body: the TASTy writer withholds the case's creation (its
// value class extends the enum alone), and the products' reader refuses the enum rather than make the case without it.
package kep
class A(val s: String)
trait T(using a: A):
  println("T " + a.s)
object Lib:
  enum E:
    case K extends E with T(using { println("a"); A("wa") })
  def make: Unit =
    println(E.K.ordinal)
    println("done")
