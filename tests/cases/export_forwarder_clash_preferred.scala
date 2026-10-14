// Two export forwarders of one name that erase alike (`concat` of `IOps` and of `SOps`, both returning `S[Int]` once
// `S`'s base type `IOps[A, S[A]]` is the meet of its parents'): the one overloading resolution prefers is kept, the
// other dropped, as dotty's `avoidClashes` does (Namer.scala, pos/i14966), so that the export is an import whose use
// site's reference would resolve.
trait I[+A] extends IOps[A, I[A]]

trait S[A] extends I[A], SOps[A, S[A]]

trait IOps[+A, +C <: I[A]]:
  def concat[B >: A](other: IterableOnce[B]): C

trait SOps[A, +C <: S[A]] extends IOps[A, C]:
  def concat(other: IterableOnce[A]): C

trait IOpsImpl[+A, +C <: I[A]] extends IOps[A, C]:
  def concat[B >: A](other: IterableOnce[B]): C = widened
  def widened: C

trait SOpsImpl[A, +C <: S[A]] extends SOps[A, C]:
  def concat(other: IterableOnce[A]): C = appended(other.iterator.toList)
  def appended(more: List[A]): C

class Items(val xs: List[Int]) extends S[Int], IOpsImpl[Int, S[Int]], SOpsImpl[Int, S[Int]]:
  def widened: S[Int] = new Items(xs :+ -1)
  def appended(more: List[Int]): S[Int] = new Items(xs ++ more)
  override def toString = xs.mkString("Items(", ", ", ")")

class Test(s: S[Int]):
  export s.*

@main def main(): Unit =
  val t = new Test(new Items(List(1)))
  println(t.concat(List(2, 3)))
