package rounds

import scala.quoted.*

object Counting:
  inline def sized: Int = ${ sizedImpl }
  def sizedImpl(using Quotes): Expr[Int] = Expr(rounddemand.Defaults.sized(Array(3, 1, 4)))
