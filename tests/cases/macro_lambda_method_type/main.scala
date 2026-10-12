@main def run(): Unit =
  val f = Macros.adder(10)
  println(f(1))
  println(List(1, 2).map(Macros.adder(100)))
