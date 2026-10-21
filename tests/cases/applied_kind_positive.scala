// The review of `bb04d0aaf`, item 1, accepted as scalac 3.8.4 accepts it: an application of a constructor bounded by
// another (`O.G <: P#F`, `G <: F`) has the kind of that bound applied (`AppliedType.superType`'s `applyIfParameterized`),
// the helper's own query no cycle of the reference; a constructor's type argument checked by `preCheckKind`.
class hk[F[_]] extends scala.annotation.StaticAnnotation
class Ann[F[_]]
trait P { type F[X] <: [Y] =>> Any }
trait Q { type F[X] <: [Y] =>> X }
object O { type G <: P#F }
object R { type G <: Q#F }
type AppliedKindPositive = [@hk[O.G[Int]] A] =>> Int
type AppliedParamPositive = [F[X] <: [Y] =>> Any, G <: F, @hk[G[Int]] A] =>> Int
type AppliedBoundUsed = [@hk[R.G[Int]] A] =>> Int

@main def run(): Unit =
  val k: AppliedKindPositive[String] = 1
  val u: AppliedBoundUsed[String] = 3
  println(s"${Ann[O.G[Int]]().getClass.getSimpleName} $k $u")
