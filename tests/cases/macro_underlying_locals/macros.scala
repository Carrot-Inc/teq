import scala.quoted.*

// `underlyingArgument` as dotty's `MapToUnderlying` restricted to `InlineProxy | Synthetic` locals:
// an inline method's parameter proxy gives its argument, a user's local stays a reference.
object U:
  inline def shown(inline x: Any): String = ${ shownImpl('x) }
  inline def proxied(x: Int): String = ${ shownImpl('x) }
  // A proxy inside the argument of another inline method's proxy: replaced at its depth.
  inline def nestedExpr(x: Int): String = proxied(x + 1)
  def shownImpl(x: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    Expr(x.asTerm.underlyingArgument.show)

  // The argument spliced twice: a local read twice, its right side evaluated once where it stands.
  inline def twice(inline x: Int): Int = ${ twiceImpl('x) }
  def twiceImpl(x: Expr[Int])(using Quotes): Expr[Int] =
    import quotes.reflect.*
    val u = x.asTerm.underlyingArgument.asExprOf[Int]
    '{ $u + $u }
