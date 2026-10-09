import scala.quoted.*
object PowerMacro:
  def powerCode(x: Expr[Int], n: Expr[Long])(using Quotes): Expr[Int] = powerCode(x, n.valueOrAbort)
  def powerCode(x: Expr[Int], n: Long)(using Quotes): Expr[Int] =
    if n == 0 then '{ 1 }
    else if n % 2 == 0 then '{ val y = $x * $x; ${ powerCode('y, n / 2) } }
    else '{ $x * ${ powerCode(x, n - 1) } }
class Num(x: Int):
  inline def power(inline n: Long): Int = ${ PowerMacro.powerCode('x, 'n) }
object Two:
  val x: Int = 2
  inline def power(inline n: Long): Int = ${ PowerMacro.powerCode('x, 'n) }
