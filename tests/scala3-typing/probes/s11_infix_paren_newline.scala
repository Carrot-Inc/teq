@main def run(): Unit =
  val x = 1 +
    (2)
  println(x)
  val ys = List(1, 2, 3)
    .map(_ + 1)
    .filter(_ > 2)
  println(ys)
