package rounds

import scala.quoted.*

object Counting:
  inline def seven: Int = ${ sevenImpl }
  def sevenImpl(using Quotes): Expr[Int] = Expr(7)
