// Cases written at the width of their `match` are a region of that width: a line at that width that does not begin
// with `case` ends them, so a `match` there matches the whole match before it, whatever the last case's body is:
// an indented expression, a block in braces, two statements. An operator that leads such a line ends no region and
// continues the last case's body, under cases indented below their `match` too. scalac prints the same lines.
def chained(x: Int): Any =
  x match
  case 1 => "a"
  case _ =>
    "b"
  match
  case "a" => 1
  case _ => 2

def bracedBody(x: Int): Any =
  x match
  case 1 => "a"
  case _ => { "b" }
  match
  case "a" => 1
  case _ => 2

def twoStatements(x: Int): Any =
  x match
  case 1 => "a"
  case _ =>
    val s = "b"
    s
  match
  case "a" => 1
  case _ => 2

def atMargin(x: Int): Any = x match
case 1 => "a"
case _ =>
  "b"
match
case "a" => 1
case _ => 2

def operatorLine(x: Int): Any =
  x match
  case 1 => "a"
  case _ =>
    "bcd"
  + "z"

def operatorBelowIndentedCases(x: Int): Int = x match
  case 1 =>
    val a = 10
    a
  + 1
  case _ => 0

@main def main(): Unit =
  println((chained(1), chained(5), bracedBody(1), bracedBody(5), twoStatements(1), twoStatements(5), atMargin(1), atMargin(5)))
  println((operatorLine(1), operatorLine(5), operatorBelowIndentedCases(1), operatorBelowIndentedCases(2)))
