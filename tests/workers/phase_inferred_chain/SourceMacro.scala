import scala.quoted.*
object SourceMacro:
  transparent inline def value: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(42)
