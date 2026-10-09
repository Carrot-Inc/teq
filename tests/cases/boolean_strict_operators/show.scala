import scala.quoted.*

object Show:
  inline def show(inline b: Boolean): String = ${ showImpl('b) }
  def showImpl(b: Expr[Boolean])(using Quotes): Expr[String] = Expr(b.show)

  inline def either(inline a: Boolean, inline b: Boolean): Boolean = ${ eitherImpl('a, 'b) }
  def eitherImpl(a: Expr[Boolean], b: Expr[Boolean])(using Quotes): Expr[Boolean] = '{ $a | $b }
