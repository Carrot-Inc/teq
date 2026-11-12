// A macro of the program, as a plain inline method and as an extension, whose implementation runs
// `topHelper(k)` over Kinds.scala, the program's smallest file, which holds no quoted code.
// A top-level value without a declared type, initialised by an anonymous class, which a method the macro's implementation calls reads.
package plot

import scala.quoted.*

object Scaling:
  inline def scaled(inline n: Int): Int = ${ scaledImpl('n) }
  extension (inline n: Int) inline def scaledUp: Int = ${ scaledImpl('n) }
  def scaledImpl(n: Expr[Int])(using Quotes): Expr[Int] =
    val k = n.valueOrAbort
    Expr(topHelper(k))
  transparent inline def pick(inline n: Int): Int = n
