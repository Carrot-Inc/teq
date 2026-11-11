@main def run(): Unit =
  val f: Int => Int = x => x + 1
  val fs = (f, f, f, f, f, f, f, f, f, f, f, f, f, f, f, f, f, f, f, f, f, f, f, f, f)
  println(fs.reverse(0)(1))
  println(fs.tail(0)(2))
  println(fs.init(0)(3))
