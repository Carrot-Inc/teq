// A guard's colon lambda whose body is indented, the case's `=>` on the line after it.
@main def run(): Unit =
  println(1 match
    case n if List(1).exists: x =>
      x > 0
    => n
    case _ => 0)
