package mhb

import scala.quoted.*

object Mac:
  inline def lbl(inline n: Int): String = ${ lblImpl('n) }
  def lblImpl(n: Expr[Int])(using Quotes): Expr[String] =
    Expr(mha.Help.label(n.valueOrAbort))
