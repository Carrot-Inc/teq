// expect: 12:18: error: Refinements cannot introduce overloaded definitions
// expect: 13:18: error: Refinements cannot introduce overloaded definitions
// absent: refinement_generic_erasure.scala:14:
// absent: refinement_generic_erasure.scala:16:
// A generic refinement whose type parameter's bound erases otherwise than the parent member's
// has another signature, so it overloads the member rather than refining it (dotty's
// `Denotation.matchesLoosely` by `Signature.matchDegree`, `Typer.typedRefinedTypeTree`); one
// whose bound erases alike refines it (`A & Z` erases as `A`), the member's bound seen from the
// refined type (`asSeenFrom`: `run[A <: T]` of `Parent[String]` erases `A` as `String`).
trait A; trait Z
trait B { def f[T <: String](x: T): Any; def g[T <: A & Z](x: T): Any }
type R = B { def f[T](x: T): String }
type S = B { def f[T <: AnyRef](x: T): String }
type U = B { def g[T <: A](x: T): String }
trait Parent[T] { def run[A <: T](a: A): Any }
type V = Parent[String] { def run[A <: String](a: A): A }
