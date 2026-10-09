// A tree spliced twice into one quote is copied twice: each copy takes the records of the
// original, the call the typer computed naming the copy's own receiver.
import scala.quoted.*

object ReusedExpr:
  transparent inline def twice(t: (Int, String)): Int = ${ twiceImpl('t) }
  def twiceImpl(t: Expr[(Int, String)])(using Quotes): Expr[Int] =
    val e = '{ $t.head }
    '{ $e + $e }
