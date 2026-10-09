import scala.quoted.*
object M:
  private var n = 0
  transparent inline def next: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)
class R
class S
trait A:
  extension (r: R) def pick: Int = 0
trait B:
  extension (s: S) def pick: Int
class Impl(val n: Int) extends B:
  extension (s: S) def pick: Int = n
object G:
  given a: A = new A {}
  transparent inline given b: B = new Impl(M.next)
