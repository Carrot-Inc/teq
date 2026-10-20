// A candidate given up whose expansion typed `H.helperW` on demand keeps no expansion of its
// body's plain call: the inlining phase expands it once, in `H`'s tree, whose macro warns there,
// beside the phase's error, as scalac warns (typer_state_dropped_promoted runs it without both).
// expect: Use.scala:27:17: warning: warn w
// expect: Use.scala:20:11: error: after
// expect: 1 error found
import scala.compiletime.summonInline
class Missing
class W(val n: Int)
trait Low:
  given fallback: W = new W(0)
trait Ops:
  extension [T](x: Int)
    def combine(y: Int, z: Int): String = "s " + y
  extension [T](x: Int)
    def combine(y: Int, z: String): String = "i " + y
object O extends Ops
@main def run(): Unit =
  println(O.combine[Unit](1)(summon[W].n, "z"))
  println(M.fail)
object W extends Low:
  transparent inline given chosen: W =
    H.helperW
    summonInline[Missing]
    new W(7)
object H:
  def helperW = M.warnv + "!"
