// The classes of the case's other file.
package earlykindbound

class Base
class BaseOf[A] { def value: Int = 400 }
class LFirst[F[X] <: Iterable[X]]
class LRead[F[X] <: BaseOf[X]] { def read(x: F[Int]): Int = x.value }
class LBase[F[G[_]] <: Base]
class LAny[F[G[_]] <: AnyRef]
class LSelf[F[G[_]] <: LSelf[F]]
