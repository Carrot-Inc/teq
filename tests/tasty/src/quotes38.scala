// Compiled with Scala 3.8.4, which pickles a quote as `'{ e }.apply(q)` where 3.3 wrote
// `quote[T](e).apply(q)` (`tests/classpath/js/quotes_28_8.scala`).
package fix.q38

import scala.quoted.*

object Quotes38:
  inline def twice(inline n: Int): Int = ${ twiceImpl('n) }
  def twiceImpl(n: Expr[Int])(using Quotes): Expr[Int] = '{ $n * 2 }
  inline def described(inline n: Int): String = ${ describedImpl('n) }
  def describedImpl(n: Expr[Int])(using Quotes): Expr[String] = Expr("value " + n.valueOrAbort)
