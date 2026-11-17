// Cases at the width of their `match`, the last case's body on the case's line ending in the indented expression of
// a `yield`: a `match` on a line of the cases' width ends the cases and matches the whole match before it, not the
// yielded expression. scalac prints the same line.
def afterYield(x: Int): Any =
  x match
  case 1 => List("a")
  case _ => for y <- List("b", "c") yield
    y + x
  match
  case List("a") => 1
  case l => l.length + 1

@main def main(): Unit =
  println((afterYield(1), afterYield(3)))
