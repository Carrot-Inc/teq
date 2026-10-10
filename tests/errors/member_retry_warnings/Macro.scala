import scala.quoted.*

// A transparent macro that warns at its call each time it is expanded.
object W:
  transparent inline def next: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    quotes.reflect.report.warning("macro warning")
    Expr(2)
