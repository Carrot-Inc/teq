// A using application of Object's wait, notify or notifyAll, none of whose methods is contextual: no alternative
// is applied (dotty's Applications.resolveOverloaded of an ApplyKind.Using prototype, Typer.matchingApply), no
// adaptation of the qualifier is tried, and the method misses its arguments, as scalac 3.8.4 refuses them.
object Main:
  def main(args: Array[String]): Unit =
    val g = new Object
    g.synchronized { g.wait(using 1L) }
    g.synchronized { g.notify(using 1L) }
    g.synchronized { g.notifyAll(using 1L) }
// expect: 7:22: error: Missing arguments for method wait in class Object
// expect: 8:22: error: Missing arguments for method notify in class Object
// expect: 9:22: error: Missing arguments for method notifyAll in class Object
// expect: 3 errors found
