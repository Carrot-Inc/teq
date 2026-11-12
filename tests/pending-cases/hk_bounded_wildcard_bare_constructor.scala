// A bounded wildcard of a bare class constructor (`? <: List`, `? >: List <: Seq`)
// as the argument of a higher-kinded parameter bounded by one (`F[_] <: Seq`):
// `preCheckKinds` asks the wildcard's `TypeBounds`, whose ends are proper
// types, against the bound `[_] =>> Seq`: scalac 3.8.4's kind error
// ("does not have the same kind as its bound"), the unbounded `?` exempt.
trait D[F[_] <: Seq]
type W1 = D[? <: List]
type W2 = D[?]
type W3 = D[? >: List <: Seq]
