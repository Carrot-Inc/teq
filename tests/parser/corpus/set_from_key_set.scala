// scala-library's `Set.from` tests for `HashMap[E, _]#HashKeySet`, a projection over a type
// variable of the pattern that the case does not name.
@main def run(): Unit =
  val m = Map(1 -> "a", 2 -> "b")
  println(Set.from(m.keySet))
  println(Set.from(List(3, 3, 4)))
