package meridian.core.effect

import scala.quoted.*

object TagMacros:
  def derivedImpl[A: Type](using Quotes): Expr[Tag[A]] =
    import quotes.reflect.*
    val name = TypeRepr.of[A].typeSymbol.name
    '{ Tag[A](${ Expr(name) }) }
