// A macro that counts its expansions, for inline_phase_walk_order's order of the inlining phase.
import scala.quoted.*
object P:
  var n = 0
  inline def next: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)
  inline def pair(x: Int, y: Int): String = s"$x,$y"
