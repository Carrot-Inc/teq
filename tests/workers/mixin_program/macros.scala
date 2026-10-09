import scala.quoted.*

object M:
  def impl(e: Expr[Int])(using Quotes): Expr[Int] =
    val k = e.valueOrAbort
    Expr(new C().f + k)

inline def top(inline k: Int): Int = ${ M.impl('k) }
