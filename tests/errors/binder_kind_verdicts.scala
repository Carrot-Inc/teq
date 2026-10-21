// `Checking.preCheckKind`'s verdict kept while a binder completes, an error tree on false (the review of
// `cf6cad6c1`, item 1): `hasSameKindAs` over the results (`[X] =>> [Y] =>> Int` against `F[_]`) and an abstract
// member's information (`P#F[X]` of a higher-kinded bound); the same outside a completion, teq's kind outline with
// the result's kind. scalac 3.8.4 gives these six errors, a kind error at its lambda's or selection's point where teq
// prints its start; the message is teq's established form (`kinds.scala`).
// expect: 15:31: error: type argument [X] =>> [Y] =>> Int does not have the same kind as its bound [_]
// expect: 16:31: error: type argument [X] =>> P#F[X] does not have the same kind as its bound [_]
// expect: 17:32: error: Cyclic reference involving type A
// expect: 18:26: error: type argument [X] =>> [Y] =>> Int does not have the same kind as its bound [_]
// expect: 19:32: error: type argument [X] =>> P#F[X] does not have the same kind as its bound [_]
// expect: 20:26: error: type argument P#F does not have the same kind as its bound [_]
// absent: binder_kind_verdicts.scala:21:
class ann[F[_]] extends scala.annotation.StaticAnnotation
trait P { type F[X] <: [Y] =>> Any }
type NestedResultKind = [@ann[[X] =>> [Y] =>> Int] A] =>> Int
type KindMemberResult = [@ann[[X] =>> P#F[X]] A] =>> Int
type AppliedMemberKind = [@ann[P#F[A]] A] =>> Int
type GeneralResult = ann[[X] =>> [Y] =>> Int]
type GeneralMemberResult = ann[[X] =>> P#F[X]]
type GeneralMember = ann[P#F]
type Proper = ann[List]
