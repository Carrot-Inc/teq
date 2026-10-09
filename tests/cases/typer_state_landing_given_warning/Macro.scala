import scala.quoted.*
class X(val v: Int)
object X:
  transparent inline given x: X = ${ impl }
  def impl(using Quotes): Expr[X] =
    quotes.reflect.report.warning("given warning")
    '{ new X(4) }
