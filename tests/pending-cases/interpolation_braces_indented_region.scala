// The braces of an interpolated expression hold indented blocks over lines: an `if`'s branches with their own
// statements. scalac prints vp.
@main def main(): Unit =
  val x = 3
  println(s"""v${
    if x > 0 then
      val p = "p"
      p
    else
      "n"
  }""")
