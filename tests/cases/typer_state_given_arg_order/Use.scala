// A plain inline given found for an argument of a transparent given the search expands where it
// finds it is expanded at once, after the pending call `M.a` typed before it, so that their
// macros run in scalac's order: `a`, then `a,pg`.
class Str(val s: String)
object Str:
  inline given pg: Str = Str(M.pgv)
class R(val s: String)
object R:
  transparent inline given preferred(using inline x: Str): R = R(x.s)

@main def run(): Unit =
  val a = M.a
  val r = summon[R]
  println(a)
  println(r.s)
