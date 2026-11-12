// An abstract type member applied through a type parameter's projection,
// compared with the same member applied through the parameter's
// bound: the dependent function's parent is `P => P#FCo[Int]`
// (`nonDependentResultApprox`, the prefix `c` ranged to `Nothing..P`),
// and scalac 3.8.4 accepts `P#FCo[Int] <: A#FCo[Int]` for `P <: A`
// (the same member symbol through a prefix below the other's,
// `TypeComparer`'s `isSubPrefix`), as it accepts the unapplied `P => A#T`.
class A:
  type FCo[+X]
def h[P <: A](f: (c: P) => c.FCo[Int]): P => A#FCo[Int] = f
@main def appliedMemberThroughSubtypePrefix(): Unit = println("ok")
