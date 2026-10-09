import scala.quoted.*

class R(val v: Int)

object Count:
  var n = 0
  inline def runs: Int = ${ runsImpl }
  def runsImpl(using Quotes): Expr[Int] = Expr(n)

trait Low:
  given fallback: R = R(0)

object R extends Low:
  transparent inline given viaMacro: R = ${ impl }
  def impl(using Quotes): Expr[R] =
    Count.n += 1
    quotes.reflect.report.errorAndAbort("no R from the macro")
