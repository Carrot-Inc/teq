// Cases at the width of their `match`, the last case's body on the case's line ending in the indented body of a
// `while` or of a `for`: a `match` on a line of the cases' width ends the cases and matches the whole match before
// it, not the loop's body. scalac prints the same lines.
var log = ""

def afterWhile(x: Int): Any =
  x match
  case 1 => "a"
  case _ => while log.length < x do
    log += "w"
  match
  case "a" => 1
  case _ => 2

def afterFor(x: Int): Any =
  x match
  case 1 => "a"
  case _ => for i <- 1 to x do
    log += i
  match
  case "a" => 1
  case _ => 2

@main def main(): Unit =
  println((afterWhile(1), afterWhile(3), log))
  println((afterFor(1), afterFor(2), log))
