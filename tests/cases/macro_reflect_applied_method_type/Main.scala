@main def run(): Unit =
  val n = 3
  println(Macros.result(n + 2))
  println(Macros.result(s"n=$n"))
  println(Macros.result(((x: Int) => x * 2)(n)))
