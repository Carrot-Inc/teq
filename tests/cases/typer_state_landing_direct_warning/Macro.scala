import scala.quoted.*
object M:
  transparent inline def next: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    quotes.reflect.report.warning("macro warning")
    Expr(2)
