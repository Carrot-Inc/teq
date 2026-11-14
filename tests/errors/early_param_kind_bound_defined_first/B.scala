// early_param_kind_bound's two files with their names exchanged: the classes are typed first. The same five
// applications are refused at their arguments with the same sentences.
// expect: B.scala:15:28: error: type argument Option does not conform to upper bound [X] =>> Iterable[X]
// expect: B.scala:16:27: error: type argument KA does not conform to upper bound [X] =>> BaseOf[X]
// expect: B.scala:17:28: error: type argument K does not conform to upper bound [G[_]] =>> Base
// expect: B.scala:18:28: error: type argument [F[_]] =>> KI[F] does not conform to upper bound [G[_]] =>> AnyRef
// expect: B.scala:19:28: error: type argument K does not conform to upper bound [G[_]] =>> LSelf[K]
// expect: 5 errors found
// absent: does not have the same kind
package earlykindbound

class K[F[_]]
class KA[A]
type KI[F[_]] = Int
class First extends LFirst[Option]
class Reads extends LRead[KA] { def go: Int = read(new KA[Int]) }
class OfBase extends LBase[K]
class OfAlias extends LAny[KI]
class OfSelf extends LSelf[K]
