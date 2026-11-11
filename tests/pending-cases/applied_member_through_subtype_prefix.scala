// An abstract type member applied through a type parameter's projection, compared with the same member applied through
// the parameter's bound (the closure adversary of the pass on 861d8acf1, round 22, found on A.8's source attempts):
// the dependent function's parent is `P => P#FCo[Int]` in both compilers (`nonDependentResultApprox`, the prefix
// `c` ranged to `Nothing..P`), and scalac 3.8.4 accepts `P#FCo[Int] <: A#FCo[Int]` for `P <: A` (the same member
// symbol through a prefix below the other's, `TypeComparer`'s `isSubPrefix`), as it accepts the unapplied
// `P => A#T` (both compilers). teq rejects the applied one: type mismatch: found (f : (c: P) => c.FCo[Int]),
// required P => A#FCo[Int].
class A:
  type FCo[+X]
def h[P <: A](f: (c: P) => c.FCo[Int]): P => A#FCo[Int] = f
@main def appliedMemberThroughSubtypePrefix(): Unit = println("ok")
