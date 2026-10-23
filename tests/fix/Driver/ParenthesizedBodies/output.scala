// A lambda's, a case's, a generator's and a guard's parenthesized part is within the construct, as scalameta's
// spans: each replaced whole leaves no parenthesis behind and the file types.
object Bodies:
  val f: Int => Int = identity[Int]
  def m(n: Int): Int = n match { case _ => 0 }
  def gen: List[Int] = for (x <- 2 :: Nil; if x > 1) yield x
