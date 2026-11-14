// The classes of early_param_kind's other file, each with a higher-kinded parameter that has a written bound or
// a clause whose own parameter has one.
package earlykind

class Base
class Sub extends Base
class LUpper[F[G[_]] <: AnyRef] { def value = 61 }
class LBoth[F[G[_]] >: Nothing <: AnyRef] { def value = 63 }
trait LTrait[F[G[_]] <: AnyRef] { def value = 64 }
class LTwo[F[G[_], A] <: AnyRef] { def value = 65 }
class LBase[F[G[_]] <: Base] { def value = 66 }
class LSelf[F[G[_]] <: LSelf[F]] { def value = 71 }
class LVar[F[+G[_]] <: AnyRef] { def value = 72 }
class LInner[F[G[X <: Base]]] { def value = 86 }
class LInnerB[F[G[X <: Base]] <: AnyRef] { def value = 87 }
class LWider[F[G[X <: Base]]] { def value = 134 }
class LNested[F[G[H[_]]] <: AnyRef] { def value = 171 }
trait TC[F[G[_]]]
class LCtx[F[G[_]] <: AnyRef : TC] { def value = 174 }
class LLam[F[G[_]] <: AnyRef] { def value = 183 }
class LAli[F[G[_]] <: AnyRef] { def value = 184 }
class LSib[B, F[G[_]] <: B] { def value = 141 }
class LSibIn[B, F[G[X <: B]]] { def value = 135 }
class LLoHi[F[G[X >: Lo <: Hi]]] { def value = 137 }
enum LEnum[F[G[_]] <: AnyRef] { case One() }
class LOut[F[G[_]] <: AnyRef] { def value = 318 }
class LParam[F[G[_]] <: AnyRef] { def value = 111 }
class LNoBound[F[G[_]]] { def value = 113 }
type Bd[F[_[_]]] = AnyRef
class LAliasB[F[G[_]] <: Bd[F]] { def value = 173 }
trait One[F[G[_]] <: Two[KO]] { def value = 182 }
trait Two[F[G[_]] <: One[KO]]
class KO[F[_]] extends One[KO] with Two[KO]
class LCtor[F[G[_]] <: AnyRef] { def value = 319 }
class LSibB[F[G[_]] <: AnyRef] { def value = 320 }
class LLower[F[G[_]] >: Nothing] { def value = 62 }
class LFirst[F[X] <: Iterable[X]] { def value = 69 }
class LSig[F[G[_]] <: AnyRef] { def value = 70 }
