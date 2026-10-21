// `hkResult` and `typeParams` through an indirect bound (the review of `9e9b35b3d`, item 2): an object's member bounded by a
// higher-kinded member (`O.G <: P#F`) has its kind, in a binder's completion and outside it, as scalac 3.8.4 rejects it
// (teq's established kind message; an object's member printed without its prefix).
// expect: 13:27: error: type argument G does not have the same kind as its bound
// expect: 14:38: error: type argument G does not have the same kind as its bound [_]
// expect: 15:24: error: type argument G does not have the same kind as its bound [_]
class ann[T] extends scala.annotation.StaticAnnotation
class annF[F[_]] extends scala.annotation.StaticAnnotation
trait P { type F[X] }
trait R { type F[X] <: [Y] =>> Any }
object O { type G <: P#F }
object S { type G <: R#F }
type IndirectKind = [@ann[O.G] A] =>> Int
type KindBoundsInCompletion = [@annF[S.G] A] =>> Int
type KindBounds = annF[S.G]
