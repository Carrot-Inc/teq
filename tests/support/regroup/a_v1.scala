// Version 1 of a library whose method takes its implicit arguments in a clause of two, as munit 1.0's
// `assertEquals(obtained, expected, clue)(loc, compare)` did; tests/support/regroup/b.scala is compiled
// against it.
package rga

final case class Loc(line: Int)
trait Compare[A, B]:
  def same(a: A, b: B): Boolean
object Compare:
  given Compare[Int, Int] = (a, b) => a == b

object Assertions:
  def check[A, B](obtained: A, expected: B, clue: => Any = "")(implicit loc: Loc, compare: Compare[A, B]): String =
    s"v1 ${compare.same(obtained, expected)} at ${loc.line} $clue"
