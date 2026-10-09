import scala.quoted.*

object Counter:
  private var n = 0
  transparent inline def next(): Int = ${ nextImpl }
  def nextImpl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)
