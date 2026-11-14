// A class whose higher-kinded parameter has a written bound, first applied in the file typed before the class's, to
// an argument of the parameter's kind that is not within the bound: the bound is read once it is resolved, and each
// application is refused at its argument, once, with the bound's sentence and not the kind's: a first-order clause's
// bound, one whose member the class's body calls on a value of the parameter's type, a class of the program, an alias
// that is no reference type, the class itself read with the application's arguments. scalac 3.8.4 refuses each
// (E057, at the parent and at the argument).
// expect: A.scala:19:28: error: type argument Option does not conform to upper bound [X] =>> Iterable[X]
// expect: A.scala:20:27: error: type argument KA does not conform to upper bound [X] =>> BaseOf[X]
// expect: A.scala:21:28: error: type argument K does not conform to upper bound [G[_]] =>> Base
// expect: A.scala:22:28: error: type argument [F[_]] =>> KI[F] does not conform to upper bound [G[_]] =>> AnyRef
// expect: A.scala:23:28: error: type argument K does not conform to upper bound [G[_]] =>> LSelf[K]
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
