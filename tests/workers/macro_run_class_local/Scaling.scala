// A macro whose implementation calls `Mid.via`, which calls `Deep.inner`: a method without a declared result
// type, whose body makes an anonymous class.
package plot

import scala.quoted.*

object Scaling:
  extension (inline n: Int) inline def scaledUp: Int = ${ scaledImpl('n) }
  def scaledImpl(n: Expr[Int])(using Quotes): Expr[Int] =
    val k = n.valueOrAbort
    Expr(Mid.via(k))
  transparent inline def pick(inline n: Int): Int = n
