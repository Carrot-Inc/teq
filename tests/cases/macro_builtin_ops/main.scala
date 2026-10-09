import Macros.*

@main def run(): Unit =
  val flag = false
  println(negated(flag))
  println(negated(true))
  val n = 21
  println(doubled(n))
  println(negative(n))
  println(joined("a", "b"))
