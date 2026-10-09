package shared

import scala.quoted.*

object Mac:
  inline def tidy(inline s: String): String = ${ tidyImpl('s) }
  def tidyImpl(s: Expr[String])(using Quotes): Expr[String] =
    val boxed: java.lang.Integer = s.valueOrAbort.length
    Expr(s.valueOrAbort.trim().toLowerCase(java.util.Locale.ROOT) + boxed)
