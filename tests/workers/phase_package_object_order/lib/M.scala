package counter

import scala.quoted.*

object M:
  var count = 0
  inline def next: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    count += 1
    Expr(count)
