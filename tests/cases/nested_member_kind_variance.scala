// The review of `82464a1b7`'s items 2 and 3, accepted as scalac 3.8.4 accepts them: a member declared without a lambda
// bound has its declared parameters' kinds (`N#F` for `F[G[_]]`), and an indirectly bounded constructor's arguments are
// at its parameters' variances (`AppliedType.tyconTypeParams` through `O.G <: P#F`, and `G <: F` for `F[+X]`).
class hkAnn[F[G[_]]] extends scala.annotation.StaticAnnotation
class AnnFG[F[G[_]]]
trait N { type F[G[_]] }
type NestedMemberControl = [@hkAnn[N#F] A] =>> Int
type NestedMemberApplied = AnnFG[N#F]
trait P { type F[+X] }
object O { type G <: P#F }
type IndirectVariance = [B >: O.G[List[B]]] =>> Int
type ParamIndirectVariance = [F[+X], G <: F, B >: G[List[B]]] =>> Int
@main def run(): Unit =
  val n: NestedMemberControl[String] = 1
  val v: IndirectVariance[Any] = 2
  val p: ParamIndirectVariance[List, List, Any] = 3
  println(s"${AnnFG[N#F]().getClass.getSimpleName} $n $v $p")
