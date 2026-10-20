package p

import scala.quoted.*

object Counter:
  var n: Int = 0

  def impl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)

inline def next: Int = ${ Counter.impl }
