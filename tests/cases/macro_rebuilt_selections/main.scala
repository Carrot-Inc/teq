@main def run(): Unit =
  println(M.shape(1, "a", 2.5))
  val xs = List(1, 2)
  println(M.shape(xs*))
  val x = 6
  println(M.rebuilt(x == 6))
  println(M.rebuilt(x > 7))
  val s = "str"
  val t: AnyRef = s
  println(M.rebuilt(t eq s))
