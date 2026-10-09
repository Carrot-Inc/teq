package erasedmain

@main def run(): Unit =
  println(erased.B.number(List(42)))
  println(erased.B.word(List("yes")))
