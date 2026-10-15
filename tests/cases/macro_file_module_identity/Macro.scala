import scala.quoted.*
object Inspect:
  inline def ownerOf[T]: String = ${ impl[T] }
  def impl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val t = TypeRepr.of[T]
    val s = t.typeSymbol
    val prefix = t match
      case TypeRef(q, _) => q.termSymbol.moduleClass
      case _ => Symbol.noSymbol
    Expr(s.owner.name + ":" + prefix.name + ":" + (s.owner == prefix))
