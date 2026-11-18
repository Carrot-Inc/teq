// After a for's enumerators in braces, the lines further in are its body, without a `do`. scalac prints 6.
@main def main(): Unit =
  var s = 0
  for { x <- List(1, 2, 3) }
    val y = x
    s += y
  println(s)
