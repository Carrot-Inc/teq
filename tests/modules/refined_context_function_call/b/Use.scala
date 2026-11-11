package cfb

@main def run(): Unit =
  val g: String ?=> List[Option[Nothing]] => Int = cfa.Lib.widen[Int]((s: String) ?=> xs => s.length + xs.size)
  println(g(using "abc")(List(None)))
