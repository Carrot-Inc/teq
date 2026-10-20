import scala.quoted.*
object A:
  inline def value: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(H.value(42))
