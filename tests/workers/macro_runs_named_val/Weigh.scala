package scales

import scala.quoted.*

object Weigh:
  inline def weigh(inline n: Int): Int = ${ weighCode('n) }
  def weighCode(n: Expr[Int])(using Quotes): Expr[Int] = Expr(Ledger.total(n.valueOrAbort))
