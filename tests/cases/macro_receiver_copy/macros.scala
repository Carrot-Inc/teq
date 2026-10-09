import scala.quoted.*

// A `Quotes` extension called on an `Expr` inside a transparent inline method: the expansion's
// copy keeps the `Quotes` the call reaches the member through.
object CopyReceiver:
  transparent inline def showed(x: Expr[Int])(using q: Quotes): String = x.show
  def call(x: Expr[Int])(using q: Quotes): String = showed(x)

  inline def shown(inline x: Int): String = ${ shownImpl('x) }
  def shownImpl(x: Expr[Int])(using Quotes): Expr[String] = Expr(call(x))
