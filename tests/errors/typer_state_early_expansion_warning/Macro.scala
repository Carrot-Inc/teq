import scala.quoted.*

object M:
  inline def warn: Int = ${ warnImpl }
  def warnImpl(using Quotes): Expr[Int] =
    quotes.reflect.report.warning("prefix warning")
    Expr(7)
  inline def one: 1 = 1
  transparent inline def id(inline x: Int): Int = x
