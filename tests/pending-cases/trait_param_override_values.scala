// A trait's val parameter overridden by a class's
// (`cannot override val parameter value s in trait T`, a warning scalac
// 3.8.4 gives): scalac prints the class's value, `1 1`; teq's interpreter
// and JavaScript are to print it too. Queued apart from enum-override-param.
trait T(val s: Int):
  def f = s
  val t = f
class C(override val s: Int) extends T(s + 1)

@main def run(): Unit =
  val c = C(1)
  println(s"${c.s} ${c.t}")
