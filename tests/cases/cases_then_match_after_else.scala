// Cases at the width of their `match`, the last case's body on the case's line ending in the indented branch of an
// `if`: a `match` on a line of the cases' width ends the cases and matches the whole match before it, not the
// branch. scalac prints the same lines.
def afterElse(x: Int): Any =
  x match
  case 1 => "a"
  case _ => if x > 5 then "b" else
    "c"
  match
  case "a" => 1
  case _ => 2

def afterThen(x: Int): Any =
  x match
  case 1 => "a"
  case _ => if x > 5 then
    println("then " + x)
  match
  case "a" => 1
  case _ => 2

def afterElseTwoLines(x: Int): Any =
  x match
  case 1 => "a"
  case _ => if x > 5 then "b" else
    val s = "c"
    s
  match
  case "a" => 1
  case "c" => 2
  case _ => 3

@main def main(): Unit =
  println((afterElse(1), afterElse(3), afterElse(9)))
  println((afterThen(1), afterThen(3), afterThen(9)))
  println((afterElseTwoLines(1), afterElseTwoLines(3), afterElseTwoLines(9)))
