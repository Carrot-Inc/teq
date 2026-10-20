// A macro counting its expansions, called plainly and from a transparent method's body, for
// inline_phase_transparent_nested.
import scala.quoted.*
object P:
  var n = 0
  inline def next: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)
  transparent inline def viaTransparent: Int = next + 0
  transparent inline def forced: Int = inline if flag then 7 else 8
  inline def flag: Boolean = true
