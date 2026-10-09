def f(): Unit = 5
def g(): Unit =
  val x = 1
  x + 1
@main def run(): Unit =
  f()
  g()
  val u: Unit = "s"
  println(u)
