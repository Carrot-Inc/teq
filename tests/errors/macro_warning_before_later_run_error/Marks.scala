import scala.quoted.*

object Marks:
  inline def noted(inline n: Int): Int = ${ notedImpl('n) }
  def notedImpl(n: Expr[Int])(using Quotes): Expr[Int] =
    quotes.reflect.report.warning("noted by the macro")
    '{ $n + 1 }
