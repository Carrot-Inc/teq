import scala.quoted.*
trait Show[T]:
  def name: String
object Show:
  transparent inline def derived[T]: Show[T] = ${ impl[T] }
  def impl[T: Type](using Quotes): Expr[Show[T]] =
    val n = Expr(Type.show[T])
    '{ new Show[T] { def name = $n } }
