import scala.quoted.*

object Raised:
  transparent inline def raised(inline n: Int): Int = ${ raisedImpl('n) }
  def raisedImpl(n: Expr[Int])(using Quotes): Expr[Int] =
    val counted = Marks.noted(3)
    n
