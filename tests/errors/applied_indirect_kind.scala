// `typeParams` and `hkResult` answering parameter infos (the review of `82464a1b7`, items 1-2): an application of a
// member or parameter bounded by another constructor has its `superType` applied (`AppliedType.superType`), a member
// declared without a lambda bound its declared parameters with their own kinds, and an invariant indirect constructor's
// argument in a lower bound is the cycle, as scalac 3.8.4 refuses each (teq's established kind message; an object's
// member printed without its prefix).
// expect: 18:34: error: type argument G[Int] does not have the same kind as its bound
// expect: 19:67: error: type argument G[Int] does not have the same kind as its bound
// expect: 20:32: error: Missing type parameter for G[Int]
// expect: 22:37: error: type argument N#F does not have the same kind as its bound [_]
// expect: 23:30: error: type argument N#F does not have the same kind as its bound [_]
// expect: 26:27: error: illegal cyclic type reference: lower bound G[List[B]] of type B refers back to the type itself
class ann[T] extends scala.annotation.StaticAnnotation
class annF[F[_]] extends scala.annotation.StaticAnnotation
class Ann[T]
class AnnF[F[_]]
trait P { type F[X] <: [Y] =>> Any }
object O { type G <: P#F }
type IndirectAppliedKind = [@ann[O.G[Int]] A] =>> Int
type IndirectAppliedParam = [F[X] <: [Y] =>> Any, G <: F] =>> Ann[G[Int]]
type AppliedSimple = [@ann[(x: O.G[Int]) => Int] A] =>> Int
trait N { type F[G[_]] }
type NestedMemberAnnotated = [@annF[N#F] A] =>> Int
type NestedMemberKind = AnnF[N#F]
trait V { type F[X] }
object W { type G <: V#F }
type VarianceInvariant = [B >: W.G[List[B]]] =>> Int
