// A higher-kinded parameter bounded by a bare constructor (`F[_] <: List`): dotty's bound is the lambda over the
// constructor itself, `[_] =>> List`, whose result is higher-kinded, so `List`, `Vector` and `Seq` do not have its kind
// and scalac 3.8.4's preCheckKinds rejects each argument (Checking.scala 189-200). teq resolves the lambda's body as a
// proper type, `[_] =>> List[<error>]`, and accepts them; a lower bound alone (`F[_] >: List`) is checked against `Any`'s
// kind and accepted by both. Master 54ccfd0ffb and the pass on 97f0487ac the same; round 24's checks of both bounds show
// that body in their messages (`HKLower`: `lower bound [_] =>> List[<error>]`, scalac `[_] =>> List`).
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
