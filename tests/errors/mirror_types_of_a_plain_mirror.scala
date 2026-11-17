// expect: 9:32: error: type MirroredElemTypes is not a member of (m : Mirror)
// expect: 10:40: error: type MirroredType is not a member of (m : Product)
// expect: 2 errors found
// `MirroredElemTypes` and `MirroredType` are members of a mirror's refined type (`Mirror.Of[T]`)
// and of `Mirror.Singleton`, not of `Mirror` or `Mirror.Product`: selected on a plain mirror
// they are not found, while `MirroredLabel`, which `Mirror` declares, is.
import scala.deriving.Mirror
object Main {
  def elems(m: Mirror): Option[m.MirroredElemTypes] = None
  def whole(m: Mirror.Product): Option[m.MirroredType] = None
  def label(m: Mirror): Option[m.MirroredLabel] = None
  def typed[T](m: Mirror.Of[T]): Option[m.MirroredElemTypes] = None
  def main(args: Array[String]): Unit = println(label(summon[Mirror.Of[Option[Int]]]))
}
