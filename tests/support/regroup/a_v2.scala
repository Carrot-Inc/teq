// Version 2 of tests/support/regroup/a_v1.scala: the method takes a third implicit argument, and the
// method with version 1's erasure stays for binary compatibility, its arguments in one clause, as munit
// 1.2's deprecated `assertEquals(obtained, expected, clue, loc, compare)`.
package rga

final case class Loc(line: Int)
final case class Options(verbose: Boolean)
object Options:
  given Options = Options(false)
trait Compare[A, B]:
  def same(a: A, b: B): Boolean
object Compare:
  given Compare[Int, Int] = (a, b) => a == b

object Assertions:
  @deprecated("binary compatibility", "2")
  def check[A, B](obtained: A, expected: B, clue: => Any, loc: Loc, compare: Compare[A, B]): String =
    check(obtained, expected, clue)(using loc, compare, Options(false))
  def check[A, B](obtained: A, expected: B, clue: => Any = "")(implicit loc: Loc, compare: Compare[A, B], options: Options): String =
    s"v2 ${compare.same(obtained, expected)} at ${loc.line} $clue ${options.verbose}"
