// An array's `lastIndexWhere`, up to an end or over all of it, and `startsWith` of a sequence,
// at the start or at an offset.
@main def run =
  val xs = Array(1, 5, 2, 5, 3)
  println(xs.lastIndexWhere(_ == 5))
  println(xs.lastIndexWhere(_ == 5, 2))
  println(xs.lastIndexWhere(_ > 10))
  val parts = "a.b.c".split('.')
  println(parts.startsWith(List("a", "b")))
  println(parts.startsWith(List("b", "c"), 1))
  println(parts.startsWith(List("a", "c")))
  println(parts.startsWith(Nil, 3))
