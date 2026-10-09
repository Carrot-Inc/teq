import scala.quoted.*

object Macros:
  def twice(x: Expr[Int])(using Quotes): Expr[Int] = '{ $x * 2 }
  def upper(s: Expr[String])(using Quotes): Expr[String] = Expr(s.valueOrAbort.toUpperCase)

object Dsl:
  inline def shout(inline s: String): String = ${ Macros.upper('s) }
