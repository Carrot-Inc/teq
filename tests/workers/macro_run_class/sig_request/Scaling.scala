// A macro of the program, as a plain inline method and as an extension, whose implementation runs
// `topHelper(k)` over Kinds.scala, the program's smallest file, which holds no quoted code.
// A method without a declared result type, whose body makes an anonymous class: the macro's implementation asks for its signature, then calls a method that calls it.
package plot

import scala.quoted.*

object Scaling:
  inline def scaled(inline n: Int): Int = ${ scaledImpl('n) }
  extension (inline n: Int) inline def scaledUp: Int = ${ scaledImpl('n) }
  def scaledImpl(n: Expr[Int])(using Quotes): Expr[Int] =
    val k = n.valueOrAbort
    quotes.reflect.Symbol.requiredMethod("plot.make").signature
    Expr(topHelper(k))
  transparent inline def pick(inline n: Int): Int = n
