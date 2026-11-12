package rounds

import scala.quoted.*

object Counting:
  inline def pair: Int = ${ pairImpl }
  def pairImpl(using Quotes): Expr[Int] = Expr(Array(3, 1, 4).take(2).length)
  inline def seven: Int = ${ sevenImpl }
  def sevenImpl(using Quotes): Expr[Int] = Expr(7)
