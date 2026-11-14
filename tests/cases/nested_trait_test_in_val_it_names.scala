// A type test for a trait nested in a class, inside the initialiser of a val that the trait's
// abstract method names in a parameter's type: whether the test compares the enclosing instance
// turns on the trait being a pure interface (dotty's `ExplicitOuter.needsOuterIfReferenced`),
// which dotty reads from the untyped tree (`TreeInfo.defKind`: no member with a right side, no
// parameter with a default), asking no member's type. `Slot` is one: the test is the plain
// instance test, and the val is typed.
class Rack:
  trait Slot:
    def fit(a: Yard.held.Label): Int

class Holder:
  class Label

object Yard:
  val rack = new Rack
  val held =
    val kind = (1: Any) match
      case _: rack.Slot => "slot"
      case _ => "other"
    println(kind)
    new Holder

@main def run(): Unit =
  val label = new Yard.held.Label
  println(label != null)
