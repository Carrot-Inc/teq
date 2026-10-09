// A plain inline member whose application fails gives way to an extension (the retry on the
// qualifier): the member's call, pending for the later expansion phase, goes with its application,
// set aside by the retry, so its macro does not run (scalac `40`, `1`).
extension (c: C) def f(x: Int): Int = x * 10

@main def run(): Unit =
  println(C().f(4))
  println(Counter.next)
