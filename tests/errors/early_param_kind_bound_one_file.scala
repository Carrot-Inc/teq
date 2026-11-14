// A higher-kinded parameter's written bound, in one file, where the class is applied before the bound is resolved:
// in the class's own constructor, before the class's definition, and in a block before the class's definition; each
// is refused at its argument with the bound's sentence, once, as the applications after the class's definition
// below them are: a first-order clause's bound, a bound that names a later parameter, an F-bound, a bound through
// an alias, the class in its own body, a value's type and a `new`. scalac 3.8.4 refuses each (E057).
// expect: 20:59: error: type argument Option does not conform to upper bound [X] =>> Iterable[X]
// expect: 21:28: error: type argument K does not conform to upper bound [G[_]] =>> Base
// expect: 24:32: error: type argument Option does not conform to upper bound [X] =>> Iterable[X]
// expect: 29:34: error: type argument Option does not conform to upper bound [X] =>> Iterable[X]
// expect: 31:40: error: type argument Option does not conform to upper bound [X] =>> Seq[X]
// expect: 34:27: error: type argument Bag does not conform to upper bound [X] =>> Coll[Bag]
// expect: 37:44: error: type argument Option does not conform to upper bound [X] =>> Iterable[X]
// expect: 38:53: error: type argument Option does not conform to upper bound [X] =>> Iterable[X]
// expect: 39:30: error: type argument Option does not conform to upper bound [X] =>> Iterable[X]
// expect: 39:54: error: type argument Option does not conform to upper bound [X] =>> Iterable[X]
// expect: 10 errors found
// absent: does not have the same kind
class K[F[_]]
class Base
class InCtor[F[X] <: Iterable[X]](val next: Option[InCtor[Option]])
class OfBase extends LBase[K]
class LBase[F[G[_]] <: Base]
def blockBound = {
  class Applied extends BLater[Option]
  class BLater[F[X] <: Iterable[X]] { def value = 301 }
  new Applied().value
}
class Ordered[F[X] <: Iterable[X]]
class OrderedUse extends Ordered[Option]
class LaterParam[F[X] <: G[X], G[_]]
class LaterParamUse extends LaterParam[Option, Seq]
trait Coll[C[X] <: Coll[C]]
class Bag[A]
class BagUse extends Coll[Bag]
type Bnd[X] = Iterable[X]
class ThroughAlias[F[X] <: Bnd[X]]
class ThroughAliasUse extends ThroughAlias[Option]
class InBody[F[X] <: Iterable[X]] { def bad: InBody[Option] = ??? }
object News { val a: Ordered[Option] = null; def b = new Ordered[Option]() }
