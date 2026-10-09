package mac

import scala.quoted.*

object Macros:
  def countImpl(text: Expr[String])(using Quotes): Expr[Int] =
    Expr(text.valueOrAbort.length)

inline def count(inline text: String): Int = ${ Macros.countImpl('text) }
