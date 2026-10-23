/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// Parentheses are no node, but the construct that holds a parenthesized part spans them, as scalameta's: a lambda's
// body, a case's body, a generator's right-hand side and a guard's condition in parentheses.
object Parenthesized:
  val marker = 0 // assert: DumpLayout.layout
  val f: Int => Int = x => (x + 1)
  val g: Int => Int = (x: Int) => ((x))
  val ctx: Int ?=> Int = (x: Int) ?=> x
  def m(n: Int): Int = n match { case _ => (n) }
  def gen = for (x <- (List(1)); if (x > 0)) yield x
