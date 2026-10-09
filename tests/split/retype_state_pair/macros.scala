package pair

import scala.quoted.*

/** A macro whose state connects its expansions: each has to be the one its caller says. */
object Counter:
  var n: Int = 0

  def impl(expected: Expr[Int])(using Quotes): Expr[Int] =
    n += 1
    if n != expected.valueOrAbort then quotes.reflect.report.errorAndAbort("actual " + n)
    Expr(n)

inline def check(inline expected: Int): Int = ${ Counter.impl('expected) }
