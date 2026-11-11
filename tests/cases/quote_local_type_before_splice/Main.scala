@main def run(): Unit =
  println(LocalQuote.keep(42))
  println(LocalQuote.keep("retained"))
