// A bounded wildcard of a bare class constructor (`? <: List`, `? >: List <: Seq`) as the argument of a
// higher-kinded parameter bounded by one (`F[_] <: Seq`): `preCheckKinds` asks the wildcard's `TypeBounds`, whose
// ends are proper types, against the bound `[_] =>> Seq`: scalac 3.8.4's kind error ("does not have the same kind as
// its bound"), the unbounded `?` exempt. teq accepts both (master the same; lane P's summary,
// `pi-scratch/r24p/own/h05`). The closure adversary of the pass on 97f0487ac (round 24).
trait D[F[_] <: Seq]
type W1 = D[? <: List]
type W2 = D[?]
type W3 = D[? >: List <: Seq]
