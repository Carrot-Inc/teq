import scala.quoted.*

// The macro's implementation runs a library method whose body returns early; the body is its
// own, not part of the inline expansion that runs the macro.
object Macros:
  inline def firstAbove(inline limit: Int): Int = ${ firstAboveImpl('limit) }
  def firstAboveImpl(limit: Expr[Int])(using Quotes): Expr[Int] =
    val bound = limit.valueOrAbort
    val found = IArray(1, 5, 9).find(_ > bound)
    Expr(found.getOrElse(-1))
