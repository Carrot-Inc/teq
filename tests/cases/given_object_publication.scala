// A given object of a package or of an object is a module, published before its body runs (dotty's
// static classification, `LazyVals` 74-105; `MODULE$` stored after the super call on the JVM,
// `JSCodeGen` 2308-2313): a method its body calls that reads the given gets the instance.
trait T { val self: T }
def helper: T = t
given t: T with
  val self: T = helper
object O:
  def helper: T = u
  given u: T with
    val self: T = helper
  def viaSearch: T = summon[T]
@main def run(): Unit =
  println(t.self eq t)
  println(O.u.self eq O.u)
  println(O.viaSearch eq O.u)
