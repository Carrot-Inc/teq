package copies.macros

import scala.quoted.*

trait Show:
  def show: String

object Macros:
  inline def make(inline s: String): Show = ${ makeImpl('s) }

  def makeImpl(s: Expr[String])(using Quotes): Expr[Show] =
    '{ new Show { def show: String = "<" + $s + ">" } }
