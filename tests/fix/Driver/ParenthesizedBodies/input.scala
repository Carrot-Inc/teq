/*
rules = ["file:../../support/ReplaceBodies.scala"]
*/
// A lambda's, a case's, a generator's and a guard's parenthesized part is within the construct, as scalameta's
// spans: each replaced whole leaves no parenthesis behind and the file types.
object Bodies:
  val f: Int => Int = x => (x + 1)
  def m(n: Int): Int = n match { case k if (k > 0) => (k + 1) }
  def gen: List[Int] = for (x <- (List(1)); if (x > 0)) yield x
