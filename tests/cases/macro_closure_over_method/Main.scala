@main def run(): Unit =
  println(twiceOf(new Fns)(21))
  val put = putOf(new Box(1))
  println(put(2).map(_ + 1))
