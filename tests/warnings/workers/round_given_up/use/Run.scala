import scala.quoted.*

object Run:
  inline def twice(inline n: Int): Int = ${ twiceImpl('n) }
  inline def viaTwice(n: Int): Int = twice(n) + 1
  def twiceImpl(n: Expr[Int])(using Quotes): Expr[Int] = '{ $n * 2 }
