import scala.quoted.*

object LocalMacro:
  transparent inline def run: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(99)
