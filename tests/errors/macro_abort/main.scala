// expect: main.scala:10:11: error: expected a positive number, got -3
// expect: main.scala:11:11: error: expected a literal
// expect: main.scala:12:11: error: first
// expect: main.scala:12:11: error: second
// expect: 4 errors found
import Macros.*

@main def run(): Unit =
  println(positive(3))
  println(positive(-3))
  println(positive(scala.util.Random.nextInt()))
  println(twoErrors)
