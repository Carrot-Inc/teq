// jars: scala-library
import scala.quoted.*

// A macro's run makes scala-library's `Ordering.Int`, whose trait `CachedReverse` calls
// `super.reverse`: the trait's body is typed once, under the loader's lock, by the first worker
// whose run needs it, and every worker's interpreter binds the call when it makes the object.
object M:
  def impl(e: Expr[Int])(using Quotes): Expr[Int] =
    val k = e.valueOrAbort
    Expr(List(k, k + 3, k + 1).sorted(using Ordering.Int.reverse).head)

inline def top(inline k: Int): Int = ${ M.impl('k) }
