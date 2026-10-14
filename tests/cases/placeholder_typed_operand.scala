// An ascribed placeholder `(_: T)` as an operand: the enclosing expression binds it, as a bare `_` is bound (dotty's
// `isWildcard`: `_`, `(_: T)` and their parentheses), so `(_: Int) % 2 == 0` is `x => x % 2 == 0` and
// `(_: A) == (_: A)` a function of two parameters.
def same[A]: (A, A) => Boolean = (_: A) == (_: A)

@main def main(): Unit =
  val even = (_: Int) % 2 == 0
  println(List(1, 2, 3, 4).filter(even))
  println(same[String]("a", "a") && !same[Int](1, 2))
  val scaled = List(1, 2, 3).map((_: Int) * 10 + 1)
  println(scaled)
  val f = ((_: String).length)
  println(f("four"))
