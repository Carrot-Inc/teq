// A value definition's parenthesised alternatives across a line.
@main def run(): Unit =
  val (1 |
    2) = 1: @unchecked
  println(1)
