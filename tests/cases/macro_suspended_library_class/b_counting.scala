package rounds

import scala.quoted.*

object Counting:
  inline def tally: Int = ${ tallyImpl }
  def tallyImpl(using Quotes): Expr[Int] = Expr(new rounddemand.Tally().total)
