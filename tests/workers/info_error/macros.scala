import scala.quoted.*

// Macros' runs give info messages and one of them reports an error after its message: the
// messages come before the diagnostics, once, in the order of the sites.
object Checked:
  def check(e: Expr[Int])(using Quotes): Expr[Int] =
    val k = e.valueOrAbort
    quotes.reflect.report.info(s"checking $k")
    if k < 0 then quotes.reflect.report.error(s"negative: $k")
    e

inline def checked(inline k: Int): Int = ${ Checked.check('k) }
