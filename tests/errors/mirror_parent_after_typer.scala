// expect: 18:27: error: type mismatch
// expect: 19:11: error: value fromProduct is not a member of
// expect: 21:13: error: type mismatch
// expect: 22:23: error: type mismatch
// expect: 23:29: error: type mismatch
// expect: 24:11: error: value ordinal is not a member of
// expect: 6 errors found
// A companion, a case object, is its class's mirror from `SyntheticMembers.addMirrorSupport`,
// after typing (`addParent`'s `installAfter`): the typer sees no `Mirror` parent of it, nor the
// `fromProduct` or `ordinal` added beside it.
import scala.deriving.Mirror
case class P(x: Int)
case object O
sealed trait S
object S
case class A(n: Int) extends S
object Use:
  val m: Mirror.Product = P
  val p = P.fromProduct(Tuple1(1))
  def g(o: Mirror.Product) = o
  val q = g(P)
  val s: Mirror.Sum = S
  val o: Mirror.Singleton = O
  val n = S.ordinal(A(1))
