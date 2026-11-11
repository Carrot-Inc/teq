// A qualified private member is a sequence pattern's `lengthCompare`: `private[p]` is no `Private` to the lookup.
package p

final class R:
  private[p] def lengthCompare(n: Int): Int =
    println("compare")
    1 - n
  def length: Int =
    println("length")
    1
  def apply(i: Int): Int = 42
  def drop(n: Int): Seq[Int] = Nil
  def toSeq: Seq[Int] = List(42)

object E:
  def unapplySeq(x: Int): R = new R

@main def run(): Unit =
  0 match
    case E(x) => println(x)
    case _ => println(-1)
