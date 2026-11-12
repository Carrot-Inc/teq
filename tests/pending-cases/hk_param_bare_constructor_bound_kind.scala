// A higher-kinded parameter bounded by a bare constructor (`F[_] <: List`):
// dotty's bound is the lambda over the constructor itself,
// `[_] =>> List`, whose result is higher-kinded, so `List`, `Vector`
// and `Seq` do not have its kind and scalac 3.8.4's preCheckKinds
// rejects each argument (Checking.scala 189-200); a lower bound alone
// (`F[_] >: List`) is checked against `Any`'s kind and accepted by scalac.
trait C[F[_] >: List]
type Ok = C[List]
trait D[F[_] <: List]
type Ok2 = D[List]
trait E[F[_] <: Seq]
type Ok3 = E[List]
type Ok4 = E[Vector]
trait G[F[_] >: List <: Seq]
type Ok5 = G[Seq]
type Ok6 = G[List]
