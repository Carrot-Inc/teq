// A lone surrogate escaped in a string literal is one UTF-16 unit, as on the JVM and in
// JavaScript: the string is never printed, since the platforms print it differently.
@main def Main(): Unit =
  println("\uD800".length)
  println("\uDC00".length)
  println("a\uD800b".length)
  println(("\uD800" + "x").length)
  println("\uD800".charAt(0).toInt)
  println("a\uDBFFb".charAt(1).toInt)
  println("😀".length)
  println("\uD800" == "\uD800")
  println("\uD800" == "\uDC00")
  println('\uDC01'.toInt)
