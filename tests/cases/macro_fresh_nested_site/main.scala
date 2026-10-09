@main def run(): Unit =
  println(Fresh.outer())
  println(Fresh.two())
  println(Fresh.inner() == Fresh.inner())
