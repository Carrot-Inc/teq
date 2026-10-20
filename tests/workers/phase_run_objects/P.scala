import scala.quoted.*
object P:
  var n = 0
  def impl(using Quotes): Expr[Int] = { n += 1; Expr(n) }
  inline def next: Int = ${ impl }
