def pick[T](a: T, b: T): T = b

@main def main(): Unit =
  val total = 23L
  val limit = 5L
  println(Math.min(total % limit, 1))
  println(Math.max(1, total))
  println(total / limit + Math.min(total % limit, 1))
  println(Math.max(2, 3.5))
  println(Math.max(2L, 3.5))
  val p: Int | Long = pick(1, 2L)
  println(p)
  println(pick(1.5, 2))
  println(List(1, 2L, 3).map(_ * 2L))
  println(List(1, 2.5).sum)
  val o: Int | Long = Option(1).getOrElse(2L)
  println(o)
