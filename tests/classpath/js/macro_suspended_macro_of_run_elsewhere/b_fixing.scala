package rounds

import scala.quoted.*

object Fixing:
  inline def level: Int = ${ levelImpl }
  def levelImpl(using Quotes): Expr[Int] = Expr(42)
