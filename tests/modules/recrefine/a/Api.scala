package rra

// A refinement whose members name each other (dotty's `RecType`): `U` is the refined type's own
// `T`, written as `RECtype`/`RECthis`, and a higher-kinded member keeps its variance.
object Api:
  type R = AnyRef { type T; type U = T }
  type K = AnyRef { type F[+A]; type G = F[Int] }

  class Box(val value: String):
    type T = String
    type U = String

  def make(s: String): R = Box(s)
  // An inferred result scalac pickles as `RECtype`, its self references `RECthis`.
  def inferred(s: String) =
    val r: AnyRef { type T = String; type U <: T } = Box(s)
    r
  def same(r: R)(x: r.T): r.U = x
