import scala.quoted.*
object M:
  private var n = 0
  transparent inline def next: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)
class R
trait Ops:
  def k: Int
  extension (r: R) def pick: Int = k
object Outer:
  given outerOps: Ops with
    def k: Int = -1
