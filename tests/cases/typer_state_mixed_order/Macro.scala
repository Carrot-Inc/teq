import scala.quoted.*
object Counter:
  private var n = 0
  inline def plain: Int = ${ nextImpl }
  transparent inline def trans: Int = ${ nextImpl }
  def nextImpl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)
