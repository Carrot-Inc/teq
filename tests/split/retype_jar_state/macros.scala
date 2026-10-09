package jarstate

import scala.quoted.*

/** Counts with a jar's object at every expansion, the count in the result and in a warning. */
object Macros:
  def countImpl(using Quotes): Expr[Int] =
    val n = statelib.Counter.next()
    quotes.reflect.report.warning("counter " + n)
    Expr(n)

inline def count: Int = ${ Macros.countImpl }
