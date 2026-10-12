// teq: --analysis-version 3
// expect: type_params_before_completion_analysis.scala:20:18: error: type mismatch: found "s", required Int
// expect: 1 error found
// absent: Cyclic inheritance
// absent: not found
// A type application of a class makes its type parameters without completing it (Namer's
// TypeParamsCompleter): `X[B[D]]` in `A`'s parents needs `B`'s parameters, not its constructor, whose
// `A.Id` would complete `A` again. A build that records dependencies (the analysis teq's tasks ask
// for) types it as a plain check does; only the line below is in error.
class X[T] { type Id }
object A extends X[B[D]]
class B[C](id: A.Id)
class D

trait Abstract1[C <: Abstract2[C]]
trait Abstract2[C <: Abstract2[C]] extends Abstract1[C]
class Parametrized1[T] extends Abstract1[Parametrized2[T]]
class Parametrized2[T] extends Parametrized1[T] with Abstract2[Parametrized2[T]]

val wrong: Int = "s"
