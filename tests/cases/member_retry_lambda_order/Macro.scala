// A counter of its expansions that warns at each.
import scala.quoted.*
object M:
  var n = 0
  transparent inline def next: Int = ${impl}
  def impl(using Quotes): Expr[Int] =
    n += 1
    quotes.reflect.report.warning("macro expansion")
    Expr(n)
