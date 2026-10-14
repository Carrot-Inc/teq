@main def main(): Unit =
  println(Macros.count({ case n: Int => n + 1 }, "a", 2))
  println(Macros.shown(Map(1 -> "one")))
