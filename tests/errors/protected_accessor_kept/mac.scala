package use
import scala.quoted.*
object Macros {
  inline def keep(inline x: Int): Int = ${ impl('x) }
  def impl(x: Expr[Int])(using Quotes): Expr[Int] = x
}
