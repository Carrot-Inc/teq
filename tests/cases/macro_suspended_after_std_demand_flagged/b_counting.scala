package rounds

import scala.quoted.*

object Counting:
  private var cold = true
  inline def pair: Int = ${ pairImpl }
  def pairImpl(using Quotes): Expr[Int] =
    if cold then
      cold = false
      Expr(Array(3, 1, 4).take(2).length)
    else Expr(2)
  inline def seven: Int = ${ sevenImpl }
  def sevenImpl(using Quotes): Expr[Int] = Expr(7)
