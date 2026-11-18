import scala.quoted.*

// A macro that refuses an argument it can read: its use below is an error of the program.
object Checked:
  def positiveImpl(x: Expr[Int])(using Quotes): Expr[Int] =
    x.value match
      case Some(n) if n <= 0 => quotes.reflect.report.errorAndAbort("not positive: " + n)
      case _ => x
  inline def positive(inline x: Int): Int = ${ positiveImpl('x) }
