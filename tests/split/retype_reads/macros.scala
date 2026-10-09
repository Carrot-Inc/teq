package reads

import scala.quoted.*

object Macros:
  def value: Int = setting

  def taggedImpl(text: Expr[String])(using Quotes): Expr[String] =
    Expr(Config.prefix + text.valueOrAbort + suffix + new Wrap("w").shown + value + reads.second.H.value)

inline def tagged(inline text: String): String = ${ Macros.taggedImpl('text) }
