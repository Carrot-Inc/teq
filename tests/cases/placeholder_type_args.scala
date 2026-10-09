// A placeholder argument of a method under explicit type arguments takes its type from the
// instantiated method: `identity[Int](_)` is an `Int => Int`.
def pair[A, B](a: A, b: B): (A, B) = (a, b)

@main def Main(): Unit =
  val l1 = List(Predef.identity[Int](_))
  val lc1: List[Int => Int] = l1
  println(lc1.map(_(3)))
  val f = pair[String, Int]("k", _)
  println(f(2))
  val g = pair[List[Int], Int](_, 1)
  println(g(List(1, 2)).toString)
