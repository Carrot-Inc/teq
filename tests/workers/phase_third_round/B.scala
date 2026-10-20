import scala.quoted.*
object B:
  inline def result: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(1)
