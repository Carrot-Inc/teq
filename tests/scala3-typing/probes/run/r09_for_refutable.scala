@main def run(): Unit =
  val xs = List(Option(1), None, Some(3))
  val ys = for Some(x) <- xs yield x
  println(ys)
