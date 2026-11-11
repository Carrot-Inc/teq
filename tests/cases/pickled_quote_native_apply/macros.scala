import scala.quoted.*

object NativeShapes:
  inline def of(inline value: Any): String = ${ describe('value) }

  def describe(value: Expr[Any])(using Quotes): Expr[String] = value match
    case '{ ($left: Int) + ($right: Int) } => Expr("integer")
    case '{ ($left: Int) + ($right: Long) } => Expr("mixed")
    case '{ ($left: Boolean) && ($right: Boolean) } => Expr("short circuit")
    case '{ ($left: Boolean) & ($right: Boolean) } => Expr("strict")
    case '{ ($left: Int) == ($right: Int) } => Expr("numeric equality")
    case '{ ($left: Int).equals($right: Any) } => Expr("universal equality")
    case '{ ($left: String) + ($right: Int) } => Expr("text left")
    case '{ ($left: Int) + ($right: String) } => Expr("text right")
    case _ => Expr("other")
