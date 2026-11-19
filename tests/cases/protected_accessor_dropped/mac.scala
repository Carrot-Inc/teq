package use
import scala.quoted.*
object Macros {
  inline def ignore(inline x: Int): Int = ${ impl('x) }
  def impl(x: Expr[Int])(using Quotes): Expr[Int] = Expr(7)
}
