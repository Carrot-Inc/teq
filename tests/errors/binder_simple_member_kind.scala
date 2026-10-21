// `hasSimpleKind` whole while a binder completes (the review of `9e9b35b3d`, item 1): `typeParams`, then `hasAnyKind`, then
// the dealiased type, a member's through its information: an unapplied higher-kinded member (`P#F`, `O.F`) as a named
// function type's parameter or result is E056 at its tree, as scalac 3.8.4 gives it (an object's member printed
// without its prefix, `proposed-errors/object_member_alias_prefix.scala`).
// expect: 11:21: error: Missing type parameter for P#F
// expect: 12:29: error: Missing type parameter for P#F
// expect: 13:21: error: Missing type parameter for
class ann[T] extends scala.annotation.StaticAnnotation
trait P { type F[X] }
object O { type F[X] }
type L1 = [@ann[(x: P#F) => Int] A] =>> Int
type L2 = [@ann[(x: Int) => P#F] A] =>> Int
type L3 = [@ann[(x: O.F) => Int] A] =>> Int
