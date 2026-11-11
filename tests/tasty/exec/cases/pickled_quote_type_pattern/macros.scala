import scala.quoted.*

object ElementLabel:
  inline def of(inline value: Any): String = ${ label('value) }

  def label(value: Expr[Any])(using Quotes): Expr[String] = value match
    case '{ $items: List[t] } => Expr(Type.show[t])
    case _ => Expr("other")
