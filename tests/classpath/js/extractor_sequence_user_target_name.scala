// targets: js interp
// An annotation of the program's own named `targetName` renames nothing: the plan's `lengthCompare` is the member's.
package p2

class targetName(s: String) extends scala.annotation.StaticAnnotation

final class R:
  @targetName("other")
  def lengthCompare(n: Int): Int =
    println("compare")
    1 - n
  def length: Int =
    println("length")
    1
  def apply(i: Int): Int = 7
  def drop(n: Int): Seq[Int] = Nil
  def toSeq: Seq[Int] = List(7)

object E:
  def unapplySeq(x: Int): R = new R

@main def run(): Unit =
  0 match
    case E(x) => println(x)
    case _ => println(-1)
