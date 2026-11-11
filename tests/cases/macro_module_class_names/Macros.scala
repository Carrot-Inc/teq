package q

import scala.quoted.*

object M:
  inline def sym[T]: String = ${ symImpl[T] }
  def symImpl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val s = TypeRepr.of[T].typeSymbol
    Expr(s.name + " " + s.fullName + " " + Type.show[T])
