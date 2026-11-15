package forge

class Rod(val name: String):
  def label: String = "rod " + name

object Shed:
  val made = new Rod("b" + 1)

// The anonymous class of the method's body overrides a member of its parent: a module that calls the
// method writes the class with the override, whichever worker types the call.
object Gauge:
  transparent inline def make(n: String): String =
    val b = new Rod(n) { override def label: String = "got " + name }
    b.label
