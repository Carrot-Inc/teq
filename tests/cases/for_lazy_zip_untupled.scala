// A generator's tuple pattern against a function of as many parameters (LazyZip2's foreach and map)
// takes them as parameters, as dotty's typedFunctionValue turns the synthetic match lambda into one.
@main def main(): Unit =
  for (a, b) <- List(1, 2).lazyZip(List("x", "y")) do println(s"$a$b")
  val r = for (a, b) <- List(1, 2).lazyZip(List("x", "y")) yield s"$b$a"
  println(r)
  List(3, 4).lazyZip(List("p", "q")).foreach { case (n, s) => println(s * n) }
