// expect: invalid escape sequence
@main def run(): Unit =
  val k = 1
  println(s"\$k")
