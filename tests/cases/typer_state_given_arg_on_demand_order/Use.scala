// In a body typed on demand during a flush, the plain given `pg` found for an argument of a
// transparent given expands after the pending call `M.b` typed before it, as scalac: `b|b,pg`.
class Str(val s: String)
object Str:
  inline given pg: Str = Str(M.pgv)
class R(val s: String)
object R:
  transparent inline given preferred(using inline x: Str): R = R(x.s)

object Main:
  @main def run(): Unit = println(go)
  inline def go: String = helper
  def helper = { val k = M.b; k + "|" + summon[R].s }
