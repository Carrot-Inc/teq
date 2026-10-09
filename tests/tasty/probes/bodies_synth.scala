// The synthesized members' bodies: a case class (equals' x$0, hashCode's acc,
// productElement, productElementName, copy and its defaults, the companion's apply, unapply,
// toString, fromProduct), a generic case class, a case object, an enum of values ($new's
// anonymous class, values, valueOf, fromOrdinal, ordinal) and one with a class case, a value
// class, a var's setter, defaults of a method and of a constructor, export forwarders.
package probe.synth

case class Pt(x: Int, y: String = "y")
case class Box[A](a: A, n: Int)
case object Unit1
enum Color:
  case Red, Green
enum Tree:
  case Leaf
  case Node(l: Tree, r: Tree)
class Wrap(val u: Int) extends AnyVal
class Holder:
  var count: Int = 0
  def withDefault(a: Int, b: Int = 2)(c: String = (a + b).toString): String = c * a
class WithCtorDefault(val n: Int = 5, val s: String = "s")
object Exp:
  val v: Int = 1
  def f(a: Int): Int = a
class Exporter:
  export Exp.{v, f}
