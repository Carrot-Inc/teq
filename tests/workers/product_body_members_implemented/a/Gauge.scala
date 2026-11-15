package mill

trait Rod:
  def name: String
  def label: String

object Shed:
  val made: Rod = new Rod { def name = "b1"; def label = "rod " + name }

// The anonymous class of the method's body implements its parent's abstract members: a module that
// calls the method writes the class with them, whichever worker types the call.
object Gauge:
  transparent inline def make(n: String): String =
    val b = new Rod { def name = n; def label = "got " + name }
    b.label
