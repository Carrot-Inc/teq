/*
rules = ["file:../../support/StablePatterns.scala"]
*/
// A stable identifier as a case's pattern is its `Term.Name` or `Term.Select`, a `Pat` as scalameta's.
package driver.stable
object O:
  val v = 1
object Use:
  val w = 2
  def f(x: Int): Int = x match
    case `w` => 1 // assert: StablePatterns.pattern
    case O.v => 2 // assert: StablePatterns.pattern
    case _ => 3 // assert: StablePatterns.pattern
