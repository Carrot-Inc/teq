import scala.quoted.*

class X(val n: Int)

object M:
  private var n = 0
  inline def next: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)

object X:
  inline given x: X = new X(M.next)
