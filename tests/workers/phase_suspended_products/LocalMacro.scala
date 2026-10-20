import scala.quoted.*

object LocalMacro:
  inline def run: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(99)
