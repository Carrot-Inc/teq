import scala.quoted.*

// A macro that counts its expansions: the member's retry on the qualifier adapts the argument as
// the member's application typed it, so the macro runs once whatever the retry's outcome.
object Counter:
  var n = 0
  def impl(using Quotes): Expr[String] =
    n += 1
    Expr(n.toString)
  inline def next: String = ${ impl }
