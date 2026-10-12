// A user's member spelled like a default getter is forwarded by a wildcard export: what
// Namer.addWildcardForwarders leaves with its method is a DefaultGetterName, a name's kind.
class Lib:
  def user$default$x: Int = 7
  def greet(name: String = "you"): String = s"hi $name"

class Wrap(val lib: Lib):
  export lib.*

@main def run(): Unit =
  val w = new Wrap(new Lib)
  println(w.user$default$x)
  println(w.greet())
