import scala.quoted.*

class Line(val n: Int)
object Line:
  inline given here: Line = ${ impl }
  def impl(using Quotes): Expr[Line] =
    import quotes.reflect.*
    val n = Position.ofMacroExpansion.startLine + 1
    '{ new Line(${ Expr(n) }) }
