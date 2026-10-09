import scala.quoted.*

// A counter a macro steps: how many times an argument's typing expands it.
object Counter:
  var n = 0
  def impl(using Quotes): Expr[String] =
    n += 1
    Expr(n.toString)

inline def next: String = ${ Counter.impl }
