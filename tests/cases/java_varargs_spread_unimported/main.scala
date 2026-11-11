@main def run(): Unit =
  val s = new Spread
  println(s.path(Array("a", "b")))
  println(s.format(Array("x", "y")))
  println(s.list(Array("x", "y")))
  println(s.boxed(Array(1, 2)))
