// The braces of an interpolated expression hold a block's statements. scalac prints a2b.
@main def main(): Unit =
  val s = s"a${ val x = 1; x + 1 }b"
  println(s)
