package kiln

class Rod(val name: String):
  given tag: Int = name.length

object Shed:
  val made = new Rod("b" + 1)

// The anonymous class of the method's body declares a class and a trait of its own: each expansion
// copies them with it. A module that calls the method twice builds, and its pickle leaves the caller's
// body out, with the reason.
object Gauge:
  transparent inline def make(n: String): Int =
    val b = new Rod(n) {
      class Notch(val at: Int)
      trait Mark { def at: Int }
      def got: Int = { println("got " + name); new Notch(tag).at + new Mark { def at: Int = 0 }.at }
      val first: Int = got
    }
    b.tag
