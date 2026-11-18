// Cases at the width of their `match`, the last case's body a selection on the case's line: a `match` on a line of
// the cases' width ends the cases and matches the whole match. scalac prints (1,3,3).
def chained(x: Int): Any =
  x match
  case 1 => "a"
  case _ => "c".toUpperCase
  match
  case "a" => 1
  case "C" => 3
  case _ => 2

@main def main(): Unit = println((chained(1), chained(3), chained(9)))
