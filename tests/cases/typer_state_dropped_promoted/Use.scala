// A candidate given up whose expansion typed `H.helperW` on demand, which warned, drops what it
// reported and leaves no promoted range behind (`state::diags_cut_at`): the first extension
// alternative's mismatch is its failure, and the second applies, `i 0`, as scalac and master print.
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
object W extends Low:
  transparent inline given chosen: W =
    H.helperW
    summonInline[Missing]
    new W(7)
object H:
  def helperW = M.warnv + "!"
