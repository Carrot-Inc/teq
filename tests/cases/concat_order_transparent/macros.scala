import scala.quoted.*

object Macros:
  transparent inline def tmacro(inline a: Any): String = ${ impl('a) }
  inline def pmacro(inline a: Any): String = ${ impl('a) }
  def impl(a: Expr[Any])(using Quotes): Expr[String] = '{ "" + $a }

  inline def quotedTransparent(inline a: Any, inline b: String): String = ${ quotedTransparentImpl('a, 'b) }
  def quotedTransparentImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    '{ Inlines.cat($a) + $b }

  inline def quotedPlain(inline a: Any, inline b: String): String = ${ quotedPlainImpl('a, 'b) }
  def quotedPlainImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    '{ Inlines.plain($a) + $b }

  inline def quotedAscribedTransparent(inline a: Any, inline b: String): String = ${ quotedAscribedTransparentImpl('a, 'b) }
  def quotedAscribedTransparentImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    '{ (Inlines.cat($a): String) + $b }

object Inlines:
  transparent inline def cat(inline x: Any): String = "" + x
  inline def plain(inline x: Any): String = "" + x
