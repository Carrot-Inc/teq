// The jar side of tests/cases/nested_companion_lib, compiled by scalac: case classes nested in
// traits whose companions are applied in the traits' own bodies and inside the companions
// themselves (chimney's `TransformerFlags` in `Configurations`, its `ForTotal.create` in
// `Contexts`, its `TransformerOverride.Fallback` reached through a self-typed cake), with defaults
// and a second clause, read from scalac's TASTy and run both by a macro at compile time and at
// run time.
package nclib

import scala.quoted.*

trait Exprs:
  type Expr[A] = List[A]

trait Configurations:
  this: Exprs =>
  case class Flags(depth: Int = 1, strict: Boolean = false):
    def deeper: Flags = copy(depth = depth + 1)
  object Flags:
    val global: Flags = Flags()
    def strictly(n: Int): Flags = Flags(n, strict = true)
  def withDepth(n: Int): Flags = Flags(depth = n)
  object TransformerOverride:
    case class Fallback(runtimeData: Expr[Int])

trait Contexts:
  object TransformationContext:
    case class ForTotal[A](src: A)(val journal: List[String]):
      def describe: String = s"$src:${journal.mkString(",")}"
    object ForTotal:
      def create[A](src: A): ForTotal[A] = ForTotal[A](src = src)(journal = List("start"))
  def total[A](src: A): TransformationContext.ForTotal[A] = TransformationContext.ForTotal.create(src)

trait Rules:
  this: Exprs & Configurations =>
  def fallback(e: Expr[Int]): TransformerOverride.Fallback = TransformerOverride.Fallback(e)

object Derivation extends Exprs, Configurations, Contexts, Rules:
  def run(n: Int): String =
    val f = withDepth(n).deeper
    val g = Flags.global
    val s = Flags.strictly(n)
    total(f.depth + g.depth).describe + s" ${f.strict} ${s.strict} ${fallback(List(n)).runtimeData}"

object Macros:
  inline def derive(inline n: Int): String = ${ deriveImpl('n) }
  def deriveImpl(n: Expr[Int])(using Quotes): Expr[String] =
    Expr(Derivation.run(n.valueOrAbort))
