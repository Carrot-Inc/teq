import scala.quoted.*

class R(val n: Int)
trait LowPrio { given fb: R = new R(0) }
object R extends LowPrio:
  transparent inline given ig: R = ${ impl }
  def impl(using Quotes): Expr[R] =
    import quotes.reflect.*
    if Position.ofMacroExpansion.startLine >= 3 then '{ new R(1) } else report.errorAndAbort("too early")
