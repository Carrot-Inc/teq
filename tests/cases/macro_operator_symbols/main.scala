@main def run(): Unit =
  val x = 1
  val y = 2
  val l = 3L
  println(Ops.same(x + y, y + x))
  println(Ops.same(x + y, 4 + x))
  println(Ops.same(x + y, x - y))
  println(Ops.same(x + y, x + l))
  println(Ops.same(x < y, y < x))
  println(Ops.same(x == y, y == 2))
