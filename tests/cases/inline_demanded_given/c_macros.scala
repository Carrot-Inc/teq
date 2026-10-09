package demandedgiven

import scala.quoted.*

inline def assertValue(inline expected: String): Int =
  ${ assertValueImpl('expected) }

def assertValueImpl(expected: Expr[String])(using Quotes): Expr[Int] =
  val actual = Holder.value
  if actual != expected.valueOrAbort then
    quotes.reflect.report.errorAndAbort("actual: " + actual)
  Expr(actual.length)
