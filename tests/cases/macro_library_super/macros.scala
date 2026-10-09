import scala.quoted.*

// `Ordering.Int.reverse` is scala-library's `CachedReverse`, whose val calls `super.reverse`:
// the trait is typed while the macro runs, and its super call binds then.
object Macros:
  inline def top(inline a: Int, inline b: Int): String = ${ topImpl('a, 'b) }
  def topImpl(a: Expr[Int], b: Expr[Int])(using Quotes): Expr[String] =
    val xs = List(a.valueOrAbort, b.valueOrAbort, 5)
    Expr(xs.sorted(using Ordering.Int.reverse).mkString(",") + " " + xs.max(using Ordering[Int].reverse))
