// The expansion of the plain given `pg`, found
// for an argument of a transparent given, types
// `H.helperW` on demand, whose warning its definition
// keeps: the second alternative applies, `i w!`.
trait Ops:
  extension [T](x: Int)
    def combine(y: String, z: Int): String = "s " + y
  extension [T](x: Int)
    def combine(y: String, z: String): String = "i " + y
object O extends Ops

@main def run(): Unit =
  println(O.combine[Unit](1)(summon[R].s, "z"))

class Str(val s: String)
object Str:
  inline given pg: Str = Str(H.helperW)
class R(val s: String)
object R:
  transparent inline given preferred(using inline x: Str): R = R(x.s)
object H:
  def helperW = M.warnv + "!"
