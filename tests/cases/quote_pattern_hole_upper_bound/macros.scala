// A quote pattern's hole is typed against its prototype once `isFullyDefined(pt, ForceDegree.flipBottom)`
// holds (dotty's `typedSplicePattern`): a type argument nothing bounds below is its upper bound, so `$v` of
// `id($v)` is an `Expr[String]` where `id[T <: String]`, and an `Expr[Any]` where `T` has none. The pattern's
// `id`, written without a prefix inside `M`, matches the scrutinee's `M.id(..)` by its symbol, as dotty's
// `QuoteMatcher` matches a reference (and a reference without a prefix in the scrutinee a selection).
import scala.quoted.*

object M:
  def id[T <: String](x: T): T = x
  def any[T](x: T): T = x
  inline def unwrap(inline x: Any): String = ${ impl('x) }
  def impl(x: Expr[Any])(using Quotes): Expr[String] = x match
    case '{ id($v) } =>
      val typed: Expr[String] = v
      typed
    case '{ M.any($w) } => Expr("any " + w.show)
    case _ => Expr("no match")
  inline def inside: String = unwrap(any(1))
