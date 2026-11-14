// A member class extends a trait nested in a class whose abstract method names, in a parameter's
// type, a val of the member class's own object, and that val's initialiser names the class: the
// method's signature needs the val, the val's body the class.
class Rack:
  trait Slot:
    def fit(a: Yard.held.Label): Int

class Holder:
  class Label

object Yard:
  val rack = new Rack
  abstract class Crate extends rack.Slot
  val held =
    val none: Crate = null
    println(none == null)
    new Holder

@main def run(): Unit =
  val label = new Yard.held.Label
  println(label != null)
