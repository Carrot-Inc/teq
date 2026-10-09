import scala.quoted.*

object Counter:
  private var n = 0
  inline def next: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    n += 1
    if n == 1 then quotes.reflect.report.errorAndAbort("first expansion")
    Expr(n)
