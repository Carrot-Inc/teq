// A macro of the program, as a plain inline method and as an extension, whose implementation runs
// `new Gauge(1).twice(k)` over Kinds.scala, the program's smallest file, which holds no quoted code.
// A class the macro's implementation constructs.
package plot

import scala.quoted.*

object Scaling:
  inline def scaled(inline n: Int): Int = ${ scaledImpl('n) }
  extension (inline n: Int) inline def scaledUp: Int = ${ scaledImpl('n) }
  def scaledImpl(n: Expr[Int])(using Quotes): Expr[Int] =
    val k = n.valueOrAbort
    Expr(new Gauge(1).twice(k))
  transparent inline def pick(inline n: Int): Int = n
