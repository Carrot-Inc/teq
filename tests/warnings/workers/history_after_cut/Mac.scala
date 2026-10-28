import scala.quoted.*
object Mac:
  transparent inline def msg(inline n: Int): String = ${ msgImpl('n) }
  def msgImpl(n: Expr[Int])(using Quotes): Expr[String] = '{ "m" + $n }
