import scala.quoted.*
object N:
  inline def value: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(7)
