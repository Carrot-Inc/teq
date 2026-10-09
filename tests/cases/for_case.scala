@main def run(): Unit =
  val xs = List(Option(1), None, Some(3))
  val ys = for case Some(x) <- xs yield x
  println(ys)
  val zs: List[Any] = List(1, "s", 2)
  val ints = for case i: Int <- zs yield i
  println(ints)
  for case Some(v) <- xs do println(v)
