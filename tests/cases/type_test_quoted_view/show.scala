import scala.quoted.*

object Show:
  inline def show(inline e: Any): String = ${ showImpl('e) }
  def showImpl(e: Expr[Any])(using Quotes): Expr[String] = Expr(e.show)

  inline def kind(inline e: Any): String = ${ kindImpl('e) }
  def kindImpl(e: Expr[Any])(using Quotes): Expr[String] = e match
    case '{ ($x: Any).isInstanceOf[AnyVal] } => Expr("a test of AnyVal")
    case '{ ($x: Any).isInstanceOf[Any] } => Expr("a test of Any")
    case '{ ($x: Any).isInstanceOf[Int] } => Expr("a test of Int")
    case _ => Expr("another")
