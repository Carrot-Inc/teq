import scala.quoted.*
object A:
  inline def value: String = ${ impl }
  def impl(using Quotes): Expr[String] = Expr(H.name)
