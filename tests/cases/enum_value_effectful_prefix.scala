// An enum value selected on a prefix that is no path: the prefix is evaluated before the value is read, as the
// qualifier of any selection is.
enum Color { case Red, Green }
def pick(): Color.type = { println("pick"); Color }
@main def m(): Unit =
  println(pick().Red)
  println(Color.Green)
