// `tyconTypeParams` of an applied member through a refinement's member and a bounded path (the review of `9e9b35b3d`,
// item 3): an invariant member's argument in a lower bound is the cycle, E140 as scalac 3.8.4 gives it (the covariant
// ones accepted, `tests/cases/indirect_param_kind_variance.scala`; teq prints the refinement for the alias `Q`).
// expect: 8:32: error: illegal cyclic type reference: lower bound
// expect: 12:32: error: illegal cyclic type reference: lower bound C.this.q.F[List[B]] of type B refers back to the type itself
trait P
type Q = P { type F[X] }
type RefinedVarianceControl = [B >: Q#F[List[B]]] =>> Int
trait P2 { type F[X] }
class C[Q2 <: P2] {
  val q: Q2 = ???
  type ParamVarianceControl = [B >: q.F[List[B]]] =>> Int
}
