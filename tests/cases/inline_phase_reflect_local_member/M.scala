import scala.quoted.*

object M:
  inline def memberHasRhs[T]: Boolean = ${ memberImpl[T] }
  inline def classTreeHasRhs[T]: Boolean = ${ classImpl[T] }

  def memberImpl[T: Type](using Quotes): Expr[Boolean] =
    import quotes.reflect.*
    val method = TypeRepr.of[T].typeSymbol.declaredMethod("value").head
    Expr(method.tree.asInstanceOf[DefDef].rhs.nonEmpty)

  def classImpl[T: Type](using Quotes): Expr[Boolean] =
    import quotes.reflect.*
    TypeRepr.of[T].typeSymbol.tree match
      case ClassDef(_, _, _, _, body) =>
        Expr(body.exists {
          case d: DefDef if d.name == "value" => d.rhs.nonEmpty
          case _ => false
        })
      case _ => Expr(false)
