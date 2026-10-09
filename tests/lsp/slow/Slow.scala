import scala.quoted.*

object Slow:
  inline def run: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    while true do ()
    Expr(1)
