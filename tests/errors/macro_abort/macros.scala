import scala.quoted.*

object Macros:
  inline def positive(inline x: Int): Int = ${ positiveImpl('x) }
  def positiveImpl(x: Expr[Int])(using Quotes): Expr[Int] =
    import quotes.reflect.*
    x.value match
      case Some(v) if v > 0 => x
      case Some(v) => report.errorAndAbort("expected a positive number, got " + v)
      case None =>
        report.error("expected a literal")
        x

  inline def twoErrors: Int = ${ twoErrorsImpl }
  def twoErrorsImpl(using Quotes): Expr[Int] =
    quotes.reflect.report.error("first")
    quotes.reflect.report.error("second")
    '{ 1 }
