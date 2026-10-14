@main def run(): Unit =
  val f: Int => String = (x: Int) =>
    val local = 1
    inspect
  println(f(2))
  val g: (Int, String) => Int = (a, b) => inspectCount
  println(g(1, "b"))
