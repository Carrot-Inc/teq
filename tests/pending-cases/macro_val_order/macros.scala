package mac

import scala.quoted.*

object Macros:
  def shapeImpl(text: Expr[String])(using Quotes): Expr[String] =
    Expr(text.valueOrAbort + separator)

inline def shape(inline text: String): String = ${ Macros.shapeImpl('text) }
