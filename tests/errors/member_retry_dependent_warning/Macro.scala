// Counters that warn at each expansion.
import scala.quoted.*
object M:
  var n = 0
  transparent inline def next: Int = ${impl}
  def impl(using Quotes): Expr[Int] =
    n += 1
    quotes.reflect.report.warning("macro expansion")
    Expr(n)
  transparent inline def same(x: Int): x.type = ${sameImpl[x.type]('x)}
  def sameImpl[T: Type](x: Expr[Int])(using Quotes): Expr[T] =
    n += 1
    quotes.reflect.report.warning("same expansion")
    '{ $x.asInstanceOf[T] }
