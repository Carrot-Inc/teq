// targets: js interp
// A member `@targetName` renames is not a sequence pattern's `lengthCompare`: the plan tests `length`.
package p

import scala.annotation.targetName

final class R:
  @targetName("other")
  def lengthCompare(n: Int): Int =
    println("compare")
    -1
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
