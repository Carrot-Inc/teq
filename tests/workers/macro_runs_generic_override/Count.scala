package tills

import scala.quoted.*

object Count:
  inline def count(inline n: Int): Int = ${ countCode('n) }
  def countCode(n: Expr[Int])(using Quotes): Expr[Int] = Expr(Till.total(n.valueOrAbort))
