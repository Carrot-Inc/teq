// An alias whose right side applies a class whose parameter's bound applies the alias (`class C[A <: L[A]]; type L[X]
// = C[X]`): scalac 3.8.4's PostTyper checks `C[X]` at the alias's right side against the parameter's bound seen from
// it, `X <: L[X]`, that is `X <: C[X]`, and rejects it (E057). teq resolves the alias's right side while the
// parameter's bound is being completed and checks it against no bound; with the bound written directly (`A <: C[A]`)
// or through another class's alias (`A <: B[A]`, `type B[X] = List[X]`) it rejects as scalac does. The closure
// adversary of the pass on 5bd85bcd1 (round 23); the round's base the same.
class C[A <: L[A]]
type L[X] = C[X]
