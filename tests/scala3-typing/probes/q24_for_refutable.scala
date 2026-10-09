@main def run(): Unit =
  val xs = List(Option(1), None)
  val ys = for Some(x) <- xs yield x
  println(ys)
  val (a, b) = (1, 2)
  println(a + b)
  val zs: List[Any] = List(1, "s")
  val ints = for (i: Int) <- zs yield i
  println(ints)
