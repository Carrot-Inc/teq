import scala.quoted.*

trait X:
  def v: Int

object Counter:
  private var n = 0
  inline def next: Int = ${ nextImpl }
  def nextImpl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)

object X:
  transparent inline given x: X = new X { def v: Int = Counter.next }
