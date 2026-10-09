//> using platform js
// The unit printed: `println(())` and `print(())` show Scala.js's `undefined` (scalac's `()` on
// the JVM, tests/jvm-expected), the empty `println()` an empty line, and the overloads stay
// those of a function value.
@main def run(): Unit =
  println(())
  println()
  println("x")
  print(())
  println()
  List(1, 2).foreach(println)
  val f: Any => Unit = println
  f(3)
  Console.println(())
