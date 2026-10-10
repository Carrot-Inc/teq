import scala.quoted.*

// Two counters of their expansions: a transparent one, expanded where its call is adapted, and a
// plain one, expanded once in the later phase.
object T:
  var n = 0
  transparent inline def next: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)

object P:
  var n = 0
  inline def next: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)
