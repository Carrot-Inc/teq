// A counter macro's runs in the order of the tree the inlining phase walks (`InliningTreeMap`):
// the method's call before the class statement after it, where an order by the members' places
// put a class's statements first (scalac prints 2 then 1).
import scala.quoted.*
object M:
  var n = 0
  inline def next: Int = ${impl}
  def impl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)
