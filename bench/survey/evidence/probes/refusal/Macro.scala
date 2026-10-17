import scala.quoted.*

object Probe:
  inline def fun(inline x: Any): String = ${ funImpl('x) }
  def funImpl(x: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val t = x.asTerm.underlyingArgument
    Expr(t.asInstanceOf[Apply].fun.show)
  inline def cond(inline x: Any): String = ${ condImpl('x) }
  def condImpl(x: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    x.asTerm.underlyingArgument match
      case If(c, _, _) => Expr(c.show)
      case t => Expr(t.asInstanceOf[If].cond.show)
