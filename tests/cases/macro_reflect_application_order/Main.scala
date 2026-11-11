@main def run(): Unit =
  println(Inspect.result)
  val p = Make.pair
  println(p.a.toString + "," + p.b.toString)
  println(Make.method)
