// The interpreter runs matches an inline expansion's walk settled in place (a type test the literal's type makes
// redundant), inside a macro's implementation run twice: its plans are made at their first use, after the walk,
// and held across the runs.
import scala.quoted.*

inline def kind(inline x: Any): Int = x match
  case _: String => 1
  case _: Int => 2
  case _ => 0

object M:
  inline def count(inline tag: String): Int = ${ countImpl('tag) }
  def countImpl(tag: Expr[String])(using Quotes): Expr[Int] =
    val n = kind("s") + kind(3) + kind(2.0) + tag.valueOrAbort.length
    Expr(n)
