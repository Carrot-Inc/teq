import Macros.*

given String = "x"

@main def run(): Unit =
  println(ctor[Api])
  println(make[Api].show)
  println(curried)
  println(threeClauses)
  println(swapped)
  println(dependent)
