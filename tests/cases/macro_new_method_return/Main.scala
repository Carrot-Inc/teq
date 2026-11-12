def twice(): Int =
  val f = early
  val a = f(41)
  println("after " + a)
  a + f(-1)
@main def run(): Unit =
  println(twice())
