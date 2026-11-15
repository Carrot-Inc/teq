package vat

class Rod(val name: String):
  val len: Int = name.length

// The anonymous class of the method's body declares a member by the singleton type of a member of
// its own class: the class each expansion makes downstream declares it by that member's type.
object Gauge:
  inline def make(n: String): Int =
    val b = new Rod(n) { val got: this.len.type = len }
    b.len
