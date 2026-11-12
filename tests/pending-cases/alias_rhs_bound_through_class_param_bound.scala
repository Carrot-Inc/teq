// An alias whose right side applies a class whose
// parameter's bound applies the alias (`class C[A <:
// L[A]]; type L[X] = C[X]`): scalac 3.8.4's PostTyper
// checks `C[X]` at the alias's right side against
// the parameter's bound seen from it, `X <: L[X]`,
// that is `X <: C[X]`, and rejects it (E057).
class C[A <: L[A]]
type L[X] = C[X]
