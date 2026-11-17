// Cases at the width of their `match`, the last case's body on the case's line ending in the indented block of a
// `finally`: a `match` on a line of the cases' width ends the cases and matches the whole match before it, not the
// finalizer. scalac prints the same lines.
def afterFinally(x: Int): Any =
  x match
  case 1 => "a"
  case _ => try "b" finally
    println("finally " + x)
  match
  case "a" => 1
  case _ => 2

@main def main(): Unit =
  println((afterFinally(1), afterFinally(3)))
