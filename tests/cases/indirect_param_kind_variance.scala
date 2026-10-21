// The review of `9e9b35b3d`'s items 2 and 3, accepted as scalac 3.8.4 accepts them: a parameter bounded by a
// higher-kinded one (`G <: F`) has `F`'s kind (`hkResult` through its bound, never an arity); an applied member's
// arguments at its parameters' variances through a refinement's member (`P { type F[+X] }`) and a bounded path
// (`q.F` for `q: Q2 <: P`).
class Ann[F[_]]
type L = [F[_], G <: F] =>> Ann[G]
trait P { type F[+X] }
type Q = P { type F[+X] }
type RefinedVariance = [B >: Q#F[List[B]]] =>> Int
class C[Q2 <: P] {
  val q: Q2 = ???
  type ParamVariance = [B >: q.F[List[B]]] =>> Int
}
@main def run(): Unit =
  val a: L[List, List] = Ann[List]()
  val r: RefinedVariance[Any] = 1
  println(s"${a.getClass.getSimpleName} $r")
