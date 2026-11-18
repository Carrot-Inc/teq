// A case's body is a block's statements: the lines after a body that begins on the case's line, further in than
// the cases, at two widths, after a block in braces; in braces every line up to the next `case`, at the cases'
// own width too; and a line at the width of indented cases that begins with no `case` ends them and is the next
// statement after the match. scalac prints the same lines.
var seen = ""

def indentedCases(n: Int): String = n match
  case 1 => seen += "a"
    seen += "b"
    "one"
  case _ => "other"

def casesAtMatchWidth(n: Int): String =
  n match
  case 1 => "one"
  case _ => seen += "c"
    seen += "d"
    "other"

def twoWidths(n: Int): String = n match
  case 1 => seen += "e"
      seen += "f"
    seen += "g"
    "one"
  case _ => "other"

def afterBraceBlock(n: Int): String = n match
  case 1 => { seen += "h" }
    "one"
  case _ => "other"

def definitionFirst(xs: List[Int], flag: Boolean): Int = flag match {
  case true => def twice(i: Int) = i * 2
    var sum = 0
    for x <- xs do sum += twice(x)
    sum
  case false => 0
}

def inBracesAtCasesWidth(x: Int): Int = x match {
  case 1 =>
  seen += "i"
  seen += "j"
  10
  case _ => 0
}

def lambdaOfCases(xs: List[Int]): List[Int] = xs.map {
  case 1 => seen += "k"
  10
  case _ => 0
}

def statementAfterCases(x: Int): Unit =
  x match
    case 1 => seen += "l"
    case _ => seen += "m"
    seen += "n"

@main def main(): Unit =
  println((indentedCases(1), indentedCases(2), casesAtMatchWidth(1), casesAtMatchWidth(2)))
  println((twoWidths(1), afterBraceBlock(1), definitionFirst(List(1, 2, 3), true), definitionFirst(List(1), false)))
  println((inBracesAtCasesWidth(1), inBracesAtCasesWidth(2), lambdaOfCases(List(1, 2))))
  statementAfterCases(1)
  statementAfterCases(2)
  println(seen)
