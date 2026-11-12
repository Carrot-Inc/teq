@main def run(): Unit =
  val a: Any = "abc"
  println(castTo[String](a).length)
