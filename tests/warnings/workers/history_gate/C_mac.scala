import scala.quoted.*
object Mac {
  transparent inline def twice(inline n: Int): Int = ${ twiceImpl('n) }
  def twiceImpl(n: Expr[Int])(using Quotes): Expr[Int] = '{ $n * 2 }
}
