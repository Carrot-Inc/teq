// A local class that extends a trait nested in a class, inside the initialiser of a val that the
// trait's abstract method names in a parameter's type: the class defines the trait's outer
// accessor unless the trait is a pure interface (dotty's `ExplicitOuter.needsOuterIfReferenced`),
// which is read from what the trait's members are written as (`TreeInfo.defKind`), no member's
// type asked. `Slot` is one, and the val is typed.
class Rack:
  trait Slot:
    def fit(a: Yard.held.Label): Int

class Holder:
  class Label

object Yard:
  val rack = new Rack
  val held =
    abstract class Local extends rack.Slot
    println("local")
    new Holder

@main def run(): Unit =
  val label = new Yard.held.Label
  println(label != null)
