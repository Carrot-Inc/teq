import scala.quoted.*
class X(val v: Int)
object Counter:
  private var n = 0
  inline def next: Int = ${ nextImpl }
  def nextImpl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)
object X:
  transparent inline given x: X = ${ impl }
  def impl(using Quotes): Expr[X] =
    val v = Counter.nextImpl
    '{ new X($v) }
