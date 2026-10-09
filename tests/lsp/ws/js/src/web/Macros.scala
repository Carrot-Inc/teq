package web

import scala.quoted.*

object Macros:
  inline def plusOne(inline x: Int): Int = ${ plusOneImpl('x) }
  def plusOneImpl(x: Expr[Int])(using Quotes): Expr[Int] = '{ $x + 1 }
