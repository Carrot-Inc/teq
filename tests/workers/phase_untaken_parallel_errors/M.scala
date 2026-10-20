import scala.quoted.*
object M:
  inline def value: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(7)
