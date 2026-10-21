// `hasSameKindAs` whole while a binder completes (the review of `4f8b8bb39`, item 1): both `hkResult`s, then the
// results' kinds and the parameters' infos, each `&&` stopping the reads after it (`ShortCircuit`: the bound has no
// result, `S` is never read, the kind error alone); `typedAppliedTypeTree`'s `dealias` of a binder constructor before
// its arguments (`AppBinder`); `adaptType`'s eta-expansion of a bare binder written with a clause of its own
// (`BareBinder`, `LambdaBody`), not of one bounded by a written lambda (`WrittenLambda`: `preCheckKind`'s `isRef` at
// the application); a `_` binder printed `_` (`RefinedPrinter.nameString`).
// expect: 18:25: error: Cyclic reference involving type A
// expect: 19:26: error: Cyclic reference involving type A
// expect: 20:28: error: type argument [X] =>> S does not have the same kind as its bound
// expect: 21:32: error: Cyclic reference involving type F
// expect: 22:25: error: Cyclic reference involving type F
// expect: 23:33: error: Cyclic reference involving type F
// expect: 24:24: error: Cyclic reference involving type F
// expect: 25:28: error: illegal cyclic type reference: upper bound A of type _ refers back to the type itself
// absent: Cyclic reference involving type S
import scala.annotation.StaticAnnotation
class ann[F[_]] extends StaticAnnotation
type BinderHKResult = [@ann[[X] =>> A] A] =>> Int
type ResultParamInfo = [@ann3[[G[_]] =>> A] A] =>> Int
type ShortCircuit = [@annP[[X] =>> S] S] =>> Int
type AppBinder = [@ann[[X] =>> F[X]] F[_]] =>> Int
type BareBinder = [@ann[F] F[_]] =>> Int
type LambdaBody = [@ann[[X] =>> F] F[_]] =>> Int
type WrittenLambda = [@ann[F] F <: [X] =>> Any] =>> Int
type ParamInfo = [@ann3[[G[_ <: A]] =>> Int] A] =>> Int
class ann3[F[_[_]]] extends StaticAnnotation
class annP[T] extends StaticAnnotation
